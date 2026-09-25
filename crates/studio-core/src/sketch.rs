//! Boundary sketches for floors and ceilings, as in Revit's sketch mode (ADR-021).
//!
//! A sketch is a set of curves (lines and arcs). Lines picked from walls stay locked to a
//! face of their wall and are re-trimmed against their neighbors whenever the model
//! regenerates, so a floor sketched by Pick Walls follows the walls. On Finish the curves
//! must form closed loops that don't intersect; inner loops become openings.

use serde::{Deserialize, Serialize};
use studio_geom::{line_intersection, point_in_ring, project_to_segment, signed_area, Poly, Pt};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};

const TAU: f64 = std::f64::consts::TAU;
/// Endpoints closer than this join (mm).
pub const JOIN_TOL: f64 = 1.0;

/// Which line of a wall a locked sketch line follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WallFace {
    Exterior,
    Interior,
    CoreExterior,
    CoreInterior,
    Centerline,
}

/// A sketch line locked to a wall face, `offset` mm further out from the wall.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WallRef {
    pub wall: ElementId,
    pub face: WallFace,
    pub offset: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SketchCurve {
    Line {
        a: Pt,
        b: Pt,
        #[serde(default)]
        wall: Option<WallRef>,
    },
    /// Counter-clockwise from angle `start` by `sweep` radians (negative: clockwise). A
    /// full circle has |sweep| = 2π.
    Arc {
        center: Pt,
        radius: f64,
        start: f64,
        sweep: f64,
    },
}

/// Revit's boundary line draw tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DrawTool {
    Line,
    Rectangle,
    InscribedPolygon,
    CircumscribedPolygon,
    Circle,
    StartEndRadiusArc,
    CenterEndsArc,
}

impl DrawTool {
    /// Clicks the tool needs.
    pub fn points(self) -> usize {
        match self {
            DrawTool::Line
            | DrawTool::Rectangle
            | DrawTool::InscribedPolygon
            | DrawTool::CircumscribedPolygon
            | DrawTool::Circle => 2,
            DrawTool::StartEndRadiusArc | DrawTool::CenterEndsArc => 3,
        }
    }
}

/// Options-bar settings of the draw tools (lengths in mm).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DrawOptions {
    pub offset: f64,
    /// Fillet radius for chained lines and rectangles (None: sharp corners).
    pub radius: Option<f64>,
    pub sides: u32,
}

impl Default for DrawOptions {
    fn default() -> Self {
        Self {
            offset: 0.0,
            radius: None,
            sides: 6,
        }
    }
}

/// Why a sketch can't finish, with the curves to highlight (Revit's wording).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SketchError {
    pub message: String,
    pub bad: Vec<usize>,
}

impl SketchError {
    fn new(message: &str, bad: Vec<usize>) -> Self {
        Self {
            message: message.into(),
            bad,
        }
    }
}

fn polar(c: Pt, r: f64, a: f64) -> Pt {
    c.add(Pt::new(a.cos() * r, a.sin() * r))
}

impl SketchCurve {
    pub fn line(a: Pt, b: Pt) -> Self {
        SketchCurve::Line { a, b, wall: None }
    }

    pub fn ends(&self) -> (Pt, Pt) {
        match self {
            SketchCurve::Line { a, b, .. } => (*a, *b),
            SketchCurve::Arc {
                center,
                radius,
                start,
                sweep,
            } => (
                polar(*center, *radius, *start),
                polar(*center, *radius, start + sweep),
            ),
        }
    }

    pub fn is_circle(&self) -> bool {
        matches!(self, SketchCurve::Arc { sweep, .. } if sweep.abs() >= TAU - 1e-9)
    }

    pub fn reversed(&self) -> Self {
        match self {
            SketchCurve::Line { a, b, wall } => SketchCurve::Line {
                a: *b,
                b: *a,
                wall: *wall,
            },
            SketchCurve::Arc {
                center,
                radius,
                start,
                sweep,
            } => SketchCurve::Arc {
                center: *center,
                radius: *radius,
                start: start + sweep,
                sweep: -sweep,
            },
        }
    }

    /// Points along the curve, ends included (arcs every 5° or so).
    pub fn points(&self) -> Vec<Pt> {
        match self {
            SketchCurve::Line { a, b, .. } => vec![*a, *b],
            SketchCurve::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let n = ((sweep.abs() / 5f64.to_radians()).ceil() as usize).max(2);
                (0..=n)
                    .map(|i| polar(*center, *radius, start + sweep * i as f64 / n as f64))
                    .collect()
            }
        }
    }

    pub fn length(&self) -> f64 {
        match self {
            SketchCurve::Line { a, b, .. } => a.dist(*b),
            SketchCurve::Arc { radius, sweep, .. } => radius * sweep.abs(),
        }
    }

    /// The curve with every point mapped by `f` (a rigid motion; `mirror` flips arcs).
    pub fn mapped(&self, f: &dyn Fn(Pt) -> Pt, mirror: bool) -> Self {
        match self {
            SketchCurve::Line { a, b, wall } => SketchCurve::Line {
                a: f(*a),
                b: f(*b),
                wall: *wall,
            },
            SketchCurve::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let c = f(*center);
                let s = f(polar(*center, *radius, *start)).sub(c);
                let a0 = s.y.atan2(s.x);
                SketchCurve::Arc {
                    center: c,
                    radius: *radius,
                    start: a0,
                    sweep: if mirror { -sweep } else { *sweep },
                }
            }
        }
    }

    /// Distance from `p` to the curve.
    pub fn distance(&self, p: Pt) -> f64 {
        let pts = self.points();
        pts.windows(2)
            .map(|w| project_to_segment(p, w[0], w[1]).1)
            .fold(f64::INFINITY, f64::min)
    }
}

/// Offset of a wall face from the wall's centerline (+ = the wall's left, exterior side)
/// and the wall's line, for the wall as it is now.
fn face_line(doc: &Document, r: &WallRef) -> Option<(Pt, Pt, f64)> {
    let ElementData::Wall {
        type_id,
        start,
        end,
        ..
    } = doc.data(r.wall).ok()?
    else {
        return None;
    };
    let (width, layers) = match doc.data(*type_id).ok()? {
        ElementData::WallType {
            thickness, layers, ..
        } => (*thickness, layers.as_slice()),
        _ => return None,
    };
    let (ce, ci) = crate::compound::core_faces(layers, width);
    let (o, out) = match r.face {
        WallFace::Exterior => (width / 2.0, 1.0),
        WallFace::Interior => (-width / 2.0, -1.0),
        WallFace::CoreExterior => (ce, 1.0),
        WallFace::CoreInterior => (ci, -1.0),
        WallFace::Centerline => (0.0, 1.0),
    };
    Some((*start, *end, o + out * r.offset))
}

