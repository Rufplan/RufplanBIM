//! Views and sheets as Revit's project browser handles them (ADR-074): new plan, ceiling
//! plan and 3D views, Duplicate View (with or without its detailing) and Duplicate Sheet
//! (empty, with its detailing, or with its views).

use std::collections::HashMap;

use serde::Deserialize;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{Anchor, ElementData, ElementId, ViewKind};

fn name_taken(tx: &Tx<'_>, n: &str) -> bool {
    tx.iter()
        .any(|e| matches!(&e.data, ElementData::View { name, .. } if name == n))
}

/// Revit's naming for a copy: "Level 1 Copy 1", "Level 1 Copy 2"…
fn copy_name(tx: &Tx<'_>, base: &str) -> String {
    (1..)
        .map(|i| format!("{base} Copy {i}"))
        .find(|n| !name_taken(tx, n))
        .unwrap_or_default()
}

/// Revit's naming for another plan of a level: "Level 1 (1)", "Level 1 (2)"…
fn next_name(tx: &Tx<'_>, base: &str) -> String {
    if !name_taken(tx, base) {
        return base.to_owned();
    }
    (1..)
        .map(|i| format!("{base} ({i})"))
        .find(|n| !name_taken(tx, n))
        .unwrap_or_default()
}

/// View > Plan Views > Floor Plan (or Reflected Ceiling Plan) of \`level\`.
pub fn create_plan(doc: &mut Document, level: ElementId, ceiling: bool) -> CoreResult<ElementId> {
    let name = match doc.data(level)? {
        ElementData::Level { name, .. } => name.clone(),
        _ => return Err(CoreError::Invalid("pick a level".into())),
    };
    let label = if ceiling {
        "Create ceiling plan"
    } else {
        "Create floor plan"
    };
    doc.transact(label, |tx| {
        let name = next_name(tx, &name);
        let kind = if ceiling {
            ViewKind::CeilingPlan { level }
        } else {
            ViewKind::FloorPlan { level }
        };
        Ok(tx.insert(ElementData::view(name, kind, 48)))
    })
}

/// View > 3D View: a new 3D view ("3D View 1").
pub fn create_3d(doc: &mut Document) -> CoreResult<ElementId> {
    doc.transact("Create 3D view", |tx| {
        let name = (1..)
            .map(|i| format!("3D View {i}"))
            .find(|n| !name_taken(tx, n))
            .unwrap_or_default();
        Ok(tx.insert(ElementData::view(name, ViewKind::ThreeD, 96)))
    })
}

/// The view (or sheet) an annotation belongs to, if it's one a copy carries.
fn owner(data: &ElementData) -> Option<ElementId> {
    match data {
        ElementData::TextNote { view, .. }
        | ElementData::DetailLine { view, .. }
        | ElementData::FilledRegion { view, .. }
        | ElementData::DetailComponent { view, .. }
        | ElementData::Dimension { view, .. }
        | ElementData::AngularDimension { view, .. }
        | ElementData::Tag { view, .. }
        | ElementData::SpotElevation { view, .. }
        | ElementData::SpotSlope { view, .. }
        | ElementData::NorthArrow { view, .. }
        | ElementData::GraphicScale { view, .. } => Some(*view),
        ElementData::KeyPlan { sheet, .. } => Some(*sheet),
        _ => None,
    }
}

fn set_owner(data: &mut ElementData, to: ElementId) {
    match data {
        ElementData::TextNote { view, .. }
        | ElementData::DetailLine { view, .. }
        | ElementData::FilledRegion { view, .. }
        | ElementData::DetailComponent { view, .. }
        | ElementData::Dimension { view, .. }
        | ElementData::AngularDimension { view, .. }
        | ElementData::Tag { view, .. }
        | ElementData::SpotElevation { view, .. }
        | ElementData::SpotSlope { view, .. }
        | ElementData::NorthArrow { view, .. }
        | ElementData::GraphicScale { view, .. } => *view = to,
        ElementData::KeyPlan { sheet, .. } => *sheet = to,
        _ => {}
    }
}

