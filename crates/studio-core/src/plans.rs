//! Models from floor plans (ADR-036): Claude reads plan drawings (scans, PDFs, screenshots)
//! and returns, per sheet, the walls, doors, windows, rooms and slab outlines it sees in
//! the image's pixels, with the sheet's scale and an anchor point shared by every floor.
//! This turns that into a model: pixels to feet on one common grid, walls squared up and
//! their ends snapped together, openings placed in the nearest wall, levels, rooms,
//! floors and a roof, as one undoable step.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{
    Category, DoorFamily, ElementData, ElementId, LayerFunction, ViewKind, WallFunction, WallLayer,
    WallTop, WindowFamily,
};
use crate::units::{MM_PER_FT, MM_PER_IN};

/// One plan sheet as Claude read it. Points are image pixels: x right, y down.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanSheet {
    /// The level this plan shows ("First Floor").
    pub level: String,
    /// Floor elevation, feet, relative to the lowest floor (0).
    pub elevation: f64,
    /// Wall height on this floor, feet.
    pub wall_height: f64,
    /// Image pixels per foot of the drawing, from its scale bar or a dimension string.
    pub pixels_per_foot: f64,
    /// A point shared by every sheet (a corner, column or stair that stacks), in this
    /// image's pixels, and where it is on the common grid, in feet.
    pub anchor_px: [f64; 2],
    #[serde(default)]
    pub anchor_ft: [f64; 2],
    #[serde(default)]
    pub walls: Vec<PlanWall>,
    #[serde(default)]
    pub doors: Vec<PlanOpening>,
    #[serde(default)]
    pub windows: Vec<PlanOpening>,
    #[serde(default)]
    pub rooms: Vec<PlanRoom>,
    /// Floor slab outlines (rooms and terraces), polygons in pixels.
    #[serde(default)]
    pub slabs: Vec<Vec<[f64; 2]>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanWall {
    /// Centerline ends, pixels.
    pub a: [f64; 2],
    pub b: [f64; 2],
    /// Inches.
    pub thickness: f64,
    #[serde(default)]
    pub exterior: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanOpening {
    /// The opening's center, pixels.
    pub at: [f64; 2],
    /// Inches.
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    /// Windows: sill above the floor, inches.
    #[serde(default)]
    pub sill: f64,
    /// swing, double, sliding, pocket, bifold, garage, french (doors); fixed, casement,
    /// double_hung, slider, awning (windows).
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlanRoom {
    pub name: String,
    /// A point inside the room, pixels.
    pub at: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanSet {
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub sheets: Vec<PlanSheet>,
    /// hip, gable or flat.
    #[serde(default)]
    pub roof: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PlanReport {
    pub name: String,
    pub levels: usize,
    pub walls: usize,
    pub doors: usize,
    pub windows: usize,
    pub rooms: usize,
    pub floors: usize,
    pub roofs: usize,
    pub warnings: Vec<String>,
}

/// A sheet's pixels to feet on the common grid.
fn to_ft(s: &PlanSheet, p: [f64; 2]) -> Pt {
    let k = 1.0 / s.pixels_per_foot;
    Pt::new(
        s.anchor_ft[0] + (p[0] - s.anchor_px[0]) * k,
        s.anchor_ft[1] - (p[1] - s.anchor_px[1]) * k,
    )
}

/// A wall on the common grid, feet.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Line {
    a: Pt,
    b: Pt,
    t: f64,
    exterior: bool,
}

/// Squares near-orthogonal walls (within 3°) and snaps wall ends within `snap` feet of
/// another wall's end, or onto another wall's line (T joints), together.
fn clean(mut walls: Vec<Line>, snap: f64) -> Vec<Line> {
    for w in &mut walls {
        let d = w.b.sub(w.a);
        let ang = d.y.atan2(d.x).to_degrees().rem_euclid(180.0);
        let near = |t: f64| (ang - t).abs() < 3.0;
        if near(0.0) || near(180.0) {
            let y = (w.a.y + w.b.y) / 2.0;
            w.a.y = y;
            w.b.y = y;
        } else if near(90.0) {
            let x = (w.a.x + w.b.x) / 2.0;
            w.a.x = x;
            w.b.x = x;
        }
    }
    // Snap ends to ends: each cluster takes its mean, keeping squared walls square.
    let n = walls.len();
    let mut ends: Vec<(usize, bool, Pt)> = vec![];
    for (i, w) in walls.iter().enumerate() {
        ends.push((i, false, w.a));
        ends.push((i, true, w.b));
    }
    let mut done = vec![false; ends.len()];
    for i in 0..ends.len() {
        if done[i] {
            continue;
        }
        let group: Vec<usize> = (i..ends.len())
            .filter(|j| !done[*j] && ends[*j].2.dist(ends[i].2) <= snap)
            .collect();
        let c = group
            .iter()
            .fold(Pt::new(0.0, 0.0), |s, j| s.add(ends[*j].2))
            .scale(1.0 / group.len() as f64);
        for j in group {
            done[j] = true;
            let (wi, end, _) = ends[j];
            let w = &mut walls[wi];
            let horizontal = (w.a.y - w.b.y).abs() < 1e-9;
            let vertical = (w.a.x - w.b.x).abs() < 1e-9;
            let p = if end { &mut w.b } else { &mut w.a };
            // A squared wall moves only along itself.
            *p = if horizontal {
                Pt::new(c.x, p.y)
            } else if vertical {
                Pt::new(p.x, c.y)
            } else {
                c
            };
        }
    }
    // Keep square walls square after snapping.
    for w in &mut walls {
        if (w.a.y - w.b.y).abs() < snap * 0.2 && (w.a.x - w.b.x).abs() > snap {
            let y = (w.a.y + w.b.y) / 2.0;
            w.a.y = y;
            w.b.y = y;
        }
        if (w.a.x - w.b.x).abs() < snap * 0.2 && (w.a.y - w.b.y).abs() > snap {
            let x = (w.a.x + w.b.x) / 2.0;
            w.a.x = x;
            w.b.x = x;
        }
    }
    // Ends that stop just short of another wall's line meet it (T joints).
    for i in 0..n {
        for end in [false, true] {
            let p = if end { walls[i].b } else { walls[i].a };
            let dir = walls[i].b.sub(walls[i].a).norm();
            let mut best: Option<(f64, Pt)> = None;
            for (j, o) in walls.iter().enumerate() {
                if j == i {
                    continue;
                }
                let od = o.b.sub(o.a);
                let len = od.len();
                if len < 1e-6 || od.norm().cross(dir).abs() < 0.3 {
                    continue;
                }
                let t = p.sub(o.a).dot(od) / (len * len);
                if !(-0.02..=1.02).contains(&t) {
                    continue;
                }
                let q = o.a.add(od.scale(t.clamp(0.0, 1.0)));
                let d = p.dist(q);
                if d <= snap && best.is_none_or(|b| d < b.0) {
                    best = Some((d, q));
                }
            }
            if let Some((_, q)) = best {
                if end {
                    walls[i].b = q;
                } else {
                    walls[i].a = q;
                }
            }
        }
    }
    walls.retain(|w| w.a.dist(w.b) > 0.5);
    walls
}

fn validate(set: &PlanSet) -> CoreResult<()> {
    if set.sheets.is_empty() {
        return Err(CoreError::Invalid("no plans were read".into()));
    }
    for s in &set.sheets {
        if !(s.pixels_per_foot > 0.5 && s.pixels_per_foot < 500.0) {
            return Err(CoreError::Invalid(format!(
                "{}: the scale ({:.2} px/ft) doesn't look right",
                s.level, s.pixels_per_foot
            )));
        }
    }
    Ok(())
}

/// Builds `set` as the project's building (replacing its walls, openings, rooms, floors
/// and roofs), one undo step.
pub fn build(doc: &mut Document, set: &PlanSet) -> CoreResult<PlanReport> {
    validate(set)?;
    let mark = doc.undo_depth();
    let mut report = PlanReport {
        name: set.name.clone(),
        ..Default::default()
    };
    match build_steps(doc, set, &mut report) {
        Ok(()) => {
            doc.merge_undo(mark, &format!("Model {} from plans", set.name));
            Ok(report)
        }
        Err(e) => {
            while doc.undo_depth() > mark {
                doc.undo()?;
            }
            Err(e)
        }
    }
}

const BUILDING: &[Category] = &[
    Category::Wall,
    Category::Door,
    Category::Window,
    Category::Floor,
    Category::Ceiling,
    Category::Roof,
    Category::Room,
    Category::Stair,
    Category::Railing,
    Category::RoomSeparator,
];

fn build_steps(doc: &mut Document, set: &PlanSet, report: &mut PlanReport) -> CoreResult<()> {
    // Clear the old building.
    let old: Vec<ElementId> = doc
        .iter()
        .filter(|e| BUILDING.contains(&e.category()))
        .map(|e| e.id)
        .collect();
    if !old.is_empty() {
        let pinned: Vec<ElementId> = old
            .iter()
            .copied()
            .filter(|id| crate::visibility::is_pinned(doc, *id))
            .collect();
        if !pinned.is_empty() {
            crate::visibility::set_pinned(doc, &pinned, false)?;
        }
        crate::ops::delete(doc, &old)?;
    }

    // Levels: one per sheet by elevation (sheets of one floor share a level), and a roof
    // level above the top floor.
    let mut sheets: Vec<&PlanSheet> = set.sheets.iter().collect();
    sheets.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let mut floors: Vec<(String, f64, f64)> = vec![];
    for s in &sheets {
        if !floors.iter().any(|f| (f.1 - s.elevation).abs() < 0.5) {
            floors.push((s.level.clone(), s.elevation, s.wall_height.max(7.0)));
        }
    }
    let top = floors.last().map_or(9.0, |f| f.1 + f.2);
    let mut wanted: Vec<(String, f64)> = floors
        .iter()
        .map(|f| (f.0.clone(), f.1 * MM_PER_FT))
        .collect();
    wanted.push(("Roof".into(), top * MM_PER_FT));
    let mut levels: Vec<ElementId> = doc.levels().into_iter().map(|l| l.0).collect();
    while levels.len() < wanted.len() {
        levels.push(crate::ops::create_level(
            doc,
            1.0e6 + levels.len() as f64 * 1000.0,
        )?);
    }
    doc.transact("Set levels", |tx| {
        for (i, id) in levels.iter().enumerate() {
            let (name, z) = match wanted.get(i) {
                Some(w) => w.clone(),
                None => (format!("Level {}", i + 1), (top + (i + 1 - wanted.len()) as f64 * 10.0) * MM_PER_FT),
            };
            let old = tx.data(*id)?.name();
            tx.modify(*id, |d| {
                if let ElementData::Level { name: n, elevation } = d {
                    *n = name.clone();
                    *elevation = z;
                }
            })?;
            let views: Vec<ElementId> = tx
                .of(Category::View)
                .filter(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level }, name, .. } if level == id && *name == old))
                .map(|e| e.id)
                .collect();
            for v in views {
                tx.modify(v, |d| {
                    if let ElementData::View { name: n, .. } = d {
                        *n = name.clone();
                    }
                })?;
            }
        }
        Ok(())
    })?;
    report.levels = floors.len();
    let level_at = |elev: f64| -> (ElementId, f64) {
        let i = floors
            .iter()
            .position(|f| (f.1 - elev).abs() < 0.5)
            .unwrap_or(0);
        (levels[i], floors[i].2)
    };

    // Everything is centred on the lowest floor's walls, around the origin.
    let all: Vec<Pt> = sheets
        .iter()
        .flat_map(|s| {
            s.walls
                .iter()
                .flat_map(move |w| [to_ft(s, w.a), to_ft(s, w.b)])
        })
        .collect();
    let center = studio_geom::bounds_of(&all).map_or(Pt::new(0.0, 0.0), |(lo, hi)| {
        Pt::new(((lo.x + hi.x) / 2.0).round(), ((lo.y + hi.y) / 2.0).round())
    });
    let mm = |p: Pt| p.sub(center).scale(MM_PER_FT);

    let mut wall_types: Vec<(i64, bool, ElementId)> = vec![];
    let mut door_types: Vec<(String, i64, i64, ElementId)> = vec![];
    let mut window_types: Vec<(String, i64, i64, i64, ElementId)> = vec![];
    let mut top_outline: Vec<Pt> = vec![];
    for s in &sheets {
        let (level, height) = level_at(s.elevation);
        // Walls.
        let lines: Vec<Line> = s
            .walls
            .iter()
            .map(|w| Line {
                a: to_ft(s, w.a),
                b: to_ft(s, w.b),
                t: w.thickness.clamp(3.0, 36.0),
                exterior: w.exterior,
            })
            .collect();
        let lines = clean(lines, 0.75);
        let mut made: Vec<(ElementId, Pt, Pt)> = vec![];
        for l in &lines {
            let key = (l.t * 4.0).round() as i64;
            let wt = match wall_types.iter().find(|x| x.0 == key && x.1 == l.exterior) {
                Some(x) => x.2,
                None => {
                    let name = format!(
                        "{} - {}\"",
                        if l.exterior {
                            "Plan Exterior"
                        } else {
                            "Plan Interior"
                        },
                        (l.t * 4.0).round() / 4.0
                    );
                    let t_mm = l.t * MM_PER_IN;
                    let id = doc.transact("Plan wall type", |tx| {
                        Ok(tx.insert(ElementData::WallType {
                            name: name.clone(),
                            thickness: t_mm,
                            function: if l.exterior {
                                WallFunction::Exterior
                            } else {
                                WallFunction::Interior
                            },
                            layers: vec![WallLayer {
                                name: name.clone(),
                                thickness: t_mm,
                                function: LayerFunction::Structure,
                                material: None,
                            }],
                        }))
                    })?;
                    wall_types.push((key, l.exterior, id));
                    id
                }
            };
            let (a, b) = (mm(l.a), mm(l.b));
            let h = height * MM_PER_FT;
            let w = doc.transact("Plan wall", |tx| {
                Ok(tx.insert(ElementData::Wall {
                    type_id: wt,
                    start: a,
                    end: b,
                    base_level: level,
                    base_offset: 0.0,
                    top: WallTop::Unconnected { height: h },
                    location: Default::default(),
                    attach_top: false,
                }))
            });
            if let Ok(w) = w {
                made.push((w, a, b));
                report.walls += 1;
            }
        }
        // Openings go in the nearest wall within 3 feet.
        for (is_door, list) in [(true, &s.doors), (false, &s.windows)] {
            for o in list {
                let p = mm(to_ft(s, o.at));
                let width = o.width.clamp(12.0, 480.0) * MM_PER_IN;
                let mut found = host_of(&made, p, true);
                // Traced with a gap at the opening: close the gap with wall like its
                // neighbour's.
                if found.is_none() {
                    if let Some((from, a, b)) = gap_of(&made, p, width) {
                        let mut d = doc.data(from)?.clone();
                        if let ElementData::Wall { start, end, .. } = &mut d {
                            *start = a;
                            *end = b;
                            let w = doc.transact("Plan wall", |tx| Ok(tx.insert(d)))?;
                            made.push((w, a, b));
                            report.walls += 1;
                            let (tt, _) = studio_geom::project_to_segment(p, a, b);
                            found = Some((w, tt * a.dist(b), 0.0, b.sub(a)));
                        }
                    }
                }
                if found.is_none() {
                    found = host_of(&made, p, false);
                }
                let Some((wall, off, _, dir)) = found else {
                    report.warnings.push(format!(
                        "{}: a {} at ({:.0}, {:.0}) px has no wall near it",
                        s.level,
                        if is_door { "door" } else { "window" },
                        o.at[0],
                        o.at[1]
                    ));
                    continue;
                };
                let len = dir.len();
                let offset = off.clamp(width / 2.0, (len - width / 2.0).max(width / 2.0));
                if width > len {
                    continue;
                }
                let kind = o.kind.to_lowercase();
                let made_ok = if is_door {
                    let h = if o.height > 60.0 {
                        o.height
                    } else if kind.contains("garage") {
                        84.0
                    } else {
                        80.0
                    };
                    let (wk, hk) = ((o.width).round() as i64, h.round() as i64);
                    let ty = match door_types
                        .iter()
                        .find(|x| x.0 == kind && x.1 == wk && x.2 == hk)
                    {
                        Some(x) => x.3,
                        None => {
                            let (family, leaf) = match kind.as_str() {
                                k if k.contains("double") || k.contains("french") => (
                                    DoorFamily::DoubleFlush,
                                    if k.contains("french") {
                                        crate::doors::LeafStyle::FifteenLite
                                    } else {
                                        crate::doors::LeafStyle::Flush
                                    },
                                ),
                                k if k.contains("slid") => {
                                    (DoorFamily::SlidingGlass, crate::doors::LeafStyle::FullLite)
                                }
                                k if k.contains("pocket") => {
                                    (DoorFamily::Pocket, crate::doors::LeafStyle::Flush)
                                }
                                k if k.contains("bifold") => {
                                    (DoorFamily::Bifold, crate::doors::LeafStyle::Flush)
                                }
                                k if k.contains("garage") => {
                                    (DoorFamily::Garage, crate::doors::LeafStyle::Flush)
                                }
                                k if k.contains("glass") => {
                                    (DoorFamily::SingleFlush, crate::doors::LeafStyle::FullLite)
                                }
                                _ => (DoorFamily::SingleFlush, crate::doors::LeafStyle::Flush),
                            };
                            let spec = crate::doors::DoorSpec {
                                family,
                                leaf,
                                panels: 0,
                                width,
                                height: h * MM_PER_IN,
                                finish: None,
                            };
                            let id = crate::doors::load(doc, &[spec])
                                .ok()
                                .and_then(|v| v.first().copied())
                                .or_else(|| crate::ops::first_of(doc, Category::DoorType));
                            let Some(id) = id else { continue };
                            door_types.push((kind.clone(), wk, hk, id));
                            id
                        }
                    };
                    crate::ops::create_door(doc, ty, wall, offset, false).is_ok()
                } else {
                    let h = if o.height > 6.0 { o.height } else { 48.0 };
                    let sill = if o.sill > 0.0 {
                        o.sill
                    } else {
                        (84.0 - h).max(12.0)
                    };
                    let (wk, hk, sk) = (
                        o.width.round() as i64,
                        h.round() as i64,
                        sill.round() as i64,
                    );
                    let ty = match window_types
                        .iter()
                        .find(|x| x.0 == kind && x.1 == wk && x.2 == hk && x.3 == sk)
                    {
                        Some(x) => x.4,
                        None => {
                            let family = match kind.as_str() {
                                k if k.contains("casement") => WindowFamily::Casement,
                                k if k.contains("hung") => WindowFamily::DoubleHung,
                                k if k.contains("slid") => WindowFamily::Slider,
                                k if k.contains("awning") => WindowFamily::Awning,
                                k if k.contains("store") || k.contains("curtain") => {
                                    WindowFamily::Storefront
                                }
                                _ => WindowFamily::Fixed,
                            };
                            let spec = crate::windows::WindowSpec {
                                family,
                                units: 1,
                                width,
                                height: h * MM_PER_IN,
                                sill: sill * MM_PER_IN,
                                grille: Default::default(),
                                finish: Default::default(),
                            };
                            let id = crate::windows::load(doc, &[spec])
                                .ok()
                                .and_then(|v| v.first().copied())
                                .or_else(|| crate::ops::first_of(doc, Category::WindowType));
                            let Some(id) = id else { continue };
                            window_types.push((kind.clone(), wk, hk, sk, id));
                            id
                        }
                    };
                    crate::ops::create_window(doc, ty, wall, offset, false).is_ok()
                };
                if made_ok {
                    if is_door {
                        report.doors += 1;
                    } else {
                        report.windows += 1;
                    }
                }
            }
        }
        // Slabs: as read, else the outline of the level's walls.
        let floor_type = crate::ops::first_of(doc, Category::FloorType);
        let mut slabs: Vec<Vec<Pt>> = s
            .slabs
            .iter()
            .map(|poly| poly.iter().map(|p| mm(to_ft(s, *p))).collect::<Vec<Pt>>())
            .filter(|p| p.len() >= 3 && studio_geom::signed_area(p).abs() > 1.0e6)
            .collect();
        if slabs.is_empty() {
            let m = studio_regen_outline(doc, level);
            slabs.extend(m);
        }
        if let Some(ft) = floor_type {
            for poly in &slabs {
                if crate::ops::create_floor(doc, ft, level, poly.clone()).is_ok() {
                    report.floors += 1;
                }
            }
        }
        if let Some(big) = slabs.iter().max_by(|a, b| {
            studio_geom::signed_area(a)
                .abs()
                .total_cmp(&studio_geom::signed_area(b).abs())
        }) {
            if s.elevation >= sheets.last().map_or(0.0, |x| x.elevation) - 0.5 {
                top_outline = big.clone();
            }
        }
        // Rooms.
        for r in &s.rooms {
            let p = mm(to_ft(s, r.at));
            if let Ok(id) = crate::ops::create_room(doc, level, p) {
                let name = r.name.trim();
                if !name.is_empty() {
                    let _ = crate::ops::set_property(doc, id, "name", name, 0);
                }
                report.rooms += 1;
            }
        }
    }

    // The roof over the top floor.
    if top_outline.len() >= 3 {
        if let Some(rt) = crate::ops::first_of(doc, Category::RoofType) {
            let roof_level = levels[floors.len()];
            let kind = set.roof.to_lowercase();
            let slope = if kind == "flat" || kind.is_empty() {
                0.0
            } else {
                (6.0_f64 / 12.0).atan()
            };
            let made = if slope == 0.0 {
                crate::build::create_roof(doc, rt, roof_level, 0.0, top_outline.clone(), 0.0)
                    .map(|_| 1)
            } else {
                crate::build::create_roofs_by_footprint(
                    doc,
                    rt,
                    roof_level,
                    0.0,
                    &top_outline,
                    18.0 * MM_PER_IN,
                    slope,
                )
                .map(|v| v.len())
            };
            match made {
                Ok(n) => report.roofs += n,
                Err(e) => report.warnings.push(format!("Roof: {e}")),
            }
        }
    }
    if report.walls == 0 {
        return Err(CoreError::Invalid(
            "no walls could be read from the plans".into(),
        ));
    }
    Ok(())
}

