//! Columns, beams and railings (ADR-019): built-in types, placement and properties.

use studio_geom::{project_to_segment, Pt};

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{BeamShape, Category, ColumnShape, ElementData, ElementId, WallTop};
use crate::ops::{
    choice, flag, len, level_options, non_empty, options_of, parse_id, parse_len, positive, ro,
    text, PropOption, Property, DEFAULT_WALL_HEIGHT,
};
use crate::units::{format_ft_in, MM_PER_IN};

fn inch(v: f64) -> f64 {
    v * MM_PER_IN
}

/// Built-in column, beam and railing types (AISC sizes for the steel shapes).
pub(crate) fn seed_structure_types(tx: &mut Tx<'_>) {
    let columns = [
        (
            "Concrete Square - 12\" x 12\"",
            ColumnShape::Rectangular {
                width: inch(12.0),
                depth: inch(12.0),
            },
            true,
        ),
        (
            "Concrete Round - 16\"",
            ColumnShape::Round {
                diameter: inch(16.0),
            },
            true,
        ),
        (
            "Steel W10x33",
            ColumnShape::WideFlange {
                depth: inch(9.73),
                flange: inch(7.96),
                flange_t: inch(0.435),
                web_t: inch(0.29),
            },
            true,
        ),
        (
            "Wood Post - 6x6",
            ColumnShape::Rectangular {
                width: inch(5.5),
                depth: inch(5.5),
            },
            true,
        ),
        (
            "Architectural Round - 12\"",
            ColumnShape::Round {
                diameter: inch(12.0),
            },
            false,
        ),
    ];
    for (name, shape, structural) in columns {
        tx.insert(ElementData::ColumnType {
            name: name.into(),
            shape,
            structural,
            material: None,
        });
    }
    let beams = [
        (
            "Steel W12x26",
            BeamShape::WideFlange {
                depth: inch(12.22),
                flange: inch(6.49),
                flange_t: inch(0.38),
                web_t: inch(0.23),
            },
        ),
        (
            "Steel W8x18",
            BeamShape::WideFlange {
                depth: inch(8.14),
                flange: inch(5.25),
                flange_t: inch(0.33),
                web_t: inch(0.23),
            },
        ),
        (
            "Glulam - 5 1/8\" x 12\"",
            BeamShape::Rectangular {
                width: inch(5.125),
                depth: inch(12.0),
            },
        ),
        (
            "Concrete - 12\" x 18\"",
            BeamShape::Rectangular {
                width: inch(12.0),
                depth: inch(18.0),
            },
        ),
    ];
    for (name, shape) in beams {
        tx.insert(ElementData::BeamType {
            name: name.into(),
            shape,
            material: None,
        });
    }
    for (name, h) in [("Guardrail - 42\"", 42.0), ("Handrail - 36\"", 36.0)] {
        tx.insert(ElementData::RailingType {
            name: name.into(),
            height: inch(h),
        });
    }
}

/// Adds the built-in structure and railing types to projects saved before they existed.
pub fn ensure_structure_types(doc: &mut Document) -> CoreResult<()> {
    if doc.count(Category::ColumnType) > 0 {
        return Ok(());
    }
    doc.transact("Add structure types", |tx| {
        seed_structure_types(tx);
        Ok(())
    })
}

/// The first type (by name) of a category, e.g. the default column type.
pub fn default_type(doc: &Document, cat: Category) -> Option<ElementId> {
    options_of(doc, cat).first().and_then(|o| o.id.parse().ok())
}

fn check_type(tx: &Tx<'_>, type_id: ElementId, want: Category) -> CoreResult<()> {
    if tx.data(type_id)?.category() != want {
        return Err(CoreError::Invalid(format!(
            "pick a {} type",
            want.as_str().trim_end_matches("Type").to_lowercase()
        )));
    }
    Ok(())
}

fn column_top(doc: &Document, level: ElementId) -> WallTop {
    match crate::build::level_above(doc, level) {
        Some(l) => WallTop::UpToLevel {
            level: l,
            offset: 0.0,
        },
        None => WallTop::Unconnected {
            height: DEFAULT_WALL_HEIGHT,
        },
    }
}

