//! IPC commands for schedules on sheets (ADR-110): a schedule's appearance, splitting it
//! into parts, and the placeholder sheets the sheet index lists. Thin wrappers over
//! studio-sheets `schedule`.

use studio_core::text::TextFont;
use studio_core::ElementId;
use studio_sheets::schedule::{self, PlaceholderSheet, ScheduleStyle};
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

/// A schedule's style (its own, else the office's text types).
#[tauri::command]
pub fn schedule_style(
    view: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<ScheduleStyle> {
    let s = lock(&state)?;
    Ok(schedule::style_of(s.doc()?, view))
}

#[tauri::command]
pub fn set_schedule_style(
    view: ElementId,
    style: ScheduleStyle,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| schedule::set_style(d, view, style))?;
    finish(&window, &s)
}

/// The fonts schedules can print in, with their names.
#[tauri::command]
pub fn text_fonts() -> Vec<(TextFont, String)> {
    TextFont::ALL
        .iter()
        .map(|f| (*f, f.label().to_string()))
        .collect()
}

/// How many parts a schedule on a sheet is split into.
#[tauri::command]
pub fn schedule_parts(viewport: ElementId, state: State<'_, SessionState>) -> CommandResult<usize> {
    let s = lock(&state)?;
    Ok(schedule::split_of(s.doc()?, viewport).breaks.len() + 1)
}

/// Split Schedule Table: halves its longest part.
#[tauri::command]
pub fn split_schedule(
    viewport: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| schedule::split_schedule(d, viewport))?;
    finish(&window, &s)
}

/// Joins a split schedule back into one.
#[tauri::command]
pub fn join_schedule(
    viewport: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| schedule::join_schedule(d, viewport))?;
    finish(&window, &s)
}

/// The placeholder sheets the sheet index lists.
#[tauri::command]
pub fn placeholder_sheets(state: State<'_, SessionState>) -> CommandResult<Vec<PlaceholderSheet>> {
    let s = lock(&state)?;
    Ok(schedule::placeholders(s.doc()?))
}

#[tauri::command]
pub fn set_placeholder_sheets(
    sheets: Vec<PlaceholderSheet>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    s.edit(|d| schedule::set_placeholders(d, sheets))?;
    finish(&window, &s)
}
