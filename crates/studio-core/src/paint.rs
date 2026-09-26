//! Paint (ADR-034): a material applied to one element, as Revit's Paint tool does, without
//! changing its type. Stored as the element's `rufplan.paint` parameter, so every element
//! kind keeps its data unchanged; it wins over the type's outside finish in 3D, renderings
//! and elevation surface patterns.

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{ElementData, ElementId};
use crate::params::ParamValue;

/// The parameter holding an element's paint.
pub const KEY: &str = "rufplan.paint";

/// Whether `data` can be painted: walls, floors, ceilings, roofs, columns and beams.
pub fn paintable(data: &ElementData) -> bool {
    matches!(
        data,
        ElementData::Wall { .. }
            | ElementData::Floor { .. }
            | ElementData::Ceiling { .. }
            | ElementData::Roof { .. }
            | ElementData::Column { .. }
            | ElementData::Beam { .. }
    )
}

/// The material painted on `el`, if it still exists.
pub fn paint_of(doc: &Document, el: ElementId) -> Option<ElementId> {
    let ParamValue::Text(s) = doc.param(el, KEY)? else {
        return None;
    };
    let id = crate::ops::parse_id(s).ok()?;
    matches!(doc.data(id), Ok(ElementData::Material { .. })).then_some(id)
}

/// What shows on an element's outside: its paint, else its type's finish.
pub fn surface_material(doc: &Document, el: ElementId) -> Option<ElementId> {
    paint_of(doc, el).or_else(|| crate::library::finish_of(doc, el))
}

/// Paints `elements` with `material` (or removes their paint with None), one undo step.
/// Elements that can't be painted are skipped; returns how many were painted.
pub fn paint(
    doc: &mut Document,
    elements: &[ElementId],
    material: Option<ElementId>,
) -> CoreResult<usize> {
    let name = match material {
        Some(m) => match doc.data(m)? {
            ElementData::Material { name, .. } => Some(name.clone()),
            _ => return Err(CoreError::Invalid("pick a material to paint with".into())),
        },
        None => None,
    };
    let targets: Vec<ElementId> = elements
        .iter()
        .copied()
        .filter(|e| doc.data(*e).is_ok_and(paintable))
        .collect();
    if targets.is_empty() {
        return Err(CoreError::Invalid(
            "paint walls, floors, ceilings, roofs, columns or beams".into(),
        ));
    }
    let label = match &name {
        Some(n) => format!("Paint {n}"),
        None => "Remove paint".into(),
    };
    doc.transact(&label, |tx| {
        for e in &targets {
            tx.set_param(*e, KEY, material.map(|m| ParamValue::Text(m.to_string())))?;
        }
        Ok(targets.len())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Category;
    use crate::ops;
    use studio_geom::Pt;

    #[test]
    fn paint_overrides_the_type_finish_on_one_element() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let l1 = doc.levels()[0].0;
        let a =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let b = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(4000.0, 0.0),
            Pt::new(4000.0, 4000.0),
        )
        .unwrap();
        let brick = crate::library::add_preset(&mut doc, "masonry-red-brick").unwrap();
        let finish = crate::library::finish_of(&doc, a);
        assert_eq!(paint(&mut doc, &[a], Some(brick)).unwrap(), 1);
        assert_eq!(surface_material(&doc, a), Some(brick));
        // The other wall of the same type keeps the type's finish.
        assert_eq!(surface_material(&doc, b), finish);
        assert_eq!(
            crate::library::finish_of(&doc, a),
            finish,
            "the type is unchanged"
        );
        // One undo step; removing paint restores the finish.
        doc.undo().unwrap();
        assert_eq!(paint_of(&doc, a), None);
        paint(&mut doc, &[a, b], Some(brick)).unwrap();
        paint(&mut doc, &[a], None).unwrap();
        assert_eq!((paint_of(&doc, a), paint_of(&doc, b)), (None, Some(brick)));
        // Levels can't be painted; nor can anything with a non-material.
        assert!(paint(&mut doc, &[l1], Some(brick)).is_err());
        assert!(paint(&mut doc, &[a], Some(wt)).is_err());
        // A deleted material leaves no dangling paint.
        doc.transact("rm", |tx| tx.delete(brick).map(|_| ()))
            .unwrap();
        assert_eq!(paint_of(&doc, b), None);
    }
}
