//! IPC commands for worksets (ADR-079). Thin wrappers over studio-core `worksets`.

use studio_core::worksets::{self, WorksetInfo};
use studio_core::ElementId;
use tauri::{State, WebviewWindow};

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

/// The Worksets dialog's list, with element counts.
#[tauri::command]
pub fn worksets_list(state: State<'_, SessionState>) -> Result<Vec<WorksetInfo>, CommandError> {
    let session = lock(&state)?;
    Ok(worksets::list(session.doc()?))
}

/// Collaborate > Active Workset: where new elements go (this session only).
#[tauri::command]
pub fn set_active_workset(
    ws: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        if !matches!(d.data(ws)?, studio_core::ElementData::Workset { .. }) {
            return Err(studio_core::CoreError::Invalid("pick a workset".into()));
        }
        d.set_active_workset(Some(ws));
        Ok(())
    })
}

#[tauri::command]
pub fn create_workset(
    name: String,
    visible: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        worksets::create(d, &name, visible).map(|_| ())
    })
}

#[tauri::command]
pub fn rename_workset(
    ws: ElementId,
    name: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| worksets::rename(d, ws, &name))
}

/// Deletes `ws`, moving its elements to `move_to`. If it was active, the default becomes
/// active.
#[tauri::command]
pub fn delete_workset(
    ws: ElementId,
    move_to: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        worksets::delete(d, ws, move_to)?;
        if d.active_workset() == Some(ws) {
            let def = worksets::default_workset(d);
            d.set_active_workset(def);
        }
        Ok(())
    })
}

#[tauri::command]
pub fn set_workset_visible_in_all_views(
    ws: ElementId,
    visible: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        worksets::set_visible_in_all_views(d, ws, visible)
    })
}

#[tauri::command]
pub fn set_workset_visible_in_view(
    view: ElementId,
    ws: ElementId,
    visible: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        worksets::set_visible_in_view(d, view, ws, visible)
    })
}

/// Moves the selection to `ws` (Modify > Workset).
#[tauri::command]
pub fn set_elements_workset(
    ids: Vec<ElementId>,
    ws: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |d| {
        worksets::set_workset(d, &ids, ws).map(|_| ())
    })
}

/// Each model element's workset, for Gray Inactive Workset Graphics.
#[tauri::command]
pub fn element_worksets(
    state: State<'_, SessionState>,
) -> Result<Vec<(ElementId, ElementId)>, CommandError> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    Ok(doc
        .iter()
        .filter_map(|e| worksets::workset_of(doc, e.id).map(|w| (e.id, w)))
        .collect())
}
