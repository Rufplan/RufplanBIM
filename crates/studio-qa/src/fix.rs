//! Fix Issues (ADR-089): a concrete repair for each finding the model can fix on its own —
//! renumbering, tagging, stair and ceiling dimensions, moving openings into their walls,
//! swapping to code-compliant types, adding membrane layers (weather barrier,
//! dampproofing, underlayment, vapor retarder), placing views on sheets, schedules and the
//! project manual. Fixes that change the design (deleting elements, roof slopes, adding
//! windows) are marked so they can be approved one by one. What needs judgment (a room's
//! use, a missing exit, a TBD) is left for the architect.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use studio_core::element::{LayerFunction, WallLayer};
use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::{Category as Cat, CoreError, CoreResult, Document, ElementData, ElementId};
use studio_geom::{point_in_ring, Pt};
use studio_regen::Model;
use ts_rs::TS;

use crate::ctx::{clear_opening, has_any, Ctx};
use crate::{Finding, Report};

/// What a fix does to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "QaFixAction")]
#[serde(tag = "kind")]
pub enum Action {
    /// A text property (a mark, number or name) through the Properties rules.
    SetText {
        id: ElementId,
        key: String,
        value: String,
    },
    /// Tags elements in a plan.
    Tag {
        view: ElementId,
        ids: Vec<ElementId>,
    },
    Stair {
        id: ElementId,
        max_riser: Option<f64>,
        tread: Option<f64>,
        width: Option<f64>,
        railings: Option<bool>,
    },
    CeilingHeight {
        id: ElementId,
        height: f64,
    },
    /// Moves a door or window along its wall, or changes a window's sill.
    Opening {
        id: ElementId,
        offset: Option<f64>,
        sill: Option<f64>,
    },
    /// Gives elements another type.
    SwapType {
        ids: Vec<ElementId>,
        type_id: ElementId,
    },
    /// Adds a membrane layer to a wall, floor or roof type.
    AddLayer {
        type_id: ElementId,
        index: usize,
        name: String,
    },
    RailingHeight {
        type_id: ElementId,
        height: f64,
    },
    /// Roof slope, rise per unit run.
    RoofSlope {
        id: ElementId,
        slope: f64,
    },
    Delete {
        ids: Vec<ElementId>,
    },
    AddWindow {
        wall: ElementId,
        type_id: ElementId,
        offset: f64,
        sill: f64,
    },
    /// Adds the standard schedules the project is missing.
    Schedules,
    /// A new sheet with these views on it.
    PlaceOnSheet {
        views: Vec<ElementId>,
        name: String,
    },
    GenerateSpecs,
    AddSpecs {
        numbers: Vec<String>,
    },
    IncludeSpecs {
        numbers: Vec<String>,
    },
    /// Gives doors a 20-minute solid-core copy of their type (ADR-094).
    RatedDoor {
        ids: Vec<ElementId>,
    },
    /// Gives walls a wet-wall copy of their type: its gypsum faces become glass-mat tile
    /// backer board (ADR-094).
    WetWall {
        type_id: ElementId,
        walls: Vec<ElementId>,
    },
    /// Adds a section to the manual (one Claude wrote), or replaces the one of its number.
    AddSection {
        section: studio_core::specs::SpecSection,
    },
    /// Edits references to Section `to` out of the `from` sections (ADR-094).
    StripSpecRef {
        from: Vec<String>,
        to: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "QaFix")]
pub struct Fix {
    /// The finding it fixes.
    pub finding: String,
    pub title: String,
    /// What will change, in words.
    pub change: String,
    /// It changes the design (deletes, reshapes or adds), not just the documents.
    pub design_change: bool,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "QaFixPlan")]
pub struct Plan {
    pub fixes: Vec<Fix>,
    /// Findings that need the architect: id and why.
    pub manual: Vec<(String, String)>,
}

fn inch(mm: f64) -> String {
    format!("{:.0}\"", mm / MM_PER_IN)
}

/// The first MasterFormat number in a text ("… section: 07 25 00 WEATHER BARRIERS").
fn first_number(text: &str) -> Option<String> {
    let b = text.as_bytes();
    (0..b.len().saturating_sub(7)).find_map(|i| {
        let s = text.get(i..i + 8)?;
        (studio_core::specs::valid_number(s) && (i == 0 || !b[i - 1].is_ascii_digit())).then(|| {
            // Keep a ".NN" suffix.
            match text.get(i..i + 11) {
                Some(l) if studio_core::specs::valid_number(l) => l.to_string(),
                _ => s.to_string(),
            }
        })
    })
}

struct Planner<'a> {
    c: Ctx<'a>,
    fixes: Vec<Fix>,
    manual: Vec<(String, String)>,
    /// Marks handed out, per category.
    marks: HashMap<&'static str, u32>,
    taken: HashMap<&'static str, HashSet<String>>,
}

