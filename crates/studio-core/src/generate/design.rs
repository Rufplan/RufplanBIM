//! The building's design moves (ADR-099): cladding by story, a roof over each story's
//! exposed part, columns under porches and cantilevers, railings round decks, and exterior
//! walls turned to face out.

use studio_geom::{difference, offset_ring, signed_area, union_all, Poly, Pt};

use super::{BuildReport, BuildingSpec, Rect, RoofKind, RoomKind, StoryPlan};
use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::units::{MM_PER_FT, MM_PER_IN};

/// A copy of exterior wall type `base` faced in library material `preset` (its outer
/// layer), for one story's cladding. Reuses a copy already in the project.
pub(super) fn clad_type(
    doc: &mut Document,
    base: ElementId,
    preset: &str,
) -> CoreResult<ElementId> {
    let mat = crate::library::add_preset(doc, preset)?;
    let mat_name = doc.data(mat)?.name();
    let ElementData::WallType { name, .. } = doc.data(base)? else {
        return Err(CoreError::Invalid("not a wall type".into()));
    };
    let name = format!("{name} - {mat_name}");
    if let Some(e) = doc.of(Category::WallType).find(|e| e.data.name() == name) {
        return Ok(e.id);
    }
    let mut data = doc.data(base)?.clone();
    if let ElementData::WallType {
        name: n, layers, ..
    } = &mut data
    {
        *n = name.clone();
        if let Some(l) = layers.first_mut() {
            l.material = Some(mat);
            l.name = mat_name;
        }
    }
    doc.transact("Clad wall type", |tx| Ok(tx.insert(data)))
}

fn union_of<'a>(rects: impl Iterator<Item = &'a Rect>) -> Vec<Poly> {
    let polys: Vec<Poly> = rects.map(Rect::poly).collect();
    if polys.is_empty() {
        vec![]
    } else {
        union_all(&polys)
    }
}

/// What a story roofs over: its rooms and porches.
fn covered(p: &StoryPlan) -> Vec<Poly> {
    union_of(
        p.indoor.iter().map(|r| &r.1).chain(
            p.outdoor
                .iter()
                .filter(|(k, _)| *k == RoomKind::Porch)
                .map(|(_, r)| r),
        ),
    )
}

/// What sits on a story's roof: the next story's rooms, decks and porches.
fn above(p: Option<&StoryPlan>) -> Vec<Poly> {
    p.map(|q| {
        union_of(
            q.indoor
                .iter()
                .map(|r| &r.1)
                .chain(q.outdoor.iter().map(|(_, r)| r)),
        )
    })
    .unwrap_or_default()
}

fn ccw(mut ring: Vec<Pt>) -> Vec<Pt> {
    if signed_area(&ring) < 0.0 {
        ring.reverse();
    }
    ring
}

/// An orthogonal polygon (with holes) as rectangles: rows of cells merged along x, then
/// rows of the same span merged along y. Each grows `lap` so neighbours overlap (their
/// fascias between them hide).
fn strips(p: &Poly, lap: f64) -> Vec<Vec<Pt>> {
    let mut xs: Vec<f64> = vec![];
    let mut ys: Vec<f64> = vec![];
    for q in std::iter::once(&p.outer).chain(&p.holes).flatten() {
        xs.push(q.x);
        ys.push(q.y);
    }
    let sort = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    };
    sort(&mut xs);
    sort(&mut ys);
    // (x0, x1, y0, y1)
    let mut rects: Vec<(f64, f64, f64, f64)> = vec![];
    for yw in ys.windows(2) {
        let ym = (yw[0] + yw[1]) / 2.0;
        let mut run: Option<f64> = None;
        for (i, xw) in xs.windows(2).enumerate() {
            let inside = p.contains(Pt::new((xw[0] + xw[1]) / 2.0, ym));
            match (inside, run) {
                (true, None) => run = Some(xw[0]),
                (false, Some(x0)) => {
                    rects.push((x0, xw[0], yw[0], yw[1]));
                    run = None;
                }
                _ => {}
            }
            if inside && i + 2 == xs.len() {
                if let Some(x0) = run.take() {
                    rects.push((x0, xw[1], yw[0], yw[1]));
                }
            }
        }
    }
    // Merge a rectangle into the one below it when they span the same x.
    let mut merged: Vec<(f64, f64, f64, f64)> = vec![];
    for r in rects {
        match merged
            .iter_mut()
            .find(|m| (m.0 - r.0).abs() < 1.0 && (m.1 - r.1).abs() < 1.0 && (m.3 - r.2).abs() < 1.0)
        {
            Some(m) => m.3 = r.3,
            None => merged.push(r),
        }
    }
    merged
        .into_iter()
        .map(|(x0, x1, y0, y1)| {
            vec![
                Pt::new(x0 - lap, y0 - lap),
                Pt::new(x1 + lap, y0 - lap),
                Pt::new(x1 + lap, y1 + lap),
                Pt::new(x0 - lap, y1 + lap),
            ]
        })
        .collect()
}

