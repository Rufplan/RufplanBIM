//! IPC for annotation symbols (ADR-048): thin wrappers over studio-core's `symbols`.

use studio_core::{symbols, ElementId};
use studio_geom::Pt;
use tauri::{State, WebviewWindow};

use crate::commands::{edit, SessionState, StateResult};

/// A spot elevation at `at`, its text at `leader` (the same point: no leader).
#[tauri::command]
pub fn create_spot_elevation(
    view: ElementId,
    at: Pt,
    leader: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| symbols::create_spot_elevation(d, view, at, leader))
    })
}

/// A north arrow in a plan or on a sheet.
#[tauri::command]
pub fn create_north_arrow(
    view: ElementId,
    at: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| symbols::create_north_arrow(d, view, at))
    })
}

/// A graphic scale in a plan, elevation or section.
#[tauri::command]
pub fn create_graphic_scale(
    view: ElementId,
    at: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| symbols::create_graphic_scale(d, view, at))
    })
}

/// A key plan on a sheet.
#[tauri::command]
pub fn create_key_plan(
    sheet: ElementId,
    at: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| symbols::create_key_plan(d, sheet, at))
    })
}
