//! Curtain wall details (ADR-125): head, sill and jamb, the vertical mullion and the
//! horizontal (transom), the slab edge with its spandrel and fire safing, and a unitized
//! system's stack joint, drawn from a system's published face width, depth and glass.
//!
//! A member is drawn once in its own terms (n through the wall, interior +, 0 the glass's
//! centre plane; s along the wall), then laid as a plan (exterior up) or a section
//! (exterior to the left).

use super::super::FillPattern::{Concrete, Gray, Masonry, RigidInsulation, Sand, Solid};
use super::super::{DetailInfo, DetailSource, D};
use super::{fr, slug};
use crate::lines::LineStyle::{self, Beyond, Medium, Thin, Wide};

pub(super) const CW_TYPICAL: &str = "Curtain Wall - Typical";
pub(super) const CW_CAPTURED: &str = "Curtain Wall - Captured";
pub(super) const CW_SSG: &str = "Curtain Wall - Structural Silicone";
pub(super) const CW_UNITIZED: &str = "Curtain Wall - Unitized";
pub(super) const CW_EUROPEAN: &str = "Curtain Wall - European";

/// How the glass is held.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Glazing {
    /// A pressure plate and cover cap each side of every lite.
    Captured,
    /// Captured at the horizontals, structural silicone at the verticals.
    Ssg2,
    /// Structural silicone on all four sides, a silicone weather seal between lites.
    Ssg4,
    /// Factory-glazed units meeting at split mullions and stack joints.
    Unitized,
}

/// What the curtain wall meets at its jamb and head.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Adjoin {
    /// A steel stud wall with sheathing, continuous insulation and metal panel.
    StudWall,
    /// Masonry (CMU) or concrete.
    Masonry,
}

/// A curtain wall system. Lengths in inches.
#[derive(Debug, Clone, Copy)]
pub(super) struct Cw {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub maker: &'static str,
    pub product: &'static str,
    pub document: &'static str,
    pub url: &'static str,
    pub summary: &'static str,
    pub glazing: Glazing,
    pub adjoin: Adjoin,
    /// Face width (sightline) of a mullion or horizontal.
    pub face: f64,
    /// System depth, from the cover cap's face (or the glass) to the back of the mullion.
    pub depth: f64,
    /// Insulating glass thickness.
    pub glass: f64,
    /// Cover cap depth (0 where there is none).
    pub cap: f64,
    /// A unitized system's stack joint movement (0 for stick systems).
    pub movement: f64,
    /// The gap between the back of the mullions and the slab edge, filled with safing.
    pub safing_gap: f64,
    pub frame_note: &'static str,
    pub break_note: &'static str,
    pub glass_note: &'static str,
    pub anchor_note: &'static str,
    pub sill_note: &'static str,
    /// Sizes the source doesn't state, drawn at typical values.
    pub est: &'static str,
}

pub(super) const CW_BASE: Cw = Cw {
    id: "",
    name: "",
    category: CW_TYPICAL,
    maker: "",
    product: "",
    document: "",
    url: "",
    summary: "",
    glazing: Glazing::Captured,
    adjoin: Adjoin::StudWall,
    face: 2.5,
    depth: 7.5,
    glass: 1.0,
    cap: 1.0,
    movement: 0.0,
    safing_gap: 4.0,
    frame_note: "",
    break_note: "THERMAL ISOLATOR AT PRESSURE PLATE",
    glass_note: "",
    anchor_note: "",
    sill_note: "",
    est: "",
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Part {
    Head,
    Sill,
    Jamb,
    Mullion,
    Horizontal,
    SlabEdge,
    Stack,
}

impl Part {
    fn name(self) -> &'static str {
        match self {
            Part::Head => "Head",
            Part::Sill => "Sill",
            Part::Jamb => "Jamb",
            Part::Mullion => "Mullion",
            Part::Horizontal => "Horizontal",
            Part::SlabEdge => "Slab Edge",
            Part::Stack => "Stack Joint",
        }
    }
    fn plan(self) -> bool {
        matches!(self, Part::Jamb | Part::Mullion)
    }
    fn scale(self) -> u32 {
        match self {
            Part::Mullion | Part::Horizontal | Part::Stack | Part::Jamb => 2,
            _ => 4,
        }
    }
}