/// Flat roof outlines over `exposed`, overhanging `oh` outward and stopping at the
/// volume above.
fn flat_rings(exposed: &Poly, above: &[Poly], oh: f64) -> Vec<Vec<Pt>> {
    let grown = Poly {
        outer: offset_ring(&ccw(exposed.outer.clone()), oh),
        // Courtyards shrink by the overhang too.
        holes: exposed
            .holes
            .iter()
            .map(|h| offset_ring(&ccw(h.clone()), -oh))
            .filter(|h| signed_area(h).abs() > 1.0e5)
            .collect(),
    };
    let parts = if above.is_empty() {
        vec![grown]
    } else {
        difference(&grown, above)
    };
    let mut out = vec![];
    for part in parts {
        if part.area() < 1.0e5 {
            continue;
        }
        if part.holes.is_empty() {
            out.push(ccw(part.outer));
        } else {
            out.extend(strips(&part, 10.0));
        }
    }
    out
}

/// Tilts flat roof `id` into a shed falling toward `low` (plan unit vector): its edge
/// facing most that way is the eave.
fn shed(doc: &mut Document, id: ElementId, low: Pt, slope: f64) -> CoreResult<()> {
    doc.transact("Shed roof", |tx| {
        tx.modify(id, |d| {
            if let ElementData::Roof {
                boundary,
                slope: s,
                sloped,
                ..
            } = d
            {
                let n = boundary.len();
                // Stored counter-clockwise: outward is right of each edge.
                let score = |i: usize| {
                    let (a, b) = (boundary[i], boundary[(i + 1) % n]);
                    let dir = b.sub(a);
                    let out = Pt::new(dir.y, -dir.x).norm();
                    (out.dot(low), dir.len())
                };
                let best = (0..n)
                    .max_by(|i, j| {
                        let (a, b) = (score(*i), score(*j));
                        a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
                    })
                    .unwrap_or(0);
                *sloped = (0..n).map(|i| i == best).collect();
                *s = slope;
            }
        })
    })
}

fn centroid(polys: &[Poly]) -> Option<Pt> {
    let pts: Vec<Pt> = polys.iter().flat_map(|p| p.outer.clone()).collect();
    (!pts.is_empty()).then(|| {
        let n = pts.len() as f64;
        Pt::new(
            pts.iter().map(|p| p.x).sum::<f64>() / n,
            pts.iter().map(|p| p.y).sum::<f64>() / n,
        )
    })
}

