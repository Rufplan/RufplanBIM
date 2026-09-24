//! Element kinds. All lengths are mm, measured from the project internal origin.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;
use uuid::Uuid;

/// Stable element identity (UUID v7). Serialized as a hyphenated string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ElementId(#[ts(type = "string")] pub Uuid);

impl ElementId {
    /// A new, time-ordered id.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
    pub fn from_bytes(b: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(b))
    }
}

impl Default for ElementId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for ElementId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// Element category, used for indexing, browser grouping and visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Category {
    Level,
    Grid,
    WallType,
    Wall,
    FloorType,
    Floor,
    CeilingType,
    Ceiling,
    View,
    ProjectInfo,
    Stage,
    DoorType,
    Door,
    WindowType,
    Window,
    Room,
    Dimension,
    TextNote,
    Sheet,
    Viewport,
    Tag,
    Issuance,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Level => "Level",
            Category::Grid => "Grid",
            Category::WallType => "WallType",
            Category::Wall => "Wall",
            Category::FloorType => "FloorType",
            Category::Floor => "Floor",
            Category::CeilingType => "CeilingType",
            Category::Ceiling => "Ceiling",
            Category::View => "View",
            Category::ProjectInfo => "ProjectInfo",
            Category::Stage => "Stage",
            Category::DoorType => "DoorType",
            Category::Door => "Door",
            Category::WindowType => "WindowType",
            Category::Window => "Window",
            Category::Room => "Room",
            Category::Dimension => "Dimension",
            Category::TextNote => "TextNote",
            Category::Sheet => "Sheet",
            Category::Viewport => "Viewport",
            Category::Tag => "Tag",
            Category::Issuance => "Issuance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WallFunction {
    Exterior,
    Interior,
}

/// Built-in door families (code-defined for v0.1, see DATA_MODEL.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DoorFamily {
    SingleFlush,
    DoubleFlush,
}

/// Built-in window families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WindowFamily {
    Fixed,
    Casement,
}

/// What sets the top of a wall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WallTop {
    /// Top follows a level (plus offset, mm).
    UpToLevel { level: ElementId, offset: f64 },
    /// Fixed height above the base, mm.
    Unconnected { height: f64 },
}

/// Compass direction an elevation view looks toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Compass {
    North,
    South,
    East,
    West,
}

impl Compass {
    /// Unit vector the viewer looks along (plan coordinates, +y = north).
    pub fn look(self) -> Pt {
        match self {
            Compass::North => Pt::new(0.0, 1.0),
            Compass::South => Pt::new(0.0, -1.0),
            Compass::East => Pt::new(1.0, 0.0),
            Compass::West => Pt::new(-1.0, 0.0),
        }
    }
}

/// Where a dimension end is attached, so it follows the model. Lengths in mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Anchor {
    /// `t` is the fraction along the wall's location line from its start; `side` the signed
    /// distance to the left of it (e.g. ±half the thickness for a face).
    Wall { wall: ElementId, t: f64, side: f64 },
    /// `t` is the fraction along the grid line from its start.
    Grid { grid: ElementId, t: f64 },
}

fn default_text_size() -> f64 {
    3.0
}

/// What a schedule view lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ScheduleKind {
    Doors,
    Windows,
    Rooms,
    Sheets,
}

/// Printed sheet sizes (landscape).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SheetSize {
    /// ARCH D, 36" × 24".
    ArchD,
    /// Tabloid, 17" × 11".
    Tabloid,
}

