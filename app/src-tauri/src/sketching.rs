//! Sketch mode for floor and ceiling boundaries (ADR-021): the sketch being edited lives in
//! the session until Finish. Thin wrappers over `studio_core::sketch`.

use serde::Serialize;
use studio_core::sketch::{self, DrawOptions, DrawTool, SketchCurve, SketchKind};
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::Pt;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::{AppState, Session};

type StateResult = Result<Option<AppState>, CommandError>;
type CommandResult<T> = Result<T, CommandError>;

/// Picked lines join neighbors whose ends are this close to their corner (mm).
const PICK_JOIN: f64 = 600.0;

/// The sketch in progress.
#[derive(Debug, Clone)]
pub struct SketchSession {
    pub kind: SketchKind,
    pub view: ElementId,
    pub level: ElementId,
    /// The floor or ceiling whose boundary is being edited (None: a new one).
    pub target: Option<ElementId>,
    pub type_id: ElementId,
    /// Height of the sketch's work plane in 3D (mm).
    pub elevation: f64,
    pub curves: Vec<SketchCurve>,
    undo: Vec<Vec<SketchCurve>>,
    redo: Vec<Vec<SketchCurve>>,
    /// Curves the last Finish complained about, and why.
    pub bad: Vec<usize>,
    pub error: Option<String>,
}

/// A sketch curve as drawn: its points, and whether it's locked to a wall.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SketchItem {
    pub pts: Vec<Pt>,
    pub locked: bool,
    pub is_line: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SketchInfo {
    pub kind: SketchKind,
    pub view: ElementId,
    pub level: ElementId,
    /// Height of the work plane the sketch is drawn on in 3D (mm).
    pub elevation: f64,
    pub target: Option<ElementId>,
    pub type_id: ElementId,
    pub curves: Vec<SketchItem>,
    pub bad: Vec<usize>,
    pub error: Option<String>,
    pub can_undo: bool,
    pub can_redo: bool,
}

impl SketchSession {
    pub fn info(&self) -> SketchInfo {
        SketchInfo {
            kind: self.kind,
            view: self.view,
            level: self.level,
            elevation: self.elevation,
            target: self.target,
            type_id: self.type_id,
            curves: self
                .curves
                .iter()
                .map(|c| SketchItem {
                    pts: c.points(),
                    locked: matches!(c, SketchCurve::Line { wall: Some(_), .. }),
                    is_line: matches!(c, SketchCurve::Line { .. }),
                })
                .collect(),
            bad: self.bad.clone(),
            error: self.error.clone(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
        }
    }
}

/// Runs an edit of the sketch as one undo step.
fn sketch_edit(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&Document, &mut Vec<SketchCurve>, ElementId) -> anyhow::Result<()>,
) -> StateResult {
    let mut session = lock(state)?;
    let s: &mut Session = &mut session;
    let (doc, sk) = s.doc_and_sketch()?;
    let before = sk.curves.clone();
    let mut next = before.clone();
    f(doc, &mut next, sk.level)?;
    if next != before {
        sk.undo.push(before);
        sk.redo.clear();
        sk.curves = next;
        sk.bad.clear();
        sk.error = None;
    }
    finish(window, &session)
}

fn default_type(doc: &Document, kind: SketchKind) -> Option<ElementId> {
    let cat = match kind {
        SketchKind::Floor => Category::FloorType,
        SketchKind::Ceiling => Category::CeilingType,
    };
    studio_core::ops::first_of(doc, cat)
}

/// Enters sketch mode: a new floor or ceiling on the view's level (or `level`, from 3D), or
/// Edit Boundary of `target`. Outside a plan the sketch goes through the level's plan
/// (ADR-025).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn sketch_begin(
    view: ElementId,
    kind: SketchKind,
    target: Option<ElementId>,
    type_id: Option<ElementId>,
    level: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut session = lock(&state)?;
    let doc = session.doc()?;
    let (level, curves, type_id) = match target {
        Some(id) => {
            let d = doc.data(id)?;
            let (level, kind_ok) = match d {
                ElementData::Floor { level, .. } => (*level, kind == SketchKind::Floor),
                ElementData::Ceiling { level, .. } => (*level, kind == SketchKind::Ceiling),
                _ => (ElementId::default(), false),
            };
            if !kind_ok {
                return Err(
                    anyhow::anyhow!("select a floor or ceiling to edit its boundary").into(),
                );
            }
            let outline = studio_regen::derived::slab_outline(doc, id);
            let curves = sketch::curves_of(doc, id, outline)?;
            let t = d
                .type_id()
                .ok_or_else(|| anyhow::anyhow!("that has no type"))?;
            (level, curves, t)
        }
        None => {
            let level = match (session.view_level(view), level) {
                (Ok(l), _) => l,
                (Err(_), Some(l)) => l,
                (Err(e), None) => return Err(e.into()),
            };
            let t = type_id
                .or_else(|| default_type(doc, kind))
                .ok_or_else(|| anyhow::anyhow!("no types for this sketch"))?;
            (level, vec![], t)
        }
    };
    let view = if session.view_level(view).is_ok() {
        view
    } else {
        sketch::plan_for(doc, level, kind)
            .ok_or_else(|| anyhow::anyhow!("that level has no floor plan to sketch through"))?
    };
    let elevation = sketch::work_plane_z(doc, level, kind, target)?;
    session.set_sketch(Some(SketchSession {
        kind,
        view,
        level,
        target,
        type_id,
        elevation,
        curves,
        undo: vec![],
        redo: vec![],
        bad: vec![],
        error: None,
    }));
    finish(&window, &session)
}

