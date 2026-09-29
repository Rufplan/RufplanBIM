//! Editing aids for the selection (ADR-017): grips to drag, temporary dimensions to type
//! into, reference lines for Align, and the Offset tool's preview. Computed here so the
//! UI only draws what it's given and sends back the handle's key.

use serde::Serialize;
use studio_core::units::format_ft_in;
use studio_core::{Category, Document, ElementData, ElementId, ViewKind};
use studio_geom::{project_to_segment, Pt};
use studio_regen::regenerate;
use ts_rs::TS;

use crate::{dimension, Builder, Dash, Item, Prim, PLAN_CUT, RCP_CUT};

/// A draggable point. Dragging it calls `drag_handle(id, key, to)`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Grip {
    pub id: ElementId,
    pub key: String,
    pub at: Pt,
    /// The fixed point a rubber-band line is drawn from while dragging, if any.
    pub anchor: Option<Pt>,
}

/// A temporary dimension; clicking its value lets the user type a new one, sent as
/// `set_temp_dimension(id, key, text)`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TempDim {
    pub id: ElementId,
    pub key: String,
    pub value: String,
    pub label_at: Pt,
    pub items: Vec<Item>,
}

/// An area that drags as a whole (a view title on a sheet, ADR-039): pressing inside it
/// and dragging calls `drag_handle(id, key, at + the drag)`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct DragArea {
    pub id: ElementId,
    pub key: String,
    pub min: Pt,
    pub max: Pt,
    /// The point the drag moves (sent moved by the drag).
    pub at: Pt,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Handles {
    pub grips: Vec<Grip>,
    pub dims: Vec<TempDim>,
    pub areas: Vec<DragArea>,
}

/// Where a dragged grid end snaps (ADR-060), as Revit's grid bubbles do: the end stays on
/// its grid's line, and snaps level with the ends of the grids parallel to it (their
/// bubbles line up) and to where other gridlines cross it. None for other grips.
pub fn grip_snap(
    doc: &Document,
    id: ElementId,
    key: &str,
    p: Pt,
    tol: f64,
) -> Option<crate::snap::SnapResult> {
    use crate::snap::{SnapKind, SnapResult};
    let ElementData::Grid { start, end, .. } = doc.data(id).ok()? else {
        return None;
    };
    let fixed = match key {
        "start" => *end,
        "end" => *start,
        _ => return None,
    };
    let moving = if key == "start" { *start } else { *end };
    let d = moving.sub(fixed).norm();
    let along = |q: Pt| fixed.add(d.scale(q.sub(fixed).dot(d)));
    let q = along(p);
    let mut best: Option<(f64, Pt, SnapKind, String)> = None;
    for e in doc.of(Category::Grid).filter(|e| e.id != id) {
        let ElementData::Grid {
            name,
            start: s2,
            end: e2,
            ..
        } = &e.data
        else {
            continue;
        };
        let d2 = e2.sub(*s2).norm();
        let cands: Vec<(Pt, SnapKind, String)> = if d.cross(d2).abs() < 1e-3 {
            // Parallel: level with its ends.
            [*s2, *e2]
                .iter()
                .map(|x| {
                    (
                        along(*x),
                        SnapKind::Endpoint,
                        format!("Aligned with Grid {name}"),
                    )
                })
                .collect()
        } else {
            studio_geom::line_intersection(fixed, d, *s2, d2)
                .map(|x| vec![(x, SnapKind::Intersection, format!("Grid {name}"))])
                .unwrap_or_default()
        };
        for (c, kind, label) in cands {
            let dist = c.dist(q);
            // Never onto (or past) the other end.
            if dist <= tol && c.sub(fixed).dot(d) > 1.0 && best.as_ref().is_none_or(|b| dist < b.0)
            {
                best = Some((dist, c, kind, label));
            }
        }
    }
    Some(match best {
        Some((_, pt, kind, label)) => SnapResult {
            pt,
            kind,
            label: Some(label),
        },
        None => SnapResult {
            pt: q,
            kind: SnapKind::None,
            label: Some(format_ft_in(q.dist(fixed))),
        },
    })
}

