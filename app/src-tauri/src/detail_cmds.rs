//! The Details tab (ADR-069): drafting views and the typical detail library. Thin wrappers
//! over studio-core `details` and studio-views `drafting`.

use studio_core::details::{self, DetailInfo};
use studio_core::ElementId;
use studio_views::DisplayList;
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub fn detail_library() -> Vec<DetailInfo> {
    details::catalog()
}

/// A library detail drawn as its drafting view will be (for its thumbnail).
#[tauri::command]
pub fn detail_preview(id: String) -> CommandResult<DisplayList> {
    studio_views::drafting::detail_preview(&id)
        .ok_or_else(|| anyhow::anyhow!("that detail isn't in the library").into())
}

/// Inserts a detail as a new drafting view; returns its id with the new state.
#[tauri::command]
pub fn detail_insert(
    id: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<(ElementId, Option<AppState>)> {
    let mut session = lock(&state)?;
    let view = session.edit(|d| details::insert(d, &id))?;
    Ok((view, finish(&window, &session)?))
}

/// View > Drafting View: a new empty drafting view; returns its id with the new state.
#[tauri::command]
pub fn create_drafting_view(
    name: String,
    scale: u32,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<(ElementId, Option<AppState>)> {
    let mut session = lock(&state)?;
    let view = session.edit(|d| details::create_drafting_view(d, &name, scale))?;
    Ok((view, finish(&window, &session)?))
}