/// Adds curves from a draw tool's clicks. Chained lines with a radius round the corner
/// with the previous line.
#[tauri::command]
pub fn sketch_draw(
    tool: DrawTool,
    pts: Vec<Pt>,
    options: DrawOptions,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    sketch_edit(&window, &state, |_, curves, _| {
        let made = sketch::draw(tool, &pts, &options)?;
        let prev = curves.len().checked_sub(1);
        curves.extend(made);
        if let (DrawTool::Line, Some(r), Some(p)) = (tool, options.radius, prev) {
            let joined = matches!(&curves[p], SketchCurve::Line { b, .. } if b.dist(pts[0]) < 1.0);
            if joined {
                let n = curves.len() - 1;
                sketch::fillet(curves, p, n, r)?;
            }
        }
        Ok(())
    })
}

/// Pick Walls at `cursor`: the wall's face on the cursor's side (or the whole chain).
#[tauri::command]
pub fn sketch_pick_walls(
    cursor: Pt,
    tol: f64,
    chain: bool,
    core: bool,
    offset: f64,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    // A floor reaches the outside of its exterior walls by default; a ceiling takes the
    // face on the cursor's side.
    let floor = lock(&state)?
        .sketch()
        .is_some_and(|s| s.kind == SketchKind::Floor);
    sketch_edit(&window, &state, |doc, curves, level| {
        let w = sketch::wall_at(doc, level, cursor, tol)
            .ok_or_else(|| anyhow::anyhow!("click a wall on this level"))?;
        let picked = if floor {
            sketch::pick_floor_walls(doc, w, cursor, chain, core, offset)?
        } else {
            sketch::pick_walls(doc, w, cursor, chain, core, offset)?
        };
        sketch::add_picked(curves, picked, PICK_JOIN);
        Ok(())
    })
}

/// Pick Lines at `cursor`: a wall face or centerline or a grid.
#[tauri::command]
pub fn sketch_pick_line(
    cursor: Pt,
    tol: f64,
    offset: f64,
    lock_to_wall: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let view = {
        let session = lock(&state)?;
        session.sketch().map(|s| s.view)
    };
    sketch_edit(&window, &state, |doc, curves, _| {
        let view = view.ok_or_else(|| anyhow::anyhow!("no sketch in progress"))?;
        let r = studio_views::ref_line(doc, view, cursor, tol, None)
            .ok_or_else(|| anyhow::anyhow!("click a wall face, a wall centerline or a grid"))?;
        let wall = matches!(doc.data(r.el), Ok(ElementData::Wall { .. })).then_some(r.el);
        let c = sketch::pick_line(doc, r.a, r.b, wall, cursor, offset, lock_to_wall);
        sketch::add_picked(curves, vec![c], PICK_JOIN);
        Ok(())
    })
}

/// The sketch curve nearest `p`, within `tol`.
#[tauri::command]
pub fn sketch_hit(p: Pt, tol: f64, state: State<'_, SessionState>) -> CommandResult<Option<usize>> {
    let session = lock(&state)?;
    Ok(session.sketch().and_then(|s| {
        s.curves
            .iter()
            .enumerate()
            .map(|(i, c)| (c.distance(p), i))
            .filter(|(d, _)| *d <= tol)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|x| x.1)
    }))
}

#[tauri::command]
pub fn sketch_fillet(
    a: usize,
    b: usize,
    radius: f64,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    sketch_edit(&window, &state, |_, curves, _| {
        Ok(sketch::fillet(curves, a, b, radius)?)
    })
}

#[tauri::command]
pub fn sketch_trim(
    a: usize,
    a_pick: Pt,
    b: usize,
    b_pick: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    sketch_edit(&window, &state, |_, curves, _| {
        Ok(sketch::trim_corner(curves, a, a_pick, b, b_pick)?)
    })
}

