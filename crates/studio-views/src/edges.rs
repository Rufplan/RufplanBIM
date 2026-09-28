//! A wall in 3D without seams (ADR-038). Its solid is split in pieces around its openings;
//! drawn as they are, the pieces show a seam wherever they meet: the lines found from
//! their triangles, and the faces between them flickering through the wall's surface.
//! So a wall brings its own lines (its outline and its openings') and its triangles leave
//! out the faces where one piece meets another.

use studio_geom::{point_in_ring, signed_area, Pt};
use studio_regen::{OpeningSolid, WallSolid};

/// A wall's triangles (9 floats each, mm, z-up): its pieces' top and bottom faces, and
/// their side faces except where they meet another piece.
pub fn wall_triangles(w: &WallSolid) -> Vec<f32> {
    let mut out: Vec<f32> = vec![];
    let sloped = |z1: f64| w.top_profile.is_some() && z1 >= w.z1 - 0.5;
    for (i, p) in w.pieces.iter().enumerate() {
        let top = |q: Pt| if sloped(p.z1) { w.top_at(q) } else { p.z1 };
        // Top and bottom: the triangles over three distinct plan points (a side face's
        // triangles have two above one another).
        let all = if sloped(p.z1) {
            studio_geom::prism_triangles_to(&p.base, p.z0, top)
        } else {
            p.triangles()
        };
        for t in all.chunks(9) {
            let xy = |k: usize| (t[k * 3], t[k * 3 + 1]);
            let same =
                |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;
            if !same(xy(0), xy(1)) && !same(xy(1), xy(2)) && !same(xy(0), xy(2)) {
                out.extend_from_slice(t);
            }
        }
        // Sides, outward: where another piece touches, only the parts it doesn't cover.
        let ring = &p.base.outer;
        let ccw = signed_area(ring) > 0.0;
        let others: Vec<_> = w
            .pieces
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, q)| q)
            .collect();
        for k in 0..ring.len() {
            let (a, b) = if ccw {
                (ring[k], ring[(k + 1) % ring.len()])
            } else {
                (ring[(k + 1) % ring.len()], ring[k])
            };
            if a.dist(b) < 0.5 {
                continue;
            }
            // Just outside this face.
            let d = b.sub(a).norm();
            let probe = a.lerp(b, 0.5).add(Pt::new(d.y, -d.x).scale(1.0));
            let touching: Vec<_> = others
                .iter()
                .filter(|q| point_in_ring(probe, &q.base.outer))
                .collect();
            let (ta, tb) = (top(a), top(b));
            let mut quad = |z0: f64, za: f64, zb: f64| {
                let v = [
                    [a.x, a.y, z0],
                    [b.x, b.y, z0],
                    [b.x, b.y, zb],
                    [a.x, a.y, z0],
                    [b.x, b.y, zb],
                    [a.x, a.y, za],
                ];
                for p in v {
                    out.extend(p.map(|c| c as f32));
                }
            };
            if touching.is_empty() {
                quad(p.z0, ta, tb);
                continue;
            }
            // A face between pieces is level along its top (it runs across the wall).
            let hi = ta.min(tb);
            let mut cuts: Vec<f64> = vec![p.z0, hi];
            for q in &touching {
                cuts.extend([q.z0, q.z1].into_iter().filter(|z| *z > p.z0 && *z < hi));
            }
            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|x, y| (*x - *y).abs() < 0.5);
            for s in cuts.windows(2) {
                let mid = (s[0] + s[1]) / 2.0;
                if !touching.iter().any(|q| mid > q.z0 && mid < q.z1) {
                    quad(s[0], s[1], s[1]);
                }
            }
        }
    }
    out
}

