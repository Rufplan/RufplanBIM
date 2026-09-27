//! Annotation symbols (ADR-048): spot elevations, north arrows, graphic scales and key
//! plans. They are placed like Revit's, and they draw in the style their standard is set to
//! on the Standards tab (a catalog choice; see `standards_catalog`).

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{ElementData, ElementId, ViewKind};
use crate::ops::{len, ro, Property};
use crate::standards::choice;
use crate::units::{format_ft_in, MM_PER_FT};
use studio_geom::Pt;

/// How spot elevations draw ("Spot Elevations" standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotStyle {
    /// A solid triangle; the project elevation (Level 1 = 0'-0").
    Triangle,
    /// A quartered target; relative to Level 1 = 100'-0".
    Target,
    /// A cross; the surveyed elevation (site datum) in decimal feet.
    Cross,
    /// No symbol: "EL." and the project elevation.
    TextOnly,
}

/// How north arrows draw ("North Arrow" standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NorthStyle {
    ProjectAndTrue,
    Circle,
    HalfArrow,
    Compass,
    TrueOnly,
}

/// How graphic scales draw ("Graphic Scale" standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleStyle {
    Alternating,
    Ticks,
    Checkered,
    WithText,
}

/// Where key plans and north arrows go on sheets ("Key Plan & North Arrow" standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPlanStyle {
    /// In the title block, with a north arrow.
    TitleBlock,
    /// Placed on sheets; the title block leaves it out.
    Placed,
    /// Only a north arrow in the title block.
    ArrowOnly,
    None,
}

impl SpotStyle {
    pub const ALL: [SpotStyle; 4] = [Self::Triangle, Self::Target, Self::Cross, Self::TextOnly];
    pub fn of(doc: &Document) -> Self {
        pick(doc, "sym", "Spot Elevations", &Self::ALL)
    }
}

impl NorthStyle {
    pub const ALL: [NorthStyle; 5] = [
        Self::ProjectAndTrue,
        Self::Circle,
        Self::HalfArrow,
        Self::Compass,
        Self::TrueOnly,
    ];
    pub fn of(doc: &Document) -> Self {
        pick(doc, "sym", "North Arrow", &Self::ALL)
    }
}

impl ScaleStyle {
    pub const ALL: [ScaleStyle; 4] = [
        Self::Alternating,
        Self::Ticks,
        Self::Checkered,
        Self::WithText,
    ];
    pub fn of(doc: &Document) -> Self {
        pick(doc, "sym", "Graphic Scale", &Self::ALL)
    }
}

impl KeyPlanStyle {
    pub const ALL: [KeyPlanStyle; 4] =
        [Self::TitleBlock, Self::Placed, Self::ArrowOnly, Self::None];
    pub fn of(doc: &Document) -> Self {
        pick(doc, "sheet", "Key Plan & North Arrow", &Self::ALL)
    }
}

/// The style a standard is set to; the first (the default) when it's open or custom.
fn pick<T: Copy>(doc: &Document, category: &str, item: &str, all: &[T]) -> T {
    choice(doc, category, item)
        .and_then(|i| all.get(i).copied())
        .unwrap_or(all[0])
}

/// Relative elevations put Level 1 at 100'-0".
pub const RELATIVE_DATUM: f64 = 100.0 * MM_PER_FT;

/// A spot elevation's text for project height `z` (mm); `survey` is the site's datum
/// (the surveyed height of project 0), when there is a site.
pub fn spot_text(style: SpotStyle, z: f64, survey: Option<f64>) -> String {
    let ft_in = |mm: f64| format_ft_in(mm).replacen("'-", "' - ", 1);
    match style {
        SpotStyle::Triangle => ft_in(z),
        SpotStyle::Target => ft_in(z + RELATIVE_DATUM),
        SpotStyle::Cross => match survey {
            Some(d) => format!("{:.2}'", (z + d) / MM_PER_FT),
            None => ft_in(z),
        },
        SpotStyle::TextOnly => format!("EL. {}", ft_in(z)),
    }
}

/// Graphic scale divisions for a 1:`scale` view: the unit `u` (feet) such that the bar,
/// marked 0 · u · 2u · 4u, is at most 60 mm on paper.
pub fn scale_unit(scale: f64) -> f64 {
    const UNITS: [f64; 13] = [
        0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 10.0, 20.0, 40.0, 50.0, 100.0, 200.0,
    ];
    let paper = |u: f64| 4.0 * u * MM_PER_FT / scale.max(1.0);
    UNITS
        .iter()
        .copied()
        .rev()
        .find(|&u| paper(u) <= 60.0 + 1e-6)
        .unwrap_or(UNITS[0])
}

