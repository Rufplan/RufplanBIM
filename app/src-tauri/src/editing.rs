//! IPC commands for the modify tools, grips and temporary dimensions, roofs and stairs,
//! and project parameters (ADR-017, ADR-018). Thin wrappers over studio-core and
//! studio-views.

use studio_core::edit::{self, Xform};
use studio_core::{build, params, Category, ElementData, ElementId, ParamKind, ParamScope};
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
    let doc = session.doc()?;
    // On a sheet: the ends of selected viewports' title rules (ADR-039).
    if matches!(doc.data(view), Ok(studio_core::ElementData::Sheet { .. })) {
        return Ok(studio_sheets::sheet_handles(doc, view, &ids));
    }
    Ok(studio_views::handles(doc, view, &ids))
}

/// A viewport's sheet, view and center (paper mm), for activating it (ADR-039).
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct ViewportInfo {
    pub sheet: ElementId,
    pub view: ElementId,
    pub center: Pt,
}

#[tauri::command]
pub fn viewport_info(
    id: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<Option<ViewportInfo>> {
    let session = lock(&state)?;
    Ok(match session.doc()?.data(id) {
        Ok(studio_core::ElementData::Viewport {
            sheet,
            view,
            center,
            ..
        }) => Some(ViewportInfo {
            sheet: *sheet,
            view: *view,
            center: *center,
        }),
        _ => None,
    })
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
        s.edit(|d| {
            if key == "title_end" {
                studio_sheets::drag_title(d, id, to)
            } else if key == "title_start" {
                studio_sheets::drag_title_start(d, id, to)
            } else if key == "title_move" {
                studio_sheets::move_title(d, id, to)
            } else {
                edit::drag_handle(d, id, &key, to)
            }
        })
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
            // To a parallel wall or grid, or a permanent dimension's segment (ADR-041).
            k if k.starts_with("to:") => match k[3..].parse() {
                Ok(other) => studio_core::dimension::set_distance_to(d, id, other, mm),
                Err(_) => Err(studio_core::CoreError::Invalid(format!(
                    "unknown dimension {k}"
                ))),
            },
            k if k.starts_with("dim:") => {
                let mut parts = k[4..].split(':');
                match (
                    parts.next().and_then(|s| s.parse().ok()),
                    parts.next().and_then(|s| s.parse().ok()),
                ) {
                    (Some(dim), Some(seg)) => {
                        studio_core::dimension::set_dimension_segment(d, id, dim, seg, mm)
                    }
                    _ => Err(studio_core::CoreError::Invalid(format!(
                        "unknown dimension {k}"
                    ))),
                }
            }
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
        let openings: Vec<ElementId> = ids
            .iter()
            .copied()
            .filter(|id| {
                matches!(
                    doc.data(*id),
                    Ok(ElementData::Door { .. } | ElementData::Window { .. })
                )
            })
            .collect();
        if walls.is_empty() && openings.is_empty() {
            anyhow::bail!("select walls, doors or windows to flip");
        }
        s.edit(|d| {
            let mark = d.undo_depth();
            if !walls.is_empty() {
                edit::flip_walls(d, &walls)?;
            }
            // Doors turn through their four swings, windows flip their facing (Revit's).
            if !openings.is_empty() {
                edit::flip_openings(d, &openings, edit::OpeningFlip::Cycle)?;
            }
            d.merge_undo(mark, "Flip");
            Ok(())
        })?;
        Ok(())
    })
}

/// The 20 typical fascias (ADR-095).
#[tauri::command]
pub fn fascia_catalog() -> Vec<studio_core::fascia::FasciaSpec> {
    studio_core::fascia::catalog()
}

/// Gives roofs a fascia by name, or removes it; every roof when `ids` is None.
#[tauri::command]
pub fn set_fascia(
    ids: Option<Vec<ElementId>>,
    name: Option<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let roofs = match ids {
            Some(v) => v,
            None => s.doc()?.of(Category::Roof).map(|e| e.id).collect(),
        };
        if roofs.is_empty() {
            anyhow::bail!("there are no roofs to give a fascia");
        }
        s.edit(|d| studio_core::fascia::set(d, &roofs, name.as_deref()))?;
        Ok(())
    })
}

/// Revit's flip controls: the left/right arrows flip a door's hand, the up/down arrows
/// its facing.
#[tauri::command]
pub fn flip_opening(
    id: ElementId,
    flip: edit::OpeningFlip,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| edit::flip_openings(d, &[id], flip))?;
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

/// Align (AL) by picked references (ADR-042): moves the element `target` is on (any model
/// element, ADR-098) onto `reference`. In an elevation or section (`view`), the move is
/// along the view and up or down.
#[tauri::command]
pub fn align_references(
    view: Option<ElementId>,
    reference: studio_core::dimension::Reference,
    target: studio_core::dimension::Reference,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        let (id, d) = studio_core::dimension::align_target(&reference, &target)?;
        let frame = view.and_then(|v| studio_views::view_frame(s.doc().ok()?, v));
        let delta = match frame {
            Some((_, right, _)) => [right.x * d.x, right.y * d.x, d.y],
            None => [d.x, d.y, 0.0],
        };
        s.edit(|doc| studio_core::modify::align_3d(doc, id, delta))?;
        Ok(())
    })
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

/// Revit's Save to Project (ADR-095): keeps a rendered image (base64 JPEG or PNG) as a
/// Rendering view that can go on a sheet.
#[tauri::command]
pub fn save_rendering(
    name: String,
    mime: String,
    data: String,
    width: u32,
    height: u32,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| studio_core::renderings::save(d, &name, &mime, data, width, height))?;
        Ok(())
    })
}

/// A saved rendering's image as a data URL, for drawing it in views and on sheets.
#[tauri::command]
pub fn render_image(id: ElementId, state: State<'_, SessionState>) -> CommandResult<String> {
    let session = lock(&state)?;
    match session.doc()?.data(id) {
        Ok(ElementData::RenderImage { mime, data, .. }) => Ok(format!("data:{mime};base64,{data}")),
        _ => Err(anyhow::anyhow!("that isn't a saved rendering").into()),
    }
}

/// Align in 3D (ADR-097): moves `target` by `delta` (x, y, z mm), the distance between the
/// picked faces along the reference's normal, as one undo step.
#[tauri::command]
pub fn align_3d(
    target: ElementId,
    delta: [f64; 3],
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| studio_core::modify::align_3d(d, target, delta))?;
        Ok(())
    })
}
