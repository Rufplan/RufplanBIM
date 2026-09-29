//! Features: what the architectural model says about structure, found deterministically
//! (levels, heights, outlines, walls and their stacking, openings, cores, open zones,
//! discontinuities). Reads the model; never changes it.

use std::collections::BTreeMap;

use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::{Document, ElementId};
use studio_geom::Pt;
use studio_regen::Model;

use crate::rules::Rules;
use crate::types::*;

const SQFT: f64 = MM_PER_FT * MM_PER_FT;

pub(crate) fn bbox(pts: &[Pt]) -> Option<(Pt, Pt)> {
    studio_geom::bounds_of(pts)
}

/// Shoelace area, positive.
pub(crate) fn area(poly: &[Pt]) -> f64 {
    signed_area(poly).abs()
}

fn signed_area(poly: &[Pt]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

pub(crate) fn centroid(poly: &[Pt]) -> Pt {
    let a = signed_area(poly);
    if a.abs() < 1e-6 {
        let n = poly.len().max(1) as f64;
        let s = poly.iter().fold(Pt::new(0.0, 0.0), |s, p| s.add(*p));
        return s.scale(1.0 / n);
    }
    let n = poly.len();
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let c = p.x * q.y - q.x * p.y;
        cx += (p.x + q.x) * c;
        cy += (p.y + q.y) * c;
    }
    Pt::new(cx / (6.0 * a), cy / (6.0 * a))
}

pub(crate) fn inside(poly: &[Pt], p: Pt) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
    }
    c
}

fn rect(min: Pt, max: Pt) -> Vec<Pt> {
    vec![min, Pt::new(max.x, min.y), max, Pt::new(min.x, max.y)]
}

/// Which way a line runs.
pub(crate) fn axis_of(a: Pt, b: Pt) -> Axis {
    let d = b.sub(a);
    let len = d.len();
    if len < 1e-6 {
        return Axis::Other;
    }
    if (d.y / len).abs() < 0.02 {
        Axis::X
    } else if (d.x / len).abs() < 0.02 {
        Axis::Y
    } else {
        Axis::Other
    }
}

