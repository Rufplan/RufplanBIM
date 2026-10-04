//! IPC commands for the title block (ADR-111): its fields, and whether a sheet point lies on
//! it (double-clicking there opens them). Thin wrappers over studio-sheets `titleblock`.

use studio_core::ElementId;
use studio_geom::Pt;
use studio_sheets::titleblock::{self, TitleBlockFields};
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub fn title_block_fields(
    sheet: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<TitleBlockFields> {
    let s = lock(&state)?;
    Ok(titleblock::fields(s.doc()?, sheet)?)
}

#[tauri::command]
pub fn set_title_block_fields(
    sheet: ElementId,
    fields: TitleBlockFields,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| titleblock::set_fields(d, sheet, &fields))?;
    finish(&window, &s)
}

#[tauri::command]
pub fn title_block_at(
    sheet: ElementId,
    at: Pt,
    state: State<'_, SessionState>,
) -> CommandResult<bool> {
    let s = lock(&state)?;
    Ok(titleblock::hit(s.doc()?, sheet, at))
}