impl Planner<'_> {
    fn add(
        &mut self,
        f: &Finding,
        title: impl Into<String>,
        change: impl Into<String>,
        design: bool,
        action: Action,
    ) {
        self.fixes.push(Fix {
            finding: f.id.clone(),
            title: title.into(),
            change: change.into(),
            design_change: design,
            action,
        });
    }
    fn manual(&mut self, f: &Finding, why: &str) {
        self.manual.push((f.id.clone(), why.into()));
    }
    fn next_mark(&mut self, kind: &'static str) -> String {
        let taken = self.taken.entry(kind).or_default();
        let n = self.marks.entry(kind).or_insert_with(|| {
            taken
                .iter()
                .filter_map(|m| m.parse::<u32>().ok())
                .max()
                .unwrap_or(0)
        });
        loop {
            *n += 1;
            let s = n.to_string();
            if taken.insert(s.clone()) {
                return s;
            }
        }
    }
    fn types(&self, cat: Cat) -> Vec<(ElementId, &ElementData)> {
        self.c.doc.of(cat).map(|e| (e.id, &e.data)).collect()
    }
}

/// The fixes for a report's findings.
pub fn plan(doc: &Document, model: &Model, report: &Report) -> Plan {
    let c = Ctx::new(doc, model, report.milestone);
    let mut taken: HashMap<&'static str, HashSet<String>> = HashMap::new();
    taken.insert(
        "door",
        c.openings
            .iter()
            .filter(|o| o.door)
            .map(|o| o.mark.clone())
            .collect(),
    );
    taken.insert(
        "window",
        c.openings
            .iter()
            .filter(|o| !o.door)
            .map(|o| o.mark.clone())
            .collect(),
    );
    taken.insert("room", c.rooms.iter().map(|r| r.number.clone()).collect());
    let mut p = Planner {
        c,
        fixes: vec![],
        manual: vec![],
        marks: HashMap::new(),
        taken,
    };
    let mut schedules = false;
    for f in &report.findings {
        if f.source != "rules" {
            p.manual(f, "Claude's finding: review it yourself.");
            continue;
        }
        plan_one(&mut p, f, &mut schedules);
    }
    Plan {
        fixes: p.fixes,
        manual: p.manual,
    }
}

