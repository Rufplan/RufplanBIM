//! Drafting views and the typical detail library (ADR-069).
//!
//! A drafting view is Revit's 2D-only view: detail lines, filled regions and text at a
//! scale, with no model in it. The library draws typical construction details (footings,
//! wall bases, window heads, eaves, stairs…) each at the scale such a detail is usually
//! drawn at; inserting one makes a drafting view of it, every line and region its own
//! element to edit.

mod library;

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, ViewKind};
use crate::lines::LineStyle;
use crate::sketch::SketchCurve;
use crate::units::MM_PER_IN;

/// A filled region's pattern (Revit's drafting patterns), drawn at a paper size so it reads
/// the same at any scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FillPattern {
    Solid,
    Gray,
    #[default]
    Diagonal,
    CrossHatch,
    Concrete,
    Earth,
    Gravel,
    Sand,
    Masonry,
    RigidInsulation,
    Wood,
    Steel,
}

impl FillPattern {
    pub const ALL: [FillPattern; 12] = [
        Self::Solid,
        Self::Gray,
        Self::Diagonal,
        Self::CrossHatch,
        Self::Concrete,
        Self::Earth,
        Self::Gravel,
        Self::Sand,
        Self::Masonry,
        Self::RigidInsulation,
        Self::Wood,
        Self::Steel,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid Fill",
            Self::Gray => "Solid Gray",
            Self::Diagonal => "Diagonal Up",
            Self::CrossHatch => "Crosshatch",
            Self::Concrete => "Concrete",
            Self::Earth => "Earth",
            Self::Gravel => "Gravel",
            Self::Sand => "Sand / Gypsum",
            Self::Masonry => "Masonry - Brick",
            Self::RigidInsulation => "Rigid Insulation",
            Self::Wood => "Wood - Finish",
            Self::Steel => "Steel",
        }
    }
}

/// A detail in the library.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DetailInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    /// Drawing scale denominator: the scale this kind of detail is usually drawn at.
    pub scale: u32,
    pub scale_label: String,
    pub description: String,
}

/// A line of a detail, in model mm.
#[derive(Debug, Clone, PartialEq)]
pub struct DLine {
    pub pts: Vec<Pt>,
    pub closed: bool,
    pub style: LineStyle,
}

/// A note with its leader: the text starts at `at`, the leader runs to `to`.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub at: Pt,
    pub to: Pt,
    pub text: String,
}

/// A detail's drawing (model mm), before it becomes elements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drawing {
    pub lines: Vec<DLine>,
    pub regions: Vec<(Vec<Pt>, FillPattern)>,
    pub notes: Vec<Note>,
}

/// Draws in inches (x right, y up), as details are dimensioned.
pub(crate) struct D {
    pub d: Drawing,
    /// Notes as (target, text), placed beside the drawing once it's done.
    notes: Vec<((f64, f64), String)>,
}

pub(crate) fn p(x: f64, y: f64) -> Pt {
    Pt::new(x * MM_PER_IN, y * MM_PER_IN)
}