/// A column at `at` from `level` up to the level above (or 10'-0" unconnected).
pub fn create_column(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    at: Pt,
    rotation: f64,
) -> CoreResult<ElementId> {
    let top = column_top(doc, level);
    doc.transact("Place column", |tx| {
        check_type(tx, type_id, Category::ColumnType)?;
        Ok(tx.insert(ElementData::Column {
            type_id,
            base_level: level,
            base_offset: 0.0,
            top,
            at,
            rotation,
        }))
    })
}

/// Grid intersections (points where two grid segments cross).
pub fn grid_intersections(doc: &Document) -> Vec<Pt> {
    let grids: Vec<(Pt, Pt)> = doc
        .of(Category::Grid)
        .filter_map(|e| match &e.data {
            ElementData::Grid { start, end, .. } => Some((*start, *end)),
            _ => None,
        })
        .collect();
    let mut out: Vec<Pt> = vec![];
    for i in 0..grids.len() {
        for j in (i + 1)..grids.len() {
            let ((a, b), (c, d)) = (grids[i], grids[j]);
            let Some(x) = studio_geom::line_intersection(a, b.sub(a), c, d.sub(c)) else {
                continue;
            };
            let on = |p: Pt, q: Pt| project_to_segment(x, p, q).1 < 0.5;
            if on(a, b) && on(c, d) && !out.iter().any(|o| o.dist(x) < 1.0) {
                out.push(x);
            }
        }
    }
    out
}

/// Revit's Column "At Grids": a column at every grid intersection on `level` that doesn't
/// already have one. Returns the new columns.
pub fn columns_at_grids(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
) -> CoreResult<Vec<ElementId>> {
    let existing: Vec<Pt> = doc
        .of(Category::Column)
        .filter_map(|e| match &e.data {
            ElementData::Column { base_level, at, .. } if *base_level == level => Some(*at),
            _ => None,
        })
        .collect();
    let points: Vec<Pt> = grid_intersections(doc)
        .into_iter()
        .filter(|p| !existing.iter().any(|q| q.dist(*p) < 1.0))
        .collect();
    if points.is_empty() {
        return Err(CoreError::Invalid(
            "every grid intersection already has a column (or there are no grids)".into(),
        ));
    }
    let top = column_top(doc, level);
    doc.transact("Columns at grids", |tx| {
        check_type(tx, type_id, Category::ColumnType)?;
        Ok(points
            .into_iter()
            .map(|at| {
                tx.insert(ElementData::Column {
                    type_id,
                    base_level: level,
                    base_offset: 0.0,
                    top: top.clone(),
                    at,
                    rotation: 0.0,
                })
            })
            .collect())
    })
}

/// A beam from `start` to `end` with its top at `level`.
pub fn create_beam(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    start: Pt,
    end: Pt,
) -> CoreResult<ElementId> {
    doc.transact("Place beam", |tx| {
        check_type(tx, type_id, Category::BeamType)?;
        Ok(tx.insert(ElementData::Beam {
            type_id,
            level,
            offset: 0.0,
            start,
            end,
        }))
    })
}

/// A railing along `path` on `level`.
pub fn create_railing(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    path: Vec<Pt>,
) -> CoreResult<ElementId> {
    doc.transact("Place railing", |tx| {
        check_type(tx, type_id, Category::RailingType)?;
        Ok(tx.insert(ElementData::Railing {
            type_id,
            level,
            offset: 0.0,
            path,
        }))
    })
}

fn top_props(doc: &Document, top: &WallTop, props: &mut Vec<Property>) {
    let mut opts = vec![PropOption {
        id: "unconnected".into(),
        label: "Unconnected".into(),
    }];
    opts.extend(level_options(doc).into_iter().map(|o| PropOption {
        label: format!("Up to level: {}", o.label),
        ..o
    }));
    match top {
        WallTop::UpToLevel { level, offset } => {
            props.push(choice(
                "top",
                "Top Level",
                "Constraints",
                level.to_string(),
                opts,
            ));
            props.push(len("top_offset", "Top Offset", "Constraints", *offset));
        }
        WallTop::Unconnected { height } => {
            props.push(choice(
                "top",
                "Top Level",
                "Constraints",
                "unconnected".into(),
                opts,
            ));
            props.push(len("height", "Unconnected Height", "Constraints", *height));
        }
    }
}

fn level_choice(doc: &Document, key: &str, label: &str, cur: ElementId) -> Property {
    choice(
        key,
        label,
        "Constraints",
        cur.to_string(),
        level_options(doc),
    )
}