/// The infinite line a locked sketch line lies on now: a point and a unit direction.
fn locked_line(doc: &Document, r: &WallRef) -> Option<(Pt, Pt)> {
    let (s, e, o) = face_line(doc, r)?;
    let d = e.sub(s).norm();
    Some((s.add(d.perp().scale(o)), d))
}

/// A curve with a locked line moved onto its wall's current face (its ends projected).
pub fn resolve(doc: &Document, c: &SketchCurve) -> SketchCurve {
    if let SketchCurve::Line {
        a,
        b,
        wall: Some(r),
    } = c
    {
        if let Some((p, d)) = locked_line(doc, r) {
            let on = |q: Pt| p.add(d.scale(q.sub(p).dot(d)));
            return SketchCurve::Line {
                a: on(*a),
                b: on(*b),
                wall: Some(*r),
            };
        }
    }
    c.clone()
}

/// The wall face nearest the cursor's side, as a locked line along the whole wall.
pub fn pick_wall(
    doc: &Document,
    wall: ElementId,
    cursor: Pt,
    core: bool,
    offset: f64,
) -> CoreResult<SketchCurve> {
    let ElementData::Wall { start, end, .. } = doc.data(wall)? else {
        return Err(CoreError::Invalid("pick a wall".into()));
    };
    let left = end.sub(*start).cross(cursor.sub(*start)) > 0.0;
    let face = match (left, core) {
        (true, false) => WallFace::Exterior,
        (false, false) => WallFace::Interior,
        (true, true) => WallFace::CoreExterior,
        (false, true) => WallFace::CoreInterior,
    };
    let r = WallRef { wall, face, offset };
    let (p, d) = locked_line(doc, &r).ok_or_else(|| CoreError::Invalid("pick a wall".into()))?;
    let len = start.dist(*end);
    Ok(SketchCurve::Line {
        a: p,
        b: p.add(d.scale(len)),
        wall: Some(r),
    })
}

/// A picked line (a wall face, centerline or grid) `offset` mm toward the cursor. When
/// `lock` and the line is a face or centerline of `wall`, it stays on that wall.
pub fn pick_line(
    doc: &Document,
    a: Pt,
    b: Pt,
    wall: Option<ElementId>,
    cursor: Pt,
    offset: f64,
    lock: bool,
) -> SketchCurve {
    let d = b.sub(a).norm();
    let toward = if d.cross(cursor.sub(a)) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let shift = d.perp().scale(toward * offset);
    let free = SketchCurve::line(a.add(shift), b.add(shift));
    let Some(w) = wall.filter(|_| lock) else {
        return free;
    };
    // Which face of the wall the picked line is.
    for face in [
        WallFace::Exterior,
        WallFace::Interior,
        WallFace::CoreExterior,
        WallFace::CoreInterior,
        WallFace::Centerline,
    ] {
        let r = WallRef {
            wall: w,
            face,
            offset: 0.0,
        };
        let Some((p, dir)) = locked_line(doc, &r) else {
            return free;
        };
        if a.sub(p).dot(dir.perp()).abs() < 0.5 {
            // Offset away from the wall on the cursor's side (a centerline goes left).
            let out = match face {
                WallFace::Interior | WallFace::CoreInterior => -1.0,
                _ => 1.0,
            };
            let side = if dir.perp().dot(shift) >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let r = WallRef {
                offset: offset * side * out,
                ..r
            };
            if let SketchCurve::Line { a, b, .. } = free {
                return SketchCurve::Line {
                    a,
                    b,
                    wall: Some(r),
                };
            }
        }
    }
    free
}

/// Pick Walls: the hovered wall's face on the cursor's side, or with `chain` (Tab) every
/// wall connected to it, each on the same side (inside or outside) of a closed chain.
pub fn pick_walls(
    doc: &Document,
    hovered: ElementId,
    cursor: Pt,
    chain: bool,
    core: bool,
    offset: f64,
) -> CoreResult<Vec<SketchCurve>> {
    if !chain {
        return Ok(vec![pick_wall(doc, hovered, cursor, core, offset)?]);
    }
    let walls = wall_chain(doc, hovered);
    let line = |w: ElementId| match doc.data(w) {
        Ok(ElementData::Wall { start, end, .. }) => Some((*start, *end)),
        _ => None,
    };
    let center: Vec<SketchCurve> = walls
        .iter()
        .filter_map(|w| line(*w).map(|(a, b)| SketchCurve::line(a, b)))
        .collect();
    let ring = loops(&center)
        .ok()
        .filter(|l| l.len() == 1)
        .map(|l| loop_points(&l[0]));
    let (ha, hb) = line(hovered).ok_or_else(|| CoreError::Invalid("pick a wall".into()))?;
    let hover_left = hb.sub(ha).cross(cursor.sub(ha)) > 0.0;
    let inside = ring.as_ref().map(|r| point_in_ring(cursor, r));
    let mut out = vec![];
    for w in &walls {
        let Some((a, b)) = line(*w) else { continue };
        let n = b.sub(a).norm().perp();
        let mid = a.lerp(b, 0.5);
        // A point just off the wall on the side to pick.
        let left = match (&ring, inside) {
            (Some(r), Some(ins)) => point_in_ring(mid.add(n.scale(10.0)), r) == ins,
            _ => hover_left,
        };
        let probe = mid.add(n.scale(if left { 10.0 } else { -10.0 }));
        out.push(pick_wall(doc, *w, probe, core, offset)?);
    }
    Ok(out)
}

/// Adds picked lines, joining each to the lines already there at their corners.
pub fn add_picked(curves: &mut Vec<SketchCurve>, picked: Vec<SketchCurve>, tol: f64) {
    for c in picked {
        curves.push(c);
        let n = curves.len() - 1;
        auto_join(curves, n, tol);
    }
}