fn plan_one(p: &mut Planner, f: &Finding, schedules: &mut bool) {
    let doc = p.c.doc;
    let data = |id: ElementId| doc.data(id).ok();
    match f.rule.as_str() {
        "dup-door-mark" | "dup-window-mark" | "dup-room-number" => {
            let (kind, key) = match f.rule.as_str() {
                "dup-door-mark" => ("door", "mark"),
                "dup-window-mark" => ("window", "mark"),
                _ => ("room", "number"),
            };
            for id in f.elements.iter().skip(1) {
                let m = p.next_mark(kind);
                p.add(
                    f,
                    format!("Renumber a duplicate {kind} to {m}"),
                    format!("{key} → {m}"),
                    false,
                    Action::SetText {
                        id: *id,
                        key: key.into(),
                        value: m,
                    },
                );
            }
        }
        "dup-sheet" => {
            let numbers: HashSet<String> = doc
                .of(Cat::Sheet)
                .filter_map(|e| match &e.data {
                    ElementData::Sheet { number, .. } => Some(number.clone()),
                    _ => None,
                })
                .collect();
            for (i, id) in f.elements.iter().skip(1).enumerate() {
                let Some(ElementData::Sheet { number, .. }) = data(*id) else {
                    continue;
                };
                let mut n = format!("{number}.{}", i + 1);
                let mut k = i + 1;
                while numbers.contains(&n) {
                    k += 1;
                    n = format!("{number}.{k}");
                }
                p.add(
                    f,
                    format!("Renumber the duplicate sheet {number} to {n}"),
                    format!("sheet number → {n}"),
                    false,
                    Action::SetText {
                        id: *id,
                        key: "number".into(),
                        value: n,
                    },
                );
            }
        }
        "dup-grid" | "dup-level" => {
            for (i, id) in f.elements.iter().skip(1).enumerate() {
                let Some(d) = data(*id) else { continue };
                let base = match d {
                    ElementData::Grid { name, .. } | ElementData::Level { name, .. } => {
                        name.clone()
                    }
                    _ => continue,
                };
                let n = if f.rule == "dup-grid" {
                    format!("{base}.{}", i + 1)
                } else {
                    format!("{base} ({})", i + 2)
                };
                p.add(
                    f,
                    format!(
                        "Rename the duplicate {} to {n}",
                        if f.rule == "dup-grid" {
                            "grid"
                        } else {
                            "level"
                        }
                    ),
                    format!("name → {n}"),
                    false,
                    Action::SetText {
                        id: *id,
                        key: "name".into(),
                        value: n,
                    },
                );
            }
        }
        "untagged-doors" | "untagged-windows" | "untagged-rooms" => {
            let mut by_view: HashMap<ElementId, Vec<ElementId>> = HashMap::new();
            for id in &f.elements {
                if let Some(v) = p.c.view_for(*id) {
                    by_view.entry(v).or_default().push(*id);
                }
            }
            if by_view.is_empty() {
                p.manual(f, "No floor plan shows them to tag them in.");
            }
            for (view, ids) in by_view {
                let name = data(view).map(|d| d.name()).unwrap_or_default();
                p.add(
                    f,
                    format!(
                        "Tag {} in {name}",
                        f.title.split(' ').nth(1).unwrap_or("elements")
                    ),
                    format!("{} tags added", ids.len()),
                    false,
                    Action::Tag { view, ids },
                );
            }
        }
        "no-door-schedule"
        | "no-window-schedule"
        | "no-room-finish-schedule"
        | "no-sheet-index" => {
            if !*schedules {
                *schedules = true;
                p.add(
                    f,
                    "Add the missing schedules",
                    "Door, window, room and sheet schedules added where missing",
                    false,
                    Action::Schedules,
                );
            }
        }
        "views-unplaced" | "ref-unplaced" => {
            let name = if f.rule == "ref-unplaced" {
                "Sections and Elevations"
            } else {
                "Plans and Details"
            };
            let views: Vec<ElementId> = f
                .elements
                .iter()
                .copied()
                .filter(|v| !p.c.on_sheet.contains_key(v))
                .take(9)
                .collect();
            if !views.is_empty() {
                p.add(
                    f,
                    format!("Place {} views on a new sheet \"{name}\"", views.len()),
                    "A new sheet with the views laid out on it",
                    false,
                    Action::PlaceOnSheet {
                        views,
                        name: name.into(),
                    },
                );
            }
        }
        "stair-riser" | "stair-tread" | "stair-width" | "stair-handrail" => {
            let Some(id) = f.elements.first().copied() else {
                return;
            };
            let (max_r, min_t, min_w) = if p.c.residential {
                (7.75, 10.0, 36.0)
            } else {
                (7.0, 11.0, 44.0)
            };
            let (a, change, design) = match f.rule.as_str() {
                "stair-riser" => (
                    Action::Stair {
                        id,
                        max_riser: Some(max_r * MM_PER_IN),
                        tread: None,
                        width: None,
                        railings: None,
                    },
                    format!("maximum riser → {max_r}\" (more risers, a longer run)"),
                    true,
                ),
                "stair-tread" => (
                    Action::Stair {
                        id,
                        max_riser: None,
                        tread: Some(min_t * MM_PER_IN),
                        width: None,
                        railings: None,
                    },
                    format!("tread → {min_t}\" (a longer run)"),
                    true,
                ),
                "stair-width" => (
                    Action::Stair {
                        id,
                        max_riser: None,
                        tread: None,
                        width: Some(min_w * MM_PER_IN),
                        railings: None,
                    },
                    format!("width → {min_w}\""),
                    true,
                ),
                _ => (
                    Action::Stair {
                        id,
                        max_riser: None,
                        tread: None,
                        width: None,
                        railings: Some(true),
                    },
                    "railings on both sides".into(),
                    false,
                ),
            };
            p.add(
                f,
                format!("Fix the stair: {change}"),
                change.clone(),
                design,
                a,
            );
        }
        "rail-height" => {
            let Some(id) = f.elements.first().copied() else {
                return;
            };
            let Some(ElementData::Railing { type_id, .. }) = data(id) else {
                return;
            };
            let h = if p.c.residential { 36.0 } else { 42.0 } * MM_PER_IN;
            p.add(
                f,
                format!("Raise the railing type to {}", inch(h)),
                format!("railing height → {}", inch(h)),
                false,
                Action::RailingHeight {
                    type_id: *type_id,
                    height: h,
                },
            );
        }
        "ceiling-height" => {
            let Some(id) = f.elements.first().copied() else {
                return;
            };
            let h = if p.c.residential { 7.0 } else { 7.5 } * MM_PER_FT;
            p.add(
                f,
                format!(
                    "Raise the ceiling to {}'-{}\"",
                    (h / MM_PER_FT) as i64,
                    ((h / MM_PER_IN) as i64) % 12
                ),
                "ceiling height → the code minimum",
                true,
                Action::CeilingHeight { id, height: h },
            );
        }
        "egress-size" | "egress-none" => plan_egress(p, f),
        "egress-door-size" | "door-height" | "garage-door" | "door-clear" => plan_door(p, f),
        "wrb" | "below-grade" => {
            let Some(w) = f.elements.first().and_then(|id| p.c.wall(*id)) else {
                return;
            };
            let Some(ElementData::WallType { layers, .. }) = data(w.type_id) else {
                return;
            };
            let (name, index) = if f.rule == "wrb" {
                // Behind the cladding: after the exterior finish layers.
                (
                    "Weather Barrier",
                    layers
                        .iter()
                        .take_while(|l| l.function == LayerFunction::Finish)
                        .count()
                        .max(1)
                        .min(layers.len()),
                )
            } else {
                ("Dampproofing", 0)
            };
            p.add(
                f,
                format!("Add a {name} layer to {}", w.type_name),
                format!("{name} membrane added to the wall type (every wall of the type)"),
                false,
                Action::AddLayer {
                    type_id: w.type_id,
                    index,
                    name: name.into(),
                },
            );
        }
        "roof-underlayment" => {
            let Some(ElementData::Roof { type_id, .. }) =
                f.elements.first().and_then(|id| data(*id))
            else {
                return;
            };
            let n = match data(*type_id) {
                Some(ElementData::RoofType { layers, .. }) => layers
                    .iter()
                    .take_while(|l| l.function == LayerFunction::Finish)
                    .count()
                    .max(1)
                    .min(layers.len()),
                _ => 1,
            };
            p.add(
                f,
                "Add an underlayment layer to the roof type",
                "Underlayment membrane under the roof covering",
                false,
                Action::AddLayer {
                    type_id: *type_id,
                    index: n,
                    name: "Underlayment".into(),
                },
            );
        }
        "slab-vapor" => {
            let Some(ElementData::Floor { type_id, .. }) =
                f.elements.first().and_then(|id| data(*id))
            else {
                return;
            };
            let n = match data(*type_id) {
                Some(ElementData::FloorType { layers, .. }) => layers.len(),
                _ => 0,
            };
            p.add(
                f,
                "Add a vapor retarder under the slab",
                "Vapor retarder membrane at the bottom of the floor type",
                false,
                Action::AddLayer {
                    type_id: *type_id,
                    index: n,
                    name: "Vapor Retarder".into(),
                },
            );
        }
        "wet-floor" => {
            let Some(id) = f.elements.first().copied() else {
                return;
            };
            let tile = p.types(Cat::FloorType).into_iter().find(|(_, d)| {
                has_any(
                    &d.name().to_lowercase(),
                    &["tile", "porcelain", "ceramic", "vinyl", "lvt"],
                )
            });
            match tile {
                Some((t, d)) => p.add(
                    f,
                    format!("Change the floor to {}", d.name()),
                    format!("floor type → {}", d.name()),
                    true,
                    Action::SwapType {
                        ids: vec![id],
                        type_id: t,
                    },
                ),
                None => p.manual(f, "There's no tile or vinyl floor type to switch to."),
            }
        }
        "roof-shingle-slope" => {
            let Some(id) = f.elements.first().copied() else {
                return;
            };
            p.add(
                f,
                "Steepen the roof to 4:12",
                "roof slope → 4:12",
                true,
                Action::RoofSlope {
                    id,
                    slope: 4.0 / 12.0,
                },
            );
        }
        "opening-past-end" | "opening-head" | "opening-clash" | "opening-tight" => {
            plan_opening(p, f)
        }
        "wall-short" => p.add(
            f,
            "Delete the stray short wall",
            "wall deleted",
            true,
            Action::Delete {
                ids: f.elements.clone(),
            },
        ),
        "wall-overlap" => {
            let shorter = f.elements.iter().copied().min_by(|a, b| {
                let l = |id: &ElementId| p.c.wall(*id).map_or(0.0, |w| w.len);
                l(a).total_cmp(&l(b))
            });
            if let Some(id) = shorter {
                p.add(
                    f,
                    "Delete the shorter of the overlapping walls",
                    "one wall deleted",
                    true,
                    Action::Delete { ids: vec![id] },
                );
            }
        }
        "room-dup" => {
            if let Some(id) = f.elements.get(1).copied() {
                p.add(
                    f,
                    "Delete the duplicate room",
                    "the second room deleted",
                    true,
                    Action::Delete { ids: vec![id] },
                );
            }
        }
        "no-specs" => p.add(
            f,
            "Generate the project manual",
            "Project manual generated from the model and Project Info",
            false,
            Action::GenerateSpecs,
        ),
        "spec-ref" if f.severity == crate::Severity::Info => {
            // "Section A, B refers to X, which doesn't apply here".
            let (from, to) = f.title.split_once(" refers to ").unwrap_or((&f.title, ""));
            let from: Vec<String> = from
                .trim_start_matches("Section ")
                .split(", ")
                .filter(|n| studio_core::specs::valid_number(n))
                .map(str::to_string)
                .collect();
            match first_number(to) {
                Some(to) if !from.is_empty() => p.add(
                    f,
                    format!("Edit the reference to {to} out of {}", from.join(", ")),
                    "the reference edited out of the section text",
                    false,
                    Action::StripSpecRef { from, to },
                ),
                _ => p.manual(f, "Couldn't tell which sections."),
            }
        }
        "roof-drain-slope" => {
            for id in &f.elements {
                p.add(
                    f,
                    "Slope the membrane roof 1/4\":12 to drain",
                    "roof slope → 1/4\":12 (tapered to drain)",
                    true,
                    Action::RoofSlope {
                        id: *id,
                        slope: 0.25 / 12.0,
                    },
                );
            }
        }
        "wet-backer" => {
            let Some(w) = f.elements.first().and_then(|id| p.c.wall(*id)) else {
                return;
            };
            // Every wall of the type along a bath or shower.
            let walls: Vec<ElementId> =
                p.c.walls
                    .iter()
                    .filter(|x| {
                        x.type_id == w.type_id
                            && p.c.rooms_along(x).iter().any(|r| r.is(&["bath", "shower"]))
                    })
                    .map(|x| x.id)
                    .collect();
            let (type_id, name, n) = (w.type_id, w.type_name.clone(), walls.len());
            p.add(
                f,
                format!("Give the bath walls of {name} a tile backer"),
                format!(
                    "{n} wall{} → {name} - Wet Wall (glass-mat tile backer board)",
                    if n == 1 { "" } else { "s" },
                ),
                false,
                Action::WetWall { type_id, walls },
            );
        }
        "spec-missing" | "spec-ref" | "keynote-spec" | "note-spec" => {
            // spec-ref: the section referred to is the last number in the title.
            let n = if f.rule == "spec-ref" {
                f.title.rsplit(" refers to ").next().and_then(first_number)
            } else if f.rule == "spec-missing" {
                first_number(&f.title)
            } else {
                f.title.split("Section ").nth(1).and_then(first_number)
            };
            match n {
                Some(n) if studio_specs::library::entry(&n).is_some() => {
                    let exists =
                        studio_core::specs::book(doc).is_some_and(|b| b.section(&n).is_some());
                    if exists {
                        p.add(
                            f,
                            format!("Include Section {n}"),
                            format!("{n} included in the manual"),
                            false,
                            Action::IncludeSpecs { numbers: vec![n] },
                        );
                    } else {
                        p.add(
                            f,
                            format!("Add Section {n} to the manual"),
                            format!("{n} added from the library"),
                            false,
                            Action::AddSpecs { numbers: vec![n] },
                        );
                    }
                }
                Some(n) => p.manual(
                    f,
                    &format!("Section {n} isn't in the library: write it (Specifications > New)."),
                ),
                None => p.manual(f, "Couldn't tell which section."),
            }
        }
        "spec-excluded" => {
            if let Some(n) = first_number(&f.title) {
                p.add(
                    f,
                    format!("Include Section {n}"),
                    format!("{n} included"),
                    false,
                    Action::IncludeSpecs { numbers: vec![n] },
                );
            }
        }
        "tbd" => p.manual(f, "Only you can resolve a TBD."),
        "room-unnamed" => p.manual(f, "Name the rooms for their use."),
        "exits" => p.manual(f, "A second exit is a design decision."),
        "corridor" | "hallway" | "turning" => {
            p.manual(f, "Widening a space moves walls: a design decision.")
        }
        "room-no-door" | "room-open" => p.manual(f, "Decide where the door or wall goes."),
        _ => p.manual(f, "Needs your review."),
    }
}