/// The wall within 3 feet of `p` and alongside it: (wall, distance along it, 0, direction).
/// With `beside` false, a wall whose end is nearest will do.
fn host_of(made: &[(ElementId, Pt, Pt)], p: Pt, beside: bool) -> Option<(ElementId, f64, f64, Pt)> {
    made.iter()
        .filter_map(|(id, a, b)| {
            let (t, d) = studio_geom::project_to_segment(p, *a, *b);
            let inside = t > 0.0 && t < 1.0;
            (d <= 3.0 * MM_PER_FT && (inside || !beside)).then_some((
                *id,
                t * a.dist(*b),
                d,
                b.sub(*a),
            ))
        })
        .min_by(|x, y| x.2.total_cmp(&y.2))
        .map(|(id, off, _, dir)| (id, off, 0.0, dir))
}

/// Two wall ends facing each other across `p` (a gap left where an opening is): the
/// wall to copy and the gap's ends.
fn gap_of(made: &[(ElementId, Pt, Pt)], p: Pt, width: f64) -> Option<(ElementId, Pt, Pt)> {
    let reach = width / 2.0 + 3.0 * MM_PER_FT;
    let ends: Vec<(usize, Pt, Pt)> = made
        .iter()
        .enumerate()
        .flat_map(|(i, (_, a, b))| [(i, *a, b.sub(*a).norm()), (i, *b, a.sub(*b).norm())])
        .filter(|(_, e, _)| e.dist(p) <= reach)
        .collect();
    let mut best: Option<(f64, ElementId, Pt, Pt)> = None;
    for (i, e1, _) in &ends {
        for (j, e2, _) in &ends {
            if i >= j {
                continue;
            }
            let gap = e2.sub(*e1);
            if gap.len() < width * 0.8 || gap.len() > width + 6.0 * MM_PER_FT {
                continue;
            }
            // Both walls run along the gap, and the opening sits in it.
            let g = gap.norm();
            let along = |k: usize| {
                let (_, a, b) = made[k];
                b.sub(a).norm().cross(g).abs() < 0.2
            };
            let (t, d) = studio_geom::project_to_segment(p, *e1, *e2);
            if along(*i)
                && along(*j)
                && d <= 1.5 * MM_PER_FT
                && (0.05..=0.95).contains(&t)
                && best.is_none_or(|x| d < x.0)
            {
                best = Some((d, made[*i].0, *e1, *e2));
            }
        }
    }
    best.map(|(_, w, a, b)| (w, a, b))
}

