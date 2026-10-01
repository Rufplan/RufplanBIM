//! Furniture and equipment (ADR-090), the FF&E of Revit's Furniture and Specialty
//! Equipment families: a library of the pieces most common in homes, apartment buildings
//! and hotels, each a parametric archetype (a sofa with its seats, a bed with its pillows,
//! a range with its burners) sized by its type. Loaded types live in the project; placed
//! pieces sit on a level (or on a wall or counter at their type's mounting height).

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{Category, ElementData, ElementId};
use crate::units::MM_PER_IN;

/// Furniture, or equipment (appliances, mechanical, amenity and fitness equipment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FfeClass {
    Furniture,
    Equipment,
}

impl FfeClass {
    /// Revit's category of the placed pieces.
    pub fn category(self) -> Category {
        match self {
            FfeClass::Furniture => Category::Furniture,
            FfeClass::Equipment => Category::SpecialtyEquipment,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            FfeClass::Furniture => "Furniture",
            FfeClass::Equipment => "Equipment",
        }
    }
}

/// The shape a piece is built as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FfeKind {
    // Furniture
    Sofa,
    Sectional,
    Armchair,
    Chaise,
    Chair,
    Stool,
    OfficeChair,
    Ottoman,
    Bench,
    Booth,
    Table,
    RoundTable,
    Desk,
    Bed,
    BunkBed,
    Crib,
    Casegood,
    Cabinet,
    Wardrobe,
    Bookcase,
    Rug,
    LuggageRack,
    PoolTable,
    Umbrella,
    // Equipment
    Refrigerator,
    Range,
    Cooktop,
    WallOven,
    Microwave,
    OtrMicrowave,
    Hood,
    Dishwasher,
    Washer,
    Dryer,
    StackedLaundry,
    WaterHeater,
    WallBox,
    Furnace,
    Condenser,
    MiniSplit,
    Ptac,
    Tv,
    WineCooler,
    IceMachine,
    Vending,
    ChestFreezer,
    CounterAppliance,
    Cart,
    Lockers,
    Mailboxes,
    Treadmill,
    Elliptical,
    Bike,
    WeightBench,
    Rack,
    Grill,
    Kiosk,
}

/// Where a piece sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FfeMount {
    Floor,
    /// On a wall, its base at the mounting height.
    Wall,
    /// On a counter or a piece of furniture, its base at the mounting height.
    Counter,
}

impl FfeKind {
    pub fn class(self) -> FfeClass {
        use FfeKind::*;
        match self {
            Sofa | Sectional | Armchair | Chaise | Chair | Stool | OfficeChair | Ottoman
            | Bench | Booth | Table | RoundTable | Desk | Bed | BunkBed | Crib | Casegood
            | Cabinet | Wardrobe | Bookcase | Rug | LuggageRack | PoolTable | Umbrella => {
                FfeClass::Furniture
            }
            _ => FfeClass::Equipment,
        }
    }
    /// How it mounts by default, and the base's height above the floor (inches).
    pub fn mount(self, count: u32) -> (FfeMount, f64) {
        use FfeKind::*;
        match self {
            OtrMicrowave => (FfeMount::Wall, 54.0),
            Hood => (FfeMount::Wall, 66.0),
            MiniSplit => (FfeMount::Wall, 84.0),
            Tv => (FfeMount::Wall, 40.0),
            WallBox => (FfeMount::Wall, 48.0),
            Ptac => (FfeMount::Wall, 4.0),
            Mailboxes => (FfeMount::Wall, 12.0),
            Cooktop | Microwave | CounterAppliance => (FfeMount::Counter, 36.0),
            WallOven => (FfeMount::Floor, if count >= 2 { 12.0 } else { 30.0 }),
            _ => (FfeMount::Floor, 0.0),
        }
    }
}

/// Building types a library piece is typical of (the picker's filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FfeUse {
    Residential,
    Multifamily,
    Hospitality,
}

impl FfeUse {
    pub const ALL: [FfeUse; 3] = [
        FfeUse::Residential,
        FfeUse::Multifamily,
        FfeUse::Hospitality,
    ];
    pub fn label(self) -> &'static str {
        match self {
            FfeUse::Residential => "Residential",
            FfeUse::Multifamily => "Multifamily",
            FfeUse::Hospitality => "Hospitality",
        }
    }
}

/// A furniture or equipment type's data (mm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FfeSpec {
    pub class: FfeClass,
    pub kind: FfeKind,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub mount: FfeMount,
    /// The base's height above the floor, for wall and counter pieces.
    pub mount_height: f64,
    /// Seats, pillows, drawers, doors, burners, columns: the archetype's count.
    pub count: u32,
    /// Main and accent colors (sRGB).
    pub color: [u8; 3],
    pub accent: [u8; 3],
}

/// A library piece (a Revit family type).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FfePreset {
    pub name: String,
    pub description: String,
    pub group: String,
    pub uses: Vec<FfeUse>,
    pub spec: FfeSpec,
}

const fn hex(v: u32) -> [u8; 3] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

// Finishes.
const OAK: u32 = 0xa67c52;
const WALNUT: u32 = 0x6b4a2f;
const WHITE: u32 = 0xf1f0ec;
const CHARCOAL: u32 = 0x4a4d52;
const LINEN: u32 = 0xcfc6b4;
const COGNAC: u32 = 0x8a4f2a;
const NAVY: u32 = 0x2f3e5a;
const SAGE: u32 = 0x8a9a7b;
const CREAM: u32 = 0xe5dccb;
const BLACK: u32 = 0x262626;
const STEEL: u32 = 0xc4c9ce;
const GRAPHITE: u32 = 0x55595e;
const TEAK: u32 = 0x9a6b3e;
const FELT: u32 = 0x2e6b4a;
const RUST: u32 = 0xa8552f;
const GREY: u32 = 0x9a9a96;
const BEIGE: u32 = 0xd9cbb0;

use FfeUse::{Hospitality as H, Multifamily as M, Residential as R};

// (name, group, kind, uses, [w, d, h] inches, count, color, accent, description)
type Row = (
    &'static str,
    &'static str,
    FfeKind,
    &'static [FfeUse],
    [f64; 3],
    u32,
    u32,
    u32,
    &'static str,
);

const RMH: &[FfeUse] = &[R, M, H];
const RM: &[FfeUse] = &[R, M];
const MH: &[FfeUse] = &[M, H];
const HO: &[FfeUse] = &[H];
const MO: &[FfeUse] = &[M];
const RO: &[FfeUse] = &[R];

use FfeKind as K;

