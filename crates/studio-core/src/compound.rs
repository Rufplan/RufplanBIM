//! Compound wall structure (ADR-018): the layers of a wall type, exterior first.

use crate::document::{CoreError, CoreResult};
use crate::element::{LayerFunction, LocationLine, WallLayer};
use crate::ops::{choice, len, non_empty, p, parse_len, text, PropKind, PropOption, Property};
use crate::units::MM_PER_IN;

fn layer(name: &str, inches: f64, function: LayerFunction) -> WallLayer {
    WallLayer {
        name: name.into(),
        thickness: inches * MM_PER_IN,
        function,
    }
}

/// Layers of the built-in wall types (they add up to each type's nominal width).
pub fn default_layers(type_name: &str) -> Vec<WallLayer> {
    use LayerFunction::*;
    if type_name.starts_with("Exterior - 8") {
        vec![
            layer("Siding on Furring", 1.375, Finish),
            layer("Plywood Sheathing", 0.5, Substrate),
            layer("Wood Stud 2x6 with Batt Insulation", 5.5, Structure),
            layer("Gypsum Board", 0.625, Finish),
        ]
    } else if type_name.starts_with("Interior - 6") {
        vec![
            layer("Gypsum Board", 0.625, Finish),
            layer("Wood Stud with Sound Batt", 4.75, Structure),
            layer("Gypsum Board", 0.625, Finish),
        ]
    } else if type_name.starts_with("Interior - 4 7/8") {
        vec![
            layer("Gypsum Board", 0.625, Finish),
            layer("Metal Stud 3 5/8\"", 3.625, Structure),
            layer("Gypsum Board", 0.625, Finish),
        ]
    } else if type_name.starts_with("Exterior - 12") {
        vec![
            layer("Stucco", 0.875, Finish),
            layer("CMU 8\"", 7.625, Structure),
            layer("Rigid Insulation", 2.0, Insulation),
            layer("Air Gap", 0.875, AirGap),
            layer("Gypsum Board", 0.625, Finish),
        ]
    } else {
        vec![]
    }
}

/// Layers of the built-in floor, ceiling and roof types (top / outside first).
pub fn default_type_layers(type_name: &str) -> Vec<WallLayer> {
    use LayerFunction::*;
    if type_name.starts_with("Wood Joist Floor - 12") {
        vec![
            layer("Hardwood Flooring", 0.75, Finish),
            layer("Plywood Subfloor", 0.75, Substrate),
            layer("Wood I-Joists", 9.875, Structure),
            layer("Gypsum Board", 0.625, Finish),
        ]
    } else if type_name.starts_with("Concrete Slab - 6") {
        vec![layer("Concrete", 6.0, Structure)]
    } else if type_name.starts_with("Asphalt Shingle on Rafters - 10") {
        vec![
            layer("Asphalt Shingles", 0.25, Finish),
            layer("Plywood Sheathing", 0.625, Substrate),
            layer("Wood Rafters with Batt Insulation", 9.125, Structure),
        ]
    } else if type_name.starts_with("Flat Membrane on Deck - 12") {
        vec![
            layer("Roof Membrane", 0.25, Membrane),
            layer("Cover Board", 0.5, Substrate),
            layer("Rigid Insulation", 3.25, Insulation),
            layer("Concrete Deck", 8.0, Structure),
        ]
    } else {
        vec![]
    }
}

/// Offsets of the core's exterior and interior faces from the centerline (mm, positive =
/// exterior side). The core runs from the first to the last structure layer; with none,
/// it is the whole wall.
pub fn core_faces(layers: &[WallLayer], width: f64) -> (f64, f64) {
    let first = layers
        .iter()
        .position(|l| l.function == LayerFunction::Structure);
    let last = layers
        .iter()
        .rposition(|l| l.function == LayerFunction::Structure);
    match (first, last) {
        (Some(a), Some(b)) => {
            let before: f64 = layers[..a].iter().map(|l| l.thickness).sum();
            let after: f64 = layers[b + 1..].iter().map(|l| l.thickness).sum();
            (width / 2.0 - before, -width / 2.0 + after)
        }
        _ => (width / 2.0, -width / 2.0),
    }
}

