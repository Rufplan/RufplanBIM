//! Door details (ADR-122): the head, jamb and sill of a door, and where a pair's leaves,
//! a slider's panels or a folding door's panels meet, for typical doors and for
//! manufacturers' doors, each redrawn from the dimensions its maker publishes and citing
//! the document. Drawn in inches like the rest of the library: sections with the exterior
//! to the left, plans with the exterior up. Inserted, each is a drafting view of lines,
//! regions, components and notes to edit like any other detail.
//!
//! A door is data (`Door`): the engine draws the frame, leaves and glass at its sizes in
//! the wall it is set in. The jamb is the head turned into plan, so the two always agree.

use super::FillPattern::{
    self, Concrete, Diagonal, Gray, Masonry, RigidInsulation, Sand, Solid, Wood,
};
use super::{DetailInfo, DetailSource, D};
use crate::lines::LineStyle::{self, Beyond, Hidden, Medium, Thin, Wide};

mod catalog;
mod window_catalog;
mod windows;

pub(super) const TYPICAL: &str = "Doors - Typical";
pub(super) const PATIO: &str = "Doors - Hinged Patio";
pub(super) const SLIDING: &str = "Doors - Sliding & Folding";
pub(super) const ENTRY: &str = "Doors - Entry";
pub(super) const COMMERCIAL: &str = "Doors - Commercial";
pub(super) const SPECIALTY: &str = "Doors - Interior & Specialty";

/// How a door opens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Op {
    /// Hinged, swinging in or out; a pair meets at an astragal.
    Swing { out: bool, pair: bool },
    /// Panels sliding on `tracks` tracks (lift-and-slide panels lift off their seals).
    Slide { tracks: u8, lift: bool },
    /// Folding panels hung from the head, folding out.
    Fold,
    /// A pivoting leaf on a floor pivot.
    Pivot,
    /// An aluminum entrance door in storefront framing.
    Storefront { pair: bool },
    /// A steel door in a pressed steel frame.
    HollowMetal,
    /// A leaf in a wood jamb in a partition.
    Interior,
    /// A leaf sliding into a pocket in the wall.
    Pocket,
    /// A leaf hung from a track on the face of the wall.
    Barn,
    /// A sectional overhead (garage) door.
    Overhead,
    /// A window (ADR-123).
    Window(Win),
}

/// How a window opens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Win {
    /// Hinged at a jamb (casement) or the head (awning), the sash swinging out.
    Casement,
    Awning,
    /// Glass in a fixed sash, or set straight into the frame when the sash is 0 thick.
    Fixed,
    /// Two sashes in two tracks: both sliding (double) or the upper fixed (single).
    Hung {
        double: bool,
    },
    /// Two sashes side by side, one sliding past the other.
    Slider,
    /// The European inswing sash that tilts in at the top and turns on its side.
    TiltTurn,
}

/// What the frame (and the leaf) is made of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Mat {
    Wood,
    /// Wood inside, extruded aluminum outside.
    CladWood,
    Fiberglass,
    /// Thermally broken extruded aluminum.
    Aluminum,
    /// Extruded aluminum without a thermal break.
    PlainAluminum,
    /// Thermally broken steel sections.
    Steel,
    /// Solid hot-rolled steel sections.
    SolidSteel,
    /// Multi-chambered extruded vinyl (PVC).
    Vinyl,
    /// A wood-fiber composite (Andersen's Fibrex).
    Composite,
}

/// The wall the door is set in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Wall {
    /// 2x6 studs, sheathing, WRB and lap siding.
    Siding,
    /// 2x6 studs, sheathing, WRB and three-coat stucco.
    Stucco,
    /// 2x6 studs, sheathing, WRB, 1" air space and brick veneer.
    Brick,
    /// 8" CMU, painted both sides.
    Cmu,
    /// 6" metal studs, sheathing, 1 1/2" rigid insulation and metal panel.
    MetalPanel,
    /// A 2x4 partition, gypsum both sides.
    Partition,
    /// A 3 5/8" metal stud partition, 5/8" gypsum both sides.
    MetalPartition,
}

/// What the door stands on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Floor {
    /// Subfloor on a rim joist, a deck outside.
    Framed,
    /// Slab on grade, paving outside.
    Slab,
    /// A slab recessed this deep for a flush sill, paving outside nearly level.
    Recessed(f64),
    /// Inside: the same floor each side.
    Inside,
}

/// How the frame is fastened at the rough opening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Mount {
    /// Through a nailing fin on the sheathing.
    Fin,
    /// Through a brickmould casing at the exterior face.
    Brickmould,
    /// Screwed through the frame (a block frame).
    Block,
}

/// The sill's material.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum SillMat {
    Aluminum,
    Composite,
    Oak,
    Stainless,
}

/// A door to draw. Lengths in inches.
#[derive(Debug, Clone, Copy)]
pub(super) struct Door {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    /// The manufacturer, "" for a typical door.
    pub maker: &'static str,
    pub product: &'static str,
    /// The published drawing, guide or standard the detail follows, and where it is.
    pub document: &'static str,
    pub url: &'static str,
    /// One sentence on the door, the start of each detail's description.
    pub summary: &'static str,
    pub op: Op,
    pub mat: Mat,
    pub wall: Wall,
    pub floor: Floor,
    pub mount: Mount,
    /// Overall frame depth.
    pub depth: f64,
    /// The frame head and jamb's thickness (its sightline beyond the leaf).
    pub jamb: f64,
    /// How far the frame stands out past the sheathing.
    pub proj: f64,
    /// Leaf (panel) thickness, stile width, top and bottom rail heights.
    pub leaf: f64,
    pub stile: f64,
    pub top_rail: f64,
    pub bottom_rail: f64,
    /// Insulating glass thickness; 0 for a solid leaf.
    pub glass: f64,
    /// The sill's top above the subfloor or slab, and its overall depth.
    pub sill_h: f64,
    pub sill_d: f64,
    pub sill_mat: SillMat,
    /// Rough opening: unit size plus this, overall.
    pub ro: f64,
    /// The notes that make it this door (upper case, as drafted).
    pub frame_note: &'static str,
    pub leaf_note: &'static str,
    pub glass_note: &'static str,
    pub sill_note: &'static str,
    pub seal_note: &'static str,
    pub head_note: &'static str,
    pub meet_note: &'static str,
    /// Sizes the source doesn't state, drawn at typical values (upper case), "" if none.
    pub est: &'static str,
    /// A hung window's meeting (check) rail height.
    pub meet_rail: f64,
    /// A window frame's sill thickness where it differs from the head and jambs (0: the same).
    pub frame_sill: f64,
}

/// A door with typical proportions, for the catalog to fill in.
pub(super) const BASE: Door = Door {
    id: "",
    name: "",
    category: TYPICAL,
    maker: "",
    product: "",
    document: "",
    url: "",
    summary: "",
    op: Op::Swing {
        out: false,
        pair: false,
    },
    mat: Mat::Wood,
    wall: Wall::Siding,
    floor: Floor::Framed,
    mount: Mount::Brickmould,
    depth: 6.5625,
    jamb: 1.25,
    proj: 1.0,
    leaf: 1.75,
    stile: 4.75,
    top_rail: 4.75,
    bottom_rail: 9.5,
    glass: 0.0,
    sill_h: 1.25,
    sill_d: 7.5,
    sill_mat: SillMat::Aluminum,
    ro: 0.5,
    frame_note: "",
    leaf_note: "",
    glass_note: "",
    sill_note: "",
    seal_note: "",
    head_note: "",
    meet_note: "",
    est: "",
    meet_rail: 1.5,
    frame_sill: 0.0,
};

/// A detail of a door.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Part {
    Head,
    Jamb,
    Sill,
    /// Where a pair's leaves, two sliding panels or two folding panels meet.
    Meet,
}

impl Door {
    fn parts(&self) -> Vec<Part> {
        let meets = match self.op {
            Op::Swing { pair, .. } | Op::Storefront { pair } => pair,
            Op::Slide { .. } | Op::Fold => true,
            Op::Window(Win::Hung { .. } | Win::Slider) => true,
            _ => false,
        };
        let mut parts = vec![Part::Head, Part::Jamb, Part::Sill];
        if meets {
            parts.push(Part::Meet);
        }
        parts
    }

    fn part_name(&self, part: Part) -> &'static str {
        match (part, self.op) {
            (Part::Head, _) => "Head",
            (Part::Jamb, _) => "Jamb",
            (Part::Sill, Op::Interior | Op::Pocket | Op::Barn) => "Floor",
            (Part::Sill, Op::Storefront { .. } | Op::HollowMetal) => "Threshold",
            (Part::Sill, _) => "Sill",
            (Part::Meet, Op::Window(Win::Hung { .. })) => "Meeting Rail",
            (Part::Meet, Op::Slide { .. } | Op::Window(Win::Slider)) => "Interlock",
            (Part::Meet, Op::Fold) => "Fold Joint",
            (Part::Meet, _) => "Meeting Stile",
        }
    }

    fn scale(&self, part: Part) -> u32 {
        match (part, self.op) {
            (Part::Head, Op::Overhead) => 8,
            _ => 4,
        }
    }

    /// How its frame and sashes lie, in door terms: casements, awnings and fixed sashes
    /// close against an interior stop like an outswing leaf, a tilt-turn against an
    /// exterior one like an inswing leaf, and hung and sliding sashes run in two tracks.
    fn shape(&self) -> Op {
        match self.op {
            Op::Window(Win::Casement | Win::Awning | Win::Fixed) => Op::Swing {
                out: true,
                pair: false,
            },
            Op::Window(Win::TiltTurn) => Op::Swing {
                out: false,
                pair: false,
            },
            Op::Window(Win::Hung { .. } | Win::Slider) => Op::Slide {
                tracks: 2,
                lift: false,
            },
            op => op,
        }
    }

    fn is_window(&self) -> bool {
        matches!(self.op, Op::Window(_))
    }
}

/// Every door detail, with its drawing.
pub(super) fn items() -> Vec<(DetailInfo, Box<dyn Fn() -> D + Send + Sync>)> {
    catalog::DOORS
        .iter()
        .chain(window_catalog::WINDOWS)
        .chain(window_catalog::METAL_WINDOWS)
        .chain(window_catalog::HUNG_WINDOWS)
        .chain(window_catalog::COMMERCIAL_WINDOWS)
        .chain(window_catalog::CASEMENT_WINDOWS)
        .chain(window_catalog::VINYL_WINDOWS)
        .flat_map(|door| {
            door.parts().into_iter().map(move |part| {
                let what = door.part_name(part);
                let plan = part == Part::Jamb
                    || (part == Part::Meet
                        && door.op != Op::Window(Win::Hung { double: true })
                        && door.op != Op::Window(Win::Hung { double: false }));
                let scale = door.scale(part);
                let info = DetailInfo {
                    id: format!("{}-{}", door.id, slug(what)),
                    name: format!("{} - {what}", door.name),
                    category: door.category.into(),
                    scale,
                    scale_label: crate::ops::scale_label(scale),
                    description: format!(
                        "{} {what} {}.",
                        door.summary,
                        if plan { "in plan" } else { "in section" }
                    ),
                    user: false,
                    source: (!door.maker.is_empty()).then(|| DetailSource {
                        manufacturer: door.maker.into(),
                        product: door.product.into(),
                        document: door.document.into(),
                        url: door.url.into(),
                    }),
                };
                let draw: Box<dyn Fn() -> D + Send + Sync> = Box::new(move || draw(door, part));
                (info, draw)
            })
        })
        .collect()
}

fn slug(s: &str) -> String {
    s.to_lowercase().replace(' ', "-")
}

/// Inches as drafted: 4-9/16", 1-3/4", 3/4", 6".
pub(super) fn fr(x: f64) -> String {
    let n = (x * 16.0).round() as i64;
    let (whole, mut num, mut den) = (n / 16, n % 16, 16);
    while num > 0 && num % 2 == 0 {
        num /= 2;
        den /= 2;
    }
    match (whole, num) {
        (w, 0) => format!("{w}\""),
        (0, n) => format!("{n}/{den}\""),
        (w, n) => format!("{w}-{n}/{den}\""),
    }
}

fn draw(door: &Door, part: Part) -> D {
    let mut d = D::new();
    let l = Lay::of(door);
    match (door.op, part) {
        (Op::Window(_), _) => windows::draw(&mut d, door, &l, part),
        (Op::HollowMetal, _) => hollow_metal(&mut d, door, &l, part),
        (Op::Storefront { .. }, _) => storefront(&mut d, door, &l, part),
        (Op::Interior | Op::Pocket | Op::Barn, _) => interior(&mut d, door, &l, part),
        (Op::Pivot, _) if door.wall == Wall::Partition => frameless_pivot(&mut d, door, &l, part),
        (Op::Overhead, _) => overhead(&mut d, door, &l, part),
        (_, Part::Head) => {
            let mut t = T::head(&mut d, &l);
            framed_head_or_jamb(&mut t, door, &l, false);
        }
        (_, Part::Jamb) => {
            let mut t = T::jamb(&mut d, &l);
            framed_head_or_jamb(&mut t, door, &l, true);
        }
        (_, Part::Sill) => framed_sill(&mut d, door, &l),
        (_, Part::Meet) => meet(&mut d, door, &l),
    }
    if part == Part::Jamb && !matches!(door.wall, Wall::Partition | Wall::MetalPartition) {
        d.caption("PLAN: EXTERIOR AT TOP.");
    }
    source(&mut d, door);
    d
}

/// The source under the drawing: the document it was redrawn from, and a caution.
fn source(d: &mut D, door: &Door) {
    if door.maker.is_empty() {
        if !door.document.is_empty() {
            d.caption(&format!("BASED ON: {}", door.document.to_uppercase()));
        }
        d.caption("TYPICAL DETAIL. ADJUST TO THE PRODUCT SPECIFIED.");
    } else {
        d.caption(&format!(
            "REDRAWN FROM {} PUBLISHED DETAILS: {}",
            door.maker.to_uppercase(),
            door.document.to_uppercase()
        ));
        d.caption(door.url);
        d.caption("VERIFY WITH THE MANUFACTURER'S CURRENT DETAILS AND INSTALLATION INSTRUCTIONS.");
    }
    if !door.est.is_empty() {
        d.caption(&format!(
            "NOT STATED IN THE SOURCE, DRAWN AT TYPICAL SIZES: {}",
            door.est
        ));
    }
}

