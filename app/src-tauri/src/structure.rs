//! IPC commands for columns, beams, railings, attached walls and shaped stairs (ADR-019).
//! Thin wrappers over studio-core.

use studio_core::{build, structure, Category, ElementId, LocationLine};
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

fn type_or_default(
    s: &crate::session::Session,
    type_id: Option<ElementId>,
    cat: Category,
) -> anyhow::Result<ElementId> {
    type_id
        .or_else(|| s.doc().ok().and_then(|d| structure::default_type(d, cat)))
        .ok_or_else(|| anyhow::anyhow!("no {} types in this project", cat.as_str()))
}

#[tauri::command]
pub fn create_column(
    view: ElementId,
    type_id: Option<ElementId>,
    at: Pt,
    rotation: Option<f64>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        let t = type_or_default(s, type_id, Category::ColumnType)?;
        s.edit(|d| structure::create_column(d, t, level, at, rotation.unwrap_or(0.0)))?;
        Ok(())
    })
}

/// Columns at every grid intersection on the view's level that doesn't have one yet.
#[tauri::command]
pub fn columns_at_grids(
    view: ElementId,
    type_id: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        let t = type_or_default(s, type_id, Category::ColumnType)?;
        let made = s.edit(|d| structure::columns_at_grids(d, t, level))?;
        if made.is_empty() {
            anyhow::bail!("no free grid intersections on this level");
        }
        Ok(())
    })
}

#[tauri::command]
pub fn create_beam(
    view: ElementId,
    type_id: Option<ElementId>,
    start: Pt,
    end: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        // A beam drawn in a plan frames the floor above it (Revit's structural plan
        // convention); in the top level's plan it sits at that level.
        let level = s.view_level(view)?;
        let doc = s.doc()?;
        let at = build::level_above(doc, level).unwrap_or(level);
        let t = type_or_default(s, type_id, Category::BeamType)?;
        s.edit(|d| structure::create_beam(d, t, at, start, end))?;
        Ok(())
    })
}

#[tauri::command]
pub fn create_railing(
    view: ElementId,
    type_id: Option<ElementId>,
    path: Vec<Pt>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        let t = type_or_default(s, type_id, Category::RailingType)?;
        s.edit(|d| structure::create_railing(d, t, level, path))?;
        Ok(())
    })
}

/// Attaches the selected walls' tops to the roofs above (or detaches them).
#[tauri::command]
pub fn attach_wall_tops(
    ids: Vec<ElementId>,
    attach: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let n = s.edit(|d| structure::attach_wall_tops(d, &ids, attach))?;
        if n == 0 {
            anyhow::bail!(
                "select walls to {}",
                if attach { "attach" } else { "detach" }
            );
        }
        Ok(())
    })
}

/// A wall drawn along its location line (Revit's Location Line option).
#[tauri::command]
pub fn create_wall_located(
    view: ElementId,
    type_id: ElementId,
    start: Pt,
    end: Pt,
    location: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let loc = LocationLine::parse(&location)
        .ok_or_else(|| anyhow::anyhow!("unknown location line {location}"))?;
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        s.edit(|d| studio_core::ops::create_wall_located(d, type_id, level, start, end, loc))?;
        Ok(())
    })
}

/// (id, label) pairs for a choice in the options bar.
type Choices = Vec<(String, String)>;

/// Location line and stair shape choices for the options bar, as (id, label).
#[tauri::command]
pub fn drawing_options() -> (Choices, Choices) {
    (
        LocationLine::ALL
            .iter()
            .map(|l| (format!("{l:?}"), l.label().to_owned()))
            .collect(),
        build::STAIR_SHAPES
            .iter()
            .map(|(id, label)| ((*id).to_owned(), (*label).to_owned()))
            .collect(),
    )
}
