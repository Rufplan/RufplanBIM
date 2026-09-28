//! Painted grass (ADR-065), after D5 Render's grass brush: grass painted onto the ground
//! (or a floor or roof) in strokes of round dabs, in one of D5's grass kinds (lawn, lush
//! lawn, meadow, wild grass, dry grass, clover, tall grass), each with its height, height
//! variation, density and colour. Its 3D blades are drawn in Realistic views and renders.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::lighting::{num, opts, parse_num};
use crate::ops::{choice, len, level_options, parse_len, positive, ro, text, Property};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum GrassKind {
    Lawn,
    LushLawn,
    Meadow,
    WildGrass,
    DryGrass,
    Clover,
    TallGrass,
}

impl GrassKind {
    pub const ALL: [GrassKind; 7] = [
        GrassKind::Lawn,
        GrassKind::LushLawn,
        GrassKind::Meadow,
        GrassKind::WildGrass,
        GrassKind::DryGrass,
        GrassKind::Clover,
        GrassKind::TallGrass,
    ];
    pub fn label(self) -> &'static str {
        match self {
            GrassKind::Lawn => "Lawn",
            GrassKind::LushLawn => "Lush Lawn",
            GrassKind::Meadow => "Meadow with Flowers",
            GrassKind::WildGrass => "Wild Grass",
            GrassKind::DryGrass => "Dry Grass",
            GrassKind::Clover => "Clover",
            GrassKind::TallGrass => "Tall Grass",
        }
    }
    /// The kind's own look: height (mm), height variation, density and colour.
    pub fn spec(self) -> GrassSpec {
        let (height, variation, density, color) = match self {
            GrassKind::Lawn => (70.0, 0.35, 1.0, [86, 124, 54]),
            GrassKind::LushLawn => (95.0, 0.3, 1.25, [70, 118, 44]),
            GrassKind::Meadow => (320.0, 0.55, 0.9, [104, 132, 62]),
            GrassKind::WildGrass => (420.0, 0.65, 0.8, [112, 128, 66]),
            GrassKind::DryGrass => (260.0, 0.5, 0.8, [168, 150, 92]),
            GrassKind::Clover => (80.0, 0.3, 1.1, [76, 122, 58]),
            GrassKind::TallGrass => (750.0, 0.45, 0.7, [110, 134, 70]),
        };
        GrassSpec {
            kind: self,
            height,
            variation,
            density,
            color,
        }
    }
}

/// How painted grass grows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GrassSpec {
    pub kind: GrassKind,
    /// Blade height, mm.
    pub height: f64,
    /// 0 (even) to 1 (very uneven).
    pub variation: f64,
    /// Relative to the kind's natural density, 0.2 to 2.
    pub density: f64,
    /// sRGB.
    pub color: [u8; 3],
}

impl GrassSpec {
    pub fn check(&self) -> CoreResult<()> {
        if !(10.0..=3000.0).contains(&self.height) {
            return Err(CoreError::Invalid(
                "grass is between 1/2\" and 10' tall".into(),
            ));
        }
        Ok(())
    }
}

/// A dab of the brush: its centre (project mm, z the surface painted) and radius.
pub type Dab = [f64; 4];

/// Paints a stroke of dabs on `level` (one undo step).
pub fn paint(
    doc: &mut Document,
    level: ElementId,
    dabs: &[Dab],
    spec: GrassSpec,
) -> CoreResult<ElementId> {
    spec.check()?;
    doc.level_elevation(level)?;
    let dabs: Vec<Dab> = dabs
        .iter()
        .copied()
        .filter(|d| d.iter().all(|v| v.is_finite()) && d[3] > 1.0)
        .collect();
    if dabs.is_empty() {
        return Err(CoreError::Invalid("paint where there's ground".into()));
    }
    doc.transact("Paint grass", |tx| {
        Ok(tx.insert(ElementData::GrassPatch {
            level,
            dabs: dabs.clone(),
            spec,
        }))
    })
}

/// Erases painted grass under a stroke (one undo step): dabs whose centres the stroke
/// covers go; patches left with none are deleted.
pub fn erase(doc: &mut Document, dabs: &[Dab]) -> CoreResult<usize> {
    let hit = |d: &Dab| {
        dabs.iter()
            .any(|e| (d[0] - e[0]).hypot(d[1] - e[1]) < e[3] + d[3] * 0.25)
    };
    let mut changes: Vec<(ElementId, Vec<Dab>)> = vec![];
    for e in doc.of(Category::GrassPatch) {
        if let ElementData::GrassPatch { dabs: mine, .. } = &e.data {
            if mine.iter().any(hit) {
                changes.push((e.id, mine.iter().copied().filter(|d| !hit(d)).collect()));
            }
        }
    }
    if changes.is_empty() {
        return Ok(0);
    }
    let n = changes.len();
    doc.transact("Erase grass", |tx| {
        for (id, left) in changes {
            if left.is_empty() {
                tx.delete(id)?;
            } else {
                tx.modify(id, |d| {
                    if let ElementData::GrassPatch { dabs, .. } = d {
                        *dabs = left.clone();
                    }
                })?;
            }
        }
        Ok(())
    })?;
    Ok(n)
}

/// The patch's area as rings (each dab a 24-sided circle), in plan.
pub fn dab_rings(dabs: &[Dab]) -> Vec<Vec<Pt>> {
    dabs.iter()
        .map(|d| {
            (0..24)
                .map(|k| {
                    let a = k as f64 / 24.0 * std::f64::consts::TAU;
                    Pt::new(d[0] + a.cos() * d[3], d[1] + a.sin() * d[3])
                })
                .collect()
        })
        .collect()
}

