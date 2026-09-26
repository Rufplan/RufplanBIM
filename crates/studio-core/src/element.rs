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
    RoofType,
    Roof,
    Stair,
    ColumnType,
    Column,
    BeamType,
    Beam,
    RailingType,
    Railing,
    Material,
    RoomSeparator,
    ElevationMarker,
    ElevationMarkerType,
    Site,
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
            Category::RoofType => "RoofType",
            Category::Roof => "Roof",
            Category::Stair => "Stair",
            Category::ColumnType => "ColumnType",
            Category::Column => "Column",
            Category::BeamType => "BeamType",
            Category::Beam => "Beam",
            Category::RailingType => "RailingType",
            Category::Railing => "Railing",
            Category::Material => "Material",
            Category::RoomSeparator => "RoomSeparator",
            Category::ElevationMarker => "ElevationMarker",
            Category::ElevationMarkerType => "ElevationMarkerType",
            Category::Site => "Site",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WallFunction {
    Exterior,
    Interior,
}

/// What a wall layer does (Revit's layer functions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LayerFunction {
    Structure,
    Substrate,
    Insulation,
    Finish,
    Membrane,
    AirGap,
}

impl LayerFunction {
    pub const ALL: [LayerFunction; 6] = [
        LayerFunction::Structure,
        LayerFunction::Substrate,
        LayerFunction::Insulation,
        LayerFunction::Finish,
        LayerFunction::Membrane,
        LayerFunction::AirGap,
    ];
    pub fn label(self) -> &'static str {
        match self {
            LayerFunction::Structure => "Structure",
            LayerFunction::Substrate => "Substrate",
            LayerFunction::Insulation => "Insulation",
            LayerFunction::Finish => "Finish",
            LayerFunction::Membrane => "Membrane",
            LayerFunction::AirGap => "Air Gap",
        }
    }
    pub fn parse(s: &str) -> Option<LayerFunction> {
        LayerFunction::ALL
            .into_iter()
            .find(|f| f.label() == s || format!("{f:?}") == s)
    }
}

/// One layer of a compound wall type. Layers are listed from the exterior face inward;
/// the exterior face is on the wall's left, looking from its start to its end (walls drawn
/// clockwise have their exterior outside, as in Revit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WallLayer {
    /// Layer description, e.g. "Gypsum Board" (the material's name when one is picked).
    pub name: String,
    /// mm.
    pub thickness: f64,
    pub function: LayerFunction,
    /// The layer's material (ADR-020); None falls back to rules on the name.
    #[serde(default)]
    pub material: Option<ElementId>,
}

/// How a material's cut face is hatched in plans and sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum CutPattern {
    #[default]
    None,
    /// Batt insulation zigzag.
    Insulation,
    /// Rigid insulation: a diagonal crosshatch.
    Rigid,
    /// Brick, block and stone: diagonal lines.
    Masonry,
    /// Diagonal dashes with aggregate triangles.
    Concrete,
    /// Framing lumber: a sparse diagonal cross.
    Wood,
    /// Solid fill (steel at small scales).
    Solid,
}

impl CutPattern {
    pub const ALL: [CutPattern; 7] = [
        CutPattern::None,
        CutPattern::Insulation,
        CutPattern::Rigid,
        CutPattern::Masonry,
        CutPattern::Concrete,
        CutPattern::Wood,
        CutPattern::Solid,
    ];
    pub fn label(self) -> &'static str {
        match self {
            CutPattern::None => "None",
            CutPattern::Insulation => "Batt Insulation",
            CutPattern::Rigid => "Rigid Insulation",
            CutPattern::Masonry => "Masonry",
            CutPattern::Concrete => "Concrete",
            CutPattern::Wood => "Wood Framing",
            CutPattern::Solid => "Solid Fill",
        }
    }
    pub fn parse(s: &str) -> Option<CutPattern> {
        CutPattern::ALL
            .into_iter()
            .find(|c| c.label() == s || format!("{c:?}") == s)
    }
}