fn column_shape_props(shape: &ColumnShape, props: &mut Vec<Property>) {
    const G: &str = "Dimensions";
    match shape {
        ColumnShape::Rectangular { width, depth } => {
            props.push(len("width", "b (Width)", G, *width));
            props.push(len("depth", "h (Depth)", G, *depth));
        }
        ColumnShape::Round { diameter } => props.push(len("diameter", "Diameter", G, *diameter)),
        ColumnShape::WideFlange {
            depth,
            flange,
            flange_t,
            web_t,
        } => wf_props(*depth, *flange, *flange_t, *web_t, props),
    }
}

fn material_row(doc: &Document, material: Option<ElementId>, props: &mut Vec<Property>) {
    props.push(crate::ops::choice(
        "material",
        "Material",
        "Materials and Finishes",
        material.map(|m| m.to_string()).unwrap_or_default(),
        crate::material::options(doc),
    ));
}

fn parse_material(doc: &Document, value: &str) -> CoreResult<Option<ElementId>> {
    if value.is_empty() {
        return Ok(None);
    }
    let id = crate::ops::parse_id(value)?;
    crate::material::name_of(doc, id)
        .map(|_| Some(id))
        .ok_or_else(|| CoreError::Invalid("pick a material".into()))
}

fn wf_props(depth: f64, flange: f64, flange_t: f64, web_t: f64, props: &mut Vec<Property>) {
    const G: &str = "Dimensions";
    props.push(len("depth", "d (Depth)", G, depth));
    props.push(len("flange", "bf (Flange Width)", G, flange));
    props.push(len("flange_t", "tf (Flange Thickness)", G, flange_t));
    props.push(len("web_t", "tw (Web Thickness)", G, web_t));
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(data) = doc.data(id) else { return };
    match data {
        ElementData::ColumnType {
            name,
            shape,
            structural,
            material,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            column_shape_props(shape, props);
            props.push(flag("structural", "Structural", "Structural", *structural));
            material_row(doc, *material, props);
        }
        ElementData::BeamType {
            name,
            shape,
            material,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            material_row(doc, *material, props);
            match shape {
                BeamShape::Rectangular { width, depth } => {
                    props.push(len("width", "b (Width)", "Dimensions", *width));
                    props.push(len("depth", "h (Depth)", "Dimensions", *depth));
                }
                BeamShape::WideFlange {
                    depth,
                    flange,
                    flange_t,
                    web_t,
                } => wf_props(*depth, *flange, *flange_t, *web_t, props),
            }
        }
        ElementData::RailingType { name, height } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(len("height", "Railing Height", "Construction", *height));
        }
        ElementData::Column {
            base_level,
            base_offset,
            top,
            rotation,
            at,
            ..
        } => {
            props.push(crate::ops::ro(
                "location_mark",
                "Column Location Mark",
                "Identity Data",
                crate::detail::column_mark(doc, *at),
            ));
            props.push(level_choice(doc, "base_level", "Base Level", *base_level));
            props.push(len(
                "base_offset",
                "Base Offset",
                "Constraints",
                *base_offset,
            ));
            top_props(doc, top, props);
            props.push(text(
                "rotation",
                "Rotation (degrees)",
                "Constraints",
                &format!("{:.1}", rotation.to_degrees()),
            ));
        }
        ElementData::Beam {
            level,
            offset,
            start,
            end,
            ..
        } => {
            props.push(level_choice(doc, "level", "Reference Level", *level));
            props.push(len("offset", "Top Offset", "Constraints", *offset));
            props.push(ro(
                "length",
                "Length",
                "Dimensions",
                format_ft_in(start.dist(*end)),
            ));
        }
        ElementData::Railing {
            level,
            offset,
            path,
            ..
        } => {
            props.push(level_choice(doc, "level", "Base Level", *level));
            props.push(len("offset", "Base Offset", "Constraints", *offset));
            let l: f64 = path.windows(2).map(|w| w[0].dist(w[1])).sum();
            props.push(ro("length", "Length", "Dimensions", format_ft_in(l)));
        }
        _ => {}
    }
}