/// A roof over each story's exposed part (ADR-099): what nothing above covers, overhung
/// and stopped at the volume above. Its kind is the story's, else the building's; a
/// pitched roof against an upper volume leans to (a shed falling away from it).
pub(super) fn roofs(
    doc: &mut Document,
    spec: &BuildingSpec,
    plans: &[StoryPlan],
    rt: ElementId,
    report: &mut BuildReport,
) -> CoreResult<Vec<ElementId>> {
    let oh = spec.overhang.unwrap_or(1.5).clamp(0.0, 12.0) * MM_PER_FT;
    let mut all = vec![];
    for (s, p) in plans.iter().enumerate() {
        let story = &spec.stories[s];
        let up = above(plans.get(s + 1));
        let exposed: Vec<Poly> = covered(p)
            .iter()
            .flat_map(|c| {
                if up.is_empty() {
                    vec![c.clone()]
                } else {
                    difference(c, &up)
                }
            })
            .filter(|e| e.area() > 2.0 * MM_PER_FT * MM_PER_FT * 10.0)
            .collect();
        if exposed.is_empty() {
            continue;
        }
        let kind = story.roof.map_or(spec.roof, |r| r.kind);
        let pitch = story
            .roof
            .and_then(|r| r.pitch)
            .unwrap_or(spec.pitch)
            .clamp(0.5, 18.0);
        let slope = (pitch / 12.0).atan();
        let low_side = story.roof.and_then(|r| r.low_side).map(|s| s.dir());
        let level = p.top;
        let mut made: Vec<ElementId> = vec![];
        for e in &exposed {
            let simple = e.holes.is_empty() && up.is_empty();
            // A lean-to falls away from the volume it abuts.
            let away = || {
                let c = centroid(&up)?;
                let m = centroid(std::slice::from_ref(e))?;
                let d = m.sub(c);
                (d.len() > 1.0).then(|| {
                    // The nearest compass direction, so the eave is a whole side.
                    if d.x.abs() >= d.y.abs() {
                        Pt::new(d.x.signum(), 0.0)
                    } else {
                        Pt::new(0.0, d.y.signum())
                    }
                })
            };
            let flats = |doc: &mut Document| -> CoreResult<Vec<ElementId>> {
                flat_rings(e, &up, oh)
                    .into_iter()
                    .map(|ring| crate::build::create_roof(doc, rt, level, 0.0, ring, 0.0))
                    .collect()
            };
            let ids = match kind {
                RoofKind::Gable if simple && e.outer.len() == 4 => {
                    super::gable(doc, rt, level, &e.outer, slope)?
                }
                RoofKind::Hip | RoofKind::Gable if simple => {
                    match crate::build::create_roofs_by_footprint(
                        doc, rt, level, 0.0, &e.outer, oh, slope,
                    ) {
                        Ok(ids) => ids,
                        Err(_) => flats(doc)?,
                    }
                }
                RoofKind::Hip | RoofKind::Gable => {
                    let ids = flats(doc)?;
                    if let Some(dir) = away() {
                        for id in &ids {
                            shed(doc, *id, dir, slope)?;
                        }
                    }
                    ids
                }
                RoofKind::Flat => flats(doc)?,
                RoofKind::Shed => {
                    let ids = flats(doc)?;
                    let dir = low_side.or_else(away).unwrap_or(Pt::new(0.0, 1.0));
                    for id in &ids {
                        shed(doc, *id, dir, slope)?;
                    }
                    ids
                }
                RoofKind::Butterfly => {
                    let rings = flat_rings(e, &up, oh);
                    let mut ids = vec![];
                    for ring in rings {
                        let (lo, hi) = studio_geom::bounds_of(&ring).unwrap_or_default();
                        if ring.len() == 4 {
                            // Two halves falling in to a valley down the long middle.
                            let long_x = hi.x - lo.x >= hi.y - lo.y;
                            let halves = if long_x {
                                let m = (lo.y + hi.y) / 2.0;
                                [
                                    (
                                        Rect {
                                            x0: lo.x,
                                            y0: lo.y,
                                            x1: hi.x,
                                            y1: m,
                                        },
                                        Pt::new(0.0, 1.0),
                                    ),
                                    (
                                        Rect {
                                            x0: lo.x,
                                            y0: m,
                                            x1: hi.x,
                                            y1: hi.y,
                                        },
                                        Pt::new(0.0, -1.0),
                                    ),
                                ]
                            } else {
                                let m = (lo.x + hi.x) / 2.0;
                                [
                                    (
                                        Rect {
                                            x0: lo.x,
                                            y0: lo.y,
                                            x1: m,
                                            y1: hi.y,
                                        },
                                        Pt::new(1.0, 0.0),
                                    ),
                                    (
                                        Rect {
                                            x0: m,
                                            y0: lo.y,
                                            x1: hi.x,
                                            y1: hi.y,
                                        },
                                        Pt::new(-1.0, 0.0),
                                    ),
                                ]
                            };
                            for (r, dir) in halves {
                                let id =
                                    crate::build::create_roof(doc, rt, level, 0.0, r.ring(), 0.0)?;
                                shed(doc, id, dir, slope)?;
                                ids.push(id);
                            }
                        } else {
                            let id = crate::build::create_roof(doc, rt, level, 0.0, ring, 0.0)?;
                            shed(doc, id, low_side.unwrap_or(Pt::new(0.0, 1.0)), slope)?;
                            ids.push(id);
                        }
                    }
                    ids
                }
            };
            made.extend(ids);
        }
        // Walls under sloped roofs rise to meet them (gable ends, a shed's high side).
        let sloping = matches!(kind, RoofKind::Gable | RoofKind::Shed | RoofKind::Butterfly)
            || (matches!(kind, RoofKind::Hip) && !up.is_empty());
        if sloping {
            let under: Vec<ElementId> = p
                .ext_walls
                .iter()
                .copied()
                .filter(|w| match doc.data(*w) {
                    Ok(ElementData::Wall { start, end, .. }) => {
                        let m = start.lerp(*end, 0.5);
                        exposed.iter().any(|e| {
                            Poly::simple(offset_ring(&ccw(e.outer.clone()), 300.0)).contains(m)
                        }) && !up.iter().any(|u| u.contains(m))
                    }
                    _ => false,
                })
                .collect();
            crate::structure::attach_wall_tops(doc, &under, true)?;
        }
        report.roofs += made.len();
        all.extend(made);
    }
    Ok(all)
}