/// The box around what `id` draws in `view` (text by its size), for dragging.
fn drawn_extent(doc: &Document, view: ElementId, id: ElementId) -> Option<(Pt, Pt)> {
    let dl = crate::display_list_shared(doc, view)?;
    let mut lo = Pt::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut grow = |p: Pt| {
        lo = Pt::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Pt::new(hi.x.max(p.x), hi.y.max(p.y));
    };
    for it in dl.items.iter().filter(|i| i.el == Some(id)) {
        match &it.prim {
            Prim::Text { at, text, size, .. } => {
                // Centered text, about 0.6 of its height per character.
                let hw = text.chars().count() as f64 * size * 0.3;
                grow(Pt::new(at[0] - hw, at[1] - size * 0.6));
                grow(Pt::new(at[0] + hw, at[1] + size * 0.6));
            }
            Prim::Line { pts, .. } => pts.iter().for_each(|p| grow(Pt::new(p[0], p[1]))),
            Prim::Fill { rings, .. } => rings
                .iter()
                .flatten()
                .for_each(|p| grow(Pt::new(p[0], p[1]))),
            Prim::Circle { c, r, .. } => {
                grow(Pt::new(c[0] - r, c[1] - r));
                grow(Pt::new(c[0] + r, c[1] + r));
            }
        }
    }
    (lo.x.is_finite() && hi.x > lo.x).then_some((lo, hi))
}

fn view_of(doc: &Document, view: ElementId) -> Option<(&ViewKind, f64)> {
    match doc.data(view).ok()? {
        ElementData::View { kind, scale, .. } => Some((kind, f64::from(*scale))),
        _ => None,
    }
}

fn is_plan(kind: &ViewKind) -> bool {
    matches!(
        kind,
        ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. }
    )
}

fn temp_dim(scale: f64, id: ElementId, key: &str, a: Pt, b: Pt, offset: f64) -> Option<TempDim> {
    if a.dist(b) < 1.0 {
        return None;
    }
    let mut bld = Builder::new(scale);
    dimension(&mut bld, None, a, b, offset);
    let label_at = bld.items.iter().find_map(|it| match &it.prim {
        Prim::Text { at, .. } => Some(Pt::new(at[0], at[1])),
        _ => None,
    })?;
    Some(TempDim {
        id,
        key: key.into(),
        value: format_ft_in(a.dist(b)),
        label_at,
        items: bld.items,
    })
}

/// Revit's temporary dimensions from a selected wall or grid (ADR-041): to the nearest
/// parallel wall or grid on each side that runs alongside it, centerline to centerline.
/// Typing one moves the selection (`set_temp_dimension(id, "to:<other>", value)`).
fn distances_to_parallels(
    doc: &Document,
    model: &studio_regen::Model,
    scale: f64,
    id: ElementId,
    level: Option<ElementId>,
) -> Vec<TempDim> {
    let Some((a, b)) = studio_core::dimension::line_of(doc, id) else {
        return vec![];
    };
    let len = a.dist(b);
    if len < 1.0 {
        return vec![];
    }
    let u = b.sub(a).norm();
    let n = u.perp();
    let walls = model
        .walls
        .iter()
        .filter(|w| level.is_none_or(|l| w.level == l))
        .map(|w| (w.id, w.start, w.end));
    let grids = doc.of(Category::Grid).filter_map(|e| match &e.data {
        ElementData::Grid { start, end, .. } => Some((e.id, *start, *end)),
        _ => None,
    });
    // The nearest on each side: (distance, other, where along the selection).
    let mut best: [Option<(f64, ElementId, f64)>; 2] = [None, None];
    for (other, s, e) in walls.chain(grids) {
        if other == id || s.dist(e) < 1.0 || e.sub(s).norm().cross(u).abs() > 1e-3 {
            continue;
        }
        let (t0, t1) = {
            let (p, q) = (s.sub(a).dot(u), e.sub(a).dot(u));
            (p.min(q).max(0.0), p.max(q).min(len))
        };
        if t1 - t0 < 1.0 {
            continue; // Doesn't run alongside.
        }
        let d = s.sub(a).dot(n);
        if d.abs() < 1.0 {
            continue;
        }
        let k = usize::from(d < 0.0);
        if best[k].is_none_or(|x| d.abs() < x.0.abs()) {
            best[k] = Some((d, other, (t0 + t1) / 2.0));
        }
    }
    best.into_iter()
        .flatten()
        .filter_map(|(d, other, t)| {
            let from = a.add(u.scale(t));
            temp_dim(
                scale,
                id,
                &format!("to:{other}"),
                from,
                from.add(n.scale(d)),
                0.0,
            )
        })
        .collect()
}

