//! Derived geometry: wall solids with joins, floor and ceiling slabs, room regions.
//!
//! Prototype strategy: every call regenerates the whole model. For the model sizes of
//! the prototype this takes well under a millisecond per element; dependency-graph based
//! incremental regeneration (ARCHITECTURE.md) replaces it when it becomes a bottleneck.

use studio_core::{
    Category, Document, DoorFamily, ElementData, ElementId, WallFunction, WallTop, WindowFamily,
};
use studio_geom::{clip_half_plane, line_intersection, union_all, Poly, Prism, Pt};

#[derive(Debug, Clone, PartialEq)]
pub struct WallSolid {
    pub id: ElementId,
    pub level: ElementId,
    pub start: Pt,
    pub end: Pt,
    pub thickness: f64,
    pub exterior: bool,
    /// Whole footprint, ignoring openings (used for rooms, picking and projection).
    pub footprint: Poly,
    pub z0: f64,
    pub z1: f64,
    /// The wall's solid material: the footprint split around door and window openings.
    pub pieces: Vec<Prism>,
}

impl WallSolid {
    /// Unit vector from start to end.
    pub fn dir(&self) -> Pt {
        self.end.sub(self.start).norm()
    }
}

/// What kind of opening, with its family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningKind {
    Door(DoorFamily),
    Window(WindowFamily),
}

/// A door or window resolved against its host wall. Lengths in mm; `t0`/`t1` are distances
/// from the host's start along its location line, `z0`/`z1` are absolute heights.
#[derive(Debug, Clone, PartialEq)]
pub struct OpeningSolid {
    pub id: ElementId,
    pub host: ElementId,
    pub kind: OpeningKind,
    pub wall_start: Pt,
    pub dir: Pt,
    pub half_thickness: f64,
    pub t0: f64,
    pub t1: f64,
    pub z0: f64,
    pub z1: f64,
    pub flip_hand: bool,
    pub flip_facing: bool,
}

