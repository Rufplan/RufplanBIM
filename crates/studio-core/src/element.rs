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
    /// The project's drawing-set standards (ADR-047).
    Standards,
    /// The project manual (ADR-085).
    SpecBook,
    /// Model and detail groups (ADR-087).
    GroupType,
    /// Furniture and equipment types (ADR-090); placed pieces are Furniture or
    /// SpecialtyEquipment.
    FfeType,
    /// A rendered image saved to the project (ADR-095).
    RenderImage,
    Group,
    /// Annotation symbols (ADR-048).
    SpotElevation,
    NorthArrow,
    GraphicScale,
    KeyPlan,
    SpotSlope,
    /// Lines (ADR-054).
    DetailLine,
    ModelLine,
    /// Lighting fixtures (ADR-057).
    LightingFixtureType,
    LightingFixture,
    /// Wall openings (ADR-058).
    WallOpening,
    /// Planting and ground regions (ADR-064).
    PlantingType,
    Planting,
    GroundRegion,
    /// Painted grass (ADR-065).
    GrassPatch,
    /// Categories only in-place families have yet (ADR-068).
    GenericModel,
    Furniture,
    Casework,
    SpecialtyEquipment,
    PlumbingFixture,
    /// Filled regions in drafting views and details (ADR-069).
    FilledRegion,
    /// Detail components (ADR-071).
    DetailComponent,
    /// Reference sections and callouts (ADR-076).
    ViewReference,
    /// Worksets (ADR-079).
    Workset,
    /// The structural layer (ADR-080).
    StructuralScheme,
    /// Keynotes (ADR-081): the project's table, and tags in views.
    KeynoteTable,
    KeynoteTag,
    /// The MEPT layers (ADR-082).
    MepScheme,
    /// A location or vicinity map on a sheet (ADR-107); its imagery is fetched when shown.
    MapFrame,
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
            Category::Standards => "Standards",
            Category::SpecBook => "SpecBook",
            Category::GroupType => "GroupType",
            Category::FfeType => "FfeType",
            Category::RenderImage => "RenderImage",
            Category::Group => "Group",
            Category::SpotElevation => "SpotElevation",
            Category::NorthArrow => "NorthArrow",
            Category::GraphicScale => "GraphicScale",
            Category::KeyPlan => "KeyPlan",
            Category::SpotSlope => "SpotSlope",
            Category::DetailLine => "DetailLine",
            Category::ModelLine => "ModelLine",
            Category::LightingFixtureType => "LightingFixtureType",
            Category::LightingFixture => "LightingFixture",
            Category::WallOpening => "WallOpening",
            Category::PlantingType => "PlantingType",
            Category::Planting => "Planting",
            Category::GroundRegion => "GroundRegion",
            Category::GrassPatch => "GrassPatch",
            Category::GenericModel => "GenericModel",
            Category::Furniture => "Furniture",
            Category::Casework => "Casework",
            Category::SpecialtyEquipment => "SpecialtyEquipment",
            Category::PlumbingFixture => "PlumbingFixture",
            Category::FilledRegion => "FilledRegion",
            Category::DetailComponent => "DetailComponent",
            Category::ViewReference => "ViewReference",
            Category::Workset => "Workset",
            Category::StructuralScheme => "StructuralScheme",
            Category::KeynoteTable => "KeynoteTable",
            Category::KeynoteTag => "KeynoteTag",
            Category::MepScheme => "MepScheme",
            Category::MapFrame => "MapFrame",
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
        const VERTICAL: f64 = 100_000.0;
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
                "asphalt",
                "Shingle Courses 5-5/8\"",
                SurfacePattern::Lap {
                    spacing: 5.625 * IN,
                },
            ),
            (
                "tile13",
                "Roof Tile Courses 13\"",
                SurfacePattern::Lap { spacing: 13.0 * IN },
            ),
            // Vertical boards and seams (ADR-061): grid columns whose rows are too tall to
            // meet.
            (
                "vert2",
                "Vertical Slats 2\"",
                SurfacePattern::Grid {
                    width: 2.0 * IN,
                    height: VERTICAL,
                },
            ),
            (
                "vert6",
                "Vertical Boards 6\"",
                SurfacePattern::Grid {
                    width: 6.0 * IN,
                    height: VERTICAL,
                },
            ),
            (
                "batten12",
                "Board and Batten 12\"",
                SurfacePattern::Grid {
                    width: 12.0 * IN,
                    height: VERTICAL,
                },
            ),
            (
                "seam16",
                "Standing Seam 16\"",
                SurfacePattern::Grid {
                    width: 16.0 * IN,
                    height: VERTICAL,
                },
            ),
            (
                "seam18",
                "Standing Seam 18\"",
                SurfacePattern::Grid {
                    width: 18.0 * IN,
                    height: VERTICAL,
                },
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

/// A view's Detail Level (ADR-067), as in Revit: how much of each element it draws.
/// Coarse: walls in solid poché, doors and windows as bare openings. Medium: wall core
/// boundaries and frames. Fine: every layer with its cut pattern, casings and muntins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DetailLevel {
    Coarse,
    Medium,
    Fine,
}

