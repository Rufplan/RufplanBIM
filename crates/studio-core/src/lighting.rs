//! Lighting fixtures and sun settings (ADR-057), after Revit.
//! - A lighting fixture type carries Revit's photometrics: the light source's shape
//!   (point, line, rectangle, circle) and distribution (spherical, hemispherical, spot),
//!   its initial intensity (lumens and watts) and initial color (color temperature).
//! - A fixture sits on a level at an elevation: ceiling and pendant fixtures at the
//!   ceiling, wall fixtures on a wall face at their mounting height, floor and site
//!   fixtures on the floor or ground. Each can be switched off or dimmed (Revit's Artificial
//!   Lights).
//! - The library is the typical fixtures of every building type (residential, office,
//!   retail, hospitality, healthcare, education, industrial and site), loaded into the
//!   project like doors and windows.
//! - Sun Settings: Revit's Still (the site's sun on a date and time) or Lighting (an
//!   azimuth and altitude), saved with the project for renders and 3D.

use serde::{Deserialize, Serialize};
use studio_geom::{point_in_ring, project_to_segment, Pt};
use ts_rs::TS;

use crate::camera::{project_sun, SunPosition};
use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{Category, ElementData, ElementId};
use crate::ops::{
    choice, flag, len, level_options, non_empty, parse_id, parse_len, positive, ro, text,
    PropOption, Property, DEFAULT_CEILING_HEIGHT,
};
use crate::units::MM_PER_IN;

/// The fixture's body, as studio-views draws it in plan and 3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LightFamily {
    Downlight,
    Gimbal,
    Troffer,
    FlatPanel,
    FlushMount,
    CeilingFan,
    TrackLight,
    DrumPendant,
    GlobePendant,
    MiniPendant,
    Chandelier,
    LinearPendant,
    LinearSlot,
    StripLight,
    UnderCabinet,
    CoveLight,
    WallSconce,
    VanityLight,
    WallWasher,
    FloorLamp,
    TableLamp,
    HighBay,
    ExitSign,
    EmergencyLight,
    WallPack,
    Lantern,
    Bollard,
    AreaPole,
    PostTop,
    FloodLight,
    InGrade,
    StepLight,
    LandscapeSpot,
}

/// How the fixture is placed (Revit hosts: ceiling face, suspended, wall face, floor or
/// the ground).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LightMount {
    Ceiling,
    Pendant,
    Wall,
    Floor,
    Ground,
}

/// Revit's "Emit from Shape".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LightShape {
    Point,
    Line,
    Rectangle,
    Circle,
}

/// Revit's light distribution (photometric webs aren't supported).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LightDistribution {
    Spherical,
    Hemispherical,
    Spot,
}

/// Building types a library fixture is typical of (the picker's filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum BuildingUse {
    Residential,
    Office,
    Retail,
    Hospitality,
    Healthcare,
    Education,
    Industrial,
    Exterior,
}

impl BuildingUse {
    pub const ALL: [BuildingUse; 8] = [
        BuildingUse::Residential,
        BuildingUse::Office,
        BuildingUse::Retail,
        BuildingUse::Hospitality,
        BuildingUse::Healthcare,
        BuildingUse::Education,
        BuildingUse::Industrial,
        BuildingUse::Exterior,
    ];
    pub fn label(self) -> &'static str {
        match self {
            BuildingUse::Residential => "Residential",
            BuildingUse::Office => "Office",
            BuildingUse::Retail => "Retail",
            BuildingUse::Hospitality => "Hospitality",
            BuildingUse::Healthcare => "Healthcare",
            BuildingUse::Education => "Education",
            BuildingUse::Industrial => "Industrial",
            BuildingUse::Exterior => "Site & Exterior",
        }
    }
}

impl LightMount {
    pub fn label(self) -> &'static str {
        match self {
            LightMount::Ceiling => "Ceiling",
            LightMount::Pendant => "Pendant (suspended)",
            LightMount::Wall => "Wall",
            LightMount::Floor => "Floor or furniture",
            LightMount::Ground => "Ground",
        }
    }
    /// Revit shows lighting fixtures in ceiling plans; floor-standing, wall and site
    /// fixtures show in floor plans too (the owner's choice, ADR-057).
    pub fn in_floor_plan(self) -> bool {
        matches!(
            self,
            LightMount::Wall | LightMount::Floor | LightMount::Ground
        )
    }
}

impl LightFamily {
    /// The picker's group.
    pub fn group(self) -> &'static str {
        use LightFamily::*;
        match self {
            Downlight | Gimbal | Troffer | FlatPanel | FlushMount | CeilingFan | TrackLight
            | WallWasher => "Recessed & Ceiling",
            DrumPendant | GlobePendant | MiniPendant | Chandelier => "Pendants & Chandeliers",
            LinearPendant | LinearSlot | StripLight | UnderCabinet | CoveLight => "Linear",
            WallSconce | VanityLight => "Wall",
            FloorLamp | TableLamp => "Floor & Table",
            HighBay => "Industrial",
            ExitSign | EmergencyLight => "Emergency",
            WallPack | Lantern | Bollard | AreaPole | PostTop | FloodLight | InGrade
            | StepLight | LandscapeSpot => "Site & Exterior",
        }
    }
    pub const GROUPS: [&'static str; 8] = [
        "Recessed & Ceiling",
        "Pendants & Chandeliers",
        "Linear",
        "Wall",
        "Floor & Table",
        "Industrial",
        "Emergency",
        "Site & Exterior",
    ];
    /// Which way the light points: straight down, straight up, or out from a wall and
    /// down 30° (wall packs, floods, step lights).
    pub fn aim(self) -> LightAim {
        use LightFamily::*;
        match self {
            InGrade | LandscapeSpot | CoveLight => LightAim::Up,
            WallPack | FloodLight | StepLight | ExitSign => LightAim::Out,
            _ => LightAim::Down,
        }
    }
}

/// Which way a fixture's light points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum LightAim {
    Down,
    Up,
    Out,
}