/// Walls on `level` connected end to end with `wall` (Tab-selecting a chain in Revit).
pub fn wall_chain(doc: &Document, wall: ElementId) -> Vec<ElementId> {
    let walls: Vec<(ElementId, ElementId, Pt, Pt)> = doc
        .of(Category::Wall)
        .filter_map(|e| match &e.data {
            ElementData::Wall {
                base_level,
                start,
                end,
                ..
            } => Some((e.id, *base_level, *start, *end)),
            _ => None,
        })
        .collect();
    let Some(level) = walls.iter().find(|w| w.0 == wall).map(|w| w.1) else {
        return vec![];
    };
    let mut out = vec![wall];
    let mut i = 0;
    while i < out.len() {
        let Some(&(_, _, s, e)) = walls.iter().find(|w| w.0 == out[i]) else {
            break;
        };
        let found: Vec<ElementId> = walls
            .iter()
            .filter(|w| w.1 == level && !out.contains(&w.0))
            .filter(|w| {
                [w.2, w.3]
                    .iter()
                    .any(|p| p.dist(s) < JOIN_TOL || p.dist(e) < JOIN_TOL)
            })
            .map(|w| w.0)
            .collect();
        out.extend(found);
        i += 1;
    }
    out
}

/// The wall on `level` under `p` (within `tol` of its faces).
pub fn wall_at(doc: &Document, level: ElementId, p: Pt, tol: f64) -> Option<ElementId> {
    doc.of(Category::Wall)
        .filter_map(|e| match &e.data {
            ElementData::Wall {
                base_level,
                start,
                end,
                type_id,
                ..
            } if *base_level == level => {
                let w = match doc.data(*type_id).ok()? {
                    ElementData::WallType { thickness, .. } => *thickness,
                    _ => 0.0,
                };
                let d = project_to_segment(p, *start, *end).1;
                (d <= w / 2.0 + tol).then_some((d, e.id))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| x.1)
}

/// Curves a draw tool makes from its clicks (`pts.len() == tool.points()`).
pub fn draw(tool: DrawTool, pts: &[Pt], o: &DrawOptions) -> CoreResult<Vec<SketchCurve>> {
    let bad = |m: &str| Err(CoreError::Invalid(m.into()));
    if pts.len() < tool.points() {
        return bad("pick more points");
    }
    let (p0, p1) = (pts[0], pts[1]);
    match tool {
        DrawTool::Line => {
            if p0.dist(p1) < 1.0 {
                return bad("line is too short");
            }
            let n = p1.sub(p0).norm().perp().scale(o.offset);
            Ok(vec![SketchCurve::line(p0.add(n), p1.add(n))])
        }
        DrawTool::Rectangle => {
            let (lo, hi) = (
                Pt::new(p0.x.min(p1.x) - o.offset, p0.y.min(p1.y) - o.offset),
                Pt::new(p0.x.max(p1.x) + o.offset, p0.y.max(p1.y) + o.offset),
            );
            if hi.x - lo.x < 1.0 || hi.y - lo.y < 1.0 {
                return bad("rectangle is too small");
            }
            let c = [lo, Pt::new(hi.x, lo.y), hi, Pt::new(lo.x, hi.y)];
            let r = o.radius.unwrap_or(0.0);
            if r <= 0.0 {
                return Ok((0..4)
                    .map(|i| SketchCurve::line(c[i], c[(i + 1) % 4]))
                    .collect());
            }
            if 2.0 * r >= (hi.x - lo.x).min(hi.y - lo.y) {
                return bad("the radius is too large for this rectangle");
            }
            // Counter-clockwise: each edge shortened by r, then a quarter arc at its end.
            let mut out = vec![];
            for i in 0..4 {
                let (a, b) = (c[i], c[(i + 1) % 4]);
                let d = b.sub(a).norm();
                out.push(SketchCurve::line(a.add(d.scale(r)), b.sub(d.scale(r))));
                let center = b.sub(d.scale(r)).add(d.perp().scale(r));
                let s = d.perp().scale(-1.0);
                out.push(SketchCurve::Arc {
                    center,
                    radius: r,
                    start: s.y.atan2(s.x),
                    sweep: std::f64::consts::FRAC_PI_2,
                });
            }
            Ok(out)
        }
        DrawTool::InscribedPolygon | DrawTool::CircumscribedPolygon => {
            let n = o.sides.clamp(3, 64) as usize;
            let v = p1.sub(p0);
            let d = v.len() + o.offset;
            if d < 1.0 {
                return bad("polygon is too small");
            }
            let a0 = v.y.atan2(v.x);
            let step = TAU / n as f64;
            let (r, first) = if tool == DrawTool::InscribedPolygon {
                (d, a0)
            } else {
                // The cursor sets the apothem, at the middle of an edge.
                (d / (step / 2.0).cos(), a0 - step / 2.0)
            };
            let c: Vec<Pt> = (0..n)
                .map(|i| polar(p0, r, first + step * i as f64))
                .collect();
            Ok((0..n)
                .map(|i| SketchCurve::line(c[i], c[(i + 1) % n]))
                .collect())
        }
        DrawTool::Circle => {
            let r = p0.dist(p1) + o.offset;
            if r < 1.0 {
                return bad("circle is too small");
            }
            Ok(vec![SketchCurve::Arc {
                center: p0,
                radius: r,
                start: 0.0,
                sweep: TAU,
            }])
        }
        DrawTool::StartEndRadiusArc => {
            // Through the start, the end and the third point.
            let (a, b, m) = (p0, p1, pts[2]);
            let Some(c) = circumcenter(a, b, m) else {
                return bad("the three points are in a line");
            };
            let ang = |p: Pt| p.sub(c).y.atan2(p.sub(c).x);
            let (sa, sb, sm) = (ang(a), ang(b), ang(m));
            let ccw = |from: f64, to: f64| (to - from).rem_euclid(TAU);
            let sweep = if ccw(sa, sm) < ccw(sa, sb) {
                ccw(sa, sb)
            } else {
                ccw(sa, sb) - TAU
            };
            Ok(vec![SketchCurve::Arc {
                center: c,
                radius: c.dist(a),
                start: sa,
                sweep,
            }])
        }
        DrawTool::CenterEndsArc => {
            let (c, a, e) = (p0, p1, pts[2]);
            let r = c.dist(a);
            if r < 1.0 {
                return bad("arc is too small");
            }
            let sa = a.sub(c).y.atan2(a.sub(c).x);
            let se = e.sub(c).y.atan2(e.sub(c).x);
            // Up to a half circle, toward the cursor (as Revit's Center-ends Arc).
            let mut sweep = (se - sa).rem_euclid(TAU);
            if sweep > std::f64::consts::PI {
                sweep -= TAU;
            }
            if sweep.abs() < 1e-6 {
                return bad("arc is too small");
            }
            Ok(vec![SketchCurve::Arc {
                center: c,
                radius: r,
                start: sa,
                sweep,
            }])
        }
    }
}

fn circumcenter(a: Pt, b: Pt, c: Pt) -> Option<Pt> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a.dot(a), b.dot(b), c.dot(c));
    Some(Pt::new(
        (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    ))
}

fn line_parts(c: &SketchCurve) -> Option<(Pt, Pt)> {
    match c {
        SketchCurve::Line { a, b, .. } => Some((*a, *b)),
        SketchCurve::Arc { .. } => None,
    }
}

fn set_line(c: &mut SketchCurve, na: Pt, nb: Pt) {
    if let SketchCurve::Line { a, b, .. } = c {
        *a = na;
        *b = nb;
    }
}

/// Rounds the corner between lines `i` and `j` with an arc of radius `r` (Fillet Arc),
/// trimming both lines to it. Inserts the arc after the first of them.
pub fn fillet(curves: &mut Vec<SketchCurve>, i: usize, j: usize, r: f64) -> CoreResult<()> {
    let bad = |m: &str| Err(CoreError::Invalid(m.into()));
    let (Some((a1, b1)), Some((a2, b2))) = (
        curves.get(i).and_then(line_parts),
        curves.get(j).and_then(line_parts),
    ) else {
        return bad("fillet two lines");
    };
    if i == j || r <= 0.0 {
        return bad("fillet two different lines with a radius");
    }
    let Some(x) = line_intersection(a1, b1.sub(a1), a2, b2.sub(a2)) else {
        return bad("parallel lines can't be filleted");
    };
    let far = |a: Pt, b: Pt| if a.dist(x) > b.dist(x) { a } else { b };
    let (f1, f2) = (far(a1, b1), far(a2, b2));
    let (u1, u2) = (f1.sub(x).norm(), f2.sub(x).norm());
    let theta = u1.dot(u2).clamp(-1.0, 1.0).acos();
    if theta < 1e-3 || (std::f64::consts::PI - theta) < 1e-3 {
        return bad("those lines don't form a corner");
    }
    let t = r / (theta / 2.0).tan();
    if t > f1.dist(x) || t > f2.dist(x) {
        return bad("the radius is too large for these lines");
    }
    let (t1, t2) = (x.add(u1.scale(t)), x.add(u2.scale(t)));
    let bis = u1.add(u2).norm();
    let center = x.add(bis.scale(r / (theta / 2.0).sin()));
    let ang = |p: Pt| p.sub(center).y.atan2(p.sub(center).x);
    let (s1, s2) = (ang(t1), ang(t2));
    let mut sweep = (s2 - s1).rem_euclid(TAU);
    if sweep > std::f64::consts::PI {
        sweep -= TAU;
    }
    // Keep each line's far end; its near end moves to the tangent point.
    let near1 = if a1.dist(x) < b1.dist(x) {
        (t1, b1)
    } else {
        (a1, t1)
    };
    let near2 = if a2.dist(x) < b2.dist(x) {
        (t2, b2)
    } else {
        (a2, t2)
    };
    set_line(&mut curves[i], near1.0, near1.1);
    set_line(&mut curves[j], near2.0, near2.1);
    curves.insert(
        i.max(j),
        SketchCurve::Arc {
            center,
            radius: r,
            start: s1,
            sweep,
        },
    );
    Ok(())
}

/// Trims or extends lines `i` and `j` to their corner, keeping the side of each that was
/// clicked (`pi`, `pj`), as Revit's Trim/Extend to Corner.
pub fn trim_corner(
    curves: &mut [SketchCurve],
    i: usize,
    pi: Pt,
    j: usize,
    pj: Pt,
) -> CoreResult<()> {
    let (Some((a1, b1)), Some((a2, b2))) = (
        curves.get(i).and_then(line_parts),
        curves.get(j).and_then(line_parts),
    ) else {
        return Err(CoreError::Invalid("trim two lines to a corner".into()));
    };
    let Some(x) = line_intersection(a1, b1.sub(a1), a2, b2.sub(a2)) else {
        return Err(CoreError::Invalid("parallel lines have no corner".into()));
    };
    let keep = |a: Pt, b: Pt, pick: Pt| {
        let d = b.sub(a);
        let t = |p: Pt| p.sub(a).dot(d) / d.dot(d);
        if t(pick) < t(x) {
            (a, x)
        } else {
            (x, b)
        }
    };
    let (n1, n2) = (keep(a1, b1, pi), keep(a2, b2, pj));
    set_line(&mut curves[i], n1.0, n1.1);
    set_line(&mut curves[j], n2.0, n2.1);
    Ok(())
}

/// After adding curve `new` (a picked line), joins its ends to nearby line ends at their
/// corner, as Revit trims picked walls' lines to each other.
pub fn auto_join(curves: &mut [SketchCurve], new: usize, tol: f64) {
    let Some((na, nb)) = curves.get(new).and_then(line_parts) else {
        return;
    };
    for k in 0..curves.len() {
        if k == new {
            continue;
        }
        let Some((oa, ob)) = line_parts(&curves[k]) else {
            continue;
        };
        let Some(x) = line_intersection(na, nb.sub(na), oa, ob.sub(oa)) else {
            continue;
        };
        let (cur_a, cur_b) = line_parts(&curves[new]).unwrap_or((na, nb));
        let near_new = if cur_a.dist(x) < cur_b.dist(x) { 0 } else { 1 };
        let near_old = if oa.dist(x) < ob.dist(x) { 0 } else { 1 };
        let (en, eo) = (
            if near_new == 0 { cur_a } else { cur_b },
            if near_old == 0 { oa } else { ob },
        );
        if en.dist(x) <= tol && eo.dist(x) <= tol {
            let (a, b) = if near_new == 0 {
                (x, cur_b)
            } else {
                (cur_a, x)
            };
            set_line(&mut curves[new], a, b);
            let (a, b) = if near_old == 0 { (x, ob) } else { (oa, x) };
            set_line(&mut curves[k], a, b);
        }
    }
}

/// Moves every line end at `from` to `to` (dragging a sketch vertex).
pub fn move_vertex(curves: &mut [SketchCurve], from: Pt, to: Pt) {
    for c in curves.iter_mut() {
        if let SketchCurve::Line { a, b, .. } = c {
            if a.dist(from) < JOIN_TOL {
                *a = to;
            }
            if b.dist(from) < JOIN_TOL {
                *b = to;
            }
        }
    }
}

/// Flips locked lines to the wall's other face, re-joining their neighbors.
pub fn flip(doc: &Document, curves: &mut [SketchCurve], which: &[usize]) {
    for &i in which {
        let Some(SketchCurve::Line {
            a,
            b,
            wall: Some(r),
        }) = curves.get(i).cloned()
        else {
            continue;
        };
        let face = match r.face {
            WallFace::Exterior => WallFace::Interior,
            WallFace::Interior => WallFace::Exterior,
            WallFace::CoreExterior => WallFace::CoreInterior,
            WallFace::CoreInterior => WallFace::CoreExterior,
            WallFace::Centerline => WallFace::Centerline,
        };
        let nr = WallRef { face, ..r };
        let Some((p, d)) = locked_line(doc, &nr) else {
            continue;
        };
        let on = |q: Pt| p.add(d.scale(q.sub(p).dot(d)));
        let (na, nb) = (on(a), on(b));
        curves[i] = SketchCurve::Line {
            a: na,
            b: nb,
            wall: Some(nr),
        };
        // Neighbors that met the old line's ends meet the new one at their corners.
        let others: Vec<usize> = (0..curves.len()).filter(|k| *k != i).collect();
        for k in others {
            let Some((ka, kb)) = line_parts(&curves[k]) else {
                continue;
            };
            for (old_end, is_a) in [(a, true), (b, false)] {
                let hit_a = ka.dist(old_end) < JOIN_TOL;
                let hit_b = kb.dist(old_end) < JOIN_TOL;
                if !(hit_a || hit_b) {
                    continue;
                }
                if let Some(x) = line_intersection(ka, kb.sub(ka), na, d) {
                    let (ca, cb) = line_parts(&curves[i]).unwrap_or((na, nb));
                    set_line(
                        &mut curves[i],
                        if is_a { x } else { ca },
                        if is_a { cb } else { x },
                    );
                    let (ka2, kb2) = line_parts(&curves[k]).unwrap_or((ka, kb));
                    set_line(
                        &mut curves[k],
                        if hit_a { x } else { ka2 },
                        if hit_b { x } else { kb2 },
                    );
                }
            }
        }
    }
}

/// Endpoints and midpoints of the sketch, for snapping.
pub fn snap_points(curves: &[SketchCurve]) -> Vec<Pt> {
    let mut out = vec![];
    for c in curves {
        let (a, b) = c.ends();
        out.push(a);
        out.push(b);
        if let SketchCurve::Line { .. } = c {
            out.push(a.lerp(b, 0.5));
        }
        if let SketchCurve::Arc { center, .. } = c {
            out.push(*center);
        }
    }
    out
}

fn segments_cross(p1: Pt, p2: Pt, q1: Pt, q2: Pt) -> bool {
    let d = |a: Pt, b: Pt, c: Pt| b.sub(a).cross(c.sub(a));
    let (d1, d2, d3, d4) = (d(q1, q2, p1), d(q1, q2, p2), d(p1, p2, q1), d(p1, p2, q2));
    ((d1 > 1e-6 && d2 < -1e-6) || (d1 < -1e-6 && d2 > 1e-6))
        && ((d3 > 1e-6 && d4 < -1e-6) || (d3 < -1e-6 && d4 > 1e-6))
}

/// Orders the sketch into closed loops (each head to tail), or says why it can't.
pub fn loops(curves: &[SketchCurve]) -> Result<Vec<Vec<SketchCurve>>, SketchError> {
    if curves.is_empty() {
        return Err(SketchError::new(
            "The sketch is empty. Draw a closed loop of boundary lines.",
            vec![],
        ));
    }
    // Overlapping duplicates: same ends, and each one's middle lies on the other.
    let middle = |c: &SketchCurve| {
        let p = c.points();
        if p.len() == 2 {
            p[0].lerp(p[1], 0.5)
        } else {
            p[p.len() / 2]
        }
    };
    for i in 0..curves.len() {
        for j in (i + 1)..curves.len() {
            let (a, b) = curves[i].ends();
            let (c, d) = curves[j].ends();
            let same = (a.dist(c) < JOIN_TOL && b.dist(d) < JOIN_TOL)
                || (a.dist(d) < JOIN_TOL && b.dist(c) < JOIN_TOL);
            let on = curves[j].distance(middle(&curves[i])) < JOIN_TOL
                && curves[i].distance(middle(&curves[j])) < JOIN_TOL;
            if same && on {
                return Err(SketchError::new(
                    "Highlighted lines overlap. Lines may not overlap.",
                    vec![i, j],
                ));
            }
        }
    }
    // Ends: each must meet exactly one other end.
    let ends: Vec<(usize, bool, Pt)> = curves
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_circle())
        .flat_map(|(i, c)| {
            let (a, b) = c.ends();
            [(i, false, a), (i, true, b)]
        })
        .collect();
    let mut open = vec![];
    let mut branch = vec![];
    for (i, _, p) in &ends {
        let n = ends
            .iter()
            .filter(|(_, _, q)| q.dist(*p) < JOIN_TOL)
            .count()
            - 1;
        if n == 0 {
            open.push(*i);
        } else if n > 1 {
            branch.push(*i);
        }
    }
    if !open.is_empty() {
        open.dedup();
        return Err(SketchError::new(
            "Lines must be in closed loops. The highlighted lines are open on one end.",
            open,
        ));
    }
    if !branch.is_empty() {
        branch.sort_unstable();
        branch.dedup();
        return Err(SketchError::new(
            "Lines must not intersect. More than two lines meet at the highlighted ends.",
            branch,
        ));
    }
    // Crossings between curves (their tessellations), other than at shared ends.
    let polys: Vec<Vec<Pt>> = curves.iter().map(SketchCurve::points).collect();
    for i in 0..polys.len() {
        for j in (i + 1)..polys.len() {
            let hit = polys[i].windows(2).any(|s| {
                polys[j]
                    .windows(2)
                    .any(|t| segments_cross(s[0], s[1], t[0], t[1]))
            });
            if hit {
                return Err(SketchError::new(
                    "Lines must not intersect. The highlighted lines intersect.",
                    vec![i, j],
                ));
            }
        }
    }
    // Walk the loops.
    let mut used = vec![false; curves.len()];
    let mut out = vec![];
    for start in 0..curves.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        if curves[start].is_circle() {
            out.push(vec![curves[start].clone()]);
            continue;
        }
        let mut chain = vec![curves[start].clone()];
        loop {
            let tail = chain.last().map(|c| c.ends().1).unwrap_or_default();
            let head = chain[0].ends().0;
            if tail.dist(head) < JOIN_TOL && chain.len() > 1 {
                break;
            }
            let next = (0..curves.len()).find(|k| {
                !used[*k] && {
                    let (a, b) = curves[*k].ends();
                    a.dist(tail) < JOIN_TOL || b.dist(tail) < JOIN_TOL
                }
            });
            let Some(k) = next else {
                break;
            };
            used[k] = true;
            let c = &curves[k];
            chain.push(if c.ends().0.dist(tail) < JOIN_TOL {
                c.clone()
            } else {
                c.reversed()
            });
        }
        out.push(chain);
    }
    for l in &out {
        if signed_area(&loop_points(l)).abs() < 1.0 {
            return Err(SketchError::new("A loop has no area.", vec![]));
        }
    }
    Ok(out)
}