/// The values of permanent dimensions in `view` that end on the selected wall or grid, made
/// typeable as in Revit (ADR-041): typing one moves the selection so that segment measures it
/// (`set_temp_dimension(id, "dim:<dimension>:<segment>", value)`).
fn permanent_values(doc: &Document, view: ElementId, scale: f64, id: ElementId) -> Vec<TempDim> {
    use studio_core::dimension::{anchored_to, string_anchors, string_points};
    let mut out = vec![];
    for e in doc.of(Category::Dimension) {
        let ElementData::Dimension {
            view: v, offset, ..
        } = &e.data
        else {
            continue;
        };
        if *v != view {
            continue;
        }
        let anchors = string_anchors(&e.data);
        if !anchors.iter().any(|a| anchored_to(a, id)) {
            continue;
        }
        let Some((pts, u)) = string_points(doc, &e.data) else {
            continue;
        };
        let labels = crate::segment_labels(&pts, u, *offset, scale);
        for (i, at) in labels.into_iter().enumerate() {
            // Only segments with exactly one end on the selection move it.
            if anchored_to(&anchors[i], id) == anchored_to(&anchors[i + 1], id) {
                continue;
            }
            out.push(TempDim {
                id,
                key: format!("dim:{}:{i}", e.id),
                value: format_ft_in(pts[i + 1].sub(pts[i]).dot(u).abs()),
                label_at: at,
                items: vec![],
            });
        }
    }
    out
}

