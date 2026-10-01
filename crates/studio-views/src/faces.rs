//! The faces Paint paints (ADR-096): which face of a wall, floor, ceiling or roof a point on
//! its surface is on, named as studio-core `paint` stores them. The 3D view's meshes split
//! along them so each painted face shows its own material.
//!
//! - Walls: `exterior` and `interior` (the location line's left and right faces), `start`
//!   and `end`, `top` and `bottom`. Faces inside openings (jambs, heads, sills) aren't
//!   named, so they stay with the wall.
//! - Floors, ceilings and roofs: `top`, `bottom` (the soffit) and `edge:N`, the side along
//!   the Nth boundary segment (the outer loop first, then any holes).

use studio_geom::Pt;
use studio_regen::{Model, RoofSolid, SlabSolid, WallSolid};

use crate::ElementId;

/// An element's shape, as far as naming its faces goes.
pub enum Shape<'a> {
    Wall(&'a WallSolid),
    Slab(&'a SlabSolid),
    Roof(&'a RoofSolid),
}

impl<'a> Shape<'a> {
    /// The shape of `el` in the regenerated model; None for elements painted only whole
    /// (columns, beams).
    pub fn of(m: &'a Model, el: ElementId) -> Option<Self> {
        if let Some(w) = m.walls.iter().find(|w| w.id == el) {
            return Some(Shape::Wall(w));
        }
        if let Some(s) = m.floors.iter().chain(&m.ceilings).find(|s| s.id == el) {
            return Some(Shape::Slab(s));
        }
        m.roofs.iter().find(|r| r.id == el).map(Shape::Roof)
    }

    /// The face at point `p` on the surface, where it faces `n` (either sign).
    pub fn face(&self, p: [f64; 3], n: [f64; 3]) -> Option<String> {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len < 1e-12 {
            return None;
        }
        let n = [n[0] / len, n[1] / len, n[2] / len];
        let plan = Pt::new(p[0], p[1]);
        match self {
            Shape::Wall(w) => wall_face(w, plan, p[2], n),
            Shape::Slab(s) => {
                let top = s.top_at(plan);
                let bottom = top - (s.z1 - s.z0);
                let rings: Vec<&[Pt]> = std::iter::once(s.base.outer.as_slice())
                    .chain(s.base.holes.iter().map(Vec::as_slice))
                    .collect();
                slab_face(&rings, plan, p[2], n, top, bottom)
            }
            Shape::Roof(r) => slab_face(
                &[r.boundary.as_slice()],
                plan,
                p[2],
                n,
                r.top(plan),
                r.underside(plan),
            ),
        }
    }

    /// The face of a triangle (9 floats, z-up), from its centroid and normal.
    pub fn triangle_face(&self, t: &[f32]) -> Option<String> {
        let v = |i: usize| {
            [
                f64::from(t[i * 3]),
                f64::from(t[i * 3 + 1]),
                f64::from(t[i * 3 + 2]),
            ]
        };
        let (a, b, c) = (v(0), v(1), v(2));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let mid = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        self.face(mid, n)
    }
}

fn wall_face(w: &WallSolid, p: Pt, z: f64, n: [f64; 3]) -> Option<String> {
    let d = w.dir();
    let left = d.perp();
    let along = |q: Pt| q.sub(w.start).dot(d);
    let off = |q: Pt| q.sub(w.start).dot(left);
    let range = |f: &dyn Fn(Pt) -> f64| {
        w.footprint
            .outer
            .iter()
            .map(|q| f(*q))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    };
    let (a0, a1) = range(&along);
    let (o0, o1) = range(&off);
    // Within this of a wall's end or its top and bottom counts as that face.
    let near = w.thickness.max(25.0);
    if n[2].abs() > 0.7 {
        return if z <= w.z0 + 1.0 {
            Some("bottom".into())
        } else if z >= w.top_at(p) - 1.0 {
            Some("top".into())
        } else {
            None // An opening's head or sill.
        };
    }
    let nd = n[0] * d.x + n[1] * d.y;
    if nd.abs() > 0.7 {
        let t = along(p);
        return if t <= a0 + near {
            Some("start".into())
        } else if t >= a1 - near {
            Some("end".into())
        } else {
            None // An opening's jamb.
        };
    }
    Some(
        if off(p) > (o0 + o1) / 2.0 {
            "exterior"
        } else {
            "interior"
        }
        .into(),
    )
}

fn slab_face(rings: &[&[Pt]], p: Pt, z: f64, n: [f64; 3], top: f64, bottom: f64) -> Option<String> {
    if n[2].abs() > 0.5 {
        return Some(
            if (z - top).abs() <= (z - bottom).abs() {
                "top"
            } else {
                "bottom"
            }
            .into(),
        );
    }
    // A side: the boundary segment it stands on.
    let mut best: Option<(f64, usize)> = None;
    let mut k = 0;
    for r in rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let (_, d) = studio_geom::project_to_segment(p, a, b);
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, k));
            }
            k += 1;
        }
    }
    best.map(|(_, i)| format!("edge:{i}"))
}

