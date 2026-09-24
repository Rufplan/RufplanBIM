//! Model operations used by the app. Each is one undoable transaction.

use serde::Serialize;
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{
    Category, Compass, ElementData, ElementId, StageChange, ViewKind, WallFunction, WallTop,
};
use crate::units::{format_area_sf, format_ft_in, parse_length, MM_PER_FT, MM_PER_IN};

/// Default wall height when there is no level above, mm (10'-0").
pub const DEFAULT_WALL_HEIGHT: f64 = 10.0 * MM_PER_FT;
/// Default ceiling height above its level, mm (9'-0").
pub const DEFAULT_CEILING_HEIGHT: f64 = 9.0 * MM_PER_FT;
/// Default level-to-level height, mm (10'-0").
pub const DEFAULT_FLOOR_TO_FLOOR: f64 = 10.0 * MM_PER_FT;

/// Standard architectural scales as (denominator, label).
pub const SCALES: &[(u32, &str)] = &[
    (192, "1/16\" = 1'-0\""),
    (96, "1/8\" = 1'-0\""),
    (64, "3/16\" = 1'-0\""),
    (48, "1/4\" = 1'-0\""),
    (32, "3/8\" = 1'-0\""),
    (24, "1/2\" = 1'-0\""),
    (16, "3/4\" = 1'-0\""),
    (12, "1\" = 1'-0\""),
];

pub fn scale_label(scale: u32) -> String {
    SCALES
        .iter()
        .find(|s| s.0 == scale)
        .map_or_else(|| format!("1 : {scale}"), |s| s.1.to_owned())
}

const DEFAULT_STAGES: &[(&str, &str)] = &[
    ("Pre-Design", "PD"),
    ("Schematic Design", "SD"),
    ("Design Development", "DD"),
    ("Construction Documents", "CD"),
    ("Bidding / Negotiation", "BN"),
    ("Construction Administration", "CA"),
];

/// Seeds a new project: two levels with plan views, default types, four elevations,
/// a 3D view, project information and the six design stages (current: SD).
pub fn seed_default_project(doc: &mut Document) -> CoreResult<()> {
    doc.transact("New project", |tx| {
        add_level(tx, "Level 1", 0.0);
        add_level(tx, "Level 2", DEFAULT_FLOOR_TO_FLOOR);
        for (name, t, f) in [
            ("Exterior - 8\" Stud", 8.0, WallFunction::Exterior),
            ("Interior - 6\" Stud", 6.0, WallFunction::Interior),
            (
                "Interior - 4 7/8\" Partition",
                4.875,
                WallFunction::Interior,
            ),
            ("Exterior - 12\" CMU", 12.0, WallFunction::Exterior),
        ] {
            tx.insert(ElementData::WallType {
                name: name.into(),
                thickness: t * MM_PER_IN,
                function: f,
            });
        }
        for (name, t) in [
            ("Concrete Slab - 6\"", 6.0),
            ("Wood Joist Floor - 12\"", 12.0),
        ] {
            tx.insert(ElementData::FloorType {
                name: name.into(),
                thickness: t * MM_PER_IN,
            });
        }
        for (name, t) in [("ACT 2x4 Ceiling", 1.0), ("GWB Ceiling - 5/8\"", 0.625)] {
            tx.insert(ElementData::CeilingType {
                name: name.into(),
                thickness: t * MM_PER_IN,
            });
        }
        for (facing, name) in [
            (Compass::North, "North"),
            (Compass::South, "South"),
            (Compass::East, "East"),
            (Compass::West, "West"),
        ] {
            tx.insert(ElementData::View {
                name: name.into(),
                kind: ViewKind::Elevation { facing },
                scale: 96,
            });
        }
        tx.insert(ElementData::View {
            name: "{3D}".into(),
            kind: ViewKind::ThreeD,
            scale: 96,
        });
        let mut sd = None;
        for (i, (name, abbr)) in DEFAULT_STAGES.iter().enumerate() {
            let id = tx.insert(ElementData::Stage {
                name: (*name).into(),
                abbreviation: (*abbr).into(),
                order: i as i32,
                start: String::new(),
                target: String::new(),
            });
            if *abbr == "SD" {
                sd = Some(id);
            }
        }
        tx.insert(ElementData::ProjectInfo {
            name: "New Project".into(),
            number: "0001".into(),
            client: String::new(),
            address: String::new(),
            current_stage: sd,
            stage_history: vec![],
        });
        Ok(())
    })
}