impl D {
    pub fn new() -> Self {
        Self {
            d: Drawing::default(),
            notes: vec![],
        }
    }
    pub fn line(&mut self, pts: &[(f64, f64)], style: LineStyle) {
        self.d.lines.push(DLine {
            pts: pts.iter().map(|(x, y)| p(*x, *y)).collect(),
            closed: false,
            style,
        });
    }
    pub fn poly(&mut self, pts: &[(f64, f64)], style: LineStyle) {
        self.d.lines.push(DLine {
            pts: pts.iter().map(|(x, y)| p(*x, *y)).collect(),
            closed: true,
            style,
        });
    }
    pub fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, style: LineStyle) {
        self.poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], style);
    }
    pub fn region(&mut self, pts: &[(f64, f64)], pattern: FillPattern) {
        self.d
            .regions
            .push((pts.iter().map(|(x, y)| p(*x, *y)).collect(), pattern));
    }
    pub fn region_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, pattern: FillPattern) {
        self.region(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], pattern);
    }
    /// Cut material: its pattern inside a wide outline.
    pub fn cut(&mut self, pts: &[(f64, f64)], pattern: FillPattern) {
        self.region(pts, pattern);
        self.poly(pts, LineStyle::Wide);
    }
    pub fn cut_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, pattern: FillPattern) {
        self.cut(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], pattern);
    }
    /// Lumber cut across its length: the outline with an X (Revit's wood blocking).
    pub fn lumber(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.rect(x0, y0, x1, y1, LineStyle::Medium);
        self.line(&[(x0, y0), (x1, y1)], LineStyle::Thin);
        self.line(&[(x0, y1), (x1, y0)], LineStyle::Thin);
    }
    /// Lumber seen along its length (a stud or joist in elevation).
    pub fn board(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.rect(x0, y0, x1, y1, LineStyle::Medium);
    }
    /// Sheathing or plywood cut: a thin outline with a single diagonal line inside.
    pub fn sheet(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.rect(x0, y0, x1, y1, LineStyle::Medium);
    }
    /// Gypsum board cut: outline with a sand stipple.
    pub fn gyp(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.region_rect(x0, y0, x1, y1, FillPattern::Sand);
        self.rect(x0, y0, x1, y1, LineStyle::Medium);
    }
    /// Batt insulation between `a` and `b`, `w` thick: Revit's zigzag.
    pub fn batt(&mut self, a: (f64, f64), b: (f64, f64), w: f64) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy, ux);
        let step = w / 2.0;
        let n = (len / step).floor().max(1.0) as usize;
        let step = len / n as f64;
        let pts: Vec<(f64, f64)> = (0..=n)
            .map(|i| {
                let t = i as f64 * step;
                let s = if i % 2 == 0 { -w / 2.0 } else { w / 2.0 };
                (a.0 + ux * t + nx * s, a.1 + uy * t + ny * s)
            })
            .collect();
        self.line(&pts, LineStyle::Thin);
    }
    /// A break line from `a` to `b`, with its zig in the middle.
    pub fn brk(&mut self, a: (f64, f64), b: (f64, f64)) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy, ux);
        let (mx, my) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let z = (len * 0.08).clamp(0.5, 2.0);
        self.line(
            &[
                (a.0 - ux * z * 0.5, a.1 - uy * z * 0.5),
                (mx - ux * z, my - uy * z),
                (mx - ux * z * 0.3 + nx * z, my - uy * z * 0.3 + ny * z),
                (mx + ux * z * 0.3 - nx * z, my + uy * z * 0.3 - ny * z),
                (mx + ux * z, my + uy * z),
                (b.0 + ux * z * 0.5, b.1 + uy * z * 0.5),
            ],
            LineStyle::Thin,
        );
    }
    /// Earth below grade, with the grade line.
    pub fn earth(&mut self, pts: &[(f64, f64)]) {
        self.region(pts, FillPattern::Earth);
    }
    /// A reinforcing bar cut: a small solid dot.
    pub fn rebar(&mut self, x: f64, y: f64, dia: f64) {
        let r = dia / 2.0;
        let ring: Vec<(f64, f64)> = (0..10)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 10.0;
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        self.region(&ring, FillPattern::Solid);
    }
    /// A round thing cut (backer rod, pipe, bolt): its circle.
    pub fn circle(&mut self, x: f64, y: f64, r: f64, style: LineStyle) {
        let ring: Vec<(f64, f64)> = (0..16)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 16.0;
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        self.poly(&ring, style);
    }
    /// Sheet metal flashing along `pts`: a heavy thin line.
    pub fn flashing(&mut self, pts: &[(f64, f64)]) {
        self.line(pts, LineStyle::Wide);
    }
    pub fn note(&mut self, to: (f64, f64), text: &str) {
        self.notes.push((to, text.to_owned()));
    }

    /// The drawing with its notes set out in columns either side of it, text `size` paper
    /// mm at 1:`scale`, leaders to their targets.
    pub fn finish(mut self, scale: u32, size: f64) -> Drawing {
        let pts: Vec<Pt> = self
            .d
            .lines
            .iter()
            .flat_map(|l| l.pts.iter().copied())
            .chain(self.d.regions.iter().flat_map(|r| r.0.iter().copied()))
            .collect();
        let Some((lo, hi)) = studio_geom::bounds_of(&pts) else {
            return self.d;
        };
        let s = f64::from(scale);
        let (h, gap, row) = (size * s, 10.0 * s, size * s * 2.0);
        let mid = (lo.x + hi.x) / 2.0;
        let char_w = size * s * 0.62;
        let mut sides: [Vec<(Pt, String)>; 2] = [vec![], vec![]];
        for ((x, y), text) in self.notes.drain(..) {
            let to = p(x, y);
            sides[usize::from(to.x >= mid)].push((to, text));
        }
        for (side, notes) in sides.iter_mut().enumerate() {
            notes.sort_by(|a, b| b.0.y.total_cmp(&a.0.y));
            let mut last = f64::INFINITY;
            for (to, text) in notes.iter() {
                let y = to.y.min(last - row);
                last = y;
                let w = text.chars().count() as f64 * char_w;
                let (at, end) = if side == 1 {
                    let x = hi.x + gap;
                    (Pt::new(x, y - h / 2.0), Pt::new(x - s * 1.5, y))
                } else {
                    let x = lo.x - gap - w;
                    (Pt::new(x, y - h / 2.0), Pt::new(lo.x - gap + s * 1.5, y))
                };
                // The leader, with a small solid arrowhead at the target.
                self.d.lines.push(DLine {
                    pts: vec![end, *to],
                    closed: false,
                    style: LineStyle::Thin,
                });
                let dir = to.sub(end).norm();
                let (len, half) = (s * 1.8, s * 0.45);
                let base = to.sub(dir.scale(len));
                let n = dir.perp();
                self.d.regions.push((
                    vec![*to, base.add(n.scale(half)), base.sub(n.scale(half))],
                    FillPattern::Solid,
                ));
                self.d.notes.push(Note {
                    at,
                    to: *to,
                    text: text.clone(),
                });
            }
        }
        self.d
    }
}

