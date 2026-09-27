//! The site's lot and preliminary topography, placed in the project (ADR-023).

use studio_core::site::{to_project, Topo};
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::Pt;

/// The lot and ground, in project coordinates. Heights are project z (mm above Level 1's
/// datum); `datum` is the absolute elevation of project z = 0.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteSolid {
    pub id: ElementId,
    /// Lot boundary in the project, and as surveyed (local east/north, for bearings).
    pub boundary: Vec<Pt>,
    pub boundary_local: Vec<Pt>,
    pub offset: Pt,
    pub rotation: f64,
    pub datum: f64,
    pub contour: f64,
    pub topo: Option<Topo>,
}

impl SiteSolid {
    pub(crate) fn from_doc(doc: &Document) -> Option<SiteSolid> {
        let e = doc.of(Category::Site).next()?;
        let ElementData::Site {
            boundary,
            offset,
            rotation,
            base_elevation,
            contour,
            topo,
            ..
        } = &e.data
        else {
            return None;
        };
        Some(SiteSolid {
            id: e.id,
            boundary: boundary
                .iter()
                .map(|p| to_project(*offset, *rotation, *p))
                .collect(),
            boundary_local: boundary.clone(),
            offset: *offset,
            rotation: *rotation,
            datum: *base_elevation,
            contour: *contour,
            topo: topo.clone(),
        })
    }

    /// Local site frame → project.
    pub fn place(&self, p: Pt) -> Pt {
        to_project(self.offset, self.rotation, p)
    }

    /// Project → local site frame.
    pub fn local(&self, p: Pt) -> Pt {
        let q = p.sub(self.offset);
        let (s, c) = (-self.rotation).sin_cos();
        Pt::new(q.x * c - q.y * s, q.x * s + q.y * c)
    }

    /// Ground height (project z) at a project point, if the topography covers it.
    pub fn ground_at(&self, p: Pt) -> Option<f64> {
        let t = self.topo.as_ref()?;
        t.sample(self.local(p)).map(|z| z - self.datum)
    }

    /// The ground surface as triangles (9 floats each, project mm, z-up).
    pub fn mesh(&self) -> Vec<f32> {
        let Some(t) = &self.topo else {
            return vec![];
        };
        let mut out = vec![];
        let v = |i: u32, j: u32| {
            let p = self.place(t.node(i, j));
            [p.x as f32, p.y as f32, (t.at(i, j) - self.datum) as f32]
        };
        for j in 0..t.ny.saturating_sub(1) {
            for i in 0..t.nx.saturating_sub(1) {
                let (a, b, c, d) = (v(i, j), v(i + 1, j), v(i + 1, j + 1), v(i, j + 1));
                for tri in [[a, b, c], [a, c, d]] {
                    for p in tri {
                        out.extend_from_slice(&p);
                    }
                }
            }
        }
        out
    }

    /// The lowest ground (project z), if there's topography.
    pub fn lowest(&self) -> Option<f64> {
        let t = self.topo.as_ref()?;
        let lo = t.z.iter().fold(f64::INFINITY, |a, z| a.min(f64::from(*z)));
        lo.is_finite().then_some(lo - self.datum)
    }