// ------------------------------------------------------------------------------------------
// Layout
// ------------------------------------------------------------------------------------------

/// Where things are through the wall (section x, exterior -x): the exterior finish face,
/// the sheathing's outer face, the stud depth and the interior face; the frame's two faces;
/// each leaf or panel's two faces, exterior first.
#[derive(Clone)]
struct Lay {
    xo: f64,
    xs: f64,
    wd: f64,
    xi: f64,
    f0: f64,
    f1: f64,
    leaves: Vec<(f64, f64)>,
}

impl Lay {
    fn of(door: &Door) -> Lay {
        let (xo, xs, wd, xi) = match door.wall {
            Wall::Siding => (-1.25, -0.5, 5.5, 6.0),
            Wall::Stucco => (-1.375, -0.5, 5.5, 6.0),
            Wall::Brick => (-5.125, -0.5, 5.5, 6.0),
            Wall::Cmu => (0.0, 0.0, 7.625, 7.625),
            Wall::MetalPanel => (-2.75, -0.625, 6.0, 6.625),
            Wall::Partition => (-0.5, 0.0, 3.5, 4.0),
            Wall::MetalPartition => (-0.625, 0.0, 3.625, 4.25),
        };
        let f0 = match door.mount {
            Mount::Brickmould => xs,
            Mount::Fin | Mount::Block => xs - door.proj,
        };
        let f1 = f0 + door.depth;
        let t = door.leaf;
        let leaves = match door.shape() {
            Op::Swing { out: false, .. } | Op::Pivot => {
                let stop = ((door.depth - t) * 0.4).clamp(0.5, 1.5);
                vec![(f0 + stop + 0.125, f0 + stop + 0.125 + t)]
            }
            Op::Swing { out: true, .. } | Op::Fold => {
                let x0 = f0 + ((door.depth - t) * 0.2).clamp(0.25, 1.0);
                vec![(x0, x0 + t)]
            }
            Op::Slide { tracks, .. } => {
                let n = f64::from(tracks.max(1));
                let pitch = (door.depth - 0.75) / n;
                (0..tracks.max(1))
                    .map(|i| {
                        let c = f0 + 0.375 + pitch * (f64::from(i) + 0.5);
                        (c - t / 2.0, c + t / 2.0)
                    })
                    .collect()
            }
            _ => vec![(f0, f0 + t)],
        };
        Lay {
            xo,
            xs,
            wd,
            xi,
            f0,
            f1,
            leaves,
        }
    }
}

/// Draws in a head's frame (x through the wall, y up from the rough opening into the
/// head), as the head section itself or turned into the jamb's plan: the opening to +x,
/// the exterior up.
struct T<'a> {
    d: &'a mut D,
    plan: bool,
    xi: f64,
}

impl<'a> T<'a> {
    fn head(d: &'a mut D, l: &Lay) -> Self {
        T {
            d,
            plan: false,
            xi: l.xi,
        }
    }
    fn jamb(d: &'a mut D, l: &Lay) -> Self {
        T {
            d,
            plan: true,
            xi: l.xi,
        }
    }
    fn p(&self, (x, y): (f64, f64)) -> (f64, f64) {
        if self.plan {
            (-y, self.xi - x)
        } else {
            (x, y)
        }
    }
    fn ps(&self, pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
        pts.iter().map(|q| self.p(*q)).collect()
    }
    fn corners(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> (f64, f64, f64, f64) {
        let (a, b) = (self.p((x0, y0)), self.p((x1, y1)));
        (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
    }
    fn line(&mut self, pts: &[(f64, f64)], s: LineStyle) {
        let pts = self.ps(pts);
        self.d.line(&pts, s);
    }
    fn poly(&mut self, pts: &[(f64, f64)], s: LineStyle) {
        let pts = self.ps(pts);
        self.d.poly(&pts, s);
    }
    fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, s: LineStyle) {
        let (a, b, c, e) = self.corners(x0, y0, x1, y1);
        self.d.rect(a, b, c, e, s);
    }
    fn region(&mut self, pts: &[(f64, f64)], f: FillPattern) {
        let pts = self.ps(pts);
        self.d.region(&pts, f);
    }
    fn region_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, f: FillPattern) {
        let (a, b, c, e) = self.corners(x0, y0, x1, y1);
        self.d.region_rect(a, b, c, e, f);
    }
    fn cut(&mut self, pts: &[(f64, f64)], f: FillPattern) {
        let pts = self.ps(pts);
        self.d.cut(&pts, f);
    }
    fn cut_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, f: FillPattern) {
        let (a, b, c, e) = self.corners(x0, y0, x1, y1);
        self.d.cut_rect(a, b, c, e, f);
    }
    fn lumber(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let (a, b, c, e) = self.corners(x0, y0, x1, y1);
        self.d.lumber(a, b, c, e);
    }
    fn sheet(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let (a, b, c, e) = self.corners(x0, y0, x1, y1);
        self.d.sheet(a, b, c, e);
    }
    fn gyp(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let (a, b, c, e) = self.corners(x0, y0, x1, y1);
        self.d.gyp(a, b, c, e);
    }
    fn batt(&mut self, a: (f64, f64), b: (f64, f64), w: f64) {
        let (a, b) = (self.p(a), self.p(b));
        self.d.batt(a, b, w);
    }
    fn brk(&mut self, a: (f64, f64), b: (f64, f64)) {
        let (a, b) = (self.p(a), self.p(b));
        self.d.brk(a, b);
    }
    fn circle(&mut self, x: f64, y: f64, r: f64, s: LineStyle) {
        let (x, y) = self.p((x, y));
        self.d.circle(x, y, r, s);
    }
    fn flashing(&mut self, pts: &[(f64, f64)]) {
        let pts = self.ps(pts);
        self.d.flashing(&pts);
    }
    /// A component; the plan's turn is a mirror, so it flips.
    fn comp(&mut self, key: &'static str, a: (f64, f64), b: (f64, f64), flip: bool) {
        let (a, b) = (self.p(a), self.p(b));
        self.d.comp(key, a, b, flip ^ self.plan);
    }
    /// A sealant joint at `at`, its bead facing `face` (a unit direction).
    fn seal(&mut self, key: &'static str, at: (f64, f64), face: (f64, f64)) {
        // The component's v (its bead) is u turned left: u is `face` turned right.
        self.comp(key, at, (at.0 + face.1, at.1 - face.0), false);
    }
    fn note(&mut self, to: (f64, f64), text: &str) {
        let to = self.p(to);
        self.d.note(to, text);
    }
}

// ------------------------------------------------------------------------------------------
// Frames and leaves
// ------------------------------------------------------------------------------------------

fn mat_note(m: Mat) -> &'static str {
    match m {
        Mat::Wood => "WOOD",
        Mat::CladWood => "ALUMINUM-CLAD WOOD",
        Mat::Fiberglass => "FIBERGLASS",
        Mat::Aluminum => "THERMALLY BROKEN ALUMINUM",
        Mat::PlainAluminum => "ALUMINUM",
        Mat::Steel => "THERMALLY BROKEN STEEL",
        Mat::SolidSteel => "HOT-ROLLED STEEL",
        Mat::Vinyl => "VINYL",
        Mat::Composite => "COMPOSITE",
    }
}

/// A section through a member of `m`: wood and composites cut, extrusions hollow with a
/// thermal break across the middle.
fn member(t: &mut T, m: Mat, pts: &[(f64, f64)], x0: f64, y0: f64, x1: f64, y1: f64) {
    match m {
        Mat::Wood | Mat::CladWood => t.cut(pts, Wood),
        Mat::Fiberglass => t.cut(pts, Gray),
        Mat::SolidSteel => t.cut(pts, FillPattern::Steel),
        Mat::Composite => t.cut(pts, Gray),
        Mat::Vinyl => {
            t.poly(pts, Wide);
            let n = ((x1 - x0) / 0.75).floor().max(1.0) as usize;
            for i in 1..n {
                let x = x0 + (x1 - x0) * i as f64 / n as f64;
                t.line(&[(x, y0 + 0.08), (x, y1 - 0.08)], Thin);
            }
        }
        Mat::Aluminum | Mat::Steel | Mat::PlainAluminum => {
            t.poly(pts, Wide);
            let w = if m == Mat::Steel { 0.1 } else { 0.08 };
            let mid = (x0 + x1) / 2.0;
            let (ya, yb) = (y0 + w, y1 - w);
            if yb - ya > 0.2 && x1 - x0 > 1.0 {
                t.rect(x0 + w, ya, mid - 0.2, yb, Thin);
                t.rect(mid + 0.2, ya, x1 - w, yb, Thin);
                if m != Mat::PlainAluminum {
                    t.cut_rect(mid - 0.2, ya + 0.1, mid + 0.2, yb - 0.1, Solid);
                } else {
                    t.line(&[(mid - 0.2, ya), (mid - 0.2, yb)], Thin);
                }
            }
        }
    }
}

/// The frame's head (or jamb) member with its stops, from `yt` down: returns the y of its
/// face to the opening.
fn frame(t: &mut T, door: &Door, l: &Lay, yt: f64) -> f64 {
    let (f0, f1, j) = (l.f0, l.f1, door.jamb);
    let yb = yt - j;
    let lip = 0.625;
    let (x0, x1) = l.leaves[0];
    let pts: Vec<(f64, f64)> = match door.shape() {
        Op::Swing { out: false, .. } | Op::Pivot => {
            let s = x0 - 0.125;
            vec![
                (f0, yt),
                (f1, yt),
                (f1, yb),
                (s, yb),
                (s, yb - lip),
                (f0, yb - lip),
            ]
        }
        Op::Swing { out: true, .. } | Op::Fold => {
            let s = x1 + 0.125;
            vec![
                (f0, yt),
                (f1, yt),
                (f1, yb - lip),
                (s, yb - lip),
                (s, yb),
                (f0, yb),
            ]
        }
        _ => vec![(f0, yt), (f1, yt), (f1, yb), (f0, yb)],
    };
    member(t, door.mat, &pts, f0, yb, f1, yt);
    // A slider's guides between and outside its panels.
    if let Op::Slide { .. } = door.shape() {
        let mut xs = vec![f0];
        for w in l.leaves.windows(2) {
            xs.push((w[0].1 + w[1].0) / 2.0 - 0.09);
        }
        xs.push(f1 - 0.18);
        for x in xs {
            t.rect(x, yb - 0.75, x + 0.18, yb, Medium);
        }
    }
    if door.op == Op::Fold {
        // The hanging track and a carrier's wheel up in the panel's top.
        let c = (x0 + x1) / 2.0;
        t.rect(c - 0.6, yb - 0.9, c + 0.6, yb, Medium);
        t.circle(c, yb - 0.55, 0.3, Hidden);
    }
    if door.mat == Mat::CladWood {
        let low = pts
            .iter()
            .filter(|p| p.0 == f0)
            .map(|p| p.1)
            .fold(yb, f64::min);
        let end = match door.shape() {
            Op::Swing { out: false, .. } | Op::Pivot => x0 - 0.125,
            _ => f0 + 0.75,
        };
        t.line(
            &[(f0 - 0.06, yt), (f0 - 0.06, low - 0.06), (end, low - 0.06)],
            Wide,
        );
    }
    if door.mount == Mount::Fin {
        t.rect(l.xs - 0.0625, yt - 0.25, l.xs, yt + 1.5, Medium);
    }
    yb
}

/// A leaf or panel from its edge at `edge` toward `far` (dir +1 up from a sill, -1 down
/// from a head): its rail `rail` high, then glass with its stops, or the leaf's core.
#[allow(clippy::too_many_arguments)]
fn leaf(t: &mut T, door: &Door, x0: f64, x1: f64, edge: f64, dir: f64, rail: f64, far: f64) {
    if door.leaf <= 0.0 {
        return direct_glaze(t, door, (x0 + x1) / 2.0, edge, dir, far);
    }
    let r = edge + dir * rail;
    let (ya, yb) = (edge.min(r), edge.max(r));
    let rail_pts = [(x0, ya), (x1, ya), (x1, yb), (x0, yb)];
    member(t, door.mat, &rail_pts, x0, ya, x1, yb);
    if door.mat == Mat::CladWood {
        t.line(&[(x0 - 0.05, edge), (x0 - 0.05, r)], Wide);
    }
    let cx = (x0 + x1) / 2.0;
    if door.glass > 0.0 {
        let g = door.glass;
        t.line(&[(cx - g / 2.0, r), (cx - g / 2.0, far)], Medium);
        t.line(&[(cx + g / 2.0, r), (cx + g / 2.0, far)], Medium);
        let (sa, sb) = (r.min(r + dir * 0.35), r.max(r + dir * 0.35));
        t.region_rect(cx - g / 2.0, sa, cx + g / 2.0, sb, Gray);
        let w = ((x1 - x0 - g) / 2.0 * 0.8).clamp(0.12, 0.6);
        for (side, face) in [(-1.0, cx - g / 2.0), (1.0, cx + g / 2.0)] {
            t.poly(
                &[
                    (face, r),
                    (face + side * w, r),
                    (face + side * w, r + dir * 0.2),
                    (face, r + dir * 0.65),
                ],
                Medium,
            );
        }
    } else {
        t.line(&[(x0, r), (x0, far)], Medium);
        t.line(&[(x1, r), (x1, far)], Medium);
        match door.mat {
            Mat::Wood | Mat::CladWood => {
                // A raised panel, its tongue in the rail.
                let h = ((x1 - x0) * 0.32).min(0.5625);
                let (pa, pb) = (r - dir * 0.5, far);
                t.region(
                    &[(cx - h, pa), (cx + h, pa), (cx + h, pb), (cx - h, pb)],
                    Wood,
                );
                t.line(&[(cx - h, pa), (cx - h, pb)], Medium);
                t.line(&[(cx + h, pa), (cx + h, pb)], Medium);
                t.line(&[(x0, r), (x1, r)], Medium);
            }
            _ => {
                let (ca, cb) = (r.min(far), r.max(far));
                t.region_rect(x0 + 0.06, ca, x1 - 0.06, cb, RigidInsulation);
                t.line(&[(x0 + 0.06, r), (x0 + 0.06, far)], Thin);
                t.line(&[(x1 - 0.06, r), (x1 - 0.06, far)], Thin);
                t.line(&[(x0, r), (x1, r)], Medium);
            }
        }
    }
}