impl OpeningSolid {
    pub fn width(&self) -> f64 {
        self.t1 - self.t0
    }
    /// Point on the location line at distance `t` from the wall start.
    pub fn at(&self, t: f64) -> Pt {
        self.wall_start.add(self.dir.scale(t))
    }
    /// A thin prism in the wall's center plane (door leaf or glass), `depth` mm thick.
    pub fn panel(&self, depth: f64, z0: f64, z1: f64) -> Prism {
        let n = self.dir.perp().scale(depth / 2.0);
        let (a, b) = (self.at(self.t0), self.at(self.t1));
        Prism {
            base: Poly::simple(vec![a.sub(n), b.sub(n), b.add(n), a.add(n)]),
            z0,
            z1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlabSolid {
    pub id: ElementId,
    pub category: Category,
    pub level: ElementId,
    pub base: Poly,
    pub z0: f64,
    pub z1: f64,
}

impl SlabSolid {
    pub fn prism(&self) -> Prism {
        Prism {
            base: self.base.clone(),
            z0: self.z0,
            z1: self.z1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridLine {
    pub id: ElementId,
    pub name: String,
    pub start: Pt,
    pub end: Pt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelInfo {
    pub id: ElementId,
    pub name: String,
    pub elevation: f64,
}

/// Everything derived from the document that views need.
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub walls: Vec<WallSolid>,
    pub floors: Vec<SlabSolid>,
    pub ceilings: Vec<SlabSolid>,
    pub grids: Vec<GridLine>,
    pub levels: Vec<LevelInfo>,
    pub openings: Vec<OpeningSolid>,
}

impl Model {
    /// Plan-space bounding box of all model geometry and grids, or None if empty.
    pub fn plan_bounds(&self) -> Option<(Pt, Pt)> {
        let mut pts: Vec<Pt> = vec![];
        for w in &self.walls {
            pts.extend(&w.footprint.outer);
        }
        for s in self.floors.iter().chain(&self.ceilings) {
            pts.extend(&s.base.outer);
        }
        for g in &self.grids {
            pts.push(g.start);
            pts.push(g.end);
        }
        bounds(&pts)
    }

    /// Lowest and highest z of model solids (mm).
    pub fn z_range(&self) -> (f64, f64) {
        let mut lo = self
            .levels
            .iter()
            .map(|l| l.elevation)
            .fold(f64::INFINITY, f64::min);
        let mut hi = self
            .levels
            .iter()
            .map(|l| l.elevation)
            .fold(f64::NEG_INFINITY, f64::max);
        for (a, b) in self.walls.iter().map(|w| (w.z0, w.z1)).chain(
            self.floors
                .iter()
                .chain(&self.ceilings)
                .map(|s| (s.z0, s.z1)),
        ) {
            lo = lo.min(a);
            hi = hi.max(b);
        }
        if !lo.is_finite() {
            (0.0, 3000.0)
        } else {
            (lo, hi)
        }
    }
}

pub fn bounds(pts: &[Pt]) -> Option<(Pt, Pt)> {
    let first = *pts.first()?;
    Some(pts.iter().fold((first, first), |(lo, hi), p| {
        (
            Pt::new(lo.x.min(p.x), lo.y.min(p.y)),
            Pt::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}

/// Rebuilds all derived geometry from the document.
pub fn regenerate(doc: &Document) -> Model {
    let elev = |id: ElementId| doc.level_elevation(id).unwrap_or(0.0);
    let mut raw = vec![];
    for e in doc.of(Category::Wall) {
        let ElementData::Wall {
            type_id,
            start,
            end,
            base_level,
            base_offset,
            top,
        } = &e.data
        else {
            continue;
        };
        let (thickness, exterior) = match doc.data(*type_id) {
            Ok(ElementData::WallType {
                thickness,
                function,
                ..
            }) => (*thickness, *function == WallFunction::Exterior),
            _ => continue,
        };
        let z0 = elev(*base_level) + base_offset;
        let z1 = match top {
            WallTop::UpToLevel { level, offset } => elev(*level) + offset,
            WallTop::Unconnected { height } => z0 + height,
        };
        if z1 - z0 < 1.0 {
            continue;
        }
        raw.push((e.id, *base_level, *start, *end, thickness, exterior, z0, z1));
    }

    let mut walls: Vec<WallSolid> = raw
        .iter()
        .map(|&(id, level, start, end, thickness, exterior, z0, z1)| {
            let h = thickness / 2.0;
            let dir = end.sub(start).norm();
            // Other walls sharing an endpoint with this end and overlapping in height.
            let partner = |p: Pt| -> Option<(Pt, f64)> {
                let mut found = raw
                    .iter()
                    .filter(|o| o.0 != id && o.6 < z1 && o.7 > z0)
                    .filter_map(|o| {
                        if o.2.dist(p) < studio_geom::tol::JOIN {
                            Some((o.3.sub(o.2).norm(), o.4 / 2.0))
                        } else if o.3.dist(p) < studio_geom::tol::JOIN {
                            Some((o.2.sub(o.3).norm(), o.4 / 2.0))
                        } else {
                            None
                        }
                    });
                let first = found.next();
                // Only clean two-wall corners are mitered; 3+ walls fall back to butt ends.
                if found.next().is_some() {
                    None
                } else {
                    first
                }
            };
            let (sl, sr) = end_corners(start, dir, h, partner(start));
            let (el, er) = end_corners(end, dir.scale(-1.0), h, partner(end));
            WallSolid {
                id,
                level,
                start,
                end,
                thickness,
                exterior,
                footprint: Poly::simple([sr, el, er, sl].map(snap).to_vec()),
                z0,
                z1,
                pieces: vec![],
            }
        })
        .collect();

    let openings = resolve_openings(doc, &walls);
    for w in &mut walls {
        let mine: Vec<&OpeningSolid> = openings.iter().filter(|o| o.host == w.id).collect();
        w.pieces = wall_pieces(w, &mine);
    }

    let slab = |cat: Category| -> Vec<SlabSolid> {
        doc.of(cat)
            .filter_map(|e| match &e.data {
                ElementData::Floor {
                    type_id,
                    level,
                    offset,
                    boundary,
                } => {
                    let t = type_thickness(doc, *type_id)?;
                    let top = elev(*level) + offset;
                    Some(SlabSolid {
                        id: e.id,
                        category: cat,
                        level: *level,
                        base: Poly::simple(boundary.clone()),
                        z0: top - t,
                        z1: top,
                    })
                }
                ElementData::Ceiling {
                    type_id,
                    level,
                    height,
                    boundary,
                } => {
                    let t = type_thickness(doc, *type_id)?;
                    let bottom = elev(*level) + height;
                    Some(SlabSolid {
                        id: e.id,
                        category: cat,
                        level: *level,
                        base: Poly::simple(boundary.clone()),
                        z0: bottom,
                        z1: bottom + t,
                    })
                }
                _ => None,
            })
            .collect()
    };

    let grids = doc
        .of(Category::Grid)
        .filter_map(|e| match &e.data {
            ElementData::Grid { name, start, end } => Some(GridLine {
                id: e.id,
                name: name.clone(),
                start: *start,
                end: *end,
            }),
            _ => None,
        })
        .collect();

    let levels = doc
        .levels()
        .into_iter()
        .map(|(id, name, elevation)| LevelInfo {
            id,
            name,
            elevation,
        })
        .collect();

    Model {
        openings,
        walls,
        floors: slab(Category::Floor),
        ceilings: slab(Category::Ceiling),
        grids,
        levels,
    }
}

/// Rounds to a 0.0001 mm grid so corners shared by two walls, computed independently,
/// are bit-identical and polygon union merges them.
fn snap(p: Pt) -> Pt {
    const Q: f64 = 1e4;
    Pt::new((p.x * Q).round() / Q, (p.y * Q).round() / Q)
}

fn resolve_openings(doc: &Document, walls: &[WallSolid]) -> Vec<OpeningSolid> {
    let get = |id: ElementId| doc.get(id).map(|e| &e.data);
    let mut out = vec![];
    for e in doc.iter() {
        let Some(fit) = studio_core::hosting::opening_fit(&get, &e.data) else {
            continue;
        };
        let Some(w) = walls.iter().find(|w| w.id == fit.host) else {
            continue;
        };
        let (kind, flip_hand, flip_facing) = match (&e.data, e.data.type_id().and_then(&get)) {
            (
                ElementData::Door {
                    flip_hand,
                    flip_facing,
                    ..
                },
                Some(ElementData::DoorType { family, .. }),
            ) => (OpeningKind::Door(*family), *flip_hand, *flip_facing),
            (
                ElementData::Window { flip_facing, .. },
                Some(ElementData::WindowType { family, .. }),
            ) => (OpeningKind::Window(*family), false, *flip_facing),
            _ => continue,
        };
        out.push(OpeningSolid {
            id: e.id,
            host: w.id,
            kind,
            wall_start: w.start,
            dir: w.dir(),
            half_thickness: w.thickness / 2.0,
            t0: fit.t0,
            t1: fit.t1,
            z0: w.z0 + fit.z0,
            z1: w.z0 + fit.z1,
            flip_hand,
            flip_facing,
        });
    }
    out
}

/// Splits a wall's footprint along its length at each opening: full-height pieces between
/// openings, and sill / head pieces below and above each opening.
fn wall_pieces(w: &WallSolid, openings: &[&OpeningSolid]) -> Vec<Prism> {
    let mut ops: Vec<&&OpeningSolid> = openings.iter().collect();
    ops.sort_by(|a, b| a.t0.total_cmp(&b.t0));
    let dir = w.dir();
    let slice = |a: Option<f64>, b: Option<f64>| -> Poly {
        let mut ring = w.footprint.outer.clone();
        if let Some(a) = a {
            ring = clip_half_plane(&ring, w.start.add(dir.scale(a)), dir);
        }
        if let Some(b) = b {
            ring = clip_half_plane(&ring, w.start.add(dir.scale(b)), dir.scale(-1.0));
        }
        Poly::simple(ring)
    };
    let mut pieces = vec![];
    let mut cursor: Option<f64> = None;
    let mut push = |base: Poly, z0: f64, z1: f64| {
        if base.outer.len() >= 3 && base.area() > 1.0 && z1 - z0 > 1.0 {
            pieces.push(Prism { base, z0, z1 });
        }
    };
    for o in &ops {
        push(slice(cursor, Some(o.t0)), w.z0, w.z1);
        let gap = slice(Some(o.t0), Some(o.t1));
        push(gap.clone(), w.z0, o.z0);
        push(gap, o.z1, w.z1);
        cursor = Some(o.t1);
    }
    push(slice(cursor, None), w.z0, w.z1);
    pieces
}

fn type_thickness(doc: &Document, id: ElementId) -> Option<f64> {
    match doc.data(id).ok()? {
        ElementData::FloorType { thickness, .. } | ElementData::CeilingType { thickness, .. } => {
            Some(*thickness)
        }
        _ => None,
    }
}

/// Footprint corners at a wall end located at `p`, where `u` points from `p` into the wall.
/// Returns (left, right) relative to `u`. With a partner wall (its direction away from `p`
/// and half-thickness) the corners are mitered to meet the partner's faces.
fn end_corners(p: Pt, u: Pt, h: f64, partner: Option<(Pt, f64)>) -> (Pt, Pt) {
    let n = u.perp();
    let butt = (p.add(n.scale(h)), p.sub(n.scale(h)));
    let Some((u2, h2)) = partner else { return butt };
    if u.cross(u2).abs() < 1e-6 {
        return butt;
    }
    let n2 = u2.perp();
    let left = line_intersection(p.add(n.scale(h)), u, p.sub(n2.scale(h2)), u2);
    let right = line_intersection(p.sub(n.scale(h)), u, p.add(n2.scale(h2)), u2);
    let limit = 4.0 * h.max(h2);
    match (left, right) {
        (Some(l), Some(r)) if l.dist(p) <= limit && r.dist(p) <= limit => (l, r),
        _ => butt,
    }
}

/// Union of the footprints of walls based on `level`.
pub fn wall_regions(model: &Model, level: ElementId) -> Vec<Poly> {
    let polys: Vec<Poly> = model
        .walls
        .iter()
        .filter(|w| w.level == level)
        .map(|w| w.footprint.clone())
        .collect();
    union_all(&polys)
}

/// Outer boundary of the largest group of walls on `level` (exterior faces), for
/// "floor by picking walls".
pub fn outer_boundary(model: &Model, level: ElementId) -> Option<Vec<Pt>> {
    wall_regions(model, level)
        .into_iter()
        .max_by(|a, b| {
            studio_geom::signed_area(&a.outer)
                .abs()
                .total_cmp(&studio_geom::signed_area(&b.outer).abs())
        })
        .map(|p| p.outer)
}

/// The enclosed room region (inside wall faces) containing `pt` on `level`.
pub fn room_at(model: &Model, level: ElementId, pt: Pt) -> Option<Vec<Pt>> {
    let mut best: Option<Vec<Pt>> = None;
    for region in wall_regions(model, level) {
        for hole in region.holes {
            if studio_geom::point_in_ring(pt, &hole) {
                let smaller = best.as_ref().is_none_or(|b| {
                    studio_geom::signed_area(&hole).abs() < studio_geom::signed_area(b).abs()
                });
                if smaller {
                    best = Some(hole);
                }
            }
        }
    }
    best
}

/// Crate version from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ops;
    use studio_core::units::{MM_PER_FT, MM_PER_IN};

    const EPS: f64 = 1e-6;

    fn project() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .map(|e| e.id)
            .unwrap();
        (doc, l1, wt)
    }

    /// 40' × 30' rectangle of exterior walls (centerlines).
    fn rectangle(doc: &mut Document, l1: ElementId, wt: ElementId) -> Vec<ElementId> {
        let (w, h) = (40.0 * MM_PER_FT, 30.0 * MM_PER_FT);
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ];
        (0..4)
            .map(|i| ops::create_wall(doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
            .collect()
    }

    #[test]
    fn l_corner_is_mitered_to_exact_face_intersections() {
        let (mut doc, l1, wt) = project();
        let a =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).unwrap();
        ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(0.0, 4000.0)).unwrap();
        let m = regenerate(&doc);
        let wa = m.walls.iter().find(|w| w.id == a).unwrap();
        let h = 4.0 * MM_PER_IN;
        // Outer corner (-h, -h) and inner corner (h, h) at the shared start point.
        let has = |p: Pt| wa.footprint.outer.iter().any(|q| q.dist(p) < EPS);
        assert!(has(Pt::new(-h, -h)), "{:?}", wa.footprint.outer);
        assert!(has(Pt::new(h, h)));
        // Free end is square.
        assert!(has(Pt::new(5000.0, h)) && has(Pt::new(5000.0, -h)));
    }

    #[test]
    fn rectangle_of_walls_encloses_one_room() {
        let (mut doc, l1, wt) = project();
        rectangle(&mut doc, l1, wt);
        let m = regenerate(&doc);
        let regions = wall_regions(&m, l1);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].holes.len(), 1);
        let t = 8.0 * MM_PER_IN;
        let room = room_at(&m, l1, Pt::new(1000.0, 1000.0)).unwrap();
        let expected = (40.0 * MM_PER_FT - t) * (30.0 * MM_PER_FT - t);
        // Union grows inputs by tol::LINEAR, so allow perimeter × 0.01 mm (< 0.01 SF).
        assert!((studio_geom::signed_area(&room).abs() - expected).abs() < 1000.0);
        let outer = outer_boundary(&m, l1).unwrap();
        let expected_outer = (40.0 * MM_PER_FT + t) * (30.0 * MM_PER_FT + t);
        assert!((studio_geom::signed_area(&outer).abs() - expected_outer).abs() < 1000.0);
        assert!(room_at(&m, l1, Pt::new(-5000.0, 0.0)).is_none());
    }

    #[test]
    fn level_elevation_change_updates_wall_height_and_undo_restores_it() {
        let (mut doc, l1, wt) = project();
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).unwrap();
        let height = |doc: &Document| {
            let m = regenerate(doc);
            let s = m.walls.iter().find(|s| s.id == w).unwrap();
            s.z1 - s.z0
        };
        assert!((height(&doc) - 10.0 * MM_PER_FT).abs() < EPS);
        let l2 = doc.levels()[1].0;
        ops::set_property(&mut doc, l2, "elevation", "12'-6\"", 0).unwrap();
        assert!((height(&doc) - 12.5 * MM_PER_FT).abs() < EPS);
        doc.undo().unwrap();
        assert!((height(&doc) - 10.0 * MM_PER_FT).abs() < EPS);
    }

    #[test]
    fn type_thickness_change_updates_footprint() {
        let (mut doc, l1, wt) = project();
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).unwrap();
        let area = |doc: &Document| {
            regenerate(doc)
                .walls
                .iter()
                .find(|s| s.id == w)
                .unwrap()
                .footprint
                .area()
        };
        assert!((area(&doc) - 5000.0 * 8.0 * MM_PER_IN).abs() < 1e-3);
        ops::set_property(&mut doc, wt, "thickness", "12\"", 0).unwrap();
        assert!((area(&doc) - 5000.0 * 12.0 * MM_PER_IN).abs() < 1e-3);
        doc.undo().unwrap();
        assert!((area(&doc) - 5000.0 * 8.0 * MM_PER_IN).abs() < 1e-3);
    }

    #[test]
    fn door_splits_wall_and_window_keeps_sill_and_head() {
        let (mut doc, l1, wt) = project();
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).unwrap();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let wn = doc
            .of(Category::WindowType)
            .find(|e| e.data.name().starts_with("Fixed 48"))
            .unwrap()
            .id;
        ops::create_door(&mut doc, dt, w, 1000.0, false).unwrap();
        ops::create_window(&mut doc, wn, w, 3500.0, false).unwrap();
        let m = regenerate(&doc);
        let wall = m.walls.iter().find(|s| s.id == w).unwrap();
        // Solid | door head | solid | window sill | window head | solid.
        assert_eq!(wall.pieces.len(), 6, "{:#?}", wall.pieces);
        let cut = 48.0 * MM_PER_IN;
        let at_cut: Vec<_> = wall
            .pieces
            .iter()
            .filter(|p| p.z0 <= cut && p.z1 > cut)
            .collect();
        // At the plan cut height both openings are gaps: three solid pieces remain.
        assert_eq!(at_cut.len(), 3);
        let door = m
            .openings
            .iter()
            .find(|o| matches!(o.kind, OpeningKind::Door(_)))
            .unwrap();
        assert!((door.width() - 36.0 * MM_PER_IN).abs() < EPS);
        assert!((door.z1 - 84.0 * MM_PER_IN).abs() < EPS);
        let win = m
            .openings
            .iter()
            .find(|o| matches!(o.kind, OpeningKind::Window(_)))
            .unwrap();
        assert!((win.z0 - 36.0 * MM_PER_IN).abs() < EPS);
        // Material volume = wall minus both openings.
        let vol: f64 = wall
            .pieces
            .iter()
            .map(|p| p.base.area() * (p.z1 - p.z0))
            .sum();
        let t = 8.0 * MM_PER_IN;
        let full = 5000.0 * t * 10.0 * MM_PER_FT;
        let holes = t * (36.0 * 84.0 + 48.0 * 48.0) * MM_PER_IN * MM_PER_IN;
        assert!((vol - (full - holes)).abs() / full < 1e-9);
    }

    #[test]
    fn floor_and_ceiling_heights() {
        let (mut doc, l1, wt) = project();
        rectangle(&mut doc, l1, wt);
        let m = regenerate(&doc);
        let boundary = outer_boundary(&m, l1).unwrap();
        let ft = ops::first_of(&doc, Category::FloorType).unwrap();
        let ct = ops::first_of(&doc, Category::CeilingType).unwrap();
        ops::create_floor(&mut doc, ft, l1, boundary).unwrap();
        let room = room_at(&m, l1, Pt::new(1000.0, 1000.0)).unwrap();
        ops::create_ceiling(&mut doc, ct, l1, room).unwrap();
        let m = regenerate(&doc);
        assert!((m.floors[0].z1 - 0.0).abs() < EPS);
        assert!(m.floors[0].z0 < 0.0);
        assert!((m.ceilings[0].z0 - 9.0 * MM_PER_FT).abs() < EPS);
    }
}
