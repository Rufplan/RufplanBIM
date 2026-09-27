//! Edit Model with Claude (ADR-050): a plain-language change ("make all doors 3'-0" wide",
//! "make the building 50'-0" wide") comes back from Claude as a structured [`ModelEdit`].
//! This module checks it against the model, previews it (on a copy of the document) and
//! applies it as one undo step. Claude only proposes; every change goes through the same
//! properties and transactions as the Properties panel.
//!
//! Two kinds of edit:
//! - `set_parameter`: any writable property of the elements of a category, as the
//!   Properties panel shows it, over the whole model, a level, the selection or the view.
//!   A type parameter (a door's width) moves the instances to a type with that value,
//!   reusing one that matches or making one, as you would in Revit.
//! - `resize_building`: the building's overall width (east–west) or depth (north–south),
//!   stretched across its middle like Revit's Stretch. Walls, floors, roofs, grids and
//!   everything else past the middle move; walls that cross it get longer or shorter.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, ViewKind};
use crate::ops::{parse_len, properties, set_property, PropKind, Property};
use crate::units::{format_ft_in, MM_PER_FT};

/// What Claude proposes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ModelEdit {
    /// `set_parameter`, `resize_building`, or `none` (with `message` saying why).
    pub action: String,
    /// Walls, Doors, Windows, Floors, Ceilings, Roofs, Rooms, Columns, Beams, Railings,
    /// Stairs, Levels or Grids.
    pub category: String,
    /// A property as Properties names it: "Width", "Unconnected Height", "Sill Height"…
    pub parameter: String,
    /// The new value: `3'-0"`, a type name, a choice's label.
    pub value: String,
    /// `model`, `level`, `selection` or `view`.
    pub scope: String,
    /// The level's name, for `level` scope.
    pub level: String,
    /// Only elements whose type name contains this (e.g. "Single").
    pub type_filter: String,
    /// For `resize_building`: `east-west` (width) or `north-south` (depth).
    pub axis: String,
    /// For `resize_building`: the side that stays put — `west`, `east`, `south`, `north`
    /// or `center`.
    pub anchor: String,
    /// One line: "Width → 3'-0" on 7 doors".
    pub summary: String,
    /// For `none`: why the request can't be done.
    pub message: String,
}

/// What an edit changes, for the preview card.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditPreview {
    pub category: String,
    pub parameter: String,
    pub from: String,
    pub to: String,
    pub scope: String,
    pub count: usize,
    /// The elements it changes, to highlight.
    pub ids: Vec<ElementId>,
    pub summary: String,
}

/// Where the request was made: the selection, and the active view's level and contents.
#[derive(Debug, Clone, Default)]
pub struct EditContext {
    pub selection: Vec<ElementId>,
    pub view_level: Option<ElementId>,
    /// Elements drawn in the active view (for `view` scope).
    pub view_ids: Option<Vec<ElementId>>,
    pub view_name: String,
}

/// The categories an edit can address, by the names Claude is given.
pub const CATEGORIES: [(&str, Category); 13] = [
    ("Walls", Category::Wall),
    ("Doors", Category::Door),
    ("Windows", Category::Window),
    ("Floors", Category::Floor),
    ("Ceilings", Category::Ceiling),
    ("Roofs", Category::Roof),
    ("Rooms", Category::Room),
    ("Columns", Category::Column),
    ("Beams", Category::Beam),
    ("Railings", Category::Railing),
    ("Stairs", Category::Stair),
    ("Levels", Category::Level),
    ("Grids", Category::Grid),
];