/// Glass set straight into a frame at `cx` (a direct-glazed or commercial fixed window):
/// the frame's integral leg outside, a snap-in stop inside, a gasket each side.
fn direct_glaze(t: &mut T, door: &Door, cx: f64, edge: f64, dir: f64, far: f64) {
    let g = door.glass;
    let (ga, gb) = (cx - g / 2.0, cx + g / 2.0);
    let r = edge + dir * 0.75;
    t.line(&[(ga, edge), (ga, far)], Medium);
    t.line(&[(gb, edge), (gb, far)], Medium);
    let (sa, sb) = (edge.min(edge + dir * 0.4), edge.max(edge + dir * 0.4));
    t.region_rect(ga, sa, gb, sb, Gray);
    let (ra, rb) = (edge.min(r), edge.max(r));
    t.poly(
        &[
            (ga - 0.6, ra),
            (ga - 0.125, ra),
            (ga - 0.125, rb),
            (ga - 0.6, rb),
        ],
        Wide,
    );
    t.poly(
        &[
            (gb + 0.125, ra),
            (gb + 0.6, ra),
            (gb + 0.6, rb),
            (gb + 0.125, rb),
        ],
        Medium,
    );
    t.region_rect(ga - 0.125, ra, ga, rb, Solid);
    t.region_rect(gb, ra, gb + 0.125, rb, Solid);
}

/// A head (or, turned, a jamb) of a framed door: wall, frame, leaves, trim and notes.
fn framed_head_or_jamb(t: &mut T, door: &Door, l: &Lay, jamb: bool) {
    let yt = -0.375;
    let yb = frame(t, door, l, yt);
    let far = -14.0;
    for &(x0, x1) in &l.leaves {
        let edge = match door.shape() {
            Op::Slide { .. } => yb - 0.125,
            Op::Fold => yb - 0.875,
            _ => yb - 0.125,
        };
        let rail = if jamb { door.stile } else { door.top_rail };
        leaf(t, door, x0, x1, edge, -1.0, rail, far);
    }
    // Shims and foam in the gap at the rough opening.
    t.region_rect(l.xs.max(l.f0), yt, l.f1.min(l.wd), 0.0, Sand);
    exterior_trim(t, door, l, yt, jamb);
    interior_trim(t, door, l, yb);
    if jamb {
        jamb_wall(t, door, l);
    } else {
        head_wall(t, door, l);
    }
    // Notes.
    let (x0, x1) = l.leaves[0];
    let what = if jamb { "JAMB" } else { "HEAD" };
    let mut frame_note = format!(
        "{} {} {what}, {} FRAME DEPTH",
        door.frame_note_or(),
        mat_note(door.mat),
        fr(door.depth)
    );
    if door.mount == Mount::Fin {
        frame_note.push_str(" W/ NAILING FIN");
    }
    t.note(((l.f0 + l.f1) / 2.0, yt - door.jamb / 2.0), &frame_note);
    let rail = if jamb { door.stile } else { door.top_rail };
    let leaf_name = match door.shape() {
        _ if door.is_window() => "SASH",
        Op::Slide { .. } | Op::Fold => "PANEL",
        _ => "LEAF",
    };
    let part = if jamb { "STILE" } else { "TOP RAIL" };
    if door.leaf > 0.0 {
        t.note(
            ((x0 + x1) / 2.0, yb - 0.6 - rail / 2.0),
            &format!(
                "{}{} {leaf_name}, {} {part}",
                leaf_label(door),
                fr(door.leaf),
                fr(rail)
            ),
        );
    } else {
        t.note(
            ((x0 + x1) / 2.0, yb - 0.4),
            "GLASS SET IN FRAME W/ GASKETS & SNAP-IN STOPS",
        );
    }
    if door.glass > 0.0 {
        let note = if door.glass_note.is_empty() {
            format!("{} INSULATING GLASS", fr(door.glass))
        } else {
            door.glass_note.to_owned()
        };
        t.note(((x0 + x1) / 2.0, -11.0), &note);
    }
    if !door.seal_note.is_empty() {
        t.note((x0, yb - 0.4), door.seal_note);
    }
    if !door.head_note.is_empty() && !jamb {
        t.note(((l.f0 + l.f1) / 2.0, yt + 0.1), door.head_note);
    }
    if l.leaves.len() > 1 {
        t.note(
            (l.leaves[1].0, yb - 2.0),
            &if door.is_window() {
                format!(
                    "SASHES IN {} TRACKS, {} DEEP",
                    l.leaves.len(),
                    fr(door.leaf)
                )
            } else {
                format!("{} TRACKS, PANELS {} DEEP", l.leaves.len(), fr(door.leaf))
            },
        );
    }
    t.note(
        ((l.xs.max(l.f0) + l.f1.min(l.wd)) / 2.0, yt / 2.0),
        &format!("SHIM & LOW-EXPANSION FOAM, R.O. = UNIT + {}", fr(door.ro)),
    );
    t.brk(
        (x0 - 1.0, far),
        (l.leaves.last().map_or(x0, |p| p.1) + 1.0, far),
    );
}

impl Door {
    fn frame_note_or(&self) -> String {
        if self.frame_note.is_empty() {
            if self.is_window() {
                "WINDOW FRAME"
            } else {
                "DOOR FRAME"
            }
            .into()
        } else {
            self.frame_note.to_owned()
        }
    }
}

fn leaf_label(door: &Door) -> String {
    if door.leaf_note.is_empty() {
        String::new()
    } else {
        format!("{}, ", door.leaf_note)
    }
}

/// Outside the frame: casing or brickmould, flashing, and the wall's finish to it.
fn exterior_trim(t: &mut T, door: &Door, l: &Lay, yt: f64, jamb: bool) {
    let (xs, xo) = (l.xs, l.xo);
    match door.mount {
        Mount::Brickmould => {
            // Brickmould 1 1/4" x 2" on the frame's exterior face.
            let pts = [
                (xs - 1.25, yt - door.jamb + 0.25),
                (xs, yt - door.jamb + 0.25),
                (xs, yt + 1.75),
                (xs - 0.75, yt + 1.75),
                (xs - 1.25, yt + 1.25),
            ];
            t.cut(&pts, Wood);
            t.note((xs - 0.6, yt + 0.5), "1-1/4\" x 2\" BRICKMOULD");
        }
        Mount::Fin | Mount::Block => {}
    }
    let top = match door.mount {
        Mount::Brickmould => yt + 1.75,
        _ => yt,
    };
    let out = match door.mount {
        Mount::Brickmould => xs - 1.25,
        _ => l.f0,
    };
    match door.wall {
        Wall::Siding => {
            // Trim board, then the drip cap with the WRB lapped over it (at the head).
            let tw = (xs - xo).max(0.75);
            t.lumber(xs - tw, top, xs, top + 3.5);
            if jamb {
                t.line(&[(xs - 0.05, top + 3.5), (xs - 0.05, 14.0)], Hidden);
                t.rect(xo, top + 3.6, xs, 14.0, Medium);
                t.seal("seal-14", (xs - tw - 0.15, top + 3.55), (-1.0, 0.0));
                t.note((xs - 0.3, top + 1.8), "5/4x4 EXTERIOR TRIM");
            } else {
                let y = top + 3.56;
                t.flashing(&[
                    (xs - 0.06, y + 2.0),
                    (xs - 0.06, y),
                    (xs - tw - 0.3, y - 0.12),
                    (xs - tw - 0.3, y - 0.6),
                ]);
                t.line(&[(xs - 0.12, 14.0), (xs - 0.12, y + 0.3)], Hidden);
                siding_up(t, xs, y + 0.3, 14.0);
                t.note((xs - tw * 0.5, y - 0.06), "METAL DRIP CAP, END DAMS");
                t.note((xs - 0.12, y + 2.5), "WRB LAPPED OVER DRIP CAP FLANGE");
                t.note((xs - 0.4, top + 1.8), "5/4x4 EXTERIOR TRIM");
            }
            t.seal("seal-14", (out - 0.05, top - 0.15), (-1.0, 0.0));
        }
        Wall::Stucco => {
            t.cut_rect(xo, top + 0.5, xs, 14.0, Sand);
            t.line(&[(xo, top + 0.5), (xs, top + 0.5), (xs, top + 1.5)], Wide);
            t.seal("seal-14", ((xo + xs) / 2.0, top + 0.3), (0.0, -1.0));
            if !jamb {
                t.flashing(&[
                    (xs - 0.06, top + 2.5),
                    (xs - 0.06, top + 0.06),
                    (out - 0.3, top - 0.05),
                ]);
                t.note((out, top), "HEAD FLASHING, WRB LAPPED OVER");
            }
            t.note((xo + 0.4, top + 4.0), "7/8\" 3-COAT STUCCO ON 2 LAYERS WRB");
            t.note((xs - 0.1, top + 1.0), "CASING BEAD");
            t.note(((xo + xs) / 2.0, top + 0.3), "SEALANT & BACKER ROD");
        }
        Wall::Brick => {
            // The veneer on a loose steel lintel at the head; returned at the jamb.
            let back = xo + 3.625;
            if jamb {
                t.cut_rect(xo, top + 0.375, back, 14.0, Masonry);
                t.seal("seal-12", ((xo + out) / 2.0, top + 0.2), (0.0, -1.0));
                t.note(((xo + back) / 2.0, top + 3.0), "BRICK VENEER");
            } else {
                let heel = (back + 0.375, top + 0.25);
                t.comp("l4x4", heel, (heel.0 - 1.0, heel.1), true);
                t.comp("brick-mod", (back, heel.1 + 0.375), (back, 14.0), false);
                t.flashing(&[
                    (xs - 0.06, heel.1 + 8.0),
                    (xs - 0.06, heel.1 + 0.5),
                    (back - 3.6, heel.1 + 0.4),
                    (xo - 0.4, heel.1 + 0.3),
                    (xo - 0.4, heel.1 - 0.1),
                ]);
                t.seal("seal-12", (out - 0.3, heel.1 - 0.15), (-1.0, 0.0));
                t.note(
                    (xo + 1.5, heel.1 + 0.15),
                    "L4x4x3/8 LOOSE LINTEL, 8\" BEARING",
                );
                t.note(
                    (xs - 0.06, heel.1 + 5.0),
                    "THRU-WALL FLASHING W/ END DAMS, WEEPS @ 33\" O.C. MAX.",
                );
                t.note(
                    ((xo + back) / 2.0, heel.1 + 5.0),
                    "BRICK VENEER, 1\" AIR SPACE",
                );
            }
            t.line(&[(xs - 0.06, top), (xs - 0.06, 14.0)], Hidden);
        }
        _ => {}
    }
}

/// Lap siding outside the sheathing face `x`, boards of 7" exposure from `y0` up.
fn siding_up(t: &mut T, x: f64, y0: f64, y1: f64) {
    let mut pts = vec![(x - 0.12, y0)];
    let mut y = y0;
    while y < y1 - 0.1 {
        let top = (y + 7.0).min(y1);
        pts.push((x - 0.75, y));
        pts.push((x - 0.3, top));
        pts.push((x - 0.12, top));
        y = top;
    }
    t.line(&pts, Medium);
}

/// Inside the frame: an extension jamb out to the wall's face where the frame is shallower,
/// and the casing.
fn interior_trim(t: &mut T, door: &Door, l: &Lay, yb: f64) {
    let face = yb;
    if l.f1 < l.xi - 0.05 {
        t.cut_rect(l.f1, face, l.xi, face + door.jamb.min(0.75), Wood);
        t.note(((l.f1 + l.xi) / 2.0, face + 0.3), "WOOD EXTENSION JAMB");
    }
    let x = l.xi.max(l.f1);
    t.lumber(x, face + 0.1875, x + 0.6875, face + 0.1875 + 3.5);
    t.note(
        (x + 0.35, face + 2.0),
        "11/16\" x 3-1/2\" CASING, 3/16\" REVEAL",
    );
}

/// The head's wall: a header and plates, sheathing, gypsum; the opening below it.
/// A steel stud wall's skins from `y0` to `y1` (head terms): gypsum sheathing, 1 1/2"
/// continuous insulation and metal panel outside, gypsum inside.
fn metal_panel_skin(t: &mut T, l: &Lay, y0: f64, y1: f64) {
    t.sheet(-0.625, y0, 0.0, y1);
    t.cut_rect(-2.125, y0, -0.625, y1, RigidInsulation);
    t.rect(l.xo, y0 + 0.5, -2.125, y1, Medium);
    t.gyp(l.wd, y0, l.xi, y1);
    t.note(
        ((l.xo - 2.125) / 2.0, y1 - 4.0),
        "METAL PANEL ON 1-1/2\" CI",
    );
    t.note(
        (-0.3, y1 - 6.0),
        "5/8\" EXT. GYP. SHEATHING & AIR/WATER BARRIER",
    );
}