/// Grips and temporary dimensions for the selected elements in `view`.
pub fn handles(doc: &Document, view: ElementId, ids: &[ElementId]) -> Handles {
    let mut out = Handles::default();
    let Some((kind, scale)) = view_of(doc, view) else {
        return out;
    };
    let plan = is_plan(kind);
    let view_level = match kind {
        ViewKind::FloorPlan { level } => Some(*level),
        _ => None,
    };
    let model = regenerate(doc);
    let one = ids.len() == 1;
    for id in ids {
        let Ok(data) = doc.data(*id) else { continue };
        match data {
            ElementData::Wall { start, end, .. } if plan => {
                out.grips.push(Grip {
                    id: *id,
                    key: "start".into(),
                    at: *start,
                    anchor: Some(*end),
                });
                out.grips.push(Grip {
                    id: *id,
                    key: "end".into(),
                    at: *end,
                    anchor: Some(*start),
                });
                if one {
                    let half = model
                        .walls
                        .iter()
                        .find(|w| w.id == *id)
                        .map_or(100.0, |w| w.thickness / 2.0);
                    // Length, drawn off the wall's exterior (left) face.
                    let off = half + scale * 6.0;
                    out.dims
                        .extend(temp_dim(scale, *id, "length", *start, *end, off));
                    out.dims
                        .extend(distances_to_parallels(doc, &model, scale, *id, view_level));
                    out.dims.extend(permanent_values(doc, view, scale, *id));
                }
            }
            // A level's ends in an elevation or section (ADR-052). The key carries the view
            // and where the other end is, since this view's ends are both kept once dragged.
            ElementData::Level { elevation, .. } if !plan => {
                let Some(dl) = crate::display_list_shared(doc, view) else {
                    continue;
                };
                let Some((_, x0, x1)) = crate::level_line(&dl.items, *id) else {
                    continue;
                };
                let (l, r) = (Pt::new(x0, *elevation), Pt::new(x1, *elevation));
                out.grips.push(Grip {
                    id: *id,
                    key: format!("level_end:{view}:left:{x1}"),
                    at: l,
                    anchor: Some(r),
                });
                out.grips.push(Grip {
                    id: *id,
                    key: format!("level_end:{view}:right:{x0}"),
                    at: r,
                    anchor: Some(l),
                });
            }
            ElementData::DetailLine {
                view: v,
                curve: studio_core::sketch::SketchCurve::Line { a, b, .. },
                ..
            } if *v == view => {
                for (key, at, anchor) in [("start", *a, *b), ("end", *b, *a)] {
                    out.grips.push(Grip {
                        id: *id,
                        key: key.into(),
                        at,
                        anchor: Some(anchor),
                    });
                }
            }
            ElementData::ModelLine {
                level,
                curve: studio_core::sketch::SketchCurve::Line { a, b, .. },
                ..
            } if plan && view_level == Some(*level) => {
                for (key, at, anchor) in [("start", *a, *b), ("end", *b, *a)] {
                    out.grips.push(Grip {
                        id: *id,
                        key: key.into(),
                        at,
                        anchor: Some(anchor),
                    });
                }
            }
            ElementData::Grid { start, end, .. } if plan => {
                for (key, at, anchor) in [("start", *start, *end), ("end", *end, *start)] {
                    out.grips.push(Grip {
                        id: *id,
                        key: key.into(),
                        at,
                        anchor: Some(anchor),
                    });
                }
                if one {
                    out.dims
                        .extend(distances_to_parallels(doc, &model, scale, *id, view_level));
                    out.dims.extend(permanent_values(doc, view, scale, *id));
                }
            }
            ElementData::Door { .. } | ElementData::Window { .. } if plan && one => {
                let Some(o) = model.openings.iter().find(|o| o.id == *id) else {
                    continue;
                };
                let Some(w) = model.walls.iter().find(|w| w.id == o.host) else {
                    continue;
                };
                let len = w.start.dist(w.end);
                // On the side the door doesn't swing to, clear of the wall.
                let side = if o.flip_facing { 1.0 } else { -1.0 };
                let off = side * (w.thickness / 2.0 + scale * 5.0);
                out.dims
                    .extend(temp_dim(scale, *id, "gap_start", w.start, o.at(o.t0), off));
                out.dims.extend(temp_dim(
                    scale,
                    *id,
                    "gap_end",
                    o.at(o.t1),
                    w.start.add(o.dir.scale(len)),
                    off,
                ));
            }
            d @ ElementData::Dimension { offset, .. } => {
                if let Some((pts, u)) = studio_core::dimension::string_points(doc, d) {
                    // The middle of the dimension line.
                    let o = pts[0];
                    let span = pts.iter().map(|p| p.sub(o).dot(u));
                    let (lo, hi) =
                        span.fold((f64::MAX, f64::MIN), |(l, h), s| (l.min(s), h.max(s)));
                    out.grips.push(Grip {
                        id: *id,
                        key: "line".into(),
                        at: o.add(u.scale((lo + hi) / 2.0)).add(u.perp().scale(*offset)),
                        anchor: None,
                    });
                }
            }
            d @ ElementData::AngularDimension { .. } => {
                if let Some(arc) = studio_core::dimension::angular_arc(doc, d) {
                    let a0 = arc.from.y.atan2(arc.from.x) + arc.sweep / 2.0;
                    out.grips.push(Grip {
                        id: *id,
                        key: "arc".into(),
                        at: arc
                            .center
                            .add(Pt::new(a0.cos(), a0.sin()).scale(arc.radius)),
                        anchor: Some(arc.center),
                    });
                }
            }
            // A camera's eye and target, in plans of its level (ADR-027).
            ElementData::View {
                camera: Some(c), ..
            } if plan && *id != view && view_level == Some(c.level) => {
                out.grips.push(Grip {
                    id: *id,
                    key: "camera:eye".into(),
                    at: c.eye,
                    anchor: Some(c.target),
                });
                out.grips.push(Grip {
                    id: *id,
                    key: "camera:target".into(),
                    at: c.target,
                    anchor: Some(c.eye),
                });
            }
            ElementData::View { crop: Some(c), .. } if *id == view => {
                let mid = |a: Pt, b: Pt| a.lerp(b, 0.5);
                let (lo, hi) = (c.min, c.max);
                for (key, at) in [
                    ("crop:left", mid(lo, Pt::new(lo.x, hi.y))),
                    ("crop:right", mid(Pt::new(hi.x, lo.y), hi)),
                    ("crop:bottom", mid(lo, Pt::new(hi.x, lo.y))),
                    ("crop:top", mid(Pt::new(lo.x, hi.y), hi)),
                ] {
                    out.grips.push(Grip {
                        id: view,
                        key: key.into(),
                        at,
                        anchor: None,
                    });
                }
            }
            // A selected tag drags by its text (ADR-060), in any view it's drawn in.
            ElementData::Tag { offset, .. } => {
                if let Some((min, max)) = drawn_extent(doc, view, *id) {
                    out.areas.push(DragArea {
                        id: *id,
                        key: "tag".into(),
                        min,
                        max,
                        at: *offset,
                    });
                }
            }
            // A text note (ADR-070): drag the text (its arrowheads stay), each leader's
            // arrowhead and elbow (a straight leader's middle bends it), and the wrap width.
            ElementData::TextNote {
                at, leaders, align, ..
            } => {
                let Ok((tb, lines, _)) = studio_core::ops::text_note_box(doc, *id) else {
                    continue;
                };
                out.areas.push(DragArea {
                    id: *id,
                    key: "text_move".into(),
                    min: tb.min,
                    max: tb.max,
                    at: *at,
                });
                for (i, (l, pts)) in leaders.iter().zip(&lines).enumerate() {
                    out.grips.push(Grip {
                        id: *id,
                        key: format!("leader:{i}:end"),
                        at: l.end,
                        anchor: pts.first().copied(),
                    });
                    let elbow = l.elbow.unwrap_or_else(|| match pts.as_slice() {
                        [a, .., b] => a.lerp(*b, 0.5),
                        _ => l.end,
                    });
                    out.grips.push(Grip {
                        id: *id,
                        key: format!("leader:{i}:elbow"),
                        at: elbow,
                        anchor: None,
                    });
                }
                let y = (tb.min.y + tb.max.y) / 2.0;
                let x = if *align == studio_core::text::TextAlign::Right {
                    tb.min.x
                } else {
                    tb.max.x
                };
                out.grips.push(Grip {
                    id: *id,
                    key: "text_width".into(),
                    at: Pt::new(x, y),
                    anchor: None,
                });
            }
            _ => {}
        }
    }
    out
}

