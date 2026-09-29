//! IPC commands for the MEPT tab (ADR-082): each discipline's Suggest and overlay. Thin
//! wrappers over `studio-mep` (analysis) and studio-views `mep` (overlay).
//!
//! Preliminary — not engineered. Requires review by a licensed engineer.

use serde::Serialize;
use studio_core::mep::{Climate, Discipline, MepLayout, MepSettings};
use studio_core::{Category, ElementData, ElementId};
use studio_geom::Pt;
use studio_mep::{MepProposal, Rules};
use studio_views::structural::{OverlayInfo, OverlayMesh, OverlayPrim};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

fn rules_file(app: &tauri::AppHandle) -> anyhow::Result<std::path::PathBuf> {
    let dir = app.path().app_local_data_dir()?;
    let path = dir.join("mep_rules.toml");
    if !path.exists() {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&path, studio_mep::rules::DEFAULT_RULES)?;
    }
    Ok(path)
}

fn load_rules(app: &tauri::AppHandle) -> anyhow::Result<Rules> {
    let path = rules_file(app)?;
    let text = std::fs::read_to_string(&path)?;
    Rules::parse(&text).map_err(|e| anyhow::anyhow!("{e} (in {})", path.display()))
}

/// A discipline's Suggest: every candidate system scored.
#[tauri::command]
pub fn mep_suggest(
    discipline: Discipline,
    climate: Climate,
    app: tauri::AppHandle,
    state: State<'_, SessionState>,
) -> Result<MepProposal, CommandError> {
    let rules = load_rules(&app)?;
    let session = lock(&state)?;
    let doc = session.doc()?;
    let model = studio_regen::regenerate(doc);
    let f = studio_mep::extract(doc, &model, &rules);
    Ok(studio_mep::propose(discipline, &f, &rules, climate))
}

/// Generate overlay: lays out the chosen system and saves it as that discipline's layer
/// (replacing any earlier one), on the MEP workset. One undo step.
#[tauri::command]
pub fn mep_generate(
    settings: MepSettings,
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<Option<AppState>, CommandError> {
    let rules = load_rules(&app)?;
    let mut session = lock(&state)?;
    let layout = {
        let doc = session.doc()?;
        let model = studio_regen::regenerate(doc);
        let f = studio_mep::extract(doc, &model, &rules);
        studio_mep::layout(&f, &rules, &settings)
    };
    session.edit(|d| {
        let existing = d.of(Category::MepScheme).find_map(|e| match &e.data {
            ElementData::MepScheme { settings: s, .. } if s.discipline == settings.discipline => {
                Some(e.id)
            }
            _ => None,
        });
        let name = format!("Generate {} Layer", settings.discipline.label());
        d.transact(&name, |tx| {
            let data = ElementData::MepScheme { settings, layout };
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

/// A discipline's layer, for exports and the panel.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MepLayer {
    pub id: ElementId,
    pub settings: MepSettings,
    pub layout: MepLayout,
    pub disclaimer: String,
}

fn layer(doc: &studio_core::Document, d: Discipline) -> Option<MepLayer> {
    doc.of(Category::MepScheme).find_map(|e| match &e.data {
        ElementData::MepScheme { settings, layout } if settings.discipline == d => Some(MepLayer {
            id: e.id,
            settings: settings.clone(),
            layout: layout.clone(),
            disclaimer: d.disclaimer().into(),
        }),
        _ => None,
    })
}

#[tauri::command]
pub fn mep_overlay_2d(
    view: ElementId,
    disciplines: Vec<Discipline>,
    state: State<'_, SessionState>,
) -> Result<Vec<OverlayPrim>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::mep::overlay_2d(
        session.doc()?,
        view,
        &disciplines,
    ))
}

#[tauri::command]
pub fn mep_overlay_3d(
    disciplines: Vec<Discipline>,
    state: State<'_, SessionState>,
) -> Result<Vec<OverlayMesh>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::mep::overlay_3d(session.doc()?, &disciplines))
}

#[tauri::command]
pub fn mep_pick(
    view: ElementId,
    at: Pt,
    tol: f64,
    disciplines: Vec<Discipline>,
    state: State<'_, SessionState>,
) -> Result<Option<OverlayInfo>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::mep::pick(
        session.doc()?,
        view,
        at,
        tol,
        &disciplines,
    ))
}

#[tauri::command]
pub fn mep_info(
    item: Option<u32>,
    flag: Option<u32>,
    state: State<'_, SessionState>,
) -> Result<Option<OverlayInfo>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::mep::info(session.doc()?, item, flag))
}

/// Exports a discipline's layer as JSON.
#[tauri::command]
pub fn mep_export_json(
    discipline: Discipline,
    path: String,
    state: State<'_, SessionState>,
) -> Result<String, CommandError> {
    let session = lock(&state)?;
    let l = layer(session.doc()?, discipline).ok_or_else(|| {
        anyhow::anyhow!(
            "generate the {} layer first",
            discipline.label().to_lowercase()
        )
    })?;
    let mut p = std::path::PathBuf::from(&path);
    if p.extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("json"))
    {
        p.set_extension("json");
    }
    let text = serde_json::to_string_pretty(&l).map_err(anyhow::Error::from)?;
    std::fs::write(&p, text)
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", p.display()))?;
    Ok(p.display().to_string())
}

/// Opens the MEPT rules file in the default editor.
#[tauri::command]
pub fn mep_edit_rules(app: tauri::AppHandle) -> Result<String, CommandError> {
    let path = rules_file(&app)?;
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(|e| anyhow::anyhow!("could not open {}: {e}", path.display()))?;
    Ok(path.display().to_string())
}