/// A graphic scale mark's label: feet (16'), or inches below a foot (6").
pub fn scale_label(ft: f64) -> String {
    if ft == 0.0 {
        "0".into()
    } else if ft < 1.0 {
        format!("{}\"", (ft * 12.0 * 100.0).round() / 100.0)
    } else {
        format!("{}'", (ft * 100.0).round() / 100.0)
    }
}

fn view_kind(doc: &Document, view: ElementId) -> CoreResult<Option<ViewKind>> {
    match doc.data(view)? {
        ElementData::View { kind, .. } => Ok(Some(kind.clone())),
        ElementData::Sheet { .. } => Ok(None),
        _ => Err(CoreError::Invalid("not a view".into())),
    }
}

fn is_plan(k: &ViewKind) -> bool {
    matches!(k, ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. })
}

fn is_drawn(k: &ViewKind) -> bool {
    !matches!(k, ViewKind::ThreeD | ViewKind::Schedule { .. })
}

/// Places a spot elevation measuring the model at `at`, its symbol and text at `leader`
/// (plans, ceiling plans, elevations and sections).
pub fn create_spot_elevation(
    doc: &mut Document,
    view: ElementId,
    at: Pt,
    leader: Pt,
) -> CoreResult<ElementId> {
    match view_kind(doc, view)? {
        Some(k) if is_drawn(&k) => {}
        _ => {
            return Err(CoreError::Invalid(
                "spot elevations go in plans, elevations and sections".into(),
            ))
        }
    }
    doc.transact("Place spot elevation", |tx| {
        Ok(tx.insert(ElementData::SpotElevation { view, at, leader }))
    })
}

/// Places a north arrow in a plan or on a sheet.
pub fn create_north_arrow(doc: &mut Document, view: ElementId, at: Pt) -> CoreResult<ElementId> {
    match view_kind(doc, view)? {
        None => {}
        Some(k) if is_plan(&k) => {}
        _ => {
            return Err(CoreError::Invalid(
                "north arrows go in plans or on sheets".into(),
            ))
        }
    }
    doc.transact("Place north arrow", |tx| {
        Ok(tx.insert(ElementData::NorthArrow { view, at }))
    })
}

/// Places a graphic scale in a plan, elevation or section; its divisions follow the view.
pub fn create_graphic_scale(doc: &mut Document, view: ElementId, at: Pt) -> CoreResult<ElementId> {
    match view_kind(doc, view)? {
        Some(k) if is_drawn(&k) => {}
        _ => {
            return Err(CoreError::Invalid(
                "graphic scales go in plans, elevations and sections".into(),
            ))
        }
    }
    doc.transact("Place graphic scale", |tx| {
        Ok(tx.insert(ElementData::GraphicScale { view, at }))
    })
}

/// A new key plan's width on paper, mm (3").
pub const KEY_PLAN_WIDTH: f64 = 76.2;

/// Places a key plan on a sheet, centered at `at` (paper mm).
pub fn create_key_plan(doc: &mut Document, sheet: ElementId, at: Pt) -> CoreResult<ElementId> {
    if view_kind(doc, sheet)?.is_some() {
        return Err(CoreError::Invalid("key plans go on sheets".into()));
    }
    doc.transact("Place key plan", |tx| {
        Ok(tx.insert(ElementData::KeyPlan {
            sheet,
            at,
            width: KEY_PLAN_WIDTH,
        }))
    })
}

fn style_label(doc: &Document, category: &str, item: &str) -> String {
    let i = choice(doc, category, item).unwrap_or(0);
    crate::standards_catalog::choices(category, item)
        .get(i)
        .map_or_else(String::new, |c| c.label.to_owned())
}