/// A straight reference an element offers for Align: a wall face or centerline, or a grid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RefLine {
    pub el: ElementId,
    pub a: Pt,
    pub b: Pt,
}

/// Reference lines of the walls cut by a plan view and of grids.
fn ref_lines(doc: &Document, view: ElementId) -> Vec<RefLine> {
    let Some((kind, _)) = view_of(doc, view) else {
        return vec![];
    };
    let (level, cut) = match kind {
        ViewKind::FloorPlan { level } => (*level, PLAN_CUT),
        ViewKind::CeilingPlan { level } => (*level, RCP_CUT),
        _ => return vec![],
    };
    let z = doc.level_elevation(level).unwrap_or(0.0) + cut;
    let model = regenerate(doc);
    let mut out = vec![];
    for w in model.walls.iter().filter(|w| w.z0 <= z && w.z1 > z) {
        let n = w.dir().perp();
        for off in [0.0, w.thickness / 2.0, -w.thickness / 2.0] {
            out.push(RefLine {
                el: w.id,
                a: w.start.add(n.scale(off)),
                b: w.end.add(n.scale(off)),
            });
        }
    }
    for g in &model.grids {
        out.push(RefLine {
            el: g.id,
            a: g.start,
            b: g.end,
        });
    }
    out
}

/// The reference line nearest `p` within `tol` mm, optionally skipping one element.
pub fn ref_line(
    doc: &Document,
    view: ElementId,
    p: Pt,
    tol: f64,
    skip: Option<ElementId>,
) -> Option<RefLine> {
    ref_lines(doc, view)
        .into_iter()
        .filter(|r| Some(r.el) != skip)
        .map(|r| (project_to_segment(p, r.a, r.b).1, r))
        .filter(|(d, _)| *d <= tol)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, r)| r)
}

/// How far to move `target` so it lies on `reference` (both must be parallel).
pub fn align_delta(reference: &RefLine, target: &RefLine) -> Option<Pt> {
    let d1 = reference.b.sub(reference.a).norm();
    let d2 = target.b.sub(target.a).norm();
    if d1.cross(d2).abs() > 0.01 {
        return None;
    }
    let n = d1.perp();
    let dist = reference.a.sub(target.a).dot(n);
    Some(n.scale(dist))
}

/// What Offset would create for the cursor at `p`: the line `distance` mm from the wall
/// or grid under the cursor, on the cursor's side.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct OffsetPreview {
    pub id: ElementId,
    pub items: Vec<Item>,
}

