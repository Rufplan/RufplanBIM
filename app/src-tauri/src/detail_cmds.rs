//! The Details tab (ADR-069): drafting views and the typical detail library. Thin wrappers
//! over studio-core `details` and studio-views `drafting`.

use studio_core::details::{self, DetailInfo};
use studio_core::ElementId;
use studio_geom::Pt;
use studio_views::DisplayList;
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

/// Detail Component (ADR-071): the types, as the type selector lists them.
#[tauri::command]
pub fn detail_component_types() -> Vec<studio_core::details::components::ComponentTypeInfo> {
    studio_core::details::components::catalog()
}

/// What a component of `key` would draw from `start` to `end`, as polylines (the
/// placement preview).
#[tauri::command]
pub fn detail_component_preview(key: String, start: Pt, end: Pt, flip: bool) -> Vec<Vec<Pt>> {
    use studio_core::details::components::{parts, type_of};
    type_of(&key)
        .map(|t| {
            let p = parts(t, start, end, flip);
            p.lines
                .into_iter()
                .map(|l| {
                    let mut pts = l.pts;
                    if l.closed {
                        if let Some(f) = pts.first().copied() {
                            pts.push(f);
                        }
                    }
                    pts
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn create_detail_component(
    view: ElementId,
    key: String,
    start: Pt,
    end: Pt,
    flip: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<Option<AppState>, CommandError> {
    let mut session = lock(&state)?;
    session.edit(|d| details::create_component(d, view, &key, start, end, flip))?;
    finish(&window, &session)
}

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
