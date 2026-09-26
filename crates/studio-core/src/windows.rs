//! Window families (ADR-031): the window types common in US construction, how each one
//! divides into frame, sashes and lites, and a catalog of standard sizes to load as types.
//!
//! A layout is seen from outside: `u` runs left to right across the rough opening, from 0
//! to its width, and `z` up from 0 at the sill. Lengths are mm.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, WindowFamily};
use crate::units::MM_PER_IN;

/// Grille (simulated divided lite) pattern of a window type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Grille {
    #[default]
    None,
    /// A grid of roughly 9" x 12" lites (6-over-6 on a 3' double-hung).
    Colonial,
    /// Bars inset around the edge, leaving a large centre lite.
    Prairie,
    /// Vertical bars in the upper sash or band only (3-over-1).
    Craftsman,
}

/// Frame and sash finish of a window type (its colour in 3D and renderings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FrameFinish {
    #[default]
    White,
    Almond,
    Bronze,
    Black,
    /// Stained wood, as on a clad-wood window's interior.
    Wood,
}

impl FrameFinish {
    pub const ALL: [FrameFinish; 5] = [
        FrameFinish::White,
        FrameFinish::Almond,
        FrameFinish::Bronze,
        FrameFinish::Black,
        FrameFinish::Wood,
    ];
    pub fn label(self) -> &'static str {
        match self {
            FrameFinish::White => "White",
            FrameFinish::Almond => "Almond",
            FrameFinish::Bronze => "Dark Bronze",
            FrameFinish::Black => "Black",
            FrameFinish::Wood => "Natural Wood",
        }
    }
    /// Shaded colour (sRGB).
    pub fn color(self) -> [u8; 3] {
        match self {
            FrameFinish::White => [240, 240, 236],
            FrameFinish::Almond => [222, 208, 184],
            FrameFinish::Bronze => [74, 60, 48],
            FrameFinish::Black => [34, 34, 34],
            FrameFinish::Wood => [176, 128, 82],
        }
    }
}

impl Grille {
    pub const ALL: [Grille; 4] = [
        Grille::None,
        Grille::Colonial,
        Grille::Prairie,
        Grille::Craftsman,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Grille::None => "None",
            Grille::Colonial => "Colonial",
            Grille::Prairie => "Prairie",
            Grille::Craftsman => "Craftsman",
        }
    }
}

/// Everything about a window type that shapes its drawing, apart from its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowStyle {
    pub family: WindowFamily,
    /// Units mulled side by side (a twin double-hung is 2).
    pub units: u32,
    pub grille: Grille,
    pub finish: FrameFinish,
}

impl WindowStyle {
    pub fn new(family: WindowFamily) -> Self {
        Self {
            family,
            units: 1,
            grille: Grille::None,
            finish: FrameFinish::White,
        }
    }
    /// The style of a window type.
    pub fn of(data: &ElementData) -> Option<Self> {
        match data {
            ElementData::WindowType {
                family,
                units,
                grille,
                finish,
                ..
            } => Some(Self {
                family: *family,
                units: *units,
                grille: *grille,
                finish: *finish,
            }),
            _ => None,
        }
    }
}

/// A family's name and what it's for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FamilyInfo {
    pub family: WindowFamily,
    pub label: &'static str,
    pub description: &'static str,
    /// Whether units can be mulled side by side into twins and triples.
    pub mullable: bool,
    /// Whether it takes grilles (storefront doesn't).
    pub grilles: bool,
}

const fn fam(
    family: WindowFamily,
    label: &'static str,
    description: &'static str,
    mullable: bool,
    grilles: bool,
) -> FamilyInfo {
    FamilyInfo {
        family,
        label,
        description,
        mullable,
        grilles,
    }
}

/// The families, in the order the Window Library lists them.
pub const FAMILIES: &[FamilyInfo] = &[
    fam(
        WindowFamily::DoubleHung,
        "Double-Hung",
        "Two sashes that both slide up and down; the classic American window for traditional houses.",
        true,
        true,
    ),
    fam(
        WindowFamily::SingleHung,
        "Single-Hung",
        "A fixed upper sash over a sliding lower sash; the builder-grade standard in production homes and apartments.",
        true,
        true,
    ),
    fam(
        WindowFamily::Casement,
        "Casement",
        "A side-hinged sash that cranks outward; seals tight and opens fully for egress. Mull two or three for a pair or triple.",
        true,
        true,
    ),
    fam(
        WindowFamily::Awning,
        "Awning",
        "A top-hinged sash that opens out at the bottom and vents in the rain; high on a wall or under a picture window.",
        true,
        true,
    ),
    fam(
        WindowFamily::Hopper,
        "Hopper",
        "A bottom-hinged sash that tips in at the top; basements, bathrooms and laundries.",
        false,
        true,
    ),
    fam(
        WindowFamily::Slider,
        "Horizontal Slider",
        "One sash slides past a fixed lite (XO); vinyl replacement, basements and mid-century houses.",
        false,
        true,
    ),
    fam(
        WindowFamily::Slider3,
        "3-Lite End-Vent Slider",
        "A fixed centre lite with sliding sashes at both ends (XOX); wide living-room and apartment openings.",
        false,
        true,
    ),
    fam(
        WindowFamily::Fixed,
        "Fixed / Picture",
        "A lite that doesn't open, for views and light; at small sizes, transoms and clerestories.",
        true,
        true,
    ),
    fam(
        WindowFamily::PictureCasement,
        "Picture with Flanking Casements",
        "A centre picture window between two casements that swing out to the sides; the living-room staple.",
        false,
        true,
    ),
    fam(
        WindowFamily::PictureAwning,
        "Picture over Awning",
        "A fixed lite over a venting awning; modern houses, multifamily and hotel guest rooms.",
        false,
        true,
    ),
    fam(
        WindowFamily::Bay,
        "Bay",
        "Three units projecting from the wall at 45°: a picture window flanked by double-hungs, with seat and head boards.",
        false,
        true,
    ),
    fam(
        WindowFamily::Storefront,
        "Storefront",
        "Aluminum-framed fixed glazing in vertical lites, with a transom bar when tall; lobbies, retail and amenity spaces.",
        false,
        false,
    ),
];