fn head_wall(t: &mut T, door: &Door, l: &Lay) {
    let top = 14.0;
    match door.wall {
        Wall::Cmu => {
            t.cut_rect(0.0, 0.0, l.wd, 7.625, Concrete);
            t.comp("bar-5", (1.75, 1.5), (2.75, 1.5), false);
            t.comp("bar-5", (l.wd - 1.75, 1.5), (l.wd - 0.75, 1.5), false);
            t.comp("cmu-8", (l.wd, 7.625), (l.wd, top), false);
            t.note((l.wd / 2.0, 4.0), "CMU BOND BEAM LINTEL, (2) #5 CONT.");
        }
        Wall::MetalPanel => {
            t.flashing(&[(0.0, 1.25), (0.0, 0.0), (l.wd, 0.0), (l.wd, 1.25)]);
            t.comp("stud-6", (l.wd / 2.0, 1.25), (l.wd / 2.0, 2.25), false);
            t.comp("stud-6", (l.wd / 2.0, 4.5), (l.wd / 2.0, 3.5), false);
            metal_panel_skin(t, l, 0.75, top);
            t.flashing(&[(-0.6, 0.75), (l.xo - 0.6, 0.45), (l.xo - 0.6, 0.0)]);
            t.note((l.wd / 2.0, 0.0), "STEEL STUD BOX HEADER ON TRACK");
            t.note((l.xo - 0.3, 0.45), "HEAD FLASHING W/ DRIP, END DAMS");
        }
        _ => {
            t.lumber(0.0, 0.0, 1.5, 9.25);
            t.lumber(1.5, 0.0, 3.0, 9.25);
            t.cut_rect(3.0, 0.0, l.wd, 9.25, RigidInsulation);
            t.lumber(0.0, 9.25, l.wd, 10.75);
            t.lumber(0.0, 10.75, l.wd, 12.25);
            t.sheet(l.xs.min(-0.5), 0.0, 0.0, top);
            t.gyp(l.wd, 0.0, l.xi, top);
            t.note((1.5, 5.0), "(2) 2x10 HEADER W/ RIGID INSUL.");
            t.note((l.wd / 2.0, 11.0), "DOUBLE TOP PLATE");
        }
    }
    t.brk((l.xo - 2.0, top), (l.xi + 2.0, top));
}

/// The jamb's wall in plan (drawn in head terms: y runs into the wall): king and jack
/// studs, insulation, sheathing and gypsum.
fn jamb_wall(t: &mut T, door: &Door, l: &Lay) {
    let end = 14.0;
    match door.wall {
        Wall::Cmu => {
            t.cut_rect(0.0, 0.0, l.wd, end, Masonry);
            t.note((l.wd / 2.0, 5.0), "8\" CMU, GROUTED SOLID AT JAMB");
        }
        Wall::MetalPanel => {
            t.comp("stud-6", (l.wd / 2.0, 0.0), (l.wd / 2.0, 1.0), false);
            t.comp("stud-6", (l.wd / 2.0, 1.75), (l.wd / 2.0, 2.75), false);
            t.batt((l.wd / 2.0, 3.5), (l.wd / 2.0, end), l.wd);
            metal_panel_skin(t, l, 0.0, end);
            t.seal("seal-12", (l.xo - 0.25, 0.5), (-1.0, 0.0));
            t.note((l.wd / 2.0, 1.0), "DOUBLED STEEL STUDS AT JAMB");
        }
        _ => {
            t.lumber(0.0, 0.0, l.wd, 1.5);
            t.lumber(0.0, 1.5, l.wd, 3.0);
            t.batt((l.wd / 2.0, 3.0), (l.wd / 2.0, end), l.wd);
            t.sheet(l.xs.min(-0.5), 0.0, 0.0, end);
            t.gyp(l.wd, 0.0, l.xi, end);
            t.note((l.wd / 2.0, 0.75), "JACK STUD");
            t.note((l.wd / 2.0, 2.25), "KING STUD");
        }
    }
    t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
}

/// The sill of a framed door: floor, sill pan, sill, leaves' bottom rails, notes.
fn framed_sill(d: &mut D, door: &Door, l: &Lay) {
    let mut t = T::head(d, l);
    let floor = match door.floor {
        Floor::Recessed(r) => -r,
        _ => 0.0,
    };
    let ff = match door.floor {
        Floor::Framed => 0.75,
        _ => 0.5,
    };
    floor_under(&mut t, door, l, floor, ff);
    // The sill pan: back dam up inside, down over the face outside.
    let base = floor + 0.06;
    let back = l.f1 + 0.06;
    t.flashing(&[
        (back, floor + 1.0),
        (back, floor + 0.03),
        (l.xs - 0.03, floor + 0.03),
        (l.xs - 0.03, floor - 2.0),
    ]);
    // The sill's top from its nosing to the interior, by how the door works.
    let h = floor + door.sill_h;
    let depth = if door.sill_d > 0.0 {
        door.sill_d
    } else {
        l.f1 - l.f0 + 0.75
    };
    let xn = (l.f1 - depth).min(l.f0 - 0.5);
    let (x0, x1) = l.leaves[0];
    let mut top = vec![(xn, base + 0.45)];
    match door.op {
        Op::Swing { out: false, .. } | Op::Pivot => {
            top.extend([
                (x0 - 0.2, h - 0.35),
                (x0 - 0.2, h),
                (x0 + door.leaf * 0.7, h),
                (x0 + door.leaf * 0.7, h - 0.25),
                (l.f1, h - 0.25),
            ]);
        }
        Op::Swing { out: true, .. } | Op::Fold => {
            top.extend([
                (x1 + 0.125, h - 0.4),
                (x1 + 0.125, h),
                (x1 + 0.6, h),
                (x1 + 0.6, h - 0.2),
                (l.f1, h - 0.2),
            ]);
        }
        Op::Slide { .. } => {
            let n = l.leaves.len();
            for (i, &(a, b)) in l.leaves.iter().enumerate() {
                let s = base + 0.2 + (h - base - 0.6) * (i as f64 + 0.5) / n as f64;
                let c = (a + b) / 2.0;
                top.extend([
                    (a - 0.25, s),
                    (c - 0.125, s),
                    (c - 0.125, s + 0.375),
                    (c + 0.125, s + 0.375),
                    (c + 0.125, s),
                    (b + 0.25, s),
                ]);
            }
            top.extend([(l.f1 - 0.25, h), (l.f1, h)]);
        }
        _ => top.push((l.f1, h)),
    }
    let mut pts = vec![
        (l.f1, base),
        (xn + 0.15, base),
        (xn + 0.15, base - 0.4),
        (xn, base - 0.4),
    ];
    pts.extend(top.iter().copied());
    match door.sill_mat {
        SillMat::Aluminum | SillMat::Stainless => {
            t.poly(&pts, Wide);
            let m = (l.f0 + l.f1) / 2.0;
            t.cut_rect(m - 0.2, base + 0.08, m + 0.2, base + 0.3, Solid);
        }
        SillMat::Composite => t.cut(&pts, Gray),
        SillMat::Oak => t.cut(&pts, Wood),
    }
    // The leaves' bottoms, each a little above the sill under it.
    let top_at = |x: f64| -> f64 {
        top.windows(2)
            .filter(|w| x >= w[0].0.min(w[1].0) && x <= w[0].0.max(w[1].0))
            .map(|w| {
                if (w[1].0 - w[0].0).abs() < 1e-9 {
                    w[0].1.max(w[1].1)
                } else {
                    w[0].1 + (w[1].1 - w[0].1) * (x - w[0].0) / (w[1].0 - w[0].0)
                }
            })
            .fold(base, f64::max)
    };
    for &(a, b) in &l.leaves {
        let under = (a..=b).step_by_float(8).map(top_at).fold(base, f64::max);
        let edge = match door.op {
            Op::Slide { .. } => top_at((a + b) / 2.0) + 0.05,
            _ => under + 0.125,
        };
        leaf(&mut t, door, a, b, edge, 1.0, door.bottom_rail, 14.0);
        match door.op {
            Op::Slide { .. } => t.circle((a + b) / 2.0, edge + 0.55, 0.35, Hidden),
            _ => t.rect(a + 0.15, edge - 0.12, b - 0.15, edge, Thin),
        }
    }
    t.brk(
        (x0 - 1.0, 14.0),
        (l.leaves.last().map_or(x0, |p| p.1) + 1.0, 14.0),
    );
    // Notes.
    let sill = if door.sill_note.is_empty() {
        format!("{} SILL", sill_mat_note(door.sill_mat))
    } else {
        door.sill_note.to_owned()
    };
    t.note(
        ((xn + l.f1) / 2.0, base + 0.2),
        &if door.sill_d > 0.0 {
            format!(
                "{sill}, {} HIGH x {} DEEP",
                fr(door.sill_h),
                fr(door.sill_d)
            )
        } else {
            format!("{sill}, {} HIGH", fr(door.sill_h))
        },
    );
    t.note(
        (l.xs + 0.5, floor + 0.03),
        "SILL PAN FLASHING SLOPED TO EXTERIOR, END DAMS",
    );
    t.note((back, floor + 0.7), "BACK DAM, SILL SET IN SEALANT");
    let step = floor + door.sill_h - ff;
    if step > 0.0 {
        t.note(
            (l.f1 + 1.0, floor + ff),
            &format!("{} ABOVE FINISH FLOOR", fr(step)),
        );
    } else {
        t.note((l.f1 + 1.0, floor + ff), "SILL FLUSH WITH FINISH FLOOR");
    }
    t.note(
        ((x0 + x1) / 2.0, h + 1.0 + door.bottom_rail / 2.0),
        &format!("{}{} BOTTOM RAIL", leaf_label(door), fr(door.bottom_rail)),
    );
    match door.op {
        Op::Slide { lift: true, .. } => {
            t.note(((x0 + x1) / 2.0, h + 0.6), "LIFT-AND-SLIDE ROLLERS & SEALS")
        }
        Op::Slide { .. } => t.note(((x0 + x1) / 2.0, h + 0.6), "TANDEM ROLLERS ON TRACK"),
        _ => t.note((x0 + 0.3, h + 0.05), "DOOR BOTTOM SWEEP"),
    }
}

trait StepFloat {
    fn step_by_float(self, n: usize) -> Box<dyn Iterator<Item = f64>>;
}

impl StepFloat for std::ops::RangeInclusive<f64> {
    fn step_by_float(self, n: usize) -> Box<dyn Iterator<Item = f64>> {
        let (a, b) = (*self.start(), *self.end());
        Box::new((0..=n).map(move |i| a + (b - a) * i as f64 / n as f64))
    }
}

fn sill_mat_note(m: SillMat) -> &'static str {
    match m {
        SillMat::Aluminum => "EXTRUDED ALUMINUM",
        SillMat::Composite => "COMPOSITE",
        SillMat::Oak => "OAK",
        SillMat::Stainless => "STAINLESS STEEL",
    }
}

/// The floor under a sill (y 0 its top, or `floor` where recessed), the floor finish
/// `ff` thick inside, and what's outside.
fn floor_under(t: &mut T, door: &Door, l: &Lay, floor: f64, ff: f64) {
    let inside = l.f1 + 0.125;
    match door.floor {
        Floor::Framed => {
            t.sheet(0.0, -0.75, 16.0, 0.0);
            t.lumber(0.0, -10.0, 1.5, -0.75);
            t.batt((4.0, -0.75), (4.0, -10.0), 3.5);
            t.sheet(-0.5, -12.0, 0.0, 0.0);
            t.line(&[(-0.53, -2.0), (-0.53, -12.0)], Hidden);
            siding_up(t, -0.5, -12.0, -2.5);
            t.cut_rect(inside, 0.0, 16.0, ff, Wood);
            // Deck boards outside, 2" below.
            for i in 0..2 {
                let x = l.xo - 0.75 - f64::from(i) * 5.75;
                t.cut_rect(x - 5.5, -3.0, x, -2.0, Wood);
            }
            t.line(&[(l.xo - 12.5, -3.0), (l.xo - 0.75, -3.0)], Beyond);
            t.note((4.0, -0.4), "3/4\" SUBFLOOR");
            t.note((0.75, -6.0), "RIM JOIST, SPRAY FOAM SEALED");
            t.note((l.xo - 3.5, -2.5), "DECK, 2\" MIN. BELOW SILL");
            t.brk((-3.0, -12.0), (16.0, -12.0));
        }
        Floor::Slab | Floor::Inside => {
            t.cut(
                &[(l.xs, 0.0), (16.0, 0.0), (16.0, -6.0), (l.xs, -6.0)],
                Concrete,
            );
            t.cut_rect(inside, 0.0, 16.0, ff, Diagonal);
            if door.floor == Floor::Slab {
                t.cut(
                    &[
                        (l.xo - 14.0, -1.25),
                        (l.xs - 0.5, -1.0),
                        (l.xs - 0.5, -5.0),
                        (l.xo - 14.0, -5.0),
                    ],
                    Concrete,
                );
                t.region_rect(l.xs - 0.5, -5.0, l.xs, -1.0, Sand);
                t.note(
                    (l.xo - 6.0, -2.5),
                    "CONC. LANDING, 1-1/2\" MAX. BELOW SILL, SLOPE 1/4\":12\"",
                );
                t.note((l.xs - 0.25, -3.0), "1/2\" EXP. JOINT W/ SEALANT");
            }
            t.note((8.0, -3.0), "CONC. SLAB");
            t.note((10.0, ff / 2.0), "FLOOR FINISH");
            t.brk((16.0, 1.0), (16.0, -6.0));
        }
        Floor::Recessed(r) => {
            let (a, b) = (l.f0 - 0.5, l.f1 + 0.125);
            t.cut(
                &[
                    (l.xs, 0.0),
                    (a.max(l.xs), 0.0),
                    (a.max(l.xs), floor),
                    (b, floor),
                    (b, 0.0),
                    (16.0, 0.0),
                    (16.0, -8.0),
                    (l.xs, -8.0),
                ],
                Concrete,
            );
            t.cut_rect(inside, 0.0, 16.0, ff, Diagonal);
            // Paving outside just below the sill, over a drain.
            let pave = ff - 0.25;
            t.cut_rect(l.xo - 14.0, pave - 2.0, l.xs - 0.75, pave, Concrete);
            t.rect(l.xs - 3.75, pave - 4.0, l.xs - 0.75, pave - 2.0, Medium);
            t.note(
                ((a + b) / 2.0, floor - 0.3),
                &format!("SLAB RECESSED {} FOR SILL", fr(r)),
            );
            t.note(
                (l.xs - 2.25, pave - 3.0),
                "TRENCH DRAIN AT SILL WHERE REQ'D BY MFR.",
            );
            t.note(
                (l.xo - 8.0, pave - 1.0),
                "PAVERS ON PEDESTALS, 1/4\" BELOW FLOOR",
            );
            t.note((10.0, ff / 2.0), "FLOOR FINISH");
            t.brk((16.0, 1.0), (16.0, -8.0));
        }
    }
}

