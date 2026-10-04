//! IPC commands for editing the sheet index (ADR-113) and turning view titles off. Thin
//! wrappers over studio-sheets `sheet_index` and `sheet`.

use studio_core::ElementId;
use studio_sheets::sheet_index::{self, IndexRow};
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

/// The sheet index's rows in index order.
#[tauri::command]
pub fn sheet_index_rows(state: State<'_, SessionState>) -> CommandResult<Vec<IndexRow>> {
    let s = lock(&state)?;
    Ok(sheet_index::rows(s.doc()?))
}

#[tauri::command]
pub fn set_sheet_index_rows(
    rows: Vec<IndexRow>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| sheet_index::set_rows(d, &rows))?;
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