const FURNITURE: &[Row] = &[
    // Living
    (
        "Sofa 84\"",
        "Living",
        K::Sofa,
        RMH,
        [84.0, 38.0, 34.0],
        3,
        CHARCOAL,
        BLACK,
        "Three-seat sofa, the living room's anchor",
    ),
    (
        "Sofa 96\"",
        "Living",
        K::Sofa,
        RM,
        [96.0, 40.0, 34.0],
        4,
        LINEN,
        WALNUT,
        "Long four-seat sofa for larger living rooms",
    ),
    (
        "Apartment Sofa 72\"",
        "Living",
        K::Sofa,
        MH,
        [72.0, 34.0, 33.0],
        3,
        SAGE,
        BLACK,
        "Compact sofa scaled for apartments and suites",
    ),
    (
        "Loveseat",
        "Living",
        K::Sofa,
        RMH,
        [62.0, 36.0, 34.0],
        2,
        CHARCOAL,
        BLACK,
        "Two-seat sofa",
    ),
    (
        "Sleeper Sofa",
        "Living",
        K::Sofa,
        RMH,
        [82.0, 40.0, 36.0],
        3,
        NAVY,
        BLACK,
        "Sofa with a queen pull-out bed; hotel suites and dens",
    ),
    (
        "Sectional Sofa L",
        "Living",
        K::Sectional,
        RM,
        [112.0, 92.0, 34.0],
        3,
        CHARCOAL,
        BLACK,
        "L-shaped sectional, corner seat",
    ),
    (
        "Sectional with Chaise",
        "Living",
        K::Sectional,
        RM,
        [104.0, 64.0, 34.0],
        2,
        LINEN,
        BLACK,
        "Sofa with a chaise end",
    ),
    (
        "Modular Lounge Sofa",
        "Living",
        K::Sofa,
        MH,
        [96.0, 36.0, 30.0],
        3,
        CREAM,
        BLACK,
        "Low modular lounge seating for amenity and lobby spaces",
    ),
    (
        "Club Chair",
        "Living",
        K::Armchair,
        RMH,
        [34.0, 36.0, 32.0],
        1,
        COGNAC,
        BLACK,
        "Upholstered club armchair",
    ),
    (
        "Accent Chair",
        "Living",
        K::Armchair,
        RMH,
        [30.0, 32.0, 32.0],
        1,
        SAGE,
        WALNUT,
        "Smaller upholstered accent chair",
    ),
    (
        "Recliner",
        "Living",
        K::Armchair,
        RM,
        [36.0, 38.0, 40.0],
        1,
        CHARCOAL,
        BLACK,
        "Reclining armchair",
    ),
    (
        "Swivel Chair",
        "Living",
        K::Armchair,
        RMH,
        [32.0, 34.0, 32.0],
        1,
        CREAM,
        GRAPHITE,
        "Swivel lounge chair",
    ),
    (
        "Chaise Lounge",
        "Living",
        K::Chaise,
        RH,
        [64.0, 28.0, 30.0],
        1,
        LINEN,
        WALNUT,
        "Indoor chaise for reading",
    ),
    (
        "Ottoman",
        "Living",
        K::Ottoman,
        RMH,
        [36.0, 24.0, 17.0],
        0,
        COGNAC,
        BLACK,
        "Rectangular upholstered ottoman",
    ),
    (
        "Round Pouf",
        "Living",
        K::Ottoman,
        RMH,
        [20.0, 20.0, 17.0],
        1,
        CREAM,
        CREAM,
        "Round pouf or footstool",
    ),
    (
        "Coffee Table",
        "Living",
        K::Table,
        RMH,
        [48.0, 24.0, 17.0],
        0,
        WALNUT,
        BLACK,
        "Rectangular coffee table",
    ),
    (
        "Round Coffee Table",
        "Living",
        K::RoundTable,
        RMH,
        [36.0, 36.0, 16.0],
        0,
        OAK,
        BLACK,
        "Round coffee table",
    ),
    (
        "End Table",
        "Living",
        K::Table,
        RMH,
        [22.0, 22.0, 24.0],
        0,
        WALNUT,
        BLACK,
        "Square end table",
    ),
    (
        "Round Side Table",
        "Living",
        K::RoundTable,
        RMH,
        [18.0, 18.0, 22.0],
        0,
        BLACK,
        BLACK,
        "Small round drink table",
    ),
    (
        "Console Table",
        "Living",
        K::Table,
        RMH,
        [54.0, 14.0, 30.0],
        0,
        OAK,
        BLACK,
        "Narrow console behind a sofa or in an entry",
    ),
    (
        "Media Console",
        "Living",
        K::Cabinet,
        RMH,
        [72.0, 18.0, 24.0],
        3,
        WALNUT,
        BLACK,
        "Low media cabinet under a TV",
    ),
    (
        "Bookcase",
        "Living",
        K::Bookcase,
        RMH,
        [36.0, 12.0, 72.0],
        5,
        OAK,
        OAK,
        "Five-shelf bookcase",
    ),
    (
        "Tall Bookcase",
        "Living",
        K::Bookcase,
        RM,
        [36.0, 14.0, 84.0],
        6,
        WHITE,
        WHITE,
        "Seven-foot bookcase",
    ),
    (
        "Etagere",
        "Living",
        K::Bookcase,
        RMH,
        [48.0, 16.0, 80.0],
        5,
        GRAPHITE,
        OAK,
        "Open metal and wood shelving",
    ),
    (
        "Area Rug 8x10",
        "Living",
        K::Rug,
        RMH,
        [120.0, 96.0, 0.5],
        0,
        BEIGE,
        RUST,
        "8' x 10' area rug",
    ),
    (
        "Area Rug 5x8",
        "Living",
        K::Rug,
        RMH,
        [96.0, 60.0, 0.5],
        0,
        GREY,
        CREAM,
        "5' x 8' area rug",
    ),
    (
        "Area Rug 9x12",
        "Living",
        K::Rug,
        RM,
        [144.0, 108.0, 0.5],
        0,
        CREAM,
        NAVY,
        "9' x 12' area rug for large rooms",
    ),
    (
        "Round Rug 8'",
        "Living",
        K::Rug,
        RMH,
        [96.0, 96.0, 0.5],
        1,
        BEIGE,
        CREAM,
        "8' round rug",
    ),
    (
        "Hall Runner",
        "Living",
        K::Rug,
        RMH,
        [96.0, 30.0, 0.5],
        0,
        RUST,
        CREAM,
        "2'-6\" x 8' runner",
    ),
    (
        "Storage Bench",
        "Living",
        K::Bench,
        RMH,
        [48.0, 18.0, 18.0],
        1,
        OAK,
        LINEN,
        "Entry or window bench with storage",
    ),
    (
        "Shoe Cabinet",
        "Living",
        K::Cabinet,
        RM,
        [36.0, 14.0, 40.0],
        2,
        WHITE,
        BLACK,
        "Entry shoe cabinet",
    ),
    // Dining
    (
        "Dining Table 4-Seat",
        "Dining",
        K::Table,
        RMH,
        [48.0, 36.0, 30.0],
        0,
        OAK,
        BLACK,
        "Four-seat rectangular dining table",
    ),
    (
        "Dining Table 6-Seat",
        "Dining",
        K::Table,
        RM,
        [72.0, 40.0, 30.0],
        0,
        WALNUT,
        BLACK,
        "Six-seat dining table",
    ),
    (
        "Dining Table 8-Seat",
        "Dining",
        K::Table,
        RO,
        [96.0, 42.0, 30.0],
        0,
        WALNUT,
        BLACK,
        "Eight-seat dining table",
    ),
    (
        "Round Dining Table 48\"",
        "Dining",
        K::RoundTable,
        RMH,
        [48.0, 48.0, 30.0],
        0,
        OAK,
        BLACK,
        "Round pedestal table seating four",
    ),
    (
        "Round Dining Table 60\"",
        "Dining",
        K::RoundTable,
        RH,
        [60.0, 60.0, 30.0],
        0,
        WALNUT,
        BLACK,
        "Round table seating six",
    ),
    (
        "Bistro Table 24\"",
        "Dining",
        K::RoundTable,
        RMH,
        [24.0, 24.0, 30.0],
        0,
        BLACK,
        BLACK,
        "Two-seat bistro table",
    ),
    (
        "Pub Table 30\"",
        "Dining",
        K::RoundTable,
        MH,
        [30.0, 30.0, 42.0],
        0,
        WALNUT,
        BLACK,
        "Bar-height round table",
    ),
    (
        "Dining Chair",
        "Dining",
        K::Chair,
        RMH,
        [18.0, 22.0, 36.0],
        0,
        OAK,
        OAK,
        "Wood side chair",
    ),
    (
        "Dining Armchair",
        "Dining",
        K::Chair,
        RMH,
        [23.0, 23.0, 36.0],
        1,
        WALNUT,
        WALNUT,
        "Dining chair with arms",
    ),
    (
        "Upholstered Dining Chair",
        "Dining",
        K::Chair,
        RMH,
        [20.0, 24.0, 38.0],
        0,
        LINEN,
        WALNUT,
        "Upholstered parsons chair",
    ),
    (
        "Counter Stool",
        "Dining",
        K::Stool,
        RMH,
        [17.0, 17.0, 26.0],
        1,
        OAK,
        BLACK,
        "26\" counter-height stool with back",
    ),
    (
        "Bar Stool",
        "Dining",
        K::Stool,
        RMH,
        [18.0, 18.0, 30.0],
        1,
        BLACK,
        BLACK,
        "30\" bar-height stool with back",
    ),
    (
        "Backless Counter Stool",
        "Dining",
        K::Stool,
        RMH,
        [16.0, 16.0, 26.0],
        0,
        WALNUT,
        BLACK,
        "Backless counter stool",
    ),
    (
        "Dining Bench",
        "Dining",
        K::Bench,
        RMH,
        [60.0, 15.0, 18.0],
        0,
        OAK,
        BLACK,
        "Bench along a dining table",
    ),
    (
        "Sideboard",
        "Dining",
        K::Cabinet,
        RMH,
        [66.0, 18.0, 34.0],
        4,
        WALNUT,
        BLACK,
        "Buffet or sideboard",
    ),
    (
        "China Cabinet",
        "Dining",
        K::Wardrobe,
        RO,
        [48.0, 18.0, 78.0],
        2,
        WHITE,
        BLACK,
        "Display hutch",
    ),
    (
        "Banquette",
        "Dining",
        K::Booth,
        RMH,
        [60.0, 24.0, 40.0],
        1,
        NAVY,
        WALNUT,
        "Built-in style banquette seat",
    ),
    // Bedroom
    (
        "King Bed",
        "Bedroom",
        K::Bed,
        RM,
        [80.0, 84.0, 50.0],
        3,
        CREAM,
        WALNUT,
        "King bed with headboard (76\" x 80\" mattress)",
    ),
    (
        "California King Bed",
        "Bedroom",
        K::Bed,
        RO,
        [76.0, 88.0, 50.0],
        3,
        CREAM,
        WALNUT,
        "California king (72\" x 84\" mattress)",
    ),
    (
        "Queen Bed",
        "Bedroom",
        K::Bed,
        RMH,
        [64.0, 84.0, 48.0],
        2,
        CREAM,
        OAK,
        "Queen bed with headboard (60\" x 80\" mattress)",
    ),
    (
        "Full Bed",
        "Bedroom",
        K::Bed,
        RM,
        [58.0, 80.0, 46.0],
        2,
        CREAM,
        OAK,
        "Full / double bed",
    ),
    (
        "Twin Bed",
        "Bedroom",
        K::Bed,
        RM,
        [42.0, 80.0, 40.0],
        1,
        CREAM,
        WHITE,
        "Twin bed",
    ),
    (
        "Twin XL Bed",
        "Bedroom",
        K::Bed,
        RM,
        [42.0, 84.0, 40.0],
        1,
        CREAM,
        WHITE,
        "Twin extra-long bed",
    ),
    (
        "Daybed",
        "Bedroom",
        K::Bed,
        RM,
        [42.0, 80.0, 34.0],
        1,
        LINEN,
        WHITE,
        "Twin daybed",
    ),
    (
        "Bunk Bed",
        "Bedroom",
        K::BunkBed,
        RM,
        [42.0, 80.0, 66.0],
        1,
        WHITE,
        CREAM,
        "Twin-over-twin bunk bed",
    ),
    (
        "Crib",
        "Bedroom",
        K::Crib,
        RM,
        [30.0, 54.0, 38.0],
        0,
        WHITE,
        CREAM,
        "Standard crib",
    ),
    (
        "Nightstand",
        "Bedroom",
        K::Casegood,
        RMH,
        [24.0, 18.0, 26.0],
        2,
        WALNUT,
        BLACK,
        "Two-drawer nightstand",
    ),
    (
        "Dresser",
        "Bedroom",
        K::Casegood,
        RMH,
        [60.0, 20.0, 34.0],
        3,
        WALNUT,
        BLACK,
        "Six-drawer dresser",
    ),
    (
        "Tall Chest",
        "Bedroom",
        K::Casegood,
        RM,
        [36.0, 20.0, 50.0],
        5,
        OAK,
        BLACK,
        "Five-drawer chest",
    ),
    (
        "Wardrobe",
        "Bedroom",
        K::Wardrobe,
        RMH,
        [48.0, 24.0, 78.0],
        2,
        WHITE,
        BLACK,
        "Freestanding wardrobe or armoire",
    ),
    (
        "Bed Bench",
        "Bedroom",
        K::Bench,
        RMH,
        [52.0, 18.0, 18.0],
        1,
        LINEN,
        WALNUT,
        "Upholstered bench at the foot of a bed",
    ),
    (
        "Vanity Table",
        "Bedroom",
        K::Desk,
        RO,
        [42.0, 18.0, 30.0],
        0,
        WHITE,
        CREAM,
        "Dressing table",
    ),
    (
        "Luggage Rack",
        "Bedroom",
        K::LuggageRack,
        HO,
        [26.0, 16.0, 20.0],
        0,
        WALNUT,
        BLACK,
        "Folding luggage rack",
    ),
    // Office
    (
        "Desk",
        "Office",
        K::Desk,
        RMH,
        [60.0, 30.0, 30.0],
        1,
        OAK,
        BLACK,
        "Desk with a drawer pedestal",
    ),
    (
        "Writing Desk",
        "Office",
        K::Desk,
        RMH,
        [48.0, 24.0, 30.0],
        0,
        WALNUT,
        BLACK,
        "Writing desk",
    ),
    (
        "Executive Desk",
        "Office",
        K::Desk,
        RO,
        [72.0, 36.0, 30.0],
        2,
        WALNUT,
        BLACK,
        "Double-pedestal desk",
    ),
    (
        "Office Chair",
        "Office",
        K::OfficeChair,
        RMH,
        [26.0, 26.0, 40.0],
        0,
        BLACK,
        GRAPHITE,
        "Task chair",
    ),
    (
        "Guest Chair",
        "Office",
        K::Chair,
        RMH,
        [24.0, 24.0, 33.0],
        1,
        CHARCOAL,
        BLACK,
        "Side chair with arms",
    ),
    (
        "File Cabinet",
        "Office",
        K::Casegood,
        RM,
        [15.0, 24.0, 28.0],
        2,
        GRAPHITE,
        BLACK,
        "Two-drawer file cabinet",
    ),
    (
        "Lateral File",
        "Office",
        K::Casegood,
        RM,
        [36.0, 18.0, 28.0],
        2,
        WHITE,
        BLACK,
        "Two-drawer lateral file",
    ),
    (
        "Low Bookshelf",
        "Office",
        K::Bookcase,
        RMH,
        [36.0, 12.0, 30.0],
        2,
        OAK,
        OAK,
        "Two-shelf low bookcase",
    ),
    (
        "Coworking Desk",
        "Office",
        K::Desk,
        MO,
        [48.0, 24.0, 30.0],
        0,
        WHITE,
        GRAPHITE,
        "Amenity coworking desk",
    ),
    (
        "Conference Table",
        "Office",
        K::Table,
        MH,
        [96.0, 42.0, 30.0],
        0,
        WALNUT,
        GRAPHITE,
        "Eight-seat conference table",
    ),
    (
        "Conference Chair",
        "Office",
        K::OfficeChair,
        MH,
        [24.0, 24.0, 36.0],
        0,
        CHARCOAL,
        GRAPHITE,
        "Meeting room chair",
    ),
    // Hospitality
    (
        "Hotel King Bed",
        "Hospitality",
        K::Bed,
        HO,
        [80.0, 84.0, 54.0],
        4,
        WHITE,
        WALNUT,
        "Guestroom king with a tall upholstered headboard",
    ),
    (
        "Hotel Queen Bed",
        "Hospitality",
        K::Bed,
        HO,
        [64.0, 84.0, 52.0],
        2,
        WHITE,
        WALNUT,
        "Guestroom queen, for double-queen rooms",
    ),
    (
        "Hotel Nightstand",
        "Hospitality",
        K::Casegood,
        HO,
        [22.0, 18.0, 24.0],
        1,
        WALNUT,
        BLACK,
        "Guestroom night table with a drawer",
    ),
    (
        "Hotel Desk",
        "Hospitality",
        K::Desk,
        HO,
        [48.0, 22.0, 30.0],
        0,
        WALNUT,
        BLACK,
        "Guestroom work desk",
    ),
    (
        "Hotel Desk Chair",
        "Hospitality",
        K::Chair,
        HO,
        [22.0, 22.0, 33.0],
        1,
        CHARCOAL,
        BLACK,
        "Guestroom desk chair",
    ),
    (
        "Guestroom Lounge Chair",
        "Hospitality",
        K::Armchair,
        HO,
        [30.0, 32.0, 33.0],
        1,
        NAVY,
        WALNUT,
        "Lounge chair by the window",
    ),
    (
        "Hotel Dresser",
        "Hospitality",
        K::Cabinet,
        HO,
        [60.0, 22.0, 32.0],
        3,
        WALNUT,
        BLACK,
        "Dresser / TV credenza",
    ),
    (
        "Luggage Bench",
        "Hospitality",
        K::Bench,
        HO,
        [42.0, 20.0, 20.0],
        0,
        WALNUT,
        BLACK,
        "Luggage bench",
    ),
    (
        "Open Closet Unit",
        "Hospitality",
        K::Bookcase,
        HO,
        [40.0, 24.0, 80.0],
        2,
        WALNUT,
        BLACK,
        "Open wardrobe with shelves",
    ),
    (
        "Lobby Sofa",
        "Hospitality",
        K::Sofa,
        HO,
        [90.0, 36.0, 31.0],
        3,
        NAVY,
        BLACK,
        "Lobby lounge sofa",
    ),
    (
        "Lobby Lounge Chair",
        "Hospitality",
        K::Armchair,
        HO,
        [32.0, 34.0, 30.0],
        1,
        COGNAC,
        BLACK,
        "Lobby lounge chair",
    ),
    (
        "Lobby Coffee Table",
        "Hospitality",
        K::Table,
        HO,
        [60.0, 30.0, 16.0],
        0,
        BLACK,
        BLACK,
        "Large lobby cocktail table",
    ),
    (
        "Communal Table",
        "Hospitality",
        K::Table,
        MH,
        [120.0, 42.0, 30.0],
        0,
        OAK,
        BLACK,
        "Long communal table for lobbies and club rooms",
    ),
    (
        "Reception Desk",
        "Hospitality",
        K::Desk,
        HO,
        [120.0, 36.0, 42.0],
        3,
        WALNUT,
        CREAM,
        "Front desk, transaction-top height",
    ),
    (
        "Lounge Banquette",
        "Hospitality",
        K::Booth,
        HO,
        [96.0, 26.0, 40.0],
        1,
        COGNAC,
        WALNUT,
        "Long upholstered banquette",
    ),
    (
        "Restaurant Table 2-Top",
        "Hospitality",
        K::Table,
        HO,
        [30.0, 30.0, 30.0],
        0,
        WALNUT,
        BLACK,
        "Two-top restaurant table",
    ),
    (
        "Restaurant Table 4-Top",
        "Hospitality",
        K::Table,
        HO,
        [36.0, 36.0, 30.0],
        0,
        WALNUT,
        BLACK,
        "Four-top restaurant table",
    ),
    (
        "Restaurant Booth",
        "Hospitality",
        K::Booth,
        HO,
        [48.0, 24.0, 42.0],
        1,
        RUST,
        WALNUT,
        "Single booth seat",
    ),
    (
        "Bar Height Table",
        "Hospitality",
        K::RoundTable,
        HO,
        [30.0, 30.0, 42.0],
        0,
        BLACK,
        BLACK,
        "Round bar-height table",
    ),
    // Amenity
    (
        "Pool Table",
        "Amenity",
        K::PoolTable,
        RM,
        [100.0, 56.0, 31.0],
        0,
        WALNUT,
        FELT,
        "8' pool table for game rooms and club rooms",
    ),
    (
        "Game Table",
        "Amenity",
        K::Table,
        MH,
        [36.0, 36.0, 30.0],
        0,
        OAK,
        FELT,
        "Card and game table",
    ),
    // Outdoor
    (
        "Outdoor Sofa",
        "Outdoor",
        K::Sofa,
        RMH,
        [80.0, 34.0, 30.0],
        3,
        GREY,
        TEAK,
        "Teak-frame outdoor sofa",
    ),
    (
        "Outdoor Lounge Chair",
        "Outdoor",
        K::Armchair,
        RMH,
        [30.0, 32.0, 28.0],
        1,
        GREY,
        TEAK,
        "Outdoor lounge chair",
    ),
    (
        "Adirondack Chair",
        "Outdoor",
        K::Armchair,
        RO,
        [30.0, 34.0, 36.0],
        1,
        WHITE,
        WHITE,
        "Adirondack chair",
    ),
    (
        "Pool Chaise",
        "Outdoor",
        K::Chaise,
        MH,
        [78.0, 28.0, 14.0],
        0,
        WHITE,
        GRAPHITE,
        "Pool lounger",
    ),
    (
        "Outdoor Dining Table",
        "Outdoor",
        K::Table,
        RMH,
        [72.0, 40.0, 29.0],
        0,
        TEAK,
        TEAK,
        "Six-seat outdoor dining table",
    ),
    (
        "Outdoor Dining Chair",
        "Outdoor",
        K::Chair,
        RMH,
        [22.0, 24.0, 34.0],
        1,
        TEAK,
        TEAK,
        "Outdoor dining armchair",
    ),
    (
        "Fire Table",
        "Outdoor",
        K::RoundTable,
        RMH,
        [42.0, 42.0, 24.0],
        0,
        GRAPHITE,
        RUST,
        "Round gas fire table",
    ),
    (
        "Patio Umbrella 9'",
        "Outdoor",
        K::Umbrella,
        RMH,
        [108.0, 108.0, 96.0],
        0,
        CREAM,
        GRAPHITE,
        "9' market umbrella",
    ),
];

