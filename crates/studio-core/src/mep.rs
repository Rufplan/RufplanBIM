//! The MEPT layers' saved data (ADR-082): for each discipline (Mechanical, Electrical,
//! Plumbing, Technology) the system picked from its Suggest command, the settings, and the
//! preliminary layout generated for it. One element per discipline, on the MEP workset; it
//! never changes the architectural model. The analysis is the `studio-mep` crate's.
//!
//! Preliminary — not engineered. Requires review by a licensed engineer.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::element::ElementId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Discipline {
    Mechanical,
    Electrical,
    Plumbing,
    Technology,
}

impl Discipline {
    pub const ALL: [Discipline; 4] = [
        Discipline::Mechanical,
        Discipline::Electrical,
        Discipline::Plumbing,
        Discipline::Technology,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Discipline::Mechanical => "Mechanical",
            Discipline::Electrical => "Electrical",
            Discipline::Plumbing => "Plumbing",
            Discipline::Technology => "Technology",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Discipline::Mechanical => "mechanical",
            Discipline::Electrical => "electrical",
            Discipline::Plumbing => "plumbing",
            Discipline::Technology => "technology",
        }
    }
    /// The label every proposal, overlay and export of this discipline carries.
    pub fn disclaimer(self) -> &'static str {
        match self {
            Discipline::Technology => {
                "Preliminary — not engineered. Requires review by a qualified technology designer or licensed engineer."
            }
            Discipline::Electrical => {
                "Preliminary — not engineered. Requires review by a licensed electrical engineer."
            }
            Discipline::Plumbing => {
                "Preliminary — not engineered. Requires review by a licensed plumbing engineer."
            }
            Discipline::Mechanical => {
                "Preliminary — not engineered. Requires review by a licensed mechanical engineer."
            }
        }
    }
}

/// The site's climate, as the user sets it (for mechanical systems).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export, rename = "MepClimate")]
pub enum Climate {
    Hot,
    #[default]
    Mixed,
    Cold,
}

/// What the user picked before generating.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MepSettings {
    pub discipline: Discipline,
    /// The system's key in the rules file, e.g. "vrf_doas".
    pub system: String,
    pub climate: Climate,
}

/// Every kind of item the four layers place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum MepKind {
    // Mechanical
    Equipment,
    OutdoorUnit,
    IndoorUnit,
    Diffuser,
    ReturnGrille,
    SupplyDuct,
    Refrigerant,
    Piping,
    Shaft,
    // Electrical
    Service,
    Panel,
    Transformer,
    Generator,
    Feeder,
    Light,
    Receptacle,
    // Plumbing
    WaterService,
    WaterHeater,
    Fixture,
    Stack,
    BuildingDrain,
    ColdWater,
    HotWater,
    Vent,
    // Technology
    Mdf,
    Idf,
    Pathway,
    Backbone,
    DataOutlet,
    AccessPoint,
    Camera,
    AccessControl,
    AvDisplay,
}

impl MepKind {
    pub fn label(self) -> &'static str {
        match self {
            MepKind::Equipment => "Equipment",
            MepKind::OutdoorUnit => "Outdoor Unit",
            MepKind::IndoorUnit => "Indoor Unit",
            MepKind::Diffuser => "Supply Diffuser",
            MepKind::ReturnGrille => "Return Grille",
            MepKind::SupplyDuct => "Supply Duct",
            MepKind::Refrigerant => "Refrigerant Piping",
            MepKind::Piping => "Hydronic Piping",
            MepKind::Shaft => "Shaft",
            MepKind::Service => "Service Entrance",
            MepKind::Panel => "Panel",
            MepKind::Transformer => "Transformer",
            MepKind::Generator => "Generator",
            MepKind::Feeder => "Feeder",
            MepKind::Light => "Light Fixture",
            MepKind::Receptacle => "Receptacle",
            MepKind::WaterService => "Water Service",
            MepKind::WaterHeater => "Water Heater",
            MepKind::Fixture => "Plumbing Fixture",
            MepKind::Stack => "Waste/Vent Stack",
            MepKind::BuildingDrain => "Building Drain",
            MepKind::ColdWater => "Cold Water",
            MepKind::HotWater => "Hot Water",
            MepKind::Vent => "Vent",
            MepKind::Mdf => "MDF (Main Telecom Room)",
            MepKind::Idf => "IDF (Telecom Closet)",
            MepKind::Pathway => "Cable Pathway",
            MepKind::Backbone => "Backbone Riser",
            MepKind::DataOutlet => "Data Outlet",
            MepKind::AccessPoint => "Wi-Fi Access Point",
            MepKind::Camera => "Security Camera",
            MepKind::AccessControl => "Access Control",
            MepKind::AvDisplay => "AV Display",
        }
    }
    /// A short plan label for point items.
    pub fn abbr(self) -> &'static str {
        match self {
            MepKind::Equipment => "EQ",
            MepKind::OutdoorUnit => "CU",
            MepKind::IndoorUnit => "IU",
            MepKind::Diffuser => "S",
            MepKind::ReturnGrille => "R",
            MepKind::Shaft => "SH",
            MepKind::Service => "SE",
            MepKind::Panel => "P",
            MepKind::Transformer => "T",
            MepKind::Generator => "G",
            MepKind::Light => "L",
            MepKind::Receptacle => "⏚",
            MepKind::WaterService => "WS",
            MepKind::WaterHeater => "WH",
            MepKind::Fixture => "F",
            MepKind::Stack => "ST",
            MepKind::Mdf => "MDF",
            MepKind::Idf => "IDF",
            MepKind::DataOutlet => "▽",
            MepKind::AccessPoint => "AP",
            MepKind::Camera => "CAM",
            MepKind::AccessControl => "AC",
            MepKind::AvDisplay => "AV",
            _ => "",
        }
    }
    pub fn discipline(self) -> Discipline {
        use MepKind::*;
        match self {
            Equipment | OutdoorUnit | IndoorUnit | Diffuser | ReturnGrille | SupplyDuct
            | Refrigerant | Piping | Shaft => Discipline::Mechanical,
            Service | Panel | Transformer | Generator | Feeder | Light | Receptacle => {
                Discipline::Electrical
            }
            WaterService | WaterHeater | Fixture | Stack | BuildingDrain | ColdWater | HotWater
            | Vent => Discipline::Plumbing,
            Mdf | Idf | Pathway | Backbone | DataOutlet | AccessPoint | Camera | AccessControl
            | AvDisplay => Discipline::Technology,
        }
    }
}

/// A placed item: a point device (one point) or a run (a polyline), on `level`, from
/// elevation `base` to `top` (mm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MepItem {
    pub kind: MepKind,
    pub level: ElementId,
    pub pts: Vec<Pt>,
    pub base: f64,
    pub top: f64,
    /// Preliminary size or rating, e.g. "3-ton condenser (prelim.)".
    pub size: String,
    /// Plan width of a run (duct width, tray width), mm; 0 for a line.
    pub width: f64,
    /// The rule that placed and sized it.
    pub rule: String,
}

/// A zone (thermal zone, circuit area, coverage cell) drawn as a tinted area.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MepZone {
    pub level: ElementId,
    pub ring: Vec<Pt>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MepFlag {
    pub title: String,
    #[ts(optional)]
    pub level: Option<ElementId>,
    pub at: Pt,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MepLayout {
    pub items: Vec<MepItem>,
    pub zones: Vec<MepZone>,
    pub flags: Vec<MepFlag>,
    /// Whole-building numbers and notes (loads, service size, fixture units).
    pub notes: Vec<String>,
}