fn add_level(tx: &mut Tx<'_>, name: &str, elevation: f64) -> ElementId {
    let id = tx.insert(ElementData::Level {
        name: name.into(),
        elevation,
    });
    tx.insert(ElementData::View {
        name: name.into(),
        kind: ViewKind::FloorPlan { level: id },
        scale: 48,
    });
    tx.insert(ElementData::View {
        name: name.into(),
        kind: ViewKind::CeilingPlan { level: id },
        scale: 48,
    });
    id
}

/// Adds a level named "Level N" with its floor and ceiling plan views.
pub fn create_level(doc: &mut Document, elevation: f64) -> CoreResult<ElementId> {
    let n = doc.of(Category::Level).count() + 1;
    let mut name = format!("Level {n}");
    let mut k = n;
    while doc
        .iter()
        .any(|e| matches!(&e.data, ElementData::Level { name: m, .. } if *m == name))
    {
        k += 1;
        name = format!("Level {k}");
    }
    doc.transact("Create level", |tx| Ok(add_level(tx, &name, elevation)))
}

/// Next grid name after `last`: 1 → 2, A → B, Z → AA, A1 → A2.
pub fn next_grid_name(last: Option<&str>) -> String {
    let Some(last) = last else { return "1".into() };
    if let Ok(n) = last.parse::<u64>() {
        return (n + 1).to_string();
    }
    let digits: String = last
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if !digits.is_empty() {
        let prefix = &last[..last.len() - digits.len()];
        let n: u64 = digits.parse().unwrap_or(0);
        return format!("{prefix}{}", n + 1);
    }
    if last.chars().all(|c| c.is_ascii_uppercase()) && !last.is_empty() {
        let mut chars: Vec<u8> = last.bytes().collect();
        let mut i = chars.len();
        loop {
            if i == 0 {
                chars.insert(0, b'A');
                break;
            }
            i -= 1;
            if chars[i] == b'Z' {
                chars[i] = b'A';
            } else {
                chars[i] += 1;
                break;
            }
        }
        return String::from_utf8(chars).unwrap_or_else(|_| "A".into());
    }
    format!("{last}1")
}

pub fn create_grid(doc: &mut Document, start: Pt, end: Pt) -> CoreResult<ElementId> {
    if start.dist(end) < 10.0 {
        return Err(CoreError::Invalid("grid is too short".into()));
    }
    // UUID v7 ids sort by creation time, so the max id is the latest grid.
    let last = doc
        .of(Category::Grid)
        .max_by_key(|e| e.id)
        .and_then(|e| match &e.data {
            ElementData::Grid { name, .. } => Some(name.clone()),
            _ => None,
        });
    let name = next_grid_name(last.as_deref());
    doc.transact("Create grid", |tx| {
        Ok(tx.insert(ElementData::Grid { name, start, end }))
    })
}

/// The level directly above `level`, if any.
pub fn level_above(doc: &Document, level: ElementId) -> Option<ElementId> {
    let levels = doc.levels();
    let i = levels.iter().position(|l| l.0 == level)?;
    levels.get(i + 1).map(|l| l.0)
}

/// Creates a straight wall on `level`. Its top follows the level above when there is one.
pub fn create_wall(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    start: Pt,
    end: Pt,
) -> CoreResult<ElementId> {
    let top = match level_above(doc, level) {
        Some(l) => WallTop::UpToLevel {
            level: l,
            offset: 0.0,
        },
        None => WallTop::Unconnected {
            height: DEFAULT_WALL_HEIGHT,
        },
    };
    doc.transact("Create wall", |tx| {
        Ok(tx.insert(ElementData::Wall {
            type_id,
            start,
            end,
            base_level: level,
            base_offset: 0.0,
            top,
        }))
    })
}

