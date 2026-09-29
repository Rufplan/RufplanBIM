//! Revit-style dimensions (ADR-040): the references a dimension can pick (wall faces,
//! centerlines and core faces, grid lines, points), dimension strings measured across
//! parallel references, linear (horizontal or vertical) dimensions, and angular dimensions
//! between two lines.

use serde::{Deserialize, Serialize};
use studio_geom::{line_intersection, project_to_segment, Pt};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Anchor, DimKind, DimRef, ElementData, ElementId, ViewKind};
use crate::ops::anchor_point;

/// Which wall line a pick inside a wall takes when the cursor isn't at a face (Revit's
/// "Prefer" option).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Prefer {
    #[default]
    WallCenterlines,
    WallFaces,
    CenterOfCore,
    FacesOfCore,
}

/// A reference under the cursor: a line (a wall face, centerline or core face, or a grid)
/// or a point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Reference {
    /// What it is, for the status bar ("Wall: exterior face", "Grid 3", "Point").
    pub label: String,
    /// On the reference, nearest the cursor.
    pub at: Pt,
    /// The reference's extent, to highlight (the same point twice for a point).
    pub from: Pt,
    pub to: Pt,
    /// The line's direction (unit); None for a point.
    pub dir: Option<Pt>,
    pub anchor: Option<Anchor>,
}

impl Reference {
    /// A point reference (a snapped endpoint or intersection).
    pub fn point(doc: &Document, view: ElementId, at: Pt) -> Reference {
        Reference {
            label: "Point".into(),
            at,
            from: at,
            to: at,
            dir: None,
            anchor: crate::ops::anchor_at(doc, view, at),
        }
    }
}

fn plan_level(doc: &Document, view: ElementId) -> Option<ElementId> {
    match doc.data(view).ok()? {
        ElementData::View {
            kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
            ..
        } => Some(*level),
        _ => None,
    }
}