/// A lighting fixture type's data (mm, lumens, kelvin, watts, degrees).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FixtureSpec {
    pub family: LightFamily,
    pub mount: LightMount,
    /// Body size: across, front to back (or the diameter, for round bodies) and tall.
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    /// Suspension length from the ceiling (pendants), else 0.
    pub drop: f64,
    /// Default elevation for wall, floor and ground fixtures (the fixture's center on a
    /// wall, the base on the floor).
    pub mount_height: f64,
    /// Initial intensity.
    pub lumens: f64,
    pub watts: f64,
    /// Initial color: color temperature.
    pub kelvin: f64,
    pub shape: LightShape,
    pub distribution: LightDistribution,
    /// Spot beam (full) angle.
    pub beam: f64,
}

/// A library fixture (a Revit family type).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LightPreset {
    pub name: String,
    pub description: String,
    pub group: String,
    pub uses: Vec<BuildingUse>,
    pub spec: FixtureSpec,
}

// (name, description, family, mount, uses, [w, d, h, drop, mount height] inches,
// lumens, watts, kelvin, shape, distribution, beam°)
type Row = (
    &'static str,
    &'static str,
    LightFamily,
    LightMount,
    &'static [BuildingUse],
    [f64; 5],
    f64,
    f64,
    f64,
    LightShape,
    LightDistribution,
    f64,
);

use BuildingUse::{
    Education as Ed, Exterior as Ex, Healthcare as He, Hospitality as Ho, Industrial as In,
    Office as Of, Residential as Re, Retail as Rt,
};
use LightDistribution::{Hemispherical as Hemi, Spherical as Sph, Spot};
use LightFamily as F;
use LightMount::{Ceiling as C, Floor as Fl, Ground as G, Pendant as P, Wall as W};
use LightShape::{Circle as Ci, Line as Li, Point as Pt0, Rectangle as Rc};

const INTERIOR: &[BuildingUse] = &[Re, Of, Rt, Ho, He, Ed];
const COMMERCIAL: &[BuildingUse] = &[Of, Rt, He, Ed];