impl Cw {
    fn parts(&self) -> Vec<Part> {
        let mut p = vec![
            Part::Head,
            Part::Sill,
            Part::Jamb,
            Part::Mullion,
            Part::Horizontal,
            Part::SlabEdge,
        ];
        if self.glazing == Glazing::Unitized {
            p.push(Part::Stack);
        }
        p
    }
}

/// Every curtain wall detail, with its drawing.
pub(super) fn items() -> Vec<(DetailInfo, Box<dyn Fn() -> D + Send + Sync>)> {
    super::curtain_catalog::SYSTEMS
        .iter()
        .flat_map(|cw| {
            cw.parts().into_iter().map(move |part| {
                let what = part.name();
                let scale = part.scale();
                let info = DetailInfo {
                    id: format!("{}-{}", cw.id, slug(what)),
                    name: format!("{} - {what}", cw.name),
                    category: cw.category.into(),
                    scale,
                    scale_label: crate::ops::scale_label(scale),
                    description: format!(
                        "{} {what} {}.",
                        cw.summary,
                        if part.plan() { "in plan" } else { "in section" }
                    ),
                    user: false,
                    source: (!cw.maker.is_empty()).then(|| DetailSource {
                        manufacturer: cw.maker.into(),
                        product: cw.product.into(),
                        document: cw.document.into(),
                        url: cw.url.into(),
                    }),
                };
                let draw: Box<dyn Fn() -> D + Send + Sync> = Box::new(move || draw(cw, part));
                (info, draw)
            })
        })
        .collect()
}

/// Draws in a member's terms (n through the wall, interior +; s along it) as a plan
/// (exterior up) or a section (exterior to the left).
struct M<'a> {
    d: &'a mut D,
    plan: bool,
}

impl M<'_> {
    fn p(&self, (n, s): (f64, f64)) -> (f64, f64) {
        if self.plan {
            (s, -n)
        } else {
            (n, s)
        }
    }
    fn ps(&self, pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
        pts.iter().map(|q| self.p(*q)).collect()
    }
    fn box_(&self, n0: f64, s0: f64, n1: f64, s1: f64) -> (f64, f64, f64, f64) {
        let (a, b) = (self.p((n0, s0)), self.p((n1, s1)));
        (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
    }
    fn line(&mut self, pts: &[(f64, f64)], s: LineStyle) {
        let pts = self.ps(pts);
        self.d.line(&pts, s);
    }
    fn rect(&mut self, n0: f64, s0: f64, n1: f64, s1: f64, s: LineStyle) {
        let (a, b, c, e) = self.box_(n0, s0, n1, s1);
        self.d.rect(a, b, c, e, s);
    }
    fn region_rect(&mut self, n0: f64, s0: f64, n1: f64, s1: f64, f: super::FillPattern) {
        let (a, b, c, e) = self.box_(n0, s0, n1, s1);
        self.d.region_rect(a, b, c, e, f);
    }
    fn cut_rect(&mut self, n0: f64, s0: f64, n1: f64, s1: f64, f: super::FillPattern) {
        let (a, b, c, e) = self.box_(n0, s0, n1, s1);
        self.d.cut_rect(a, b, c, e, f);
    }
    fn sheet(&mut self, n0: f64, s0: f64, n1: f64, s1: f64) {
        let (a, b, c, e) = self.box_(n0, s0, n1, s1);
        self.d.sheet(a, b, c, e);
    }
    fn gyp(&mut self, n0: f64, s0: f64, n1: f64, s1: f64) {
        let (a, b, c, e) = self.box_(n0, s0, n1, s1);
        self.d.gyp(a, b, c, e);
    }
    fn flashing(&mut self, pts: &[(f64, f64)]) {
        let pts = self.ps(pts);
        self.d.flashing(&pts);
    }
    fn brk(&mut self, a: (f64, f64), b: (f64, f64)) {
        let (a, b) = (self.p(a), self.p(b));
        self.d.brk(a, b);
    }
    fn comp(&mut self, key: &'static str, a: (f64, f64), b: (f64, f64)) {
        let (a, b) = (self.p(a), self.p(b));
        self.d.comp(key, a, b, false);
    }
    /// A sealant joint at `at`, its bead facing `face` (in member terms).
    fn seal(&mut self, key: &'static str, at: (f64, f64), face: (f64, f64)) {
        let at = self.p(at);
        let tip = self.p((face.0, face.1));
        let o = self.p((0.0, 0.0));
        let f = (tip.0 - o.0, tip.1 - o.1);
        self.d.comp(key, at, (at.0 + f.1, at.1 - f.0), false);
    }
    fn note(&mut self, to: (f64, f64), text: &str) {
        let to = self.p(to);
        self.d.note(to, text);
    }
}

