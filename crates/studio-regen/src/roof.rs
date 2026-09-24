//! Roof by footprint (ADR-018). Every sloped eave edge rises inward at the roof's slope,
//! so the top surface over a point is `base + tan(slope) × (distance to the nearest sloped
//! edge line)`. Each sloped edge owns the part of the footprint where it is the nearest
//! (a half-plane test against every other sloped edge), which gives hips and ridges for
//! convex footprints — rectangles, and L- or T-shaped houses split into convex roofs.

use studio_core::ElementId;
use studio_geom::{clip_half_plane, signed_area, triangulate, Poly, Pt};

/// One planar roof face: the plan polygon where eave edge `edge` is the nearest.
#[derive(Debug, Clone, PartialEq)]
pub struct RoofFace {
    pub poly: Vec<Pt>,
    /// A point on the eave edge and the edge's inward unit normal.
    pub a: Pt,
    pub n: Pt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoofSolid {
    pub id: ElementId,
    pub level: ElementId,
    /// Eave outline, counter-clockwise.
    pub boundary: Vec<Pt>,
    /// Underside height at the eave line, mm.
    pub base: f64,
    pub slope: f64,
    /// Thickness square to the surface, mm.
    pub thickness: f64,
    /// Sloped faces; empty for a flat roof.
    pub faces: Vec<RoofFace>,
}

impl RoofSolid {
    pub fn build(
        id: ElementId,
        level: ElementId,
        boundary: &[Pt],
        base: f64,
        slope: f64,
        sloped: &[bool],
        thickness: f64,
    ) -> RoofSolid {
        let mut ring = boundary.to_vec();
        let mut flags = sloped.to_vec();
        if signed_area(&ring) < 0.0 {
            // Keep "inward" on the left of each edge.
            ring.reverse();
            flags.reverse();
            flags.rotate_left(1);
        }
        let n = ring.len();
        let edges: Vec<(Pt, Pt)> = (0..n)
            .filter(|i| slope > 1e-6 && flags.get(*i).copied().unwrap_or(false))
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                (a, b.sub(a).norm().perp())
            })
            .collect();
        let mut faces = vec![];
        for (i, &(ai, ni)) in edges.iter().enumerate() {
            let mut poly = ring.clone();
            for (j, &(aj, nj)) in edges.iter().enumerate() {
                if i == j || poly.len() < 3 {
                    continue;
                }
                // Keep where d_i <= d_j:  p·(n_i - n_j) <= a_i·n_i - a_j·n_j.
                let m = ni.sub(nj);
                let k = ai.dot(ni) - aj.dot(nj);
                let mm = m.dot(m);
                if mm < 1e-12 {
                    // Parallel edges facing the same way: the nearer line always wins.
                    if k > 0.0 {
                        poly.clear();
                    }
                    continue;
                }
                let p0 = m.scale(k / mm);
                poly = clip_half_plane(&poly, p0, m.scale(-1.0));
            }
            if poly.len() >= 3 && signed_area(&poly).abs() > 1.0 {
                faces.push(RoofFace { poly, a: ai, n: ni });
            }
        }
        RoofSolid {
            id,
            level,
            boundary: ring,
            base,
            slope,
            thickness,
            faces,
        }
    }

    pub fn is_flat(&self) -> bool {
        self.faces.is_empty()
    }

    /// Vertical thickness (the square thickness measured plumb).
    pub fn plumb_thickness(&self) -> f64 {
        self.thickness / self.slope.cos().max(0.2)
    }

    /// Height of the underside above point `p` (mm).
    pub fn underside(&self, p: Pt) -> f64 {
        if self.is_flat() {
            return self.base;
        }
        let d = self
            .faces
            .iter()
            .map(|f| p.sub(f.a).dot(f.n))
            .fold(f64::INFINITY, f64::min)
            .max(0.0);
        self.base + self.slope.tan() * d
    }

    /// Height of the top surface above point `p` (mm).
    pub fn top(&self, p: Pt) -> f64 {
        self.underside(p) + self.plumb_thickness()
    }

    /// Top z on one face's plane (exact on that face, used for its vertices).
    pub fn face_top(&self, f: &RoofFace, p: Pt) -> f64 {
        self.base + self.plumb_thickness() + self.slope.tan() * p.sub(f.a).dot(f.n).max(0.0)
    }

    /// Highest point of the roof (mm).
    pub fn peak(&self) -> f64 {
        let mut z = self.base + self.plumb_thickness();
        for f in &self.faces {
            for p in &f.poly {
                z = z.max(self.face_top(f, *p));
            }
        }
        z
    }

    /// Points along boundary edge `i` (from vertex i) where the top surface bends:
    /// the edge ends plus every face corner lying on it, in order.
    fn edge_breaks(&self, i: usize) -> Vec<Pt> {
        let n = self.boundary.len();
        let (a, b) = (self.boundary[i], self.boundary[(i + 1) % n]);
        let len = a.dist(b);
        let dir = b.sub(a).norm();
        let mut ts = vec![0.0, len];
        for f in &self.faces {
            for p in &f.poly {
                let t = p.sub(a).dot(dir);
                let off = p.sub(a).cross(dir).abs();
                if off < 0.5 && t > 0.5 && t < len - 0.5 {
                    ts.push(t);
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 0.5);
        ts.into_iter().map(|t| a.add(dir.scale(t))).collect()
    }

    /// Visible surfaces as 3D polygons (x, y, z): top faces (or the flat top), and the
    /// fascia / gable-end faces around the boundary.
    pub fn surfaces(&self) -> Vec<Vec<[f64; 3]>> {
        let mut out = vec![];
        if self.is_flat() {
            let z = self.base + self.thickness;
            out.push(self.boundary.iter().map(|p| [p.x, p.y, z]).collect());
        } else {
            for f in &self.faces {
                out.push(
                    f.poly
                        .iter()
                        .map(|p| [p.x, p.y, self.face_top(f, *p)])
                        .collect(),
                );
            }
        }
        let n = self.boundary.len();
        let t = if self.is_flat() {
            self.thickness
        } else {
            self.plumb_thickness()
        };
        for i in 0..n {
            let pts = self.edge_breaks(i);
            for w in pts.windows(2) {
                let (p, q) = (w[0], w[1]);
                let (bp, bq) = (self.underside(p), self.underside(q));
                out.push(vec![
                    [p.x, p.y, bp],
                    [q.x, q.y, bq],
                    [q.x, q.y, bq + t],
                    [p.x, p.y, bp + t],
                ]);
            }
        }
        out
    }

    /// Flat-shaded triangle soup (9 floats per triangle), like `Prism::triangles`.
    pub fn triangles(&self) -> Vec<f32> {
        let mut out = vec![];
        let mut tri = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
            for v in [a, b, c] {
                out.extend_from_slice(&[v[0] as f32, v[1] as f32, v[2] as f32]);
            }
        };
        let t = if self.is_flat() {
            self.thickness
        } else {
            self.plumb_thickness()
        };
        let faces: Vec<(Vec<Pt>, Option<&RoofFace>)> = if self.is_flat() {
            vec![(self.boundary.clone(), None)]
        } else {
            self.faces
                .iter()
                .map(|f| (f.poly.clone(), Some(f)))
                .collect()
        };
        for (poly, face) in &faces {
            let (verts, tris) = triangulate(&Poly::simple(poly.clone()));
            let top = |p: Pt| match face {
                Some(f) => self.face_top(f, p),
                None => self.base + t,
            };
            for k in &tris {
                let (a, b, c) = (verts[k[0]], verts[k[1]], verts[k[2]]);
                let up = b.sub(a).cross(c.sub(a)) > 0.0;
                let (b, c) = if up { (b, c) } else { (c, b) };
                tri([a.x, a.y, top(a)], [b.x, b.y, top(b)], [c.x, c.y, top(c)]);
                let bot = |p: Pt| top(p) - t;
                tri([a.x, a.y, bot(a)], [c.x, c.y, bot(c)], [b.x, b.y, bot(b)]);
            }
        }
        // Sides (the boundary is counter-clockwise, so outward is to the right).
        let n = self.boundary.len();
        for i in 0..n {
            let pts = self.edge_breaks(i);
            for w in pts.windows(2) {
                let (p, q) = (w[0], w[1]);
                let (bp, bq) = (self.underside(p), self.underside(q));
                let p0 = [p.x, p.y, bp];
                let q0 = [q.x, q.y, bq];
                let q1 = [q.x, q.y, bq + t];
                let p1 = [p.x, p.y, bp + t];
                tri(p0, q0, q1);
                tri(p0, q1, p1);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    fn rect(w: f64, h: f64) -> Vec<Pt> {
        vec![
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ]
    }

    #[test]
    fn hip_roof_has_four_faces_and_peaks_on_the_ridge() {
        let slope = 0.5f64.atan(); // 6:12
        let r = RoofSolid::build(
            ElementId::new(),
            ElementId::new(),
            &rect(12000.0, 8000.0),
            3000.0,
            slope,
            &[true; 4],
            200.0,
        );
        assert_eq!(r.faces.len(), 4);
        // The ridge is 4 m in from the long edges: underside rises 0.5 × 4000.
        assert!((r.underside(Pt::new(6000.0, 4000.0)) - 5000.0).abs() < EPS);
        assert!((r.underside(Pt::new(0.0, 0.0)) - 3000.0).abs() < EPS);
        let area: f64 = r.faces.iter().map(|f| signed_area(&f.poly).abs()).sum();
        assert!(
            (area - 12000.0 * 8000.0).abs() < 1.0,
            "faces tile the footprint"
        );
        assert!((r.peak() - (5000.0 + r.plumb_thickness())).abs() < EPS);
        // Closed mesh: every triangle is present (non-empty and a multiple of 9 floats).
        let t = r.triangles();
        assert!(!t.is_empty() && t.len().is_multiple_of(9));
    }

    #[test]
    fn gable_roof_slopes_only_on_flagged_edges() {
        let slope = 0.5f64.atan();
        // Only the long (south and north) edges slope.
        let r = RoofSolid::build(
            ElementId::new(),
            ElementId::new(),
            &rect(12000.0, 8000.0),
            0.0,
            slope,
            &[true, false, true, false],
            200.0,
        );
        assert_eq!(r.faces.len(), 2);
        // Ridge runs full length at y = 4000.
        assert!((r.underside(Pt::new(0.0, 4000.0)) - 2000.0).abs() < EPS);
        assert!((r.underside(Pt::new(12000.0, 4000.0)) - 2000.0).abs() < EPS);
        // Gable end faces include the ridge point in their outline.
        let gable = r
            .surfaces()
            .into_iter()
            .filter(|s| s.len() == 4 && s.iter().all(|v| v[0].abs() < EPS))
            .count();
        assert_eq!(gable, 2, "the west gable is split at the ridge");
    }

    #[test]
    fn flat_roof_and_clockwise_boundaries() {
        let mut b = rect(5000.0, 5000.0);
        b.reverse();
        let r = RoofSolid::build(
            ElementId::new(),
            ElementId::new(),
            &b,
            3000.0,
            0.0,
            &[false; 4],
            300.0,
        );
        assert!(r.is_flat());
        assert!(signed_area(&r.boundary) > 0.0);
        assert!((r.peak() - 3300.0).abs() < EPS);
    }
}
