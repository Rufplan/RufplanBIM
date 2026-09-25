//! Edits that need the regenerated model (ADR-020): creating floors and ceilings bound to
//! walls, detaching them, and turning on a 3D view's section box around the model.

use studio_core::{
    ops, CoreError, CoreResult, Document, ElementData, ElementId, SectionBox, SlabBound,
};
use studio_geom::{Poly, Pt};

use crate::regenerate;

/// Properties handled here rather than in \`studio_core::ops::set_property\`.
pub fn handles(key: &str, value: &str) -> bool {
    matches!((key, value), ("section_box", "yes") | ("bound", "sketch"))
}

/// Sets a property that needs the model. Call when [`handles`] is true.
pub fn set_property(doc: &mut Document, id: ElementId, key: &str, value: &str) -> CoreResult<()> {
    match (key, value) {
        ("section_box", "yes") => {
            let b = model_box(doc)?;
            set_section_box(doc, id, Some(b))
        }
        ("bound", "sketch") => {
            let ring = current_outline(doc, id)
                .ok_or_else(|| CoreError::Invalid("that isn't a floor or ceiling".into()))?;
            ops::set_slab_bound(doc, id, SlabBound::Sketch, Some(ring))
        }
        _ => ops::set_property(doc, id, key, value, 0),
    }
}

/// The model's extents with a 1'-0" margin, as a section box.
pub fn model_box(doc: &Document) -> CoreResult<SectionBox> {
    let m = regenerate(doc);
    let (lo, hi) = m
        .plan_bounds()
        .ok_or_else(|| CoreError::Invalid("model something first".into()))?;
    let (z0, z1) = m.z_range();
    let pad = 304.8;
    Ok(SectionBox {
        min: [lo.x - pad, lo.y - pad, z0 - pad],
        max: [hi.x + pad, hi.y + pad, z1 + pad],
    })
}

/// Sets (or clears) a 3D view's section box.
pub fn set_section_box(
    doc: &mut Document,
    view: ElementId,
    b: Option<SectionBox>,
) -> CoreResult<()> {
    if !matches!(
        doc.data(view)?,
        ElementData::View {
            kind: studio_core::ViewKind::ThreeD,
            ..
        }
    ) {
        return Err(CoreError::Invalid(
            "section boxes belong to 3D views".into(),
        ));
    }
    let label = if b.is_some() {
        "Change section box"
    } else {
        "Remove section box"
    };
    doc.transact(label, |tx| {
        tx.modify(view, |d| {
            if let ElementData::View { section_box, .. } = d {
                *section_box = b;
            }
        })
    })
}

/// The outline a floor or ceiling has in the model right now (its largest part).
fn current_outline(doc: &Document, id: ElementId) -> Option<Vec<Pt>> {
    let m = regenerate(doc);
    m.floors
        .iter()
        .chain(&m.ceilings)
        .filter(|s| s.id == id)
        .max_by(|a, b| a.base.area().total_cmp(&b.base.area()))
        .map(|s| s.base.outer.clone())
}

/// Floor: Pick Walls. A floor at the outer faces of the level's walls that keeps
/// following them.
pub fn create_floor_by_walls(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
) -> CoreResult<ElementId> {
    let m = regenerate(doc);
    let ring = crate::outer_boundary(&m, level).ok_or_else(|| {
        CoreError::Invalid("draw walls on this level first, or sketch the floor boundary".into())
    })?;
    ops::create_floor_by_walls(doc, type_id, level, ring)
}

/// Ceiling: Auto Room. A ceiling filling the room around `inside` that keeps following its
/// walls.
pub fn create_ceiling_in_room(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    inside: Pt,
) -> CoreResult<ElementId> {
    let m = regenerate(doc);
    let ring = crate::room_at(&m, level, inside)
        .ok_or_else(|| CoreError::Invalid("click inside a room fully enclosed by walls".into()))?;
    ops::create_ceiling_in_room(doc, type_id, level, ring, inside)
}