#[tauri::command]
pub fn sketch_delete(
    indices: Vec<usize>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    sketch_edit(&window, &state, |_, curves, _| {
        let mut keep = 0;
        curves.retain(|_| {
            keep += 1;
            !indices.contains(&(keep - 1))
        });
        Ok(())
    })
}

#[tauri::command]
pub fn sketch_move_vertex(
    from: Pt,
    to: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    sketch_edit(&window, &state, |_, curves, _| {
        sketch::move_vertex(curves, from, to);
        Ok(())
    })
}

#[tauri::command]
pub fn sketch_flip(
    indices: Vec<usize>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    sketch_edit(&window, &state, |doc, curves, _| {
        sketch::flip(doc, curves, &indices);
        Ok(())
    })
}

#[tauri::command]
pub fn sketch_undo(
    redo: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut session = lock(&state)?;
    let (_, sk) = session.doc_and_sketch()?;
    let (from, to) = if redo {
        (&mut sk.redo, &mut sk.undo)
    } else {
        (&mut sk.undo, &mut sk.redo)
    };
    if let Some(prev) = from.pop() {
        to.push(std::mem::replace(&mut sk.curves, prev));
        sk.bad.clear();
        sk.error = None;
    }
    finish(&window, &session)
}

#[tauri::command]
pub fn sketch_set_type(
    type_id: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut session = lock(&state)?;
    let (_, sk) = session.doc_and_sketch()?;
    sk.type_id = type_id;
    finish(&window, &session)
}

/// Finish (✓): creates the floor or ceiling, or explains what's wrong with the sketch
/// (the state's `sketch.error`, with the curves to highlight).
#[tauri::command]
pub fn sketch_finish(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    let mut session = lock(&state)?;
    let Some(sk) = session.sketch().cloned() else {
        return finish(&window, &session);
    };
    let result = session.edit(|d| {
        Ok(sketch::finish(
            d, sk.kind, sk.target, sk.type_id, sk.level, &sk.curves,
        ))
    })?;
    match result {
        Ok(_) => session.set_sketch(None),
        Err(e) => {
            if let Ok((_, s)) = session.doc_and_sketch() {
                s.bad = e.bad;
                s.error = Some(e.message);
            }
        }
    }
    finish(&window, &session)
}

/// Cancel (✗): leaves sketch mode without changing the model.
#[tauri::command]
pub fn sketch_cancel(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    let mut session = lock(&state)?;
    session.set_sketch(None);
    finish(&window, &session)
}

/// What the active tool would add for the cursor at `cursor`, as polylines.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn sketch_preview(
    mode: String,
    pts: Vec<Pt>,
    cursor: Pt,
    options: DrawOptions,
    tol: f64,
    chain: bool,
    core: bool,
    state: State<'_, SessionState>,
) -> CommandResult<Vec<Vec<Pt>>> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let Some(sk) = session.sketch() else {
        return Ok(vec![]);
    };
    let curves: Vec<SketchCurve> = match mode.as_str() {
        "PickWalls" => sketch::wall_at(doc, sk.level, cursor, tol)
            .and_then(|w| sketch::pick_walls(doc, w, cursor, chain, core, options.offset).ok())
            .unwrap_or_default(),
        "PickLines" => studio_views::ref_line(doc, sk.view, cursor, tol, None)
            .map(|r| {
                vec![sketch::pick_line(
                    doc,
                    r.a,
                    r.b,
                    None,
                    cursor,
                    options.offset,
                    false,
                )]
            })
            .unwrap_or_default(),
        tool => {
            let Ok(tool) =
                serde_json::from_value::<DrawTool>(serde_json::Value::String(tool.into()))
            else {
                return Ok(vec![]);
            };
            let mut all = pts.clone();
            all.push(cursor);
            // Before the last click, preview with the cursor as the missing points.
            while all.len() < tool.points() {
                all.push(cursor);
            }
            if all.len() > tool.points() {
                all.truncate(tool.points());
            }
            sketch::draw(tool, &all, &options).unwrap_or_default()
        }
    };
    Ok(curves.iter().map(SketchCurve::points).collect())
}

/// Sketch endpoints near `p` to snap to (Revit snaps to its own sketch lines).
pub fn sketch_snap(session: &Session, p: Pt, tol: f64) -> Option<Pt> {
    let sk = session.sketch()?;
    sketch::snap_points(&sk.curves)
        .into_iter()
        .filter(|q| q.dist(p) <= tol)
        .min_by(|a, b| a.dist(p).total_cmp(&b.dist(p)))
}
