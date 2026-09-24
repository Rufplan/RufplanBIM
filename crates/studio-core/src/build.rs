//! Roofs and stairs (ADR-018): creation, derived layout and properties.

use studio_geom::Pt;

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{Category, ElementData, ElementId, StairShape};
use crate::ops::{
    choice, flag, len, non_empty, options_of, parse_id, parse_len, positive, ro, text, PropOption,
    Property,
};
use crate::units::{format_ft_in, MM_PER_FT, MM_PER_IN};

/// 6:12, the everyday residential pitch.
pub const DEFAULT_ROOF_SLOPE: f64 = 0.463_647_609_000_806_1; // atan(6/12)
pub const DEFAULT_OVERHANG: f64 = 18.0 * MM_PER_IN;
pub const DEFAULT_STAIR_WIDTH: f64 = 42.0 * MM_PER_IN;
pub const DEFAULT_TREAD: f64 = 11.0 * MM_PER_IN;
pub const DEFAULT_MAX_RISER: f64 = 7.0 * MM_PER_IN;

/// Built-in roof types.
pub(crate) fn seed_roof_types(tx: &mut Tx<'_>) {
    for (name, t) in [
        ("Asphalt Shingle on Rafters - 10\"", 10.0),
        ("Flat Membrane on Deck - 12\"", 12.0),
    ] {
        tx.insert(ElementData::RoofType {
            name: name.into(),
            thickness: t * MM_PER_IN,
            layers: crate::compound::default_type_layers(name),
        });
    }
}

/// Adds the built-in roof types to projects saved before roofs existed.
pub fn ensure_roof_types(doc: &mut Document) -> CoreResult<()> {
    if doc.count(Category::RoofType) > 0 {
        return Ok(());
    }
    doc.transact("Add roof types", |tx| {
        seed_roof_types(tx);
        Ok(())
    })
}

/// Rise over a 12" run, e.g. "6/12", with the angle.
pub fn format_slope(rad: f64) -> String {
    let rise = rad.tan() * 12.0;
    let r = (rise * 8.0).round() / 8.0;
    let rs = if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round())
    } else {
        format!("{r}")
    };
    format!("{rs}/12 ({:.1}°)", rad.to_degrees())
}

/// Parses "6/12", "6:12", "30°" or "30 deg" into radians.
pub fn parse_slope(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some((a, b)) = s.split_once(['/', ':']) {
        let rise: f64 = a.trim().parse().ok()?;
        let run: f64 = b
            .split_whitespace()
            .next()
            .unwrap_or("12")
            .trim()
            .parse()
            .ok()?;
        if run <= 0.0 {
            return None;
        }
        return Some((rise / run).atan());
    }
    let deg: f64 = s
        .trim_end_matches("deg")
        .trim_end_matches('°')
        .trim()
        .parse()
        .ok()?;
    Some(deg.to_radians())
}

/// A roof over `boundary` (eave line) with every edge sloped (a hip roof).
pub fn create_roof(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    offset: f64,
    mut boundary: Vec<Pt>,
    slope: f64,
) -> CoreResult<ElementId> {
    // Store counter-clockwise so "inward" is always the left of each edge.
    if studio_geom::signed_area(&boundary) < 0.0 {
        boundary.reverse();
    }
    let sloped = vec![slope > 0.0; boundary.len()];
    doc.transact("Create roof", |tx| {
        if tx.data(type_id)?.category() != Category::RoofType {
            return Err(CoreError::Invalid("pick a roof type".into()));
        }
        Ok(tx.insert(ElementData::Roof {
            type_id,
            level,
            offset,
            boundary,
            slope,
            sloped,
        }))
    })
}