/// Line segments, 6 floats each (mm, z-up), for a wall and the openings it hosts.
pub fn wall_edges(w: &WallSolid, openings: &[&OpeningSolid]) -> Vec<f32> {
    let mut out: Vec<f32> = vec![];
    let mut seg = |a: [f64; 3], b: [f64; 3]| {
        out.extend([a[0], a[1], a[2], b[0], b[1], b[2]].map(|v| v as f32));
    };
    let top = |p: Pt| w.top_at(p);
    // The outline without repeated points.
    let mut ring: Vec<Pt> = vec![];
    for p in &w.footprint.outer {
        if ring.last().is_none_or(|q: &Pt| q.dist(*p) > 0.5) {
            ring.push(*p);
        }
    }
    while ring.len() > 1 && ring[0].dist(ring[ring.len() - 1]) <= 0.5 {
        ring.pop();
    }
    let n = ring.len();
    if n < 3 {
        return out;
    }
    for i in 0..n {
        let (prev, a, b) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        seg([a.x, a.y, w.z0], [b.x, b.y, w.z0]);
        seg([a.x, a.y, top(a)], [b.x, b.y, top(b)]);
        // A corner where the outline turns; none where it runs straight on (a T join
        // adds a point along a face).
        if a.sub(prev).norm().cross(b.sub(a).norm()).abs() > 0.02 {
            seg([a.x, a.y, w.z0], [a.x, a.y, top(a)]);
        }
    }
    // Each opening: its outline on both faces and its four corners through the wall.
    let dir = w.dir();
    let nrm = dir.perp();
    let offsets = ring.iter().map(|p| p.sub(w.start).dot(nrm));
    let (lo, hi) = offsets.fold((f64::MAX, f64::MIN), |(l, h), s| (l.min(s), h.max(s)));
    for o in openings {
        let z0 = o.z0.max(w.z0);
        let z1 = o.z1.min(top(w.start.add(dir.scale((o.t0 + o.t1) / 2.0))));
        if z1 - z0 < 1.0 || o.t1 - o.t0 < 1.0 {
            continue;
        }
        let at = |t: f64, off: f64, z: f64| {
            let p = w.start.add(dir.scale(t)).add(nrm.scale(off));
            [p.x, p.y, z]
        };
        for off in [lo, hi] {
            let c = [
                at(o.t0, off, z0),
                at(o.t1, off, z0),
                at(o.t1, off, z1),
                at(o.t0, off, z1),
            ];
            for k in 0..4 {
                seg(c[k], c[(k + 1) % 4]);
            }
        }
        for (t, z) in [(o.t0, z0), (o.t1, z0), (o.t1, z1), (o.t0, z1)] {
            seg(at(t, lo, z), at(t, hi, z));
        }
    }
    out
}

// ---------- Sketched wall openings (ADR-058) ----------

/// A wall with sketched openings: its triangles, and each opening's reveals (the hole's
/// sides through the wall, drawn as that opening so it can be picked).
pub struct HoledWall {
    pub wall: Vec<f32>,
    pub reveals: Vec<(studio_core::ElementId, Vec<f32>)>,
}

fn tri3(out: &mut Vec<f32>, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
    for p in [a, b, c] {
        out.extend(p.map(|v| v as f32));
    }
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The outline without repeated points.
fn outline(w: &WallSolid) -> Vec<Pt> {
    let mut ring: Vec<Pt> = vec![];
    for p in &w.footprint.outer {
        if ring.last().is_none_or(|q: &Pt| q.dist(*p) > 0.5) {
            ring.push(*p);
        }
    }
    while ring.len() > 1 && ring[0].dist(ring[ring.len() - 1]) <= 0.5 {
        ring.pop();
    }
    ring
}

/// The wall's two long faces: (offset from the location line, u from, u to) for each
/// stretch of footprint running along it on either side.
fn long_faces(w: &WallSolid) -> Vec<(f64, f64, f64)> {
    let (dir, nrm) = (w.dir(), w.dir().perp());
    let ring = outline(w);
    let off = |p: Pt| p.sub(w.start).dot(nrm);
    let u = |p: Pt| p.sub(w.start).dot(dir);
    let half = w.thickness / 2.0;
    let n = ring.len();
    let mut out = vec![];
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if a.dist(b) < 1.0 || b.sub(a).norm().dot(dir).abs() < 0.999 {
            continue;
        }
        let o = (off(a) + off(b)) / 2.0;
        if (o.abs() - half).abs() > 1.0 {
            continue;
        }
        let (u0, u1) = (u(a).min(u(b)), u(a).max(u(b)));
        out.push((o, u0, u1));
    }
    out
}