/// Area of a slab outline with its holes (mm²), for reports.
pub fn net_area(p: &Poly) -> f64 {
    p.area()
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::units::MM_PER_FT;
    use studio_core::Category;

    fn house() -> (Document, ElementId, Vec<ElementId>) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let ft = |x: f64, y: f64| Pt::new(x * MM_PER_FT, y * MM_PER_FT);
        let c = [ft(0.0, 0.0), ft(40.0, 0.0), ft(40.0, 30.0), ft(0.0, 30.0)];
        let walls = (0..4)
            .map(|i| ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
            .collect();
        (doc, l1, walls)
    }

    #[test]
    fn bound_floors_and_ceilings_follow_moved_walls_until_detached() {
        let (mut doc, l1, walls) = house();
        let ftype = ops::first_of(&doc, Category::FloorType).unwrap();
        let ctype = ops::first_of(&doc, Category::CeilingType).unwrap();
        let floor = create_floor_by_walls(&mut doc, ftype, l1).unwrap();
        let ceiling = create_ceiling_in_room(&mut doc, ctype, l1, Pt::new(3000.0, 3000.0)).unwrap();
        let area = |doc: &Document, id| {
            regenerate(doc)
                .floors
                .iter()
                .chain(&regenerate(doc).ceilings)
                .find(|s| s.id == id)
                .map(|s| s.base.area())
                .unwrap()
        };
        // Within 0.001 m² (the polygon union rounds on a fine grid).
        let (h, ft) = (4.0 * 25.4, MM_PER_FT);
        assert!((area(&doc, floor) - (40.0 * ft + 2.0 * h) * (30.0 * ft + 2.0 * h)).abs() < 1000.0);
        assert!(
            (area(&doc, ceiling) - (40.0 * ft - 2.0 * h) * (30.0 * ft - 2.0 * h)).abs() < 1000.0
        );
        // Push the east wall out 5': both follow.
        studio_core::modify::move_elements(&mut doc, &[walls[1]], Pt::new(5.0 * ft, 0.0)).unwrap();
        assert!((area(&doc, floor) - (45.0 * ft + 2.0 * h) * (30.0 * ft + 2.0 * h)).abs() < 1000.0);
        assert!(
            (area(&doc, ceiling) - (45.0 * ft - 2.0 * h) * (30.0 * ft - 2.0 * h)).abs() < 1000.0
        );
        // Detached, the floor keeps its current shape when the wall moves back.
        set_property(&mut doc, floor, "bound", "sketch").unwrap();
        studio_core::modify::move_elements(&mut doc, &[walls[1]], Pt::new(-5.0 * ft, 0.0)).unwrap();
        assert!((area(&doc, floor) - (45.0 * ft + 2.0 * h) * (30.0 * ft + 2.0 * h)).abs() < 1000.0);
        assert!(
            (area(&doc, ceiling) - (40.0 * ft - 2.0 * h) * (30.0 * ft - 2.0 * h)).abs() < 1000.0
        );
        // Re-attached, it follows again.
        ops::set_property(&mut doc, floor, "bound", "walls", 0).unwrap();
        assert!((area(&doc, floor) - (40.0 * ft + 2.0 * h) * (30.0 * ft + 2.0 * h)).abs() < 1000.0);
        assert!(*regenerate(&doc) == crate::regenerate_full(&doc));
    }

    #[test]
    fn section_box_starts_around_the_model_and_edits_by_face() {
        let (mut doc, _, _) = house();
        let v3d = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: studio_core::ViewKind::ThreeD,
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        assert!(handles("section_box", "yes"));
        set_property(&mut doc, v3d, "section_box", "yes").unwrap();
        let get = |doc: &Document| match doc.data(v3d).unwrap() {
            ElementData::View { section_box, .. } => *section_box,
            _ => None,
        };
        let b = get(&doc).unwrap();
        let ft = MM_PER_FT;
        assert!((b.min[0] - (-4.0 * 25.4 - 304.8)).abs() < 1e-6);
        assert!((b.max[1] - (30.0 * ft + 4.0 * 25.4 + 304.8)).abs() < 1e-6);
        ops::set_property(&mut doc, v3d, "box_max_2", "6'", 0).unwrap();
        assert!((get(&doc).unwrap().max[2] - 6.0 * ft).abs() < 1e-6);
        ops::set_property(&mut doc, v3d, "section_box", "no", 0).unwrap();
        assert!(get(&doc).is_none());
        assert!(ops::set_property(&mut doc, v3d, "box_max_2", "6'", 0).is_err());
    }
}