fn remap(a: &mut Option<Anchor>, map: &HashMap<ElementId, ElementId>) {
    match a {
        Some(Anchor::DetailLine { line, .. }) => {
            if let Some(n) = map.get(line) {
                *line = *n;
            }
        }
        Some(Anchor::Component { component, .. }) => {
            if let Some(n) = map.get(component) {
                *component = *n;
            }
        }
        _ => {}
    }
}

/// Copies \`from\`'s annotations to \`to\` (dimensions last, re-linked to the copies).
fn copy_detailing(tx: &mut Tx<'_>, from: ElementId, to: ElementId) {
    let items: Vec<(ElementId, ElementData)> = tx
        .iter()
        .filter(|e| owner(&e.data) == Some(from))
        .map(|e| (e.id, e.data.clone()))
        .collect();
    let dims = |d: &ElementData| matches!(d, ElementData::Dimension { .. });
    let mut map = HashMap::new();
    for (old, data) in items.iter().filter(|(_, d)| !dims(d)) {
        let mut data = data.clone();
        set_owner(&mut data, to);
        map.insert(*old, tx.insert(data));
    }
    for (_, data) in items.iter().filter(|(_, d)| dims(d)) {
        let mut data = data.clone();
        set_owner(&mut data, to);
        if let ElementData::Dimension {
            a_ref,
            b_ref,
            between,
            ..
        } = &mut data
        {
            remap(a_ref, &map);
            remap(b_ref, &map);
            for r in between.iter_mut() {
                remap(&mut r.anchor, &map);
            }
        }
        tx.insert(data);
    }
}

fn duplicate_in(tx: &mut Tx<'_>, view: ElementId, detailing: bool) -> CoreResult<ElementId> {
    let mut data = tx.data(view)?.clone();
    let ElementData::View { name, .. } = &mut data else {
        return Err(CoreError::Invalid("pick a view to duplicate".into()));
    };
    *name = copy_name(tx, name);
    let copy = tx.insert(data);
    if detailing {
        copy_detailing(tx, view, copy);
    }
    Ok(copy)
}

/// Duplicate View: the view with its settings (crop, hidden elements…), and with Detailing
/// its annotations too (text, detail lines and components, dimensions, tags, symbols).
pub fn duplicate_view(
    doc: &mut Document,
    view: ElementId,
    detailing: bool,
) -> CoreResult<ElementId> {
    let label = if detailing {
        "Duplicate with Detailing"
    } else {
        "Duplicate View"
    };
    doc.transact(label, |tx| duplicate_in(tx, view, detailing))
}

/// How Duplicate Sheet copies (Revit's three choices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub enum SheetCopy {
    /// The title block alone.
    Empty,
    /// Its text, lines and symbols too.
    WithDetailing,
    /// And a copy of each view on it, placed where it was (schedules are placed again).
    WithViews,
}