/// Roofs over a building footprint, bearing `bearing` mm above `level` with `overhang`
/// eaves. A convex footprint gets one hipped roof. A right-angled L, T or U plan gets one
/// hipped roof per wing (overlapping rectangles), which intersect in valleys. Other shapes
/// are refused: sketch those roofs edge by edge.
pub fn create_roofs_by_footprint(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    bearing: f64,
    footprint: &[Pt],
    overhang: f64,
    slope: f64,
) -> CoreResult<Vec<ElementId>> {
    let mut ring = footprint.to_vec();
    if studio_geom::signed_area(&ring) < 0.0 {
        ring.reverse();
    }
    let parts = if studio_geom::is_convex(&ring) {
        vec![ring]
    } else {
        studio_geom::rect_cover(&ring).ok_or_else(|| {
            CoreError::Invalid(
                "this footprint isn't convex or right-angled; sketch the roof instead".into(),
            )
        })?
    };
    // Lower the eave so the roof's underside meets the wall line at the bearing height.
    let offset = bearing - slope.tan() * overhang;
    doc.transact("Create roof", |tx| {
        if tx.data(type_id)?.category() != Category::RoofType {
            return Err(CoreError::Invalid("pick a roof type".into()));
        }
        let mut ids = vec![];
        for part in parts {
            let mut part = part;
            if studio_geom::signed_area(&part) < 0.0 {
                part.reverse();
            }
            let mut boundary = studio_geom::offset_ring(&part, overhang);
            if studio_geom::signed_area(&boundary) < 0.0 {
                boundary.reverse();
            }
            let sloped = vec![slope > 0.0; boundary.len()];
            ids.push(tx.insert(ElementData::Roof {
                type_id,
                level,
                offset,
                boundary,
                slope,
                sloped,
            }));
        }
        Ok(ids)
    })
}

/// Riser count, riser height and horizontal run (mm) of a stair climbing `rise` mm.
pub fn stair_layout(rise: f64, tread: f64, max_riser: f64) -> (usize, f64, f64) {
    let n = ((rise / max_riser) - 1e-9).ceil().max(1.0) as usize;
    let riser = rise / n as f64;
    // n risers need n - 1 treads; the last riser lands on the upper floor.
    (n, riser, tread * (n.saturating_sub(1)) as f64)
}

/// The next level above `level`.
pub fn level_above(doc: &Document, level: ElementId) -> Option<ElementId> {
    let levels = doc.levels();
    let i = levels.iter().position(|l| l.0 == level)?;
    levels.get(i + 1).map(|l| l.0)
}

/// A stair from `level` to the level above, starting at `start` and climbing toward `toward`.
pub fn create_stair(
    doc: &mut Document,
    level: ElementId,
    start: Pt,
    toward: Pt,
    width: f64,
) -> CoreResult<ElementId> {
    create_stair_shaped(doc, level, start, toward, width, StairShape::Straight)
}

/// Like [`create_stair`], with an L- or U-shaped plan (half the risers in each run).
pub fn create_stair_shaped(
    doc: &mut Document,
    level: ElementId,
    start: Pt,
    toward: Pt,
    width: f64,
    shape: StairShape,
) -> CoreResult<ElementId> {
    let top = level_above(doc, level).ok_or_else(|| {
        CoreError::Invalid("add a level above this one for the stair to reach".into())
    })?;
    doc.transact("Create stair", |tx| {
        Ok(tx.insert(ElementData::Stair {
            base_level: level,
            top_level: top,
            start,
            end: toward,
            width,
            tread: DEFAULT_TREAD,
            max_riser: DEFAULT_MAX_RISER,
            shape,
            first_run: 0,
            railings: true,
        }))
    })
}

/// Shape options as (id, label).
pub const STAIR_SHAPES: [(&str, &str); 5] = [
    ("straight", "Straight"),
    ("l-left", "L-Shaped, Turning Left"),
    ("l-right", "L-Shaped, Turning Right"),
    ("u-left", "U-Shaped, Turning Left"),
    ("u-right", "U-Shaped, Turning Right"),
];

pub fn stair_shape_id(s: StairShape) -> &'static str {
    match s {
        StairShape::Straight => "straight",
        StairShape::LShaped { left: true } => "l-left",
        StairShape::LShaped { left: false } => "l-right",
        StairShape::UShaped { left: true } => "u-left",
        StairShape::UShaped { left: false } => "u-right",
    }
}

pub fn parse_stair_shape(id: &str) -> Option<StairShape> {
    Some(match id {
        "straight" => StairShape::Straight,
        "l-left" => StairShape::LShaped { left: true },
        "l-right" => StairShape::LShaped { left: false },
        "u-left" => StairShape::UShaped { left: true },
        "u-right" => StairShape::UShaped { left: false },
        _ => return None,
    })
}

