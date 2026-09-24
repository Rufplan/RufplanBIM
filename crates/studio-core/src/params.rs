//! Parameters (DATA_MODEL.md, ADR-017).
//!
//! Built-in parameters stay typed struct fields on [`ElementData`](crate::ElementData);
//! [`crate::ops::param`] reads any of them by stable key as a [`ParamValue`]. Project
//! parameters — defined by the user for chosen categories — live in each element's
//! key → value map ([`crate::Element::params`]), so new ones never need a file-format change.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::element::Category;
use crate::units::{format_area_sf, format_ft_in, parse_length};

/// A parameter value. Lengths are mm, areas mm², angles radians.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "value")]
#[ts(export)]
pub enum ParamValue {
    Length(f64),
    Area(f64),
    Angle(f64),
    Number(f64),
    Integer(#[ts(type = "number")] i64),
    Bool(bool),
    Text(String),
}

/// The kind of value a parameter holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ParamKind {
    Length,
    Area,
    Angle,
    Number,
    Integer,
    Bool,
    Text,
}

impl ParamKind {
    pub const ALL: [ParamKind; 7] = [
        ParamKind::Text,
        ParamKind::Length,
        ParamKind::Area,
        ParamKind::Angle,
        ParamKind::Number,
        ParamKind::Integer,
        ParamKind::Bool,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ParamKind::Length => "Length",
            ParamKind::Area => "Area",
            ParamKind::Angle => "Angle",
            ParamKind::Number => "Number",
            ParamKind::Integer => "Integer",
            ParamKind::Bool => "Yes/No",
            ParamKind::Text => "Text",
        }
    }

    pub fn parse(s: &str) -> Option<ParamKind> {
        ParamKind::ALL
            .into_iter()
            .find(|k| k.label() == s || format!("{k:?}") == s)
    }
}

/// Whether a project parameter belongs to each instance or to its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ParamScope {
    Instance,
    Type,
}

/// A user-defined project parameter, stored on the project information element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParamDef {
    /// Stable key in element parameter maps.
    pub key: String,
    pub label: String,
    pub kind: ParamKind,
    pub scope: ParamScope,
    /// Instance categories it applies to (for type parameters, the instances' category).
    pub categories: Vec<Category>,
}

impl ParamValue {
    pub fn kind(&self) -> ParamKind {
        match self {
            ParamValue::Length(_) => ParamKind::Length,
            ParamValue::Area(_) => ParamKind::Area,
            ParamValue::Angle(_) => ParamKind::Angle,
            ParamValue::Number(_) => ParamKind::Number,
            ParamValue::Integer(_) => ParamKind::Integer,
            ParamValue::Bool(_) => ParamKind::Bool,
            ParamValue::Text(_) => ParamKind::Text,
        }
    }

    /// Display text: feet-inches, square feet, degrees, "Yes"/"No".
    pub fn display(&self) -> String {
        match self {
            ParamValue::Length(mm) => format_ft_in(*mm),
            ParamValue::Area(mm2) => format_area_sf(*mm2),
            ParamValue::Angle(rad) => format!("{:.2}°", rad.to_degrees()),
            ParamValue::Number(v) => {
                let s = format!("{v:.3}");
                s.trim_end_matches('0').trim_end_matches('.').to_owned()
            }
            ParamValue::Integer(v) => v.to_string(),
            ParamValue::Bool(b) => if *b { "Yes" } else { "No" }.into(),
            ParamValue::Text(t) => t.clone(),
        }
    }

    /// Parses what the user typed for a parameter of `kind`.
    pub fn parse(kind: ParamKind, s: &str) -> Option<ParamValue> {
        let s = s.trim();
        Some(match kind {
            ParamKind::Length => ParamValue::Length(parse_length(s)?),
            ParamKind::Area => {
                let n: f64 = s
                    .trim_end_matches("SF")
                    .trim_end_matches("sf")
                    .trim()
                    .replace(',', "")
                    .parse()
                    .ok()?;
                ParamValue::Area(n * 304.8 * 304.8)
            }
            ParamKind::Angle => {
                let n: f64 = s.trim_end_matches('°').trim().parse().ok()?;
                ParamValue::Angle(n.to_radians())
            }
            ParamKind::Number => ParamValue::Number(s.parse().ok()?),
            ParamKind::Integer => ParamValue::Integer(s.parse().ok()?),
            ParamKind::Bool => ParamValue::Bool(matches!(
                s.to_ascii_lowercase().as_str(),
                "yes" | "true" | "1" | "y"
            )),
            ParamKind::Text => ParamValue::Text(s.to_owned()),
        })
    }