/// Columns at a porch's free corners (and along long free sides) and, when asked, under
/// the outer corners of a ground floor's cantilevers; railings round decks (ADR-099).
pub(super) fn supports(
    doc: &mut Document,
    spec: &BuildingSpec,
    plans: &[StoryPlan],
    report: &mut BuildReport,
) -> CoreResult<()> {
    let column = doc
        .of(Category::ColumnType)
        .find(|e| e.data.name().to_lowercase().contains("steel"))
        .map(|e| e.id)
        .or_else(|| crate::ops::first_of(doc, Category::ColumnType));
    let railing = crate::ops::first_of(doc, Category::RailingType);
    let inset = 6.0 * MM_PER_IN;
    for (s, p) in plans.iter().enumerate() {
        let touches = |q: Pt| p.indoor.iter().any(|(_, r, _)| r.holds(q, 25.0));
        // Porches: posts where nothing else holds the roof up.
        if let Some(ct) = column {
            for (_, r) in p.outdoor.iter().filter(|(k, _)| *k == RoomKind::Porch) {
                let c = r.center();
                let ring = r.ring();
                let mut posts: Vec<Pt> = vec![];
                for i in 0..4 {
                    let (a, b) = (ring[i], ring[(i + 1) % 4]);
                    if !touches(a) {
                        posts.push(a);
                    }
                    // Long free sides get posts between, about 12' apart.
                    let len = a.dist(b);
                    if !touches(a) && !touches(b) && len > 16.0 * MM_PER_FT {
                        let n = (len / (12.0 * MM_PER_FT)).round() as usize;
                        for k in 1..n {
                            posts.push(a.lerp(b, k as f64 / n as f64));
                        }
                    }
                }
                for q in posts {
                    let at = q.add(c.sub(q).norm().scale(inset));
                    crate::structure::create_column(doc, ct, p.level, at, 0.0)?;
                    report.columns += 1;
                }
            }
            // Cantilevers over the ground: posts at their outer corners.
            if s == 0 && spec.cantilever_columns {
                if let Some(next) = plans.get(1) {
                    let below = covered(p);
                    let upper = union_of(next.indoor.iter().map(|r| &r.1));
                    for u in &upper {
                        for void in difference(u, &below) {
                            if void.area() < 20.0 * MM_PER_FT * MM_PER_FT {
                                continue;
                            }
                            let c = centroid(std::slice::from_ref(&void)).unwrap_or_default();
                            for q in &void.outer {
                                let held = below.iter().any(|b| {
                                    b.contains(*q)
                                        || b.outer.iter().enumerate().any(|(i, a)| {
                                            let bb = b.outer[(i + 1) % b.outer.len()];
                                            studio_geom::project_to_segment(*q, *a, bb).1
                                                < MM_PER_FT
                                        })
                                });
                                if held {
                                    continue;
                                }
                                let at = q.add(c.sub(*q).norm().scale(MM_PER_FT));
                                crate::structure::create_column(doc, ct, p.level, at, 0.0)?;
                                report.columns += 1;
                            }
                        }
                    }
                }
            }
        }
        // Decks and upper porches: a guard along every side open to the air.
        let Some(rt) = railing else { continue };
        for (k, r) in &p.outdoor {
            let up_in_air = s > 0 || *k == RoomKind::Deck;
            if !up_in_air || *k == RoomKind::Courtyard {
                continue;
            }
            let ring = r.ring();
            for i in 0..4 {
                let (a, b) = (ring[i], ring[(i + 1) % 4]);
                for (u0, u1) in open_spans(a, b, &p.indoor.iter().map(|x| x.1).collect::<Vec<_>>())
                {
                    if u1 - u0 < 2.0 * MM_PER_FT {
                        continue;
                    }
                    let d = b.sub(a).norm();
                    let inward = r.center().sub(a.lerp(b, 0.5)).norm().scale(4.0 * MM_PER_IN);
                    let path = vec![
                        a.add(d.scale(u0)).add(inward),
                        a.add(d.scale(u1)).add(inward),
                    ];
                    crate::structure::create_railing(doc, rt, p.level, path)?;
                    report.railings += 1;
                }
            }
        }
    }
    Ok(())
}