fn set_dim(v: &mut f64, value: &str) -> CoreResult<()> {
    *v = positive(parse_len(value)?)?;
    Ok(())
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let mut d = doc.data(id)?.clone();
    let unknown = || CoreError::Invalid(format!("unknown property {key}"));
    match &mut d {
        ElementData::ColumnType {
            name,
            shape,
            structural,
            material,
        } => match (key, shape) {
            ("name", _) => *name = non_empty(value)?,
            ("material", _) => *material = parse_material(doc, value)?,
            ("structural", _) => *structural = value == "yes",
            ("width", ColumnShape::Rectangular { width, .. }) => set_dim(width, value)?,
            ("depth", ColumnShape::Rectangular { depth, .. }) => set_dim(depth, value)?,
            ("diameter", ColumnShape::Round { diameter }) => set_dim(diameter, value)?,
            ("depth", ColumnShape::WideFlange { depth, .. }) => set_dim(depth, value)?,
            ("flange", ColumnShape::WideFlange { flange, .. }) => set_dim(flange, value)?,
            ("flange_t", ColumnShape::WideFlange { flange_t, .. }) => set_dim(flange_t, value)?,
            ("web_t", ColumnShape::WideFlange { web_t, .. }) => set_dim(web_t, value)?,
            _ => return Err(unknown()),
        },
        ElementData::BeamType {
            name,
            shape,
            material,
        } => match (key, shape) {
            ("name", _) => *name = non_empty(value)?,
            ("material", _) => *material = parse_material(doc, value)?,
            ("width", BeamShape::Rectangular { width, .. }) => set_dim(width, value)?,
            ("depth", BeamShape::Rectangular { depth, .. }) => set_dim(depth, value)?,
            ("depth", BeamShape::WideFlange { depth, .. }) => set_dim(depth, value)?,
            ("flange", BeamShape::WideFlange { flange, .. }) => set_dim(flange, value)?,
            ("flange_t", BeamShape::WideFlange { flange_t, .. }) => set_dim(flange_t, value)?,
            ("web_t", BeamShape::WideFlange { web_t, .. }) => set_dim(web_t, value)?,
            _ => return Err(unknown()),
        },
        ElementData::RailingType { name, height } => match key {
            "name" => *name = non_empty(value)?,
            "height" => set_dim(height, value)?,
            _ => return Err(unknown()),
        },
        ElementData::Column {
            type_id,
            base_level,
            base_offset,
            top,
            rotation,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "base_level" => *base_level = parse_id(value)?,
            "base_offset" => *base_offset = parse_len(value)?,
            "top" => {
                *top = if value == "unconnected" {
                    WallTop::Unconnected {
                        height: DEFAULT_WALL_HEIGHT,
                    }
                } else {
                    WallTop::UpToLevel {
                        level: parse_id(value)?,
                        offset: 0.0,
                    }
                }
            }
            "top_offset" => {
                if let WallTop::UpToLevel { offset, .. } = top {
                    *offset = parse_len(value)?;
                }
            }
            "height" => {
                if let WallTop::Unconnected { height } = top {
                    set_dim(height, value)?;
                }
            }
            "rotation" => {
                let deg: f64 = value
                    .trim()
                    .trim_end_matches('°')
                    .parse()
                    .map_err(|_| CoreError::Invalid(format!("\"{value}\" is not an angle")))?;
                *rotation = deg.to_radians();
            }
            _ => return Err(unknown()),
        },
        ElementData::Beam {
            type_id,
            level,
            offset,
            ..
        }
        | ElementData::Railing {
            type_id,
            level,
            offset,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "level" => *level = parse_id(value)?,
            "offset" => *offset = parse_len(value)?,
            _ => return Err(unknown()),
        },
        _ => return Err(unknown()),
    }
    let want = match &d {
        ElementData::Column { .. } => Some(Category::ColumnType),
        ElementData::Beam { .. } => Some(Category::BeamType),
        ElementData::Railing { .. } => Some(Category::RailingType),
        _ => None,
    };
    let label = format!("Change {}", key.replace('_', " "));
    doc.transact(&label, |tx| {
        if let (Some(want), Some(t)) = (want, d.type_id()) {
            check_type(tx, t, want)?;
        }
        tx.set(id, d)
    })
}