/// Emergency escape: swap the room's best window to a type that qualifies, or add one.
fn plan_egress(p: &mut Planner, f: &Finding) {
    let doc = p.c.doc;
    let Some(room) = f
        .elements
        .iter()
        .find_map(|id| p.c.rooms.iter().find(|r| r.id == *id))
    else {
        return;
    };
    let grade = p.c.lowest_level == Some(room.level);
    let min_area = if grade { 5.0 } else { 5.7 } * MM_PER_FT * MM_PER_FT;
    let good = |d: &ElementData| {
        let ElementData::WindowType { sill, .. } = d else {
            return false;
        };
        clear_opening(d).is_some_and(|(w, h)| {
            w >= 20.0 * MM_PER_IN && h >= 24.0 * MM_PER_IN && w * h >= min_area
        }) && *sill <= 44.0 * MM_PER_IN
    };
    // The smallest qualifying type.
    let mut types: Vec<(ElementId, &ElementData)> = p
        .types(Cat::WindowType)
        .into_iter()
        .filter(|(_, d)| good(d))
        .collect();
    types.sort_by(|a, b| {
        let area = |d: &ElementData| match d {
            ElementData::WindowType { width, height, .. } => width * height,
            _ => 0.0,
        };
        area(a.1).total_cmp(&area(b.1))
    });
    let Some(&(t, td)) = types.first() else {
        p.manual(
            f,
            "There's no window type big enough for escape: load a larger casement.",
        );
        return;
    };
    let (tw, tsill) = match td {
        ElementData::WindowType { width, sill, .. } => (*width, *sill),
        _ => (0.0, 0.0),
    };
    if f.rule == "egress-size" {
        let windows: Vec<ElementId> = f
            .elements
            .iter()
            .copied()
            .filter(|id| matches!(doc.data(*id), Ok(ElementData::Window { .. })))
            .collect();
        let fits = windows.iter().copied().find(|w| {
            p.c.openings
                .iter()
                .find(|o| o.id == *w)
                .and_then(|o| p.c.wall(o.host).map(|wall| (o, wall)))
                .is_some_and(|(o, wall)| {
                    o.offset - tw / 2.0 > 0.0 && o.offset + tw / 2.0 < wall.len
                })
        });
        match fits {
            Some(w) => p.add(
                f,
                format!("Change a {} window to {}", room.name, td.name()),
                format!(
                    "window type → {} (sill {}), which meets emergency escape",
                    td.name(),
                    inch(tsill)
                ),
                true,
                Action::SwapType {
                    ids: vec![w],
                    type_id: t,
                },
            ),
            None => p.manual(
                f,
                "No window in the room has room in its wall for a larger egress window.",
            ),
        }
        return;
    }
    // No window at all: one in the room's longest exterior wall, centered on the room.
    let wall =
        p.c.rooms_along_ext(room.id)
            .into_iter()
            .max_by(|a, b| a.1.total_cmp(&b.1));
    match wall {
        Some((w, _, offset)) if offset - tw / 2.0 > 0.0 => {
            p.add(
                f,
                format!("Add an egress window ({}) to {}", td.name(), room.name),
                format!(
                    "a {} in the exterior wall, sill {}",
                    td.name(),
                    inch(tsill.min(44.0 * MM_PER_IN))
                ),
                true,
                Action::AddWindow {
                    wall: w,
                    type_id: t,
                    offset,
                    sill: tsill.min(44.0 * MM_PER_IN),
                },
            );
        }
        _ => p.manual(
            f,
            "The room has no exterior wall long enough for an egress window.",
        ),
    }
}