/// The member's glazing geometry: the glass's half thickness, where the mullion's face and
/// back are, and the cover cap's outer face.
struct G {
    gi: f64,
    front: f64,
    back: f64,
    outer: f64,
    /// Where the glass's edge stops, either side of the member's centre.
    edge: f64,
}

impl G {
    fn of(cw: &Cw) -> G {
        let gi = cw.glass / 2.0;
        let (front, outer) = match cw.glazing {
            Glazing::Ssg4 => (gi + 0.25, -gi),
            _ => (gi + 0.1875, -(gi + 0.4375 + cw.cap)),
        };
        let back = (outer + cw.depth).max(front + 2.0);
        G {
            gi,
            front,
            back,
            outer,
            edge: 0.375,
        }
    }
}

/// A mullion or horizontal across s 0 (its face width centred there): the back member, and
/// the glass held each side by `sides` (-1, +1, or both).
fn member(m: &mut M, cw: &Cw, g: &G, sides: &[f64], vertical_ssg: bool) {
    let hw = cw.face / 2.0;
    let unitized = cw.glazing == Glazing::Unitized;
    // The back member: one tube, or a unitized split mullion's two halves.
    if unitized {
        for k in [-1.0, 1.0] {
            let (a, b) = (k * 0.0625, k * hw);
            m.rect(g.front, a.min(b), g.back, a.max(b), Wide);
            m.rect(
                g.front + 0.1,
                a.min(b) + 0.1,
                g.back - 0.1,
                a.max(b) - 0.1,
                Thin,
            );
        }
        // Interlocking legs and the gasket between the halves.
        m.line(
            &[
                (g.back - 1.0, -0.0625),
                (g.back - 1.0, 0.25),
                (g.back - 0.6, 0.25),
            ],
            Wide,
        );
        m.line(
            &[
                (g.front + 1.0, 0.0625),
                (g.front + 1.0, -0.25),
                (g.front + 0.6, -0.25),
            ],
            Wide,
        );
        m.region_rect(g.front + 0.3, -0.0625, g.front + 0.6, 0.0625, Solid);
    } else {
        m.rect(g.front, -hw, g.back, hw, Wide);
        m.rect(g.front + 0.1, -hw + 0.1, g.back - 0.1, hw - 0.1, Thin);
        m.line(
            &[
                ((g.front + g.back) / 2.0, -hw + 0.1),
                ((g.front + g.back) / 2.0, hw - 0.1),
            ],
            Thin,
        );
    }
    // The glass each side, its edge in the pocket, and the gaskets on it.
    for &k in sides {
        let e = k * g.edge;
        let far = k * 14.0;
        m.line(&[(-g.gi, e), (-g.gi, far)], Medium);
        m.line(&[(g.gi, e), (g.gi, far)], Medium);
        m.region_rect(-g.gi, e, g.gi, e + k * 0.4, Gray);
        let ga = k * (hw - 0.15);
        if !(vertical_ssg || cw.glazing == Glazing::Ssg4) {
            m.region_rect(-g.gi - 0.1875, e + k * 0.1, -g.gi, ga, Solid);
        }
        m.region_rect(g.gi, e + k * 0.1, g.front, ga, Solid);
    }
    let captured = matches!(cw.glazing, Glazing::Captured | Glazing::Unitized)
        || !vertical_ssg && cw.glazing == Glazing::Ssg2;
    if captured {
        // The nose through the joint, the isolator, the pressure plate and its cap.
        let pp = -g.gi - 0.1875;
        m.rect(-g.gi + 0.05, -0.3, g.front, 0.3, Medium);
        m.cut_rect(pp, -0.3, -g.gi + 0.05, 0.3, Solid);
        if !unitized {
            m.rect(pp - 0.25, -hw + 0.125, pp, hw - 0.125, Wide);
            m.line(&[(pp - 0.25, 0.0), (g.front + 0.5, 0.0)], Thin);
        }
        let cap_in = if unitized { pp } else { pp - 0.25 };
        let o = cap_in - cw.cap.max(0.5);
        m.line(
            &[(cap_in, -hw + 0.1), (o, -hw), (o, hw), (cap_in, hw - 0.1)],
            Wide,
        );
    } else {
        // Structural silicone bite to the mullion, spacer tape outboard of it, and the weather
        // seal between the lites.
        for &k in sides {
            m.region_rect(g.gi, k * (g.edge + 0.1), g.front, k * (hw - 0.45), Gray);
            m.region_rect(g.gi, k * (hw - 0.45), g.front, k * (hw - 0.1), Solid);
            m.rect(g.front + 0.6, k * 0.35, g.front + 1.1, k * 0.85, Thin);
        }
        m.seal("seal-34", (-g.gi + 0.25, 0.0), (-1.0, 0.0));
    }
}