/// The face of `el` seen in a 2D view at view point `p` (ADR-096): in a plan, a floor's or
/// roof's top, a ceiling's soffit (in a ceiling plan), the side of a wall clicked; in an
/// elevation or section, the face toward the viewer: a wall's exterior or interior, a
/// floor's or roof's edge there. None paints the element whole.
pub fn face_in_view(
    doc: &studio_core::Document,
    view: ElementId,
    el: ElementId,
    p: Pt,
) -> Option<String> {
    use studio_core::{ElementData, ViewKind};
    let m = studio_regen::regenerate(doc);
    let shape = Shape::of(&m, el)?;
    let ElementData::View { kind, .. } = doc.data(view).ok()? else {
        return None;
    };
    match kind {
        ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. } => match shape {
            Shape::Wall(w) => {
                let off = |q: Pt| q.sub(w.start).dot(w.dir().perp());
                let mid = w.footprint.outer.iter().map(|q| off(*q)).sum::<f64>()
                    / w.footprint.outer.len().max(1) as f64;
                Some(if off(p) > mid { "exterior" } else { "interior" }.into())
            }
            _ if matches!(kind, ViewKind::CeilingPlan { .. }) => Some("bottom".into()),
            _ => Some("top".into()),
        },
        _ => {
            let (origin, right, look) = crate::view_frame(doc, view)?;
            match shape {
                Shape::Wall(w) => Some(
                    if w.dir().perp().dot(look) < 0.0 {
                        "exterior"
                    } else {
                        "interior"
                    }
                    .into(),
                ),
                Shape::Slab(s) => {
                    let rings: Vec<&[Pt]> = std::iter::once(s.base.outer.as_slice())
                        .chain(s.base.holes.iter().map(Vec::as_slice))
                        .collect();
                    seen_edge(&rings, &s.base, origin, right, look, p.x)
                }
                Shape::Roof(r) => {
                    let base = studio_geom::Poly::simple(r.boundary.clone());
                    seen_edge(&[r.boundary.as_slice()], &base, origin, right, look, p.x)
                }
            }
        }
    }
}