impl SheetSize {
    /// Paper width and height in mm.
    pub fn mm(self) -> (f64, f64) {
        match self {
            SheetSize::ArchD => (914.4, 609.6),
            SheetSize::Tabloid => (431.8, 279.4),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SheetSize::ArchD => "ARCH D 36\" x 24\"",
            SheetSize::Tabloid => "Tabloid 17\" x 11\"",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViewKind {
    FloorPlan {
        level: ElementId,
    },
    CeilingPlan {
        level: ElementId,
    },
    /// Elevation looking in the given direction. A "North Elevation" looks south, like Revit's
    /// convention of naming the elevation after the facade it shows.
    Elevation {
        facing: Compass,
    },
    ThreeD,
    /// Section along the line `start` → `end`, looking to the line's left (like a Revit
    /// section drawn left to right looks up the screen), showing `depth` mm beyond the cut.
    Section {
        start: Pt,
        end: Pt,
        depth: f64,
    },
    Schedule {
        kind: ScheduleKind,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageChange {
    pub from: Option<ElementId>,
    pub to: ElementId,
    /// Unix milliseconds.
    pub at: i64,
    pub note: String,
}

/// The Rufplan.io project a model publishes to (ADR-016).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RufplanLink {
    /// `open_projects.id`.
    pub id: String,
    pub name: String,
    /// For the project page URL, rufplan.io/projects/<slug>.
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElementData {
    Level {
        name: String,
        elevation: f64,
    },
    Grid {
        name: String,
        start: Pt,
        end: Pt,
    },
    WallType {
        name: String,
        thickness: f64,
        function: WallFunction,
    },
    Wall {
        type_id: ElementId,
        /// Location line (centerline) endpoints.
        start: Pt,
        end: Pt,
        base_level: ElementId,
        base_offset: f64,
        top: WallTop,
    },
    FloorType {
        name: String,
        thickness: f64,
    },
    /// Floor slab whose top surface sits at `level + offset`.
    Floor {
        type_id: ElementId,
        level: ElementId,
        offset: f64,
        boundary: Vec<Pt>,
    },
    CeilingType {
        name: String,
        thickness: f64,
    },
    /// Ceiling whose underside sits at `level + height`.
    Ceiling {
        type_id: ElementId,
        level: ElementId,
        height: f64,
        boundary: Vec<Pt>,
    },
    View {
        name: String,
        kind: ViewKind,
        /// Drawing scale denominator, e.g. 48 for 1/4" = 1'-0".
        scale: u32,
    },
    ProjectInfo {
        name: String,
        number: String,
        client: String,
        address: String,
        current_stage: Option<ElementId>,
        stage_history: Vec<StageChange>,
        #[serde(default)]
        rufplan: Option<RufplanLink>,
    },
    DoorType {
        name: String,
        family: DoorFamily,
        /// Rough opening width and height, mm.
        width: f64,
        height: f64,
    },
    /// A door hosted by a wall. `offset` is the distance from the wall's start point to the
    /// door's center, along the location line (mm).
    Door {
        type_id: ElementId,
        host: ElementId,
        offset: f64,
        /// Hinge on the wall-end side instead of the wall-start side.
        flip_hand: bool,
        /// Swing to the wall's right side (looking from start to end) instead of the left.
        flip_facing: bool,
        mark: String,
    },
    WindowType {
        name: String,
        family: WindowFamily,
        width: f64,
        height: f64,
        /// Default sill height for new instances, mm above the host wall's base.
        sill: f64,
    },
    Window {
        type_id: ElementId,
        host: ElementId,
        offset: f64,
        /// Sill height above the host wall's base, mm.
        sill: f64,
        flip_facing: bool,
        mark: String,
    },
    /// A room: a named, numbered space on a level. Its boundary is derived from the walls
    /// enclosing `point` and is not stored.
    Room {
        level: ElementId,
        point: Pt,
        name: String,
        number: String,
    },
    /// Aligned dimension in a view between `a` and `b`; the dimension line sits `offset`
    /// mm to the left of a→b (negative = right). Coordinates are the view's own (plan x/y,
    /// or elevation/section u/z).
    Dimension {
        view: ElementId,
        a: Pt,
        b: Pt,
        offset: f64,
        /// When set, the ends follow these elements; `a`/`b` are the fallback positions.
        #[serde(default)]
        a_ref: Option<Anchor>,
        #[serde(default)]
        b_ref: Option<Anchor>,
    },
    /// A text note in a view, `at` in the view's coordinates.
    TextNote {
        view: ElementId,
        at: Pt,
        text: String,
        /// Printed text height, paper mm.
        #[serde(default = "default_text_size")]
        size: f64,
    },
    Sheet {
        number: String,
        name: String,
        size: SheetSize,
        /// Design stages whose deliverable set includes this sheet (ADR-010).
        #[serde(default)]
        stages: Vec<ElementId>,
    },
    /// A tag in `view` showing `target`'s mark / name, moved `offset` mm (view coordinates)
    /// from its default spot. Deleting the tag hides it in that view only.
    Tag {
        view: ElementId,
        target: ElementId,
        offset: Pt,
    },
    /// A recorded issue of a drawing set (e.g. "Permit Set") in a design stage.
    Issuance {
        name: String,
        stage: Option<ElementId>,
        /// YYYY-MM-DD.
        date: String,
        sheets: Vec<ElementId>,
    },
    /// A view placed on a sheet; `center` is in paper mm from the sheet's bottom-left.
    Viewport {
        sheet: ElementId,
        view: ElementId,
        center: Pt,
    },
    /// A design stage (ADR-010).
    Stage {
        name: String,
        abbreviation: String,
        order: i32,
        /// ISO dates (YYYY-MM-DD), optional.
        start: String,
        target: String,
    },
}

impl ElementData {
    /// Host wall of a door or window.
    pub fn host(&self) -> Option<ElementId> {
        match self {
            ElementData::Door { host, .. } | ElementData::Window { host, .. } => Some(*host),
            _ => None,
        }
    }

    pub fn category(&self) -> Category {
        match self {
            ElementData::Level { .. } => Category::Level,
            ElementData::Grid { .. } => Category::Grid,
            ElementData::WallType { .. } => Category::WallType,
            ElementData::Wall { .. } => Category::Wall,
            ElementData::FloorType { .. } => Category::FloorType,
            ElementData::Floor { .. } => Category::Floor,
            ElementData::CeilingType { .. } => Category::CeilingType,
            ElementData::Ceiling { .. } => Category::Ceiling,
            ElementData::View { .. } => Category::View,
            ElementData::ProjectInfo { .. } => Category::ProjectInfo,
            ElementData::Stage { .. } => Category::Stage,
            ElementData::DoorType { .. } => Category::DoorType,
            ElementData::Door { .. } => Category::Door,
            ElementData::WindowType { .. } => Category::WindowType,
            ElementData::Window { .. } => Category::Window,
            ElementData::Room { .. } => Category::Room,
            ElementData::Dimension { .. } => Category::Dimension,
            ElementData::TextNote { .. } => Category::TextNote,
            ElementData::Sheet { .. } => Category::Sheet,
            ElementData::Viewport { .. } => Category::Viewport,
            ElementData::Tag { .. } => Category::Tag,
            ElementData::Issuance { .. } => Category::Issuance,
        }
    }

    /// Elements this one depends on. Deleting any of them deletes this element too.
    pub fn refs(&self) -> Vec<ElementId> {
        match self {
            ElementData::Wall {
                type_id,
                base_level,
                top,
                ..
            } => {
                let mut v = vec![*type_id, *base_level];
                if let WallTop::UpToLevel { level, .. } = top {
                    v.push(*level);
                }
                v
            }
            ElementData::Floor { type_id, level, .. }
            | ElementData::Ceiling { type_id, level, .. } => {
                vec![*type_id, *level]
            }
            ElementData::Room { level, .. } => vec![*level],
            ElementData::Dimension { view, .. } | ElementData::TextNote { view, .. } => vec![*view],
            ElementData::Viewport { sheet, view, .. } => vec![*sheet, *view],
            ElementData::Tag { view, target, .. } => vec![*view, *target],
            ElementData::Door { type_id, host, .. } | ElementData::Window { type_id, host, .. } => {
                vec![*type_id, *host]
            }
            ElementData::View {
                kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
                ..
            } => {
                vec![*level]
            }
            _ => vec![],
        }
    }

    /// Display name used in the browser and properties.
    pub fn name(&self) -> String {
        match self {
            ElementData::Level { name, .. }
            | ElementData::WallType { name, .. }
            | ElementData::FloorType { name, .. }
            | ElementData::CeilingType { name, .. }
            | ElementData::View { name, .. }
            | ElementData::DoorType { name, .. }
            | ElementData::WindowType { name, .. }
            | ElementData::Stage { name, .. } => name.clone(),
            ElementData::Door { mark, .. } => format!("Door {mark}"),
            ElementData::Window { mark, .. } => format!("Window {mark}"),
            ElementData::Room { name, number, .. } => format!("{name} {number}"),
            ElementData::Dimension { .. } => "Dimension".into(),
            ElementData::TextNote { text, .. } => text.clone(),
            ElementData::Sheet { number, name, .. } => format!("{number} - {name}"),
            ElementData::Viewport { .. } => "Viewport".into(),
            ElementData::Tag { .. } => "Tag".into(),
            ElementData::Issuance { name, date, .. } => format!("{name} ({date})"),
            ElementData::Grid { name, .. } => format!("Grid {name}"),
            ElementData::Wall { .. } => "Wall".into(),
            ElementData::Floor { .. } => "Floor".into(),
            ElementData::Ceiling { .. } => "Ceiling".into(),
            ElementData::ProjectInfo { .. } => "Project Information".into(),
        }
    }

    /// The level an element is associated with, for the level index.
    pub fn level(&self) -> Option<ElementId> {
        match self {
            ElementData::Wall { base_level, .. } => Some(*base_level),
            ElementData::Floor { level, .. }
            | ElementData::Ceiling { level, .. }
            | ElementData::Room { level, .. } => Some(*level),
            ElementData::View {
                kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
                ..
            } => Some(*level),
            _ => None,
        }
    }

    /// The type element of an instance, if any.
    pub fn type_id(&self) -> Option<ElementId> {
        match self {
            ElementData::Wall { type_id, .. }
            | ElementData::Floor { type_id, .. }
            | ElementData::Ceiling { type_id, .. }
            | ElementData::Door { type_id, .. }
            | ElementData::Window { type_id, .. } => Some(*type_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    /// Increments on every committed modification (for sync).
    pub rev: u64,
    pub data: ElementData,
}

impl Element {
    pub fn new(data: ElementData) -> Self {
        Self {
            id: ElementId::new(),
            rev: 1,
            data,
        }
    }
    pub fn category(&self) -> Category {
        self.data.category()
    }
}