pub fn info(family: WindowFamily) -> &'static FamilyInfo {
    // Every family is listed (tested); the first stands in otherwise.
    FAMILIES
        .iter()
        .find(|f| f.family == family)
        .unwrap_or(&FAMILIES[0])
}

// ---------------------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------------------

/// Frame face width: head, jambs and sill.
pub const FRAME: f64 = 2.0 * MM_PER_IN;
/// Stile and rail width of an operable sash.
pub const RAIL: f64 = 1.75 * MM_PER_IN;
/// Mullion between lites inside one frame.
pub const MULLION: f64 = 1.5 * MM_PER_IN;
/// Mull joint between units mulled side by side (two frames and the mull).
pub const MULL: f64 = 3.0 * MM_PER_IN;
/// Frame depth through the wall.
pub const DEPTH: f64 = 3.25 * MM_PER_IN;
/// Offset of a hung or sliding sash's track from the frame's centre plane.
pub const TRACK: f64 = 0.75 * MM_PER_IN;
/// Grille bar width.
pub const BAR: f64 = 0.875 * MM_PER_IN;
/// Largest projection of a bay past the wall's outside face.
pub const BAY_PROJECTION: f64 = 18.0 * MM_PER_IN;

/// How a lite opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Fixed,
    /// Side-hinged; `hinge_left` as seen from outside.
    Casement {
        hinge_left: bool,
    },
    /// Top-hinged, opening out.
    Awning,
    /// Bottom-hinged, opening in.
    Hopper,
    /// Slides vertically.
    Hung,
    /// Slides horizontally; `to_right` is the way it opens, seen from outside.
    Slide {
        to_right: bool,
    },
}

/// One sash, or one piece of glass glazed straight into the frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Lite {
    /// Outside edges (u0, z0, u1, z1) of the sash, or of the glass for a lite with no sash.
    pub rect: [f64; 4],
    pub operation: Operation,
    /// Stile and rail width; 0 where the glass sits in the frame.
    pub rail: f64,
    /// Depth offset from the frame's centre plane, + outward: hung and sliding sashes run
    /// in two tracks.
    pub track: f64,
    /// Grille bars as centre lines (u0, z0, u1, z1) across the glass.
    pub bars: Vec<[f64; 4]>,
}

impl Lite {
    /// The glass (u0, z0, u1, z1): inside the stiles and rails.
    pub fn glass(&self) -> [f64; 4] {
        let [u0, z0, u1, z1] = self.rect;
        let r = self.rail;
        [u0 + r, z0 + r, u1 - r, z1 - r]
    }
}

/// A window type's parts at a given size.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowLayout {
    pub width: f64,
    pub height: f64,
    /// Frame face width (the storefront's is its aluminum frame).
    pub frame: f64,
    pub depth: f64,
    /// Mullions and mulls between lites and units, as (u0, z0, u1, z1).
    pub mullions: Vec<[f64; 4]>,
    pub lites: Vec<Lite>,
    /// A bay's projection past the wall's outside face at the centre unit; 0 otherwise.
    pub projection: f64,
}

impl WindowLayout {
    /// Where a bay's flankers meet the centre unit, as u; empty for flat windows.
    pub fn corners(&self) -> Vec<f64> {
        if self.projection > 0.0 {
            vec![self.projection, self.width - self.projection]
        } else {
            vec![]
        }
    }
    /// How far out of the wall's centre plane the unit sits at `u`, for a bay: 0 at the
    /// ends, rising at 45° to the projection. `face` is the wall's half thickness.
    pub fn bay_offset(&self, u: f64, face: f64) -> f64 {
        if self.projection <= 0.0 {
            return 0.0;
        }
        face + u.min(self.width - u).clamp(0.0, self.projection)
    }
}

fn lite(rect: [f64; 4], operation: Operation, rail: f64, track: f64) -> Lite {
    Lite {
        rect,
        operation,
        rail,
        track,
        bars: vec![],
    }
}

/// Splits [a, b] into `n` equal cells with `gap` between them.
fn cells(a: f64, b: f64, n: u32, gap: f64) -> Vec<(f64, f64)> {
    let n = n.max(1);
    let w = (b - a - gap * f64::from(n - 1)) / f64::from(n);
    (0..n)
        .map(|i| {
            let s = a + f64::from(i) * (w + gap);
            (s, s + w)
        })
        .collect()
}