const FT: f64 = 304.8;

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(ElementData::GrassPatch { level, dabs, spec }) = doc.data(id) else {
        return;
    };
    const G: &str = "Grass";
    props.push(choice(
        "kind",
        "Grass",
        G,
        format!("{:?}", spec.kind),
        opts(&GrassKind::ALL, |k| format!("{k:?}"), GrassKind::label),
    ));
    props.push(len("height", "Height", G, spec.height));
    props.push(num(
        "variation",
        "Height Variation",
        G,
        (spec.variation * 100.0).round(),
        "%",
    ));
    props.push(num(
        "density",
        "Density",
        G,
        (spec.density * 100.0).round(),
        "%",
    ));
    props.push(text(
        "color",
        "Colour",
        G,
        &format!(
            "#{:02x}{:02x}{:02x}",
            spec.color[0], spec.color[1], spec.color[2]
        ),
    ));
    props.push(choice(
        "level",
        "Level",
        "Constraints",
        level.to_string(),
        level_options(doc),
    ));
    let area: f64 = dabs
        .iter()
        .map(|d| std::f64::consts::PI * d[3] * d[3])
        .sum();
    props.push(ro(
        "area",
        "Area (painted, approx.)",
        "Dimensions",
        format!("{:.0} SF", area / (FT * FT)),
    ));
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let mut d = doc.data(id)?.clone();
    let unknown = || CoreError::Invalid(format!("unknown property {key}"));
    let ElementData::GrassPatch { level, spec, .. } = &mut d else {
        return Err(unknown());
    };
    match key {
        "kind" => {
            let k = *GrassKind::ALL
                .iter()
                .find(|k| format!("{k:?}") == value.trim())
                .ok_or_else(unknown)?;
            // A new kind brings its own look.
            *spec = k.spec();
        }
        "height" => spec.height = positive(parse_len(value)?)?,
        "variation" => spec.variation = (parse_num(value, "%")? / 100.0).clamp(0.0, 1.0),
        "density" => spec.density = (parse_num(value, "%")? / 100.0).clamp(0.2, 2.0),
        "color" => {
            let s = value.trim().trim_start_matches('#');
            let b = |i: usize| {
                s.get(i..i + 2)
                    .and_then(|x| u8::from_str_radix(x, 16).ok())
                    .ok_or_else(|| CoreError::Invalid(format!("\"{value}\" is not a colour")))
            };
            spec.color = [b(0)?, b(2)?, b(4)?];
        }
        "level" => {
            let l = crate::ops::parse_id(value)?;
            doc.level_elevation(l)?;
            *level = l;
        }
        _ => return Err(unknown()),
    }
    spec.check()?;
    let label = format!("Change {}", key.replace('_', " "));
    doc.transact(&label, |tx| tx.set(id, d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    #[test]
    fn grass_paints_in_strokes_erases_and_edits() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let stroke: Vec<Dab> = (0..5)
            .map(|k| [f64::from(k) * 500.0, 0.0, 0.0, 600.0])
            .collect();
        let id = paint(&mut doc, l1, &stroke, GrassKind::Meadow.spec()).unwrap();
        let Ok(ElementData::GrassPatch { dabs, spec, .. }) = doc.data(id) else {
            panic!()
        };
        assert_eq!(dabs.len(), 5);
        assert_eq!(spec.kind, GrassKind::Meadow);
        assert!(paint(&mut doc, l1, &[], GrassKind::Lawn.spec()).is_err());
        // Erasing the middle keeps the ends; erasing the rest deletes the patch.
        assert_eq!(erase(&mut doc, &[[1000.0, 0.0, 0.0, 300.0]]).unwrap(), 1);
        let Ok(ElementData::GrassPatch { dabs, .. }) = doc.data(id) else {
            panic!()
        };
        assert_eq!(dabs.len(), 4);
        erase(&mut doc, &[[1000.0, 0.0, 0.0, 5000.0]]).unwrap();
        assert!(doc.data(id).is_err());
        doc.undo().unwrap();
        // Properties: the kind brings its own height; height edits.
        ops::set_property(&mut doc, id, "kind", "TallGrass", 0).unwrap();
        let Ok(ElementData::GrassPatch { spec, .. }) = doc.data(id) else {
            panic!()
        };
        assert!((spec.height - 750.0).abs() < 1e-9);
        ops::set_property(&mut doc, id, "height", "4\"", 0).unwrap();
        ops::set_property(&mut doc, id, "density", "150%", 0).unwrap();
        let Ok(ElementData::GrassPatch { spec, .. }) = doc.data(id) else {
            panic!()
        };
        assert!((spec.height - 101.6).abs() < 1e-6);
        assert!((spec.density - 1.5).abs() < 1e-9);
        assert!(ops::set_property(&mut doc, id, "height", "20'", 0).is_err());
        // It moves with Move.
        crate::modify::move_elements(&mut doc, &[id], Pt::new(1000.0, 0.0)).unwrap();
        let Ok(ElementData::GrassPatch { dabs, .. }) = doc.data(id) else {
            panic!()
        };
        assert!((dabs[0][0] - 1000.0).abs() < 1e-9);
        assert_eq!(dab_rings(&dabs[..1])[0].len(), 24);
    }
}