fn draw(cw: &Cw, part: Part) -> D {
    let mut d = D::new();
    let g = G::of(cw);
    {
        let mut m = M {
            d: &mut d,
            plan: part.plan(),
        };
        match part {
            Part::Mullion => mullion(&mut m, cw, &g),
            Part::Horizontal => horizontal(&mut m, cw, &g),
            Part::Jamb => jamb(&mut m, cw, &g),
            Part::Head => head(&mut m, cw, &g),
            Part::Sill => sill(&mut m, cw, &g),
            Part::SlabEdge => slab_edge(&mut m, cw, &g),
            Part::Stack => stack(&mut m, cw, &g),
        }
    }
    if part.plan() {
        d.caption("PLAN: EXTERIOR AT TOP.");
    }
    source(&mut d, cw);
    d
}

fn source(d: &mut D, cw: &Cw) {
    if cw.maker.is_empty() {
        d.caption(&format!("BASED ON: {}", cw.document.to_uppercase()));
        d.caption("TYPICAL DETAIL. ADJUST TO THE SYSTEM SPECIFIED.");
    } else {
        d.caption(&format!(
            "REDRAWN FROM {} PUBLISHED DETAILS: {}",
            cw.maker.to_uppercase(),
            cw.document.to_uppercase()
        ));
        d.caption(cw.url);
        d.caption("VERIFY WITH THE MANUFACTURER'S CURRENT DETAILS, ENGINEERING AND SHOP DRAWINGS.");
    }
    if !cw.est.is_empty() {
        d.caption(&format!(
            "NOT STATED IN THE SOURCE, DRAWN AT TYPICAL SIZES: {}",
            cw.est
        ));
    }
}

fn frame_name(cw: &Cw) -> String {
    if cw.frame_note.is_empty() {
        "CURTAIN WALL".into()
    } else {
        cw.frame_note.to_owned()
    }
}

fn glass_name(cw: &Cw) -> String {
    if cw.glass_note.is_empty() {
        format!("{} INSULATING GLASS", fr(cw.glass))
    } else {
        cw.glass_note.to_owned()
    }
}

