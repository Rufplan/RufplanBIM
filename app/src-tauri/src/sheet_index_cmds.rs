//! IPC commands for editing the sheet index (ADR-113) and turning view titles off. Thin
//! wrappers over studio-sheets `sheet_index` and `sheet`.

use studio_core::ElementId;
use studio_sheets::sets::BuildingType;
use studio_sheets::sheet_index::{self, IndexIssue, IndexRow};
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

/// The sheet index's rows in index order: `stage`'s set, else the current stage's.
#[tauri::command]
pub fn sheet_index_rows(
    stage: Option<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<Vec<IndexRow>> {
    let s = lock(&state)?;
    let doc = s.doc()?;
    Ok(sheet_index::rows_for(
        doc,
        stage.or_else(|| sheet_index::current_stage(doc)),
    ))
}

/// The issues (phases' deliverables) the index offers, and the current stage (ADR-116).
#[tauri::command]
pub fn sheet_index_issues(
    state: State<'_, SessionState>,
) -> CommandResult<(Vec<IndexIssue>, Option<ElementId>)> {
    let s = lock(&state)?;
    let doc = s.doc()?;
    Ok((sheet_index::issues(doc), sheet_index::current_stage(doc)))
}

/// A stage's typical sheet list for the building type.
#[tauri::command]
pub fn sheet_index_typical(
    stage: ElementId,
    building: BuildingType,
    state: State<'_, SessionState>,
) -> CommandResult<Vec<IndexRow>> {
    let s = lock(&state)?;
    Ok(sheet_index::typical(s.doc()?, stage, building))
}

#[tauri::command]
pub fn set_sheet_index_rows(
    rows: Vec<IndexRow>,
    stage: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| sheet_index::set_rows(d, &rows, stage))?;
    finish(&window, &s)
}

/// The number a sheet added after `after` gets, not one of `taken`.
#[tauri::command]
pub fn next_index_number(after: String, taken: Vec<String>) -> String {
    sheet_index::next_number(&after, &taken)
}

#[tauri::command]
pub fn viewport_title_shown(
    viewport: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<bool> {
    let s = lock(&state)?;
    Ok(studio_sheets::sheet::title_shown(s.doc()?, viewport))
}

#[tauri::command]
pub fn set_viewport_title_shown(
    viewport: ElementId,
    shown: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| studio_sheets::sheet::set_title_shown(d, viewport, shown))?;
    finish(&window, &s)
}
