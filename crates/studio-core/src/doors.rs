//! Door families (ADR-033): the door types common in US construction, glass doors included,
//! how each divides into frame, leaves, panels and lites, and a catalog of standard sizes to
//! load as types.
//!
//! A layout runs `u` along the wall from the opening's start (the wall-start side) to its
//! width, and `z` up from the floor. Lengths are mm.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, DoorFamily, ElementData, ElementId};
use crate::units::{format_ft_in, MM_PER_IN};

/// What a door leaf looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LeafStyle {
    #[default]
    Flush,
    /// Colonial six-panel.
    SixPanel,
    /// One recessed panel in a square frame.
    Shaker,
    FivePanel,
    /// Three lites over two tall panels.
    Craftsman,
    FullLite,
    HalfLite,
    /// French door: 3 x 5 divided lites.
    FifteenLite,
    /// Commercial narrow vision lite.
    VisionLite,
    Louvered,
    /// Barn door with an X brace.
    BarnX,
}

impl LeafStyle {
    pub const ALL: [LeafStyle; 11] = [
        LeafStyle::Flush,
        LeafStyle::SixPanel,
        LeafStyle::Shaker,
        LeafStyle::FivePanel,
        LeafStyle::Craftsman,
        LeafStyle::FullLite,
        LeafStyle::HalfLite,
        LeafStyle::FifteenLite,
        LeafStyle::VisionLite,
        LeafStyle::Louvered,
        LeafStyle::BarnX,
    ];
    pub fn label(self) -> &'static str {
        match self {
            LeafStyle::Flush => "Flush",
            LeafStyle::SixPanel => "Six-Panel",
            LeafStyle::Shaker => "Shaker",
            LeafStyle::FivePanel => "Five-Panel Shaker",
            LeafStyle::Craftsman => "Craftsman",
            LeafStyle::FullLite => "Full Lite",
            LeafStyle::HalfLite => "Half Lite",
            LeafStyle::FifteenLite => "15-Lite French",
            LeafStyle::VisionLite => "Vision Lite",
            LeafStyle::Louvered => "Louvered",
            LeafStyle::BarnX => "X-Brace",
        }
    }
    /// Whether the leaf has glass.
    pub fn glazed(self) -> bool {
        matches!(
            self,
            LeafStyle::Craftsman
                | LeafStyle::FullLite
                | LeafStyle::HalfLite
                | LeafStyle::FifteenLite
                | LeafStyle::VisionLite
        )
    }
}

/// Leaf and frame finish (its colour in 3D and renderings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DoorFinish {
    PaintedWhite,
    PaintedBlack,
    PaintedGray,
    StainedOak,
    Walnut,
    Aluminum,
    Bronze,
}

impl DoorFinish {
    pub const ALL: [DoorFinish; 7] = [
        DoorFinish::PaintedWhite,
        DoorFinish::PaintedBlack,
        DoorFinish::PaintedGray,
        DoorFinish::StainedOak,
        DoorFinish::Walnut,
        DoorFinish::Aluminum,
        DoorFinish::Bronze,
    ];
    pub fn label(self) -> &'static str {
        match self {
            DoorFinish::PaintedWhite => "Painted White",
            DoorFinish::PaintedBlack => "Painted Black",
            DoorFinish::PaintedGray => "Painted Gray",
            DoorFinish::StainedOak => "Stained Oak",
            DoorFinish::Walnut => "Walnut",
            DoorFinish::Aluminum => "Clear Aluminum",
            DoorFinish::Bronze => "Dark Bronze",
        }
    }
    pub fn color(self) -> [u8; 3] {
        match self {
            DoorFinish::PaintedWhite => [238, 238, 234],
            DoorFinish::PaintedBlack => [36, 36, 38],
            DoorFinish::PaintedGray => [150, 154, 158],
            DoorFinish::StainedOak => [168, 118, 70],
            DoorFinish::Walnut => [104, 70, 46],
            DoorFinish::Aluminum => [190, 194, 198],
            DoorFinish::Bronze => [74, 60, 48],
        }
    }
}

/// A family's name, what it's for and which options it takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FamilyInfo {
    pub family: DoorFamily,
    pub label: &'static str,
    pub description: &'static str,
    /// Leaf styles it comes in (empty: all-glass or sectional, no choice).
    pub leaves: &'static [LeafStyle],
    /// (min, max, default) panel count: leaves, sidelites or sections; None when fixed.
    pub panels: Option<(u32, u32, u32)>,
    pub default_finish: DoorFinish,
    /// The panel count's name in the UI.
    pub panels_label: &'static str,
}

use LeafStyle as L;

const SWING_LEAVES: &[LeafStyle] = &[
    L::Flush,
    L::SixPanel,
    L::Shaker,
    L::FivePanel,
    L::Craftsman,
    L::FullLite,
    L::HalfLite,
    L::FifteenLite,
    L::VisionLite,
    L::Louvered,
];