/// Properties of a symbol: its style (set on the Standards tab) and a key plan's width.
pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    const G: &str = "Graphics";
    let std_row =
        |item: &str, cat: &str| ro("style", "Style (Standards)", G, style_label(doc, cat, item));
    match doc.data(id) {
        Ok(ElementData::SpotElevation { at, leader, .. }) => {
            props.push(std_row("Spot Elevations", "sym"));
            let has = at.dist(*leader) > 1e-6;
            props.push(ro(
                "leader",
                "Leader",
                G,
                if has { "Yes" } else { "No" }.into(),
            ));
        }
        Ok(ElementData::NorthArrow { .. }) => props.push(std_row("North Arrow", "sym")),
        Ok(ElementData::GraphicScale { .. }) => props.push(std_row("Graphic Scale", "sym")),
        Ok(ElementData::KeyPlan { width, .. }) => {
            props.push(std_row("Key Plan & North Arrow", "sheet"));
            props.push(len("width", "Width", G, *width));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Category;
    use crate::standards::set_standard;

    fn plan_and_sheet(doc: &mut Document) -> (ElementId, ElementId, ElementId) {
        crate::ops::seed_default_project(doc).unwrap();
        let view = |doc: &Document, f: fn(&ViewKind) -> bool| {
            doc.of(Category::View)
                .find(|e| matches!(&e.data, ElementData::View { kind, .. } if f(kind)))
                .unwrap()
                .id
        };
        let plan = view(doc, |k| matches!(k, ViewKind::FloorPlan { .. }));
        let d3 = view(doc, |k| matches!(k, ViewKind::ThreeD));
        let sheet =
            crate::ops::create_sheet(doc, "Plans", crate::element::SheetSize::ArchD).unwrap();
        (plan, d3, sheet)
    }

    #[test]
    fn symbols_go_where_revit_puts_them() {
        let mut doc = Document::new();
        let (plan, d3, sheet) = plan_and_sheet(&mut doc);
        let p = Pt::new(1000.0, 2000.0);
        create_spot_elevation(&mut doc, plan, p, p.add(Pt::new(600.0, 600.0))).unwrap();
        create_north_arrow(&mut doc, plan, p).unwrap();
        create_north_arrow(&mut doc, sheet, Pt::new(50.0, 50.0)).unwrap();
        create_graphic_scale(&mut doc, plan, p).unwrap();
        create_key_plan(&mut doc, sheet, Pt::new(100.0, 100.0)).unwrap();
        assert!(create_spot_elevation(&mut doc, d3, p, p).is_err());
        assert!(create_graphic_scale(&mut doc, sheet, p).is_err());
        assert!(create_key_plan(&mut doc, plan, p).is_err());
        assert_eq!(doc.count(Category::SpotElevation), 1);
        assert_eq!(doc.count(Category::NorthArrow), 2);
        assert_eq!(doc.count(Category::GraphicScale), 1);
        assert_eq!(doc.count(Category::KeyPlan), 1);
        doc.undo().unwrap();
        assert_eq!(doc.count(Category::KeyPlan), 0);
    }

    #[test]
    fn styles_follow_the_standards() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        // Defaults: the default values, or the first choice when open.
        assert_eq!(NorthStyle::of(&doc), NorthStyle::ProjectAndTrue);
        assert_eq!(SpotStyle::of(&doc), SpotStyle::Triangle);
        assert_eq!(ScaleStyle::of(&doc), ScaleStyle::Alternating);
        assert_eq!(KeyPlanStyle::of(&doc), KeyPlanStyle::TitleBlock);
        // North Arrow is sym item 9; Compass rose is its fourth choice.
        set_standard(&mut doc, "sym", 9, Some("Compass rose"), None).unwrap();
        assert_eq!(NorthStyle::of(&doc), NorthStyle::Compass);
        set_standard(&mut doc, "sym", 9, Some("my own arrow"), None).unwrap();
        assert_eq!(NorthStyle::of(&doc), NorthStyle::ProjectAndTrue);
        doc.undo().unwrap();
        assert_eq!(NorthStyle::of(&doc), NorthStyle::Compass);
    }

    #[test]
    fn spot_text_by_style() {
        let z = 3086.1; // 10'-1 1/2"
        assert_eq!(spot_text(SpotStyle::Triangle, z, None), "10' - 1 1/2\"");
        assert_eq!(spot_text(SpotStyle::Target, z, None), "110' - 1 1/2\"");
        assert_eq!(spot_text(SpotStyle::TextOnly, 0.0, None), "EL. 0' - 0\"");
        // Survey: project 0 is 412.00' (125577.6 mm).
        assert_eq!(
            spot_text(SpotStyle::Cross, 1524.0, Some(125_577.6)),
            "417.00'"
        );
    }

    #[test]
    fn graphic_scale_units_fit_sixty_mm() {
        // 1/8" = 1'-0": 0 · 4 · 8 · 16' (50.8 mm).
        assert_eq!(scale_unit(96.0), 4.0);
        assert_eq!(scale_unit(48.0), 2.0);
        assert_eq!(scale_unit(24.0), 1.0);
        assert_eq!(scale_unit(240.0), 10.0);
        assert_eq!(scale_unit(16.0), 0.5);
        assert_eq!(scale_unit(4.0), 0.125);
        assert_eq!(scale_label(0.5), "6\"");
        assert_eq!(scale_label(16.0), "16'");
        assert_eq!(scale_label(0.0), "0");
    }
}
