//! Materials (ADR-020): cut patterns for plans and sections, surface patterns for
//! elevations and shaded colors for 3D, and the link from type layers to them.

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{Category, CutPattern, ElementData, ElementId, SurfacePattern, WallLayer};
use crate::ops::{choice, non_empty, ro, text, PropOption, Property};

/// A resolved material: what drawings need to know about it.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialInfo {
    pub id: Option<ElementId>,
    pub name: String,
    pub cut: CutPattern,
    pub surface: SurfacePattern,
    pub color: [u8; 3],
}

/// Built-in materials as (name, cut pattern, surface preset id, color).
const BUILT_IN: &[(&str, CutPattern, &str, [u8; 3])] = &[
    ("Concrete", CutPattern::Concrete, "none", [190, 188, 182]),
    (
        "Concrete Masonry Unit",
        CutPattern::Masonry,
        "block",
        [176, 172, 164],
    ),
    ("Brick", CutPattern::Masonry, "brick", [168, 82, 60]),
    ("Stucco", CutPattern::None, "none", [226, 220, 206]),
    ("Wood Siding", CutPattern::None, "lap8", [205, 190, 160]),
    ("Plywood", CutPattern::None, "none", [214, 190, 140]),
    ("Wood Framing", CutPattern::Wood, "none", [200, 170, 120]),
    (
        "Batt Insulation",
        CutPattern::Insulation,
        "none",
        [240, 210, 160],
    ),
    (
        "Rigid Insulation",
        CutPattern::Rigid,
        "none",
        [200, 220, 235],
    ),
    ("Gypsum Board", CutPattern::None, "none", [245, 245, 242]),
    ("Metal Stud", CutPattern::None, "none", [170, 175, 180]),
    ("Air", CutPattern::None, "none", [255, 255, 255]),
    (
        "Hardwood Flooring",
        CutPattern::None,
        "none",
        [170, 120, 80],
    ),
    (
        "Asphalt Shingles",
        CutPattern::None,
        "shingle5",
        [80, 84, 90],
    ),
    (
        "Roofing Membrane",
        CutPattern::None,
        "none",
        [110, 110, 110],
    ),
    ("Cover Board", CutPattern::None, "none", [200, 200, 195]),
    (
        "Structural Steel",
        CutPattern::Solid,
        "none",
        [110, 122, 134],
    ),
    ("Glulam", CutPattern::Wood, "none", [205, 165, 110]),
    ("Default", CutPattern::None, "none", [200, 200, 200]),
];

fn preset(id: &str) -> SurfacePattern {
    SurfacePattern::presets()
        .into_iter()
        .find(|(p, _, _)| *p == id)
        .map_or(SurfacePattern::None, |(_, _, s)| s)
}

/// The built-in material a layer or type name most likely means ("Wood Stud 2x6 with Batt
/// Insulation" → "Batt Insulation"), used to link built-in and older types.
pub fn material_for_name(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    let has = |k: &[&str]| k.iter().any(|w| n.contains(w));
    Some(if has(&["rigid"]) {
        "Rigid Insulation"
    } else if has(&["batt", "insulation"]) {
        "Batt Insulation"
    } else if has(&["cmu", "block"]) {
        "Concrete Masonry Unit"
    } else if has(&["brick", "masonry", "stone"]) {
        "Brick"
    } else if has(&["stucco", "plaster"]) {
        "Stucco"
    } else if has(&["siding"]) {
        "Wood Siding"
    } else if has(&["plywood", "osb", "sheathing", "subfloor"]) {
        "Plywood"
    } else if has(&["gypsum", "drywall"]) {
        "Gypsum Board"
    } else if has(&["metal stud"]) {
        "Metal Stud"
    } else if has(&["air"]) {
        "Air"
    } else if has(&["hardwood"]) {
        "Hardwood Flooring"
    } else if has(&["shingle"]) {
        "Asphalt Shingles"
    } else if has(&["membrane"]) {
        "Roofing Membrane"
    } else if has(&["cover board"]) {
        "Cover Board"
    } else if has(&["steel"]) {
        "Structural Steel"
    } else if has(&["glulam"]) {
        "Glulam"
    } else if has(&["concrete"]) {
        "Concrete"
    } else if has(&["joist", "stud", "rafter", "wood", "lumber", "post"]) {
        "Wood Framing"
    } else {
        return None;
    })
}