/// A door type: id, name, width and height.
type DoorT = (ElementId, String, f64, f64);

/// Doors: a type that meets the size or rating the finding asks for.
fn plan_door(p: &mut Planner, f: &Finding) {
    let doc = p.c.doc;
    let types: Vec<DoorT> = doc
        .of(Cat::DoorType)
        .filter_map(|e| match &e.data {
            ElementData::DoorType {
                name,
                width,
                height,
                ..
            } => Some((e.id, name.clone(), *width, *height)),
            _ => None,
        })
        .collect();
    for id in f.elements.iter().copied() {
        let Some(o) = p.c.openings.iter().find(|o| o.id == id && o.door) else {
            continue;
        };
        let pick = |ok: &dyn Fn(&DoorT) -> bool| {
            types
                .iter()
                .filter(|t| t.0 != id && ok(t))
                .min_by(|a, b| {
                    ((a.2 - o.width).abs() + (a.3 - o.height).abs())
                        .total_cmp(&((b.2 - o.width).abs() + (b.3 - o.height).abs()))
                })
                .cloned()
        };
        let choice = match f.rule.as_str() {
            "garage-door" => pick(&|t| {
                has_any(
                    &t.1.to_lowercase(),
                    &["solid", "20 min", "fire", "rated", "steel", "metal"],
                )
            }),
            "door-height" => {
                pick(&|t| t.3 >= 80.0 * MM_PER_IN - 1.0 && (t.2 - o.width).abs() < 1.0)
            }
            "door-clear" => pick(&|t| t.2 >= 36.0 * MM_PER_IN - 1.0 && t.3 >= o.height - 1.0),
            _ => pick(&|t| t.2 >= 36.0 * MM_PER_IN - 1.0 && t.3 >= 80.0 * MM_PER_IN - 1.0),
        };
        match choice {
            Some((t, name, ..)) => {
                let fits = p.c.wall(o.host).is_some_and(|w| {
                    let wd = types.iter().find(|x| x.0 == t).map_or(0.0, |x| x.2);
                    o.offset - wd / 2.0 > 0.0 && o.offset + wd / 2.0 < w.len
                });
                if fits {
                    p.add(
                        f,
                        format!(
                            "Change door {} to {name}",
                            if o.mark.is_empty() {
                                &o.type_name
                            } else {
                                &o.mark
                            }
                        ),
                        format!("door type → {name}"),
                        true,
                        Action::SwapType {
                            ids: vec![id],
                            type_id: t,
                        },
                    );
                    if f.rule == "egress-door-size" {
                        break;
                    }
                } else {
                    p.manual(f, "The larger door doesn't fit in its wall.");
                }
            }
            None if f.rule == "garage-door" => {
                let mark = if o.mark.is_empty() {
                    o.type_name.clone()
                } else {
                    o.mark.clone()
                };
                let name = o.type_name.clone();
                p.add(
                    f,
                    format!("Make door {mark} 20-minute rated, solid core"),
                    format!("door type → {name} - 20 Min Solid Core"),
                    false,
                    Action::RatedDoor { ids: vec![id] },
                )
            }
            None => p.manual(
                f,
                "There's no door type that meets it: load one (Openings > Door).",
            ),
        }
    }
}