/// Where a location line sits relative to the centerline (mm, positive = exterior side).
pub fn location_offset(layers: &[WallLayer], width: f64, loc: LocationLine) -> f64 {
    let (ce, ci) = core_faces(layers, width);
    match loc {
        LocationLine::Centerline => 0.0,
        LocationLine::CoreCenterline => (ce + ci) / 2.0,
        LocationLine::FinishExterior => width / 2.0,
        LocationLine::FinishInterior => -width / 2.0,
        LocationLine::CoreExterior => ce,
        LocationLine::CoreInterior => ci,
    }
}

fn function_options() -> Vec<PropOption> {
    LayerFunction::ALL
        .iter()
        .map(|f| PropOption {
            id: f.label().into(),
            label: f.label().into(),
        })
        .collect()
}

const GROUP: &str = "Structure (Exterior to Interior)";
/// Group label for floor, ceiling and roof layers.
pub(crate) const GROUP_TOP_DOWN: &str = "Structure (Top to Bottom)";

/// Property rows for editing a wall type's layers.
pub(crate) fn layer_properties(layers: &[WallLayer], props: &mut Vec<Property>) {
    layer_properties_in(layers, props, GROUP);
}

/// Property rows for editing layers, under `group`.
pub(crate) fn layer_properties_in(layers: &[WallLayer], props: &mut Vec<Property>, group: &str) {
    let n = layers.len();
    for (i, l) in layers.iter().enumerate() {
        let tag = format!("{}.", i + 1);
        props.push(text(
            &format!("layer:{i}:name"),
            &format!("{tag} Material"),
            group,
            &l.name,
        ));
        props.push(choice(
            &format!("layer:{i}:function"),
            &format!("{tag} Function"),
            group,
            l.function.label().into(),
            function_options(),
        ));
        props.push(len(
            &format!("layer:{i}:thickness"),
            &format!("{tag} Thickness"),
            group,
            l.thickness,
        ));
        if i > 0 {
            props.push(p(
                &format!("layer:{i}:up"),
                &format!("{tag} Move Toward Exterior"),
                group,
                "Move Up".into(),
                PropKind::Action,
            ));
        }
        if n > 1 {
            props.push(p(
                &format!("layer:{i}:remove"),
                &format!("{tag} Delete Layer"),
                group,
                "Delete".into(),
                PropKind::Action,
            ));
        }
    }
    props.push(p(
        "layer:add",
        if n == 0 {
            "Define Layers"
        } else {
            "Add Layer (Interior Face)"
        },
        group,
        "Add".into(),
        PropKind::Action,
    ));
}

/// Applies a `layer:*` property edit. `width` is the type's current width, used when a
/// homogeneous type gets its first layers.
pub(crate) fn set_layer_property(
    layers: &mut Vec<WallLayer>,
    width: f64,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let bad = || CoreError::Invalid(format!("unknown property {key}"));
    if key == "layer:add" {
        if layers.is_empty() {
            layers.push(layer(
                "Structure",
                width / MM_PER_IN,
                LayerFunction::Structure,
            ));
        } else {
            layers.push(layer("Gypsum Board", 0.625, LayerFunction::Finish));
        }
        return Ok(());
    }
    let mut parts = key.split(':').skip(1);
    let i: usize = parts.next().and_then(|s| s.parse().ok()).ok_or_else(bad)?;
    let what = parts.next().ok_or_else(bad)?;
    if i >= layers.len() {
        return Err(CoreError::Invalid("that layer no longer exists".into()));
    }
    match what {
        "name" => layers[i].name = non_empty(value)?,
        "thickness" => {
            let t = parse_len(value)?;
            if t < 0.0 {
                return Err(CoreError::Invalid("thickness can't be negative".into()));
            }
            layers[i].thickness = t;
        }
        "function" => {
            layers[i].function = LayerFunction::parse(value)
                .ok_or_else(|| CoreError::Invalid(format!("unknown layer function {value}")))?;
        }
        "up" if i > 0 => layers.swap(i - 1, i),
        "remove" if layers.len() > 1 => {
            layers.remove(i);
        }
        _ => return Err(bad()),
    }
    Ok(())
}