pub fn create_floor(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    boundary: Vec<Pt>,
) -> CoreResult<ElementId> {
    doc.transact("Create floor", |tx| {
        Ok(tx.insert(ElementData::Floor {
            type_id,
            level,
            offset: 0.0,
            boundary: ccw(boundary),
        }))
    })
}

pub fn create_ceiling(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    boundary: Vec<Pt>,
) -> CoreResult<ElementId> {
    doc.transact("Create ceiling", |tx| {
        Ok(tx.insert(ElementData::Ceiling {
            type_id,
            level,
            height: DEFAULT_CEILING_HEIGHT,
            boundary: ccw(boundary),
        }))
    })
}

fn ccw(mut ring: Vec<Pt>) -> Vec<Pt> {
    if studio_geom::signed_area(&ring) < 0.0 {
        ring.reverse();
    }
    ring
}

/// Deletes elements (and their dependents) in one transaction.
pub fn delete(doc: &mut Document, ids: &[ElementId]) -> CoreResult<usize> {
    for id in ids {
        if matches!(doc.data(*id)?, ElementData::ProjectInfo { .. }) {
            return Err(CoreError::Invalid(
                "project information can't be deleted".into(),
            ));
        }
        if matches!(doc.data(*id)?, ElementData::View { .. }) && doc.of(Category::View).count() <= 1
        {
            return Err(CoreError::Invalid(
                "a project needs at least one view".into(),
            ));
        }
    }
    doc.transact("Delete", |tx| {
        let mut n = 0;
        for id in ids {
            if tx.get(*id).is_some() {
                n += tx.delete(*id)?.len();
            }
        }
        Ok(n)
    })
}

/// The project information element.
pub fn project_info(doc: &Document) -> Option<ElementId> {
    doc.of(Category::ProjectInfo).next().map(|e| e.id)
}

/// Stages sorted by order, as (id, name, abbreviation).
pub fn stages(doc: &Document) -> Vec<(ElementId, String, String)> {
    let mut v: Vec<_> = doc
        .of(Category::Stage)
        .filter_map(|e| match &e.data {
            ElementData::Stage {
                name,
                abbreviation,
                order,
                ..
            } => Some((*order, e.id, name.clone(), abbreviation.clone())),
            _ => None,
        })
        .collect();
    v.sort_by_key(|s| s.0);
    v.into_iter().map(|(_, id, n, a)| (id, n, a)).collect()
}

/// Moves the project to `stage` and appends to the stage history.
pub fn set_current_stage(
    doc: &mut Document,
    stage: ElementId,
    note: &str,
    now_ms: i64,
) -> CoreResult<()> {
    if !matches!(doc.data(stage)?, ElementData::Stage { .. }) {
        return Err(CoreError::Invalid("not a design stage".into()));
    }
    let info = project_info(doc)
        .ok_or_else(|| CoreError::Invalid("project information is missing".into()))?;
    doc.transact("Change design stage", |tx| {
        tx.modify(info, |d| {
            if let ElementData::ProjectInfo {
                current_stage,
                stage_history,
                ..
            } = d
            {
                if *current_stage != Some(stage) {
                    stage_history.push(StageChange {
                        from: *current_stage,
                        to: stage,
                        at: now_ms,
                        note: note.into(),
                    });
                    *current_stage = Some(stage);
                }
            }
        })
    })
}