/// Openings: into the wall, below its top, apart from each other.
fn plan_opening(p: &mut Planner, f: &Finding) {
    let Some(o) = f
        .elements
        .first()
        .and_then(|id| p.c.openings.iter().find(|o| o.id == *id))
    else {
        return;
    };
    let Some(w) = p.c.wall(o.host) else { return };
    let margin = 2.0 * MM_PER_IN;
    match f.rule.as_str() {
        "opening-past-end" => {
            if o.width + 2.0 * margin > w.len {
                p.manual(f, "The opening is wider than its wall.");
                return;
            }
            let off = o
                .offset
                .clamp(o.width / 2.0 + margin, w.len - o.width / 2.0 - margin);
            p.add(
                f,
                "Move the opening back into its wall",
                format!("moved {} along the wall", inch((off - o.offset).abs())),
                false,
                Action::Opening {
                    id: o.id,
                    offset: Some(off),
                    sill: None,
                },
            );
        }
        "opening-head" => {
            if o.door {
                p.manual(f, "Raise the wall or use a shorter door.");
                return;
            }
            let sill = w.height - 6.0 * MM_PER_IN - o.height;
            if sill < 0.0 {
                p.manual(f, "The window is taller than the wall allows.");
                return;
            }
            p.add(
                f,
                "Lower the window to leave room for the header",
                format!("sill → {}", inch(sill)),
                false,
                Action::Opening {
                    id: o.id,
                    offset: None,
                    sill: Some(sill),
                },
            );
        }
        _ => {
            let Some(q) = f
                .elements
                .get(1)
                .and_then(|id| p.c.openings.iter().find(|x| x.id == *id))
            else {
                return;
            };
            let gap = 4.0 * MM_PER_IN;
            let dir = if q.offset >= o.offset { 1.0 } else { -1.0 };
            let off = o.offset + dir * ((o.width + q.width) / 2.0 + gap);
            if off - q.width / 2.0 < margin || off + q.width / 2.0 > w.len - margin {
                p.manual(f, "There isn't room in the wall to space them apart.");
                return;
            }
            p.add(
                f,
                "Space the openings apart",
                format!(
                    "moved {} {} along the wall",
                    q.type_name,
                    inch((off - q.offset).abs())
                ),
                false,
                Action::Opening {
                    id: q.id,
                    offset: Some(off),
                    sill: None,
                },
            );
        }
    }
}