pub const FAMILIES: &[FamilyInfo] = &[
    FamilyInfo {
        family: DoorFamily::SingleFlush,
        label: "Single Swing",
        description:
            "One hinged leaf: interior passage doors, entries, and commercial hollow-metal doors.",
        leaves: SWING_LEAVES,
        panels: None,
        default_finish: DoorFinish::PaintedWhite,
        panels_label: "",
    },
    FamilyInfo {
        family: DoorFamily::DoubleFlush,
        label: "Double Swing (Pair)",
        description:
            "A pair of hinged leaves: French doors to patios, double entries, and wide corridors.",
        leaves: SWING_LEAVES,
        panels: None,
        default_finish: DoorFinish::PaintedWhite,
        panels_label: "",
    },
    FamilyInfo {
        family: DoorFamily::Sidelites,
        label: "Entry with Sidelites",
        description: "A front door flanked by glass sidelites, the classic American entry.",
        leaves: &[
            L::SixPanel,
            L::Craftsman,
            L::Shaker,
            L::FullLite,
            L::HalfLite,
            L::Flush,
        ],
        panels: Some((1, 2, 2)),
        default_finish: DoorFinish::PaintedWhite,
        panels_label: "Sidelites",
    },
    FamilyInfo {
        family: DoorFamily::SlidingGlass,
        label: "Sliding Glass Patio",
        description: "Glass panels that slide past each other (OX, OXO, OXXO) to decks and patios.",
        leaves: &[],
        panels: Some((2, 4, 2)),
        default_finish: DoorFinish::PaintedWhite,
        panels_label: "Panels",
    },
    FamilyInfo {
        family: DoorFamily::Pocket,
        label: "Pocket",
        description: "A leaf that slides into the wall: baths, pantries and tight halls.",
        leaves: &[
            L::Flush,
            L::Shaker,
            L::SixPanel,
            L::FivePanel,
            L::FullLite,
            L::FifteenLite,
        ],
        panels: Some((1, 2, 1)),
        default_finish: DoorFinish::PaintedWhite,
        panels_label: "Leaves",
    },
    FamilyInfo {
        family: DoorFamily::Barn,
        label: "Barn (Surface Sliding)",
        description:
            "A leaf hung from an exposed track on the wall face: farmhouse and loft interiors.",
        leaves: &[L::BarnX, L::Shaker, L::FivePanel, L::Flush, L::FullLite],
        panels: Some((1, 2, 1)),
        default_finish: DoorFinish::StainedOak,
        panels_label: "Leaves",
    },
    FamilyInfo {
        family: DoorFamily::Bifold,
        label: "Bifold (Closet)",
        description: "Hinged panels that fold back on a track: closets and laundry alcoves.",
        leaves: &[L::Flush, L::Louvered, L::Shaker, L::SixPanel, L::FullLite],
        panels: Some((2, 4, 2)),
        default_finish: DoorFinish::PaintedWhite,
        panels_label: "Panels",
    },
    FamilyInfo {
        family: DoorFamily::FoldingWall,
        label: "Folding Glass Wall",
        description: "Glass panels that fold and stack to open a whole wall to the outside.",
        leaves: &[],
        panels: Some((3, 8, 4)),
        default_finish: DoorFinish::PaintedBlack,
        panels_label: "Panels",
    },
    FamilyInfo {
        family: DoorFamily::Storefront,
        label: "Storefront (Aluminum)",
        description: "Narrow-stile aluminum and glass doors: lobbies, retail and amenity entries.",
        leaves: &[],
        panels: Some((1, 2, 1)),
        default_finish: DoorFinish::Aluminum,
        panels_label: "Leaves",
    },
    FamilyInfo {
        family: DoorFamily::Garage,
        label: "Garage (Sectional Overhead)",
        description: "A raised-panel sectional door that rolls up overhead.",
        leaves: &[],
        panels: None,
        default_finish: DoorFinish::PaintedWhite,
        panels_label: "",
    },
];

pub fn info(family: DoorFamily) -> &'static FamilyInfo {
    FAMILIES
        .iter()
        .find(|f| f.family == family)
        .unwrap_or(&FAMILIES[0])
}

/// Everything about a door type that shapes its drawing, apart from its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorStyle {
    pub family: DoorFamily,
    pub leaf: LeafStyle,
    /// Leaves, panels or sidelites, resolved to the family's range.
    pub panels: u32,
    pub finish: DoorFinish,
}