// ---------------------------------------------------------------------------------------
// Properties panel
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum PropKind {
    Length,
    Text,
    Choice,
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PropOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Property {
    pub key: String,
    pub label: String,
    pub group: String,
    /// Display text; for choices, the selected option id.
    pub value: String,
    pub kind: PropKind,
    pub options: Vec<PropOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PropertySheet {
    pub id: ElementId,
    pub category: Category,
    pub title: String,
    pub type_id: Option<ElementId>,
    pub properties: Vec<Property>,
}

fn p(key: &str, label: &str, group: &str, value: String, kind: PropKind) -> Property {
    Property {
        key: key.into(),
        label: label.into(),
        group: group.into(),
        value,
        kind,
        options: vec![],
    }
}

fn len(key: &str, label: &str, group: &str, mm: f64) -> Property {
    p(key, label, group, format_ft_in(mm), PropKind::Length)
}

fn text(key: &str, label: &str, group: &str, v: &str) -> Property {
    p(key, label, group, v.to_owned(), PropKind::Text)
}

fn ro(key: &str, label: &str, group: &str, v: String) -> Property {
    p(key, label, group, v, PropKind::ReadOnly)
}

fn choice(
    key: &str,
    label: &str,
    group: &str,
    selected: String,
    options: Vec<PropOption>,
) -> Property {
    Property {
        options,
        ..p(key, label, group, selected, PropKind::Choice)
    }
}

fn options_of(doc: &Document, cat: Category) -> Vec<PropOption> {
    let mut v: Vec<PropOption> = doc
        .of(cat)
        .map(|e| PropOption {
            id: e.id.to_string(),
            label: e.data.name(),
        })
        .collect();
    v.sort_by(|a, b| natural_cmp(&a.label, &b.label));
    v
}

fn level_options(doc: &Document) -> Vec<PropOption> {
    doc.levels()
        .into_iter()
        .map(|(id, name, _)| PropOption {
            id: id.to_string(),
            label: name,
        })
        .collect()
}

/// Properties of an element for the properties panel.
pub fn properties(doc: &Document, id: ElementId) -> CoreResult<PropertySheet> {
    let el = doc.get(id).ok_or(CoreError::NotFound(id))?;
    let mut props = vec![];
    match &el.data {
        ElementData::Level { name, elevation } => {
            props.push(text("name", "Name", "Identity Data", name));
            props.push(len("elevation", "Elevation", "Constraints", *elevation));
        }
        ElementData::Grid { name, start, end } => {
            props.push(text("name", "Name", "Identity Data", name));
            props.push(ro(
                "length",
                "Length",
                "Dimensions",
                format_ft_in(start.dist(*end)),
            ));
        }
        ElementData::WallType {
            name,
            thickness,
            function,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(len("thickness", "Width", "Construction", *thickness));
            props.push(choice(
                "function",
                "Function",
                "Construction",
                format!("{function:?}"),
                ["Exterior", "Interior"]
                    .iter()
                    .map(|s| PropOption {
                        id: (*s).into(),
                        label: (*s).into(),
                    })
                    .collect(),
            ));
        }
        ElementData::Wall {
            start,
            end,
            base_level,
            base_offset,
            top,
            ..
        } => {
            props.push(choice(
                "base_level",
                "Base Constraint",
                "Constraints",
                base_level.to_string(),
                level_options(doc),
            ));
            props.push(len(
                "base_offset",
                "Base Offset",
                "Constraints",
                *base_offset,
            ));
            let mut top_opts = vec![PropOption {
                id: "unconnected".into(),
                label: "Unconnected".into(),
            }];
            top_opts.extend(level_options(doc).into_iter().map(|o| PropOption {
                label: format!("Up to level: {}", o.label),
                ..o
            }));
            match top {
                WallTop::UpToLevel { level, offset } => {
                    props.push(choice(
                        "top",
                        "Top Constraint",
                        "Constraints",
                        level.to_string(),
                        top_opts,
                    ));
                    props.push(len("top_offset", "Top Offset", "Constraints", *offset));
                }
                WallTop::Unconnected { height } => {
                    props.push(choice(
                        "top",
                        "Top Constraint",
                        "Constraints",
                        "unconnected".into(),
                        top_opts,
                    ));
                    props.push(len("height", "Unconnected Height", "Constraints", *height));
                }
            }
            props.push(len("length", "Length", "Dimensions", start.dist(*end)));
        }
        ElementData::FloorType { name, thickness }
        | ElementData::CeilingType { name, thickness } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(len("thickness", "Thickness", "Construction", *thickness));
        }
        ElementData::Floor {
            level,
            offset,
            boundary,
            ..
        } => {
            props.push(choice(
                "level",
                "Level",
                "Constraints",
                level.to_string(),
                level_options(doc),
            ));
            props.push(len(
                "offset",
                "Height Offset From Level",
                "Constraints",
                *offset,
            ));
            props.push(ro(
                "area",
                "Area",
                "Dimensions",
                format_area_sf(studio_geom::signed_area(boundary).abs()),
            ));
        }
        ElementData::Ceiling {
            level,
            height,
            boundary,
            ..
        } => {
            props.push(choice(
                "level",
                "Level",
                "Constraints",
                level.to_string(),
                level_options(doc),
            ));
            props.push(len(
                "height",
                "Height Offset From Level",
                "Constraints",
                *height,
            ));
            props.push(ro(
                "area",
                "Area",
                "Dimensions",
                format_area_sf(studio_geom::signed_area(boundary).abs()),
            ));
        }
        ElementData::View { name, kind, scale } => {
            props.push(text("name", "View Name", "Identity Data", name));
            props.push(choice(
                "scale",
                "View Scale",
                "Graphics",
                scale.to_string(),
                SCALES
                    .iter()
                    .map(|(d, l)| PropOption {
                        id: d.to_string(),
                        label: (*l).into(),
                    })
                    .collect(),
            ));
            let kind_label = match kind {
                ViewKind::FloorPlan { .. } => "Floor Plan",
                ViewKind::CeilingPlan { .. } => "Ceiling Plan",
                ViewKind::Elevation { .. } => "Elevation",
                ViewKind::ThreeD => "3D View",
            };
            props.push(ro("kind", "View Type", "Identity Data", kind_label.into()));
            if let Some(l) = el.data.level() {
                props.push(ro(
                    "level",
                    "Associated Level",
                    "Identity Data",
                    doc.data(l).map(|d| d.name()).unwrap_or_default(),
                ));
            }
        }
        ElementData::ProjectInfo {
            name,
            number,
            client,
            address,
            current_stage,
            stage_history,
        } => {
            props.push(text("name", "Project Name", "Project", name));
            props.push(text("number", "Project Number", "Project", number));
            props.push(text("client", "Client Name", "Project", client));
            props.push(text("address", "Project Address", "Project", address));
            let opts = stages(doc)
                .into_iter()
                .map(|(id, n, a)| PropOption {
                    id: id.to_string(),
                    label: format!("{a} — {n}"),
                })
                .collect();
            props.push(choice(
                "current_stage",
                "Current Stage",
                "Design Stage",
                current_stage.map(|s| s.to_string()).unwrap_or_default(),
                opts,
            ));
            props.push(ro(
                "history",
                "Stage Changes",
                "Design Stage",
                stage_history.len().to_string(),
            ));
        }
        ElementData::Stage {
            name,
            abbreviation,
            start,
            target,
            ..
        } => {
            props.push(text("name", "Name", "Identity Data", name));
            props.push(text(
                "abbreviation",
                "Abbreviation",
                "Identity Data",
                abbreviation,
            ));
            props.push(text(
                "start",
                "Planned Start (YYYY-MM-DD)",
                "Schedule",
                start,
            ));
            props.push(text(
                "target",
                "Target Date (YYYY-MM-DD)",
                "Schedule",
                target,
            ));
        }
    }
    Ok(PropertySheet {
        id,
        category: el.category(),
        title: el.data.name(),
        type_id: el.data.type_id(),
        properties: props,
    })
}

fn parse_len(v: &str) -> CoreResult<f64> {
    parse_length(v)
        .ok_or_else(|| CoreError::Invalid(format!("\"{v}\" is not a length (try 10'-6\")")))
}

fn parse_id(v: &str) -> CoreResult<ElementId> {
    v.parse()
        .map_err(|_| CoreError::Invalid(format!("\"{v}\" is not an element id")))
}

/// Sets one property from the text the user typed or the option they picked.
pub fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
    now_ms: i64,
) -> CoreResult<()> {
    let data = doc.data(id)?.clone();
    if let (ElementData::ProjectInfo { .. }, "current_stage") = (&data, key) {
        return set_current_stage(doc, parse_id(value)?, "", now_ms);
    }
    let unknown = || CoreError::Invalid(format!("unknown property {key}"));
    let mut d = data;
    match &mut d {
        ElementData::Level { name, elevation } => match key {
            "name" => *name = non_empty(value)?,
            "elevation" => *elevation = parse_len(value)?,
            _ => return Err(unknown()),
        },
        ElementData::Grid { name, .. } => match key {
            "name" => *name = non_empty(value)?,
            _ => return Err(unknown()),
        },
        ElementData::WallType {
            name,
            thickness,
            function,
        } => match key {
            "name" => *name = non_empty(value)?,
            "thickness" => *thickness = positive(parse_len(value)?)?,
            "function" => {
                *function = if value == "Exterior" {
                    WallFunction::Exterior
                } else {
                    WallFunction::Interior
                }
            }
            _ => return Err(unknown()),
        },
        ElementData::FloorType { name, thickness }
        | ElementData::CeilingType { name, thickness } => match key {
            "name" => *name = non_empty(value)?,
            "thickness" => *thickness = positive(parse_len(value)?)?,
            _ => return Err(unknown()),
        },
        ElementData::Wall {
            type_id,
            start,
            end,
            base_level,
            base_offset,
            top,
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
                    *height = positive(parse_len(value)?)?;
                }
            }
            "length" => {
                let l = positive(parse_len(value)?)?;
                let dir = end.sub(*start).norm();
                *end = start.add(dir.scale(l));
            }
            _ => return Err(unknown()),
        },
        ElementData::Floor {
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
        ElementData::Ceiling {
            type_id,
            level,
            height,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "level" => *level = parse_id(value)?,
            "height" => *height = parse_len(value)?,
            _ => return Err(unknown()),
        },
        ElementData::View { name, scale, .. } => match key {
            "name" => *name = non_empty(value)?,
            "scale" => {
                *scale = value
                    .parse()
                    .map_err(|_| CoreError::Invalid("bad scale".into()))?
            }
            _ => return Err(unknown()),
        },
        ElementData::ProjectInfo {
            name,
            number,
            client,
            address,
            ..
        } => match key {
            "name" => *name = value.into(),
            "number" => *number = value.into(),
            "client" => *client = value.into(),
            "address" => *address = value.into(),
            _ => return Err(unknown()),
        },
        ElementData::Stage {
            name,
            abbreviation,
            start,
            target,
            ..
        } => match key {
            "name" => *name = non_empty(value)?,
            "abbreviation" => *abbreviation = non_empty(value)?,
            "start" => *start = date(value)?,
            "target" => *target = date(value)?,
            _ => return Err(unknown()),
        },
    }
    let label = format!("Change {}", key.replace('_', " "));
    doc.transact(&label, |tx| {
        tx.set(id, d)?;
        check_type_category(tx, id)
    })
}