/// The parts of a window of `style`, `width` by `height`.
pub fn layout(style: WindowStyle, width: f64, height: f64) -> WindowLayout {
    let f = if style.family == WindowFamily::Storefront {
        1.75 * MM_PER_IN
    } else {
        FRAME
    };
    let depth = if style.family == WindowFamily::Storefront {
        4.5 * MM_PER_IN
    } else {
        DEPTH
    };
    // Keep tiny windows drawable.
    let f = f.min(width / 6.0).min(height / 6.0);
    let (u0, u1, z0, z1) = (f, width - f, f, height - f);
    let mut mullions = vec![];
    let mut lites = vec![];
    let mut projection = 0.0;
    let mid_z = (z0 + z1) / 2.0;
    let hung = |lites: &mut Vec<Lite>, a: f64, b: f64, top: Operation| {
        lites.push(lite(
            [a, z0, b, mid_z + RAIL / 2.0],
            Operation::Hung,
            RAIL,
            -TRACK,
        ));
        lites.push(lite([a, mid_z - RAIL / 2.0, b, z1], top, RAIL, TRACK));
    };
    let units = if info(style.family).mullable {
        style.units.clamp(1, 4)
    } else {
        1
    };
    let unit_cells = cells(u0, u1, units, MULL);
    for w in unit_cells.windows(2) {
        mullions.push([w[0].1, z0, w[1].0, z1]);
    }
    match style.family {
        WindowFamily::DoubleHung | WindowFamily::SingleHung => {
            let top = if style.family == WindowFamily::DoubleHung {
                Operation::Hung
            } else {
                Operation::Fixed
            };
            for (a, b) in &unit_cells {
                hung(&mut lites, *a, *b, top);
            }
        }
        WindowFamily::Casement | WindowFamily::Awning | WindowFamily::Fixed => {
            let n = unit_cells.len();
            for (i, (a, b)) in unit_cells.iter().enumerate() {
                let (op, rail) = match style.family {
                    WindowFamily::Casement => (
                        Operation::Casement {
                            hinge_left: 2 * i < n,
                        },
                        RAIL,
                    ),
                    WindowFamily::Awning => (Operation::Awning, RAIL),
                    _ => (Operation::Fixed, 0.0),
                };
                lites.push(lite([*a, z0, *b, z1], op, rail, 0.0));
            }
        }
        WindowFamily::Hopper => lites.push(lite([u0, z0, u1, z1], Operation::Hopper, RAIL, 0.0)),
        WindowFamily::Slider => {
            let m = (u0 + u1) / 2.0;
            lites.push(lite(
                [u0, z0, m + RAIL / 2.0, z1],
                Operation::Slide { to_right: true },
                RAIL,
                -TRACK,
            ));
            lites.push(lite(
                [m - RAIL / 2.0, z0, u1, z1],
                Operation::Fixed,
                RAIL,
                TRACK,
            ));
        }
        WindowFamily::Slider3 => {
            let q = (u1 - u0) / 4.0;
            let (a, b) = (u0 + q, u1 - q);
            lites.push(lite(
                [u0, z0, a + RAIL / 2.0, z1],
                Operation::Slide { to_right: true },
                RAIL,
                -TRACK,
            ));
            lites.push(lite(
                [a - RAIL / 2.0, z0, b + RAIL / 2.0, z1],
                Operation::Fixed,
                RAIL,
                TRACK,
            ));
            lites.push(lite(
                [b - RAIL / 2.0, z0, u1, z1],
                Operation::Slide { to_right: false },
                RAIL,
                -TRACK,
            ));
        }
        WindowFamily::PictureCasement => {
            // Casements a quarter each, the picture half, split by mullions.
            let q = (u1 - u0 - 2.0 * MULLION) / 4.0;
            let (a, b) = (u0 + q, u1 - q);
            lites.push(lite(
                [u0, z0, a, z1],
                Operation::Casement { hinge_left: true },
                RAIL,
                0.0,
            ));
            mullions.push([a, z0, a + MULLION, z1]);
            lites.push(lite(
                [a + MULLION, z0, b - MULLION, z1],
                Operation::Fixed,
                0.0,
                0.0,
            ));
            mullions.push([b - MULLION, z0, b, z1]);
            lites.push(lite(
                [b, z0, u1, z1],
                Operation::Casement { hinge_left: false },
                RAIL,
                0.0,
            ));
        }
        WindowFamily::PictureAwning => {
            // The awning a third of the height, under the picture.
            let a = z0 + (z1 - z0 - MULLION) / 3.0;
            lites.push(lite([u0, z0, u1, a], Operation::Awning, RAIL, 0.0));
            mullions.push([u0, a, u1, a + MULLION]);
            lites.push(lite([u0, a + MULLION, u1, z1], Operation::Fixed, 0.0, 0.0));
        }
        WindowFamily::Bay => {
            projection = BAY_PROJECTION.min(width / 4.0);
            let (a, b) = (projection, width - projection);
            // Corner posts where the flankers meet the centre unit.
            mullions.push([a - MULLION / 2.0, z0, a + MULLION / 2.0, z1]);
            mullions.push([b - MULLION / 2.0, z0, b + MULLION / 2.0, z1]);
            hung(&mut lites, u0, a - MULLION / 2.0, Operation::Hung);
            lites.push(lite(
                [a + MULLION / 2.0, z0, b - MULLION / 2.0, z1],
                Operation::Fixed,
                0.0,
                0.0,
            ));
            hung(&mut lites, b + MULLION / 2.0, u1, Operation::Hung);
        }
        WindowFamily::Storefront => {
            // Lites at most 5' wide, and a transom bar at door-head height when tall.
            let mull = 2.0 * MM_PER_IN;
            let n = ((u1 - u0) / (60.0 * MM_PER_IN)).ceil().max(1.0) as u32;
            let columns = cells(u0, u1, n, mull);
            for w in columns.windows(2) {
                mullions.push([w[0].1, z0, w[1].0, z1]);
            }
            let bar = (height > 102.0 * MM_PER_IN).then_some(84.0 * MM_PER_IN);
            if let Some(t) = bar {
                mullions.push([u0, t, u1, t + mull]);
            }
            for (a, b) in columns {
                match bar {
                    Some(t) => {
                        lites.push(lite([a, z0, b, t], Operation::Fixed, 0.0, 0.0));
                        lites.push(lite([a, t + mull, b, z1], Operation::Fixed, 0.0, 0.0));
                    }
                    None => lites.push(lite([a, z0, b, z1], Operation::Fixed, 0.0, 0.0)),
                }
            }
        }
    }
    if info(style.family).grilles {
        let hung_family = matches!(
            style.family,
            WindowFamily::DoubleHung | WindowFamily::SingleHung | WindowFamily::Bay
        );
        for l in &mut lites {
            let upper = l.track > 0.0;
            l.bars = grille_bars(style.grille, l.glass(), hung_family, upper);
        }
    }
    WindowLayout {
        width,
        height,
        frame: f,
        depth,
        mullions,
        lites,
        projection,
    }
}

