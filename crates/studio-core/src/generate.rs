//! Buildings from a room layout (ADR-030): Claude plans the building as rectangular rooms
//! per story (feet, x east, y north); this turns the plan into a real model — levels,
//! walls where rooms meet and around the outside, doors that connect every room, windows
//! on outside walls, stairs, floor slabs, a roof, named rooms and library materials — as
//! one undoable step.

use std::collections::{BTreeMap, BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};
use studio_geom::{Poly, Pt};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, StairShape, ViewKind};
use crate::units::{MM_PER_FT, MM_PER_IN};

/// What a room is for: sets its doors, windows and whether it opens into its neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RoomKind {
    Living,
    Dining,
    Kitchen,
    Bedroom,
    Bathroom,
    Closet,
    Laundry,
    Garage,
    Corridor,
    Stair,
    Entry,
    Lobby,
    Office,
    Unit,
    GuestRoom,
    Retail,
    Amenity,
    Mechanical,
    Storage,
    Other,
}

impl RoomKind {
    /// Circulation: doors route through these.
    fn circulation(self) -> bool {
        matches!(
            self,
            RoomKind::Corridor
                | RoomKind::Entry
                | RoomKind::Lobby
                | RoomKind::Living
                | RoomKind::Stair
        )
    }
    /// Dead ends: a door in, never through.
    fn leaf(self) -> bool {
        matches!(
            self,
            RoomKind::Bathroom
                | RoomKind::Closet
                | RoomKind::Laundry
                | RoomKind::Mechanical
                | RoomKind::Storage
                | RoomKind::Garage
        )
    }
    /// Open plan: no wall between two of these.
    fn open(self) -> bool {
        matches!(
            self,
            RoomKind::Living | RoomKind::Dining | RoomKind::Kitchen | RoomKind::Entry
        )
    }
    fn windows(self) -> Option<Glazing> {
        match self {
            RoomKind::Bedroom | RoomKind::Kitchen | RoomKind::Office | RoomKind::GuestRoom => {
                Some(Glazing::Punched)
            }
            RoomKind::Living | RoomKind::Dining | RoomKind::Unit | RoomKind::Amenity => {
                Some(Glazing::Large)
            }
            RoomKind::Lobby | RoomKind::Retail => Some(Glazing::Storefront),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Glazing {
    Punched,
    Large,
    Storefront,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomSpec {
    pub name: String,
    pub kind: RoomKind,
    /// South-west corner and size, feet.
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub depth: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StorySpec {
    /// Floor to floor, feet.
    pub height: f64,
    pub rooms: Vec<RoomSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RoofKind {
    #[default]
    Hip,
    Gable,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Structure {
    /// Wood studs and joists (houses, walk-ups, podium upper floors).
    #[default]
    Wood,
    /// Masonry walls and concrete slabs.
    Masonry,
}

/// Library preset ids (ADR-029) for the building's main surfaces.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct MaterialSpec {
    pub exterior_walls: Option<String>,
    pub interior_walls: Option<String>,
    pub floors: Option<String>,
    pub roof: Option<String>,
}

/// The building's windows (ADR-031): a family for bedrooms, kitchens and offices (living
/// spaces and units take its twin), with a grille and frame finish.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WindowChoice {
    #[serde(default)]
    pub family: Option<crate::element::WindowFamily>,
    #[serde(default)]
    pub grille: crate::windows::Grille,
    #[serde(default)]
    pub finish: crate::windows::FrameFinish,
}

/// A building as Claude plans it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BuildingSpec {
    pub name: String,
    /// A few sentences on the design, for the owner.
    #[serde(default)]
    pub summary: String,
    pub stories: Vec<StorySpec>,
    #[serde(default)]
    pub roof: RoofKind,
    /// Roof pitch, rise per 12 (hip and gable).
    #[serde(default = "default_pitch")]
    pub pitch: f64,
    #[serde(default)]
    pub structure: Structure,
    #[serde(default)]
    pub materials: MaterialSpec,
    #[serde(default)]
    pub windows: WindowChoice,
}

fn default_pitch() -> f64 {
    6.0
}

/// What was built.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BuildReport {
    pub name: String,
    pub levels: usize,
    pub walls: usize,
    pub doors: usize,
    pub windows: usize,
    pub floors: usize,
    pub rooms: usize,
    pub stairs: usize,
    pub roofs: usize,
    pub materials: usize,
    pub warnings: Vec<String>,
}

/// Categories a generated building replaces.
const BUILDING: &[Category] = &[
    Category::Wall,
    Category::Door,
    Category::Window,
    Category::Floor,
    Category::Ceiling,
    Category::Roof,
    Category::Room,
    Category::Stair,
    Category::Railing,
    Category::Column,
    Category::Beam,
    Category::RoomSeparator,
];

#[derive(Clone, Copy, Debug)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    fn center(&self) -> Pt {
        Pt::new((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }
    fn ring(&self) -> Vec<Pt> {
        vec![
            Pt::new(self.x0, self.y0),
            Pt::new(self.x1, self.y0),
            Pt::new(self.x1, self.y1),
            Pt::new(self.x0, self.y1),
        ]
    }
}

/// A stretch of a line between two rooms (or a room and the outside).
#[derive(Clone, Copy, Debug)]
struct Piece {
    a: f64,
    b: f64,
    /// Rooms on either side (index into the story's rooms).
    left: Option<usize>,
    right: Option<usize>,
}

/// A wall to build: along x (horizontal) at `at`, or along y, from `a` to `b`.
#[derive(Debug)]
struct Run {
    horizontal: bool,
    at: f64,
    a: f64,
    b: f64,
    exterior: bool,
    pieces: Vec<Piece>,
    id: Option<ElementId>,
    /// Openings placed, as ranges along the wall from `a`.
    taken: Vec<(f64, f64)>,
}

impl Run {
    fn point(&self, t: f64) -> Pt {
        if self.horizontal {
            Pt::new(t, self.at)
        } else {
            Pt::new(self.at, t)
        }
    }
    /// Places an opening `width` wide centred at `t` if it fits clear of others.
    fn claim(&mut self, t: f64, width: f64) -> bool {
        let clear = 4.0 * MM_PER_IN;
        let (lo, hi) = (t - width / 2.0, t + width / 2.0);
        if lo < self.a + clear || hi > self.b - clear {
            return false;
        }
        if self
            .taken
            .iter()
            .any(|(p, q)| lo < q + clear && hi > p - clear)
        {
            return false;
        }
        self.taken.push((lo, hi));
        true
    }
}

/// A room's edge on a grid line: (from, to, room, the room is on the line's low side).
type Edge = (f64, f64, usize, bool);

fn key(v: f64) -> i64 {
    (v * 10.0).round() as i64
}

/// Splits the story's room edges into walls: runs along each grid line, exterior where only
/// one side is a room, interior between rooms — none between two open-plan rooms.
fn runs(rooms: &[(RoomKind, Rect)]) -> Vec<Run> {
    let mut out = vec![];
    for horizontal in [true, false] {
        // line → (a, b, room, room is on the low side)
        let mut lines: BTreeMap<i64, (f64, Vec<Edge>)> = BTreeMap::new();
        for (i, (_, r)) in rooms.iter().enumerate() {
            let (lo, hi, a, b) = if horizontal {
                (r.y0, r.y1, r.x0, r.x1)
            } else {
                (r.x0, r.x1, r.y0, r.y1)
            };
            // The room is above (high side of) its low edge, below its high edge.
            lines
                .entry(key(lo))
                .or_insert((lo, vec![]))
                .1
                .push((a, b, i, false));
            lines
                .entry(key(hi))
                .or_insert((hi, vec![]))
                .1
                .push((a, b, i, true));
        }
        for (_, (at, edges)) in lines {
            let mut cuts: Vec<f64> = edges.iter().flat_map(|e| [e.0, e.1]).collect();
            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|p, q| (*p - *q).abs() < 1.0);
            let mut pieces: Vec<(Piece, u8)> = vec![];
            for w in cuts.windows(2) {
                let (a, b) = (w[0], w[1]);
                if b - a < 1.0 {
                    continue;
                }
                let m = (a + b) / 2.0;
                let side = |low: bool| {
                    edges
                        .iter()
                        .find(|e| e.3 == low && e.0 <= m && m <= e.1)
                        .map(|e| e.2)
                };
                let (left, right) = (side(true), side(false));
                let class = match (left, right) {
                    (None, None) => continue,
                    (Some(l), Some(r)) if rooms[l].0.open() && rooms[r].0.open() => 0,
                    (Some(_), Some(_)) => 2,
                    _ => 1,
                };
                pieces.push((Piece { a, b, left, right }, class));
            }
            // Merge touching pieces of the same class into walls.
            let mut i = 0;
            while i < pieces.len() {
                let class = pieces[i].1;
                let mut j = i;
                while j + 1 < pieces.len()
                    && pieces[j + 1].1 == class
                    && (pieces[j + 1].0.a - pieces[j].0.b).abs() < 1.0
                {
                    j += 1;
                }
                out.push(Run {
                    horizontal,
                    at,
                    a: pieces[i].0.a,
                    b: pieces[j].0.b,
                    exterior: class == 1,
                    pieces: pieces[i..=j].iter().map(|p| p.0).collect(),
                    id: None,
                    taken: vec![],
                });
                i = j + 1;
            }
        }
    }
    out
}

fn snap(v: f64) -> f64 {
    let s = 0.5 * MM_PER_IN;
    (v / s).round() * s
}

/// Checks the plan Claude returned before anything is built.
pub fn validate(spec: &BuildingSpec) -> CoreResult<Vec<String>> {
    let bad = |m: String| Err(CoreError::Invalid(m));
    if spec.stories.is_empty() || spec.stories.len() > 20 {
        return bad(format!(
            "a building needs 1 to 20 stories, not {}",
            spec.stories.len()
        ));
    }
    let mut warnings = vec![];
    for (s, story) in spec.stories.iter().enumerate() {
        if !(7.0..=30.0).contains(&story.height) {
            return bad(format!(
                "story {} is {} ft floor to floor",
                s + 1,
                story.height
            ));
        }
        if story.rooms.is_empty() || story.rooms.len() > 300 {
            return bad(format!("story {} has {} rooms", s + 1, story.rooms.len()));
        }
        for r in &story.rooms {
            if !(r.width >= 2.0 && r.depth >= 2.0 && r.width <= 1000.0 && r.depth <= 1000.0)
                || !r.x.is_finite()
                || !r.y.is_finite()
                || r.x.abs() > 5000.0
                || r.y.abs() > 5000.0
            {
                return bad(format!("{} is {} x {} ft", r.name, r.width, r.depth));
            }
        }
        for (i, a) in story.rooms.iter().enumerate() {
            for b in &story.rooms[i + 1..] {
                let ox = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
                let oy = (a.y + a.depth).min(b.y + b.depth) - a.y.max(b.y);
                if ox > 0.25 && oy > 0.25 {
                    warnings.push(format!(
                        "Story {}: {} and {} overlap by {:.1}' x {:.1}'",
                        s + 1,
                        a.name,
                        b.name,
                        ox,
                        oy
                    ));
                }
            }
        }
    }
    Ok(warnings)
}

fn type_named(doc: &Document, cat: Category, words: &[&str]) -> Option<ElementId> {
    doc.of(cat)
        .find(|e| {
            let n = e.data.name().to_lowercase();
            words.iter().all(|w| n.contains(w))
        })
        .map(|e| e.id)
        .or_else(|| crate::ops::first_of(doc, cat))
}

/// Builds `spec` as the project's building (replacing any walls, floors, roofs, rooms…),
/// centred on the site's lot when there is one. One undo step.
pub fn build(doc: &mut Document, spec: &BuildingSpec) -> CoreResult<BuildReport> {
    let mut report = BuildReport {
        name: spec.name.clone(),
        warnings: validate(spec)?,
        ..BuildReport::default()
    };
    let mark = doc.undo_depth();
    let result = build_steps(doc, spec, &mut report);
    match result {
        Ok(()) => {
            doc.merge_undo(mark, &format!("Generate {}", spec.name));
            Ok(report)
        }
        Err(e) => {
            // Leave the model as it was.
            while doc.undo_depth() > mark {
                doc.undo()?;
            }
            Err(e)
        }
    }
}

/// Loads the building's window types from the library: its family at about 3'-0" x
/// 5'-0" for punched openings, a twin (or about 6'-0" wide) for living spaces, and 6'-0"
/// storefront, all in the chosen grille and finish.
fn window_types(doc: &mut Document, choice: WindowChoice) -> CoreResult<[ElementId; 3]> {
    use crate::element::WindowFamily as F;
    use crate::windows::{info, WindowSpec, CATALOG};
    let family = choice
        .family
        .filter(|f| !matches!(f, F::Bay | F::Storefront))
        .unwrap_or(F::DoubleHung);
    let near = |f: F, units: u32, w: f64, h: f64| {
        CATALOG
            .iter()
            .filter(|p| p.family == f && p.units == units)
            .min_by(|a, b| {
                let d = |p: &crate::windows::Preset| (p.width - w).abs() + (p.height - h).abs();
                d(a).total_cmp(&d(b))
            })
            .copied()
    };
    let punched = near(family, 1, 36.0, 60.0);
    let large = info(family)
        .mullable
        .then(|| near(family, 2, 72.0, 60.0))
        .flatten()
        .or_else(|| near(family, 1, 72.0, 60.0));
    let store = near(F::Storefront, 1, 72.0, 96.0);
    let specs: Vec<WindowSpec> = [punched, large, store]
        .into_iter()
        .flatten()
        .map(|p| {
            let mut s: WindowSpec = p.into();
            s.grille = choice.grille;
            s.finish = choice.finish;
            s
        })
        .collect();
    match crate::windows::load(doc, &specs)?[..] {
        [a, b, c] => Ok([a, b, c]),
        _ => Err(CoreError::Invalid("no window sizes for that family".into())),
    }
}

fn build_steps(
    doc: &mut Document,
    spec: &BuildingSpec,
    report: &mut BuildReport,
) -> CoreResult<()> {
    // 1. Clear the old building.
    let old: Vec<ElementId> = doc
        .iter()
        .filter(|e| BUILDING.contains(&e.category()))
        .map(|e| e.id)
        .collect();
    if !old.is_empty() {
        let pinned: Vec<ElementId> = old
            .iter()
            .copied()
            .filter(|id| crate::visibility::is_pinned(doc, *id))
            .collect();
        if !pinned.is_empty() {
            crate::visibility::set_pinned(doc, &pinned, false)?;
        }
        crate::ops::delete(doc, &old)?;
    }

    // 2. Levels: one per story and a roof level on top.
    let mut elevations = vec![0.0];
    for s in &spec.stories {
        let z = elevations.last().copied().unwrap_or(0.0) + s.height * MM_PER_FT;
        elevations.push(z);
    }
    let mut levels: Vec<ElementId> = doc.levels().into_iter().map(|l| l.0).collect();
    while levels.len() < elevations.len() {
        levels.push(crate::ops::create_level(
            doc,
            1.0e6 + levels.len() as f64 * 1000.0,
        )?);
    }
    let top = *elevations.last().unwrap_or(&0.0);
    doc.transact("Set levels", |tx| {
        for (i, id) in levels.iter().enumerate() {
            let (name, z) = if i + 1 == elevations.len() {
                ("Roof".to_owned(), elevations[i])
            } else if i < elevations.len() {
                (format!("Level {}", i + 1), elevations[i])
            } else {
                // Levels the building doesn't use go above the roof, out of the way.
                (format!("Level {}", i + 1), top + (i + 1 - elevations.len()) as f64 * 3048.0)
            };
            let old = tx.data(*id)?.name();
            tx.modify(*id, |d| {
                if let ElementData::Level { name: n, elevation } = d {
                    *n = name.clone();
                    *elevation = z;
                }
            })?;
            // Their plans follow the new name.
            let views: Vec<ElementId> = tx
                .of(Category::View)
                .filter(|e| {
                    matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level }, name, .. } if level == id && *name == old)
                })
                .map(|e| e.id)
                .collect();
            for v in views {
                tx.modify(v, |d| {
                    if let ElementData::View { name: n, .. } = d {
                        *n = name.clone();
                    }
                })?;
            }
        }
        Ok(())
    })?;
    report.levels = elevations.len();

    // Centre the building on the lot (or the origin).
    let site_center = crate::site::site_of(doc)
        .and_then(|id| match doc.data(id).ok()? {
            ElementData::Site { offset, .. } => Some(*offset),
            _ => None,
        })
        .unwrap_or_default();
    let all: Vec<&RoomSpec> = spec.stories.iter().flat_map(|s| &s.rooms).collect();
    let (mut lo, mut hi) = (Pt::new(f64::MAX, f64::MAX), Pt::new(f64::MIN, f64::MIN));
    for r in spec.stories[0].rooms.iter().chain(all.iter().copied()) {
        lo = Pt::new(lo.x.min(r.x), lo.y.min(r.y));
        hi = Pt::new(hi.x.max(r.x + r.width), hi.y.max(r.y + r.depth));
    }
    let shift = site_center.sub(Pt::new(
        (lo.x + hi.x) / 2.0 * MM_PER_FT,
        (lo.y + hi.y) / 2.0 * MM_PER_FT,
    ));

    let wood = spec.structure == Structure::Wood;
    let ext_type = if wood {
        type_named(doc, Category::WallType, &["exterior", "stud"])
    } else {
        type_named(doc, Category::WallType, &["exterior", "cmu"])
    }
    .ok_or_else(|| CoreError::Invalid("no wall types in this project".into()))?;
    let int_type = type_named(doc, Category::WallType, &["interior", "6"])
        .ok_or_else(|| CoreError::Invalid("no wall types in this project".into()))?;
    let door_wide = type_named(doc, Category::DoorType, &["36"]);
    let door_narrow = type_named(doc, Category::DoorType, &["30"]);
    let door_double = type_named(doc, Category::DoorType, &["double"]);
    let [win_punched, win_large, win_store] = window_types(doc, spec.windows)?.map(Some);
    let slab = type_named(doc, Category::FloorType, &["slab"]);
    let joist = type_named(doc, Category::FloorType, &["joist"]);
    let roof_type = crate::ops::first_of(doc, Category::RoofType);
    let width_of = |doc: &Document, t: Option<ElementId>| match t.and_then(|t| doc.data(t).ok()) {
        Some(ElementData::DoorType { width, .. } | ElementData::WindowType { width, .. }) => *width,
        _ => 900.0,
    };

    let mut exterior_walls = vec![];
    let mut interior_walls = vec![];
    let mut floors = vec![];
    let mut roofs = vec![];
    let mut top_outline: Vec<Poly> = vec![];

    for (s, story) in spec.stories.iter().enumerate() {
        let level = levels[s];
        let rooms: Vec<(RoomKind, Rect)> = story
            .rooms
            .iter()
            .map(|r| {
                let p = |x: f64, y: f64| {
                    Pt::new(snap(x * MM_PER_FT + shift.x), snap(y * MM_PER_FT + shift.y))
                };
                let (a, b) = (p(r.x, r.y), p(r.x + r.width, r.y + r.depth));
                (
                    r.kind,
                    Rect {
                        x0: a.x,
                        y0: a.y,
                        x1: b.x,
                        y1: b.y,
                    },
                )
            })
            .collect();

        // 3. Walls.
        let mut runs = runs(&rooms);
        for run in &mut runs {
            let open = !run.exterior
                && run.pieces.iter().all(|p| match (p.left, p.right) {
                    (Some(l), Some(r)) => rooms[l].0.open() && rooms[r].0.open(),
                    _ => false,
                });
            if open && run.b - run.a >= 300.0 {
                // Open plan: a room separator (no wall) keeps each room's own area.
                crate::detail::create_room_separator(
                    doc,
                    level,
                    run.point(run.a),
                    run.point(run.b),
                )?;
                continue;
            }
            if open || run.b - run.a < 300.0 {
                continue;
            }
            let t = if run.exterior { ext_type } else { int_type };
            let id = crate::ops::create_wall(doc, t, level, run.point(run.a), run.point(run.b))?;
            run.id = Some(id);
            if run.exterior {
                exterior_walls.push(id);
            } else {
                interior_walls.push(id);
            }
            report.walls += 1;
        }

        // 4. Doors: in from the street, then along the cheapest routes to every room.
        let door_for = |k: RoomKind| match k {
            RoomKind::Bathroom | RoomKind::Closet | RoomKind::Laundry | RoomKind::Bedroom => {
                door_narrow
            }
            _ => door_wide,
        };
        let place = |doc: &mut Document,
                     runs: &mut [Run],
                     ri: usize,
                     t: f64,
                     ty: Option<ElementId>|
         -> bool {
            let (Some(ty), Some(host)) = (ty, runs[ri].id) else {
                return false;
            };
            let w = width_of(doc, Some(ty));
            if !runs[ri].claim(t, w) {
                return false;
            }
            let offset = t - runs[ri].a;
            let is_door = matches!(doc.data(ty), Ok(ElementData::DoorType { .. }));
            let made = if is_door {
                crate::ops::create_door(doc, ty, host, offset, false)
            } else {
                crate::ops::create_window(doc, ty, host, offset, false)
            };
            made.is_ok()
        };
        let mut reached = vec![false; rooms.len()];
        let mut heap = BinaryHeap::new();
        // Roots: entries (or the best room) on the ground floor; stairs above.
        let mut roots: Vec<usize> = rooms
            .iter()
            .enumerate()
            .filter(|(_, (k, _))| {
                if s == 0 {
                    matches!(
                        k,
                        RoomKind::Entry | RoomKind::Lobby | RoomKind::Retail | RoomKind::Garage
                    )
                } else {
                    *k == RoomKind::Stair
                }
            })
            .map(|(i, _)| i)
            .collect();
        if roots.is_empty() {
            let pick = [RoomKind::Living, RoomKind::Corridor, RoomKind::Lobby]
                .iter()
                .find_map(|k| rooms.iter().position(|r| r.0 == *k))
                .unwrap_or(0);
            roots.push(pick);
        }
        if s == 0 {
            for &r in &roots {
                // The longest outside wall of the room takes the door.
                let best = runs
                    .iter()
                    .enumerate()
                    .filter(|(_, run)| run.exterior && run.id.is_some())
                    .flat_map(|(ri, run)| {
                        run.pieces
                            .iter()
                            .filter(move |p| p.left == Some(r) || p.right == Some(r))
                            .map(move |p| (ri, *p))
                    })
                    .max_by(|a, b| (a.1.b - a.1.a).total_cmp(&(b.1.b - b.1.a)));
                if let Some((ri, p)) = best {
                    let ty = match rooms[r].0 {
                        RoomKind::Lobby | RoomKind::Garage => door_double.or(door_wide),
                        _ => door_wide,
                    };
                    if place(doc, &mut runs, ri, (p.a + p.b) / 2.0, ty) {
                        report.doors += 1;
                    }
                }
            }
        }
        // Neighbours: (room, other, run, piece) for pieces at least a door wide.
        let mut nbrs: HashMap<usize, Vec<(usize, usize, Piece)>> = HashMap::new();
        for (ri, run) in runs.iter().enumerate() {
            if run.exterior {
                continue;
            }
            for p in &run.pieces {
                if let (Some(l), Some(r)) = (p.left, p.right) {
                    if p.b - p.a >= 3.5 * MM_PER_FT {
                        nbrs.entry(l).or_default().push((r, ri, *p));
                        nbrs.entry(r).or_default().push((l, ri, *p));
                    }
                }
            }
        }
        for &r in &roots {
            heap.push((std::cmp::Reverse(0u32), r, None::<usize>));
        }
        // How each queued step got there: (from room, run, piece).
        let mut vias: Vec<(usize, usize, Piece)> = vec![];
        // Rooms that got a door (or an open connection) in.
        let mut doored = vec![false; rooms.len()];
        // Rooms with a way onto the floor (a door or opening to a room that isn't a dead end).
        let mut onto_floor = vec![false; rooms.len()];
        while let Some((std::cmp::Reverse(cost), room, via)) = heap.pop() {
            let via = via.map(|i| vias[i]);
            if reached[room] {
                continue;
            }
            reached[room] = true;
            if let Some((from, ri, p)) = via {
                let (a, b) = (rooms[from].0, rooms[room].0);
                let open = runs[ri].id.is_none() || (a.open() && b.open());
                if !a.leaf() {
                    // Tentatively; undone below if the door doesn't fit.
                    onto_floor[room] = true;
                }
                if !b.leaf() {
                    onto_floor[from] = true;
                }
                if open {
                    doored[room] = true;
                    doored[from] = true;
                } else {
                    let ty = door_for(if b.leaf() || b == RoomKind::Bedroom {
                        b
                    } else {
                        a
                    });
                    if place(doc, &mut runs, ri, (p.a + p.b) / 2.0, ty) {
                        report.doors += 1;
                        doored[room] = true;
                        doored[from] = true;
                    } else {
                        onto_floor[room] = false;
                        onto_floor[from] = false;
                    }
                }
            }
            if rooms[room].0.leaf() && via.is_some() {
                continue;
            }
            for (other, ri, p) in nbrs.get(&room).cloned().unwrap_or_default() {
                if reached[other] {
                    continue;
                }
                let k = rooms[other].0;
                let step = if k.circulation() {
                    1
                } else if k.leaf() {
                    2
                } else {
                    3
                };
                vias.push((room, ri, p));
                heap.push((std::cmp::Reverse(cost + step), other, Some(vias.len() - 1)));
            }
        }
        // Every stair opens onto the floor: a stair that was a starting point (upper
        // floors) still needs a door to the corridor, or it's a sealed shaft.
        for st in 0..rooms.len() {
            if rooms[st].0 != RoomKind::Stair || onto_floor[st] {
                continue;
            }
            let best = nbrs
                .get(&st)
                .into_iter()
                .flatten()
                .filter(|(o, _, _)| !rooms[*o].0.leaf())
                .max_by(|x, y| {
                    let score = |n: &(usize, usize, Piece)| {
                        (
                            rooms[n.0].0.circulation() as u8,
                            ((n.2.b - n.2.a) * 10.0) as i64,
                        )
                    };
                    score(x).cmp(&score(y))
                })
                .copied();
            if let Some((_, ri, p)) = best {
                if place(doc, &mut runs, ri, (p.a + p.b) / 2.0, door_wide) {
                    report.doors += 1;
                    doored[st] = true;
                }
            }
        }
        for (i, &ok) in reached.iter().enumerate() {
            if ok && !doored[i] && !roots.contains(&i) {
                report.warnings.push(format!(
                    "Story {}: no room for a door into {}",
                    s + 1,
                    story.rooms[i].name
                ));
            }
        }
        for (i, ok) in reached.iter().enumerate() {
            if !ok {
                report.warnings.push(format!(
                    "Story {}: {} has no route in (no shared wall long enough for a door)",
                    s + 1,
                    story.rooms[i].name
                ));
            }
        }

        // 5. Windows on the outside walls of rooms that want them.
        for ri in 0..runs.len() {
            if !runs[ri].exterior || runs[ri].id.is_none() {
                continue;
            }
            for p in runs[ri].pieces.clone() {
                let Some(r) = p.left.or(p.right) else {
                    continue;
                };
                let Some(glazing) = rooms[r].0.windows() else {
                    continue;
                };
                let (ty, spacing) = match glazing {
                    Glazing::Punched => (win_punched, 7.0 * MM_PER_FT),
                    Glazing::Large => (win_large, 7.0 * MM_PER_FT),
                    Glazing::Storefront => (win_store, 7.0 * MM_PER_FT),
                };
                let len = p.b - p.a;
                if len < 4.0 * MM_PER_FT {
                    continue;
                }
                let n = ((len / spacing).floor() as usize).max(1);
                for k in 0..n {
                    let t = p.a + (k as f64 + 0.5) * len / n as f64;
                    if place(doc, &mut runs, ri, t, ty) {
                        report.windows += 1;
                    }
                }
            }
        }

        // 6. Stairs up from each stair room (not from the top story).
        if s + 1 < spec.stories.len() {
            let rise = story.height * MM_PER_FT;
            let (_, _, run_len) = crate::build::stair_layout(
                rise,
                crate::build::DEFAULT_TREAD,
                crate::build::DEFAULT_MAX_RISER,
            );
            for (_, r) in rooms.iter().filter(|(k, _)| *k == RoomKind::Stair) {
                let (w, d) = (r.x1 - r.x0, r.y1 - r.y0);
                let along_x = w >= d;
                let (long, short) = if along_x { (w, d) } else { (d, w) };
                let inset = 6.0 * MM_PER_IN;
                let c = r.center();
                let (shape, width) = if long >= run_len + 2.0 * inset + 900.0 {
                    (
                        StairShape::Straight,
                        (short - 2.0 * inset).min(44.0 * MM_PER_IN),
                    )
                } else {
                    (
                        StairShape::UShaped { left: false },
                        ((short - 2.0 * inset) / 2.0).min(44.0 * MM_PER_IN),
                    )
                };
                if width < 30.0 * MM_PER_IN {
                    report.warnings.push(format!(
                        "Story {}: a stair room is too narrow for a stair",
                        s + 1
                    ));
                    continue;
                }
                // Start at one end of the room, on its centre line (a U-stair's first run
                // sits on one side).
                let side = if matches!(shape, StairShape::UShaped { .. }) {
                    width / 2.0
                } else {
                    0.0
                };
                let (start, toward) = if along_x {
                    let y = c.y - side;
                    (Pt::new(r.x0 + inset, y), Pt::new(r.x1, y))
                } else {
                    let x = c.x + side;
                    (Pt::new(x, r.y0 + inset), Pt::new(x, r.y1))
                };
                if crate::build::create_stair_shaped(doc, level, start, toward, width, shape)
                    .is_ok()
                {
                    report.stairs += 1;
                }
            }
        }

        // 7. The floor slab: the outline of the story's rooms.
        let outline = studio_geom::union_all(
            &rooms
                .iter()
                .map(|(_, r)| Poly::simple(r.ring()))
                .collect::<Vec<_>>(),
        );
        let ftype = if s == 0 || !wood {
            slab.or(joist)
        } else {
            joist.or(slab)
        };
        if let Some(ft) = ftype {
            for p in &outline {
                floors.push(crate::ops::create_floor(doc, ft, level, p.outer.clone())?);
                report.floors += 1;
            }
        }
        if s + 1 == spec.stories.len() {
            top_outline = outline;
        }

        // 8. Rooms, named as planned.
        for (i, (_, r)) in rooms.iter().enumerate() {
            let id = crate::ops::create_room(doc, level, r.center())?;
            let name = story.rooms[i].name.clone();
            doc.transact("Name room", |tx| {
                tx.modify(id, |d| {
                    if let ElementData::Room { name: n, .. } = d {
                        *n = name.clone();
                    }
                })
            })?;
            report.rooms += 1;
        }
    }

    // 9. The roof over the top story.
    let roof_level = levels[spec.stories.len()];
    if let Some(rt) = roof_type {
        let slope = (spec.pitch.clamp(1.0, 18.0) / 12.0).atan();
        for p in &top_outline {
            let ring = p.outer.clone();
            let made = match spec.roof {
                RoofKind::Flat => crate::build::create_roof(doc, rt, roof_level, 0.0, ring, 0.0)
                    .map(|id| vec![id]),
                RoofKind::Gable if ring.len() == 4 => gable(doc, rt, roof_level, &ring, slope),
                _ => crate::build::create_roofs_by_footprint(
                    doc,
                    rt,
                    roof_level,
                    0.0,
                    &ring,
                    18.0 * MM_PER_IN,
                    slope,
                )
                .or_else(|_| {
                    crate::build::create_roof(doc, rt, roof_level, 0.0, ring, 0.0)
                        .map(|id| vec![id])
                }),
            };
            match made {
                Ok(ids) => {
                    report.roofs += ids.len();
                    roofs.extend(ids);
                }
                Err(e) => report.warnings.push(format!("Roof: {e}")),
            }
        }
    }

    // 10. Library materials on the main surfaces.
    let m = &spec.materials;
    for (preset, targets) in [
        (&m.exterior_walls, &exterior_walls),
        (&m.interior_walls, &interior_walls),
        (&m.floors, &floors),
        (&m.roof, &roofs),
    ] {
        let Some(id) = preset.as_deref() else {
            continue;
        };
        if targets.is_empty() {
            continue;
        }
        if crate::library::preset(id).is_none() {
            report.warnings.push(format!("No library material {id}"));
            continue;
        }
        let mat = crate::library::add_preset(doc, id)?;
        if crate::library::apply_to(doc, targets, mat).is_ok() {
            report.materials += 1;
        }
    }
    Ok(())
}

