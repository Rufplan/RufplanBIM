//! View generation: display lists for plans, ceiling plans and elevations, 3D meshes,
//! picking and snapping. Display-list coordinates are model mm (plans: x east, y north;
//! elevations: u to the viewer's right, z up). Annotation sizes are paper mm × scale.

use serde::Serialize;
use studio_core::units::{format_ft_in, MM_PER_IN};
use studio_core::{Category, Document, ElementData, ElementId, ViewKind};
use studio_geom::{point_in_ring, project_to_segment, Pt};
use studio_regen::{bounds, regenerate, Model};
use ts_rs::TS;

pub mod snap;
pub use snap::{snap, SnapResult};

/// Plan cut plane height above the level, mm (4'-0").
pub const PLAN_CUT: f64 = 48.0 * MM_PER_IN;
/// Reflected ceiling plan cut plane above the level, mm (7'-6").
pub const RCP_CUT: f64 = 90.0 * MM_PER_IN;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum Dash {
    Solid,
    Dashed,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum FillKind {
    /// Cut material (walls in plan).
    Poche,
    /// Opaque paper-white (painter's hidden-line fill in elevations).
    Paper,
    /// Light slab tone (floors).
    Slab,
    /// Ceiling tone.
    Ceiling,
    /// Solid ink (level and elevation markers).
    Ink,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum Anchor {
    Center,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "t")]
#[ts(export)]
pub enum Prim {
    /// Polyline; `w` is a pen weight 1 (finest) to 6 (heaviest).
    Line {
        pts: Vec<[f64; 2]>,
        closed: bool,
        w: u8,
        dash: Dash,
    },
    /// Filled polygon with holes (first ring outer).
    Fill {
        rings: Vec<Vec<[f64; 2]>>,
        fill: FillKind,
    },
    /// Text with its height in model mm.
    Text {
        at: [f64; 2],
        text: String,
        size: f64,
        anchor: Anchor,
    },
    Circle {
        c: [f64; 2],
        r: f64,
        w: u8,
        filled: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Item {
    /// Source element, used for selection and highlighting.
    pub el: Option<ElementId>,
    pub prim: Prim,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum ViewType {
    Plan,
    CeilingPlan,
    Elevation,
    ThreeD,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayList {
    pub view_type: ViewType,
    /// Drawing scale denominator.
    pub scale: u32,
    /// [min x, min y, max x, max y] in model mm.
    pub bounds: [f64; 4],
    pub items: Vec<Item>,
}

fn a(p: Pt) -> [f64; 2] {
    [p.x, p.y]
}

fn ring(pts: &[Pt]) -> Vec<[f64; 2]> {
    pts.iter().copied().map(a).collect()
}

struct Builder {
    items: Vec<Item>,
    scale: f64,
}

impl Builder {
    fn push(&mut self, el: Option<ElementId>, prim: Prim) {
        self.items.push(Item { el, prim });
    }
    fn line(&mut self, el: Option<ElementId>, pts: &[Pt], closed: bool, w: u8, dash: Dash) {
        self.push(
            el,
            Prim::Line {
                pts: ring(pts),
                closed,
                w,
                dash,
            },
        );
    }
    fn fill(&mut self, el: Option<ElementId>, rings: Vec<Vec<[f64; 2]>>, fill: FillKind) {
        self.push(el, Prim::Fill { rings, fill });
    }
    /// Text sized in paper mm.
    fn text(&mut self, el: Option<ElementId>, at: Pt, text: String, paper_mm: f64, anchor: Anchor) {
        self.push(
            el,
            Prim::Text {
                at: a(at),
                text,
                size: paper_mm * self.scale,
                anchor,
            },
        );
    }
    /// Circle sized in paper mm.
    fn circle(&mut self, el: Option<ElementId>, c: Pt, paper_r: f64, w: u8, filled: bool) {
        self.push(
            el,
            Prim::Circle {
                c: a(c),
                r: paper_r * self.scale,
                w,
                filled,
            },
        );
    }
    fn paper(&self, mm: f64) -> f64 {
        mm * self.scale
    }
}

/// Generates the display list of a 2D view. Returns None for 3D views.
pub fn display_list(doc: &Document, view: ElementId) -> Option<DisplayList> {
    let ElementData::View { kind, scale, .. } = doc.data(view).ok()? else {
        return None;
    };
    let model = regenerate(doc);
    let mut b = Builder {
        items: vec![],
        scale: f64::from(*scale),
    };
    let (view_type, bounds) = match kind {
        ViewKind::FloorPlan { level } => (ViewType::Plan, plan(doc, &model, &mut b, *level, false)),
        ViewKind::CeilingPlan { level } => (
            ViewType::CeilingPlan,
            plan(doc, &model, &mut b, *level, true),
        ),
        ViewKind::Elevation { facing } => (
            ViewType::Elevation,
            elevation(&model, &mut b, facing.look().scale(-1.0)),
        ),
        ViewKind::ThreeD => return None,
    };
    Some(DisplayList {
        view_type,
        scale: *scale,
        bounds,
        items: b.items,
    })
}

/// Default plan extents when the model is empty: 60' × 40' around the origin.
fn plan_extents(model: &Model) -> (Pt, Pt) {
    model
        .plan_bounds()
        .unwrap_or((Pt::new(-3000.0, -3000.0), Pt::new(15000.0, 9000.0)))
}

fn plan(
    doc: &Document,
    model: &Model,
    b: &mut Builder,
    level: ElementId,
    ceiling: bool,
) -> [f64; 4] {
    let elev = doc.level_elevation(level).unwrap_or(0.0);
    let cut = elev + if ceiling { RCP_CUT } else { PLAN_CUT };

    if !ceiling {
        for f in model
            .floors
            .iter()
            .filter(|f| f.z1 >= elev - 1.0 && f.z1 <= cut)
        {
            b.fill(Some(f.id), vec![ring(&f.base.outer)], FillKind::Slab);
            b.line(Some(f.id), &f.base.outer, true, 1, Dash::Solid);
        }
    } else {
        for c in model.ceilings.iter().filter(|c| c.level == level) {
            b.fill(Some(c.id), vec![ring(&c.base.outer)], FillKind::Ceiling);
            let is_act = doc
                .data(c.id)
                .ok()
                .and_then(|d| d.type_id())
                .and_then(|t| doc.data(t).ok())
                .is_some_and(|t| t.name().contains("ACT"));
            if is_act {
                // 2' × 4' acoustical tile grid aligned to the project origin.
                for seg in grid_hatch(&c.base.outer, 24.0 * MM_PER_IN, 48.0 * MM_PER_IN) {
                    b.line(Some(c.id), &seg, false, 1, Dash::Solid);
                }
            }
            b.line(Some(c.id), &c.base.outer, true, 2, Dash::Solid);
        }
    }

    // Walls below the cut plane shown in projection.
    for w in model
        .walls
        .iter()
        .filter(|w| w.z1 <= cut && w.z1 > elev + 1.0)
    {
        b.line(Some(w.id), &w.footprint.outer, true, 2, Dash::Solid);
    }
    // Walls through the cut plane: per-wall poché for picking, then the merged outline.
    let cut_walls: Vec<_> = model
        .walls
        .iter()
        .filter(|w| w.z0 <= cut && w.z1 > cut)
        .collect();
    for w in &cut_walls {
        b.fill(Some(w.id), vec![ring(&w.footprint.outer)], FillKind::Poche);
    }
    let merged = studio_geom::union_all(
        &cut_walls
            .iter()
            .map(|w| w.footprint.clone())
            .collect::<Vec<_>>(),
    );
    for region in &merged {
        b.line(None, &region.outer, true, 5, Dash::Solid);
        for h in &region.holes {
            b.line(None, h, true, 5, Dash::Solid);
        }
    }

    let (lo, hi) = plan_extents(model);
    let margin = b.paper(30.0);
    for g in &model.grids {
        grid_in_plan(b, g.id, &g.name, g.start, g.end);
    }
    if !ceiling {
        elevation_markers(doc, b, lo, hi, margin);
    }
    let mut pts = vec![lo, hi];
    for it in &b.items {
        match &it.prim {
            Prim::Circle { c, r, .. } => {
                pts.push(Pt::new(c[0] - r, c[1] - r));
                pts.push(Pt::new(c[0] + r, c[1] + r));
            }
            Prim::Text { at, .. } => pts.push(Pt::new(at[0], at[1])),
            _ => {}
        }
    }
    let (lo, hi) = bounds(&pts).unwrap_or((lo, hi));
    [lo.x - margin, lo.y - margin, hi.x + margin, hi.y + margin]
}

fn grid_in_plan(b: &mut Builder, id: ElementId, name: &str, start: Pt, end: Pt) {
    let r = 6.0;
    let dir = end.sub(start).norm();
    b.line(Some(id), &[start, end], false, 2, Dash::Center);
    for (p, d) in [(end, dir), (start, dir.scale(-1.0))] {
        let c = p.add(d.scale(b.paper(r)));
        b.circle(Some(id), c, r, 2, false);
        b.text(Some(id), c, name.to_owned(), 4.5, Anchor::Center);
    }
}

/// Four elevation markers around the plan, each linked to its elevation view.
fn elevation_markers(doc: &Document, b: &mut Builder, lo: Pt, hi: Pt, margin: f64) {
    let mid = lo.lerp(hi, 0.5);
    let off = margin * 1.5 + b.paper(8.0);
    for v in doc.of(Category::View) {
        let ElementData::View {
            kind: ViewKind::Elevation { facing },
            ..
        } = &v.data
        else {
            continue;
        };
        let out = facing.look();
        let c = match (out.x as i32, out.y as i32) {
            (0, 1) => Pt::new(mid.x, hi.y + off),
            (0, _) => Pt::new(mid.x, lo.y - off),
            (1, _) => Pt::new(hi.x + off, mid.y),
            _ => Pt::new(lo.x - off, mid.y),
        };
        // The marker points back at the building (the direction the elevation looks).
        let look = out.scale(-1.0);
        let r = b.paper(5.0);
        b.circle(Some(v.id), c, 5.0, 2, false);
        let tip = c.add(look.scale(r * 1.7));
        let side = look.perp().scale(r * 0.95);
        let base = c.add(look.scale(r * 0.25));
        b.fill(
            Some(v.id),
            vec![ring(&[tip, base.add(side), base.sub(side)])],
            FillKind::Ink,
        );
    }
}

/// Clips a rectangular hatch grid (spacing `dx` along x, `dy` along y) to a ring.
fn grid_hatch(ring_pts: &[Pt], dy: f64, dx: f64) -> Vec<[Pt; 2]> {
    let Some((lo, hi)) = bounds(ring_pts) else {
        return vec![];
    };
    let mut out = vec![];
    let n = ring_pts.len();
    let mut scan = |horizontal: bool, step: f64| {
        let (a0, a1) = if horizontal {
            (lo.y, hi.y)
        } else {
            (lo.x, hi.x)
        };
        let mut k = (a0 / step).ceil() * step;
        while k < a1 {
            let mut xs = vec![];
            for i in 0..n {
                let (p, q) = (ring_pts[i], ring_pts[(i + 1) % n]);
                let (pa, qa, pb, qb) = if horizontal {
                    (p.y, q.y, p.x, q.x)
                } else {
                    (p.x, q.x, p.y, q.y)
                };
                if (pa <= k) != (qa <= k) {
                    xs.push(pb + (k - pa) / (qa - pa) * (qb - pb));
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                let (s, e) = if horizontal {
                    (Pt::new(pair[0], k), Pt::new(pair[1], k))
                } else {
                    (Pt::new(k, pair[0]), Pt::new(k, pair[1]))
                };
                out.push([s, e]);
            }
            k += step;
        }
    };
    scan(true, dy);
    scan(false, dx);
    out
}

fn elevation(model: &Model, b: &mut Builder, look: Pt) -> [f64; 4] {
    let right = Pt::new(look.y, -look.x);
    let u_of = |p: Pt| p.dot(right);
    let depth_of = |p: Pt| p.dot(look);

    struct Face {
        el: ElementId,
        u0: f64,
        u1: f64,
        z0: f64,
        z1: f64,
        near: f64,
        mid: f64,
        fill: FillKind,
    }
    let face = |el: ElementId, pts: &[Pt], z0: f64, z1: f64, fill: FillKind| {
        let us: Vec<f64> = pts.iter().map(|p| u_of(*p)).collect();
        Face {
            el,
            u0: us.iter().copied().fold(f64::INFINITY, f64::min),
            u1: us.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            z0,
            z1,
            near: pts
                .iter()
                .map(|p| depth_of(*p))
                .fold(f64::INFINITY, f64::min),
            mid: pts.iter().map(|p| depth_of(*p)).sum::<f64>() / pts.len().max(1) as f64,
            fill,
        }
    };
    let mut faces: Vec<Face> = vec![];
    for w in &model.walls {
        faces.push(face(w.id, &w.footprint.outer, w.z0, w.z1, FillKind::Paper));
    }
    for f in &model.floors {
        faces.push(face(f.id, &f.base.outer, f.z0, f.z1, FillKind::Slab));
    }
    for c in &model.ceilings {
        faces.push(face(c.id, &c.base.outer, c.z0, c.z1, FillKind::Paper));
    }
    // Painter's algorithm: farthest first so nearer faces cover what they hide. Mitered
    // corners make side walls reach as near as the facade, so ties break on average depth.
    faces.sort_by(|a, b| b.near.total_cmp(&a.near).then(b.mid.total_cmp(&a.mid)));

    let (plo, phi) = model
        .plan_bounds()
        .unwrap_or((Pt::new(0.0, 0.0), Pt::new(12000.0, 9000.0)));
    let corners = [plo, phi, Pt::new(plo.x, phi.y), Pt::new(phi.x, plo.y)];
    let umin = corners
        .iter()
        .map(|p| u_of(*p))
        .fold(f64::INFINITY, f64::min);
    let umax = corners
        .iter()
        .map(|p| u_of(*p))
        .fold(f64::NEG_INFINITY, f64::max);
    let (zmin, zmax) = model.z_range();

    for f in &faces {
        let r = [
            Pt::new(f.u0, f.z0),
            Pt::new(f.u1, f.z0),
            Pt::new(f.u1, f.z1),
            Pt::new(f.u0, f.z1),
        ];
        b.fill(Some(f.el), vec![ring(&r)], f.fill);
        b.line(Some(f.el), &r, true, 2, Dash::Solid);
    }

    let ext = b.paper(12.0);
    b.line(
        None,
        &[Pt::new(umin - ext, 0.0), Pt::new(umax + ext, 0.0)],
        false,
        6,
        Dash::Solid,
    );

    for l in &model.levels {
        let x1 = umax + ext * 2.0;
        b.line(
            Some(l.id),
            &[Pt::new(umin - ext, l.elevation), Pt::new(x1, l.elevation)],
            false,
            1,
            Dash::Center,
        );
        let r = 3.0;
        let c = Pt::new(x1 + b.paper(r), l.elevation);
        b.circle(Some(l.id), c, r, 2, true);
        let tx = x1 + b.paper(r * 2.0 + 2.0);
        b.text(
            Some(l.id),
            Pt::new(tx, l.elevation + b.paper(2.8)),
            l.name.clone(),
            4.2,
            Anchor::Left,
        );
        b.text(
            Some(l.id),
            Pt::new(tx, l.elevation - b.paper(2.8)),
            format_ft_in(l.elevation),
            3.6,
            Anchor::Left,
        );
    }

    for g in &model.grids {
        let d = g.end.sub(g.start).norm();
        if d.dot(right).abs() > 0.05 {
            continue; // Only grids running along the view direction appear as lines.
        }
        let u = u_of(g.start);
        let top = zmax + b.paper(10.0);
        b.line(
            Some(g.id),
            &[Pt::new(u, zmin - b.paper(4.0)), Pt::new(u, top)],
            false,
            2,
            Dash::Center,
        );
        let c = Pt::new(u, top + b.paper(6.0));
        b.circle(Some(g.id), c, 6.0, 2, false);
        b.text(Some(g.id), c, g.name.clone(), 4.5, Anchor::Center);
    }

    let mut maxx = umax + ext * 2.0 + b.paper(30.0);
    for it in &b.items {
        if let Prim::Text { at, .. } = &it.prim {
            maxx = maxx.max(at[0] + b.paper(25.0));
        }
    }
    let margin = b.paper(15.0);
    [
        umin - ext - margin,
        zmin.min(0.0) - margin,
        maxx,
        zmax + b.paper(30.0),
    ]
}

/// Topmost element at `p` (display-list coordinates) within `tol` mm.
pub fn pick(dl: &DisplayList, p: Pt, tol: f64) -> Option<ElementId> {
    let mut best: Option<(f64, ElementId)> = None;
    for it in dl.items.iter().rev() {
        let Some(el) = it.el else { continue };
        let d = match &it.prim {
            Prim::Line { pts, closed, .. } => {
                let n = pts.len();
                let segs = if *closed { n } else { n.saturating_sub(1) };
                (0..segs)
                    .map(|i| {
                        let (a0, b0) = (pts[i], pts[(i + 1) % n]);
                        project_to_segment(p, Pt::new(a0[0], a0[1]), Pt::new(b0[0], b0[1])).1
                    })
                    .fold(f64::INFINITY, f64::min)
            }
            Prim::Circle { c, r, .. } => (p.dist(Pt::new(c[0], c[1])) - r).max(0.0),
            Prim::Text { at, size, .. } => (p.dist(Pt::new(at[0], at[1])) - size).max(0.0),
            Prim::Fill { rings, fill } => {
                let outer: Vec<Pt> = rings
                    .first()
                    .map(|r| r.iter().map(|q| Pt::new(q[0], q[1])).collect())
                    .unwrap_or_default();
                if !point_in_ring(p, &outer) {
                    f64::INFINITY
                } else if matches!(fill, FillKind::Slab | FillKind::Ceiling) {
                    // Floors and ceilings rank below nearby lines (grids crossing a room).
                    tol * 0.9
                } else {
                    // Inside a wall (or marker): the wall wins over grids on its centerline.
                    0.0
                }
            }
        };
        if d <= tol && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, el));
        }
    }
    best.map(|b| b.1)
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Mesh {
    pub el: ElementId,
    pub category: Category,
    /// Exterior walls render in a different tone.
    pub exterior: bool,
    /// Triangle soup, 9 floats per triangle, mm, z-up.
    pub positions: Vec<f32>,
}

/// Meshes for the 3D view.
pub fn meshes(doc: &Document) -> Vec<Mesh> {
    let m = regenerate(doc);
    let mut out = vec![];
    for w in &m.walls {
        out.push(Mesh {
            el: w.id,
            category: Category::Wall,
            exterior: w.exterior,
            positions: w.prism().triangles(),
        });
    }
    for s in m.floors.iter().chain(&m.ceilings) {
        out.push(Mesh {
            el: s.id,
            category: s.category,
            exterior: false,
            positions: s.prism().triangles(),
        });
    }
    out
}

/// Crate version from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ops;
    use studio_core::units::MM_PER_FT;

    fn building() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .map(|e| e.id)
            .unwrap();
        let (w, h) = (40.0 * MM_PER_FT, 30.0 * MM_PER_FT);
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ];
        for i in 0..4 {
            ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap();
        }
        let m = regenerate(&doc);
        let ft = ops::first_of(&doc, Category::FloorType).unwrap();
        ops::create_floor(
            &mut doc,
            ft,
            l1,
            studio_regen::outer_boundary(&m, l1).unwrap(),
        )
        .unwrap();
        let ct = doc
            .of(Category::CeilingType)
            .find(|e| e.data.name().contains("ACT"))
            .map(|e| e.id)
            .unwrap();
        ops::create_ceiling(
            &mut doc,
            ct,
            l1,
            studio_regen::room_at(&m, l1, Pt::new(1000.0, 1000.0)).unwrap(),
        )
        .unwrap();
        ops::create_grid(&mut doc, Pt::new(0.0, -2000.0), Pt::new(0.0, h + 2000.0)).unwrap();
        (doc, l1)
    }

    fn view(doc: &Document, pred: impl Fn(&ViewKind) -> bool) -> ElementId {
        doc.of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind, .. } if pred(kind)))
            .map(|e| e.id)
            .unwrap()
    }

    fn count(dl: &DisplayList, f: impl Fn(&Prim) -> bool) -> usize {
        dl.items.iter().filter(|i| f(&i.prim)).count()
    }

    #[test]
    fn floor_plan_has_cut_walls_one_merged_outline_and_grid() {
        let (doc, l1) = building();
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let dl = display_list(&doc, v).unwrap();
        assert_eq!(dl.view_type, ViewType::Plan);
        assert_eq!(
            count(&dl, |p| matches!(
                p,
                Prim::Fill {
                    fill: FillKind::Poche,
                    ..
                }
            )),
            4
        );
        // Merged cut outline: one outer + one inner ring, heavy pen, no element tag.
        let heavy: Vec<_> = dl
            .items
            .iter()
            .filter(|i| i.el.is_none() && matches!(i.prim, Prim::Line { w: 5, .. }))
            .collect();
        assert_eq!(heavy.len(), 2);
        assert_eq!(
            count(&dl, |p| matches!(
                p,
                Prim::Fill {
                    fill: FillKind::Slab,
                    ..
                }
            )),
            1
        );
        assert!(
            count(&dl, |p| matches!(
                p,
                Prim::Line {
                    dash: Dash::Center,
                    ..
                }
            )) >= 1
        );
        // Grid bubbles at both ends plus four elevation markers.
        assert_eq!(count(&dl, |p| matches!(p, Prim::Circle { .. })), 2 + 4);
    }

    #[test]
    fn level_2_plan_does_not_cut_level_1_walls() {
        let (doc, _) = building();
        let l2 = doc.levels()[1].0;
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l2),
        );
        let dl = display_list(&doc, v).unwrap();
        assert_eq!(
            count(&dl, |p| matches!(
                p,
                Prim::Fill {
                    fill: FillKind::Poche,
                    ..
                }
            )),
            0
        );
    }

    #[test]
    fn ceiling_plan_draws_act_grid() {
        let (doc, l1) = building();
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::CeilingPlan { level } if *level == l1),
        );
        let dl = display_list(&doc, v).unwrap();
        assert_eq!(
            count(&dl, |p| matches!(
                p,
                Prim::Fill {
                    fill: FillKind::Ceiling,
                    ..
                }
            )),
            1
        );
        // ~39'-4" × 29'-4" room: 14 horizontal (2') and 9 vertical (4') grid lines.
        let hatch = dl
            .items
            .iter()
            .filter(|i| matches!(&i.prim, Prim::Line { w: 1, pts, .. } if pts.len() == 2))
            .count();
        assert!((20..=26).contains(&hatch), "{hatch}");
    }

    #[test]
    fn south_elevation_shows_levels_and_hides_back_wall() {
        let (doc, _) = building();
        let v = view(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: studio_core::Compass::South
                }
            )
        });
        let dl = display_list(&doc, v).unwrap();
        assert_eq!(dl.view_type, ViewType::Elevation);
        // Level heads carry name and elevation text.
        let texts: Vec<_> = dl
            .items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"Level 2".to_string()) && texts.contains(&"10'-0\"".to_string()));
        // The south wall (nearest) is drawn after the north wall (farthest).
        let fills: Vec<_> = dl
            .items
            .iter()
            .filter(|i| {
                matches!(
                    i.prim,
                    Prim::Fill {
                        fill: FillKind::Paper,
                        ..
                    }
                )
            })
            .collect();
        let last = fills.last().unwrap();
        let south = regenerate(&doc)
            .walls
            .iter()
            .find(|w| w.start.y.abs() < 1.0 && w.end.y.abs() < 1.0)
            .unwrap()
            .id;
        assert_eq!(last.el, Some(south));
        // Picking the middle of the facade finds the south wall, not the one behind it.
        let mid = Pt::new(20.0 * MM_PER_FT, 1500.0);
        assert_eq!(pick(&dl, mid, 50.0), Some(south));
    }

    #[test]
    fn pick_prefers_lines_over_fills() {
        let (doc, l1) = building();
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let dl = display_list(&doc, v).unwrap();
        let grid = doc.of(Category::Grid).next().unwrap().id;
        assert_eq!(pick(&dl, Pt::new(0.0, -1500.0), 100.0), Some(grid));
        assert!(pick(&dl, Pt::new(-50_000.0, -50_000.0), 100.0).is_none());
        // The grid runs along the west wall's centerline; clicking inside the wall picks it.
        let west = regenerate(&doc)
            .walls
            .iter()
            .find(|w| w.start.x.abs() < 1.0 && w.end.x.abs() < 1.0)
            .unwrap()
            .id;
        assert_eq!(pick(&dl, Pt::new(10.0, 3000.0), 100.0), Some(west));
    }

    #[test]
    fn meshes_cover_walls_floor_and_ceiling() {
        let (doc, _) = building();
        let ms = meshes(&doc);
        assert_eq!(ms.len(), 6);
        assert!(ms
            .iter()
            .all(|m| !m.positions.is_empty() && m.positions.len() % 9 == 0));
    }
}