const EQUIPMENT: &[Row] = &[
    // Kitchen
    (
        "Refrigerator French Door 36\"",
        "Kitchen",
        K::Refrigerator,
        RMH,
        [36.0, 30.0, 70.0],
        3,
        STEEL,
        GRAPHITE,
        "French-door refrigerator with bottom freezer",
    ),
    (
        "Refrigerator Top Freezer 30\"",
        "Kitchen",
        K::Refrigerator,
        RM,
        [30.0, 30.0, 67.0],
        2,
        WHITE,
        GREY,
        "Top-freezer refrigerator, the apartment standard",
    ),
    (
        "Refrigerator Bottom Freezer 30\"",
        "Kitchen",
        K::Refrigerator,
        RM,
        [30.0, 32.0, 68.0],
        4,
        STEEL,
        GRAPHITE,
        "Bottom-freezer refrigerator",
    ),
    (
        "Refrigerator Side-by-Side 36\"",
        "Kitchen",
        K::Refrigerator,
        RO,
        [36.0, 30.0, 70.0],
        5,
        STEEL,
        GRAPHITE,
        "Side-by-side refrigerator",
    ),
    (
        "Column Refrigerator 30\"",
        "Kitchen",
        K::Refrigerator,
        RO,
        [30.0, 25.0, 84.0],
        1,
        STEEL,
        GRAPHITE,
        "Built-in column refrigerator",
    ),
    (
        "Counter-Depth Refrigerator 24\"",
        "Kitchen",
        K::Refrigerator,
        MO,
        [24.0, 25.0, 70.0],
        4,
        STEEL,
        GRAPHITE,
        "Narrow refrigerator for micro-units",
    ),
    (
        "Undercounter Refrigerator",
        "Kitchen",
        K::Refrigerator,
        RMH,
        [24.0, 24.0, 34.0],
        1,
        STEEL,
        GRAPHITE,
        "Undercounter refrigerator",
    ),
    (
        "Mini Refrigerator",
        "Kitchen",
        K::Refrigerator,
        HO,
        [20.0, 20.0, 33.0],
        1,
        BLACK,
        GRAPHITE,
        "Guestroom mini fridge",
    ),
    (
        "Upright Freezer",
        "Kitchen",
        K::Refrigerator,
        RO,
        [28.0, 28.0, 68.0],
        1,
        WHITE,
        GREY,
        "Upright freezer",
    ),
    (
        "Chest Freezer",
        "Kitchen",
        K::ChestFreezer,
        RO,
        [42.0, 22.0, 34.0],
        0,
        WHITE,
        GREY,
        "Chest freezer",
    ),
    (
        "Range 30\"",
        "Kitchen",
        K::Range,
        RMH,
        [30.0, 28.0, 47.0],
        4,
        STEEL,
        BLACK,
        "Freestanding 30\" range, four burners",
    ),
    (
        "Range 24\"",
        "Kitchen",
        K::Range,
        MO,
        [24.0, 26.0, 47.0],
        4,
        WHITE,
        BLACK,
        "Apartment-size range",
    ),
    (
        "Range 36\"",
        "Kitchen",
        K::Range,
        RO,
        [36.0, 28.0, 47.0],
        6,
        STEEL,
        BLACK,
        "36\" range, six burners",
    ),
    (
        "Professional Range 48\"",
        "Kitchen",
        K::Range,
        RO,
        [48.0, 28.0, 50.0],
        8,
        STEEL,
        BLACK,
        "Pro-style 48\" range",
    ),
    (
        "Cooktop 30\"",
        "Kitchen",
        K::Cooktop,
        RMH,
        [30.0, 21.0, 4.0],
        4,
        BLACK,
        GRAPHITE,
        "Electric or gas cooktop",
    ),
    (
        "Cooktop 36\"",
        "Kitchen",
        K::Cooktop,
        RO,
        [36.0, 21.0, 4.0],
        5,
        BLACK,
        GRAPHITE,
        "Five-burner cooktop",
    ),
    (
        "Induction Cooktop 30\"",
        "Kitchen",
        K::Cooktop,
        RM,
        [30.0, 21.0, 3.0],
        4,
        BLACK,
        GREY,
        "Induction cooktop",
    ),
    (
        "Wall Oven",
        "Kitchen",
        K::WallOven,
        RO,
        [30.0, 24.0, 29.0],
        1,
        STEEL,
        BLACK,
        "Single built-in wall oven",
    ),
    (
        "Double Wall Oven",
        "Kitchen",
        K::WallOven,
        RO,
        [30.0, 24.0, 52.0],
        2,
        STEEL,
        BLACK,
        "Double built-in wall oven",
    ),
    (
        "Countertop Microwave",
        "Kitchen",
        K::Microwave,
        RMH,
        [22.0, 16.0, 13.0],
        0,
        STEEL,
        BLACK,
        "Countertop microwave",
    ),
    (
        "Over-the-Range Microwave",
        "Kitchen",
        K::OtrMicrowave,
        RM,
        [30.0, 16.0, 17.0],
        0,
        STEEL,
        BLACK,
        "Microwave with a vent, over the range",
    ),
    (
        "Range Hood 30\"",
        "Kitchen",
        K::Hood,
        RMH,
        [30.0, 20.0, 36.0],
        1,
        STEEL,
        GRAPHITE,
        "Wall chimney hood",
    ),
    (
        "Range Hood 36\"",
        "Kitchen",
        K::Hood,
        RO,
        [36.0, 22.0, 36.0],
        1,
        STEEL,
        GRAPHITE,
        "36\" wall chimney hood",
    ),
    (
        "Under-Cabinet Hood 30\"",
        "Kitchen",
        K::Hood,
        RM,
        [30.0, 20.0, 6.0],
        0,
        STEEL,
        GRAPHITE,
        "Under-cabinet hood",
    ),
    (
        "Dishwasher 24\"",
        "Kitchen",
        K::Dishwasher,
        RMH,
        [24.0, 24.0, 34.0],
        0,
        STEEL,
        BLACK,
        "Built-in dishwasher",
    ),
    (
        "Dishwasher 18\"",
        "Kitchen",
        K::Dishwasher,
        MO,
        [18.0, 24.0, 34.0],
        0,
        STEEL,
        BLACK,
        "Compact dishwasher",
    ),
    (
        "Trash Compactor",
        "Kitchen",
        K::Dishwasher,
        RO,
        [15.0, 24.0, 34.0],
        1,
        STEEL,
        BLACK,
        "Undercounter trash compactor",
    ),
    (
        "Wine Cooler",
        "Kitchen",
        K::WineCooler,
        RMH,
        [24.0, 24.0, 34.0],
        1,
        STEEL,
        BLACK,
        "Undercounter wine cooler",
    ),
    (
        "Beverage Center",
        "Kitchen",
        K::WineCooler,
        RMH,
        [24.0, 24.0, 34.0],
        2,
        STEEL,
        BLACK,
        "Undercounter beverage refrigerator",
    ),
    (
        "Undercounter Ice Maker",
        "Kitchen",
        K::IceMachine,
        RO,
        [15.0, 24.0, 34.0],
        0,
        STEEL,
        BLACK,
        "Undercounter ice maker",
    ),
    (
        "Coffee Maker",
        "Kitchen",
        K::CounterAppliance,
        RMH,
        [9.0, 12.0, 14.0],
        0,
        BLACK,
        STEEL,
        "Drip coffee maker",
    ),
    (
        "Espresso Machine",
        "Kitchen",
        K::CounterAppliance,
        RH,
        [12.0, 16.0, 16.0],
        1,
        STEEL,
        BLACK,
        "Espresso machine",
    ),
    // Laundry
    (
        "Front-Load Washer",
        "Laundry",
        K::Washer,
        RMH,
        [27.0, 31.0, 39.0],
        0,
        WHITE,
        GRAPHITE,
        "Front-load washer",
    ),
    (
        "Front-Load Dryer",
        "Laundry",
        K::Dryer,
        RMH,
        [27.0, 31.0, 39.0],
        0,
        WHITE,
        GRAPHITE,
        "Front-load dryer",
    ),
    (
        "Top-Load Washer",
        "Laundry",
        K::Washer,
        RO,
        [27.0, 28.0, 42.0],
        1,
        WHITE,
        GRAPHITE,
        "Top-load washer",
    ),
    (
        "Stacked Washer / Dryer",
        "Laundry",
        K::StackedLaundry,
        RM,
        [27.0, 31.0, 77.0],
        0,
        WHITE,
        GRAPHITE,
        "Front-load pair, stacked",
    ),
    (
        "Compact Stacked Washer / Dryer",
        "Laundry",
        K::StackedLaundry,
        MO,
        [24.0, 25.0, 67.0],
        0,
        WHITE,
        GRAPHITE,
        "24\" compact stacked pair for apartments",
    ),
    (
        "Commercial Washer",
        "Laundry",
        K::Washer,
        MH,
        [27.0, 30.0, 43.0],
        0,
        STEEL,
        BLACK,
        "Coin / card washer for shared laundry",
    ),
    (
        "Commercial Stacked Dryer",
        "Laundry",
        K::StackedLaundry,
        MH,
        [31.0, 44.0, 76.0],
        1,
        STEEL,
        BLACK,
        "Stacked commercial dryers",
    ),
    // Mechanical & Plumbing
    (
        "Water Heater 50 gal",
        "Mechanical & Plumbing",
        K::WaterHeater,
        RM,
        [22.0, 22.0, 60.0],
        0,
        WHITE,
        GREY,
        "Tank water heater",
    ),
    (
        "Water Heater 80 gal",
        "Mechanical & Plumbing",
        K::WaterHeater,
        RO,
        [26.0, 26.0, 63.0],
        0,
        WHITE,
        GREY,
        "Large tank water heater",
    ),
    (
        "Heat Pump Water Heater",
        "Mechanical & Plumbing",
        K::WaterHeater,
        RM,
        [22.0, 22.0, 70.0],
        1,
        WHITE,
        GRAPHITE,
        "Heat pump water heater (fan on top)",
    ),
    (
        "Tankless Water Heater",
        "Mechanical & Plumbing",
        K::WallBox,
        RMH,
        [14.0, 10.0, 27.0],
        1,
        WHITE,
        GREY,
        "Wall-hung tankless water heater",
    ),
    (
        "Furnace",
        "Mechanical & Plumbing",
        K::Furnace,
        RO,
        [21.0, 28.0, 40.0],
        0,
        GREY,
        GRAPHITE,
        "Gas furnace",
    ),
    (
        "Air Handler",
        "Mechanical & Plumbing",
        K::Furnace,
        RM,
        [21.0, 21.0, 48.0],
        1,
        GREY,
        GRAPHITE,
        "Fan coil / air handler",
    ),
    (
        "Condensing Unit",
        "Mechanical & Plumbing",
        K::Condenser,
        RMH,
        [30.0, 30.0, 32.0],
        0,
        GREY,
        GRAPHITE,
        "Outdoor AC or heat pump condenser",
    ),
    (
        "Mini-Split Wall Unit",
        "Mechanical & Plumbing",
        K::MiniSplit,
        RMH,
        [32.0, 9.0, 12.0],
        0,
        WHITE,
        GREY,
        "Ductless indoor head",
    ),
    (
        "PTAC Unit",
        "Mechanical & Plumbing",
        K::Ptac,
        MH,
        [42.0, 14.0, 16.0],
        0,
        WHITE,
        GREY,
        "Packaged terminal AC under a window",
    ),
    (
        "Electrical Panel",
        "Mechanical & Plumbing",
        K::WallBox,
        RMH,
        [14.0, 4.0, 30.0],
        0,
        GREY,
        GRAPHITE,
        "Load center / panelboard",
    ),
    (
        "EV Charger",
        "Mechanical & Plumbing",
        K::WallBox,
        RMH,
        [10.0, 5.0, 15.0],
        2,
        WHITE,
        BLACK,
        "Level 2 EV charger",
    ),
    // Electronics
    (
        "TV 43\"",
        "Electronics",
        K::Tv,
        MH,
        [38.0, 3.0, 22.0],
        0,
        BLACK,
        BLACK,
        "Guestroom TV",
    ),
    (
        "TV 55\"",
        "Electronics",
        K::Tv,
        RMH,
        [49.0, 3.0, 29.0],
        0,
        BLACK,
        BLACK,
        "55\" wall-mounted TV",
    ),
    (
        "TV 65\"",
        "Electronics",
        K::Tv,
        RMH,
        [57.0, 3.0, 33.0],
        0,
        BLACK,
        BLACK,
        "65\" wall-mounted TV",
    ),
    (
        "TV 75\"",
        "Electronics",
        K::Tv,
        RO,
        [66.0, 3.0, 38.0],
        0,
        BLACK,
        BLACK,
        "75\" wall-mounted TV",
    ),
    (
        "In-Room Safe",
        "Electronics",
        K::CounterAppliance,
        HO,
        [17.0, 14.0, 9.0],
        2,
        BLACK,
        GRAPHITE,
        "Guestroom safe (closet shelf)",
    ),
    // Hospitality & Amenity
    (
        "Ice Machine with Bin",
        "Hospitality & Amenity",
        K::IceMachine,
        HO,
        [30.0, 34.0, 80.0],
        1,
        STEEL,
        BLACK,
        "Ice machine on a storage bin, for ice / vending rooms",
    ),
    (
        "Vending Machine",
        "Hospitality & Amenity",
        K::Vending,
        MH,
        [41.0, 35.0, 72.0],
        0,
        BLACK,
        STEEL,
        "Snack / drink vending machine",
    ),
    (
        "Luggage Cart",
        "Hospitality & Amenity",
        K::Cart,
        HO,
        [46.0, 26.0, 74.0],
        0,
        STEEL,
        CREAM,
        "Bellman's luggage cart",
    ),
    (
        "Housekeeping Cart",
        "Hospitality & Amenity",
        K::Cart,
        HO,
        [50.0, 22.0, 40.0],
        1,
        GRAPHITE,
        CREAM,
        "Housekeeping cart",
    ),
    (
        "Self Check-in Kiosk",
        "Hospitality & Amenity",
        K::Kiosk,
        HO,
        [20.0, 20.0, 56.0],
        0,
        BLACK,
        STEEL,
        "Lobby check-in kiosk",
    ),
    (
        "Package Lockers",
        "Hospitality & Amenity",
        K::Lockers,
        MO,
        [72.0, 20.0, 72.0],
        6,
        GRAPHITE,
        STEEL,
        "Smart package lockers",
    ),
    (
        "Cluster Mailboxes",
        "Hospitality & Amenity",
        K::Mailboxes,
        MO,
        [32.0, 16.0, 58.0],
        4,
        GREY,
        GRAPHITE,
        "Recessed or surface mailboxes",
    ),
    (
        "Built-in Grill",
        "Hospitality & Amenity",
        K::Grill,
        RMH,
        [36.0, 26.0, 46.0],
        0,
        STEEL,
        BLACK,
        "Gas grill in an outdoor kitchen",
    ),
    // Fitness
    (
        "Treadmill",
        "Fitness",
        K::Treadmill,
        MH,
        [80.0, 35.0, 60.0],
        0,
        BLACK,
        GRAPHITE,
        "Treadmill",
    ),
    (
        "Elliptical",
        "Fitness",
        K::Elliptical,
        MH,
        [84.0, 30.0, 70.0],
        0,
        BLACK,
        GRAPHITE,
        "Elliptical trainer",
    ),
    (
        "Exercise Bike",
        "Fitness",
        K::Bike,
        RMH,
        [48.0, 24.0, 50.0],
        0,
        BLACK,
        GRAPHITE,
        "Upright / spin bike",
    ),
    (
        "Weight Bench",
        "Fitness",
        K::WeightBench,
        RMH,
        [50.0, 24.0, 18.0],
        0,
        BLACK,
        GRAPHITE,
        "Adjustable weight bench",
    ),
    (
        "Power Rack",
        "Fitness",
        K::WeightBench,
        MH,
        [54.0, 48.0, 90.0],
        1,
        BLACK,
        GRAPHITE,
        "Squat / power rack with bench",
    ),
    (
        "Dumbbell Rack",
        "Fitness",
        K::Rack,
        MH,
        [72.0, 22.0, 32.0],
        2,
        BLACK,
        GRAPHITE,
        "Two-tier dumbbell rack",
    ),
];

