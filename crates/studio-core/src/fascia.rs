//! Roof fascias (ADR-095): the trim around a roof's edge. A profile is a stack of
//! rectangles in the plane square to the edge, `out` measured outward from the roof edge
//! and `z` from the roof's top surface at the edge; the roof sweeps it around its outline
//! (mitred at the corners, following the rakes of a sloped roof).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::units::MM_PER_IN;

/// One rectangle of a fascia profile (mm): from `out0` to `out1` outward of the roof edge,
/// and from `z0` to `z1` relative to the roof's top surface there.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FasciaPart {
    pub out0: f64,
    pub out1: f64,
    pub z0: f64,
    pub z1: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FasciaSpec {
    pub name: String,
    /// What it is and where it's typical.
    pub description: String,
    pub parts: Vec<FasciaPart>,
    /// Finish color (sRGB).
    pub color: [u8; 3],
    /// A material library preset for the finish ("metal-matte-black"…), or empty.
    pub finish: String,
}

impl FasciaSpec {
    /// Its overall height and projection (mm).
    pub fn extent(&self) -> (f64, f64) {
        let z0 = self
            .parts
            .iter()
            .map(|p| p.z0)
            .fold(f64::INFINITY, f64::min);
        let z1 = self
            .parts
            .iter()
            .map(|p| p.z1)
            .fold(f64::NEG_INFINITY, f64::max);
        let o = self.parts.iter().map(|p| p.out1).fold(0.0, f64::max);
        (z1 - z0, o)
    }
}

const WHITE: [u8; 3] = [244, 243, 238];
const BLACK: [u8; 3] = [32, 32, 34];
const BRONZE: [u8; 3] = [70, 58, 46];
const CEDAR: [u8; 3] = [168, 112, 70];
const CONCRETE: [u8; 3] = [178, 176, 170];
const ZINC: [u8; 3] = [124, 128, 130];

/// Parts in inches: (out0, out1, z0, z1).
type Row = (
    &'static str,
    &'static str,
    &'static [(f64, f64, f64, f64)],
    [u8; 3],
    &'static str,
);

const PROFILES: &[Row] = &[
    (
        "Modern Stepped Band 12\"",
        "A deep painted band with a slim cap stepping out at the top: the crisp edge of a modern flat roof.",
        &[(0.0, 1.5, -10.0, 0.75), (0.0, 2.25, 0.75, 2.5)],
        WHITE,
        "",
    ),
    (
        "Modern Flat Band 8\"",
        "A plain painted band standing 2\" above the roof: minimal flat-roof edge.",
        &[(0.0, 1.0, -6.0, 2.0)],
        WHITE,
        "",
    ),
    (
        "Deep Band 16\"",
        "A tall, heavy painted band for a strong horizontal roof line.",
        &[(0.0, 1.5, -14.0, 2.0)],
        WHITE,
        "",
    ),
    (
        "Double Band 14\"",
        "Two bands, the lower stepping out: a layered modern cornice.",
        &[(0.0, 1.0, -6.0, 2.0), (0.0, 2.0, -14.0, -6.0)],
        WHITE,
        "",
    ),
    (
        "Knife Edge 3\"",
        "A thin dark metal edge: the roof reads as a floating plane.",
        &[(0.0, 0.5, -2.0, 1.0)],
        BLACK,
        "metal-matte-black",
    ),
    (
        "Tapered Soffit Edge",
        "A thin edge over a 1' soffit plate: a sharp overhang with no visible depth.",
        &[(0.0, 1.0, -2.0, 1.0), (-12.0, 1.0, -3.0, -2.0)],
        WHITE,
        "",
    ),
    (
        "Gravel Stop 6\"",
        "Formed sheet-metal gravel stop on a built-up or membrane roof.",
        &[(0.0, 0.75, -4.0, 2.0)],
        ZINC,
        "metal-galvanized",
    ),
    (
        "Metal Coping",
        "Sheet-metal coping wrapping the roof edge, sloped cap and drip face.",
        &[(-6.0, 1.5, 4.0, 5.0), (1.0, 1.5, -2.0, 4.0)],
        BRONZE,
        "metal-anodized-bronze",
    ),
    (
        "1x6 Wood Fascia",
        "A painted 1x6 board: the standard fascia on a sloped residential roof.",
        &[(0.0, 0.75, -5.5, 0.0)],
        WHITE,
        "",
    ),
    (
        "1x8 Wood Fascia",
        "A painted 1x8 board, for deeper rafters and trusses.",
        &[(0.0, 0.75, -7.25, 0.0)],
        WHITE,
        "",
    ),
    (
        "2x10 Fascia with Drip Edge",
        "A 2x10 sub-fascia with a metal drip edge lapping its top.",
        &[(0.0, 1.5, -9.25, 0.0), (0.0, 2.0, -0.75, 0.25)],
        WHITE,
        "",
    ),
    (
        "Fascia with K-Style Gutter 5\"",
        "A 1x8 fascia carrying a 5\" ogee-front aluminum gutter.",
        &[
            (0.0, 0.75, -7.25, 0.0),
            (0.75, 5.75, -5.0, -4.75),
            (5.25, 5.75, -5.0, 0.0),
            (5.0, 6.0, -1.0, 0.0),
        ],
        WHITE,
        "",
    ),
    (
        "Fascia with Half-Round Gutter 6\"",
        "A 1x8 fascia with a 6\" half-round gutter: traditional and craftsman homes.",
        &[
            (0.0, 0.75, -7.25, 0.0),
            (0.75, 6.75, -3.5, -2.0),
            (1.5, 6.0, -2.0, -0.5),
        ],
        ZINC,
        "metal-galvanized",
    ),
    (
        "Box Gutter Cornice",
        "A built-in box gutter in a wood cornice: historic and commercial buildings.",
        &[(0.0, 8.0, -10.0, -1.0), (6.5, 8.0, -1.0, 0.5)],
        WHITE,
        "",
    ),
    (
        "Crown Molding Cornice",
        "A fascia with a stepped crown molding: classical and colonial homes.",
        &[
            (0.0, 1.0, -8.0, 0.0),
            (1.0, 2.0, -4.0, -2.0),
            (1.0, 3.0, -2.0, 0.0),
        ],
        WHITE,
        "",
    ),
    (
        "Aluminum Wrapped Fascia 6\"",
        "Prefinished aluminum coil wrapped over a 1x6: low-maintenance siding homes.",
        &[(0.0, 0.25, -6.0, 0.0)],
        WHITE,
        "",
    ),
    (
        "PVC Trim Fascia 1x10",
        "Cellular PVC trim board, crisp and paintable: coastal and modern farmhouse.",
        &[(0.0, 0.75, -9.25, 0.0)],
        WHITE,
        "",
    ),
    (
        "Black Metal Band 10\"",
        "A dark metal band with a 2\" upstand: modern farmhouse and industrial.",
        &[(0.0, 1.0, -8.0, 2.0)],
        BLACK,
        "metal-matte-black",
    ),
    (
        "Cedar Slat Fascia 10\"",
        "A clear-finished cedar band matching wood cladding.",
        &[(0.0, 1.0, -8.0, 2.0)],
        CEDAR,
        "siding-cedar-vertical",
    ),
    (
        "Concrete Edge 16\"",
        "A cast-in-place concrete roof edge: brutalist and contemporary.",
        &[(0.0, 2.0, -14.0, 2.0)],
        CONCRETE,
        "concrete-architectural",
    ),
];

