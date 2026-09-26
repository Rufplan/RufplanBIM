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