/// The cut pattern a name implies when there is no material (files from before ADR-020).
pub fn cut_for_name(name: &str) -> CutPattern {
    material_for_name(name)
        .and_then(|m| BUILT_IN.iter().find(|b| b.0 == m))
        .map_or(CutPattern::None, |b| b.1)
}

fn info_of(doc: &Document, id: ElementId) -> Option<MaterialInfo> {
    match doc.data(id).ok()? {
        ElementData::Material {
            name,
            cut,
            surface,
            color,
            ..
        } => Some(MaterialInfo {
            id: Some(id),
            name: name.clone(),
            cut: *cut,
            surface: *surface,
            color: *color,
        }),
        _ => None,
    }
}

fn by_name(doc: &Document, name: &str) -> Option<ElementId> {
    doc.of(Category::Material)
        .find(|e| e.data.name() == name)
        .map(|e| e.id)
}

/// A layer's material: the one it references, else the built-in one its name implies
/// (by name in this project), else just the implied cut pattern.
pub fn resolve(doc: &Document, layer: &WallLayer) -> MaterialInfo {
    if let Some(m) = layer.material.and_then(|id| info_of(doc, id)) {
        return m;
    }
    resolve_name(doc, &layer.name)
}

/// The material a type name implies, for types without layers (columns, beams).
pub fn resolve_type(doc: &Document, material: Option<ElementId>, type_name: &str) -> MaterialInfo {
    material
        .and_then(|id| info_of(doc, id))
        .unwrap_or_else(|| resolve_name(doc, type_name))
}

fn resolve_name(doc: &Document, name: &str) -> MaterialInfo {
    if let Some(m) = material_for_name(name)
        .and_then(|m| by_name(doc, m))
        .and_then(|id| info_of(doc, id))
    {
        return m;
    }
    let b = material_for_name(name).and_then(|m| BUILT_IN.iter().find(|b| b.0 == m));
    MaterialInfo {
        id: None,
        name: b.map_or_else(|| name.to_owned(), |b| b.0.to_owned()),
        cut: b.map_or(CutPattern::None, |b| b.1),
        surface: b.map_or(SurfacePattern::None, |b| preset(b.2)),
        color: b.map_or([200, 200, 200], |b| b.3),
    }
}

/// Adds the built-in materials and links every type's layers to them by name.
pub(crate) fn seed_materials(tx: &mut Tx<'_>) {
    for (name, cut, surface, color) in BUILT_IN {
        tx.insert(ElementData::Material {
            name: (*name).into(),
            cut: *cut,
            surface: preset(surface),
            color: *color,
            appearance: built_in_appearance(name),
        });
    }
    link_types(tx);
}

/// Points layers (and column and beam types) without a material at the material their
/// name implies.
fn link_types(tx: &mut Tx<'_>) {
    let materials: Vec<(String, ElementId)> = tx
        .of(Category::Material)
        .map(|e| (e.data.name(), e.id))
        .collect();
    let find = |name: &str| {
        material_for_name(name).and_then(|m| materials.iter().find(|x| x.0 == m).map(|x| x.1))
    };
    let types: Vec<ElementId> = tx
        .iter()
        .filter(|e| {
            matches!(
                e.category(),
                Category::WallType
                    | Category::FloorType
                    | Category::CeilingType
                    | Category::RoofType
                    | Category::ColumnType
                    | Category::BeamType
            )
        })
        .map(|e| e.id)
        .collect();
    for id in types {
        let _ = tx.modify(id, |d| match d {
            ElementData::WallType { layers, .. }
            | ElementData::FloorType { layers, .. }
            | ElementData::CeilingType { layers, .. }
            | ElementData::RoofType { layers, .. } => {
                for l in layers.iter_mut().filter(|l| l.material.is_none()) {
                    l.material = find(&l.name);
                }
            }
            ElementData::ColumnType { name, material, .. }
            | ElementData::BeamType { name, material, .. }
                if material.is_none() =>
            {
                *material = find(name);
            }
            _ => {}
        });
    }
}

/// Adds the built-in materials to projects saved before materials existed.
pub fn ensure_materials(doc: &mut Document) -> CoreResult<()> {
    if doc.count(Category::Material) > 0 {
        return Ok(());
    }
    doc.transact("Add materials", |tx| {
        seed_materials(tx);
        Ok(())
    })
}