/// Grille bars across glass `g` (u0, z0, u1, z1). `hung` and `upper` say whether it's a
/// hung window's upper sash, which alone takes a craftsman grille.
fn grille_bars(grille: Grille, g: [f64; 4], hung: bool, upper: bool) -> Vec<[f64; 4]> {
    let [u0, z0, u1, z1] = g;
    let (w, h) = (u1 - u0, z1 - z0);
    if w <= 0.0 || h <= 0.0 {
        return vec![];
    }
    let mut bars = vec![];
    let verticals = |bars: &mut Vec<[f64; 4]>, n: u32, za: f64, zb: f64| {
        for k in 1..n {
            let u = u0 + w * f64::from(k) / f64::from(n);
            bars.push([u, za, u, zb]);
        }
    };
    match grille {
        Grille::None => {}
        Grille::Colonial => {
            let cols = (w / (9.0 * MM_PER_IN)).round().max(1.0) as u32;
            let rows = (h / (12.0 * MM_PER_IN)).round().max(1.0) as u32;
            verticals(&mut bars, cols, z0, z1);
            for k in 1..rows {
                let z = z0 + h * f64::from(k) / f64::from(rows);
                bars.push([u0, z, u1, z]);
            }
        }
        Grille::Prairie => {
            let i = 3.5 * MM_PER_IN;
            if w > 3.0 * i && h > 3.0 * i {
                bars.push([u0 + i, z0, u0 + i, z1]);
                bars.push([u1 - i, z0, u1 - i, z1]);
                bars.push([u0, z0 + i, u1, z0 + i]);
                bars.push([u0, z1 - i, u1, z1 - i]);
            }
        }
        Grille::Craftsman => {
            if hung {
                if upper {
                    verticals(&mut bars, 3, z0, z1);
                }
            } else {
                // A band of three lites across the top quarter.
                let z = z1 - h * 0.28;
                bars.push([u0, z, u1, z]);
                verticals(&mut bars, 3, z, z1);
            }
        }
    }
    bars
}

// ---------------------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------------------

/// A standard size of a family, in inches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    pub family: WindowFamily,
    pub units: u32,
    pub width: f64,
    pub height: f64,
    pub sill: f64,
}

/// Head height that windows line up with: the top of a 7'-0" door.
const HEAD: f64 = 84.0;

const fn at_head(family: WindowFamily, units: u32, width: f64, height: f64) -> Preset {
    let sill = HEAD - height;
    Preset {
        family,
        units,
        width,
        height,
        sill: if sill < 12.0 { 12.0 } else { sill },
    }
}

const fn sized(family: WindowFamily, units: u32, width: f64, height: f64, sill: f64) -> Preset {
    Preset {
        family,
        units,
        width,
        height,
        sill,
    }
}

use WindowFamily as F;

