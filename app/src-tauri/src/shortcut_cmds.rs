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

/// The selected elements' categories, for the ribbon's contextual Modify tab. Elements that
/// no longer exist are left out.
#[tauri::command]
pub fn selection_categories(
    ids: Vec<ElementId>,
    state: State<'_, SessionState>,
) -> Result<Vec<Category>, CommandError> {
    let s = lock(&state)?;
    let doc = s.doc()?;
    let mut cats: Vec<Category> = vec![];
    for id in ids {
        if let Ok(d) = doc.data(id) {
            if !cats.contains(&d.category()) {
                cats.push(d.category());
            }
        }
    }
    Ok(cats)
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

/// Tag on the contextual Modify tab: the selection's untagged elements, one undo.
#[tauri::command]
pub fn tag_elements(
    view: ElementId,
    targets: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| visibility::tag_elements(d, view, &targets))?;
        Ok(())
    })
}

/// Tag in a section or elevation (ADR-060): the room under the click, its tag placed there.
#[tauri::command]
pub fn tag_room_in_view(
    view: ElementId,
    at: studio_geom::Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let doc = s.doc()?;
        let room = studio_views::view_refs::room_in_view(doc, view, at)
            .ok_or_else(|| anyhow::anyhow!("click inside a room (at its level's height)"))?;
        let base = studio_views::view_refs::room_tag_base(doc, view, room)
            .ok_or_else(|| anyhow::anyhow!("tag rooms in a section or elevation"))?;
        s.edit(|d| visibility::tag_room_in_view(d, view, room, at.sub(base)))?;
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