/// Where two leaves or panels meet, in plan (exterior up): an astragal between a pair, an
/// interlock between sliding panels, a hinge between folding ones.
fn meet(d: &mut D, door: &Door, l: &Lay) {
    let py = |x: f64| l.xi - x;
    let span = 14.0;
    let g = door.glass;
    // A leaf in plan from its edge at `e` toward `to`, its faces at section x0..x1.
    let leaf_plan = |d: &mut D, x0: f64, x1: f64, e: f64, to: f64| {
        let (ya, yb) = (py(x1), py(x0));
        let s = e + (to - e).signum() * door.stile;
        let (sa, sb) = (e.min(s), e.max(s));
        let pts = [(sa, ya), (sb, ya), (sb, yb), (sa, yb)];
        let mut t = T {
            d,
            plan: false,
            xi: l.xi,
        };
        member(&mut t, door.mat, &pts, sa, ya, sb, yb);
        if door.mat == Mat::CladWood {
            d.line(&[(sa, yb + 0.05), (sb, yb + 0.05)], Wide);
        }
        if g > 0.0 {
            let c = (ya + yb) / 2.0;
            d.line(&[(s, c - g / 2.0), (to, c - g / 2.0)], Medium);
            d.line(&[(s, c + g / 2.0), (to, c + g / 2.0)], Medium);
            let k = (to - e).signum() * 0.35;
            d.region_rect(s.min(s + k), c - g / 2.0, s.max(s + k), c + g / 2.0, Gray);
        } else {
            d.line(&[(s, ya), (to, ya)], Medium);
            d.line(&[(s, yb), (to, yb)], Medium);
        }
    };
    match door.shape() {
        Op::Slide { .. } => {
            let (a, b) = (l.leaves[0], l.leaves[1.min(l.leaves.len() - 1)]);
            leaf_plan(d, a.0, a.1, 0.75, -span);
            leaf_plan(d, b.0, b.1, -0.75, span);
            // Interlock hooks with their seals.
            let mid = (py(a.0) + py(b.1)) / 2.0;
            let gap = (a.0 - b.1).abs();
            d.line(
                &[(0.75, py(a.0)), (0.75, mid + 0.12), (-0.25, mid + 0.12)],
                Wide,
            );
            d.line(
                &[(-0.75, py(b.1)), (-0.75, mid - 0.12), (0.25, mid - 0.12)],
                Wide,
            );
            d.rect(-0.25, mid - 0.06, 0.25, mid + 0.06, Thin);
            d.brk((-span, py(a.1) - 1.0), (-span, py(a.0) + 1.0));
            d.brk((span, py(b.1) - 1.0), (span, py(b.0) + 1.0));
            let mn = if door.meet_note.is_empty() {
                format!("INTERLOCKING STILES, {} BETWEEN TRACKS", fr(gap))
            } else {
                door.meet_note.to_owned()
            };
            d.note((0.0, mid), &mn);
            d.note((-span / 2.0, py(a.0)), "EXTERIOR PANEL");
            d.note((span / 2.0, py(b.0)), "INTERIOR PANEL");
        }
        Op::Fold => {
            let (x0, x1) = l.leaves[0];
            leaf_plan(d, x0, x1, -0.1875, -span);
            leaf_plan(d, x0, x1, 0.1875, span);
            // The hinge on the outside face, a seal between.
            d.circle(0.0, py(x0) + 0.25, 0.25, Medium);
            d.rect(-0.1875, py(x1) + 0.2, 0.1875, py(x0) - 0.2, Thin);
            d.brk((-span, py(x1) - 1.0), (-span, py(x0) + 1.0));
            d.brk((span, py(x1) - 1.0), (span, py(x0) + 1.0));
            let mn = if door.meet_note.is_empty() {
                "PANEL HINGE, GASKET BETWEEN STILES".to_owned()
            } else {
                door.meet_note.to_owned()
            };
            d.note((0.0, py(x0) + 0.25), &mn);
            d.note((-span / 2.0, py((x0 + x1) / 2.0)), "FOLDING PANEL");
        }
        _ => {
            // A pair: the active leaf closes against an astragal on the inactive one.
            let (x0, x1) = l.leaves[0];
            leaf_plan(d, x0, x1, -0.0625, -span);
            leaf_plan(d, x0, x1, 0.0625, span);
            // Inswing: on the inactive leaf's exterior face, which the active leaf closes
            // against; outswing: on its interior face.
            let out = matches!(door.op, Op::Swing { out: true, .. });
            let face = if out { py(x1) } else { py(x0) };
            let k = if out { -1.0 } else { 1.0 };
            let pts = [
                (-1.0, face),
                (1.0, face),
                (1.0, face + k * 0.375),
                (-1.0, face + k * 0.375),
            ];
            let mut t = T {
                d,
                plan: false,
                xi: l.xi,
            };
            member(
                &mut t,
                door.mat,
                &pts,
                -1.0,
                face.min(face + k * 0.375),
                1.0,
                face.max(face + k * 0.375),
            );
            d.rect(-0.0625, py(x1) + 0.3, 0.0625, py(x0) - 0.3, Thin);
            d.brk((-span, py(x1) - 1.0), (-span, py(x0) + 1.0));
            d.brk((span, py(x1) - 1.0), (span, py(x0) + 1.0));
            let mn = if door.meet_note.is_empty() {
                "ASTRAGAL W/ WEATHERSTRIP, MULTI-POINT LOCK".to_owned()
            } else {
                door.meet_note.to_owned()
            };
            d.note((0.5, face + k * 0.2), &mn);
            d.note((-span / 2.0, py((x0 + x1) / 2.0)), "ACTIVE LEAF");
            d.note(
                (span / 2.0, py((x0 + x1) / 2.0)),
                "INACTIVE LEAF, FLUSH BOLTS",
            );
        }
    }
    if g > 0.0 {
        let (x0, x1) = l.leaves[0];
        let note = if door.glass_note.is_empty() {
            format!("{} INSULATING GLASS", fr(g))
        } else {
            door.glass_note.to_owned()
        };
        d.note((-span + 2.0, py((x0 + x1) / 2.0)), &note);
    }
    let x1 = l.leaves[0].1;
    d.note(
        (span - 2.0 - door.stile.min(6.0), py(x1)),
        &format!("{}{} STILES", leaf_label(door), fr(door.stile)),
    );
    d.caption("PLAN: EXTERIOR AT TOP.");
}

// ------------------------------------------------------------------------------------------
// Commercial: hollow metal and storefront
// ------------------------------------------------------------------------------------------

/// A steel door in a pressed steel frame wrapping the wall (SDI-111): faces 2" (4" head
/// optional), backbends standing 7/16"-1/2" off the wall, a 5/8" stop between rabbets.
fn hollow_metal(d: &mut D, door: &Door, l: &Lay, part: Part) {
    let bb = if door.wall == Wall::Cmu { 0.5 } else { 0.4375 };
    let (fa, fb) = (l.xo - bb, l.xi + bb);
    let face = if part == Part::Head && door.jamb > 2.0 {
        door.jamb
    } else {
        2.0
    };
    let r1 = 1.9375;
    let stop = 0.625;
    let xr1 = fa + r1;
    let xr2 = (fb - r1).max(xr1 + 0.5);
    if part == Part::Sill {
        hm_threshold(d, door, l, fa, xr1);
        return;
    }
    let mut t = if part == Part::Jamb {
        T::jamb(d, l)
    } else {
        T::head(d, l)
    };
    // The frame's profile, its sheet's centre line.
    t.flashing(&[
        (l.xo, face - 0.375),
        (l.xo, face),
        (fa, face),
        (fa, 0.0),
        (xr1, 0.0),
        (xr1, -stop),
        (xr2, -stop),
        (xr2, 0.0),
        (fb, 0.0),
        (fb, face),
        (l.xi, face),
        (l.xi, face - 0.375),
    ]);
    // The door: 1 3/4" against the stop, 1/8" clear of the soffit.
    let (x0, x1) = (xr1 - 0.0625 - door.leaf, xr1 - 0.0625);
    t.poly(
        &[(x0, -0.125), (x1, -0.125), (x1, -14.0), (x0, -14.0)],
        Medium,
    );
    let edge = if part == Part::Jamb {
        door.stile
    } else {
        door.top_rail
    };
    t.line(
        &[(x0 + 0.05, -0.125 - edge), (x1 - 0.05, -0.125 - edge)],
        Thin,
    );
    t.region_rect(x0 + 0.05, -14.0, x1 - 0.05, -0.125 - edge, RigidInsulation);
    t.circle(xr1 + 0.06, -0.35, 0.08, Thin);
    // The wall in the throat.
    let (end, inner_top) = (14.0, face + 0.5);
    match door.wall {
        Wall::Cmu => {
            t.region(
                &[(l.xo, 0.1), (l.xi, 0.1), (l.xi, face), (l.xo, face)],
                Concrete,
            );
            if part == Part::Jamb {
                t.cut_rect(l.xo, face, l.xi, end, Masonry);
                t.flashing(&[
                    (l.xo + 1.0, 0.6),
                    (l.xo + 1.0, face + 2.5),
                    (l.xi - 1.0, face + 2.5),
                ]);
                t.note(
                    (l.xo + 1.0, face + 2.0),
                    "MASONRY T-ANCHOR, 3 PER JAMB MIN.",
                );
                t.note((l.wd / 2.0, 8.0), "8\" CMU");
            } else {
                t.cut_rect(l.xo, face, l.xi, face + 7.625, Concrete);
                t.comp("bar-5", (1.75, face + 1.5), (2.75, face + 1.5), false);
                t.comp(
                    "bar-5",
                    (l.wd - 1.75, face + 1.5),
                    (l.wd - 0.75, face + 1.5),
                    false,
                );
                t.comp("cmu-8", (l.xi, face + 7.625), (l.xi, end), false);
                t.note((l.wd / 2.0, face + 5.0), "CMU BOND BEAM LINTEL, (2) #5");
            }
            t.note((l.wd / 2.0, face * 0.6), "FRAME GROUTED SOLID");
        }
        _ => {
            let stud = if l.wd > 4.0 { "stud-6" } else { "stud-358" };
            let gyp = 0.625;
            t.gyp(l.xo, 0.25, l.xo + gyp, end);
            t.gyp(l.xi - gyp, 0.25, l.xi, end);
            if door.wall == Wall::MetalPanel {
                t.cut_rect(l.xo - 1.5, inner_top, l.xo, end, RigidInsulation);
            }
            let mid = (l.xo + gyp + l.xi - gyp) / 2.0;
            t.comp(stud, (mid, 0.3), (mid, 1.3), false);
            if part == Part::Jamb {
                t.comp(stud, (mid, 2.0), (mid, 3.0), false);
                t.batt((mid, 3.8), (mid, end), l.wd);
                t.line(
                    &[(mid - 0.6, 0.3), (mid - 0.6, 1.2), (mid + 0.6, 1.2)],
                    Wide,
                );
                t.note((mid, 1.0), "STEEL STUD ANCHOR, 3 PER JAMB, DOUBLED STUD");
            } else {
                t.comp(stud, (mid, 2.0), (mid, 3.0), false);
                t.note((mid, 1.0), "STUD HEADER, TRACK & CRIPPLES ABOVE");
            }
            t.note(
                (l.xi - gyp / 2.0, 6.0),
                "5/8\" TYPE X GYP. BD. INTO FRAME THROAT",
            );
        }
    }
    t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
    t.brk((x0 - 1.0, -14.0), (x1 + 1.0, -14.0));
    let what = if part == Part::Jamb { "JAMB" } else { "HEAD" };
    t.note(
        (fa, face * 0.5),
        &format!(
            "{} {what}, {} FACE, {} STOP, {} RABBET",
            door.frame_note_or(),
            fr(face),
            fr(stop),
            fr(r1)
        ),
    );
    t.note((fb, face * 0.5), &format!("{} JAMB DEPTH", fr(fb - fa)));
    t.note(
        ((x0 + x1) / 2.0, -6.0),
        &format!("{}{} STEEL DOOR", leaf_label(door), fr(door.leaf)),
    );
    t.note((xr1 + 0.06, -0.35), "RUBBER SILENCERS, 3 PER STRIKE JAMB");
    if !door.seal_note.is_empty() {
        t.note((xr1 - 0.3, -0.5), door.seal_note);
    }
    if part == Part::Head && !door.head_note.is_empty() {
        t.note(((xr1 + xr2) / 2.0, -0.4), door.head_note);
    }
}