/// Notes common to a mullion or horizontal.
fn member_notes(m: &mut M, cw: &Cw, g: &G, what: &str, glass_side: f64) {
    // Two-sided SSG: its verticals are structurally glazed, its horizontals captured.
    let vertical = what.contains("MULLION") || what.contains("JAMB");
    let ssg = cw.glazing == Glazing::Ssg4 || (cw.glazing == Glazing::Ssg2 && vertical);
    m.note(
        ((g.front + g.back) / 2.0, cw.face / 2.0 - 0.1),
        &format!(
            "{} {what}, {} FACE x {} SYSTEM DEPTH",
            frame_name(cw),
            fr(cw.face),
            fr(cw.depth)
        ),
    );
    m.note((-g.gi, glass_side * 8.0), &glass_name(cw));
    match cw.glazing {
        _ if ssg => {
            m.note((g.gi + 0.1, g.edge + 0.6), "STRUCTURAL SILICONE");
            m.note((g.gi + 0.1, -(cw.face / 2.0 - 0.3)), "SPACER TAPE");
            m.note((-g.gi + 0.25, 0.0), "SILICONE WEATHER SEAL & BACKER ROD");
        }
        _ => {
            m.note((g.outer, 0.4), "SNAP-ON COVER CAP");
            if cw.glazing != Glazing::Unitized {
                m.note((-g.gi - 0.3, -0.6), "PRESSURE PLATE, SCREWED AT 9\" O.C.");
            }
            m.note((-g.gi - 0.1, 0.15), cw.break_note);
            m.note((g.gi + 0.1, -(g.edge + 0.6)), "EPDM GLAZING GASKETS");
        }
    }
}

fn mullion(m: &mut M, cw: &Cw, g: &G) {
    let ssg = matches!(cw.glazing, Glazing::Ssg2 | Glazing::Ssg4);
    member(m, cw, g, &[-1.0, 1.0], ssg);
    m.brk((g.outer - 1.0, -14.0), (g.back + 1.0, -14.0));
    m.brk((g.outer - 1.0, 14.0), (g.back + 1.0, 14.0));
    let what = if cw.glazing == Glazing::Unitized {
        "SPLIT MULLION"
    } else {
        "MULLION"
    };
    member_notes(m, cw, g, what, -1.0);
    if cw.glazing == Glazing::Unitized {
        m.note(
            (g.front + 0.45, 0.0),
            "INTERLOCKING HALVES, GASKET BETWEEN UNITS",
        );
    }
}

fn horizontal(m: &mut M, cw: &Cw, g: &G) {
    member(m, cw, g, &[-1.0, 1.0], false);
    // The setting blocks under the upper lite.
    m.rect(-g.gi + 0.05, g.edge - 0.25, g.gi - 0.05, g.edge, Medium);
    m.brk((g.outer - 1.0, -14.0), (g.back + 1.0, -14.0));
    m.brk((g.outer - 1.0, 14.0), (g.back + 1.0, 14.0));
    member_notes(m, cw, g, "HORIZONTAL", -1.0);
    m.note((0.0, g.edge - 0.12), "SETTING BLOCKS AT 1/4 POINTS");
    m.note(
        (g.back - 0.5, -cw.face / 2.0 + 0.2),
        "WEEPS AT HORIZONTAL, DRAINED TO EXTERIOR",
    );
}

