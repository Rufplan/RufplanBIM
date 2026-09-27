//! Detail lines and model lines (ADR-054), as in Revit.
//! - A detail line belongs to one view (a plan, elevation, section or sheet) and is drawn
//!   only there.
//! - A model line lies on a level's work plane. It shows in that level's plans, in
//!   elevations and sections, and in 3D.
//!
//! Both are drawn with the sketch tools (line, rectangle, polygons, circle, arcs) and a
//! Revit line style.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{ElementData, ElementId, ViewKind};
use crate::ops::{choice, ro, PropOption, Property};
use crate::sketch::{DrawOptions, DrawTool, SketchCurve};
use crate::units::format_ft_in;

/// Revit's default line styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum LineStyle {
    #[default]
    Thin,
    Medium,
    Wide,
    Hidden,
    Centerline,
    Overhead,
    Demolished,
    Beyond,
}

impl LineStyle {
    pub const ALL: [LineStyle; 8] = [
        Self::Thin,
        Self::Medium,
        Self::Wide,
        Self::Hidden,
        Self::Centerline,
        Self::Overhead,
        Self::Demolished,
        Self::Beyond,
    ];

    /// The name Revit gives it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Thin => "Thin Lines",
            Self::Medium => "Medium Lines",
            Self::Wide => "Wide Lines",
            Self::Hidden => "<Hidden>",
            Self::Centerline => "<Centerline>",
            Self::Overhead => "<Overhead>",
            Self::Demolished => "<Demolished>",
            Self::Beyond => "<Beyond>",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let n: String = s
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase();
        Self::ALL.into_iter().find(|l| {
            let m: String = l
                .label()
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect::<String>()
                .to_ascii_lowercase();
            m == n || m.trim_end_matches("lines") == n || format!("{l:?}").to_ascii_lowercase() == n
        })
    }
}

/// Where new lines go: a view (detail lines) or a level's work plane (model lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinesOn {
    View(ElementId),
    Level(ElementId),
}

/// Draws lines with a sketch tool from its picked points, one transaction. Returns them.
pub fn create_lines(
    doc: &mut Document,
    on: LinesOn,
    tool: DrawTool,
    pts: &[Pt],
    options: &DrawOptions,
    style: LineStyle,
) -> CoreResult<Vec<ElementId>> {
    let curves = crate::sketch::draw(tool, pts, options)?;
    if curves.is_empty() {
        return Err(CoreError::Invalid("nothing to draw".into()));
    }
    match on {
        LinesOn::View(v) => match doc.data(v)? {
            ElementData::Sheet { .. }
            | ElementData::View {
                kind:
                    ViewKind::FloorPlan { .. }
                    | ViewKind::CeilingPlan { .. }
                    | ViewKind::Elevation { .. }
                    | ViewKind::Section { .. }
                    | ViewKind::MarkerElevation { .. },
                ..
            } => {}
            _ => {
                return Err(CoreError::Invalid(
                    "detail lines go in plans, elevations, sections and sheets".into(),
                ))
            }
        },
        LinesOn::Level(l) => {
            doc.level_elevation(l)?;
        }
    }
    let (label, detail) = match on {
        LinesOn::View(_) => ("Detail lines", true),
        LinesOn::Level(_) => ("Model lines", false),
    };
    doc.transact(label, |tx| {
        Ok(curves
            .into_iter()
            .map(|curve| {
                tx.insert(match on {
                    LinesOn::View(view) if detail => ElementData::DetailLine { view, curve, style },
                    LinesOn::View(_) | LinesOn::Level(_) => ElementData::ModelLine {
                        level: match on {
                            LinesOn::Level(l) => l,
                            LinesOn::View(v) => v,
                        },
                        curve,
                        style,
                    },
                })
            })
            .collect())
    })
}

/// A line's curve, if it is one.
pub fn curve_of(d: &ElementData) -> Option<&SketchCurve> {
    match d {
        ElementData::DetailLine { curve, .. } | ElementData::ModelLine { curve, .. } => Some(curve),
        _ => None,
    }
}