/// A hole clipped to the wall's face (u from `u0` to `u1`, z from `z0` to `z1`).
fn clip_hole(ring: &[Pt], u0: f64, u1: f64, z0: f64, z1: f64) -> Vec<Pt> {
    let mut r = ring.to_vec();
    for (p, n) in [
        (Pt::new(u0, 0.0), Pt::new(1.0, 0.0)),
        (Pt::new(u1, 0.0), Pt::new(-1.0, 0.0)),
        (Pt::new(0.0, z0), Pt::new(0.0, 1.0)),
        (Pt::new(0.0, z1), Pt::new(0.0, -1.0)),
    ] {
        if r.len() < 3 {
            break;
        }
        r = studio_geom::clip_half_plane(&r, p, n);
    }
    r
}

/// The wall's overall extent in (u, z): the long faces' span and its base and top.
fn face_box(w: &WallSolid, faces: &[(f64, f64, f64)]) -> (f64, f64, f64, f64) {
    let u0 = faces.iter().map(|f| f.1).fold(f64::INFINITY, f64::min);
    let u1 = faces.iter().map(|f| f.2).fold(f64::NEG_INFINITY, f64::max);
    (u0, u1, w.z0, w.z1)
}

/// A wall cut by sketched openings (and its doors and windows): the pieces' tops, bottoms,
/// ends and door and window reveals as usual, the two long faces as regions with the
/// holes cut out, and each sketched hole's reveals through the wall.
pub fn wall_with_holes(w: &WallSolid, openings: &[&OpeningSolid]) -> HoledWall {
    let (dir, nrm) = (w.dir(), w.dir().perp());
    let n3 = [nrm.x, nrm.y, 0.0];
    // The pieces without their faces along the wall.
    let mut wall: Vec<f32> = vec![];
    for t in wall_triangles(w).chunks(9) {
        let p = |k: usize| {
            [
                f64::from(t[k * 3]),
                f64::from(t[k * 3 + 1]),
                f64::from(t[k * 3 + 2]),
            ]
        };
        let c = cross3(sub3(p(1), p(0)), sub3(p(2), p(0)));
        let len = dot3(c, c).sqrt();
        if len > 1e-9 && (dot3(c, n3) / len).abs() > 0.999 {
            continue;
        }
        wall.extend_from_slice(t);
    }
    let faces = long_faces(w);
    let at = |u: f64, off: f64, z: f64| {
        let q = w.start.add(dir.scale(u)).add(nrm.scale(off));
        [q.x, q.y, z]
    };
    let top = |u: f64| w.top_at(w.start.add(dir.scale(u)));
    let mut cuts: Vec<studio_geom::Poly> = w
        .holes
        .iter()
        .map(|(_, r)| studio_geom::Poly::simple(r.clone()))
        .collect();
    for o in openings {
        cuts.push(studio_geom::Poly::simple(vec![
            Pt::new(o.t0, o.z0),
            Pt::new(o.t1, o.z0),
            Pt::new(o.t1, o.z1),
            Pt::new(o.t0, o.z1),
        ]));
    }
    for (off, u0, u1) in &faces {
        let mut outer = vec![
            Pt::new(*u0, w.z0),
            Pt::new(*u1, w.z0),
            Pt::new(*u1, top(*u1)),
        ];
        if let Some(prof) = &w.top_profile {
            for (u, z) in prof.iter().rev() {
                if *u > *u0 + 0.5 && *u < *u1 - 0.5 {
                    outer.push(Pt::new(*u, *z));
                }
            }
        }
        outer.push(Pt::new(*u0, top(*u0)));
        for region in studio_geom::difference(&studio_geom::Poly::simple(outer), &cuts) {
            let (verts, tris) = studio_geom::triangulate(&region);
            for [i, j, k] in tris {
                let (a, b, c) = (verts[i], verts[j], verts[k]);
                // Counter-clockwise in (u, z) faces the wall's right: that side's outside.
                let ccw = b.sub(a).cross(c.sub(a)) > 0.0;
                let right = *off < 0.0;
                let (b, c) = if ccw == right { (b, c) } else { (c, b) };
                tri3(
                    &mut wall,
                    at(a.x, *off, a.y),
                    at(b.x, *off, b.y),
                    at(c.x, *off, c.y),
                );
            }
        }
    }
    // Reveals: each hole edge through the wall, facing into the hole.
    let mut reveals: Vec<(studio_core::ElementId, Vec<f32>)> = vec![];
    if faces.is_empty() {
        return HoledWall { wall, reveals };
    }
    let (bu0, bu1, bz0, bz1) = face_box(w, &faces);
    let half = w.thickness / 2.0;
    for (id, ring) in &w.holes {
        let r = clip_hole(ring, bu0, bu1, bz0, bz1);
        if r.len() < 3 {
            continue;
        }
        let mut tris = vec![];
        // Edges left along the wall's own edges by the clip get no reveal.
        let along_edge = |a: Pt, b: Pt| {
            let same = |x: f64, y: f64, v: f64| (x - v).abs() < 0.5 && (y - v).abs() < 0.5;
            same(a.x, b.x, bu0) || same(a.x, b.x, bu1) || same(a.y, b.y, bz0) || same(a.y, b.y, bz1)
        };
        let s = if signed_area(&r) > 0.0 { 1.0 } else { -1.0 };
        for k in 0..r.len() {
            let (a, b) = (r[k], r[(k + 1) % r.len()]);
            if a.dist(b) < 0.1 || along_edge(a, b) {
                continue;
            }
            // Into the hole: left of a counter-clockwise ring's edge.
            let (du, dz) = (b.x - a.x, b.y - a.y);
            let want = [-dir.x * dz * s, -dir.y * dz * s, du * s];
            let q = [
                at(a.x, -half, a.y),
                at(b.x, -half, b.y),
                at(b.x, half, b.y),
                at(a.x, half, a.y),
            ];
            let normal = cross3(sub3(q[1], q[0]), sub3(q[3], q[0]));
            if dot3(normal, want) >= 0.0 {
                tri3(&mut tris, q[0], q[1], q[2]);
                tri3(&mut tris, q[0], q[2], q[3]);
            } else {
                tri3(&mut tris, q[0], q[2], q[1]);
                tri3(&mut tris, q[0], q[3], q[2]);
            }
        }
        match reveals.iter_mut().find(|(o, _)| o == id) {
            Some((_, t)) => t.extend(tris),
            None => reveals.push((*id, tris)),
        }
    }
    HoledWall { wall, reveals }
}