/// The library, grouped by category.
pub fn catalog() -> Vec<DetailInfo> {
    library::DETAILS
        .iter()
        .map(|d| DetailInfo {
            id: d.id.into(),
            name: d.name.into(),
            category: d.category.into(),
            scale: d.scale,
            scale_label: crate::ops::scale_label(d.scale),
            description: d.description.into(),
        })
        .collect()
}

/// A detail's drawing (text size in paper mm).
pub fn drawing(id: &str) -> Option<(u32, Drawing)> {
    let d = library::DETAILS.iter().find(|d| d.id == id)?;
    Some((d.scale, (d.draw)().finish(d.scale, TEXT_SIZE)))
}

/// Text height of the library's notes, paper mm (3/32").
pub const TEXT_SIZE: f64 = 2.4;

fn unique_view_name(doc: &Document, base: &str) -> String {
    let taken = |n: &str| {
        doc.of(Category::View)
            .any(|e| matches!(&e.data, ElementData::View { name, .. } if name == n))
    };
    if !taken(base) {
        return base.to_owned();
    }
    (2..)
        .map(|i| format!("{base} ({i})"))
        .find(|n| !taken(n))
        .unwrap_or_default()
}

/// View > Drafting View: a new, empty drafting view.
pub fn create_drafting_view(doc: &mut Document, name: &str, scale: u32) -> CoreResult<ElementId> {
    if !crate::ops::SCALES.iter().any(|(s, _)| *s == scale) {
        return Err(CoreError::Invalid("pick a drawing scale".into()));
    }
    let name = unique_view_name(
        doc,
        if name.trim().is_empty() {
            "Drafting 1"
        } else {
            name.trim()
        },
    );
    doc.transact("Create Drafting View", |tx| {
        Ok(tx.insert(ElementData::view(name, ViewKind::Drafting, scale)))
    })
}

/// Inserts a library detail: a drafting view at its scale, drawn with detail lines, filled
/// regions and text notes, one undo step.
pub fn insert(doc: &mut Document, id: &str) -> CoreResult<ElementId> {
    let info = library::DETAILS
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| CoreError::Invalid("that detail isn't in the library".into()))?;
    let (scale, drawing) = drawing(id).ok_or_else(|| CoreError::Invalid("no detail".into()))?;
    let name = unique_view_name(doc, info.name);
    doc.transact(&format!("Insert Detail {}", info.name), |tx| {
        let view = tx.insert(ElementData::view(name, ViewKind::Drafting, scale));
        for (ring, pattern) in &drawing.regions {
            tx.insert(ElementData::FilledRegion {
                view,
                boundary: vec![ring.clone()],
                pattern: *pattern,
                outline: None,
            });
        }
        for l in &drawing.lines {
            let n = l.pts.len();
            let segs = if l.closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let (a, b) = (l.pts[i], l.pts[(i + 1) % n]);
                if a.dist(b) > 0.01 {
                    tx.insert(ElementData::DetailLine {
                        view,
                        curve: SketchCurve::line(a, b),
                        style: l.style,
                    });
                }
            }
        }
        for note in &drawing.notes {
            tx.insert(ElementData::TextNote {
                view,
                at: note.at,
                text: note.text.clone(),
                size: TEXT_SIZE,
            });
        }
        Ok(view)
    })
}