/// How a material's surface reads in elevations (lines at true size, mm).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SurfacePattern {
    #[default]
    None,
    /// Horizontal lines (lap siding, shingle courses) `spacing` apart.
    Lap { spacing: f64 },
    /// Running bond: courses `course` high, units `unit` long, joints staggered by half.
    Running { course: f64, unit: f64 },
    /// A rectangular grid (tile, panels) `width` × `height`.
    Grid { width: f64, height: f64 },
}

impl SurfacePattern {
    /// Built-in patterns as (id, label, pattern), for the material's properties.
    pub fn presets() -> Vec<(&'static str, &'static str, SurfacePattern)> {
        const IN: f64 = 25.4;
        vec![
            ("none", "None", SurfacePattern::None),
            (
                "lap6",
                "Lap Siding 6\"",
                SurfacePattern::Lap { spacing: 6.0 * IN },
            ),
            (
                "lap8",
                "Lap Siding 8\"",
                SurfacePattern::Lap { spacing: 8.0 * IN },
            ),
            (
                "shingle5",
                "Shingle Courses 5\"",
                SurfacePattern::Lap { spacing: 5.0 * IN },
            ),
            (
                "brick",
                "Brick Running Bond",
                SurfacePattern::Running {
                    course: 8.0 / 3.0 * IN,
                    unit: 8.0 * IN,
                },
            ),
            (
                "block",
                "Block 8x16",
                SurfacePattern::Running {
                    course: 8.0 * IN,
                    unit: 16.0 * IN,
                },
            ),
            (
                "panel4x8",
                "Panels 4'x8'",
                SurfacePattern::Grid {
                    width: 48.0 * IN,
                    height: 96.0 * IN,
                },
            ),
            (
                "tile2x2",
                "Tile 2'x2'",
                SurfacePattern::Grid {
                    width: 24.0 * IN,
                    height: 24.0 * IN,
                },
            ),
        ]
    }
    pub fn preset_id(self) -> &'static str {
        Self::presets()
            .into_iter()
            .find(|(_, _, p)| *p == self)
            .map_or("custom", |(id, _, _)| id)
    }
}

/// Where a floor's or ceiling's boundary comes from.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SlabBound {
    /// The stored sketch.
    #[default]
    Sketch,
    /// The outer faces of the walls on its level (Floor: Pick Walls); follows them.
    Walls,
    /// The room enclosing `point` (Ceiling: Auto Room); follows its walls.
    Room { point: Pt },
}

/// How an elevation mark is drawn (the symbol of its family type, ADR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum MarkStyle {
    /// A round body with a filled arrowhead per view.
    #[default]
    CircleArrow,
    /// A round body whose half toward each view is filled, with a point.
    CircleHalf,
    /// A circle in a square turned 45°, the corner toward each view filled.
    Diamond,
}

impl MarkStyle {
    pub const ALL: [MarkStyle; 3] = [
        MarkStyle::CircleArrow,
        MarkStyle::CircleHalf,
        MarkStyle::Diamond,
    ];
    pub fn label(self) -> &'static str {
        match self {
            MarkStyle::CircleArrow => "Circle - Filled Arrow",
            MarkStyle::CircleHalf => "Circle - Filled Half",
            MarkStyle::Diamond => "Diamond - Filled Corners",
        }
    }
}

/// A 3D view's section box (mm, model coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SectionBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

/// Which line of a wall its drawn points follow (Revit's Location Line). The wall is
/// stored by its centerline; the location line decides how drawn points map to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LocationLine {
    #[default]
    Centerline,
    CoreCenterline,
    FinishExterior,
    FinishInterior,
    CoreExterior,
    CoreInterior,
}

impl LocationLine {
    pub const ALL: [LocationLine; 6] = [
        LocationLine::Centerline,
        LocationLine::CoreCenterline,
        LocationLine::FinishExterior,
        LocationLine::FinishInterior,
        LocationLine::CoreExterior,
        LocationLine::CoreInterior,
    ];
    pub fn label(self) -> &'static str {
        match self {
            LocationLine::Centerline => "Wall Centerline",
            LocationLine::CoreCenterline => "Core Centerline",
            LocationLine::FinishExterior => "Finish Face: Exterior",
            LocationLine::FinishInterior => "Finish Face: Interior",
            LocationLine::CoreExterior => "Core Face: Exterior",
            LocationLine::CoreInterior => "Core Face: Interior",
        }
    }
    pub fn parse(s: &str) -> Option<LocationLine> {
        LocationLine::ALL
            .into_iter()
            .find(|l| l.label() == s || format!("{l:?}") == s)
    }
}

