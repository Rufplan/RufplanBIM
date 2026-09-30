//! The structural layer's saved data (ADR-080): the scheme picked from Suggest Structure,
//! its settings, and the preliminary layout generated for it. It is one element on the
//! Structural workset and never changes the architectural model. The analysis (features,
//! scoring, layout, sizing) is the `studio-structural` crate's.
//!
//! Preliminary — not engineered. Requires review by a licensed structural engineer.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::element::ElementId;

/// The label every proposal, overlay and export carries.
pub const DISCLAIMER: &str =
    "Preliminary — not engineered. Requires review by a licensed structural engineer.";

/// The candidate structural systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SchemeKind {
    /// Light wood frame with wood structural panel shear walls.
    LightWood,
    /// Cold-formed steel bearing walls with CFS shear walls or straps.
    ColdFormedSteel,
    /// Concrete podium below, wood or CFS above.
    Podium,
    /// Steel frame on composite deck with braced or moment frames.
    SteelFrame,
    /// Concrete flat plate with shear walls or a core.
    ConcreteFlatPlate,
    /// Glulam and CLT with braced frames or CLT shear walls.
    MassTimber,
}

impl SchemeKind {
    pub const ALL: [SchemeKind; 6] = [
        SchemeKind::LightWood,
        SchemeKind::ColdFormedSteel,
        SchemeKind::Podium,
        SchemeKind::SteelFrame,
        SchemeKind::ConcreteFlatPlate,
        SchemeKind::MassTimber,
    ];
    pub fn label(self) -> &'static str {
        match self {
            SchemeKind::LightWood => "Light Wood Frame + Wood Shear Walls",
            SchemeKind::ColdFormedSteel => "Cold-Formed Steel Bearing Walls",
            SchemeKind::Podium => "Podium (Concrete Below, Light Frame Above)",
            SchemeKind::SteelFrame => "Steel Frame on Composite Deck",
            SchemeKind::ConcreteFlatPlate => "Concrete Flat Plate + Shear Walls/Core",
            SchemeKind::MassTimber => "Mass Timber (Glulam + CLT)",
        }
    }
    /// The rules file's key.
    pub fn key(self) -> &'static str {
        match self {
            SchemeKind::LightWood => "light_wood",
            SchemeKind::ColdFormedSteel => "cold_formed_steel",
            SchemeKind::Podium => "podium",
            SchemeKind::SteelFrame => "steel_frame",
            SchemeKind::ConcreteFlatPlate => "concrete_flat_plate",
            SchemeKind::MassTimber => "mass_timber",
        }
    }
}

/// The site's seismic region, as the user sets it (no hazard data is looked up).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Seismic {
    Low,
    #[default]
    Moderate,
    High,
}

/// The lateral system a scheme uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LateralKind {
    WoodShearWalls,
    CfsShearWalls,
    ConcreteShearWalls,
    BracedFrames,
    MomentFrames,
    CltShearWalls,
}

impl LateralKind {
    pub fn label(self) -> &'static str {
        match self {
            LateralKind::WoodShearWalls => "Wood Shear Walls",
            LateralKind::CfsShearWalls => "CFS Shear Walls",
            LateralKind::ConcreteShearWalls => "Concrete Shear Walls",
            LateralKind::BracedFrames => "Braced Frames",
            LateralKind::MomentFrames => "Moment Frames",
            LateralKind::CltShearWalls => "CLT Shear Walls",
        }
    }
}

/// Which way joists or deck span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SpanDir {
    /// The shorter way between supports.
    #[default]
    Auto,
    /// Along x (east–west).
    X,
    /// Along y (north–south).
    Y,
}

/// What the user picked and adjusted before generating.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SchemeSettings {
    pub kind: SchemeKind,
    pub seismic: Seismic,
    /// Target grid spacing each way, mm.
    pub grid_x: f64,
    pub grid_y: f64,
    pub lateral: LateralKind,
    pub span_dir: SpanDir,
}

