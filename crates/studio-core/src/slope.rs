//! Slopes (ADR-049): floors that fall toward a direction (sidewalks, ramps), and spot slope
//! annotations that read the slope of roofs, floors and the ground. Slopes are kept as rise
//! over run (the tangent); this module reads and writes them the ways architects do.

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{ElementData, ElementId, SlopeFormat, ViewKind};
use crate::ops::{choice, text, PropOption, Property};
use crate::units::parse_length;
use studio_geom::Pt;

/// What a slope was read from; `Auto` formats each the usual way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlopeSource {
    Roof,
    Floor,
    Ground,
}

/// A 1:20 slope; steeper walking surfaces are ramps.
pub const RAMP: f64 = 1.0 / 20.0;

/// Inches with eighths: `6"`, `1/4"`, `1 1/2"`.
fn inches(v: f64) -> String {
    let eighths = (v * 8.0).round() as i64;
    let (whole, mut num) = (eighths / 8, eighths % 8);
    let mut den = 8;
    while num > 0 && num % 2 == 0 {
        num /= 2;
        den /= 2;
    }
    match (whole, num) {
        (w, 0) => format!("{w}\""),
        (0, n) => format!("{n}/{den}\""),
        (w, n) => format!("{w} {n}/{den}\""),
    }
}