const CATALOG: &[Row] = &[
    // Recessed & ceiling.
    (
        "4\" LED Downlight",
        "Recessed 4\" can with a flush trim: halls, baths, closets",
        F::Downlight,
        C,
        INTERIOR,
        [4.75, 4.75, 3.0, 0.0, 0.0],
        750.0,
        9.0,
        3000.0,
        Ci,
        Spot,
        60.0,
    ),
    (
        "6\" LED Downlight",
        "Recessed 6\" can: general light in living areas, lobbies and corridors",
        F::Downlight,
        C,
        INTERIOR,
        [7.5, 7.5, 3.5, 0.0, 0.0],
        1200.0,
        14.0,
        3000.0,
        Ci,
        Spot,
        70.0,
    ),
    (
        "6\" Adjustable Gimbal",
        "Recessed accent light that tilts to wash art, displays and fireplaces",
        F::Gimbal,
        C,
        &[Re, Rt, Ho],
        [7.5, 7.5, 4.0, 0.0, 0.0],
        1000.0,
        13.0,
        3000.0,
        Ci,
        Spot,
        40.0,
    ),
    (
        "2x4 LED Troffer",
        "Lay-in 2'x4' troffer for acoustical ceiling grids",
        F::Troffer,
        C,
        COMMERCIAL,
        [24.0, 48.0, 4.0, 0.0, 0.0],
        4000.0,
        32.0,
        4000.0,
        Rc,
        Hemi,
        180.0,
    ),
    (
        "2x2 LED Troffer",
        "Lay-in 2'x2' troffer for acoustical ceiling grids",
        F::Troffer,
        C,
        COMMERCIAL,
        [24.0, 24.0, 4.0, 0.0, 0.0],
        3200.0,
        26.0,
        4000.0,
        Rc,
        Hemi,
        180.0,
    ),
    (
        "1x4 LED Flat Panel",
        "Edge-lit 1'x4' panel, surface or grid mounted",
        F::FlatPanel,
        C,
        &[Of, He, Ed, Re],
        [12.0, 48.0, 1.5, 0.0, 0.0],
        3000.0,
        25.0,
        4000.0,
        Rc,
        Hemi,
        180.0,
    ),
    (
        "14\" Flush Mount",
        "Surface ceiling light for bedrooms, halls and closets",
        F::FlushMount,
        C,
        &[Re, Ho],
        [14.0, 14.0, 5.0, 0.0, 0.0],
        1600.0,
        18.0,
        3000.0,
        Ci,
        Hemi,
        180.0,
    ),
    (
        "52\" Ceiling Fan with Light",
        "Five-blade fan with an LED light kit",
        F::CeilingFan,
        P,
        &[Re, Ho],
        [52.0, 52.0, 12.0, 6.0, 0.0],
        1100.0,
        15.0,
        3000.0,
        Ci,
        Hemi,
        180.0,
    ),
    (
        "4' Track, 3 Heads",
        "Ceiling track with three adjustable spot heads",
        F::TrackLight,
        C,
        &[Re, Rt, Ho],
        [48.0, 2.5, 6.0, 0.0, 0.0],
        1800.0,
        24.0,
        3000.0,
        Li,
        Spot,
        36.0,
    ),
    (
        "Recessed Wall Washer",
        "Recessed downlight with a kick reflector that washes the wall beside it",
        F::WallWasher,
        C,
        &[Of, Rt, Ho],
        [7.5, 7.5, 4.0, 0.0, 0.0],
        900.0,
        12.0,
        3500.0,
        Ci,
        Spot,
        60.0,
    ),
    // Pendants & chandeliers.
    (
        "18\" Drum Pendant",
        "Fabric drum shade on a stem: dining, foyers, bedrooms",
        F::DrumPendant,
        P,
        &[Re, Ho],
        [18.0, 18.0, 10.0, 24.0, 0.0],
        2000.0,
        22.0,
        2700.0,
        Pt0,
        Sph,
        180.0,
    ),
    (
        "12\" Globe Pendant",
        "Opal glass globe: stairs, dining and hospitality",
        F::GlobePendant,
        P,
        &[Re, Ho, Rt],
        [12.0, 12.0, 12.0, 24.0, 0.0],
        1200.0,
        14.0,
        2700.0,
        Pt0,
        Sph,
        180.0,
    ),
    (
        "6\" Mini Pendant",
        "Small cylinder over kitchen islands, bars and counters",
        F::MiniPendant,
        P,
        &[Re, Ho, Rt],
        [6.0, 6.0, 9.0, 30.0, 0.0],
        500.0,
        7.0,
        2700.0,
        Pt0,
        Spot,
        90.0,
    ),
    (
        "30\" Chandelier",
        "Six-arm chandelier: dining rooms, entries and ballrooms",
        F::Chandelier,
        P,
        &[Re, Ho],
        [30.0, 30.0, 24.0, 18.0, 0.0],
        3000.0,
        36.0,
        2700.0,
        Pt0,
        Sph,
        180.0,
    ),
    (
        "4' Linear Pendant",
        "Direct/indirect suspended linear: open offices and classrooms",
        F::LinearPendant,
        P,
        &[Of, Ed, He],
        [4.0, 48.0, 3.0, 18.0, 0.0],
        4000.0,
        32.0,
        4000.0,
        Li,
        Hemi,
        180.0,
    ),
    (
        "8' Linear Pendant",
        "Direct/indirect suspended linear, 8' run",
        F::LinearPendant,
        P,
        &[Of, Ed, He],
        [4.0, 96.0, 3.0, 18.0, 0.0],
        8000.0,
        64.0,
        4000.0,
        Li,
        Hemi,
        180.0,
    ),
    // Linear.
    (
        "4' Recessed Linear Slot",
        "Narrow recessed slot for drywall ceilings",
        F::LinearSlot,
        C,
        &[Of, Rt, Ho, Re],
        [4.0, 48.0, 4.0, 0.0, 0.0],
        2500.0,
        22.0,
        3500.0,
        Li,
        Hemi,
        180.0,
    ),
    (
        "4' LED Strip",
        "Surface strip for garages, storage and utility rooms",
        F::StripLight,
        C,
        &[Re, In, Ed, Rt],
        [3.0, 48.0, 3.0, 0.0, 0.0],
        4000.0,
        32.0,
        4000.0,
        Li,
        Hemi,
        180.0,
    ),
    (
        "8' Industrial Strip",
        "Surface or chain-hung strip for warehouses and workshops",
        F::StripLight,
        C,
        &[In, Rt],
        [4.0, 96.0, 3.5, 0.0, 0.0],
        8000.0,
        60.0,
        4000.0,
        Li,
        Hemi,
        180.0,
    ),
    (
        "24\" Under-Cabinet Light",
        "Puck-thin bar under upper cabinets, over the counter",
        F::UnderCabinet,
        W,
        &[Re, Ho, He],
        [24.0, 3.0, 1.0, 0.0, 54.0],
        400.0,
        6.0,
        3000.0,
        Li,
        Hemi,
        180.0,
    ),
    (
        "4' LED Cove",
        "LED tape in a ceiling cove, lighting it from below",
        F::CoveLight,
        W,
        &[Of, Ho, Re],
        [48.0, 1.0, 1.0, 0.0, 102.0],
        800.0,
        10.0,
        3000.0,
        Li,
        Hemi,
        180.0,
    ),
    // Wall.
    (
        "Wall Sconce",
        "Up/down wall sconce: corridors, living rooms and guest rooms",
        F::WallSconce,
        W,
        &[Re, Ho, Of, He],
        [5.0, 4.0, 12.0, 0.0, 66.0],
        800.0,
        10.0,
        3000.0,
        Pt0,
        Hemi,
        180.0,
    ),
    (
        "24\" Vanity Bar",
        "Bath light over the mirror",
        F::VanityLight,
        W,
        &[Re, Ho, He],
        [24.0, 5.0, 6.0, 0.0, 80.0],
        1600.0,
        18.0,
        3000.0,
        Li,
        Hemi,
        180.0,
    ),
    // Floor & table.
    (
        "Floor Lamp",
        "64\" floor lamp with a drum shade",
        F::FloorLamp,
        Fl,
        &[Re, Ho],
        [15.0, 15.0, 64.0, 0.0, 0.0],
        1600.0,
        16.0,
        2700.0,
        Pt0,
        Sph,
        180.0,
    ),
    (
        "Table Lamp",
        "26\" table lamp, set on a 30\" table",
        F::TableLamp,
        Fl,
        &[Re, Ho, Of],
        [14.0, 14.0, 26.0, 0.0, 30.0],
        800.0,
        9.0,
        2700.0,
        Pt0,
        Sph,
        180.0,
    ),
    // Industrial.
    (
        "LED High Bay",
        "Round high bay on a hook: warehouses, gyms and big-box stores",
        F::HighBay,
        P,
        &[In, Rt, Ed],
        [16.0, 16.0, 8.0, 24.0, 0.0],
        24000.0,
        150.0,
        5000.0,
        Ci,
        Spot,
        110.0,
    ),
    // Emergency.
    (
        "Exit Sign",
        "Red LED exit sign, wall or ceiling mounted over the exit",
        F::ExitSign,
        W,
        &[Of, Rt, Ho, He, Ed, In],
        [12.0, 2.0, 8.0, 0.0, 90.0],
        50.0,
        2.0,
        1200.0,
        Rc,
        Hemi,
        180.0,
    ),
    (
        "Emergency Light",
        "Twin-head battery unit, on in a power outage",
        F::EmergencyLight,
        W,
        &[Of, Rt, Ho, He, Ed, In],
        [12.0, 4.0, 6.0, 0.0, 90.0],
        300.0,
        3.0,
        5000.0,
        Pt0,
        Spot,
        100.0,
    ),
    // Site & exterior.
    (
        "LED Wall Pack",
        "Building-mounted area light over doors and loading docks",
        F::WallPack,
        W,
        &[Ex, In, Rt],
        [12.0, 8.0, 9.0, 0.0, 144.0],
        5000.0,
        40.0,
        5000.0,
        Rc,
        Spot,
        120.0,
    ),
    (
        "Exterior Wall Lantern",
        "Coach lantern beside entry doors and garages",
        F::Lantern,
        W,
        &[Ex, Re, Ho],
        [8.0, 9.0, 18.0, 0.0, 72.0],
        800.0,
        9.0,
        2700.0,
        Pt0,
        Sph,
        180.0,
    ),
    (
        "42\" Bollard",
        "Path and plaza bollard",
        F::Bollard,
        G,
        &[Ex],
        [8.0, 8.0, 42.0, 0.0, 0.0],
        900.0,
        12.0,
        3000.0,
        Pt0,
        Hemi,
        180.0,
    ),
    (
        "20' Area Pole Light",
        "Parking lot shoebox head on a 20' pole",
        F::AreaPole,
        G,
        &[Ex, Rt, In],
        [24.0, 12.0, 240.0, 0.0, 0.0],
        15000.0,
        110.0,
        4000.0,
        Rc,
        Spot,
        120.0,
    ),
    (
        "12' Post Top",
        "Decorative lantern on a 12' post: streetscapes and campuses",
        F::PostTop,
        G,
        &[Ex, Ed, Ho],
        [18.0, 18.0, 144.0, 0.0, 0.0],
        5000.0,
        40.0,
        3000.0,
        Pt0,
        Sph,
        180.0,
    ),
    (
        "LED Flood Light",
        "Wall or eave flood for yards, facades and signs",
        F::FloodLight,
        W,
        &[Ex, Re, In],
        [10.0, 4.0, 9.0, 0.0, 180.0],
        6000.0,
        50.0,
        5000.0,
        Rc,
        Spot,
        60.0,
    ),
    (
        "In-Grade Uplight",
        "Buried uplight for facades, columns and trees",
        F::InGrade,
        G,
        &[Ex, Ho, Rt],
        [6.0, 6.0, 4.0, 0.0, 0.0],
        600.0,
        8.0,
        3000.0,
        Ci,
        Spot,
        25.0,
    ),
    (
        "Step Light",
        "Recessed wall light lighting stairs and paths",
        F::StepLight,
        W,
        &[Ex, Re, Ho],
        [5.0, 3.0, 3.0, 0.0, 18.0],
        150.0,
        3.0,
        3000.0,
        Rc,
        Spot,
        100.0,
    ),
    (
        "Landscape Spot",
        "Stake-mounted spot for trees and planting",
        F::LandscapeSpot,
        G,
        &[Ex, Re],
        [3.0, 3.0, 6.0, 0.0, 0.0],
        400.0,
        6.0,
        3000.0,
        Ci,
        Spot,
        35.0,
    ),
    (
        "Soffit Downlight",
        "Recessed downlight in an exterior soffit or canopy",
        F::Downlight,
        C,
        &[Ex, Re, Rt],
        [6.0, 6.0, 3.5, 0.0, 0.0],
        1000.0,
        12.0,
        3000.0,
        Ci,
        Spot,
        70.0,
    ),
];

