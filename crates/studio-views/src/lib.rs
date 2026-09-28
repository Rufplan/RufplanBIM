//! View generation: display lists for plans, ceiling plans and elevations, 3D meshes,
//! picking and snapping. Display-list coordinates are model mm (plans: x east, y north;
//! elevations: u to the viewer's right, z up). Annotation sizes are paper mm × scale.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use studio_core::units::{format_area_sf, format_ft_in, MM_PER_IN};
use studio_core::{Category, CropBox, Document, ElementData, ElementId, ViewKind};
use studio_geom::{point_in_ring, project_to_segment, Pt};
use studio_regen::{bounds, regenerate, Model, OpeningKind, OpeningSolid};
use ts_rs::TS;

pub mod caps;
pub mod doors;
pub mod edges;
pub mod handles;
pub mod lighting;
mod plan_parts;
pub mod site_plan;
pub mod slopes;
pub mod snap;
pub mod standards_preview;
pub mod symbols;
pub mod terrain;
pub mod texgen;
pub mod thumbs;
pub mod view_refs;
pub mod windows;
pub use handles::{
    align_delta, handles, offset_preview, ref_line, Grip, Handles, OffsetPreview, RefLine, TempDim,
};
pub use snap::{snap, snap_only, SnapKind, SnapResult};

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
        callout_of,
        ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    // A callout of a section or elevation shows its parent's cut, even after it moves.
    let parent_kind = callout_of.and_then(|p| match doc.data(p) {
        Ok(ElementData::View {
            kind: k @ (ViewKind::Section { .. } | ViewKind::Elevation { .. }),
            ..
        }) => Some(k.clone()),
        _ => None,
    });
    let kind = parent_kind.as_ref().unwrap_or(kind);
    // Interior elevations crop to their room unless the view has its own crop.
    let mut auto_crop: Option<CropBox> = None;
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
            elevation(doc, &model, &mut b, facing.look().scale(-1.0)),
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
                projected(doc, &model, &mut b, d.perp(), Some(&cut)),
            )
        }
        ViewKind::MarkerElevation { marker, facing } => {
            let look = facing.look();
            match doc.data(*marker) {
                Ok(ElementData::ElevationMarker {
                    level,
                    at,
                    interior: true,
                    ..
                }) => {
                    let (cut, c) = interior_cut(doc, &model, *level, *at, look);
                    auto_crop = Some(c);
                    (
                        ViewType::Elevation,
                        projected(doc, &model, &mut b, look, Some(&cut)),
                    )
                }
                _ => (
                    ViewType::Elevation,
                    projected(doc, &model, &mut b, look, None),
                ),
            }
        }
        ViewKind::ThreeD | ViewKind::Schedule { .. } => return None,
    };
    if let ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level } = kind {
        plan_model_lines(doc, &mut b, *level);
    }
    if matches!(
        kind,
        ViewKind::Elevation { .. } | ViewKind::Section { .. } | ViewKind::MarkerElevation { .. }
    ) {
        view_room_tags(doc, &model, &mut b, view);
    }
    annotations(doc, &mut b, view);
    if let Ok(ElementData::View { level_ends, .. }) = doc.data(view) {
        apply_level_ends(&mut b.items, level_ends);
    }
    callout_markers(doc, &mut b, view);
    camera_markers(doc, &mut b, view);
    let crop_margin = b.paper(8.0);
    let crop = crop.or(auto_crop);
    // Hide in View (ADR-024).
    let vdata = doc.data(view).ok()?.clone();
    if matches!(&vdata, ElementData::View { hidden, hidden_categories, .. } if !hidden.is_empty() || !hidden_categories.is_empty())
    {
        b.items.retain(|i| {
            !i.el
                .is_some_and(|e| studio_core::visibility::hidden_in(doc, &vdata, e))
        });
    }
    // Placed symbols (ADR-048) are part of the view's extent, like Revit's annotations.
    let bounds = symbols::grow_bounds(doc, &b.items, bounds, b.paper(4.0));
    let (items, bounds) = match &crop {
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
    // Site plans show the ground's contours under everything (ADR-023).
    let site_view = matches!(doc.data(view), Ok(ElementData::View { site: true, .. }));
    if site_view {
        if let Some(s) = &model.site {
            site_plan::contours(b, s);
        }
    }

    if !ceiling {
        for f in model
            .floors
            .iter()
            .filter(|f| f.z1 >= elev - 1.0 && f.z1 <= cut)
        {
            let mut rings = vec![ring(&f.base.outer)];
            rings.extend(f.base.holes.iter().map(|h| ring(h)));
            b.fill(Some(f.id), rings, FillKind::Slab);
            b.line(Some(f.id), &f.base.outer, true, 1, Dash::Solid);
            for h in &f.base.holes {
                b.line(Some(f.id), h, true, 1, Dash::Solid);
            }
        }
    } else {
        for c in model.ceilings.iter().filter(|c| c.level == level) {
            let mut rings = vec![ring(&c.base.outer)];
            rings.extend(c.base.holes.iter().map(|h| ring(h)));
            b.fill(Some(c.id), rings, FillKind::Ceiling);
            for h in &c.base.holes {
                b.line(Some(c.id), h, true, 2, Dash::Solid);
            }
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
    // Sketched wall openings (ADR-058) crossing the cut are gaps too.
    let cut_pieces: Vec<(ElementId, studio_geom::Poly)> = cut_walls
        .iter()
        .flat_map(|w| {
            let holes = edges::hole_cuts_at(w, cut);
            w.pieces
                .iter()
                .filter(|p| p.z0 <= cut && p.z1 > cut)
                .flat_map(move |p| {
                    if holes.is_empty() {
                        vec![p.base.clone()]
                    } else {
                        studio_geom::difference(&p.base, &holes)
                    }
                })
                .map(move |p| (w.id, p))
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
    // Architectural columns in a wall join it: one poché and one outline, as in Revit.
    // Structural columns stay separate and are drawn over the wall.
    let joined: Vec<&studio_regen::ColumnSolid> = model
        .columns
        .iter()
        .filter(|c| !c.structural && c.z0 <= cut && c.z1 > cut)
        .filter(|c| {
            cut_pieces.iter().any(|(_, p)| {
                p.contains(c.at)
                    || c.base.outer.iter().any(|q| p.contains(*q))
                    || p.outer.iter().any(|q| c.base.contains(*q))
            })
        })
        .collect();
    for c in &joined {
        let fill = if detail {
            FillKind::PocheLight
        } else {
            FillKind::Poche
        };
        b.fill(Some(c.id), vec![ring(&c.base.outer)], fill);
    }
    let joined_ids: Vec<ElementId> = joined.iter().map(|c| c.id).collect();
    if detail {
        plan_parts::wall_layer_detail(b, &cut_walls, cut);
    }
    let merged = studio_geom::union_all(
        &cut_pieces
            .iter()
            .map(|(_, p)| p.clone())
            .chain(joined.iter().map(|c| c.base.clone()))
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
    plan_parts::columns_in_plan(b, model, elev, cut, &joined_ids);
    lighting::plan_symbols(doc, b, level, ceiling);
    if !ceiling {
        // Room separation lines (thin, like Revit's).
        for e in doc.of(Category::RoomSeparator) {
            if let ElementData::RoomSeparator {
                level: l,
                start,
                end,
            } = &e.data
            {
                if *l == level {
                    b.line(Some(e.id), &[*start, *end], false, 1, Dash::Solid);
                }
            }
        }
    }
    if !ceiling {
        plan_parts::beams_in_plan(b, model, elev, cut);
        plan_parts::stairs_in_plan(b, model, level, cut);
        plan_parts::railings_in_plan(b, model, level);
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
            } else if let Some(c) = model.columns.iter().find(|c| c.id == *target) {
                plan_parts::column_tag(doc, b, c, Some(e.id), *offset);
            } else if let Some(m) = model.beams.iter().find(|m| m.id == *target) {
                plan_parts::beam_tag(doc, b, m, Some(e.id), *offset);
            }
        }
        section_markers(doc, b);
    }

    // The property line: with bearings, distances and north in site plans; plain in the
    // plans of the lowest level.
    let lowest = model
        .levels
        .iter()
        .min_by(|a, c| a.elevation.total_cmp(&c.elevation))
        .map(|l| l.id);
    let mut site_pts: Vec<Pt> = vec![];
    if let Some(s) = model.site.as_ref().filter(|_| !ceiling) {
        if site_view {
            site_plan::property_line(b, s, true);
            site_plan::north_arrow(b, s);
            site_pts.extend(s.boundary.iter().copied());
        } else if lowest == Some(level) {
            site_plan::property_line(b, s, false);
        }
    }
    let (lo, hi) = plan_extents(model);
    let (lo, hi) = bounds(&[&[lo, hi][..], &site_pts].concat()).unwrap_or((lo, hi));
    let margin = b.paper(12.0);
    for g in &model.grids {
        grid_in_plan(b, g.id, &g.name, g.start, g.end);
    }
    if !ceiling {
        elevation_markers(doc, b, lo, hi, margin);
        placed_elevation_marks(doc, b, level);
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
pub(crate) fn clip_line_convex(a: Pt, d: Pt, ring_pts: &[Pt]) -> Option<(Pt, Pt)> {
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
            for v in &f.visible {
                b.line(el, v, true, 1, dash);
            }
        }
        b.line(el, &r.boundary, true, 2, dash);
    }
}

/// Points on a circular arc from angle `a0` sweeping `sweep` radians.
pub(crate) fn arc(c: Pt, r: f64, a0: f64, sweep: f64) -> Vec<Pt> {
    let n = 18;
    (0..=n)
        .map(|i| {
            let a = a0 + sweep * f64::from(i) / f64::from(n);
            c.add(Pt::new(a.cos(), a.sin()).scale(r))
        })
        .collect()
}

/// Plan symbol of a door or window, by family (ADR-031, ADR-033).
fn opening_symbol(b: &mut Builder, el: Option<ElementId>, o: &OpeningSolid) {
    match o.kind {
        OpeningKind::Door(style) => doors::plan_symbol(b, el, o, style),
        OpeningKind::Window(style) => windows::plan_symbol(b, el, o, style),
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
            kind: ViewKind::Section { start, end, .. },
            ..
        } = &v.data
        else {
            continue;
        };
        let el = Some(v.id);
        let d = end.sub(*start).norm();
        let look = d.perp();
        // The head is the building elevation mark (the owner's request): its body, the
        // large right-angled pointer toward the view and the detail over sheet number, at the
        // default exterior mark's size. The line runs from the head to Section Tail - Filled,
        // a 3/32" x 3/8" bar on the view's side.
        let symbol = studio_core::detail::mark_symbol(doc, None, false);
        let r = b.paper(symbol.1);
        let c = start.sub(d.scale(r));
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
        elevation_mark(doc, b, el, c, &[(v.id, look)], symbol);
        let (len, w) = (b.paper(9.525), b.paper(2.38));
        let tail = [
            *end,
            end.add(look.scale(len)),
            end.add(look.scale(len)).sub(d.scale(w)),
            end.sub(d.scale(w)),
        ];
        b.fill(el, vec![ring(&tail)], FillKind::Ink);
    }
}

/// Room tag at the room's point: name, number and area (or "Not Enclosed").
fn room_tag(b: &mut Builder, r: &studio_regen::RoomInfo, el: Option<ElementId>, offset: Pt) {
    room_tag_at(b, r, el, r.point.add(offset));
}

/// Room tags placed in a section or elevation (ADR-060).
fn view_room_tags(doc: &Document, model: &Model, b: &mut Builder, view: ElementId) {
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
        let (Some(r), Some(base)) = (
            model.rooms.iter().find(|r| r.id == *target),
            view_refs::room_tag_base(doc, view, *target),
        ) else {
            continue;
        };
        room_tag_at(b, r, Some(e.id), base.add(*offset));
    }
}

fn room_tag_at(b: &mut Builder, r: &studio_regen::RoomInfo, el: Option<ElementId>, p: Pt) {
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
            mark_type,
            ..
        } = &v.data
        else {
            continue;
        };
        let symbol = studio_core::detail::mark_symbol(doc, *mark_type, false);
        let out = facing.look();
        let c = match (out.x as i32, out.y as i32) {
            (0, 1) => Pt::new(mid.x, hi.y + off),
            (0, _) => Pt::new(mid.x, lo.y - off),
            (1, _) => Pt::new(hi.x + off, mid.y),
            _ => Pt::new(lo.x - off, mid.y),
        };
        // The marker points back at the building (the direction the elevation looks).
        let look = out.scale(-1.0);
        elevation_mark(doc, b, Some(v.id), c, &[(v.id, look)], symbol);
    }
}

/// Revit's Level Head - Circle: a datum target (a circle with two opposite quarters filled)
/// at the end of the level line, with the name and elevation above the line beside it.
pub(crate) fn level_head(b: &mut Builder, el: ElementId, end: Pt, name: &str, elevation: f64) {
    let r = b.paper(2.4);
    let c = end.add(Pt::new(r, 0.0));
    let el = Some(el);
    let quarter = |a0: f64| {
        let mut q = vec![c];
        q.extend(arc(c, r, a0, std::f64::consts::FRAC_PI_2));
        q
    };
    b.fill(
        el,
        vec![ring(&arc(c, r, 0.0, std::f64::consts::TAU))],
        FillKind::Paper,
    );
    b.fill(
        el,
        vec![ring(&quarter(std::f64::consts::FRAC_PI_2))],
        FillKind::Ink,
    );
    b.fill(
        el,
        vec![ring(&quarter(-std::f64::consts::FRAC_PI_2))],
        FillKind::Ink,
    );
    b.circle(el, c, 2.4, 2, false);
    b.line(
        el,
        &[c.sub(Pt::new(r, 0.0)), c.add(Pt::new(r, 0.0))],
        false,
        1,
        Dash::Solid,
    );
    b.line(
        el,
        &[c.sub(Pt::new(0.0, r)), c.add(Pt::new(0.0, r))],
        false,
        1,
        Dash::Solid,
    );
    // Name over elevation, both above the line, ending at the head.
    let x = end.x - b.paper(1.0);
    b.text(
        el,
        Pt::new(x, end.y + b.paper(5.2)),
        name.to_owned(),
        2.8,
        Anchor::Right,
    );
    b.text(
        el,
        Pt::new(x, end.y + b.paper(1.6)),
        revit_ft_in(elevation),
        2.4,
        Anchor::Right,
    );
}

/// A level's line in an elevation or section: its longest horizontal center line, as
/// (item index, left x, right x).
pub(crate) fn level_line(items: &[Item], level: ElementId) -> Option<(usize, f64, f64)> {
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.el == Some(level))
        .filter_map(|(i, it)| match &it.prim {
            Prim::Line {
                pts,
                dash: Dash::Center,
                ..
            } if pts.len() == 2 && (pts[0][1] - pts[1][1]).abs() < 1e-6 => {
                Some((i, pts[0][0].min(pts[1][0]), pts[0][0].max(pts[1][0])))
            }
            _ => None,
        })
        .max_by(|a, b| (a.2 - a.1).total_cmp(&(b.2 - b.1)))
}

/// Levels whose ends were dragged in this view (ADR-052): the line runs between them and the
/// head (everything else drawn for the level) moves with the right end.
fn apply_level_ends(items: &mut [Item], ends: &[studio_core::LevelEnds]) {
    for e in ends {
        let Some((line, x0, x1)) = level_line(items, e.level) else {
            continue;
        };
        let left = e.left.unwrap_or(x0);
        let right = e.right.unwrap_or(x1).max(left + 1.0);
        let dx = right - x1;
        for (i, it) in items.iter_mut().enumerate() {
            if it.el != Some(e.level) {
                continue;
            }
            if i == line {
                if let Prim::Line { pts, .. } = &mut it.prim {
                    let (a, b) = if pts[0][0] <= pts[1][0] {
                        (0, 1)
                    } else {
                        (1, 0)
                    };
                    pts[a][0] = left;
                    pts[b][0] = right;
                }
                continue;
            }
            match &mut it.prim {
                Prim::Line { pts, .. } => pts.iter_mut().for_each(|p| p[0] += dx),
                Prim::Fill { rings, .. } => rings.iter_mut().flatten().for_each(|p| p[0] += dx),
                Prim::Text { at, .. } => at[0] += dx,
                Prim::Circle { c, .. } => c[0] += dx,
            }
        }
    }
}

/// Feet-inches with spaced dash, as Revit prints levels: 10' - 0".
fn revit_ft_in(mm: f64) -> String {
    format_ft_in(mm).replacen("'-", "' - ", 1)
}

/// A view's reference on a sheet: its detail number (order placed) and the sheet number.
pub(crate) fn view_ref(doc: &Document, view: ElementId) -> Option<(String, String)> {
    let sheet = doc.iter().find_map(|e| match &e.data {
        ElementData::Viewport { sheet, view: v, .. } if *v == view => Some(*sheet),
        _ => None,
    })?;
    let mut on_sheet: Vec<ElementId> = doc
        .iter()
        .filter_map(|e| match &e.data {
            ElementData::Viewport {
                sheet: s, view: v, ..
            } if *s == sheet => Some(*v),
            _ => None,
        })
        .collect();
    on_sheet.sort();
    let n = on_sheet.iter().position(|v| *v == view)? + 1;
    let number = studio_core::ops::sheets(doc)
        .into_iter()
        .find(|s| s.0 == sheet)
        .map(|s| s.1)?;
    Some((n.to_string(), number))
}