/// A wall's type must be a wall type, a floor's a floor type, and so on.
fn check_type_category(tx: &Tx<'_>, id: ElementId) -> CoreResult<()> {
    let data = tx.data(id)?;
    let want = match data {
        ElementData::Wall { .. } => Category::WallType,
        ElementData::Floor { .. } => Category::FloorType,
        ElementData::Ceiling { .. } => Category::CeilingType,
        _ => return Ok(()),
    };
    if let Some(t) = data.type_id() {
        if tx.data(t)?.category() != want {
            return Err(CoreError::Invalid(
                "that type belongs to another category".into(),
            ));
        }
    }
    let levels: Vec<ElementId> = match data {
        ElementData::Wall {
            base_level, top, ..
        } => {
            let mut v = vec![*base_level];
            if let WallTop::UpToLevel { level, .. } = top {
                v.push(*level);
            }
            v
        }
        _ => data.level().into_iter().collect(),
    };
    for l in levels {
        if tx.data(l)?.category() != Category::Level {
            return Err(CoreError::Invalid("constraint must be a level".into()));
        }
    }
    Ok(())
}

fn non_empty(v: &str) -> CoreResult<String> {
    let t = v.trim();
    if t.is_empty() {
        Err(CoreError::Invalid("name can't be empty".into()))
    } else {
        Ok(t.to_owned())
    }
}

