//! Revit's everyday element commands (ADR-024): pin/unpin, select all instances, tag an
//! element, and hide elements or categories in a view.

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, ViewKind};
use crate::params::ParamValue;

/// The parameter that marks an element pinned (kept in the element's parameter map).
pub const PINNED: &str = "__pinned";

pub fn is_pinned(doc: &Document, id: ElementId) -> bool {
    matches!(doc.param(id, PINNED), Some(ParamValue::Bool(true)))
}

/// Refuses to change pinned elements, as Revit does.
pub fn ensure_unpinned(doc: &Document, ids: &[ElementId]) -> CoreResult<()> {
    if ids.iter().any(|id| is_pinned(doc, *id)) {
        return Err(CoreError::Invalid(
            "a pinned element can't be moved or deleted; unpin it first (UP)".into(),
        ));
    }
    Ok(())
}

/// Pins (PN) or unpins (UP) elements; returns how many changed.
pub fn set_pinned(doc: &mut Document, ids: &[ElementId], pinned: bool) -> CoreResult<usize> {
    let change: Vec<ElementId> = ids
        .iter()
        .copied()
        .filter(|id| doc.get(*id).is_some() && is_pinned(doc, *id) != pinned)
        .collect();
    if change.is_empty() {
        return Ok(0);
    }
    doc.transact(if pinned { "Pin" } else { "Unpin" }, |tx| {
        for id in &change {
            tx.set_param(*id, PINNED, pinned.then_some(ParamValue::Bool(true)))?;
        }
        Ok(change.len())
    })
}

/// Select All Instances (SA): every element of the same type as `id` (or, for elements
/// without a type, of the same category).
pub fn all_instances(doc: &Document, id: ElementId) -> CoreResult<Vec<ElementId>> {
    let d = doc.data(id)?;
    let (cat, ty) = (d.category(), d.type_id());
    Ok(doc
        .of(cat)
        .filter(|e| ty.is_none() || e.data.type_id() == ty)
        .map(|e| e.id)
        .collect())
}

/// Tag (TG): tags one door, window, room, column or beam in `view`.
pub fn tag_element(
    doc: &mut Document,
    view: ElementId,
    target: ElementId,
) -> CoreResult<ElementId> {
    if !matches!(
        doc.data(view)?,
        ElementData::View {
            kind: ViewKind::FloorPlan { .. },
            ..
        }
    ) {
        return Err(CoreError::Invalid("tag in a floor plan".into()));
    }
    let taggable = matches!(
        doc.data(target)?.category(),
        Category::Door | Category::Window | Category::Room | Category::Column | Category::Beam
    );
    if !taggable {
        return Err(CoreError::Invalid(
            "tag a door, window, room, column or beam".into(),
        ));
    }
    let already = doc.iter().any(|e| {
        matches!(&e.data, ElementData::Tag { view: v, target: t, .. } if *v == view && *t == target)
    });
    if already {
        return Err(CoreError::Invalid(
            "that element is already tagged in this view".into(),
        ));
    }
    doc.transact("Tag", |tx| {
        Ok(tx.insert(ElementData::Tag {
            view,
            target,
            offset: studio_geom::Pt::default(),
        }))
    })
}

/// Hide in View > Elements (EH).
pub fn hide_elements(doc: &mut Document, view: ElementId, ids: &[ElementId]) -> CoreResult<()> {
    doc.transact("Hide in view", |tx| {
        tx.modify(view, |d| {
            if let ElementData::View { hidden, .. } = d {
                for id in ids {
                    if !hidden.contains(id) {
                        hidden.push(*id);
                    }
                }
            }
        })
    })
}

/// Hide in View > Category (VH), or show it again (Visibility/Graphics).
pub fn set_category_visible(
    doc: &mut Document,
    view: ElementId,
    cats: &[Category],
    visible: bool,
) -> CoreResult<()> {
    doc.transact(
        if visible {
            "Show category"
        } else {
            "Hide category"
        },
        |tx| {
            tx.modify(view, |d| {
                if let ElementData::View {
                    hidden_categories, ..
                } = d
                {
                    for c in cats {
                        hidden_categories.retain(|x| x != c);
                        if !visible {
                            hidden_categories.push(*c);
                        }
                    }
                }
            })
        },
    )
}

/// Unhides everything hidden in `view`.
pub fn unhide_all(doc: &mut Document, view: ElementId) -> CoreResult<()> {
    doc.transact("Unhide all", |tx| {
        tx.modify(view, |d| {
            if let ElementData::View {
                hidden,
                hidden_categories,
                ..
            } = d
            {
                hidden.clear();
                hidden_categories.clear();
            }
        })
    })
}

/// Whether `view` hides element `el` (itself or its category).
pub fn hidden_in(doc: &Document, view: &ElementData, el: ElementId) -> bool {
    let ElementData::View {
        hidden,
        hidden_categories,
        ..
    } = view
    else {
        return false;
    };
    if hidden.contains(&el) {
        return true;
    }
    !hidden_categories.is_empty()
        && doc
            .data(el)
            .is_ok_and(|d| hidden_categories.contains(&d.category()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use studio_geom::Pt;

    fn setup() -> (Document, ElementId, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let a =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).unwrap();
        let b = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 3000.0),
            Pt::new(5000.0, 3000.0),
        )
        .unwrap();
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        (doc, a, b, plan)
    }

    #[test]
    fn pinned_elements_resist_moves_and_deletes() {
        let (mut doc, a, _, _) = setup();
        assert_eq!(set_pinned(&mut doc, &[a], true).unwrap(), 1);
        assert!(is_pinned(&doc, a));
        assert!(crate::modify::move_elements(&mut doc, &[a], Pt::new(100.0, 0.0)).is_err());
        assert!(ops::delete(&mut doc, &[a]).is_err());
        assert!(crate::edit::transform_elements(
            &mut doc,
            &[a],
            crate::edit::Xform::rotate(Pt::default(), 1.0),
            "Rotate"
        )
        .is_err());
        set_pinned(&mut doc, &[a], false).unwrap();
        assert!(crate::modify::move_elements(&mut doc, &[a], Pt::new(100.0, 0.0)).is_ok());
    }

    #[test]
    fn select_all_instances_and_tagging() {
        let (mut doc, a, b, plan) = setup();
        let all = all_instances(&doc, a).unwrap();
        assert!(all.contains(&a) && all.contains(&b));
        let dt = doc.of(Category::DoorType).next().unwrap().id;
        let door = ops::create_door(&mut doc, dt, a, 2500.0, false).unwrap();
        // Doors are tagged on placement; a second tag is refused.
        assert!(tag_element(&mut doc, plan, door).is_err());
        assert!(
            tag_element(&mut doc, plan, a).is_err(),
            "walls aren't tagged"
        );
    }

    #[test]
    fn hide_elements_and_categories_in_a_view() {
        let (mut doc, a, b, plan) = setup();
        hide_elements(&mut doc, plan, &[a]).unwrap();
        let v = doc.data(plan).unwrap().clone();
        assert!(hidden_in(&doc, &v, a) && !hidden_in(&doc, &v, b));
        set_category_visible(&mut doc, plan, &[Category::Wall], false).unwrap();
        let v = doc.data(plan).unwrap().clone();
        assert!(hidden_in(&doc, &v, b));
        unhide_all(&mut doc, plan).unwrap();
        let v = doc.data(plan).unwrap().clone();
        assert!(!hidden_in(&doc, &v, a) && !hidden_in(&doc, &v, b));
    }
}