/// Plan shape of a stair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum StairShape {
    #[default]
    Straight,
    /// Two runs at right angles with a square landing, turning left or right.
    LShaped { left: bool },
    /// Two runs side by side, doubling back at a landing.
    UShaped { left: bool },
}

/// A column's cross-section (mm).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ColumnShape {
    Rectangular {
        width: f64,
        depth: f64,
    },
    Round {
        diameter: f64,
    },
    /// Steel I-shape: overall depth, flange width, flange and web thickness.
    WideFlange {
        depth: f64,
        flange: f64,
        flange_t: f64,
        web_t: f64,
    },
}

/// A beam's cross-section (mm).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum BeamShape {
    Rectangular {
        width: f64,
        depth: f64,
    },
    WideFlange {
        depth: f64,
        flange: f64,
        flange_t: f64,
        web_t: f64,
    },
}

/// A view's crop region in view coordinates (mm).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CropBox {
    pub min: Pt,
    pub max: Pt,
}

impl CropBox {
    pub fn contains(&self, p: Pt) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

fn yes() -> bool {
    true
}

/// Built-in door families (ADR-033, see `doors`). `SingleFlush` and `DoubleFlush` are the
/// single and double swing families (their leaf style is a type option); the names stay
/// for files saved before door families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DoorFamily {
    SingleFlush,
    DoubleFlush,
    Sidelites,
    SlidingGlass,
    Pocket,
    Barn,
    Bifold,
    FoldingWall,
    Storefront,
    Garage,
}

/// Built-in window families: the common US window types (ADR-031, see `windows`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WindowFamily {
    Fixed,
    Casement,
    DoubleHung,
    SingleHung,
    Awning,
    Hopper,
    Slider,
    Slider3,
    PictureCasement,
    PictureAwning,
    Bay,
    Storefront,
}