fn positive(v: f64) -> CoreResult<f64> {
    if v > 0.0 {
        Ok(v)
    } else {
        Err(CoreError::Invalid("value must be greater than zero".into()))
    }
}

fn date(v: &str) -> CoreResult<String> {
    let t = v.trim();
    let ok = t.is_empty()
        || (t.len() == 10
            && t.as_bytes()[4] == b'-'
            && t.as_bytes()[7] == b'-'
            && t.chars().filter(|c| c.is_ascii_digit()).count() == 8);
    if ok {
        Ok(t.to_owned())
    } else {
        Err(CoreError::Invalid("dates are YYYY-MM-DD".into()))
    }
}

/// First element of a category (e.g. the default wall type).
pub fn first_of(doc: &Document, cat: Category) -> Option<ElementId> {
    let mut v: Vec<_> = doc.of(cat).map(|e| (e.data.name(), e.id)).collect();
    v.sort();
    v.first().map(|x| x.1)
}

/// Type options for a category, for the type selector.
pub fn type_options(doc: &Document, cat: Category) -> Vec<PropOption> {
    options_of(doc, cat)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded() -> Document {
        let mut doc = Document::new();
        seed_default_project(&mut doc).unwrap();
        doc
    }

    fn l(doc: &Document, i: usize) -> ElementId {
        doc.levels()[i].0
    }

    #[test]
    fn default_project_contents() {
        let doc = seeded();
        assert_eq!(doc.levels().len(), 2);
        assert_eq!(doc.of(Category::WallType).count(), 4);
        // 2 levels × (plan + RCP) + 4 elevations + 3D.
        assert_eq!(doc.of(Category::View).count(), 9);
        assert_eq!(stages(&doc).len(), 6);
        let info = project_info(&doc).unwrap();
        let sheet = properties(&doc, info).unwrap();
        let stage = sheet
            .properties
            .iter()
            .find(|p| p.key == "current_stage")
            .unwrap();
        let sd = stages(&doc).into_iter().find(|s| s.2 == "SD").unwrap().0;
        assert_eq!(stage.value, sd.to_string());
    }

    #[test]
    fn wall_top_follows_level_above() {
        let mut doc = seeded();
        let wt = first_of(&doc, Category::WallType).unwrap();
        let l0 = l(&doc, 0);
        let w = create_wall(&mut doc, wt, l0, Pt::new(0.0, 0.0), Pt::new(3000.0, 0.0)).unwrap();
        assert!(matches!(
            doc.data(w).unwrap(),
            ElementData::Wall {
                top: WallTop::UpToLevel { .. },
                ..
            }
        ));
        let l1 = l(&doc, 1);
        let w2 = create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(3000.0, 0.0)).unwrap();
        assert!(matches!(
            doc.data(w2).unwrap(),
            ElementData::Wall {
                top: WallTop::Unconnected { .. },
                ..
            }
        ));
    }

    #[test]
    fn grid_names_increment() {
        assert_eq!(next_grid_name(None), "1");
        assert_eq!(next_grid_name(Some("9")), "10");
        assert_eq!(next_grid_name(Some("A")), "B");
        assert_eq!(next_grid_name(Some("Z")), "AA");
        assert_eq!(next_grid_name(Some("AZ")), "BA");
        assert_eq!(next_grid_name(Some("A1")), "A2");
        let mut doc = seeded();
        create_grid(&mut doc, Pt::new(0.0, 0.0), Pt::new(0.0, 5000.0)).unwrap();
        let g = create_grid(&mut doc, Pt::new(5000.0, 0.0), Pt::new(5000.0, 5000.0)).unwrap();
        assert_eq!(doc.data(g).unwrap().name(), "Grid 2");
    }

    #[test]
    fn set_length_property_moves_end() {
        let mut doc = seeded();
        let wt = first_of(&doc, Category::WallType).unwrap();
        let l0 = l(&doc, 0);
        let w = create_wall(&mut doc, wt, l0, Pt::new(0.0, 0.0), Pt::new(3000.0, 0.0)).unwrap();
        set_property(&mut doc, w, "length", "20'", 0).unwrap();
        match doc.data(w).unwrap() {
            ElementData::Wall { end, .. } => assert!((end.x - 20.0 * MM_PER_FT).abs() < 1e-6),
            _ => unreachable!(),
        }
        assert!(set_property(&mut doc, w, "length", "banana", 0).is_err());
        assert!(
            set_property(&mut doc, w, "type", &l0.to_string(), 0).is_err(),
            "level is not a wall type"
        );
    }

    #[test]
    fn stage_change_is_logged_and_undoable() {
        let mut doc = seeded();
        let dd = stages(&doc).into_iter().find(|s| s.2 == "DD").unwrap().0;
        let info = project_info(&doc).unwrap();
        set_property(&mut doc, info, "current_stage", &dd.to_string(), 1234).unwrap();
        match doc.data(info).unwrap() {
            ElementData::ProjectInfo {
                current_stage,
                stage_history,
                ..
            } => {
                assert_eq!(*current_stage, Some(dd));
                assert_eq!(stage_history.len(), 1);
                assert_eq!(stage_history[0].at, 1234);
            }
            _ => unreachable!(),
        }
        doc.undo().unwrap();
        match doc.data(info).unwrap() {
            ElementData::ProjectInfo { stage_history, .. } => assert!(stage_history.is_empty()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn creating_a_level_adds_views_and_deleting_removes_them() {
        let mut doc = seeded();
        let l3 = create_level(&mut doc, 2.0 * DEFAULT_FLOOR_TO_FLOOR).unwrap();
        assert_eq!(doc.data(l3).unwrap().name(), "Level 3");
        assert_eq!(doc.of(Category::View).count(), 11);
        delete(&mut doc, &[l3]).unwrap();
        assert_eq!(doc.of(Category::View).count(), 9);
    }

    #[test]
    fn project_info_cannot_be_deleted() {
        let mut doc = seeded();
        let info = project_info(&doc).unwrap();
        assert!(delete(&mut doc, &[info]).is_err());
    }
}

/// Compares names with embedded numbers by value: `8" Stud` sorts before `12" CMU`.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let key = |s: &str| {
        let mut parts: Vec<(String, f64)> = vec![];
        let mut text = String::new();
        let mut num = String::new();
        for c in s.chars() {
            if c.is_ascii_digit() || (c == '.' && !num.is_empty()) {
                num.push(c);
            } else {
                if !num.is_empty() {
                    parts.push((std::mem::take(&mut text), num.parse().unwrap_or(0.0)));
                    num.clear();
                }
                text.push(c.to_ascii_lowercase());
            }
        }
        parts.push((text, num.parse().unwrap_or(-1.0)));
        parts
    };
    let (ka, kb) = (key(a), key(b));
    for (x, y) in ka.iter().zip(&kb) {
        let o = x.0.cmp(&y.0).then(x.1.total_cmp(&y.1));
        if o != std::cmp::Ordering::Equal {
            return o;
        }
    }
    ka.len().cmp(&kb.len())
}

#[cfg(test)]
mod natural_tests {
    use super::natural_cmp;

    #[test]
    fn numbers_sort_by_value() {
        let mut v = vec![
            "Exterior - 12\" CMU",
            "Exterior - 8\" Stud",
            "Interior - 6\" Stud",
            "Interior - 4 7/8\" Partition",
        ];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            v,
            [
                "Exterior - 8\" Stud",
                "Exterior - 12\" CMU",
                "Interior - 4 7/8\" Partition",
                "Interior - 6\" Stud"
            ]
        );
        let mut l = vec!["Level 10", "Level 2", "Level 1"];
        l.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(l, ["Level 1", "Level 2", "Level 10"]);
    }
}
