//! What Suggest Structure finds and proposes (serde, exported to TypeScript).

use serde::{Deserialize, Serialize};
use studio_core::structural::{LateralKind, SchemeKind, SchemeSettings, Seismic, SpanDir};
use studio_core::ElementId;
use studio_geom::Pt;
use ts_rs::TS;

/// A building use read from room names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum UseKind {
    Residential,
    Office,
    Retail,
    Parking,
    Assembly,
    Education,
}

impl UseKind {
    pub const ALL: [UseKind; 6] = [
        UseKind::Residential,
        UseKind::Office,
        UseKind::Retail,
        UseKind::Parking,
        UseKind::Assembly,
        UseKind::Education,
    ];
    pub fn key(self) -> &'static str {
        match self {
            UseKind::Residential => "residential",
            UseKind::Office => "office",
            UseKind::Retail => "retail",
            UseKind::Parking => "parking",
            UseKind::Assembly => "assembly",
            UseKind::Education => "education",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            UseKind::Residential => "residential",
            UseKind::Office => "office",
            UseKind::Retail => "retail",
            UseKind::Parking => "parking",
            UseKind::Assembly => "assembly",
            UseKind::Education => "education",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Axis {
    X,
    Y,
    Other,
}

/// One level of the building.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LevelFeature {
    pub id: ElementId,
    pub name: String,
    /// mm.
    pub elevation: f64,
    /// To the next level up (or the tops of this level's walls at the roof), mm.
    pub floor_to_floor: f64,
    /// Its floor outline (the largest floor on it), or its walls' extent; mm.
    pub outline: Vec<Pt>,
    /// mm².
    pub area: f64,
    pub wall_length: f64,
    pub exterior_wall_length: f64,
    /// Share of this level's wall length standing on walls of the level below (1 at the
    /// bottom).
    pub stacking_ratio: f64,
    /// Main uses on this level, from its rooms' names.
    pub uses: Vec<UseKind>,
    /// Its longest clear span: the short side of its largest room, mm.
    pub max_room_span: f64,
}

/// A wall, for stacking and lateral candidates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WallFeature {
    pub id: ElementId,
    pub level: ElementId,
    pub start: Pt,
    pub end: Pt,
    pub length: f64,
    pub thickness: f64,
    pub exterior: bool,
    pub axis: Axis,
    /// Length taken by doors and windows, mm.
    pub opening_length: f64,
    /// Opening-free stretches as distances along it from its start, mm.
    pub solid: Vec<(f64, f64)>,
    /// Whether walls on the level below carry it.
    pub stacks: bool,
    /// Around a stair, elevator or shaft.
    pub at_core: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum CoreKind {
    Stair,
    Elevator,
    Shaft,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CoreFeature {
    pub kind: CoreKind,
    pub level: ElementId,
    pub min: Pt,
    pub max: Pt,
}

/// A room wanting a long clear span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpenZone {
    pub room: ElementId,
    pub name: String,
    pub level: ElementId,
    /// mm².
    pub area: f64,
    /// Short side of its extent, mm.
    pub span: f64,
    pub center: Pt,
    /// Must be column-free (assembly, gym…); retail and parking take columns on a grid.
    pub column_free: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DiscontinuityKind {
    NonStackingWall,
    Cantilever,
    ReEntrantCorner,
    Setback,
    SoftStory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Discontinuity {
    pub kind: DiscontinuityKind,
    pub level: ElementId,
    pub at: Pt,
    pub message: String,
}

/// Everything the scoring reads, found deterministically from the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Features {
    /// Levels with something built on them, lowest first.
    pub levels: Vec<LevelFeature>,
    pub story_count: usize,
    /// From the lowest level to the top of the highest walls, mm.
    pub total_height: f64,
    /// Overall plan extent, mm.
    pub min: Pt,
    pub max: Pt,
    /// Long side over short side.
    pub aspect_ratio: f64,
    pub footprint_area: f64,
    pub walls: Vec<WallFeature>,
    pub cores: Vec<CoreFeature>,
    pub open_zones: Vec<OpenZone>,
    pub discontinuities: Vec<Discontinuity>,
    pub reentrant_corners: usize,
    /// The longest room span (short side of the largest room), mm: what joists between
    /// bearing walls must reach.
    pub max_span: f64,
    /// The longest span a column-free room needs, mm (0 if none): what any scheme must reach.
    pub column_free_span: f64,
    /// Uses across the building, most common first.
    pub uses: Vec<UseKind>,
    /// Upper levels' average wall stacking ratio.
    pub stacking: f64,
}

/// How one scheme scored on one criterion (0–1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Criterion {
    pub name: String,
    pub score: f64,
    pub note: String,
}

/// A candidate scheme, scored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SchemeProposal {
    pub kind: SchemeKind,
    pub label: String,
    /// 0–100.
    pub score: f64,
    /// Fails a hard limit (height or story count).
    pub ruled_out: bool,
    pub criteria: Vec<Criterion>,
    pub rationale: String,
    pub grid: String,
    pub member_depths: Vec<String>,
    pub red_flags: Vec<String>,
    /// Starting settings for Generate overlay.
    pub settings: SchemeSettings,
    pub laterals: Vec<LateralKind>,
}

/// Suggest Structure's answer: every scheme ranked, best first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StructuralProposal {
    pub disclaimer: String,
    pub seismic: Seismic,
    pub schemes: Vec<SchemeProposal>,
    pub assumptions: Vec<String>,
    pub questions: Vec<String>,
    /// A short summary of what was found.
    pub summary: Vec<String>,
}

/// The default span direction for a new settings value.
pub(crate) fn default_span() -> SpanDir {
    SpanDir::Auto
}