fn trim(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A slope (rise over run) as `format` writes it; `Auto` picks by what it was read from.
pub fn format_slope(tan: f64, format: SlopeFormat, source: SlopeSource) -> String {
    let tan = tan.abs();
    let format = match format {
        SlopeFormat::Auto => match source {
            SlopeSource::Roof => SlopeFormat::RisePer12,
            SlopeSource::Floor if tan >= RAMP - 1e-9 => SlopeFormat::Ratio,
            _ => SlopeFormat::Percent,
        },
        f => f,
    };
    match format {
        SlopeFormat::RisePer12 | SlopeFormat::Auto => format!("{} / 12\"", inches(tan * 12.0)),
        SlopeFormat::Percent => format!("{:.2}%", tan * 100.0),
        SlopeFormat::Ratio if tan < 1e-9 => "0".into(),
        SlopeFormat::Ratio => format!("1:{}", trim(1.0 / tan)),
        SlopeFormat::Degrees => format!("{:.2}°", tan.atan().to_degrees()),
    }
}

/// Reads a slope typed as `2%`, `1:12`, `1/4" / 12"`, `6/12` or `5°`; blank or 0 is flat.
pub fn parse_slope(input: &str) -> CoreResult<f64> {
    let bad = || {
        CoreError::Invalid(format!(
            "can't read the slope {input:?} (try 2%, 1:12, 1/4\"/12\" or 5°)"
        ))
    };
    let s = input.trim();
    let num = |v: &str| v.trim().parse::<f64>().map_err(|_| bad());
    if s.is_empty() || s == "0" {
        return Ok(0.0);
    }
    let tan = if let Some(p) = s.strip_suffix('%') {
        num(p)? / 100.0
    } else if let Some(d) = s.strip_suffix('°').or_else(|| s.strip_suffix("deg")) {
        num(d)?.to_radians().tan()
    } else if let Some((rise, run)) = s.split_once(':') {
        num(rise)? / num(run)?
    } else if let Some(i) = s.find("\"/").or_else(|| s.find("\" /")) {
        // 1/4"/12": two lengths, the rise ending with its inch mark.
        let len = |v: &str| parse_length(v.trim()).ok_or_else(bad);
        len(&s[..=i])? / len(s[i + 1..].trim_start_matches(['/', ' ']))?
    } else if let Some((rise, run)) = s.rsplit_once('/') {
        num(rise)? / num(run)?
    } else {
        return Err(bad());
    };
    if !tan.is_finite() || !(0.0..=10.0).contains(&tan.abs()) {
        return Err(bad());
    }
    Ok(tan.abs())
}

/// The eight directions a floor can fall toward, as (label, degrees from east).
pub const DIRECTIONS: [(&str, f64); 8] = [
    ("North", 90.0),
    ("Northeast", 45.0),
    ("East", 0.0),
    ("Southeast", -45.0),
    ("South", -90.0),
    ("Southwest", -135.0),
    ("West", 180.0),
    ("Northwest", 135.0),
];

/// A floor's slope rows: its slope and the direction it falls toward.
pub(crate) fn floor_properties(slope: &crate::element::FloorSlope, props: &mut Vec<Property>) {
    props.push(text(
        "slope",
        "Slope",
        "Constraints",
        &if slope.is_flat() {
            "0".to_owned()
        } else {
            format_slope(slope.rise, SlopeFormat::Auto, SlopeSource::Floor)
        },
    ));
    let deg = slope.dir.to_degrees();
    let same = |a: f64| ((a - deg + 540.0).rem_euclid(360.0) - 180.0).abs() < 0.5;
    let mut options: Vec<PropOption> = DIRECTIONS
        .iter()
        .map(|(l, d)| PropOption {
            id: d.to_string(),
            label: (*l).into(),
        })
        .collect();
    let current = match DIRECTIONS.iter().find(|(_, d)| same(*d)) {
        Some((_, d)) => d.to_string(),
        None => {
            options.push(PropOption {
                id: format!("{deg:.1}"),
                label: format!("{deg:.1}°"),
            });
            format!("{deg:.1}")
        }
    };
    props.push(choice(
        "slope_dir",
        "Slopes Down Toward",
        "Constraints",
        current,
        options,
    ));
}

const FORMATS: [(SlopeFormat, &str, &str); 5] = [
    (
        SlopeFormat::Auto,
        "auto",
        "Auto (roofs rise/12\", paving %)",
    ),
    (SlopeFormat::RisePer12, "rise12", "Rise / 12\""),
    (SlopeFormat::Percent, "percent", "Percent"),
    (SlopeFormat::Ratio, "ratio", "Ratio (1:12)"),
    (SlopeFormat::Degrees, "degrees", "Degrees"),
];

/// A spot slope's rows: how it writes the slope, and arrow or triangle.
pub(crate) fn spot_properties(format: SlopeFormat, triangle: bool, props: &mut Vec<Property>) {
    let key = FORMATS
        .iter()
        .find(|f| f.0 == format)
        .map_or("auto", |f| f.1);
    props.push(choice(
        "format",
        "Slope Format",
        "Graphics",
        key.into(),
        FORMATS
            .iter()
            .map(|(_, id, l)| PropOption {
                id: (*id).into(),
                label: (*l).into(),
            })
            .collect(),
    ));
    props.push(choice(
        "triangle",
        "Representation",
        "Graphics",
        if triangle { "triangle" } else { "arrow" }.into(),
        vec![
            PropOption {
                id: "arrow".into(),
                label: "Arrow".into(),
            },
            PropOption {
                id: "triangle".into(),
                label: "Triangle (elevations and sections)".into(),
            },
        ],
    ));
}

/// Sets a spot slope's format or representation.
pub(crate) fn set_spot(
    format: &mut SlopeFormat,
    triangle: &mut bool,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    match key {
        "format" => {
            *format = FORMATS
                .iter()
                .find(|f| f.1 == value)
                .map(|f| f.0)
                .ok_or_else(|| CoreError::Invalid(format!("no slope format {value}")))?
        }
        "triangle" => *triangle = value == "triangle",
        _ => return Err(CoreError::Invalid(format!("unknown property {key}"))),
    }
    Ok(())
}

/// Sets a floor's slope or direction.
pub(crate) fn set_floor(
    slope: &mut crate::element::FloorSlope,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    match key {
        "slope" => slope.rise = parse_slope(value)?,
        "slope_dir" => {
            let deg: f64 = value
                .trim()
                .trim_end_matches('°')
                .parse()
                .map_err(|_| CoreError::Invalid("bad direction".into()))?;
            slope.dir = deg.to_radians();
        }
        _ => return Err(CoreError::Invalid(format!("unknown property {key}"))),
    }
    Ok(())
}

/// Places a spot slope at `at` in a plan, elevation or section. Whether there is a slope
/// there is the caller's to check (it needs the regenerated model).
pub fn create_spot_slope(doc: &mut Document, view: ElementId, at: Pt) -> CoreResult<ElementId> {
    match doc.data(view)? {
        ElementData::View {
            kind:
                ViewKind::FloorPlan { .. }
                | ViewKind::Elevation { .. }
                | ViewKind::Section { .. }
                | ViewKind::MarkerElevation { .. },
            ..
        } => {}
        _ => {
            return Err(CoreError::Invalid(
                "spot slopes go in plans, elevations and sections".into(),
            ))
        }
    }
    doc.transact("Place spot slope", |tx| {
        Ok(tx.insert(ElementData::SpotSlope {
            view,
            at,
            format: SlopeFormat::Auto,
            triangle: false,
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slopes_read_and_write_as_architects_write_them() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(close(parse_slope("2%").unwrap(), 0.02));
        assert!(close(parse_slope("1:12").unwrap(), 1.0 / 12.0));
        assert!(close(parse_slope("1/4\"/12\"").unwrap(), 0.25 / 12.0));
        assert!(close(parse_slope("1/4\" / 12\"").unwrap(), 0.25 / 12.0));
        assert!(close(parse_slope("6/12").unwrap(), 0.5));
        assert!(close(parse_slope("45°").unwrap(), 1.0));
        assert_eq!(parse_slope("").unwrap(), 0.0);
        assert!(parse_slope("steep").is_err());
        let (roof, floor, ground) = (SlopeSource::Roof, SlopeSource::Floor, SlopeSource::Ground);
        let a = SlopeFormat::Auto;
        assert_eq!(format_slope(0.5, a, roof), "6\" / 12\"");
        assert_eq!(format_slope(0.25 / 12.0, a, roof), "1/4\" / 12\"");
        assert_eq!(format_slope(1.5 / 12.0, a, roof), "1 1/2\" / 12\"");
        // A 1:12 ramp as a ratio, a sidewalk's cross slope as a percent.
        assert_eq!(format_slope(1.0 / 12.0, a, floor), "1:12");
        assert_eq!(format_slope(0.02, a, floor), "2.00%");
        assert_eq!(format_slope(0.05, a, ground), "5.00%");
        assert_eq!(format_slope(1.0, SlopeFormat::Degrees, roof), "45.00°");
        assert_eq!(
            format_slope(1.0 / 12.0, SlopeFormat::Percent, roof),
            "8.33%"
        );
    }

    #[test]
    fn floors_slope_through_properties() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let ft = crate::ops::first_of(&doc, crate::element::Category::FloorType).unwrap();
        let sq = vec![
            Pt::new(0.0, 0.0),
            Pt::new(3000.0, 0.0),
            Pt::new(3000.0, 1500.0),
            Pt::new(0.0, 1500.0),
        ];
        let f = crate::ops::create_floor(&mut doc, ft, l1, sq).unwrap();
        crate::ops::set_property(&mut doc, f, "slope", "1:12", 0).unwrap();
        crate::ops::set_property(&mut doc, f, "slope_dir", "0", 0).unwrap();
        let Ok(ElementData::Floor { slope, .. }) = doc.data(f) else {
            panic!()
        };
        assert!((slope.rise - 1.0 / 12.0).abs() < 1e-12 && slope.down().x > 0.999);
        let props = crate::ops::properties(&doc, f).unwrap();
        let row = |k: &str| {
            props
                .properties
                .iter()
                .find(|p| p.key == k)
                .unwrap()
                .value
                .clone()
        };
        assert_eq!(row("slope"), "1:12");
        assert_eq!(row("slope_dir"), "0");
        doc.undo().unwrap();
        doc.undo().unwrap();
        assert!(matches!(doc.data(f), Ok(ElementData::Floor { slope, .. }) if slope.is_flat()));
    }
}
