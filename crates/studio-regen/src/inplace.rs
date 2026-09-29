//! Solids of in-place elements (ADR-068): each form as triangles, void extrusions cut
//! out of the solid extrusions. Views cut and project these; 3D and IFC show them.

use studio_core::inplace::{self, Form, FormKind, SweepProfile};
use studio_core::sketch::polygons;
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::{signed_area, triangulate, Poly, Prism, Pt};

/// An in-place element's solid, in model mm (z-up).
#[derive(Debug, Clone, PartialEq)]
pub struct InPlaceSolid {
    pub id: ElementId,
    pub category: Category,
    pub level: ElementId,
    pub material: Option<ElementId>,
    /// Closed triangle meshes (9 floats a triangle, outward-facing), one a piece of a form:
    /// sections cut each on its own and join the cuts, so overlapping forms stay solid.
    pub pieces: Vec<Vec<f32>>,
    /// Lowest and highest z.
    pub z0: f64,
    pub z1: f64,
}

impl InPlaceSolid {
    /// Every triangle.
    pub fn triangles(&self) -> Vec<f32> {
        self.pieces.concat()
    }

    /// Plans cut it (Revit's cuttable categories).
    pub fn cuttable(&self) -> bool {
        inplace::cuttable(self.category)
    }
}

/// Every in-place element with forms.
pub fn solids(doc: &Document) -> Vec<InPlaceSolid> {
    doc.iter()
        .filter(|e| matches!(e.data, ElementData::InPlace { .. }))
        .filter_map(|e| solid(doc, e.id))
        .collect()
}

pub fn solid(doc: &Document, id: ElementId) -> Option<InPlaceSolid> {
    let ElementData::InPlace {
        category,
        level,
        material,
        forms,
        ..
    } = doc.data(id).ok()?
    else {
        return None;
    };
    let z = doc.level_elevation(*level).ok()?;
    let pieces: Vec<Vec<f32>> = form_pieces(doc, forms, z)
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect();
    if pieces.is_empty() {
        return None;
    }
    let zs = pieces
        .iter()
        .flat_map(|p| p.iter().skip(2).step_by(3))
        .map(|v| f64::from(*v));
    let (z0, z1) = zs.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
        (a.min(v), b.max(v))
    });
    Some(InPlaceSolid {
        id,
        category: *category,
        level: *level,
        material: *material,
        pieces,
        z0,
        z1,
    })
}

/// The forms' closed pieces, with the level at `z`.
pub fn form_pieces(doc: &Document, forms: &[Form], z: f64) -> Vec<Vec<f32>> {
    let voids: Vec<(Vec<Poly>, f64, f64)> = forms
        .iter()
        .filter(|f| f.void)
        .map(|f| {
            let (lo, hi) = f.z_range();
            (polygons(doc, &f.sketch), z + lo, z + hi)
        })
        .collect();
    let mut out = vec![];
    for f in forms.iter().filter(|f| !f.void) {
        let (lo, hi) = f.z_range();
        match &f.kind {
            FormKind::Extrusion { .. } => {
                for p in extrusion(&polygons(doc, &f.sketch), z + lo, z + hi, &voids) {
                    out.push(p.triangles());
                }
            }
            FormKind::Blend { top_sketch, .. } => {
                let base = polygons(doc, &f.sketch);
                let top = polygons(doc, std::slice::from_ref(top_sketch));
                if let (Some(b), Some(t)) = (base.first(), top.first()) {
                    let (bz, tz) = match f.kind {
                        FormKind::Blend { base, top, .. } => (z + base, z + top),
                        _ => unreachable!(),
                    };
                    out.push(blend(&b.outer, bz, &t.outer, tz));
                }
            }
            FormKind::Sweep { elevation, profile } => {
                if let Some(p) = f.sketch.first() {
                    let (pts, closed) = inplace::path_points(p);
                    out.push(sweep(&pts, closed, z + elevation, *profile));
                }
            }
        }
    }
    out
}

