//! The site and the furniture of a generated building (ADR-099): a lawn, trees round the
//! lot, foundation beds along the facades, walks and a drive to the street doors, and
//! furniture in each room and on the terraces.

use std::f64::consts::{FRAC_PI_2, TAU};

use studio_geom::{signed_area, union_all, Pt};

use super::{BuildReport, BuildingSpec, Rect, RoomKind, StoryPlan, StreetDoor};
use crate::document::{CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::units::MM_PER_FT;

pub(super) const DEFAULT_TREES: &[&str] = &[
    "Red Maple",
    "Honey Locust",
    "River Birch",
    "White Oak",
    "Eastern White Pine",
    "Serviceberry",
];
pub(super) const DEFAULT_SHRUBS: &[&str] = &[
    "Boxwood, Round",
    "Bigleaf Hydrangea",
    "Fountain Grass",
    "English Lavender",
];

fn ft(v: f64) -> f64 {
    v * MM_PER_FT
}

/// Plant types by name, the catalog's only (unknown names are skipped); `fallback` when
/// none of them is.
fn plant_types(
    doc: &mut Document,
    names: &[String],
    fallback: &[&str],
) -> CoreResult<Vec<ElementId>> {
    let known: Vec<String> = crate::planting::catalog()
        .into_iter()
        .map(|p| p.name)
        .collect();
    let mut picked: Vec<String> = names
        .iter()
        .filter(|n| known.contains(n))
        .cloned()
        .collect();
    if picked.is_empty() {
        picked = fallback.iter().map(|s| s.to_string()).collect();
    }
    crate::planting::load(doc, &picked)
}

fn region(
    doc: &mut Document,
    level: ElementId,
    material: ElementId,
    ring: Vec<Pt>,
) -> CoreResult<()> {
    doc.transact("Create ground region", |tx| {
        Ok(tx.insert(ElementData::GroundRegion {
            level,
            material,
            boundary: ring,
            sketch: vec![],
        }))
    })?;
    Ok(())
}

/// A rectangle `w` wide from `a` out `len` along unit `dir`.
fn strip(a: Pt, dir: Pt, len: f64, w: f64) -> Vec<Pt> {
    let side = dir.perp().scale(w / 2.0);
    let b = a.add(dir.scale(len));
    vec![a.sub(side), b.sub(side), b.add(side), a.add(side)]
}

/// Lawn, trees, beds, walks and a drive round the building (ADR-099).
pub(super) fn landscape(
    doc: &mut Document,
    spec: &BuildingSpec,
    plans: &[StoryPlan],
    doors: &[StreetDoor],
    report: &mut BuildReport,
) -> CoreResult<()> {
    let Some(ground) = plans.first() else {
        return Ok(());
    };
    let level = ground.level;
    // The lawn underfoot.
    let lawn = crate::planting::ground_material(doc, "site-lawn-lush")?;
    crate::planting::set_ground(doc, Some(lawn))?;
    let mulch = crate::library::add_preset(doc, "site-bark-mulch")?;
    let paving = match spec
        .materials
        .paving
        .as_deref()
        .filter(|p| crate::library::preset(p).is_some())
    {
        Some(p) => crate::library::add_preset(doc, p)?,
        None => crate::library::add_preset(doc, "site-bluestone-pattern")?,
    };
    let concrete = crate::library::add_preset(doc, "site-concrete-broom")?;

    // Everything built, for the extent.
    let all: Vec<Rect> = plans
        .iter()
        .flat_map(|p| {
            p.indoor
                .iter()
                .map(|r| r.1)
                .chain(p.outdoor.iter().map(|o| o.1))
        })
        .collect();
    let (mut lo, mut hi) = (Pt::new(f64::MAX, f64::MAX), Pt::new(f64::MIN, f64::MIN));
    for r in &all {
        lo = Pt::new(lo.x.min(r.x0), lo.y.min(r.y0));
        hi = Pt::new(hi.x.max(r.x1), hi.y.max(r.y1));
    }
    if lo.x > hi.x {
        return Ok(());
    }
    let c = lo.lerp(hi, 0.5);

    // Walks to the doors, a drive to the garage.
    let mut kept_clear: Vec<(Pt, f64)> = vec![];
    for d in doors {
        if d.garage {
            region(
                doc,
                level,
                concrete,
                strip(d.at, d.out, ft(30.0), d.width + ft(4.0)),
            )?;
        } else {
            region(doc, level, paving, strip(d.at, d.out, ft(18.0), ft(4.0)))?;
        }
        kept_clear.push((d.at, d.width / 2.0 + ft(3.0)));
    }

    // Foundation beds along the facades that face the street and the sides, clear of
    // doors, the garage and the terraces.
    let shrubs = plant_types(doc, &spec.planting.shrubs, DEFAULT_SHRUBS)?;
    let footprint = union_all(
        &ground
            .indoor
            .iter()
            .map(|r| r.1.poly())
            .chain(ground.outdoor.iter().map(|o| o.1.poly()))
            .collect::<Vec<_>>(),
    );
    let blocked = |q: Pt| {
        kept_clear.iter().any(|(p, r)| p.dist(q) < *r)
            || ground
                .indoor
                .iter()
                .any(|(k, r, _)| *k == RoomKind::Garage && r.holds(q, ft(1.5)))
            || ground.outdoor.iter().any(|(_, r)| r.holds(q, ft(1.5)))
    };
    let mut shrub_spots: Vec<Vec<(Pt, f64, f64)>> = vec![vec![]; shrubs.len()];
    let mut k = 0usize;
    for poly in &footprint {
        let mut ring = poly.outer.clone();
        if signed_area(&ring) < 0.0 {
            ring.reverse();
        }
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let len = a.dist(b);
            if len < ft(6.0) {
                continue;
            }
            let dir = b.sub(a).norm();
            let out = Pt::new(dir.y, -dir.x);
            // Not along the back.
            if out.y > 0.7 {
                continue;
            }
            // Bed pieces between the blocked stretches, sampled each foot.
            let steps = (len / ft(1.0)).floor() as usize;
            let mut start: Option<f64> = None;
            for s in 0..=steps {
                let t = (s as f64 * ft(1.0)).min(len);
                let q = a.add(dir.scale(t)).add(out.scale(ft(2.5)));
                let free = s < steps && !blocked(q);
                match (free, start) {
                    (true, None) => start = Some(t),
                    (false, Some(t0)) => {
                        start = None;
                        if t - t0 < ft(5.0) {
                            continue;
                        }
                        let p0 = a.add(dir.scale(t0));
                        let p1 = a.add(dir.scale(t));
                        let near = out.scale(ft(1.0));
                        let far = out.scale(ft(5.0));
                        region(
                            doc,
                            level,
                            mulch,
                            vec![p0.add(near), p1.add(near), p1.add(far), p0.add(far)],
                        )?;
                        // Shrubs down the middle about 3'-6" apart.
                        let m = ((t - t0) / ft(3.5)).floor().max(1.0) as usize;
                        for j in 0..m {
                            let at = p0
                                .add(dir.scale((j as f64 + 0.5) * (t - t0) / m as f64))
                                .add(out.scale(ft(3.0)));
                            let pick = k % shrubs.len();
                            shrub_spots[pick].push((at, j as f64 * 1.3, 1.0));
                            k += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    for (t, spots) in shrubs.iter().zip(&shrub_spots) {
        if !spots.is_empty() {
            report.plants += crate::planting::create_plants(doc, *t, level, spots)?.len();
        }
    }

    // Trees round the lot, leaving the view from the street open.
    let trees = plant_types(doc, &spec.planting.trees, DEFAULT_TREES)?;
    let (rx, ry) = (
        (hi.x - lo.x) / 2.0 + ft(55.0),
        (hi.y - lo.y) / 2.0 + ft(50.0),
    );
    let count = 16;
    let mut by_type: Vec<Vec<(Pt, f64, f64)>> = vec![vec![]; trees.len()];
    for i in 0..count {
        let a = TAU * (i as f64 + 0.35) / count as f64;
        // Framing the house, not hiding it: the front quarter (the street) and the
        // middle of the back (the garden view) stay open.
        let deg = a.to_degrees();
        if (200.0..340.0).contains(&deg) || (65.0..115.0).contains(&deg) {
            continue;
        }
        let wobble = 1.0 + 0.12 * ((i as f64) * 2.399).sin();
        let at = Pt::new(c.x + rx * wobble * a.cos(), c.y + ry * wobble * a.sin());
        let pick = i % trees.len();
        by_type[pick].push((at, i as f64 * 1.7, 0.9 + 0.1 * ((i as f64) * 1.3).cos()));
    }
    for (t, spots) in trees.iter().zip(&by_type) {
        if !spots.is_empty() {
            report.plants += crate::planting::create_plants(doc, *t, level, spots)?.len();
        }
    }
    Ok(())
}

/// A piece of furniture in a room's own frame: library name, feet across and along the
/// room (local x and y), and the way its front faces in degrees (270 = local -y).
struct Piece(&'static str, f64, f64, f64);

/// What furnishes a room of `kind` `w` x `d` feet (its long side local x), if it's big
/// enough: pieces about its centre.
fn layout(kind: RoomKind, name: &str, w: f64, d: f64) -> Vec<Piece> {
    let n = name.to_lowercase();
    let primary = ["primary", "master", "owner"].iter().any(|k| n.contains(k));
    match kind {
        RoomKind::Living if w >= 12.0 && d >= 11.0 => vec![
            Piece("Area Rug 8x10", 0.0, 0.0, 270.0),
            Piece("Sofa 84\"", 0.0, -3.2, 90.0),
            Piece("Coffee Table", 0.0, -0.2, 270.0),
            Piece("Club Chair", -2.9, 2.9, 270.0),
            Piece("Club Chair", 2.9, 2.9, 270.0),
        ],
        RoomKind::Dining if w >= 10.0 && d >= 9.0 => {
            let mut v = vec![Piece("Dining Table 6-Seat", 0.0, 0.0, 270.0)];
            for x in [-2.3, 0.0, 2.3] {
                v.push(Piece("Dining Chair", x, 2.4, 270.0));
                v.push(Piece("Dining Chair", x, -2.4, 90.0));
            }
            v
        }
        RoomKind::Bedroom if w >= 10.0 && d >= 10.0 => {
            let king = primary && w >= 13.0 && d >= 13.0;
            let bed = if king { "King Bed" } else { "Queen Bed" };
            let half = if king { 3.6 } else { 2.9 };
            // Headboard to the local north wall.
            let y = d / 2.0 - 0.5 - 3.5;
            vec![
                Piece(bed, 0.0, y, 270.0),
                Piece("Nightstand", -(half + 1.2), d / 2.0 - 1.4, 270.0),
                Piece("Nightstand", half + 1.2, d / 2.0 - 1.4, 270.0),
            ]
        }
        RoomKind::Office if w >= 8.0 && d >= 8.0 => vec![
            Piece("Desk", 0.0, d / 2.0 - 2.0, 270.0),
            Piece("Office Chair", 0.0, d / 2.0 - 4.2, 90.0),
        ],
        RoomKind::GuestRoom if w >= 11.0 && d >= 12.0 => vec![
            Piece("Hotel King Bed", 0.0, d / 2.0 - 4.0, 270.0),
            Piece("Hotel Nightstand", -4.8, d / 2.0 - 1.4, 270.0),
            Piece("Hotel Nightstand", 4.8, d / 2.0 - 1.4, 270.0),
        ],
        RoomKind::Lobby if w >= 16.0 && d >= 14.0 => vec![
            Piece("Lobby Sofa", 0.0, -3.0, 90.0),
            Piece("Lobby Coffee Table", 0.0, 0.0, 270.0),
            Piece("Lobby Lounge Chair", -3.2, 3.0, 270.0),
            Piece("Lobby Lounge Chair", 3.2, 3.0, 270.0),
        ],
        RoomKind::Terrace | RoomKind::Deck if w >= 16.0 && d >= 10.0 => {
            // Dining at one end, lounging at the other.
            let mut v = vec![Piece("Outdoor Dining Table", -w / 4.0, 0.0, 270.0)];
            for x in [-2.0, 0.0, 2.0] {
                v.push(Piece("Outdoor Dining Chair", -w / 4.0 + x, 2.3, 270.0));
                v.push(Piece("Outdoor Dining Chair", -w / 4.0 + x, -2.3, 90.0));
            }
            v.push(Piece("Outdoor Sofa", w / 4.0, -2.6, 90.0));
            v.push(Piece("Fire Table", w / 4.0, 0.4, 270.0));
            v.push(Piece("Outdoor Lounge Chair", w / 4.0 - 2.4, 3.2, 270.0));
            v.push(Piece("Outdoor Lounge Chair", w / 4.0 + 2.4, 3.2, 270.0));
            v
        }
        RoomKind::Terrace | RoomKind::Deck if w >= 9.0 && d >= 8.0 => vec![
            Piece("Outdoor Lounge Chair", -1.6, 0.0, 90.0),
            Piece("Outdoor Lounge Chair", 1.6, 0.0, 90.0),
        ],
        RoomKind::Porch if w >= 8.0 && d >= 6.0 => vec![
            Piece("Adirondack Chair", -1.8, 0.0, 270.0),
            Piece("Adirondack Chair", 1.8, 0.0, 270.0),
        ],
        _ => vec![],
    }
}

/// The library names a room's layout uses (for checking the libraries have them).
#[cfg(test)]
pub(super) fn layout_names(kind: RoomKind, name: &str, w: f64, d: f64) -> Vec<&'static str> {
    layout(kind, name, w, d).into_iter().map(|p| p.0).collect()
}

/// Furniture in the rooms and on the terraces (ADR-099), kept clear of the doors.
pub(super) fn furnish(
    doc: &mut Document,
    plans: &[StoryPlan],
    report: &mut BuildReport,
) -> CoreResult<()> {
    for p in plans {
        // Door centres on this story, to keep a swing clear.
        let doors: Vec<Pt> = doc
            .of(Category::Door)
            .filter_map(|e| {
                let ElementData::Door { host, offset, .. } = &e.data else {
                    return None;
                };
                match doc.data(*host).ok()? {
                    ElementData::Wall {
                        start,
                        end,
                        base_level,
                        ..
                    } if *base_level == p.level => {
                        Some(start.add(end.sub(*start).norm().scale(*offset)))
                    }
                    _ => None,
                }
            })
            .collect();
        let rooms: Vec<(RoomKind, Rect, String)> = p
            .indoor
            .iter()
            .cloned()
            .chain(p.outdoor.iter().map(|(k, r)| (*k, *r, String::new())))
            .collect();
        for (kind, r, name) in rooms {
            // The room's long side is its local x.
            let turned = r.depth() > r.width();
            let (w, d) = if turned {
                (r.depth(), r.width())
            } else {
                (r.width(), r.depth())
            };
            let (w, d) = (w / MM_PER_FT, d / MM_PER_FT);
            let c = r.center();
            for Piece(what, x, y, deg) in layout(kind, &name, w, d) {
                let (x, y) = (x * MM_PER_FT, y * MM_PER_FT);
                let (at, rot) = if turned {
                    // Local x runs north: rotate the layout a quarter turn.
                    (c.add(Pt::new(-y, x)), (deg + 90.0).to_radians() + FRAC_PI_2)
                } else {
                    (c.add(Pt::new(x, y)), deg.to_radians() + FRAC_PI_2)
                };
                if doors.iter().any(|q| q.dist(at) < 4.0 * MM_PER_FT) {
                    continue;
                }
                let Ok(t) = crate::ffe::load(doc, &[what.to_string()]) else {
                    continue;
                };
                let rot = rot.rem_euclid(TAU);
                if crate::ffe::create(doc, t[0], p.level, at, rot).is_ok() {
                    report.furniture += 1;
                }
            }
        }
    }
    Ok(())
}