/// Standard US sizes (nominal rough openings), heads at 7'-0" unless noted.
pub const CATALOG: &[Preset] = &[
    // Double-hung: 2436 to 3672, twins and a triple.
    at_head(F::DoubleHung, 1, 24.0, 36.0),
    at_head(F::DoubleHung, 1, 28.0, 54.0),
    at_head(F::DoubleHung, 1, 30.0, 48.0),
    at_head(F::DoubleHung, 1, 30.0, 60.0),
    at_head(F::DoubleHung, 1, 32.0, 54.0),
    at_head(F::DoubleHung, 1, 36.0, 48.0),
    at_head(F::DoubleHung, 1, 36.0, 60.0),
    at_head(F::DoubleHung, 1, 36.0, 72.0),
    at_head(F::DoubleHung, 2, 60.0, 60.0),
    at_head(F::DoubleHung, 2, 72.0, 60.0),
    at_head(F::DoubleHung, 3, 108.0, 60.0),
    // Single-hung.
    at_head(F::SingleHung, 1, 24.0, 36.0),
    at_head(F::SingleHung, 1, 30.0, 48.0),
    at_head(F::SingleHung, 1, 32.0, 60.0),
    at_head(F::SingleHung, 1, 36.0, 48.0),
    at_head(F::SingleHung, 1, 36.0, 60.0),
    at_head(F::SingleHung, 1, 36.0, 72.0),
    at_head(F::SingleHung, 2, 72.0, 60.0),
    // Casements, pairs and a triple.
    at_head(F::Casement, 1, 24.0, 36.0),
    at_head(F::Casement, 1, 24.0, 48.0),
    at_head(F::Casement, 1, 24.0, 60.0),
    at_head(F::Casement, 1, 30.0, 48.0),
    at_head(F::Casement, 1, 30.0, 60.0),
    sized(F::Casement, 1, 36.0, 48.0, 36.0),
    at_head(F::Casement, 2, 48.0, 48.0),
    at_head(F::Casement, 2, 60.0, 48.0),
    at_head(F::Casement, 2, 60.0, 60.0),
    at_head(F::Casement, 2, 72.0, 60.0),
    at_head(F::Casement, 3, 90.0, 60.0),
    // Awnings.
    at_head(F::Awning, 1, 24.0, 20.0),
    at_head(F::Awning, 1, 30.0, 24.0),
    at_head(F::Awning, 1, 36.0, 24.0),
    at_head(F::Awning, 1, 48.0, 24.0),
    at_head(F::Awning, 1, 36.0, 36.0),
    at_head(F::Awning, 2, 72.0, 24.0),
    // Hoppers.
    at_head(F::Hopper, 1, 32.0, 16.0),
    at_head(F::Hopper, 1, 32.0, 20.0),
    at_head(F::Hopper, 1, 32.0, 24.0),
    at_head(F::Hopper, 1, 36.0, 24.0),
    // Sliders.
    at_head(F::Slider, 1, 36.0, 24.0),
    at_head(F::Slider, 1, 48.0, 36.0),
    at_head(F::Slider, 1, 48.0, 48.0),
    at_head(F::Slider, 1, 60.0, 36.0),
    at_head(F::Slider, 1, 60.0, 48.0),
    at_head(F::Slider, 1, 72.0, 48.0),
    at_head(F::Slider3, 1, 96.0, 48.0),
    at_head(F::Slider3, 1, 96.0, 60.0),
    at_head(F::Slider3, 1, 108.0, 48.0),
    at_head(F::Slider3, 1, 120.0, 60.0),
    // Fixed: pictures, then transoms over 7'-0" doors.
    at_head(F::Fixed, 1, 24.0, 24.0),
    at_head(F::Fixed, 1, 36.0, 36.0),
    sized(F::Fixed, 1, 48.0, 48.0, 36.0),
    at_head(F::Fixed, 1, 60.0, 48.0),
    at_head(F::Fixed, 1, 60.0, 60.0),
    sized(F::Fixed, 1, 72.0, 60.0, 30.0),
    at_head(F::Fixed, 1, 96.0, 72.0),
    sized(F::Fixed, 1, 36.0, 12.0, 86.0),
    sized(F::Fixed, 1, 36.0, 18.0, 86.0),
    sized(F::Fixed, 1, 72.0, 16.0, 86.0),
    sized(F::Fixed, 1, 72.0, 18.0, 86.0),
    // Combinations.
    at_head(F::PictureCasement, 1, 72.0, 48.0),
    at_head(F::PictureCasement, 1, 96.0, 48.0),
    at_head(F::PictureCasement, 1, 96.0, 60.0),
    at_head(F::PictureCasement, 1, 120.0, 60.0),
    at_head(F::PictureAwning, 1, 36.0, 60.0),
    at_head(F::PictureAwning, 1, 48.0, 72.0),
    at_head(F::PictureAwning, 1, 60.0, 72.0),
    at_head(F::PictureAwning, 1, 72.0, 72.0),
    // Bays.
    at_head(F::Bay, 1, 72.0, 48.0),
    at_head(F::Bay, 1, 84.0, 54.0),
    at_head(F::Bay, 1, 96.0, 60.0),
    at_head(F::Bay, 1, 108.0, 60.0),
    // Storefront on a 6" curb.
    sized(F::Storefront, 1, 60.0, 96.0, 6.0),
    sized(F::Storefront, 1, 72.0, 96.0, 6.0),
    sized(F::Storefront, 1, 120.0, 96.0, 6.0),
    sized(F::Storefront, 1, 120.0, 120.0, 6.0),
];

/// What a new project starts with: the common sizes of every family. The first three are
/// the original built-in types, kept by name.
pub const STARTER: &[(WindowFamily, u32, f64, f64)] = &[
    (F::Fixed, 1, 48.0, 48.0),
    (F::Casement, 1, 36.0, 48.0),
    (F::Fixed, 1, 72.0, 60.0),
    (F::DoubleHung, 1, 30.0, 60.0),
    (F::DoubleHung, 1, 36.0, 60.0),
    (F::DoubleHung, 2, 72.0, 60.0),
    (F::SingleHung, 1, 36.0, 60.0),
    (F::Casement, 2, 60.0, 48.0),
    (F::Awning, 1, 36.0, 24.0),
    (F::Hopper, 1, 32.0, 20.0),
    (F::Slider, 1, 60.0, 48.0),
    (F::Slider3, 1, 96.0, 60.0),
    (F::Fixed, 1, 36.0, 18.0),
    (F::PictureCasement, 1, 96.0, 60.0),
    (F::PictureAwning, 1, 48.0, 72.0),
    (F::Bay, 1, 96.0, 60.0),
    (F::Storefront, 1, 120.0, 96.0),
];

/// The catalog entry for a size, if it's a standard one.
pub fn preset(family: WindowFamily, units: u32, width_in: f64, height_in: f64) -> Option<Preset> {
    CATALOG
        .iter()
        .find(|p| {
            p.family == family
                && p.units == units
                && (p.width - width_in).abs() < 0.01
                && (p.height - height_in).abs() < 0.01
        })
        .copied()
}

