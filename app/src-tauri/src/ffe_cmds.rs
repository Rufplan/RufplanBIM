//! IPC commands for furniture and equipment (ADR-090). Thin wrappers over studio-core
//! `ffe` and studio-views `ffe`.

use serde::{Deserialize, Serialize};
use studio_core::ffe::{self, FfeClass, FfePreset, FfeUse};
use studio_core::ElementId;
use studio_geom::Pt;
use studio_views::ffe::FfeThumb;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;
use crate::window_cmds::LoadedWindows;

type CommandResult<T> = Result<T, CommandError>;
type StateResult = Result<Option<AppState>, CommandError>;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FfeUseOption {
    pub id: FfeUse,
    pub label: String,
}

/// The furniture or equipment library: its pieces, the picker's groups and the uses.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FfeLibrary {
    pub presets: Vec<FfePreset>,
    pub groups: Vec<String>,
    pub uses: Vec<FfeUseOption>,
}

#[tauri::command]
pub fn ffe_library(class: FfeClass) -> FfeLibrary {
    FfeLibrary {
        presets: ffe::catalog(class),
        groups: ffe::groups(class),
        uses: FfeUse::ALL
            .iter()
            .map(|u| FfeUseOption {
                id: *u,
                label: u.label().into(),
            })
            .collect(),
    }
}

/// Loads library pieces by name (one undo step), reusing any already there.
#[tauri::command]
pub fn load_ffe_types(
    names: Vec<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<LoadedWindows> {
    let mut s = lock(&state)?;
    let ids = s.edit(|d| ffe::load(d, &names))?;
    let state = finish(&window, &s)?;
    Ok(LoadedWindows { state, ids })
}

/// What an FFE thumbnail shows: a project type, or a library piece by name.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub enum FfeSource {
    Type(ElementId),
    Preset(String),
}

#[tauri::command]
pub fn ffe_thumbnail(source: FfeSource, state: State<'_, SessionState>) -> CommandResult<FfeThumb> {
    let spec = match source {
        FfeSource::Preset(name) => [FfeClass::Furniture, FfeClass::Equipment]
            .into_iter()
            .flat_map(ffe::catalog)
            .find(|p| p.name == name)
            .map(|p| p.spec)
            .ok_or_else(|| anyhow::anyhow!("no piece \"{name}\" in the library"))?,
        FfeSource::Type(id) => {
            let s = lock(&state)?;
            ffe::spec_of(s.doc()?, id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not a furniture or equipment type"))?
        }
    };
    Ok(studio_views::ffe::thumb(&spec))
}

/// Places a piece in a plan, on the view's level: floor pieces at `at`, wall pieces
/// against the nearest wall (their back to it), counter pieces at counter height.
#[tauri::command]
pub fn create_ffe(
    view: ElementId,
    type_id: ElementId,
    at: Pt,
    rotation: Option<f64>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    let level = s.view_level(view)?;
    s.edit(|d| ffe::create(d, type_id, level, at, rotation.unwrap_or(0.0)))?;
    finish(&window, &s)
}