/// Revit's "Filled Arrow" pointer: two lines tangent to a round body of radius `r` at `c`,
/// meeting at a right angle on the side it looks, filled between them and the body.
pub(crate) fn filled_arrow(c: Pt, r: f64, look: Pt) -> Vec<Pt> {
    use std::f64::consts::FRAC_PI_4;
    let a = look.y.atan2(look.x);
    // Tangents from a tip r·√2 away touch the body 45° either side of the look direction.
    let tip = c.add(look.scale(r * std::f64::consts::SQRT_2));
    let mut out = vec![tip];
    out.extend(arc(c, r, a + FRAC_PI_4, -2.0 * FRAC_PI_4));
    out
}

/// Revit's standard elevation mark pointer, measured from the owner's references: a
/// right-angled triangle pointing `look` whose sides are tangent to the body. Its base is the
/// body's diameter across the look, √2 radii each side, and its point is √2 radii out, so the
/// body drawn over it leaves the black point and a wing at each end of the diameter.
pub(crate) fn mark_arrow(c: Pt, r: f64, look: Pt) -> [Pt; 3] {
    let k = r * std::f64::consts::SQRT_2;
    let side = look.perp().scale(k);
    [c.add(look.scale(k)), c.add(side), c.sub(side)]
}

/// How far a mark's pointer reaches from its center, in radii.
fn mark_reach(style: studio_core::MarkStyle) -> f64 {
    match style {
        studio_core::MarkStyle::CircleArrow => std::f64::consts::SQRT_2,
        studio_core::MarkStyle::CircleHalf => 1.55,
        studio_core::MarkStyle::Diamond => std::f64::consts::SQRT_2,
    }
}

/// Revit's elevation mark: a round body with a filled arrow pointer for each view (the
/// pointer is the view: double-click it to open). One view shows its detail number over
/// its sheet number; several show each detail number by its pointer.
pub(crate) fn elevation_mark(
    doc: &Document,
    b: &mut Builder,
    body: Option<ElementId>,
    c: Pt,
    pointers: &[(ElementId, Pt)],
    (style, rp): (studio_core::MarkStyle, f64),
) {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, TAU};
    use studio_core::MarkStyle;
    let r = b.paper(rp);
    let angle = |v: Pt| v.y.atan2(v.x);
    match style {
        MarkStyle::CircleArrow => {
            // One view: the large right-angled point behind the body (the building mark in
            // the owner's reference). Several (an interior mark): small tangent points, so
            // four of them square the body into Revit's diamond.
            let single = pointers.len() == 1;
            for (view, look) in pointers {
                let shape = if single {
                    mark_arrow(c, r, *look).to_vec()
                } else {
                    filled_arrow(c, r, *look)
                };
                b.fill(Some(*view), vec![ring(&shape)], FillKind::Ink);
            }
            b.fill(body, vec![ring(&arc(c, r, 0.0, TAU))], FillKind::Paper);
            b.circle(body, c, rp, 2, false);
        }
        MarkStyle::CircleHalf => {
            b.fill(body, vec![ring(&arc(c, r, 0.0, TAU))], FillKind::Paper);
            // The body's half (or quarter, with several views) toward each view, filled,
            // with a point beyond it.
            let sweep = if pointers.len() > 1 {
                FRAC_PI_2
            } else {
                std::f64::consts::PI
            };
            for (view, look) in pointers {
                let a0 = angle(*look) - sweep / 2.0;
                let mut wedge = vec![c];
                wedge.extend(arc(c, r, a0, sweep));
                b.fill(Some(*view), vec![ring(&wedge)], FillKind::Ink);
                let side = look.perp().scale(r * 0.45);
                let tip = c.add(look.scale(r * 1.55));
                let base = c.add(look.scale(r * 0.85));
                b.fill(
                    Some(*view),
                    vec![ring(&[tip, base.add(side), base.sub(side)])],
                    FillKind::Ink,
                );
            }
            b.circle(body, c, rp, 2, false);
        }
        MarkStyle::Diamond => {
            // A square turned 45° around the circle; each view fills its corner.
            let d = r * std::f64::consts::SQRT_2;
            let corners = [
                Pt::new(0.0, 1.0),
                Pt::new(1.0, 0.0),
                Pt::new(0.0, -1.0),
                Pt::new(-1.0, 0.0),
            ]
            .map(|v| c.add(v.scale(d)));
            b.fill(body, vec![ring(&corners)], FillKind::Paper);
            for (view, look) in pointers {
                let corner = c.add(look.scale(d));
                let a = angle(*look);
                let mut region = vec![corner];
                region.extend(arc(c, r, a - FRAC_PI_4, FRAC_PI_2).into_iter().rev());
                b.fill(Some(*view), vec![ring(&region)], FillKind::Ink);
            }
            b.line(body, &corners, true, 2, Dash::Solid);
            b.circle(body, c, rp, 1, false);
        }
    }
    let ink_half = style == MarkStyle::CircleHalf;
    match pointers {
        [(view, look)] => {
            let (detail, sheet) =
                view_ref(doc, *view).unwrap_or_else(|| ("—".into(), String::new()));
            if ink_half {
                // The empty half carries the reference.
                let at = c.sub(look.scale(r * 0.45)).sub(Pt::new(0.0, b.paper(0.8)));
                b.text(
                    Some(*view),
                    at,
                    format!("{detail}/{sheet}"),
                    1.6,
                    Anchor::Center,
                );
            } else {
                b.line(
                    body,
                    &[c.sub(Pt::new(r, 0.0)), c.add(Pt::new(r, 0.0))],
                    false,
                    2,
                    Dash::Solid,
                );
                b.text(
                    Some(*view),
                    c.add(Pt::new(0.0, r * 0.42)),
                    detail,
                    rp * 0.49,
                    Anchor::Center,
                );
                b.text(
                    Some(*view),
                    c.sub(Pt::new(0.0, r * 0.58)),
                    sheet,
                    rp * 0.4,
                    Anchor::Center,
                );
            }
        }
        _ => {
            // Revit's interior mark: the sheet number in the body, each view's number
            // outside, just beyond its point.
            let sheet = pointers
                .iter()
                .find_map(|(v, _)| view_ref(doc, *v).map(|r| r.1))
                .unwrap_or_else(|| "—".into());
            b.text(body, c, sheet, rp * 0.4, Anchor::Center);
            // Several views: CircleArrow draws tangent points, which reach r·√2.
            let reach = if style == studio_core::MarkStyle::CircleArrow {
                r * std::f64::consts::SQRT_2
            } else {
                r * mark_reach(style)
            } + b.paper(rp * 0.4);
            for (view, look) in pointers {
                let label = view_ref(doc, *view).map_or_else(|| "—".into(), |r| r.0);
                b.text(
                    Some(*view),
                    c.add(look.scale(reach)),
                    label,
                    rp * 0.45,
                    Anchor::Center,
                );
            }
        }
    }
}

/// Elevation markers placed on `level` (interior) and building markers on any level.
fn placed_elevation_marks(doc: &Document, b: &mut Builder, level: ElementId) {
    for e in doc.of(Category::ElevationMarker) {
        let ElementData::ElevationMarker {
            level: l,
            at,
            interior,
            type_id,
        } = &e.data
        else {
            continue;
        };
        let symbol = studio_core::detail::mark_symbol(doc, *type_id, *interior);
        if *interior && *l != level {
            continue;
        }
        let pointers: Vec<(ElementId, Pt)> = studio_core::detail::marker_views(doc, e.id)
            .into_iter()
            .map(|(c, v)| (v, c.look()))
            .collect();
        elevation_mark(doc, b, Some(e.id), *at, &pointers, symbol);
    }
}

/// An interior elevation's cut: through the marker, as wide as its room plus a foot each
/// side (so the side walls show cut), to the far wall; and its crop, floor to the level
/// above.
/// An elevation or section's frame in plan (ADR-058): the origin and right direction of its
/// display x (x = (p - origin)·right, y = z), and the way it looks. None for other views.
pub fn view_frame(doc: &Document, view: ElementId) -> Option<(Pt, Pt, Pt)> {
    let ElementData::View { kind, .. } = doc.data(view).ok()? else {
        return None;
    };
    let (origin, look) = match kind {
        ViewKind::Elevation { facing } => (Pt::default(), facing.look().scale(-1.0)),
        ViewKind::Section { start, end, .. } => (*start, end.sub(*start).norm().perp()),
        ViewKind::MarkerElevation { marker, facing } => {
            let look = facing.look();
            match doc.data(*marker) {
                Ok(ElementData::ElevationMarker {
                    level,
                    at,
                    interior: true,
                    ..
                }) => {
                    let model = regenerate(doc);
                    (interior_cut(doc, &model, *level, *at, look).0.origin, look)
                }
                _ => (Pt::default(), look),
            }
        }
        _ => return None,
    };
    Some((origin, Pt::new(look.y, -look.x), look))
}