impl DetailLevel {
    pub const ALL: [DetailLevel; 3] = [DetailLevel::Coarse, DetailLevel::Medium, DetailLevel::Fine];

    /// The level a view has until one is chosen: Fine at 1/4" = 1'-0" and larger (what
    /// such views always drew), Coarse smaller, like Revit's templates.
    pub fn for_scale(scale: u32) -> Self {
        if scale <= 50 {
            DetailLevel::Fine
        } else {
            DetailLevel::Coarse
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DetailLevel::Coarse => "Coarse",
            DetailLevel::Medium => "Medium",
            DetailLevel::Fine => "Fine",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|d| d.label().eq_ignore_ascii_case(s.trim()))
    }
}

/// Where a level's line starts and ends in one elevation or section (ADR-052), along the
/// view (its x, mm). None: that end where the view puts it.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct LevelEnds {
    pub level: ElementId,
    pub left: Option<f64>,
    pub right: Option<f64>,
}

/// A floor's slope (ADR-049), like Revit's slope arrow: it falls `rise` per unit of run
/// toward plan direction `dir` (radians, counter-clockwise from east), and its top is at the
/// level plus offset along its highest edge. A rise of 0 is a flat floor.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct FloorSlope {
    pub rise: f64,
    pub dir: f64,
}

impl FloorSlope {
    pub fn is_flat(&self) -> bool {
        self.rise.abs() < 1e-9
    }
    /// The unit plan direction it falls toward.
    pub fn down(&self) -> Pt {
        Pt::new(self.dir.cos(), self.dir.sin())
    }
}

/// How a spot slope writes its value (ADR-049).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SlopeFormat {
    /// Roofs as rise over 12", floors and ground as a percent (a ratio at ramp slopes).
    #[default]
    Auto,
    /// 6" / 12".
    RisePer12,
    /// 2.00%.
    Percent,
    /// 1:12.
    Ratio,
    /// 26.57°.
    Degrees,
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

fn full() -> f64 {
    1.0
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "DimAnchor")]
pub enum Anchor {
    /// `t` is the fraction along the wall's location line from its start; `side` the signed
    /// distance to the left of it (e.g. ±half the thickness for a face).
    Wall { wall: ElementId, t: f64, side: f64 },
    /// `t` is the fraction along the grid line from its start.
    Grid { grid: ElementId, t: f64 },
    /// A detail line (ADR-072): `t` is the fraction along it from its start.
    DetailLine { line: ElementId, t: f64 },
    /// A detail component (ADR-072): a point in its own frame (mm along it from its start,
    /// and to its left, or right when flipped), so it follows the component.
    Component {
        component: ElementId,
        u: f64,
        v: f64,
    },
}

/// A reference of a dimension string between its first and last (ADR-040).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DimRef {
    pub at: Pt,
    #[serde(default)]
    pub anchor: Option<Anchor>,
}

/// How a dimension measures (ADR-040): Revit's Aligned (across the references it was
/// picked from) or Linear (horizontal or vertical).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DimKind {
    #[default]
    Aligned,
    Linear,
}

