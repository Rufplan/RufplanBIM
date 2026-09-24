//! Math, 2D polygon operations and prismatic solids.
//!
//! All lengths are millimetres. The native kernel only handles vertical extrusions of
//! simple polygons (ADR-006), which covers walls, floors and ceilings.

pub mod tol {
    /// Linear tolerance in mm.
    pub const LINEAR: f64 = 0.01;
    /// Angular tolerance in radians.
    pub const ANGULAR: f64 = 1e-9;
    /// Distance under which two points are treated as the same join point, in mm.
    pub const JOIN: f64 = 1.0;
}

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A 2D point or vector in mm.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

// Named methods rather than operator traits keep chained vector math explicit.
#[allow(clippy::should_implement_trait)]
impl Pt {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn add(self, o: Pt) -> Pt {
        Pt::new(self.x + o.x, self.y + o.y)
    }
    pub fn sub(self, o: Pt) -> Pt {
        Pt::new(self.x - o.x, self.y - o.y)
    }
    pub fn scale(self, k: f64) -> Pt {
        Pt::new(self.x * k, self.y * k)
    }
    pub fn dot(self, o: Pt) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: Pt) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn dist(self, o: Pt) -> f64 {
        self.sub(o).len()
    }
    /// Unit vector, or zero for a zero-length vector.
    pub fn norm(self) -> Pt {
        let l = self.len();
        if l < tol::LINEAR {
            Pt::default()
        } else {
            self.scale(1.0 / l)
        }
    }
    /// Rotated 90° counter-clockwise.
    pub fn perp(self) -> Pt {
        Pt::new(-self.y, self.x)
    }
    pub fn lerp(self, o: Pt, t: f64) -> Pt {
        self.add(o.sub(self).scale(t))
    }
}

/// Intersection of the infinite lines `p + t*d` and `q + s*e`, or None if parallel.
pub fn line_intersection(p: Pt, d: Pt, q: Pt, e: Pt) -> Option<Pt> {
    let denom = d.cross(e);
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = q.sub(p).cross(e) / denom;
    Some(p.add(d.scale(t)))
}

/// Parameter `t` in [0, 1] of the closest point on segment ab to p, and that distance.
pub fn project_to_segment(p: Pt, a: Pt, b: Pt) -> (f64, f64) {
    let ab = b.sub(a);
    let l2 = ab.dot(ab);
    if l2 < tol::LINEAR * tol::LINEAR {
        return (0.0, p.dist(a));
    }
    let t = (p.sub(a).dot(ab) / l2).clamp(0.0, 1.0);
    (t, p.dist(a.lerp(b, t)))
}

/// Signed area of a ring in mm² (positive = counter-clockwise).
pub fn signed_area(ring: &[Pt]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| ring[i].cross(ring[(i + 1) % n]))
        .sum::<f64>()
        / 2.0
}

/// Ray-casting point-in-ring test.
pub fn point_in_ring(p: Pt, ring: &[Pt]) -> bool {
    let n = ring.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// A polygon with an outer ring and optional holes. Rings are not closed (no repeated
/// first point).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Poly {
    pub outer: Vec<Pt>,
    pub holes: Vec<Vec<Pt>>,
}

impl Poly {
    pub fn simple(outer: Vec<Pt>) -> Self {
        Self {
            outer,
            holes: vec![],
        }
    }
    pub fn contains(&self, p: Pt) -> bool {
        point_in_ring(p, &self.outer) && !self.holes.iter().any(|h| point_in_ring(p, h))
    }
    pub fn area(&self) -> f64 {
        signed_area(&self.outer).abs()
            - self.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
    }
}

fn to_geo(p: &Poly) -> geo::Polygon<f64> {
    let ring = |r: &[Pt]| geo::LineString::from(r.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>());
    geo::Polygon::new(ring(&p.outer), p.holes.iter().map(|h| ring(h)).collect())
}

fn from_geo_ring(ls: &geo::LineString<f64>) -> Vec<Pt> {
    let mut pts: Vec<Pt> = ls.coords().map(|c| Pt::new(c.x, c.y)).collect();
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    simplify_ring(pts)
}

/// Drops vertices that lie within 0.05 mm of the line through their neighbours.
pub fn simplify_ring(mut pts: Vec<Pt>) -> Vec<Pt> {
    let mut changed = true;
    while changed && pts.len() > 3 {
        changed = false;
        let n = pts.len();
        for i in 0..n {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let (_, d) = project_to_segment(b, a, c);
            if d < 0.05 || a.dist(b) < 0.05 {
                pts.remove(i);
                changed = true;
                break;
            }
        }
    }
    pts
}