/// What the curtain wall meets beside or above it, from s `s0` outward (toward +s
/// when `up`, else -s).
fn adjoining(m: &mut M, cw: &Cw, g: &G, s0: f64, dir: f64) {
    let far = s0 + dir * 14.0;
    let (sa, sb) = (s0.min(far), s0.max(far));
    match cw.adjoin {
        Adjoin::Masonry => {
            m.cut_rect(g.outer + 1.0, sa, g.outer + 8.625, sb, Masonry);
            m.note((g.outer + 4.8, s0 + dir * 7.0), "8\" CMU");
        }
        Adjoin::StudWall => {
            let n0 = g.outer;
            m.rect(n0, sa, n0 + 0.5, sb, Medium);
            m.cut_rect(n0 + 0.5, sa, n0 + 2.0, sb, RigidInsulation);
            m.sheet(n0 + 2.0, sa, n0 + 2.625, sb);
            m.comp(
                "stud-6",
                (n0 + 5.625, s0 + dir * 1.0),
                (n0 + 5.625, s0 + dir * 2.0),
            );
            m.gyp(n0 + 8.625, sa, n0 + 9.25, sb);
            m.note((n0 + 0.25, s0 + dir * 8.0), "METAL PANEL ON 1-1/2\" CI");
            m.note((n0 + 2.3, s0 + dir * 10.0), "SHEATHING & AIR/WATER BARRIER");
            m.note((n0 + 5.625, s0 + dir * 1.5), "6\" STEEL STUDS");
        }
    }
    // The perimeter joint, sealed both sides, and the air barrier's transition membrane.
    m.seal("seal-34", (g.outer + 0.2, s0 - dir * 0.375), (-1.0, 0.0));
    m.seal("seal-12", (g.back - 0.25, s0 - dir * 0.375), (1.0, 0.0));
    m.flashing(&[
        (g.front + 0.5, s0 - dir * 0.75),
        (g.front + 0.5, s0 + dir * 0.2),
        (g.outer + 2.2, s0 + dir * 0.2),
        (g.outer + 2.2, s0 + dir * 4.0),
    ]);
    m.note(
        (g.outer + 0.2, s0 - dir * 0.375),
        "3/4\" PERIMETER JOINT, SEALANT & BACKER ROD",
    );
    m.note(
        (g.front + 0.5, s0 + dir * 0.1),
        "SILICONE TRANSITION MEMBRANE TO AIR BARRIER",
    );
}

fn jamb(m: &mut M, cw: &Cw, g: &G) {
    let ssg = matches!(cw.glazing, Glazing::Ssg2 | Glazing::Ssg4);
    member(m, cw, g, &[1.0], ssg);
    adjoining(m, cw, g, -cw.face / 2.0 - 0.75, -1.0);
    m.brk((g.outer - 1.0, 14.0), (g.back + 1.0, 14.0));
    member_notes(m, cw, g, "JAMB", 1.0);
    m.note(
        (g.front + 0.4, -cw.face / 2.0 - 0.4),
        "SHIM, ANCHOR CLIP AT JAMB",
    );
}

fn head(m: &mut M, cw: &Cw, g: &G) {
    member(m, cw, g, &[-1.0], false);
    adjoining(m, cw, g, cw.face / 2.0 + 0.75, 1.0);
    m.brk((g.outer - 1.0, -14.0), (g.back + 1.0, -14.0));
    member_notes(m, cw, g, "HEAD", -1.0);
    m.note(
        (g.back, cw.face / 2.0),
        "HEAD RECEPTOR / ANCHOR, DEFLECTION SLIP",
    );
}

fn sill(m: &mut M, cw: &Cw, g: &G) {
    member(m, cw, g, &[1.0], false);
    let hw = cw.face / 2.0;
    // The sill flashing with its back leg and end dams, on the curb.
    let base = -hw - 0.75;
    m.flashing(&[
        (g.outer - 1.0, base - 0.4),
        (g.outer - 1.0, base),
        (g.back + 0.25, base + 0.2),
        (g.back + 0.25, base + 1.5),
    ]);
    m.cut_rect(
        g.outer - 0.5,
        base - 8.0,
        g.back + 14.0,
        base - 0.0625,
        Concrete,
    );
    m.seal("seal-34", (g.outer + 0.3, -hw - 0.375), (-1.0, 0.0));
    m.seal("seal-12", (g.back - 0.25, -hw - 0.375), (1.0, 0.0));
    m.region_rect(g.front + 0.5, base, g.back - 0.5, -hw, Sand);
    m.brk((g.outer - 1.0, 14.0), (g.back + 1.0, 14.0));
    m.brk((g.back + 14.0, base - 8.0), (g.back + 14.0, base));
    member_notes(m, cw, g, "SILL", 1.0);
    let sill = if cw.sill_note.is_empty() {
        "SILL FLASHING W/ END DAMS, SLOPED TO DRAIN"
    } else {
        cw.sill_note
    };
    m.note(((g.outer + g.back) / 2.0, base + 0.1), sill);
    m.note((g.front + 1.0, base + 0.4), "SHIMS & SILL ANCHORS");
    m.note((g.back + 6.0, base - 4.0), "CONC. CURB / SLAB");
    m.note(
        (g.outer + 0.3, -hw - 0.375),
        "SEALANT & BACKER ROD; WEEPS OPEN",
    );
}

