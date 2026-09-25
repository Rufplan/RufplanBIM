//! Commands behind Revit's everyday shortcuts (ADR-024): pin/unpin, select all instances,
//! tag, and hide in view. Thin wrappers over `studio_core::visibility`.

use studio_core::{visibility, Category, ElementId};
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

/// PN / UP.
#[tauri::command]
pub fn set_pinned(
    ids: Vec<ElementId>,
    pinned: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| visibility::set_pinned(d, &ids, pinned))?;
        Ok(())
    })
}

/// SA: every instance of the selected element's type.
#[tauri::command]
pub fn select_all_instances(
    id: ElementId,
    state: State<'_, SessionState>,
) -> Result<Vec<ElementId>, CommandError> {
    let s = lock(&state)?;
    Ok(visibility::all_instances(s.doc()?, id)?)
}

/// TG / RT: tags one element in a floor plan.
#[tauri::command]
pub fn tag_element(
    view: ElementId,
    target: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| visibility::tag_element(d, view, target))?;
        Ok(())
    })
}

/// EH: Hide in View > Elements.
#[tauri::command]
pub fn hide_elements(
    view: ElementId,
    ids: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| visibility::hide_elements(d, view, &ids))?;
        Ok(())
    })
}

/// VH (hide) and Visibility/Graphics (VV) check boxes.
#[tauri::command]
pub fn set_category_visible(
    view: ElementId,
    categories: Vec<Category>,
    visible: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| visibility::set_category_visible(d, view, &categories, visible))?;
        Ok(())
    })
}

#[tauri::command]
pub fn unhide_all(
    view: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| visibility::unhide_all(d, view))?;
        Ok(())
    })
}

/// Each element shown in `view` with its category.
#[tauri::command]
pub fn view_categories(
    view: ElementId,
    state: State<'_, SessionState>,
) -> Result<Vec<(ElementId, Category)>, CommandError> {
    let s = lock(&state)?;
    Ok(studio_views::view_categories(s.doc()?, view))
}