/// The spans (distance from `a`) of side a→b not against any of `walls`' rectangles.
fn open_spans(a: Pt, b: Pt, walls: &[Rect]) -> Vec<(f64, f64)> {
    let len = a.dist(b);
    let d = b.sub(a).norm();
    let mut blocked: Vec<(f64, f64)> = walls
        .iter()
        .filter_map(|r| {
            // The part of the side lying on the rectangle's edge.
            let on = |q: Pt| r.holds(q, 25.0);
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            let n = (len / 100.0).ceil().max(1.0) as usize;
            for k in 0..=n {
                let t = len * k as f64 / n as f64;
                if on(a.add(d.scale(t))) {
                    lo = lo.min(t);
                    hi = hi.max(t);
                }
            }
            (hi > lo).then_some((lo, hi))
        })
        .collect();
    blocked.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out = vec![];
    let mut at = 0.0;
    for (lo, hi) in blocked {
        if lo > at {
            out.push((at, lo));
        }
        at = at.max(hi);
    }
    if at < len {
        out.push((at, len));
    }
    out
}

/// Turns exterior walls so their exterior face (the cladding) is outside the building.
pub(super) fn face_out(
    doc: &mut Document,
    plans: &[StoryPlan],
    walls: &[ElementId],
) -> CoreResult<()> {
    let inside: Vec<Vec<Rect>> = plans
        .iter()
        .map(|p| p.indoor.iter().map(|r| r.1).collect())
        .collect();
    let mut flip = vec![];
    for (s, p) in plans.iter().enumerate() {
        for w in walls.iter().filter(|w| p.ext_walls.contains(w)) {
            let Ok(ElementData::Wall { start, end, .. }) = doc.data(*w) else {
                continue;
            };
            let m = start.lerp(*end, 0.5);
            // The exterior face is the location line's left.
            let left = end.sub(*start).norm().perp();
            let probe = m.add(left.scale(MM_PER_FT));
            if inside[s].iter().any(|r| r.holds(probe, 0.0)) {
                flip.push(*w);
            }
        }
    }
    if !flip.is_empty() {
        crate::edit::flip_walls(doc, &flip)?;
    }
    Ok(())
}
