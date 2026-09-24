//! Columns, beams, railings and stairs (ADR-019), resolved into solids.

use studio_core::{BeamShape, ColumnShape, Document, ElementData, ElementId, StairShape, WallTop};
use studio_geom::{box_corners, Poly, Prism, Pt};

/// A vertical column: its cross-section placed in plan, from `z0` to `z1`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSolid {
    pub id: ElementId,
    pub level: ElementId,
    pub at: Pt,
    pub base: Poly,
    pub z0: f64,
    pub z1: f64,
    pub structural: bool,
}

impl ColumnSolid {
    pub fn prism(&self) -> Prism {
        Prism {
            base: self.base.clone(),
            z0: self.z0,
            z1: self.z1,
        }
    }
}

/// A beam as prisms (one for a rectangular section; flanges and web for an I-shape).
#[derive(Debug, Clone, PartialEq)]
pub struct BeamSolid {
    pub id: ElementId,
    pub level: ElementId,
    pub start: Pt,
    pub end: Pt,
    pub width: f64,
    pub z_top: f64,
    pub depth: f64,
    pub prisms: Vec<Prism>,
}

/// A railing: rails as boxes along 3D segments, balusters and posts as prisms, and its plan
/// path (at its base).
#[derive(Debug, Clone, PartialEq)]
pub struct RailSolid {
    /// The railing element, or the stair it belongs to.
    pub id: ElementId,
    pub level: ElementId,
    pub path: Vec<Pt>,
    pub boxes: Vec<[[f64; 3]; 8]>,
    pub posts: Vec<Prism>,
}

/// One flight of a stair.
#[derive(Debug, Clone, PartialEq)]
pub struct StairRun {
    /// Center of its first riser, and the climbing direction.
    pub start: Pt,
    pub dir: Pt,
    pub treads: usize,
    /// Height it climbs from (the floor or landing below its first riser).
    pub z0: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StairSolid {
    pub id: ElementId,
    pub base_level: ElementId,
    pub top_level: ElementId,
    pub width: f64,
    pub tread: f64,
    pub risers: usize,
    pub riser: f64,
    /// Height of the base level (mm).
    pub z0: f64,
    pub runs: Vec<StairRun>,
    /// Landing outlines with their top height.
    pub landings: Vec<(Vec<Pt>, f64)>,
    /// Solid blocks: one per tread, then one per landing.
    pub steps: Vec<Prism>,
    pub railings: bool,
}

impl StairRun {
    /// Plan corners (start-left, start-right, end-right, end-left) for width `w`.
    pub fn outline(&self, w: f64, tread: f64) -> [Pt; 4] {
        let n = self.dir.perp().scale(w / 2.0);
        let b = self.start.add(self.dir.scale(tread * self.treads as f64));
        [self.start.add(n), self.start.sub(n), b.sub(n), b.add(n)]
    }
}

impl StairSolid {
    /// Top of the stair: the upper level's height (mm).
    pub fn z1(&self) -> f64 {
        self.z0 + self.riser * self.risers as f64
    }

    /// Plan outlines of the runs and landings (the opening it needs in the floor above).
    pub fn footprint(&self) -> Vec<Vec<Pt>> {
        let mut out: Vec<Vec<Pt>> = self
            .runs
            .iter()
            .map(|r| r.outline(self.width, self.tread).to_vec())
            .collect();
        out.extend(self.landings.iter().map(|l| l.0.clone()));
        out
    }