/// The references within `tol` of `cursor` in a plan view, best first: the wall lines and
/// grid lines nearest it; inside a wall but at none of its lines, the preferred line first
/// and the wall's other lines after it (for Tab to cycle).
pub fn references_at(
    doc: &Document,
    view: ElementId,
    cursor: Pt,
    tol: f64,
    prefer: Prefer,
) -> Vec<Reference> {
    let Some(level) = plan_level(doc, view) else {
        return vec![];
    };
    // (rank, distance, reference): rank 0 is a line within reach, 1 a wall's preferred line
    // from inside it, 2 its other lines.
    let mut found: Vec<(u8, f64, Reference)> = vec![];
    for e in doc.iter() {
        match &e.data {
            ElementData::Wall {
                type_id,
                start,
                end,
                base_level,
                ..
            } if *base_level == level => {
                let Ok(ElementData::WallType {
                    thickness, layers, ..
                }) = doc.data(*type_id)
                else {
                    continue;
                };
                let h = thickness / 2.0;
                let len = start.dist(*end);
                if len < 1.0 {
                    continue;
                }
                let u = end.sub(*start).norm();
                let n = u.perp();
                let along = cursor.sub(*start).dot(u);
                let side = cursor.sub(*start).dot(n);
                if along < -h - tol || along > len + h + tol || side.abs() > h + tol {
                    continue;
                }
                let (ce, ci) = crate::compound::core_faces(layers, *thickness);
                let mut lines: Vec<(f64, &str, bool)> = vec![
                    (
                        h,
                        "Wall: exterior face",
                        prefer == Prefer::WallFaces && side >= 0.0,
                    ),
                    (
                        -h,
                        "Wall: interior face",
                        prefer == Prefer::WallFaces && side < 0.0,
                    ),
                    (0.0, "Wall: centerline", prefer == Prefer::WallCenterlines),
                ];
                let mid = (ce + ci) / 2.0;
                if (ce - h).abs() > 0.5 || (ci + h).abs() > 0.5 {
                    lines.push((
                        ce,
                        "Wall: core exterior face",
                        prefer == Prefer::FacesOfCore && side >= mid,
                    ));
                    lines.push((
                        ci,
                        "Wall: core interior face",
                        prefer == Prefer::FacesOfCore && side < mid,
                    ));
                } else if prefer == Prefer::FacesOfCore {
                    // A single-layer wall's core is the wall.
                    for l in &mut lines[..2] {
                        l.2 = (l.0 >= 0.0) == (side >= 0.0);
                    }
                }
                if mid.abs() > 0.5 {
                    lines.push((mid, "Wall: core centerline", prefer == Prefer::CenterOfCore));
                } else if prefer == Prefer::CenterOfCore {
                    lines[2].2 = true;
                }
                let inside = side.abs() <= h;
                let t = along.clamp(0.0, len);
                for (s, label, preferred) in lines {
                    let d = (side - s).abs();
                    let rank = if d <= tol {
                        0
                    } else if inside && preferred {
                        1
                    } else if inside {
                        2
                    } else {
                        continue;
                    };
                    let off = n.scale(s);
                    found.push((
                        rank,
                        d,
                        Reference {
                            label: label.into(),
                            at: start.add(u.scale(t)).add(off),
                            from: start.add(off),
                            to: end.add(off),
                            dir: Some(u),
                            anchor: Some(Anchor::Wall {
                                wall: e.id,
                                t: t / len,
                                side: s,
                            }),
                        },
                    ));
                }
            }
            ElementData::Grid {
                name, start, end, ..
            } => {
                let (t, d) = project_to_segment(cursor, *start, *end);
                if d <= tol && start.dist(*end) > 1.0 {
                    found.push((
                        0,
                        d,
                        Reference {
                            label: format!("Grid {name}"),
                            at: start.lerp(*end, t),
                            from: *start,
                            to: *end,
                            dir: Some(end.sub(*start).norm()),
                            anchor: Some(Anchor::Grid { grid: e.id, t }),
                        },
                    ));
                }
            }
            _ => {}
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    found.into_iter().map(|f| f.2).collect()
}

/// The direction of the line an anchor sits on, now (unit), if its element is still there.
pub fn anchor_dir(doc: &Document, anchor: &Anchor) -> Option<Pt> {
    match anchor {
        Anchor::Wall { wall, .. } => match doc.data(*wall).ok()? {
            ElementData::Wall { start, end, .. } => Some(end.sub(*start).norm()),
            _ => None,
        },
        Anchor::Grid { grid, .. } => match doc.data(*grid).ok()? {
            ElementData::Grid { start, end, .. } => Some(end.sub(*start).norm()),
            _ => None,
        },
        Anchor::DetailLine { line, .. } => match doc.data(*line).ok()? {
            ElementData::DetailLine {
                curve: crate::sketch::SketchCurve::Line { a, b, .. },
                ..
            } => Some(b.sub(*a).norm()),
            _ => None,
        },
        // A point on a component: no line of its own.
        Anchor::Component { .. } => None,
    }
}

/// A dimension string laid out from picked references: the references in order along it,
/// the direction it measures along, whether that direction is fixed (else from the first to
/// the last), and the dimension line's offset through `cursor`.
pub struct StringPlan {
    /// (where, anchor, picked as a point rather than a line).
    pub refs: Vec<(Pt, Option<Anchor>, bool)>,
    pub along: Pt,
    pub fixed: bool,
    pub offset: f64,
}

/// Lays out a dimension string. Aligned: across parallel lines (every line picked must be
/// parallel to the first), else point to point. Linear: horizontal or vertical, whichever
/// the cursor is beyond (above or below the points: horizontal).
pub fn plan_string(refs: &[Reference], cursor: Pt, kind: DimKind) -> CoreResult<StringPlan> {
    if refs.len() < 2 {
        return Err(CoreError::Invalid("pick at least two references".into()));
    }
    let (along, fixed) = match kind {
        DimKind::Linear => {
            let (mut lo, mut hi) = (refs[0].at, refs[0].at);
            for r in refs {
                lo = Pt::new(lo.x.min(r.at.x), lo.y.min(r.at.y));
                hi = Pt::new(hi.x.max(r.at.x), hi.y.max(r.at.y));
            }
            let out_x = (lo.x - cursor.x).max(cursor.x - hi.x).max(0.0);
            let out_y = (lo.y - cursor.y).max(cursor.y - hi.y).max(0.0);
            let horizontal = out_y > out_x || (out_y == out_x && hi.x - lo.x >= hi.y - lo.y);
            (
                if horizontal {
                    Pt::new(1.0, 0.0)
                } else {
                    Pt::new(0.0, 1.0)
                },
                true,
            )
        }
        DimKind::Aligned => match refs.iter().find_map(|r| r.dir) {
            Some(d) => {
                // Across the lines: 1° off parallel is too far.
                let parallel = refs
                    .iter()
                    .filter_map(|r| r.dir)
                    .all(|e| e.cross(d).abs() < 0.0175);
                if !parallel {
                    return Err(CoreError::Invalid(
                        "pick references parallel to the first".into(),
                    ));
                }
                (d.perp(), true)
            }
            None => {
                let (a, b) = (refs[0].at, refs[refs.len() - 1].at);
                if a.dist(b) < 1.0 {
                    return Err(CoreError::Invalid("pick two different points".into()));
                }
                (b.sub(a).norm(), refs.len() > 2)
            }
        },
    };
    let origin = refs[0].at;
    let mut sorted: Vec<(f64, Pt, Option<Anchor>, bool)> = refs
        .iter()
        .map(|r| (r.at.sub(origin).dot(along), r.at, r.anchor, r.dir.is_none()))
        .collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    sorted.dedup_by(|x, y| (x.0 - y.0).abs() < 0.5);
    if sorted.len() < 2 {
        return Err(CoreError::Invalid(
            "those references are in the same place".into(),
        ));
    }
    let first = sorted[0].1;
    Ok(StringPlan {
        refs: sorted.into_iter().map(|s| (s.1, s.2, s.3)).collect(),
        along,
        fixed,
        offset: cursor.sub(first).dot(along.perp()),
    })
}

/// Places an aligned or linear dimension (a string when more than two references are
/// picked) with its line through `cursor`.
pub fn create_string(
    doc: &mut Document,
    view: ElementId,
    refs: &[Reference],
    cursor: Pt,
    kind: DimKind,
) -> CoreResult<ElementId> {
    let plan = plan_string(refs, cursor, kind)?;
    // A picked point follows the wall or grid that crosses the dimension there (Revit's
    // references), not one running along it, whose stretching would slide it.
    let refs: Vec<(Pt, Option<Anchor>)> = plan
        .refs
        .iter()
        .map(|(at, anchor, point)| {
            (
                *at,
                if *point {
                    anchor_across(doc, view, *at, plan.along)
                } else {
                    *anchor
                },
            )
        })
        .collect();
    let n = refs.len();
    let (a, a_ref) = refs[0];
    let (b, b_ref) = refs[n - 1];
    let between = refs[1..n - 1]
        .iter()
        .map(|(at, anchor)| DimRef {
            at: *at,
            anchor: *anchor,
        })
        .collect();
    doc.transact("Place dimension", |tx| {
        Ok(tx.insert(ElementData::Dimension {
            view,
            a,
            b,
            offset: plan.offset,
            a_ref,
            b_ref,
            between,
            along: plan.fixed.then_some(plan.along),
            kind,
        }))
    })
}

/// A dimension's references where they are now (first, between…, last), and the
/// direction it measures along.
pub fn string_points(doc: &Document, data: &ElementData) -> Option<(Vec<Pt>, Pt)> {
    let ElementData::Dimension { between, along, .. } = data else {
        return None;
    };
    let (a, b) = crate::ops::dimension_ends(doc, data)?;
    let mut pts = vec![a];
    pts.extend(between.iter().map(|r| {
        r.anchor
            .as_ref()
            .and_then(|x| anchor_point(doc, x))
            .unwrap_or(r.at)
    }));
    pts.push(b);
    let u = match along {
        Some(u) => *u,
        None if a.dist(b) > 1e-9 => b.sub(a).norm(),
        None => Pt::new(1.0, 0.0),
    };
    Some((pts, u))
}

/// The lengths of a dimension's segments (mm), in order.
pub fn segments(pts: &[Pt], u: Pt) -> Vec<f64> {
    pts.windows(2)
        .map(|w| w[1].sub(w[0]).dot(u).abs())
        .collect()
}

/// An angular dimension's arc: centered where the lines meet, from ray `from` to ray `to`
/// (units, the arc sweeping `sweep` radians, counter-clockwise when positive), and where
/// each line's reference is.
#[derive(Debug, Clone, PartialEq)]
pub struct AngleArc {
    pub center: Pt,
    pub radius: f64,
    pub from: Pt,
    pub to: Pt,
    pub sweep: f64,
    pub refs: [Pt; 2],
}

impl AngleArc {
    /// The angle it measures, degrees.
    pub fn degrees(&self) -> f64 {
        self.sweep.abs().to_degrees()
    }
}

/// The arc between lines (a, da) and (b, db) through `at`: in the angle between them that
/// holds `at`. None for parallel lines.
pub fn angle_arc(a: Pt, da: Pt, b: Pt, db: Pt, at: Pt) -> Option<AngleArc> {
    let center = line_intersection(a, da, b, db)?;
    let radius = at.dist(center);
    if radius < 1e-6 {
        return None;
    }
    let w = at.sub(center).norm();
    for ra in [da, da.scale(-1.0)] {
        for rb in [db, db.scale(-1.0)] {
            let s = ra.cross(rb);
            if s.abs() < 1e-9 {
                continue;
            }
            if ra.cross(w) * s >= 0.0 && w.cross(rb) * s >= 0.0 {
                let sweep = s.abs().atan2(ra.dot(rb)) * s.signum();
                return Some(AngleArc {
                    center,
                    radius,
                    from: ra,
                    to: rb,
                    sweep,
                    refs: [a, b],
                });
            }
        }
    }
    None
}

/// An angular dimension's arc now: its lines follow their elements.
pub fn angular_arc(doc: &Document, data: &ElementData) -> Option<AngleArc> {
    let ElementData::AngularDimension {
        a,
        a_dir,
        a_ref,
        b,
        b_dir,
        b_ref,
        at,
        ..
    } = data
    else {
        return None;
    };
    let line = |p: &Pt, d: &Pt, r: &Option<Anchor>| {
        let Some(x) = r else { return (*p, *d) };
        (
            anchor_point(doc, x).unwrap_or(*p),
            anchor_dir(doc, x).unwrap_or(*d),
        )
    };
    let (pa, da) = line(a, a_dir, a_ref);
    let (pb, db) = line(b, b_dir, b_ref);
    angle_arc(pa, da, pb, db, *at)
}

/// Places an angular dimension between two picked lines, its arc through `cursor`.
pub fn create_angular(
    doc: &mut Document,
    view: ElementId,
    first: &Reference,
    second: &Reference,
    cursor: Pt,
) -> CoreResult<ElementId> {
    let (Some(da), Some(db)) = (first.dir, second.dir) else {
        return Err(CoreError::Invalid("pick two lines (walls or grids)".into()));
    };
    if da.cross(db).abs() < 0.0175 {
        return Err(CoreError::Invalid("those lines are parallel".into()));
    }
    angle_arc(first.at, da, second.at, db, cursor)
        .ok_or_else(|| CoreError::Invalid("move the arc away from the corner".into()))?;
    doc.transact("Place angular dimension", |tx| {
        Ok(tx.insert(ElementData::AngularDimension {
            view,
            a: first.at,
            a_dir: da,
            a_ref: first.anchor,
            b: second.at,
            b_dir: db,
            b_ref: second.anchor,
            at: cursor,
        }))
    })
}

/// What a dimension point at `p` measuring along `u` follows: the wall line (a face or the
/// centerline) or grid through it that crosses the dimension; else a wall end it sits at.
/// A point partway along a wall that runs with the dimension follows nothing (it would
/// slide as that wall stretches).
pub fn anchor_across(doc: &Document, view: ElementId, p: Pt, u: Pt) -> Option<Anchor> {
    let level = plan_level(doc, view)?;
    let slop = 1.0;
    let mut best: Option<(f64, Anchor)> = None;
    for e in doc.iter() {
        match &e.data {
            ElementData::Wall {
                type_id,
                start,
                end,
                base_level,
                ..
            } if *base_level == level => {
                let Ok(ElementData::WallType { thickness, .. }) = doc.data(*type_id) else {
                    continue;
                };
                let len = start.dist(*end);
                if len < 1.0 {
                    continue;
                }
                let d = end.sub(*start).norm();
                if d.dot(u).abs() > 0.02 {
                    continue; // Doesn't cross the dimension.
                }
                let h = thickness / 2.0;
                let along = p.sub(*start).dot(d);
                let side = p.sub(*start).dot(d.perp());
                if along < -h - slop || along > len + h + slop {
                    continue;
                }
                let dev = [-h, 0.0, h]
                    .iter()
                    .map(|f| (side - f).abs())
                    .fold(f64::INFINITY, f64::min);
                if dev <= slop && best.as_ref().is_none_or(|b| dev < b.0) {
                    best = Some((
                        dev,
                        Anchor::Wall {
                            wall: e.id,
                            t: along / len,
                            side,
                        },
                    ));
                }
            }
            ElementData::Grid { start, end, .. } => {
                let d = end.sub(*start).norm();
                let (t, dist) = project_to_segment(p, *start, *end);
                if d.dot(u).abs() <= 0.02
                    && dist <= slop
                    && best.as_ref().is_none_or(|b| dist < b.0)
                {
                    best = Some((dist, Anchor::Grid { grid: e.id, t }));
                }
            }
            _ => {}
        }
    }
    if let Some((_, a)) = best {
        return Some(a);
    }
    // At a wall's end (its end face), the end follows the wall.
    match crate::ops::anchor_at(doc, view, p) {
        Some(Anchor::Wall { wall, t, side }) => {
            let (len, h) = match (doc.data(wall).ok()?, ()) {
                (
                    ElementData::Wall {
                        start,
                        end,
                        type_id,
                        ..
                    },
                    (),
                ) => (
                    start.dist(*end),
                    match doc.data(*type_id).ok()? {
                        ElementData::WallType { thickness, .. } => thickness / 2.0,
                        _ => 0.0,
                    },
                ),
                _ => return None,
            };
            let along = t * len;
            (along <= h + slop || along >= len - h - slop).then_some(Anchor::Wall { wall, t, side })
        }
        other => other,
    }
}

/// Align (AL), as Revit's: moves the element `target` is on (a wall or grid) square to
/// itself so that line lies on `reference`. The lines must be parallel.
pub fn align(doc: &mut Document, reference: &Reference, target: &Reference) -> CoreResult<()> {
    let (Some(dr), Some(dt)) = (reference.dir, target.dir) else {
        return Err(CoreError::Invalid(
            "pick lines: wall faces, centerlines or grids".into(),
        ));
    };
    let id = match target.anchor {
        Some(Anchor::Wall { wall, .. }) => wall,
        Some(Anchor::Grid { grid, .. }) => grid,
        Some(Anchor::DetailLine { line, .. }) => line,
        Some(Anchor::Component { component, .. }) => component,
        None => {
            return Err(CoreError::Invalid(
                "pick a line on the element to align".into(),
            ))
        }
    };
    if anchored_to(&reference.anchor, id) {
        return Err(CoreError::Invalid(
            "pick a line on another element to align to the reference".into(),
        ));
    }
    if dr.cross(dt).abs() > 0.0175 {
        return Err(CoreError::Invalid("those lines aren't parallel".into()));
    }
    let n = dr.perp();
    let delta = n.scale(reference.at.sub(target.at).dot(n));
    crate::modify::move_elements(doc, &[id], delta)
}

/// A wall's centerline or a grid's line, for the temporary and permanent dimensions that
/// move them (ADR-041).
pub fn line_of(doc: &Document, id: ElementId) -> Option<(Pt, Pt)> {
    match doc.data(id).ok()? {
        ElementData::Wall { start, end, .. } | ElementData::Grid { start, end, .. } => {
            Some((*start, *end))
        }
        _ => None,
    }
}

/// Whether an anchor sits on element `id` (a wall or a grid).
pub fn anchored_to(anchor: &Option<Anchor>, id: ElementId) -> bool {
    matches!(anchor, Some(Anchor::Wall { wall, .. }) if *wall == id)
        || matches!(anchor, Some(Anchor::Grid { grid, .. }) if *grid == id)
        || matches!(anchor, Some(Anchor::DetailLine { line, .. }) if *line == id)
        || matches!(anchor, Some(Anchor::Component { component, .. }) if *component == id)
}

/// A dimension's anchors in order along it (first, between…, last).
pub fn string_anchors(data: &ElementData) -> Vec<Option<Anchor>> {
    match data {
        ElementData::Dimension {
            a_ref,
            b_ref,
            between,
            ..
        } => {
            let mut v = vec![*a_ref];
            v.extend(between.iter().map(|r| r.anchor));
            v.push(*b_ref);
            v
        }
        _ => vec![],
    }
}

/// Typing a temporary dimension to a parallel wall or grid (Revit's): moves wall or grid
/// `id` square to itself so its line is `mm` from `other`'s, on the side it is on now.
/// Walls joined to it stretch to follow.
pub fn set_distance_to(
    doc: &mut Document,
    id: ElementId,
    other: ElementId,
    mm: f64,
) -> CoreResult<()> {
    let bad = || CoreError::Invalid("that dimension is gone".into());
    let (a, b) = line_of(doc, id).ok_or_else(bad)?;
    let (c, _) = line_of(doc, other).ok_or_else(bad)?;
    if mm < 0.0 {
        return Err(CoreError::Invalid("type a positive distance".into()));
    }
    let n = b.sub(a).norm().perp();
    let s = c.sub(a).dot(n);
    let side = if s < 0.0 { -1.0 } else { 1.0 };
    crate::modify::move_elements(doc, &[id], n.scale(s - side * mm))
}

/// Typing a segment of a permanent dimension while its wall or grid is selected (Revit's):
/// moves element `id` along the dimension so segment `seg` measures `mm`. The segment's
/// other end stays put.
pub fn set_dimension_segment(
    doc: &mut Document,
    id: ElementId,
    dim: ElementId,
    seg: usize,
    mm: f64,
) -> CoreResult<()> {
    let data = doc.data(dim)?.clone();
    let (pts, u) = string_points(doc, &data)
        .ok_or_else(|| CoreError::Invalid("that isn't a dimension".into()))?;
    let anchors = string_anchors(&data);
    if seg + 1 >= pts.len() || anchors.len() != pts.len() {
        return Err(CoreError::Invalid(
            "that dimension has no such segment".into(),
        ));
    }
    if mm < 0.0 {
        return Err(CoreError::Invalid("type a positive distance".into()));
    }
    let now = pts[seg + 1].sub(pts[seg]).dot(u);
    let dir = if now < 0.0 { -1.0 } else { 1.0 };
    let (near, far) = (
        anchored_to(&anchors[seg], id),
        anchored_to(&anchors[seg + 1], id),
    );
    let delta = match (near, far) {
        (false, true) => u.scale(dir * mm - now),
        (true, false) => u.scale(now - dir * mm),
        (true, true) => {
            return Err(CoreError::Invalid(
                "both ends are on that element; change its thickness instead".into(),
            ))
        }
        (false, false) => {
            return Err(CoreError::Invalid(
                "select the wall or grid that segment ends on".into(),
            ))
        }
    };
    crate::modify::move_elements(doc, &[id], delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use crate::units::MM_PER_FT;
    use crate::Category;

    fn plan_with_walls() -> (Document, ElementId, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let ft = MM_PER_FT;
        // Two parallel walls 20' apart (centerlines), running north.
        let w1 =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(0.0, 30.0 * ft)).unwrap();
        let w2 = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(20.0 * ft, 0.0),
            Pt::new(20.0 * ft, 30.0 * ft),
        )
        .unwrap();
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        (doc, plan, w1, w2)
    }

    #[test]
    fn walls_offer_faces_and_centerlines_and_prefer_decides_inside() {
        let (doc, plan, w1, _) = plan_with_walls();
        let h = 4.0 * 25.4;
        // Near the exterior (left, -x) face: the face first.
        let near = references_at(
            &doc,
            plan,
            Pt::new(-h + 5.0, 3000.0),
            20.0,
            Prefer::WallCenterlines,
        );
        assert_eq!(near[0].label, "Wall: exterior face");
        assert!((near[0].at.x + h).abs() < 1e-9);
        // In the middle of the wall, off every line: the preferred line.
        let mid = Pt::new(40.0, 3000.0);
        let c = references_at(&doc, plan, mid, 20.0, Prefer::WallCenterlines);
        assert_eq!(c[0].label, "Wall: centerline");
        assert!(
            matches!(c[0].anchor, Some(Anchor::Wall { wall, side, .. }) if wall == w1 && side == 0.0)
        );
        let f = references_at(&doc, plan, mid, 20.0, Prefer::WallFaces);
        assert!(
            f[0].label.contains("face") && (f[0].at.x - (-h)).abs() < 1e-9
                || (f[0].at.x - h).abs() < 1e-9
        );
        // The other lines follow, for Tab.
        assert!(c.len() >= 3);
        // Far from everything: nothing.
        assert!(references_at(
            &doc,
            plan,
            Pt::new(3000.0, 3000.0),
            20.0,
            Prefer::WallCenterlines
        )
        .is_empty());
    }

    #[test]
    fn a_grid_line_is_a_reference() {
        let (mut doc, plan, _, _) = plan_with_walls();
        let g = ops::create_grid(
            &mut doc,
            Pt::new(10.0 * MM_PER_FT, -2000.0),
            Pt::new(10.0 * MM_PER_FT, 12000.0),
        )
        .unwrap();
        let r = references_at(
            &doc,
            plan,
            Pt::new(10.0 * MM_PER_FT + 8.0, 500.0),
            20.0,
            Prefer::WallCenterlines,
        );
        assert!(r[0].label.starts_with("Grid"));
        assert!(matches!(r[0].anchor, Some(Anchor::Grid { grid, .. }) if grid == g));
    }

    #[test]
    fn an_aligned_string_measures_across_parallel_walls_and_follows_them() {
        let (mut doc, plan, w1, w2) = plan_with_walls();
        let ft = MM_PER_FT;
        let h = 4.0 * 25.4;
        let pick = |doc: &Document, x: f64| {
            references_at(doc, plan, Pt::new(x, 3000.0), 20.0, Prefer::WallCenterlines)[0].clone()
        };
        // Centerline of wall 1, exterior face of wall 2 (its left looking north is -x: its
        // interior face is +x... pick the face at x = 20' - h), and centerline of wall 2.
        let refs = vec![
            pick(&doc, 0.0 + 3.0),
            pick(&doc, 20.0 * ft + 3.0),
            pick(&doc, 20.0 * ft - h + 3.0),
        ];
        let d = create_string(
            &mut doc,
            plan,
            &refs,
            Pt::new(10.0 * ft, 5000.0),
            DimKind::Aligned,
        )
        .unwrap();
        let data = doc.data(d).unwrap().clone();
        let (pts, u) = string_points(&doc, &data).unwrap();
        assert_eq!(pts.len(), 3, "a string of three references, in order");
        assert!(
            (u.x.abs() - 1.0).abs() < 1e-9,
            "measured across the walls (east-west)"
        );
        // In order along it: the face 4" from the centerline, then 20' less 4" to wall 1.
        let mut seg = segments(&pts, u);
        seg.sort_by(f64::total_cmp);
        assert!((seg[0] - h).abs() < 1e-6, "{seg:?}");
        assert!((seg[1] - (20.0 * ft - h)).abs() < 1e-6, "{seg:?}");
        // Moving wall 2 east 2' moves both of its references.
        crate::modify::move_elements(&mut doc, &[w2], Pt::new(2.0 * ft, 0.0)).unwrap();
        let (pts, u) = string_points(&doc, doc.data(d).unwrap()).unwrap();
        let mut seg = segments(&pts, u);
        seg.sort_by(f64::total_cmp);
        assert!((seg[1] - (22.0 * ft - h)).abs() < 1e-6, "{seg:?}");
        let _ = w1;
    }

    #[test]
    fn references_must_be_parallel_for_an_aligned_string() {
        let (mut doc, plan, _, _) = plan_with_walls();
        let g =
            ops::create_grid(&mut doc, Pt::new(-5000.0, 1000.0), Pt::new(15000.0, 1000.0)).unwrap();
        let _ = g;
        let a = references_at(
            &doc,
            plan,
            Pt::new(3.0, 3000.0),
            20.0,
            Prefer::WallCenterlines,
        )[0]
        .clone();
        let b = references_at(
            &doc,
            plan,
            Pt::new(3000.0, 1005.0),
            20.0,
            Prefer::WallCenterlines,
        )[0]
        .clone();
        assert!(plan_string(&[a, b], Pt::new(0.0, 0.0), DimKind::Aligned).is_err());
    }

    #[test]
    fn a_linear_dimension_is_horizontal_or_vertical_by_the_cursor() {
        let r = |x: f64, y: f64| Reference {
            label: "Point".into(),
            at: Pt::new(x, y),
            from: Pt::new(x, y),
            to: Pt::new(x, y),
            dir: None,
            anchor: None,
        };
        let pts = [r(0.0, 0.0), r(3000.0, 4000.0)];
        // Above the points: horizontal, 3000 long.
        let h = plan_string(&pts, Pt::new(1500.0, 6000.0), DimKind::Linear).unwrap();
        assert_eq!(h.along, Pt::new(1.0, 0.0));
        assert!((segments(&[pts[0].at, pts[1].at], h.along)[0] - 3000.0).abs() < 1e-9);
        // Beside them: vertical, 4000 long.
        let v = plan_string(&pts, Pt::new(-2000.0, 2000.0), DimKind::Linear).unwrap();
        assert_eq!(v.along, Pt::new(0.0, 1.0));
        assert!((segments(&[pts[0].at, pts[1].at], v.along)[0] - 4000.0).abs() < 1e-9);
        // Aligned between the same points: 5000, point to point.
        let a = plan_string(&pts, Pt::new(0.0, 3000.0), DimKind::Aligned).unwrap();
        assert!((segments(&[pts[0].at, pts[1].at], a.along)[0] - 5000.0).abs() < 1e-9);
        assert!(!a.fixed);
    }

    #[test]
    fn an_angular_dimension_measures_the_angle_that_holds_the_arc() {
        // Lines along +x and at 30° through the origin.
        let (da, db) = (
            Pt::new(1.0, 0.0),
            Pt::new(30f64.to_radians().cos(), 30f64.to_radians().sin()),
        );
        let inside = angle_arc(
            Pt::new(5.0, 0.0),
            da,
            Pt::new(0.0, 0.0),
            db,
            Pt::new(10.0, 2.0),
        )
        .unwrap();
        assert!((inside.degrees() - 30.0).abs() < 1e-9);
        assert!((inside.radius - Pt::new(10.0, 2.0).dist(Pt::new(0.0, 0.0))).abs() < 1e-9);
        // On the other side of the first line: the supplementary angle.
        let outside = angle_arc(
            Pt::new(5.0, 0.0),
            da,
            Pt::new(0.0, 0.0),
            db,
            Pt::new(-10.0, 3.0),
        )
        .unwrap();
        assert!((outside.degrees() - 150.0).abs() < 1e-9);
        // Parallel lines have no angle.
        assert!(angle_arc(
            Pt::new(0.0, 0.0),
            da,
            Pt::new(0.0, 5.0),
            da,
            Pt::new(1.0, 1.0)
        )
        .is_none());
    }

    #[test]
    fn an_angular_dimension_between_walls_follows_them() {
        let (mut doc, plan, _, _) = plan_with_walls();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        // A wall at 45° from wall 1's south end.
        let w3 =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(5000.0, 5000.0)).unwrap();
        let _ = w3;
        let a = references_at(
            &doc,
            plan,
            Pt::new(2.0, 4000.0),
            20.0,
            Prefer::WallCenterlines,
        )[0]
        .clone();
        let b = references_at(
            &doc,
            plan,
            Pt::new(3000.0, 3002.0),
            20.0,
            Prefer::WallCenterlines,
        )[0]
        .clone();
        let d = create_angular(&mut doc, plan, &a, &b, Pt::new(800.0, 2000.0)).unwrap();
        let arc = angular_arc(&doc, doc.data(d).unwrap()).unwrap();
        assert!((arc.degrees() - 45.0).abs() < 1e-6, "{}", arc.degrees());
        // Parallel picks are refused.
        let c = references_at(
            &doc,
            plan,
            Pt::new(20.0 * MM_PER_FT + 2.0, 4000.0),
            20.0,
            Prefer::WallCenterlines,
        )[0]
        .clone();
        assert!(create_angular(&mut doc, plan, &a, &c, Pt::new(800.0, 2000.0)).is_err());
    }

    #[test]
    fn copying_walls_with_their_string_keeps_every_reference_on_the_copies() {
        let (mut doc, plan, w1, w2) = plan_with_walls();
        let ft = MM_PER_FT;
        let pick = |doc: &Document, x: f64| {
            references_at(doc, plan, Pt::new(x, 3000.0), 20.0, Prefer::WallCenterlines)[0].clone()
        };
        let h = 4.0 * 25.4;
        let refs = vec![
            pick(&doc, 3.0),
            pick(&doc, 20.0 * ft - h + 3.0),
            pick(&doc, 20.0 * ft + 3.0),
        ];
        let d = create_string(
            &mut doc,
            plan,
            &refs,
            Pt::new(10.0 * ft, 5000.0),
            DimKind::Aligned,
        )
        .unwrap();
        let copies = crate::edit::copy_elements(
            &mut doc,
            &[w1, w2, d],
            &[crate::edit::Xform::translate(Pt::new(0.0, 40.0 * ft))],
            "Copy",
        )
        .unwrap();
        let dc = *copies
            .iter()
            .find(|id| {
                matches!(
                    doc.data(**id),
                    Ok(ElementData::AngularDimension { .. }) | Ok(ElementData::Dimension { .. })
                )
            })
            .unwrap();
        let data = doc.data(dc).unwrap().clone();
        let anchors: Vec<_> = data
            .dimension_anchors()
            .into_iter()
            .flatten()
            .copied()
            .collect();
        assert_eq!(anchors.len(), 3, "all three references stay attached");
        for a in anchors {
            let Anchor::Wall { wall, .. } = a else {
                panic!("{a:?}")
            };
            assert!(wall != w1 && wall != w2, "on the copied walls");
        }
        let (pts, u) = string_points(&doc, &data).unwrap();
        let mut seg = segments(&pts, u);
        seg.sort_by(f64::total_cmp);
        assert!(
            (seg[0] - h).abs() < 1e-6 && (seg[1] - (20.0 * ft - h)).abs() < 1e-6,
            "{seg:?}"
        );
    }

    #[test]
    fn typing_a_distance_moves_the_selected_wall() {
        let (mut doc, plan, w1, w2) = plan_with_walls();
        let ft = MM_PER_FT;
        // Wall 2 is 20' east of wall 1: make it 15'.
        set_distance_to(&mut doc, w2, w1, 15.0 * ft).unwrap();
        let (a, _) = line_of(&doc, w2).unwrap();
        assert!((a.x - 15.0 * ft).abs() < 1e-6, "{a:?}");
        // A permanent string across both walls: typing its segment moves the selected wall.
        let pick = |doc: &Document, x: f64| {
            references_at(doc, plan, Pt::new(x, 3000.0), 20.0, Prefer::WallCenterlines)[0].clone()
        };
        let refs = vec![pick(&doc, 3.0), pick(&doc, 15.0 * ft + 3.0)];
        let d = create_string(
            &mut doc,
            plan,
            &refs,
            Pt::new(0.0, -2000.0),
            DimKind::Aligned,
        )
        .unwrap();
        set_dimension_segment(&mut doc, w2, d, 0, 12.0 * ft).unwrap();
        let (a, _) = line_of(&doc, w2).unwrap();
        assert!((a.x - 12.0 * ft).abs() < 1e-6, "{a:?}");
        let (pts, u) = string_points(&doc, doc.data(d).unwrap()).unwrap();
        assert!((segments(&pts, u)[0] - 12.0 * ft).abs() < 1e-6);
        // Selecting wall 1 instead moves wall 1, keeping wall 2.
        set_dimension_segment(&mut doc, w1, d, 0, 10.0 * ft).unwrap();
        let (a1, _) = line_of(&doc, w1).unwrap();
        assert!((a1.x - 2.0 * ft).abs() < 1e-6, "{a1:?}");
        assert!((line_of(&doc, w2).unwrap().0.x - 12.0 * ft).abs() < 1e-6);
        // An element the segment doesn't end on is refused.
        let (wt, l1) = (
            ops::first_of(&doc, Category::WallType).unwrap(),
            doc.levels()[0].0,
        );
        let other = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 50.0 * ft),
            Pt::new(5.0 * ft, 50.0 * ft),
        )
        .unwrap();
        assert!(set_dimension_segment(&mut doc, other, d, 0, 5.0 * ft).is_err());
    }

    #[test]
    fn align_moves_the_second_picks_element_onto_the_reference() {
        let (mut doc, plan, w1, w2) = plan_with_walls();
        let ft = MM_PER_FT;
        let h = 4.0 * 25.4;
        // Align defaults to faces: wall 1's east face (x = h), wall 2's west face.
        let reference = references_at(
            &doc,
            plan,
            Pt::new(h - 3.0, 3000.0),
            20.0,
            Prefer::WallFaces,
        )[0]
        .clone();
        let target = references_at(
            &doc,
            plan,
            Pt::new(20.0 * ft - h + 3.0, 3000.0),
            20.0,
            Prefer::WallFaces,
        )[0]
        .clone();
        assert!(reference.label.contains("face") && target.label.contains("face"));
        align(&mut doc, &reference, &target).unwrap();
        // Wall 2's west face now lies on wall 1's east face: centerlines 2h apart.
        let (a2, _) = line_of(&doc, w2).unwrap();
        assert!((a2.x - 2.0 * h).abs() < 1e-6, "{a2:?}");
        assert!(
            line_of(&doc, w1).unwrap().0.x.abs() < 1e-9,
            "the reference stays"
        );
        // Its own lines can't be the target.
        let again = references_at(
            &doc,
            plan,
            Pt::new(-h + 3.0, 3000.0),
            20.0,
            Prefer::WallFaces,
        )[0]
        .clone();
        assert!(align(&mut doc, &reference, &again).is_err());
    }

    #[test]
    fn a_dimension_point_follows_the_wall_that_crosses_it() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let ft = MM_PER_FT;
        // A north wall and an interior wall meeting it in a T at x = 16'.
        let north = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 30.0 * ft),
            Pt::new(40.0 * ft, 30.0 * ft),
        )
        .unwrap();
        let inner = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(16.0 * ft, 0.0),
            Pt::new(16.0 * ft, 30.0 * ft),
        )
        .unwrap();
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        // A two-point dimension from the north wall's start to the T.
        let d = ops::create_dimension(
            &mut doc,
            plan,
            Pt::new(0.0, 30.0 * ft),
            Pt::new(16.0 * ft, 30.0 * ft),
            900.0,
        )
        .unwrap();
        // The T end follows the interior wall (which crosses the dimension), not the north wall.
        let anchors = string_anchors(doc.data(d).unwrap());
        assert!(anchored_to(&anchors[1], inner), "{anchors:?}");
        // Stretching the north wall doesn't slide it.
        crate::edit::drag_handle(&mut doc, north, "end", Pt::new(50.0 * ft, 30.0 * ft)).unwrap();
        let (pts, u) = string_points(&doc, doc.data(d).unwrap()).unwrap();
        assert!((segments(&pts, u)[0] - 16.0 * ft).abs() < 1e-6, "{pts:?}");
    }
}
