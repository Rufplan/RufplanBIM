//! What the MEPT suggestions find and propose (serde, exported to TypeScript).

use serde::{Deserialize, Serialize};
use studio_core::mep::{Climate, Discipline, MepSettings};
use studio_core::ElementId;
use studio_geom::Pt;
use ts_rs::TS;

/// A room read for MEP: its type from its name, size, and whether it touches the outside.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MepSpace")]
pub struct Space {
    pub id: ElementId,
    pub level: ElementId,
    pub name: String,
    /// The space type from the rules (Bedroom, Restroom, Office…).
    pub kind: String,
    pub ring: Vec<Pt>,
    pub min: Pt,
    pub max: Pt,
    pub center: Pt,
    /// mm².
    pub area: f64,
    /// Length of its edges on exterior walls, mm, and the longest such edge.
    pub exterior_len: f64,
    #[ts(optional)]
    pub exterior_edge: Option<(Pt, Pt)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MepLevel")]
pub struct Level {
    pub id: ElementId,
    pub name: String,
    pub elevation: f64,
    /// The next level up, or the top of its walls.
    pub top: f64,
    pub outline: Vec<Pt>,
    pub area: f64,
}

/// An exterior wall, with the direction that points outside.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MepExteriorWall")]
pub struct ExteriorWall {
    pub level: ElementId,
    pub start: Pt,
    pub end: Pt,
    pub thickness: f64,
    pub outward: Pt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MepDoor")]
pub struct Door {
    pub level: ElementId,
    pub at: Pt,
    pub outward: Pt,
}

/// The building, as the MEP suggestions read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MepFeatures")]
pub struct Features {
    pub levels: Vec<Level>,
    pub spaces: Vec<Space>,
    pub exterior_walls: Vec<ExteriorWall>,
    pub exterior_doors: Vec<Door>,
    /// Stair and shaft extents per level (vertical routes).
    pub cores: Vec<(ElementId, Pt, Pt)>,
    pub stories: usize,
    /// Floor area of all levels, mm².
    pub area: f64,
    pub min: Pt,
    pub max: Pt,
    /// Uses by floor area, largest first ("residential", "office"…).
    pub uses: Vec<String>,
    /// Kitchens in residential buildings: one per dwelling.
    pub dwelling_units: usize,
    pub residential: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MepCriterion")]
pub struct Criterion {
    pub name: String,
    pub score: f64,
    pub note: String,
}

/// A candidate system, scored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemProposal {
    pub key: String,
    pub label: String,
    pub description: String,
    /// 0–100.
    pub score: f64,
    pub ruled_out: bool,
    pub criteria: Vec<Criterion>,
    pub rationale: String,
    /// Its key numbers: loads, equipment, sizes.
    pub highlights: Vec<String>,
    pub red_flags: Vec<String>,
    pub settings: MepSettings,
}

/// A discipline's Suggest answer: every system ranked, best first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MepProposal {
    pub discipline: Discipline,
    pub disclaimer: String,
    pub climate: Climate,
    pub systems: Vec<SystemProposal>,
    pub assumptions: Vec<String>,
    pub questions: Vec<String>,
    pub summary: Vec<String>,
}
