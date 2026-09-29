//! Detail components (ADR-071), after Revit's Detail Items library: 2D families placed in a
//! view. Line-based ones follow two clicks (break lines, sheathing, gypsum, insulation,
//! flashing, anchor bolts, repeating brick and CMU); point-based ones go at a click and
//! turn (cut lumber, headers, backer rod, rebar, steel shapes, metal studs).
//!
//! A component's geometry is drawn in its own frame, in inches: u along it (from its start
//! toward its end), v to its left (to its right when flipped).

use serde::Serialize;
use studio_geom::Pt;
use ts_rs::TS;

use super::DLine;
use super::FillPattern::{self, CrossHatch, Masking, Masonry, RigidInsulation, Sand, Solid, Steel};
use crate::lines::LineStyle::{self, Medium, Thin, Wide};
use crate::units::MM_PER_IN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum Family {
    BreakLine,
    Plywood,
    Gypsum,
    CutLumber,
    LumberSide,
    Header,
    BattInsulation,
    RigidInsulation,
    BrickCoursing,
    CmuCoursing,
    Flashing,
    AnchorBolt,
    Sealant,
    Rebar,
    WideFlange,
    MetalStud,
    SteelAngle,
}

impl Family {
    /// Revit's family name.
    pub fn label(self) -> &'static str {
        match self {
            Family::BreakLine => "Break Line",
            Family::Plywood => "Plywood-Section",
            Family::Gypsum => "Gypsum Wallboard-Section",
            Family::CutLumber => "Nominal Cut Lumber-Section",
            Family::LumberSide => "Nominal Lumber-Side",
            Family::Header => "Wood Header-Section",
            Family::BattInsulation => "Insulation-Batt",
            Family::RigidInsulation => "Rigid Insulation-Section",
            Family::BrickCoursing => "Brick-Standard-Section (Repeating)",
            Family::CmuCoursing => "CMU-Section (Repeating)",
            Family::Flashing => "Metal Flashing",
            Family::AnchorBolt => "Anchor Bolt",
            Family::Sealant => "Sealant-Backer Rod",
            Family::Rebar => "Rebar Bar-Section",
            Family::WideFlange => "W-Wide Flange-Section",
            Family::MetalStud => "Metal Stud-Section",
            Family::SteelAngle => "L-Angle-Section",
        }
    }

    /// Placed by two clicks (true) or one (false).
    pub fn line_based(self) -> bool {
        matches!(
            self,
            Family::BreakLine
                | Family::Plywood
                | Family::Gypsum
                | Family::LumberSide
                | Family::BattInsulation
                | Family::RigidInsulation
                | Family::BrickCoursing
                | Family::CmuCoursing
                | Family::Flashing
                | Family::AnchorBolt
        )
    }
}

/// A component type: its family and sizes (inches).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentType {
    pub key: &'static str,
    pub family: Family,
    pub name: &'static str,
    pub a: f64,
    pub b: f64,
}

const fn t(key: &'static str, family: Family, name: &'static str, a: f64, b: f64) -> ComponentType {
    ComponentType {
        key,
        family,
        name,
        a,
        b,
    }
}

use Family as F;

