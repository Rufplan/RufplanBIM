//! View generation: display lists for plans, ceiling plans and elevations, 3D meshes,
//! picking and snapping. Display-list coordinates are model mm (plans: x east, y north;
//! elevations: u to the viewer's right, z up). Annotation sizes are paper mm × scale.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use studio_core::units::{format_area_sf, format_ft_in, MM_PER_IN};
use studio_core::{Category, CropBox, Document, ElementData, ElementId, ViewKind};
use studio_core::{DoorFamily, WindowFamily};
use studio_geom::{point_in_ring, project_to_segment, Pt};
use studio_regen::{bounds, regenerate, Model, OpeningKind, OpeningSolid};
use ts_rs::TS;

pub mod handles;
pub mod snap;
pub use handles::{
    align_delta, handles, offset_preview, ref_line, Grip, Handles, OffsetPreview, RefLine, TempDim,
};
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
    /// Lighter cut material, so layer lines show inside layered walls.
    PocheLight,
    /// Opaque paper-white (painter's hidden-line fill in elevations).
    Paper,
    /// Light slab tone (floors).
    Slab,
    /// Ceiling tone.
    Ceiling,
    /// Solid ink (level and elevation markers).
    Ink,
    /// Window glass.
    Glass,
    /// Room region: invisible, but selectable and highlighted when selected.
    Room,
    /// Brand accent (title blocks).
    Accent,
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
    /// Text with its height in model mm, rotated `angle` radians counter-clockwise.
    Text {
        at: [f64; 2],
        text: String,
        size: f64,
        anchor: Anchor,
        angle: f64,
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
    Section,
    Schedule,
    Sheet,
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

pub fn ring(pts: &[Pt]) -> Vec<[f64; 2]> {
    pts.iter().copied().map(a).collect()
}

pub struct Builder {
    pub items: Vec<Item>,
    /// Drawing scale denominator; paper sizes are multiplied by it.
    pub scale: f64,
}

impl Builder {
    pub fn new(scale: f64) -> Self {
        Self {
            items: vec![],
            scale,
        }
    }
    pub fn push(&mut self, el: Option<ElementId>, prim: Prim) {
        self.items.push(Item { el, prim });
    }
    pub fn line(&mut self, el: Option<ElementId>, pts: &[Pt], closed: bool, w: u8, dash: Dash) {
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
    pub fn fill(&mut self, el: Option<ElementId>, rings: Vec<Vec<[f64; 2]>>, fill: FillKind) {
        self.push(el, Prim::Fill { rings, fill });
    }
    /// Text sized in paper mm.
    pub fn text(
        &mut self,
        el: Option<ElementId>,
        at: Pt,
        text: String,
        paper_mm: f64,
        anchor: Anchor,
    ) {
        self.text_rot(el, at, text, paper_mm, anchor, 0.0);
    }
    /// Rotated text sized in paper mm.
    pub fn text_rot(
        &mut self,
        el: Option<ElementId>,
        at: Pt,
        text: String,
        paper_mm: f64,
        anchor: Anchor,
        angle: f64,
    ) {
        self.push(
            el,
            Prim::Text {
                at: a(at),
                text,
                size: paper_mm * self.scale,
                anchor,
                angle,
            },
        );
    }
    /// Circle sized in paper mm.
    pub fn circle(&mut self, el: Option<ElementId>, c: Pt, paper_r: f64, w: u8, filled: bool) {
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
    pub fn paper(&self, mm: f64) -> f64 {
        mm * self.scale
    }
}

/// Display lists of a document's views, by view, with the stamp they were made for.
#[derive(Default)]
struct DisplayCache(HashMap<ElementId, (u64, Arc<DisplayList>)>);
const CACHE_KEY: &str = "studio-views";

/// Generates the display list of a 2D view. Returns None for 3D views.
pub fn display_list(doc: &Document, view: ElementId) -> Option<DisplayList> {
    display_list_shared(doc, view).map(|d| (*d).clone())
}

/// Like [`display_list`], shared from the document's cache when the model is unchanged
/// (picking and hovering ask for it on every mouse move).
pub fn display_list_shared(doc: &Document, view: ElementId) -> Option<Arc<DisplayList>> {
    let cache = doc
        .derived()
        .get::<Mutex<DisplayCache>>(CACHE_KEY)
        .unwrap_or_else(|| {
            let c = Arc::new(Mutex::new(DisplayCache::default()));
            doc.derived().put(CACHE_KEY, c.clone());
            c
        });
    let stamp = doc.stamp();
    if let Some((s, dl)) = cache.lock().unwrap_or_else(|e| e.into_inner()).0.get(&view) {
        if *s == stamp {
            return Some(dl.clone());
        }
    }
    let dl = Arc::new(render(doc, view)?);
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .0
        .insert(view, (stamp, dl.clone()));
    Some(dl)
}

fn render(doc: &Document, view: ElementId) -> Option<DisplayList> {
    let ElementData::View {
        kind,
        scale,
        crop,
        show_crop,
        ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    let model = regenerate(doc);
    let mut b = Builder {
        items: vec![],
        scale: f64::from(*scale),
    };
    let (view_type, bounds) = match kind {
        ViewKind::FloorPlan { level } => (
            ViewType::Plan,
            plan(doc, &model, &mut b, view, *level, false),
        ),
        ViewKind::CeilingPlan { level } => (
            ViewType::CeilingPlan,
            plan(doc, &model, &mut b, view, *level, true),
        ),
        ViewKind::Elevation { facing } => (
            ViewType::Elevation,
            elevation(&model, &mut b, facing.look().scale(-1.0)),
        ),
        ViewKind::Section { start, end, depth } => {
            let d = end.sub(*start).norm();
            let cut = Cut {
                origin: *start,
                length: start.dist(*end),
                depth: *depth,
            };
            (
                ViewType::Section,
                projected(&model, &mut b, d.perp(), Some(&cut)),
            )
        }
        ViewKind::ThreeD | ViewKind::Schedule { .. } => return None,
    };
    annotations(doc, &mut b, view);
    let crop_margin = b.paper(8.0);
    let (items, bounds) = match crop {
        Some(c) => {
            let mut items = crop_items(b.items, c);
            if *show_crop {
                let r = [
                    c.min,
                    Pt::new(c.max.x, c.min.y),
                    c.max,
                    Pt::new(c.min.x, c.max.y),
                ];
                // Drawn with the view as its element: selectable, and left off sheets.
                items.push(Item {
                    el: Some(view),
                    prim: Prim::Line {
                        pts: ring(&r),
                        closed: true,
                        w: 1,
                        dash: Dash::Solid,
                    },
                });
            }
            let m = crop_margin;
            (items, [c.min.x - m, c.min.y - m, c.max.x + m, c.max.y + m])
        }
        None => (b.items, bounds),
    };
    Some(DisplayList {
        view_type,
        scale: *scale,
        bounds,
        items,
    })
}

/// Clips a segment to a box (Liang–Barsky); None when it lies outside.
fn clip_segment(a: Pt, b: Pt, c: &CropBox) -> Option<(Pt, Pt)> {
    let d = b.sub(a);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-d.x, a.x - c.min.x),
        (d.x, c.max.x - a.x),
        (-d.y, a.y - c.min.y),
        (d.y, c.max.y - a.y),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 <= t1).then(|| (a.add(d.scale(t0)), a.add(d.scale(t1))))
}

/// Everything in `items` that lies inside the crop region, cut at its edges.
fn crop_items(items: Vec<Item>, c: &CropBox) -> Vec<Item> {
    let mut out = vec![];
    let clip_ring = |r: &[[f64; 2]]| -> Vec<[f64; 2]> {
        let mut pts: Vec<Pt> = r.iter().map(|q| Pt::new(q[0], q[1])).collect();
        for (p0, n) in [
            (c.min, Pt::new(1.0, 0.0)),
            (c.max, Pt::new(-1.0, 0.0)),
            (c.min, Pt::new(0.0, 1.0)),
            (c.max, Pt::new(0.0, -1.0)),
        ] {
            if pts.len() < 3 {
                break;
            }
            pts = studio_geom::clip_half_plane(&pts, p0, n);
        }
        if pts.len() < 3 {
            vec![]
        } else {
            ring(&pts)
        }
    };
    for it in items {
        match it.prim {
            Prim::Line {
                pts,
                closed,
                w,
                dash,
            } => {
                let n = pts.len();
                let segs = if closed { n } else { n.saturating_sub(1) };
                let mut run: Vec<Pt> = vec![];
                let flush = |run: &mut Vec<Pt>, out: &mut Vec<Item>| {
                    if run.len() >= 2 {
                        out.push(Item {
                            el: it.el,
                            prim: Prim::Line {
                                pts: ring(run),
                                closed: false,
                                w,
                                dash,
                            },
                        });
                    }
                    run.clear();
                };
                let all_inside = pts.iter().all(|q| c.contains(Pt::new(q[0], q[1])));
                if all_inside {
                    out.push(Item {
                        el: it.el,
                        prim: Prim::Line {
                            pts,
                            closed,
                            w,
                            dash,
                        },
                    });
                    continue;
                }
                for i in 0..segs {
                    let (a, b) = (pts[i], pts[(i + 1) % n]);
                    match clip_segment(Pt::new(a[0], a[1]), Pt::new(b[0], b[1]), c) {
                        Some((p, q)) => {
                            if run.last().is_none_or(|l| l.dist(p) > 1e-6) {
                                flush(&mut run, &mut out);
                                run.push(p);
                            }
                            run.push(q);
                        }
                        None => flush(&mut run, &mut out),
                    }
                }
                flush(&mut run, &mut out);
            }
            Prim::Fill { rings, fill } => {
                let rings: Vec<Vec<[f64; 2]>> = rings.iter().map(|r| clip_ring(r)).collect();
                if rings.first().is_some_and(|r| !r.is_empty()) {
                    out.push(Item {
                        el: it.el,
                        prim: Prim::Fill {
                            rings: rings.into_iter().filter(|r| !r.is_empty()).collect(),
                            fill,
                        },
                    });
                }
            }
            Prim::Text { at, .. } | Prim::Circle { c: at, .. } => {
                if c.contains(Pt::new(at[0], at[1])) {
                    out.push(it);
                }
            }
        }
    }
    out
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
    view: ElementId,
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

    // Room regions (invisible, pickable), under the walls so walls win when clicked.
    if !ceiling {
        for r in model.rooms.iter().filter(|r| r.level == level) {
            if let Some(outline) = &r.boundary {
                b.fill(Some(r.id), vec![ring(outline)], FillKind::Room);
            }
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
    // Only the wall material at the cut height: door and window openings become gaps.
    let cut_pieces: Vec<(ElementId, &studio_geom::Poly)> = cut_walls
        .iter()
        .flat_map(|w| {
            w.pieces
                .iter()
                .filter(|p| p.z0 <= cut && p.z1 > cut)
                .map(move |p| (w.id, &p.base))
        })
        .collect();
    // Compound layers show at 1/4" = 1'-0" and larger (Revit's medium detail), on a
    // lighter cut fill so the layer lines read.
    let detail = b.scale <= 50.0;
    let layered = |id: &ElementId| {
        detail
            && cut_walls
                .iter()
                .any(|w| w.id == *id && !w.layers.is_empty())
    };
    for (id, base) in &cut_pieces {
        let fill = if layered(id) {
            FillKind::PocheLight
        } else {
            FillKind::Poche
        };
        b.fill(Some(*id), vec![ring(&base.outer)], fill);
    }
    if detail {
        for w in cut_walls.iter().filter(|w| !w.layers.is_empty()) {
            let n = w.dir().perp();
            for off in &w.layers {
                let (a, d) = (w.start.add(n.scale(*off)), w.dir());
                for p in w.pieces.iter().filter(|p| p.z0 <= cut && p.z1 > cut) {
                    if let Some((s, e)) = clip_line_convex(a, d, &p.base.outer) {
                        b.line(Some(w.id), &[s, e], false, 1, Dash::Solid);
                    }
                }
            }
        }
    }
    let merged = studio_geom::union_all(
        &cut_pieces
            .iter()
            .map(|(_, p)| (*p).clone())
            .collect::<Vec<_>>(),
    );
    for region in &merged {
        b.line(None, &region.outer, true, 5, Dash::Solid);
        for h in &region.holes {
            b.line(None, h, true, 5, Dash::Solid);
        }
    }

    let cut_ids: Vec<ElementId> = cut_walls.iter().map(|w| w.id).collect();
    for o in model
        .openings
        .iter()
        .filter(|o| cut_ids.contains(&o.host) && o.z0 <= cut && o.z1 > cut)
    {
        opening_symbol(b, Some(o.id), o);
    }
    if !ceiling {
        stairs_in_plan(b, model, level, elev, cut);
        roofs_in_plan(b, model, level, cut);
    }
    if !ceiling {
        // Tags are elements owned by this view (created on placement, movable, deletable).
        for e in doc.iter() {
            let ElementData::Tag {
                view: v,
                target,
                offset,
            } = &e.data
            else {
                continue;
            };
            if *v != view {
                continue;
            }
            if let Some(r) = model.rooms.iter().find(|r| r.id == *target) {
                room_tag(b, r, Some(e.id), *offset);
            } else if let Some(o) = model
                .openings
                .iter()
                .find(|o| o.id == *target && cut_ids.contains(&o.host))
            {
                opening_tag(doc, b, o, Some(e.id), *offset);
            }
        }
        section_markers(doc, b);
    }

    let (lo, hi) = plan_extents(model);
    let margin = b.paper(12.0);
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

/// The part of the line through `a` along `d` inside a convex ring, if any.
fn clip_line_convex(a: Pt, d: Pt, ring_pts: &[Pt]) -> Option<(Pt, Pt)> {
    let n = ring_pts.len();
    if n < 3 {
        return None;
    }
    let ccw = studio_geom::signed_area(ring_pts) > 0.0;
    let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
    for i in 0..n {
        let (p, q) = (ring_pts[i], ring_pts[(i + 1) % n]);
        let e = q.sub(p);
        // Inward normal of the edge.
        let inward = if ccw { e.perp() } else { e.perp().scale(-1.0) };
        let denom = d.dot(inward);
        let num = a.sub(p).dot(inward);
        if denom.abs() < 1e-12 {
            if num < 0.0 {
                return None;
            }
            continue;
        }
        let t = -num / denom;
        if denom > 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
    }
    (t0.is_finite() && t1.is_finite() && t1 - t0 > 0.5)
        .then(|| (a.add(d.scale(t0)), a.add(d.scale(t1))))
}

/// Stairs based on this level (treads up to the cut plane, a break line, and an UP
/// arrow) and stairs arriving from below (all treads and DN).
fn stairs_in_plan(b: &mut Builder, model: &Model, level: ElementId, elev: f64, cut: f64) {
    for s in &model.stairs {
        let up = s.base_level == level;
        let down = s.top_level == level;
        if !up && !down {
            continue;
        }
        let el = Some(s.id);
        let o = s.outline();
        b.fill(el, vec![ring(&o)], FillKind::Room);
        let n = s.dir.perp().scale(s.width / 2.0);
        let run = s.tread * s.steps.len() as f64;
        // Where the cut plane crosses the run (distance from the first riser).
        let cut_at = if up {
            (((cut - elev) / s.riser).floor() * s.tread).clamp(s.tread, run)
        } else {
            run
        };
        for k in 0..=s.steps.len() {
            let t = k as f64 * s.tread;
            let p = s.start.add(s.dir.scale(t));
            let dash = if up && t > cut_at + 1.0 {
                Dash::Dashed
            } else {
                Dash::Solid
            };
            b.line(el, &[p.add(n), p.sub(n)], false, 1, dash);
        }
        let side = |a: Pt, t0: f64, t1: f64, dash: Dash, b: &mut Builder| {
            b.line(
                el,
                &[a.add(s.dir.scale(t0)), a.add(s.dir.scale(t1))],
                false,
                2,
                dash,
            );
        };
        for edge in [s.start.add(n), s.start.sub(n)] {
            side(edge, 0.0, cut_at, Dash::Solid, b);
            if cut_at < run {
                side(edge, cut_at, run, Dash::Dashed, b);
            }
        }
        if up && cut_at < run {
            // Diagonal break line across the run at the cut.
            let c = s.start.add(s.dir.scale(cut_at));
            let skew = s.dir.scale(s.tread * 0.8);
            b.line(
                el,
                &[c.add(n).add(skew), c.sub(n).sub(skew)],
                false,
                2,
                Dash::Solid,
            );
        }
        // Walking line with an arrowhead, UP from this level or DN from above.
        let (from, to) = if up {
            (
                s.start.add(s.dir.scale(s.tread * 0.5)),
                s.start.add(s.dir.scale(cut_at.min(run) - s.tread * 0.5)),
            )
        } else {
            (
                s.start.add(s.dir.scale(run - s.tread * 0.5)),
                s.start.add(s.dir.scale(s.tread * 0.5)),
            )
        };
        b.line(el, &[from, to], false, 1, Dash::Solid);
        let back = from.sub(to).norm();
        let head = b.paper(2.0);
        let wing = back.perp().scale(head * 0.5);
        b.line(
            el,
            &[
                to.add(back.scale(head)).add(wing),
                to,
                to.add(back.scale(head)).sub(wing),
            ],
            false,
            1,
            Dash::Solid,
        );
        b.text(
            el,
            from.sub(back.scale(b.paper(0.5)))
                .sub(s.dir.scale(b.paper(3.0))),
            if up { "UP".into() } else { "DN".into() },
            2.5,
            Anchor::Center,
        );
    }
}

/// Roofs based on this level, as in a roof plan: eave outline, hips and ridges. Dashed
/// when the roof is above the cut plane (seen overhead).
fn roofs_in_plan(b: &mut Builder, model: &Model, level: ElementId, cut: f64) {
    for r in model.roofs.iter().filter(|r| r.level == level) {
        let el = Some(r.id);
        let dash = if r.base > cut {
            Dash::Dashed
        } else {
            Dash::Solid
        };
        b.fill(el, vec![ring(&r.boundary)], FillKind::Room);
        for f in &r.faces {
            b.line(el, &f.poly, true, 1, dash);
        }
        b.line(el, &r.boundary, true, 2, dash);
    }
}

/// Points on a circular arc from angle `a0` sweeping `sweep` radians.
fn arc(c: Pt, r: f64, a0: f64, sweep: f64) -> Vec<Pt> {
    let n = 18;
    (0..=n)
        .map(|i| {
            let a = a0 + sweep * f64::from(i) / f64::from(n);
            c.add(Pt::new(a.cos(), a.sin()).scale(r))
        })
        .collect()
}

/// Plan symbol of a door (leaf + swing arc) or window (sill and glass lines).
fn opening_symbol(b: &mut Builder, el: Option<ElementId>, o: &OpeningSolid) {
    let d = o.dir;
    let n = d.perp();
    let h = o.half_thickness;
    let (j0, j1) = (o.at(o.t0), o.at(o.t1));
    match o.kind {
        OpeningKind::Door(family) => {
            // Swing side: the wall's left (+n) unless flipped.
            let s = if o.flip_facing { -1.0 } else { 1.0 };
            let out = n.scale(s);
            let face = out.scale(h);
            let leaves: Vec<(Pt, Pt, f64)> = match family {
                DoorFamily::SingleFlush => {
                    let (hinge, other) = if o.flip_hand { (j1, j0) } else { (j0, j1) };
                    vec![(hinge, other, o.width())]
                }
                DoorFamily::DoubleFlush => {
                    let half = o.width() / 2.0;
                    vec![(j0, j1, half), (j1, j0, half)]
                }
            };
            for (hinge, other, r) in leaves {
                let hf = hinge.add(face);
                let tip = hf.add(out.scale(r));
                b.line(el, &[hf, tip], false, 3, Dash::Solid);
                let closing = other.sub(hinge).norm();
                let a0 = out.y.atan2(out.x);
                let a1 = closing.y.atan2(closing.x);
                let mut sweep = a1 - a0;
                while sweep > std::f64::consts::PI {
                    sweep -= std::f64::consts::TAU;
                }
                while sweep < -std::f64::consts::PI {
                    sweep += std::f64::consts::TAU;
                }
                b.line(el, &arc(hf, r, a0, sweep), false, 1, Dash::Solid);
            }
        }
        OpeningKind::Window(family) => {
            for off in [h, -h] {
                b.line(
                    el,
                    &[j0.add(n.scale(off)), j1.add(n.scale(off))],
                    false,
                    1,
                    Dash::Solid,
                );
            }
            let g = h * 0.22;
            for off in [g, -g] {
                b.line(
                    el,
                    &[j0.add(n.scale(off)), j1.add(n.scale(off))],
                    false,
                    2,
                    Dash::Solid,
                );
            }
            if family == WindowFamily::Casement {
                // Sash swing indicator, outward.
                let s = if o.flip_facing { -1.0 } else { 1.0 };
                let out = n.scale(s);
                let tip = j0
                    .add(out.scale(h + o.width() * 0.35))
                    .add(d.scale(o.width() * 0.2));
                b.line(el, &[j0.add(out.scale(h)), tip], false, 1, Dash::Dashed);
            }
        }
    }
}

/// Door tag (mark in a rectangle) on the side away from the swing; window tag (mark in a
/// hexagon) on the window's facing side.
fn opening_tag(
    doc: &Document,
    b: &mut Builder,
    o: &OpeningSolid,
    el: Option<ElementId>,
    offset: Pt,
) {
    let mark = match doc.data(o.id) {
        Ok(ElementData::Door { mark, .. } | ElementData::Window { mark, .. }) => mark.clone(),
        _ => return,
    };
    let n = o.dir.perp();
    let s = if o.flip_facing { -1.0 } else { 1.0 };
    let mid = o.at((o.t0 + o.t1) / 2.0).add(offset);
    match o.kind {
        OpeningKind::Door(_) => {
            let c = mid.sub(n.scale(s * (o.half_thickness + b.paper(5.0))));
            let (hw, hh) = (b.paper(4.0), b.paper(2.6));
            let r = [
                c.add(Pt::new(-hw, -hh)),
                c.add(Pt::new(hw, -hh)),
                c.add(Pt::new(hw, hh)),
                c.add(Pt::new(-hw, hh)),
            ];
            b.fill(el, vec![ring(&r)], FillKind::Paper);
            b.line(el, &r, true, 2, Dash::Solid);
            b.text(el, c, mark, 2.6, Anchor::Center);
        }
        OpeningKind::Window(_) => {
            let c = mid.add(n.scale(s * (o.half_thickness + b.paper(6.0))));
            let r = b.paper(3.4);
            let hex: Vec<Pt> = (0..6)
                .map(|i| {
                    let t = std::f64::consts::PI / 3.0 * f64::from(i);
                    c.add(Pt::new(t.cos() * r, t.sin() * r * 0.8))
                })
                .collect();
            b.fill(el, vec![ring(&hex)], FillKind::Paper);
            b.line(el, &hex, true, 2, Dash::Solid);
            b.text(el, c, mark, 2.4, Anchor::Center);
        }
    }
}

/// Section lines with heads in plan views, each linked to its section view.
fn section_markers(doc: &Document, b: &mut Builder) {
    for v in doc.of(Category::View) {
        let ElementData::View {
            name,
            kind: ViewKind::Section { start, end, .. },
            ..
        } = &v.data
        else {
            continue;
        };
        let el = Some(v.id);
        let d = end.sub(*start).norm();
        let look = d.perp();
        let r = b.paper(5.0);
        b.line(el, &[*start, *end], false, 1, Dash::Center);
        let seg = b.paper(8.0);
        b.line(
            el,
            &[*start, start.add(d.scale(seg))],
            false,
            5,
            Dash::Solid,
        );
        b.line(el, &[*end, end.sub(d.scale(seg))], false, 5, Dash::Solid);
        let c = start.sub(d.scale(r));
        b.circle(el, c, 5.0, 2, false);
        let tip = c.add(look.scale(r * 1.8));
        b.fill(
            el,
            vec![ring(&[tip, c.add(d.scale(r)), c.sub(d.scale(r))])],
            FillKind::Ink,
        );
        b.circle(el, c, 5.0, 2, false);
        let label: String = name.chars().filter(|ch| ch.is_ascii_digit()).collect();
        b.text(
            el,
            c,
            if label.is_empty() { "S".into() } else { label },
            3.4,
            Anchor::Center,
        );
    }
}

/// Room tag at the room's point: name, number and area (or "Not Enclosed").
fn room_tag(b: &mut Builder, r: &studio_regen::RoomInfo, el: Option<ElementId>, offset: Pt) {
    let p = r.point.add(offset);
    let line = b.paper(4.2);
    b.text(
        el,
        p.add(Pt::new(0.0, line)),
        r.name.to_uppercase(),
        3.4,
        Anchor::Center,
    );
    b.text(el, p, r.number.clone(), 3.0, Anchor::Center);
    if r.boundary.is_some() {
        b.text(
            el,
            p.sub(Pt::new(0.0, line)),
            format_area_sf(r.area()),
            2.6,
            Anchor::Center,
        );
    } else {
        b.text(
            el,
            p.sub(Pt::new(0.0, line)),
            "NOT ENCLOSED".into(),
            2.6,
            Anchor::Center,
        );
        let k = b.paper(2.0);
        b.line(
            el,
            &[
                p.add(Pt::new(-k, -k - line * 2.0)),
                p.add(Pt::new(k, k - line * 2.0)),
            ],
            false,
            2,
            Dash::Solid,
        );
        b.line(
            el,
            &[
                p.add(Pt::new(-k, k - line * 2.0)),
                p.add(Pt::new(k, -k - line * 2.0)),
            ],
            false,
            2,
            Dash::Solid,
        );
    }
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
    // Just outside the grid bubbles, so viewports on sheets stay compact.
    let off = margin + b.paper(14.0);
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

/// A section's cut plane: the line from `origin` (length mm) and how far beyond it to show.
pub struct Cut {
    pub origin: Pt,
    pub length: f64,
    pub depth: f64,
}

#[derive(PartialEq)]
enum Seen {
    Beyond,
    Cut,
    Hidden,
}

fn elevation(model: &Model, b: &mut Builder, look: Pt) -> [f64; 4] {
    projected(model, b, look, None)
}

/// Elevation (no cut) or section (cut plane with far clip), seen looking along `look`.
fn projected(model: &Model, b: &mut Builder, look: Pt, cut: Option<&Cut>) -> [f64; 4] {
    let right = Pt::new(look.y, -look.x);
    let origin = cut.map_or(Pt::default(), |c| c.origin);
    let u_of = |p: Pt| p.sub(origin).dot(right);
    let depth_of = |p: Pt| p.sub(origin).dot(look);
    let seen = |pts: &[Pt]| -> Seen {
        let Some(c) = cut else { return Seen::Beyond };
        let ds: Vec<f64> = pts.iter().map(|p| depth_of(*p)).collect();
        let us: Vec<f64> = pts.iter().map(|p| u_of(*p)).collect();
        let (dmin, dmax) = (
            ds.iter().copied().fold(f64::INFINITY, f64::min),
            ds.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        let (umin, umax) = (
            us.iter().copied().fold(f64::INFINITY, f64::min),
            us.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        if dmax <= 0.0 || dmin >= c.depth || umax <= 0.0 || umin >= c.length {
            Seen::Hidden
        } else if dmin < 0.0 {
            Seen::Cut
        } else {
            Seen::Beyond
        }
    };
    // Where a polygon crosses the cut plane, as a u-interval clipped to the section width.
    let cut_interval = |pts: &[Pt]| -> Option<(f64, f64)> {
        let c = cut?;
        let n = pts.len();
        let mut us = vec![];
        for i in 0..n {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            let (dp, dq) = (depth_of(p), depth_of(q));
            if (dp < 0.0) != (dq < 0.0) {
                us.push(u_of(p.lerp(q, dp / (dp - dq))));
            }
        }
        let lo = us.iter().copied().fold(f64::INFINITY, f64::min).max(0.0);
        let hi = us
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            .min(c.length);
        (us.len() >= 2 && hi - lo > 0.5).then_some((lo, hi))
    };
    let mut cut_rects: Vec<(ElementId, f64, f64, f64, f64)> = vec![];

    struct Face {
        el: ElementId,
        u0: f64,
        u1: f64,
        z0: f64,
        z1: f64,
        near: f64,
        mid: f64,
        fill: FillKind,
        detail: Option<OpeningKind>,
        /// A sloped face's outline in (u, z); rectangles leave this empty.
        poly: Option<Vec<Pt>>,
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
            detail: None,
            poly: None,
        }
    };
    let mut faces: Vec<Face> = vec![];
    // Cut profiles that aren't rectangles (sloped roofs), drawn with the cut rectangles.
    let mut cut_polys: Vec<(ElementId, Vec<Pt>)> = vec![];
    // Whole walls, not their pieces: every opening is drawn on top with its own door or
    // glass fill, and piece seams would read as false joints in the facade.
    for w in &model.walls {
        match seen(&w.footprint.outer) {
            Seen::Beyond => faces.push(face(w.id, &w.footprint.outer, w.z0, w.z1, FillKind::Paper)),
            Seen::Cut => {
                for p in &w.pieces {
                    if let Some((u0, u1)) = cut_interval(&p.base.outer) {
                        cut_rects.push((w.id, u0, u1, p.z0, p.z1));
                    }
                }
            }
            Seen::Hidden => {}
        }
    }
    for o in &model.openings {
        let Some(host) = model.walls.iter().find(|w| w.id == o.host) else {
            continue;
        };
        if seen(&host.footprint.outer) != Seen::Beyond {
            continue; // Openings of cut walls show as gaps between the cut pieces.
        }
        let host_near = host
            .footprint
            .outer
            .iter()
            .map(|p| depth_of(*p))
            .fold(f64::INFINITY, f64::min);
        let fill = if matches!(o.kind, OpeningKind::Door(_)) {
            FillKind::Paper
        } else {
            FillKind::Glass
        };
        let mut f = face(o.id, &[o.at(o.t0), o.at(o.t1)], o.z0, o.z1, fill);
        if f.u1 - f.u0 < 1.0 {
            continue; // Opening in a wall seen edge-on.
        }
        // Drawn just after its host wall so the host's pieces frame it.
        f.near = host_near - 0.5;
        f.mid = f.near;
        f.detail = Some(o.kind);
        faces.push(f);
    }
    for (slabs, fill) in [
        (&model.floors, FillKind::Slab),
        (&model.ceilings, FillKind::Paper),
    ] {
        for f in slabs {
            match seen(&f.base.outer) {
                Seen::Beyond => faces.push(face(f.id, &f.base.outer, f.z0, f.z1, fill)),
                Seen::Cut => {
                    if let Some((u0, u1)) = cut_interval(&f.base.outer) {
                        cut_rects.push((f.id, u0, u1, f.z0, f.z1));
                    }
                }
                Seen::Hidden => {}
            }
        }
    }
    for s in &model.stairs {
        for step in &s.steps {
            match seen(&step.base.outer) {
                Seen::Beyond => faces.push(face(
                    s.id,
                    &step.base.outer,
                    step.z0,
                    step.z1,
                    FillKind::Paper,
                )),
                Seen::Cut => {
                    if let Some((u0, u1)) = cut_interval(&step.base.outer) {
                        cut_rects.push((s.id, u0, u1, step.z0, step.z1));
                    }
                }
                Seen::Hidden => {}
            }
        }
    }
    // A 3D polygon as a face in (u, z), with its depth for the painter's sort.
    let poly_face = |el: ElementId, poly3: &[[f64; 3]]| -> Option<Face> {
        if poly3.len() < 3 {
            return None;
        }
        let uz: Vec<Pt> = poly3
            .iter()
            .map(|v| Pt::new(u_of(Pt::new(v[0], v[1])), v[2]))
            .collect();
        if studio_geom::signed_area(&uz).abs() < 1.0 {
            return None; // Seen edge-on.
        }
        let depths: Vec<f64> = poly3
            .iter()
            .map(|v| depth_of(Pt::new(v[0], v[1])))
            .collect();
        Some(Face {
            el,
            u0: uz.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
            u1: uz.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
            z0: uz.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
            z1: uz.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
            near: depths.iter().copied().fold(f64::INFINITY, f64::min),
            mid: depths.iter().sum::<f64>() / depths.len() as f64,
            fill: FillKind::Paper,
            detail: None,
            poly: Some(uz),
        })
    };
    for r in &model.roofs {
        match seen(&r.boundary) {
            Seen::Beyond => {
                for s in r.surfaces() {
                    faces.extend(poly_face(r.id, &s));
                }
            }
            Seen::Cut => {
                // What lies beyond the cut plane, then the cut profile.
                for s in r.surfaces() {
                    let kept = clip3(&s, depth_of);
                    faces.extend(poly_face(r.id, &kept));
                }
                let t = if r.is_flat() {
                    r.thickness
                } else {
                    r.plumb_thickness()
                };
                if r.is_flat() {
                    if let Some((u0, u1)) = cut_interval(&r.boundary) {
                        cut_rects.push((r.id, u0, u1, r.base, r.base + t));
                    }
                }
                for f in &r.faces {
                    // Where the face's top surface crosses the cut plane.
                    let n = f.poly.len();
                    let mut hits: Vec<Pt> = vec![];
                    for i in 0..n {
                        let (a, b) = (f.poly[i], f.poly[(i + 1) % n]);
                        let (da, db) = (depth_of(a), depth_of(b));
                        if (da < 0.0) != (db < 0.0) {
                            let q = a.lerp(b, da / (da - db));
                            hits.push(Pt::new(u_of(q), r.face_top(f, q)));
                        }
                    }
                    if hits.len() >= 2 {
                        let (p, q) = (hits[0], hits[1]);
                        let lo = |v: Pt| Pt::new(v.x, v.y - t);
                        cut_polys.push((r.id, vec![p, q, lo(q), lo(p)]));
                    }
                }
            }
            Seen::Hidden => {}
        }
    }
    // Painter's algorithm: farthest first so nearer faces cover what they hide. Mitered
    // corners make side walls reach as near as the facade, so ties break on average depth.
    faces.sort_by(|a, b| b.near.total_cmp(&a.near).then(b.mid.total_cmp(&a.mid)));

    let (plo, phi) = model
        .plan_bounds()
        .unwrap_or((Pt::new(0.0, 0.0), Pt::new(12000.0, 9000.0)));
    let corners = [plo, phi, Pt::new(plo.x, phi.y), Pt::new(phi.x, plo.y)];
    let (umin, umax) = match cut {
        Some(c) => (0.0, c.length),
        None => (
            corners
                .iter()
                .map(|p| u_of(*p))
                .fold(f64::INFINITY, f64::min),
            corners
                .iter()
                .map(|p| u_of(*p))
                .fold(f64::NEG_INFINITY, f64::max),
        ),
    };
    let (zmin, zmax) = model.z_range();

    for f in &faces {
        let r = match &f.poly {
            Some(p) => p.clone(),
            None => vec![
                Pt::new(f.u0, f.z0),
                Pt::new(f.u1, f.z0),
                Pt::new(f.u1, f.z1),
                Pt::new(f.u0, f.z1),
            ],
        };
        b.fill(Some(f.el), vec![ring(&r)], f.fill);
        b.line(Some(f.el), &r, true, 2, Dash::Solid);
        if let Some(kind) = f.detail {
            opening_elevation_detail(b, f.el, kind, f.u0, f.u1, f.z0, f.z1);
        }
    }
    // Cut material over everything beyond it.
    for (el, u0, u1, z0, z1) in &cut_rects {
        let r = [
            Pt::new(*u0, *z0),
            Pt::new(*u1, *z0),
            Pt::new(*u1, *z1),
            Pt::new(*u0, *z1),
        ];
        b.fill(Some(*el), vec![ring(&r)], FillKind::Poche);
        b.line(Some(*el), &r, true, 4, Dash::Solid);
    }
    for (el, r) in &cut_polys {
        b.fill(Some(*el), vec![ring(r)], FillKind::Poche);
        b.line(Some(*el), r, true, 4, Dash::Solid);
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
        let u = match cut {
            // Sections show grids that cross the cut line.
            Some(c) => {
                let Some(x) = studio_geom::line_intersection(g.start, d, origin, right) else {
                    continue;
                };
                let on_grid = project_to_segment(x, g.start, g.end).1 < 1.0;
                let u = u_of(x);
                if !on_grid || u < 0.0 || u > c.length {
                    continue;
                }
                u
            }
            None => {
                if d.dot(right).abs() > 0.05 {
                    continue; // Only grids running along the view direction appear as lines.
                }
                u_of(g.start)
            }
        };
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

/// The part of a planar 3D polygon where `depth` (of its plan position) is ≥ 0.
fn clip3(poly: &[[f64; 3]], depth: impl Fn(Pt) -> f64) -> Vec<[f64; 3]> {
    let d: Vec<f64> = poly.iter().map(|v| depth(Pt::new(v[0], v[1]))).collect();
    let n = poly.len();
    let mut out = vec![];
    for i in 0..n {
        let j = (i + 1) % n;
        if d[i] >= 0.0 {
            out.push(poly[i]);
        }
        if (d[i] >= 0.0) != (d[j] >= 0.0) {
            let t = d[i] / (d[i] - d[j]);
            let (a, b) = (poly[i], poly[j]);
            out.push([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]);
        }
    }
    out
}

/// Frame, panel and swing lines of a door or window seen in elevation.
fn opening_elevation_detail(
    b: &mut Builder,
    el: ElementId,
    kind: OpeningKind,
    u0: f64,
    u1: f64,
    z0: f64,
    z1: f64,
) {
    let el = Some(el);
    let frame = 2.0 * MM_PER_IN;
    match kind {
        OpeningKind::Door(family) => {
            if family == DoorFamily::DoubleFlush {
                let m = (u0 + u1) / 2.0;
                b.line(
                    el,
                    &[Pt::new(m, z0), Pt::new(m, z1 - frame)],
                    false,
                    1,
                    Dash::Solid,
                );
            }
            let r = [
                Pt::new(u0 + frame, z0),
                Pt::new(u0 + frame, z1 - frame),
                Pt::new(u1 - frame, z1 - frame),
                Pt::new(u1 - frame, z0),
            ];
            b.line(el, &r, false, 1, Dash::Solid);
        }
        OpeningKind::Window(family) => {
            let r = [
                Pt::new(u0 + frame, z0 + frame),
                Pt::new(u1 - frame, z0 + frame),
                Pt::new(u1 - frame, z1 - frame),
                Pt::new(u0 + frame, z1 - frame),
            ];
            b.line(el, &r, true, 1, Dash::Solid);
            if family == WindowFamily::Casement {
                // Swing indicator: hinge on the left, point to the right side's midpoint.
                let tip = Pt::new(u1 - frame, (z0 + z1) / 2.0);
                b.line(
                    el,
                    &[
                        Pt::new(u0 + frame, z0 + frame),
                        tip,
                        Pt::new(u0 + frame, z1 - frame),
                    ],
                    false,
                    1,
                    Dash::Dashed,
                );
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomPreview {
    /// Outline of the enclosed area the room would fill; empty when not enclosed.
    pub items: Vec<Item>,
    /// Area of that region, or why a room can't go here.
    pub label: String,
    /// True when a room can be placed here.
    pub valid: bool,
}

/// What a room placed at `p` in a floor plan would fill.
pub fn room_preview(doc: &Document, view: ElementId, p: Pt) -> Option<RoomPreview> {
    let ElementData::View {
        kind: ViewKind::FloorPlan { level },
        scale,
        ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    let model = regenerate(doc);
    let Some(area) = studio_regen::room_at(&model, *level, p) else {
        return Some(RoomPreview {
            items: vec![],
            label: "Not enclosed by walls".into(),
            valid: false,
        });
    };
    let mut b = Builder {
        items: vec![],
        scale: f64::from(*scale),
    };
    b.line(None, &area, true, 3, Dash::Solid);
    let (label, valid) = match studio_regen::room_occupying(&model, *level, p) {
        Some(r) => (format!("Already room {} {}", r.name, r.number), false),
        None => (format_area_sf(studio_geom::signed_area(&area).abs()), true),
    };
    Some(RoomPreview {
        items: b.items,
        label,
        valid,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpeningPreview {
    pub host: ElementId,
    /// Center distance from the host's start, mm.
    pub offset: f64,
    pub flip_facing: bool,
    /// False when the opening would overlap another one in the wall.
    pub valid: bool,
    /// Distances from the opening's edges to the wall ends.
    pub label: String,
    pub items: Vec<Item>,
}

/// Where a door or window of `type_id` would go for a cursor at `p` in a plan view.
pub fn opening_preview(
    doc: &Document,
    view: ElementId,
    type_id: ElementId,
    p: Pt,
    tol: f64,
) -> Option<OpeningPreview> {
    let ElementData::View {
        kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
        scale,
        ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    let (width, is_door) = match doc.data(type_id).ok()? {
        ElementData::DoorType { width, .. } => (*width, true),
        ElementData::WindowType { width, .. } => (*width, false),
        _ => return None,
    };
    let model = regenerate(doc);
    let elev = doc.level_elevation(*level).ok()?;
    let cut = elev + PLAN_CUT;
    let (wall, t, dist) = model
        .walls
        .iter()
        .filter(|w| w.z0 <= cut && w.z1 > cut)
        .map(|w| {
            let (t, d) = project_to_segment(p, w.start, w.end);
            (w, t, d)
        })
        .filter(|(w, _, d)| *d <= w.thickness / 2.0 + tol)
        .min_by(|a, b| a.2.total_cmp(&b.2))?;
    let _ = dist;
    let len = wall.start.dist(wall.end);
    if len < width {
        return None;
    }
    let hw = width / 2.0;
    let mut offset = (t * len).clamp(hw, len - hw);
    if (offset - len / 2.0).abs() < tol {
        offset = len / 2.0; // Snap to the wall's center.
    } else {
        // Keep the gap to the wall start a whole number of inches.
        let left = ((offset - hw) / MM_PER_IN).round() * MM_PER_IN;
        offset = (left + hw).clamp(hw, len - hw);
    }
    let dir = wall.dir();
    let flip_facing = p.sub(wall.start).dot(dir.perp()) < 0.0;
    let valid = !model
        .openings
        .iter()
        .any(|o| o.host == wall.id && offset - hw < o.t1 - 0.5 && offset + hw > o.t0 + 0.5);
    let temp = OpeningSolid {
        id: wall.id,
        host: wall.id,
        kind: if is_door {
            match doc.data(type_id).ok()? {
                ElementData::DoorType { family, .. } => OpeningKind::Door(*family),
                _ => return None,
            }
        } else {
            match doc.data(type_id).ok()? {
                ElementData::WindowType { family, .. } => OpeningKind::Window(*family),
                _ => return None,
            }
        },
        wall_start: wall.start,
        dir,
        half_thickness: wall.thickness / 2.0,
        t0: offset - hw,
        t1: offset + hw,
        z0: 0.0,
        z1: 0.0,
        flip_hand: false,
        flip_facing,
    };
    let mut b = Builder {
        items: vec![],
        scale: f64::from(*scale),
    };
    let n = dir.perp().scale(wall.thickness / 2.0);
    let (j0, j1) = (temp.at(temp.t0), temp.at(temp.t1));
    b.line(
        None,
        &[j0.add(n), j1.add(n), j1.sub(n), j0.sub(n)],
        true,
        2,
        Dash::Solid,
    );
    opening_symbol(&mut b, None, &temp);
    let label = format!(
        "{}  ◂▸  {}",
        format_ft_in(offset - hw),
        format_ft_in(len - offset - hw)
    );
    Some(OpeningPreview {
        host: wall.id,
        offset,
        flip_facing,
        valid,
        label,
        items: b.items,
    })
}

/// Dimensions and text notes owned by `view`.
pub fn annotations(doc: &Document, b: &mut Builder, view: ElementId) {
    for e in doc.iter() {
        match &e.data {
            ElementData::Dimension {
                view: v, offset, ..
            } if *v == view => {
                if let Some((a, p2)) = studio_core::ops::dimension_ends(doc, &e.data) {
                    dimension(b, Some(e.id), a, p2, *offset);
                }
            }
            ElementData::TextNote {
                view: v,
                at,
                text,
                size,
            } if *v == view => {
                b.text(Some(e.id), *at, text.clone(), *size, Anchor::Left);
            }
            _ => {}
        }
    }
}

/// An aligned dimension: witness lines, dimension line with architectural ticks, and the
/// length in feet-inches above the line, kept upright.
pub fn dimension(b: &mut Builder, el: Option<ElementId>, a: Pt, p2: Pt, offset: f64) {
    let u = p2.sub(a).norm();
    let n = u.perp();
    let (da, db) = (a.add(n.scale(offset)), p2.add(n.scale(offset)));
    let side = if offset < 0.0 { -1.0 } else { 1.0 };
    let gap = b.paper(1.5) * side;
    let ext = b.paper(2.0) * side;
    if offset.abs() > gap.abs() {
        b.line(
            el,
            &[a.add(n.scale(gap)), da.add(n.scale(ext))],
            false,
            1,
            Dash::Solid,
        );
        b.line(
            el,
            &[p2.add(n.scale(gap)), db.add(n.scale(ext))],
            false,
            1,
            Dash::Solid,
        );
    }
    let over = b.paper(2.0);
    b.line(
        el,
        &[da.sub(u.scale(over)), db.add(u.scale(over))],
        false,
        1,
        Dash::Solid,
    );
    let t = u.add(n).norm().scale(b.paper(1.5));
    for p in [da, db] {
        b.line(el, &[p.sub(t), p.add(t)], false, 4, Dash::Solid);
    }
    let mut angle = u.y.atan2(u.x);
    let mut up = n;
    if angle > std::f64::consts::FRAC_PI_2 + 1e-9 || angle <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
        angle += if angle > 0.0 {
            -std::f64::consts::PI
        } else {
            std::f64::consts::PI
        };
        up = n.scale(-1.0);
    }
    let mid = da.lerp(db, 0.5).add(up.scale(b.paper(2.2)));
    b.text_rot(
        el,
        mid,
        format_ft_in(a.dist(p2)),
        2.6,
        Anchor::Center,
        angle,
    );
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct DimensionPreview {
    pub offset: f64,
    pub items: Vec<Item>,
}

/// The dimension a→b with its line through `cursor`, for the dimension tool.
pub fn dimension_preview(
    doc: &Document,
    view: ElementId,
    a: Pt,
    p2: Pt,
    cursor: Pt,
) -> Option<DimensionPreview> {
    let ElementData::View { scale, .. } = doc.data(view).ok()? else {
        return None;
    };
    if a.dist(p2) < 1.0 {
        return None;
    }
    let offset = cursor.sub(a).dot(p2.sub(a).norm().perp());
    let mut b = Builder::new(f64::from(*scale));
    dimension(&mut b, None, a, p2, offset);
    Some(DimensionPreview {
        offset,
        items: b.items,
    })
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
                } else if matches!(fill, FillKind::Slab | FillKind::Ceiling | FillKind::Room) {
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
        let positions = w.pieces.iter().flat_map(|p| p.triangles()).collect();
        out.push(Mesh {
            el: w.id,
            category: Category::Wall,
            exterior: w.exterior,
            positions,
        });
    }
    for o in &m.openings {
        let (category, depth) = match o.kind {
            OpeningKind::Door(_) => (Category::Door, 1.75 * MM_PER_IN),
            OpeningKind::Window(_) => (Category::Window, 0.75 * MM_PER_IN),
        };
        out.push(Mesh {
            el: o.id,
            category,
            exterior: false,
            positions: o.panel(depth, o.z0, o.z1).triangles(),
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
    for r in &m.roofs {
        out.push(Mesh {
            el: r.id,
            category: Category::Roof,
            exterior: true,
            positions: r.triangles(),
        });
    }
    for s in &m.stairs {
        out.push(Mesh {
            el: s.id,
            category: Category::Stair,
            exterior: false,
            positions: s.steps.iter().flat_map(|p| p.triangles()).collect(),
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
    use studio_core::Compass;

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
                    fill: FillKind::Poche | FillKind::PocheLight,
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
                    fill: FillKind::Poche | FillKind::PocheLight,
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
        let ceiling = dl
            .items
            .iter()
            .find(|i| {
                matches!(
                    i.prim,
                    Prim::Fill {
                        fill: FillKind::Ceiling,
                        ..
                    }
                )
            })
            .and_then(|i| i.el);
        let hatch = dl
            .items
            .iter()
            .filter(|i| i.el == ceiling)
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

    fn with_openings() -> (Document, ElementId, ElementId, ElementId) {
        let (mut doc, l1) = building();
        let south = regenerate(&doc)
            .walls
            .iter()
            .find(|w| w.start.y.abs() < 1.0 && w.end.y.abs() < 1.0)
            .unwrap()
            .id;
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let wn = doc
            .of(Category::WindowType)
            .find(|e| e.data.name().starts_with("Casement"))
            .unwrap()
            .id;
        let d = ops::create_door(&mut doc, dt, south, 3000.0, false).unwrap();
        let w = ops::create_window(&mut doc, wn, south, 8000.0, false).unwrap();
        let _ = l1;
        (doc, south, d, w)
    }

    #[test]
    fn plan_shows_gaps_and_symbols_for_openings() {
        let (doc, south, d, w) = with_openings();
        let l1 = doc.levels()[0].0;
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let dl = display_list(&doc, v).unwrap();
        // The south wall is split into three poché pieces at the cut.
        let south_fills = dl
            .items
            .iter()
            .filter(|i| i.el == Some(south) && matches!(i.prim, Prim::Fill { .. }))
            .count();
        assert_eq!(south_fills, 3);
        // Door: leaf + arc. Casement window: 2 sill + 2 glass + swing line. (Their tags are
        // separate Tag elements.)
        assert_eq!(dl.items.iter().filter(|i| i.el == Some(d)).count(), 2);
        assert_eq!(dl.items.iter().filter(|i| i.el == Some(w)).count(), 5);
        // Picking the door's leaf selects the door.
        let leaf = dl
            .items
            .iter()
            .find_map(|i| match (&i.el, &i.prim) {
                (Some(id), Prim::Line { pts, .. }) if *id == d && pts.len() == 2 => Some(pts[1]),
                _ => None,
            })
            .unwrap();
        assert_eq!(pick(&dl, Pt::new(leaf[0], leaf[1]), 30.0), Some(d));
    }

    #[test]
    fn door_swing_follows_flip_facing() {
        let (mut doc, _, d, _) = with_openings();
        let l1 = doc.levels()[0].0;
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let leaf_tip_y = |doc: &Document| {
            display_list(doc, v)
                .unwrap()
                .items
                .iter()
                .find_map(|i| match (&i.el, &i.prim) {
                    (Some(id), Prim::Line { pts, .. }) if *id == d && pts.len() == 2 => {
                        Some(pts[1][1])
                    }
                    _ => None,
                })
                .unwrap()
        };
        // South wall runs west→east, so its left side is north (+y): the door swings in.
        assert!(leaf_tip_y(&doc) > 800.0);
        ops::set_property(&mut doc, d, "flip_facing", "yes", 0).unwrap();
        assert!(leaf_tip_y(&doc) < -800.0);
    }

    #[test]
    fn elevation_draws_openings_over_their_wall() {
        let (doc, _, d, w) = with_openings();
        let v = view(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: studio_core::Compass::South
                }
            )
        });
        let dl = display_list(&doc, v).unwrap();
        assert!(dl.items.iter().any(|i| i.el == Some(w)
            && matches!(
                i.prim,
                Prim::Fill {
                    fill: FillKind::Glass,
                    ..
                }
            )));
        assert!(dl.items.iter().any(|i| i.el == Some(d)
            && matches!(
                i.prim,
                Prim::Fill {
                    fill: FillKind::Paper,
                    ..
                }
            )));
        // Casement swing indicator is dashed.
        assert!(dl.items.iter().any(|i| i.el == Some(w)
            && matches!(
                i.prim,
                Prim::Line {
                    dash: Dash::Dashed,
                    ..
                }
            )));
        // Clicking the middle of the door in elevation picks the door, not the wall.
        let door_mid = Pt::new(3000.0, 1000.0);
        assert_eq!(pick(&dl, door_mid, 20.0), Some(d));
    }

    #[test]
    fn preview_snaps_to_wall_center_and_reports_overlap() {
        let (doc, south, _, _) = with_openings();
        let l1 = doc.levels()[0].0;
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 30"))
            .unwrap()
            .id;
        let len = 40.0 * MM_PER_FT;
        let p = opening_preview(&doc, v, dt, Pt::new(len / 2.0 + 50.0, -300.0), 200.0).unwrap();
        assert_eq!(p.host, south);
        assert!((p.offset - len / 2.0).abs() < 1e-9);
        assert!(
            p.flip_facing,
            "cursor south of a west→east wall is its right side"
        );
        assert!(p.valid);
        assert!(!p.items.is_empty());
        let over = opening_preview(&doc, v, dt, Pt::new(3100.0, 0.0), 200.0).unwrap();
        assert!(!over.valid);
        assert!(opening_preview(&doc, v, dt, Pt::new(5000.0, 5000.0), 200.0).is_none());
    }

    #[test]
    fn rooms_are_tagged_pickable_and_previewed() {
        let (mut doc, l1) = building();
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let pv = room_preview(&doc, v, Pt::new(2000.0, 2000.0)).unwrap();
        assert!(pv.valid);
        assert!(pv.label.ends_with("SF"), "{}", pv.label);
        let r = ops::create_room(&mut doc, l1, Pt::new(2000.0, 2000.0)).unwrap();
        let dl = display_list(&doc, v).unwrap();
        let tag = tag_of(&doc, r);
        let texts: Vec<_> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(tag))
            .filter_map(|i| match &i.prim {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts[0], "ROOM");
        assert_eq!(texts[1], "1");
        assert!(texts[2].ends_with(" SF"));
        // Clicking inside the room (away from walls and grids) selects it, not the floor.
        assert_eq!(pick(&dl, Pt::new(6000.0, 4000.0), 50.0), Some(r));
        // A second room in the same area is refused.
        let again = room_preview(&doc, v, Pt::new(8000.0, 3000.0)).unwrap();
        assert!(!again.valid);
        assert!(!room_preview(&doc, v, Pt::new(-9000.0, 0.0)).unwrap().valid);
    }

    #[test]
    fn section_cuts_walls_and_floor_and_projects_beyond() {
        let (mut doc, _, d, _) = with_openings();
        // Drawn north → south through the middle of the 40' × 30' building: a section looks
        // to the left of its line, so this one looks east.
        let x = 20.0 * MM_PER_FT;
        let s = ops::create_section(
            &mut doc,
            Pt::new(x, 32.0 * MM_PER_FT),
            Pt::new(x, -2.0 * MM_PER_FT),
        )
        .unwrap();
        let dl = display_list(&doc, s).unwrap();
        assert_eq!(dl.view_type, ViewType::Section);
        let poche: Vec<_> = dl
            .items
            .iter()
            .filter(|i| {
                matches!(
                    i.prim,
                    Prim::Fill {
                        fill: FillKind::Poche,
                        ..
                    }
                )
            })
            .collect();
        // South and north walls (one piece each at this x), the floor and the ceiling.
        assert_eq!(poche.len(), 4, "{poche:#?}");
        // The east wall is beyond the cut: drawn as a face, not poché.
        let east = regenerate(&doc)
            .walls
            .iter()
            .find(|w| {
                (w.start.x - 40.0 * MM_PER_FT).abs() < 1.0
                    && (w.end.x - 40.0 * MM_PER_FT).abs() < 1.0
            })
            .unwrap()
            .id;
        assert!(dl.items.iter().any(|i| i.el == Some(east)
            && matches!(
                i.prim,
                Prim::Fill {
                    fill: FillKind::Paper,
                    ..
                }
            )));
        // The door (at x = 3000 mm, beyond the cut) is visible on the south wall face? No:
        // the south wall is cut, so its openings are not drawn as faces.
        assert!(!dl.items.iter().any(|i| i.el == Some(d)));
        // Plans show the section marker linked to the view.
        let l1 = doc.levels()[0].0;
        let plan = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let pl = display_list(&doc, plan).unwrap();
        assert!(pl.items.iter().any(|i| i.el == Some(s)));
    }

    #[test]
    fn plans_tag_doors_and_windows() {
        let (doc, _, d, w) = with_openings();
        let l1 = doc.levels()[0].0;
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let dl = display_list(&doc, v).unwrap();
        let text_of = |id| {
            dl.items.iter().find_map(|i| match (&i.el, &i.prim) {
                (Some(x), Prim::Text { text, .. }) if *x == id => Some(text.clone()),
                _ => None,
            })
        };
        assert_eq!(text_of(tag_of(&doc, d)).as_deref(), Some("1"));
        assert_eq!(text_of(tag_of(&doc, w)).as_deref(), Some("1"));
        // Moving a tag moves only the tag; deleting it hides it in this view.
        let mut doc = doc;
        let t = tag_of(&doc, d);
        let at = |doc: &Document| {
            display_list(doc, v)
                .unwrap()
                .items
                .iter()
                .find_map(|i| match (&i.el, &i.prim) {
                    (Some(x), Prim::Text { at, .. }) if *x == t => Some(*at),
                    _ => None,
                })
        };
        let before = at(&doc).unwrap();
        studio_core::modify::move_elements(&mut doc, &[t], Pt::new(0.0, 900.0)).unwrap();
        assert!((at(&doc).unwrap()[1] - before[1] - 900.0).abs() < 1e-6);
        ops::delete(&mut doc, &[t]).unwrap();
        assert!(at(&doc).is_none());
        assert!(doc.get(d).is_some());
    }

    fn tag_of(doc: &Document, target: ElementId) -> ElementId {
        doc.of(Category::Tag)
            .find(|e| matches!(&e.data, ElementData::Tag { target: t, .. } if *t == target))
            .map(|e| e.id)
            .unwrap()
    }

    #[test]
    fn dimensions_read_true_length_and_stay_upright() {
        let (mut doc, l1) = building();
        let v = view(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        // Drawn right-to-left: text must still be upright (angle 0, not 180°).
        let dim = ops::create_dimension(
            &mut doc,
            v,
            Pt::new(40.0 * MM_PER_FT, 0.0),
            Pt::new(0.0, 0.0),
            -1500.0,
        )
        .unwrap();
        let dl = display_list(&doc, v).unwrap();
        let (text, angle) = dl
            .items
            .iter()
            .find_map(|i| match (&i.el, &i.prim) {
                (Some(x), Prim::Text { text, angle, .. }) if *x == dim => {
                    Some((text.clone(), *angle))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(text, "40'-0\"");
        assert!(angle.abs() < 1e-9, "{angle}");
        let pv = dimension_preview(
            &doc,
            v,
            Pt::new(0.0, 0.0),
            Pt::new(3000.0, 0.0),
            Pt::new(1000.0, 800.0),
        )
        .unwrap();
        assert!((pv.offset - 800.0).abs() < 1e-9);
    }

    #[test]
    fn meshes_cover_walls_floor_and_ceiling() {
        let (doc, _) = building();
        let ms = meshes(&doc);
        assert_eq!(ms.len(), 6);
        let (doc, _, _, _) = with_openings();
        let ms = meshes(&doc);
        assert_eq!(
            ms.iter().filter(|m| m.category == Category::Door).count(),
            1
        );
        assert_eq!(
            ms.iter().filter(|m| m.category == Category::Window).count(),
            1
        );
        assert!(ms
            .iter()
            .all(|m| !m.positions.is_empty() && m.positions.len() % 9 == 0));
    }

    /// A 40' × 30' box of 8" exterior walls on Level 1 with a hip roof on Level 2 and a
    /// stair from Level 1 to Level 2.
    fn roofed_house() -> (Document, ElementId, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let levels = doc.levels();
        let (l1, l2) = (levels[0].0, levels[1].0);
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let ft = studio_core::units::MM_PER_FT;
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(0.0, 30.0 * ft),
            Pt::new(40.0 * ft, 30.0 * ft),
            Pt::new(40.0 * ft, 0.0),
        ];
        for i in 0..4 {
            ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap();
        }
        let rt = studio_core::build::default_roof_type(&doc).unwrap();
        let eave = studio_geom::offset_ring(&c, 18.0 * MM_PER_IN);
        let roof = studio_core::build::create_roof(
            &mut doc,
            rt,
            l2,
            0.0,
            eave,
            studio_core::build::DEFAULT_ROOF_SLOPE,
        )
        .unwrap();
        let stair = studio_core::build::create_stair(
            &mut doc,
            l1,
            Pt::new(2.0 * ft, 8.0 * ft),
            Pt::new(2.0 * ft, 20.0 * ft),
            studio_core::build::DEFAULT_STAIR_WIDTH,
        )
        .unwrap();
        (doc, l1, roof, stair)
    }

    fn view_where(doc: &Document, f: impl Fn(&ViewKind) -> bool) -> ElementId {
        doc.of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind, .. } if f(kind)))
            .unwrap()
            .id
    }

    #[test]
    fn roofs_show_sloped_faces_in_elevation_and_cut_in_section() {
        let (mut doc, _, roof, _) = roofed_house();
        let south = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::North
                }
            )
        });
        let dl = display_list(&doc, south).unwrap();
        let polys = dl
            .items
            .iter()
            .filter(|i| i.el == Some(roof))
            .filter(|i| {
                matches!(
                    &i.prim,
                    Prim::Fill {
                        fill: FillKind::Paper,
                        ..
                    }
                )
            })
            .count();
        // Seen from the south: both slopes and both fascias (the hip ends are edge-on).
        assert_eq!(polys, 4, "roof faces and fascias seen as polygons");
        let sec = ops::create_section(&mut doc, Pt::new(-3000.0, 4500.0), Pt::new(15000.0, 4500.0))
            .unwrap();
        let dl = display_list(&doc, sec).unwrap();
        let cut = dl
            .items
            .iter()
            .filter(|i| i.el == Some(roof))
            .filter(|i| {
                matches!(
                    &i.prim,
                    Prim::Fill {
                        fill: FillKind::Poche,
                        ..
                    }
                )
            })
            .count();
        assert!(cut >= 2, "both roof slopes cut by the section: {cut}");
    }

    #[test]
    fn stair_plan_symbol_and_meshes() {
        let (doc, l1, roof, stair) = roofed_house();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let dl = display_list(&doc, plan).unwrap();
        let text = |s: &str| {
            dl.items.iter().any(|i| {
                i.el == Some(stair) && matches!(&i.prim, Prim::Text { text, .. } if text == s)
            })
        };
        assert!(text("UP"));
        let dashed = dl.items.iter().any(|i| {
            i.el == Some(stair)
                && matches!(
                    &i.prim,
                    Prim::Line {
                        dash: Dash::Dashed,
                        ..
                    }
                )
        });
        assert!(dashed, "treads above the cut plane are dashed");
        let l2 = doc.levels()[1].0;
        let upper = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l2),
        );
        let dl2 = display_list(&doc, upper).unwrap();
        assert!(dl2
            .items
            .iter()
            .any(|i| i.el == Some(stair)
                && matches!(&i.prim, Prim::Text { text, .. } if text == "DN")));
        assert!(
            dl2.items.iter().any(|i| i.el == Some(roof)),
            "roof plan on Level 2"
        );
        let m = meshes(&doc);
        assert!(m.iter().any(|x| x.el == roof && !x.positions.is_empty()));
        assert!(m
            .iter()
            .any(|x| x.el == stair && x.positions.len() == 17 * 12 * 9));
    }

    #[test]
    fn crop_region_clips_and_bounds_the_view() {
        let (mut doc, l1, _, _) = roofed_house();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let full = display_list(&doc, plan).unwrap();
        let ft = studio_core::units::MM_PER_FT;
        studio_core::edit::set_crop(
            &mut doc,
            plan,
            Some(CropBox {
                min: Pt::new(-2.0 * ft, -2.0 * ft),
                max: Pt::new(20.0 * ft, 15.0 * ft),
            }),
        )
        .unwrap();
        let dl = display_list(&doc, plan).unwrap();
        assert!(dl.items.len() < full.items.len());
        let inside = |q: &[f64; 2]| q[0] >= -2.0 * ft - 1e-6 && q[0] <= 20.0 * ft + 1e-6;
        for it in &dl.items {
            if let Prim::Line { pts, .. } = &it.prim {
                assert!(pts.iter().all(inside), "every line is clipped");
            }
        }
        assert!(
            dl.items.iter().any(|i| i.el == Some(plan)),
            "crop boundary drawn"
        );
        assert!(dl.bounds[2] < 25.0 * ft);
        ops::set_property(&mut doc, plan, "show_crop", "no", 0).unwrap();
        assert!(!display_list(&doc, plan)
            .unwrap()
            .items
            .iter()
            .any(|i| i.el == Some(plan)));
    }

    #[test]
    fn layers_draw_at_quarter_inch_scale_and_lists_are_cached() {
        let (mut doc, l1, _, _) = roofed_house();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let a = display_list_shared(&doc, plan).unwrap();
        let b = display_list_shared(&doc, plan).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "unchanged model → cached display list");
        let walls: std::collections::HashSet<ElementId> =
            doc.of(Category::Wall).map(|e| e.id).collect();
        let layer_lines = |dl: &DisplayList| {
            dl.items
                .iter()
                .filter(|i| i.el.is_some_and(|e| walls.contains(&e)))
                .filter(|i| matches!(&i.prim, Prim::Line { w: 1, .. }))
                .count()
        };
        // Four walls × three layer boundaries.
        assert_eq!(layer_lines(&a), 12);
        ops::set_property(&mut doc, plan, "scale", "96", 0).unwrap();
        assert_eq!(
            layer_lines(&display_list(&doc, plan).unwrap()),
            0,
            "coarse at 1/8\""
        );
    }

    /// M2 acceptance timing: `cargo test --release -p studio-views bench_500_walls -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn bench_500_walls() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        // 125 separate 10' × 8' rooms of four walls each: 500 walls.
        for k in 0..125 {
            let (x, y) = ((k % 25) as f64 * 4000.0, (k / 25) as f64 * 3500.0);
            let c = [
                Pt::new(x, y),
                Pt::new(x, y + 2400.0),
                Pt::new(x + 3000.0, y + 2400.0),
                Pt::new(x + 3000.0, y),
            ];
            for i in 0..4 {
                ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap();
            }
        }
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        display_list(&doc, plan).unwrap();
        let t0 = std::time::Instant::now();
        ops::set_property(&mut doc, wt, "thickness", "10\"", 0).unwrap();
        display_list(&doc, plan).unwrap();
        let type_edit = t0.elapsed();
        let t1 = std::time::Instant::now();
        let w = doc.of(Category::Wall).next().unwrap().id;
        studio_core::modify::move_elements(&mut doc, &[w], Pt::new(100.0, 0.0)).unwrap();
        display_list(&doc, plan).unwrap();
        let move_one = t1.elapsed();
        let t2 = std::time::Instant::now();
        for _ in 0..100 {
            snap(&doc, plan, Pt::new(1000.0, 1000.0), None, 50.0);
            pick(
                &display_list_shared(&doc, plan).unwrap(),
                Pt::new(1000.0, 1000.0),
                50.0,
            );
        }
        let hover = t2.elapsed() / 100;
        eprintln!("BENCH type edit {type_edit:?}, move one wall {move_one:?}, hover {hover:?}");

        // One connected building: a 25 × 10 grid of 10' × 8' bays (535 walls, T and
        // cross joins everywhere).
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Interior - 4"))
            .unwrap()
            .id;
        for r in 0..=10 {
            for c in 0..25 {
                let y = r as f64 * 2400.0;
                ops::create_wall(
                    &mut doc,
                    wt,
                    l1,
                    Pt::new(c as f64 * 3000.0, y),
                    Pt::new((c + 1) as f64 * 3000.0, y),
                )
                .unwrap();
            }
        }
        for c in 0..=25 {
            for r in 0..10 {
                let x = c as f64 * 3000.0;
                ops::create_wall(
                    &mut doc,
                    wt,
                    l1,
                    Pt::new(x, r as f64 * 2400.0),
                    Pt::new(x, (r + 1) as f64 * 2400.0),
                )
                .unwrap();
            }
        }
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        display_list(&doc, plan).unwrap();
        let t0 = std::time::Instant::now();
        ops::set_property(&mut doc, wt, "thickness", "6\"", 0).unwrap();
        display_list(&doc, plan).unwrap();
        let type_edit = t0.elapsed();
        let t1 = std::time::Instant::now();
        let w = doc.of(Category::Wall).next().unwrap().id;
        ops::set_property(&mut doc, w, "base_offset", "1\"", 0).unwrap();
        display_list(&doc, plan).unwrap();
        eprintln!(
            "BENCH connected: {} walls, type edit {type_edit:?}, edit one wall {:?}",
            doc.count(Category::Wall),
            t1.elapsed()
        );
    }
}
