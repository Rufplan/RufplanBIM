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

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Handles {
    pub grips: Vec<Grip>,
    pub dims: Vec<TempDim>,
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
                if let Some((a, b)) = studio_core::ops::dimension_ends(doc, d) {
                    let n = b.sub(a).norm().perp();
                    out.grips.push(Grip {
                        id: *id,
                        key: "line".into(),
                        at: a.lerp(b, 0.5).add(n.scale(*offset)),
                        anchor: None,
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
}
