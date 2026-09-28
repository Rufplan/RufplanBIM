//! Picking in elevations and sections (ADR-059): the model's drawn edges there (levels,
//! grids, wall and roof outlines, openings' heads, sills and jambs, floor and ceiling
//! lines) as snap points and as dimension references, as Revit dimensions to model edges
//! in any view.

use studio_core::dimension::Reference;
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::{line_intersection, project_to_segment, Pt};

use crate::snap::SnapKind;
use crate::Prim;

/// The model's straight edges drawn in `view` (not annotations): (element, from, to).
pub fn segments(doc: &Document, view: ElementId) -> Vec<(ElementId, Pt, Pt)> {
    let Some(dl) = crate::display_list(doc, view) else {
        return vec![];
    };
    let mut out = vec![];
    for it in &dl.items {
        let Some(el) = it.el else { continue };
        let Prim::Line { pts, closed, .. } = &it.prim else {
            continue;
        };
        // Annotations aren't model edges (a dimension measures the model).
        if doc.data(el).is_ok_and(|d| {
            matches!(
                d.category(),
                Category::Dimension
                    | Category::TextNote
                    | Category::Tag
                    | Category::SpotElevation
                    | Category::SpotSlope
                    | Category::View
                    | Category::ElevationMarker
                    | Category::GraphicScale
                    | Category::NorthArrow
            )
        }) {
            continue;
        }
        let n = pts.len();
        let segs = if *closed { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let (a, b) = (Pt::new(a[0], a[1]), Pt::new(b[0], b[1]));
            if a.dist(b) > 1.0 {
                out.push((el, a, b));
            }
        }
    }
    out
}

/// Snap points near `p` in an elevation or section: edge ends, midpoints and crossings,
/// then the nearest point on an edge. (kind, point), within `tol`, best first.
pub fn snaps(doc: &Document, view: ElementId, p: Pt, tol: f64) -> Vec<(SnapKind, Pt)> {
    let near: Vec<(ElementId, Pt, Pt)> = segments(doc, view)
        .into_iter()
        .filter(|(_, a, b)| project_to_segment(p, *a, *b).1 <= tol * 3.0)
        .collect();
    let mut cands: Vec<(SnapKind, Pt)> = vec![];
    for (_, a, b) in &near {
        cands.push((SnapKind::Endpoint, *a));
        cands.push((SnapKind::Endpoint, *b));
        cands.push((SnapKind::Midpoint, a.lerp(*b, 0.5)));
    }
    for i in 0..near.len() {
        for j in (i + 1)..near.len() {
            let ((_, a, b), (_, c, d)) = (near[i], near[j]);
            let (u, v) = (b.sub(a), d.sub(c));
            if u.norm().cross(v.norm()).abs() < 1e-3 {
                continue;
            }
            if let Some(x) = line_intersection(a, u, c, v) {
                if project_to_segment(x, a, b).1 < 0.5 && project_to_segment(x, c, d).1 < 0.5 {
                    cands.push((SnapKind::Intersection, x));
                }
            }
        }
    }
    let mut out: Vec<(SnapKind, Pt)> = cands
        .into_iter()
        .filter(|(_, q)| q.dist(p) <= tol)
        .collect();
    out.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.dist(p).total_cmp(&y.1.dist(p))));
    if out.is_empty() {
        let best = near
            .iter()
            .map(|(_, a, b)| {
                let (t, d) = project_to_segment(p, *a, *b);
                (d, a.lerp(*b, t))
            })
            .filter(|(d, _)| *d <= tol * 0.6)
            .min_by(|x, y| x.0.total_cmp(&y.0));
        if let Some((_, q)) = best {
            out.push((SnapKind::Nearest, q));
        }
    }
    out
}

fn label(doc: &Document, el: ElementId, horizontal: bool) -> String {
    let Ok(d) = doc.data(el) else {
        return "Edge".into();
    };
    match d {
        ElementData::Level { name, .. } => format!("Level: {name}"),
        ElementData::Grid { name, .. } => format!("Grid {name}"),
        _ => {
            let what = match d.category() {
                Category::Wall => "Wall",
                Category::Door => "Door",
                Category::Window => "Window",
                Category::Floor => "Floor",
                Category::Ceiling => "Ceiling",
                Category::Roof => "Roof",
                Category::Stair => "Stair",
                Category::Column => "Column",
                Category::Beam => "Beam",
                Category::Railing => "Railing",
                Category::WallOpening => "Wall Opening",
                Category::LightingFixture => "Lighting Fixture",
                Category::DetailLine | Category::ModelLine => "Line",
                _ => "Edge",
            };
            format!(
                "{what}: {} edge",
                if horizontal { "horizontal" } else { "vertical" }
            )
        }
    }
}