    /// The value as a number for sorting and comparisons, if it has one.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ParamValue::Length(v)
            | ParamValue::Area(v)
            | ParamValue::Angle(v)
            | ParamValue::Number(v) => Some(*v),
            ParamValue::Integer(v) => Some(*v as f64),
            ParamValue::Bool(b) => Some(f64::from(u8::from(*b))),
            ParamValue::Text(_) => None,
        }
    }
}

/// A parameter key made from a label: lowercase words joined by underscores.
pub fn key_for(label: &str) -> String {
    let mut out = String::new();
    for c in label.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_owned()
}

// ---- Project parameter operations -------------------------------------------------------

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{ElementData, ElementId};
use crate::ops::{flag, len, project_info, text, Property};

/// The instance category a type element's instances belong to.
pub fn instance_category_of_type(c: Category) -> Option<Category> {
    Some(match c {
        Category::WallType => Category::Wall,
        Category::DoorType => Category::Door,
        Category::WindowType => Category::Window,
        Category::FloorType => Category::Floor,
        Category::CeilingType => Category::Ceiling,
        Category::RoofType => Category::Roof,
        _ => return None,
    })
}

/// The project's parameter definitions.
pub fn defs(doc: &Document) -> Vec<ParamDef> {
    project_info(doc)
        .and_then(|i| match doc.data(i) {
            Ok(ElementData::ProjectInfo { param_defs, .. }) => Some(param_defs.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Definitions that apply to an element with category `cat`: instance parameters of
/// its category, or type parameters when it is a type.
pub fn defs_for(doc: &Document, cat: Category) -> Vec<ParamDef> {
    let (want, scope) = match instance_category_of_type(cat) {
        Some(inst) => (inst, ParamScope::Type),
        None => (cat, ParamScope::Instance),
    };
    defs(doc)
        .into_iter()
        .filter(|d| d.scope == scope && d.categories.contains(&want))
        .collect()
}

/// Adds a project parameter. Returns its key.
pub fn add_def(
    doc: &mut Document,
    label: &str,
    kind: ParamKind,
    scope: ParamScope,
    categories: Vec<Category>,
) -> CoreResult<String> {
    let label = label.trim();
    let key = key_for(label);
    if key.is_empty() {
        return Err(CoreError::Invalid("name the parameter".into()));
    }
    if categories.is_empty() {
        return Err(CoreError::Invalid("pick at least one category".into()));
    }
    let info = project_info(doc).ok_or_else(|| CoreError::Invalid("no project info".into()))?;
    if defs(doc).iter().any(|d| d.key == key) {
        return Err(CoreError::Invalid(format!(
            "a parameter named \"{label}\" already exists"
        )));
    }
    let def = ParamDef {
        key: key.clone(),
        label: label.into(),
        kind,
        scope,
        categories,
    };
    doc.transact("Add project parameter", |tx| {
        tx.modify(info, |d| {
            if let ElementData::ProjectInfo { param_defs, .. } = d {
                param_defs.push(def);
            }
        })
    })?;
    Ok(key)
}

/// Removes a project parameter and its values on every element.
pub fn remove_def(doc: &mut Document, key: &str) -> CoreResult<()> {
    let info = project_info(doc).ok_or_else(|| CoreError::Invalid("no project info".into()))?;
    let holders: Vec<ElementId> = doc
        .iter()
        .filter(|e| e.params.contains_key(key))
        .map(|e| e.id)
        .collect();
    doc.transact("Remove project parameter", |tx| {
        tx.modify(info, |d| {
            if let ElementData::ProjectInfo { param_defs, .. } = d {
                param_defs.retain(|p| p.key != key);
            }
        })?;
        for id in holders {
            tx.set_param(id, key, None)?;
        }
        Ok(())
    })
}

/// Sets a project parameter from typed text; empty text clears it.
pub fn set_value(doc: &mut Document, id: ElementId, key: &str, value: &str) -> CoreResult<()> {
    let cat = doc.data(id)?.category();
    let def = defs_for(doc, cat)
        .into_iter()
        .find(|d| d.key == key)
        .ok_or_else(|| CoreError::Invalid(format!("no parameter {key} for this element")))?;
    let v = if value.trim().is_empty() && def.kind != ParamKind::Bool {
        None
    } else {
        Some(ParamValue::parse(def.kind, value).ok_or_else(|| {
            CoreError::Invalid(format!(
                "\"{value}\" is not a valid {} value",
                def.kind.label().to_lowercase()
            ))
        })?)
    };
    doc.transact(&format!("Change {}", def.label), |tx| {
        tx.set_param(id, key, v)
    })
}

/// Property rows for the project parameters that apply to an element.
pub(crate) fn param_properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Some(el) = doc.get(id) else { return };
    for d in defs_for(doc, el.category()) {
        let key = format!("param:{}", d.key);
        let cur = el.params.get(&d.key);
        props.push(match d.kind {
            ParamKind::Length => {
                let mut row = len(&key, &d.label, "Other", 0.0);
                row.value = cur.map(ParamValue::display).unwrap_or_default();
                row
            }
            ParamKind::Bool => flag(
                &key,
                &d.label,
                "Other",
                matches!(cur, Some(ParamValue::Bool(true))),
            ),
            _ => text(
                &key,
                &d.label,
                "Other",
                &cur.map(ParamValue::display).unwrap_or_default(),
            ),
        });
    }
}

#[cfg(test)]
mod op_tests {
    use super::*;
    use crate::ops;

    fn project() -> Document {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        doc
    }

    #[test]
    fn project_parameters_apply_by_category_and_undo() {
        let mut doc = project();
        let key = add_def(
            &mut doc,
            "Fire Rating",
            ParamKind::Text,
            ParamScope::Instance,
            vec![Category::Door],
        )
        .unwrap();
        assert_eq!(key, "fire_rating");
        assert!(add_def(
            &mut doc,
            "fire rating",
            ParamKind::Text,
            ParamScope::Instance,
            vec![Category::Door]
        )
        .is_err());
        let level = doc.levels()[0].0;
        let wt = doc.of(Category::WallType).next().unwrap().id;
        let w = ops::create_wall(
            &mut doc,
            wt,
            level,
            studio_geom::Pt::new(0.0, 0.0),
            studio_geom::Pt::new(6000.0, 0.0),
        )
        .unwrap();
        let dt = doc.of(Category::DoorType).next().unwrap().id;
        let door = ops::create_door(&mut doc, dt, w, 3000.0, false).unwrap();
        // Walls don't get door parameters.
        assert!(set_value(&mut doc, w, &key, "1 HR").is_err());
        set_value(&mut doc, door, &key, "20 MIN").unwrap();
        assert_eq!(
            doc.param(door, &key),
            Some(&ParamValue::Text("20 MIN".into()))
        );
        let sheet = ops::properties(&doc, door).unwrap();
        let row = sheet
            .properties
            .iter()
            .find(|p| p.key == "param:fire_rating")
            .unwrap();
        assert_eq!(row.value, "20 MIN");
        doc.undo().unwrap();
        assert_eq!(doc.param(door, &key), None);
        doc.redo().unwrap();
        remove_def(&mut doc, &key).unwrap();
        assert_eq!(doc.param(door, &key), None);
        assert!(defs(&doc).is_empty());
        doc.undo().unwrap();
        assert!(doc.param(door, &key).is_some(), "undo restores values");
    }

    #[test]
    fn type_parameters_show_on_types() {
        let mut doc = project();
        add_def(
            &mut doc,
            "STC",
            ParamKind::Integer,
            ParamScope::Type,
            vec![Category::Wall],
        )
        .unwrap();
        let wt = doc.of(Category::WallType).next().unwrap().id;
        set_value(&mut doc, wt, "stc", "45").unwrap();
        assert_eq!(doc.param(wt, "stc"), Some(&ParamValue::Integer(45)));
        assert!(set_value(&mut doc, wt, "stc", "loud").is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::MM_PER_FT;

    #[test]
    fn values_parse_and_display_round_trip() {
        let l = ParamValue::parse(ParamKind::Length, "10'-6\"").unwrap();
        assert_eq!(l, ParamValue::Length(10.5 * MM_PER_FT));
        assert_eq!(l.display(), "10'-6\"");
        let a = ParamValue::parse(ParamKind::Area, "120 SF").unwrap();
        assert!((a.as_f64().unwrap() - 120.0 * MM_PER_FT * MM_PER_FT).abs() < 1e-6);
        assert_eq!(
            ParamValue::parse(ParamKind::Bool, "Yes"),
            Some(ParamValue::Bool(true))
        );
        assert_eq!(ParamValue::parse(ParamKind::Integer, "x"), None);
        assert_eq!(ParamValue::Number(2.5).display(), "2.5");
        assert_eq!(ParamValue::Number(3.0).display(), "3");
        assert_eq!(
            ParamValue::parse(ParamKind::Angle, "90°")
                .unwrap()
                .display(),
            "90.00°"
        );
    }

    #[test]
    fn keys_from_labels() {
        assert_eq!(key_for("Fire Rating"), "fire_rating");
        assert_eq!(key_for("  STC (rated) "), "stc_rated");
        assert_eq!(ParamKind::parse("Yes/No"), Some(ParamKind::Bool));
    }
}