/// A loop's outline from its curves as stored.
fn loop_points(l: &[SketchCurve]) -> Vec<Pt> {
    let mut pts = vec![];
    for c in l {
        let p = c.points();
        pts.extend_from_slice(&p[..p.len() - 1]);
    }
    if l.len() == 1 && l[0].is_circle() {
        pts = l[0].points();
        pts.pop();
    }
    pts
}

/// A loop's outline now: locked lines on their walls' faces and each corner between two
/// lines at their intersection (so neighbors stretch or trim with a moved wall).
pub fn loop_polygon(doc: &Document, l: &[SketchCurve]) -> Vec<Pt> {
    let cur: Vec<SketchCurve> = l.iter().map(|c| resolve(doc, c)).collect();
    let n = cur.len();
    if n == 1 {
        let mut p = cur[0].points();
        p.pop();
        return p;
    }
    // Corner k joins curve k's end to curve k+1's start.
    let corner = |k: usize| -> Pt {
        let (c, d) = (&cur[k], &cur[(k + 1) % n]);
        if let (Some((a1, b1)), Some((a2, b2))) = (line_parts(c), line_parts(d)) {
            let (d1, d2) = (b1.sub(a1), b2.sub(a2));
            if d1.norm().cross(d2.norm()).abs() > 1e-6 {
                if let Some(x) = line_intersection(a1, d1, a2, d2) {
                    return x;
                }
            }
        }
        c.ends().1
    };
    let mut pts = vec![];
    for (k, c) in cur.iter().enumerate() {
        let start = corner((k + n - 1) % n);
        match c {
            SketchCurve::Line { .. } => pts.push(start),
            arc @ SketchCurve::Arc { .. } => {
                let p = arc.points();
                pts.extend_from_slice(&p[..p.len() - 1]);
            }
        }
    }
    pts
}