/// Merges intervals, sorted.
pub(crate) fn merge(mut v: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = vec![];
    for (a, b) in v {
        match out.last_mut() {
            Some(l) if a <= l.1 => l.1 = l.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

/// The uses a room name suggests.
fn uses_of(rules: &Rules, name: &str) -> Vec<UseKind> {
    let n = name.to_lowercase();
    UseKind::ALL
        .into_iter()
        .filter(|u| {
            rules
                .occupancy
                .get(u.key())
                .is_some_and(|ks| ks.iter().any(|k| n.contains(k.as_str())))
        })
        .collect()
}

fn has_any(list: &[String], name: &str) -> bool {
    let n = name.to_lowercase();
    list.iter().any(|k| n.contains(k.as_str()))
}

/// Finds the features of `model` (regenerated from `doc`).
pub fn extract(doc: &Document, model: &Model, rules: &Rules) -> Features {
    let g = &rules.general;
    let mut levels_all: Vec<_> = model.levels.clone();
    levels_all.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let built: Vec<_> = levels_all
        .iter()
        .filter(|l| {
            model.walls.iter().any(|w| w.level == l.id)
                || model.floors.iter().any(|f| f.level == l.id)
                || model.rooms.iter().any(|r| r.level == l.id)
        })
        .cloned()
        .collect();
    let _ = doc;

    // Cores: stairs span their levels; rooms named for stairs, elevators, shafts.
    let mut cores: Vec<CoreFeature> = vec![];
    for s in &model.stairs {
        let pts: Vec<Pt> = s.footprint().into_iter().flatten().collect();
        let Some((min, max)) = bbox(&pts) else {
            continue;
        };
        let (b, t) = (
            levels_all
                .iter()
                .find(|l| l.id == s.base_level)
                .map(|l| l.elevation),
            levels_all
                .iter()
                .find(|l| l.id == s.top_level)
                .map(|l| l.elevation),
        );
        for l in &built {
            let on = match (b, t) {
                (Some(b), Some(t)) => l.elevation >= b - 1.0 && l.elevation <= t + 1.0,
                _ => l.id == s.base_level,
            };
            if on {
                cores.push(CoreFeature {
                    kind: CoreKind::Stair,
                    level: l.id,
                    min,
                    max,
                });
            }
        }
    }
    for r in &model.rooms {
        let Some(bd) = &r.boundary else { continue };
        let Some((min, max)) = bbox(bd) else { continue };
        let kind = if has_any(&g.elevator_keywords, &r.name) {
            CoreKind::Elevator
        } else if has_any(&g.shaft_keywords, &r.name) {
            CoreKind::Shaft
        } else if has_any(&g.stair_keywords, &r.name) {
            CoreKind::Stair
        } else {
            continue;
        };
        cores.push(CoreFeature {
            kind,
            level: r.level,
            min,
            max,
        });
    }

    // Walls with their openings.
    let tol = g.stack_tolerance_in * MM_PER_IN;
    let prev_of = |id: ElementId| -> Option<ElementId> {
        let i = built.iter().position(|l| l.id == id)?;
        (i > 0).then(|| built[i - 1].id)
    };
    let mut walls: Vec<WallFeature> = vec![];
    for w in &model.walls {
        if !built.iter().any(|l| l.id == w.level) {
            continue;
        }
        let len = w.start.dist(w.end);
        if len < 1.0 {
            continue;
        }
        let holes = merge(
            model
                .openings
                .iter()
                .filter(|o| o.host == w.id)
                .map(|o| (o.t0.clamp(0.0, len), o.t1.clamp(0.0, len)))
                .collect(),
        );
        let opening_length = holes.iter().map(|h| h.1 - h.0).sum();
        let mut solid = vec![];
        let mut t = 0.0;
        for (a, b) in &holes {
            if *a > t {
                solid.push((t, *a));
            }
            t = t.max(*b);
        }
        if len > t {
            solid.push((t, len));
        }
        // Carried by walls on the level below?
        let stacks = match prev_of(w.level) {
            None => true,
            Some(below) => {
                let u = w.end.sub(w.start).scale(1.0 / len);
                let covered: Vec<(f64, f64)> = model
                    .walls
                    .iter()
                    .filter(|o| o.level == below)
                    .filter_map(|o| {
                        let ol = o.start.dist(o.end);
                        if ol < 1.0 {
                            return None;
                        }
                        let ou = o.end.sub(o.start).scale(1.0 / ol);
                        if (u.x * ou.y - u.y * ou.x).abs() > 0.03 {
                            return None;
                        }
                        let off = o.start.sub(w.start);
                        let dist = (off.x * u.y - off.y * u.x).abs();
                        if dist > tol + 0.5 * w.thickness.max(o.thickness) {
                            return None;
                        }
                        let (a, b) = (off.dot(u), o.end.sub(w.start).dot(u));
                        let (a, b) = (a.min(b).max(0.0), a.max(b).min(len));
                        (b > a).then_some((a, b))
                    })
                    .collect();
                let got: f64 = merge(covered).iter().map(|c| c.1 - c.0).sum();
                got >= g.stack_overlap * len
            }
        };
        let mid = w.start.add(w.end).scale(0.5);
        let at_core = cores.iter().any(|c| {
            c.level == w.level
                && mid.x >= c.min.x - 450.0
                && mid.x <= c.max.x + 450.0
                && mid.y >= c.min.y - 450.0
                && mid.y <= c.max.y + 450.0
        });
        walls.push(WallFeature {
            id: w.id,
            level: w.level,
            start: w.start,
            end: w.end,
            length: len,
            thickness: w.thickness,
            exterior: w.exterior,
            axis: axis_of(w.start, w.end),
            opening_length,
            solid,
            stacks,
            at_core,
        });
    }

    // Rooms: open zones, spans and uses.
    let mut open_zones = vec![];
    let mut room_span: BTreeMap<ElementId, f64> = BTreeMap::new();
    let mut use_counts: BTreeMap<ElementId, BTreeMap<UseKind, usize>> = BTreeMap::new();
    for r in &model.rooms {
        let Some(bd) = &r.boundary else { continue };
        let Some((min, max)) = bbox(bd) else { continue };
        let span = (max.x - min.x).min(max.y - min.y);
        let e = room_span.entry(r.level).or_insert(0.0);
        *e = e.max(span);
        for u in uses_of(rules, &r.name) {
            *use_counts.entry(r.level).or_default().entry(u).or_default() += 1;
        }
        let a = r.area();
        if has_any(&g.open_zone_keywords, &r.name) || a >= g.open_zone_min_area_sf * SQFT {
            open_zones.push(OpenZone {
                room: r.id,
                name: r.name.clone(),
                level: r.level,
                area: a,
                span,
                center: min.add(max).scale(0.5),
                column_free: has_any(&g.column_free_keywords, &r.name),
            });
        }
    }

    // Levels.
    let mut levels: Vec<LevelFeature> = vec![];
    for (i, l) in built.iter().enumerate() {
        let on: Vec<&WallFeature> = walls.iter().filter(|w| w.level == l.id).collect();
        let largest = model
            .floors
            .iter()
            .filter(|f| f.level == l.id)
            .max_by(|a, b| area(&a.base.outer).total_cmp(&area(&b.base.outer)));
        let outline = match largest {
            Some(f) => f.base.outer.clone(),
            None => {
                let pts: Vec<Pt> = model
                    .walls
                    .iter()
                    .filter(|w| w.level == l.id)
                    .flat_map(|w| w.footprint.outer.iter().copied())
                    .collect();
                bbox(&pts).map(|(a, b)| rect(a, b)).unwrap_or_default()
            }
        };
        let top = model
            .walls
            .iter()
            .filter(|w| w.level == l.id)
            .map(|w| w.z1)
            .fold(f64::NAN, f64::max);
        let floor_to_floor = match built.get(i + 1) {
            Some(n) => n.elevation - l.elevation,
            None if top.is_finite() => (top - l.elevation).max(1.0),
            None => 10.0 * MM_PER_FT,
        };
        let wall_length = on.iter().map(|w| w.length).sum::<f64>();
        let stacked: f64 = on.iter().filter(|w| w.stacks).map(|w| w.length).sum();
        let mut uses: Vec<(UseKind, usize)> = use_counts
            .get(&l.id)
            .map(|m| m.iter().map(|(k, v)| (*k, *v)).collect())
            .unwrap_or_default();
        uses.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        levels.push(LevelFeature {
            id: l.id,
            name: l.name.clone(),
            elevation: l.elevation,
            floor_to_floor,
            area: area(&outline),
            outline,
            wall_length,
            exterior_wall_length: on.iter().filter(|w| w.exterior).map(|w| w.length).sum(),
            stacking_ratio: if i == 0 || wall_length < 1.0 {
                1.0
            } else {
                stacked / wall_length
            },
            uses: uses.into_iter().map(|u| u.0).collect(),
            max_room_span: room_span.get(&l.id).copied().unwrap_or(0.0),
        });
    }

    // Discontinuities.
    let mut disc = vec![];
    let name_of = |id: ElementId| {
        levels
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.name.clone())
            .unwrap_or_default()
    };
    for w in &walls {
        if !w.stacks && w.length >= 1500.0 {
            disc.push(Discontinuity {
                kind: DiscontinuityKind::NonStackingWall,
                level: w.level,
                at: w.start.add(w.end).scale(0.5),
                message: format!(
                    "An {} wall on {} isn't carried by a wall below.",
                    if w.exterior { "exterior" } else { "interior" },
                    name_of(w.level)
                ),
            });
        }
    }
    let cant = g.cantilever_ft * MM_PER_FT;
    for pair in levels.windows(2) {
        let (lo, hi) = (&pair[0], &pair[1]);
        if let (Some((a0, a1)), Some((b0, b1))) = (bbox(&lo.outline), bbox(&hi.outline)) {
            let reach = [
                (a0.x - b0.x, Pt::new(b0.x, (b0.y + b1.y) / 2.0)),
                (b1.x - a1.x, Pt::new(b1.x, (b0.y + b1.y) / 2.0)),
                (a0.y - b0.y, Pt::new((b0.x + b1.x) / 2.0, b0.y)),
                (b1.y - a1.y, Pt::new((b0.x + b1.x) / 2.0, b1.y)),
            ];
            for (d, at) in reach {
                if d > cant {
                    disc.push(Discontinuity {
                        kind: DiscontinuityKind::Cantilever,
                        level: hi.id,
                        at,
                        message: format!(
                            "{} reaches {:.0}' past {} below: a cantilever or transfer.",
                            hi.name,
                            d / MM_PER_FT,
                            lo.name
                        ),
                    });
                }
            }
        }
        if lo.area > 0.0 && hi.area < g.setback_ratio * lo.area && hi.area > 0.0 {
            disc.push(Discontinuity {
                kind: DiscontinuityKind::Setback,
                level: hi.id,
                at: centroid(&hi.outline),
                message: format!(
                    "{} is set back ({:.0}% of the floor below).",
                    hi.name,
                    100.0 * hi.area / lo.area
                ),
            });
        }
        if hi.wall_length > 0.0 && lo.wall_length < g.soft_story_ratio * hi.wall_length {
            disc.push(Discontinuity {
                kind: DiscontinuityKind::SoftStory,
                level: lo.id,
                at: centroid(&lo.outline),
                message: format!(
                    "{} has {:.0}% of the wall length of {} above: a possible soft or weak story.",
                    lo.name,
                    100.0 * lo.wall_length / hi.wall_length,
                    hi.name
                ),
            });
        }
    }
    let mut reentrant_corners = 0;
    if let Some(base) = levels.first() {
        let poly = &base.outline;
        if let Some((a, b)) = bbox(poly) {
            let dim = (b.x - a.x).max(b.y - a.y);
            let ccw = signed_area(poly) > 0.0;
            let n = poly.len();
            for i in 0..n {
                let (p, q, r) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
                let cross = (q.x - p.x) * (r.y - q.y) - (q.y - p.y) * (r.x - q.x);
                let reflex = if ccw { cross < -1.0 } else { cross > 1.0 };
                if reflex
                    && p.dist(q) >= g.reentrant_share * dim
                    && q.dist(r) >= g.reentrant_share * dim
                {
                    reentrant_corners += 1;
                    disc.push(Discontinuity {
                        kind: DiscontinuityKind::ReEntrantCorner,
                        level: base.id,
                        at: q,
                        message: "A re-entrant corner: the wings may need collectors or a joint."
                            .into(),
                    });
                }
            }
        }
    }

    // The whole building.
    let all: Vec<Pt> = levels
        .iter()
        .flat_map(|l| l.outline.iter().copied())
        .collect();
    let (min, max) = bbox(&all).unwrap_or((Pt::new(0.0, 0.0), Pt::new(0.0, 0.0)));
    let (w, h) = (max.x - min.x, max.y - min.y);
    let top = model.walls.iter().map(|w| w.z1).fold(f64::NAN, f64::max);
    let bottom = levels.first().map_or(0.0, |l| l.elevation);
    let total_height = if top.is_finite() {
        top - bottom
    } else {
        levels.iter().map(|l| l.floor_to_floor).sum()
    };
    let mut totals: BTreeMap<UseKind, usize> = BTreeMap::new();
    for m in use_counts.values() {
        for (k, v) in m {
            *totals.entry(*k).or_default() += v;
        }
    }
    let mut uses: Vec<(UseKind, usize)> = totals.into_iter().collect();
    uses.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let upper: Vec<f64> = levels.iter().skip(1).map(|l| l.stacking_ratio).collect();
    Features {
        story_count: levels.len(),
        total_height,
        min,
        max,
        aspect_ratio: if w.min(h) > 1.0 {
            w.max(h) / w.min(h)
        } else {
            1.0
        },
        footprint_area: levels.iter().map(|l| l.area).fold(0.0, f64::max),
        max_span: levels
            .iter()
            .map(|l| l.max_room_span)
            .chain(open_zones.iter().map(|z| z.span))
            .fold(0.0, f64::max),
        column_free_span: open_zones
            .iter()
            .filter(|z| z.column_free)
            .map(|z| z.span)
            .fold(0.0, f64::max),
        uses: uses.into_iter().map(|u| u.0).collect(),
        stacking: if upper.is_empty() {
            1.0
        } else {
            upper.iter().sum::<f64>() / upper.len() as f64
        },
        levels,
        walls,
        cores,
        open_zones,
        discontinuities: disc,
        reentrant_corners,
    }
}