/// Attaches (or detaches) the tops of the given walls to the roofs above them, in one
/// undo step. Returns how many walls changed; other elements are ignored.
pub fn attach_wall_tops(doc: &mut Document, ids: &[ElementId], attach: bool) -> CoreResult<usize> {
    let walls: Vec<ElementId> = ids
        .iter()
        .copied()
        .filter(|id| matches!(doc.data(*id), Ok(ElementData::Wall { attach_top, .. }) if *attach_top != attach))
        .collect();
    if walls.is_empty() {
        return Ok(0);
    }
    let label = if attach { "Attach top" } else { "Detach top" };
    doc.transact(label, |tx| {
        for id in &walls {
            tx.modify(*id, |d| {
                if let ElementData::Wall { attach_top, .. } = d {
                    *attach_top = attach;
                }
            })?;
        }
        Ok(walls.len())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use crate::units::MM_PER_FT;

    fn project() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        (doc, l1)
    }

    #[test]
    fn columns_at_every_grid_intersection_once() {
        let (mut doc, l1) = project();
        for x in [0.0, 20.0, 40.0] {
            ops::create_grid(
                &mut doc,
                Pt::new(x * MM_PER_FT, -5000.0),
                Pt::new(x * MM_PER_FT, 20000.0),
            )
            .unwrap();
        }
        for y in [0.0, 30.0] {
            ops::create_grid(
                &mut doc,
                Pt::new(-5000.0, y * MM_PER_FT),
                Pt::new(20000.0, y * MM_PER_FT),
            )
            .unwrap();
        }
        let ct = default_type(&doc, Category::ColumnType).unwrap();
        let made = columns_at_grids(&mut doc, ct, l1).unwrap();
        assert_eq!(made.len(), 6);
        assert!(columns_at_grids(&mut doc, ct, l1).is_err(), "no duplicates");
        // Columns reach the level above.
        match doc.data(made[0]).unwrap() {
            ElementData::Column {
                top: WallTop::UpToLevel { .. },
                ..
            } => {}
            other => panic!("{other:?}"),
        }
        let props = ops::properties(&doc, made[0]).unwrap();
        assert!(props.properties.iter().any(|p| p.key == "rotation"));
        ops::set_property(&mut doc, made[0], "rotation", "45", 0).unwrap();
    }

    #[test]
    fn beam_and_railing_types_and_edits() {
        let (mut doc, l1) = project();
        let bt = doc
            .of(Category::BeamType)
            .find(|e| e.data.name() == "Steel W12x26")
            .unwrap()
            .id;
        let b = create_beam(&mut doc, bt, l1, Pt::new(0.0, 0.0), Pt::new(6000.0, 0.0)).unwrap();
        ops::set_property(&mut doc, b, "offset", "-1'", 0).unwrap();
        assert!(create_beam(&mut doc, bt, l1, Pt::new(0.0, 0.0), Pt::new(1.0, 0.0)).is_err());
        ops::set_property(&mut doc, bt, "depth", "14\"", 0).unwrap();
        assert!(ops::set_property(&mut doc, bt, "diameter", "1'", 0).is_err());
        assert!(
            ops::set_property(&mut doc, bt, "flange_t", "1'", 0).is_err(),
            "flanges too thick"
        );
        let rt = default_type(&doc, Category::RailingType).unwrap();
        let r = create_railing(
            &mut doc,
            rt,
            l1,
            vec![
                Pt::new(0.0, 0.0),
                Pt::new(3000.0, 0.0),
                Pt::new(3000.0, 2000.0),
            ],
        )
        .unwrap();
        let props = ops::properties(&doc, r).unwrap();
        assert!(props
            .properties
            .iter()
            .any(|p| p.key == "length" && p.value == "16'-4 7/8\""));
        let ct = default_type(&doc, Category::ColumnType).unwrap();
        assert!(create_beam(&mut doc, ct, l1, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).is_err());
    }
    #[test]
    fn attach_and_detach_wall_tops_in_one_step() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = crate::ops::first_of(&doc, Category::WallType).unwrap();
        let a = crate::ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(3000.0, 0.0))
            .unwrap();
        let b = crate::ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(0.0, 3000.0))
            .unwrap();
        let top = |doc: &Document, id| {
            matches!(
                doc.data(id),
                Ok(ElementData::Wall {
                    attach_top: true,
                    ..
                })
            )
        };
        assert_eq!(attach_wall_tops(&mut doc, &[a, b, l1], true).unwrap(), 2);
        assert!(top(&doc, a) && top(&doc, b));
        assert_eq!(
            attach_wall_tops(&mut doc, &[a], true).unwrap(),
            0,
            "already attached"
        );
        doc.undo().unwrap();
        assert!(!top(&doc, a) && !top(&doc, b), "one undo step");
    }
}