/// The loops with every locked line freed at its current place (detaching a sketch from
/// its walls).
pub fn unlock(doc: &Document, loops: &[Vec<SketchCurve>]) -> Vec<Vec<SketchCurve>> {
    loops
        .iter()
        .map(|l| {
            let cur: Vec<SketchCurve> = l.iter().map(|c| resolve(doc, c)).collect();
            let n = cur.len();
            let corner = |k: usize| -> Pt {
                let (c, d) = (&cur[k % n], &cur[(k + 1) % n]);
                if let (Some((a1, b1)), Some((a2, b2))) = (line_parts(c), line_parts(d)) {
                    let (d1, d2) = (b1.sub(a1), b2.sub(a2));
                    if d1.norm().cross(d2.norm()).abs() > 1e-6 {
                        if let Some(x) = line_intersection(a1, d1, a2, d2) {
                            return x;
                        }
                    }
                }
                c.ends().1
            };
            (0..n)
                .map(|k| match &cur[k] {
                    SketchCurve::Line { .. } if n > 1 => {
                        SketchCurve::line(corner(k + n - 1), corner(k))
                    }
                    SketchCurve::Line { a, b, .. } => SketchCurve::line(*a, *b),
                    arc => arc.clone(),
                })
                .collect()
        })
        .collect()
}