pub fn offset_preview(
    doc: &Document,
    view: ElementId,
    p: Pt,
    tol: f64,
    distance: f64,
) -> Option<OffsetPreview> {
    let (_, scale) = view_of(doc, view)?;
    let model = regenerate(doc);
    let (id, s, e, _) = model
        .walls
        .iter()
        .map(|w| (w.id, w.start, w.end, w.thickness / 2.0))
        .chain(
            model
                .grids
                .iter()
                .filter(|_| doc.count(Category::Grid) > 0)
                .map(|g| (g.id, g.start, g.end, 0.0)),
        )
        .map(|c| (project_to_segment(p, c.1, c.2).1, c))
        .filter(|(d, c)| *d <= c.3 + tol)
        .min_by(|a, b| a.0.total_cmp(&b.0))?
        .1;
    let n = e.sub(s).norm().perp();
    let sign = if p.sub(s).dot(n) >= 0.0 { 1.0 } else { -1.0 };
    let d = n.scale(sign * distance.abs());
    let mut b = Builder::new(scale);
    b.line(None, &[s.add(d), e.add(d)], false, 3, Dash::Dashed);
    Some(OffsetPreview { id, items: b.items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ops;
    use studio_core::units::MM_PER_FT;

    fn setup() -> (Document, ElementId, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let w = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 0.0),
            Pt::new(20.0 * MM_PER_FT, 0.0),
        )
        .unwrap();
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let door = ops::create_door(&mut doc, dt, w, 5.0 * MM_PER_FT, false).unwrap();
        (doc, plan, w, door)
    }

    #[test]
    fn wall_grips_and_length_dimension() {
        let (doc, plan, w, _) = setup();
        let h = handles(&doc, plan, &[w]);
        assert_eq!(h.grips.len(), 2);
        assert_eq!(h.grips[1].key, "end");
        assert_eq!(h.dims.len(), 1);
        assert_eq!(h.dims[0].value, "20'-0\"");
        assert!(
            h.dims[0].label_at.y > 0.0,
            "drawn off the exterior (left) side"
        );
    }

    #[test]
    fn door_gap_dimensions() {
        let (doc, plan, _, door) = setup();
        let h = handles(&doc, plan, &[door]);
        let v: Vec<&str> = h.dims.iter().map(|d| d.value.as_str()).collect();
        // 36" door centered 5' along a 20' wall: 3'-6" and 13'-6" clear.
        assert_eq!(v, ["3'-6\"", "13'-6\""]);
        assert!(
            handles(&doc, plan, &[door, door]).dims.is_empty(),
            "only for one element"
        );
    }

    #[test]
    fn align_moves_onto_the_reference() {
        let (doc, plan, w, _) = setup();
        let face = ref_line(&doc, plan, Pt::new(1000.0, 101.0), 20.0, None).unwrap();
        assert_eq!(face.el, w);
        assert!(
            (face.a.y - 4.0 * 25.4).abs() < 1e-6,
            "the exterior face of an 8\" wall"
        );
        let other = RefLine {
            el: w,
            a: Pt::new(0.0, 500.0),
            b: Pt::new(10.0, 500.0),
        };
        let d = align_delta(&face, &other).unwrap();
        assert!((d.y - (4.0 * 25.4 - 500.0)).abs() < 1e-6);
        let skew = RefLine {
            el: w,
            a: Pt::new(0.0, 0.0),
            b: Pt::new(10.0, 10.0),
        };
        assert!(align_delta(&face, &skew).is_none());
    }

    #[test]
    fn offset_preview_follows_the_cursor_side() {
        let (doc, plan, w, _) = setup();
        let p = offset_preview(&doc, plan, Pt::new(1000.0, -120.0), 50.0, 1000.0).unwrap();
        assert_eq!(p.id, w);
        match &p.items[0].prim {
            Prim::Line { pts, .. } => assert!((pts[0][1] + 1000.0).abs() < 1e-6),
            _ => unreachable!(),
        }
    }

    #[test]
    fn a_grid_end_stays_on_its_line_and_snaps_to_other_grids() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let g = |doc: &mut Document, a: (f64, f64), b: (f64, f64)| {
            studio_core::ops::create_grid(doc, Pt::new(a.0, a.1), Pt::new(b.0, b.1)).unwrap()
        };
        // Three vertical grids, the second shorter; one horizontal grid across them.
        let a = g(&mut doc, (0.0, 0.0), (0.0, 10000.0));
        g(&mut doc, (6000.0, 0.0), (6000.0, 12000.0));
        g(&mut doc, (-3000.0, 4000.0), (12000.0, 4000.0));
        // Dragging A's top end sideways and near 12 m: on A's line, level with the other end.
        let s = grip_snap(&doc, a, "end", Pt::new(250.0, 11900.0), 300.0).unwrap();
        assert!(
            (s.pt.x).abs() < 1e-9 && (s.pt.y - 12000.0).abs() < 1e-9,
            "{:?}",
            s.pt
        );
        assert!(s.label.unwrap().starts_with("Aligned with Grid"));
        // Near the horizontal grid: its crossing.
        let s = grip_snap(&doc, a, "end", Pt::new(100.0, 4150.0), 300.0).unwrap();
        assert!(matches!(s.kind, crate::snap::SnapKind::Intersection));
        assert!((s.pt.y - 4000.0).abs() < 1e-9 && s.pt.x.abs() < 1e-9);
        // Nothing near: just along the line.
        let s = grip_snap(&doc, a, "end", Pt::new(400.0, 8000.0), 300.0).unwrap();
        assert!(s.pt.x.abs() < 1e-9 && (s.pt.y - 8000.0).abs() < 1e-9);
        assert!(grip_snap(&doc, a, "line", Pt::new(0.0, 0.0), 300.0).is_none());
    }

    #[test]
    fn room_tags_drag_and_go_in_sections() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = studio_core::ops::first_of(&doc, Category::WallType).unwrap();
        let pts = [(0.0, 0.0), (6000.0, 0.0), (6000.0, 4000.0), (0.0, 4000.0)];
        for k in 0..4 {
            let (a, b) = (pts[k], pts[(k + 1) % 4]);
            studio_core::ops::create_wall(&mut doc, wt, l1, Pt::new(a.0, a.1), Pt::new(b.0, b.1))
                .unwrap();
        }
        let room = studio_core::ops::create_room(&mut doc, l1, Pt::new(3000.0, 2000.0)).unwrap();
        // A section across the room, looking north.
        let sec = studio_core::ops::create_section(
            &mut doc,
            Pt::new(-2000.0, 2000.0),
            Pt::new(8000.0, 2000.0),
        )
        .unwrap();
        let z0 = doc.levels()[0].2;
        let click = Pt::new(4000.0, z0 + 1500.0);
        assert_eq!(crate::view_refs::room_in_view(&doc, sec, click), Some(room));
        let base = crate::view_refs::room_tag_base(&doc, sec, room).unwrap();
        let tag = studio_core::visibility::tag_room_in_view(&mut doc, sec, room, click.sub(base))
            .unwrap();
        // Drawn where it was clicked, and selectable there.
        let dl = crate::display_list(&doc, sec).unwrap();
        assert!(crate::pick(&dl, click, 50.0) == Some(tag));
        // Selected, it drags by its text: the area is around the click.
        let h = handles(&doc, sec, &[tag]);
        let area = h.areas.iter().find(|a| a.id == tag).unwrap();
        assert!(area.min.x < click.x && area.max.x > click.x);
        assert!(area.min.y < click.y + 500.0 && area.max.y > click.y - 500.0);
        studio_core::edit::drag_handle(&mut doc, tag, "tag", area.at.add(Pt::new(600.0, 300.0)))
            .unwrap();
        let dl = crate::display_list(&doc, sec).unwrap();
        assert_eq!(
            crate::pick(&dl, click.add(Pt::new(600.0, 300.0)), 50.0),
            Some(tag)
        );
        // Plans' room tags drag the same way.
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        let ptag = doc
            .of(Category::Tag)
            .find(|e| matches!(&e.data, ElementData::Tag { view, target, .. } if *view == plan && *target == room))
            .map(|e| e.id);
        if let Some(ptag) = ptag {
            let h = handles(&doc, plan, &[ptag]);
            assert!(h.areas.iter().any(|a| a.id == ptag && a.key == "tag"));
        }
        // Only one tag per room per view.
        assert!(
            studio_core::visibility::tag_room_in_view(&mut doc, sec, room, Pt::default()).is_err()
        );
    }
}