fn preset(r: &Row) -> LightPreset {
    let (name, description, family, mount, uses, d, lumens, watts, kelvin, shape, dist, beam) = r;
    let inch = |v: f64| v * MM_PER_IN;
    LightPreset {
        name: (*name).into(),
        description: (*description).into(),
        group: family.group().into(),
        uses: uses.to_vec(),
        spec: FixtureSpec {
            family: *family,
            mount: *mount,
            width: inch(d[0]),
            depth: inch(d[1]),
            height: inch(d[2]),
            drop: inch(d[3]),
            mount_height: inch(d[4]),
            lumens: *lumens,
            watts: *watts,
            kelvin: *kelvin,
            shape: *shape,
            distribution: *dist,
            beam: *beam,
        },
    }
}

/// The lighting library.
pub fn catalog() -> Vec<LightPreset> {
    CATALOG.iter().map(preset).collect()
}

/// Types a new project starts with.
const STARTER: [&str; 3] = ["6\" LED Downlight", "2x4 LED Troffer", "Wall Sconce"];

pub(crate) fn seed_types(tx: &mut Tx) {
    for p in catalog()
        .into_iter()
        .filter(|p| STARTER.contains(&p.name.as_str()))
    {
        tx.insert(ElementData::LightingFixtureType {
            name: p.name,
            spec: p.spec,
        });
    }
}

/// Adds the starter types to a project saved before lighting fixtures existed.
pub fn ensure_types(doc: &mut Document) -> CoreResult<()> {
    if doc.count(Category::LightingFixtureType) > 0 {
        return Ok(());
    }
    doc.transact("Add lighting fixture types", |tx| {
        seed_types(tx);
        Ok(())
    })
}

/// Loads library fixtures by name (one undo step), reusing types already there by name.
pub fn load(doc: &mut Document, names: &[String]) -> CoreResult<Vec<ElementId>> {
    let all = catalog();
    let picked: Vec<LightPreset> = names
        .iter()
        .map(|n| {
            all.iter()
                .find(|p| &p.name == n)
                .cloned()
                .ok_or_else(|| CoreError::Invalid(format!("no fixture \"{n}\" in the library")))
        })
        .collect::<CoreResult<_>>()?;
    if picked.is_empty() {
        return Err(CoreError::Invalid("choose a fixture to load".into()));
    }
    let label = if picked.len() == 1 {
        format!("Load {}", picked[0].name)
    } else {
        format!("Load {} lighting fixtures", picked.len())
    };
    doc.transact(&label, |tx| {
        let mut out = vec![];
        for p in picked {
            let existing = tx
                .of(Category::LightingFixtureType)
                .find(|e| e.data.name() == p.name)
                .map(|e| e.id);
            out.push(match existing {
                Some(id) => id,
                None => tx.insert(ElementData::LightingFixtureType {
                    name: p.name,
                    spec: p.spec,
                }),
            });
        }
        Ok(out)
    })
}

/// A type's spec.
pub fn spec_of(doc: &Document, type_id: ElementId) -> Option<&FixtureSpec> {
    match doc.data(type_id).ok()? {
        ElementData::LightingFixtureType { spec, .. } => Some(spec),
        _ => None,
    }
}

