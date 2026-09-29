//! Reference sections and callouts (ADR-076), after Revit's "Reference Other View": a
//! section line or callout box drawn in a view that points at a view already made (a
//! typical detail in a drafting view, another section, a callout) instead of making a new
//! one. Its head shows the detail and sheet numbers where that view is placed.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, ViewKind};
use crate::ops::{choice, ro, PropOption, Property};

/// The mark's shape, in its view's coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RefShape {
    /// A section line drawn `start` → `end`, looking to its left like a section.
    Section { start: Pt, end: Pt },
    /// A callout box between corners `min` and `max`.
    Callout { min: Pt, max: Pt },
}

/// A view a reference can point at, as listed in the options bar.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct RefTarget {
    pub id: ElementId,
    /// "Drafting View: Typ. Eave", "Section: Section 2", "Detail View: Callout of Level 1".
    pub label: String,
}

/// Views a reference can be drawn in: plans, ceiling plans, elevations, sections and
/// drafting views.
pub fn can_host(doc: &Document, view: ElementId) -> bool {
    matches!(
        doc.data(view),
        Ok(ElementData::View {
            kind: ViewKind::FloorPlan { .. }
                | ViewKind::CeilingPlan { .. }
                | ViewKind::Elevation { .. }
                | ViewKind::Section { .. }
                | ViewKind::MarkerElevation { .. }
                | ViewKind::Drafting,
            ..
        })
    )
}

/// What `view` is as a target of a section (`callout` false) or a callout, with the label
/// Revit's list gives it; None when it can't be one.
fn target_label(data: &ElementData, callout: bool) -> Option<String> {
    let ElementData::View {
        name,
        kind,
        callout_of,
        ..
    } = data
    else {
        return None;
    };
    match kind {
        ViewKind::Drafting => Some(format!("Drafting View: {name}")),
        // A section references other sections; a callout, other detail views.
        ViewKind::Section { .. } if !callout && callout_of.is_none() => {
            Some(format!("Section: {name}"))
        }
        _ if callout && callout_of.is_some() => Some(format!("Detail View: {name}")),
        _ => None,
    }
}

/// The views a section (`callout` false) or callout drawn in `host` can reference, drafting
/// views first, each group by name.
pub fn targets(doc: &Document, host: ElementId, callout: bool) -> Vec<RefTarget> {
    let mut out: Vec<(u8, RefTarget)> = doc
        .of(Category::View)
        .filter(|e| e.id != host)
        .filter_map(|e| {
            let label = target_label(&e.data, callout)?;
            let group = u8::from(!matches!(
                &e.data,
                ElementData::View {
                    kind: ViewKind::Drafting,
                    ..
                }
            ));
            Some((group, RefTarget { id: e.id, label }))
        })
        .collect();
    out.sort_by(|a, b| (a.0, &a.1.label).cmp(&(b.0, &b.1.label)));
    out.into_iter().map(|(_, t)| t).collect()
}

fn check_shape(shape: &RefShape) -> CoreResult<()> {
    match shape {
        RefShape::Section { start, end } if start.dist(*end) < 300.0 => {
            Err(CoreError::Invalid("draw a longer section line".into()))
        }
        RefShape::Callout { min, max } if max.x - min.x < 150.0 || max.y - min.y < 150.0 => {
            Err(CoreError::Invalid("draw a larger callout".into()))
        }
        _ => Ok(()),
    }
}

/// The shape with a callout's corners ordered min/max.
fn normalized(shape: RefShape) -> RefShape {
    match shape {
        RefShape::Callout { min: a, max: b } => RefShape::Callout {
            min: Pt::new(a.x.min(b.x), a.y.min(b.y)),
            max: Pt::new(a.x.max(b.x), a.y.max(b.y)),
        },
        s => s,
    }
}