/// Every type. \`a\`/\`b\`: width and depth of lumber, thickness of sheets, width of insulation,
/// the mask's depth of a break line, a bolt's diameter and hook, a shape's depth and width.
pub static TYPES: &[ComponentType] = &[
    t("break", F::BreakLine, "Break Line", 12.0, 0.0),
    t("break-small", F::BreakLine, "Break Line - Small", 3.0, 0.0),
    t("ply-716", F::Plywood, "7/16\" OSB", 0.4375, 0.0),
    t("ply-12", F::Plywood, "1/2\" Plywood", 0.5, 0.0),
    t("ply-58", F::Plywood, "5/8\" Plywood", 0.625, 0.0),
    t("ply-34", F::Plywood, "3/4\" Plywood", 0.75, 0.0),
    t("gyp-12", F::Gypsum, "1/2\" Gypsum Board", 0.5, 0.0),
    t("gyp-58", F::Gypsum, "5/8\" Type X Gypsum Board", 0.625, 0.0),
    t("lum-2x4", F::CutLumber, "2x4", 1.5, 3.5),
    t("lum-2x6", F::CutLumber, "2x6", 1.5, 5.5),
    t("lum-2x8", F::CutLumber, "2x8", 1.5, 7.25),
    t("lum-2x10", F::CutLumber, "2x10", 1.5, 9.25),
    t("lum-2x12", F::CutLumber, "2x12", 1.5, 11.25),
    t("lum-4x4", F::CutLumber, "4x4", 3.5, 3.5),
    t("lum-4x6", F::CutLumber, "4x6", 3.5, 5.5),
    t("lum-6x6", F::CutLumber, "6x6", 5.5, 5.5),
    t("side-2x4", F::LumberSide, "2x4", 3.5, 0.0),
    t("side-2x6", F::LumberSide, "2x6", 5.5, 0.0),
    t("side-2x8", F::LumberSide, "2x8", 7.25, 0.0),
    t("side-2x10", F::LumberSide, "2x10", 9.25, 0.0),
    t("side-2x12", F::LumberSide, "2x12", 11.25, 0.0),
    t("hdr-2x8", F::Header, "(2) 2x8 w/ 1/2\" Spacer", 7.25, 2.0),
    t("hdr-2x10", F::Header, "(2) 2x10 w/ 1/2\" Spacer", 9.25, 2.0),
    t(
        "hdr-2x12",
        F::Header,
        "(2) 2x12 w/ 1/2\" Spacer",
        11.25,
        2.0,
    ),
    t("hdr-3-2x10", F::Header, "(3) 2x10", 9.25, 3.0),
    t("batt-35", F::BattInsulation, "3 1/2\" Batt", 3.5, 0.0),
    t("batt-55", F::BattInsulation, "5 1/2\" Batt", 5.5, 0.0),
    t("batt-925", F::BattInsulation, "9 1/4\" Batt", 9.25, 0.0),
    t("rigid-1", F::RigidInsulation, "1\" Rigid", 1.0, 0.0),
    t("rigid-2", F::RigidInsulation, "2\" Rigid", 2.0, 0.0),
    t("rigid-3", F::RigidInsulation, "3\" Rigid", 3.0, 0.0),
    t(
        "brick-mod",
        F::BrickCoursing,
        "Modular (3 5/8\")",
        3.625,
        2.667,
    ),
    t("brick-queen", F::BrickCoursing, "Queen (3\")", 3.0, 2.667),
    t("cmu-8", F::CmuCoursing, "8\" CMU", 7.625, 8.0),
    t("cmu-6", F::CmuCoursing, "6\" CMU", 5.625, 8.0),
    t("cmu-12", F::CmuCoursing, "12\" CMU", 11.625, 8.0),
    t(
        "flash",
        F::Flashing,
        "Sheet Metal Flashing w/ Hem",
        0.5,
        0.0,
    ),
    t("ab-12", F::AnchorBolt, "1/2\" Anchor Bolt", 0.5, 3.0),
    t("ab-58", F::AnchorBolt, "5/8\" Anchor Bolt", 0.625, 3.0),
    t("seal-14", F::Sealant, "1/4\" Joint", 0.25, 0.0),
    t("seal-12", F::Sealant, "1/2\" Joint", 0.5, 0.0),
    t("seal-34", F::Sealant, "3/4\" Joint", 0.75, 0.0),
    t("bar-4", F::Rebar, "#4", 0.5, 0.0),
    t("bar-5", F::Rebar, "#5", 0.625, 0.0),
    t("bar-6", F::Rebar, "#6", 0.75, 0.0),
    t("w8x31", F::WideFlange, "W8x31", 8.0, 8.0),
    t("w10x33", F::WideFlange, "W10x33", 9.73, 7.96),
    t("w12x26", F::WideFlange, "W12x26", 12.2, 6.49),
    t("stud-358", F::MetalStud, "3 5/8\" Metal Stud", 3.625, 1.625),
    t("stud-6", F::MetalStud, "6\" Metal Stud", 6.0, 1.625),
    t("l3x3", F::SteelAngle, "L3x3x1/4", 3.0, 0.25),
    t("l4x4", F::SteelAngle, "L4x4x3/8", 4.0, 0.375),
];

pub fn type_of(key: &str) -> Option<&'static ComponentType> {
    TYPES.iter().find(|t| t.key == key)
}

/// A component type for the UI's type selector.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComponentTypeInfo {
    pub key: String,
    pub family: Family,
    pub family_label: String,
    pub name: String,
    pub line_based: bool,
}