    /// The walking line through the runs' centers and landings, bottom to top.
    pub fn walking_line(&self) -> Vec<Pt> {
        let mut pts = vec![];
        for r in &self.runs {
            pts.push(r.start);
            pts.push(r.start.add(r.dir.scale(self.tread * r.treads as f64)));
        }
        pts
    }
}

fn rot(p: Pt, a: f64) -> Pt {
    let (s, c) = a.sin_cos();
    Pt::new(p.x * c - p.y * s, p.x * s + p.y * c)
}

/// A column's section outline around the origin, before rotation.
pub fn column_profile(shape: &ColumnShape) -> Vec<Pt> {
    match *shape {
        ColumnShape::Rectangular { width, depth } => {
            let (x, y) = (width / 2.0, depth / 2.0);
            vec![
                Pt::new(-x, -y),
                Pt::new(x, -y),
                Pt::new(x, y),
                Pt::new(-x, y),
            ]
        }
        ColumnShape::Round { diameter } => (0..24)
            .map(|i| {
                let a = i as f64 / 24.0 * std::f64::consts::TAU;
                Pt::new(a.cos() * diameter / 2.0, a.sin() * diameter / 2.0)
            })
            .collect(),
        ColumnShape::WideFlange {
            depth,
            flange,
            flange_t,
            web_t,
        } => {
            // Strong axis along y: flanges run along x at the top and bottom.
            let (b, d, tf, tw) = (flange / 2.0, depth / 2.0, flange_t, web_t / 2.0);
            vec![
                Pt::new(-b, -d),
                Pt::new(b, -d),
                Pt::new(b, -d + tf),
                Pt::new(tw, -d + tf),
                Pt::new(tw, d - tf),
                Pt::new(b, d - tf),
                Pt::new(b, d),
                Pt::new(-b, d),
                Pt::new(-b, d - tf),
                Pt::new(-tw, d - tf),
                Pt::new(-tw, -d + tf),
                Pt::new(-b, -d + tf),
            ]
        }
    }
}

pub(crate) fn columns(doc: &Document, elev: &dyn Fn(ElementId) -> f64) -> Vec<ColumnSolid> {
    doc.of(studio_core::Category::Column)
        .filter_map(|e| {
            let ElementData::Column {
                type_id,
                base_level,
                base_offset,
                top,
                at,
                rotation,
            } = &e.data
            else {
                return None;
            };
            let ElementData::ColumnType {
                shape, structural, ..
            } = doc.data(*type_id).ok()?
            else {
                return None;
            };
            let z0 = elev(*base_level) + base_offset;
            let z1 = match top {
                WallTop::UpToLevel { level, offset } => elev(*level) + offset,
                WallTop::Unconnected { height } => z0 + height,
            };
            if z1 - z0 < 1.0 {
                return None;
            }
            let ring = column_profile(shape)
                .into_iter()
                .map(|p| at.add(rot(p, *rotation)))
                .collect();
            Some(ColumnSolid {
                id: e.id,
                level: *base_level,
                at: *at,
                base: Poly::simple(ring),
                z0,
                z1,
                structural: *structural,
            })
        })
        .collect()
}

/// A plan rectangle along a→b, `w` wide.
fn strip(a: Pt, b: Pt, w: f64) -> Poly {
    let n = b.sub(a).norm().perp().scale(w / 2.0);
    Poly::simple(vec![a.sub(n), b.sub(n), b.add(n), a.add(n)])
}

pub(crate) fn beams(doc: &Document, elev: &dyn Fn(ElementId) -> f64) -> Vec<BeamSolid> {
    doc.of(studio_core::Category::Beam)
        .filter_map(|e| {
            let ElementData::Beam {
                type_id,
                level,
                offset,
                start,
                end,
            } = &e.data
            else {
                return None;
            };
            let ElementData::BeamType { shape, .. } = doc.data(*type_id).ok()? else {
                return None;
            };
            let top = elev(*level) + offset;
            let (prisms, width, depth) = match *shape {
                BeamShape::Rectangular { width, depth } => (
                    vec![Prism {
                        base: strip(*start, *end, width),
                        z0: top - depth,
                        z1: top,
                    }],
                    width,
                    depth,
                ),
                BeamShape::WideFlange {
                    depth,
                    flange,
                    flange_t,
                    web_t,
                } => (
                    vec![
                        Prism {
                            base: strip(*start, *end, flange),
                            z0: top - flange_t,
                            z1: top,
                        },
                        Prism {
                            base: strip(*start, *end, web_t),
                            z0: top - depth + flange_t,
                            z1: top - flange_t,
                        },
                        Prism {
                            base: strip(*start, *end, flange),
                            z0: top - depth,
                            z1: top - depth + flange_t,
                        },
                    ],
                    flange,
                    depth,
                ),
            };
            Some(BeamSolid {
                id: e.id,
                level: *level,
                start: *start,
                end: *end,
                width,
                z_top: top,
                depth,
                prisms,
            })
        })
        .collect()
}

const IN: f64 = 25.4;

/// Rails (top and bottom), balusters and end posts along a flat or sloped line from `a`
/// (at height `za` of the walking surface) to `b` (`zb`). Balusters stand `spacing` apart.
fn rail_along(out: &mut RailSolid, a: Pt, za: f64, b: Pt, zb: f64, height: f64, posts: bool) {
    let len = a.dist(b);
    if len < 1.0 {
        return;
    }
    let top = |t: f64| za + (zb - za) * t + height - IN;
    out.boxes.push(box_corners(
        [a.x, a.y, top(0.0)],
        [b.x, b.y, top(1.0)],
        2.0 * IN,
        2.0 * IN,
    ));
    out.boxes.push(box_corners(
        [a.x, a.y, za + 4.0 * IN],
        [b.x, b.y, zb + 4.0 * IN],
        1.5 * IN,
        1.5 * IN,
    ));
    let spacing = 4.0 * IN;
    let n = (len / spacing).floor() as usize;
    let dir = b.sub(a).norm();
    let bal = 0.75 * IN;
    for k in 1..n {
        let t = k as f64 * spacing / len;
        let p = a.add(dir.scale(k as f64 * spacing));
        let z0 = za + (zb - za) * t + 4.0 * IN;
        let z1 = top(t) - IN;
        if z1 - z0 > 10.0 {
            out.posts.push(Prism {
                base: strip(
                    p.sub(dir.scale(bal / 2.0)),
                    p.add(dir.scale(bal / 2.0)),
                    bal,
                ),
                z0,
                z1,
            });
        }
    }
    if posts {
        for (p, z) in [(a, za), (b, zb)] {
            out.posts.push(Prism {
                base: strip(p.sub(dir.scale(IN)), p.add(dir.scale(IN)), 2.0 * IN),
                z0: z,
                z1: z + height,
            });
        }
    }
}

pub(crate) fn railings(doc: &Document, elev: &dyn Fn(ElementId) -> f64) -> Vec<RailSolid> {
    doc.of(studio_core::Category::Railing)
        .filter_map(|e| {
            let ElementData::Railing {
                type_id,
                level,
                offset,
                path,
            } = &e.data
            else {
                return None;
            };
            let ElementData::RailingType { height, .. } = doc.data(*type_id).ok()? else {
                return None;
            };
            let z = elev(*level) + offset;
            let mut r = RailSolid {
                id: e.id,
                level: *level,
                path: path.clone(),
                boxes: vec![],
                posts: vec![],
            };
            for w in path.windows(2) {
                rail_along(&mut r, w[0], z, w[1], z, *height, true);
            }
            Some(r)
        })
        .collect()
}

pub(crate) fn stairs(doc: &Document, elev: &dyn Fn(ElementId) -> f64) -> Vec<StairSolid> {
    doc.of(studio_core::Category::Stair)
        .filter_map(|e| {
            let ElementData::Stair {
                base_level,
                top_level,
                start,
                end,
                width,
                tread,
                max_riser,
                shape,
                first_run,
                railings,
            } = &e.data
            else {
                return None;
            };
            let (z0, z1) = (elev(*base_level), elev(*top_level));
            if z1 - z0 < 1.0 {
                return None;
            }
            let (risers, riser, _) = studio_core::build::stair_layout(z1 - z0, *tread, *max_riser);
            let counts = studio_core::build::stair_runs(*shape, risers, *first_run);
            let d = end.sub(*start).norm();
            let (w, t) = (*width, *tread);
            let mut runs = vec![StairRun {
                start: *start,
                dir: d,
                treads: counts[0].saturating_sub(1),
                z0,
            }];
            let mut landings = vec![];
            if counts.len() == 2 {
                let l1 = t * runs[0].treads as f64;
                let (left, u) = match shape {
                    StairShape::LShaped { left } => (*left, false),
                    StairShape::UShaped { left } => (*left, true),
                    StairShape::Straight => (true, false),
                };
                let n = if left { d.perp() } else { d.perp().scale(-1.0) };
                let zl = z0 + counts[0] as f64 * riser;
                let at = |along: f64, side: f64| start.add(d.scale(along)).add(n.scale(side));
                if u {
                    landings.push((
                        vec![
                            at(l1, -w / 2.0),
                            at(l1 + w, -w / 2.0),
                            at(l1 + w, 1.5 * w),
                            at(l1, 1.5 * w),
                        ],
                        zl,
                    ));
                    runs.push(StairRun {
                        start: at(l1, w),
                        dir: d.scale(-1.0),
                        treads: counts[1].saturating_sub(1),
                        z0: zl,
                    });
                } else {
                    landings.push((
                        vec![
                            at(l1, -w / 2.0),
                            at(l1 + w, -w / 2.0),
                            at(l1 + w, w / 2.0),
                            at(l1, w / 2.0),
                        ],
                        zl,
                    ));
                    runs.push(StairRun {
                        start: at(l1 + w / 2.0, w / 2.0),
                        dir: n,
                        treads: counts[1].saturating_sub(1),
                        z0: zl,
                    });
                }
            }
            let mut steps = vec![];
            for r in &runs {
                let n = r.dir.perp().scale(w / 2.0);
                for k in 0..r.treads {
                    let a = r.start.add(r.dir.scale(k as f64 * t));
                    let b = r.start.add(r.dir.scale((k + 1) as f64 * t));
                    steps.push(Prism {
                        base: Poly::simple(vec![a.sub(n), b.sub(n), b.add(n), a.add(n)]),
                        z0,
                        z1: r.z0 + (k + 1) as f64 * riser,
                    });
                }
            }
            for (ring, z) in &landings {
                steps.push(Prism {
                    base: Poly::simple(ring.clone()),
                    z0,
                    z1: *z,
                });
            }
            Some(StairSolid {
                id: e.id,
                base_level: *base_level,
                top_level: *top_level,
                width: w,
                tread: t,
                risers,
                riser,
                z0,
                runs,
                landings,
                steps,
                railings: *railings,
            })
        })
        .collect()
}

/// Handrails along both sides of every run of the stairs that have them, and around the
/// open edges of their landings.
pub(crate) fn stair_railings(stairs: &[StairSolid]) -> Vec<RailSolid> {
    let height = 36.0 * IN;
    let inset = 2.0 * IN;
    let mut out = vec![];
    for s in stairs.iter().filter(|s| s.railings) {
        let mut r = RailSolid {
            id: s.id,
            level: s.base_level,
            path: vec![],
            boxes: vec![],
            posts: vec![],
        };
        for run in &s.runs {
            let n = run.dir.perp();
            let len = s.tread * run.treads as f64;
            for side in [-1.0, 1.0] {
                let off = n.scale(side * (s.width / 2.0 - inset));
                let a = run.start.add(off);
                let b = run.start.add(run.dir.scale(len)).add(off);
                // Along the nosing line: from the first riser's top to the last's.
                let za = run.z0 + s.riser;
                let zb = run.z0 + s.riser * (run.treads + 1) as f64;
                rail_along(&mut r, a, za, b, zb, height, false);
            }
        }
        // Landing edges that no run attaches to.
        for (ring, z) in &s.landings {
            let m = ring.len();
            let ccw = if studio_geom::signed_area(ring) > 0.0 {
                1.0
            } else {
                -1.0
            };
            for i in 0..m {
                let (a, b) = (ring[i], ring[(i + 1) % m]);
                let mid = a.lerp(b, 0.5);
                let attached = s.runs.iter().any(|run| {
                    let o = run.outline(s.width, s.tread);
                    // A run touches this edge at its start (riser) or end.
                    [o[0].lerp(o[1], 0.5), o[2].lerp(o[3], 0.5)]
                        .iter()
                        .any(|p| {
                            p.dist(mid) < s.width * 0.6
                                && studio_geom::project_to_segment(*p, a, b).1 < 1.0
                        })
                });
                if !attached {
                    let inward = b.sub(a).norm().perp().scale(inset * ccw);
                    rail_along(&mut r, a.add(inward), *z, b.add(inward), *z, height, true);
                }
            }
        }
        out.push(r);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_flange_profile_area() {
        let p = column_profile(&ColumnShape::WideFlange {
            depth: 250.0,
            flange: 200.0,
            flange_t: 10.0,
            web_t: 8.0,
        });
        let area = studio_geom::signed_area(&p).abs();
        assert!((area - (2.0 * 200.0 * 10.0 + 230.0 * 8.0)).abs() < 1e-9);
    }
}