/// Each sketched hole's outline on both faces (6 floats per segment).
pub fn hole_edges(w: &WallSolid) -> Vec<f32> {
    let (dir, nrm) = (w.dir(), w.dir().perp());
    let faces = long_faces(w);
    if faces.is_empty() {
        return vec![];
    }
    let (u0, u1, z0, z1) = face_box(w, &faces);
    let half = w.thickness / 2.0;
    let mut out = vec![];
    for (_, ring) in &w.holes {
        let r = clip_hole(ring, u0, u1, z0, z1);
        for off in [-half, half] {
            for k in 0..r.len() {
                let (a, b) = (r[k], r[(k + 1) % r.len()]);
                let pa = w.start.add(dir.scale(a.x)).add(nrm.scale(off));
                let pb = w.start.add(dir.scale(b.x)).add(nrm.scale(off));
                out.extend([pa.x, pa.y, a.y, pb.x, pb.y, b.y].map(|v| v as f32));
            }
        }
    }
    out
}

/// Where a horizontal cut at `z` crosses a hole (u from, u to), for plans.
pub fn hole_spans(ring: &[Pt], z: f64) -> Vec<(f64, f64)> {
    let mut xs: Vec<f64> = vec![];
    let n = ring.len();
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        if (a.y <= z) != (b.y <= z) {
            xs.push(a.x + (b.x - a.x) * (z - a.y) / (b.y - a.y));
        }
    }
    xs.sort_by(f64::total_cmp);
    xs.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])).collect()
}

/// The plan rectangles a wall's holes cut out of it at height `z`.
pub fn hole_cuts_at(w: &WallSolid, z: f64) -> Vec<studio_geom::Poly> {
    let (dir, nrm) = (w.dir(), w.dir().perp());
    let reach = w.thickness / 2.0 + 5.0;
    w.holes
        .iter()
        .flat_map(|(_, r)| hole_spans(r, z))
        .filter(|(a, b)| b - a > 0.5)
        .map(|(a, b)| {
            let p = |u: f64, o: f64| w.start.add(dir.scale(u)).add(nrm.scale(o));
            studio_geom::Poly::simple(vec![p(a, -reach), p(b, -reach), p(b, reach), p(a, reach)])
        })
        .collect()
}