fn norm(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn category_of(name: &str) -> CoreResult<(&'static str, Category)> {
    let n = norm(name);
    CATEGORIES
        .iter()
        .find(|(label, c)| {
            let l = norm(label);
            l == n || l.trim_end_matches('s') == n || norm(c.as_str()) == n
        })
        .copied()
        .ok_or_else(|| {
            CoreError::Invalid(format!(
                "no category \"{name}\" (try walls, doors, windows, floors, ceilings, roofs, rooms, columns, beams, railings, stairs, levels or grids)"
            ))
        })
}

fn bad(msg: impl Into<String>) -> CoreError {
    CoreError::Invalid(msg.into())
}

/// A writable property that matches `parameter`, on the element or on its type.
enum Found {
    Instance(Property),
    Type(ElementId, Property),
}

fn writable(p: &Property) -> bool {
    !matches!(p.kind, PropKind::ReadOnly | PropKind::Action)
}

fn find_prop(doc: &Document, id: ElementId, parameter: &str) -> Option<Found> {
    let want = norm(parameter);
    let hit = |props: Vec<Property>| {
        props
            .into_iter()
            .find(|p| writable(p) && (norm(&p.label) == want || norm(&p.key) == want))
    };
    if let Some(p) = properties(doc, id).ok().and_then(|s| hit(s.properties)) {
        return Some(Found::Instance(p));
    }
    let t = doc.data(id).ok()?.type_id()?;
    let p = properties(doc, t).ok().and_then(|s| hit(s.properties))?;
    Some(Found::Type(t, p))
}

/// The value to set on `p` for what Claude wrote: a choice by its id or label, a length
/// checked, text as it is.
fn value_for(p: &Property, value: &str) -> CoreResult<String> {
    match p.kind {
        PropKind::Choice => {
            let v = norm(value);
            p.options
                .iter()
                .find(|o| norm(&o.id) == v || norm(&o.label) == v)
                .or_else(|| {
                    // A partial name ("Double Flush" for "Double Flush 72" x 80"").
                    let hits: Vec<_> = p
                        .options
                        .iter()
                        .filter(|o| !v.is_empty() && norm(&o.label).contains(&v))
                        .collect();
                    (hits.len() == 1).then(|| hits[0])
                })
                .map(|o| o.id.clone())
                .ok_or_else(|| {
                    let labels: Vec<&str> = p.options.iter().map(|o| o.label.as_str()).collect();
                    bad(format!(
                        "{} can't be \"{value}\"; it can be {}",
                        p.label,
                        labels.join(", ")
                    ))
                })
        }
        PropKind::Length => {
            parse_len(value)?;
            Ok(value.to_owned())
        }
        _ => Ok(value.to_owned()),
    }
}

/// How a property reads (a choice's label, not its id).
fn shown(p: &Property) -> String {
    match p.kind {
        PropKind::Choice => p
            .options
            .iter()
            .find(|o| o.id == p.value)
            .map_or_else(|| p.value.clone(), |o| o.label.clone()),
        _ => p.value.clone(),
    }
}

fn describe_values(values: &BTreeSet<String>, lengths: bool) -> String {
    match values.len() {
        0 => "—".into(),
        1 => values.iter().next().cloned().unwrap_or_default(),
        _ if lengths => {
            let mut mm: Vec<f64> = values.iter().filter_map(|v| parse_len(v).ok()).collect();
            mm.sort_by(f64::total_cmp);
            match (mm.first(), mm.last()) {
                (Some(a), Some(b)) => {
                    format!("Varies ({} – {})", format_ft_in(*a), format_ft_in(*b))
                }
                _ => "Varies".into(),
            }
        }
        _ => "Varies".into(),
    }
}

fn level_named(doc: &Document, name: &str) -> CoreResult<ElementId> {
    let n = norm(name);
    doc.levels()
        .into_iter()
        .find(|(_, l, _)| norm(l) == n)
        .map(|l| l.0)
        .ok_or_else(|| bad(format!("no level named \"{name}\"")))
}

/// The elements of `cat` the scope and type filter pick, and how the scope reads.
fn candidates(
    doc: &Document,
    cat: Category,
    edit: &ModelEdit,
    ctx: &EditContext,
) -> CoreResult<(Vec<ElementId>, String)> {
    let mut ids: Vec<ElementId> = doc.of(cat).map(|e| e.id).collect();
    let scope = match norm(&edit.scope).as_str() {
        "" | "model" | "all" => "Entire model".to_owned(),
        "level" => {
            let level = if edit.level.trim().is_empty() {
                ctx.view_level.ok_or_else(|| bad("say which level"))?
            } else {
                level_named(doc, &edit.level)?
            };
            ids.retain(|id| {
                *id == level
                    || doc.data(*id).is_ok_and(|d| {
                        d.level() == Some(level)
                            || matches!(d, ElementData::Door { host, .. } | ElementData::Window { host, .. }
                                if doc.data(*host).is_ok_and(|w| w.level() == Some(level)))
                    })
            });
            doc.data(level).map(|d| d.name()).unwrap_or_default()
        }
        "selection" => {
            ids.retain(|id| ctx.selection.contains(id));
            "Selection".into()
        }
        "view" => {
            if let Some(v) = &ctx.view_ids {
                ids.retain(|id| v.contains(id));
            }
            format!("In {}", ctx.view_name)
        }
        other => return Err(bad(format!("unknown scope {other}"))),
    };
    if !edit.type_filter.trim().is_empty() {
        let f = norm(&edit.type_filter);
        ids.retain(|id| {
            doc.data(*id)
                .ok()
                .and_then(ElementData::type_id)
                .and_then(|t| doc.data(t).ok())
                .is_some_and(|t| norm(&t.name()).contains(&f))
        });
    }
    Ok((ids, scope))
}

/// A type's data with its name blanked, to find a type that already has the new values.
fn nameless(d: &ElementData) -> ElementData {
    let mut d = d.clone();
    match &mut d {
        ElementData::WallType { name, .. }
        | ElementData::FloorType { name, .. }
        | ElementData::CeilingType { name, .. }
        | ElementData::DoorType { name, .. }
        | ElementData::WindowType { name, .. }
        | ElementData::RoofType { name, .. }
        | ElementData::ColumnType { name, .. }
        | ElementData::BeamType { name, .. }
        | ElementData::RailingType { name, .. }
        | ElementData::ElevationMarkerType { name, .. } => name.clear(),
        _ => {}
    }
    d
}

/// A type like `t` with `key` set to `value`: an existing one that already matches, or a
/// new one named for its size.
fn variant_type(
    doc: &mut Document,
    t: ElementId,
    p: &Property,
    value: &str,
) -> CoreResult<ElementId> {
    let data = doc.data(t)?.clone();
    let cat = data.category();
    let copy = doc.transact("Duplicate type", |tx| Ok(tx.insert(data.clone())))?;
    set_property(doc, copy, &p.key, value, 0)?;
    let new = doc.data(copy)?.clone();
    let want = nameless(&new);
    let same = doc
        .of(cat)
        .find(|e| e.id != copy && nameless(&e.data) == want)
        .map(|e| e.id);
    if let Some(same) = same {
        doc.transact("Remove duplicate", |tx| tx.delete(copy).map(|_| ()))?;
        return Ok(same);
    }
    let name = match &new {
        ElementData::DoorType { width, height, .. } => {
            crate::doors::DoorStyle::of(&new).map(|s| crate::doors::type_name(s, *width, *height))
        }
        ElementData::WindowType { width, height, .. } => crate::windows::WindowStyle::of(&new)
            .map(|s| crate::windows::type_name(s, *width, *height)),
        _ => None,
    }
    .unwrap_or_else(|| format!("{} ({} {})", data.name(), p.label, value.trim()));
    // Types are known by name, so a clash gets a number.
    let taken = |n: &str| doc.of(cat).any(|e| e.id != copy && e.data.name() == n);
    let mut unique = name.clone();
    let mut k = 2;
    while taken(&unique) {
        unique = format!("{name} {k}");
        k += 1;
    }
    set_property(doc, copy, "name", &unique, 0)?;
    Ok(copy)
}

fn set_parameter(
    doc: &mut Document,
    edit: &ModelEdit,
    ctx: &EditContext,
) -> CoreResult<EditPreview> {
    let (label, cat) = category_of(&edit.category)?;
    let (ids, scope) = candidates(doc, cat, edit, ctx)?;
    if ids.is_empty() {
        return Err(bad(format!(
            "no {} in {}",
            label.to_lowercase(),
            scope.to_lowercase()
        )));
    }
    if edit.value.trim().is_empty() {
        return Err(bad(format!("say what to set {} to", edit.parameter)));
    }
    let found: Vec<(ElementId, Found)> = ids
        .iter()
        .filter_map(|id| find_prop(doc, *id, &edit.parameter).map(|f| (*id, f)))
        .collect();
    if found.is_empty() {
        let names: BTreeSet<String> = ids
            .iter()
            .take(20)
            .flat_map(|id| {
                let mut v: Vec<String> = properties(doc, *id)
                    .map(|s| {
                        s.properties
                            .into_iter()
                            .filter(writable)
                            .map(|p| p.label)
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(t) = doc.data(*id).ok().and_then(ElementData::type_id) {
                    v.extend(
                        properties(doc, t)
                            .map(|s| {
                                s.properties
                                    .into_iter()
                                    .filter(writable)
                                    .map(|p| p.label)
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default(),
                    );
                }
                v
            })
            .collect();
        return Err(bad(format!(
            "{} have no \"{}\" to change; they have {}",
            label,
            edit.parameter,
            names.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }
    let first = match &found[0].1 {
        Found::Instance(p) | Found::Type(_, p) => p.clone(),
    };
    let lengths = first.kind == PropKind::Length;
    let before: BTreeSet<String> = found
        .iter()
        .map(|(_, f)| match f {
            Found::Instance(p) | Found::Type(_, p) => shown(p),
        })
        .collect();
    let value = value_for(&first, &edit.value)?;
    let to = if lengths {
        format_ft_in(parse_len(&value)?)
    } else if first.kind == PropKind::Choice {
        first
            .options
            .iter()
            .find(|o| o.id == value)
            .map_or_else(|| value.clone(), |o| o.label.clone())
    } else {
        value.clone()
    };
    // Type parameters: one variant per type, then the instances move to it.
    let mut variants: HashMap<ElementId, ElementId> = HashMap::new();
    let mut changed = vec![];
    for (id, f) in &found {
        match f {
            Found::Instance(p) => {
                if shown(p) == to || p.value == value {
                    continue;
                }
                set_property(doc, *id, &p.key, &value_for(p, &edit.value)?, 0)?;
            }
            Found::Type(t, p) => {
                if shown(p) == to {
                    continue;
                }
                let v = match variants.get(t) {
                    Some(v) => *v,
                    None => {
                        let v = variant_type(doc, *t, p, &value)?;
                        variants.insert(*t, v);
                        v
                    }
                };
                set_property(doc, *id, "type", &v.to_string(), 0)?;
            }
        }
        changed.push(*id);
    }
    if changed.is_empty() {
        return Err(bad(format!(
            "every one of those {} already has {} {}",
            label.to_lowercase(),
            first.label,
            to
        )));
    }
    let noun = if changed.len() == 1 {
        label.trim_end_matches('s').to_lowercase()
    } else {
        label.to_lowercase()
    };
    Ok(EditPreview {
        category: label.into(),
        parameter: first.label.clone(),
        from: describe_values(&before, lengths),
        to: to.clone(),
        scope,
        count: changed.len(),
        summary: format!("{} → {} on {} {}", first.label, to, changed.len(), noun),
        ids: changed,
    })
}

/// The building's extent along x (`east_west`) or y, from its walls' faces.
pub fn building_extent(doc: &Document, east_west: bool) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for e in doc.of(Category::Wall) {
        let ElementData::Wall {
            type_id,
            start,
            end,
            ..
        } = &e.data
        else {
            continue;
        };
        let t = match doc.data(*type_id) {
            Ok(ElementData::WallType { thickness, .. }) => *thickness,
            _ => 0.0,
        };
        let d = end.sub(*start).norm();
        // Half the thickness out from the centerline, across the wall.
        let half = (if east_west { d.y } else { d.x }).abs() * t / 2.0;
        for p in [start, end] {
            let c = if east_west { p.x } else { p.y };
            lo = lo.min(c - half);
            hi = hi.max(c + half);
        }
    }
    (hi > lo).then_some((lo, hi))
}

fn resize_building(doc: &mut Document, edit: &ModelEdit) -> CoreResult<EditPreview> {
    let a = norm(&edit.axis);
    let east_west =
        !(a.contains("north") || a.contains("south") || a == "y" || a.contains("depth"));
    let length = parse_len(&edit.value)?;
    if length < 3.0 * MM_PER_FT {
        return Err(bad("that's too small for a building"));
    }
    let (lo, hi) = building_extent(doc, east_west).ok_or_else(|| bad("draw walls first"))?;
    let current = hi - lo;
    let delta = length - current;
    if delta.abs() < 1.0 {
        return Err(bad(format!(
            "the building is already {} {}",
            format_ft_in(current),
            if east_west { "wide" } else { "deep" }
        )));
    }
    let cut = (lo + hi) / 2.0;
    let anchor = norm(&edit.anchor);
    // How far a coordinate moves: the far side from the anchor moves the whole change.
    let (neg, pos) = match anchor.as_str() {
        "east" | "north" => (-delta, 0.0),
        "center" | "centre" | "middle" => (-delta / 2.0, delta / 2.0),
        _ => (0.0, delta),
    };
    let shift = move |c: f64| if c > cut { pos } else { neg };
    let f = move |p: Pt| {
        if east_west {
            Pt::new(p.x + shift(p.x), p.y)
        } else {
            Pt::new(p.x, p.y + shift(p.y))
        }
    };
    let changed = crate::modify::stretch(doc, &f, "Resize building")?;
    let dir = if east_west { "Width" } else { "Depth" };
    Ok(EditPreview {
        category: "Building".into(),
        parameter: format!(
            "Overall {} ({})",
            dir.to_lowercase(),
            if east_west {
                "east–west"
            } else {
                "north–south"
            }
        ),
        from: format_ft_in(current),
        to: format_ft_in(length),
        scope: match anchor.as_str() {
            "east" | "north" => format!(
                "Stretched {}, {} side fixed",
                if east_west { "west" } else { "south" },
                anchor
            ),
            "center" | "centre" | "middle" => "Stretched both ways from the middle".into(),
            _ => format!(
                "Stretched {}, {} side fixed",
                if east_west { "east" } else { "north" },
                if east_west { "west" } else { "south" }
            ),
        },
        count: changed.len(),
        summary: format!(
            "{dir} → {} ({} elements)",
            format_ft_in(length),
            changed.len()
        ),
        ids: changed,
    })
}

fn run(doc: &mut Document, edit: &ModelEdit, ctx: &EditContext) -> CoreResult<EditPreview> {
    match norm(&edit.action).as_str() {
        "setparameter" => set_parameter(doc, edit, ctx),
        "resizebuilding" => resize_building(doc, edit),
        _ => Err(bad(if edit.message.trim().is_empty() {
            "Claude couldn't turn that into a model change".to_owned()
        } else {
            edit.message.clone()
        })),
    }
}

/// What `edit` would change, worked out on a copy of the document.
pub fn preview(doc: &Document, edit: &ModelEdit, ctx: &EditContext) -> CoreResult<EditPreview> {
    let mut copy = doc.clone();
    run(&mut copy, edit, ctx)
}

/// Applies `edit` as one undo step named for its summary; on an error nothing changes.
pub fn apply(doc: &mut Document, edit: &ModelEdit, ctx: &EditContext) -> CoreResult<EditPreview> {
    let mark = doc.undo_depth();
    match run(doc, edit, ctx) {
        Ok(p) => {
            doc.merge_undo(mark, &undo_label(&p));
            Ok(p)
        }
        Err(e) => {
            while doc.undo_depth() > mark {
                doc.undo()?;
            }
            Err(e)
        }
    }
}

/// The undo step an applied edit makes, so the history can find it on the undo stack.
pub fn undo_label(p: &EditPreview) -> String {
    format!("Edit model: {}", p.summary)
}

/// The model as Claude is told about it: levels, the building's size, and for each
/// category its count, types and the properties that can be changed (with their values).
pub fn describe(doc: &Document, ctx: &EditContext) -> String {
    let mut out = String::new();
    let ft = |mm: f64| format_ft_in(mm);
    out.push_str("Levels: ");
    let levels: Vec<String> = doc
        .levels()
        .into_iter()
        .map(|(_, n, e)| format!("{n} at {}", ft(e)))
        .collect();
    out.push_str(&levels.join("; "));
    out.push('\n');
    if let (Some((x0, x1)), Some((y0, y1))) =
        (building_extent(doc, true), building_extent(doc, false))
    {
        out.push_str(&format!(
            "Building (outside faces of walls): {} wide east–west × {} deep north–south.\n",
            ft(x1 - x0),
            ft(y1 - y0)
        ));
    }
    out.push_str(&format!("Active view: {}.\n", ctx.view_name));
    if !ctx.selection.is_empty() {
        let mut counts: BTreeSet<String> = BTreeSet::new();
        for id in &ctx.selection {
            if let Ok(d) = doc.data(*id) {
                counts.insert(d.category().as_str().to_owned());
            }
        }
        out.push_str(&format!(
            "Selection: {} elements ({}).\n",
            ctx.selection.len(),
            counts.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for (label, cat) in CATEGORIES {
        let ids: Vec<ElementId> = doc.of(cat).map(|e| e.id).collect();
        if ids.is_empty() {
            continue;
        }
        out.push_str(&format!("\n{label}: {}\n", ids.len()));
        // Each property with the values it has (a few), instance then type.
        let mut rows: Vec<(String, BTreeSet<String>)> = vec![];
        let mut add = |p: &Property, prefix: &str| {
            let name = format!("{prefix}{}", p.label);
            let v = shown(p);
            match rows.iter_mut().find(|r| r.0 == name) {
                Some(r) => {
                    r.1.insert(v);
                }
                None => rows.push((name, BTreeSet::from([v]))),
            }
        };
        let mut types = BTreeSet::new();
        for id in ids.iter().take(60) {
            if let Ok(s) = properties(doc, *id) {
                for p in s.properties.iter().filter(|p| writable(p)) {
                    add(p, "");
                }
            }
            if let Some(t) = doc.data(*id).ok().and_then(ElementData::type_id) {
                if let Ok(d) = doc.data(t) {
                    types.insert(d.name());
                }
                if let Ok(s) = properties(doc, t) {
                    for p in s
                        .properties
                        .iter()
                        .filter(|p| writable(p) && p.key != "name")
                    {
                        add(p, "type: ");
                    }
                }
            }
        }
        if !types.is_empty() {
            out.push_str(&format!(
                "  types in use: {}\n",
                types.into_iter().collect::<Vec<_>>().join(" | ")
            ));
        }
        for (name, vals) in rows {
            let vals: Vec<String> = vals.into_iter().take(6).collect();
            out.push_str(&format!("  {name}: {}\n", vals.join(" | ")));
        }
        if cat == Category::Door || cat == Category::Window {
            let t = if cat == Category::Door {
                Category::DoorType
            } else {
                Category::WindowType
            };
            let names: Vec<String> = doc.of(t).map(|e| e.data.name()).take(40).collect();
            out.push_str(&format!("  types loaded: {}\n", names.join(" | ")));
        }
    }
    let _ = ViewKind::ThreeD;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    /// A 40' × 30' box of walls on Level 1 with two doors.
    fn house() -> (Document, Vec<ElementId>, Vec<ElementId>) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let (w, d) = (40.0 * MM_PER_FT, 30.0 * MM_PER_FT);
        let pts = [
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, d),
            Pt::new(0.0, d),
        ];
        let walls: Vec<ElementId> = (0..4)
            .map(|i| ops::create_wall(&mut doc, wt, l1, pts[i], pts[(i + 1) % 4]).unwrap())
            .collect();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Six-Panel 32"))
            .map(|e| e.id)
            .unwrap();
        let doors = vec![
            ops::create_door(&mut doc, dt, walls[0], 10.0 * MM_PER_FT, false).unwrap(),
            ops::create_door(&mut doc, dt, walls[0], 30.0 * MM_PER_FT, false).unwrap(),
        ];
        (doc, walls, doors)
    }

    fn edit(action: &str, category: &str, parameter: &str, value: &str) -> ModelEdit {
        ModelEdit {
            action: action.into(),
            category: category.into(),
            parameter: parameter.into(),
            value: value.into(),
            scope: "model".into(),
            ..Default::default()
        }
    }

    fn door_width(doc: &Document, door: ElementId) -> f64 {
        let t = doc.data(door).unwrap().type_id().unwrap();
        match doc.data(t).unwrap() {
            ElementData::DoorType { width, .. } => *width,
            _ => panic!(),
        }
    }

    #[test]
    fn doors_move_to_a_type_of_the_new_width_in_one_undo() {
        let (mut doc, _, doors) = house();
        let ctx = EditContext::default();
        let e = edit("set_parameter", "Doors", "Width", "3'-0\"");
        let before = doc.clone();
        // The preview changes nothing.
        let p = preview(&doc, &e, &ctx).unwrap();
        assert_eq!(
            (p.count, p.from.as_str(), p.to.as_str()),
            (2, "2'-8\"", "3'-0\"")
        );
        assert_eq!(p.summary, "Width → 3'-0\" on 2 doors");
        assert!((door_width(&doc, doors[0]) - 32.0 * 25.4).abs() < 1e-6);
        let p = apply(&mut doc, &e, &ctx).unwrap();
        for d in &doors {
            assert!((door_width(&doc, *d) - 36.0 * 25.4).abs() < 1e-6);
        }
        // The same type for both, named for its size (an existing 36" one if loaded).
        let t = doc.data(doors[0]).unwrap().type_id().unwrap();
        assert_eq!(doc.data(doors[1]).unwrap().type_id(), Some(t));
        assert!(
            doc.data(t).unwrap().name().contains("36\""),
            "{}",
            doc.data(t).unwrap().name()
        );
        assert_eq!(doc.can_undo(), Some(undo_label(&p).as_str()));
        doc.undo().unwrap();
        assert!((door_width(&doc, doors[0]) - 32.0 * 25.4).abs() < 1e-6);
        assert_eq!(
            doc.count(Category::DoorType),
            before.count(Category::DoorType)
        );
    }

    #[test]
    fn instance_parameters_scopes_and_refusals() {
        let (mut doc, walls, _) = house();
        let ctx = EditContext {
            selection: vec![walls[0]],
            ..Default::default()
        };
        // Wall heights, only the selected wall.
        let mut e = edit("set_parameter", "walls", "Top Offset", "1'");
        e.scope = "selection".into();
        let p = apply(&mut doc, &e, &ctx).unwrap();
        assert_eq!(p.count, 1);
        assert_eq!(p.ids, vec![walls[0]]);
        // Level scope by name.
        let mut e = edit("set_parameter", "Walls", "top offset", "6\"");
        e.scope = "level".into();
        e.level = "level 1".into();
        assert_eq!(preview(&doc, &e, &ctx).unwrap().count, 4);
        // Refusals: no such parameter, a bad value, nothing to do, Claude said no.
        let err = |e: &ModelEdit| preview(&doc, e, &ctx).unwrap_err().to_string();
        assert!(err(&edit("set_parameter", "Doors", "Color", "red")).contains("no \"Color\""));
        assert!(err(&edit("set_parameter", "Doors", "Width", "wide")).contains("length"));
        assert!(err(&edit("set_parameter", "Doors", "Width", "2'-8\"")).contains("already"));
        assert!(err(&edit("set_parameter", "Spaceships", "Width", "1'")).contains("no category"));
        let mut none = edit("none", "", "", "");
        none.message = "I can only change the model".into();
        assert_eq!(err(&none), "I can only change the model");
        // A failed apply leaves nothing behind.
        let depth = doc.undo_depth();
        assert!(apply(
            &mut doc,
            &edit("set_parameter", "Doors", "Width", "wide"),
            &ctx
        )
        .is_err());
        assert_eq!(doc.undo_depth(), depth);
    }

    #[test]
    fn the_building_stretches_to_a_new_width() {
        let (mut doc, walls, doors) = house();
        let (lo, hi) = building_extent(&doc, true).unwrap();
        let t = hi - lo - 40.0 * MM_PER_FT; // the wall thickness
        let ctx = EditContext::default();
        let target = 50.0 * MM_PER_FT;
        let mut e = edit("resize_building", "", "", "50'-0\"");
        e.axis = "east-west".into();
        let p = apply(&mut doc, &e, &ctx).unwrap();
        assert_eq!(p.to, "50'-0\"");
        let (lo2, hi2) = building_extent(&doc, true).unwrap();
        assert!((hi2 - lo2 - target).abs() < 1e-6, "{}", hi2 - lo2);
        assert!((lo2 - lo).abs() < 1e-9, "the west side stays");
        // The east wall moved; the south wall stretched; its east door moved with it.
        let wall = |id| match doc.data(id).unwrap() {
            ElementData::Wall { start, end, .. } => (*start, *end),
            _ => panic!(),
        };
        let grow = target - 40.0 * MM_PER_FT - t;
        assert!((wall(walls[1]).0.x - (40.0 * MM_PER_FT + grow)).abs() < 1e-6);
        assert!((wall(walls[0]).1.x - (40.0 * MM_PER_FT + grow)).abs() < 1e-6);
        let off = |id| match doc.data(id).unwrap() {
            ElementData::Door { offset, .. } => *offset,
            _ => panic!(),
        };
        assert!(
            (off(doors[0]) - 10.0 * MM_PER_FT).abs() < 1e-6,
            "west of the middle: stays"
        );
        assert!(
            (off(doors[1]) - (30.0 * MM_PER_FT + grow)).abs() < 1e-6,
            "east: moves"
        );
        doc.undo().unwrap();
        let (lo3, hi3) = building_extent(&doc, true).unwrap();
        assert!((hi3 - lo3 - (hi - lo)).abs() < 1e-9);
    }

    #[test]
    fn claude_is_told_what_can_change() {
        let (doc, _, _) = house();
        let d = describe(&doc, &EditContext::default());
        assert!(d.contains("Walls: 4"));
        assert!(d.contains("Doors: 2"));
        assert!(d.contains("type: Width: 2'-8\""), "{d}");
        assert!(d.contains("wide east–west"));
    }
}