const RH: &[FfeUse] = &[R, H];

fn preset(r: &Row) -> FfePreset {
    let (name, group, kind, uses, d, count, color, accent, description) = r;
    let (mount, mh) = kind.mount(*count);
    let inch = |v: f64| v * MM_PER_IN;
    FfePreset {
        name: (*name).into(),
        description: (*description).into(),
        group: (*group).into(),
        uses: uses.to_vec(),
        spec: FfeSpec {
            class: kind.class(),
            kind: *kind,
            width: inch(d[0]),
            depth: inch(d[1]),
            height: inch(d[2]),
            mount,
            mount_height: inch(mh),
            count: *count,
            color: hex(*color),
            accent: hex(*accent),
        },
    }
}

/// The library of one class, in its groups' order.
pub fn catalog(class: FfeClass) -> Vec<FfePreset> {
    let rows = match class {
        FfeClass::Furniture => FURNITURE,
        FfeClass::Equipment => EQUIPMENT,
    };
    rows.iter().map(preset).collect()
}

/// The groups of a class, in order.
pub fn groups(class: FfeClass) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for p in catalog(class) {
        if !out.contains(&p.group) {
            out.push(p.group);
        }
    }
    out
}

/// Types a new project starts with.
const STARTER: [&str; 5] = [
    "Sofa 84\"",
    "Queen Bed",
    "Dining Table 6-Seat",
    "Refrigerator French Door 36\"",
    "Range 30\"",
];

