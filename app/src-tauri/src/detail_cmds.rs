//! The Details tab (ADR-069): drafting views and the typical detail library. Thin wrappers
//! over studio-core `details` and studio-views `drafting`.

use studio_core::details::user::{self, SavedDetail};
use studio_core::details::{self, DetailInfo};
use studio_core::ElementId;
use studio_geom::Pt;
use studio_views::DisplayList;
use tauri::Manager;
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

// Your details (ADR-073) are kept on this computer, in one JSON file.

fn library_file(app: &tauri::AppHandle) -> anyhow::Result<std::path::PathBuf> {
    Ok(app
        .path()
        .app_local_data_dir()?
        .join("details")
        .join("my-details.json"))
}

pub(crate) fn read_saved(path: &std::path::Path) -> Vec<SavedDetail> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn write_saved(path: &std::path::Path, list: &[SavedDetail]) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string(list)?)?;
    Ok(())
}

fn info(d: &SavedDetail) -> DetailInfo {
    DetailInfo {
        id: format!("user:{}", d.id),
        name: d.name.clone(),
        category: d.category.clone(),
        scale: d.scale,
        scale_label: studio_core::ops::scale_label(d.scale),
        description: d.description.clone(),
        user: true,
    }
}

fn saved(app: &tauri::AppHandle, id: &str) -> anyhow::Result<SavedDetail> {
    let key = id.strip_prefix("user:").unwrap_or(id);
    read_saved(&library_file(app)?)
        .into_iter()
        .find(|d| d.id == key)
        .ok_or_else(|| anyhow::anyhow!("that detail isn't in your library any more"))
}

/// A saved detail drawn as it would be inserted, from a scratch project.
pub(crate) fn saved_preview(d: &SavedDetail) -> anyhow::Result<DisplayList> {
    let mut doc = studio_core::Document::new();
    let view = user::insert_saved(&mut doc, d)?;
    studio_views::display_list(&doc, view).ok_or_else(|| anyhow::anyhow!("nothing to draw"))
}

/// The library: the built-in details, then yours.
#[tauri::command]
pub fn detail_library(app: tauri::AppHandle) -> Vec<DetailInfo> {
    let mut out = details::catalog();
    if let Ok(path) = library_file(&app) {
        out.extend(read_saved(&path).iter().map(info));
    }
    out
}

/// A library detail drawn as its drafting view will be (for its thumbnail).
#[tauri::command]
pub fn detail_preview(id: String, app: tauri::AppHandle) -> CommandResult<DisplayList> {
    if id.starts_with("user:") {
        return Ok(saved_preview(&saved(&app, &id)?)?);
    }
    studio_views::drafting::detail_preview(&id)
        .ok_or_else(|| anyhow::anyhow!("that detail isn't in the library").into())
}

/// Inserts a detail as a new drafting view; returns its id with the new state.
#[tauri::command]
pub fn detail_insert(
    id: String,
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<(ElementId, Option<AppState>)> {
    let mut session = lock(&state)?;
    let view = if id.starts_with("user:") {
        let d = saved(&app, &id)?;
        session.edit(|doc| user::insert_saved(doc, &d))?
    } else {
        session.edit(|d| details::insert(d, &id))?
    };
    Ok((view, finish(&window, &session)?))
}

/// Save to Library: a drafting view as one of your details (replacing one of the same name
/// and category).
#[tauri::command]
pub fn detail_save(
    view: ElementId,
    name: String,
    category: String,
    description: String,
    app: tauri::AppHandle,
    state: State<'_, SessionState>,
) -> CommandResult<DetailInfo> {
    let session = lock(&state)?;
    let id = studio_core::ElementId::new().to_string();
    let d = user::capture(session.doc()?, view, &id, &name, &category, &description)?;
    let path = library_file(&app)?;
    let mut list = read_saved(&path);
    list.retain(|x| !(x.name == d.name && x.category == d.category));
    list.push(d.clone());
    write_saved(&path, &list)?;
    Ok(info(&d))
}

/// Removes one of your details from the library.
#[tauri::command]
pub fn detail_delete(id: String, app: tauri::AppHandle) -> CommandResult<()> {
    let key = id.strip_prefix("user:").unwrap_or(&id).to_owned();
    let path = library_file(&app)?;
    let mut list = read_saved(&path);
    let before = list.len();
    list.retain(|d| d.id != key);
    if list.len() == before {
        return Err(anyhow::anyhow!("only details you saved can be deleted").into());
    }
    write_saved(&path, &list)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn your_library_file_keeps_details_and_draws_their_previews() {
        let dir = std::env::temp_dir().join(format!("rufplan-details-{}", ElementId::new()));
        let path = dir.join("my-details.json");
        assert!(read_saved(&path).is_empty());
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let v = details::insert(&mut doc, "eave").unwrap();
        let d = user::capture(&doc, v, "abc", "Eave Mod", "Roofs", "").unwrap();
        write_saved(&path, std::slice::from_ref(&d)).unwrap();
        let back = read_saved(&path);
        assert_eq!(back.len(), 1);
        assert_eq!(info(&back[0]).id, "user:abc");
        assert!(info(&back[0]).user);
        let dl = saved_preview(&back[0]).unwrap();
        assert!(dl.items.len() > 50);
        let _ = std::fs::remove_dir_all(dir);
    }
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

// Views and sheets in the project browser (ADR-074).

/// View > Plan Views: a new floor plan (or reflected ceiling plan) of `level`.
#[tauri::command]
pub fn create_plan_view(
    level: ElementId,
    ceiling: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<(ElementId, Option<AppState>)> {
    let mut session = lock(&state)?;
    let view = session.edit(|d| studio_core::views::create_plan(d, level, ceiling))?;
    Ok((view, finish(&window, &session)?))
}

/// View > 3D View: a new 3D view.
#[tauri::command]
pub fn create_3d_view(
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<(ElementId, Option<AppState>)> {
    let mut session = lock(&state)?;
    let view = session.edit(studio_core::views::create_3d)?;
    Ok((view, finish(&window, &session)?))
}

/// Duplicate View (with or without its detailing).
#[tauri::command]
pub fn duplicate_view(
    view: ElementId,
    detailing: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<(ElementId, Option<AppState>)> {
    let mut session = lock(&state)?;
    let copy = session.edit(|d| studio_core::views::duplicate_view(d, view, detailing))?;
    Ok((copy, finish(&window, &session)?))
}

/// Duplicate Sheet: empty, with detailing, or with its views.
#[tauri::command]
pub fn duplicate_sheet(
    sheet: ElementId,
    how: studio_core::views::SheetCopy,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<(ElementId, Option<AppState>)> {
    let mut session = lock(&state)?;
    let copy = session.edit(|d| studio_core::views::duplicate_sheet(d, sheet, how))?;
    Ok((copy, finish(&window, &session)?))
}