/// Draws a reference section or callout in `host` pointing at `target`, or with None, at a
/// new drafting view (Revit's "<New drafting view>") made in the same undo step.
pub fn create(
    doc: &mut Document,
    host: ElementId,
    shape: RefShape,
    target: Option<ElementId>,
) -> CoreResult<ElementId> {
    if !can_host(doc, host) {
        return Err(CoreError::Invalid(
            "draw references in a plan, elevation, section or drafting view".into(),
        ));
    }
    let shape = normalized(shape);
    check_shape(&shape)?;
    let callout = matches!(shape, RefShape::Callout { .. });
    if let Some(t) = target {
        if t == host || target_label(doc.data(t)?, callout).is_none() {
            return Err(CoreError::Invalid(if callout {
                "a callout references a drafting view or another callout".into()
            } else {
                "a section references a drafting view or another section".into()
            }));
        }
    }
    let new_name = crate::details::unique_view_name(doc, "Drafting 1");
    let what = if callout { "Callout" } else { "Section" };
    doc.transact(&format!("Create Reference {what}"), |tx| {
        let target = match target {
            Some(t) => t,
            None => tx.insert(ElementData::view(
                new_name,
                ViewKind::Drafting,
                crate::detail::CALLOUT_SCALE,
            )),
        };
        Ok(tx.insert(ElementData::ViewReference {
            view: host,
            target,
            shape,
        }))
    })
}

/// Where `view` is placed: its detail number on the sheet and the sheet's number.
pub fn placement(doc: &Document, view: ElementId) -> Option<(String, String)> {
    let sheet = doc.iter().find_map(|e| match &e.data {
        ElementData::Viewport { sheet, view: v, .. } if *v == view => Some(*sheet),
        _ => None,
    })?;
    let mut on_sheet: Vec<ElementId> = doc
        .iter()
        .filter_map(|e| match &e.data {
            ElementData::Viewport {
                sheet: s, view: v, ..
            } if *s == sheet => Some(*v),
            _ => None,
        })
        .collect();
    on_sheet.sort();
    let n = on_sheet.iter().position(|v| *v == view)? + 1;
    let number = crate::ops::sheets(doc)
        .into_iter()
        .find(|s| s.0 == sheet)
        .map(|s| s.1)?;
    Some((n.to_string(), number))
}

/// Properties: the referenced view (a choice, as Revit's) and where it is placed.
pub fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(ElementData::ViewReference {
        view,
        target,
        shape,
    }) = doc.data(id)
    else {
        return;
    };
    let callout = matches!(shape, RefShape::Callout { .. });
    let options = targets(doc, *view, callout)
        .into_iter()
        .map(|t| PropOption {
            id: t.id.to_string(),
            label: t.label,
        })
        .collect();
    props.push(choice(
        "target",
        "Referenced View",
        "Identity Data",
        target.to_string(),
        options,
    ));
    let (detail, sheet) = placement(doc, *target).unwrap_or_default();
    props.push(ro(
        "ref_detail",
        "Reference Detail",
        "Identity Data",
        detail,
    ));
    props.push(ro("ref_sheet", "Reference Sheet", "Identity Data", sheet));
}

/// Points a reference at another view (Properties > Referenced View).
pub fn set_property(doc: &mut Document, id: ElementId, key: &str, value: &str) -> CoreResult<()> {
    let ElementData::ViewReference { view, shape, .. } = doc.data(id)?.clone() else {
        return Err(CoreError::Invalid("not a reference".into()));
    };
    if key != "target" {
        return Err(CoreError::Invalid(format!("unknown property {key}")));
    }
    let to = crate::ops::parse_id(value)?;
    let callout = matches!(shape, RefShape::Callout { .. });
    if to == view || target_label(doc.data(to)?, callout).is_none() {
        return Err(CoreError::Invalid(
            "that view can't be referenced here".into(),
        ));
    }
    doc.transact("Change Referenced View", |tx| {
        tx.modify(id, |d| {
            if let ElementData::ViewReference { target, .. } = d {
                *target = to;
            }
        })
    })
}