pub(crate) fn seed_types(tx: &mut Tx) {
    for class in [FfeClass::Furniture, FfeClass::Equipment] {
        for p in catalog(class)
            .into_iter()
            .filter(|p| STARTER.contains(&p.name.as_str()))
        {
            tx.insert(ElementData::FfeType {
                name: p.name,
                spec: p.spec,
            });
        }
    }
}

/// Loads library pieces by name (one undo step), reusing types already there by name.
pub fn load(doc: &mut Document, names: &[String]) -> CoreResult<Vec<ElementId>> {
    let all: Vec<FfePreset> = catalog(FfeClass::Furniture)
        .into_iter()
        .chain(catalog(FfeClass::Equipment))
        .collect();
    let picked: Vec<FfePreset> = names
        .iter()
        .map(|n| {
            all.iter()
                .find(|p| &p.name == n)
                .cloned()
                .ok_or_else(|| CoreError::Invalid(format!("no \"{n}\" in the library")))
        })
        .collect::<CoreResult<_>>()?;
    if picked.is_empty() {
        return Err(CoreError::Invalid("choose a piece to load".into()));
    }
    let label = match picked.as_slice() {
        [one] => format!("Load {}", one.name),
        _ => format!("Load {} types", picked.len()),
    };
    doc.transact(&label, |tx| {
        let mut out = vec![];
        for p in picked {
            let existing = tx
                .of(Category::FfeType)
                .find(|e| e.data.name() == p.name)
                .map(|e| e.id);
            out.push(match existing {
                Some(id) => id,
                None => tx.insert(ElementData::FfeType {
                    name: p.name,
                    spec: p.spec,
                }),
            });
        }
        Ok(out)
    })
}

