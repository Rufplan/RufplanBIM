//! Sketch mode for floor and ceiling boundaries (ADR-021): the sketch being edited lives in
//! the session until Finish. Thin wrappers over `studio_core::sketch`.

use serde::Serialize;
use studio_core::details::FillPattern;
use studio_core::inplace::{self, Form, FormKind};
use studio_core::sketch::{self, DrawOptions, DrawTool, SketchCurve, SketchKind};
use studio_core::wall_opening::{self, WallFrame};
use studio_core::{Category, Document, ElementData, ElementId, ViewKind};
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
    /// A wall opening's work plane: the wall face it's sketched on (ADR-058).
    pub wall: Option<WallPlane>,
    /// An in-place element's form being sketched (ADR-068).
    pub form: Option<FormDraft>,
    /// A filled region's pattern (ADR-069).
    pub region: Option<FillPattern>,
}

/// The form an in-place sketch makes (ADR-068), with the options bar's settings.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FormDraft {
    pub kind: FormKind,
    pub void: bool,
    /// The form being edited (Edit Sketch); None for a new one.
    pub index: Option<usize>,
    /// A blend's top is being sketched (its base is done).
    pub top: bool,
    #[serde(skip)]
    #[ts(skip)]
    pub base: Option<Vec<Vec<SketchCurve>>>,
}

impl SketchSession {
    /// A sketch of an in-place element's form on `level` (at `elevation` in 3D).
    #[allow(clippy::too_many_arguments)]
    pub fn for_form(
        view: ElementId,
        level: ElementId,
        element: ElementId,
        elevation: f64,
        curves: Vec<SketchCurve>,
        form: FormDraft,
    ) -> Self {
        Self {
            kind: SketchKind::InPlace,
            view,
            level,
            target: Some(element),
            type_id: element,
            elevation,
            curves,
            undo: vec![],
            redo: vec![],
            bad: vec![],
            error: None,
            wall: None,
            form: Some(form),
            region: None,
        }
    }
}

/// A wall face as the sketch's work plane (ADR-058), and how the sketch's coordinates map
/// onto it: in an elevation or section they are the view's (x across, y the elevation),
/// with the wall's u = `a` + `s`·x and z = y − `z_off`; in 3D they are the wall's own
/// (u, z) (a = 0, s = 1, z_off = 0).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WallPlane {
    pub frame: WallFrame,
    pub a: f64,
    pub s: f64,
    pub z_off: f64,
}

impl WallPlane {
    /// A curve in the sketch's coordinates, in the wall's (u, z).
    pub fn onto_wall(&self, c: &SketchCurve) -> SketchCurve {
        let (a, s, z) = (self.a, self.s, self.z_off);
        c.mapped(&|p| Pt::new(a + s * p.x, p.y - z), s < 0.0)
    }
    /// A curve in the wall's (u, z), in the sketch's coordinates.
    pub fn off_wall(&self, c: &SketchCurve) -> SketchCurve {
        let (a, s, z) = (self.a, self.s, self.z_off);
        c.mapped(&|p| Pt::new((p.x - a) * s, p.y + z), s < 0.0)
    }
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
    pub wall: Option<WallPlane>,
    pub form: Option<FormDraft>,
    pub region: Option<FillPattern>,
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
            wall: self.wall,
            form: self.form.clone(),
            region: self.region,
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
        SketchKind::WallOpening | SketchKind::InPlace | SketchKind::FilledRegion => return None,
        // A new ground region takes the base ground's material, else the first one.
        SketchKind::GroundRegion => {
            return studio_core::planting::ground(doc)
                .or_else(|| studio_core::ops::first_of(doc, Category::Material))
        }
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
    host: Option<ElementId>,
    toward: Option<Pt>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut session = lock(&state)?;
    if kind == SketchKind::WallOpening {
        let sk = wall_sketch(&session, view, target, host, toward)?;
        session.set_sketch(Some(sk));
        return finish(&window, &session);
    }
    if kind == SketchKind::FilledRegion {
        let sk = region_sketch(&session, view, target)?;
        session.set_sketch(Some(sk));
        return finish(&window, &session);
    }
    let doc = session.doc()?;
    let (level, curves, type_id) = match target {
        Some(id) => {
            let d = doc.data(id)?;
            let (level, kind_ok) = match d {
                ElementData::Floor { level, .. } => (*level, kind == SketchKind::Floor),
                ElementData::Ceiling { level, .. } => (*level, kind == SketchKind::Ceiling),
                ElementData::GroundRegion { level, .. } => {
                    (*level, kind == SketchKind::GroundRegion)
                }
                _ => (ElementId::default(), false),
            };
            if !kind_ok {
                return Err(
                    anyhow::anyhow!("select a floor or ceiling to edit its boundary").into(),
                );
            }
            let outline = studio_regen::derived::slab_outline(doc, id);
            let curves = sketch::curves_of(doc, id, outline)?;
            let t = match d {
                ElementData::GroundRegion { material, .. } => *material,
                _ => d
                    .type_id()
                    .ok_or_else(|| anyhow::anyhow!("that has no type"))?,
            };
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
        wall: None,
        form: None,
        region: None,
    }));
    finish(&window, &session)
}