/// True when any line of the sketch is locked to a wall.
pub fn has_locked(loops: &[Vec<SketchCurve>]) -> bool {
    loops
        .iter()
        .flatten()
        .any(|c| matches!(c, SketchCurve::Line { wall: Some(_), .. }))
}

/// The floor's regions: outer loops with the loops inside them as openings.
pub fn polygons(doc: &Document, loops: &[Vec<SketchCurve>]) -> Vec<Poly> {
    let mut rings: Vec<Vec<Pt>> = loops.iter().map(|l| loop_polygon(doc, l)).collect();
    rings.retain(|r| r.len() >= 3 && signed_area(r).abs() > 1.0);
    rings.sort_by(|a, b| signed_area(b).abs().total_cmp(&signed_area(a).abs()));
    let mut polys: Vec<(usize, Poly)> = vec![];
    for (i, r) in rings.iter().enumerate() {
        let mut r = r.clone();
        let inside: Vec<usize> = (0..i).filter(|&j| point_in_ring(r[0], &rings[j])).collect();
        if inside.len().is_multiple_of(2) {
            if signed_area(&r) < 0.0 {
                r.reverse();
            }
            polys.push((i, Poly::simple(r)));
        } else if let Some(&parent) = inside
            .iter()
            .rev()
            .find(|j| polys.iter().any(|p| p.0 == **j))
        {
            if signed_area(&r) > 0.0 {
                r.reverse();
            }
            if let Some(p) = polys.iter_mut().find(|p| p.0 == parent) {
                p.1.holes.push(r);
            }
        }
    }
    polys.into_iter().map(|p| p.1).collect()
}

/// Which slab a sketch makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SketchKind {
    Floor,
    Ceiling,
}

/// The curves to edit for an existing floor or ceiling (its sketch, or its outline as
/// lines for ones made before sketches).
pub fn curves_of(
    doc: &Document,
    id: ElementId,
    outline: Option<Vec<Pt>>,
) -> CoreResult<Vec<SketchCurve>> {
    let (sketch, boundary) = match doc.data(id)? {
        ElementData::Floor {
            sketch, boundary, ..
        }
        | ElementData::Ceiling {
            sketch, boundary, ..
        } => (sketch.clone(), boundary.clone()),
        _ => return Err(CoreError::Invalid("select a floor or ceiling".into())),
    };
    if !sketch.is_empty() {
        return Ok(sketch.into_iter().flatten().collect());
    }
    let ring = outline.unwrap_or(boundary);
    let n = ring.len();
    Ok((0..n)
        .map(|i| SketchCurve::line(ring[i], ring[(i + 1) % n]))
        .collect())
}