pub fn spec_of(doc: &Document, type_id: ElementId) -> Option<&FfeSpec> {
    match doc.data(type_id).ok()? {
        ElementData::FfeType { spec, .. } => Some(spec),
        _ => None,
    }
}

/// Places a piece at `at`, facing `rotation` (its front toward -y rotated by it), on its
/// level or at its type's mounting height. Wall pieces go on the nearest wall's face.
pub fn create(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    at: Pt,
    rotation: f64,
) -> CoreResult<ElementId> {
    let spec = spec_of(doc, type_id)
        .cloned()
        .ok_or_else(|| CoreError::Invalid("pick a furniture or equipment type".into()))?;
    doc.level_elevation(level)?;
    let (at, rotation) = match spec.mount {
        FfeMount::Wall => match crate::lighting::on_wall(doc, level, at) {
            // The wall's outward normal is the way the piece faces; its back sits on the
            // face, so its center is half its depth out.
            Some((p, r)) => {
                let out = Pt::new(r.cos(), r.sin());
                (
                    p.add(out.scale(spec.depth / 2.0)),
                    r + std::f64::consts::FRAC_PI_2,
                )
            }
            None => (at, rotation),
        },
        _ => (at, rotation),
    };
    let label = format!("Place {}", spec.class.label().to_lowercase());
    doc.transact(&label, |tx| {
        Ok(tx.insert(ElementData::Ffe {
            type_id,
            class: spec.class,
            level,
            at,
            rotation,
            offset: spec.mount_height,
        }))
    })
}