/// Offsets a ring outward by `d` mm by moving each edge along its normal and
/// re-intersecting neighbours. Exact for convex rings; fine for small `d` otherwise.
pub fn offset_ring(ring: &[Pt], d: f64) -> Vec<Pt> {
    let n = ring.len();
    if n < 3 {
        return ring.to_vec();
    }
    // Outward normal of a CCW ring is the clockwise perpendicular of each edge.
    let sign = if signed_area(ring) > 0.0 { -1.0 } else { 1.0 };
    let edge = |i: usize| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let dir = b.sub(a).norm();
        (a.add(dir.perp().scale(sign * d)), dir)
    };
    (0..n)
        .map(|i| {
            let (p0, d0) = edge((i + n - 1) % n);
            let (p1, d1) = edge(i);
            line_intersection(p0, d0, p1, d1).unwrap_or(p1)
        })
        .collect()
}

/// Union of polygons. Touching or overlapping inputs merge into single outlines.
///
/// Inputs are grown by `tol::LINEAR` first: the boolean engine can leave polygons that
/// share an edge exactly (e.g. mitered wall corners) as separate pieces, while
/// overlapping ones always merge. The result is at most 0.01 mm larger than exact.
pub fn union_all(polys: &[Poly]) -> Vec<Poly> {
    use geo::BooleanOps;
    let mut acc = geo::MultiPolygon::<f64>::new(vec![]);
    for p in polys {
        if p.outer.len() < 3 || signed_area(&p.outer).abs() < 1.0 {
            continue;
        }
        let grown = Poly {
            outer: offset_ring(&p.outer, tol::LINEAR),
            holes: p.holes.clone(),
        };
        acc = acc.union(&geo::MultiPolygon::new(vec![to_geo(&grown)]));
    }
    acc.0
        .iter()
        .map(|g| Poly {
            outer: from_geo_ring(g.exterior()),
            holes: g.interiors().iter().map(from_geo_ring).collect(),
        })
        .collect()
}

/// Triangulates a polygon (with holes). Returns vertex list and triangle index triples.
pub fn triangulate(p: &Poly) -> (Vec<Pt>, Vec<[usize; 3]>) {
    let mut verts: Vec<Pt> = p.outer.clone();
    let mut hole_starts = vec![];
    for h in &p.holes {
        hole_starts.push(verts.len());
        verts.extend_from_slice(h);
    }
    let flat: Vec<f64> = verts.iter().flat_map(|v| [v.x, v.y]).collect();
    let idx = earcutr::earcut(&flat, &hole_starts, 2).unwrap_or_default();
    let tris = idx.as_chunks::<3>().0.to_vec();
    (verts, tris)
}

/// A vertical extrusion of `base` from `z0` to `z1` (mm).
#[derive(Debug, Clone, PartialEq)]
pub struct Prism {
    pub base: Poly,
    pub z0: f64,
    pub z1: f64,
}

impl Prism {
    /// Flat-shaded triangle soup: 9 floats per triangle (x, y, z per vertex), z-up.
    pub fn triangles(&self) -> Vec<f32> {
        let mut out = vec![];
        let (verts, tris) = triangulate(&self.base);
        let ccw = signed_area(&self.base.outer) > 0.0;
        let mut push = |a: Pt, az: f64, b: Pt, bz: f64, c: Pt, cz: f64| {
            for (p, z) in [(a, az), (b, bz), (c, cz)] {
                out.extend_from_slice(&[p.x as f32, p.y as f32, z as f32]);
            }
        };
        for t in &tris {
            let (a, b, c) = (verts[t[0]], verts[t[1]], verts[t[2]]);
            // Orient caps outward: top faces +z, bottom faces -z.
            let up = b.sub(a).cross(c.sub(a)) > 0.0;
            if up {
                push(a, self.z1, b, self.z1, c, self.z1);
                push(a, self.z0, c, self.z0, b, self.z0);
            } else {
                push(a, self.z1, c, self.z1, b, self.z1);
                push(a, self.z0, b, self.z0, c, self.z0);
            }
        }
        let mut side = |ring: &[Pt], outward_ccw: bool| {
            let n = ring.len();
            for i in 0..n {
                let (mut a, mut b) = (ring[i], ring[(i + 1) % n]);
                if !outward_ccw {
                    std::mem::swap(&mut a, &mut b);
                }
                push(a, self.z0, b, self.z0, b, self.z1);
                push(a, self.z0, b, self.z1, a, self.z1);
            }
        };
        side(&self.base.outer, ccw);
        for h in &self.base.holes {
            side(h, signed_area(h) < 0.0);
        }
        out
    }
}