pub fn catalog() -> Vec<ComponentTypeInfo> {
    TYPES
        .iter()
        .map(|t| ComponentTypeInfo {
            key: t.key.into(),
            family: t.family,
            family_label: t.family.label().into(),
            name: t.name.into(),
            line_based: t.family.line_based(),
        })
        .collect()
}

/// A component's drawing: lines, and regions (each loops, the first the outline).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parts {
    pub lines: Vec<DLine>,
    pub regions: Vec<(Vec<Vec<Pt>>, FillPattern)>,
}

struct Frame {
    o: Pt,
    u: Pt,
    v: Pt,
}

impl Frame {
    fn at(&self, x: f64, y: f64) -> Pt {
        self.o
            .add(self.u.scale(x * MM_PER_IN))
            .add(self.v.scale(y * MM_PER_IN))
    }
}

struct Pen<'a> {
    f: &'a Frame,
    out: Parts,
}

impl Pen<'_> {
    fn line(&mut self, pts: &[(f64, f64)], style: LineStyle) {
        self.out.lines.push(DLine {
            pts: pts.iter().map(|(x, y)| self.f.at(*x, *y)).collect(),
            closed: false,
            style,
        });
    }
    fn poly(&mut self, pts: &[(f64, f64)], style: LineStyle) {
        self.out.lines.push(DLine {
            pts: pts.iter().map(|(x, y)| self.f.at(*x, *y)).collect(),
            closed: true,
            style,
        });
    }
    fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, style: LineStyle) {
        self.poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], style);
    }
    fn region(&mut self, loops: &[&[(f64, f64)]], pattern: FillPattern) {
        let rings = loops
            .iter()
            .map(|l| l.iter().map(|(x, y)| self.f.at(*x, *y)).collect())
            .collect();
        self.out.regions.push((rings, pattern));
    }
    fn region_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, pattern: FillPattern) {
        self.region(&[&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]], pattern);
    }
    fn circle(&mut self, x: f64, y: f64, r: f64) -> Vec<(f64, f64)> {
        (0..20)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 20.0;
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect()
    }
}