/// Changes a layered type's total width by resizing its (first) structure layer, as when
/// a wall type's width is typed directly. A single-layer type just takes the new width.
pub(crate) fn resize_structure(layers: &mut [WallLayer], width: f64) -> CoreResult<()> {
    if layers.is_empty() {
        return Ok(());
    }
    let total: f64 = layers.iter().map(|l| l.thickness).sum();
    let Some(core) = layers
        .iter_mut()
        .find(|l| l.function == LayerFunction::Structure)
    else {
        return Err(CoreError::Invalid(
            "this type has no structure layer to resize; edit its layers instead".into(),
        ));
    };
    let t = core.thickness + (width - total);
    if t <= 0.0 {
        return Err(CoreError::Invalid(
            "that is thinner than the type's other layers".into(),
        ));
    }
    core.thickness = t;
    Ok(())
}

/// Signed offsets (mm, positive = exterior side, i.e. the wall's left) of the boundaries
/// between layers, measured from the location line (the wall centerline).
pub fn layer_boundaries(layers: &[WallLayer], width: f64) -> Vec<f64> {
    let mut out = vec![];
    let mut at = width / 2.0;
    for l in layers.iter().take(layers.len().saturating_sub(1)) {
        at -= l.thickness;
        out.push(at);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layers_add_up_to_nominal_widths() {
        for (name, inches) in [
            ("Exterior - 8\" Stud", 8.0),
            ("Interior - 6\" Stud", 6.0),
            ("Interior - 4 7/8\" Partition", 4.875),
            ("Exterior - 12\" CMU", 12.0),
        ] {
            let sum: f64 = default_layers(name).iter().map(|l| l.thickness).sum();
            assert!((sum - inches * MM_PER_IN).abs() < 1e-9, "{name}");
        }
        assert!(default_layers("Custom").is_empty());
    }

    #[test]
    fn editing_layers() {
        let mut ls = default_layers("Interior - 4 7/8\" Partition");
        set_layer_property(&mut ls, 0.0, "layer:add", "").unwrap();
        assert_eq!(ls.len(), 4);
        set_layer_property(&mut ls, 0.0, "layer:3:thickness", "1/2\"").unwrap();
        assert!((ls[3].thickness - 12.7).abs() < 1e-9);
        set_layer_property(&mut ls, 0.0, "layer:3:up", "").unwrap();
        assert!((ls[2].thickness - 12.7).abs() < 1e-9);
        set_layer_property(&mut ls, 0.0, "layer:0:function", "Air Gap").unwrap();
        assert_eq!(ls[0].function, LayerFunction::AirGap);
        set_layer_property(&mut ls, 0.0, "layer:0:remove", "").unwrap();
        assert_eq!(ls.len(), 3);
        assert!(set_layer_property(&mut ls, 0.0, "layer:9:name", "x").is_err());
        let mut empty = vec![];
        set_layer_property(&mut empty, 150.0, "layer:add", "").unwrap();
        assert!((empty[0].thickness - 150.0).abs() < 1e-9);
    }

    #[test]
    fn boundaries_run_from_exterior_face() {
        let ls = default_layers("Interior - 4 7/8\" Partition");
        let w: f64 = ls.iter().map(|l| l.thickness).sum();
        let b = layer_boundaries(&ls, w);
        assert_eq!(b.len(), 2);
        assert!((b[0] - (w / 2.0 - 0.625 * MM_PER_IN)).abs() < 1e-9);
        assert!((b[1] + (w / 2.0 - 0.625 * MM_PER_IN)).abs() < 1e-9);
    }

    #[test]
    fn location_lines_of_a_layered_wall() {
        // Exterior 8": siding 1 3/8, sheathing 1/2, stud 5 1/2 (structure), gypsum 5/8.
        let ls = default_layers("Exterior - 8\" Stud");
        let w = 8.0 * MM_PER_IN;
        let off = |l| location_offset(&ls, w, l);
        assert!((off(LocationLine::FinishExterior) - 4.0 * MM_PER_IN).abs() < 1e-9);
        assert!((off(LocationLine::FinishInterior) + 4.0 * MM_PER_IN).abs() < 1e-9);
        assert!((off(LocationLine::CoreExterior) - (4.0 - 1.875) * MM_PER_IN).abs() < 1e-9);
        assert!((off(LocationLine::CoreInterior) + (4.0 - 0.625) * MM_PER_IN).abs() < 1e-9);
        assert!((off(LocationLine::CoreCenterline) + 0.625 * MM_PER_IN).abs() < 1e-9);
        assert_eq!(off(LocationLine::Centerline), 0.0);
    }
}