impl Ctx<'_> {
    /// A room's exterior walls: (wall, overlap length, offset of the overlap's middle).
    fn rooms_along_ext(&self, room: ElementId) -> Vec<(ElementId, f64, f64)> {
        let Some(r) = self.rooms.iter().find(|r| r.id == room) else {
            return vec![];
        };
        let Some(ring) = &r.ring else { return vec![] };
        self.walls
            .iter()
            .filter(|w| w.exterior && w.level == r.level && w.len > 0.0)
            .filter_map(|w| {
                let dir = w.end.sub(w.start).scale(1.0 / w.len);
                // Sample along the wall: the stretch next to the room.
                let n = 40;
                let inside: Vec<f64> = (0..=n)
                    .map(|i| w.len * i as f64 / n as f64)
                    .filter(|t| {
                        let p = w.start.add(dir.scale(*t));
                        r.edge_dist(p) < 12.0 * MM_PER_IN || point_in_ring(p, ring)
                    })
                    .collect();
                let (a, b) = (*inside.first()?, *inside.last()?);
                (b - a > 0.0).then_some((w.id, b - a, (a + b) / 2.0))
            })
            .collect()
    }
}

fn add_layer(layers: &mut Vec<WallLayer>, index: usize, name: &str) {
    let l = WallLayer {
        name: name.into(),
        thickness: 0.0,
        function: LayerFunction::Membrane,
        material: None,
    };
    let i = index.min(layers.len());
    layers.insert(i, l);
}

/// Applies fixes; each fix is its own step, merged into one undo step named `label`.
/// Returns how many applied, and the errors of those that couldn't.
pub fn apply(
    doc: &mut Document,
    model: &Model,
    actions: &[Action],
    label: &str,
) -> (usize, Vec<String>) {
    let mark = doc.undo_depth();
    let mut n = 0;
    let mut errors = vec![];
    for a in actions {
        match apply_one(doc, model, a) {
            Ok(()) => n += 1,
            Err(e) => errors.push(e.to_string()),
        }
    }
    if doc.undo_depth() > mark {
        doc.merge_undo(mark, label);
    }
    (n, errors)
}