/// Crate version from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Poly {
        Poly::simple(vec![
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ])
    }

    #[test]
    fn line_intersection_of_axes() {
        let p = line_intersection(
            Pt::new(0.0, 5.0),
            Pt::new(1.0, 0.0),
            Pt::new(3.0, 0.0),
            Pt::new(0.0, 1.0),
        );
        assert_eq!(p, Some(Pt::new(3.0, 5.0)));
        assert!(line_intersection(
            Pt::default(),
            Pt::new(1.0, 0.0),
            Pt::new(0.0, 1.0),
            Pt::new(2.0, 0.0)
        )
        .is_none());
    }

    #[test]
    fn union_of_overlapping_rects_is_one_polygon() {
        let u = union_all(&[rect(0.0, 0.0, 10.0, 10.0), rect(5.0, 0.0, 15.0, 10.0)]);
        assert_eq!(u.len(), 1);
        assert!((u[0].area() - 150.0).abs() < 1.0);
        assert_eq!(u[0].outer.len(), 4);
    }

    #[test]
    fn union_of_ring_of_rects_has_hole() {
        // Four 1 mm-thick bars forming a 10 × 10 frame.
        let u = union_all(&[
            rect(0.0, 0.0, 10.0, 1.0),
            rect(9.0, 0.0, 10.0, 10.0),
            rect(0.0, 9.0, 10.0, 10.0),
            rect(0.0, 0.0, 1.0, 10.0),
        ]);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].holes.len(), 1);
        assert!((u[0].area() - 36.0).abs() < 1.0);
        assert!(u[0].contains(Pt::new(0.5, 5.0)));
        assert!(!u[0].contains(Pt::new(5.0, 5.0)));
    }

    #[test]
    fn prism_triangle_count() {
        let p = Prism {
            base: rect(0.0, 0.0, 1.0, 1.0),
            z0: 0.0,
            z1: 1.0,
        };
        // 2 caps × 2 triangles + 4 sides × 2 triangles = 12 triangles × 9 floats.
        assert_eq!(p.triangles().len(), 12 * 9);
    }

    #[test]
    fn projection_to_segment() {
        let (t, d) = project_to_segment(Pt::new(5.0, 3.0), Pt::new(0.0, 0.0), Pt::new(10.0, 0.0));
        assert!((t - 0.5).abs() < 1e-12);
        assert!((d - 3.0).abs() < 1e-12);
    }
}
#[cfg(test)]
mod union_edge_cases {
    use super::*;

    #[test]
    fn mitered_wall_quads_sharing_an_edge_merge() {
        let w1 = Poly::simple(vec![
            Pt::new(-101.6, -101.6),
            Pt::new(12293.6, -101.6),
            Pt::new(12090.4, 101.6),
            Pt::new(101.6, 101.6),
        ]);
        let w2 = Poly::simple(vec![
            Pt::new(12293.6, -101.6),
            Pt::new(12293.6, 9245.6),
            Pt::new(12090.4, 9042.4),
            Pt::new(12090.4, 101.6),
        ]);
        let u = union_all(&[w1.clone(), w2.clone()]);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].outer.len(), 6);
        assert!((u[0].area() - (w1.area() + w2.area())).abs() < 1000.0);
    }

    #[test]
    fn offset_square_grows_each_side() {
        let sq = vec![
            Pt::new(0.0, 0.0),
            Pt::new(10.0, 0.0),
            Pt::new(10.0, 10.0),
            Pt::new(0.0, 10.0),
        ];
        let o = offset_ring(&sq, 1.0);
        assert!(o.iter().any(|p| p.dist(Pt::new(-1.0, -1.0)) < 1e-9));
        assert!((signed_area(&o) - 144.0).abs() < 1e-9);
        let mut cw = sq.clone();
        cw.reverse();
        assert!((signed_area(&offset_ring(&cw, 1.0)).abs() - 144.0).abs() < 1e-9);
    }
}