/// Material choices for a layer or type row: "By Name" plus every material.
pub(crate) fn options(doc: &Document) -> Vec<PropOption> {
    let mut v: Vec<(String, ElementId)> = doc
        .of(Category::Material)
        .map(|e| (e.data.name(), e.id))
        .collect();
    v.sort();
    std::iter::once(PropOption {
        id: String::new(),
        label: "<By Name>".into(),
    })
    .chain(v.into_iter().map(|(name, id)| PropOption {
        id: id.to_string(),
        label: name,
    }))
    .collect()
}

/// The name of material `id`, if it is one (for renaming a layer when it's picked).
pub(crate) fn name_of(doc: &Document, id: ElementId) -> Option<String> {
    matches!(doc.data(id), Ok(ElementData::Material { .. }))
        .then(|| doc.data(id).map(|d| d.name()).unwrap_or_default())
}

/// How many types use material `id`.
pub fn uses(doc: &Document, id: ElementId) -> usize {
    doc.iter()
        .filter(|e| match &e.data {
            ElementData::WallType { layers, .. }
            | ElementData::FloorType { layers, .. }
            | ElementData::CeilingType { layers, .. }
            | ElementData::RoofType { layers, .. } => layers.iter().any(|l| l.material == Some(id)),
            ElementData::ColumnType { material, .. } | ElementData::BeamType { material, .. } => {
                *material == Some(id)
            }
            _ => false,
        })
        .count()
}

fn unique_name(doc: &Document, base: &str) -> String {
    let taken = |n: &str| doc.of(Category::Material).any(|e| e.data.name() == n);
    if !taken(base) {
        return base.to_owned();
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| base.to_owned())
}

/// A new material: a copy of `from` (or a plain default), named uniquely.
pub fn create_material(doc: &mut Document, from: Option<ElementId>) -> CoreResult<ElementId> {
    let (name, cut, surface, color, appearance) = match from.map(|id| doc.data(id)).transpose()? {
        Some(ElementData::Material {
            name,
            cut,
            surface,
            color,
            appearance,
        }) => (name.clone(), *cut, *surface, *color, appearance.clone()),
        Some(_) => return Err(CoreError::Invalid("pick a material to duplicate".into())),
        None => (
            "New Material".to_owned(),
            CutPattern::None,
            SurfacePattern::None,
            [200, 200, 200],
            crate::library::Appearance::default(),
        ),
    };
    let name = unique_name(doc, &name);
    doc.transact("Create material", |tx| {
        Ok(tx.insert(ElementData::Material {
            name,
            cut,
            surface,
            color,
            appearance,
        }))
    })
}

/// A number from 0 to `max`.
fn unit(value: &str, max: f64) -> CoreResult<f64> {
    let v: f64 = value
        .trim()
        .parse()
        .map_err(|_| CoreError::Invalid(format!("enter a number from 0 to {max}")))?;
    if !(0.0..=max).contains(&v) {
        return Err(CoreError::Invalid(format!(
            "enter a number from 0 to {max}"
        )));
    }
    Ok(v)
}