impl DimKind {
    pub fn is_aligned(&self) -> bool {
        *self == DimKind::Aligned
    }
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
    /// Revit's Keynote Legend (ADR-081): the keynotes used, filtered to the sheet it's on.
    Keynotes,
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
    /// Revit's drafting view (ADR-069): a 2D sheet of detail lines, filled regions and
    /// text at a scale, with no model in it (typical details).
    Drafting,
    /// A rendering saved to the project (ADR-095), like Revit's Renderings: the image
    /// element it shows, placeable on sheets.
    Rendering {
        image: ElementId,
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
        /// Flat unless set (ADR-049): sidewalks, ramps, sloped slabs.
        #[serde(default)]
        slope: FloorSlope,
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
        /// Elevations and sections (ADR-052): levels whose ends were dragged in this view
        /// (Revit's 2D extents). Levels not listed span the view.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        level_ends: Vec<LevelEnds>,
        /// Detail Level (ADR-067); None follows the scale ([`DetailLevel::for_scale`]).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail_level: Option<DetailLevel>,
        /// Visibility/Graphics > Worksets: worksets hidden in this view (ADR-079).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        hidden_worksets: Vec<ElementId>,
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
        /// Revit's Sun Settings (ADR-057).
        #[serde(default)]
        sun: crate::lighting::SunSettings,
        /// The base ground's material (ADR-064): the topography, or the ground around the
        /// model without one. None: plain lawn.
        #[serde(default)]
        ground: Option<ElementId>,
        /// Everything else about the job (ADR-084): client, team, budget, codes…
        #[serde(default)]
        details: Box<crate::project::ProjectDetails>,
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
        /// References between the first (`a`) and last (`b`) of a dimension string, in
        /// order along it (ADR-040).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        between: Vec<DimRef>,
        /// The direction it measures along (unit): across the parallel references it was
        /// picked from, or horizontal or vertical for a linear dimension. None measures from
        /// `a` toward `b`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        along: Option<Pt>,
        #[serde(default, skip_serializing_if = "DimKind::is_aligned")]
        kind: DimKind,
    },
    /// The angle between two lines (walls or grids) in a view (ADR-040). Each line runs
    /// through its point in its direction, following its element when anchored; the arc
    /// passes through `at`, in the angle between the lines that holds it.
    AngularDimension {
        view: ElementId,
        a: Pt,
        a_dir: Pt,
        #[serde(default)]
        a_ref: Option<Anchor>,
        b: Pt,
        b_dir: Pt,
        #[serde(default)]
        b_ref: Option<Anchor>,
        at: Pt,
    },
    /// A text note in a view, `at` in the view's coordinates.
    TextNote {
        view: ElementId,
        /// Its first line's anchor (left end, middle or right end by `align`), at the line's
        /// vertical middle.
        at: Pt,
        /// Lines are separated by \n; longer lines wrap at `width`.
        text: String,
        /// Printed text height, paper mm.
        #[serde(default = "default_text_size")]
        size: f64,
        /// Revit's leaders (ADR-070).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        leaders: Vec<crate::text::Leader>,
        #[serde(default)]
        align: crate::text::TextAlign,
        /// Wrap width, paper mm; None: the lines as typed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        /// Rotation about `at`, radians counter-clockwise (ADR-108).
        #[serde(default)]
        angle: f64,
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
        /// Where the title was moved to (Shift + drag), paper mm from its place under the
        /// view (ADR-039); None keeps it there.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title_offset: Option<Pt>,
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
        /// The trim swept around its edge (ADR-095), if any.
        #[serde(default)]
        fascia: Option<crate::fascia::FasciaSpec>,
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
    /// The project's drawing-set standards (ADR-047): one, made on the first edit.
    Standards(crate::standards::Standards),
    /// The project manual (ADR-085): one, made when it's first generated.
    SpecBook(Box<crate::specs::SpecBook>),
    /// A rendered image (ADR-095): JPEG or PNG bytes as base64, its pixel size, and the
    /// paper width it prints at on a sheet (mm).
    RenderImage {
        name: String,
        mime: String,
        data: String,
        width: u32,
        height: u32,
        paper_width: f64,
    },
    /// A map on a sheet (ADR-107): the location or vicinity of `(lat, lon)` at `zoom` in the
    /// box `min`–`max` (paper mm), with `label` (the address) over it. Only these are kept:
    /// the imagery comes from Google when the sheet is shown or printed, never saved.
    MapFrame {
        sheet: ElementId,
        kind: crate::maps::MapKind,
        min: Pt,
        max: Pt,
        lat: f64,
        lon: f64,
        zoom: u32,
        label: String,
    },
    /// A furniture or equipment type (ADR-090).
    FfeType {
        name: String,
        spec: crate::ffe::FfeSpec,
    },
    /// A placed piece of furniture or equipment: `at` its center, `rotation` its facing,
    /// `offset` its base above the level (wall and counter pieces).
    Ffe {
        type_id: ElementId,
        class: crate::ffe::FfeClass,
        level: ElementId,
        at: Pt,
        rotation: f64,
        offset: f64,
    },
    /// A group type (ADR-087): Revit's model or detail group definition.
    GroupType {
        name: String,
        kind: crate::groups::GroupKind,
    },
    /// A placed group: its members are real elements; `origin`, `angle` and `mirrored`
    /// place it, so edits carry to the other instances placed the same way.
    Group {
        type_id: ElementId,
        origin: Pt,
        angle: f64,
        mirrored: bool,
        /// A model group's level; a detail group's view.
        level: Option<ElementId>,
        view: Option<ElementId>,
        members: Vec<ElementId>,
    },
    /// A spot elevation (ADR-048): the height of the model at `at` (view coordinates), its
    /// symbol and text at `leader` (the same point: no leader). Follows the model.
    SpotElevation {
        view: ElementId,
        at: Pt,
        leader: Pt,
    },
    /// A spot slope (ADR-049): the slope of the roof, floor or ground at `at` (view
    /// coordinates), shown as an arrow (or, in elevations and sections, a triangle).
    SpotSlope {
        view: ElementId,
        at: Pt,
        #[serde(default)]
        format: SlopeFormat,
        #[serde(default)]
        triangle: bool,
    },
    /// A detail line (ADR-054): a line or arc drawn in one view (view coordinates).
    DetailLine {
        view: ElementId,
        curve: crate::sketch::SketchCurve,
        #[serde(default)]
        style: crate::lines::LineStyle,
    },
    /// A model line (ADR-054): a line or arc on `level`'s work plane, seen in its plans,
    /// in elevations and sections, and in 3D.
    ModelLine {
        level: ElementId,
        curve: crate::sketch::SketchCurve,
        #[serde(default)]
        style: crate::lines::LineStyle,
    },
    /// A hole of any shape cut through `host` (ADR-058): its sketch's loops in the wall's
    /// frame (x along the location line from the start, y up from the base).
    WallOpening {
        host: ElementId,
        sketch: Vec<Vec<crate::sketch::SketchCurve>>,
    },
    /// A lighting fixture type (ADR-057): its body and Revit photometrics.
    LightingFixtureType {
        name: String,
        spec: crate::lighting::FixtureSpec,
    },
    /// A lighting fixture on `level` at `at`, its mounting point `elevation` above the
    /// level (the ceiling, a wall height or the floor), rotated `rotation` radians (a wall
    /// fixture faces that way). It can be off or dimmed (1 = full).
    LightingFixture {
        type_id: ElementId,
        level: ElementId,
        at: Pt,
        elevation: f64,
        rotation: f64,
        #[serde(default = "yes")]
        on: bool,
        #[serde(default = "full")]
        dimming: f64,
    },
    /// A planting type (ADR-064): a species from the Asset Library, its size and look.
    PlantingType {
        name: String,
        spec: crate::planting::PlantSpec,
    },
    /// A tree, shrub or grass on `level` at `at` (on the ground where the topography is
    /// under a level at grade), `offset` above it, turned `rotation` radians and sized
    /// `scale` times its type.
    Planting {
        type_id: ElementId,
        level: ElementId,
        at: Pt,
        #[serde(default)]
        offset: f64,
        #[serde(default)]
        rotation: f64,
        #[serde(default = "full")]
        scale: f64,
    },
    /// Revit's topography subregion (ADR-064): an area of the ground (a drive, a lawn, a
    /// bed) finished in `material`, sketched on `level` and laid over the topography.
    GroundRegion {
        level: ElementId,
        material: ElementId,
        boundary: Vec<Pt>,
        #[serde(default)]
        sketch: Vec<Vec<crate::sketch::SketchCurve>>,
    },
    /// Grass painted with D5's brush (ADR-065): a stroke of round dabs (x, y, the surface's z,
    /// radius) on `level`, growing as `spec`.
    GrassPatch {
        level: ElementId,
        dabs: Vec<[f64; 4]>,
        spec: crate::grass::GrassSpec,
    },
    /// A filled region (ADR-069): an area of a view hatched with a pattern (concrete, earth,
    /// insulation…), its first loop the outline and the rest holes.
    FilledRegion {
        view: ElementId,
        boundary: Vec<Vec<Pt>>,
        pattern: crate::details::FillPattern,
        /// Its boundary drawn in this line style; None: invisible, as Revit's detail
        /// components mostly are.
        #[serde(default)]
        outline: Option<crate::lines::LineStyle>,
    },
    /// A detail component (ADR-071), after Revit's detail items: a 2D family of type
    /// `type_key` (studio-core `details::components`), from `start` to `end`, or at
    /// `start` turned toward `end`.
    DetailComponent {
        view: ElementId,
        type_key: String,
        start: Pt,
        end: Pt,
        #[serde(default)]
        flip: bool,
    },
    /// A MEPT layer (ADR-082): one discipline's system, settings and preliminary layout, on
    /// the MEP workset.
    MepScheme {
        settings: crate::mep::MepSettings,
        layout: crate::mep::MepLayout,
    },
    /// The project's keynote table (ADR-081): Revit's keynote file, kept in the project.
    KeynoteTable {
        entries: Vec<crate::keynotes::Keynote>,
        numbering: crate::keynotes::KeynoteNumbering,
    },
    /// A keynote tag (ADR-081) in `view`: its box at `at`, its leader's arrow at `arrow`.
    KeynoteTag {
        view: ElementId,
        source: crate::keynotes::KeynoteSource,
        at: Pt,
        #[serde(default)]
        arrow: Option<Pt>,
        #[serde(default)]
        style: crate::keynotes::KeynoteStyle,
    },
    /// A workset (ADR-079), as Revit's user-created worksets.
    Workset {
        name: String,
        role: crate::worksets::WorksetRole,
        visible_in_all_views: bool,
    },
    /// The structural layer (ADR-080): the scheme picked from Suggest Structure, its
    /// settings and the preliminary layout generated for it, on the Structural workset.
    StructuralScheme {
        settings: crate::structural::SchemeSettings,
        layout: crate::structural::StructLayout,
    },
    /// Revit's reference section or callout (ADR-076): a mark drawn in `view` that points
    /// at `target`, a view already made (a drafting view, section or callout), instead of
    /// making a new one. Its head shows where the target is placed on the sheets.
    ViewReference {
        view: ElementId,
        target: ElementId,
        shape: crate::references::RefShape,
    },
    /// Model In-Place (ADR-068): a one-off element modelled from forms (extrusions, blends,
    /// sweeps and voids), in the category chosen for it: it is a wall, a door, furniture…
    /// for visibility, filters, schedules and IFC, as in Revit.
    InPlace {
        name: String,
        category: Category,
        level: ElementId,
        #[serde(default)]
        material: Option<ElementId>,
        forms: Vec<crate::inplace::Form>,
    },
    /// A north arrow (ADR-048) in a plan or on a sheet, centered at `at`.
    NorthArrow {
        view: ElementId,
        at: Pt,
    },
    /// A graphic scale (ADR-048) in a view, its left end at `at`; its divisions follow the
    /// view's scale.
    GraphicScale {
        view: ElementId,
        at: Pt,
    },
    /// A key plan on a sheet (ADR-048), centered at `at` (paper mm), `width` wide: the
    /// building's outline with the area the sheet's plans show shaded.
    KeyPlan {
        sheet: ElementId,
        at: Pt,
        width: f64,
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
            ElementData::Dimension { .. } | ElementData::AngularDimension { .. } => {
                Category::Dimension
            }
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
            ElementData::Standards(_) => Category::Standards,
            ElementData::SpecBook(_) => Category::SpecBook,
            ElementData::GroupType { .. } => Category::GroupType,
            ElementData::FfeType { .. } => Category::FfeType,
            ElementData::RenderImage { .. } => Category::RenderImage,
            ElementData::MapFrame { .. } => Category::MapFrame,
            ElementData::Ffe { class, .. } => class.category(),
            ElementData::Group { .. } => Category::Group,
            ElementData::SpotElevation { .. } => Category::SpotElevation,
            ElementData::NorthArrow { .. } => Category::NorthArrow,
            ElementData::GraphicScale { .. } => Category::GraphicScale,
            ElementData::KeyPlan { .. } => Category::KeyPlan,
            ElementData::SpotSlope { .. } => Category::SpotSlope,
            ElementData::DetailLine { .. } => Category::DetailLine,
            ElementData::ModelLine { .. } => Category::ModelLine,
            ElementData::LightingFixtureType { .. } => Category::LightingFixtureType,
            ElementData::LightingFixture { .. } => Category::LightingFixture,
            ElementData::WallOpening { .. } => Category::WallOpening,
            ElementData::PlantingType { .. } => Category::PlantingType,
            ElementData::Planting { .. } => Category::Planting,
            ElementData::GroundRegion { .. } => Category::GroundRegion,
            ElementData::GrassPatch { .. } => Category::GrassPatch,
            ElementData::InPlace { category, .. } => *category,
            ElementData::FilledRegion { .. } => Category::FilledRegion,
            ElementData::DetailComponent { .. } => Category::DetailComponent,
            ElementData::ViewReference { .. } => Category::ViewReference,
            ElementData::Workset { .. } => Category::Workset,
            ElementData::KeynoteTable { .. } => Category::KeynoteTable,
            ElementData::KeynoteTag { .. } => Category::KeynoteTag,
            ElementData::MepScheme { .. } => Category::MepScheme,
            ElementData::StructuralScheme { .. } => Category::StructuralScheme,
        }
    }