/// Where a new fixture's elevation (above its level) goes: ceiling and pendant fixtures
/// at the ceiling over `at` (the level's ceiling containing it, else 9'-0"); wall, floor
/// and ground fixtures at their type's mounting height.
pub fn default_elevation(doc: &Document, level: ElementId, at: Pt, spec: &FixtureSpec) -> f64 {
    match spec.mount {
        LightMount::Ceiling | LightMount::Pendant => doc
            .of(Category::Ceiling)
            .find_map(|e| match &e.data {
                ElementData::Ceiling {
                    level: l,
                    height,
                    boundary,
                    ..
                } if *l == level && boundary.len() >= 3 && point_in_ring(at, boundary) => {
                    Some(*height)
                }
                _ => None,
            })
            .unwrap_or(DEFAULT_CEILING_HEIGHT),
        _ => spec.mount_height,
    }
}

/// Revit hosts wall fixtures on a wall face: the face of the nearest wall on `level`
/// within 5' of `at` (on the side `at` is on), and the rotation facing out of it.
pub fn on_wall(doc: &Document, level: ElementId, at: Pt) -> Option<(Pt, f64)> {
    let mut best: Option<(f64, Pt, f64)> = None;
    for e in doc.of(Category::Wall) {
        let ElementData::Wall {
            type_id,
            start,
            end,
            base_level,
            ..
        } = &e.data
        else {
            continue;
        };
        if *base_level != level || start.dist(*end) < 1.0 {
            continue;
        }
        let half = match doc.data(*type_id) {
            Ok(ElementData::WallType { thickness, .. }) => thickness / 2.0,
            _ => 0.0,
        };
        let (s, d) = project_to_segment(at, *start, *end);
        let on = start.lerp(*end, s);
        if d > 5.0 * 304.8 || best.is_some_and(|b| d >= b.0) {
            continue;
        }
        let dir = end.sub(*start).norm();
        let mut out = dir.perp();
        if at.sub(on).dot(out) < 0.0 {
            out = out.scale(-1.0);
        }
        best = Some((d, on.add(out.scale(half)), out.y.atan2(out.x)));
    }
    best.map(|(_, p, r)| (p, r))
}

/// Places a fixture: at the ceiling, on a wall face or on the floor, per its type.
/// `elevation` overrides where it goes (from a pick in 3D).
pub fn create_fixture(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    at: Pt,
    rotation: f64,
    elevation: Option<f64>,
) -> CoreResult<ElementId> {
    let spec = spec_of(doc, type_id)
        .cloned()
        .ok_or_else(|| CoreError::Invalid("pick a lighting fixture type".into()))?;
    doc.level_elevation(level)?;
    let (at, rotation) = match spec.mount {
        LightMount::Wall => on_wall(doc, level, at).unwrap_or((at, rotation)),
        _ => (at, rotation),
    };
    let elevation = elevation.unwrap_or_else(|| default_elevation(doc, level, at, &spec));
    doc.transact("Place lighting fixture", |tx| {
        Ok(tx.insert(ElementData::LightingFixture {
            type_id,
            level,
            at,
            elevation,
            rotation,
            on: true,
            dimming: 1.0,
        }))
    })
}

/// A fixture's body as a box, for IFC: its footprint and its bottom and top (absolute
/// elevations, mm). Ceiling fixtures hang under the ceiling, pendants below their
/// suspension, wall fixtures are centered on their height and stand out of the wall, and
/// floor and site fixtures stand on their base.
pub fn envelope(doc: &Document, id: ElementId) -> Option<(Vec<Pt>, f64, f64)> {
    let ElementData::LightingFixture {
        type_id,
        level,
        at,
        elevation,
        rotation,
        ..
    } = doc.data(id).ok()?
    else {
        return None;
    };
    let s = spec_of(doc, *type_id)?;
    let z = doc.level_elevation(*level).ok()? + elevation;
    let u = Pt::new(rotation.cos(), rotation.sin());
    let v = u.perp();
    let (f0, f1) = match s.mount {
        LightMount::Wall => (0.0, s.depth),
        _ => (-s.depth / 2.0, s.depth / 2.0),
    };
    let w = s.width / 2.0;
    let p = |f: f64, a: f64| at.add(u.scale(f)).add(v.scale(a));
    let foot = vec![p(f0, -w), p(f1, -w), p(f1, w), p(f0, w)];
    let (z0, z1) = match s.mount {
        LightMount::Ceiling => (z - s.height, z),
        LightMount::Pendant => (z - s.drop - s.height, z - s.drop),
        LightMount::Wall => (z - s.height / 2.0, z + s.height / 2.0),
        LightMount::Floor | LightMount::Ground => (z, z + s.height),
    };
    Some((foot, z0, z1))
}

/// Color of a color temperature (Tanner Helland's fit of the black-body curve).
pub fn kelvin_rgb(kelvin: f64) -> [u8; 3] {
    let t = kelvin.clamp(1000.0, 40000.0) / 100.0;
    let c = |v: f64| v.clamp(0.0, 255.0).round() as u8;
    let r = if t <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2)
    };
    let g = if t <= 66.0 {
        99.470_802_586_1 * t.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    };
    [c(r), c(g), c(b)]
}

fn opts<T: Copy>(
    all: &[T],
    id: impl Fn(T) -> String,
    label: impl Fn(T) -> &'static str,
) -> Vec<PropOption> {
    all.iter()
        .map(|&v| PropOption {
            id: id(v),
            label: label(v).into(),
        })
        .collect()
}

const MOUNTS: [LightMount; 5] = [
    LightMount::Ceiling,
    LightMount::Pendant,
    LightMount::Wall,
    LightMount::Floor,
    LightMount::Ground,
];
const SHAPES: [LightShape; 4] = [
    LightShape::Point,
    LightShape::Line,
    LightShape::Rectangle,
    LightShape::Circle,
];
const DISTRIBUTIONS: [LightDistribution; 3] = [
    LightDistribution::Spherical,
    LightDistribution::Hemispherical,
    LightDistribution::Spot,
];