fn apply_one(doc: &mut Document, model: &Model, a: &Action) -> CoreResult<()> {
    use studio_core::ops;
    let modify = |doc: &mut Document, id: ElementId, f: &dyn Fn(&mut ElementData)| {
        doc.transact("QA/QC Fix", |tx| tx.modify(id, |d| f(d)))
    };
    match a {
        Action::SetText { id, key, value } => ops::set_property(doc, *id, key, value, 0),
        Action::Tag { view, ids } => {
            studio_core::visibility::tag_elements(doc, *view, ids).map(|_| ())
        }
        Action::Stair {
            id,
            max_riser: r,
            tread: t,
            width: w,
            railings: rl,
        } => modify(doc, *id, &|d| {
            if let ElementData::Stair {
                max_riser,
                tread,
                width,
                railings,
                ..
            } = d
            {
                if let Some(v) = r {
                    *max_riser = *v;
                }
                if let Some(v) = t {
                    *tread = *v;
                }
                if let Some(v) = w {
                    *width = *v;
                }
                if let Some(v) = rl {
                    *railings = *v;
                }
            }
        }),
        Action::CeilingHeight { id, height: h } => modify(doc, *id, &|d| {
            if let ElementData::Ceiling { height, .. } = d {
                *height = *h;
            }
        }),
        Action::Opening {
            id,
            offset: o,
            sill: s,
        } => modify(doc, *id, &|d| match d {
            ElementData::Door { offset, .. } => {
                if let Some(v) = o {
                    *offset = *v;
                }
            }
            ElementData::Window { offset, sill, .. } => {
                if let Some(v) = o {
                    *offset = *v;
                }
                if let Some(v) = s {
                    *sill = *v;
                }
            }
            _ => {}
        }),
        Action::SwapType { ids, type_id } => doc.transact("QA/QC Fix", |tx| {
            for id in ids {
                tx.modify(*id, |d| match d {
                    ElementData::Door { type_id: t, .. }
                    | ElementData::Window { type_id: t, .. }
                    | ElementData::Floor { type_id: t, .. }
                    | ElementData::Wall { type_id: t, .. } => *t = *type_id,
                    _ => {}
                })?;
            }
            Ok(())
        }),
        Action::AddLayer {
            type_id,
            index,
            name,
        } => modify(doc, *type_id, &|d| match d {
            ElementData::WallType { layers, .. }
            | ElementData::FloorType { layers, .. }
            | ElementData::RoofType { layers, .. } => add_layer(layers, *index, name),
            _ => {}
        }),
        Action::RailingHeight { type_id, height: h } => modify(doc, *type_id, &|d| {
            if let ElementData::RailingType { height, .. } = d {
                *height = *h;
            }
        }),
        Action::RoofSlope { id, slope: s } => modify(doc, *id, &|d| {
            if let ElementData::Roof {
                slope,
                sloped,
                boundary,
                ..
            } = d
            {
                *slope = *s;
                // A dead-flat roof drains one way: from its longest edge, like tapered
                // insulation to a gutter or scuppers (ADR-094).
                if !sloped.iter().any(|x| *x) && !boundary.is_empty() {
                    let n = boundary.len();
                    let longest = (0..n)
                        .max_by(|a, b| {
                            let l = |i: usize| boundary[i].dist(boundary[(i + 1) % n]);
                            l(*a).total_cmp(&l(*b))
                        })
                        .unwrap_or(0);
                    sloped.resize(n, false);
                    sloped[longest] = true;
                }
            }
        }),
        Action::Delete { ids } => ops::delete(doc, ids).map(|_| ()),
        Action::AddWindow {
            wall,
            type_id,
            offset,
            sill,
        } => {
            let id = ops::create_window(doc, *type_id, *wall, *offset, false)?;
            modify(doc, id, &|d| {
                if let ElementData::Window { sill: s, .. } = d {
                    *s = *sill;
                }
            })
        }
        Action::Schedules => ops::ensure_schedules(doc),
        Action::PlaceOnSheet { views, name } => {
            let sheet = ops::create_sheet(doc, name, studio_core::element::SheetSize::ArchD)?;
            // A 3 x 3 grid on a 36" x 24" sheet, left of the title block.
            for (i, v) in views.iter().enumerate() {
                let (col, row) = ((i % 3) as f64, (i / 3) as f64);
                let at = Pt::new(130.0 + col * 230.0, 500.0 - row * 180.0);
                ops::place_view(doc, sheet, *v, at)?;
            }
            Ok(())
        }
        Action::GenerateSpecs => {
            let f = studio_specs::features::facts(doc, model);
            let date = String::new();
            studio_core::specs::save(
                doc,
                "Generate Project Manual",
                studio_specs::generate::generate(&f, "csi-classic", "Issued for Permit", &date),
            )
        }
        Action::AddSpecs { numbers } => {
            let f = studio_specs::features::facts(doc, model);
            let secs: Vec<_> = numbers
                .iter()
                .filter_map(|n| studio_specs::library::entry(n))
                .map(|l| studio_specs::generate::from_library(l, &f))
                .collect();
            if studio_core::specs::book(doc).is_none() {
                return Err(CoreError::Invalid(
                    "generate the project manual first".into(),
                ));
            }
            studio_core::specs::add_sections(doc, secs)
        }
        Action::IncludeSpecs { numbers } => studio_core::specs::set_included(doc, numbers, true),
        Action::RatedDoor { ids } => {
            for id in ids {
                let Ok(ElementData::Door { type_id, .. }) = doc.data(*id) else {
                    continue;
                };
                let Ok(ElementData::DoorType {
                    name,
                    family,
                    width,
                    height,
                    leaf,
                    panels,
                    finish,
                }) = doc.data(*type_id).cloned()
                else {
                    continue;
                };
                let rated = format!("{name} - 20 Min Solid Core");
                let copy = ElementData::DoorType {
                    name: rated.clone(),
                    family,
                    width,
                    height,
                    leaf,
                    panels,
                    finish,
                };
                let t = type_named(doc, Cat::DoorType, &rated, copy)?;
                doc.transact("QA/QC Fix", |tx| {
                    tx.modify(*id, |d| {
                        if let ElementData::Door { type_id, .. } = d {
                            *type_id = t;
                        }
                    })
                })?;
            }
            Ok(())
        }
        Action::WetWall { type_id, walls } => {
            let Ok(ElementData::WallType {
                name,
                thickness,
                function,
                layers,
            }) = doc.data(*type_id).cloned()
            else {
                return Err(CoreError::Invalid("not a wall type".into()));
            };
            let wet = format!("{name} - Wet Wall");
            let layers = layers
                .into_iter()
                .map(|mut l| {
                    if l.function == LayerFunction::Finish
                        && has_any(&l.name.to_lowercase(), &["gypsum", "gwb", "drywall"])
                    {
                        l.name = "Glass-Mat Tile Backer Board".into();
                        l.material = None;
                    }
                    l
                })
                .collect();
            let data = ElementData::WallType {
                name: wet.clone(),
                thickness,
                function,
                layers,
            };
            let t = type_named(doc, Cat::WallType, &wet, data)?;
            apply_one(
                doc,
                model,
                &Action::SwapType {
                    ids: walls.clone(),
                    type_id: t,
                },
            )
        }
        Action::AddSection { section } => {
            let Some(book) = studio_core::specs::book(doc) else {
                return Err(CoreError::Invalid(
                    "generate the project manual first".into(),
                ));
            };
            if book.section(&section.number).is_some() {
                studio_core::specs::set_section(doc, &section.number, section.clone())
            } else {
                studio_core::specs::add_sections(doc, vec![section.clone()])
            }
        }
        Action::StripSpecRef { from, to } => {
            let Some(book) = studio_core::specs::book(doc) else {
                return Err(CoreError::Invalid("there's no project manual".into()));
            };
            let mut n = 0;
            for number in from {
                if let Some(s) = book.section(number) {
                    let mut s = s.clone();
                    if studio_specs::coord::strip_reference(&mut s, to) > 0 {
                        studio_core::specs::set_section(doc, number, s)?;
                        n += 1;
                    }
                }
            }
            if n == 0 {
                return Err(CoreError::Invalid(format!(
                    "no reference to {to} to edit out"
                )));
            }
            Ok(())
        }
    }
}

/// The type of `cat` named `name`, made from `data` when there's none yet.
fn type_named(
    doc: &mut Document,
    cat: Cat,
    name: &str,
    data: ElementData,
) -> CoreResult<ElementId> {
    if let Some(e) = doc.of(cat).find(|e| e.data.name() == name) {
        return Ok(e.id);
    }
    doc.transact("QA/QC Fix", |tx| Ok(tx.insert(data)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_found_in_titles() {
        assert_eq!(
            first_number("The drawings show work with no spec section: 07 25 00 WEATHER BARRIERS")
                .as_deref(),
            Some("07 25 00")
        );
        assert_eq!(
            first_number("Keynote 23 74 16.11.A1 points").as_deref(),
            Some("23 74 16.11")
        );
        assert_eq!(first_number("no number here 12 34"), None);
    }
}