/// A steel door's threshold on a slab: a saddle 1/2" high set in sealant, the frame's
/// jambs beyond, and the door's bottom over it.
fn hm_threshold(d: &mut D, door: &Door, l: &Lay, fa: f64, xr1: f64) {
    let w = door.sill_d;
    let h = door.sill_h;
    let c = xr1 - door.leaf / 2.0;
    d.cut(
        &[
            (l.xo - 14.0, 0.0),
            (l.xi + 14.0, 0.0),
            (l.xi + 14.0, -5.0),
            (l.xo - 14.0, -5.0),
        ],
        Concrete,
    );
    let (a, b) = (c - w / 2.0, c + w / 2.0);
    if h > 0.0 {
        d.poly(
            &[(a, 0.0), (a + h * 2.0, h), (b - h * 2.0, h), (b, 0.0)],
            Wide,
        );
        d.region_rect(a, -0.06, b, 0.0, Solid);
    }
    let (x0, x1) = (xr1 - 0.0625 - door.leaf, xr1 - 0.0625);
    let under = if h > 0.0 { h + 0.125 } else { 0.75 };
    d.rect(x0, under, x1, 14.0, Medium);
    d.line(
        &[
            (x0 + 0.05, under + door.bottom_rail),
            (x1 - 0.05, under + door.bottom_rail),
        ],
        Thin,
    );
    d.region_rect(
        x0 + 0.05,
        under + door.bottom_rail,
        x1 - 0.05,
        14.0,
        RigidInsulation,
    );
    d.line(&[(fa, h.max(0.0)), (fa, 14.0)], Beyond);
    d.line(&[(l.xi + 0.4375, 0.0), (l.xi + 0.4375, 14.0)], Beyond);
    if door.wall != Wall::MetalPartition && door.wall != Wall::Partition {
        d.rect(x0 + 0.1, under - 0.125, x1 - 0.1, under, Thin);
        d.note((x0 + 0.2, under - 0.06), "DOOR SWEEP");
    }
    d.brk((x0 - 1.0, 14.0), (x1 + 1.0, 14.0));
    d.brk((l.xo - 14.0, -5.0), (l.xi + 14.0, -5.0));
    let sill = if door.sill_note.is_empty() {
        "ALUMINUM SADDLE THRESHOLD"
    } else {
        door.sill_note
    };
    if h > 0.0 {
        d.note(
            (c, h * 0.6),
            &format!("{sill}, {} x {}, SET IN SEALANT, 1:2 BEVELS", fr(w), fr(h)),
        );
    } else {
        d.note((c, 0.0), "NO THRESHOLD, FLOOR CONTINUOUS");
    }
    d.note(
        ((x0 + x1) / 2.0, under + 4.0),
        &format!("{} STEEL DOOR", fr(door.leaf)),
    );
    d.note(
        (x1, under - 0.06),
        &format!(
            "{} CLEAR TO THRESHOLD (3/4\" MAX., NFPA 80)",
            fr(under - h.max(0.0))
        ),
    );
    d.note((fa, 10.0), "FRAME JAMB BEYOND");
    d.note((l.xi + 8.0, -2.5), "CONC. SLAB");
}

/// An aluminum entrance door in storefront framing: frame 2" x 4 1/2" (or as the door's
/// `depth` and `jamb`), stiles and rails of the series, glass in gaskets.
fn storefront(d: &mut D, door: &Door, l: &Lay, part: Part) {
    let (fd, fw) = (door.depth, door.jamb);
    // The frame sits centred in the wall's opening.
    let f0 = (l.xs + l.wd) / 2.0 - fd / 2.0;
    let f1 = f0 + fd;
    let c = (f0 + f1) / 2.0;
    let (x0, x1) = (c - door.leaf / 2.0, c + door.leaf / 2.0);
    if part == Part::Sill {
        storefront_threshold(d, door, l, f0, f1, x0, x1);
        return;
    }
    if part == Part::Meet {
        // A pair's meeting stiles: pile weatherstrip between, the active one's astragal.
        let py = |x: f64| l.xi - x;
        for (e, to) in [(-0.0625_f64, -14.0_f64), (0.0625, 14.0)] {
            let s = e + (to - e).signum() * door.stile;
            let (sa, sb) = (e.min(s), e.max(s));
            d.rect(sa, py(x1), sb, py(x0), Wide);
            d.rect(sa + 0.125, py(x1) + 0.125, sb - 0.125, py(x0) - 0.125, Thin);
            let gc = py(c);
            let g = door.glass;
            d.line(&[(s, gc - g / 2.0), (to, gc - g / 2.0)], Medium);
            d.line(&[(s, gc + g / 2.0), (to, gc + g / 2.0)], Medium);
        }
        d.rect(-0.5, py(x0), 0.5, py(x0) + 0.25, Wide);
        d.rect(-0.0625, py(x1) + 0.3, 0.0625, py(x0) - 0.3, Thin);
        // The glazing gaskets each side of the glass, and the push/pull beyond.
        for k in [-1.0, 1.0] {
            let s = k * (door.stile + 0.0625);
            let gc = py(c);
            let g = door.glass;
            d.rect(s, gc - g / 2.0 - 0.25, s + k * 0.5, gc - g / 2.0, Medium);
            d.rect(s, gc + g / 2.0, s + k * 0.5, gc + g / 2.0 + 0.25, Medium);
        }
        d.line(
            &[
                (-2.5, py(x1) - 0.1),
                (-2.5, py(x1) - 2.5),
                (-1.5, py(x1) - 2.5),
            ],
            Beyond,
        );
        d.note((-2.5, py(x1) - 1.5), "PULL HANDLE BEYOND");
        d.brk((-14.0, py(x1) - 1.0), (-14.0, py(x0) + 1.0));
        d.brk((14.0, py(x1) - 1.0), (14.0, py(x0) + 1.0));
        d.note((0.3, py(x0) + 0.15), "ASTRAGAL W/ PILE WEATHERSTRIP");
        d.note(
            (-10.0, py(c)),
            &format!("{} INSULATING GLASS IN GASKETS", fr(door.glass)),
        );
        d.note(
            (-door.stile / 2.0, py(x1)),
            &format!(
                "{}{} STILES, {} DEEP",
                leaf_label(door),
                fr(door.stile),
                fr(door.leaf)
            ),
        );
        d.note((10.0, py(c)), "INACTIVE LEAF, FLUSH BOLTS OR CVR");
        d.caption("PLAN: EXTERIOR AT TOP.");
        return;
    }
    let jamb = part == Part::Jamb;
    let mut t = if jamb { T::jamb(d, l) } else { T::head(d, l) };
    let yt = -0.375;
    let yb = yt - fw;
    // The frame tube, thermally broken, and the door stop with its weatherstrip.
    let pts = [(f0, yb), (f1, yb), (f1, yt), (f0, yt)];
    member(&mut t, Mat::Aluminum, &pts, f0, yb, f1, yt);
    t.rect(x1 + 0.0625, yb - 0.625, x1 + 0.625, yb, Wide);
    t.circle(x1 + 0.03, yb - 0.35, 0.09, Thin);
    if !jamb && door.head_note.contains("CONCEALED") {
        t.rect(f0 + 0.3, yb + 0.15, f1 - 0.3, yt - 0.15, Hidden);
    }
    // The door's top rail (or hinge stile), glass in gaskets.
    let edge = yb - 0.125;
    let rail = if jamb { door.stile } else { door.top_rail };
    let r = edge - rail;
    t.poly(&[(x0, edge), (x1, edge), (x1, r), (x0, r)], Wide);
    t.rect(x0 + 0.125, edge - 0.125, x1 - 0.125, r + 0.125, Thin);
    let g = door.glass;
    t.line(&[(c - g / 2.0, r - 0.5), (c - g / 2.0, -14.0)], Medium);
    t.line(&[(c + g / 2.0, r - 0.5), (c + g / 2.0, -14.0)], Medium);
    t.rect(x0, r, c - g / 2.0, r - 0.6, Medium);
    t.rect(c + g / 2.0, r, x1, r - 0.6, Medium);
    // Shim, sealant both sides, the wall.
    t.seal("seal-12", (f0 - 0.0, yt + 0.19), (-1.0, 0.0));
    t.seal("seal-12", (f1, yt + 0.19), (1.0, 0.0));
    let end = 14.0;
    let stud = if l.wd > 4.0 { "stud-6" } else { "stud-358" };
    let mid = l.wd / 2.0;
    t.comp(stud, (mid, 0.0), (mid, 1.0), false);
    t.comp(stud, (mid, 1.75), (mid, 2.75), false);
    t.sheet(-0.625, 0.0, 0.0, end);
    t.gyp(l.wd, 0.0, l.xi, end);
    if door.wall == Wall::MetalPanel {
        t.cut_rect(-2.125, 0.0, -0.625, end, RigidInsulation);
        t.rect(l.xo, 0.0, -2.125, end, Medium);
        t.flashing(&[(l.xo - 0.5, 0.0), (f1 - 1.0, 0.0)]);
        t.note(((l.xo - 2.125) / 2.0, 6.0), "METAL PANEL ON 1-1/2\" CI");
    }
    if !jamb {
        t.flashing(&[(l.xo - 0.75, 0.08), (l.xo - 0.75, -0.3)]);
    }
    t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
    t.brk((x0 - 1.0, -14.0), (x1 + 1.0, -14.0));
    let what = if jamb { "JAMB" } else { "HEAD" };
    t.note(
        (f0 + 0.5, yt - fw / 2.0),
        &format!(
            "{} {what}, {} x {}, THERMALLY BROKEN",
            door.frame_note_or(),
            fr(fw),
            fr(fd)
        ),
    );
    let part_name = if jamb { "HINGE STILE" } else { "TOP RAIL" };
    t.note(
        (c, r + rail / 2.0),
        &format!(
            "{}{} {part_name}, {} DEEP",
            leaf_label(door),
            fr(rail),
            fr(door.leaf)
        ),
    );
    t.note(
        (c, -11.0),
        &format!("{} INSULATING GLASS IN GASKETS", fr(g)),
    );
    t.note((x1 + 0.35, yb - 0.3), "DOOR STOP W/ WEATHERSTRIP");
    t.note((f1 + 0.25, yt + 0.19), "SEALANT & BACKER ROD, SHIM");
    t.note((mid, 0.5), "DOUBLED STUD FRAMING AT OPENING");
    if !jamb && !door.head_note.is_empty() {
        t.note((c, (yb + yt) / 2.0), door.head_note);
    }
}

#[allow(clippy::too_many_arguments)]
fn storefront_threshold(d: &mut D, door: &Door, l: &Lay, f0: f64, f1: f64, x0: f64, x1: f64) {
    let (w, h) = (door.sill_d, door.sill_h);
    let c = (x0 + x1) / 2.0;
    d.cut(
        &[
            (l.xo - 14.0, 0.0),
            (l.xi + 14.0, 0.0),
            (l.xi + 14.0, -5.0),
            (l.xo - 14.0, -5.0),
        ],
        Concrete,
    );
    let (a, b) = (c - w / 2.0, c + w / 2.0);
    d.poly(&[(a, 0.0), (a + 1.0, h), (b - 1.0, h), (b, 0.0)], Wide);
    d.region_rect(a, -0.06, b, 0.0, Solid);
    let under = h + 0.1875;
    d.poly(
        &[
            (x0, under),
            (x1, under),
            (x1, under + door.bottom_rail),
            (x0, under + door.bottom_rail),
        ],
        Wide,
    );
    d.rect(
        x0 + 0.125,
        under + 0.125,
        x1 - 0.125,
        under + door.bottom_rail - 0.125,
        Thin,
    );
    let g = door.glass;
    let r = under + door.bottom_rail;
    d.line(&[(c - g / 2.0, r + 0.5), (c - g / 2.0, 14.0)], Medium);
    d.line(&[(c + g / 2.0, r + 0.5), (c + g / 2.0, 14.0)], Medium);
    d.rect(x0, r, c - g / 2.0, r + 0.6, Medium);
    d.rect(c + g / 2.0, r, x1, r + 0.6, Medium);
    d.rect(x0 + 0.2, under - 0.18, x1 - 0.2, under, Thin);
    d.line(&[(f0, 0.0), (f0, 14.0)], Beyond);
    d.line(&[(f1, 0.0), (f1, 14.0)], Beyond);
    d.brk((x0 - 1.0, 14.0), (x1 + 1.0, 14.0));
    d.brk((l.xo - 14.0, -5.0), (l.xi + 14.0, -5.0));
    let sill = if door.sill_note.is_empty() {
        "ALUMINUM THRESHOLD"
    } else {
        door.sill_note
    };
    d.note(
        (c, h * 0.5),
        &format!("{sill}, {} x {}, IN FULL BED OF SEALANT", fr(w), fr(h)),
    );
    d.note(
        (c, under + door.bottom_rail / 2.0),
        &format!("{}{} BOTTOM RAIL", leaf_label(door), fr(door.bottom_rail)),
    );
    d.note((x1 - 0.3, under - 0.1), "EPDM SWEEP");
    d.note((f0, 10.0), "FRAME JAMB BEYOND");
    d.note((l.xi + 8.0, -2.5), "CONC. SLAB");
    d.note((c, 12.0), &format!("{} INSULATING GLASS", fr(g)));
}

// ------------------------------------------------------------------------------------------
// Interior and specialty
// ------------------------------------------------------------------------------------------