fn shape_label(s: LightShape) -> &'static str {
    match s {
        LightShape::Point => "Point",
        LightShape::Line => "Line",
        LightShape::Rectangle => "Rectangle",
        LightShape::Circle => "Circle",
    }
}
fn distribution_label(d: LightDistribution) -> &'static str {
    match d {
        LightDistribution::Spherical => "Spherical",
        LightDistribution::Hemispherical => "Hemispherical",
        LightDistribution::Spot => "Spot",
    }
}

fn num(key: &str, label: &str, group: &str, v: f64, unit: &str) -> Property {
    let s = if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    };
    text(key, label, group, &format!("{s} {unit}"))
}

fn parse_num(value: &str, unit: &str) -> CoreResult<f64> {
    let v = value
        .trim()
        .trim_end_matches(unit)
        .trim_end_matches('%')
        .trim();
    v.parse::<f64>()
        .map_err(|_| CoreError::Invalid(format!("\"{value}\" is not a number")))
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(data) = doc.data(id) else { return };
    match data {
        ElementData::LightingFixtureType { name, spec } => {
            const PH: &str = "Photometrics";
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(ro(
                "family",
                "Family",
                "Identity Data",
                format!("{:?}", spec.family),
            ));
            props.push(choice(
                "mount",
                "Mounting",
                "Constraints",
                format!("{:?}", spec.mount),
                opts(&MOUNTS, |m| format!("{m:?}"), LightMount::label),
            ));
            props.push(num(
                "lumens",
                "Initial Intensity (Luminous Flux)",
                PH,
                spec.lumens,
                "lm",
            ));
            props.push(num("watts", "Wattage", PH, spec.watts, "W"));
            props.push(num(
                "efficacy",
                "Efficacy",
                PH,
                spec.lumens / spec.watts.max(0.1),
                "lm/W",
            ));
            props.push(num(
                "kelvin",
                "Initial Color (Color Temperature)",
                PH,
                spec.kelvin,
                "K",
            ));
            props.push(choice(
                "shape",
                "Light Source: Emit from Shape",
                PH,
                format!("{:?}", spec.shape),
                opts(&SHAPES, |s| format!("{s:?}"), shape_label),
            ));
            props.push(choice(
                "distribution",
                "Light Source: Distribution",
                PH,
                format!("{:?}", spec.distribution),
                opts(&DISTRIBUTIONS, |d| format!("{d:?}"), distribution_label),
            ));
            if spec.distribution == LightDistribution::Spot {
                props.push(num("beam", "Spot Beam Angle", PH, spec.beam, "°"));
            }
            props.push(len("width", "Width", "Dimensions", spec.width));
            props.push(len("depth", "Depth", "Dimensions", spec.depth));
            props.push(len("height", "Height", "Dimensions", spec.height));
            if spec.mount == LightMount::Pendant {
                props.push(len("drop", "Suspension Length", "Dimensions", spec.drop));
            }
            if !matches!(spec.mount, LightMount::Ceiling | LightMount::Pendant) {
                props.push(len(
                    "mount_height",
                    "Default Mounting Height",
                    "Constraints",
                    spec.mount_height,
                ));
            }
        }
        ElementData::LightingFixture {
            level,
            elevation,
            rotation,
            on,
            dimming,
            ..
        } => {
            props.push(choice(
                "level",
                "Level",
                "Constraints",
                level.to_string(),
                level_options(doc),
            ));
            props.push(len(
                "elevation",
                "Elevation from Level",
                "Constraints",
                *elevation,
            ));
            props.push(text(
                "rotation",
                "Rotation (degrees)",
                "Constraints",
                &format!("{:.1}", rotation.to_degrees()),
            ));
            props.push(flag("on", "Light On", "Lighting", *on));
            props.push(num(
                "dimming",
                "Dimming",
                "Lighting",
                (dimming * 100.0).round(),
                "%",
            ));
        }
        _ => {}
    }
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let mut d = doc.data(id)?.clone();
    let unknown = || CoreError::Invalid(format!("unknown property {key}"));
    match &mut d {
        ElementData::LightingFixtureType { name, spec } => match key {
            "name" => *name = non_empty(value)?,
            "mount" => {
                spec.mount = *MOUNTS
                    .iter()
                    .find(|m| format!("{m:?}") == value.trim())
                    .ok_or_else(unknown)?
            }
            "shape" => {
                spec.shape = *SHAPES
                    .iter()
                    .find(|s| format!("{s:?}") == value.trim())
                    .ok_or_else(unknown)?
            }
            "distribution" => {
                spec.distribution = *DISTRIBUTIONS
                    .iter()
                    .find(|s| format!("{s:?}") == value.trim())
                    .ok_or_else(unknown)?
            }
            "lumens" => spec.lumens = positive(parse_num(value, "lm")?)?,
            "watts" => spec.watts = positive(parse_num(value, "W")?)?,
            "kelvin" => spec.kelvin = parse_num(value, "K")?.clamp(1000.0, 12000.0),
            "beam" => spec.beam = parse_num(value, "°")?.clamp(1.0, 179.0),
            "width" => spec.width = positive(parse_len(value)?)?,
            "depth" => spec.depth = positive(parse_len(value)?)?,
            "height" => spec.height = positive(parse_len(value)?)?,
            "drop" => spec.drop = parse_len(value)?.max(0.0),
            "mount_height" => spec.mount_height = parse_len(value)?,
            _ => return Err(unknown()),
        },
        ElementData::LightingFixture {
            type_id,
            level,
            elevation,
            rotation,
            on,
            dimming,
            ..
        } => match key {
            "type" => {
                let t = parse_id(value)?;
                if spec_of(doc, t).is_none() {
                    return Err(CoreError::Invalid("not a lighting fixture type".into()));
                }
                *type_id = t;
            }
            "level" => {
                let l = parse_id(value)?;
                doc.level_elevation(l)?;
                *level = l;
            }
            "elevation" => *elevation = parse_len(value)?,
            "rotation" => {
                let deg: f64 = value
                    .trim()
                    .trim_end_matches('°')
                    .parse()
                    .map_err(|_| CoreError::Invalid(format!("\"{value}\" is not an angle")))?;
                *rotation = deg.to_radians();
            }
            "on" => *on = value == "yes",
            "dimming" => *dimming = (parse_num(value, "%")? / 100.0).clamp(0.0, 1.0),
            _ => return Err(unknown()),
        },
        _ => return Err(unknown()),
    }
    let label = format!("Change {}", key.replace('_', " "));
    doc.transact(&label, |tx| tx.set(id, d))
}