/// The references within `tol` of `cursor` in an elevation or section, nearest first:
/// the model edges there, one per line (an element's collinear pieces count once). They
/// measure where they are when placed; plans' wall and grid references follow the model.
pub fn references(doc: &Document, view: ElementId, cursor: Pt, tol: f64) -> Vec<Reference> {
    // (distance, datum first, element, reference)
    let mut found: Vec<(f64, u8, ElementId, Reference)> = vec![];
    for (el, a, b) in segments(doc, view) {
        let (t, d) = project_to_segment(cursor, a, b);
        if d > tol {
            continue;
        }
        let dir = b.sub(a).norm();
        // One per line: an element's pieces on the same line are one reference.
        let dup = found.iter().any(|(_, _, e, r)| {
            *e == el
                && r.dir.is_some_and(|rd| {
                    rd.cross(dir).abs() < 1e-6 && rd.cross(a.sub(r.from)).abs() < 1.0
                })
        });
        if dup {
            continue;
        }
        let horizontal = dir.y.abs() < 1e-6;
        // Levels and grids win ties with the edges drawn on them (Tab reaches those).
        let datum = u8::from(!matches!(
            doc.data(el),
            Ok(ElementData::Level { .. } | ElementData::Grid { .. })
        ));
        found.push((
            (d * 2.0).round() / 2.0,
            datum,
            el,
            Reference {
                label: label(doc, el, horizontal),
                at: a.lerp(b, t),
                from: a,
                to: b,
                dir: Some(dir),
                anchor: None,
            },
        ));
    }
    found.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
    found.into_iter().map(|(_, _, _, r)| r).collect()
}

/// A place in the model picked in a view: plan point, height, and the wall it's on with
/// its t along it (0 to 1).
pub type ModelPoint = (Pt, f64, Option<(ElementId, f64)>);

/// What's under `p` in an elevation or section, as a place in the model (ADR-059): the
/// wall whose face it's on (the face toward the viewer) and where along it, or, in a
/// section, the point on the cut plane. (plan point, height, wall and its t from 0 to 1).
pub fn model_point(doc: &Document, view: ElementId, p: Pt, tol: f64) -> Option<ModelPoint> {
    let (origin, right, look) = crate::view_frame(doc, view)?;
    let dl = crate::display_list(doc, view)?;
    let model = studio_regen::regenerate(doc);
    let at = origin.add(right.scale(p.x));
    for id in crate::pick_all(&dl, p, tol) {
        let Some(w) = model.walls.iter().find(|w| w.id == id) else {
            continue;
        };
        let d = w.dir();
        if d.cross(look).abs() < 0.2 {
            continue; // Seen edge-on.
        }
        let x = line_intersection(at, look, w.start, d)?;
        let len = w.start.dist(w.end);
        let t = x.sub(w.start).dot(d) / len;
        if !(-0.001..=1.001).contains(&t) {
            continue;
        }
        let toward = if d.perp().dot(look) < 0.0 {
            d.perp()
        } else {
            d.perp().scale(-1.0)
        };
        return Some((
            x.add(toward.scale(w.thickness / 2.0)),
            p.y,
            Some((id, t.clamp(0.0, 1.0))),
        ));
    }
    // A section's cut plane.
    match doc.data(view).ok()? {
        ElementData::View {
            kind: studio_core::ViewKind::Section { .. },
            ..
        } => Some((at, p.y, None)),
        _ => None,
    }
}

/// Height of a room's tag above its level in sections and elevations (ADR-060).
pub const ROOM_TAG_HEIGHT: f64 = 4.0 * 304.8;

/// Where `room`'s tag sits by default in a section or elevation: over the room's point,
/// 4'-0" above its level.
pub fn room_tag_base(doc: &Document, view: ElementId, room: ElementId) -> Option<Pt> {
    let (origin, right, _) = crate::view_frame(doc, view)?;
    let ElementData::Room { level, point, .. } = doc.data(room).ok()? else {
        return None;
    };
    let z = doc.level_elevation(*level).ok()?;
    Some(Pt::new(point.sub(origin).dot(right), z + ROOM_TAG_HEIGHT))
}