fn inches(mm: f64) -> String {
    let i = mm / MM_PER_IN;
    if (i - i.round()).abs() < 0.01 {
        format!("{}\"", i.round())
    } else {
        format!("{:.1}\"", i)
    }
}

/// The short name types of a family take: "Double Hung Twin", "Casement Pair", "Transom".
pub fn base_name(family: WindowFamily, units: u32, width: f64, height: f64) -> String {
    let units = if info(family).mullable { units } else { 1 };
    let name = match family {
        F::DoubleHung => "Double Hung",
        F::SingleHung => "Single Hung",
        F::Casement => match units {
            2 => return "Casement Pair".into(),
            3 => return "Casement Triple".into(),
            _ => "Casement",
        },
        F::Awning => "Awning",
        F::Hopper => "Hopper",
        F::Slider => "Slider",
        F::Slider3 => "Slider 3-Lite",
        F::Fixed if units == 1 && height <= 20.0 * MM_PER_IN && width >= 2.0 * height => "Transom",
        F::Fixed => "Fixed",
        F::PictureCasement => "Picture + Casements",
        F::PictureAwning => "Picture + Awning",
        F::Bay => "Bay",
        F::Storefront => "Storefront",
    };
    match units {
        1 => name.into(),
        2 => format!("{name} Twin"),
        3 => format!("{name} Triple"),
        n => format!("{name} {n}-Wide"),
    }
}

/// A type's name, as Revit names them: `Double Hung 36" x 60" - Colonial, Black`.
pub fn type_name(style: WindowStyle, width: f64, height: f64) -> String {
    let mut name = format!(
        "{} {} x {}",
        base_name(style.family, style.units, width, height),
        inches(width),
        inches(height)
    );
    let mut extra = vec![];
    if style.grille != Grille::None && info(style.family).grilles {
        extra.push(style.grille.label());
    }
    if style.finish != FrameFinish::White {
        extra.push(style.finish.label());
    }
    if !extra.is_empty() {
        name.push_str(" - ");
        name.push_str(&extra.join(", "));
    }
    name
}

/// A window type to load: a family at a size, with options. Lengths mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WindowSpec {
    pub family: WindowFamily,
    pub units: u32,
    pub width: f64,
    pub height: f64,
    pub sill: f64,
    pub grille: Grille,
    pub finish: FrameFinish,
}

impl WindowSpec {
    pub fn style(&self) -> WindowStyle {
        WindowStyle {
            family: self.family,
            units: if info(self.family).mullable {
                self.units.clamp(1, 4)
            } else {
                1
            },
            grille: if info(self.family).grilles {
                self.grille
            } else {
                Grille::None
            },
            finish: self.finish,
        }
    }
    pub fn name(&self) -> String {
        type_name(self.style(), self.width, self.height)
    }
    pub fn data(&self) -> ElementData {
        let s = self.style();
        ElementData::WindowType {
            name: self.name(),
            family: s.family,
            width: self.width,
            height: self.height,
            sill: self.sill,
            units: s.units,
            grille: s.grille,
            finish: s.finish,
        }
    }
    fn check(&self) -> CoreResult<()> {
        let min = 12.0 * MM_PER_IN;
        let min_w = match self.family {
            // A bay needs room for three units.
            F::Bay => 48.0 * MM_PER_IN,
            _ => min * f64::from(self.style().units),
        };
        if !(self.width.is_finite() && self.height.is_finite() && self.sill.is_finite()) {
            return Err(CoreError::Invalid("window size must be a number".into()));
        }
        if self.width < min_w || self.height < min {
            return Err(CoreError::Invalid(format!(
                "a {} window must be at least {} wide and 1'-0\" tall",
                info(self.family).label.to_lowercase(),
                crate::units::format_ft_in(min_w)
            )));
        }
        if self.width > 40.0 * 12.0 * MM_PER_IN || self.height > 20.0 * 12.0 * MM_PER_IN {
            return Err(CoreError::Invalid(
                "a window can be at most 40' wide and 20' tall".into(),
            ));
        }
        if self.sill < 0.0 {
            return Err(CoreError::Invalid("sill height can't be negative".into()));
        }
        Ok(())
    }
}

impl From<Preset> for WindowSpec {
    fn from(p: Preset) -> Self {
        WindowSpec {
            family: p.family,
            units: p.units,
            width: p.width * MM_PER_IN,
            height: p.height * MM_PER_IN,
            sill: p.sill * MM_PER_IN,
            grille: Grille::None,
            finish: FrameFinish::White,
        }
    }
}

/// The starter types, as specs.
pub fn starter() -> Vec<WindowSpec> {
    STARTER
        .iter()
        .filter_map(|(f, u, w, h)| preset(*f, *u, *w, *h).map(Into::into))
        .collect()
}

/// Loads window types into the project, one undo step; a type already there by name is
/// reused. Returns the type for each spec, in order.
pub fn load(doc: &mut Document, specs: &[WindowSpec]) -> CoreResult<Vec<ElementId>> {
    if specs.is_empty() {
        return Err(CoreError::Invalid("choose a window size to load".into()));
    }
    for s in specs {
        s.check()?;
    }
    let label = if specs.len() == 1 {
        format!("Load {}", specs[0].name())
    } else {
        format!("Load {} window types", specs.len())
    };
    doc.transact(&label, |tx| {
        let mut out = vec![];
        for s in specs {
            let name = s.name();
            let existing = tx
                .of(Category::WindowType)
                .find(|e| e.data.name() == name)
                .map(|e| e.id);
            out.push(match existing {
                Some(id) => id,
                None => tx.insert(s.data()),
            });
        }
        Ok(out)
    })
}