/// The boundary segment seen nearest at view position `u`: facing the viewer, spanning
/// `u` across the view, and the closest along `look`.
fn seen_edge(
    rings: &[&[Pt]],
    poly: &studio_geom::Poly,
    origin: Pt,
    right: Pt,
    look: Pt,
    u: f64,
) -> Option<String> {
    let mut best: Option<(f64, usize)> = None;
    let mut k = 0;
    for r in rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let idx = k;
            k += 1;
            let d = b.sub(a);
            if d.len() < 1e-6 {
                continue;
            }
            // Outward: the side away from the slab.
            let mut out = Pt::new(d.y, -d.x).norm();
            let mid = a.lerp(b, 0.5);
            if poly.contains(mid.add(out.scale(5.0))) {
                out = out.scale(-1.0);
            }
            if out.dot(look) > -0.2 {
                continue; // Facing away, or edge-on.
            }
            let (ua, ub) = (a.sub(origin).dot(right), b.sub(origin).dot(right));
            if u < ua.min(ub) - 1.0 || u > ua.max(ub) + 1.0 {
                continue;
            }
            // Depth where the view line at u crosses it.
            let t = if (ub - ua).abs() < 1e-9 {
                0.5
            } else {
                (u - ua) / (ub - ua)
            };
            let depth = a.lerp(b, t).sub(origin).dot(look);
            if best.is_none_or(|(bd, _)| depth < bd) {
                best = Some((depth, idx));
            }
        }
    }
    best.map(|(_, i)| format!("edge:{i}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::{ops, Category, Document};

    fn box_floor(doc: &mut Document) -> ElementId {
        let ft = ops::first_of(doc, Category::FloorType).unwrap();
        let l1 = doc.levels()[0].0;
        ops::create_floor(
            doc,
            ft,
            l1,
            vec![
                Pt::new(0.0, 0.0),
                Pt::new(4000.0, 0.0),
                Pt::new(4000.0, 3000.0),
                Pt::new(0.0, 3000.0),
            ],
        )
        .unwrap()
    }

    #[test]
    fn a_floor_has_a_top_a_soffit_and_an_edge_per_side() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let f = box_floor(&mut doc);
        let m = studio_regen::regenerate(&doc);
        let s = Shape::of(&m, f).unwrap();
        let Shape::Slab(slab) = &s else { panic!() };
        let (z0, z1) = (slab.z0, slab.z1);
        let mid = (z0 + z1) / 2.0;
        assert_eq!(
            s.face([2000.0, 1500.0, z1], [0.0, 0.0, 1.0]).unwrap(),
            "top"
        );
        // The normal's sign doesn't matter: the height says which.
        assert_eq!(
            s.face([2000.0, 1500.0, z0], [0.0, 0.0, 1.0]).unwrap(),
            "bottom"
        );
        // The four sides, along the boundary as drawn: south, east, north, west.
        assert_eq!(
            s.face([2000.0, 0.0, mid], [0.0, -1.0, 0.0]).unwrap(),
            "edge:0"
        );
        assert_eq!(
            s.face([4000.0, 1500.0, mid], [1.0, 0.0, 0.0]).unwrap(),
            "edge:1"
        );
        assert_eq!(
            s.face([2000.0, 3000.0, mid], [0.0, 1.0, 0.0]).unwrap(),
            "edge:2"
        );
        assert_eq!(
            s.face([0.0, 1500.0, mid], [-1.0, 0.0, 0.0]).unwrap(),
            "edge:3"
        );
        // Every triangle of its mesh is named: 2 top, 2 bottom, 2 per side.
        let tris = slab.triangles();
        let mut names: Vec<String> = tris
            .as_chunks::<9>()
            .0
            .iter()
            .map(|t| s.triangle_face(t).unwrap())
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(
            names,
            ["bottom", "edge:0", "edge:1", "edge:2", "edge:3", "top"]
        );
    }

    #[test]
    fn a_wall_has_two_faces_two_ends_a_top_and_a_bottom() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let l1 = doc.levels()[0].0;
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let m = studio_regen::regenerate(&doc);
        let s = Shape::of(&m, w).unwrap();
        let Shape::Wall(ws) = &s else { panic!() };
        let (t, z1) = (ws.thickness, ws.z1);
        // Exterior is the location line's left: +y for a wall drawn west to east.
        assert_eq!(
            s.face([2000.0, t / 2.0, 1000.0], [0.0, 1.0, 0.0]).unwrap(),
            "exterior"
        );
        assert_eq!(
            s.face([2000.0, -t / 2.0, 1000.0], [0.0, -1.0, 0.0])
                .unwrap(),
            "interior"
        );
        assert_eq!(
            s.face([0.0, 0.0, 1000.0], [-1.0, 0.0, 0.0]).unwrap(),
            "start"
        );
        assert_eq!(
            s.face([4000.0, 0.0, 1000.0], [1.0, 0.0, 0.0]).unwrap(),
            "end"
        );
        assert_eq!(s.face([2000.0, 0.0, z1], [0.0, 0.0, 1.0]).unwrap(), "top");
        assert_eq!(
            s.face([2000.0, 0.0, ws.z0], [0.0, 0.0, -1.0]).unwrap(),
            "bottom"
        );
        // A jamb halfway along isn't a named face.
        assert_eq!(s.face([2000.0, 0.0, 1000.0], [1.0, 0.0, 0.0]), None);
    }

    #[test]
    fn painted_faces_split_off_in_3d_with_their_material() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let f = box_floor(&mut doc);
        let wood = studio_core::library::add_preset(&mut doc, "siding-cedar-lap-stained").unwrap();
        let white = studio_core::library::add_preset(&mut doc, "plaster-stucco-white").unwrap();
        studio_core::paint::paint_face(&mut doc, f, "edge:0", Some(wood)).unwrap();
        studio_core::paint::paint_face(&mut doc, f, "bottom", Some(white)).unwrap();
        let meshes: Vec<crate::Mesh> = crate::meshes(&doc)
            .into_iter()
            .filter(|m| m.el == f)
            .collect();
        let tris = |m: &crate::Mesh| m.positions.len() / 9;
        let by = |mat: ElementId| meshes.iter().find(|m| m.material == Some(mat)).unwrap();
        // The south edge (2 triangles) in wood, the soffit (2) in white, the rest as typed.
        assert_eq!(tris(by(wood)), 2);
        assert_eq!(tris(by(white)), 2);
        let finish = studio_core::library::finish_of(&doc, f);
        let rest = meshes.iter().find(|m| m.material == finish).unwrap();
        assert_eq!(tris(rest), 2 + 3 * 2);
        assert_eq!(meshes.iter().map(tris).sum::<usize>(), 12);
    }
}