/// The component's geometry in the view (model mm). Line-based: from \`start\` to \`end\`;
/// point-based: at \`start\`, turned toward \`end\`. \`flip\` mirrors it across its line.
pub fn parts(t: &ComponentType, start: Pt, end: Pt, flip: bool) -> Parts {
    let d = end.sub(start);
    let len_mm = d.len();
    let u = if len_mm > 1e-6 {
        d.scale(1.0 / len_mm)
    } else {
        Pt::new(1.0, 0.0)
    };
    let v = if flip { u.perp().scale(-1.0) } else { u.perp() };
    let f = Frame { o: start, u, v };
    let l = len_mm / MM_PER_IN;
    let mut p = Pen {
        f: &f,
        out: Parts::default(),
    };
    let (a, b) = (t.a, t.b);
    match t.family {
        F::BreakLine => {
            // Masks a band `a` deep on its left, under the zigzag in its middle.
            let m = l / 2.0;
            let z = (l * 0.05).clamp(0.5, 3.0);
            p.region_rect(0.0, 0.0, l, a, Masking);
            p.line(
                &[
                    (-0.5, 0.0),
                    (m - z, 0.0),
                    (m - z * 0.35, z * 1.6),
                    (m + z * 0.35, -z * 1.6),
                    (m + z, 0.0),
                    (l + 0.5, 0.0),
                ],
                Thin,
            );
        }
        F::Plywood => {
            p.region_rect(0.0, 0.0, l, a, FillPattern::Masking);
            p.rect(0.0, 0.0, l, a, Medium);
            if a >= 0.45 {
                for k in [1.0, 2.0] {
                    p.line(&[(0.0, a * k / 3.0), (l, a * k / 3.0)], Thin);
                }
            }
        }
        F::Gypsum => {
            p.region_rect(0.0, 0.0, l, a, Sand);
            p.rect(0.0, 0.0, l, a, Medium);
        }
        F::RigidInsulation => {
            p.region_rect(0.0, 0.0, l, a, RigidInsulation);
            p.rect(0.0, 0.0, l, a, Medium);
        }
        F::LumberSide => {
            p.region_rect(0.0, 0.0, l, a, Masking);
            p.rect(0.0, 0.0, l, a, Medium);
        }
        F::CutLumber => {
            let (hw, hd) = (a / 2.0, b / 2.0);
            p.region_rect(-hw, -hd, hw, hd, Masking);
            p.rect(-hw, -hd, hw, hd, Medium);
            p.line(&[(-hw, -hd), (hw, hd)], Thin);
            p.line(&[(-hw, hd), (hw, -hd)], Thin);
        }
        F::Header => {
            // Plies across u, centered; a 1/2" spacer between two plies.
            let n = b as usize;
            let spacer = if n == 2 { 0.5 } else { 0.0 };
            let total = n as f64 * 1.5 + spacer;
            let (mut x, hd) = (-total / 2.0, a / 2.0);
            for i in 0..n {
                p.region_rect(x, -hd, x + 1.5, hd, Masking);
                p.rect(x, -hd, x + 1.5, hd, Medium);
                p.line(&[(x, -hd), (x + 1.5, hd)], Thin);
                p.line(&[(x, hd), (x + 1.5, -hd)], Thin);
                x += 1.5;
                if i == 0 && spacer > 0.0 {
                    p.region_rect(x, -hd, x + spacer, hd, Masking);
                    p.rect(x, -hd, x + spacer, hd, Thin);
                    x += spacer;
                }
            }
        }
        F::BattInsulation => {
            // Revit's batt: a line of loops (a prolate cycloid), `a` wide, centered.
            let r = a / 2.0;
            let step = a * 0.5;
            let k = step / std::f64::consts::TAU;
            let n = ((l / step) * 16.0).ceil().max(16.0) as usize;
            let pts: Vec<(f64, f64)> = (0..=n)
                .map(|i| {
                    let th = i as f64 / n as f64 * (l / k);
                    (k * th - r * 0.6 * th.sin(), -r * th.cos())
                })
                .filter(|(x, _)| *x >= -0.01 && *x <= l + 0.01)
                .collect();
            p.line(&pts, Thin);
        }
        F::BrickCoursing | F::CmuCoursing => {
            // Units along the line, one course each, with a 3/8" joint.
            let course = b;
            let unit = course - 0.375;
            let mut x = 0.0;
            while x + 0.2 < l {
                let x1 = (x + unit).min(l);
                if t.family == F::BrickCoursing {
                    p.region_rect(x, 0.0, x1, a, Masonry);
                    p.rect(x, 0.0, x1, a, Medium);
                } else {
                    // Hollow units: hatched face shells around the cell.
                    let shell = 1.25;
                    let outer = [(x, 0.0), (x1, 0.0), (x1, a), (x, a)];
                    let (c0, c1) = (x + 1.0, x1 - 1.0);
                    if c1 - c0 > 0.5 {
                        let cell = [(c0, shell), (c1, shell), (c1, a - shell), (c0, a - shell)];
                        p.region(&[&outer, &cell], CrossHatch);
                        p.poly(&cell, Thin);
                    } else {
                        p.region(&[&outer], CrossHatch);
                    }
                    p.poly(&outer, Medium);
                }
                x += course;
            }
        }
        F::Flashing => {
            // Along the line, with a hemmed drip turned down at its end.
            p.line(
                &[(0.0, 0.0), (l, 0.0), (l + a, -a), (l + a * 0.6, -a * 1.3)],
                Wide,
            );
        }
        F::AnchorBolt => {
            // Its shank from the nut down, hooked at the bottom; nut and washer on top.
            let r = a / 2.0;
            let hook = b;
            p.poly(
                &[
                    (0.0, r),
                    (l, r),
                    (l, r + hook),
                    (l - a, r + hook),
                    (l - a, -r),
                    (0.0, -r),
                ],
                Medium,
            );
            p.rect(-0.25, -1.0, 0.0, 1.0, Medium);
            p.rect(-0.25 - a, -a * 0.9, -0.25, a * 0.9, Medium);
        }
        F::Sealant => {
            // A backer rod in the joint with the sealant's concave bead over it.
            let w = a;
            let r = w * 0.6;
            let rod = p.circle(0.0, -r, r);
            p.poly(&rod, Thin);
            let bead = [
                (-w / 2.0, 0.0),
                (-w / 2.0, w * 0.5),
                (0.0, w * 0.25),
                (w / 2.0, w * 0.5),
                (w / 2.0, 0.0),
                (0.0, w * 0.12),
            ];
            p.region(&[&bead], FillPattern::Gray);
            p.poly(&bead, Thin);
        }
        F::Rebar => {
            let c = p.circle(0.0, 0.0, a / 2.0);
            p.region(&[&c], Solid);
        }
        F::WideFlange => {
            // An I: depth `a` along v, flange width `b` along u.
            let (hd, hb) = (a / 2.0, b / 2.0);
            let (tf, tw) = (a.max(8.0) * 0.05, a.max(8.0) * 0.035);
            let shape = [
                (-hb, -hd),
                (hb, -hd),
                (hb, -hd + tf),
                (tw / 2.0, -hd + tf),
                (tw / 2.0, hd - tf),
                (hb, hd - tf),
                (hb, hd),
                (-hb, hd),
                (-hb, hd - tf),
                (-tw / 2.0, hd - tf),
                (-tw / 2.0, -hd + tf),
                (-hb, -hd + tf),
            ];
            p.region(&[&shape], Steel);
            p.poly(&shape, Wide);
        }
        F::MetalStud => {
            // A C: web `a` along v, flanges `b` along u, 1/2" lips.
            let (hd, fl, lip) = (a / 2.0, b, 0.5);
            p.line(
                &[
                    (fl, -hd + lip),
                    (fl, -hd),
                    (0.0, -hd),
                    (0.0, hd),
                    (fl, hd),
                    (fl, hd - lip),
                ],
                Wide,
            );
        }
        F::SteelAngle => {
            // Heel at the insertion point, legs `a` along u and v, `b` thick.
            let shape = [(0.0, 0.0), (a, 0.0), (a, b), (b, b), (b, a), (0.0, a)];
            p.region(&[&shape], Steel);
            p.poly(&shape, Wide);
        }
    }
    p.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_draws_and_line_based_ones_follow_their_line() {
        let (s, e) = (Pt::new(0.0, 0.0), Pt::new(48.0 * MM_PER_IN, 0.0));
        let mut keys = std::collections::HashSet::new();
        for t in TYPES {
            assert!(keys.insert(t.key), "duplicate {}", t.key);
            let p = parts(t, s, e, false);
            assert!(!p.lines.is_empty() || !p.regions.is_empty(), "{}", t.key);
            // Line-based: nothing strays far past the line's ends.
            if t.family.line_based() {
                for q in p.lines.iter().flat_map(|l| &l.pts) {
                    assert!(
                        q.x > -4.0 * MM_PER_IN && q.x < 52.0 * MM_PER_IN,
                        "{} {q:?}",
                        t.key
                    );
                }
            }
        }
        // A 2x6 cut is 1 1/2" by 5 1/2" around its insertion point, turned with its line.
        let lum = type_of("lum-2x6").unwrap();
        let up = parts(lum, s, Pt::new(0.0, 100.0), false);
        let xs: Vec<f64> = up.lines[0].pts.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = up.lines[0].pts.iter().map(|p| p.y).collect();
        let span = |v: &[f64]| {
            v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                - v.iter().cloned().fold(f64::INFINITY, f64::min)
        };
        assert!(
            (span(&ys) - 1.5 * MM_PER_IN).abs() < 1e-6,
            "turned: its width runs along the line"
        );
        assert!((span(&xs) - 5.5 * MM_PER_IN).abs() < 1e-6);
        // Brick: one unit per 2 2/3" course along 48".
        let brick = parts(type_of("brick-mod").unwrap(), s, e, false);
        assert_eq!(brick.regions.len(), 18);
        // CMU units are hollow: each region has its cell as a hole.
        let cmu = parts(type_of("cmu-8").unwrap(), s, e, false);
        assert_eq!(cmu.regions.len(), 6);
        assert!(cmu.regions.iter().all(|(r, _)| r.len() == 2));
        // Flipped, a sheet lies on the line's other side.
        let ply = type_of("ply-12").unwrap();
        let left = parts(ply, s, e, false);
        let right = parts(ply, s, e, true);
        assert!(left.lines[0].pts.iter().all(|q| q.y >= -1e-9));
        assert!(right.lines[0].pts.iter().all(|q| q.y <= 1e-9));
        // The batt spans its width, centered.
        let batt = parts(type_of("batt-55").unwrap(), s, e, false);
        let ys: Vec<f64> = batt.lines[0].pts.iter().map(|p| p.y).collect();
        assert!((span(&ys) - 5.5 * MM_PER_IN).abs() < 0.5);
    }
}