/// An extrusion as prisms: split where voids start and stop, the voids cut from each.
pub fn extrusion(polys: &[Poly], z0: f64, z1: f64, voids: &[(Vec<Poly>, f64, f64)]) -> Vec<Prism> {
    let mut cuts = vec![z0, z1];
    for (_, a, b) in voids {
        for v in [*a, *b] {
            if v > z0 + 0.5 && v < z1 - 0.5 {
                cuts.push(v);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    let mut out = vec![];
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let active: Vec<Poly> = voids
            .iter()
            .filter(|(_, lo, hi)| *lo <= a + 0.5 && *hi >= b - 0.5)
            .flat_map(|(p, _, _)| p.iter().cloned())
            .collect();
        for p in polys {
            let pieces = if active.is_empty() {
                vec![p.clone()]
            } else {
                studio_geom::difference(p, &active)
            };
            out.extend(pieces.into_iter().map(|base| Prism { base, z0: a, z1: b }));
        }
    }
    out
}

fn push(out: &mut Vec<f32>, v: [[f64; 3]; 3]) {
    for p in v {
        out.extend_from_slice(&[p[0] as f32, p[1] as f32, p[2] as f32]);
    }
}

/// Counter-clockwise copy of a ring.
fn ccw(ring: &[Pt]) -> Vec<Pt> {
    let mut r = ring.to_vec();
    if signed_area(&r) < 0.0 {
        r.reverse();
    }
    r
}

/// `n` points evenly along a closed ring's perimeter.
fn resample(ring: &[Pt], n: usize) -> Vec<Pt> {
    let m = ring.len();
    let lens: Vec<f64> = (0..m).map(|i| ring[i].dist(ring[(i + 1) % m])).collect();
    let total: f64 = lens.iter().sum();
    let mut out = vec![];
    let (mut edge, mut acc) = (0, 0.0);
    for k in 0..n {
        let s = total * k as f64 / n as f64;
        while edge < m - 1 && acc + lens[edge] < s {
            acc += lens[edge];
            edge += 1;
        }
        let t = if lens[edge] > 0.0 {
            ((s - acc) / lens[edge]).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push(ring[edge].lerp(ring[(edge + 1) % m], t));
    }
    out
}

fn centroid(r: &[Pt]) -> Pt {
    let n = r.len().max(1) as f64;
    Pt::new(
        r.iter().map(|p| p.x).sum::<f64>() / n,
        r.iter().map(|p| p.y).sum::<f64>() / n,
    )
}

/// The base and top loops with matching points: as drawn when they have as many corners
/// (a tapered box keeps its corners), else resampled evenly; the top turned to line up
/// with the base.
pub fn blend_rings(base: &[Pt], top: &[Pt]) -> (Vec<Pt>, Vec<Pt>) {
    let (b, t) = (ccw(base), ccw(top));
    let (b, t) = if b.len() == t.len() {
        (b, t)
    } else {
        let n = b.len().max(t.len()).max(48) * 2;
        (resample(&b, n), resample(&t, n))
    };
    let n = b.len();
    let (cb, ct) = (centroid(&b), centroid(&t));
    let cost = |k: usize| {
        (0..n)
            .map(|i| b[i].sub(cb).dist(t[(i + k) % n].sub(ct)))
            .sum::<f64>()
    };
    let k = (0..n)
        .min_by(|x, y| cost(*x).total_cmp(&cost(*y)))
        .unwrap_or(0);
    let t = (0..n).map(|i| t[(i + k) % n]).collect();
    (b, t)
}

/// A blend's triangles: its sides between the matched loops, and both ends capped.
pub fn blend(base: &[Pt], z0: f64, top: &[Pt], z1: f64) -> Vec<f32> {
    let (b, t) = blend_rings(base, top);
    let (b, t, z0, z1) = if z0 <= z1 {
        (b, t, z0, z1)
    } else {
        (t, b, z1, z0)
    };
    let n = b.len();
    let mut out = vec![];
    for i in 0..n {
        let j = (i + 1) % n;
        let (b0, b1) = ([b[i].x, b[i].y, z0], [b[j].x, b[j].y, z0]);
        let (t0, t1) = ([t[i].x, t[i].y, z1], [t[j].x, t[j].y, z1]);
        push(&mut out, [b0, b1, t1]);
        push(&mut out, [b0, t1, t0]);
    }
    cap(&mut out, &b, z0, false);
    cap(&mut out, &t, z1, true);
    out
}

/// A flat cap of a counter-clockwise ring, facing up or down.
fn cap(out: &mut Vec<f32>, ring: &[Pt], z: f64, up: bool) {
    let (verts, tris) = triangulate(&Poly::simple(ring.to_vec()));
    for tr in tris {
        let (a, b, c) = (verts[tr[0]], verts[tr[1]], verts[tr[2]]);
        let ccw = b.sub(a).cross(c.sub(a)) > 0.0;
        let (b, c) = if ccw == up { (b, c) } else { (c, b) };
        push(out, [[a.x, a.y, z], [b.x, b.y, z], [c.x, c.y, z]]);
    }
}

/// The profile's outline across the path (s across to the left, v up), counter-clockwise
/// seen looking along the path.
fn profile_ring(p: SweepProfile) -> Vec<(f64, f64)> {
    match p {
        SweepProfile::Rectangle { width, height } => {
            let h = width / 2.0;
            vec![(h, 0.0), (-h, 0.0), (-h, height), (h, height)]
        }
        SweepProfile::Circle { diameter } => {
            let r = diameter / 2.0;
            (0..16)
                .map(|i| {
                    let a = -std::f64::consts::FRAC_PI_2 - i as f64 * std::f64::consts::TAU / 16.0;
                    (r * a.cos(), r + r * a.sin())
                })
                .collect()
        }
    }
}

/// A sweep's triangles: the profile carried along the path with mitred corners, capped
/// at the ends of an open path.
pub fn sweep(path: &[Pt], closed: bool, z: f64, profile: SweepProfile) -> Vec<f32> {
    let n = path.len();
    if n < 2 {
        return vec![];
    }
    let seg = |i: usize| path[(i + 1) % n].sub(path[i]).norm();
    let segs = if closed { n } else { n - 1 };
    let prof = profile_ring(profile);
    // Each station: its point, its across direction (left) with the mitre's stretch.
    let station = |i: usize| -> (Pt, Pt) {
        let (din, dout) = match (closed, i) {
            (false, 0) => (seg(0), seg(0)),
            (false, k) if k == n - 1 => (seg(k - 1), seg(k - 1)),
            (_, k) => (seg((k + n - 1) % n), seg(k)),
        };
        let t = din.add(dout).norm();
        let t = if t.len() < 1e-9 { dout } else { t };
        let left = t.perp();
        let stretch = 1.0 / left.dot(dout.perp()).max(0.2);
        (path[i], left.scale(stretch))
    };
    let ring = |i: usize| -> Vec<[f64; 3]> {
        let (p, left) = station(i % n);
        prof.iter()
            .map(|(s, v)| {
                let q = p.add(left.scale(*s));
                [q.x, q.y, z + v]
            })
            .collect()
    };
    let mut out = vec![];
    let m = prof.len();
    for i in 0..segs {
        let (a, b) = (ring(i), ring(i + 1));
        for k in 0..m {
            let l = (k + 1) % m;
            // Outward: the profile runs counter-clockwise seen along the path.
            push(&mut out, [a[k], b[k], b[l]]);
            push(&mut out, [a[k], b[l], a[l]]);
        }
    }
    if !closed {
        for (i, facing_back) in [(0, false), (n - 1, true)] {
            let r = ring(i);
            for k in 1..m - 1 {
                let tri = [r[0], r[k], r[k + 1]];
                push(
                    &mut out,
                    if facing_back {
                        [tri[0], tri[2], tri[1]]
                    } else {
                        tri
                    },
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::inplace::{add_form, create, default_kind, Form};
    use studio_core::ops;
    use studio_core::sketch::SketchCurve;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<SketchCurve> {
        let p = [
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ];
        (0..4)
            .map(|i| SketchCurve::line(p[i], p[(i + 1) % 4]))
            .collect()
    }

    /// Volume of a closed triangle soup (divergence theorem); positive when outward.
    fn volume(t: &[f32]) -> f64 {
        t.chunks(9)
            .map(|c| {
                let v = |k: usize| {
                    [
                        f64::from(c[3 * k]),
                        f64::from(c[3 * k + 1]),
                        f64::from(c[3 * k + 2]),
                    ]
                };
                let (a, b, d) = (v(0), v(1), v(2));
                (a[0] * (b[1] * d[2] - b[2] * d[1]) - a[1] * (b[0] * d[2] - b[2] * d[0])
                    + a[2] * (b[0] * d[1] - b[1] * d[0]))
                    / 6.0
            })
            .sum()
    }

    #[test]
    fn a_void_extrusion_cuts_the_solid_one() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let id = create(&mut doc, Category::GenericModel, None, l1).unwrap();
        // 2000 × 1000 × 1000 box with a 500 × 1000 notch 500 deep through its top.
        let mut solid = Form {
            kind: default_kind("Extrusion").unwrap(),
            sketch: vec![rect(0.0, 0.0, 2000.0, 1000.0)],
            void: false,
        };
        solid.kind = FormKind::Extrusion {
            start: 0.0,
            end: 1000.0,
        };
        add_form(&mut doc, id, solid).unwrap();
        let s = solid_of(&doc, id);
        assert!((volume(&s.triangles()) - 2.0e9).abs() < 1.0e6);
        add_form(
            &mut doc,
            id,
            Form {
                kind: FormKind::Extrusion {
                    start: 500.0,
                    end: 1500.0,
                },
                sketch: vec![rect(750.0, -100.0, 1250.0, 1100.0)],
                void: true,
            },
        )
        .unwrap();
        let s = solid_of(&doc, id);
        assert!((volume(&s.triangles()) - (2.0e9 - 500.0 * 1000.0 * 500.0)).abs() < 1.0e6);
        assert_eq!((s.z0, s.z1), (0.0, 1000.0));
        assert_eq!(s.category, Category::GenericModel);
    }

    fn solid_of(doc: &Document, id: ElementId) -> InPlaceSolid {
        solid(doc, id).unwrap()
    }

    #[test]
    fn a_blend_tapers_from_base_to_top_and_a_sweep_follows_its_path() {
        // A 1000-square base to a 500-square top, 1000 up: a frustum of 7/12 × 10⁹ mm³.
        let base = [
            Pt::new(0.0, 0.0),
            Pt::new(1000.0, 0.0),
            Pt::new(1000.0, 1000.0),
            Pt::new(0.0, 1000.0),
        ];
        let top: Vec<Pt> = base
            .iter()
            .map(|p| Pt::new(250.0 + p.x / 2.0, 250.0 + p.y / 2.0))
            .collect();
        let t = blend(&base, 0.0, &top, 1000.0);
        assert!((volume(&t) - 7.0e9 / 12.0).abs() < 1.0e6, "{}", volume(&t));
        // A circle to a square still closes (resampled), within a few percent.
        let circle: Vec<Pt> = (0..64)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 64.0;
                Pt::new(500.0 + 500.0 * a.cos(), 500.0 + 500.0 * a.sin())
            })
            .collect();
        let v = volume(&blend(&circle, 0.0, &base, 1000.0));
        let expect = (std::f64::consts::PI * 250_000.0 + 1.0e6) / 2.0 * 1000.0;
        assert!((v / expect - 1.0).abs() < 0.08, "{v} vs {expect}");
        // A 100 × 50 rectangle along an L, 2000 + 1000 long, mitred: 5000 × 3000 mm² × …
        // (Off the origin, so both end caps count toward the volume.)
        let path = [
            Pt::new(3000.0, 700.0),
            Pt::new(5000.0, 700.0),
            Pt::new(5000.0, 1700.0),
        ];
        let prof = SweepProfile::Rectangle {
            width: 100.0,
            height: 50.0,
        };
        let v = volume(&sweep(&path, false, 0.0, prof));
        // The centerline length times the area (a mitred corner adds nothing).
        assert!((v - 3000.0 * 100.0 * 50.0).abs() < 1.0e3, "{v}");
        // Closed around a square: 4000 of centerline.
        let sq = [
            Pt::new(0.0, 0.0),
            Pt::new(1000.0, 0.0),
            Pt::new(1000.0, 1000.0),
            Pt::new(0.0, 1000.0),
        ];
        let v = volume(&sweep(&sq, true, 0.0, prof));
        assert!((v - 4000.0 * 100.0 * 50.0).abs() < 1.0e3, "{v}");
    }
}