impl DoorStyle {
    pub fn new(family: DoorFamily) -> Self {
        Self::resolve(family, LeafStyle::Flush, 0, None)
    }
    /// A style with the family's defaults filled in and options clamped to what it takes.
    pub fn resolve(
        family: DoorFamily,
        leaf: LeafStyle,
        panels: u32,
        finish: Option<DoorFinish>,
    ) -> Self {
        let f = info(family);
        let leaf = if f.leaves.is_empty() {
            LeafStyle::FullLite
        } else if f.leaves.contains(&leaf) {
            leaf
        } else {
            f.leaves[0]
        };
        let panels = match f.panels {
            Some((lo, hi, d)) => {
                if panels == 0 {
                    d
                } else {
                    panels.clamp(lo, hi)
                }
            }
            None => 1,
        };
        Self {
            family,
            leaf,
            panels,
            finish: finish.unwrap_or(f.default_finish),
        }
    }
    pub fn of(data: &ElementData) -> Option<Self> {
        match data {
            ElementData::DoorType {
                family,
                leaf,
                panels,
                finish,
                ..
            } => Some(Self::resolve(*family, *leaf, *panels, *finish)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------------------

/// Leaf thickness.
pub const LEAF: f64 = 1.75 * MM_PER_IN;
/// Stile and top-rail width of a panel or glass leaf.
pub const STILE: f64 = 4.5 * MM_PER_IN;
/// Bottom rail.
pub const BOTTOM_RAIL: f64 = 9.0 * MM_PER_IN;
/// Jamb face inside the rough opening.
pub const JAMB: f64 = 1.5 * MM_PER_IN;
/// Casing (trim) width around the opening, on both wall faces.
pub const CASING: f64 = 3.5 * MM_PER_IN;
/// Aluminum or vinyl frame face of glass doors.
pub const GLASS_FRAME: f64 = 2.0 * MM_PER_IN;
/// Stile of a sliding, folding or storefront glass panel.
pub const GLASS_STILE: f64 = 2.5 * MM_PER_IN;
/// Muntin width.
pub const BAR: f64 = 0.875 * MM_PER_IN;
/// Track offset of sliding panels from the wall's centre plane.
pub const TRACK: f64 = 0.9 * MM_PER_IN;

/// How a leaf moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorOp {
    /// Hinged at its u0 edge (`hinge_left`) or u1 edge.
    Swing {
        hinge_left: bool,
    },
    /// Slides past another panel.
    Slide {
        to_right: bool,
    },
    Fixed,
    /// Slides into the wall on its u0 side (`into_left`) or u1 side.
    Pocket {
        into_left: bool,
    },
    /// Hangs on the wall face and slides along it.
    Barn {
        to_right: bool,
    },
    /// Folds; `hinge_left` is the jamb its pair hinges from.
    Fold {
        hinge_left: bool,
    },
    Overhead,
}

/// One leaf, panel or sidelite.
#[derive(Debug, Clone, PartialEq)]
pub struct Leaf {
    /// (u0, z0, u1, z1).
    pub rect: [f64; 4],
    pub op: DoorOp,
    /// Depth offset from the wall's centre plane, + toward the swing (facing) side.
    pub track: f64,
    /// Glass openings in the leaf.
    pub glass: Vec<[f64; 4]>,
    /// Recessed panels in the leaf.
    pub panels: Vec<[f64; 4]>,
    /// Muntins, louver slats and braces as centre lines (u0, z0, u1, z1).
    pub bars: Vec<[f64; 4]>,
    /// Louver slats instead of solid panels.
    pub louvered: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoorLayout {
    pub width: f64,
    pub height: f64,
    /// Frame face inside the opening (jamb, or aluminum frame).
    pub frame: f64,
    /// Casing around the opening on the wall faces; 0 for glass and garage doors.
    pub casing: f64,
    pub leaves: Vec<Leaf>,
    /// Mullions between a door and its sidelites.
    pub mullions: Vec<[f64; 4]>,
}

fn leaf(rect: [f64; 4], op: DoorOp, style: LeafStyle, track: f64) -> Leaf {
    let mut l = Leaf {
        rect,
        op,
        track,
        glass: vec![],
        panels: vec![],
        bars: vec![],
        louvered: false,
    };
    fill_style(&mut l, style);
    l
}

/// A glass panel with narrow stiles (sliding, folding, storefront, sidelites).
fn glass_panel(rect: [f64; 4], op: DoorOp, track: f64, stile: f64, bottom: f64) -> Leaf {
    let [u0, z0, u1, z1] = rect;
    Leaf {
        rect,
        op,
        track,
        glass: vec![[u0 + stile, z0 + bottom, u1 - stile, z1 - stile]],
        panels: vec![],
        bars: vec![],
        louvered: false,
    }
}

/// Panels, lites and bars of a leaf style.
fn fill_style(l: &mut Leaf, style: LeafStyle) {
    let [u0, z0, u1, z1] = l.rect;
    let (w, h) = (u1 - u0, z1 - z0);
    let s = STILE.min(w / 5.0);
    let (x0, x1) = (u0 + s, u1 - s);
    let (bot, top) = (z0 + BOTTOM_RAIL.min(h / 8.0), z1 - s);
    let at = |f: f64| z0 + h * f;
    let mid = (x0 + x1) / 2.0;
    let half = s / 2.0;
    match style {
        LeafStyle::Flush => {}
        LeafStyle::SixPanel => {
            // Small top pair, tall middle and bottom pairs, split by a centre stile.
            let rows = [(bot, at(0.42)), (at(0.47), at(0.76)), (at(0.80), top)];
            for (a, b) in rows {
                l.panels.push([x0, a, mid - half, b]);
                l.panels.push([mid + half, a, x1, b]);
            }
        }
        LeafStyle::Shaker => l.panels.push([x0, bot, x1, top]),
        LeafStyle::FivePanel => {
            let n = 5.0;
            let gap = s * 0.8;
            let ph = (top - bot - gap * (n - 1.0)) / n;
            for i in 0..5 {
                let a = bot + f64::from(i) * (ph + gap);
                l.panels.push([x0, a, x1, a + ph]);
            }
        }
        LeafStyle::Craftsman => {
            // Three lites over two tall panels, with a shelf rail between.
            let shelf = at(0.66);
            let lite = [x0, shelf + s, x1, top];
            l.glass.push(lite);
            for k in 1..3 {
                let u = x0 + (x1 - x0) * f64::from(k) / 3.0;
                l.bars.push([u, lite[1], u, lite[3]]);
            }
            l.panels.push([x0, bot, mid - half, shelf]);
            l.panels.push([mid + half, bot, x1, shelf]);
        }
        LeafStyle::FullLite => l.glass.push([x0, bot, x1, top]),
        LeafStyle::HalfLite => {
            let rail = at(0.5);
            l.glass.push([x0, rail + s / 2.0, x1, top]);
            l.panels.push([x0, bot, x1, rail - s / 2.0]);
        }
        LeafStyle::FifteenLite => {
            l.glass.push([x0, bot, x1, top]);
            for k in 1..3 {
                let u = x0 + (x1 - x0) * f64::from(k) / 3.0;
                l.bars.push([u, bot, u, top]);
            }
            for k in 1..5 {
                let z = bot + (top - bot) * f64::from(k) / 5.0;
                l.bars.push([x0, z, x1, z]);
            }
        }
        LeafStyle::VisionLite => {
            // 5" x 30" on the latch side, its top at 66".
            let lw = 5.0 * MM_PER_IN;
            let latch_right = !matches!(l.op, DoorOp::Swing { hinge_left: false });
            let (a, b) = if latch_right {
                (u1 - s - 3.0 * MM_PER_IN - lw, u1 - s - 3.0 * MM_PER_IN)
            } else {
                (u0 + s + 3.0 * MM_PER_IN, u0 + s + 3.0 * MM_PER_IN + lw)
            };
            let zt = (z0 + 66.0 * MM_PER_IN).min(top);
            l.glass.push([a, (zt - 30.0 * MM_PER_IN).max(bot), b, zt]);
        }
        LeafStyle::Louvered => {
            l.louvered = true;
            l.panels.push([x0, bot, x1, top]);
            let pitch = 2.0 * MM_PER_IN;
            let mut z = bot + pitch;
            while z < top - pitch / 2.0 {
                l.bars.push([x0, z, x1, z]);
                z += pitch;
            }
        }
        LeafStyle::BarnX => {
            // Rails top, middle and bottom; an X in each half.
            let m = at(0.5);
            for (a, b) in [(bot, m - s / 2.0), (m + s / 2.0, top)] {
                l.panels.push([x0, a, x1, b]);
                l.bars.push([x0, a, x1, b]);
                l.bars.push([x0, b, x1, a]);
            }
        }
    }
}

/// The parts of a door of `style`, `width` by `height`. `hinge_left` puts a single swing
/// door's hinge (or a pocket's pocket, or a slider's fixed panel) at u = 0.
pub fn layout(style: DoorStyle, width: f64, height: f64, hinge_left: bool) -> DoorLayout {
    let (w, h) = (width, height);
    let mut leaves = vec![];
    let mut mullions = vec![];
    let n = style.panels;
    let (frame, casing) = match style.family {
        DoorFamily::SlidingGlass | DoorFamily::FoldingWall | DoorFamily::Storefront => {
            (GLASS_FRAME, 0.0)
        }
        DoorFamily::Garage => (0.0, CASING),
        _ => (JAMB, CASING),
    };
    let f = frame.min(w / 8.0);
    let (u0, u1, top) = (f, w - f, h - f);
    let cells = |a: f64, b: f64, n: u32| -> Vec<(f64, f64)> {
        let step = (b - a) / f64::from(n.max(1));
        (0..n.max(1))
            .map(|i| (a + step * f64::from(i), a + step * f64::from(i + 1)))
            .collect()
    };
    match style.family {
        DoorFamily::SingleFlush => {
            leaves.push(leaf(
                [u0, 0.0, u1, top],
                DoorOp::Swing { hinge_left },
                style.leaf,
                0.0,
            ));
        }
        DoorFamily::DoubleFlush => {
            let m = w / 2.0;
            leaves.push(leaf(
                [u0, 0.0, m, top],
                DoorOp::Swing { hinge_left: true },
                style.leaf,
                0.0,
            ));
            leaves.push(leaf(
                [m, 0.0, u1, top],
                DoorOp::Swing { hinge_left: false },
                style.leaf,
                0.0,
            ));
        }
        DoorFamily::Sidelites => {
            // A 36" leaf (or what's left), with the sidelites sharing the rest.
            let mull = 1.5 * MM_PER_IN;
            let spare = u1 - u0 - mull * f64::from(n);
            let door = (36.0 * MM_PER_IN).min(spare * 0.7);
            let side = (spare - door) / f64::from(n);
            let (d0, d1) = if n == 2 {
                (u0 + side + mull, u0 + side + mull + door)
            } else if hinge_left {
                (u0, u0 + door)
            } else {
                (u1 - door, u1)
            };
            leaves.push(leaf(
                [d0, 0.0, d1, top],
                DoorOp::Swing { hinge_left },
                style.leaf,
                0.0,
            ));
            let mut sides = vec![];
            if d0 > u0 + 1.0 {
                sides.push([u0, 0.0, d0 - mull, top]);
                mullions.push([d0 - mull, 0.0, d0, top]);
            }
            if d1 < u1 - 1.0 {
                sides.push([d1 + mull, 0.0, u1, top]);
                mullions.push([d1, 0.0, d1 + mull, top]);
            }
            for r in sides {
                leaves.push(glass_panel(
                    r,
                    DoorOp::Fixed,
                    0.0,
                    2.0 * MM_PER_IN,
                    BOTTOM_RAIL,
                ));
            }
        }
        DoorFamily::SlidingGlass => {
            // OX, OXO and OXXO: operable panels run in the inside track.
            let overlap = GLASS_STILE;
            let step = (u1 - u0) / f64::from(n);
            for i in 0..n {
                let a = u0 + step * f64::from(i) - if i > 0 { overlap / 2.0 } else { 0.0 };
                let b = u0 + step * f64::from(i + 1) + if i + 1 < n { overlap / 2.0 } else { 0.0 };
                let operable = match n {
                    2 => (i == 1) == hinge_left,
                    3 => i == 1,
                    _ => i == 1 || i == 2,
                };
                let (op, track) = if operable {
                    (
                        DoorOp::Slide {
                            to_right: f64::from(i) < f64::from(n) / 2.0,
                        },
                        -TRACK,
                    )
                } else {
                    (DoorOp::Fixed, TRACK)
                };
                leaves.push(glass_panel(
                    [a, 0.0, b, top],
                    op,
                    track,
                    GLASS_STILE,
                    4.0 * MM_PER_IN,
                ));
            }
        }
        DoorFamily::Pocket => {
            if n == 2 {
                let m = w / 2.0;
                leaves.push(leaf(
                    [u0, 0.0, m, top],
                    DoorOp::Pocket { into_left: true },
                    style.leaf,
                    0.0,
                ));
                leaves.push(leaf(
                    [m, 0.0, u1, top],
                    DoorOp::Pocket { into_left: false },
                    style.leaf,
                    0.0,
                ));
            } else {
                leaves.push(leaf(
                    [u0, 0.0, u1, top],
                    DoorOp::Pocket {
                        into_left: hinge_left,
                    },
                    style.leaf,
                    0.0,
                ));
            }
        }
        DoorFamily::Barn => {
            // Leaves wider and taller than the opening, on the facing side's wall face.
            let lap = 2.0 * MM_PER_IN;
            let parts: Vec<(f64, f64, bool)> = if n == 2 {
                vec![(-lap, w / 2.0, false), (w / 2.0, w + lap, true)]
            } else {
                vec![(-lap, w + lap, hinge_left)]
            };
            for (a, b, right) in parts {
                leaves.push(leaf(
                    [a, 0.0, b, h + lap],
                    DoorOp::Barn { to_right: right },
                    style.leaf,
                    0.0,
                ));
            }
        }
        DoorFamily::Bifold => {
            // Pairs hinged at the jambs: 2 panels hinge on one side, 4 split two a side.
            let cs = cells(u0, u1, n);
            for (i, (a, b)) in cs.iter().enumerate() {
                let left = if n == 2 { hinge_left } else { i < 2 };
                leaves.push(leaf(
                    [*a, 0.0, *b, top],
                    DoorOp::Fold { hinge_left: left },
                    style.leaf,
                    0.0,
                ));
            }
        }
        DoorFamily::FoldingWall => {
            for (i, (a, b)) in cells(u0, u1, n).into_iter().enumerate() {
                let mut p = glass_panel(
                    [a, 0.0, b, top],
                    DoorOp::Fold { hinge_left },
                    0.0,
                    GLASS_STILE,
                    4.0 * MM_PER_IN,
                );
                // Alternate panels fold the other way.
                if i % 2 == 1 {
                    p.op = DoorOp::Fold {
                        hinge_left: !hinge_left,
                    };
                }
                leaves.push(p);
            }
        }
        DoorFamily::Storefront => {
            let bottom = 10.0 * MM_PER_IN;
            if n == 2 {
                let m = w / 2.0;
                leaves.push(glass_panel(
                    [u0, 0.0, m, top],
                    DoorOp::Swing { hinge_left: true },
                    0.0,
                    3.5 * MM_PER_IN,
                    bottom,
                ));
                leaves.push(glass_panel(
                    [m, 0.0, u1, top],
                    DoorOp::Swing { hinge_left: false },
                    0.0,
                    3.5 * MM_PER_IN,
                    bottom,
                ));
            } else {
                leaves.push(glass_panel(
                    [u0, 0.0, u1, top],
                    DoorOp::Swing { hinge_left },
                    0.0,
                    3.5 * MM_PER_IN,
                    bottom,
                ));
            }
        }
        DoorFamily::Garage => {
            // Four sections, each with a row of raised panels about 3' long.
            let mut l = Leaf {
                rect: [0.0, 0.0, w, h],
                op: DoorOp::Overhead,
                track: 0.0,
                glass: vec![],
                panels: vec![],
                bars: vec![],
                louvered: false,
            };
            let rows = 4;
            let sh = h / f64::from(rows);
            let cols = ((w / (36.0 * MM_PER_IN)).round() as u32).max(2);
            let pw = w / f64::from(cols);
            let inset = 3.0 * MM_PER_IN;
            for r in 0..rows {
                let z = sh * f64::from(r);
                if r > 0 {
                    l.bars.push([0.0, z, w, z]);
                }
                for c in 0..cols {
                    let u = pw * f64::from(c);
                    l.panels
                        .push([u + inset, z + inset, u + pw - inset, z + sh - inset]);
                }
            }
            leaves.push(l);
        }
    }
    DoorLayout {
        width: w,
        height: h,
        frame: f,
        casing,
        leaves,
        mullions,
    }
}

// ---------------------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------------------

/// A standard size, inches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    pub family: DoorFamily,
    pub leaf: LeafStyle,
    pub panels: u32,
    pub width: f64,
    pub height: f64,
    pub finish: Option<DoorFinish>,
}

const fn p(family: DoorFamily, leaf: LeafStyle, panels: u32, width: f64, height: f64) -> Preset {
    Preset {
        family,
        leaf,
        panels,
        width,
        height,
        finish: None,
    }
}

const fn pf(
    family: DoorFamily,
    leaf: LeafStyle,
    panels: u32,
    width: f64,
    height: f64,
    finish: DoorFinish,
) -> Preset {
    Preset {
        family,
        leaf,
        panels,
        width,
        height,
        finish: Some(finish),
    }
}

use DoorFamily as F;

/// Standard US door sizes (rough openings), 6'-8" and 7'-0" heights.
pub const CATALOG: &[Preset] = &[
    // Single swing: flush and panel passage doors, glass, commercial.
    p(F::SingleFlush, L::Flush, 0, 36.0, 84.0),
    p(F::SingleFlush, L::Flush, 0, 30.0, 80.0),
    p(F::SingleFlush, L::Flush, 0, 24.0, 80.0),
    p(F::SingleFlush, L::Flush, 0, 28.0, 80.0),
    p(F::SingleFlush, L::Flush, 0, 32.0, 80.0),
    p(F::SingleFlush, L::Flush, 0, 36.0, 80.0),
    p(F::SingleFlush, L::SixPanel, 0, 30.0, 80.0),
    p(F::SingleFlush, L::SixPanel, 0, 32.0, 80.0),
    p(F::SingleFlush, L::SixPanel, 0, 36.0, 80.0),
    p(F::SingleFlush, L::Shaker, 0, 28.0, 80.0),
    p(F::SingleFlush, L::Shaker, 0, 30.0, 80.0),
    p(F::SingleFlush, L::Shaker, 0, 32.0, 80.0),
    p(F::SingleFlush, L::FivePanel, 0, 30.0, 80.0),
    p(F::SingleFlush, L::FivePanel, 0, 32.0, 80.0),
    p(F::SingleFlush, L::Craftsman, 0, 36.0, 80.0),
    p(F::SingleFlush, L::FullLite, 0, 36.0, 80.0),
    p(F::SingleFlush, L::HalfLite, 0, 36.0, 80.0),
    p(F::SingleFlush, L::FifteenLite, 0, 30.0, 80.0),
    p(F::SingleFlush, L::FifteenLite, 0, 36.0, 80.0),
    pf(
        F::SingleFlush,
        L::VisionLite,
        0,
        36.0,
        84.0,
        DoorFinish::PaintedGray,
    ),
    // Pairs: French doors, double entries, commercial.
    p(F::DoubleFlush, L::Flush, 0, 72.0, 84.0),
    p(F::DoubleFlush, L::Flush, 0, 48.0, 80.0),
    p(F::DoubleFlush, L::Flush, 0, 60.0, 80.0),
    p(F::DoubleFlush, L::SixPanel, 0, 60.0, 80.0),
    p(F::DoubleFlush, L::FifteenLite, 0, 60.0, 80.0),
    p(F::DoubleFlush, L::FifteenLite, 0, 72.0, 80.0),
    p(F::DoubleFlush, L::FullLite, 0, 72.0, 80.0),
    pf(
        F::DoubleFlush,
        L::VisionLite,
        0,
        72.0,
        84.0,
        DoorFinish::PaintedGray,
    ),
    // Entries.
    p(F::Sidelites, L::SixPanel, 2, 64.0, 80.0),
    p(F::Sidelites, L::Craftsman, 2, 64.0, 80.0),
    p(F::Sidelites, L::Craftsman, 1, 50.0, 80.0),
    p(F::Sidelites, L::Shaker, 2, 72.0, 96.0),
    // Sliding glass patio doors.
    p(F::SlidingGlass, L::FullLite, 2, 60.0, 80.0),
    p(F::SlidingGlass, L::FullLite, 2, 72.0, 80.0),
    p(F::SlidingGlass, L::FullLite, 2, 96.0, 80.0),
    p(F::SlidingGlass, L::FullLite, 3, 108.0, 80.0),
    p(F::SlidingGlass, L::FullLite, 4, 144.0, 80.0),
    p(F::SlidingGlass, L::FullLite, 2, 72.0, 96.0),
    // Pocket.
    p(F::Pocket, L::Flush, 1, 30.0, 80.0),
    p(F::Pocket, L::Shaker, 1, 28.0, 80.0),
    p(F::Pocket, L::Shaker, 1, 30.0, 80.0),
    p(F::Pocket, L::Shaker, 1, 32.0, 80.0),
    p(F::Pocket, L::FifteenLite, 1, 30.0, 80.0),
    p(F::Pocket, L::Shaker, 2, 60.0, 80.0),
    // Barn.
    p(F::Barn, L::BarnX, 1, 36.0, 84.0),
    p(F::Barn, L::Shaker, 1, 36.0, 84.0),
    p(F::Barn, L::FivePanel, 1, 42.0, 84.0),
    p(F::Barn, L::BarnX, 2, 72.0, 84.0),
    // Bifold closets.
    p(F::Bifold, L::Flush, 2, 24.0, 80.0),
    p(F::Bifold, L::Louvered, 2, 30.0, 80.0),
    p(F::Bifold, L::Shaker, 2, 36.0, 80.0),
    p(F::Bifold, L::Louvered, 4, 48.0, 80.0),
    p(F::Bifold, L::Shaker, 4, 60.0, 80.0),
    p(F::Bifold, L::Flush, 4, 72.0, 80.0),
    // Folding glass walls.
    p(F::FoldingWall, L::FullLite, 4, 144.0, 96.0),
    p(F::FoldingWall, L::FullLite, 6, 192.0, 96.0),
    p(F::FoldingWall, L::FullLite, 8, 240.0, 96.0),
    // Storefront.
    p(F::Storefront, L::FullLite, 1, 36.0, 84.0),
    p(F::Storefront, L::FullLite, 2, 72.0, 84.0),
    p(F::Storefront, L::FullLite, 2, 72.0, 96.0),
    // Garages.
    p(F::Garage, L::Flush, 1, 96.0, 84.0),
    p(F::Garage, L::Flush, 1, 108.0, 84.0),
    p(F::Garage, L::Flush, 1, 108.0, 96.0),
    p(F::Garage, L::Flush, 1, 192.0, 84.0),
    p(F::Garage, L::Flush, 1, 192.0, 96.0),
];

/// What a new project starts with; the first three are the original built-in types.
pub const STARTER: &[(DoorFamily, LeafStyle, u32, f64, f64)] = &[
    (F::SingleFlush, L::Flush, 0, 36.0, 84.0),
    (F::SingleFlush, L::Flush, 0, 30.0, 80.0),
    (F::DoubleFlush, L::Flush, 0, 72.0, 84.0),
    (F::SingleFlush, L::SixPanel, 0, 32.0, 80.0),
    (F::SingleFlush, L::Shaker, 0, 30.0, 80.0),
    (F::SingleFlush, L::FullLite, 0, 36.0, 80.0),
    (F::DoubleFlush, L::FifteenLite, 0, 60.0, 80.0),
    (F::Sidelites, L::Craftsman, 2, 64.0, 80.0),
    (F::SlidingGlass, L::FullLite, 2, 72.0, 80.0),
    (F::Pocket, L::Shaker, 1, 30.0, 80.0),
    (F::Barn, L::BarnX, 1, 36.0, 84.0),
    (F::Bifold, L::Louvered, 4, 48.0, 80.0),
    (F::FoldingWall, L::FullLite, 4, 144.0, 96.0),
    (F::Storefront, L::FullLite, 1, 36.0, 84.0),
    (F::Storefront, L::FullLite, 2, 72.0, 84.0),
    (F::Garage, L::Flush, 1, 108.0, 84.0),
    (F::Garage, L::Flush, 1, 192.0, 84.0),
];

pub fn preset(family: DoorFamily, leaf: LeafStyle, panels: u32, w: f64, h: f64) -> Option<Preset> {
    CATALOG
        .iter()
        .find(|p| {
            p.family == family
                && p.leaf == leaf
                && (p.panels == panels || info(family).panels.is_none())
                && (p.width - w).abs() < 0.01
                && (p.height - h).abs() < 0.01
        })
        .copied()
}

fn inches(mm: f64) -> String {
    let i = mm / MM_PER_IN;
    if (i - i.round()).abs() < 0.01 {
        format!("{}\"", i.round())
    } else {
        format!("{i:.1}\"")
    }
}

/// A type's name: `Single Six-Panel 32" x 80"`, `Garage Sectional 16'-0" x 7'-0"`.
pub fn type_name(s: DoorStyle, width: f64, height: f64) -> String {
    let leaf = s.leaf.label();
    let base = match s.family {
        F::SingleFlush => format!("Single {leaf}"),
        F::DoubleFlush => format!("Double {leaf}"),
        F::Sidelites => format!(
            "Entry {leaf} w/ {} Sidelite{}",
            s.panels,
            if s.panels == 1 { "" } else { "s" }
        ),
        F::SlidingGlass => match s.panels {
            2 => "Sliding Glass".into(),
            n => format!("Sliding Glass {n}-Panel"),
        },
        F::Pocket => {
            if s.panels == 2 {
                format!("Double Pocket {leaf}")
            } else {
                format!("Pocket {leaf}")
            }
        }
        F::Barn => {
            if s.panels == 2 {
                format!("Double Barn {leaf}")
            } else {
                format!("Barn {leaf}")
            }
        }
        F::Bifold => format!("Bifold {}-Panel {leaf}", s.panels),
        F::FoldingWall => format!("Folding Glass Wall {}-Panel", s.panels),
        F::Storefront => {
            if s.panels == 2 {
                "Storefront Pair".into()
            } else {
                "Storefront Single".into()
            }
        }
        F::Garage => "Garage Sectional".into(),
    };
    let size = if s.family == F::Garage {
        format!("{} x {}", format_ft_in(width), format_ft_in(height))
    } else {
        format!("{} x {}", inches(width), inches(height))
    };
    let mut name = format!("{base} {size}");
    if s.finish != info(s.family).default_finish {
        name.push_str(" - ");
        name.push_str(s.finish.label());
    }
    name
}

/// A door type to load. Lengths mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DoorSpec {
    pub family: DoorFamily,
    pub leaf: LeafStyle,
    pub panels: u32,
    pub width: f64,
    pub height: f64,
    pub finish: Option<DoorFinish>,
}

impl DoorSpec {
    pub fn style(&self) -> DoorStyle {
        DoorStyle::resolve(self.family, self.leaf, self.panels, self.finish)
    }
    pub fn name(&self) -> String {
        type_name(self.style(), self.width, self.height)
    }
    pub fn data(&self) -> ElementData {
        let s = self.style();
        ElementData::DoorType {
            name: self.name(),
            family: s.family,
            width: self.width,
            height: self.height,
            leaf: s.leaf,
            panels: s.panels,
            finish: Some(s.finish),
        }
    }
    fn check(&self) -> CoreResult<()> {
        if !(self.width.is_finite() && self.height.is_finite()) {
            return Err(CoreError::Invalid("door size must be a number".into()));
        }
        let s = self.style();
        let min_w = match s.family {
            F::SlidingGlass | F::FoldingWall => 24.0 * MM_PER_IN * f64::from(s.panels),
            F::Garage => 72.0 * MM_PER_IN,
            F::Sidelites => 48.0 * MM_PER_IN,
            F::DoubleFlush => 36.0 * MM_PER_IN,
            _ => 18.0 * MM_PER_IN * f64::from(s.panels.max(1)),
        };
        if self.width < min_w || self.height < 72.0 * MM_PER_IN {
            return Err(CoreError::Invalid(format!(
                "a {} door must be at least {} wide and 6'-0\" tall",
                info(s.family).label.to_lowercase(),
                format_ft_in(min_w)
            )));
        }
        if self.width > 40.0 * 12.0 * MM_PER_IN || self.height > 20.0 * 12.0 * MM_PER_IN {
            return Err(CoreError::Invalid(
                "a door can be at most 40' wide and 20' tall".into(),
            ));
        }
        Ok(())
    }
}

impl From<Preset> for DoorSpec {
    fn from(p: Preset) -> Self {
        DoorSpec {
            family: p.family,
            leaf: p.leaf,
            panels: p.panels,
            width: p.width * MM_PER_IN,
            height: p.height * MM_PER_IN,
            finish: p.finish,
        }
    }
}

pub fn starter() -> Vec<DoorSpec> {
    STARTER
        .iter()
        .filter_map(|(f, l, n, w, h)| preset(*f, *l, *n, *w, *h).map(Into::into))
        .collect()
}

/// Loads door types (one undo step), reusing any already there by name.
pub fn load(doc: &mut Document, specs: &[DoorSpec]) -> CoreResult<Vec<ElementId>> {
    if specs.is_empty() {
        return Err(CoreError::Invalid("choose a door size to load".into()));
    }
    for s in specs {
        s.check()?;
    }
    let label = if specs.len() == 1 {
        format!("Load {}", specs[0].name())
    } else {
        format!("Load {} door types", specs.len())
    };
    doc.transact(&label, |tx| {
        let mut out = vec![];
        for s in specs {
            let name = s.name();
            let existing = tx
                .of(Category::DoorType)
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

/// The project's door type of `family` (and leaf, when given) nearest `width` mm.
pub fn nearest_type(
    doc: &Document,
    family: DoorFamily,
    leaf: Option<LeafStyle>,
    width: f64,
) -> Option<ElementId> {
    doc.of(Category::DoorType)
        .filter_map(|e| {
            let s = DoorStyle::of(&e.data)?;
            let w = match &e.data {
                ElementData::DoorType { width, .. } => *width,
                _ => return None,
            };
            (s.family == family && leaf.is_none_or(|l| l == s.leaf))
                .then_some((e.id, (w - width).abs()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IN: f64 = MM_PER_IN;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn old_types_keep_their_look_and_names() {
        let s = DoorStyle::new(F::SingleFlush);
        assert_eq!(
            (s.leaf, s.panels, s.finish),
            (L::Flush, 1, DoorFinish::PaintedWhite)
        );
        assert_eq!(
            type_name(s, 36.0 * IN, 84.0 * IN),
            "Single Flush 36\" x 84\""
        );
        assert_eq!(
            type_name(DoorStyle::new(F::DoubleFlush), 72.0 * IN, 84.0 * IN),
            "Double Flush 72\" x 84\""
        );
        let names: Vec<String> = starter().iter().map(DoorSpec::name).collect();
        assert_eq!(
            names[..3],
            [
                "Single Flush 36\" x 84\"",
                "Single Flush 30\" x 80\"",
                "Double Flush 72\" x 84\""
            ]
        );
        assert_eq!(names.len(), STARTER.len());
    }

    #[test]
    fn a_six_panel_leaf_has_six_panels_inside_its_rails() {
        let s = DoorStyle::resolve(F::SingleFlush, L::SixPanel, 0, None);
        let l = layout(s, 32.0 * IN, 80.0 * IN, true);
        assert_eq!(l.leaves.len(), 1);
        let leaf = &l.leaves[0];
        assert_eq!(leaf.panels.len(), 6);
        assert_eq!(leaf.op, DoorOp::Swing { hinge_left: true });
        // Inside the 1.5" jambs and 4.5" stiles.
        assert!(close(leaf.rect[0], 1.5 * IN) && close(leaf.panels[0][0], 6.0 * IN));
        assert!(close(leaf.panels[0][1], 9.0 * IN), "a 9\" bottom rail");
        assert!(leaf.glass.is_empty());
        // Casings on a swing door; none on glass sliders.
        assert!(close(l.casing, CASING));
        let sg = layout(DoorStyle::new(F::SlidingGlass), 72.0 * IN, 80.0 * IN, true);
        assert_eq!(sg.casing, 0.0);
    }

    #[test]
    fn glass_leaves_and_french_doors() {
        let french = layout(
            DoorStyle::resolve(F::DoubleFlush, L::FifteenLite, 0, None),
            60.0 * IN,
            80.0 * IN,
            true,
        );
        assert_eq!(french.leaves.len(), 2);
        for l in &french.leaves {
            assert_eq!(l.glass.len(), 1);
            // 3 x 5 lites: 2 vertical and 4 horizontal muntins.
            assert_eq!(l.bars.len(), 6);
        }
        assert_eq!(french.leaves[1].op, DoorOp::Swing { hinge_left: false });
        let c = layout(
            DoorStyle::resolve(F::SingleFlush, L::Craftsman, 0, None),
            36.0 * IN,
            80.0 * IN,
            true,
        );
        assert_eq!((c.leaves[0].glass.len(), c.leaves[0].panels.len()), (1, 2));
        // A vision lite sits on the latch side.
        let v = layout(
            DoorStyle::resolve(F::SingleFlush, L::VisionLite, 0, None),
            36.0 * IN,
            84.0 * IN,
            true,
        );
        let g = v.leaves[0].glass[0];
        assert!(g[0] > 18.0 * IN && close(g[2] - g[0], 5.0 * IN));
    }

    #[test]
    fn sliders_sidelites_bifolds_and_garages() {
        let s = layout(
            DoorStyle::resolve(F::SlidingGlass, L::FullLite, 4, None),
            144.0 * IN,
            80.0 * IN,
            true,
        );
        let ops: Vec<DoorOp> = s.leaves.iter().map(|l| l.op).collect();
        assert!(matches!(ops[0], DoorOp::Fixed) && matches!(ops[3], DoorOp::Fixed));
        assert!(matches!(ops[1], DoorOp::Slide { .. }) && matches!(ops[2], DoorOp::Slide { .. }));
        assert!(
            s.leaves[1].track < 0.0 && s.leaves[0].track > 0.0,
            "operable panels inside"
        );
        let e = layout(
            DoorStyle::resolve(F::Sidelites, L::Craftsman, 2, None),
            64.0 * IN,
            80.0 * IN,
            true,
        );
        // A centred 36" door, two sidelites, two mullions.
        assert_eq!((e.leaves.len(), e.mullions.len()), (3, 2));
        let d = e.leaves[0].rect;
        assert!(close(d[2] - d[0], 36.0 * IN));
        assert!(close((d[0] + d[2]) / 2.0, 32.0 * IN));
        let b = layout(
            DoorStyle::resolve(F::Bifold, L::Louvered, 4, None),
            48.0 * IN,
            80.0 * IN,
            true,
        );
        assert_eq!(b.leaves.len(), 4);
        assert!(b.leaves[0].louvered && b.leaves[0].bars.len() > 20);
        assert_eq!(b.leaves[3].op, DoorOp::Fold { hinge_left: false });
        let g = layout(DoorStyle::new(F::Garage), 192.0 * IN, 84.0 * IN, true);
        // 4 sections of 5 raised panels (16' / 3').
        assert_eq!(g.leaves[0].panels.len(), 20);
        assert_eq!(g.leaves[0].bars.len(), 3);
        // A barn door hangs past the opening on each side.
        let barn = layout(
            DoorStyle::resolve(F::Barn, L::BarnX, 1, None),
            36.0 * IN,
            84.0 * IN,
            true,
        );
        assert!(barn.leaves[0].rect[0] < 0.0 && barn.leaves[0].rect[2] > 36.0 * IN);
        assert_eq!(barn.leaves[0].bars.len(), 4, "an X in each half");
    }

    #[test]
    fn names_and_catalog() {
        let n =
            |f, l, p, w: f64, h: f64| type_name(DoorStyle::resolve(f, l, p, None), w * IN, h * IN);
        assert_eq!(
            n(F::SingleFlush, L::SixPanel, 0, 32.0, 80.0),
            "Single Six-Panel 32\" x 80\""
        );
        assert_eq!(
            n(F::DoubleFlush, L::FifteenLite, 0, 60.0, 80.0),
            "Double 15-Lite French 60\" x 80\""
        );
        assert_eq!(
            n(F::Sidelites, L::Craftsman, 2, 64.0, 80.0),
            "Entry Craftsman w/ 2 Sidelites 64\" x 80\""
        );
        assert_eq!(
            n(F::SlidingGlass, L::Flush, 4, 144.0, 80.0),
            "Sliding Glass 4-Panel 144\" x 80\""
        );
        assert_eq!(
            n(F::Garage, L::Flush, 0, 192.0, 84.0),
            "Garage Sectional 16'-0\" x 7'-0\""
        );
        assert_eq!(
            n(F::Storefront, L::Flush, 2, 72.0, 84.0),
            "Storefront Pair 72\" x 84\""
        );
        let black = DoorStyle::resolve(
            F::SingleFlush,
            L::FullLite,
            0,
            Some(DoorFinish::PaintedBlack),
        );
        assert_eq!(
            type_name(black, 36.0 * IN, 80.0 * IN),
            "Single Full Lite 36\" x 80\" - Painted Black"
        );
        let mut names: Vec<String> = CATALOG.iter().map(|p| DoorSpec::from(*p).name()).collect();
        let all = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), all, "unique names");
        assert!(all >= 60);
        for f in FAMILIES {
            assert!(CATALOG.iter().any(|p| p.family == f.family), "{}", f.label);
            assert_eq!(info(f.family).family, f.family);
        }
    }

    #[test]
    fn loading_checks_sizes_and_reuses_names() {
        let mut doc = Document::new();
        let spec: DoorSpec = preset(F::Pocket, L::Shaker, 1, 30.0, 80.0).unwrap().into();
        let a = load(&mut doc, &[spec]).unwrap();
        assert_eq!(load(&mut doc, &[spec]).unwrap(), a);
        let mut short = spec;
        short.height = 60.0 * IN;
        assert!(load(&mut doc, &[short]).is_err());
        let mut narrow: DoorSpec = preset(F::SlidingGlass, L::FullLite, 4, 144.0, 80.0)
            .unwrap()
            .into();
        narrow.width = 60.0 * IN;
        assert!(load(&mut doc, &[narrow]).is_err());
        assert_eq!(nearest_type(&doc, F::Pocket, None, 800.0), Some(a[0]));
        assert_eq!(nearest_type(&doc, F::Barn, None, 800.0), None);
    }
}