/// Revit's Artificial Lights: switches or dims several fixtures in one undo step.
pub fn set_lights(
    doc: &mut Document,
    ids: &[ElementId],
    on: Option<bool>,
    dimming: Option<f64>,
) -> CoreResult<usize> {
    doc.transact("Artificial lights", |tx| {
        let mut n = 0;
        for id in ids {
            let mut changed = false;
            tx.modify(*id, |d| {
                if let ElementData::LightingFixture {
                    on: o, dimming: m, ..
                } = d
                {
                    if let Some(v) = on {
                        *o = v;
                    }
                    if let Some(v) = dimming {
                        *m = v.clamp(0.0, 1.0);
                    }
                    changed = true;
                }
            })?;
            n += usize::from(changed);
        }
        Ok(n)
    })
}

/// Revit's Sun Settings > Solar Study: Still (a date and time at the site) or Lighting
/// (a sun at an azimuth and altitude).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SunMode {
    #[default]
    Still,
    Lighting,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SunSettings {
    pub mode: SunMode,
    pub month: u32,
    pub day: u32,
    /// Hours of local standard time (15.5 = 3:30 PM).
    pub hour: f64,
    /// Lighting mode: degrees clockwise from true north, and above the horizon.
    pub azimuth: f64,
    pub altitude: f64,
}

impl Default for SunSettings {
    fn default() -> Self {
        SunSettings {
            mode: SunMode::Still,
            month: 6,
            day: 21,
            hour: 15.0,
            azimuth: 225.0,
            altitude: 35.0,
        }
    }
}

impl SunSettings {
    pub fn check(&self) -> CoreResult<()> {
        if !(1..=12).contains(&self.month) || !(1..=31).contains(&self.day) {
            return Err(CoreError::Invalid("choose a month and day".into()));
        }
        if !(0.0..24.0).contains(&self.hour) {
            return Err(CoreError::Invalid(
                "the time is between 0:00 and 23:59".into(),
            ));
        }
        if !(-90.0..=90.0).contains(&self.altitude) {
            return Err(CoreError::Invalid(
                "the altitude is between -90° and 90°".into(),
            ));
        }
        Ok(())
    }
}

/// The project's sun settings.
pub fn sun_settings(doc: &Document) -> SunSettings {
    doc.of(Category::ProjectInfo)
        .find_map(|e| match &e.data {
            ElementData::ProjectInfo { sun, .. } => Some(*sun),
            _ => None,
        })
        .unwrap_or_default()
}

/// Saves the project's sun settings (one undo step).
pub fn set_sun_settings(doc: &mut Document, s: SunSettings) -> CoreResult<()> {
    s.check()?;
    let info = doc
        .of(Category::ProjectInfo)
        .next()
        .map(|e| e.id)
        .ok_or_else(|| CoreError::Invalid("no project information".into()))?;
    doc.transact("Sun Settings", |tx| {
        tx.modify(info, |d| {
            if let ElementData::ProjectInfo { sun, .. } = d {
                *sun = s;
            }
        })
    })
}

