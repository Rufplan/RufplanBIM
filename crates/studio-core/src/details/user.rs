//! Your own details (ADR-073): a drafting view saved to the Detail Library, and inserted
//! again in any project. What's saved is the view's own elements (detail lines, filled
//! regions, detail components, text notes and dimensions); dimensions keep following the
//! lines and components they measure.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Anchor, ElementData, ElementId, ViewKind};

/// A saved detail, as the library file holds it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedDetail {
    pub id: String,
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub description: String,
    pub scale: u32,
    /// The view's elements with their ids then (for re-linking dimensions).
    pub elements: Vec<(ElementId, ElementData)>,
}

/// The view an element of a detail belongs to, if it's one a detail carries.
fn owner(data: &ElementData) -> Option<ElementId> {
    match data {
        ElementData::DetailLine { view, .. }
        | ElementData::FilledRegion { view, .. }
        | ElementData::DetailComponent { view, .. }
        | ElementData::TextNote { view, .. }
        | ElementData::Dimension { view, .. } => Some(*view),
        _ => None,
    }
}

fn set_owner(data: &mut ElementData, to: ElementId) {
    match data {
        ElementData::DetailLine { view, .. }
        | ElementData::FilledRegion { view, .. }
        | ElementData::DetailComponent { view, .. }
        | ElementData::TextNote { view, .. }
        | ElementData::Dimension { view, .. } => *view = to,
        _ => {}
    }
}

/// Save to Library: a drafting view's elements as a detail named \`name\` in \`category\`.
pub fn capture(
    doc: &Document,
    view: ElementId,
    id: &str,
    name: &str,
    category: &str,
    description: &str,
) -> CoreResult<SavedDetail> {
    let ElementData::View {
        kind: ViewKind::Drafting,
        scale,
        ..
    } = doc.data(view)?
    else {
        return Err(CoreError::Invalid(
            "save a drafting view to the library (View > Drafting View)".into(),
        ));
    };
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("name the detail".into()));
    }
    let elements: Vec<(ElementId, ElementData)> = doc
        .iter()
        .filter(|e| owner(&e.data) == Some(view))
        .map(|e| (e.id, e.data.clone()))
        .collect();
    if elements.is_empty() {
        return Err(CoreError::Invalid(
            "that view has nothing drawn in it yet".into(),
        ));
    }
    Ok(SavedDetail {
        id: id.to_owned(),
        name: name.to_owned(),
        category: if category.trim().is_empty() {
            "My Details".into()
        } else {
            category.trim().to_owned()
        },
        description: description.trim().to_owned(),
        scale: *scale,
        elements,
    })
}

/// An anchor to a saved element, pointed at its copy (dropped if it pointed elsewhere).
fn remap(a: &Option<Anchor>, map: &HashMap<ElementId, ElementId>) -> Option<Anchor> {
    match a {
        Some(Anchor::DetailLine { line, t }) => map
            .get(line)
            .map(|l| Anchor::DetailLine { line: *l, t: *t }),
        Some(Anchor::Component { component, u, v }) => {
            map.get(component).map(|c| Anchor::Component {
                component: *c,
                u: *u,
                v: *v,
            })
        }
        _ => None,
    }
}

/// Inserts a saved detail: a new drafting view with copies of its elements, one undo step.
pub fn insert_saved(doc: &mut Document, d: &SavedDetail) -> CoreResult<ElementId> {
    let taken = |doc: &Document, n: &str| {
        doc.iter()
            .any(|e| matches!(&e.data, ElementData::View { name, .. } if name == n))
    };
    let mut name = d.name.clone();
    let mut i = 2;
    while taken(doc, &name) {
        name = format!("{} ({i})", d.name);
        i += 1;
    }
    doc.transact(&format!("Insert Detail {}", d.name), |tx| {
        let view = tx.insert(ElementData::view(name, ViewKind::Drafting, d.scale));
        let mut map = HashMap::new();
        // Everything but dimensions first, so dimensions can find their copies.
        for (old, data) in d
            .elements
            .iter()
            .filter(|(_, e)| !matches!(e, ElementData::Dimension { .. }))
        {
            let mut data = data.clone();
            set_owner(&mut data, view);
            map.insert(*old, tx.insert(data));
        }
        for (_, data) in d
            .elements
            .iter()
            .filter(|(_, e)| matches!(e, ElementData::Dimension { .. }))
        {
            let mut data = data.clone();
            set_owner(&mut data, view);
            if let ElementData::Dimension {
                a_ref,
                b_ref,
                between,
                ..
            } = &mut data
            {
                *a_ref = remap(a_ref, &map);
                *b_ref = remap(b_ref, &map);
                for r in between.iter_mut() {
                    r.anchor = remap(&r.anchor, &map);
                }
            }
            tx.insert(data);
        }
        Ok(view)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use studio_geom::Pt;

    #[test]
    fn a_saved_detail_comes_back_with_its_dimensions_linked() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let v = crate::details::insert(&mut doc, "window-head").unwrap();
        // Something of our own: a component and a dimension to a detail line.
        let c = crate::details::create_component(
            &mut doc,
            v,
            "lum-2x6",
            Pt::new(0.0, 0.0),
            Pt::new(100.0, 0.0),
            false,
        )
        .unwrap();
        let line = doc
            .iter()
            .find(|e| matches!(&e.data, ElementData::DetailLine { view, .. } if *view == v))
            .unwrap()
            .id;
        let (a, b) = match doc.data(line).unwrap() {
            ElementData::DetailLine { curve, .. } => curve.ends(),
            _ => unreachable!(),
        };
        doc.transact("dim", |tx| {
            tx.insert(ElementData::Dimension {
                view: v,
                a,
                b,
                offset: 50.0,
                a_ref: Some(Anchor::DetailLine { line, t: 0.0 }),
                b_ref: Some(Anchor::Component {
                    component: c,
                    u: 0.0,
                    v: 0.0,
                }),
                between: vec![],
                along: None,
                kind: Default::default(),
            });
            Ok(())
        })
        .unwrap();
        let saved = capture(&doc, v, "u1", "My Head", "", "ours").unwrap();
        assert_eq!(saved.category, "My Details");
        assert_eq!(saved.scale, 4);
        let n = saved.elements.len();
        // Round-trips through JSON, as the library file keeps it.
        let json = serde_json::to_string(&saved).unwrap();
        let back: SavedDetail = serde_json::from_str(&json).unwrap();
        // Into a fresh project.
        let mut other = Document::new();
        ops::seed_default_project(&mut other).unwrap();
        let nv = insert_saved(&mut other, &back).unwrap();
        let copies: Vec<&ElementData> = other
            .iter()
            .filter(|e| owner(&e.data) == Some(nv))
            .map(|e| &e.data)
            .collect();
        assert_eq!(copies.len(), n);
        let dim = copies
            .iter()
            .find(|d| matches!(d, ElementData::Dimension { .. }))
            .unwrap();
        let ElementData::Dimension { a_ref, b_ref, .. } = dim else {
            unreachable!()
        };
        // Its anchors point at the copies, which are in the new view.
        for r in [a_ref, b_ref] {
            let id = match r.unwrap() {
                Anchor::DetailLine { line, .. } => line,
                Anchor::Component { component, .. } => component,
                _ => panic!("anchor lost"),
            };
            assert_eq!(owner(other.data(id).unwrap()), Some(nv));
        }
        assert!(capture(&doc, nv, "x", "", "", "").is_err());
        let plan = doc
            .iter()
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
            .id;
        assert!(capture(&doc, plan, "x", "Plan", "", "").is_err());
    }
}