/// A built-in material's look: glossy steel, matte masonry, painted board…
fn built_in_appearance(name: &str) -> crate::library::Appearance {
    let mut a = crate::library::Appearance::default();
    match name {
        "Structural Steel" | "Metal Stud" => {
            a.metalness = 0.8;
            a.roughness = 0.45;
            a.reflection = 1.0;
        }
        "Hardwood Flooring" => a.roughness = 0.45,
        "Gypsum Board" => a.roughness = 0.9,
        "Brick" | "Concrete Masonry Unit" | "Concrete" | "Stucco" => a.roughness = 0.9,
        _ => {}
    }
    a
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn parse_hex(s: &str) -> CoreResult<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    let bad = || CoreError::Invalid(format!("\"{s}\" is not a color (try #C8A078)"));
    if h.len() != 6 {
        return Err(bad());
    }
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| bad());
    Ok([byte(0)?, byte(2)?, byte(4)?])
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(ElementData::Material {
        name,
        cut,
        surface,
        color,
        appearance: a,
    }) = doc.data(id)
    else {
        return;
    };
    props.push(text("name", "Name", "Identity Data", name));
    props.push(choice(
        "cut",
        "Cut Pattern",
        "Graphics",
        format!("{cut:?}"),
        CutPattern::ALL
            .iter()
            .map(|c| PropOption {
                id: format!("{c:?}"),
                label: c.label().into(),
            })
            .collect(),
    ));
    let mut surfaces: Vec<PropOption> = SurfacePattern::presets()
        .into_iter()
        .map(|(id, label, _)| PropOption {
            id: id.into(),
            label: label.into(),
        })
        .collect();
    if surface.preset_id() == "custom" {
        surfaces.push(PropOption {
            id: "custom".into(),
            label: "Custom".into(),
        });
    }
    props.push(choice(
        "surface",
        "Surface Pattern",
        "Graphics",
        surface.preset_id().into(),
        surfaces,
    ));
    props.push(text("color", "Shading Color", "Graphics", &hex(*color)));
    // Appearance (ADR-029), in V-Ray's terms.
    const A: &str = "Appearance";
    let num = |v: f64| format!("{v:.2}");
    if let Some(p) = a.preset.as_deref().and_then(crate::library::preset) {
        props.push(ro("preset", "Library Material", A, p.name));
    }
    props.push(ro(
        "texture",
        "Texture",
        A,
        match a.texture.as_deref() {
            None => "None".into(),
            Some(t) if t.starts_with("proc:") => format!("Procedural: {}", &t[5..]),
            Some(t) => format!("Photo (Poly Haven {t}, 2K)"),
        },
    ));
    if a.texture.is_some() {
        props.push(crate::ops::len(
            "scale",
            "Texture Size (real world)",
            A,
            a.scale,
        ));
        props.push(text("tint", "Texture Tint", A, &hex(a.tint)));
    }
    props.push(text(
        "glossiness",
        "Reflection Glossiness (0–1)",
        A,
        &num(1.0 - a.roughness),
    ));
    props.push(text(
        "reflection",
        "Reflection (0–1)",
        A,
        &num(a.reflection),
    ));
    props.push(text("metalness", "Metalness (0–1)", A, &num(a.metalness)));
    props.push(text(
        "refraction",
        "Refraction (0–1)",
        A,
        &num(a.refraction),
    ));
    props.push(text("ior", "IOR", A, &num(a.ior)));
    props.push(text("bump", "Bump (0–2)", A, &num(a.bump)));
    props.push(text("coat", "Coat (0–1)", A, &num(a.coat)));
    props.push(text("sheen", "Sheen (0–1)", A, &num(a.sheen)));
    props.push(ro(
        "uses",
        "Used By",
        "Identity Data",
        match uses(doc, id) {
            1 => "1 type".into(),
            n => format!("{n} types"),
        },
    ));
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let mut data = doc.data(id)?.clone();
    let ElementData::Material {
        name,
        cut,
        surface,
        color,
        appearance: a,
    } = &mut data
    else {
        return Err(CoreError::Invalid("not a material".into()));
    };
    match key {
        "name" => {
            let n = non_empty(value)?;
            if doc
                .of(Category::Material)
                .any(|e| e.id != id && e.data.name() == n)
            {
                return Err(CoreError::Invalid(format!(
                    "a material named {n} already exists"
                )));
            }
            *name = n;
        }
        "cut" => {
            *cut = CutPattern::parse(value)
                .ok_or_else(|| CoreError::Invalid(format!("unknown cut pattern {value}")))?
        }
        "surface" => {
            *surface = SurfacePattern::presets()
                .into_iter()
                .find(|(p, _, _)| *p == value)
                .map(|(_, _, s)| s)
                .ok_or_else(|| CoreError::Invalid(format!("unknown surface pattern {value}")))?
        }
        "color" => *color = parse_hex(value)?,
        "tint" => a.tint = parse_hex(value)?,
        "scale" => a.scale = crate::ops::positive(crate::ops::parse_len(value)?)?,
        "glossiness" => a.roughness = 1.0 - unit(value, 1.0)?,
        "reflection" => a.reflection = unit(value, 1.0)?,
        "metalness" => a.metalness = unit(value, 1.0)?,
        "refraction" => a.refraction = unit(value, 1.0)?,
        "coat" => a.coat = unit(value, 1.0)?,
        "sheen" => a.sheen = unit(value, 1.0)?,
        "bump" => a.bump = unit(value, 2.0)?,
        "ior" => {
            let v: f64 = value
                .trim()
                .parse()
                .map_err(|_| CoreError::Invalid("enter an IOR such as 1.5".into()))?;
            if !(1.0..=3.0).contains(&v) {
                return Err(CoreError::Invalid("use an IOR from 1.0 to 3.0".into()));
            }
            a.ior = v;
        }
        _ => return Err(CoreError::Invalid(format!("unknown property {key}"))),
    }
    doc.transact(&format!("Change material {key}"), |tx| tx.set(id, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_types_link_to_materials() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let ext = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 12"))
            .unwrap();
        let ElementData::WallType { layers, .. } = &ext.data else {
            panic!()
        };
        let names: Vec<String> = layers.iter().map(|l| resolve(&doc, l).name).collect();
        assert_eq!(
            names,
            [
                "Stucco",
                "Concrete Masonry Unit",
                "Rigid Insulation",
                "Air",
                "Gypsum Board"
            ]
        );
        assert!(layers.iter().all(|l| l.material.is_some()));
        let cmu = resolve(&doc, &layers[1]);
        assert_eq!(cmu.cut, CutPattern::Masonry);
        assert_eq!(
            cmu.surface,
            SurfacePattern::Running {
                course: 8.0 * 25.4,
                unit: 16.0 * 25.4
            }
        );
        let steel = doc
            .of(Category::ColumnType)
            .find(|e| e.data.name().starts_with("Steel"))
            .unwrap();
        let ElementData::ColumnType { material, .. } = &steel.data else {
            panic!()
        };
        assert_eq!(
            material.and_then(|m| name_of(&doc, m)).as_deref(),
            Some("Structural Steel")
        );
        let stud = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap();
        let ElementData::WallType { layers, .. } = &stud.data else {
            panic!()
        };
        assert_eq!(
            resolve(&doc, &layers[2]).cut,
            CutPattern::Insulation,
            "batt in the studs"
        );
    }

    #[test]
    fn edit_duplicate_and_guard_materials() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let brick = by_name(&doc, "Brick").unwrap();
        let copy = create_material(&mut doc, Some(brick)).unwrap();
        assert_eq!(doc.data(copy).unwrap().name(), "Brick 2");
        set_property(&mut doc, copy, "name", "Brick - Red Velour").unwrap();
        set_property(&mut doc, copy, "color", "#a04030").unwrap();
        set_property(&mut doc, copy, "cut", "Concrete").unwrap();
        set_property(&mut doc, copy, "surface", "lap6").unwrap();
        let ElementData::Material {
            color,
            cut,
            surface,
            ..
        } = doc.data(copy).unwrap()
        else {
            panic!()
        };
        assert_eq!((*color, *cut), ([0xA0, 0x40, 0x30], CutPattern::Concrete));
        assert_eq!(
            *surface,
            SurfacePattern::Lap {
                spacing: 6.0 * 25.4
            }
        );
        assert!(
            set_property(&mut doc, copy, "name", "Brick").is_err(),
            "names are unique"
        );
        assert!(set_property(&mut doc, copy, "color", "red").is_err());
        let cmu = by_name(&doc, "Concrete Masonry Unit").unwrap();
        assert!(uses(&doc, cmu) >= 1);
        assert!(crate::ops::delete(&mut doc, &[cmu]).is_err(), "in use");
        assert_eq!(crate::ops::delete(&mut doc, &[copy]).unwrap(), 1);
    }

    #[test]
    fn names_imply_materials() {
        assert_eq!(
            material_for_name("Wood Stud 2x6 with Batt Insulation"),
            Some("Batt Insulation")
        );
        assert_eq!(material_for_name("Metal Stud 3 5/8\""), Some("Metal Stud"));
        assert_eq!(material_for_name("Concrete Deck"), Some("Concrete"));
        assert_eq!(material_for_name("Steel W10x33"), Some("Structural Steel"));
        assert_eq!(material_for_name("Wood Post - 6x6"), Some("Wood Framing"));
        assert_eq!(material_for_name("Mystery"), None);
        assert_eq!(cut_for_name("CMU 8\""), CutPattern::Masonry);
    }
}