    /// Revit's toposolid look (ADR-045): the ground's sides straight down to `base` (project
    /// z) all round its edge, and its bottom, as triangles, so the terrain reads as a block
    /// cut out of the earth.
    pub fn skirt(&self, base: f64) -> Vec<f32> {
        let Some(t) = &self.topo else {
            return vec![];
        };
        if t.nx < 2 || t.ny < 2 {
            return vec![];
        }
        // The edge nodes, counter-clockwise from the south-west corner.
        let mut ring: Vec<(u32, u32)> = vec![];
        ring.extend((0..t.nx - 1).map(|i| (i, 0)));
        ring.extend((0..t.ny - 1).map(|j| (t.nx - 1, j)));
        ring.extend((1..t.nx).rev().map(|i| (i, t.ny - 1)));
        ring.extend((1..t.ny).rev().map(|j| (0, j)));
        let top = |(i, j): (u32, u32)| {
            let p = self.place(t.node(i, j));
            [p.x, p.y, t.at(i, j) - self.datum]
        };
        let mut out: Vec<f32> = vec![];
        let mut tri = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
            out.extend([a, b, c].iter().flat_map(|v| v.map(|x| x as f32)));
        };
        for k in 0..ring.len() {
            let (a, b) = (top(ring[k]), top(ring[(k + 1) % ring.len()]));
            let (a0, b0) = ([a[0], a[1], base], [b[0], b[1], base]);
            tri(a0, b0, b);
            tri(a0, b, a);
        }
        // The bottom, facing down.
        let corner = |i: u32, j: u32| {
            let p = self.place(t.node(i, j));
            [p.x, p.y, base]
        };
        let (sw, se, ne, nw) = (
            corner(0, 0),
            corner(t.nx - 1, 0),
            corner(t.nx - 1, t.ny - 1),
            corner(0, t.ny - 1),
        );
        tri(sw, ne, se);
        tri(sw, nw, ne);
        out
    }

    /// Contour lines: (absolute elevation mm, major?, segments in project coordinates).
    /// Every fifth interval is a major contour.
    pub fn contours(&self) -> Vec<(f64, bool, Vec<[Pt; 2]>)> {
        let Some(t) = &self.topo else {
            return vec![];
        };
        let (lo, hi) =
            t.z.iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), z| {
                    (a.min(f64::from(*z)), b.max(f64::from(*z)))
                });
        let step = self.contour.max(25.0);
        let first = (lo / step).ceil() as i64;
        let last = (hi / step).floor() as i64;
        if last - first > 400 {
            return vec![];
        }
        (first..=last)
            .map(|k| {
                let level = k as f64 * step;
                let segs = studio_geom::contour_segments(
                    t.nx as usize,
                    t.ny as usize,
                    |i, j| t.at(i as u32, j as u32),
                    level,
                )
                .into_iter()
                .map(|[a, b]| {
                    let g =
                        |q: Pt| self.place(Pt::new(t.x0 + q.x * t.spacing, t.y0 + q.y * t.spacing));
                    [g(a), g(b)]
                })
                .collect();
                (level, k.rem_euclid(5) == 0, segs)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::site::{set_lot, set_topo, topo_request, GeoFrame, ParcelInfo};
    use studio_core::units::MM_PER_FT;

    #[test]
    fn ground_mesh_and_contours_follow_the_samples() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let ring = [
            (37.7749, -122.4194),
            (37.7749, -122.41906),
            (37.77526, -122.41906),
            (37.77526, -122.4194),
        ];
        set_lot(&mut doc, &ring, ParcelInfo::default()).unwrap();
        let (nx, ny, origin, pts) = topo_request(&doc, 5.0 * MM_PER_FT, 10.0 * MM_PER_FT).unwrap();
        let frame = GeoFrame {
            lat0: ring.iter().map(|p| p.0).sum::<f64>() / 4.0,
            lon0: ring.iter().map(|p| p.1).sum::<f64>() / 4.0,
        };
        // Rises 1' for every 10' north.
        let m: Vec<Option<f64>> = pts
            .iter()
            .map(|(la, lo)| Some(30.0 + frame.to_local(*la, *lo).y / 10.0 / 1000.0))
            .collect();
        set_topo(&mut doc, (nx, ny, origin), 5.0 * MM_PER_FT, &m, 1.0).unwrap();
        let s = SiteSolid::from_doc(&doc).unwrap();
        // Level 1 is the ground at the lot's center (rounded to the inch).
        assert!(s.ground_at(Pt::default()).unwrap().abs() < 13.0);
        let north = s.ground_at(Pt::new(0.0, 10.0 * MM_PER_FT)).unwrap();
        assert!((north - 1.0 * MM_PER_FT).abs() < 15.0, "{north}");
        assert_eq!(s.mesh().len(), ((nx - 1) * (ny - 1) * 2 * 9) as usize);
        // One-foot contours running east-west, 10' apart, each major fifth.
        let c = s.contours();
        assert!(c.len() > 5);
        let (_, major, segs) = c.iter().find(|x| !x.2.is_empty()).unwrap();
        let _ = major;
        assert!(
            segs.iter().all(|[a, b]| (a.y - b.y).abs() < 1.0),
            "level lines run east-west"
        );
        assert!(c.iter().filter(|x| x.1).count() >= 1);
        // The toposolid: a side quad per edge span all round, and the bottom.
        let low = s.lowest().unwrap();
        let skirt = s.skirt(low - 10.0 * MM_PER_FT);
        let spans = 2 * ((nx - 1) + (ny - 1)) as usize;
        assert_eq!(skirt.len(), (spans * 2 + 2) * 9);
        let zmin = skirt
            .chunks(3)
            .map(|p| f64::from(p[2]))
            .fold(f64::INFINITY, f64::min);
        assert!((zmin - (low - 10.0 * MM_PER_FT)).abs() < 1.0);
        // Turning the site turns its contours and lot.
        let sid = studio_core::site::site_of(&doc).unwrap();
        studio_core::ops::set_property(&mut doc, sid, "rotation", "90", 0).unwrap();
        let s = SiteSolid::from_doc(&doc).unwrap();
        let segs = &s
            .contours()
            .into_iter()
            .find(|x| !x.2.is_empty())
            .unwrap()
            .2;
        assert!(
            segs.iter().all(|[a, b]| (a.x - b.x).abs() < 1.0),
            "now north-south"
        );
    }
}
