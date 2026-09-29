//! IPC commands for Suggest Structure and the structural overlay (ADR-080). Thin wrappers
//! over `studio-structural` (analysis) and studio-views `structural` (overlay).
//!
//! Preliminary — not engineered. Requires review by a licensed structural engineer.

use serde::Serialize;
use studio_core::structural::{SchemeSettings, Seismic, StructLayout, DISCLAIMER};
use studio_core::{Category, ElementData, ElementId};
use studio_geom::Pt;
use studio_structural::{Rules, StructuralProposal};
use studio_views::structural::{OverlayInfo, OverlayMesh, OverlayPrim};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

/// The editable rules file in the app's data folder, written from the built-in copy the
/// first time.
fn rules_file(app: &tauri::AppHandle) -> anyhow::Result<std::path::PathBuf> {
    let dir = app.path().app_local_data_dir()?;
    let path = dir.join("structural_rules.toml");
    if !path.exists() {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&path, studio_structural::rules::DEFAULT_RULES)?;
    }
    Ok(path)
}

fn load_rules(app: &tauri::AppHandle) -> anyhow::Result<Rules> {
    let path = rules_file(app)?;
    let text = std::fs::read_to_string(&path)?;
    Rules::parse(&text).map_err(|e| anyhow::anyhow!("{e} (in {})", path.display()))
}

/// Suggest Structure: the features scored against every scheme.
#[tauri::command]
pub fn structural_suggest(
    seismic: Seismic,
    app: tauri::AppHandle,
    state: State<'_, SessionState>,
) -> Result<StructuralProposal, CommandError> {
    let rules = load_rules(&app)?;
    let session = lock(&state)?;
    let doc = session.doc()?;
    let model = studio_regen::regenerate(doc);
    let features = studio_structural::extract(doc, &model, &rules);
    Ok(studio_structural::propose(&features, &rules, seismic))
}

/// Generate overlay: lays out `settings` and saves it as the structural layer (one per
/// project, on the Structural workset), replacing any earlier one. One undo step; the
/// architecture is untouched.
#[tauri::command]
pub fn structural_generate(
    settings: SchemeSettings,
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<Option<AppState>, CommandError> {
    let rules = load_rules(&app)?;
    let mut session = lock(&state)?;
    let layout = {
        let doc = session.doc()?;
        let model = studio_regen::regenerate(doc);
        let features = studio_structural::extract(doc, &model, &rules);
        studio_structural::layout(&features, &model, &rules, &settings)
    };
    session.edit(|d| {
        let existing = d.of(Category::StructuralScheme).next().map(|e| e.id);
        d.transact("Generate Structural Layer", |tx| {
            let data = ElementData::StructuralScheme { settings, layout };
            match existing {
                Some(id) => tx.set(id, data),
                None => {
                    tx.insert(data);
                    Ok(())
                }
            }
        })
    })?;
    finish(&window, &session)
}

/// The structural layer for the panel and exports.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StructuralLayer {
    pub id: ElementId,
    pub settings: SchemeSettings,
    pub layout: StructLayout,
    pub disclaimer: String,
}

#[tauri::command]
pub fn structural_layer(
    state: State<'_, SessionState>,
) -> Result<Option<StructuralLayer>, CommandError> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let layer = doc
        .of(Category::StructuralScheme)
        .find_map(|e| match &e.data {
            ElementData::StructuralScheme { settings, layout } => Some(StructuralLayer {
                id: e.id,
                settings: settings.clone(),
                layout: layout.clone(),
                disclaimer: DISCLAIMER.into(),
            }),
            _ => None,
        });
    Ok(layer)
}

/// The overlay for a plan view.
#[tauri::command]
pub fn structural_overlay_2d(
    view: ElementId,
    state: State<'_, SessionState>,
) -> Result<Vec<OverlayPrim>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::structural::overlay_2d(session.doc()?, view))
}

/// The overlay in 3D.
#[tauri::command]
pub fn structural_overlay_3d(
    state: State<'_, SessionState>,
) -> Result<Vec<OverlayMesh>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::structural::overlay_3d(session.doc()?))
}

/// What's under the cursor in a plan's overlay.
#[tauri::command]
pub fn structural_pick(
    view: ElementId,
    at: Pt,
    tol: f64,
    state: State<'_, SessionState>,
) -> Result<Option<OverlayInfo>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::structural::pick(
        session.doc()?,
        view,
        at,
        tol,
    ))
}

/// A member's or flag's information (3D picking).
#[tauri::command]
pub fn structural_info(
    member: Option<u32>,
    flag: Option<u32>,
    state: State<'_, SessionState>,
) -> Result<Option<OverlayInfo>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::structural::info(session.doc()?, member, flag))
}

fn with_extension(path: &str, ext: &str) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case(ext)) {
        p.set_extension(ext);
    }
    p
}

/// Exports the structural layer as JSON: the disclaimer, settings and layout.
#[tauri::command]
pub fn structural_export_json(
    path: String,
    state: State<'_, SessionState>,
) -> Result<String, CommandError> {
    let layer = structural_layer(state)?.ok_or_else(|| {
        anyhow::anyhow!("generate a structural layer first (Structure > Suggest Structure)")
    })?;
    let path = with_extension(&path, "json");
    let text = serde_json::to_string_pretty(&layer).map_err(anyhow::Error::from)?;
    std::fs::write(&path, text)
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

/// Exports the model with the preliminary structural layer as IFC4.
#[tauri::command]
pub fn structural_export_ifc(
    path: String,
    state: State<'_, SessionState>,
) -> Result<String, CommandError> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let (text, sum) = studio_io::ifc::export_ifc_with(
        doc,
        env!("CARGO_PKG_VERSION"),
        &crate::commands::now_iso(),
        true,
    );
    if sum.structural == 0 {
        return Err(anyhow::anyhow!("generate a structural layer first").into());
    }
    let path = with_extension(&path, "ifc");
    std::fs::write(&path, text)
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", path.display()))?;
    Ok(format!(
        "{} structural members to {}",
        sum.structural,
        path.display()
    ))
}

/// Opens the rules file in the default editor (made from the built-in rules if missing).
#[tauri::command]
pub fn structural_edit_rules(app: tauri::AppHandle) -> Result<String, CommandError> {
    let path = rules_file(&app)?;
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| anyhow::anyhow!("could not open {}: {e}", path.display()))?;
    Ok(path.display().to_string())
}
