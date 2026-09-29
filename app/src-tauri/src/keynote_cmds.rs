//! IPC commands for keynotes (ADR-081). Thin wrappers over studio-core `keynotes`.

use serde::Serialize;
use studio_core::keynotes::{
    self, Assignable, Keynote, KeynoteNumbering, KeynoteSource, KeynoteStyle, KeynoteUse,
};
use studio_core::ElementId;
use studio_geom::Pt;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type StateResult = Result<Option<AppState>, CommandError>;

fn edit(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&mut studio_core::Document) -> studio_core::CoreResult<()>,
) -> StateResult {
    let mut session = lock(state)?;
    session.edit(f)?;
    finish(window, &session)
}

/// The Keynote Manager's table: the keynotes in key order, the numbering, and where each is
/// used.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeynoteTableInfo {
    pub entries: Vec<Keynote>,
    pub numbering: KeynoteNumbering,
    pub usage: Vec<KeynoteUse>,
}

#[tauri::command]
pub fn keynote_table(state: State<'_, SessionState>) -> Result<KeynoteTableInfo, CommandError> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let (entries, numbering) = keynotes::table(doc);
    Ok(KeynoteTableInfo {
        entries: keynotes::sorted(&entries),
        numbering,
        usage: keynotes::usage(doc),
    })
}

#[tauri::command]
pub fn keynote_save(
    old_key: Option<String>,
    entry: Keynote,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        keynotes::save(d, old_key.as_deref(), entry)
    })
}

#[tauri::command]
pub fn keynote_delete(
    key: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| keynotes::delete(d, &key).map(|_| ()))
}

#[tauri::command]
pub fn keynote_set_numbering(
    numbering: KeynoteNumbering,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| keynotes::set_numbering(d, numbering))
}

/// Loads a Revit keynote file (.txt), replacing the table or merging into it.
#[tauri::command]
pub fn keynote_import(
    path: String,
    replace: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<(usize, Option<AppState>), CommandError> {
    let bytes = std::fs::read(&path).map_err(|e| anyhow::anyhow!("could not read {path}: {e}"))?;
    // Revit writes keynote files as UTF-8 or UTF-16 (with a byte-order mark).
    let text = if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes))
            .into_owned()
    };
    let mut session = lock(&state)?;
    let n = session.edit(|d| keynotes::import(d, &text, replace))?;
    Ok((n, finish(&window, &session)?))
}

/// Saves the table as a Revit keynote file (.txt).
#[tauri::command]
pub fn keynote_export(
    path: String,
    state: State<'_, SessionState>,
) -> Result<String, CommandError> {
    let session = lock(&state)?;
    let (entries, _) = keynotes::table(session.doc()?);
    let mut p = std::path::PathBuf::from(&path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("txt")) {
        p.set_extension("txt");
    }
    std::fs::write(&p, keynotes::to_text(&entries))
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", p.display()))?;
    Ok(p.display().to_string())
}

/// Assigns a keynote (or clears it) on types and materials.
#[tauri::command]
pub fn keynote_assign(
    ids: Vec<ElementId>,
    key: Option<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        keynotes::assign(d, &ids, key.as_deref()).map(|_| ())
    })
}

/// Types and materials, with their keynotes.
#[tauri::command]
pub fn keynote_assignables(
    state: State<'_, SessionState>,
) -> Result<Vec<Assignable>, CommandError> {
    let session = lock(&state)?;
    Ok(keynotes::assignables(session.doc()?))
}

/// What an Element or Material keynote would tag on `id`: its type and materials with their
/// keynotes.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeynoteTarget {
    pub id: ElementId,
    #[ts(optional)]
    pub type_id: Option<ElementId>,
    pub type_name: String,
    #[ts(optional)]
    pub type_key: Option<String>,
    pub materials: Vec<Assignable>,
}

#[tauri::command]
pub fn keynote_target(
    id: ElementId,
    state: State<'_, SessionState>,
) -> Result<Option<KeynoteTarget>, CommandError> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let Ok(data) = doc.data(id) else {
        return Ok(None);
    };
    let type_id = data.type_id();
    let materials = keynotes::materials_of(doc, id)
        .into_iter()
        .filter_map(|m| {
            Some(Assignable {
                id: m,
                category: studio_core::Category::Material,
                name: doc.data(m).ok()?.name(),
                key: keynotes::assigned(doc, m),
            })
        })
        .collect();
    Ok(Some(KeynoteTarget {
        id,
        type_id,
        type_name: type_id
            .and_then(|t| doc.data(t).ok())
            .map(|d| d.name())
            .unwrap_or_default(),
        type_key: type_id.and_then(|t| keynotes::assigned(doc, t)),
        materials,
    }))
}

/// Places a keynote tag.
#[tauri::command]
pub fn keynote_place(
    view: ElementId,
    source: KeynoteSource,
    arrow: Option<Pt>,
    at: Pt,
    style: KeynoteStyle,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        keynotes::create_tag(d, view, source, arrow, at, style).map(|_| ())
    })
}

/// Annotate > Keynote Legend: the legend's view (made the first time).
#[tauri::command]
pub fn keynote_legend(
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<(ElementId, Option<AppState>), CommandError> {
    let mut session = lock(&state)?;
    let id = session.edit(keynotes::legend)?;
    Ok((id, finish(&window, &session)?))
}