/// The 20 typical fascias.
pub fn catalog() -> Vec<FasciaSpec> {
    PROFILES
        .iter()
        .map(|(name, description, parts, color, finish)| FasciaSpec {
            name: (*name).into(),
            description: (*description).into(),
            parts: parts
                .iter()
                .map(|(o0, o1, z0, z1)| FasciaPart {
                    out0: o0 * MM_PER_IN,
                    out1: o1 * MM_PER_IN,
                    z0: z0 * MM_PER_IN,
                    z1: z1 * MM_PER_IN,
                })
                .collect(),
            color: *color,
            finish: (*finish).into(),
        })
        .collect()
}

pub fn by_name(name: &str) -> Option<FasciaSpec> {
    catalog().into_iter().find(|f| f.name == name)
}

/// The project material of a fascia's finish, when it's in the project.
pub fn finish_material(doc: &crate::Document, f: &FasciaSpec) -> Option<crate::ElementId> {
    let name = crate::library::preset(&f.finish)?.name;
    doc.of(crate::Category::Material)
        .find(|e| e.data.name() == name)
        .map(|e| e.id)
}

/// Gives roofs a fascia from the catalog by name, or none (one undo step). Its finish
/// material comes into the project with it.
pub fn set(
    doc: &mut crate::Document,
    roofs: &[crate::ElementId],
    name: Option<&str>,
) -> crate::CoreResult<()> {
    let spec = match name {
        Some(n) => Some(
            by_name(n).ok_or_else(|| crate::CoreError::Invalid(format!("no fascia \"{n}\"")))?,
        ),
        None => None,
    };
    if let Some(s) = &spec {
        if !s.finish.is_empty() {
            crate::library::add_preset(doc, &s.finish)?;
        }
    }
    doc.transact(
        if spec.is_some() {
            "Add fascia"
        } else {
            "Remove fascia"
        },
        |tx| {
            for id in roofs {
                tx.modify(*id, |d| {
                    if let crate::ElementData::Roof { fascia, .. } = d {
                        *fascia = spec.clone();
                    }
                })?;
            }
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_profiles_each_with_a_body() {
        let c = catalog();
        assert_eq!(c.len(), 20);
        for f in &c {
            assert!(!f.parts.is_empty(), "{}", f.name);
            for p in &f.parts {
                assert!(p.out1 > p.out0 && p.z1 > p.z0, "{}", f.name);
            }
        }
        let m = by_name("Modern Stepped Band 12\"").unwrap();
        let (h, o) = m.extent();
        assert!((h - 12.5 * MM_PER_IN).abs() < 1e-9 && (o - 2.25 * MM_PER_IN).abs() < 1e-9);
    }
}
