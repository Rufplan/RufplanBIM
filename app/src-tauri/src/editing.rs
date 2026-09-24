//! IPC commands for the modify tools, grips and temporary dimensions, roofs and stairs,
//! and project parameters (ADR-017, ADR-018). Thin wrappers over studio-core and
//! studio-views.

use studio_core::edit::{self, Xform};
use studio_core::{build, ops, params, Category, ElementData, ElementId, ParamKind, ParamScope};
use studio_geom::Pt;
use studio_views::{Handles, OffsetPreview, RefLine};
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;
type StateResult = CommandResult<Option<AppState>>;

fn edit_state(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&mut crate::session::Session) -> anyhow::Result<()>,
) -> StateResult {
    let mut session = lock(state)?;
    f(&mut session)?;
    finish(window, &session)
}

fn parse_len(text: &str) -> anyhow::Result<f64> {
    studio_core::units::parse_length(text)
        .ok_or_else(|| anyhow::anyhow!("\"{text}\" is not a length (try 10'-6\")"))
}

/// A typed length in mm (what the user types while drawing), or None if it isn't one.
#[tauri::command]
pub fn parse_length(text: String) -> Option<f64> {
    studio_core::units::parse_length(&text)
}

/// Grips and temporary dimensions for the selection in `view`.
#[tauri::command]
pub fn handles(
    view: ElementId,
    ids: Vec<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<Handles> {
    let session = lock(&state)?;
    Ok(studio_views::handles(session.doc()?, view, &ids))
}

/// Drags a grip (see `handles`) to `to`.
#[tauri::command]
pub fn drag_handle(
    id: ElementId,
    key: String,
    to: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| edit::drag_handle(d, id, &key, to))
    })
}

/// Types a new value into a temporary dimension.
#[tauri::command]
pub fn set_temp_dimension(
    id: ElementId,
    key: String,
    value: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mm = parse_len(&value)?;
    edit_state(&window, &state, |s| {
        s.edit(|d| match key.as_str() {
            "length" => edit::set_wall_length(d, id, mm),
            "gap_start" => edit::set_opening_gap(d, id, true, mm),
            "gap_end" => edit::set_opening_gap(d, id, false, mm),
            _ => Err(studio_core::CoreError::Invalid(format!(
                "unknown dimension {key}"
            ))),
        })
    })
}

/// Copy (count 1) or linear Array (count ≥ 2 copies, each `delta` further on).
#[tauri::command]
pub fn copy_elements(
    ids: Vec<ElementId>,
    delta: Pt,
    count: u32,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let n = count.clamp(1, 200);
    let xs: Vec<Xform> = (1..=n)
        .map(|k| Xform::translate(delta.scale(f64::from(k))))
        .collect();
    let name = if n > 1 { "Array" } else { "Copy" };
    edit_state(&window, &state, |s| {
        s.edit(|d| edit::copy_elements(d, &ids, &xs, name))?;
        Ok(())
    })
}

/// Rotates the selection about `center` by `angle` radians (counter-clockwise), or a copy.
#[tauri::command]
pub fn rotate_elements(
    ids: Vec<ElementId>,
    center: Pt,
    angle: f64,
    copy: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let x = Xform::rotate(center, angle);
    edit_state(&window, &state, |s| {
        if copy {
            s.edit(|d| edit::copy_elements(d, &ids, &[x], "Rotate"))?;
            Ok(())
        } else {
            s.edit(|d| edit::transform_elements(d, &ids, x, "Rotate"))
        }
    })
}

/// Mirrors the selection across the axis a→b (a copy, unless `copy` is false).
#[tauri::command]
pub fn mirror_elements(
    ids: Vec<ElementId>,
    a: Pt,
    b: Pt,
    copy: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    if a.dist(b) < 1.0 {
        return Err(anyhow::anyhow!("draw a longer mirror axis").into());
    }
    let x = Xform::mirror(a, b);
    edit_state(&window, &state, |s| {
        if copy {
            s.edit(|d| edit::copy_elements(d, &ids, &[x], "Mirror"))?;
            Ok(())
        } else {
            s.edit(|d| edit::transform_elements(d, &ids, x, "Mirror"))
        }
    })
}

/// Trim/Extend to Corner between the walls clicked at `a_pick` and `b_pick`.
#[tauri::command]
pub fn trim_extend(
    a: ElementId,
    a_pick: Pt,
    b: ElementId,
    b_pick: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| edit::trim_extend(d, a, a_pick, b, b_pick))
    })
}

#[tauri::command]
pub fn offset_preview(
    view: ElementId,
    point: Pt,
    tol: f64,
    distance: String,
    state: State<'_, SessionState>,
) -> CommandResult<Option<OffsetPreview>> {
    let session = lock(&state)?;
    let Some(mm) = studio_core::units::parse_length(&distance) else {
        return Ok(None);
    };
    Ok(studio_views::offset_preview(
        session.doc()?,
        view,
        point,
        tol,
        mm,
    ))
}

/// Offset: a copy of the wall or grid under `point`, `distance` away on the cursor's side.
#[tauri::command]
pub fn offset_element(
    view: ElementId,
    point: Pt,
    tol: f64,
    distance: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mm = parse_len(&distance)?;
    edit_state(&window, &state, |s| {
        let target = studio_views::offset_preview(s.doc()?, view, point, tol, mm)
            .ok_or_else(|| anyhow::anyhow!("click next to a wall or grid"))?;
        s.edit(|d| edit::offset_element(d, target.id, mm, point))?;
        Ok(())
    })
}

