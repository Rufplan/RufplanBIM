//! Paint (ADR-034): a material applied to one element, as Revit's Paint tool does, without
//! changing its type. Stored as the element's `rufplan.paint` parameter, so every element
//! kind keeps its data unchanged; it wins over the type's outside finish in 3D, renderings
//! and elevation surface patterns.
//!
//! Faces (ADR-096): as Revit's Paint tool, a click paints one face (a floor's top, its
//! soffit, one edge; a wall's exterior face), only its surface: the assembly's layers
//! are unchanged. The painted faces are the `rufplan.paint.faces` parameter, a list of
//! `face=material` pairs; the faces are named by studio-views `faces`.

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{ElementData, ElementId};
use crate::params::ParamValue;

/// The parameter holding an element's paint.
pub const KEY: &str = "rufplan.paint";
/// The parameter holding its painted faces (ADR-096).
pub const FACES_KEY: &str = "rufplan.paint.faces";

/// Whether `face` names a face: top, bottom, exterior, interior, start, end or edge:N.
pub fn is_face(face: &str) -> bool {
    matches!(
        face,
        "top" | "bottom" | "exterior" | "interior" | "start" | "end"
    ) || face
        .strip_prefix("edge:")
        .is_some_and(|n| n.parse::<usize>().is_ok())
}

/// A face as people say it: "Top", "Bottom (Soffit)", "Edge 3", "Exterior Face".
pub fn face_label(face: &str) -> String {
    match face {
        "top" => "Top".into(),
        "bottom" => "Bottom (Soffit)".into(),
        "exterior" => "Exterior Face".into(),
        "interior" => "Interior Face".into(),
        "start" => "Start End".into(),
        "end" => "End".into(),
        _ => face
            .strip_prefix("edge:")
            .and_then(|n| n.parse::<usize>().ok())
            .map_or_else(|| face.to_string(), |n| format!("Edge {}", n + 1)),
    }
}

/// The faces painted on `el`, in the order painted, with materials that still exist.
pub fn face_paints(doc: &Document, el: ElementId) -> Vec<(String, ElementId)> {
    let Some(ParamValue::Text(s)) = doc.param(el, FACES_KEY) else {
        return vec![];
    };
    s.split(';')
        .filter_map(|pair| {
            let (face, id) = pair.split_once('=')?;
            let id = crate::ops::parse_id(id).ok()?;
            (is_face(face) && matches!(doc.data(id), Ok(ElementData::Material { .. })))
                .then(|| (face.to_string(), id))
        })
        .collect()
}

/// The material painted on one face of `el`, if any.
pub fn face_paint(doc: &Document, el: ElementId, face: &str) -> Option<ElementId> {
    face_paints(doc, el)
        .into_iter()
        .find(|(f, _)| f == face)
        .map(|(_, m)| m)
}

/// Paints one face of `el` with `material` (or removes its paint with None), one undo
/// step. The element's layers and type are unchanged.
pub fn paint_face(
    doc: &mut Document,
    el: ElementId,
    face: &str,
    material: Option<ElementId>,
) -> CoreResult<()> {
    if !doc.data(el).is_ok_and(paintable) {
        return Err(CoreError::Invalid(
            "paint walls, floors, ceilings, roofs, columns or beams".into(),
        ));
    }
    if !is_face(face) {
        return Err(CoreError::Invalid(format!("{face} isn't a face")));
    }
    let name = match material {
        Some(m) => match doc.data(m)? {
            ElementData::Material { name, .. } => Some(name.clone()),
            _ => return Err(CoreError::Invalid("pick a material to paint with".into())),
        },
        None => None,
    };
    let mut faces: Vec<(String, ElementId)> = face_paints(doc, el)
        .into_iter()
        .filter(|(f, _)| f != face)
        .collect();
    if let Some(m) = material {
        faces.push((face.to_string(), m));
    }
    let text = faces
        .iter()
        .map(|(f, m)| format!("{f}={m}"))
        .collect::<Vec<_>>()
        .join(";");
    let label = match &name {
        Some(n) => format!("Paint {} {n}", face_label(face)),
        None => format!("Remove paint from {}", face_label(face)),
    };
    doc.transact(&label, |tx| {
        tx.set_param(
            el,
            FACES_KEY,
            (!text.is_empty()).then(|| ParamValue::Text(text.clone())),
        )
    })
}

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

/// Paints `elements` with `material` (or removes their paint with None), one undo step:
/// every face, so the faces painted one by one give way to it. Elements that can't be
/// painted are skipped; returns how many were painted.
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
            tx.set_param(*e, FACES_KEY, None)?;
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

    #[test]
    fn a_face_is_painted_alone_and_whole_paint_replaces_the_faces() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let l1 = doc.levels()[0].0;
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let wood = crate::library::add_preset(&mut doc, "siding-cedar-lap-stained").unwrap();
        let white = crate::library::add_preset(&mut doc, "plaster-stucco-white").unwrap();
        paint_face(&mut doc, w, "exterior", Some(wood)).unwrap();
        paint_face(&mut doc, w, "edge:2", Some(white)).unwrap();
        assert_eq!(
            face_paints(&doc, w),
            vec![
                ("exterior".to_string(), wood),
                ("edge:2".to_string(), white)
            ]
        );
        // Repainting a face replaces it; the element itself isn't painted.
        paint_face(&mut doc, w, "exterior", Some(white)).unwrap();
        assert_eq!(face_paint(&doc, w, "exterior"), Some(white));
        assert_eq!(paint_of(&doc, w), None);
        // One undo step each.
        doc.undo().unwrap();
        assert_eq!(face_paint(&doc, w, "exterior"), Some(wood));
        // Removing one face's paint leaves the other.
        paint_face(&mut doc, w, "exterior", None).unwrap();
        assert_eq!(face_paints(&doc, w), vec![("edge:2".to_string(), white)]);
        assert!(paint_face(&mut doc, w, "sideways", Some(wood)).is_err());
        assert!(paint_face(&mut doc, l1, "top", Some(wood)).is_err());
        // Painting the whole element paints every face.
        paint(&mut doc, &[w], Some(wood)).unwrap();
        assert!(face_paints(&doc, w).is_empty());
        assert_eq!(face_label("edge:2"), "Edge 3");
        assert_eq!(face_label("bottom"), "Bottom (Soffit)");
    }
}