fn one() -> u32 {
    1
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
    Columns,
    Beams,
    MaterialTakeoff,
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
    /// One direction of an elevation marker (ADR-021), looking `facing` (a North view looks
    /// north, at the room's north wall). Interior ones are cropped to the marker's room.
    MarkerElevation {
        marker: ElementId,
        facing: Compass,
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
        /// Total width, mm; equals the sum of `layers` when there are any.
        thickness: f64,
        function: WallFunction,
        /// Compound structure, exterior first. Empty = one homogeneous layer.
        #[serde(default)]
        layers: Vec<WallLayer>,
    },
    Wall {
        type_id: ElementId,
        /// Location line (centerline) endpoints.
        start: Pt,
        end: Pt,
        base_level: ElementId,
        base_offset: f64,
        top: WallTop,
        /// Which of the wall's lines its drawn points followed.
        #[serde(default)]
        location: LocationLine,
        /// The top follows the underside of the roof above (Revit's Attach Top).
        #[serde(default)]
        attach_top: bool,
    },
    FloorType {
        name: String,
        /// Total, mm; equals the sum of `layers` when there are any.
        thickness: f64,
        /// Layers from the top down. Empty = one homogeneous layer.
        #[serde(default)]
        layers: Vec<WallLayer>,
    },
    /// Floor slab whose top surface sits at `level + offset`.
    Floor {
        type_id: ElementId,
        level: ElementId,
        offset: f64,
        /// The sketch (or, when bound, the last shape it followed).
        boundary: Vec<Pt>,
        #[serde(default)]
        bound: SlabBound,
        /// Boundary loops as sketched (ADR-021); when present they define the outline.
        #[serde(default)]
        sketch: Vec<Vec<crate::sketch::SketchCurve>>,
    },
    CeilingType {
        name: String,
        thickness: f64,
        /// Layers from the top down.
        #[serde(default)]
        layers: Vec<WallLayer>,
    },
    /// Ceiling whose underside sits at `level + height`.
    Ceiling {
        type_id: ElementId,
        level: ElementId,
        height: f64,
        boundary: Vec<Pt>,
        #[serde(default)]
        bound: SlabBound,
        #[serde(default)]
        sketch: Vec<Vec<crate::sketch::SketchCurve>>,
    },
    View {
        name: String,
        kind: ViewKind,
        /// Drawing scale denominator, e.g. 48 for 1/4" = 1'-0".
        scale: u32,
        /// When set, the view shows only what lies inside this region.
        #[serde(default)]
        crop: Option<CropBox>,
        /// Whether the crop boundary is drawn (it is never printed).
        #[serde(default = "yes")]
        show_crop: bool,
        /// 3D views: the section box, when on.
        #[serde(default)]
        section_box: Option<SectionBox>,
        /// A callout (detail view) of this parent view (ADR-020).
        #[serde(default)]
        callout_of: Option<ElementId>,
        /// Building elevations: the elevation mark type drawn for them in plans (ADR-022).
        #[serde(default)]
        mark_type: Option<ElementId>,
        /// A site plan: shows topography contours and property lines (ADR-023).
        #[serde(default)]
        site: bool,
        /// Hide in View: elements and categories not shown in this view (ADR-024).
        #[serde(default)]
        hidden: Vec<ElementId>,
        #[serde(default)]
        hidden_categories: Vec<Category>,
        /// Camera views (ADR-027): the perspective eye and target.
        #[serde(default)]
        camera: Option<crate::camera::ViewCamera>,
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
        /// User-defined project parameters (ADR-017).
        #[serde(default)]
        param_defs: Vec<crate::params::ParamDef>,
    },
    DoorType {
        name: String,
        family: DoorFamily,
        /// Rough opening width and height, mm.
        width: f64,
        height: f64,
        /// Leaf style, panel count (0: the family's default) and finish (None: the
        /// family's default), ADR-033.
        #[serde(default)]
        leaf: crate::doors::LeafStyle,
        #[serde(default)]
        panels: u32,
        #[serde(default)]
        finish: Option<crate::doors::DoorFinish>,
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
        /// Units mulled side by side (ADR-031).
        #[serde(default = "one")]
        units: u32,
        #[serde(default)]
        grille: crate::windows::Grille,
        #[serde(default)]
        finish: crate::windows::FrameFinish,
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
        /// Length of the view title's rule, paper mm, when stretched on the sheet (ADR-039);
        /// None fits it to the title.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title_length: Option<f64>,
    },
    RoofType {
        name: String,
        /// Thickness measured square to the roof surface, mm.
        thickness: f64,
        /// Layers from the outside (top) in.
        #[serde(default)]
        layers: Vec<WallLayer>,
    },
    /// A roof by footprint: `boundary` is the eave outline (overhang included), the eaves
    /// sit at `level + offset`, and each edge flagged in `sloped` rises inward at `slope`
    /// (radians). Edges that don't slope become gable ends; with none it is a flat roof.
    Roof {
        type_id: ElementId,
        level: ElementId,
        offset: f64,
        boundary: Vec<Pt>,
        slope: f64,
        /// One flag per boundary edge (edge i runs from point i to point i + 1).
        sloped: Vec<bool>,
    },
    /// A straight stair run from `base_level` up to `top_level`, climbing from `start`
    /// toward `end` (only the direction of `end` matters: the run length follows from the
    /// riser count and tread depth). `start` is the center of the first riser.
    Stair {
        base_level: ElementId,
        top_level: ElementId,
        start: Pt,
        end: Pt,
        width: f64,
        /// Tread depth, mm.
        tread: f64,
        /// Largest allowed riser height, mm; the riser count is the smallest that fits.
        max_riser: f64,
        #[serde(default)]
        shape: StairShape,
        /// Risers in the first run of an L- or U-shaped stair (0 = half, rounded down).
        #[serde(default)]
        first_run: u32,
        /// Railings along both sides of each run.
        #[serde(default = "yes")]
        railings: bool,
    },
    ColumnType {
        name: String,
        shape: ColumnShape,
        /// Structural columns print as cut material; architectural ones as outlines.
        structural: bool,
        #[serde(default)]
        material: Option<ElementId>,
    },
    /// A vertical column centered at `at`, rotated `rotation` radians, from its base level
    /// (plus offset) up to its top constraint.
    Column {
        type_id: ElementId,
        base_level: ElementId,
        base_offset: f64,
        top: WallTop,
        at: Pt,
        rotation: f64,
    },
    BeamType {
        name: String,
        shape: BeamShape,
        #[serde(default)]
        material: Option<ElementId>,
    },
    /// A beam along `start` → `end` whose top sits at `level + offset`.
    Beam {
        type_id: ElementId,
        level: ElementId,
        offset: f64,
        start: Pt,
        end: Pt,
    },
    RailingType {
        name: String,
        /// Top of rail above the walking surface, mm.
        height: f64,
    },
    /// A railing along a sketched path on a level.
    Railing {
        type_id: ElementId,
        level: ElementId,
        offset: f64,
        path: Vec<Pt>,
    },
    /// A material: cut pattern, elevation surface pattern and shaded color (ADR-020).
    Material {
        name: String,
        cut: CutPattern,
        surface: SurfacePattern,
        /// sRGB.
        color: [u8; 3],
        /// How it renders (ADR-029): reflection, glossiness, texture, real-world scale…
        #[serde(default)]
        appearance: crate::library::Appearance,
    },
    /// An elevation marker placed in plan: up to four views, one per direction (Revit's
    /// Elevation tool; interior ones look at the walls of the room they're in).
    ElevationMarker {
        level: ElementId,
        at: Pt,
        interior: bool,
        /// Its family type (symbol and interior/building); None: the default of its kind.
        #[serde(default)]
        type_id: Option<ElementId>,
    },
    /// The project's lot (ADR-023): located at (lat, lon), bounded by `boundary` in its local
    /// east/north frame (mm), placed in the project by `offset` and `rotation`.
    Site {
        address: String,
        lat: f64,
        lon: f64,
        boundary: Vec<Pt>,
        parcel: crate::site::ParcelInfo,
        offset: Pt,
        /// Angle to true north, radians counter-clockwise.
        rotation: f64,
        /// Ground elevation (NAVD88, mm) at project height 0 (Level 1).
        base_elevation: f64,
        /// Contour interval, mm.
        contour: f64,
        #[serde(default)]
        topo: Option<crate::site::Topo>,
    },
    /// An elevation mark family type (ADR-022): interior or building, and its symbol.
    ElevationMarkerType {
        name: String,
        interior: bool,
        style: MarkStyle,
        /// Body radius on paper, mm.
        size: f64,
    },
    /// A room-bounding line on `level` for open plans (Revit's Room Separation Line).
    RoomSeparator {
        level: ElementId,
        start: Pt,
        end: Pt,
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
            ElementData::RoofType { .. } => Category::RoofType,
            ElementData::Roof { .. } => Category::Roof,
            ElementData::Stair { .. } => Category::Stair,
            ElementData::ColumnType { .. } => Category::ColumnType,
            ElementData::Column { .. } => Category::Column,
            ElementData::BeamType { .. } => Category::BeamType,
            ElementData::Beam { .. } => Category::Beam,
            ElementData::RailingType { .. } => Category::RailingType,
            ElementData::Railing { .. } => Category::Railing,
            ElementData::Material { .. } => Category::Material,
            ElementData::RoomSeparator { .. } => Category::RoomSeparator,
            ElementData::ElevationMarker { .. } => Category::ElevationMarker,
            ElementData::ElevationMarkerType { .. } => Category::ElevationMarkerType,
            ElementData::Site { .. } => Category::Site,
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
            ElementData::Room { level, .. }
            | ElementData::RoomSeparator { level, .. }
            | ElementData::ElevationMarker { level, .. } => {
                vec![*level]
            }
            ElementData::Roof { type_id, level, .. }
            | ElementData::Beam { type_id, level, .. }
            | ElementData::Railing { type_id, level, .. } => vec![*type_id, *level],
            ElementData::Column {
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
            ElementData::Stair {
                base_level,
                top_level,
                ..
            } => vec![*base_level, *top_level],
            ElementData::Dimension { view, .. } | ElementData::TextNote { view, .. } => vec![*view],
            ElementData::Viewport { sheet, view, .. } => vec![*sheet, *view],
            ElementData::Tag { view, target, .. } => vec![*view, *target],
            ElementData::Door { type_id, host, .. } | ElementData::Window { type_id, host, .. } => {
                vec![*type_id, *host]
            }
            ElementData::View {
                kind, callout_of, ..
            } => {
                // A callout goes with its parent view.
                let mut v: Vec<ElementId> = callout_of.iter().copied().collect();
                match kind {
                    ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level } => {
                        v.push(*level)
                    }
                    // The marker's views go with it.
                    ViewKind::MarkerElevation { marker, .. } => v.push(*marker),
                    _ => {}
                }
                v
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
            | ElementData::Stage { name, .. }
            | ElementData::RoofType { name, .. }
            | ElementData::ColumnType { name, .. }
            | ElementData::BeamType { name, .. }
            | ElementData::RailingType { name, .. }
            | ElementData::Material { name, .. }
            | ElementData::ElevationMarkerType { name, .. } => name.clone(),
            ElementData::RoomSeparator { .. } => "Room Separator".into(),
            ElementData::Site { address, .. } => {
                if address.is_empty() {
                    "Site".into()
                } else {
                    format!("Site: {address}")
                }
            }
            ElementData::ElevationMarker { interior, .. } => if *interior {
                "Interior Elevation Mark"
            } else {
                "Building Elevation Mark"
            }
            .into(),
            ElementData::Column { .. } => "Column".into(),
            ElementData::Beam { .. } => "Beam".into(),
            ElementData::Railing { .. } => "Railing".into(),
            ElementData::Roof { .. } => "Roof".into(),
            ElementData::Stair { .. } => "Stair".into(),
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
            | ElementData::Room { level, .. }
            | ElementData::Roof { level, .. }
            | ElementData::Beam { level, .. }
            | ElementData::Railing { level, .. }
            | ElementData::RoomSeparator { level, .. }
            | ElementData::ElevationMarker { level, .. } => Some(*level),
            ElementData::Stair { base_level, .. } | ElementData::Column { base_level, .. } => {
                Some(*base_level)
            }
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
            | ElementData::Window { type_id, .. }
            | ElementData::Roof { type_id, .. }
            | ElementData::Column { type_id, .. }
            | ElementData::Beam { type_id, .. }
            | ElementData::Railing { type_id, .. } => Some(*type_id),
            ElementData::ElevationMarker { type_id, .. } => *type_id,
            _ => None,
        }
    }

    /// A new view with no crop region.
    pub fn view(name: impl Into<String>, kind: ViewKind, scale: u32) -> Self {
        ElementData::View {
            name: name.into(),
            kind,
            scale,
            crop: None,
            show_crop: true,
            section_box: None,
            callout_of: None,
            mark_type: None,
            site: false,
            hidden: vec![],
            hidden_categories: vec![],
            camera: None,
        }
    }

    /// Geometric sanity checks, run on every element a transaction touches.
    pub fn validate(&self) -> Result<(), crate::CoreError> {
        let bad = |m: &str| Err(crate::CoreError::Invalid(m.into()));
        match self {
            ElementData::RoomSeparator { start, end, .. } if start.dist(*end) < 1.0 => {
                bad("room separator is too short")
            }
            ElementData::Material { name, .. } if name.trim().is_empty() => {
                bad("a material needs a name")
            }
            ElementData::View {
                section_box: Some(b),
                ..
            } if (0..3).any(|i| b.max[i] - b.min[i] < 100.0) => bad("section box is too small"),
            ElementData::Wall { start, end, .. } if start.dist(*end) < 1.0 => {
                bad("wall is too short")
            }
            ElementData::Floor { boundary, .. }
            | ElementData::Ceiling { boundary, .. }
            | ElementData::Roof { boundary, .. }
                if boundary.len() < 3 || studio_geom::signed_area(boundary).abs() < 1.0 =>
            {
                bad("boundary must enclose an area")
            }
            ElementData::Roof {
                boundary,
                sloped,
                slope,
                ..
            } => {
                if sloped.len() != boundary.len() {
                    bad("each roof edge needs a slope setting")
                } else if !(0.0..1.4).contains(slope) {
                    bad("roof slope must be between 0° and 80°")
                } else {
                    Ok(())
                }
            }
            ElementData::WallType {
                thickness, layers, ..
            } if !layers.is_empty() => {
                let sum: f64 = layers.iter().map(|l| l.thickness).sum();
                if layers.iter().any(|l| l.thickness < 0.0) {
                    bad("layer thickness can't be negative")
                } else if (sum - thickness).abs() > 0.01 {
                    bad("wall type width must equal the sum of its layers")
                } else if *thickness < 1.0 {
                    bad("wall type is too thin")
                } else {
                    Ok(())
                }
            }
            ElementData::Stair {
                start,
                end,
                width,
                tread,
                max_riser,
                ..
            } => {
                if start.dist(*end) < 1.0 {
                    bad("stair needs a direction")
                } else if *width < 300.0 {
                    bad("stair is too narrow")
                } else if *tread < 100.0 || *max_riser < 50.0 {
                    bad("stair treads and risers are too small")
                } else {
                    Ok(())
                }
            }
            ElementData::View { crop: Some(c), .. }
                if c.max.x - c.min.x < 10.0 || c.max.y - c.min.y < 10.0 =>
            {
                bad("crop region is too small")
            }
            ElementData::Beam { start, end, .. } if start.dist(*end) < 10.0 => {
                bad("beam is too short")
            }
            ElementData::Railing { path, .. }
                if path.len() < 2
                    || path.windows(2).map(|w| w[0].dist(w[1])).sum::<f64>() < 10.0 =>
            {
                bad("railing path is too short")
            }
            ElementData::ColumnType { shape, .. } => {
                let ok = match shape {
                    ColumnShape::Rectangular { width, depth } => *width > 1.0 && *depth > 1.0,
                    ColumnShape::Round { diameter } => *diameter > 1.0,
                    ColumnShape::WideFlange {
                        depth,
                        flange,
                        flange_t,
                        web_t,
                    } => {
                        *web_t > 0.0
                            && *web_t < *flange
                            && *flange_t > 0.0
                            && 2.0 * flange_t < *depth
                    }
                };
                if ok {
                    Ok(())
                } else {
                    bad("column size doesn't make a section")
                }
            }
            ElementData::BeamType { shape, .. } => {
                let ok = match shape {
                    BeamShape::Rectangular { width, depth } => *width > 1.0 && *depth > 1.0,
                    BeamShape::WideFlange {
                        depth,
                        flange,
                        flange_t,
                        web_t,
                    } => {
                        *web_t > 0.0
                            && *web_t < *flange
                            && *flange_t > 0.0
                            && 2.0 * flange_t < *depth
                    }
                };
                if ok {
                    Ok(())
                } else {
                    bad("beam size doesn't make a section")
                }
            }
            ElementData::RailingType { height, .. } if *height < 300.0 => bad("railing is too low"),
            ElementData::FloorType {
                thickness, layers, ..
            }
            | ElementData::CeilingType {
                thickness, layers, ..
            }
            | ElementData::RoofType {
                thickness, layers, ..
            } if !layers.is_empty()
                && (layers.iter().map(|l| l.thickness).sum::<f64>() - thickness).abs() > 0.01 =>
            {
                bad("type thickness must equal the sum of its layers")
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    /// Increments on every committed modification (for sync).
    pub rev: u64,
    pub data: ElementData,
    /// Project parameter values by key (ADR-017); built-in parameters are fields of `data`.
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, crate::params::ParamValue>,
}

impl Element {
    pub fn new(data: ElementData) -> Self {
        Self {
            id: ElementId::new(),
            rev: 1,
            data,
            params: Default::default(),
        }
    }
    pub fn category(&self) -> Category {
        self.data.category()
    }
}