/// Properties: the line style, and its length.
pub(crate) fn properties(style: LineStyle, curve: &SketchCurve, props: &mut Vec<Property>) {
    props.push(choice(
        "style",
        "Line Style",
        "Graphics",
        format!("{style:?}"),
        LineStyle::ALL
            .iter()
            .map(|l| PropOption {
                id: format!("{l:?}"),
                label: l.label().into(),
            })
            .collect(),
    ));
    props.push(ro(
        "length",
        "Length",
        "Dimensions",
        format_ft_in(curve.length()),
    ));
}

/// Sets a line's style.
pub(crate) fn set_style(style: &mut LineStyle, key: &str, value: &str) -> CoreResult<()> {
    match key {
        "style" => {
            *style = LineStyle::parse(value)
                .ok_or_else(|| CoreError::Invalid(format!("no line style {value}")))?
        }
        _ => return Err(CoreError::Invalid(format!("unknown property {key}"))),
    }
    Ok(())
}

/// Drags a straight line's end.
pub(crate) fn drag_end(doc: &mut Document, id: ElementId, start: bool, to: Pt) -> CoreResult<()> {
    doc.transact("Drag line end", |tx| {
        tx.modify(id, |d| {
            if let ElementData::DetailLine { curve, .. } | ElementData::ModelLine { curve, .. } = d
            {
                if let SketchCurve::Line { a, b, .. } = curve {
                    if start {
                        *a = to;
                    } else {
                        *b = to;
                    }
                }
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Category;

    fn plan(doc: &Document) -> ElementId {
        doc.of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::FloorPlan { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id
    }

    #[test]
    fn lines_draw_with_the_sketch_tools_in_one_undo() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let v = plan(&doc);
        let o = DrawOptions::default();
        // A rectangle of detail lines: four lines, one undo.
        let ids = create_lines(
            &mut doc,
            LinesOn::View(v),
            DrawTool::Rectangle,
            &[Pt::new(0.0, 0.0), Pt::new(3000.0, 2000.0)],
            &o,
            LineStyle::Hidden,
        )
        .unwrap();
        assert_eq!(ids.len(), 4);
        assert!(ids.iter().all(|id| matches!(
            doc.data(*id),
            Ok(ElementData::DetailLine { style: LineStyle::Hidden, view, .. }) if *view == v
        )));
        doc.undo().unwrap();
        assert_eq!(doc.count(Category::DetailLine), 0);
        // A model line on Level 1, then its style changed and its end dragged.
        let l1 = doc.levels()[0].0;
        let id = create_lines(
            &mut doc,
            LinesOn::Level(l1),
            DrawTool::Line,
            &[Pt::new(0.0, 0.0), Pt::new(1000.0, 0.0)],
            &o,
            LineStyle::Thin,
        )
        .unwrap()[0];
        crate::ops::set_property(&mut doc, id, "style", "Wide", 0).unwrap();
        crate::edit::drag_handle(&mut doc, id, "end", Pt::new(3048.0, 0.0)).unwrap();
        let Ok(ElementData::ModelLine {
            style,
            curve,
            level,
        }) = doc.data(id)
        else {
            panic!()
        };
        assert_eq!((*style, *level), (LineStyle::Wide, l1));
        assert!((curve.length() - 3048.0).abs() < 1e-6);
        let props = crate::ops::properties(&doc, id).unwrap();
        let len = props.properties.iter().find(|p| p.key == "length").unwrap();
        assert_eq!(len.value, "10'-0\"");
        // Moved with Move; not placeable in a 3D view.
        crate::modify::move_elements(&mut doc, &[id], Pt::new(0.0, 500.0)).unwrap();
        assert!(
            matches!(doc.data(id), Ok(ElementData::ModelLine { curve: SketchCurve::Line { a, .. }, .. }) if (a.y - 500.0).abs() < 1e-9)
        );
        let d3 = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::ThreeD,
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        assert!(create_lines(
            &mut doc,
            LinesOn::View(d3),
            DrawTool::Line,
            &[Pt::new(0.0, 0.0), Pt::new(1.0, 0.0)],
            &o,
            LineStyle::Thin
        )
        .is_err());
        assert_eq!(LineStyle::parse("medium lines"), Some(LineStyle::Medium));
        assert_eq!(LineStyle::parse("<Hidden>"), Some(LineStyle::Hidden));
    }
}