/// Risers in each run of a stair with `risers` in all (one run for a straight stair).
pub fn stair_runs(shape: StairShape, risers: usize, first_run: u32) -> Vec<usize> {
    match shape {
        StairShape::Straight => vec![risers],
        _ => {
            let first = if first_run == 0 {
                risers / 2
            } else {
                (first_run as usize).clamp(2, risers.saturating_sub(2).max(2))
            };
            vec![first, risers - first]
        }
    }
}

fn level_choice(doc: &Document, key: &str, label: &str, cur: ElementId) -> Property {
    let opts: Vec<PropOption> = doc
        .levels()
        .into_iter()
        .map(|(id, name, _)| PropOption {
            id: id.to_string(),
            label: name,
        })
        .collect();
    choice(key, label, "Constraints", cur.to_string(), opts)
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(data) = doc.data(id) else { return };
    match data {
        ElementData::RoofType {
            name,
            thickness,
            layers,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(len("thickness", "Thickness", "Construction", *thickness));
            crate::compound::layer_properties_in(layers, props, crate::compound::GROUP_TOP_DOWN);
        }
        ElementData::Roof {
            level,
            offset,
            boundary,
            slope,
            sloped,
            ..
        } => {
            props.push(level_choice(doc, "level", "Base Level", *level));
            props.push(len(
                "offset",
                "Base Offset From Level",
                "Constraints",
                *offset,
            ));
            props.push(text("slope", "Slope", "Dimensions", &format_slope(*slope)));
            for (i, s) in sloped.iter().enumerate() {
                let a = boundary[i];
                let b = boundary[(i + 1) % boundary.len()];
                props.push(flag(
                    &format!("edge:{i}"),
                    &format!("Edge {} ({}) Defines Slope", i + 1, format_ft_in(a.dist(b))),
                    "Edges",
                    *s,
                ));
            }
        }
        ElementData::Stair {
            base_level,
            top_level,
            width,
            tread,
            max_riser,
            shape,
            first_run,
            railings,
            ..
        } => {
            props.push(choice(
                "shape",
                "Shape",
                "Construction",
                stair_shape_id(*shape).into(),
                STAIR_SHAPES
                    .iter()
                    .map(|(id, label)| PropOption {
                        id: (*id).into(),
                        label: (*label).into(),
                    })
                    .collect(),
            ));
            if *shape != StairShape::Straight {
                props.push(text(
                    "first_run",
                    "Risers in First Run (0 = half)",
                    "Construction",
                    &first_run.to_string(),
                ));
            }
            props.push(flag("railings", "Railings", "Construction", *railings));
            props.push(level_choice(doc, "base_level", "Base Level", *base_level));
            props.push(level_choice(doc, "top_level", "Top Level", *top_level));
            props.push(len("width", "Actual Run Width", "Dimensions", *width));
            props.push(len("tread", "Actual Tread Depth", "Dimensions", *tread));
            props.push(len(
                "max_riser",
                "Maximum Riser Height",
                "Dimensions",
                *max_riser,
            ));
            let rise = doc.level_elevation(*top_level).unwrap_or(0.0)
                - doc.level_elevation(*base_level).unwrap_or(0.0);
            let (n, r, run) = stair_layout(rise, *tread, *max_riser);
            props.push(ro(
                "risers",
                "Actual Number of Risers",
                "Dimensions",
                n.to_string(),
            ));
            props.push(ro(
                "riser",
                "Actual Riser Height",
                "Dimensions",
                format_ft_in(r),
            ));
            props.push(ro("run", "Run Length", "Dimensions", format_ft_in(run)));
        }
        _ => {}
    }
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
        ElementData::RoofType {
            name,
            thickness,
            layers,
        } => match key {
            k if k.starts_with("layer") => {
                crate::compound::set_layer_property(layers, *thickness, k, value)?;
                if !layers.is_empty() {
                    *thickness = layers.iter().map(|l| l.thickness).sum();
                }
            }
            "name" => *name = non_empty(value)?,
            "thickness" => {
                let w = positive(parse_len(value)?)?;
                crate::compound::resize_structure(layers, w)?;
                *thickness = w;
            }
            _ => return Err(unknown()),
        },
        ElementData::Roof {
            type_id,
            level,
            offset,
            slope,
            sloped,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "level" => *level = parse_id(value)?,
            "offset" => *offset = parse_len(value)?,
            "slope" => {
                *slope = parse_slope(value).ok_or_else(|| {
                    CoreError::Invalid(format!("\"{value}\" is not a slope (try 6/12 or 30°)"))
                })?
            }
            k if k.starts_with("edge:") => {
                let i: usize = k["edge:".len()..].parse().map_err(|_| unknown())?;
                let s = sloped.get_mut(i).ok_or_else(unknown)?;
                *s = value == "yes";
            }
            _ => return Err(unknown()),
        },
        ElementData::Stair {
            base_level,
            top_level,
            width,
            tread,
            max_riser,
            shape,
            first_run,
            railings,
            ..
        } => match key {
            "shape" => {
                *shape = parse_stair_shape(value)
                    .ok_or_else(|| CoreError::Invalid(format!("unknown stair shape {value}")))?
            }
            "first_run" => {
                *first_run = value
                    .trim()
                    .parse()
                    .map_err(|_| CoreError::Invalid(format!("\"{value}\" is not a riser count")))?
            }
            "railings" => *railings = value == "yes",
            "base_level" => *base_level = parse_id(value)?,
            "top_level" => *top_level = parse_id(value)?,
            "width" => *width = positive(parse_len(value)?)?,
            "tread" => *tread = positive(parse_len(value)?)?,
            "max_riser" => *max_riser = positive(parse_len(value)?)?,
            _ => return Err(unknown()),
        },
        _ => return Err(unknown()),
    }
    if let ElementData::Stair {
        base_level,
        top_level,
        ..
    } = &d
    {
        let (b, t) = (
            doc.level_elevation(*base_level)?,
            doc.level_elevation(*top_level)?,
        );
        if t - b < 300.0 {
            return Err(CoreError::Invalid(
                "the top level must be above the base level".into(),
            ));
        }
    }
    let label = format!(
        "Change {}",
        key.split(':').next().unwrap_or(key).replace('_', " ")
    );
    doc.transact(&label, |tx| {
        if let Some(t) = d.type_id() {
            if tx.data(t)?.category() != Category::RoofType {
                return Err(CoreError::Invalid(
                    "that type belongs to another category".into(),
                ));
            }
        }
        tx.set(id, d)
    })
}