/// A leaf in a partition: swinging in a wood jamb, sliding into a pocket, or hung on the
/// wall's face from a barn door track.
fn interior(d: &mut D, door: &Door, l: &Lay, part: Part) {
    let t_leaf = door.leaf;
    let wall_w = l.xi - l.xo;
    match (door.op, part) {
        (Op::Barn, Part::Sill) | (Op::Pocket, Part::Sill) | (Op::Interior, Part::Sill) => {
            // The floor: continuous under the door, the leaf undercut over it.
            let ff = 0.75;
            d.sheet(l.xo - 12.0, -0.75, l.xi + 12.0, 0.0);
            d.cut_rect(l.xo - 12.0, 0.0, l.xi + 12.0, ff, Wood);
            let (x0, x1, note) = match door.op {
                Op::Barn => (
                    l.xi + 0.75,
                    l.xi + 0.75 + t_leaf,
                    "BARN DOOR, 1/2\" ABOVE FLOOR",
                ),
                _ => {
                    let c = (l.xo + l.xi) / 2.0;
                    (c - t_leaf / 2.0, c + t_leaf / 2.0, "")
                }
            };
            let under = ff + 0.5;
            d.rect(x0, under, x1, 14.0, Medium);
            d.line(
                &[
                    (x0, under + door.bottom_rail),
                    (x1, under + door.bottom_rail),
                ],
                Thin,
            );
            match door.op {
                Op::Pocket => {
                    d.rect(x0 - 0.25, ff, x1 + 0.25, ff + 0.375, Medium);
                    d.line(
                        &[
                            (x0 + 0.2, under),
                            (x0 + 0.2, under + 0.5),
                            (x1 - 0.2, under + 0.5),
                            (x1 - 0.2, under),
                        ],
                        Hidden,
                    );
                    d.note((x0 - 0.25, ff + 0.2), "FLOOR GUIDE AT POCKET OPENING");
                    d.note(
                        (x0 + 0.2, under + 0.3),
                        "ROUTED GROOVE / GUIDE IN DOOR BOTTOM",
                    );
                }
                Op::Barn => {
                    d.rect(l.xi, ff, l.xi + 0.75 + t_leaf + 0.75, ff + 0.75, Medium);
                    d.rect(l.xi, ff + 0.75, l.xi + 0.5, ff + 4.0, Medium);
                    d.note(
                        (l.xi + 0.75 + t_leaf + 0.4, ff + 0.4),
                        "FLOOR STAY / GUIDE AT WALL",
                    );
                    d.note((l.xi + 0.25, ff + 3.0), "BASE");
                }
                _ => {}
            }
            d.line(&[(l.xo, ff), (l.xo, 14.0)], Beyond);
            d.line(&[(l.xi, ff), (l.xi, 14.0)], Beyond);
            // The joist under the subfloor, and the base each side beyond the jamb.
            d.board(l.xo - 12.0, -10.0, l.xi + 12.0, -0.75);
            for (a, b) in [(l.xo - 12.0, l.xo - 0.6875), (l.xi + 0.6875, l.xi + 12.0)] {
                d.rect(a, ff, b, ff + 3.5, Beyond);
                d.line(&[(a, ff + 3.0), (b, ff + 3.0)], Beyond);
            }
            d.brk((x0 - 1.0, 14.0), (x1 + 1.0, 14.0));
            d.brk((l.xo - 12.0, -10.0), (l.xi + 12.0, -10.0));
            d.note((l.xi + 6.0, ff + 2.0), "BASE BEYOND");
            d.note((l.xi + 6.0, -5.0), "FLOOR JOIST");
            d.note(
                ((x0 + x1) / 2.0, 8.0),
                &format!("{}{} DOOR", leaf_label(door), fr(t_leaf)),
            );
            if !note.is_empty() {
                d.note((x1, under + 0.25), note);
            } else {
                d.note((x1, under), "1/2\" UNDERCUT ABOVE FINISH FLOOR");
            }
            d.note((l.xo - 6.0, ff / 2.0), "3/4\" FINISH FLOOR, CONTINUOUS");
            d.note((l.xo, 10.0), "JAMB BEYOND");
            return;
        }
        _ => {}
    }
    let jamb = part == Part::Jamb;
    let mut t = if jamb { T::jamb(d, l) } else { T::head(d, l) };
    let end = 14.0;
    match door.op {
        Op::Pocket => {
            // The pocket frame's split studs (steel-wrapped) and the track at the head.
            let c = (l.xo + l.xi) / 2.0;
            let (x0, x1) = (c - t_leaf / 2.0, c + t_leaf / 2.0);
            if jamb {
                // The pocket: split studs either side of the leaf, gypsum on them.
                for i in 0..3 {
                    let y = 2.0 + f64::from(i) * 5.5;
                    if y + 1.5 > end {
                        break;
                    }
                    t.lumber(0.0, y, x0 - 0.75, y + 1.5);
                    t.lumber(x1 + 0.75, y, l.wd, y + 1.5);
                }
                t.gyp(l.xo, -0.5, 0.0, end);
                t.gyp(l.wd, -0.5, l.xi, end);
                t.cut_rect(0.0, -0.5, l.wd, 0.25, Wood);
                t.poly(&[(x0, -6.0), (x1, -6.0), (x1, end), (x0, end)], Medium);
                t.rect(x0 - 0.25, -0.5, x0, 0.0, Medium);
                t.rect(x1, -0.5, x1 + 0.25, 0.0, Medium);
                t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
                t.note((x0 - 0.4, 4.75), "STEEL-WRAPPED SPLIT STUDS @ 16\" O.C.");
                t.note(
                    ((x0 + x1) / 2.0, 7.0),
                    &format!("{}{} DOOR IN POCKET", leaf_label(door), fr(t_leaf)),
                );
                t.note((x0 - 0.12, -0.25), "SPLIT JAMB W/ GUIDES");
                t.note((l.wd / 2.0, -0.1), "WOOD JAMB AT POCKET OPENING");
                t.note((l.xi - 0.25, 10.0), "1/2\" GYP. BD., 1\" SCREWS MAX.");
            } else {
                t.lumber(0.0, 0.0, 1.5, 5.5);
                t.lumber(l.wd - 1.5, 0.0, l.wd, 5.5);
                t.cut_rect(1.5, 0.0, l.wd - 1.5, 5.5, Wood);
                t.lumber(0.0, 5.5, l.wd, 7.0);
                t.gyp(l.xo, -0.5, 0.0, end);
                t.gyp(l.wd, -0.5, l.xi, end);
                t.flashing(&[
                    (c - 0.75, 0.0),
                    (c - 0.75, -1.25),
                    (c + 0.75, -1.25),
                    (c + 0.75, 0.0),
                ]);
                t.circle(c, -0.8, 0.3, Medium);
                t.poly(
                    &[(x0, -1.625), (x1, -1.625), (x1, -14.0), (x0, -14.0)],
                    Medium,
                );
                t.lumber(l.xo - 0.6875, -0.25, l.xo, 3.25);
                t.lumber(l.xi, -0.25, l.xi + 0.6875, 3.25);
                t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
                t.brk((x0 - 1.0, -14.0), (x1 + 1.0, -14.0));
                t.note(
                    (c, -0.4),
                    door.head_note_or("STEEL TRACK W/ BALL-BEARING HANGERS"),
                );
                t.note((c, 3.0), "HEADER BY FRAME KIT, 2x HEADER ABOVE");
                t.note(
                    ((x0 + x1) / 2.0, -8.0),
                    &format!("{}{} DOOR", leaf_label(door), fr(t_leaf)),
                );
                t.note((l.xi + 0.35, 2.0), "CASING");
            }
        }
        Op::Barn => {
            // The track on a backer on the wall's face, the leaf hung clear of it.
            let x0 = l.xi + 0.75;
            let x1 = x0 + t_leaf;
            t.lumber(0.0, 0.0, 1.5, 5.5);
            t.lumber(1.5, 0.0, 3.0, 5.5);
            t.lumber(0.0, 5.5, l.wd, 7.0);
            t.gyp(l.xo, -0.5, 0.0, end);
            t.gyp(l.wd, if jamb { 0.0 } else { -0.5 }, l.xi, end);
            t.cut_rect(l.xo, -0.5, l.wd, 0.0, Wood);
            if jamb {
                t.lumber(l.xi, -0.0, l.xi + 0.6875, 3.5);
                t.poly(&[(x0, -4.0), (x1, -4.0), (x1, 2.0), (x0, 2.0)], Medium);
                t.note((l.xi + 0.35, 1.5), "CASED OPENING TRIM");
                t.note(((x0 + x1) / 2.0, -2.0), "DOOR OVERLAPS OPENING 1\" MIN.");
            } else {
                t.cut_rect(l.xi, 1.0, l.xi + 1.5, 6.5, Wood);
                t.rect(l.xi + 1.5, 3.0, l.xi + 1.75, 4.5, Wide);
                t.circle(l.xi + 1.85, 4.0, 1.0, Medium);
                t.line(&[(l.xi + 2.05, 3.5), (l.xi + 2.05, -1.0)], Wide);
                t.poly(&[(x0, -0.5), (x1, -0.5), (x1, -14.0), (x0, -14.0)], Medium);
                t.brk((x0 - 1.0, -14.0), (x1 + 2.0, -14.0));
                t.note((l.xi + 0.75, 5.0), "2x6 BACKER, LAGGED TO EACH STUD");
                t.note(
                    (l.xi + 1.6, 3.5),
                    door.head_note_or("FLAT BAR TRACK W/ STANDOFFS"),
                );
                t.note((l.xi + 1.85, 4.0), "STRAP HANGER & WHEEL");
                t.note((l.wd / 2.0, 6.0), "SOLID BLOCKING FOR TRACK");
            }
            t.note(
                ((x0 + x1) / 2.0, if jamb { -3.0 } else { -8.0 }),
                &format!("{}{} BARN DOOR", leaf_label(door), fr(t_leaf)),
            );
            t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
        }
        _ => {
            // A leaf in a wood jamb with a stop and casings each side.
            let jt = door.jamb;
            let stop_at = l.xo + (wall_w - t_leaf) / 2.0;
            let (x0, x1) = (stop_at, stop_at + t_leaf);
            let yt = -0.25;
            let yb = yt - jt;
            t.cut_rect(l.xo - 0.0625, yb, l.xi + 0.0625, yt, Wood);
            t.cut_rect(x1 + 0.0625, yb - 0.5, x1 + 1.375 + 0.0625, yb, Wood);
            t.poly(
                &[(x0, yb - 0.125), (x1, yb - 0.125), (x1, -14.0), (x0, -14.0)],
                Medium,
            );
            let rail = if jamb { door.stile } else { door.top_rail };
            t.line(&[(x0, yb - 0.125 - rail), (x1, yb - 0.125 - rail)], Thin);
            t.region_rect(l.xo.max(0.0), yt, l.wd, 0.0, Sand);
            t.lumber(l.xo - 0.6875, yb + 0.1875, l.xo, yb + 0.1875 + 2.5);
            t.lumber(l.xi, yb + 0.1875, l.xi + 0.6875, yb + 0.1875 + 2.5);
            if jamb {
                t.lumber(0.0, 0.0, l.wd, 1.5);
                t.lumber(0.0, 1.5, l.wd, 3.0);
                t.note((l.wd / 2.0, 1.5), "DOUBLE STUD AT OPENING");
            } else {
                t.lumber(0.0, 0.0, 1.5, 5.5);
                t.cut_rect(1.5, 0.0, 2.0, 5.5, Wood);
                t.lumber(2.0, 0.0, l.wd, 5.5);
                t.lumber(0.0, 5.5, l.wd, 7.0);
                t.note(
                    (l.wd / 2.0, 3.0),
                    "(2) 2x6 HEADER, OR FLAT 2x AT NON-BEARING",
                );
            }
            t.gyp(l.xo, 0.0, 0.0, end);
            t.gyp(l.wd, 0.0, l.xi, end);
            t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
            t.brk((x0 - 1.0, -14.0), (x1 + 1.0, -14.0));
            let what = if jamb { "JAMB" } else { "HEAD JAMB" };
            t.note(
                ((l.xo + l.xi) / 2.0, (yt + yb) / 2.0),
                &format!(
                    "{} {what}, {} x {}",
                    door.frame_note_or(),
                    fr(jt),
                    fr(wall_w + 0.125)
                ),
            );
            t.note((x1 + 0.7, yb - 0.25), "WOOD STOP");
            t.note(
                ((x0 + x1) / 2.0, -8.0),
                &format!("{}{} DOOR", leaf_label(door), fr(t_leaf)),
            );
            t.note((l.xi + 0.35, yb + 1.5), "CASING, 3/16\" REVEAL");
            t.note((l.wd * 0.4, yt / 2.0), "SHIMS");
        }
    }
}

