//! Site tab commands (ADR-023): API keys, parcel lookup, the lot and its topography.
//! Network calls run on a blocking thread so the UI stays responsive. The Google Maps key
//! goes to the page (Google's map runs there); the Regrid token never leaves Rust. Both
//! live in the OS credential store.

use serde::Serialize;
use studio_core::site::{self, ParcelInfo};
use studio_sync::gis;
use tauri::{Emitter, State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

const SERVICE: &str = "Rufplan Studio";
const GOOGLE: &str = "google-maps-api-key";
const REGRID: &str = "regrid-api-token";

fn get(user: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, user)
        .ok()?
        .get_password()
        .ok()
        .filter(|s| !s.is_empty())
}

fn put(user: &str, value: &str) -> anyhow::Result<()> {
    let e = keyring::Entry::new(SERVICE, user)?;
    if value.trim().is_empty() {
        let _ = e.delete_credential();
    } else {
        e.set_password(value.trim())?;
    }
    Ok(())
}

/// Which keys are set (and the Google key itself, which the map needs in the page).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SiteKeys {
    pub google_key: Option<String>,
    pub regrid: bool,
}

#[tauri::command]
pub fn site_keys() -> SiteKeys {
    SiteKeys {
        google_key: get(GOOGLE),
        regrid: get(REGRID).is_some(),
    }
}

/// Saves keys; None leaves one unchanged, an empty string removes it.
#[tauri::command]
pub fn site_set_keys(google: Option<String>, regrid: Option<String>) -> CommandResult<SiteKeys> {
    if let Some(g) = google {
        put(GOOGLE, &g)?;
    }
    if let Some(r) = regrid {
        put(REGRID, &r)?;
    }
    Ok(site_keys())
}

/// A parcel from Regrid: its boundary as [lat, lon] pairs and its record.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParcelHit {
    pub ring: Vec<[f64; 2]>,
    pub apn: String,
    pub owner: String,
    pub address: String,
    pub acres: f64,
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(anyhow::Error::from)?
        .map_err(Into::into)
}

/// The parcel at (lat, lon), from Regrid.
#[tauri::command]
pub async fn site_parcel(lat: f64, lon: f64) -> CommandResult<ParcelHit> {
    blocking(move || {
        let token = get(REGRID).ok_or_else(|| {
            anyhow::anyhow!("add your Regrid token (Site > API Keys) to look up parcels")
        })?;
        let p = gis::regrid_parcel(&studio_sync::UreqHttp::default(), &token, lat, lon)?;
        Ok(ParcelHit {
            ring: p.ring.iter().map(|(la, lo)| [*la, *lo]).collect(),
            apn: p.apn,
            owner: p.owner,
            address: p.address,
            acres: p.acres,
        })
    })
    .await
}

/// Uses a lot: its boundary as [lat, lon] pairs, and its record.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn site_set_lot(
    ring: Vec<[f64; 2]>,
    apn: String,
    owner: String,
    address: String,
    acres: f64,
    source: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    let pts: Vec<(f64, f64)> = ring.iter().map(|p| (p[0], p[1])).collect();
    s.edit(|d| {
        site::set_lot(
            d,
            &pts,
            ParcelInfo {
                apn,
                owner,
                address,
                acres,
                source,
            },
        )
    })?;
    finish(&window, &s)
}

/// Samples the ground from USGS 3DEP `spacing` apart (mm) over the lot, or a square
/// `extent` (2 to 4) times its size around it, plus `margin` (ADR-026).
#[tauri::command]
pub async fn site_fetch_topo(
    spacing: f64,
    margin: f64,
    extent: Option<f64>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let site::TopoGrid {
        nx,
        ny,
        origin,
        spacing,
        pts,
    } = {
        let s = lock(&state)?;
        site::topo_grid(s.doc()?, spacing, margin, extent.unwrap_or(1.0))?
    };
    // Progress to the page as (batches done, batches): USGS can take a while when busy.
    let win = window.clone();
    let (values, resolution) = blocking(move || {
        Ok(gis::elevations_with(
            &studio_sync::UreqHttp::default(),
            &pts,
            &|done, total| {
                let _ = win.emit("topo-progress", (done, total));
            },
            std::time::Duration::from_secs(3),
        )?)
    })
    .await?;
    let mut s = lock(&state)?;
    s.edit(|d| site::set_topo(d, (nx, ny, origin), spacing, &values, resolution))?;
    finish(&window, &s)
}

/// Where the satellite image for the site goes, and what to fetch (ADR-026).
#[tauri::command]
pub fn site_imagery_frame(state: State<'_, SessionState>) -> CommandResult<site::ImageryFrame> {
    let s = lock(&state)?;
    Ok(site::imagery_frame(s.doc()?)?)
}

/// The satellite image for `frame` from Google's Maps Static API, as raw bytes (kept in
/// memory by the page; never saved).
#[tauri::command]
pub async fn site_imagery(frame: site::ImageryFrame) -> CommandResult<tauri::ipc::Response> {
    let key = get(GOOGLE).ok_or_else(|| {
        anyhow::anyhow!("add your Google Maps key (Site > API Keys) for the satellite overlay")
    })?;
    let bytes = blocking(move || {
        Ok(gis::static_map(
            &studio_sync::UreqHttp::default(),
            &key,
            frame.lat,
            frame.lon,
            frame.zoom,
            frame.width,
            frame.height,
        )?)
    })
    .await?;
    Ok(tauri::ipc::Response::new(bytes))
}