/// Default roof type (the first, by name).
pub fn default_roof_type(doc: &Document) -> Option<ElementId> {
    options_of(doc, Category::RoofType)
        .first()
        .and_then(|o| o.id.parse().ok())
}

/// 1'-0" — for readers of this module's defaults.
pub const FOOT: f64 = MM_PER_FT;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    #[test]
    fn slopes_parse_and_format() {
        assert!((parse_slope("6/12").unwrap() - DEFAULT_ROOF_SLOPE).abs() < 1e-12);
        assert!((parse_slope("30°").unwrap() - 30f64.to_radians()).abs() < 1e-12);
        assert_eq!(format_slope(DEFAULT_ROOF_SLOPE), "6/12 (26.6°)");
        assert_eq!(parse_slope("x"), None);
        assert_eq!(parse_slope("4/0"), None);
    }

    #[test]
    fn stair_layout_fits_the_rise() {
        // 10'-0" rise at 7" max: 18 risers of 6 2/3", 17 treads of 11".
        let (n, r, run) = stair_layout(120.0 * MM_PER_IN, DEFAULT_TREAD, DEFAULT_MAX_RISER);
        assert_eq!(n, 18);
        assert!((r - 120.0 / 18.0 * MM_PER_IN).abs() < 1e-9);
        assert!((run - 17.0 * 11.0 * MM_PER_IN).abs() < 1e-9);
        // Exactly divisible: 7 risers of 7".
        assert_eq!(
            stair_layout(49.0 * MM_PER_IN, DEFAULT_TREAD, DEFAULT_MAX_RISER).0,
            7
        );
    }

    #[test]
    fn roofs_and_stairs_are_created_and_edited() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l = doc.levels();
        let rt = default_roof_type(&doc).unwrap();
        // Clockwise boundary is stored counter-clockwise.
        let b = vec![
            Pt::new(0.0, 0.0),
            Pt::new(0.0, 6000.0),
            Pt::new(9000.0, 6000.0),
            Pt::new(9000.0, 0.0),
        ];
        let roof = create_roof(&mut doc, rt, l[1].0, 3000.0, b, DEFAULT_ROOF_SLOPE).unwrap();
        match doc.data(roof).unwrap() {
            ElementData::Roof {
                boundary, sloped, ..
            } => {
                assert!(studio_geom::signed_area(boundary) > 0.0);
                assert_eq!(sloped, &vec![true; 4]);
            }
            _ => unreachable!(),
        }
        ops::set_property(&mut doc, roof, "edge:1", "no", 0).unwrap();
        ops::set_property(&mut doc, roof, "slope", "8/12", 0).unwrap();
        let props = ops::properties(&doc, roof).unwrap();
        assert!(props
            .properties
            .iter()
            .any(|p| p.key == "slope" && p.value.starts_with("8/12")));
        assert!(ops::set_property(&mut doc, roof, "slope", "90°", 0).is_err());

        let stair = create_stair(
            &mut doc,
            l[0].0,
            Pt::new(0.0, 0.0),
            Pt::new(0.0, 100.0),
            DEFAULT_STAIR_WIDTH,
        )
        .unwrap();
        let props = ops::properties(&doc, stair).unwrap();
        assert!(props
            .properties
            .iter()
            .any(|p| p.key == "risers" && p.value == "18"));
        assert!(ops::set_property(&mut doc, stair, "top_level", &l[0].0.to_string(), 0).is_err());
        assert!(
            create_stair(
                &mut doc,
                l[1].0,
                Pt::new(0.0, 0.0),
                Pt::new(1.0, 0.0),
                DEFAULT_STAIR_WIDTH
            )
            .is_err(),
            "no level above the top level"
        );
    }

    #[test]
    fn roofs_by_footprint_split_l_plans_into_wings() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let l2 = doc.levels()[1].0;
        let rt = default_roof_type(&doc).unwrap();
        let ft = MM_PER_FT;
        let l_plan = [
            Pt::new(0.0, 0.0),
            Pt::new(40.0 * ft, 0.0),
            Pt::new(40.0 * ft, 16.0 * ft),
            Pt::new(16.0 * ft, 16.0 * ft),
            Pt::new(16.0 * ft, 36.0 * ft),
            Pt::new(0.0, 36.0 * ft),
        ];
        let ids = create_roofs_by_footprint(
            &mut doc,
            rt,
            l2,
            0.0,
            &l_plan,
            DEFAULT_OVERHANG,
            DEFAULT_ROOF_SLOPE,
        )
        .unwrap();
        assert_eq!(ids.len(), 2, "main block and wing");
        for id in &ids {
            let Ok(ElementData::Roof {
                boundary, offset, ..
            }) = doc.data(*id)
            else {
                panic!("not a roof");
            };
            assert_eq!(boundary.len(), 4);
            assert!(studio_geom::signed_area(boundary) > 0.0);
            assert!(
                (offset + DEFAULT_OVERHANG * 0.5).abs() < 1e-9,
                "6:12 over 18\" drops 9\""
            );
        }
        let square = [
            Pt::new(0.0, 0.0),
            Pt::new(0.0, 20.0 * ft),
            Pt::new(20.0 * ft, 20.0 * ft),
            Pt::new(20.0 * ft, 0.0),
        ];
        let one = create_roofs_by_footprint(
            &mut doc,
            rt,
            l2,
            0.0,
            &square,
            DEFAULT_OVERHANG,
            DEFAULT_ROOF_SLOPE,
        )
        .unwrap();
        let Ok(ElementData::Roof { boundary, .. }) = doc.data(one[0]) else {
            panic!("not a roof");
        };
        let area = studio_geom::signed_area(boundary);
        assert!((area - (23.0 * ft).powi(2)).abs() < 1.0, "{area}");
        let star = [
            Pt::new(0.0, 0.0),
            Pt::new(5000.0, 1000.0),
            Pt::new(10000.0, 0.0),
            Pt::new(5000.0, 8000.0),
        ];
        assert!(create_roofs_by_footprint(&mut doc, rt, l2, 0.0, &star, 0.0, 0.4).is_err());
    }
}