fn slab_edge(m: &mut M, cw: &Cw, g: &G) {
    // The horizontal at the floor line: vision glass above, spandrel below.
    member(m, cw, g, &[-1.0, 1.0], false);
    let hw = cw.face / 2.0;
    // Spandrel: back pan and insulation behind the spandrel glass.
    let pan = g.back - 0.0625;
    m.flashing(&[(pan, -hw - 0.25), (pan, -14.0)]);
    m.region_rect(
        g.front + 0.25,
        -14.0,
        pan - 0.1,
        -hw - 0.25,
        RigidInsulation,
    );
    // The slab edge, the safing in the gap and its smoke seal.
    let slab_n = g.back + cw.safing_gap;
    let top = -hw - 3.0;
    m.cut_rect(slab_n, top - 8.0, slab_n + 14.0, top, Concrete);
    m.region_rect(g.back, top - 4.0, slab_n, top, Sand);
    m.rect(g.back, top - 4.0, slab_n, top, Medium);
    m.region_rect(g.back, top, slab_n, top + 0.25, Solid);
    // The anchor: an angle on the slab, bolted to the mullion.
    m.comp("l4x4", (slab_n + 0.5, top), (slab_n + 1.5, top));
    m.line(&[(g.back, top + 2.5), (slab_n + 0.5, top + 2.5)], Beyond);
    m.brk((g.outer - 1.0, 14.0), (g.back + 1.0, 14.0));
    m.brk((g.outer - 1.0, -14.0), (g.back + 1.0, -14.0));
    m.brk((slab_n + 14.0, top - 8.0), (slab_n + 14.0, top));
    member_notes(m, cw, g, "HORIZONTAL AT FLOOR LINE", 1.0);
    m.note(
        (pan, -10.0),
        "MIN. 20 GA GALV. STEEL BACK PAN, SEALED TO FRAMING",
    );
    m.note(((g.front + pan) / 2.0, -7.0), "SPANDREL INSULATION");
    m.note(
        ((g.back + slab_n) / 2.0, top - 2.0),
        &format!(
            "MINERAL WOOL SAFING, {} GAP, 4\" MIN. DEEP, 33% COMPRESSED (ASTM E2307)",
            fr(cw.safing_gap)
        ),
    );
    m.note(
        ((g.back + slab_n) / 2.0, top + 0.12),
        "SMOKE SEALANT, 1\" OVERSPRAY ONTO SLAB & FRAME",
    );
    let anchor = if cw.anchor_note.is_empty() {
        "DEAD-LOAD & WIND ANCHOR ON EMBED / SLAB EDGE"
    } else {
        cw.anchor_note
    };
    m.note((slab_n + 1.0, top + 0.5), anchor);
    m.note((slab_n + 8.0, top - 4.0), "CONC. SLAB");
    // The stiffener behind the spandrel insulation at the floor line.
    m.flashing(&[(pan - 3.0, top - 2.0), (pan, top - 2.0)]);
    m.flashing(&[(pan - 0.06, top - 3.5), (pan - 0.06, top - 0.5)]);
    m.note(
        (pan - 1.5, top - 2.0),
        "3\" T-BAR STIFFENER, MIN. 20 GA, AT FLOOR LINE",
    );
    m.note((0.0, -11.0), "SPANDREL GLASS (OPAQUE) OR SHADOW BOX");
}