/// Finish: checks the sketch and creates the floor or ceiling (or updates `target`'s
/// boundary), as one undoable step.
pub fn finish(
    doc: &mut Document,
    kind: SketchKind,
    target: Option<ElementId>,
    type_id: ElementId,
    level: ElementId,
    curves: &[SketchCurve],
) -> Result<ElementId, SketchError> {
    let loops = loops(curves)?;
    let polys = polygons(doc, &loops);
    let Some(outer) = polys.first().map(|p| p.outer.clone()) else {
        return Err(SketchError::new("A loop has no area.", vec![]));
    };
    let fail = |e: CoreError| SketchError::new(&e.to_string(), vec![]);
    let label = match (kind, target) {
        (SketchKind::Floor, None) => "Create floor",
        (SketchKind::Ceiling, None) => "Create ceiling",
        (_, Some(_)) => "Edit boundary",
    };
    doc.transact(label, |tx| {
        if let Some(id) = target {
            tx.modify(id, |d| match d {
                ElementData::Floor {
                    boundary,
                    bound,
                    sketch,
                    ..
                }
                | ElementData::Ceiling {
                    boundary,
                    bound,
                    sketch,
                    ..
                } => {
                    *boundary = outer.clone();
                    *bound = crate::element::SlabBound::Sketch;
                    *sketch = loops.clone();
                }
                _ => {}
            })?;
            return Ok(id);
        }
        let want = match kind {
            SketchKind::Floor => Category::FloorType,
            SketchKind::Ceiling => Category::CeilingType,
        };
        if tx.data(type_id)?.category() != want {
            return Err(CoreError::Invalid("pick a type for the sketch".into()));
        }
        Ok(tx.insert(match kind {
            SketchKind::Floor => ElementData::Floor {
                type_id,
                level,
                offset: 0.0,
                boundary: outer.clone(),
                bound: crate::element::SlabBound::Sketch,
                sketch: loops.clone(),
            },
            SketchKind::Ceiling => ElementData::Ceiling {
                type_id,
                level,
                height: crate::ops::DEFAULT_CEILING_HEIGHT,
                boundary: outer.clone(),
                bound: crate::element::SlabBound::Sketch,
                sketch: loops.clone(),
            },
        }))
    })
    .map_err(fail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use crate::units::MM_PER_FT;

    fn ft(x: f64, y: f64) -> Pt {
        Pt::new(x * MM_PER_FT, y * MM_PER_FT)
    }

    fn house() -> (Document, ElementId, Vec<ElementId>) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let c = [ft(0.0, 0.0), ft(40.0, 0.0), ft(40.0, 30.0), ft(0.0, 30.0)];
        let walls = (0..4)
            .map(|i| ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
            .collect();
        (doc, l1, walls)
    }

    #[test]
    fn draw_tools_make_revit_shapes() {
        let o = DrawOptions::default();
        let rect = draw(DrawTool::Rectangle, &[ft(0.0, 0.0), ft(10.0, 5.0)], &o).unwrap();
        assert_eq!(rect.len(), 4);
        let l = loops(&rect).unwrap();
        assert!(
            (signed_area(&loop_points(&l[0])).abs() - 50.0 * MM_PER_FT * MM_PER_FT).abs() < 1.0
        );
        let hex = draw(
            DrawTool::InscribedPolygon,
            &[ft(0.0, 0.0), ft(5.0, 0.0)],
            &o,
        )
        .unwrap();
        assert_eq!(hex.len(), 6);
        assert!((hex[0].ends().0.dist(ft(0.0, 0.0)) - 5.0 * MM_PER_FT).abs() < 1e-6);
        let circ = draw(
            DrawTool::CircumscribedPolygon,
            &[ft(0.0, 0.0), ft(5.0, 0.0)],
            &o,
        )
        .unwrap();
        // The cursor is at the middle of an edge (the apothem).
        let (a, b) = circ[0].ends();
        assert!(
            (a.lerp(b, 0.5).dist(ft(5.0, 0.0))) < 1e-6,
            "{:?}",
            a.lerp(b, 0.5)
        );
        let c = draw(DrawTool::Circle, &[ft(0.0, 0.0), ft(3.0, 4.0)], &o).unwrap();
        assert!(c[0].is_circle() && (c[0].length() - TAU * 5.0 * MM_PER_FT).abs() < 1e-6);
        let arc = draw(
            DrawTool::StartEndRadiusArc,
            &[ft(-5.0, 0.0), ft(5.0, 0.0), ft(0.0, 5.0)],
            &o,
        )
        .unwrap();
        // A half circle over the top, radius 5'.
        let SketchCurve::Arc { radius, sweep, .. } = arc[0] else {
            panic!()
        };
        assert!(
            (radius - 5.0 * MM_PER_FT).abs() < 1e-6 && (sweep + std::f64::consts::PI).abs() < 1e-6
        );
        let off = draw(
            DrawTool::Line,
            &[ft(0.0, 0.0), ft(10.0, 0.0)],
            &DrawOptions {
                offset: 1.0 * MM_PER_FT,
                ..o
            },
        )
        .unwrap();
        assert!(
            (off[0].ends().0.y - 1.0 * MM_PER_FT).abs() < 1e-6,
            "offset to the left"
        );
        let rounded = draw(
            DrawTool::Rectangle,
            &[ft(0.0, 0.0), ft(10.0, 10.0)],
            &DrawOptions {
                radius: Some(1.0 * MM_PER_FT),
                ..o
            },
        )
        .unwrap();
        assert_eq!(rounded.len(), 8, "four lines and four arcs");
        assert!(loops(&rounded).is_ok());
    }

    #[test]
    fn finishing_checks_loops_like_revit() {
        let o = DrawOptions::default();
        let mut open = draw(DrawTool::Rectangle, &[ft(0.0, 0.0), ft(10.0, 10.0)], &o).unwrap();
        open.pop();
        let e = loops(&open).unwrap_err();
        assert!(e.message.starts_with("Lines must be in closed loops"));
        assert_eq!(e.bad, [0, 2]);
        let mut crossing = draw(DrawTool::Rectangle, &[ft(0.0, 0.0), ft(10.0, 10.0)], &o).unwrap();
        crossing.extend(draw(DrawTool::Rectangle, &[ft(5.0, 5.0), ft(15.0, 15.0)], &o).unwrap());
        assert!(loops(&crossing)
            .unwrap_err()
            .message
            .contains("must not intersect"));
        let mut dup = draw(DrawTool::Rectangle, &[ft(0.0, 0.0), ft(10.0, 10.0)], &o).unwrap();
        dup.push(dup[0].clone());
        assert!(loops(&dup).unwrap_err().message.contains("overlap"));
        assert!(loops(&[]).unwrap_err().message.contains("empty"));
    }

    #[test]
    fn inner_loops_are_openings_and_separate_loops_are_pieces() {
        let (mut doc, l1, _) = house();
        let o = DrawOptions::default();
        let mut s = draw(DrawTool::Rectangle, &[ft(0.0, 0.0), ft(20.0, 20.0)], &o).unwrap();
        s.extend(draw(DrawTool::Circle, &[ft(10.0, 10.0), ft(12.0, 10.0)], &o).unwrap());
        s.extend(draw(DrawTool::Rectangle, &[ft(30.0, 0.0), ft(35.0, 5.0)], &o).unwrap());
        let ft_type = ops::first_of(&doc, Category::FloorType).unwrap();
        let id = finish(&mut doc, SketchKind::Floor, None, ft_type, l1, &s).unwrap();
        let ElementData::Floor { sketch, .. } = doc.data(id).unwrap() else {
            panic!()
        };
        let polys = polygons(&doc, sketch);
        assert_eq!(polys.len(), 2);
        assert_eq!(polys[0].holes.len(), 1, "the circle is an opening");
        let hole = std::f64::consts::PI * (2.0 * MM_PER_FT).powi(2);
        assert!((polys[0].area() - (400.0 * MM_PER_FT * MM_PER_FT - hole)).abs() / hole < 0.01);
        // Edit Boundary replaces the sketch in one step.
        let edited = draw(DrawTool::Rectangle, &[ft(0.0, 0.0), ft(8.0, 8.0)], &o).unwrap();
        finish(&mut doc, SketchKind::Floor, Some(id), ft_type, l1, &edited).unwrap();
        let ElementData::Floor { sketch, .. } = doc.data(id).unwrap() else {
            panic!()
        };
        assert_eq!(polygons(&doc, sketch).len(), 1);
        doc.undo().unwrap();
        let ElementData::Floor { sketch, .. } = doc.data(id).unwrap() else {
            panic!()
        };
        assert_eq!(polygons(&doc, sketch).len(), 2);
    }

    #[test]
    fn picked_walls_join_at_corners_and_follow_the_walls() {
        let (mut doc, l1, walls) = house();
        let mut s: Vec<SketchCurve> = vec![];
        // Pick every wall from outside (the exterior faces).
        let outside = [
            ft(20.0, -5.0),
            ft(45.0, 15.0),
            ft(20.0, 35.0),
            ft(-5.0, 15.0),
        ];
        for (w, p) in walls.iter().zip(outside) {
            s.push(pick_wall(&doc, *w, p, false, 0.0).unwrap());
            let n = s.len() - 1;
            auto_join(&mut s, n, 600.0);
        }
        let l = loops(&s).unwrap();
        let h = 4.0 * 25.4;
        let area = signed_area(&loop_polygon(&doc, &l[0])).abs();
        assert!((area - (40.0 * MM_PER_FT + 2.0 * h) * (30.0 * MM_PER_FT + 2.0 * h)).abs() < 1.0);
        // The east wall moves out 5': the east line follows, its neighbors stretch.
        crate::modify::move_elements(&mut doc, &[walls[1]], ft(5.0, 0.0)).unwrap();
        let area = signed_area(&loop_polygon(&doc, &l[0])).abs();
        assert!((area - (45.0 * MM_PER_FT + 2.0 * h) * (30.0 * MM_PER_FT + 2.0 * h)).abs() < 1.0);
        // Flip the south line to the inside face: the corners follow.
        flip(&doc, &mut s, &[0]);
        let (a, b) = s[0].ends();
        assert!((a.y - h).abs() < 1e-6 && (b.y - h).abs() < 1e-6);
        assert!(loops(&s).is_ok(), "{:?}", loops(&s).err());
        let _ = l1;
    }

    #[test]
    fn trim_fillet_and_chains() {
        let (doc, _, walls) = house();
        assert_eq!(wall_chain(&doc, walls[0]).len(), 4);
        let mut s = vec![
            SketchCurve::line(ft(0.0, 0.0), ft(8.0, 0.0)),
            SketchCurve::line(ft(10.0, 2.0), ft(10.0, 10.0)),
        ];
        trim_corner(&mut s, 0, ft(1.0, 0.0), 1, ft(10.0, 9.0)).unwrap();
        assert_eq!(s[0].ends().1, ft(10.0, 0.0));
        assert_eq!(s[1].ends().0, ft(10.0, 0.0));
        fillet(&mut s, 0, 1, 2.0 * MM_PER_FT).unwrap();
        assert_eq!(s.len(), 3);
        assert!(s[0].ends().1.dist(ft(8.0, 0.0)) < 1e-6);
        assert!(s[2].ends().0.dist(ft(10.0, 2.0)) < 1e-6);
        let SketchCurve::Arc { center, .. } = s[1] else {
            panic!()
        };
        assert!(center.dist(ft(8.0, 2.0)) < 1e-6);
        move_vertex(&mut s, ft(0.0, 0.0), ft(-1.0, 0.0));
        assert_eq!(s[0].ends().0, ft(-1.0, 0.0));
    }

    #[test]
    fn tab_picks_the_whole_chain_on_one_side() {
        let (doc, _, walls) = house();
        // Hover the south wall from inside the house.
        let lines = pick_walls(&doc, walls[0], ft(20.0, 1.0), true, false, 0.0).unwrap();
        assert_eq!(lines.len(), 4);
        let mut s = vec![];
        add_picked(&mut s, lines, 600.0);
        let l = loops(&s).unwrap();
        let h = 4.0 * 25.4;
        let area = signed_area(&loop_polygon(&doc, &l[0])).abs();
        assert!(
            (area - (40.0 * MM_PER_FT - 2.0 * h) * (30.0 * MM_PER_FT - 2.0 * h)).abs() < 1.0,
            "inside faces"
        );
    }
}
