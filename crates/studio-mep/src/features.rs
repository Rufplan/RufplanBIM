//! Building features for MEP, found deterministically from the model: levels and heights,
//! rooms typed by name with their areas and outside walls, exterior walls and doors, and
//! cores. Reads the model; never changes it.

use std::collections::BTreeMap;

use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::Document;
use studio_geom::Pt;
use studio_regen::{Model, OpeningKind};

use crate::rules::Rules;
use crate::types::*;

pub(crate) const SQFT: f64 = MM_PER_FT * MM_PER_FT;

pub(crate) fn bbox(pts: &[Pt]) -> Option<(Pt, Pt)> {
    studio_geom::bounds_of(pts)
}

pub(crate) fn area(poly: &[Pt]) -> f64 {
    let n = poly.len();
    ((0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0)
        .abs()
}

pub(crate) fn centroid(poly: &[Pt]) -> Pt {
    let n = poly.len();
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let c = p.x * q.y - q.x * p.y;
        a += c;
        cx += (p.x + q.x) * c;
        cy += (p.y + q.y) * c;
    }
    if a.abs() < 1e-6 {
        let s = poly.iter().fold(Pt::new(0.0, 0.0), |s, p| s.add(*p));
        return s.scale(1.0 / n.max(1) as f64);
    }
    Pt::new(cx / (3.0 * a), cy / (3.0 * a))
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

/// Reads the building for MEP.
pub fn extract(doc: &Document, model: &Model, rules: &Rules) -> Features {
    let _ = doc;
    let mut lv = model.levels.clone();
    lv.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let built: Vec<_> = lv
        .into_iter()
        .filter(|l| {
            model.walls.iter().any(|w| w.level == l.id)
                || model.floors.iter().any(|f| f.level == l.id)
                || model.rooms.iter().any(|r| r.level == l.id)
        })
        .collect();
    let mut levels: Vec<Level> = vec![];
    for (i, l) in built.iter().enumerate() {
        let outline = match model
            .floors
            .iter()
            .filter(|f| f.level == l.id)
            .max_by(|a, b| area(&a.base.outer).total_cmp(&area(&b.base.outer)))
        {
            Some(f) => f.base.outer.clone(),
            None => {
                let pts: Vec<Pt> = model
                    .walls
                    .iter()
                    .filter(|w| w.level == l.id)
                    .flat_map(|w| w.footprint.outer.iter().copied())
                    .collect();
                bbox(&pts)
                    .map(|(a, b)| vec![a, Pt::new(b.x, a.y), b, Pt::new(a.x, b.y)])
                    .unwrap_or_default()
            }
        };
        let wall_top = model
            .walls
            .iter()
            .filter(|w| w.level == l.id)
            .map(|w| w.z1)
            .fold(f64::NAN, f64::max);
        let top = match built.get(i + 1) {
            Some(n) => n.elevation,
            None if wall_top.is_finite() => wall_top.max(l.elevation + 1.0),
            None => l.elevation + 10.0 * MM_PER_FT,
        };
        levels.push(Level {
            id: l.id,
            name: l.name.clone(),
            elevation: l.elevation,
            top,
            area: area(&outline),
            outline,
        });
    }

    // Exterior walls, pointing out.
    let mut exterior_walls = vec![];
    for w in model.walls.iter().filter(|w| w.exterior) {
        let Some(l) = levels.iter().find(|l| l.id == w.level) else {
            continue;
        };
        let d = w.end.sub(w.start);
        if d.len() < 1.0 {
            continue;
        }
        let c = centroid(&l.outline);
        let mut n = d.scale(1.0 / d.len()).perp();
        if w.start.add(w.end).scale(0.5).sub(c).dot(n) < 0.0 {
            n = n.scale(-1.0);
        }
        exterior_walls.push(ExteriorWall {
            level: w.level,
            start: w.start,
            end: w.end,
            thickness: w.thickness,
            outward: n,
        });
    }
    let exterior_doors = model
        .openings
        .iter()
        .filter(|o| matches!(o.kind, OpeningKind::Door(_)))
        .filter_map(|o| {
            let w = model.walls.iter().find(|w| w.id == o.host && w.exterior)?;
            let ew = exterior_walls
                .iter()
                .find(|e| e.start == w.start && e.end == w.end)?;
            Some(Door {
                level: w.level,
                at: o.at((o.t0 + o.t1) / 2.0),
                outward: ew.outward,
            })
        })
        .collect();

    // Spaces.
    let tol = rules.general.exterior_tol_in * MM_PER_IN;
    let mut spaces = vec![];
    for r in &model.rooms {
        let Some(ring) = &r.boundary else { continue };
        let Some((min, max)) = bbox(ring) else {
            continue;
        };
        let mut exterior_len = 0.0;
        let mut best: Option<(Pt, Pt)> = None;
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let m = a.add(b).scale(0.5);
            let on = exterior_walls.iter().any(|w| {
                w.level == r.level
                    && studio_geom::project_to_segment(m, w.start, w.end).1
                        <= w.thickness / 2.0 + tol
            });
            if on {
                exterior_len += a.dist(b);
                if best.is_none_or(|(p, q)| p.dist(q) < a.dist(b)) {
                    best = Some((a, b));
                }
            }
        }
        spaces.push(Space {
            id: r.id,
            level: r.level,
            name: r.name.clone(),
            kind: rules.kind_of(&r.name),
            center: centroid(ring),
            area: area(ring),
            ring: ring.clone(),
            min,
            max,
            exterior_len,
            exterior_edge: best,
        });
    }

    // Cores: stairs and shafts.
    let mut cores = vec![];
    for s in &model.stairs {
        let pts: Vec<Pt> = s.footprint().into_iter().flatten().collect();
        if let Some((a, b)) = bbox(&pts) {
            cores.push((s.base_level, a, b));
        }
    }
    for s in spaces.iter().filter(|s| s.kind == "Stair") {
        cores.push((s.level, s.min, s.max));
    }

    // Uses by area.
    let mut by_use: BTreeMap<String, f64> = BTreeMap::new();
    for s in &spaces {
        if let Some(u) = &rules.space(&s.kind).use_ {
            *by_use.entry(u.clone()).or_default() += s.area;
        }
    }
    let mut uses: Vec<(String, f64)> = by_use.into_iter().collect();
    uses.sort_by(|a, b| b.1.total_cmp(&a.1));
    let residential = spaces.iter().any(|s| s.kind == "Bedroom")
        || uses.first().is_some_and(|u| u.0 == "residential");
    let dwelling_units = if residential {
        spaces.iter().filter(|s| s.kind == "Kitchen").count().max(1)
    } else {
        0
    };
    let all: Vec<Pt> = levels
        .iter()
        .flat_map(|l| l.outline.iter().copied())
        .collect();
    let (min, max) = bbox(&all).unwrap_or((Pt::new(0.0, 0.0), Pt::new(0.0, 0.0)));
    Features {
        stories: levels.len(),
        area: levels.iter().map(|l| l.area).sum(),
        min,
        max,
        uses: uses.into_iter().map(|u| u.0).collect(),
        dwelling_units,
        residential,
        levels,
        spaces,
        exterior_walls,
        exterior_doors,
        cores,
    }
}
