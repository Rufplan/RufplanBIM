//! Derived geometry: wall solids with joins, openings, floor, ceiling and roof solids,
//! stairs and room regions.
//!
//! Regeneration is incremental (ADR-017): each expensive step — a wall's footprint, its
//! pieces around openings, a level's room regions — is memoized on exactly the inputs it
//! depends on, so an edit recomputes only what it affects (the moved wall and the walls
//! joined to it, the rooms on its level). Results are cached by the document's content
//! stamp, so repeated calls for the same state (every mouse move's snap and pick) are free.

pub mod roof;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use studio_core::{
    Category, Document, DoorFamily, ElementData, ElementId, WallFunction, WallTop, WindowFamily,
};
use studio_geom::{clip_half_plane, line_intersection, union_all, Poly, Prism, Pt};

pub use roof::{RoofFace, RoofSolid};

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
    /// Offsets of the boundaries between layers from the location line (mm, positive =
    /// exterior, the wall's left). Empty for a single-layer type.
    pub layers: Vec<f64>,
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

/// A straight stair run resolved into steps.
#[derive(Debug, Clone, PartialEq)]
pub struct StairSolid {
    pub id: ElementId,
    pub base_level: ElementId,
    pub top_level: ElementId,
    /// Center of the first riser, and the unit climbing direction.
    pub start: Pt,
    pub dir: Pt,
    pub width: f64,
    pub tread: f64,
    pub risers: usize,
    pub riser: f64,
    /// Height of the base level (mm).
    pub z0: f64,
    /// One solid block per tread, from the base level up to the tread's top.
    pub steps: Vec<Prism>,
}