/// The project's window type of `family` nearest `width` (mm), if it has one.
pub fn nearest_type(doc: &Document, family: WindowFamily, width: f64) -> Option<ElementId> {
    doc.of(Category::WindowType)
        .filter_map(|e| match &e.data {
            ElementData::WindowType {
                family: f,
                width: w,
                units,
                ..
            } if *f == family && *units == 1 => Some((e.id, (w - width).abs())),
            _ => None,
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IN: f64 = MM_PER_IN;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn a_double_hung_has_two_sashes_in_two_tracks() {
        let l = layout(WindowStyle::new(F::DoubleHung), 36.0 * IN, 60.0 * IN);
        assert_eq!(l.lites.len(), 2);
        let (bottom, top) = (&l.lites[0], &l.lites[1]);
        assert_eq!(bottom.operation, Operation::Hung);
        assert_eq!(top.operation, Operation::Hung);
        // Inside the 2" frame, meeting at mid-height with the rails overlapping.
        assert!(close(bottom.rect[0], 2.0 * IN) && close(bottom.rect[2], 34.0 * IN));
        assert!(close(bottom.rect[1], 2.0 * IN) && close(top.rect[3], 58.0 * IN));
        assert!(close(bottom.rect[3] - top.rect[1], RAIL));
        assert!(
            bottom.track < 0.0 && top.track > 0.0,
            "lower sash runs inside"
        );
        let single = layout(WindowStyle::new(F::SingleHung), 36.0 * IN, 60.0 * IN);
        assert_eq!(single.lites[1].operation, Operation::Fixed);
    }

    #[test]
    fn colonial_grilles_make_six_over_six() {
        let mut s = WindowStyle::new(F::DoubleHung);
        s.grille = Grille::Colonial;
        let l = layout(s, 36.0 * IN, 60.0 * IN);
        // Each sash: 3 lites across (2 vertical bars) and 2 high (1 horizontal bar).
        for sash in &l.lites {
            let v = sash.bars.iter().filter(|b| b[0] == b[2]).count();
            let h = sash.bars.iter().filter(|b| b[1] == b[3]).count();
            assert_eq!((v, h), (2, 1));
        }
        s.grille = Grille::Craftsman;
        let l = layout(s, 36.0 * IN, 60.0 * IN);
        assert_eq!(
            l.lites[0].bars.len(),
            0,
            "3-over-1: the lower sash is clear"
        );
        assert_eq!(l.lites[1].bars.len(), 2);
        s.grille = Grille::Prairie;
        assert_eq!(layout(s, 36.0 * IN, 60.0 * IN).lites[0].bars.len(), 4);
    }

    #[test]
    fn mulled_casements_hinge_at_the_outside() {
        let mut s = WindowStyle::new(F::Casement);
        s.units = 2;
        let l = layout(s, 60.0 * IN, 48.0 * IN);
        assert_eq!(l.lites.len(), 2);
        assert_eq!(l.mullions.len(), 1);
        assert!(close(l.mullions[0][2] - l.mullions[0][0], MULL));
        assert_eq!(
            l.lites[0].operation,
            Operation::Casement { hinge_left: true }
        );
        assert_eq!(
            l.lites[1].operation,
            Operation::Casement { hinge_left: false }
        );
        // Equal sashes: (60 - 4 - 3) / 2 = 26.5" each.
        assert!(close(l.lites[0].rect[2] - l.lites[0].rect[0], 26.5 * IN));
    }

    #[test]
    fn combinations_split_as_built() {
        let l = layout(WindowStyle::new(F::PictureCasement), 96.0 * IN, 60.0 * IN);
        let w: Vec<f64> = l
            .lites
            .iter()
            .map(|x| (x.rect[2] - x.rect[0]) / IN)
            .collect();
        // (96 - 4 - 3) / 4 = 22.25" casements; the picture twice that.
        assert!(
            close(w[0], 22.25) && close(w[2], 22.25) && close(w[1], 44.5),
            "{w:?}"
        );
        let l = layout(WindowStyle::new(F::PictureAwning), 48.0 * IN, 72.0 * IN);
        assert_eq!(l.lites[0].operation, Operation::Awning);
        let h = (l.lites[0].rect[3] - l.lites[0].rect[1]) / IN;
        assert!(close(h, (72.0 - 4.0 - 1.5) / 3.0), "{h}");
        let l = layout(WindowStyle::new(F::Slider3), 96.0 * IN, 60.0 * IN);
        assert_eq!(
            l.lites.iter().map(|x| x.operation).collect::<Vec<_>>(),
            vec![
                Operation::Slide { to_right: true },
                Operation::Fixed,
                Operation::Slide { to_right: false }
            ]
        );
    }

    #[test]
    fn a_bay_projects_at_45_degrees() {
        let l = layout(WindowStyle::new(F::Bay), 96.0 * IN, 60.0 * IN);
        assert!(close(l.projection, 18.0 * IN));
        let c = l.corners();
        assert!(close(c[0], 18.0 * IN) && close(c[1], 78.0 * IN));
        // Two double-hung flankers and a picture.
        assert_eq!(l.lites.len(), 5);
        let face = 3.0 * IN;
        assert!(close(l.bay_offset(0.0, face), face));
        assert!(close(l.bay_offset(9.0 * IN, face), face + 9.0 * IN));
        assert!(close(l.bay_offset(48.0 * IN, face), face + 18.0 * IN));
        assert!(close(l.bay_offset(96.0 * IN, face), face));
        // Small bays keep a centre unit half the width.
        assert!(close(
            layout(WindowStyle::new(F::Bay), 60.0 * IN, 48.0 * IN).projection,
            15.0 * IN
        ));
    }

    #[test]
    fn storefront_splits_into_five_foot_lites_with_a_transom() {
        let l = layout(WindowStyle::new(F::Storefront), 120.0 * IN, 120.0 * IN);
        // 120 - 3.5 = 116.5" across: two lites, each split by the transom bar.
        assert_eq!(l.lites.len(), 4);
        assert_eq!(l.mullions.len(), 2);
        assert!(l.lites.iter().all(|x| x.bars.is_empty() && x.rail == 0.0));
        let short = layout(WindowStyle::new(F::Storefront), 120.0 * IN, 96.0 * IN);
        assert_eq!(short.lites.len(), 2, "no transom bar under 8'-6\"");
    }

    #[test]
    fn names_follow_revit() {
        let mut s = WindowStyle::new(F::DoubleHung);
        assert_eq!(
            type_name(s, 36.0 * IN, 60.0 * IN),
            "Double Hung 36\" x 60\""
        );
        s.units = 2;
        s.grille = Grille::Colonial;
        s.finish = FrameFinish::Black;
        assert_eq!(
            type_name(s, 72.0 * IN, 60.0 * IN),
            "Double Hung Twin 72\" x 60\" - Colonial, Black"
        );
        let t = WindowStyle::new(F::Fixed);
        assert_eq!(type_name(t, 36.0 * IN, 12.0 * IN), "Transom 36\" x 12\"");
        assert_eq!(type_name(t, 48.0 * IN, 48.0 * IN), "Fixed 48\" x 48\"");
        let mut c = WindowStyle::new(F::Casement);
        c.units = 2;
        assert_eq!(
            type_name(c, 60.0 * IN, 48.0 * IN),
            "Casement Pair 60\" x 48\""
        );
        // The storefront ignores a grille.
        let mut sf = WindowStyle::new(F::Storefront);
        sf.grille = Grille::Colonial;
        assert_eq!(
            type_name(sf, 120.0 * IN, 96.0 * IN),
            "Storefront 120\" x 96\""
        );
    }

    #[test]
    fn the_catalog_is_unique_and_heads_line_up() {
        let names: Vec<String> = CATALOG
            .iter()
            .map(|p| WindowSpec::from(*p).name())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "{names:?}");
        assert!(CATALOG.len() >= 70);
        for f in FAMILIES {
            assert!(CATALOG.iter().any(|p| p.family == f.family), "{}", f.label);
        }
        // Heads at 7'-0" except the tallest, transoms and storefront.
        let dh = preset(F::DoubleHung, 1, 36.0, 60.0).unwrap();
        assert_eq!(dh.sill + dh.height, 84.0);
        assert_eq!(preset(F::DoubleHung, 1, 36.0, 72.0).unwrap().sill, 12.0);
        // Every starter size is in the catalog, and the originals keep their names.
        let s = starter();
        assert_eq!(s.len(), STARTER.len());
        for f in [
            F::Fixed,
            F::Casement,
            F::DoubleHung,
            F::SingleHung,
            F::Awning,
            F::Hopper,
            F::Slider,
            F::Slider3,
            F::PictureCasement,
            F::PictureAwning,
            F::Bay,
            F::Storefront,
        ] {
            assert_eq!(info(f).family, f);
        }
        assert_eq!(s[0].name(), "Fixed 48\" x 48\"");
        assert_eq!(s[1].name(), "Casement 36\" x 48\"");
        assert_eq!(s[2].name(), "Fixed 72\" x 60\"");
        assert!(close(s[0].sill, 36.0 * IN) && close(s[2].sill, 30.0 * IN));
    }

    #[test]
    fn loading_reuses_types_by_name_and_checks_sizes() {
        let mut doc = Document::new();
        let spec: WindowSpec = preset(F::Awning, 1, 36.0, 24.0).unwrap().into();
        let a = load(&mut doc, &[spec]).unwrap();
        let b = load(&mut doc, &[spec]).unwrap();
        assert_eq!(a, b);
        assert_eq!(doc.of(Category::WindowType).count(), 1);
        let mut black = spec;
        black.finish = FrameFinish::Black;
        let c = load(&mut doc, &[black]).unwrap();
        assert_ne!(a, c);
        assert_eq!(doc.data(c[0]).unwrap().name(), "Awning 36\" x 24\" - Black");
        // One undo removes a batch.
        let before = doc.of(Category::WindowType).count();
        let batch: Vec<WindowSpec> = CATALOG.iter().take(5).map(|p| (*p).into()).collect();
        load(&mut doc, &batch).unwrap();
        doc.undo().unwrap();
        assert_eq!(doc.of(Category::WindowType).count(), before);
        let mut tiny = spec;
        tiny.width = 6.0 * IN;
        assert!(load(&mut doc, &[tiny]).is_err());
        let mut bay: WindowSpec = preset(F::Bay, 1, 96.0, 60.0).unwrap().into();
        bay.width = 40.0 * IN;
        assert!(load(&mut doc, &[bay]).is_err());
        assert!(load(&mut doc, &[]).is_err());
    }
}
