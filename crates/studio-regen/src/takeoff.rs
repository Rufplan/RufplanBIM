//! Material takeoff (ADR-020): area and volume of each material across walls, floors,
//! ceilings, roofs, columns and beams.

use std::collections::BTreeMap;

use studio_core::material::{resolve, resolve_type};
use studio_core::{Document, ElementData, ElementId, WallLayer};

use crate::Model;

/// One material's totals (mm² and mm³). Columns and beams add volume only.
#[derive(Debug, Clone, PartialEq)]
pub struct TakeoffRow {
    pub material: String,
    pub area: f64,
    pub volume: f64,
}

fn layers_of(doc: &Document, element: ElementId) -> (Vec<WallLayer>, String) {
    let Some(t) = doc.data(element).ok().and_then(|d| d.type_id()) else {
        return (vec![], String::new());
    };
    match doc.data(t) {
        Ok(
            ElementData::WallType { name, layers, .. }
            | ElementData::FloorType { name, layers, .. }
            | ElementData::CeilingType { name, layers, .. }
            | ElementData::RoofType { name, layers, .. },
        ) => (layers.clone(), name.clone()),
        Ok(d) => (vec![], d.name()),
        Err(_) => (vec![], String::new()),
    }
}

/// Totals per material, sorted by name.
pub fn material_takeoff(doc: &Document, model: &Model) -> Vec<TakeoffRow> {
    let mut rows: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut add = |name: String, area: f64, volume: f64| {
        let r = rows.entry(name).or_default();
        r.0 += area;
        r.1 += volume;
    };
    // A build-up of `area` and total thickness `total`: each layer adds its share.
    let mut build_up = |doc: &Document, id: ElementId, area: f64, total: f64| {
        let (layers, type_name) = layers_of(doc, id);
        if layers.is_empty() {
            add(resolve_type(doc, None, &type_name).name, area, area * total);
        }
        for l in &layers {
            add(resolve(doc, l).name, area, area * l.thickness);
        }
    };
    for w in &model.walls {
        // Face area from the material volume, so openings and joins are accounted for.
        let v: f64 = w.pieces.iter().map(|p| p.base.area() * (p.z1 - p.z0)).sum();
        if w.thickness > 0.0 {
            build_up(doc, w.id, v / w.thickness, w.thickness);
        }
    }
    for s in model.floors.iter().chain(&model.ceilings) {
        build_up(doc, s.id, s.base.area(), s.z1 - s.z0);
    }
    for r in &model.roofs {
        let plan = studio_geom::signed_area(&r.boundary).abs();
        let area = if r.is_flat() {
            plan
        } else {
            plan / r.slope.cos()
        };
        build_up(doc, r.id, area, r.thickness);
    }
    for c in &model.columns {
        let (m, name) = type_material(doc, c.id);
        add(
            resolve_type(doc, m, &name).name,
            0.0,
            c.base.area() * (c.z1 - c.z0),
        );
    }
    for b in &model.beams {
        let (m, name) = type_material(doc, b.id);
        let v = b.prisms.iter().map(|p| p.base.area() * (p.z1 - p.z0)).sum();
        add(resolve_type(doc, m, &name).name, 0.0, v);
    }
    rows.into_iter()
        .map(|(material, (area, volume))| TakeoffRow {
            material,
            area,
            volume,
        })
        .collect()
}

fn type_material(doc: &Document, id: ElementId) -> (Option<ElementId>, String) {
    let t = doc.data(id).ok().and_then(|d| d.type_id());
    match t.and_then(|t| doc.data(t).ok()) {
        Some(
            ElementData::ColumnType { name, material, .. }
            | ElementData::BeamType { name, material, .. },
        ) => (*material, name.clone()),
        _ => (None, String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::units::{MM_PER_FT, MM_PER_IN};
    use studio_core::{ops, Category};
    use studio_geom::Pt;

    #[test]
    fn a_wall_and_a_slab_break_down_by_layer() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let cmu = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 12"))
            .unwrap()
            .id;
        let ft = MM_PER_FT;
        ops::create_wall(
            &mut doc,
            cmu,
            l1,
            Pt::new(0.0, 0.0),
            Pt::new(20.0 * ft, 0.0),
        )
        .unwrap();
        let slab = doc
            .of(Category::FloorType)
            .find(|e| e.data.name().starts_with("Concrete Slab"))
            .unwrap()
            .id;
        let sq = vec![
            Pt::new(0.0, 1000.0),
            Pt::new(10.0 * ft, 1000.0),
            Pt::new(10.0 * ft, 1000.0 + 10.0 * ft),
            Pt::new(0.0, 1000.0 + 10.0 * ft),
        ];
        ops::create_floor(&mut doc, slab, l1, sq).unwrap();
        let m = crate::regenerate(&doc);
        let rows = material_takeoff(&doc, &m);
        let get = |n: &str| rows.iter().find(|r| r.material == n).unwrap();
        // 20' × 10' of wall face.
        let face = 20.0 * ft * 10.0 * ft;
        let block = get("Concrete Masonry Unit");
        assert!((block.area - face).abs() < 1.0, "{}", block.area);
        assert!((block.volume - face * 7.625 * MM_PER_IN).abs() < 1.0e3);
        // The 6" slab's 100 sf.
        let c = get("Concrete");
        assert!((c.area - 100.0 * ft * ft).abs() < 1.0);
        assert!((c.volume - 100.0 * ft * ft * 6.0 * MM_PER_IN).abs() < 1.0e3);
        assert!(
            rows.windows(2).all(|w| w[0].material < w[1].material),
            "sorted"
        );
    }
}