/// A member of the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum MemberKind {
    Column,
    Girder,
    Beam,
    BearingWall,
    ShearWall,
    BracedFrame,
    MomentFrame,
    /// Joists or deck: drawn as a span arrow across a bay.
    Span,
    /// A transfer girder under something that doesn't stack.
    Transfer,
    /// Foundations (ADR-083): a pad under a column, a strip under a wall, a mat under the
    /// whole footprint (`start`/`end` its corners), a pile cap, and a foundation wall.
    SpreadFooting,
    StripFooting,
    Mat,
    PileCap,
    FoundationWall,
}

impl MemberKind {
    pub fn label(self) -> &'static str {
        match self {
            MemberKind::Column => "Column",
            MemberKind::Girder => "Girder",
            MemberKind::Beam => "Beam",
            MemberKind::BearingWall => "Bearing Wall",
            MemberKind::ShearWall => "Shear Wall",
            MemberKind::BracedFrame => "Braced Frame",
            MemberKind::MomentFrame => "Moment Frame",
            MemberKind::Span => "Joists/Deck",
            MemberKind::Transfer => "Transfer Girder",
            MemberKind::SpreadFooting => "Spread Footing",
            MemberKind::StripFooting => "Strip Footing",
            MemberKind::Mat => "Mat Foundation",
            MemberKind::PileCap => "Pile Cap",
            MemberKind::FoundationWall => "Foundation Wall",
        }
    }
    /// Lateral elements, which resist wind and seismic load.
    /// Foundations, below the lowest level.
    pub fn foundation(self) -> bool {
        matches!(
            self,
            MemberKind::SpreadFooting
                | MemberKind::StripFooting
                | MemberKind::Mat
                | MemberKind::PileCap
                | MemberKind::FoundationWall
        )
    }
    pub fn lateral(self) -> bool {
        matches!(
            self,
            MemberKind::ShearWall | MemberKind::BracedFrame | MemberKind::MomentFrame
        )
    }
}

/// One member: at a point (columns, `start` = `end`) or along a line, on `level`, from
/// elevation `base` up to `top` (mm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StructMember {
    pub kind: MemberKind,
    pub level: ElementId,
    pub start: Pt,
    pub end: Pt,
    pub base: f64,
    pub top: f64,
    /// Preliminary size, e.g. "W12x26 (prelim.)" or "11-7/8\" I-joist @ 16\" o.c.".
    pub size: String,
    /// Depth and width, mm (for drawing).
    pub depth: f64,
    pub width: f64,
    /// Span or height, mm.
    pub span: f64,
    /// The rule that placed and sized it, in words.
    pub rule: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FlagKind {
    Transfer,
    SpanOutOfRange,
    NonStackingLateral,
    ColumnInRoom,
    LateralDirection,
    Torsion,
    Discontinuity,
    /// Foundations (ADR-083): a mat or deep foundations likely.
    Foundation,
}

impl FlagKind {
    pub fn label(self) -> &'static str {
        match self {
            FlagKind::Transfer => "Transfer Condition",
            FlagKind::SpanOutOfRange => "Span Out of Range",
            FlagKind::NonStackingLateral => "Lateral Element Doesn't Stack",
            FlagKind::ColumnInRoom => "Column in Program Space",
            FlagKind::LateralDirection => "Lateral System Short One Way",
            FlagKind::Torsion => "Rigidity Far from Mass",
            FlagKind::Discontinuity => "Discontinuity",
            FlagKind::Foundation => "Foundation",
        }
    }
}

/// A red flag at a point, with its explanation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StructFlag {
    pub kind: FlagKind,
    pub level: Option<ElementId>,
    pub at: Pt,
    pub message: String,
}

/// The generated preliminary layout.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StructLayout {
    /// Grid line positions, mm: x values of lines running north–south, y of east–west.
    pub grid_x: Vec<f64>,
    pub grid_y: Vec<f64>,
    pub members: Vec<StructMember>,
    pub flags: Vec<StructFlag>,
    /// Whole-building notes (lateral balance, assumptions).
    pub notes: Vec<String>,
}
