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

/// A spot slope at `at` (ADR-049), if there is a sloped roof, floor or ground there.
#[tauri::command]
pub fn create_spot_slope(
    view: ElementId,
    at: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| {
            let scale = match d.data(view)? {
                studio_core::ElementData::View { scale, .. } => f64::from(*scale),
                _ => 1.0,
            };
            let model = studio_regen::regenerate(d);
            // Within 3 mm on paper of a sloped edge.
            if studio_views::slopes::slope_at(d, &model, view, at, 3.0 * scale).is_none() {
                return Err(studio_core::CoreError::Invalid(
                    "Click a sloped roof, floor or the ground (in elevations and sections, on a sloped edge).".into(),
                ));
            }
            studio_core::slope::create_spot_slope(d, view, at)
        })
    })
}