fn interior_cut(
    doc: &Document,
    model: &Model,
    level: ElementId,
    at: Pt,
    look: Pt,
) -> (Cut, CropBox) {
    let right = Pt::new(look.y, -look.x);
    let ft = studio_core::units::MM_PER_FT;
    let room = studio_regen::room_at(model, level, at).unwrap_or_else(|| {
        let h = 10.0 * ft;
        vec![
            at.add(Pt::new(-h, -h)),
            at.add(Pt::new(h, -h)),
            at.add(Pt::new(h, h)),
            at.add(Pt::new(-h, h)),
        ]
    });
    let us: Vec<f64> = room.iter().map(|p| p.sub(at).dot(right)).collect();
    let ds: Vec<f64> = room.iter().map(|p| p.sub(at).dot(look)).collect();
    let umin = us.iter().copied().fold(f64::INFINITY, f64::min);
    let umax = us.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let dmax = ds.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(ft);
    let side = ft;
    let length = umax - umin + 2.0 * side;
    let elev = doc.level_elevation(level).unwrap_or(0.0);
    let top = model
        .levels
        .iter()
        .map(|l| l.elevation)
        .filter(|z| *z > elev + 1.0)
        .fold(f64::INFINITY, f64::min);
    let top = if top.is_finite() {
        top
    } else {
        elev + 10.0 * ft
    };
    (
        Cut {
            origin: at.add(right.scale(umin - side)),
            length,
            depth: dmax + side,
        },
        CropBox {
            min: Pt::new(0.0, elev - side),
            max: Pt::new(length, top + side),
        },
    )
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

fn elevation(doc: &Document, model: &Model, b: &mut Builder, look: Pt) -> [f64; 4] {
    projected(doc, model, b, look, None)
}

/// Elevation (no cut) or section (cut plane with far clip), seen looking along `look`.
fn projected(
    doc: &Document,
    model: &Model,
    b: &mut Builder,
    look: Pt,
    cut: Option<&Cut>,
) -> [f64; 4] {
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
    // Every u-interval where a polygon with holes crosses the cut plane (even-odd).
    let cut_intervals = |poly: &studio_geom::Poly| -> Vec<(f64, f64)> {
        let Some(c) = cut else { return vec![] };
        let mut us = vec![];
        for r in std::iter::once(&poly.outer).chain(&poly.holes) {
            let n = r.len();
            for i in 0..n {
                let (p, q) = (r[i], r[(i + 1) % n]);
                let (dp, dq) = (depth_of(p), depth_of(q));
                if (dp < 0.0) != (dq < 0.0) {
                    us.push(u_of(p.lerp(q, dp / (dp - dq))));
                }
            }
        }
        us.sort_by(f64::total_cmp);
        us.chunks(2)
            .filter(|w| w.len() == 2)
            .map(|w| (w[0].max(0.0), w[1].min(c.length)))
            .filter(|(a, b)| b - a > 0.5)
            .collect()
    };
    let mut cut_rects: Vec<(ElementId, f64, f64, f64, f64)> = vec![];
    // Layer boundary lines inside cut material, in (u, z).
    let mut cut_lines: Vec<(ElementId, [Pt; 2])> = vec![];

    struct Face {
        el: ElementId,
        u0: f64,
        u1: f64,
        z0: f64,
        z1: f64,
        near: f64,
        mid: f64,
        fill: FillKind,
        /// An opening's family, whether it's seen mirrored, and a door's hand.
        detail: Option<(OpeningKind, bool, bool)>,
        /// A sloped face's outline in (u, z); rectangles leave this empty.
        poly: Option<Vec<Pt>>,
        /// Surface pattern lines in (u, z), drawn with the face (ADR-020).
        lines: Vec<[Pt; 2]>,
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
            lines: vec![],
        }
    };
    let mut faces: Vec<Face> = vec![];
    // Cut profiles that aren't rectangles (sloped roofs), drawn with the cut rectangles.
    let mut cut_polys: Vec<(ElementId, Vec<Pt>)> = vec![];
    // Whole walls, not their pieces: every opening is drawn on top with its own door or
    // glass fill, and piece seams would read as false joints in the facade.
    for w in &model.walls {
        match seen(&w.footprint.outer) {
            Seen::Beyond => {
                let mut f = face(w.id, &w.footprint.outer, w.z0, w.z1, FillKind::Paper);
                // The face toward the viewer carries its finish's surface pattern.
                let toward = w.dir().perp().dot(look) < 0.0;
                let painted = studio_core::paint::paint_of(doc, w.id);
                let surface = match painted.and_then(|p| doc.data(p).ok()) {
                    Some(ElementData::Material { surface, .. }) => *surface,
                    _ if toward => w.surfaces.0,
                    _ => w.surfaces.1,
                };
                if let Some(prof) = &w.top_profile {
                    // Seen square on, the top follows the roof above it.
                    let d = w.dir();
                    let mut outline =
                        vec![Pt::new(u_of(w.start), w.z0), Pt::new(u_of(w.end), w.z0)];
                    outline.extend(
                        prof.iter()
                            .rev()
                            .map(|(s, z)| Pt::new(u_of(w.start.add(d.scale(*s))), *z)),
                    );
                    if studio_geom::signed_area(&outline).abs() > 1.0 {
                        f.poly = Some(outline);
                    }
                }
                if f.u1 - f.u0 > 1.0 {
                    let outline = f.poly.clone().unwrap_or_else(|| {
                        vec![
                            Pt::new(f.u0, f.z0),
                            Pt::new(f.u1, f.z0),
                            Pt::new(f.u1, f.z1),
                            Pt::new(f.u0, f.z1),
                        ]
                    });
                    f.lines = surface_lines(&outline, surface, 1.0, b.paper(0.8));
                }
                faces.push(f);
            }
            Seen::Cut => {
                let c = cut.map_or(Pt::default(), |c| c.origin);
                let at = studio_geom::line_intersection(w.start, w.dir(), c, right);
                let top = at.map_or(w.z1, |x| w.top_at(x));
                // Sketched openings (ADR-058) the cut passes through: gaps in its height.
                let gaps: Vec<(f64, f64)> = at
                    .map(|x| {
                        let t = x.sub(w.start).dot(w.dir());
                        w.holes
                            .iter()
                            .flat_map(|(_, r)| {
                                let turned: Vec<Pt> = r.iter().map(|p| Pt::new(p.y, p.x)).collect();
                                edges::hole_spans(&turned, t)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                for p in &w.pieces {
                    if let Some((u0, u1)) = cut_interval(&p.base.outer) {
                        let z1 = if p.z1 >= w.z1 - 0.5 { top } else { p.z1 };
                        let mut z = p.z0;
                        let mut spans = gaps.clone();
                        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
                        for (g0, g1) in spans {
                            if g1 <= z || g0 >= z1 {
                                continue;
                            }
                            if g0 > z + 0.5 {
                                cut_rects.push((w.id, u0, u1, z, g0));
                            }
                            z = z.max(g1);
                        }
                        if z1 > z + 0.5 {
                            cut_rects.push((w.id, u0, u1, z, z1));
                        }
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
        let mirrored = match o.kind {
            OpeningKind::Window(_) => windows::Axes::of(o).ax.dot(right) < 0.0,
            OpeningKind::Door(_) => o.dir.dot(right) < 0.0,
        };
        f.detail = Some((o.kind, mirrored, o.flip_hand));
        faces.push(f);
    }
    // Sketched wall openings (ADR-058): each hole drawn over its wall, as a door opening is.
    for w in &model.walls {
        if w.holes.is_empty() || seen(&w.footprint.outer) != Seen::Beyond {
            continue;
        }
        let host_near = w
            .footprint
            .outer
            .iter()
            .map(|p| depth_of(*p))
            .fold(f64::INFINITY, f64::min);
        let d = w.dir();
        for (id, ring) in &w.holes {
            let poly: Vec<Pt> = ring
                .iter()
                .map(|p| Pt::new(u_of(w.start.add(d.scale(p.x))), p.y.clamp(w.z0, w.z1)))
                .collect();
            if studio_geom::signed_area(&poly).abs() < 1.0 {
                continue; // Seen edge-on.
            }
            let pts: Vec<Pt> = ring.iter().map(|p| w.start.add(d.scale(p.x))).collect();
            let mut f = face(
                *id,
                &pts,
                ring.iter()
                    .map(|p| p.y)
                    .fold(f64::INFINITY, f64::min)
                    .max(w.z0),
                ring.iter()
                    .map(|p| p.y)
                    .fold(f64::NEG_INFINITY, f64::max)
                    .min(w.z1),
                FillKind::Paper,
            );
            f.near = host_near - 0.5;
            f.mid = f.near;
            f.poly = Some(poly);
            faces.push(f);
        }
    }
    for (slabs, fill) in [
        (&model.floors, FillKind::Slab),
        (&model.ceilings, FillKind::Paper),
    ] {
        // Sloped floors are drawn from their faces, with the roofs (ADR-049).
        for f in slabs.iter().filter(|f| f.tilt.is_none()) {
            match seen(&f.base.outer) {
                Seen::Beyond => faces.push(face(f.id, &f.base.outer, f.z0, f.z1, fill)),
                Seen::Cut => {
                    for (u0, u1) in cut_intervals(&f.base) {
                        cut_rects.push((f.id, u0, u1, f.z0, f.z1));
                        for d in &f.layers {
                            let z = f.z1 - d;
                            cut_lines.push((f.id, [Pt::new(u0, z), Pt::new(u1, z)]));
                        }
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
    let prisms = model
        .columns
        .iter()
        .map(|c| (c.id, c.prism()))
        .chain(
            model
                .beams
                .iter()
                .flat_map(|b| b.prisms.iter().map(move |p| (b.id, p.clone()))),
        )
        .chain(
            model
                .railings
                .iter()
                .flat_map(|r| r.posts.iter().map(move |p| (r.id, p.clone()))),
        );
    for (id, p) in prisms {
        match seen(&p.base.outer) {
            Seen::Beyond => faces.push(face(id, &p.base.outer, p.z0, p.z1, FillKind::Paper)),
            Seen::Cut => {
                if let Some((u0, u1)) = cut_interval(&p.base.outer) {
                    cut_rects.push((id, u0, u1, p.z0, p.z1));
                }
            }
            Seen::Hidden => {}
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
            lines: vec![],
        })
    };
    // Rails: the outline of each box as seen (a sloped bar along a stair).
    for r in &model.railings {
        for bx in &r.boxes {
            let plan: Vec<Pt> = bx.iter().map(|v| Pt::new(v[0], v[1])).collect();
            if seen(&plan) == Seen::Hidden {
                continue;
            }
            if cut.is_some() && plan.iter().all(|p| depth_of(*p) < 0.0) {
                continue;
            }
            let uz: Vec<Pt> = bx
                .iter()
                .map(|v| Pt::new(u_of(Pt::new(v[0], v[1])), v[2]))
                .collect();
            let hull = studio_geom::convex_hull(&uz);
            if hull.len() < 3 || studio_geom::signed_area(&hull).abs() < 1.0 {
                continue;
            }
            let depths: Vec<f64> = plan.iter().map(|p| depth_of(*p)).collect();
            faces.push(Face {
                el: r.id,
                u0: hull.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
                u1: hull.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
                z0: hull.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
                z1: hull.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
                near: depths.iter().copied().fold(f64::INFINITY, f64::min),
                mid: depths.iter().sum::<f64>() / depths.len() as f64,
                fill: FillKind::Paper,
                detail: None,
                poly: Some(hull),
                lines: vec![],
            });
        }
    }
    for f in model.floors.iter().filter(|f| f.tilt.is_some()) {
        let push = |s: &[[f64; 3]], faces: &mut Vec<Face>| {
            if let Some(mut face) = poly_face(f.id, s) {
                face.fill = FillKind::Slab;
                faces.push(face);
            }
        };
        match seen(&f.base.outer) {
            Seen::Beyond => {
                for s in f.surfaces() {
                    push(&s, &mut faces);
                }
            }
            Seen::Cut => {
                for s in f.surfaces() {
                    push(&clip3(&s, depth_of), &mut faces);
                }
                // The cut profile: a parallelogram where the slab falls along the cut.
                let t = f.z1 - f.z0;
                for (u0, u1) in cut_intervals(&f.base) {
                    let top = |u: f64| f.top_at(origin.add(right.scale(u)));
                    let (t0, t1) = (top(u0), top(u1));
                    cut_polys.push((
                        f.id,
                        vec![
                            Pt::new(u0, t0 - t),
                            Pt::new(u1, t1 - t),
                            Pt::new(u1, t1),
                            Pt::new(u0, t0),
                        ],
                    ));
                    for d in &f.layers {
                        cut_lines.push((f.id, [Pt::new(u0, t0 - d), Pt::new(u1, t1 - d)]));
                    }
                }
            }
            Seen::Hidden => {}
        }
    }
    for r in &model.roofs {
        // Courses along the slope show as level lines this far apart.
        let rise = r.slope.sin().max(0.05);
        let roof_face = |s: &[[f64; 3]], faces: &mut Vec<Face>| {
            let Some(mut f) = poly_face(r.id, s) else {
                return;
            };
            if sloped_top(s) {
                if let Some(p) = &f.poly {
                    f.lines = surface_lines(p, r.surface, rise, b.paper(0.8));
                }
            }
            faces.push(f);
        };
        match seen(&r.boundary) {
            Seen::Beyond => {
                for s in r.surfaces() {
                    roof_face(&s, &mut faces);
                }
            }
            Seen::Cut => {
                // What lies beyond the cut plane, then the cut profile.
                for s in r.surfaces() {
                    let kept = clip3(&s, depth_of);
                    roof_face(&kept, &mut faces);
                }
                let t = if r.is_flat() {
                    r.thickness
                } else {
                    r.plumb_thickness()
                };
                if r.is_flat() {
                    if let Some((u0, u1)) = cut_interval(&r.boundary) {
                        cut_rects.push((r.id, u0, u1, r.base, r.base + t));
                        for d in &r.layers {
                            let z = r.base + t - d;
                            cut_lines.push((r.id, [Pt::new(u0, z), Pt::new(u1, z)]));
                        }
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
                        let lo = |v: Pt, d: f64| Pt::new(v.x, v.y - d);
                        cut_polys.push((r.id, vec![p, q, lo(q, t), lo(p, t)]));
                        // Layer depths are square to the surface; plumb they are longer.
                        let k = 1.0 / r.slope.cos().max(0.1);
                        for d in &r.layers {
                            cut_lines.push((r.id, [lo(p, d * k), lo(q, d * k)]));
                        }
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

    // The ground line of an elevation, behind the building (ADR-023).
    if let (Some(s), None) = (&model.site, cut) {
        let line = site_plan::ground_line(s, &u_of);
        if line.len() >= 2 {
            b.line(Some(s.id), &line, false, 4, Dash::Solid);
        }
    }
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
        for seg in &f.lines {
            b.line(Some(f.el), seg, false, 1, Dash::Solid);
        }
        b.line(Some(f.el), &r, true, 2, Dash::Solid);
        if let Some((kind, mirrored, hand)) = f.detail {
            opening_elevation_detail(b, f.el, kind, (f.u0, f.u1, f.z0, f.z1), mirrored, hand);
        }
    }
    // The ground a section cuts, under the building's cut material.
    if let (Some(s), Some(c)) = (&model.site, cut) {
        if let Some(poly) = site_plan::ground_cut(s, origin, right, c.length) {
            b.fill(Some(s.id), vec![ring(&poly)], FillKind::PocheLight);
            b.line(Some(s.id), &poly, true, 3, Dash::Solid);
        }
    }
    // Cut material over everything beyond it; layered material is lighter so its layer
    // lines read.
    let light = |el: &ElementId| cut_lines.iter().any(|(e, _)| e == el);
    for (el, u0, u1, z0, z1) in &cut_rects {
        let r = [
            Pt::new(*u0, *z0),
            Pt::new(*u1, *z0),
            Pt::new(*u1, *z1),
            Pt::new(*u0, *z1),
        ];
        let fill = if light(el) {
            FillKind::PocheLight
        } else {
            FillKind::Poche
        };
        b.fill(Some(*el), vec![ring(&r)], fill);
        b.line(Some(*el), &r, true, 4, Dash::Solid);
    }
    for (el, r) in &cut_polys {
        let fill = if light(el) {
            FillKind::PocheLight
        } else {
            FillKind::Poche
        };
        b.fill(Some(*el), vec![ring(r)], fill);
        b.line(Some(*el), r, true, 4, Dash::Solid);
    }
    for (el, seg) in &cut_lines {
        b.line(Some(*el), seg, false, 1, Dash::Solid);
    }

    let ext = b.paper(12.0);
    b.line(
        None,
        &[Pt::new(umin - ext, 0.0), Pt::new(umax + ext, 0.0)],
        false,
        6,
        Dash::Solid,
    );

    // Model lines (ADR-054), at their level's height; a section sees those in its depth.
    for e in doc.of(Category::ModelLine) {
        let ElementData::ModelLine {
            level,
            curve,
            style,
        } = &e.data
        else {
            continue;
        };
        let pts = curve.points();
        if cut.is_some() && seen(&pts) == Seen::Hidden {
            continue;
        }
        let z = doc.level_elevation(*level).unwrap_or(0.0);
        let uz: Vec<Pt> = pts.iter().map(|p| Pt::new(u_of(*p), z)).collect();
        let (w, dash) = line_style(*style);
        b.line(Some(e.id), &uz, false, w, dash);
    }
    for l in &model.levels {
        let x1 = umax + ext * 2.0;
        b.line(
            Some(l.id),
            &[Pt::new(umin - ext, l.elevation), Pt::new(x1, l.elevation)],
            false,
            1,
            Dash::Center,
        );
        level_head(b, l.id, Pt::new(x1, l.elevation), &l.name, l.elevation);
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

/// True for a roof's sloped top face (not a vertical fascia or a flat face seen edge-on).
fn sloped_top(poly: &[[f64; 3]]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let (a, b, c) = (poly[0], poly[1], poly[2]);
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    len > 1e-9 && n[2].abs() / len > 0.2 && n[2].abs() / len < 0.999
}

/// Where the horizontal line at height `z` crosses `poly` (even-odd), as u-intervals.
fn level_spans(poly: &[Pt], z: f64) -> Vec<(f64, f64)> {
    let n = poly.len();
    let mut xs = vec![];
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        if (p.y <= z) != (q.y <= z) {
            xs.push(p.x + (q.x - p.x) * (z - p.y) / (q.y - p.y));
        }
    }
    xs.sort_by(f64::total_cmp);
    xs.chunks(2)
        .filter(|c| c.len() == 2 && c[1] - c[0] > 0.5)
        .map(|c| (c[0], c[1]))
        .collect()
}

/// Surface pattern lines inside a face outline in (u, z), aligned to the project origin
/// so patterns line up across walls. `vscale` shrinks vertical spacing (courses along a
/// slope seen in elevation); patterns finer than `min` are skipped at this scale.
pub(crate) fn surface_lines(
    outline: &[Pt],
    s: studio_core::SurfacePattern,
    vscale: f64,
    min: f64,
) -> Vec<[Pt; 2]> {
    use studio_core::SurfacePattern as S;
    let (course, unit, stagger) = match s {
        S::None => return vec![],
        S::Lap { spacing } => (spacing, 0.0, false),
        S::Running { course, unit } => (course, unit, true),
        S::Grid { width, height } => (height, width, false),
    };
    let dz = course * vscale;
    if dz < min || (unit > 0.0 && unit < min) {
        return vec![];
    }
    let (z0, z1) = outline
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
            (a.0.min(p.y), a.1.max(p.y))
        });
    let mut out = vec![];
    // Course by course, from the one containing the face's bottom edge.
    let mut k = (z0 / dz).floor() as i64;
    const MAX: usize = 6000;
    while (k as f64) * dz < z1 && out.len() < MAX {
        let z = k as f64 * dz;
        if z > z0 + 0.5 {
            for (a, b) in level_spans(outline, z) {
                out.push([Pt::new(a, z), Pt::new(b, z)]);
            }
        }
        if unit > 0.0 {
            // Head joints within this course, staggered every other course for a bond.
            let (za, zb) = (z.max(z0), (z + dz).min(z1));
            let shift = if stagger && k.rem_euclid(2) == 1 {
                unit / 2.0
            } else {
                0.0
            };
            for (a, b) in level_spans(outline, (za + zb) / 2.0) {
                let mut j = ((a - shift) / unit).floor() as i64;
                while (j as f64) * unit + shift < b - 0.5 && out.len() < MAX {
                    let u = j as f64 * unit + shift;
                    if u > a + 0.5 {
                        out.push([Pt::new(u, za), Pt::new(u, zb)]);
                    }
                    j += 1;
                }
            }
        }
        k += 1;
    }
    out
}

/// Callout boundaries of `view`'s callouts, with a tag naming each (Revit's callout head).
/// Cameras on this floor plan's level (ADR-027), as Revit shows them: the camera at the
/// eye, its view cone out to the target, and the target. Drawn with the camera's view as
/// their element, so they select it and never print.
fn camera_markers(doc: &Document, b: &mut Builder, view: ElementId) {
    let Ok(ElementData::View {
        kind: ViewKind::FloorPlan { level },
        ..
    }) = doc.data(view)
    else {
        return;
    };
    for e in doc.of(Category::View) {
        let ElementData::View {
            camera: Some(cam), ..
        } = &e.data
        else {
            continue;
        };
        if cam.level != *level {
            continue;
        }
        let el = Some(e.id);
        let (eye, l, r) = studio_core::camera::cone(cam, 16.0 / 9.0);
        b.line(el, &[l, eye, r], false, 1, Dash::Dashed);
        b.line(el, &[l, r], false, 1, Dash::Dashed);
        b.line(
            el,
            &arc(cam.target, b.paper(1.2), 0.0, std::f64::consts::TAU),
            true,
            1,
            Dash::Solid,
        );
        // The camera: a body behind the eye and a lens toward the target.
        let d = cam.target.sub(cam.eye).norm();
        let n = d.perp();
        let (len, half) = (b.paper(4.0), b.paper(1.5));
        let back = eye.sub(d.scale(b.paper(1.6)));
        let body = [
            back.add(n.scale(half)),
            back.sub(n.scale(half)),
            back.sub(n.scale(half)).sub(d.scale(len)),
            back.add(n.scale(half)).sub(d.scale(len)),
        ];
        b.line(el, &body, true, 2, Dash::Solid);
        let lens = [
            eye,
            back.add(n.scale(half * 0.7)),
            back.sub(n.scale(half * 0.7)),
        ];
        b.fill(el, vec![ring(&lens)], FillKind::Ink);
    }
}

fn callout_markers(doc: &Document, b: &mut Builder, view: ElementId) {
    for e in doc.of(Category::View) {
        let ElementData::View {
            callout_of: Some(parent),
            crop: Some(c),
            name,
            ..
        } = &e.data
        else {
            continue;
        };
        if *parent != view {
            continue;
        }
        let el = Some(e.id);
        // Rounded corners, as Revit draws callouts.
        let r = b
            .paper(3.0)
            .min((c.max.x - c.min.x) / 4.0)
            .min((c.max.y - c.min.y) / 4.0);
        let mut pts = vec![];
        let corners = [
            (
                Pt::new(c.max.x - r, c.min.y + r),
                -std::f64::consts::FRAC_PI_2,
            ),
            (Pt::new(c.max.x - r, c.max.y - r), 0.0),
            (
                Pt::new(c.min.x + r, c.max.y - r),
                std::f64::consts::FRAC_PI_2,
            ),
            (Pt::new(c.min.x + r, c.min.y + r), std::f64::consts::PI),
        ];
        for (center, a0) in corners {
            pts.extend(arc(center, r, a0, std::f64::consts::FRAC_PI_2));
        }
        b.line(el, &pts, true, 3, Dash::Solid);
        // Head: a bubble off the top-right corner with the callout's sheet (if placed).
        let corner = Pt::new(c.max.x, c.max.y);
        let head = corner.add(Pt::new(b.paper(8.0), b.paper(8.0)));
        b.line(
            el,
            &[corner, head.sub(Pt::new(b.paper(3.5), b.paper(3.5)))],
            false,
            2,
            Dash::Solid,
        );
        // Detail number over sheet number, as Revit's callout head.
        let hr = b.paper(5.0);
        b.fill(
            el,
            vec![ring(&arc(head, hr, 0.0, std::f64::consts::TAU))],
            FillKind::Paper,
        );
        b.circle(el, head, 5.0, 2, false);
        b.line(
            el,
            &[head.sub(Pt::new(hr, 0.0)), head.add(Pt::new(hr, 0.0))],
            false,
            1,
            Dash::Solid,
        );
        let (detail, number) = view_ref(doc, e.id).unwrap_or_else(|| ("—".into(), String::new()));
        b.text(
            el,
            head.add(Pt::new(0.0, b.paper(2.1))),
            detail,
            2.4,
            Anchor::Center,
        );
        b.text(
            el,
            head.sub(Pt::new(0.0, b.paper(2.9))),
            number,
            2.0,
            Anchor::Center,
        );
        b.text(
            el,
            head.add(Pt::new(b.paper(7.0), 0.0)),
            name.to_uppercase(),
            2.2,
            Anchor::Left,
        );
    }
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

/// Frame, panel and glass lines of a door or window seen in elevation.
fn opening_elevation_detail(
    b: &mut Builder,
    el: ElementId,
    kind: OpeningKind,
    face: (f64, f64, f64, f64),
    mirrored: bool,
    flip_hand: bool,
) {
    match kind {
        OpeningKind::Window(style) => windows::elevation_detail(b, el, style, face, mirrored),
        OpeningKind::Door(style) => {
            doors::elevation_detail(b, el, style, flip_hand, face, mirrored)
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

/// A door or window placed from the 3D view (ADR-022): where it would go in `host` for a
/// hit at plan point `p`, and its box as triangles for the ghost preview.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct OpeningPreview3d {
    pub preview: OpeningPreview,
    pub positions: Vec<f32>,
}

pub fn opening_preview_3d(
    doc: &Document,
    type_id: ElementId,
    host: ElementId,
    p: Pt,
) -> Option<OpeningPreview3d> {
    let ElementData::Wall { base_level, .. } = doc.data(host).ok()? else {
        return None;
    };
    let plan = doc.of(Category::View).find(|e| {
        matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, callout_of: None, .. } if level == base_level)
    })?;
    let pv = opening_preview(doc, plan.id, type_id, p, 50.0)?;
    if pv.host != host {
        return None;
    }
    let (width, height, sill) = match doc.data(type_id).ok()? {
        ElementData::DoorType { width, height, .. } => (*width, *height, 0.0),
        ElementData::WindowType {
            width,
            height,
            sill,
            ..
        } => (*width, *height, *sill),
        _ => return None,
    };
    let w = regenerate(doc).walls.iter().find(|w| w.id == host)?.clone();
    let d = w.dir();
    let n = d.perp().scale(w.thickness / 2.0 + 20.0);
    let c = w.start.add(d.scale(pv.offset));
    let (a, b2) = (c.sub(d.scale(width / 2.0)), c.add(d.scale(width / 2.0)));
    let prism = studio_geom::Prism {
        base: studio_geom::Poly::simple(vec![a.sub(n), b2.sub(n), b2.add(n), a.add(n)]),
        z0: w.z0 + sill,
        z1: w.z0 + sill + height,
    };
    Some(OpeningPreview3d {
        preview: pv,
        positions: prism.triangles(),
    })
}

/// Where a door or window of `type_id` would go for a cursor at `p` in a plan view.
pub fn opening_preview(
    doc: &Document,
    view: ElementId,
    type_id: ElementId,
    p: Pt,
    tol: f64,
) -> Option<OpeningPreview> {
    // Elevations and sections (ADR-059): on the wall face under the cursor.
    if view_frame(doc, view).is_some() {
        return opening_preview_in_view(doc, view, type_id, p, tol);
    }
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
                t @ ElementData::DoorType { .. } => {
                    OpeningKind::Door(studio_core::doors::DoorStyle::of(t)?)
                }
                _ => return None,
            }
        } else {
            match doc.data(type_id).ok()? {
                t @ ElementData::WindowType { .. } => {
                    OpeningKind::Window(studio_core::windows::WindowStyle::of(t)?)
                }
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

/// Where along a wall `len` long a `width` opening centered at `at` goes: at the wall's
/// center when near it, else with a whole number of inches to the wall's start.
fn opening_offset(at: f64, len: f64, width: f64, tol: f64) -> f64 {
    let hw = width / 2.0;
    let offset = at.clamp(hw, len - hw);
    if (offset - len / 2.0).abs() < tol {
        return len / 2.0;
    }
    let left = ((offset - hw) / MM_PER_IN).round() * MM_PER_IN;
    (left + hw).clamp(hw, len - hw)
}

/// A door or window placed in an elevation or section (ADR-059): in the wall whose face is
/// under the cursor, where the view's line of sight meets it, facing the viewer, at its
/// type's sill. The preview is its outline in the view.
fn opening_preview_in_view(
    doc: &Document,
    view: ElementId,
    type_id: ElementId,
    p: Pt,
    tol: f64,
) -> Option<OpeningPreview> {
    let (origin, right, look) = view_frame(doc, view)?;
    let (_, _, Some((host, t))) = view_refs::model_point(doc, view, p, tol)? else {
        return None;
    };
    let (width, height, sill) = match doc.data(type_id).ok()? {
        ElementData::DoorType { width, height, .. } => (*width, *height, 0.0),
        ElementData::WindowType {
            width,
            height,
            sill,
            ..
        } => (*width, *height, *sill),
        _ => return None,
    };
    let ElementData::View { scale, .. } = doc.data(view).ok()? else {
        return None;
    };
    let model = regenerate(doc);
    let wall = model.walls.iter().find(|w| w.id == host)?;
    let len = wall.start.dist(wall.end);
    if len < width {
        return None;
    }
    let hw = width / 2.0;
    let offset = opening_offset(t * len, len, width, tol);
    let dir = wall.dir();
    // Facing the viewer: the side the elevation looks from.
    let flip_facing = look.dot(dir.perp()) > 0.0;
    let valid = !model
        .openings
        .iter()
        .any(|o| o.host == wall.id && offset - hw < o.t1 - 0.5 && offset + hw > o.t0 + 0.5);
    let u_of = |q: Pt| q.sub(origin).dot(right);
    let (u0, u1) = (
        u_of(wall.start.add(dir.scale(offset - hw))),
        u_of(wall.start.add(dir.scale(offset + hw))),
    );
    let (z0, z1) = (wall.z0 + sill, wall.z0 + sill + height);
    let mut b = Builder::new(f64::from(*scale));
    let ring = [
        Pt::new(u0, z0),
        Pt::new(u1, z0),
        Pt::new(u1, z1),
        Pt::new(u0, z1),
    ];
    b.line(None, &ring, true, 2, Dash::Solid);
    // Its center lines, so it reads as the opening to come.
    b.line(None, &[ring[0], ring[2]], false, 1, Dash::Dashed);
    b.line(None, &[ring[1], ring[3]], false, 1, Dash::Dashed);
    Some(OpeningPreview {
        host: wall.id,
        offset,
        flip_facing,
        valid,
        label: format!(
            "{}  ◂▸  {}",
            format_ft_in(offset - hw),
            format_ft_in(len - offset - hw)
        ),
        items: b.items,
    })
}

/// How a line style draws (ADR-054): pen and pattern.
pub fn line_style(style: studio_core::lines::LineStyle) -> (u8, Dash) {
    use studio_core::lines::LineStyle as L;
    match style {
        L::Thin => (1, Dash::Solid),
        L::Medium => (3, Dash::Solid),
        L::Wide => (5, Dash::Solid),
        L::Hidden => (2, Dash::Dashed),
        L::Centerline => (1, Dash::Center),
        L::Overhead => (1, Dash::Dashed),
        L::Demolished => (2, Dash::Dashed),
        L::Beyond => (1, Dash::Solid),
    }
}

/// Model lines (ADR-054) on `level`, drawn in its plans.
fn plan_model_lines(doc: &Document, b: &mut Builder, level: ElementId) {
    for e in doc.of(Category::ModelLine) {
        if let ElementData::ModelLine {
            level: l,
            curve,
            style,
        } = &e.data
        {
            if *l == level {
                let (w, dash) = line_style(*style);
                b.line(Some(e.id), &curve.points(), curve.is_circle(), w, dash);
            }
        }
    }
}

/// Dimensions, text notes and symbols (ADR-048) owned by `view`.
pub fn annotations(doc: &Document, b: &mut Builder, view: ElementId) {
    for e in doc.iter() {
        match &e.data {
            ElementData::Dimension {
                view: v, offset, ..
            } if *v == view => {
                if let Some((pts, u)) = studio_core::dimension::string_points(doc, &e.data) {
                    dimension_string(b, Some(e.id), &pts, u, *offset);
                }
            }
            ElementData::AngularDimension { view: v, .. } if *v == view => {
                if let Some(arc) = studio_core::dimension::angular_arc(doc, &e.data) {
                    angular(b, Some(e.id), &arc);
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
            ElementData::DetailLine {
                view: v,
                curve,
                style,
            } if *v == view => {
                let (w, dash) = line_style(*style);
                b.line(Some(e.id), &curve.points(), curve.is_circle(), w, dash);
            }
            _ => {}
        }
    }
    symbols::draw_symbols(doc, b, view);
}

/// An aligned dimension from `a` to `p2`: witness lines, dimension line with architectural
/// ticks, and the length in feet-inches above the line, kept upright.
pub fn dimension(b: &mut Builder, el: Option<ElementId>, a: Pt, p2: Pt, offset: f64) {
    dimension_string(b, el, &[a, p2], p2.sub(a).norm(), offset);
}

/// Text rotation along direction `u`, kept upright, and the side "above" it.
fn upright(u: Pt) -> (f64, Pt) {
    let mut angle = u.y.atan2(u.x);
    let mut up = u.perp();
    if angle > std::f64::consts::FRAC_PI_2 + 1e-9 || angle <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
        angle += if angle > 0.0 {
            -std::f64::consts::PI
        } else {
            std::f64::consts::PI
        };
        up = up.scale(-1.0);
    }
    (angle, up)
}

/// A dimension string (ADR-040): references `pts` (in order along `u`) measured along `u`,
/// the dimension line `offset` to the left of the first. A witness line from each
/// reference, a tick at each, and each segment's length above its part of the line.
pub fn dimension_string(b: &mut Builder, el: Option<ElementId>, pts: &[Pt], u: Pt, offset: f64) {
    let Some(&o) = pts.first() else { return };
    let n = u.perp();
    let on_line: Vec<Pt> = pts
        .iter()
        .map(|p| o.add(u.scale(p.sub(o).dot(u))).add(n.scale(offset)))
        .collect();
    for (p, q) in pts.iter().zip(&on_line) {
        let w = q.sub(*p).dot(n);
        let side = if w < 0.0 { -1.0 } else { 1.0 };
        let gap = b.paper(1.5) * side;
        let ext = b.paper(2.0) * side;
        if w.abs() > gap.abs() {
            b.line(
                el,
                &[p.add(n.scale(gap)), q.add(n.scale(ext))],
                false,
                1,
                Dash::Solid,
            );
        }
    }
    let (first, last) = (on_line[0], on_line[on_line.len() - 1]);
    let over = b.paper(2.0);
    b.line(
        el,
        &[first.sub(u.scale(over)), last.add(u.scale(over))],
        false,
        1,
        Dash::Solid,
    );
    let tick = u.add(n).norm().scale(b.paper(1.5));
    for q in &on_line {
        b.line(el, &[q.sub(tick), q.add(tick)], false, 4, Dash::Solid);
    }
    let (angle, _) = upright(u);
    let labels = segment_labels(pts, u, offset, b.paper(1.0));
    for (w, at) in on_line.windows(2).zip(labels) {
        let len = w[1].sub(w[0]).dot(u).abs();
        b.text_rot(el, at, format_ft_in(len), 2.6, Anchor::Center, angle);
    }
}

/// Where each segment's value of a dimension string sits (`paper_mm` is one paper mm in the
/// view's units): above the middle of its part of the dimension line.
pub fn segment_labels(pts: &[Pt], u: Pt, offset: f64, paper_mm: f64) -> Vec<Pt> {
    let Some(&o) = pts.first() else {
        return vec![];
    };
    let n = u.perp();
    let (_, up) = upright(u);
    let on: Vec<Pt> = pts
        .iter()
        .map(|p| o.add(u.scale(p.sub(o).dot(u))).add(n.scale(offset)))
        .collect();
    on.windows(2)
        .map(|w| w[0].lerp(w[1], 0.5).add(up.scale(2.2 * paper_mm)))
        .collect()
}

/// An angular dimension (ADR-040): the arc between its lines with a tick at each end,
/// extension lines along the lines out to the arc where it passes beyond them, and the
/// angle in degrees outside the arc's middle.
pub fn angular(b: &mut Builder, el: Option<ElementId>, arc: &studio_core::dimension::AngleArc) {
    let c = arc.center;
    let r = arc.radius;
    let a0 = arc.from.y.atan2(arc.from.x);
    let steps = 48;
    let pts: Vec<Pt> = (0..=steps)
        .map(|k| {
            let a = a0 + arc.sweep * f64::from(k) / f64::from(steps);
            c.add(Pt::new(a.cos(), a.sin()).scale(r))
        })
        .collect();
    b.line(el, &pts, false, 1, Dash::Solid);
    let gap = b.paper(1.5);
    let ext = b.paper(2.0);
    for (ray, p) in [(arc.from, arc.refs[0]), (arc.to, arc.refs[1])] {
        let d = p.sub(c).dot(ray);
        if d < r - gap {
            b.line(
                el,
                &[
                    c.add(ray.scale(d.max(0.0) + gap)),
                    c.add(ray.scale(r + ext)),
                ],
                false,
                1,
                Dash::Solid,
            );
        }
        // A tick across the arc's end.
        let end = c.add(ray.scale(r));
        let tangent = ray.perp();
        let t = ray.add(tangent).norm().scale(b.paper(1.5));
        b.line(el, &[end.sub(t), end.add(t)], false, 4, Dash::Solid);
    }
    let am = a0 + arc.sweep / 2.0;
    let dir = Pt::new(am.cos(), am.sin());
    let (angle, _) = upright(dir.perp());
    b.text_rot(
        el,
        c.add(dir.scale(r + b.paper(2.4))),
        format!("{:.2}°", arc.degrees()),
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

/// A dimension string from picked references, its line through `cursor` (ADR-040).
pub fn string_preview(
    doc: &Document,
    view: ElementId,
    refs: &[studio_core::dimension::Reference],
    cursor: Pt,
    kind: studio_core::DimKind,
) -> Option<DimensionPreview> {
    let ElementData::View { scale, .. } = doc.data(view).ok()? else {
        return None;
    };
    let plan = studio_core::dimension::plan_string(refs, cursor, kind).ok()?;
    let pts: Vec<Pt> = plan.refs.iter().map(|r| r.0).collect();
    let mut b = Builder::new(f64::from(*scale));
    dimension_string(&mut b, None, &pts, plan.along, plan.offset);
    Some(DimensionPreview {
        offset: plan.offset,
        items: b.items,
    })
}

/// An angular dimension between two picked lines, its arc through `cursor` (ADR-040).
pub fn angular_preview(
    doc: &Document,
    view: ElementId,
    first: &studio_core::dimension::Reference,
    second: &studio_core::dimension::Reference,
    cursor: Pt,
) -> Option<DimensionPreview> {
    let ElementData::View { scale, .. } = doc.data(view).ok()? else {
        return None;
    };
    let arc =
        studio_core::dimension::angle_arc(first.at, first.dir?, second.at, second.dir?, cursor)?;
    let mut b = Builder::new(f64::from(*scale));
    angular(&mut b, None, &arc);
    Some(DimensionPreview {
        offset: 0.0,
        items: b.items,
    })
}

/// Topmost element at `p` (display-list coordinates) within `tol` mm.
pub fn pick(dl: &DisplayList, p: Pt, tol: f64) -> Option<ElementId> {
    pick_all(dl, p, tol).into_iter().next()
}

/// Every element at `p` within `tol` mm, the one a click picks first, then the others
/// in the order Tab steps through them (nearest, then topmost).
pub fn pick_all(dl: &DisplayList, p: Pt, tol: f64) -> Vec<ElementId> {
    // Each element's nearest distance and the draw order (from the top) it was found at.
    let mut hits: Vec<(f64, usize, ElementId)> = vec![];
    for (order, it) in dl.items.iter().rev().enumerate() {
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
        if d > tol {
            continue;
        }
        match hits.iter_mut().find(|h| h.2 == el) {
            Some(h) if d < h.0 => *h = (d, order, el),
            Some(_) => {}
            None => hits.push((d, order, el)),
        }
    }
    hits.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    hits.into_iter().map(|h| h.2).collect()
}

/// One step of Revit's Tab selection: the elements it would select and the status bar's
/// name for them.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PickCandidate {
    pub ids: Vec<ElementId>,
    pub label: String,
}

/// Revit's Tab order for the elements under the cursor (`hits`, the one a click picks
/// first): each element, and after a wall the chain of walls joined to it.
pub fn pick_candidates(doc: &Document, hits: &[ElementId]) -> Vec<PickCandidate> {
    let mut out = vec![];
    for &id in hits {
        let Ok(data) = doc.data(id) else { continue };
        out.push(PickCandidate {
            ids: vec![id],
            label: pick_label(doc, data),
        });
        if data.category() == Category::Wall {
            let chain = studio_core::sketch::wall_chain(doc, id);
            if chain.len() > 1 {
                out.push(PickCandidate {
                    label: format!("Chain of walls ({})", chain.len()),
                    ids: chain,
                });
            }
        }
    }
    out
}

/// "Wall : Generic - 8\"", as Revit's status bar names what's under the cursor.
fn pick_label(doc: &Document, data: &ElementData) -> String {
    let mut cat = String::new();
    for (i, ch) in data.category().as_str().chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            cat.push(' ');
        }
        cat.push(ch);
    }
    let name = data
        .type_id()
        .and_then(|t| doc.data(t).ok())
        .map(ElementData::name)
        .unwrap_or_else(|| data.name());
    if name.is_empty() || name == cat {
        cat
    } else {
        format!("{cat} : {name}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Mesh {
    pub el: ElementId,
    pub category: Category,
    /// Exterior walls render in a different tone.
    pub exterior: bool,
    /// Shaded color from the element's material (ADR-020), when it has one.
    pub color: Option<[u8; 3]>,
    /// The material on its outside face, for renderings (ADR-029).
    pub material: Option<ElementId>,
    /// The level it's on (for placing in 3D, ADR-022).
    pub level: Option<ElementId>,
    /// Triangle soup, 9 floats per triangle, mm, z-up.
    pub positions: Vec<f32>,
    /// The lines to draw, 6 floats per segment (ADR-038); empty to find them from the
    /// triangles.
    pub edges: Vec<f32>,
    /// A lighting fixture's lit lens glows in this color (ADR-057).
    #[ts(optional)]
    pub glow: Option<[u8; 3]>,
}

/// Meshes for a 3D view, without what it hides (ADR-024).
pub fn meshes_in_view(doc: &Document, view: Option<ElementId>) -> Vec<Mesh> {
    let all = meshes(doc);
    let Some(v) = view.and_then(|v| doc.data(v).ok()) else {
        return all;
    };
    all.into_iter()
        .filter(|m| !studio_core::visibility::hidden_in(doc, v, m.el))
        .collect()
}

/// Each element drawn in `view` with its category (for hiding and isolating by category).
pub fn view_categories(doc: &Document, view: ElementId) -> Vec<(ElementId, Category)> {
    let mut ids: Vec<ElementId> = match doc.data(view) {
        Ok(ElementData::View {
            kind: ViewKind::ThreeD,
            ..
        }) => meshes(doc).into_iter().map(|m| m.el).collect(),
        _ => display_list(doc, view)
            .map(|dl| dl.items.into_iter().filter_map(|i| i.el).collect())
            .unwrap_or_default(),
    };
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .filter(|id| *id != view)
        .filter_map(|id| Some((id, doc.data(id).ok()?.category())))
        .collect()
}

/// Meshes for the 3D view.
pub fn meshes(doc: &Document) -> Vec<Mesh> {
    let m = regenerate(doc);
    let mut out = vec![];
    // Sketched wall openings' reveals (ADR-058), dressed like their wall below.
    let mut reveals: Vec<(ElementId, Mesh)> = vec![];
    for w in &m.walls {
        let hosted: Vec<&OpeningSolid> = m.openings.iter().filter(|o| o.host == w.id).collect();
        let mut lines = edges::wall_edges(w, &hosted);
        let positions = if w.holes.is_empty() {
            edges::wall_triangles(w)
        } else {
            let h = edges::wall_with_holes(w, &hosted);
            lines.extend(edges::hole_edges(w));
            for (id, tris) in h.reveals {
                reveals.push((
                    w.id,
                    Mesh {
                        glow: None,
                        edges: vec![],
                        el: id,
                        category: Category::Wall,
                        exterior: w.exterior,
                        color: w.color,
                        material: None,
                        level: Some(w.level),
                        positions: tris,
                    },
                ));
            }
            h.wall
        };
        out.push(Mesh {
            glow: None,
            edges: lines,
            el: w.id,
            category: Category::Wall,
            exterior: w.exterior,
            color: w.color,
            material: None,
            level: Some(w.level),
            positions,
        });
    }
    for o in &m.openings {
        let level = m.walls.iter().find(|w| w.id == o.host).map(|w| w.level);
        match o.kind {
            OpeningKind::Door(style) => {
                // Frame, casing and leaves in the finish (with a color); glass without.
                let p = doors::parts(o, style);
                out.push(Mesh {
                    glow: None,
                    edges: vec![],
                    el: o.id,
                    category: Category::Door,
                    exterior: false,
                    color: Some(style.finish.color()),
                    material: None,
                    level,
                    positions: p.frame,
                });
                if !p.glass.is_empty() {
                    out.push(Mesh {
                        glow: None,
                        edges: vec![],
                        el: o.id,
                        category: Category::Door,
                        exterior: false,
                        color: None,
                        material: None,
                        level,
                        positions: p.glass,
                    });
                }
            }
            OpeningKind::Window(style) => {
                // Two meshes (ADR-031): the frame, sashes and muntins in the frame finish
                // (with a color), and the glass (without).
                let p = windows::parts(o, style);
                out.push(Mesh {
                    glow: None,
                    edges: vec![],
                    el: o.id,
                    category: Category::Window,
                    exterior: false,
                    color: Some(style.finish.color()),
                    material: None,
                    level,
                    positions: p.frame,
                });
                out.push(Mesh {
                    glow: None,
                    edges: vec![],
                    el: o.id,
                    category: Category::Window,
                    exterior: false,
                    color: None,
                    material: None,
                    level,
                    positions: p.glass,
                });
            }
        }
    }
    for s in m.floors.iter().chain(&m.ceilings) {
        out.push(Mesh {
            glow: None,
            edges: vec![],
            el: s.id,
            category: s.category,
            exterior: false,
            color: s.color,
            material: None,
            level: Some(s.level),
            positions: s.triangles(),
        });
    }
    // Model lines (ADR-054): no faces, their segments at the level's height.
    for e in doc.of(Category::ModelLine) {
        let ElementData::ModelLine { level, curve, .. } = &e.data else {
            continue;
        };
        let z = doc.level_elevation(*level).unwrap_or(0.0) as f32;
        let pts = curve.points();
        let mut edges = vec![];
        for w in pts.windows(2) {
            edges.extend_from_slice(&[
                w[0].x as f32,
                w[0].y as f32,
                z,
                w[1].x as f32,
                w[1].y as f32,
                z,
            ]);
        }
        out.push(Mesh {
            glow: None,
            edges,
            el: e.id,
            category: Category::ModelLine,
            exterior: false,
            color: None,
            material: None,
            level: Some(*level),
            positions: vec![],
        });
    }
    for r in &m.roofs {
        out.push(Mesh {
            glow: None,
            edges: vec![],
            el: r.id,
            category: Category::Roof,
            exterior: true,
            color: r.color,
            material: None,
            level: Some(r.level),
            positions: r.triangles(),
        });
    }
    for s in &m.stairs {
        out.push(Mesh {
            glow: None,
            edges: vec![],
            el: s.id,
            category: Category::Stair,
            exterior: false,
            color: None,
            material: None,
            level: Some(s.base_level),
            positions: s.steps.iter().flat_map(|p| p.triangles()).collect(),
        });
    }
    if let Some(s) = &m.site {
        let positions = s.mesh();
        if !positions.is_empty() {
            out.push(Mesh {
                glow: None,
                edges: vec![],
                el: s.id,
                category: Category::Site,
                exterior: false,
                color: Some([184, 196, 160]),
                material: None,
                level: None,
                positions,
            });
        }
    }
    for c in &m.columns {
        out.push(Mesh {
            glow: None,
            edges: vec![],
            el: c.id,
            category: Category::Column,
            exterior: c.structural,
            color: c.color,
            material: None,
            level: Some(c.level),
            positions: c.prism().triangles(),
        });
    }
    for bm in &m.beams {
        out.push(Mesh {
            glow: None,
            edges: vec![],
            el: bm.id,
            category: Category::Beam,
            exterior: true,
            color: bm.color,
            material: None,
            level: Some(bm.level),
            positions: bm.prisms.iter().flat_map(|p| p.triangles()).collect(),
        });
    }
    for r in &m.railings {
        let mut positions: Vec<f32> = r
            .boxes
            .iter()
            .flat_map(studio_geom::box_triangles)
            .collect();
        positions.extend(r.posts.iter().flat_map(|p| p.triangles()));
        let is_stair = m.stairs.iter().any(|s| s.id == r.id);
        out.push(Mesh {
            glow: None,
            edges: vec![],
            el: r.id,
            category: if is_stair {
                Category::Stair
            } else {
                Category::Railing
            },
            exterior: false,
            color: None,
            material: None,
            level: Some(r.level),
            positions,
        });
    }
    // What each surface is made of, for renderings (ADR-029), and paint (ADR-034).
    for mesh in &mut out {
        if !matches!(
            mesh.category,
            Category::Wall
                | Category::Floor
                | Category::Ceiling
                | Category::Roof
                | Category::Column
                | Category::Beam
        ) {
            continue;
        }
        if let Some(p) = studio_core::paint::paint_of(doc, mesh.el) {
            if let Ok(ElementData::Material { color, .. }) = doc.data(p) {
                mesh.color = Some(*color);
            }
            mesh.material = Some(p);
        } else {
            mesh.material = studio_core::library::finish_of(doc, mesh.el);
        }
    }
    for (wall, mut r) in reveals {
        if let Some(w) = out.iter().find(|m| m.el == wall) {
            r.color = w.color;
            r.material = w.material;
        }
        out.push(r);
    }
    lighting::meshes(doc, &mut out);
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

    #[test]
    fn dimension_strings_and_angles_draw_each_value() {
        let mut b = Builder::new(48.0);
        let pts = [
            Pt::new(0.0, 0.0),
            Pt::new(3048.0, 0.0),
            Pt::new(4572.0, 0.0),
        ];
        dimension_string(&mut b, None, &pts, Pt::new(1.0, 0.0), 900.0);
        let texts: Vec<String> = b
            .items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["10'-0\"", "5'-0\""], "one value per segment");
        // Three witness lines, the dimension line and three ticks.
        let lines = b
            .items
            .iter()
            .filter(|i| matches!(i.prim, Prim::Line { .. }))
            .count();
        assert_eq!(lines, 3 + 1 + 3);
        let arc = studio_core::dimension::angle_arc(
            Pt::new(0.0, 0.0),
            Pt::new(1.0, 0.0),
            Pt::new(0.0, 0.0),
            Pt::new(1.0, 1.0).norm(),
            Pt::new(2000.0, 500.0),
        )
        .unwrap();
        let mut b = Builder::new(48.0);
        angular(&mut b, None, &arc);
        assert!(b
            .items
            .iter()
            .any(|i| matches!(&i.prim, Prim::Text { text, .. } if text == "45.00°")));
    }

    #[test]
    fn a_selected_wall_offers_its_distances_and_dimension_values_to_type() {
        use studio_core::dimension::{create_string, references_at, Prefer};
        let (mut doc, l1) = building();
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        let m = regenerate(&doc);
        let south = m
            .walls
            .iter()
            .find(|w| w.start.y.abs() < 1.0 && w.end.y.abs() < 1.0)
            .unwrap()
            .id;
        let north = m
            .walls
            .iter()
            .find(|w| w.start.y > 100.0 && w.end.y > 100.0)
            .unwrap()
            .id;
        let h = handles::handles(&doc, plan, &[south]);
        // 30' to the north wall, centerline to centerline.
        let to = h
            .dims
            .iter()
            .find(|d| d.key == format!("to:{north}"))
            .unwrap();
        assert_eq!(to.value, "30'-0\"");
        // A permanent dimension from the south wall to the north one: its value is typeable.
        let x = 20.0 * MM_PER_FT;
        let a =
            references_at(&doc, plan, Pt::new(x, 3.0), 20.0, Prefer::WallCenterlines)[0].clone();
        let b = references_at(
            &doc,
            plan,
            Pt::new(x, 30.0 * MM_PER_FT + 3.0),
            20.0,
            Prefer::WallCenterlines,
        )[0]
        .clone();
        let d = create_string(
            &mut doc,
            plan,
            &[a, b],
            Pt::new(x + 3000.0, 0.0),
            studio_core::DimKind::Aligned,
        )
        .unwrap();
        let h = handles::handles(&doc, plan, &[south]);
        let v = h
            .dims
            .iter()
            .find(|x| x.key == format!("dim:{d}:0"))
            .unwrap();
        assert_eq!(v.value, "30'-0\"");
        assert!(v.items.is_empty(), "drawn by the dimension itself");
        // Typing 25' moves the south wall 5' north.
        studio_core::dimension::set_dimension_segment(&mut doc, south, d, 0, 25.0 * MM_PER_FT)
            .unwrap();
        let (sa, _) = studio_core::dimension::line_of(&doc, south).unwrap();
        assert!((sa.y - 5.0 * MM_PER_FT).abs() < 1e-6, "{sa:?}");
    }

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
    fn cameras_show_in_plans_of_their_level_only() {
        let (mut doc, l1) = building();
        let plan_of = |doc: &Document, l: ElementId| {
            doc.of(Category::View)
                .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, callout_of: None, site: false, .. } if *level == l))
                .unwrap()
                .id
        };
        let cam = studio_core::camera::create_camera(
            &mut doc,
            l1,
            Pt::new(-5000.0, -5000.0),
            Pt::new(3000.0, 3000.0),
            studio_core::camera::DEFAULT_EYE_HEIGHT,
        )
        .unwrap();
        let v1 = plan_of(&doc, l1);
        let dl = display_list(&doc, v1).unwrap();
        let mine: Vec<_> = dl.items.iter().filter(|i| i.el == Some(cam)).collect();
        assert!(mine.len() >= 5, "cone, far edge, target, body, lens");
        assert_eq!(pick(&dl, Pt::new(-5000.0, -5000.0), 300.0), Some(cam));
        let l2 = doc.levels()[1].0;
        let dl2 = display_list(&doc, plan_of(&doc, l2)).unwrap();
        assert!(dl2.items.iter().all(|i| i.el != Some(cam)));
        // Its grips: the eye and the target.
        let h = handles::handles(&doc, v1, &[cam]);
        let keys: Vec<_> = h.grips.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(keys, ["camera:eye", "camera:target"]);
        studio_core::edit::drag_handle(&mut doc, cam, "camera:target", Pt::new(4000.0, 0.0))
            .unwrap();
        assert_eq!(
            studio_core::camera::camera_of(&doc, cam).unwrap().target,
            Pt::new(4000.0, 0.0)
        );
    }

    #[test]
    fn hide_in_view_drops_elements_and_categories() {
        let (mut doc, l1) = building();
        let v = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        let drawn = |doc: &Document, cat: Category| {
            display_list(doc, v)
                .unwrap()
                .items
                .iter()
                .filter_map(|i| i.el)
                .filter(|e| doc.data(*e).is_ok_and(|d| d.category() == cat))
                .collect::<std::collections::BTreeSet<_>>()
        };
        let walls = drawn(&doc, Category::Wall);
        assert_eq!(walls.len(), 4);
        let first = *walls.iter().next().unwrap();
        // EH on one wall.
        studio_core::visibility::hide_elements(&mut doc, v, &[first]).unwrap();
        let after = drawn(&doc, Category::Wall);
        assert_eq!(after.len(), 3);
        assert!(!after.contains(&first));
        // VH on grids; the walls stay.
        assert_eq!(drawn(&doc, Category::Grid).len(), 1);
        studio_core::visibility::set_category_visible(&mut doc, v, &[Category::Grid], false)
            .unwrap();
        assert!(drawn(&doc, Category::Grid).is_empty());
        assert_eq!(drawn(&doc, Category::Wall).len(), 3);
        // Unhide All brings everything back, undoably.
        studio_core::visibility::unhide_all(&mut doc, v).unwrap();
        assert_eq!(drawn(&doc, Category::Wall).len(), 4);
        assert_eq!(drawn(&doc, Category::Grid).len(), 1);
        // The 3D meshes follow the view's hidden list.
        let v3 = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::ThreeD,
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        let n = meshes_in_view(&doc, Some(v3)).len();
        studio_core::visibility::hide_elements(&mut doc, v3, &[first]).unwrap();
        assert_eq!(meshes_in_view(&doc, Some(v3)).len(), n - 1);
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
        assert!(texts.contains(&"Level 2".to_string()) && texts.contains(&"10' - 0\"".to_string()));
        // The south wall (nearest) is drawn after the north wall (farthest); level heads
        // (drawn last) aside.
        let levels: Vec<ElementId> = doc.levels().iter().map(|l| l.0).collect();
        let fills: Vec<_> = dl
            .items
            .iter()
            .filter(|i| !i.el.is_some_and(|e| levels.contains(&e)))
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
    fn walls_show_their_outline_and_openings_without_seams() {
        let (doc, south, _d, _w) = with_openings();
        let m = regenerate(&doc);
        let wall = m.walls.iter().find(|w| w.id == south).unwrap();
        let hosted: Vec<&OpeningSolid> = m.openings.iter().filter(|o| o.host == south).collect();
        assert_eq!(hosted.len(), 2);
        // The wall is cut in pieces around its door and window...
        assert!(wall.pieces.len() > 3);
        let mesh = meshes(&doc).into_iter().find(|x| x.el == south).unwrap();
        let segs: Vec<[f32; 6]> = mesh
            .edges
            .chunks(6)
            .map(|c| [c[0], c[1], c[2], c[3], c[4], c[5]])
            .collect();
        // ...but draws only its outline (a line along the bottom and top of each side and
        // a corner line where it turns) and each opening (a rectangle on each face and
        // four lines through the wall).
        let ring = wall.footprint.outer.len();
        assert_eq!(segs.len(), 3 * ring + 12 * 2, "{ring} outline points");
        for s in &segs {
            let vertical = (s[0] - s[3]).abs() < 0.01 && (s[1] - s[4]).abs() < 0.01;
            if !vertical {
                continue;
            }
            let (z0, z1) = (s[2].min(s[5]) as f64, s[2].max(s[5]) as f64);
            let full = (z0 - wall.z0).abs() < 0.5 && (z1 - wall.z1).abs() < 0.5;
            let jamb = hosted
                .iter()
                .any(|o| (z0 - o.z0.max(wall.z0)).abs() < 0.5 && (z1 - o.z1).abs() < 0.5);
            assert!(full || jamb, "a seam from {z0} to {z1}");
        }
        // No face lies between two pieces (it would flicker through the surface), and every
        // face looks out of the wall.
        let inside = |p: [f64; 3]| {
            wall.pieces.iter().any(|q| {
                p[2] > q.z0 + 0.1
                    && p[2] < q.z1 - 0.1
                    && studio_geom::point_in_ring(Pt::new(p[0], p[1]), &q.base.outer)
            })
        };
        for t in mesh.positions.chunks(9) {
            let v = |k: usize| [t[k * 3] as f64, t[k * 3 + 1] as f64, t[k * 3 + 2] as f64];
            let (a, b, c) = (v(0), v(1), v(2));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * w[2] - u[2] * w[1],
                u[2] * w[0] - u[0] * w[2],
                u[0] * w[1] - u[1] * w[0],
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len < 1e-6 {
                continue;
            }
            let mid = [
                (a[0] + b[0] + c[0]) / 3.0,
                (a[1] + b[1] + c[1]) / 3.0,
                (a[2] + b[2] + c[2]) / 3.0,
            ];
            let at = |s: f64| {
                [
                    mid[0] + n[0] / len * s,
                    mid[1] + n[1] / len * s,
                    mid[2] + n[2] / len * s,
                ]
            };
            assert!(!inside(at(2.0)), "a face inside the wall at {mid:?}");
            assert!(
                inside(at(-2.0)) || mid[2] <= wall.z0 + 0.1 || mid[2] >= wall.z1 - 0.1,
                "a face looking in at {mid:?}"
            );
        }
        // Other elements leave their lines to the triangles.
        assert!(meshes(&doc)
            .iter()
            .filter(|x| x.category != Category::Wall)
            .all(|x| x.edges.is_empty()));
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

    /// The south wall with a 16" round opening sketched 5'-0" up at 5.5 m (ADR-058).
    fn with_round_opening() -> (Document, ElementId, ElementId) {
        let (mut doc, south, _, _) = with_openings();
        let curves = studio_core::sketch::draw(
            studio_core::sketch::DrawTool::Circle,
            &[Pt::new(5500.0, 1500.0), Pt::new(5900.0, 1500.0)],
            &studio_core::sketch::DrawOptions::default(),
        )
        .unwrap();
        let o = studio_core::wall_opening::finish(&mut doc, None, south, &curves).unwrap();
        (doc, south, o)
    }

    #[test]
    fn a_sketched_opening_cuts_through_the_wall_in_3d() {
        let (doc, south, o) = with_round_opening();
        let m = meshes(&doc);
        let z0 = regenerate(&doc)
            .walls
            .iter()
            .find(|w| w.id == south)
            .unwrap()
            .z0;
        let wall = m.iter().find(|x| x.el == south).unwrap();
        // No face triangle covers the hole's middle.
        let (cx, cz) = (5500.0f32, (z0 + 1500.0) as f32);
        let covers = wall.positions.as_chunks::<9>().0.iter().any(|t| {
            let (a, b, c) = ((t[0], t[2]), (t[3], t[5]), (t[6], t[8]));
            let same_y = (t[1] - t[4]).abs() < 0.01 && (t[4] - t[7]).abs() < 0.01;
            let s =
                |p: (f32, f32), q: (f32, f32)| (q.0 - p.0) * (cz - p.1) - (q.1 - p.1) * (cx - p.0);
            let (d1, d2, d3) = (s(a, b), s(b, c), s(c, a));
            same_y
                && ((d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0) || (d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0))
                && (d1.abs() + d2.abs() + d3.abs()) > 1e-3
        });
        assert!(!covers, "the hole is open");
        // Its reveals are the opening's own mesh, one quad per edge of the circle.
        let r = m.iter().find(|x| x.el == o).unwrap();
        assert_eq!(r.category, Category::Wall);
        assert_eq!(r.positions.len() % 18, 0);
        assert!(r.positions.len() / 18 >= 70);
        // Every reveal faces the hole's axis.
        for t in r.positions.as_chunks::<9>().0 {
            let p = |k: usize| {
                [
                    f64::from(t[k * 3]),
                    f64::from(t[k * 3 + 1]),
                    f64::from(t[k * 3 + 2]),
                ]
            };
            let (a, b, c) = (p(0), p(1), p(2));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let to_axis = [5500.0 - a[0], 0.0, z0 + 1500.0 - a[2]];
            assert!(
                n[0] * to_axis[0] + n[2] * to_axis[2] > 0.0,
                "faces into the hole"
            );
        }
        assert!(
            wall.edges.len()
                > edges::wall_edges(
                    regenerate(&doc)
                        .walls
                        .iter()
                        .find(|w| w.id == south)
                        .unwrap(),
                    &[]
                )
                .len()
        );
    }

    #[test]
    fn a_sketched_opening_is_a_gap_in_plan_a_face_in_elevation_and_in_section() {
        let (mut doc, south, o) = with_round_opening();
        // Plan: cut at 4', through the circle (4'-1" to 6'-3"... its middle is 5'-0").
        let plan = view(&doc, |k| matches!(k, ViewKind::FloorPlan { .. }));
        let dl = display_list(&doc, plan).unwrap();
        let wall_fills: Vec<Vec<Pt>> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(south))
            .filter_map(|i| match &i.prim {
                Prim::Fill { rings, .. } => {
                    Some(rings[0].iter().map(|p| Pt::new(p[0], p[1])).collect())
                }
                _ => None,
            })
            .collect();
        assert!(!wall_fills.is_empty());
        assert!(
            wall_fills
                .iter()
                .all(|r| !point_in_ring(Pt::new(5500.0, 0.0), r)),
            "a gap"
        );
        assert!(wall_fills
            .iter()
            .any(|r| point_in_ring(Pt::new(5000.0, 0.0), r)));
        // Elevation: the opening's own face, round.
        let south_elev = view(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: studio_core::Compass::South
                }
            )
        });
        let dl = display_list(&doc, south_elev).unwrap();
        let face = dl.items.iter().find_map(|i| match &i.prim {
            Prim::Fill {
                rings,
                fill: FillKind::Paper,
            } if i.el == Some(o) => Some(rings[0].len()),
            _ => None,
        });
        assert!(face.unwrap() > 20);
        // A section through the hole: the cut wall stops below it and starts again above.
        let sec = ops::create_section(&mut doc, Pt::new(5500.0, 2000.0), Pt::new(5500.0, -2000.0))
            .unwrap();
        let dl = display_list(&doc, sec).unwrap();
        let z0 = regenerate(&doc)
            .walls
            .iter()
            .find(|w| w.id == south)
            .unwrap()
            .z0;
        let cut_at = |z: f64| {
            dl.items.iter().any(|i| {
                i.el == Some(south)
                    && matches!(&i.prim, Prim::Fill { rings, fill: FillKind::Poche | FillKind::PocheLight }
                        if point_in_ring(Pt::new(rings[0].iter().map(|p| p[0]).sum::<f64>() / rings[0].len() as f64, z), &rings[0].iter().map(|p| Pt::new(p[0], p[1])).collect::<Vec<_>>()))
            })
        };
        assert!(cut_at(z0 + 500.0) && cut_at(z0 + 2300.0));
        assert!(!cut_at(z0 + 1500.0), "the section passes through the hole");
    }

    #[test]
    fn doors_and_windows_place_on_a_wall_face_in_an_elevation() {
        let (doc, south, _, _) = with_openings();
        let elev = view(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: studio_core::Compass::South
                }
            )
        });
        let wt = doc
            .of(Category::WindowType)
            .find(|e| e.data.name().starts_with("Casement"))
            .unwrap()
            .id;
        let w0 = regenerate(&doc)
            .walls
            .iter()
            .find(|w| w.id == south)
            .unwrap()
            .clone();
        // The South elevation looks north: display x is east (the wall's u), y the height.
        let pv = opening_preview(&doc, elev, wt, Pt::new(5500.0, w0.z0 + 1200.0), 20.0).unwrap();
        assert_eq!(pv.host, south);
        assert!((pv.offset - 5500.0).abs() < MM_PER_IN);
        assert!(pv.valid);
        // Facing the viewer, on the south side: the wall's right, so flipped.
        assert_eq!(pv.flip_facing, w0.dir().perp().dot(Pt::new(0.0, 1.0)) > 0.0);
        // Over the door: not valid.
        let over = opening_preview(&doc, elev, wt, Pt::new(3000.0, w0.z0 + 1200.0), 20.0).unwrap();
        assert!(!over.valid);
        // Off the wall: nothing.
        assert!(opening_preview(&doc, elev, wt, Pt::new(5500.0, w0.z1 + 3000.0), 20.0).is_none());
    }

    #[test]
    fn plans_show_door_and_window_frames() {
        let (doc, _, d, w) = with_openings();
        let plan = view(&doc, |k| matches!(k, ViewKind::FloorPlan { .. }));
        let dl = display_list(&doc, plan).unwrap();
        // Closed four-sided outlines of the element: the jambs (and a door's casings).
        let boxes = |el: ElementId| -> Vec<f64> {
            dl.items
                .iter()
                .filter(|i| i.el == Some(el))
                .filter_map(|i| match &i.prim {
                    Prim::Line {
                        pts, closed: true, ..
                    } if pts.len() == 4 => {
                        let p = |k: usize| Pt::new(pts[k][0], pts[k][1]);
                        Some(p(0).dist(p(1)).min(p(1).dist(p(2))))
                    }
                    _ => None,
                })
                .collect()
        };
        let door = boxes(d);
        // Two jambs and four casings; each is narrow (the frame or casing width).
        assert!(door.len() >= 6, "{door:?}");
        assert!(door.iter().all(|s| *s < 150.0));
        let win = boxes(w);
        assert!(win.len() >= 2, "{win:?}");
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
        // (Without the frames' boxes, ADR-060.)
        let symbol = |el: ElementId| {
            dl.items
                .iter()
                .filter(|i| i.el == Some(el))
                .filter(|i| !matches!(&i.prim, Prim::Line { closed: true, w: 1, dash: Dash::Solid, pts } if pts.len() == 4))
                .count()
        };
        assert_eq!(symbol(d), 2);
        assert_eq!(symbol(w), 5);
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
    fn meshes_carry_their_surface_material_and_paint() {
        let (mut doc, _) = building();
        let walls: Vec<ElementId> = doc.of(Category::Wall).map(|e| e.id).collect();
        let finish = studio_core::library::finish_of(&doc, walls[0]);
        let ms = meshes(&doc);
        let wall = ms.iter().find(|m| m.el == walls[0]).unwrap();
        assert_eq!(wall.material, finish, "renders get the type's finish");
        let brick = studio_core::library::add_preset(&mut doc, "masonry-red-brick").unwrap();
        studio_core::paint::paint(&mut doc, &[walls[0]], Some(brick)).unwrap();
        let ms = meshes(&doc);
        let painted = ms.iter().find(|m| m.el == walls[0]).unwrap();
        let other = ms.iter().find(|m| m.el == walls[1]).unwrap();
        assert_eq!(painted.material, Some(brick));
        let Ok(ElementData::Material { color, .. }) = doc.data(brick) else {
            panic!()
        };
        assert_eq!(painted.color, Some(*color));
        assert_eq!(other.material, finish, "only the painted wall changes");
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
            2,
            "a window's frame and its glass"
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

    #[test]
    fn detail_lines_stay_in_their_view_and_model_lines_show_everywhere() {
        use studio_core::lines::{create_lines, LineStyle, LinesOn};
        use studio_core::sketch::{DrawOptions, DrawTool};
        let (mut doc, l1, _, _) = roofed_house();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let south = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::South
                }
            )
        });
        let o = DrawOptions::default();
        let detail = create_lines(
            &mut doc,
            LinesOn::View(plan),
            DrawTool::Line,
            &[Pt::new(0.0, -2000.0), Pt::new(3000.0, -2000.0)],
            &o,
            LineStyle::Hidden,
        )
        .unwrap()[0];
        let model = create_lines(
            &mut doc,
            LinesOn::Level(l1),
            DrawTool::Line,
            &[Pt::new(0.0, -4000.0), Pt::new(3000.0, -4000.0)],
            &o,
            LineStyle::Wide,
        )
        .unwrap()[0];
        let lines = |v, el| {
            display_list(&doc, v)
                .unwrap()
                .items
                .into_iter()
                .filter(move |i| i.el == Some(el))
                .collect::<Vec<_>>()
        };
        // The detail line: in its plan only, dashed (Hidden).
        let d = lines(plan, detail);
        assert_eq!(d.len(), 1);
        assert!(matches!(
            &d[0].prim,
            Prim::Line {
                dash: Dash::Dashed,
                w: 2,
                ..
            }
        ));
        assert!(lines(south, detail).is_empty());
        // The model line: in the plan, and in the elevation at Level 1's height.
        assert!(matches!(
            &lines(plan, model)[0].prim,
            Prim::Line { w: 5, .. }
        ));
        let e = lines(south, model);
        assert_eq!(e.len(), 1);
        let Prim::Line { pts, .. } = &e[0].prim else {
            panic!()
        };
        assert!(pts.iter().all(|p| p[1].abs() < 1e-6));
        assert!(((pts[0][0] - pts[1][0]).abs() - 3000.0).abs() < 1e-6);
        // And in 3D, as edges with no faces.
        let m = meshes(&doc).into_iter().find(|m| m.el == model).unwrap();
        assert!(m.positions.is_empty() && m.edges.len() == 6);
        // Grips at a straight line's ends; the ends snap.
        let h = handles::handles(&doc, plan, &[detail]);
        assert_eq!(
            h.grips.iter().map(|g| g.key.as_str()).collect::<Vec<_>>(),
            ["start", "end"]
        );
        let s = snap::snap(&doc, plan, Pt::new(3010.0, -2010.0), None, 50.0);
        assert!(
            (s.pt.x - 3000.0).abs() < 1e-6 && (s.pt.y + 2000.0).abs() < 1e-6,
            "{s:?}"
        );
    }

    #[test]
    fn a_section_head_is_the_exterior_elevation_mark() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let (start, end) = (Pt::new(0.0, 0.0), Pt::new(10000.0, 0.0));
        let sec = studio_core::ops::create_section(&mut doc, start, end).unwrap();
        let mut b = Builder::new(48.0);
        section_markers(&doc, &mut b);
        let (style, size) = studio_core::detail::mark_symbol(&doc, None, false);
        let r = b.paper(size);
        let c = start.sub(Pt::new(r, 0.0));
        // The same pieces the building elevation mark draws, pointing the way the view looks.
        let mut mark = Builder::new(48.0);
        let look = end.sub(start).norm().perp();
        elevation_mark(&doc, &mut mark, Some(sec), c, &[(sec, look)], (style, size));
        let head: Vec<&Item> = b
            .items
            .iter()
            .filter(|i| mark.items.iter().any(|m| m == *i))
            .collect();
        assert_eq!(
            head.len(),
            mark.items.len(),
            "every piece of the mark is in the head"
        );
        // Its pointer is the large one: the tip √2 radii out along the look.
        let tip = c.add(look.scale(r * std::f64::consts::SQRT_2));
        assert!(b.items.iter().any(
            |i| matches!(&i.prim, Prim::Fill { rings, fill: FillKind::Ink }
            if Pt::new(rings[0][0][0], rings[0][0][1]).dist(tip) < 1e-6)
        ));
    }

    #[test]
    fn pick_all_orders_what_is_under_the_cursor() {
        let (a, b, c) = (ElementId::new(), ElementId::new(), ElementId::new());
        let line = |el, y: f64| Item {
            el: Some(el),
            prim: Prim::Line {
                pts: vec![[0.0, y], [100.0, y]],
                closed: false,
                w: 1,
                dash: Dash::Solid,
            },
        };
        // a at 3 mm, b at 1 mm (drawn twice: its nearest counts), c at 3 mm drawn on top.
        let dl = DisplayList {
            view_type: ViewType::Plan,
            scale: 48,
            bounds: [0.0, 0.0, 100.0, 100.0],
            items: vec![line(a, 3.0), line(b, 4.0), line(b, 1.0), line(c, 3.0)],
        };
        let p = Pt::new(50.0, 0.0);
        assert_eq!(pick_all(&dl, p, 5.0), vec![b, c, a]);
        assert_eq!(pick(&dl, p, 5.0), Some(b));
        assert_eq!(pick_all(&dl, p, 2.0), vec![b]);
        assert!(pick_all(&dl, Pt::new(500.0, 0.0), 5.0).is_empty());
    }

    #[test]
    fn tab_steps_to_a_walls_chain_after_the_wall() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = studio_core::ops::first_of(&doc, Category::WallType).unwrap();
        let w = |doc: &mut Document, a: (f64, f64), b: (f64, f64)| {
            studio_core::ops::create_wall(doc, wt, l1, Pt::new(a.0, a.1), Pt::new(b.0, b.1))
                .unwrap()
        };
        let w1 = w(&mut doc, (0.0, 0.0), (5000.0, 0.0));
        let w2 = w(&mut doc, (5000.0, 0.0), (5000.0, 4000.0));
        let lone = w(&mut doc, (9000.0, 0.0), (9000.0, 4000.0));
        let c = pick_candidates(&doc, &[w1, lone]);
        let type_name = doc.data(wt).unwrap().name();
        assert_eq!(c.len(), 3, "the wall, its chain, the lone wall (no chain)");
        assert_eq!(c[0].ids, vec![w1]);
        assert_eq!(c[0].label, format!("Wall : {type_name}"));
        assert_eq!(c[1].ids.len(), 2);
        assert!(c[1].ids.contains(&w1) && c[1].ids.contains(&w2));
        assert_eq!(c[1].label, "Chain of walls (2)");
        assert_eq!(c[2].ids, vec![lone]);
    }

    #[test]
    fn a_mark_with_several_views_draws_small_tangent_points() {
        let doc = Document::new();
        let mut b = Builder::new(1.0);
        let (v1, v2) = (ElementId::new(), ElementId::new());
        let c = Pt::new(0.0, 0.0);
        let looks = [(v1, Pt::new(0.0, 1.0)), (v2, Pt::new(1.0, 0.0))];
        elevation_mark(
            &doc,
            &mut b,
            None,
            c,
            &looks,
            (studio_core::MarkStyle::CircleArrow, 4.0),
        );
        let tips: Vec<[f64; 2]> = b
            .items
            .iter()
            .filter(|i| i.el == Some(v1) || i.el == Some(v2))
            .filter_map(|i| match &i.prim {
                Prim::Fill { rings, .. } => Some(rings[0][0]),
                _ => None,
            })
            .collect();
        // Each point is r·√2 out (the tangent Filled Arrow between the tangent points).
        for t in &tips {
            assert!((Pt::new(t[0], t[1]).len() - 4.0 * std::f64::consts::SQRT_2).abs() < 1e-9);
        }
        assert_eq!(tips.len(), 2);
        // One view: the standard pointer (its base on the diameter).
        let mut b = Builder::new(1.0);
        elevation_mark(
            &doc,
            &mut b,
            None,
            c,
            &looks[..1],
            (studio_core::MarkStyle::CircleArrow, 4.0),
        );
        let tip = b.items.iter().find_map(|i| match &i.prim {
            Prim::Fill { rings, .. } if i.el == Some(v1) => Some(rings[0][0]),
            _ => None,
        });
        assert!((tip.unwrap()[1] - 4.0 * std::f64::consts::SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn elevation_pointer_is_a_right_angled_point_behind_the_body() {
        let (c, r) = (Pt::new(0.0, 0.0), 100.0);
        let [tip, a, b] = mark_arrow(c, r, Pt::new(-1.0, 0.0));
        let k = r * std::f64::consts::SQRT_2;
        // Its point √2 radii out; its base the diameter across the look, √2 radii each side.
        assert!((tip.x + k).abs() < 1e-9 && tip.y.abs() < 1e-9);
        assert!(a.x.abs() < 1e-9 && b.x.abs() < 1e-9 && (a.dist(b) - 2.0 * k).abs() < 1e-9);
        assert!(a.sub(tip).dot(b.sub(tip)).abs() < 1e-6, "a right angle");
        // Its sides touch the body: one radius from the center.
        let d = tip.sub(a).norm();
        assert!((c.sub(a).cross(d).abs() - r).abs() < 1e-9, "tangent");
    }

    #[test]
    fn revit_filled_arrow_is_tangent_with_a_right_angled_tip() {
        let (c, r) = (Pt::new(0.0, 0.0), 100.0);
        let pts = filled_arrow(c, r, Pt::new(0.0, -1.0));
        let tip = pts[0];
        assert!((tip.x).abs() < 1e-9 && (tip.y + r * std::f64::consts::SQRT_2).abs() < 1e-9);
        let (a, b) = (pts[1], pts[pts.len() - 1]);
        // Both ends are on the body, and the sides from the tip meet at 90° and touch it.
        assert!((a.len() - r).abs() < 1e-9 && (b.len() - r).abs() < 1e-9);
        assert!(a.sub(tip).dot(b.sub(tip)).abs() < 1e-6);
        assert!(a.sub(tip).dot(a.sub(c)).abs() < 1e-6, "tangent");
    }

    #[test]
    fn level_ends_drag_in_one_elevation_like_revit_2d_extents() {
        let (mut doc, _, _, _) = roofed_house();
        let south = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::South
                }
            )
        });
        let north = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::North
                }
            )
        });
        let l2 = doc.levels()[1].0;
        let line = |doc: &Document, v| {
            let dl = display_list(doc, v).unwrap();
            let (_, x0, x1) = level_line(&dl.items, l2).unwrap();
            let name_x = dl
                .items
                .iter()
                .find_map(|i| match &i.prim {
                    Prim::Text { text, at, .. } if i.el == Some(l2) && text == "Level 2" => {
                        Some(at[0])
                    }
                    _ => None,
                })
                .unwrap();
            (x0, x1, name_x)
        };
        let (x0, x1, name) = line(&doc, south);
        let north_before = line(&doc, north);
        // A selected level has a grip at each end.
        let h = handles::handles(&doc, south, &[l2]);
        let keys: Vec<&str> = h.grips.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys[0].starts_with("level_end:") && keys[0].contains(":left:"));
        assert!((h.grips[0].at.x - x0).abs() < 1e-6 && (h.grips[1].at.x - x1).abs() < 1e-6);
        // Pull the right end 10' in: the line ends there and the head follows.
        let right = h.grips[1].clone();
        let to = Pt::new(x1 - 3048.0, 999.0);
        studio_core::edit::drag_handle(&mut doc, l2, &right.key, to).unwrap();
        let (a0, a1, name2) = line(&doc, south);
        assert!(
            (a0 - x0).abs() < 1e-6 && (a1 - (x1 - 3048.0)).abs() < 1e-6,
            "{a0} {a1}"
        );
        assert!((name2 - (name - 3048.0)).abs() < 1e-6);
        // Then the left end 5' in; the right end stays.
        let h = handles::handles(&doc, south, &[l2]);
        studio_core::edit::drag_handle(&mut doc, l2, &h.grips[0].key, Pt::new(x0 + 1524.0, 0.0))
            .unwrap();
        let (b0, b1, _) = line(&doc, south);
        assert!((b0 - (x0 + 1524.0)).abs() < 1e-6 && (b1 - a1).abs() < 1e-6);
        // Never shorter than a foot; other views keep their own ends.
        let h = handles::handles(&doc, south, &[l2]);
        studio_core::edit::drag_handle(&mut doc, l2, &h.grips[1].key, Pt::new(b0 - 5000.0, 0.0))
            .unwrap();
        let (c0, c1, _) = line(&doc, south);
        assert!((c1 - c0 - 304.8).abs() < 1e-6, "{c0} {c1} {b0} {b1}");
        assert_eq!(line(&doc, north), north_before);
        // Each drag is one undo.
        doc.undo().unwrap();
        doc.undo().unwrap();
        doc.undo().unwrap();
        assert_eq!(line(&doc, south), (x0, x1, name));
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
                        fill: FillKind::Poche | FillKind::PocheLight,
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
        // Four walls × (three layer boundaries + the stud cavity's batt zigzag); the closed
        // rectangle has no free ends to wrap.
        assert_eq!(layer_lines(&a), 16);
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
    fn line_len(i: &Item) -> Option<(usize, f64)> {
        match &i.prim {
            Prim::Line { pts, .. } => {
                let l = pts
                    .windows(2)
                    .map(|w| Pt::new(w[0][0], w[0][1]).dist(Pt::new(w[1][0], w[1][1])))
                    .sum();
                Some((pts.len(), l))
            }
            _ => None,
        }
    }

    #[test]
    fn structure_and_l_stair_in_plan_and_section() {
        let (mut doc, l1, _, _) = roofed_house();
        studio_core::structure::ensure_structure_types(&mut doc).unwrap();
        let ft = studio_core::units::MM_PER_FT;
        let l2 = doc.levels()[1].0;
        let ct = doc
            .of(Category::ColumnType)
            .find(|e| e.data.name().starts_with("Concrete Square"))
            .unwrap()
            .id;
        let col = studio_core::structure::create_column(
            &mut doc,
            ct,
            l1,
            Pt::new(20.0 * ft, 15.0 * ft),
            0.0,
        )
        .unwrap();
        let bt = doc
            .of(Category::BeamType)
            .find(|e| e.data.name().starts_with("Steel W12"))
            .unwrap()
            .id;
        let beam = studio_core::structure::create_beam(
            &mut doc,
            bt,
            l2,
            Pt::new(0.0, 15.0 * ft),
            Pt::new(40.0 * ft, 15.0 * ft),
        )
        .unwrap();
        let stair = studio_core::build::create_stair_shaped(
            &mut doc,
            l1,
            Pt::new(30.0 * ft, 2.0 * ft),
            Pt::new(30.0 * ft, 12.0 * ft),
            studio_core::build::DEFAULT_STAIR_WIDTH,
            studio_core::StairShape::LShaped { left: true },
        )
        .unwrap();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let dl = display_list(&doc, plan).unwrap();
        let has = |el: ElementId, f: &dyn Fn(&Prim) -> bool| {
            dl.items.iter().any(|i| i.el == Some(el) && f(&i.prim))
        };
        assert!(has(col, &|p| matches!(
            p,
            Prim::Fill {
                fill: FillKind::Poche,
                ..
            }
        )));
        assert!(
            has(beam, &|p| matches!(
                p,
                Prim::Line {
                    dash: Dash::Dashed,
                    ..
                }
            )),
            "beam overhead"
        );
        assert!(has(
            stair,
            &|p| matches!(p, Prim::Text { text, .. } if text == "UP")
        ));
        // The L's second run (above the cut plane) is dashed, and so is its landing.
        assert!(has(stair, &|p| matches!(
            p,
            Prim::Line {
                dash: Dash::Dashed,
                closed: true,
                ..
            }
        )));
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
        // A section along the beam cuts the column and shows the beam and its flanges.
        let sec = ops::create_section(
            &mut doc,
            Pt::new(-3.0 * ft, 15.0 * ft),
            Pt::new(43.0 * ft, 15.0 * ft),
        )
        .unwrap();
        let dl = display_list(&doc, sec).unwrap();
        let cut = |el: ElementId| {
            dl.items
                .iter()
                .filter(|i| {
                    i.el == Some(el)
                        && matches!(
                            &i.prim,
                            Prim::Fill {
                                fill: FillKind::Poche,
                                ..
                            }
                        )
                })
                .count()
        };
        assert_eq!(cut(col), 1);
        assert_eq!(cut(beam), 3, "flanges and web cut");
        let m = meshes(&doc);
        assert!(m
            .iter()
            .any(|x| x.el == col && x.category == Category::Column && !x.positions.is_empty()));
        assert!(m
            .iter()
            .any(|x| x.el == beam && x.category == Category::Beam));
    }

    #[test]
    fn layers_wrap_at_free_ends_and_show_cut_patterns() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 12"))
            .unwrap()
            .id;
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        ops::set_property(&mut doc, plan, "scale", "24", 0).unwrap();
        let dl = display_list(&doc, plan).unwrap();
        let lines: Vec<(usize, f64)> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(w))
            .filter_map(line_len)
            .collect();
        // Stucco (7/8") and gypsum (5/8") wrap both free ends: a return across the rest.
        let ret = 10.5 * MM_PER_IN;
        let returns = lines
            .iter()
            .filter(|(n, l)| *n == 2 && (l - ret).abs() < 0.5)
            .count();
        assert_eq!(returns, 2, "{lines:?}");
        // The core's layer lines stop short of the ends by the wrap.
        let core = 4000.0 - 2.0 * 0.875 * MM_PER_IN;
        assert!(lines.iter().any(|(n, l)| *n == 2 && (l - core).abs() < 0.5));
        // CMU diagonals one way; the rigid insulation (ADR-020) crosshatches both ways.
        let slopes: Vec<f64> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(w))
            .filter_map(|i| match &i.prim {
                Prim::Line { pts, .. } if pts.len() == 2 => {
                    let (dx, dy) = (pts[1][0] - pts[0][0], pts[1][1] - pts[0][1]);
                    (dx.abs() > 1e-6 && dy.abs() > 1e-6 && dx.hypot(dy) < 400.0).then(|| dy / dx)
                }
                _ => None,
            })
            .collect();
        let up = slopes.iter().filter(|s| (**s - 1.0).abs() < 1e-6).count();
        let down = slopes.iter().filter(|s| (**s + 1.0).abs() < 1e-6).count();
        assert!(up > 50 && down > 5, "{up} / {down}");
        // At 1/8" the wall is plain poché again.
        ops::set_property(&mut doc, plan, "scale", "96", 0).unwrap();
        let dl = display_list(&doc, plan).unwrap();
        assert!(dl
            .items
            .iter()
            .filter(|i| i.el == Some(w))
            .filter_map(line_len)
            .all(|(n, _)| n > 2));
    }

    #[test]
    fn section_through_a_stair_opening_breaks_the_floor() {
        let (mut doc, l1, _, stair) = roofed_house();
        let ft = studio_core::units::MM_PER_FT;
        let l2 = doc.levels()[1].0;
        let ftype = doc
            .of(Category::FloorType)
            .find(|e| e.data.name().starts_with("Wood Joist"))
            .unwrap()
            .id;
        let slab = vec![
            Pt::new(0.0, 0.0),
            Pt::new(40.0 * ft, 0.0),
            Pt::new(40.0 * ft, 30.0 * ft),
            Pt::new(0.0, 30.0 * ft),
        ];
        let floor = ops::create_floor(&mut doc, ftype, l2, slab).unwrap();
        let _ = (l1, stair);
        // Across the stair (at x = 2', from y 8' to 20').
        let sec = ops::create_section(
            &mut doc,
            Pt::new(-3.0 * ft, 14.0 * ft),
            Pt::new(43.0 * ft, 14.0 * ft),
        )
        .unwrap();
        let dl = display_list(&doc, sec).unwrap();
        let cuts = dl
            .items
            .iter()
            .filter(|i| i.el == Some(floor))
            .filter(|i| {
                matches!(
                    &i.prim,
                    Prim::Fill {
                        fill: FillKind::Poche | FillKind::PocheLight,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(cuts, 2, "the floor is cut either side of the opening");
        // Its layers show as lines in the cut.
        let layer_lines = dl
            .items
            .iter()
            .filter(|i| i.el == Some(floor))
            .filter(|i| {
                matches!(
                    &i.prim,
                    Prim::Line {
                        closed: false,
                        w: 1,
                        ..
                    }
                )
            })
            .count();
        assert!(layer_lines >= 6, "{layer_lines}");
    }
    #[test]
    fn surface_patterns_course_and_bond() {
        let rect = [
            Pt::new(0.0, 0.0),
            Pt::new(1000.0, 0.0),
            Pt::new(1000.0, 2000.0),
            Pt::new(0.0, 2000.0),
        ];
        let lap = surface_lines(
            &rect,
            studio_core::SurfacePattern::Lap { spacing: 200.0 },
            1.0,
            10.0,
        );
        assert_eq!(lap.len(), 9, "200 to 1800");
        assert!(lap.iter().all(|s| (s[0].x, s[1].x) == (0.0, 1000.0)));
        let bond = surface_lines(
            &rect,
            studio_core::SurfacePattern::Running {
                course: 500.0,
                unit: 400.0,
            },
            1.0,
            10.0,
        );
        let joints: Vec<&[Pt; 2]> = bond.iter().filter(|s| s[0].x == s[1].x).collect();
        // Four courses, joints at 400/800 and 200/600 alternately.
        assert_eq!(bond.len() - joints.len(), 3);
        assert_eq!(joints.len(), 8);
        assert!(joints.iter().any(|s| s[0].x == 200.0 && s[0].y == 500.0));
        assert!(joints.iter().any(|s| s[0].x == 400.0 && s[0].y == 1000.0));
        // Too fine for the scale: nothing.
        assert!(surface_lines(
            &rect,
            studio_core::SurfacePattern::Lap { spacing: 5.0 },
            1.0,
            10.0
        )
        .is_empty());
    }

    #[test]
    fn siding_shows_in_elevation() {
        let (doc, _, _, _) = roofed_house();
        let south = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::North
                }
            )
        });
        let walls: Vec<ElementId> = doc.of(Category::Wall).map(|e| e.id).collect();
        let dl = display_list(&doc, south).unwrap();
        let courses = dl
            .items
            .iter()
            .filter(|i| i.el.is_some_and(|e| walls.contains(&e)))
            .filter(|i| matches!(&i.prim, Prim::Line { pts, w: 1, .. } if pts.len() == 2 && pts[0][1] == pts[1][1]))
            .count();
        // 8" lap siding on a 10' wall: 14 courses, on the south face at least.
        assert!(courses >= 14, "{courses}");
    }

    #[test]
    fn callouts_mark_their_parent_and_show_the_region_at_detail_scale() {
        let (mut doc, l1, _, _) = roofed_house();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let ft = studio_core::units::MM_PER_FT;
        let c = studio_core::detail::create_callout(
            &mut doc,
            plan,
            Pt::new(-2.0 * ft, -2.0 * ft),
            Pt::new(6.0 * ft, 6.0 * ft),
        )
        .unwrap();
        let parent = display_list(&doc, plan).unwrap();
        assert!(parent
            .items
            .iter()
            .any(|i| i.el == Some(c) && matches!(&i.prim, Prim::Circle { .. })));
        assert!(parent.items.iter().any(|i| i.el == Some(c)
            && matches!(&i.prim, Prim::Text { text, .. } if text == "CALLOUT OF LEVEL 1")));
        let dl = display_list(&doc, c).unwrap();
        assert_eq!(dl.scale, 8);
        let m = 8.0 * 8.0;
        assert_eq!(
            dl.bounds,
            [-2.0 * ft - m, -2.0 * ft - m, 6.0 * ft + m, 6.0 * ft + m]
        );
        // At 1 1/2" the corner's layers and cut patterns show.
        let walls: Vec<ElementId> = doc.of(Category::Wall).map(|e| e.id).collect();
        let thin = dl
            .items
            .iter()
            .filter(|i| i.el.is_some_and(|e| walls.contains(&e)))
            .filter(|i| matches!(&i.prim, Prim::Line { w: 1, .. }))
            .count();
        assert!(thin > 4, "{thin}");
    }

    #[test]
    fn structure_tags_joined_columns_and_separators_in_plan() {
        let (mut doc, l1, _, _) = roofed_house();
        studio_core::structure::ensure_structure_types(&mut doc).unwrap();
        let ft = studio_core::units::MM_PER_FT;
        let g1 = ops::create_grid(
            &mut doc,
            Pt::new(20.0 * ft, -5.0 * ft),
            Pt::new(20.0 * ft, 35.0 * ft),
        )
        .unwrap();
        ops::set_property(&mut doc, g1, "name", "3", 0).unwrap();
        let ga = ops::create_grid(
            &mut doc,
            Pt::new(-5.0 * ft, 15.0 * ft),
            Pt::new(45.0 * ft, 15.0 * ft),
        )
        .unwrap();
        ops::set_property(&mut doc, ga, "name", "B", 0).unwrap();
        let steel = doc
            .of(Category::ColumnType)
            .find(|e| e.data.name().starts_with("Steel W10"))
            .unwrap()
            .id;
        let col = studio_core::structure::create_column(
            &mut doc,
            steel,
            l1,
            Pt::new(20.0 * ft, 15.0 * ft),
            0.0,
        )
        .unwrap();
        let bt = doc
            .of(Category::BeamType)
            .find(|e| e.data.name().starts_with("Steel W12"))
            .unwrap()
            .id;
        let l2 = doc.levels()[1].0;
        let beam = studio_core::structure::create_beam(
            &mut doc,
            bt,
            l2,
            Pt::new(0.0, 15.0 * ft),
            Pt::new(40.0 * ft, 15.0 * ft),
        )
        .unwrap();
        // An architectural column in the west wall joins it.
        let arch = doc
            .of(Category::ColumnType)
            .find(|e| e.data.name().starts_with("Architectural"))
            .unwrap()
            .id;
        let pilaster =
            studio_core::structure::create_column(&mut doc, arch, l1, Pt::new(0.0, 10.0 * ft), 0.0)
                .unwrap();
        let sep = studio_core::detail::create_room_separator(
            &mut doc,
            l1,
            Pt::new(0.0, 25.0 * ft),
            Pt::new(40.0 * ft, 25.0 * ft),
        )
        .unwrap();
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let tagged = ops::tag_all(&mut doc, plan).unwrap();
        assert!(tagged >= 3, "column, pilaster and beam");
        let dl = display_list(&doc, plan).unwrap();
        let text = |s: &str| {
            dl.items
                .iter()
                .any(|i| matches!(&i.prim, Prim::Text { text, .. } if text == s))
        };
        assert!(text("B-3"), "column location mark");
        assert!(text("W12x26"), "beam size");
        let tag_of = |target: ElementId| {
            doc.iter()
                .any(|e| matches!(&e.data, ElementData::Tag { target: t, .. } if *t == target))
        };
        assert!(tag_of(col) && tag_of(beam));
        // The pilaster's poché merges into the wall: no outline of its own.
        assert!(dl
            .items
            .iter()
            .any(|i| i.el == Some(pilaster) && matches!(&i.prim, Prim::Fill { .. })));
        assert!(!dl
            .items
            .iter()
            .any(|i| i.el == Some(pilaster) && matches!(&i.prim, Prim::Line { w: 4, .. })));
        assert!(dl
            .items
            .iter()
            .any(|i| i.el == Some(col) && matches!(&i.prim, Prim::Line { w: 4, .. })));
        assert!(dl
            .items
            .iter()
            .any(|i| i.el == Some(sep) && matches!(&i.prim, Prim::Line { w: 1, .. })));
    }
    #[test]
    fn interior_elevation_markers_crop_to_the_room_and_show_cut_side_walls() {
        let (mut doc, l1, _, _) = roofed_house();
        let ft = studio_core::units::MM_PER_FT;
        let room = ops::create_room(&mut doc, l1, Pt::new(20.0 * ft, 15.0 * ft)).unwrap();
        ops::set_property(&mut doc, room, "name", "Great Room", 0).unwrap();
        // Nearer the north wall: the marker looks north.
        let at = Pt::new(20.0 * ft, 24.0 * ft);
        let m = studio_regen::derived::create_elevation_marker(&mut doc, l1, at, true).unwrap();
        let views = studio_core::detail::marker_views(&doc, m);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].0, studio_core::Compass::North);
        let v = views[0].1;
        assert_eq!(doc.data(v).unwrap().name(), "Great Room - North");
        let dl = display_list(&doc, v).unwrap();
        assert_eq!(dl.view_type, ViewType::Elevation);
        // Cropped to the room plus a foot each side, floor to the level above.
        let width = 40.0 * ft - 8.0 * MM_PER_IN + 2.0 * ft;
        let m8 = 48.0 * 8.0;
        assert!(
            (dl.bounds[2] - dl.bounds[0] - (width + 2.0 * m8)).abs() < 1.0,
            "{:?}",
            dl.bounds
        );
        assert!((dl.bounds[3] - (11.0 * ft + m8)).abs() < 1.0);
        // The east and west walls are cut at the edges.
        let walls: Vec<ElementId> = doc.of(Category::Wall).map(|e| e.id).collect();
        let cut = dl
            .items
            .iter()
            .filter(|i| i.el.is_some_and(|e| walls.contains(&e)))
            .filter(|i| {
                matches!(
                    &i.prim,
                    Prim::Fill {
                        fill: FillKind::Poche | FillKind::PocheLight,
                        ..
                    }
                )
            })
            .count();
        assert!(cut >= 2, "{cut}");
        // The marker in plan: its body and a pointer that is the view.
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let pdl = display_list(&doc, plan).unwrap();
        assert!(pdl
            .items
            .iter()
            .any(|i| i.el == Some(m) && matches!(&i.prim, Prim::Circle { .. })));
        assert!(pdl.items.iter().any(|i| i.el == Some(v)
            && matches!(
                &i.prim,
                Prim::Fill {
                    fill: FillKind::Ink,
                    ..
                }
            )));
        // Check the other three boxes; uncheck one; delete the marker.
        for dir in ["East", "South", "West"] {
            studio_regen::derived::set_property(&mut doc, m, &format!("view_{dir}"), "yes")
                .unwrap();
        }
        assert_eq!(studio_core::detail::marker_views(&doc, m).len(), 4);
        assert!(doc
            .of(Category::View)
            .any(|e| e.data.name() == "Great Room - West"));
        ops::set_property(&mut doc, m, "view_South", "no", 0).unwrap();
        assert_eq!(studio_core::detail::marker_views(&doc, m).len(), 3);
        let before = doc.of(Category::View).count();
        ops::delete(&mut doc, &[m]).unwrap();
        assert_eq!(doc.of(Category::View).count(), before - 3);
    }

    #[test]
    fn level_heads_look_like_revit() {
        let (doc, _, _, _) = roofed_house();
        let south = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::South
                }
            )
        });
        let dl = display_list(&doc, south).unwrap();
        let l2 = doc.levels()[1].0;
        let mine: Vec<&Item> = dl.items.iter().filter(|i| i.el == Some(l2)).collect();
        let quarters = mine
            .iter()
            .filter(|i| {
                matches!(
                    &i.prim,
                    Prim::Fill {
                        fill: FillKind::Ink,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(quarters, 2, "two filled quarters of the datum target");
        // Name above elevation, both above the line.
        let y = |s: &str| {
            mine.iter()
                .find_map(|i| match &i.prim {
                    Prim::Text { text, at, .. } if text == s => Some(at[1]),
                    _ => None,
                })
                .unwrap()
        };
        let z = 10.0 * studio_core::units::MM_PER_FT;
        assert!(y("Level 2") > y("10' - 0\"") && y("10' - 0\"") > z);
    }
    #[test]
    fn marker_types_switch_symbol_and_interior() {
        let (mut doc, l1, _, _) = roofed_house();
        let ft = studio_core::units::MM_PER_FT;
        assert_eq!(doc.count(Category::ElevationMarkerType), 5);
        let m = studio_regen::derived::create_elevation_marker(
            &mut doc,
            l1,
            Pt::new(20.0 * ft, 24.0 * ft),
            true,
        )
        .unwrap();
        let named = |doc: &Document, n: &str| {
            doc.of(Category::ElevationMarkerType)
                .find(|e| e.data.name() == n)
                .unwrap()
                .id
        };
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let v = studio_core::detail::marker_views(&doc, m)[0].1;
        // The diamond: a square body outline and a filled corner.
        let diamond = named(&doc, "Interior Elevation - Diamond");
        ops::set_property(&mut doc, m, "type", &diamond.to_string(), 0).unwrap();
        let dl = display_list(&doc, plan).unwrap();
        assert!(dl.items.iter().any(|i| i.el == Some(m)
            && matches!(&i.prim, Prim::Line { pts, closed: true, .. } if pts.len() == 4)));
        assert!(dl.items.iter().any(|i| i.el == Some(v)
            && matches!(
                &i.prim,
                Prim::Fill {
                    fill: FillKind::Ink,
                    ..
                }
            )));
        // A building type: the view is a plain elevation (no room crop).
        let building = named(&doc, "Building Elevation");
        ops::set_property(&mut doc, m, "type", &building.to_string(), 0).unwrap();
        assert!(matches!(
            doc.data(m).unwrap(),
            ElementData::ElevationMarker {
                interior: false,
                ..
            }
        ));
        let ev = display_list(&doc, v).unwrap();
        assert!(
            ev.bounds[2] - ev.bounds[0] > 40.0 * ft,
            "the whole building"
        );
        // Building elevations pick their mark too.
        let south = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::South
                }
            )
        });
        let half = named(&doc, "Building Elevation - Half Circle");
        ops::set_property(&mut doc, south, "mark_type", &half.to_string(), 0).unwrap();
        assert!(
            matches!(doc.data(south).unwrap(), ElementData::View { mark_type: Some(t), .. } if *t == half)
        );
    }

    #[test]
    fn doors_and_windows_place_from_3d_hits() {
        let (doc, _, _, _) = roofed_house();
        let ft = studio_core::units::MM_PER_FT;
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
        // A hit on the south wall's outer face, 12' along.
        let pv = opening_preview_3d(&doc, dt, south, Pt::new(12.0 * ft, -4.0 * MM_PER_IN)).unwrap();
        assert!(pv.preview.valid && pv.preview.host == south);
        assert!(!pv.positions.is_empty());
        let zmax = pv
            .positions
            .chunks(3)
            .map(|v| v[2])
            .fold(f32::MIN, f32::max);
        assert!(
            (f64::from(zmax) - 84.0 * MM_PER_IN).abs() < 1.0,
            "a 7' door"
        );
        let m = meshes(&doc);
        assert!(m
            .iter()
            .filter(|x| x.category == Category::Wall)
            .all(|x| x.level.is_some()));
    }
    #[test]
    fn site_plan_contours_property_lines_and_ground() {
        use studio_core::site::{set_lot, set_topo, topo_request, GeoFrame, ParcelInfo};
        let (mut doc, _, _, _) = roofed_house();
        let ft = studio_core::units::MM_PER_FT;
        let ring = [
            (37.7749, -122.4194),
            (37.7749, -122.41906),
            (37.77526, -122.41906),
            (37.77526, -122.4194),
        ];
        let site = set_lot(&mut doc, &ring, ParcelInfo::default()).unwrap();
        // Put the house near the lot's middle.
        ops::set_property(&mut doc, site, "offset_e", "20'", 0).unwrap();
        ops::set_property(&mut doc, site, "offset_n", "15'", 0).unwrap();
        let (nx, ny, origin, pts) = topo_request(&doc, 5.0 * ft, 10.0 * ft).unwrap();
        let frame = GeoFrame {
            lat0: 37.77508,
            lon0: -122.41923,
        };
        let m: Vec<Option<f64>> = pts
            .iter()
            .map(|(la, lo)| Some(30.0 + frame.to_local(*la, *lo).y / 10_000.0))
            .collect();
        set_topo(&mut doc, (nx, ny, origin), 5.0 * ft, &m, 1.0).unwrap();
        let sv = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { site: true, .. }))
            .unwrap()
            .id;
        let dl = display_list(&doc, sv).unwrap();
        let mine: Vec<&Item> = dl.items.iter().filter(|i| i.el == Some(site)).collect();
        let texts: Vec<String> = mine
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("N ") && t.ends_with(" E") || t.ends_with(" W")),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.ends_with("'")),
            "distances in decimal feet"
        );
        let contour_lines = mine
            .iter()
            .filter(|i| {
                matches!(
                    &i.prim,
                    Prim::Line {
                        closed: false,
                        w: 1 | 2,
                        ..
                    }
                )
            })
            .count();
        assert!(contour_lines > 20);
        assert!(
            dl.items
                .iter()
                .any(|i| matches!(&i.prim, Prim::Text { text, .. } if text == "N")),
            "north arrow"
        );
        // The Level 1 plan shows the property line but no contours.
        let l1 = doc.levels()[0].0;
        let plan = view_where(
            &doc,
            |k| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
        );
        let pl = display_list(&doc, plan).unwrap();
        assert!(pl.items.iter().any(|i| i.el == Some(site)
            && matches!(
                &i.prim,
                Prim::Line {
                    closed: true,
                    dash: Dash::Center,
                    ..
                }
            )));
        assert!(!pl
            .items
            .iter()
            .any(|i| i.el == Some(site) && matches!(&i.prim, Prim::Line { closed: false, .. })));
        // A section cuts the ground; an elevation draws its ground line; 3D has the surface.
        let sec = ops::create_section(
            &mut doc,
            Pt::new(-10.0 * ft, 15.0 * ft),
            Pt::new(50.0 * ft, 15.0 * ft),
        )
        .unwrap();
        let sd = display_list(&doc, sec).unwrap();
        assert!(sd
            .items
            .iter()
            .any(|i| i.el == Some(site) && matches!(&i.prim, Prim::Fill { .. })));
        let south = view_where(&doc, |k| {
            matches!(
                k,
                ViewKind::Elevation {
                    facing: Compass::South
                }
            )
        });
        let ed = display_list(&doc, south).unwrap();
        assert!(ed
            .items
            .iter()
            .any(|i| i.el == Some(site) && matches!(&i.prim, Prim::Line { w: 4, .. })));
        assert!(meshes(&doc)
            .iter()
            .any(|x| x.category == Category::Site && !x.positions.is_empty()));
    }
}