/// A piece's footprint (plan corners) and bottom and top (absolute, mm), for IFC.
pub fn envelope(doc: &Document, id: ElementId) -> Option<(Vec<Pt>, f64, f64)> {
    let ElementData::Ffe {
        type_id,
        level,
        at,
        rotation,
        offset,
        ..
    } = doc.data(id).ok()?
    else {
        return None;
    };
    let s = spec_of(doc, *type_id)?;
    let z0 = doc.level_elevation(*level).ok()? + offset;
    let u = Pt::new(rotation.cos(), rotation.sin());
    let v = u.perp();
    let (hw, hd) = (s.width / 2.0, s.depth / 2.0);
    let c = |x: f64, y: f64| at.add(u.scale(x)).add(v.scale(y));
    Some((
        vec![c(-hw, -hd), c(hw, -hd), c(hw, hd), c(-hw, hd)],
        z0,
        z0 + s.height.max(1.0),
    ))
}

// ---------- Properties ----------

use crate::ops::{
    choice, len, level_options, non_empty, parse_id, parse_len, positive, ro, text, Property,
};

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(data) = doc.data(id) else { return };
    match data {
        ElementData::FfeType { name, spec } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(ro(
                "class",
                "Category",
                "Identity Data",
                spec.class.label().into(),
            ));
            props.push(ro(
                "kind",
                "Family",
                "Identity Data",
                format!("{:?}", spec.kind),
            ));
            props.push(len("width", "Width", "Dimensions", spec.width));
            props.push(len("depth", "Depth", "Dimensions", spec.depth));
            props.push(len("height", "Height", "Dimensions", spec.height));
            if spec.mount != FfeMount::Floor {
                props.push(len(
                    "mount_height",
                    "Default Mounting Height",
                    "Constraints",
                    spec.mount_height,
                ));
            }
        }
        ElementData::Ffe {
            level,
            rotation,
            offset,
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
                "offset",
                "Elevation from Level",
                "Constraints",
                *offset,
            ));
            props.push(text(
                "rotation",
                "Rotation (degrees)",
                "Constraints",
                &format!("{:.1}", rotation.to_degrees()),
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
        ElementData::FfeType { name, spec } => match key {
            "name" => *name = non_empty(value)?,
            "width" => spec.width = positive(parse_len(value)?)?,
            "depth" => spec.depth = positive(parse_len(value)?)?,
            "height" => spec.height = positive(parse_len(value)?)?,
            "mount_height" => spec.mount_height = parse_len(value)?,
            _ => return Err(unknown()),
        },
        ElementData::Ffe {
            type_id,
            class,
            level,
            rotation,
            offset,
            ..
        } => match key {
            "type" => {
                let t = parse_id(value)?;
                match spec_of(doc, t) {
                    Some(s) if s.class == *class => *type_id = t,
                    Some(_) => {
                        return Err(CoreError::Invalid(format!(
                            "pick a {} type",
                            class.label().to_lowercase()
                        )))
                    }
                    None => {
                        return Err(CoreError::Invalid(
                            "not a furniture or equipment type".into(),
                        ))
                    }
                }
            }
            "level" => {
                let l = parse_id(value)?;
                doc.level_elevation(l)?;
                *level = l;
            }
            "offset" => *offset = parse_len(value)?,
            "rotation" => {
                let deg: f64 = value
                    .trim()
                    .trim_end_matches('°')
                    .parse()
                    .map_err(|_| CoreError::Invalid(format!("\"{value}\" is not an angle")))?;
                *rotation = deg.to_radians();
            }
            _ => return Err(unknown()),
        },
        _ => return Err(unknown()),
    }
    let label = format!("Change {}", key.replace('_', " "));
    doc.transact(&label, |tx| tx.set(id, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_covers_homes_apartments_and_hotels() {
        let f = catalog(FfeClass::Furniture);
        let e = catalog(FfeClass::Equipment);
        assert!(f.len() >= 100, "{} furniture", f.len());
        assert!(e.len() >= 60, "{} equipment", e.len());
        let names: std::collections::HashSet<&str> =
            f.iter().chain(&e).map(|p| p.name.as_str()).collect();
        assert_eq!(names.len(), f.len() + e.len(), "names are unique");
        for u in FfeUse::ALL {
            assert!(
                f.iter().filter(|p| p.uses.contains(&u)).count() >= 40,
                "{u:?} furniture"
            );
            assert!(
                e.iter().filter(|p| p.uses.contains(&u)).count() >= 20,
                "{u:?} equipment"
            );
        }
        assert!(f.iter().all(|p| p.spec.class == FfeClass::Furniture));
        assert!(e.iter().all(|p| p.spec.class == FfeClass::Equipment));
        // A queen is 64" x 84" (60 x 80 mattress and frame), a 30" range sits on the floor
        // and an over-the-range microwave on the wall at 54".
        let q = f.iter().find(|p| p.name == "Queen Bed").unwrap();
        assert!((q.spec.width - 64.0 * MM_PER_IN).abs() < 1e-9);
        let otr = e
            .iter()
            .find(|p| p.name == "Over-the-Range Microwave")
            .unwrap();
        assert_eq!(otr.spec.mount, FfeMount::Wall);
        assert!((otr.spec.mount_height - 54.0 * MM_PER_IN).abs() < 1e-9);
        assert_eq!(groups(FfeClass::Furniture)[0], "Living");
    }

    #[test]
    fn load_and_place_on_a_level_and_on_a_wall() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let start = doc.count(Category::FfeType);
        assert_eq!(start, 5, "starter types");
        let ids = load(
            &mut doc,
            &["King Bed".into(), "TV 65\"".into(), "Queen Bed".into()],
        )
        .unwrap();
        assert_eq!(
            doc.count(Category::FfeType),
            start + 2,
            "the queen was already there"
        );
        let l1 = doc.levels()[0].0;
        let bed = create(&mut doc, ids[0], l1, Pt::new(1000.0, 1000.0), 0.0).unwrap();
        assert_eq!(doc.data(bed).unwrap().category(), Category::Furniture);
        // A TV near a wall goes on its face at 40".
        let wt = crate::ops::first_of(&doc, Category::WallType).unwrap();
        crate::ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 3000.0),
            Pt::new(4000.0, 3000.0),
        )
        .unwrap();
        let tv = create(&mut doc, ids[1], l1, Pt::new(2000.0, 2800.0), 0.0).unwrap();
        let ElementData::Ffe {
            at, offset, class, ..
        } = doc.data(tv).unwrap()
        else {
            panic!()
        };
        assert_eq!(*class, FfeClass::Equipment);
        assert_eq!(
            doc.data(tv).unwrap().category(),
            Category::SpecialtyEquipment
        );
        assert!((offset - 40.0 * MM_PER_IN).abs() < 1e-9);
        assert!(
            at.y < 3000.0 && at.y > 2800.0,
            "on the wall's near face: {}",
            at.y
        );
        let (foot, z0, z1) = envelope(&doc, bed).unwrap();
        assert_eq!(foot.len(), 4);
        assert!((z1 - z0 - 50.0 * MM_PER_IN).abs() < 1e-6);
    }
}
