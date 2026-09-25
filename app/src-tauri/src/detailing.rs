//! IPC commands for room separators, callouts, the 3D section box and materials
//! (ADR-020). Thin wrappers over studio-core and studio-regen.

use studio_core::{detail, material, ElementId, SectionBox};
use studio_geom::Pt;
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type StateResult = Result<Option<AppState>, CommandError>;

fn edit_state(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&mut crate::session::Session) -> anyhow::Result<()>,
) -> StateResult {
    let mut session = lock(state)?;
    f(&mut session)?;
    finish(window, &session)
}

#[tauri::command]
pub fn create_room_separator(
    view: ElementId,
    start: Pt,
    end: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        s.edit(|d| detail::create_room_separator(d, level, start, end))?;
        Ok(())
    })
}

/// An elevation marker at `at` (interior, or a building elevation), looking at the nearest
/// wall.
#[tauri::command]
pub fn create_elevation_marker(
    view: ElementId,
    at: Pt,
    interior: bool,
    type_id: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        // The type decides interior or building (ADR-022).
        let interior = match type_id.and_then(|t| s.doc().ok()?.data(t).ok()) {
            Some(studio_core::ElementData::ElevationMarkerType { interior, .. }) => *interior,
            _ => interior,
        };
        let m =
            s.edit(|d| studio_regen::derived::create_elevation_marker(d, level, at, interior))?;
        if let Some(t) = type_id {
            s.edit(|d| studio_core::ops::set_property(d, m, "type", &t.to_string(), 0))?;
        }
        Ok(())
    })
}

/// A door or window placed on `host` from the 3D view: where it would go for a hit at `p`.
#[tauri::command]
pub fn opening_preview_3d(
    type_id: ElementId,
    host: ElementId,
    p: Pt,
    state: State<'_, SessionState>,
) -> Result<Option<studio_views::OpeningPreview3d>, CommandError> {
    let session = lock(&state)?;
    Ok(studio_views::opening_preview_3d(
        session.doc()?,
        type_id,
        host,
        p,
    ))
}

/// A callout of `view` between corners `a` and `b` (view coordinates).
#[tauri::command]
pub fn create_callout(
    view: ElementId,
    a: Pt,
    b: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| detail::create_callout(d, view, a, b))?;
        Ok(())
    })
}

/// Moves the faces of a 3D view's section box (from dragging its handles).
#[tauri::command]
pub fn set_section_box(
    view: ElementId,
    min: [f64; 3],
    max: [f64; 3],
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| studio_regen::derived::set_section_box(d, view, Some(SectionBox { min, max })))?;
        Ok(())
    })
}

/// A new material, copied from `from` when given.
#[tauri::command]
pub fn create_material(
    from: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| material::create_material(d, from))?;
        Ok(())
    })
}