#[tauri::command]
pub fn split_wall(
    id: ElementId,
    at: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| edit::split_wall(d, id, at))?;
        Ok(())
    })
}

/// Flips walls end for end and doors/windows to face the other way (spacebar).
#[tauri::command]
pub fn flip_selection(
    ids: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let doc = s.doc()?;
        let walls: Vec<ElementId> = ids
            .iter()
            .copied()
            .filter(|id| doc.data(*id).is_ok_and(|d| d.category() == Category::Wall))
            .collect();
        let openings: Vec<(ElementId, bool)> =
            ids.iter()
                .filter_map(|id| match doc.data(*id).ok()? {
                    ElementData::Door { flip_facing, .. }
                    | ElementData::Window { flip_facing, .. } => Some((*id, *flip_facing)),
                    _ => None,
                })
                .collect();
        if walls.is_empty() && openings.is_empty() {
            anyhow::bail!("select walls, doors or windows to flip");
        }
        if !walls.is_empty() {
            s.edit(|d| edit::flip_walls(d, &walls))?;
        }
        for (id, facing) in openings {
            s.edit(|d| {
                ops::set_property(d, id, "flip_facing", if facing { "no" } else { "yes" }, 0)
            })?;
        }
        Ok(())
    })
}

/// The reference line (wall face, centerline or grid) Align would use at `point`.
#[tauri::command]
pub fn ref_line(
    view: ElementId,
    point: Pt,
    tol: f64,
    skip: Option<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<Option<RefLine>> {
    let session = lock(&state)?;
    Ok(studio_views::ref_line(
        session.doc()?,
        view,
        point,
        tol,
        skip,
    ))
}

/// Align: moves the element whose line is at `target` onto the reference line at `reference`.
#[tauri::command]
pub fn align(
    view: ElementId,
    reference: Pt,
    target: Pt,
    tol: f64,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let doc = s.doc()?;
        let r = studio_views::ref_line(doc, view, reference, tol, None)
            .ok_or_else(|| anyhow::anyhow!("pick a wall face, centerline or grid first"))?;
        let t = studio_views::ref_line(doc, view, target, tol, Some(r.el))
            .ok_or_else(|| anyhow::anyhow!("pick a line on the element to align"))?;
        let delta = studio_views::align_delta(&r, &t)
            .ok_or_else(|| anyhow::anyhow!("those lines aren't parallel"))?;
        s.edit(|d| studio_core::modify::move_elements(d, &[t.el], delta))
    })
}

/// Roof by footprint from the walls of the view's level: an 18" overhang at 6:12, bearing
/// on the tops of the walls (on the level at that height, if there is one). Right-angled
/// L, T and U plans get a hipped roof per wing.
#[tauri::command]
pub fn create_roof(
    view: ElementId,
    type_id: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        let doc = s.doc()?;
        let model = studio_regen::regenerate(doc);
        let outer = studio_regen::outer_boundary(&model, level)
            .ok_or_else(|| anyhow::anyhow!("draw walls on this level first"))?;
        let top = model
            .walls
            .iter()
            .filter(|w| w.level == level)
            .map(|w| w.z1)
            .fold(f64::NEG_INFINITY, f64::max);
        // Bear on a level at the wall tops if one exists, else on this level.
        let (roof_level, base) = doc
            .levels()
            .into_iter()
            .find(|l| (l.2 - top).abs() < 1.0)
            .map_or_else(
                || (level, top - doc.level_elevation(level).unwrap_or(0.0)),
                |l| (l.0, 0.0),
            );
        let rt = type_id
            .or_else(|| build::default_roof_type(doc))
            .ok_or_else(|| anyhow::anyhow!("no roof types in this project"))?;
        let (overhang, slope) = (build::DEFAULT_OVERHANG, build::DEFAULT_ROOF_SLOPE);
        // Convex plans get one roof; L, T and U plans one per wing, meeting in valleys.
        s.edit(|d| {
            build::create_roofs_by_footprint(d, rt, roof_level, base, &outer, overhang, slope)
        })?;
        Ok(())
    })
}

/// A stair from the view's level up to the next, starting at `start` toward `toward`,
/// straight or L/U-shaped (`shape` is an id from `build::STAIR_SHAPES`).
#[tauri::command]
pub fn create_stair(
    view: ElementId,
    start: Pt,
    toward: Pt,
    shape: Option<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let shape = match shape.as_deref() {
        None => studio_core::StairShape::Straight,
        Some(id) => build::parse_stair_shape(id)
            .ok_or_else(|| anyhow::anyhow!("unknown stair shape {id}"))?,
    };
    edit_state(&window, &state, |s| {
        let level = s.view_level(view)?;
        s.edit(|d| {
            build::create_stair_shaped(d, level, start, toward, build::DEFAULT_STAIR_WIDTH, shape)
        })?;
        Ok(())
    })
}

/// Adds a project parameter for the given categories (Revit's Project Parameters).
#[tauri::command]
pub fn add_project_parameter(
    label: String,
    kind: String,
    type_scope: bool,
    categories: Vec<Category>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let kind =
        ParamKind::parse(&kind).ok_or_else(|| anyhow::anyhow!("unknown parameter type {kind}"))?;
    let scope = if type_scope {
        ParamScope::Type
    } else {
        ParamScope::Instance
    };
    edit_state(&window, &state, |s| {
        s.edit(|d| params::add_def(d, &label, kind, scope, categories))?;
        Ok(())
    })
}

#[tauri::command]
pub fn remove_project_parameter(
    key: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| s.edit(|d| params::remove_def(d, &key)))
}