/// The sun for the project's settings: the site's sun at the date and time (Still), or
/// the given azimuth and altitude turned into the project (Lighting).
pub fn project_sun_now(doc: &Document) -> SunPosition {
    let s = sun_settings(doc);
    match s.mode {
        SunMode::Still => project_sun(doc, s.month, s.day, s.hour),
        SunMode::Lighting => {
            let north = match crate::site::site_of(doc).and_then(|id| doc.data(id).ok()) {
                Some(ElementData::Site { rotation, .. }) => *rotation,
                _ => 0.0,
            };
            let (alt, az) = (s.altitude.to_radians(), s.azimuth.to_radians());
            let (e, n) = (alt.cos() * az.sin(), alt.cos() * az.cos());
            let (sn, cn) = north.sin_cos();
            SunPosition {
                dir: [e * cn - n * sn, e * sn + n * cn, alt.sin()],
                altitude: s.altitude,
                azimuth: s.azimuth,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    fn project() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        (doc, l1)
    }

    fn type_named(doc: &mut Document, name: &str) -> ElementId {
        load(doc, &[name.to_string()]).unwrap()[0]
    }

    #[test]
    fn the_library_covers_every_building_type_and_group() {
        let c = catalog();
        assert!(c.len() >= 35);
        for u in BuildingUse::ALL {
            assert!(
                c.iter().filter(|p| p.uses.contains(&u)).count() >= 4,
                "{u:?}"
            );
        }
        for g in LightFamily::GROUPS {
            assert!(c.iter().any(|p| p.group == g), "{g}");
        }
        let mut names: Vec<&str> = c.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), c.len(), "names are unique");
        let t = c.iter().find(|p| p.name == "2x4 LED Troffer").unwrap();
        assert!((t.spec.width - 609.6).abs() < 1e-9 && (t.spec.depth - 1219.2).abs() < 1e-9);
        assert_eq!(t.spec.lumens, 4000.0);
    }

    #[test]
    fn new_projects_start_with_three_types_and_loading_reuses_by_name() {
        let (mut doc, _) = project();
        assert_eq!(doc.count(Category::LightingFixtureType), 3);
        let a = type_named(&mut doc, "Floor Lamp");
        let b = type_named(&mut doc, "Floor Lamp");
        assert_eq!(a, b);
        assert_eq!(doc.count(Category::LightingFixtureType), 4);
        assert!(load(&mut doc, &["No Such Light".into()]).is_err());
    }

    #[test]
    fn ceiling_fixtures_go_at_the_ceiling_and_wall_ones_on_the_wall_face() {
        let (mut doc, l1) = project();
        let down = type_named(&mut doc, "6\" LED Downlight");
        // No ceiling: 9'-0".
        let a = create_fixture(&mut doc, down, l1, Pt::new(1000.0, 1000.0), 0.0, None).unwrap();
        let elev = |doc: &Document, id| match doc.data(id).unwrap() {
            ElementData::LightingFixture {
                elevation,
                at,
                rotation,
                ..
            } => (*elevation, *at, *rotation),
            _ => unreachable!(),
        };
        assert!((elev(&doc, a).0 - 2743.2).abs() < 1e-9);
        // Under an 8'-0" ceiling: at the ceiling.
        let ct = ops::first_of(&doc, Category::CeilingType).unwrap();
        let ring = vec![
            Pt::new(0.0, 0.0),
            Pt::new(4000.0, 0.0),
            Pt::new(4000.0, 4000.0),
            Pt::new(0.0, 4000.0),
        ];
        let c = ops::create_ceiling(&mut doc, ct, l1, ring).unwrap();
        ops::set_property(&mut doc, c, "height", "8'", 0).unwrap();
        let b = create_fixture(&mut doc, down, l1, Pt::new(1000.0, 1000.0), 0.0, None).unwrap();
        assert!((elev(&doc, b).0 - 2438.4).abs() < 1e-6);
        // A sconce clicked 1' in front of a wall lands on its face, facing out, at 5'-6".
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 6000.0),
            Pt::new(5000.0, 6000.0),
        )
        .unwrap();
        let half = match doc.data(wt).unwrap() {
            ElementData::WallType { thickness, .. } => thickness / 2.0,
            _ => unreachable!(),
        };
        let sconce = type_named(&mut doc, "Wall Sconce");
        let s = create_fixture(
            &mut doc,
            sconce,
            l1,
            Pt::new(2000.0, 6000.0 - 304.8),
            0.0,
            None,
        )
        .unwrap();
        let (e, at, rot) = elev(&doc, s);
        assert!((e - 66.0 * MM_PER_IN).abs() < 1e-9);
        assert!((at.x - 2000.0).abs() < 1e-6 && (at.y - (6000.0 - half)).abs() < 1e-6);
        assert!(
            (rot + std::f64::consts::FRAC_PI_2).abs() < 1e-9,
            "faces -y, out of the wall"
        );
    }

    #[test]
    fn properties_edit_photometrics_and_artificial_lights_switch_them() {
        let (mut doc, l1) = project();
        let t = type_named(&mut doc, "2x4 LED Troffer");
        ops::set_property(&mut doc, t, "lumens", "5000 lm", 0).unwrap();
        ops::set_property(&mut doc, t, "kelvin", "3500", 0).unwrap();
        let s = spec_of(&doc, t).unwrap();
        assert_eq!((s.lumens, s.kelvin), (5000.0, 3500.0));
        assert!(ops::set_property(&mut doc, t, "lumens", "-3", 0).is_err());
        let f = create_fixture(&mut doc, t, l1, Pt::new(0.0, 0.0), 0.0, None).unwrap();
        ops::set_property(&mut doc, f, "dimming", "40%", 0).unwrap();
        let g = create_fixture(&mut doc, t, l1, Pt::new(3000.0, 0.0), 0.0, None).unwrap();
        assert_eq!(set_lights(&mut doc, &[f, g], Some(false), None).unwrap(), 2);
        for id in [f, g] {
            assert!(matches!(
                doc.data(id).unwrap(),
                ElementData::LightingFixture { on: false, .. }
            ));
        }
        assert!(
            matches!(doc.data(f).unwrap(), ElementData::LightingFixture { dimming, .. } if (dimming - 0.4).abs() < 1e-9)
        );
        doc.undo().unwrap();
        assert!(matches!(
            doc.data(g).unwrap(),
            ElementData::LightingFixture { on: true, .. }
        ));
        let sheet = ops::properties(&doc, f).unwrap();
        assert!(sheet
            .properties
            .iter()
            .any(|p| p.key == "dimming" && p.value == "40 %"));
    }

    #[test]
    fn a_fixtures_envelope_is_its_body_box() {
        let (mut doc, l1) = project();
        let p = type_named(&mut doc, "18\" Drum Pendant");
        let f = create_fixture(&mut doc, p, l1, Pt::new(1000.0, 0.0), 0.0, None).unwrap();
        let (foot, z0, z1) = envelope(&doc, f).unwrap();
        // 9'-0" ceiling, 24" stem, 10" drum, 18" across.
        assert!((z1 - (2743.2 - 609.6)).abs() < 1e-6 && (z1 - z0 - 254.0).abs() < 1e-6);
        assert!(
            (foot[0].dist(foot[1]) - 457.2).abs() < 1e-6
                && (foot[1].dist(foot[2]) - 457.2).abs() < 1e-6
        );
    }

    #[test]
    fn color_temperature_runs_from_warm_to_cool() {
        assert_eq!(kelvin_rgb(6600.0), [255, 255, 255]);
        let warm = kelvin_rgb(2700.0);
        assert_eq!(warm[0], 255);
        assert!(
            warm[1] > 160 && warm[1] < 175 && warm[2] > 80 && warm[2] < 95,
            "{warm:?}"
        );
        let cool = kelvin_rgb(10000.0);
        assert!(cool[2] == 255 && cool[0] < 210);
    }

    #[test]
    fn sun_settings_are_saved_and_lighting_mode_aims_the_sun() {
        let (mut doc, _) = project();
        assert_eq!(sun_settings(&doc), SunSettings::default());
        let s = SunSettings {
            mode: SunMode::Lighting,
            azimuth: 90.0,
            altitude: 30.0,
            ..SunSettings::default()
        };
        set_sun_settings(&mut doc, s).unwrap();
        assert_eq!(sun_settings(&doc), s);
        let sun = project_sun_now(&doc);
        // Due east, 30° up.
        assert!((sun.dir[0] - 30f64.to_radians().cos()).abs() < 1e-9);
        assert!(sun.dir[1].abs() < 1e-9);
        assert!((sun.dir[2] - 0.5).abs() < 1e-9);
        assert!(set_sun_settings(&mut doc, SunSettings { hour: 25.0, ..s }).is_err());
        doc.undo().unwrap();
        assert_eq!(sun_settings(&doc), SunSettings::default());
    }
}