/// A filled region's sketch (ADR-069), in the view it's drawn in (any 2D view); or
/// `target`'s boundary to edit.
pub(crate) fn region_sketch(
    session: &Session,
    view: ElementId,
    target: Option<ElementId>,
) -> anyhow::Result<SketchSession> {
    let doc = session.doc()?;
    let (view, curves, pattern) = match target {
        Some(id) => studio_core::details::region_curves(doc, id)?,
        None => (view, vec![], FillPattern::Diagonal),
    };
    if matches!(
        doc.data(view)?,
        ElementData::View {
            kind: ViewKind::ThreeD | ViewKind::Schedule { .. },
            ..
        }
    ) {
        anyhow::bail!(
            "Filled regions go in 2D views: plans, sections, elevations and drafting views."
        );
    }
    let level = doc
        .levels()
        .first()
        .map(|l| l.0)
        .ok_or_else(|| anyhow::anyhow!("the project has no levels"))?;
    Ok(SketchSession {
        kind: SketchKind::FilledRegion,
        view,
        level,
        target,
        type_id: view,
        elevation: 0.0,
        curves,
        undo: vec![],
        redo: vec![],
        bad: vec![],
        error: None,
        wall: None,
        form: None,
        region: Some(pattern),
    })
}

/// The pattern of the filled region being sketched.
#[tauri::command]
pub fn sketch_set_pattern(
    pattern: FillPattern,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut session = lock(&state)?;
    let (_, sk) = session.doc_and_sketch()?;
    sk.region = Some(pattern);
    finish(&window, &session)
}