/// A gable roof over a rectangle: the two long sides slope, the ends are gables.
fn gable(
    doc: &mut Document,
    rt: ElementId,
    level: ElementId,
    ring: &[Pt],
    slope: f64,
) -> CoreResult<Vec<ElementId>> {
    let o = 18.0 * MM_PER_IN;
    let (mut lo, mut hi) = (ring[0], ring[0]);
    for p in ring {
        lo = Pt::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Pt::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let boundary = vec![
        Pt::new(lo.x - o, lo.y - o),
        Pt::new(hi.x + o, lo.y - o),
        Pt::new(hi.x + o, hi.y + o),
        Pt::new(lo.x - o, hi.y + o),
    ];
    // Edges: south, east, north, west. The long pair slopes.
    let long_x = hi.x - lo.x >= hi.y - lo.y;
    let sloped = vec![long_x, !long_x, long_x, !long_x];
    doc.transact("Create roof", |tx| {
        Ok(vec![tx.insert(ElementData::Roof {
            type_id: rt,
            level,
            offset: 0.0,
            boundary,
            slope,
            sloped,
        })])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    fn room(name: &str, kind: RoomKind, x: f64, y: f64, w: f64, d: f64) -> RoomSpec {
        RoomSpec {
            name: name.into(),
            kind,
            x,
            y,
            width: w,
            depth: d,
        }
    }

    /// A two-story house: open living/kitchen/dining and an entry below, bedrooms, a bath
    /// and a closet off a hall above, a stair in both.
    fn house() -> BuildingSpec {
        BuildingSpec {
            name: "Test House".into(),
            summary: String::new(),
            stories: vec![
                StorySpec {
                    height: 10.0,
                    rooms: vec![
                        room("Living", RoomKind::Living, 0.0, 0.0, 20.0, 16.0),
                        room("Kitchen", RoomKind::Kitchen, 20.0, 0.0, 14.0, 16.0),
                        room("Entry", RoomKind::Entry, 0.0, 16.0, 10.0, 10.0),
                        room("Stair", RoomKind::Stair, 10.0, 16.0, 10.0, 10.0),
                        room("Powder", RoomKind::Bathroom, 20.0, 16.0, 6.0, 10.0),
                        room("Dining", RoomKind::Dining, 26.0, 16.0, 8.0, 10.0),
                    ],
                },
                StorySpec {
                    height: 9.0,
                    rooms: vec![
                        room("Bedroom 1", RoomKind::Bedroom, 0.0, 0.0, 14.0, 13.0),
                        room("Bedroom 2", RoomKind::Bedroom, 14.0, 0.0, 12.0, 13.0),
                        room("Bath", RoomKind::Bathroom, 26.0, 0.0, 8.0, 13.0),
                        room("Hall", RoomKind::Corridor, 0.0, 13.0, 20.0, 3.5),
                        room("Closet", RoomKind::Closet, 20.0, 13.0, 14.0, 3.5),
                        room("Stair", RoomKind::Stair, 10.0, 16.5, 10.0, 9.5),
                        room("Bedroom 3", RoomKind::Bedroom, 0.0, 16.5, 10.0, 9.5),
                        room("Study", RoomKind::Office, 20.0, 16.5, 14.0, 9.5),
                    ],
                },
            ],
            roof: RoofKind::Hip,
            pitch: 6.0,
            structure: Structure::Wood,
            materials: MaterialSpec {
                exterior_walls: Some("masonry-red-brick".into()),
                roof: Some("roof-asphalt-shingle".into()),
                floors: Some("wood-white-oak-floor".into()),
                interior_walls: None,
            },
            windows: WindowChoice {
                family: Some(crate::element::WindowFamily::DoubleHung),
                grille: crate::windows::Grille::Colonial,
                finish: crate::windows::FrameFinish::White,
            },
        }
    }

    #[test]
    fn a_room_plan_becomes_a_building_in_one_undo() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let before = doc.undo_depth();
        let r = build(&mut doc, &house()).unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(doc.undo_depth(), before + 1, "one undo step");
        assert_eq!(r.levels, 3);
        let levels = doc.levels();
        assert_eq!(
            levels.iter().map(|l| l.1.as_str()).collect::<Vec<_>>(),
            ["Level 1", "Level 2", "Roof"]
        );
        assert!(
            (levels[1].2 - 10.0 * MM_PER_FT).abs() < 1e-6
                && (levels[2].2 - 19.0 * MM_PER_FT).abs() < 1e-6
        );
        // No wall between the open living, kitchen, dining and entry.
        assert!(r.walls >= 14, "{}", r.walls);
        assert_eq!(r.rooms, 14);
        assert_eq!(r.floors, 2);
        assert_eq!(r.stairs, 1);
        assert_eq!(r.roofs, 1);
        assert_eq!(r.materials, 3);
        // Every room is reachable: one entry door, then a door to each closed room.
        // Level 1: entry; stair, powder via doors (living/kitchen/dining/entry are open).
        // Level 2: from the stair: hall, three bedrooms, bath, closet, study.
        assert!(r.doors >= 8, "{}", r.doors);
        assert!(r.windows >= 8, "{}", r.windows);
        let names: Vec<String> = doc
            .of(Category::Room)
            .filter_map(|e| match &e.data {
                ElementData::Room { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        assert!(names.contains(&"Bedroom 3".to_string()));
        // Windows only on exterior walls.
        for e in doc.of(Category::Window) {
            let ElementData::Window { host, .. } = &e.data else {
                panic!()
            };
            let t = doc.data(*host).unwrap().type_id().unwrap();
            assert!(doc.data(t).unwrap().name().starts_with("Exterior"));
        }
        // Brick on the exterior walls' type.
        let w = doc
            .of(Category::Wall)
            .find(|e| {
                e.data
                    .type_id()
                    .is_some_and(|t| doc.data(t).unwrap().name().starts_with("Exterior"))
            })
            .unwrap()
            .id;
        let fin = crate::library::finish_of(&doc, w).unwrap();
        assert!(doc.data(fin).unwrap().name().starts_with("Red Brick"));
        // Undo takes the whole building away.
        doc.undo().unwrap();
        assert_eq!(doc.of(Category::Wall).count(), 0);
        assert_eq!(doc.of(Category::Room).count(), 0);
    }

    #[test]
    fn generating_again_replaces_the_building() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        build(&mut doc, &house()).unwrap();
        let walls = doc.of(Category::Wall).count();
        let mut one = house();
        one.stories.truncate(1);
        one.roof = RoofKind::Flat;
        let r = build(&mut doc, &one).unwrap();
        assert!(doc.of(Category::Wall).count() < walls);
        assert_eq!(r.stairs, 0, "no stair from the top story");
        assert_eq!(doc.levels()[1].1, "Roof");
        assert_eq!(doc.of(Category::Roof).count(), 1);
    }

    #[test]
    fn bad_plans_are_refused_or_flagged() {
        let mut bad = house();
        bad.stories.clear();
        assert!(validate(&bad).is_err());
        let mut tiny = house();
        tiny.stories[0].rooms[0].width = 0.5;
        assert!(validate(&tiny).is_err());
        let mut overlap = house();
        overlap.stories[0]
            .rooms
            .push(room("Oops", RoomKind::Other, 5.0, 5.0, 10.0, 10.0));
        let w = validate(&overlap).unwrap();
        assert!(w.iter().any(|m| m.contains("Oops")), "{w:?}");
        // A failed build leaves the model untouched.
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let depth = doc.undo_depth();
        assert!(build(&mut doc, &bad).is_err());
        assert_eq!(doc.undo_depth(), depth);
    }

    /// Garden-style apartments: three stories of eight units on a double-loaded corridor,
    /// a stair at each end, units as single rooms (as the prompt asks for bigger buildings).
    fn apartments() -> BuildingSpec {
        let story = |n: u32, lobby: bool| {
            let mut rooms = vec![
                room("Stair A", RoomKind::Stair, 0.0, 30.0, 12.0, 6.0),
                room("Stair B", RoomKind::Stair, 112.0, 30.0, 12.0, 6.0),
                room("Corridor", RoomKind::Corridor, 12.0, 30.0, 100.0, 6.0),
            ];
            for k in 0..4 {
                let x = 12.0 + k as f64 * 25.0;
                let (name, kind) = if lobby && k == 0 {
                    ("Lobby".to_string(), RoomKind::Lobby)
                } else {
                    (format!("Unit {n}0{} - 1BR", k + 1), RoomKind::Unit)
                };
                rooms.push(room(&name, kind, x, 0.0, 25.0, 30.0));
                rooms.push(room(
                    &format!("Unit {n}0{} - 2BR", k + 5),
                    RoomKind::Unit,
                    x,
                    36.0,
                    25.0,
                    30.0,
                ));
            }
            // Corner rooms beside the stairs fill out the footprint.
            rooms.push(room("Storage", RoomKind::Storage, 0.0, 0.0, 12.0, 30.0));
            rooms.push(room("Trash", RoomKind::Mechanical, 112.0, 0.0, 12.0, 30.0));
            rooms.push(room("Bike Room", RoomKind::Storage, 0.0, 36.0, 12.0, 30.0));
            rooms.push(room(
                "Mechanical",
                RoomKind::Mechanical,
                112.0,
                36.0,
                12.0,
                30.0,
            ));
            StorySpec {
                height: 10.0,
                rooms,
            }
        };
        BuildingSpec {
            name: "Maple Court".into(),
            summary: String::new(),
            stories: vec![story(1, true), story(2, false), story(3, false)],
            roof: RoofKind::Gable,
            pitch: 5.0,
            structure: Structure::Wood,
            materials: MaterialSpec::default(),
            windows: WindowChoice::default(),
        }
    }

    #[test]
    fn apartments_reach_every_unit_and_stack_their_stairs() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let r = build(&mut doc, &apartments()).unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.levels, 4);
        assert_eq!(r.stairs, 4, "two stairs between each pair of floors");
        assert_eq!(r.roofs, 1, "one gable over the bar");
        let ElementData::Roof { sloped, .. } = &doc.of(Category::Roof).next().unwrap().data else {
            panic!()
        };
        assert_eq!(
            sloped,
            &vec![true, false, true, false],
            "the long sides slope"
        );
        // Ground floor: the street door and one into each of its other 14 rooms; above,
        // 13 from the stairs' routes plus the second stair onto the corridor.
        assert_eq!(r.doors, 15 + 14 + 14, "{}", r.doors);
        // Units get windows on their outside walls; storage and stairs don't.
        assert!(r.windows >= 3 * 7 * 3, "{}", r.windows);
    }

    #[test]
    fn walls_split_where_rooms_meet() {
        // Two rooms side by side: a shared interior wall, six exterior pieces.
        let a = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 3000.0,
            y1: 3000.0,
        };
        let b = Rect {
            x0: 3000.0,
            y0: 0.0,
            x1: 6000.0,
            y1: 3000.0,
        };
        let rs = runs(&[(RoomKind::Bedroom, a), (RoomKind::Office, b)]);
        let interior: Vec<&Run> = rs.iter().filter(|r| !r.exterior).collect();
        assert_eq!(interior.len(), 1);
        assert!(!interior[0].horizontal && (interior[0].at - 3000.0).abs() < 1e-9);
        // South and north walls run the full 6 m as one wall each.
        let long = rs
            .iter()
            .filter(|r| r.exterior && r.horizontal && (r.b - r.a - 6000.0).abs() < 1e-9)
            .count();
        assert_eq!(long, 2);
    }
}