/// A filled region's boundary as sketch lines (Edit Boundary).
pub fn region_curves(
    doc: &Document,
    id: ElementId,
) -> CoreResult<(ElementId, Vec<SketchCurve>, FillPattern)> {
    match doc.data(id)? {
        ElementData::FilledRegion {
            view,
            boundary,
            pattern,
            ..
        } => Ok((
            *view,
            boundary
                .iter()
                .flat_map(|r| {
                    let n = r.len();
                    (0..n).map(move |i| SketchCurve::line(r[i], r[(i + 1) % n]))
                })
                .collect(),
            *pattern,
        )),
        _ => Err(CoreError::Invalid("select a filled region".into())),
    }
}

/// Finish of a filled region's sketch: a new region in `view`, or `target`'s new boundary.
pub fn finish_region(
    doc: &mut Document,
    view: ElementId,
    target: Option<ElementId>,
    pattern: FillPattern,
    curves: &[SketchCurve],
) -> Result<ElementId, crate::sketch::SketchError> {
    let loops = crate::sketch::loops(curves)?;
    let polys = crate::sketch::polygons(doc, &loops);
    let rings: Vec<Vec<Pt>> = polys
        .iter()
        .flat_map(|p| std::iter::once(p.outer.clone()).chain(p.holes.iter().cloned()))
        .collect();
    let fail = |e: CoreError| crate::sketch::SketchError::new(&e.to_string(), vec![]);
    if polys.is_empty() {
        return Err(crate::sketch::SketchError::new(
            "A loop has no area.",
            vec![],
        ));
    }
    match target {
        Some(id) => doc
            .transact("Edit Filled Region", |tx| {
                tx.modify(id, |d| {
                    if let ElementData::FilledRegion { boundary, .. } = d {
                        *boundary = rings.clone();
                    }
                })?;
                Ok(id)
            })
            .map_err(fail),
        None => create_region(doc, view, rings, pattern).map_err(fail),
    }
}