/// A wall opening's sketch (ADR-058): on the face of `host` toward the viewer of an
/// elevation or section, or toward `toward` (the picked face's normal) in 3D; or the
/// sketch of the opening `target` to edit.
fn wall_sketch(
    session: &Session,
    view: ElementId,
    target: Option<ElementId>,
    host: Option<ElementId>,
    toward: Option<Pt>,
) -> anyhow::Result<SketchSession> {
    let doc = session.doc()?;
    let (host, existing) = match target {
        Some(id) => wall_opening::curves_of(doc, id)?,
        None => (
            host.ok_or_else(|| anyhow::anyhow!("click the wall to cut the opening in"))?,
            vec![],
        ),
    };
    let in_3d = matches!(
        doc.data(view)?,
        ElementData::View {
            kind: ViewKind::ThreeD,
            ..
        }
    );
    let plane = if in_3d {
        let frame = wall_opening::frame(doc, host, toward.unwrap_or(Pt::new(0.0, -1.0)))?;
        WallPlane {
            frame,
            a: 0.0,
            s: 1.0,
            z_off: 0.0,
        }
    } else {
        let (origin, right, look) = studio_views::view_frame(doc, view).ok_or_else(|| {
            anyhow::anyhow!("sketch a wall opening in an elevation, a section or a 3D view")
        })?;
        let frame = wall_opening::frame(doc, host, look.scale(-1.0))?;
        let s = right.dot(frame.dir);
        if s.abs() < 0.999 {
            anyhow::bail!(
                "That wall isn't square to this view. Sketch its opening in an elevation or section facing it, or in 3D."
            );
        }
        WallPlane {
            frame,
            a: origin.sub(frame.start).dot(frame.dir),
            s: s.signum(),
            z_off: frame.base_z,
        }
    };
    let level = match doc.data(host)? {
        ElementData::Wall { base_level, .. } => *base_level,
        _ => anyhow::bail!("pick a wall"),
    };
    Ok(SketchSession {
        kind: SketchKind::WallOpening,
        view,
        level,
        target,
        type_id: host,
        elevation: plane.frame.base_z,
        curves: existing.iter().map(|c| plane.off_wall(c)).collect(),
        undo: vec![],
        redo: vec![],
        bad: vec![],
        error: None,
        wall: Some(plane),
        form: None,
        region: None,
    })
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
    let (floor, on_wall) = {
        let s = lock(&state)?;
        let sk = s.sketch();
        (
            sk.is_some_and(|s| s.kind == SketchKind::Floor),
            sk.is_some_and(|s| s.wall.is_some()),
        )
    };
    if on_wall {
        return Err(anyhow::anyhow!("Pick Walls draws floor and ceiling boundaries").into());
    }
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

/// The options bar's settings for the form being sketched (ADR-068): its heights, or a
/// sweep's profile. A blend keeps the top it has.
#[tauri::command]
pub fn sketch_set_form(
    kind: FormKind,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut session = lock(&state)?;
    let (doc, sk) = session.doc_and_sketch()?;
    let f = sk
        .form
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("no form is being sketched"))?;
    f.kind = match (kind, &f.kind) {
        (FormKind::Blend { base, top, .. }, FormKind::Blend { top_sketch, .. }) => {
            FormKind::Blend {
                base,
                top,
                top_sketch: top_sketch.clone(),
            }
        }
        (k, _) => k,
    };
    sk.elevation = doc.level_elevation(sk.level)? + form_z(f);
    finish(&window, &session)
}

/// The height above the level the form's sketch is drawn at.
fn form_z(f: &FormDraft) -> f64 {
    match &f.kind {
        FormKind::Blend { top, .. } if f.top => *top,
        FormKind::Blend { base, .. } => *base,
        FormKind::Extrusion { start, .. } => *start,
        FormKind::Sweep { elevation, .. } => *elevation,
    }
}

/// Finish (✓) of an in-place form (ADR-068): a blend's base goes on to its top; otherwise
/// the form is added to (or replaces its old self in) the element.
fn finish_form(session: &mut Session, sk: SketchSession) -> anyhow::Result<()> {
    let draft = sk
        .form
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no form is being sketched"))?;
    let element = sk.type_id;
    let doc = session.doc()?;
    let checked = inplace::sketch_for(doc, &draft.kind, draft.top, &sk.curves);
    let form = match (checked, &draft.kind) {
        (Err(e), _) => Err(e),
        (Ok(loops), FormKind::Blend { top, .. }) if !draft.top => {
            // The base is done: sketch the top (the one it had, when editing).
            let old_top: Vec<SketchCurve> = match (draft.index, &draft.kind) {
                (Some(_), FormKind::Blend { top_sketch, .. }) => top_sketch.clone(),
                _ => vec![],
            };
            let z = doc.level_elevation(sk.level)? + top;
            let (_, s) = session.doc_and_sketch()?;
            if let Some(f) = s.form.as_mut() {
                f.base = Some(loops);
                f.top = true;
            }
            s.curves = old_top;
            s.undo.clear();
            s.redo.clear();
            s.bad.clear();
            s.error = None;
            s.elevation = z;
            return Ok(());
        }
        (Ok(loops), &FormKind::Blend { base, top, .. }) => Ok(Form {
            kind: FormKind::Blend {
                base,
                top,
                top_sketch: loops.into_iter().next().unwrap_or_default(),
            },
            sketch: draft.base.clone().unwrap_or_default(),
            void: false,
        }),
        (Ok(loops), k) => Ok(Form {
            kind: k.clone(),
            sketch: loops,
            void: draft.void,
        }),
    };
    let result = match form {
        Ok(form) => session
            .edit(|d| match draft.index {
                Some(i) => inplace::set_form(d, element, i, form),
                None => inplace::add_form(d, element, form).map(|_| ()),
            })
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.message),
    };
    match result {
        Ok(()) => session.set_sketch(None),
        Err(message) => {
            if let Ok((_, s)) = session.doc_and_sketch() {
                s.error = Some(message);
            }
        }
    }
    Ok(())
}