/// Duplicate Sheet: a new sheet (the next number, the same name and size).
pub fn duplicate_sheet(
    doc: &mut Document,
    sheet: ElementId,
    how: SheetCopy,
) -> CoreResult<ElementId> {
    let ElementData::Sheet {
        name, size, stages, ..
    } = doc.data(sheet)?.clone()
    else {
        return Err(CoreError::Invalid("pick a sheet to duplicate".into()));
    };
    let number =
        crate::ops::next_sheet_number(crate::ops::sheets(doc).last().map(|s| s.1.as_str()));
    doc.transact("Duplicate Sheet", |tx| {
        let copy = tx.insert(ElementData::Sheet {
            number,
            name,
            size,
            stages,
        });
        if how != SheetCopy::Empty {
            copy_detailing(tx, sheet, copy);
        }
        if how == SheetCopy::WithViews {
            let ports: Vec<ElementData> = tx
                .iter()
                .filter(
                    |e| matches!(&e.data, ElementData::Viewport { sheet: s, .. } if *s == sheet),
                )
                .map(|e| e.data.clone())
                .collect();
            for mut vp in ports {
                if let ElementData::Viewport { sheet: s, view, .. } = &mut vp {
                    *s = copy;
                    // A drawing view is on one sheet only: place a copy with its detailing.
                    let schedule = matches!(
                        tx.data(*view),
                        Ok(ElementData::View {
                            kind: ViewKind::Schedule { .. },
                            ..
                        })
                    );
                    if !schedule {
                        *view = duplicate_in(tx, *view, true)?;
                    }
                }
                tx.insert(vp);
            }
        }
        Ok(copy)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Category;
    use crate::ops;
    use crate::SheetSize;
    use studio_geom::Pt;

    #[test]
    fn duplicates_views_and_sheets_as_revit_names_them() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        let base = doc.data(plan).unwrap().name();
        // A text note in the plan comes along only with detailing.
        ops::create_text(&mut doc, plan, Pt::new(0.0, 0.0), "NOTE").unwrap();
        let notes = |doc: &Document, v| {
            doc.of(Category::TextNote)
                .filter(|e| owner(&e.data) == Some(v))
                .count()
        };
        let plain = duplicate_view(&mut doc, plan, false).unwrap();
        assert_eq!(doc.data(plain).unwrap().name(), format!("{base} Copy 1"));
        assert_eq!(notes(&doc, plain), 0);
        let detailed = duplicate_view(&mut doc, plan, true).unwrap();
        assert_eq!(doc.data(detailed).unwrap().name(), format!("{base} Copy 2"));
        assert_eq!(notes(&doc, detailed), 1);
        // New plans of a level: "Level 1 (1)".
        let fp = create_plan(&mut doc, l1, false).unwrap();
        assert_eq!(
            doc.data(fp).unwrap().name(),
            format!("{} (1)", doc.data(l1).unwrap().name())
        );
        let rcp = create_plan(&mut doc, l1, true).unwrap();
        assert!(matches!(
            doc.data(rcp).unwrap(),
            ElementData::View {
                kind: ViewKind::CeilingPlan { .. },
                ..
            }
        ));
        let v3 = create_3d(&mut doc).unwrap();
        assert_eq!(doc.data(v3).unwrap().name(), "3D View 1");

        // Sheets: empty, with detailing, with views (a copy of the plan, placed the same).
        let sheet = ops::create_sheet(&mut doc, "Floor Plans", SheetSize::ArchD).unwrap();
        ops::place_view(&mut doc, sheet, plan, Pt::new(300.0, 250.0)).unwrap();
        ops::create_text(&mut doc, sheet, Pt::new(50.0, 50.0), "SHEET NOTE").unwrap();
        let ports = |doc: &Document, s| {
            doc.iter()
                .filter(|e| matches!(&e.data, ElementData::Viewport { sheet, .. } if *sheet == s))
                .count()
        };
        let empty = duplicate_sheet(&mut doc, sheet, SheetCopy::Empty).unwrap();
        assert_eq!((ports(&doc, empty), notes(&doc, empty)), (0, 0));
        let det = duplicate_sheet(&mut doc, sheet, SheetCopy::WithDetailing).unwrap();
        assert_eq!((ports(&doc, det), notes(&doc, det)), (0, 1));
        let views = duplicate_sheet(&mut doc, sheet, SheetCopy::WithViews).unwrap();
        assert_eq!(ports(&doc, views), 1);
        let placed = doc
            .iter()
            .find_map(|e| match &e.data {
                ElementData::Viewport {
                    sheet,
                    view,
                    center,
                    ..
                } if *sheet == views => Some((*view, *center)),
                _ => None,
            })
            .unwrap();
        assert_ne!(placed.0, plan, "a copy of the view, not the view itself");
        assert_eq!(placed.1, Pt::new(300.0, 250.0));
        assert_eq!(notes(&doc, placed.0), 1, "the copy has the plan's note");
        // Next numbers, same name.
        let nums: Vec<String> = [sheet, empty, det, views]
            .iter()
            .map(|s| match doc.data(*s).unwrap() {
                ElementData::Sheet { number, name, .. } => {
                    assert_eq!(name, "Floor Plans");
                    number.clone()
                }
                _ => unreachable!(),
            })
            .collect();
        let unique: std::collections::HashSet<&String> = nums.iter().collect();
        assert_eq!(unique.len(), 4, "{nums:?}");
        assert!(duplicate_view(&mut doc, sheet, false).is_err());
    }
}