/// A filled region from a sketch's loops, in `view`.
pub fn create_region(
    doc: &mut Document,
    view: ElementId,
    loops: Vec<Vec<Pt>>,
    pattern: FillPattern,
) -> CoreResult<ElementId> {
    if loops.first().is_none_or(|l| l.len() < 3) {
        return Err(CoreError::Invalid(
            "a filled region needs a closed loop".into(),
        ));
    }
    doc.transact("Create Filled Region", |tx| {
        Ok(tx.insert(ElementData::FilledRegion {
            view,
            boundary: loops,
            pattern,
            outline: Some(LineStyle::Thin),
        }))
    })
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<crate::ops::Property>) {
    use crate::ops::{choice, PropOption};
    let Ok(ElementData::FilledRegion {
        pattern, outline, ..
    }) = doc.data(id)
    else {
        return;
    };
    props.push(choice(
        "pattern",
        "Fill Pattern",
        "Graphics",
        format!("{pattern:?}"),
        FillPattern::ALL
            .iter()
            .map(|f| PropOption {
                id: format!("{f:?}"),
                label: f.label().into(),
            })
            .collect(),
    ));
    let mut styles = vec![PropOption {
        id: String::new(),
        label: "<Invisible lines>".into(),
    }];
    styles.extend(LineStyle::ALL.iter().map(|s| PropOption {
        id: format!("{s:?}"),
        label: s.label().into(),
    }));
    props.push(choice(
        "outline",
        "Line Style",
        "Graphics",
        outline.map(|s| format!("{s:?}")).unwrap_or_default(),
        styles,
    ));
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let mut d = doc.data(id)?.clone();
    let unknown = || CoreError::Invalid(format!("unknown property {key}"));
    let ElementData::FilledRegion {
        pattern, outline, ..
    } = &mut d
    else {
        return Err(unknown());
    };
    match key {
        "pattern" => {
            *pattern = *FillPattern::ALL
                .iter()
                .find(|f| format!("{f:?}") == value)
                .ok_or_else(unknown)?
        }
        "outline" => {
            *outline = if value.is_empty() {
                None
            } else {
                Some(
                    *LineStyle::ALL
                        .iter()
                        .find(|s| format!("{s:?}") == value)
                        .ok_or_else(unknown)?,
                )
            }
        }
        _ => return Err(unknown()),
    }
    doc.transact("Change Filled Region", |tx| tx.set(id, d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    #[test]
    fn every_detail_draws_at_its_typical_scale_with_notes_clear_of_it() {
        let cat = catalog();
        assert!(cat.len() >= 20, "{}", cat.len());
        let mut ids = std::collections::HashSet::new();
        for info in &cat {
            assert!(ids.insert(info.id.clone()), "duplicate {}", info.id);
            assert!(
                ops::SCALES.iter().any(|(s, _)| *s == info.scale),
                "{}",
                info.id
            );
            let (scale, d) = drawing(&info.id).unwrap();
            assert_eq!(scale, info.scale);
            assert!(
                d.lines.len() > 10,
                "{} has {} lines",
                info.id,
                d.lines.len()
            );
            assert!(!d.regions.is_empty(), "{}", info.id);
            assert!(d.notes.len() >= 3, "{}", info.id);
            // Notes sit outside the drawing's own extent and never overlap each other.
            let geo: Vec<Pt> = d
                .lines
                .iter()
                .filter(|l| l.style != LineStyle::Thin || l.pts.len() > 2)
                .flat_map(|l| l.pts.iter().copied())
                .collect();
            let (lo, hi) = studio_geom::bounds_of(&geo).unwrap();
            let row = TEXT_SIZE * f64::from(scale) * 1.9;
            for (i, n) in d.notes.iter().enumerate() {
                assert!(n.at.x > hi.x || n.at.x < lo.x, "{}: {}", info.id, n.text);
                for m in &d.notes[i + 1..] {
                    let same_side = (n.at.x > hi.x) == (m.at.x > hi.x);
                    assert!(!same_side || (n.at.y - m.at.y).abs() >= row, "{}", info.id);
                }
            }
        }
    }

    #[test]
    fn inserting_a_detail_makes_a_drafting_view_of_elements_in_one_step() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let depth = doc.undo_depth();
        let v = insert(&mut doc, "window-head").unwrap();
        assert_eq!(doc.undo_depth(), depth + 1);
        let ElementData::View {
            kind, scale, name, ..
        } = doc.data(v).unwrap()
        else {
            panic!()
        };
        assert_eq!(*kind, ViewKind::Drafting);
        assert_eq!(*scale, 4, "window heads are drawn at 3\" = 1'-0\"");
        assert_eq!(name, "Window Head - Wood Frame");
        let owned = |cat: Category| doc.of(cat).filter(|e| e.data.refs().contains(&v)).count();
        assert!(owned(Category::DetailLine) > 20);
        assert!(owned(Category::FilledRegion) > 2);
        assert!(owned(Category::TextNote) >= 3);
        // A second copy gets its own name.
        let v2 = insert(&mut doc, "window-head").unwrap();
        assert_eq!(doc.data(v2).unwrap().name(), "Window Head - Wood Frame (2)");
        assert!(insert(&mut doc, "nope").is_err());
        // An empty drafting view, and a filled region's properties.
        let d = create_drafting_view(&mut doc, "", 8).unwrap();
        assert_eq!(doc.data(d).unwrap().name(), "Drafting 1");
        let r = create_region(
            &mut doc,
            d,
            vec![vec![
                Pt::new(0.0, 0.0),
                Pt::new(100.0, 0.0),
                Pt::new(0.0, 100.0),
            ]],
            FillPattern::Concrete,
        )
        .unwrap();
        ops::set_property(&mut doc, r, "pattern", "Earth", 0).unwrap();
        ops::set_property(&mut doc, r, "outline", "", 0).unwrap();
        assert!(matches!(
            doc.data(r).unwrap(),
            ElementData::FilledRegion {
                pattern: FillPattern::Earth,
                outline: None,
                ..
            }
        ));
    }
}