impl StairSolid {
    /// Plan outline corners of the run (start-left, start-right, end-right, end-left).
    pub fn outline(&self) -> [Pt; 4] {
        let n = self.dir.perp().scale(self.width / 2.0);
        let run = self.dir.scale(self.tread * self.steps.len() as f64);
        let (a, b) = (self.start, self.start.add(run));
        [a.add(n), a.sub(n), b.sub(n), b.add(n)]
    }
    /// Top of the stair: the upper level's height (mm).
    pub fn z1(&self) -> f64 {
        self.z0 + self.riser * self.risers as f64
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

/// A room with its derived boundary: the inside faces of the walls enclosing its point.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomInfo {
    pub id: ElementId,
    pub level: ElementId,
    pub name: String,
    pub number: String,
    pub point: Pt,
    /// None when the point isn't enclosed by walls ("Not Enclosed").
    pub boundary: Option<Vec<Pt>>,
}

impl RoomInfo {
    /// Area in mm², zero when not enclosed.
    pub fn area(&self) -> f64 {
        self.boundary
            .as_deref()
            .map_or(0.0, |b| studio_geom::signed_area(b).abs())
    }
    /// Perimeter in mm, zero when not enclosed.
    pub fn perimeter(&self) -> f64 {
        self.boundary.as_deref().map_or(0.0, |b| {
            (0..b.len()).map(|i| b[i].dist(b[(i + 1) % b.len()])).sum()
        })
    }
}

/// Everything derived from the document that views need.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model {
    pub walls: Vec<WallSolid>,
    pub floors: Vec<SlabSolid>,
    pub ceilings: Vec<SlabSolid>,
    pub roofs: Vec<RoofSolid>,
    pub stairs: Vec<StairSolid>,
    pub grids: Vec<GridLine>,
    pub levels: Vec<LevelInfo>,
    pub openings: Vec<OpeningSolid>,
    pub rooms: Vec<RoomInfo>,
    /// Union of the wall footprints on each level that has walls (room regions are the
    /// holes).
    pub regions: HashMap<ElementId, Vec<Poly>>,
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
        for r in &self.roofs {
            pts.extend(&r.boundary);
        }
        for s in &self.stairs {
            pts.extend(s.outline());
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
        for (a, b) in self
            .walls
            .iter()
            .map(|w| (w.z0, w.z1))
            .chain(
                self.floors
                    .iter()
                    .chain(&self.ceilings)
                    .map(|s| (s.z0, s.z1)),
            )
            .chain(self.roofs.iter().map(|r| (r.base, r.peak())))
            .chain(self.stairs.iter().map(|s| (s.z0, s.z1())))
        {
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

// ---- Memoized steps ------------------------------------------------------------------

/// A wall end's miter partner: its direction away from the shared point and half width.
type Partner = Option<(Pt, f64)>;

/// Everything a wall's footprint depends on.
#[derive(Debug, Clone, PartialEq)]
struct FootprintKey {
    start: Pt,
    end: Pt,
    thickness: f64,
    at_start: Partner,
    at_end: Partner,
}

/// Everything a wall's pieces depend on.
#[derive(Debug, Clone, PartialEq)]
struct PiecesKey {
    footprint: Poly,
    z0: f64,
    z1: f64,
    openings: Vec<OpeningSolid>,
}

/// A level's walls (id, footprint, in order) and the union of those footprints.
type RegionMemo = (Vec<(ElementId, Poly)>, Vec<Poly>);

/// Remembered results of the expensive steps, keyed by what they depend on.
#[derive(Debug, Default, Clone)]
struct Memo {
    footprints: HashMap<ElementId, (FootprintKey, Poly)>,
    pieces: HashMap<ElementId, (PiecesKey, Vec<Prism>)>,
    /// Per level: its walls' (id, footprint) in order, and their union.
    regions: HashMap<ElementId, RegionMemo>,
}

/// Counts of recomputed steps in the last regeneration, for tests and diagnostics.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RegenStats {
    pub footprints: usize,
    pub pieces: usize,
    pub regions: usize,
}

struct CacheEntry {
    stamp: u64,
    model: Arc<Model>,
    memo: Arc<Memo>,
    stats: RegenStats,
}

/// A document's recent regenerations, newest last (a few, to cover undo and redo).
#[derive(Default)]
struct RegenCache(Vec<Arc<CacheEntry>>);
const CACHE_SIZE: usize = 4;
const CACHE_KEY: &str = "studio-regen";

/// The model for the document's current content, from its cache when unchanged and
/// otherwise rebuilt incrementally from its most recent regeneration.
pub fn regenerate(doc: &Document) -> Arc<Model> {
    regenerate_with_stats(doc).0
}

/// Like [`regenerate`], also returning how much work was redone.
pub fn regenerate_with_stats(doc: &Document) -> (Arc<Model>, RegenStats) {
    let stamp = doc.stamp();
    let cache = doc
        .derived()
        .get::<Mutex<RegenCache>>(CACHE_KEY)
        .unwrap_or_else(|| {
            let c = Arc::new(Mutex::new(RegenCache::default()));
            doc.derived().put(CACHE_KEY, c.clone());
            c
        });
    let prev = {
        let c = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(e) = c.0.iter().find(|e| e.stamp == stamp) {
            return (e.model.clone(), e.stats);
        }
        c.0.last().map(|e| e.memo.clone())
    };
    let mut memo = prev.map(|m| (*m).clone()).unwrap_or_default();
    let mut stats = RegenStats::default();
    let model = Arc::new(build(doc, &mut memo, &mut stats));
    let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
    c.0.push(Arc::new(CacheEntry {
        stamp,
        model: model.clone(),
        memo: Arc::new(memo),
        stats,
    }));
    if c.0.len() > CACHE_SIZE {
        c.0.remove(0);
    }
    (model, stats)
}

/// Rebuilds everything from scratch, ignoring the cache (for tests and verification).
pub fn regenerate_full(doc: &Document) -> Model {
    build(doc, &mut Memo::default(), &mut RegenStats::default())
}

struct RawWall {
    id: ElementId,
    level: ElementId,
    start: Pt,
    end: Pt,
    thickness: f64,
    exterior: bool,
    layers: Vec<f64>,
    z0: f64,
    z1: f64,
}

fn build(doc: &Document, memo: &mut Memo, stats: &mut RegenStats) -> Model {
    let elev = |id: ElementId| doc.level_elevation(id).unwrap_or(0.0);
    let mut raw: Vec<RawWall> = vec![];
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
        let (thickness, exterior, layers) = match doc.data(*type_id) {
            Ok(ElementData::WallType {
                thickness,
                function,
                layers,
                ..
            }) => (
                *thickness,
                *function == WallFunction::Exterior,
                studio_core::compound::layer_boundaries(layers, *thickness),
            ),
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
        raw.push(RawWall {
            id: e.id,
            level: *base_level,
            start: *start,
            end: *end,
            thickness,
            exterior,
            layers,
            z0,
            z1,
        });
    }

    // Other walls sharing an endpoint with `p` and overlapping `w` in height. Only clean
    // two-wall corners are mitered; three or more walls fall back to butt ends.
    let partner = |w: &RawWall, p: Pt| -> Partner {
        let mut found = raw
            .iter()
            .filter(|o| o.id != w.id && o.z0 < w.z1 && o.z1 > w.z0)
            .filter_map(|o| {
                if o.start.dist(p) < studio_geom::tol::JOIN {
                    Some((o.end.sub(o.start).norm(), o.thickness / 2.0))
                } else if o.end.dist(p) < studio_geom::tol::JOIN {
                    Some((o.start.sub(o.end).norm(), o.thickness / 2.0))
                } else {
                    None
                }
            });
        let first = found.next();
        if found.next().is_some() {
            None
        } else {
            first
        }
    };

    let mut footprints = HashMap::new();
    let mut walls: Vec<WallSolid> = raw
        .iter()
        .map(|w| {
            let key = FootprintKey {
                start: w.start,
                end: w.end,
                thickness: w.thickness,
                at_start: partner(w, w.start),
                at_end: partner(w, w.end),
            };
            let footprint = match memo.footprints.get(&w.id) {
                Some((k, poly)) if *k == key => poly.clone(),
                _ => {
                    stats.footprints += 1;
                    footprint_of(&key)
                }
            };
            footprints.insert(w.id, (key, footprint.clone()));
            WallSolid {
                id: w.id,
                level: w.level,
                start: w.start,
                end: w.end,
                thickness: w.thickness,
                exterior: w.exterior,
                footprint,
                z0: w.z0,
                z1: w.z1,
                pieces: vec![],
                layers: w.layers.clone(),
            }
        })
        .collect();
    memo.footprints = footprints;

    let openings = resolve_openings(doc, &walls);
    let mut pieces_memo = HashMap::new();
    for w in &mut walls {
        let key = PiecesKey {
            footprint: w.footprint.clone(),
            z0: w.z0,
            z1: w.z1,
            openings: openings
                .iter()
                .filter(|o| o.host == w.id)
                .cloned()
                .collect(),
        };
        w.pieces = match memo.pieces.get(&w.id) {
            Some((k, p)) if *k == key => p.clone(),
            _ => {
                stats.pieces += 1;
                let mine: Vec<&OpeningSolid> = key.openings.iter().collect();
                wall_pieces(w, &mine)
            }
        };
        pieces_memo.insert(w.id, (key, w.pieces.clone()));
    }
    memo.pieces = pieces_memo;

    let mut regions = HashMap::new();
    let mut regions_memo = HashMap::new();
    let mut level_ids: Vec<ElementId> = walls.iter().map(|w| w.level).collect();
    level_ids.sort();
    level_ids.dedup();
    for level in level_ids {
        let key: Vec<(ElementId, Poly)> = walls
            .iter()
            .filter(|w| w.level == level)
            .map(|w| (w.id, w.footprint.clone()))
            .collect();
        let union = match memo.regions.get(&level) {
            Some((k, u)) if *k == key => u.clone(),
            _ => {
                stats.regions += 1;
                union_all(&key.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>())
            }
        };
        regions.insert(level, union.clone());
        regions_memo.insert(level, (key, union));
    }
    memo.regions = regions_memo;

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

    let roofs = doc
        .of(Category::Roof)
        .filter_map(|e| match &e.data {
            ElementData::Roof {
                type_id,
                level,
                offset,
                boundary,
                slope,
                sloped,
            } => {
                let thickness = match doc.data(*type_id).ok()? {
                    ElementData::RoofType { thickness, .. } => *thickness,
                    _ => return None,
                };
                Some(RoofSolid::build(
                    e.id,
                    *level,
                    boundary,
                    elev(*level) + offset,
                    *slope,
                    sloped,
                    thickness,
                ))
            }
            _ => None,
        })
        .collect();

    let stairs = doc
        .of(Category::Stair)
        .filter_map(|e| match &e.data {
            ElementData::Stair {
                base_level,
                top_level,
                start,
                end,
                width,
                tread,
                max_riser,
            } => {
                let (z0, z1) = (elev(*base_level), elev(*top_level));
                if z1 - z0 < 1.0 {
                    return None;
                }
                let (risers, riser, _) =
                    studio_core::build::stair_layout(z1 - z0, *tread, *max_riser);
                let dir = end.sub(*start).norm();
                let n = dir.perp().scale(width / 2.0);
                let steps = (0..risers.saturating_sub(1))
                    .map(|k| {
                        let a = start.add(dir.scale(k as f64 * tread));
                        let b = start.add(dir.scale((k + 1) as f64 * tread));
                        Prism {
                            base: Poly::simple(vec![a.sub(n), b.sub(n), b.add(n), a.add(n)]),
                            z0,
                            z1: z0 + (k + 1) as f64 * riser,
                        }
                    })
                    .collect();
                Some(StairSolid {
                    id: e.id,
                    base_level: *base_level,
                    top_level: *top_level,
                    start: *start,
                    dir,
                    width: *width,
                    tread: *tread,
                    risers,
                    riser,
                    z0,
                    steps,
                })
            }
            _ => None,
        })
        .collect();

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

    let mut model = Model {
        rooms: vec![],
        openings,
        walls,
        floors: slab(Category::Floor),
        ceilings: slab(Category::Ceiling),
        roofs,
        stairs,
        grids,
        levels,
        regions,
    };
    model.rooms = resolve_rooms(doc, &model);
    model
}

fn footprint_of(k: &FootprintKey) -> Poly {
    let h = k.thickness / 2.0;
    let dir = k.end.sub(k.start).norm();
    let (sl, sr) = end_corners(k.start, dir, h, k.at_start);
    let (el, er) = end_corners(k.end, dir.scale(-1.0), h, k.at_end);
    Poly::simple([sr, el, er, sl].map(snap).to_vec())
}

/// Rounds to a 0.0001 mm grid so corners shared by two walls, computed independently,
/// are bit-identical and polygon union merges them.
fn snap(p: Pt) -> Pt {
    const Q: f64 = 1e4;
    Pt::new((p.x * Q).round() / Q, (p.y * Q).round() / Q)
}

fn resolve_openings(doc: &Document, walls: &[WallSolid]) -> Vec<OpeningSolid> {
    let get = |id: ElementId| doc.get(id).map(|e| &e.data);
    let by_id: HashMap<ElementId, &WallSolid> = walls.iter().map(|w| (w.id, w)).collect();
    let mut out = vec![];
    for e in doc.of(Category::Door).chain(doc.of(Category::Window)) {
        let Some(fit) = studio_core::hosting::opening_fit(&get, &e.data) else {
            continue;
        };
        let Some(w) = by_id.get(&fit.host) else {
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
    // Id order, as a single pass over all elements would give.
    out.sort_by_key(|o| o.id);
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

fn resolve_rooms(doc: &Document, model: &Model) -> Vec<RoomInfo> {
    doc.of(Category::Room)
        .filter_map(|e| match &e.data {
            ElementData::Room {
                level,
                point,
                name,
                number,
            } => Some(RoomInfo {
                id: e.id,
                level: *level,
                name: name.clone(),
                number: number.clone(),
                point: *point,
                boundary: model
                    .regions
                    .get(level)
                    .and_then(|regs| hole_containing(regs, *point)),
            }),
            _ => None,
        })
        .collect()
}

/// The smallest hole (enclosed area inside wall faces) that contains `pt`.
fn hole_containing(regions: &[Poly], pt: Pt) -> Option<Vec<Pt>> {
    regions
        .iter()
        .flat_map(|r| r.holes.iter())
        .filter(|h| studio_geom::point_in_ring(pt, h))
        .min_by(|a, b| {
            studio_geom::signed_area(a)
                .abs()
                .total_cmp(&studio_geom::signed_area(b).abs())
        })
        .cloned()
}

/// The existing room, if any, whose enclosed area contains `pt` on `level`.
pub fn room_occupying(model: &Model, level: ElementId, pt: Pt) -> Option<&RoomInfo> {
    let area = room_at(model, level, pt)?;
    model
        .rooms
        .iter()
        .filter(|r| r.level == level)
        .find(|r| studio_geom::point_in_ring(r.point, &area))
}

/// Union of the footprints of walls based on `level`.
pub fn wall_regions(model: &Model, level: ElementId) -> Vec<Poly> {
    model.regions.get(&level).cloned().unwrap_or_default()
}

/// Outer boundary of the largest group of walls on `level` (exterior faces), for
/// "floor by picking walls" and "roof by footprint".
pub fn outer_boundary(model: &Model, level: ElementId) -> Option<Vec<Pt>> {
    model
        .regions
        .get(&level)?
        .iter()
        .max_by(|a, b| {
            studio_geom::signed_area(&a.outer)
                .abs()
                .total_cmp(&studio_geom::signed_area(&b.outer).abs())
        })
        .map(|p| p.outer.clone())
}

/// The enclosed room region (inside wall faces) containing `pt` on `level`.
pub fn room_at(model: &Model, level: ElementId, pt: Pt) -> Option<Vec<Pt>> {
    hole_containing(model.regions.get(&level)?, pt)
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
    fn room_area_follows_walls_and_reports_not_enclosed() {
        let (mut doc, l1, wt) = project();
        let walls = rectangle(&mut doc, l1, wt);
        let r = ops::create_room(&mut doc, l1, Pt::new(1000.0, 1000.0)).unwrap();
        let area = |doc: &Document| {
            regenerate(doc)
                .rooms
                .iter()
                .find(|x| x.id == r)
                .unwrap()
                .area()
        };
        let t = 8.0 * MM_PER_IN;
        let inner = |w: f64, h: f64| (w * MM_PER_FT - t) * (h * MM_PER_FT - t);
        assert!((area(&doc) - inner(40.0, 30.0)).abs() < 1000.0);
        // Moving the east wall 5' out grows the room (acceptance: moving a wall updates areas).
        studio_core::modify::move_elements(&mut doc, &[walls[1]], Pt::new(5.0 * MM_PER_FT, 0.0))
            .unwrap();
        assert!((area(&doc) - inner(45.0, 30.0)).abs() < 1000.0);
        assert_eq!(
            room_occupying(&regenerate(&doc), l1, Pt::new(5000.0, 2000.0)).map(|x| x.id),
            Some(r)
        );
        // Deleting a wall opens the room.
        ops::delete(&mut doc, &[walls[2]]).unwrap();
        let m = regenerate(&doc);
        let info = m.rooms.iter().find(|x| x.id == r).unwrap();
        assert!(info.boundary.is_none());
        assert_eq!(info.area(), 0.0);
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

    /// xorshift64*: deterministic pseudo-random numbers for property tests.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
        fn range(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (self.next() % 10_000) as f64 / 10_000.0 * (hi - lo)
        }
    }

    /// Property: after any sequence of edits, undos and redos, the incremental model
    /// equals a model rebuilt from scratch.
    #[test]
    fn incremental_regeneration_matches_full_rebuild() {
        for seed in 1..=12u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let (mut doc, l1, wt) = project();
            let walls = rectangle(&mut doc, l1, wt);
            ops::create_room(&mut doc, l1, Pt::new(1000.0, 1000.0)).unwrap();
            let dt = ops::first_of(&doc, Category::DoorType).unwrap();
            ops::create_door(&mut doc, dt, walls[0], 3000.0, false).unwrap();
            for step in 0..25 {
                let all: Vec<ElementId> = doc.of(Category::Wall).map(|e| e.id).collect();
                let pick = |rng: &mut Rng| all[rng.below(all.len())];
                let _ = match rng.below(8) {
                    0 => {
                        let a = Pt::new(rng.range(-3000.0, 15000.0), rng.range(-3000.0, 12000.0));
                        let b = Pt::new(rng.range(-3000.0, 15000.0), rng.range(-3000.0, 12000.0));
                        ops::create_wall(&mut doc, wt, l1, a, b).map(|_| ())
                    }
                    1 => studio_core::modify::move_elements(
                        &mut doc,
                        &[pick(&mut rng)],
                        Pt::new(rng.range(-900.0, 900.0), rng.range(-900.0, 900.0)),
                    ),
                    2 if all.len() > 2 => ops::delete(&mut doc, &[pick(&mut rng)]).map(|_| ()),
                    3 => ops::set_property(&mut doc, wt, "thickness", "10\"", 0),
                    4 => doc.undo().map(|_| ()),
                    5 => doc.redo().map(|_| ()),
                    6 => ops::set_property(&mut doc, pick(&mut rng), "length", "14'", 0),
                    _ => ops::create_room(
                        &mut doc,
                        l1,
                        Pt::new(rng.range(0.0, 12000.0), rng.range(0.0, 9000.0)),
                    )
                    .map(|_| ()),
                };
                let inc = regenerate(&doc);
                let full = regenerate_full(&doc);
                assert!(*inc == full, "seed {seed}, step {step}: incremental ≠ full");
            }
        }
    }

    #[test]
    fn unchanged_documents_come_from_cache_and_edits_redo_little() {
        let (mut doc, l1, wt) = project();
        let walls = rectangle(&mut doc, l1, wt);
        // A separate wall on another part of the level, not joined to the rectangle.
        let far = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(30000.0, 0.0),
            Pt::new(35000.0, 0.0),
        )
        .unwrap();
        let (a, _) = regenerate_with_stats(&doc);
        let (b, again) = regenerate_with_stats(&doc);
        assert!(Arc::ptr_eq(&a, &b), "same stamp → cached model");
        assert!(again.footprints <= 5);
        // Moving the free-standing wall recomputes only its own footprint and pieces.
        studio_core::modify::move_elements(&mut doc, &[far], Pt::new(0.0, 1000.0)).unwrap();
        let (_, s) = regenerate_with_stats(&doc);
        assert_eq!((s.footprints, s.pieces, s.regions), (1, 1, 1));
        // Moving a rectangle wall also re-miters the two walls it meets.
        studio_core::modify::move_elements(&mut doc, &[walls[1]], Pt::new(600.0, 0.0)).unwrap();
        let (_, s) = regenerate_with_stats(&doc);
        assert_eq!(s.footprints, 3, "{s:?}");
        // Undo returns to a state regenerated before: nothing is recomputed from scratch
        // beyond what differs from the latest state.
        doc.undo().unwrap();
        let (m, _) = regenerate_with_stats(&doc);
        assert!(*m == regenerate_full(&doc));
    }

    #[test]
    fn roofs_stairs_and_layers_are_regenerated() {
        let (mut doc, l1, wt) = project();
        rectangle(&mut doc, l1, wt);
        let m = regenerate(&doc);
        let outer = outer_boundary(&m, l1).unwrap();
        let rt = studio_core::build::default_roof_type(&doc).unwrap();
        let l2 = doc.levels()[1].0;
        studio_core::build::create_roof(
            &mut doc,
            rt,
            l2,
            0.0,
            outer,
            studio_core::build::DEFAULT_ROOF_SLOPE,
        )
        .unwrap();
        studio_core::build::create_stair(
            &mut doc,
            l1,
            Pt::new(1000.0, 1000.0),
            Pt::new(1000.0, 5000.0),
            studio_core::build::DEFAULT_STAIR_WIDTH,
        )
        .unwrap();
        let m = regenerate(&doc);
        assert_eq!(m.roofs.len(), 1);
        assert_eq!(m.roofs[0].faces.len(), 4);
        let s = &m.stairs[0];
        assert_eq!(s.risers, 18);
        assert_eq!(s.steps.len(), 17);
        assert!((s.steps[16].z1 - (s.z1() - s.riser)).abs() < EPS);
        assert!((s.z1() - 10.0 * MM_PER_FT).abs() < EPS);
        // The exterior type has four layers → three boundaries inside the wall.
        assert_eq!(m.walls[0].layers.len(), 3);
        let (_, hi) = m.z_range();
        assert!(
            hi > 10.0 * MM_PER_FT + 1000.0,
            "roof peak counts in the height range"
        );
    }
}