impl Door {
    fn head_note_or(&self, or: &'static str) -> &'static str {
        if self.head_note.is_empty() {
            or
        } else {
            self.head_note
        }
    }
}

/// A sectional overhead door: the head (with its headroom), the jamb with the track, and
/// the bottom seal on the slab.
fn overhead(d: &mut D, door: &Door, l: &Lay, part: Part) {
    let t_leaf = door.leaf;
    let jt = 1.5;
    match part {
        Part::Sill => {
            d.cut(
                &[
                    (l.xo - 14.0, -0.5),
                    (l.xo, 0.0),
                    (l.xi + 14.0, 0.0),
                    (l.xi + 14.0, -5.0),
                    (l.xo - 14.0, -5.0),
                ],
                Concrete,
            );
            let x0 = l.xi - 0.25 - t_leaf;
            let x1 = x0 + t_leaf;
            d.poly(&[(x0, 0.0), (x1, 0.0), (x1, 14.0), (x0, 14.0)], Medium);
            d.region_rect(
                x0 + 0.05,
                door.bottom_rail,
                x1 - 0.05,
                14.0,
                RigidInsulation,
            );
            d.line(&[(x0, door.bottom_rail), (x1, door.bottom_rail)], Thin);
            d.poly(
                &[
                    (x0 - 0.1, 0.0),
                    (x0 - 0.1, 0.6),
                    (x1 + 0.1, 0.6),
                    (x1 + 0.1, 0.0),
                ],
                Thin,
            );
            d.rect(x0 + 0.2, -0.0, x1 - 0.2, 0.25, Thin);
            d.line(&[(x0 - 0.5, 0.0), (x0 - 0.5, 14.0)], Beyond);
            d.brk((x0 - 1.0, 14.0), (x1 + 1.0, 14.0));
            d.brk((l.xo - 14.0, -5.0), (l.xi + 14.0, -5.0));
            d.note(
                ((x0 + x1) / 2.0, 0.2),
                door.sill_note_or("VINYL BOTTOM SEAL IN ALUMINUM RETAINER"),
            );
            d.note(
                ((x0 + x1) / 2.0, 6.0),
                &format!("{}{} INSULATED STEEL SECTION", leaf_label(door), fr(t_leaf)),
            );
            d.note((l.xo - 6.0, -0.3), "CONC. APRON, SLOPED 1/8\":12\" AWAY");
            d.note((l.xi + 6.0, -2.5), "GARAGE SLAB");
            d.note((x0 - 0.5, 10.0), "JAMB & STOP MOULDING BEYOND");
        }
        Part::Jamb | Part::Head => {
            let jamb = part == Part::Jamb;
            let mut t = if jamb { T::jamb(d, l) } else { T::head(d, l) };
            let end = if jamb { 14.0 } else { 20.0 };
            // The door's sections run behind the opening, inside the wall's face.
            let x0 = l.xi + 0.25;
            let x1 = x0 + t_leaf;
            // Wood jamb (head jamb) lined to the opening, stop moulding with its seal.
            t.lumber(0.0, -jt, l.wd, 0.0);
            t.lumber(l.xo, -jt - 0.0, l.xs, 2.5);
            t.cut_rect(l.xi - 0.5, -jt - 2.0, l.xi, -jt, Wood);
            t.circle(l.xi + 0.05, -jt - 1.0, 0.2, Thin);
            t.poly(
                &[(x0, -jt - 2.0), (x1, -jt - 2.0), (x1, -14.0), (x0, -14.0)],
                Medium,
            );
            t.region_rect(x0 + 0.05, -14.0, x1 - 0.05, -jt - 2.0, RigidInsulation);
            if jamb {
                // The vertical track on its flag bracket, lagged to the jamb.
                t.line(
                    &[
                        (x1 + 0.25, -jt - 0.5),
                        (x1 + 1.75, -jt - 0.5),
                        (x1 + 1.75, -jt - 3.25),
                        (x1 + 0.25, -jt - 3.25),
                    ],
                    Wide,
                );
                t.line(&[(l.xi, -jt - 0.25), (x1 + 2.0, -jt - 0.25)], Wide);
                t.circle(x1 + 1.0, -jt - 2.0, 0.5, Medium);
                t.lumber(0.0, 0.0, l.wd, 1.5);
                t.lumber(0.0, 1.5, l.wd, 3.0);
                t.gyp(l.wd, 0.0, l.xi, end);
                t.sheet(-0.5, 0.0, 0.0, end);
                t.note(
                    (x1 + 1.75, -jt - 1.75),
                    door.head_note_or("VERTICAL TRACK ON FLAG BRACKET, 3-3/4\" SIDE ROOM"),
                );
                t.note((x1 + 1.0, -jt - 2.0), "ROLLER");
                t.note((l.wd / 2.0, 1.5), "2x6 KING & JACK STUDS");
            } else {
                t.lumber(0.0, 0.0, 1.5, 11.25);
                t.lumber(1.5, 0.0, 3.0, 11.25);
                t.lumber(3.0, 0.0, l.wd, 11.25);
                t.lumber(0.0, 11.25, l.wd, 12.75);
                t.gyp(l.wd, 0.0, l.xi, end);
                t.sheet(-0.5, 0.0, 0.0, end);
                // The spring shaft and the horizontal track, beyond.
                t.circle(l.xi + 4.0, 6.0, 0.5, Hidden);
                t.circle(l.xi + 4.0, 6.0, 2.0, Hidden);
                t.line(
                    &[(x1 + 0.5, -jt - 2.0), (x1 + 2.0, 1.0), (x1 + 14.0, 1.0)],
                    Hidden,
                );
                t.note(
                    (l.xi + 4.0, 6.0),
                    door.seal_note_or("TORSION SPRING ON SHAFT, HEADROOM PER TRACK TYPE"),
                );
                t.note((x1 + 8.0, 1.0), "HORIZONTAL TRACK BEYOND");
                t.note((1.5, 6.0), "(3) 2x12 HEADER W/ 2x6 HEAD JAMB");
            }
            t.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
            t.brk((x0 - 1.0, -14.0), (x1 + 1.0, -14.0));
            t.note((l.xs - 0.6, 1.0), "EXTERIOR TRIM, SEALANT TO JAMB");
            t.note(
                (l.xi - 0.25, -jt - 1.0),
                "STOP MOULDING W/ VINYL WEATHERSEAL ON DOOR FACE",
            );
            t.note(
                ((x0 + x1) / 2.0, -8.0),
                &format!(
                    "{}{} INSULATED STEEL SECTIONS",
                    leaf_label(door),
                    fr(t_leaf)
                ),
            );
            t.note((l.wd / 2.0, -jt / 2.0), "P.T. 2x6 JAMB");
        }
        Part::Meet => {}
    }
}

/// A frameless pivot door in a partition (FritsJurgens System M+): the hinge in the leaf's
/// bottom rail on a floor plate, a top pivot in its top rail, no frame, no floor box.
fn frameless_pivot(d: &mut D, door: &Door, l: &Lay, part: Part) {
    let t_leaf = door.leaf;
    let c = (l.xo + l.xi) / 2.0;
    let (x0, x1) = (c - t_leaf / 2.0, c + t_leaf / 2.0);
    let mm = |v: f64| v / 25.4;
    let end = 14.0;
    match part {
        Part::Sill => {
            let ff = 0.75;
            d.sheet(l.xo - 12.0, -0.75, l.xi + 12.0, 0.0);
            d.cut_rect(l.xo - 12.0, 0.0, l.xi + 12.0, ff, Wood);
            // The floor plate and the leaf 11 mm above the floor.
            let plate = ff + mm(8.0);
            d.cut_rect(c - mm(40.0), ff, c + mm(40.0), plate, Solid);
            let under = ff + mm(11.0);
            d.rect(x0, under, x1, end, Medium);
            d.rect(x0 + 0.1, under + 0.05, x1 - 0.1, under + mm(149.8), Hidden);
            d.line(&[(c, ff - mm(30.0)), (c, plate + 0.3)], Hidden);
            d.line(&[(l.xo, ff), (l.xo, end)], Beyond);
            d.line(&[(l.xi, ff), (l.xi, end)], Beyond);
            d.brk((x0 - 1.0, end), (x1 + 1.0, end));
            d.brk((l.xo - 12.0, -0.75), (l.xi + 12.0, -0.75));
            d.note(
                (c + mm(40.0), plate),
                "STAINLESS FLOOR PLATE 80 x 60 x 8 MM, GLUED; PINS IN 8 MM HOLES 30 MM DEEP",
            );
            d.note(
                (x1 - 0.1, under + 3.0),
                door.sill_note_or("BOTTOM PIVOT IN BOTTOM RAIL, 273 x 32 MM MILLING"),
            );
            d.note((x1, under), "11 MM (7/16\") GAP AT FLOOR");
            d.note(
                ((x0 + x1) / 2.0, 9.0),
                &format!("{}{} PIVOT LEAF", leaf_label(door), fr(t_leaf)),
            );
            d.note(
                (l.xo - 6.0, ff / 2.0),
                "FINISH FLOOR CONTINUOUS, NO FLOOR BOX",
            );
        }
        Part::Head => {
            // The head's soffit (gypsum on a header), the ceiling plate, the top pivot.
            d.lumber(0.0, 1.0, 1.5, 6.5);
            d.lumber(l.wd - 1.5, 1.0, l.wd, 6.5);
            d.cut_rect(1.5, 1.0, l.wd - 1.5, 6.5, Wood);
            d.gyp(l.xo, 0.5, l.xi, 1.0);
            d.gyp(l.xo, 1.0, 0.0, end);
            d.gyp(l.wd, 1.0, l.xi, end);
            d.cut_rect(c - mm(30.0), 0.5 - mm(4.0), c + mm(30.0), 0.5, Solid);
            let top = 0.5 - mm(8.0);
            d.rect(x0, -end, x1, top, Medium);
            d.rect(x0 + 0.1, top - mm(85.0), x1 - 0.1, top - 0.05, Hidden);
            d.circle(c, 0.5 - mm(10.0), mm(5.0), Thin);
            d.brk((l.xo - 2.0, end), (l.xi + 2.0, end));
            d.brk((x0 - 1.0, -end), (x1 + 1.0, -end));
            d.note(
                (c, 0.4),
                "CEILING PLATE 60 x 40 MM, 10 MM PIN IN 18 MM HOLE",
            );
            d.note(
                (x1 - 0.1, top - 2.0),
                door.head_note_or("TOP PIVOT RECESSED IN TOP RAIL"),
            );
            d.note((x1, top), "4-10 MM GAP AT HEAD");
            d.note(
                (l.wd / 2.0, 4.0),
                "SOLID HEADER / BLOCKING FOR CEILING PLATE",
            );
            d.note(
                ((x0 + x1) / 2.0, -9.0),
                &format!("{}{} PIVOT LEAF", leaf_label(door), fr(t_leaf)),
            );
        }
        _ => {
            // Plan: the leaf's edge clear of the wall's finished return, the pivot axis
            // 40 mm in from the edge.
            let py = |x: f64| l.xi - x;
            d.gyp(-0.5, py(l.xi), 0.0, py(l.xo));
            d.gyp(-end, py(l.xi), -0.5, py(l.wd));
            d.gyp(-end, py(0.0), -0.5, py(l.xo));
            d.lumber(-2.0, py(l.wd), -0.5, py(0.0));
            d.lumber(-3.5, py(l.wd), -2.0, py(0.0));
            let e = mm(7.0);
            d.rect(e, py(x1), end, py(x0), Medium);
            d.circle(e + mm(40.0), py(c), 0.12, Medium);
            d.brk((-end, py(l.xi) - 1.0), (-end, py(l.xo) + 1.0));
            d.brk((end, py(x1) - 1.0), (end, py(x0) + 1.0));
            d.note((e + mm(40.0), py(c)), "PIVOT AXIS 40 MM FROM LEAF EDGE");
            d.note((e / 2.0, py(x0)), "7 MM MIN. GAP TO WALL (40 MM LEAF)");
            d.note(
                (8.0, py(c)),
                &format!("{}{} PIVOT LEAF", leaf_label(door), fr(t_leaf)),
            );
            d.note((-0.25, py(l.xo) + 0.0), "FINISHED RETURN, NO FRAME");
            d.note((-2.75, py(c)), "DOUBLE STUD AT OPENING");
        }
    }
}

impl Door {
    fn seal_note_or(&self, or: &'static str) -> &'static str {
        if self.seal_note.is_empty() {
            or
        } else {
            self.seal_note
        }
    }
    fn sill_note_or(&self, or: &'static str) -> &'static str {
        if self.sill_note.is_empty() {
            or
        } else {
            self.sill_note
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inches_are_written_as_drafted() {
        assert_eq!(fr(4.5625), "4-9/16\"");
        assert_eq!(fr(1.75), "1-3/4\"");
        assert_eq!(fr(0.75), "3/4\"");
        assert_eq!(fr(6.0), "6\"");
        assert_eq!(fr(0.0625), "1/16\"");
    }

    #[test]
    fn every_manufacturer_door_has_its_head_jamb_and_sill_and_cites_its_source() {
        let items = items();
        for door in catalog::DOORS {
            let ids: Vec<&str> = items
                .iter()
                .filter(|(i, _)| i.id.starts_with(door.id) && i.name.starts_with(door.name))
                .map(|(i, _)| i.id.as_str())
                .collect();
            for part in ["head", "jamb"] {
                assert!(
                    ids.contains(&format!("{}-{part}", door.id).as_str()),
                    "{} has no {part}",
                    door.id
                );
            }
            assert!(ids.len() >= 3, "{}", door.id);
            if !door.maker.is_empty() {
                assert!(door.url.starts_with("https://"), "{}", door.id);
                assert!(!door.document.is_empty(), "{}", door.id);
                assert!(!door.product.is_empty(), "{}", door.id);
            }
        }
        // A sliding door has its interlock, a pair its meeting stile.
        let has = |id: &str| items.iter().any(|(i, _)| i.id == id);
        for door in catalog::DOORS {
            if matches!(door.op, Op::Slide { .. }) {
                assert!(has(&format!("{}-interlock", door.id)), "{}", door.id);
            }
        }
    }

    #[test]
    fn every_manufacturer_window_has_its_details_and_cites_its_source() {
        let items = items();
        let has = |id: String| items.iter().any(|(i, _)| i.id == id);
        let windows = window_catalog::WINDOWS
            .iter()
            .chain(window_catalog::METAL_WINDOWS)
            .chain(window_catalog::HUNG_WINDOWS)
            .chain(window_catalog::COMMERCIAL_WINDOWS)
            .chain(window_catalog::CASEMENT_WINDOWS)
            .chain(window_catalog::VINYL_WINDOWS);
        let mut n = 0;
        for w in windows {
            n += 1;
            assert!(w.is_window(), "{}", w.id);
            for part in ["head", "jamb", "sill"] {
                assert!(has(format!("{}-{part}", w.id)), "{} has no {part}", w.id);
            }
            match w.op {
                Op::Window(Win::Hung { .. }) => assert!(has(format!("{}-meeting-rail", w.id))),
                Op::Window(Win::Slider) => assert!(has(format!("{}-interlock", w.id))),
                _ => {}
            }
            if !w.maker.is_empty() {
                assert!(w.url.starts_with("https://"), "{}", w.id);
                assert!(!w.document.is_empty() && !w.product.is_empty(), "{}", w.id);
            }
            assert!(w.category.starts_with("Windows"), "{}", w.id);
        }
        assert!(n >= 60, "{n} windows");
        // A hung window's head holds its upper sash only; its sill, the lower one.
        let dh = window_catalog::HUNG_WINDOWS[0];
        let l = Lay::of(&dh);
        assert_eq!(l.leaves.len(), 2);
        assert!(l.leaves[0].0 < l.leaves[1].0, "the upper sash runs outside");
    }

    #[test]
    fn a_door_is_drawn_at_its_published_sizes() {
        // The frame spans its depth; the leaf is its thickness; the sill stands its height.
        let door = catalog::DOORS
            .iter()
            .find(|d| matches!(d.op, Op::Swing { out: false, .. }) && !d.maker.is_empty())
            .unwrap();
        let l = Lay::of(door);
        assert!((l.f1 - l.f0 - door.depth).abs() < 1e-9);
        let (x0, x1) = l.leaves[0];
        assert!((x1 - x0 - door.leaf).abs() < 1e-9);
        assert!(x0 > l.f0 && x1 < l.f1, "the leaf sits in its frame");
        // The jamb is the head turned: the same frame depth runs across the wall in plan.
        let head = draw(door, Part::Head).finish(4, super::super::TEXT_SIZE);
        let jamb = draw(door, Part::Jamb).finish(4, super::super::TEXT_SIZE);
        let span = |d: &super::super::Drawing, x: bool| {
            let v: Vec<f64> = d
                .regions
                .iter()
                .flat_map(|r| r.0.iter().map(|p| if x { p.x } else { p.y }))
                .collect();
            v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                - v.iter().cloned().fold(f64::INFINITY, f64::min)
        };
        assert!((span(&head, true) - span(&jamb, false)).abs() < 1.0);
        // Its sill carries its source as captions, and notes its height.
        let sill = draw(door, Part::Sill).finish(4, super::super::TEXT_SIZE);
        assert!(sill.captions.iter().any(|(_, t)| t == door.url));
        assert!(sill.notes.iter().any(|n| n.text.contains(&fr(door.sill_h))));
    }
}