/// The outline of a level's walls, from the model as built so far.
fn studio_regen_outline(doc: &Document, level: ElementId) -> Vec<Vec<Pt>> {
    // Walls' outer faces: the convex hull of their ends is a fair slab when nothing better
    // was read.
    let pts: Vec<Pt> = doc
        .of(Category::Wall)
        .filter_map(|e| match &e.data {
            ElementData::Wall {
                start,
                end,
                base_level,
                ..
            } if *base_level == level => Some([*start, *end]),
            _ => None,
        })
        .flatten()
        .collect();
    if pts.len() < 3 {
        return vec![];
    }
    vec![studio_geom::convex_hull(&pts)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    fn sheet(level: &str, elevation: f64) -> PlanSheet {
        // A 40' x 30' box drawn at 10 px/ft, its SW corner at pixel (100, 400).
        let px = |x: f64, y: f64| [100.0 + x * 10.0, 400.0 - y * 10.0];
        let wall = |a: [f64; 2], b: [f64; 2]| PlanWall {
            a,
            b,
            thickness: 8.0,
            exterior: true,
        };
        PlanSheet {
            level: level.into(),
            elevation,
            wall_height: 9.0,
            pixels_per_foot: 10.0,
            anchor_px: px(0.0, 0.0),
            anchor_ft: [0.0, 0.0],
            walls: vec![
                wall(px(0.0, 0.0), px(40.0, 0.2)),
                wall(px(40.1, 0.0), px(40.0, 30.0)),
                wall(px(40.0, 30.0), px(0.0, 29.9)),
                wall(px(0.3, 30.0), px(0.0, 0.0)),
                PlanWall {
                    a: px(20.0, 0.4),
                    b: px(20.0, 29.6),
                    thickness: 4.5,
                    exterior: false,
                },
            ],
            doors: vec![PlanOpening {
                at: px(10.0, 0.0),
                width: 36.0,
                height: 80.0,
                sill: 0.0,
                kind: "swing".into(),
            }],
            windows: vec![PlanOpening {
                at: px(40.0, 15.0),
                width: 48.0,
                height: 48.0,
                sill: 36.0,
                kind: "casement".into(),
            }],
            rooms: vec![
                PlanRoom {
                    name: "Living".into(),
                    at: px(10.0, 15.0),
                },
                PlanRoom {
                    name: "Dining".into(),
                    at: px(30.0, 15.0),
                },
            ],
            slabs: vec![],
        }
    }

    #[test]
    fn cleaning_squares_and_snaps_walls() {
        let l = |a: (f64, f64), b: (f64, f64)| Line {
            a: Pt::new(a.0, a.1),
            b: Pt::new(b.0, b.1),
            t: 6.0,
            exterior: true,
        };
        let out = clean(
            vec![
                l((0.0, 0.0), (10.0, 0.3)),
                l((10.2, 0.1), (10.0, 8.0)),
                l((5.0, 0.5), (5.0, 8.0)),
            ],
            0.75,
        );
        // Squared: the first wall is level, the second plumb, and they share a corner.
        assert!((out[0].a.y - out[0].b.y).abs() < 1e-9);
        assert!((out[1].a.x - out[1].b.x).abs() < 1e-9);
        assert!(out[0].b.dist(out[1].a) < 1e-9, "{out:?}");
        // The third stopped short of the first and now meets it (a T).
        assert!((out[2].a.y - out[0].a.y).abs() < 1e-9, "{out:?}");
    }

    #[test]
    fn a_door_in_a_traced_gap_closes_the_gap() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let mut s = sheet("First Floor", 0.0);
        // The south wall traced in two pieces, leaving a 4' gap where the door is.
        let px = |x: f64, y: f64| [100.0 + x * 10.0, 400.0 - y * 10.0];
        s.walls[0].b = px(8.0, 0.0);
        s.walls.push(PlanWall {
            a: px(12.0, 0.0),
            b: px(40.0, 0.0),
            thickness: 8.0,
            exterior: true,
        });
        // The door sits off the centerline, in the gap.
        s.doors[0].at = px(10.0, 0.5);
        s.windows.clear();
        let set = PlanSet {
            name: "Gap".into(),
            summary: String::new(),
            sheets: vec![s],
            roof: "flat".into(),
        };
        let r = build(&mut doc, &set).unwrap();
        assert_eq!((r.walls, r.doors), (7, 1), "{r:?}");
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    #[test]
    fn two_sheets_become_a_two_storey_building() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        // The second floor's sheet is drawn elsewhere on its image, and at 12 px/ft: its
        // anchor lines it up.
        let mut up = sheet("Second Floor", 10.0);
        let shift = |p: [f64; 2]| [300.0 + (p[0] - 100.0) * 1.2, 900.0 + (p[1] - 400.0) * 1.2];
        up.pixels_per_foot = 12.0;
        up.anchor_px = shift(up.anchor_px);
        for w in &mut up.walls {
            w.a = shift(w.a);
            w.b = shift(w.b);
        }
        for o in up.doors.iter_mut().chain(up.windows.iter_mut()) {
            o.at = shift(o.at);
        }
        up.doors.clear();
        for r in &mut up.rooms {
            r.at = shift(r.at);
        }
        let set = PlanSet {
            name: "Box".into(),
            summary: String::new(),
            sheets: vec![sheet("First Floor", 0.0), up],
            roof: "flat".into(),
        };
        let before = doc.undo_depth();
        let r = build(&mut doc, &set).unwrap();
        assert_eq!(doc.undo_depth(), before + 1, "one undo step");
        assert_eq!(
            (r.levels, r.walls, r.doors, r.windows),
            (2, 10, 1, 2),
            "{r:?}"
        );
        assert_eq!(r.rooms, 4);
        assert_eq!(r.roofs, 1);
        let levels = doc.levels();
        assert_eq!(
            levels.iter().map(|l| l.1.as_str()).collect::<Vec<_>>(),
            ["First Floor", "Second Floor", "Roof"]
        );
        // Both floors' walls stack: the same extents on each level.
        let extent = |lv: ElementId| {
            let pts: Vec<Pt> = doc
                .of(Category::Wall)
                .filter_map(|e| match &e.data {
                    ElementData::Wall {
                        start,
                        end,
                        base_level,
                        ..
                    } if *base_level == lv => Some([*start, *end]),
                    _ => None,
                })
                .flatten()
                .collect();
            studio_geom::bounds_of(&pts).unwrap()
        };
        let (a, b) = (extent(levels[0].0), extent(levels[1].0));
        assert!(a.0.dist(b.0) < 30.0 && a.1.dist(b.1) < 30.0, "{a:?} {b:?}");
        assert!(((a.1.x - a.0.x) - 40.0 * MM_PER_FT).abs() < 60.0);
        // The rooms are named.
        assert!(doc
            .of(Category::Room)
            .any(|e| matches!(&e.data, ElementData::Room { name, .. } if name == "Living")));
        // A bad scale is refused before anything changes.
        let mut bad = set.clone();
        bad.sheets[0].pixels_per_foot = 0.0;
        assert!(build(&mut doc, &bad).is_err());
    }
}