    /// A dimension's anchors (its ends and every reference between).
    pub fn dimension_anchors(&self) -> Vec<&Option<Anchor>> {
        match self {
            ElementData::Dimension {
                a_ref,
                b_ref,
                between,
                ..
            } => {
                let mut v = vec![a_ref, b_ref];
                v.extend(between.iter().map(|r| &r.anchor));
                v
            }
            ElementData::AngularDimension { a_ref, b_ref, .. } => vec![a_ref, b_ref],
            _ => vec![],
        }
    }

    /// A dimension's anchors (its ends and every reference between), to retarget or flip
    /// them when their elements are copied, split or flipped.
    pub fn dimension_anchors_mut(&mut self) -> Vec<&mut Option<Anchor>> {
        match self {
            ElementData::Dimension {
                a_ref,
                b_ref,
                between,
                ..
            } => {
                let mut v = vec![a_ref, b_ref];
                v.extend(between.iter_mut().map(|r| &mut r.anchor));
                v
            }
            ElementData::AngularDimension { a_ref, b_ref, .. } => vec![a_ref, b_ref],
            _ => vec![],
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
            ElementData::LightingFixture { type_id, level, .. } => vec![*type_id, *level],
            ElementData::WallOpening { host, .. } => vec![*host],
            ElementData::Planting { type_id, level, .. } => vec![*type_id, *level],
            ElementData::GroundRegion { level, .. } | ElementData::GrassPatch { level, .. } => {
                vec![*level]
            }
            ElementData::InPlace {
                level, material, ..
            } => std::iter::once(*level).chain(*material).collect(),
            ElementData::Dimension { view, .. }
            | ElementData::AngularDimension { view, .. }
            | ElementData::TextNote { view, .. }
            | ElementData::SpotElevation { view, .. }
            | ElementData::SpotSlope { view, .. }
            | ElementData::DetailLine { view, .. }
            | ElementData::FilledRegion { view, .. }
            | ElementData::DetailComponent { view, .. }
            | ElementData::ModelLine { level: view, .. }
            | ElementData::NorthArrow { view, .. }
            | ElementData::GraphicScale { view, .. }
            | ElementData::KeyPlan { sheet: view, .. } => vec![*view],
            ElementData::Viewport { sheet, view, .. } => vec![*sheet, *view],
            ElementData::Tag { view, target, .. } => vec![*view, *target],
            // Deleting either view deletes the reference, as in Revit.
            ElementData::ViewReference { view, target, .. } => vec![*view, *target],
            // A keynote goes with its view and what it tags.
            ElementData::KeynoteTag { view, source, .. } => match source {
                crate::keynotes::KeynoteSource::Element { target } => vec![*view, *target],
                crate::keynotes::KeynoteSource::Material { target, material } => {
                    vec![*view, *target, *material]
                }
                crate::keynotes::KeynoteSource::User { .. } => vec![*view],
            },
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
            ElementData::Standards(_) => "Drawing Set Standards".into(),
            ElementData::SpecBook(_) => "Project Manual".into(),
            ElementData::GroupType { name, .. } => name.clone(),
            ElementData::FfeType { name, .. } => name.clone(),
            ElementData::RenderImage { name, .. } => name.clone(),
            ElementData::MapFrame { kind, .. } => kind.label().into(),
            ElementData::Ffe { class, .. } => class.label().into(),
            ElementData::Group { .. } => "Group".into(),
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
            ElementData::AngularDimension { .. } => "Angular Dimension".into(),
            ElementData::TextNote { text, .. } => text.clone(),
            ElementData::SpotElevation { .. } => "Spot Elevation".into(),
            ElementData::NorthArrow { .. } => "North Arrow".into(),
            ElementData::GraphicScale { .. } => "Graphic Scale".into(),
            ElementData::KeyPlan { .. } => "Key Plan".into(),
            ElementData::SpotSlope { .. } => "Spot Slope".into(),
            ElementData::DetailLine { style, .. } => format!("Detail Line: {}", style.label()),
            ElementData::ModelLine { style, .. } => format!("Model Line: {}", style.label()),
            ElementData::LightingFixtureType { name, .. } => name.clone(),
            ElementData::LightingFixture { .. } => "Lighting Fixture".into(),
            ElementData::WallOpening { .. } => "Wall Opening".into(),
            ElementData::PlantingType { name, .. } => name.clone(),
            ElementData::Planting { .. } => "Planting".into(),
            ElementData::GroundRegion { .. } => "Ground Region".into(),
            ElementData::GrassPatch { spec, .. } => format!("Grass: {}", spec.kind.label()),
            ElementData::InPlace { name, .. } => name.clone(),
            ElementData::FilledRegion { pattern, .. } => {
                format!("Filled Region: {}", pattern.label())
            }
            ElementData::DetailComponent { type_key, .. } => {
                match crate::details::components::type_of(type_key) {
                    Some(t) => format!("{} : {}", t.family.label(), t.name),
                    None => "Detail Component".into(),
                }
            }
            ElementData::Workset { name, .. } => name.clone(),
            ElementData::KeynoteTable { .. } => "Keynote Table".into(),
            ElementData::MepScheme { settings, .. } => {
                format!("{} Layer: {}", settings.discipline.label(), settings.system)
            }
            ElementData::KeynoteTag { source, .. } => match source {
                crate::keynotes::KeynoteSource::Element { .. } => "Element Keynote".into(),
                crate::keynotes::KeynoteSource::Material { .. } => "Material Keynote".into(),
                crate::keynotes::KeynoteSource::User { .. } => "User Keynote".into(),
            },
            ElementData::StructuralScheme { settings, .. } => {
                format!("Structural Layer: {}", settings.kind.label())
            }
            ElementData::ViewReference { shape, .. } => match shape {
                crate::references::RefShape::Section { .. } => "Reference Section".into(),
                crate::references::RefShape::Callout { .. } => "Reference Callout".into(),
            },
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
            | ElementData::ElevationMarker { level, .. }
            | ElementData::LightingFixture { level, .. }
            | ElementData::Ffe { level, .. }
            | ElementData::Planting { level, .. }
            | ElementData::GroundRegion { level, .. }
            | ElementData::GrassPatch { level, .. }
            | ElementData::InPlace { level, .. } => Some(*level),
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
            | ElementData::Railing { type_id, .. }
            | ElementData::LightingFixture { type_id, .. }
            | ElementData::Planting { type_id, .. }
            | ElementData::Ffe { type_id, .. } => Some(*type_id),
            ElementData::ElevationMarker { type_id, .. } => *type_id,
            _ => None,
        }
    }

    /// A new view with no crop region.
    pub fn view(name: impl Into<String>, kind: ViewKind, scale: u32) -> Self {
        // Room separation lines never print on plans by default (ADR-106); Visibility/
        // Graphics shows them.
        let hidden_categories = match kind {
            ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. } => {
                vec![Category::RoomSeparator]
            }
            _ => vec![],
        };
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
            hidden_categories,
            camera: None,
            level_ends: vec![],
            detail_level: None,
            hidden_worksets: vec![],
        }
    }

    /// A view's Detail Level: the one chosen, else the one its scale gives (ADR-067).
    pub fn detail_level(&self) -> Option<DetailLevel> {
        match self {
            ElementData::View {
                detail_level,
                scale,
                ..
            } => Some(detail_level.unwrap_or(DetailLevel::for_scale(*scale))),
            _ => None,
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