fn stack(m: &mut M, cw: &Cw, g: &G) {
    let hw = cw.face / 2.0;
    let gap = cw.movement.max(0.5);
    // The lower unit's head (female) and the upper unit's sill (male), apart by the joint.
    for (k, s0, s1) in [(-1.0, -hw * 2.0, -gap / 2.0), (1.0, gap / 2.0, hw * 2.0)] {
        m.rect(g.front, s0, g.back, s1, Wide);
        m.rect(g.front + 0.1, s0 + 0.1, g.back - 0.1, s1 - 0.1, Thin);
        let e = k * (2.0 * hw - 0.5);
        m.line(&[(-g.gi, e), (-g.gi, k * 14.0)], Medium);
        m.line(&[(g.gi, e), (g.gi, k * 14.0)], Medium);
        m.region_rect(-g.gi, e, g.gi, e + k * 0.4, Gray);
        m.region_rect(g.gi, e, g.front, e + k * 0.6, Solid);
        let o = -(g.gi + 0.4375 + cw.cap.max(0.5));
        let (sa, sb) = (s0.min(s1), s0.max(s1));
        m.rect(o, sa, -g.gi - 0.2, sb, Wide);
    }
    // The interlocking legs across the joint, and its gaskets.
    m.line(
        &[(g.back - 1.0, gap / 2.0), (g.back - 1.0, -gap / 2.0 - 0.75)],
        Wide,
    );
    m.line(
        &[
            (g.front + 1.0, -gap / 2.0),
            (g.front + 1.0, gap / 2.0 + 0.75),
        ],
        Wide,
    );
    m.region_rect(g.front + 1.5, -gap / 2.0, g.front + 1.9, gap / 2.0, Solid);
    m.region_rect(g.back - 1.9, -gap / 2.0, g.back - 1.5, gap / 2.0, Solid);
    m.brk((g.outer - 1.0, 14.0), (g.back + 1.0, 14.0));
    m.brk((g.outer - 1.0, -14.0), (g.back + 1.0, -14.0));
    m.note(
        ((g.front + g.back) / 2.0, hw),
        &format!("{} UPPER UNIT SILL (MALE)", frame_name(cw)),
    );
    m.note(((g.front + g.back) / 2.0, -hw), "LOWER UNIT HEAD (FEMALE)");
    m.note(
        (g.front + 1.7, 0.0),
        &format!("STACK JOINT, {} INTERSTORY MOVEMENT, DUAL GASKETS", fr(gap)),
    );
    m.note((-g.gi, -8.0), &glass_name(cw));
    m.note((-g.gi, 8.0), "FACTORY-GLAZED UNIT ABOVE");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_system_has_its_details_and_cites_its_source() {
        let items = items();
        let systems = super::super::curtain_catalog::SYSTEMS;
        assert!(systems.len() >= 10, "{}", systems.len());
        for cw in systems {
            for part in ["head", "sill", "jamb", "mullion", "horizontal", "slab-edge"] {
                assert!(
                    items
                        .iter()
                        .any(|(i, _)| i.id == format!("{}-{part}", cw.id)),
                    "{} has no {part}",
                    cw.id
                );
            }
            if cw.glazing == Glazing::Unitized {
                assert!(items
                    .iter()
                    .any(|(i, _)| i.id == format!("{}-stack-joint", cw.id)));
            }
            if !cw.maker.is_empty() {
                assert!(cw.url.starts_with("https://"), "{}", cw.id);
            }
            assert!(cw.category.starts_with("Curtain Wall - "), "{}", cw.id);
        }
    }

    #[test]
    fn a_mullion_is_its_face_wide_and_its_depth_deep() {
        // In plan the mullion's back member spans its face width across the wall, and the
        // cap's face to the back of the mullion is the system's depth.
        let cw = super::super::curtain_catalog::SYSTEMS
            .iter()
            .find(|c| c.glazing == Glazing::Captured)
            .unwrap();
        let g = G::of(cw);
        assert!((g.back - g.outer - cw.depth).abs() < 1e-9);
        let d = draw(cw, Part::Mullion).finish(2, super::super::super::TEXT_SIZE);
        let w = d
            .lines
            .iter()
            .filter(|l| l.style == Wide && l.closed)
            .map(|l| {
                let xs: Vec<f64> = l.pts.iter().map(|p| p.x).collect();
                xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                    - xs.iter().cloned().fold(f64::INFINITY, f64::min)
            })
            .fold(0.0, f64::max);
        assert!((w - cw.face * crate::units::MM_PER_IN).abs() < 1.0, "{w}");
    }
}