/// The view a reference points at (double-clicking its head opens it).
pub fn target_of(doc: &Document, id: ElementId) -> Option<ElementId> {
    match doc.data(id) {
        Ok(ElementData::ViewReference { target, .. }) => Some(*target),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::details::create_drafting_view;
    use crate::element::SheetSize;

    fn plan(doc: &Document) -> ElementId {
        doc.of(Category::View)
            .find(|e| {
                matches!(
                    e.data,
                    ElementData::View {
                        kind: ViewKind::FloorPlan { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id
    }

    fn doc() -> Document {
        let mut d = Document::new();
        crate::ops::seed_default_project(&mut d).unwrap();
        d
    }

    #[test]
    fn a_reference_section_points_at_a_typical_detail_without_making_a_view() {
        let mut d = doc();
        let host = plan(&d);
        let eave = create_drafting_view(&mut d, "Typ. Eave", 8).unwrap();
        let sec =
            crate::ops::create_section(&mut d, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).unwrap();
        let views = d.count(Category::View);

        // Sections list drafting views first, then sections; never the host.
        let list = targets(&d, host, false);
        assert_eq!(list[0].label, "Drafting View: Typ. Eave");
        assert_eq!(list[1].label, "Section: Section 1");
        assert!(list.iter().all(|t| t.id != host));
        // Callouts don't list sections.
        assert!(targets(&d, host, true).iter().all(|t| t.id != sec));

        let shape = RefShape::Section {
            start: Pt::new(0.0, 2000.0),
            end: Pt::new(3000.0, 2000.0),
        };
        let r = create(&mut d, host, shape, Some(eave)).unwrap();
        assert_eq!(d.count(Category::View), views);
        assert_eq!(target_of(&d, r), Some(eave));
        assert_eq!(d.data(r).unwrap().name(), "Reference Section");

        // A plan is not something a section can reference.
        let bad = create(&mut d, host, shape, Some(host));
        assert!(bad.is_err());
        let short = RefShape::Section {
            start: Pt::new(0.0, 0.0),
            end: Pt::new(100.0, 0.0),
        };
        assert!(create(&mut d, host, short, Some(eave)).is_err());

        // Deleting the detail deletes the reference, as in Revit.
        crate::ops::delete(&mut d, &[eave]).unwrap();
        assert!(d.data(r).is_err());
    }

    #[test]
    fn new_drafting_view_references_and_shows_where_it_is_placed() {
        let mut d = doc();
        let host = plan(&d);
        let shape = RefShape::Callout {
            min: Pt::new(4000.0, 3000.0),
            max: Pt::new(1000.0, 1000.0),
        };
        let r = create(&mut d, host, shape, None).unwrap();
        let target = target_of(&d, r).unwrap();
        let ElementData::View {
            name, kind, scale, ..
        } = d.data(target).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (name.as_str(), kind, *scale),
            ("Drafting 1", &ViewKind::Drafting, 8)
        );
        // Corners are ordered.
        let ElementData::ViewReference {
            shape: RefShape::Callout { min, max },
            ..
        } = d.data(r).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            (*min, *max),
            (Pt::new(1000.0, 1000.0), Pt::new(4000.0, 3000.0))
        );
        // One undo removes both.
        d.undo().unwrap();
        assert!(d.data(target).is_err() && d.data(r).is_err());
        d.redo().unwrap();

        // Unplaced, the properties show no detail; placed, detail 1 on its sheet.
        let detail = |d: &Document| {
            let mut props = vec![];
            properties(d, r, &mut props);
            let v = |k: &str| props.iter().find(|p| p.key == k).unwrap().value.clone();
            (v("ref_detail"), v("ref_sheet"))
        };
        assert_eq!(detail(&d), (String::new(), String::new()));
        let sheet = crate::ops::create_sheet(&mut d, "Details", SheetSize::ArchD).unwrap();
        let number = crate::ops::sheets(&d)
            .into_iter()
            .find(|s| s.0 == sheet)
            .unwrap()
            .1;
        crate::ops::place_view(&mut d, sheet, target, Pt::new(400.0, 300.0)).unwrap();
        assert_eq!(detail(&d), ("1".into(), number));

        // Retargeting: to another drafting view, not to the host plan.
        let other = create_drafting_view(&mut d, "Typ. Sill", 4).unwrap();
        set_property(&mut d, r, "target", &other.to_string()).unwrap();
        assert_eq!(target_of(&d, r), Some(other));
        assert!(set_property(&mut d, r, "target", &host.to_string()).is_err());
    }
}