/// Finish of the sketch in progress as an in-place form (for tests).
#[cfg(test)]
pub(crate) fn finish_form_for_test(session: &mut Session) -> anyhow::Result<()> {
    let sk = session
        .sketch()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no sketch"))?;
    finish_form(session, sk)
}

/// Finish (✓): creates the floor or ceiling, or explains what's wrong with the sketch
/// (the state's `sketch.error`, with the curves to highlight).
#[tauri::command]
pub fn sketch_finish(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    let mut session = lock(&state)?;
    let Some(sk) = session.sketch().cloned() else {
        return finish(&window, &session);
    };
    if sk.kind == SketchKind::InPlace {
        finish_form(&mut session, sk)?;
        return finish(&window, &session);
    }
    let result = session.edit(|d| {
        Ok(match sk.wall {
            // A wall opening (ADR-058): the sketch onto its wall.
            Some(plane) => {
                let curves: Vec<SketchCurve> =
                    sk.curves.iter().map(|c| plane.onto_wall(c)).collect();
                wall_opening::finish(d, sk.target, plane.frame.wall, &curves)
            }
            None if sk.kind == SketchKind::FilledRegion => studio_core::details::finish_region(
                d,
                sk.view,
                sk.target,
                sk.region.unwrap_or_default(),
                &sk.curves,
            ),
            None => sketch::finish(d, sk.kind, sk.target, sk.type_id, sk.level, &sk.curves),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wall_plane_maps_a_mirrored_view_onto_the_wall_and_back() {
        let frame = WallFrame {
            wall: ElementId::new(),
            start: Pt::new(1000.0, 0.0),
            dir: Pt::new(1.0, 0.0),
            normal: Pt::new(0.0, 1.0),
            half: 100.0,
            base_z: 3000.0,
            length: 6000.0,
            height: 3000.0,
        };
        // Seen from the north the view's x runs west: u = 5000 − x, and y is the elevation.
        let plane = WallPlane {
            frame,
            a: 5000.0,
            s: -1.0,
            z_off: 3000.0,
        };
        let arc = SketchCurve::Arc {
            center: Pt::new(2000.0, 4000.0),
            radius: 300.0,
            start: 0.0,
            sweep: std::f64::consts::FRAC_PI_2,
        };
        let on_wall = plane.onto_wall(&arc);
        let SketchCurve::Arc {
            center,
            start,
            sweep,
            ..
        } = on_wall
        else {
            unreachable!()
        };
        assert!(center.dist(Pt::new(3000.0, 1000.0)) < 1e-9);
        // Mirrored: the same arc, swept the other way from the mirrored start.
        assert!((sweep + std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!((start - std::f64::consts::PI).abs() < 1e-9);
        let back = plane.off_wall(&on_wall);
        let (a, b) = (arc.points(), back.points());
        assert!(a.iter().zip(&b).all(|(p, q)| p.dist(*q) < 1e-6));
    }
}