/// The room under `p` in a section or elevation: on the level at or below the click, the
/// first room the line of sight enters (from the cut plane, in a section or interior
/// elevation).
pub fn room_in_view(doc: &Document, view: ElementId, p: Pt) -> Option<ElementId> {
    let (origin, right, look) = crate::view_frame(doc, view)?;
    let building = matches!(
        doc.data(view).ok()?,
        ElementData::View {
            kind: studio_core::ViewKind::Elevation { .. },
            ..
        }
    );
    let levels = doc.levels();
    let level = levels
        .iter()
        .filter(|(_, _, z)| *z <= p.y + 1.0)
        .max_by(|a, b| a.2.total_cmp(&b.2))?
        .0;
    let from = origin.add(right.scale(p.x));
    let min_t = if building { f64::NEG_INFINITY } else { 0.0 };
    let model = studio_regen::regenerate(doc);
    let mut best: Option<(f64, ElementId)> = None;
    for r in model.rooms.iter().filter(|r| r.level == level) {
        let Some(ring) = &r.boundary else { continue };
        let mut entry: Option<f64> = None;
        if min_t.is_finite() && studio_geom::point_in_ring(from, ring) {
            entry = Some(0.0);
        }
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            let Some(x) = line_intersection(from, look, a, b.sub(a)) else {
                continue;
            };
            if project_to_segment(x, a, b).1 > 0.5 {
                continue;
            }
            let t = x.sub(from).dot(look);
            if t >= min_t - 1e-6 && entry.is_none_or(|e| t < e) {
                entry = Some(t);
            }
        }
        if let Some(t) = entry {
            if best.is_none_or(|b| t < b.0) {
                best = Some((t, r.id));
            }
        }
    }
    best.map(|b| b.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::{ops, Compass, ViewKind};

    /// A south wall with a door, and the South elevation.
    fn house_with_door() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(8000.0, 0.0)).unwrap();
        let dt = doc.of(Category::DoorType).next().unwrap().id;
        let d = ops::create_door(&mut doc, dt, w, 3000.0, false).unwrap();
        let v = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Elevation {
                            facing: Compass::South
                        },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        (doc, v, d)
    }

    #[test]
    fn an_elevation_offers_levels_and_door_edges_to_dimension_and_snap() {
        let (doc, south, door) = house_with_door();
        let dl = crate::display_list(&doc, south).unwrap();
        let z2 = doc.levels()[1].2;
        // Near Level 2's line: it's the first reference, horizontal, at its height.
        let refs = references(&doc, south, Pt::new(1000.0, z2 + 20.0), 100.0);
        assert!(refs[0].label.starts_with("Level: "), "{:?}", refs[0].label);
        assert!(refs[0].dir.unwrap().y.abs() < 1e-9);
        assert!((refs[0].at.y - z2).abs() < 1e-6);
        // The door's head is a reference too, and its top corner a snap.
        let ring: Vec<Pt> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(door))
            .find_map(|i| match &i.prim {
                Prim::Fill { rings, .. } => {
                    Some(rings[0].iter().map(|p| Pt::new(p[0], p[1])).collect())
                }
                _ => None,
            })
            .unwrap();
        let head = ring.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        let left = ring.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        let mid = ring.iter().map(|p| p.x).sum::<f64>() / ring.len() as f64;
        let near_head = references(&doc, south, Pt::new(mid, head + 10.0), 50.0);
        assert!(
            near_head
                .iter()
                .any(|r| r.label.starts_with("Door") && (r.at.y - head).abs() < 1.0),
            "{near_head:?}"
        );
        let s = snaps(&doc, south, Pt::new(left + 15.0, head - 15.0), 60.0);
        let (kind, at) = s.first().copied().unwrap();
        assert!(matches!(kind, SnapKind::Endpoint | SnapKind::Intersection));
        assert!(at.dist(Pt::new(left, head)) < 1.0);
    }

    #[test]
    fn a_level_to_level_dimension_places_and_draws_in_an_elevation() {
        let (mut doc, south, _) = house_with_door();
        let (z1, z2) = (doc.levels()[0].2, doc.levels()[1].2);
        let a = references(&doc, south, Pt::new(1000.0, z1 + 10.0), 100.0).remove(0);
        let b = references(&doc, south, Pt::new(1000.0, z2 + 10.0), 100.0).remove(0);
        assert!(a.label.starts_with("Level") && b.label.starts_with("Level"));
        let id = studio_core::dimension::create_string(
            &mut doc,
            south,
            &[a, b],
            Pt::new(-3000.0, (z1 + z2) / 2.0),
            studio_core::DimKind::Aligned,
        )
        .unwrap();
        let dl = crate::display_list(&doc, south).unwrap();
        let text: Vec<String> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(id))
            .filter_map(|i| match &i.prim {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(text, vec![studio_core::units::format_ft_in(z2 - z1)]);
    }

    #[test]
    fn a_click_finds_the_wall_face_in_an_elevation_and_the_cut_in_a_section() {
        let (mut doc, south, _) = house_with_door();
        let wall = doc.of(Category::Wall).next().unwrap().id;
        let half = match doc
            .data(doc.data(wall).unwrap().type_id().unwrap())
            .unwrap()
        {
            ElementData::WallType { thickness, .. } => thickness / 2.0,
            _ => unreachable!(),
        };
        // The South elevation looks north at the wall's south face (y = -half).
        let (p, z, hit) = model_point(&doc, south, Pt::new(6000.0, 1500.0), 10.0).unwrap();
        let (id, t) = hit.unwrap();
        assert_eq!(id, wall);
        assert!((t - 0.75).abs() < 1e-9);
        assert!((p.x - 6000.0).abs() < 1e-6 && (p.y + half).abs() < 1e-6);
        assert!((z - 1500.0).abs() < 1e-9);
        // Above the wall there's nothing to place on in an elevation.
        assert!(model_point(&doc, south, Pt::new(6000.0, 90000.0), 10.0).is_none());
        // A section running north from inside: in open space, its cut plane.
        let sec = ops::create_section(&mut doc, Pt::new(-5000.0, 3000.0), Pt::new(5000.0, 3000.0))
            .unwrap();
        let (p, _, hit) = model_point(&doc, sec, Pt::new(2000.0, 2400.0), 10.0).unwrap();
        assert!(hit.is_none());
        assert!((p.y - 3000.0).abs() < 1e-6);
    }
}
