//! Drawing-set standards (ADR-047): an office's standards for sheets, symbols, tags, text,
//! linework, material graphics, phasing, dimensioning, numbering, schedules, keynotes,
//! scales and views, BIM and issuance, each defined or open, so the Standards tab is also
//! a checklist of how complete the set is. Kept in the project (one Standards element,
//! made on the first edit); a library preset loads its values into it.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};

/// One standard: its office value, preset choices (if any) and whether it's defined.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "StandardItem")]
pub struct Item {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "StandardCategory")]
pub struct StandardCategory {
    pub id: String,
    pub label: String,
    /// The ribbon button's label.
    pub short: String,
    /// SHEETS, ANNOTATION, GRAPHICS, DATA, VIEWS or OUTPUT.
    pub group: String,
    pub applies: Vec<String>,
    pub items: Vec<Item>,
}

/// The project's standards: the library they started from and every category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Standards {
    pub library: String,
    pub categories: Vec<StandardCategory>,
}

/// The office libraries (presets) the Library menu offers.
pub const LIBRARIES: [&str; 3] = ["Rufplan Default (NCS 6)", "Residential", "Preservation"];

fn item(name: &str, value: &str, options: &[&str]) -> Item {
    Item {
        name: name.into(),
        value: value.into(),
        options: options.iter().map(|s| (*s).into()).collect(),
        done: !value.is_empty(),
    }
}

fn cat(
    id: &str,
    label: &str,
    short: &str,
    group: &str,
    applies: &[&str],
    items: Vec<Item>,
) -> StandardCategory {
    StandardCategory {
        id: id.into(),
        label: label.into(),
        short: short.into(),
        group: group.into(),
        applies: applies.iter().map(|s| (*s).into()).collect(),
        items,
    }
}

/// The Rufplan default (NCS 6) standards: 14 categories, 90 standards.
fn default_categories() -> Vec<StandardCategory> {
    vec![
        cat("sheet", "Sheet Setup", "SHEETS", "SHEETS", &["Title block", "All sheets"], vec![
            item("Sheet Size", "ARCH D — 24\" × 36\"", &["ARCH D — 24\" × 36\"", "ARCH E1 — 30\" × 42\"", "ARCH E — 36\" × 48\""]),
            item("Title Block", "Rufplan vertical strip, right edge", &[]),
            item("Sheet Numbering Format", "A-101 (NCS)", &["A-101 (NCS)", "A1.01 (dot)", "A101"]),
            item("Sheet Type Series", "0 General · 1 Plans · 2 Elev · 3 Sect · 4 Enlarged · 5 Details · 6 Schedules · 9 3D", &[]),
            item("Discipline Order", "G, V, C, L, S, A, I, F, P, M, E, T", &[]),
            item("Sheet Naming", "LEVEL 2 FLOOR PLAN – AREA B", &[]),
            item("Cover Sheet Contents", "Project directory, code analysis, vicinity map, general notes", &[]),
            item("Drawing Index", "Auto-generated from Sheet Index schedule", &[]),
            item("Sheet Layout Grid", "NCS module grid — columns 1–n, rows A–F", &[]),
            item("Key Plan & North Arrow", "", &[]),
        ]),
        cat("sym", "Symbols", "SYMBOLS", "ANNOTATION", &["Plans", "Elevations", "Sections"], vec![
            item("Section Markers", "Split circle head, 1/2\" dia, tail to cut extent", &[]),
            item("Exterior Elevation Markers", "Circle with solid pointer", &[]),
            item("Interior Elevation Markers", "4-way square, numbered clockwise", &["Single", "2-way", "3-way", "4-way square, numbered clockwise"]),
            item("Detail Callouts", "Circle bubble, dashed boundary", &[]),
            item("Enlarged Callouts", "Rectangle boundary, corner tag", &[]),
            item("View Titles", "Bubble + title + underline + scale", &[]),
            item("Grid Bubbles", "Numbers E–W, letters N–S, skip I & O", &[]),
            item("Level Markers", "Target head, relative 100'-0\"", &["Target head, relative 100'-0\"", "Target head, actual elevation"]),
            item("Spot Elevations", "", &[]),
            item("North Arrow", "Project north, true north offset shown", &[]),
            item("Matchlines", "Heavy dash-dot, \"MATCHLINE – SEE A-102\"", &[]),
            item("Break Lines", "Single zigzag, thin", &[]),
            item("Revision Clouds & Deltas", "Triangle delta with issue number", &[]),
            item("Graphic Scale", "", &[]),
        ]),
        cat("tags", "Tags", "TAGS", "ANNOTATION", &["Plans", "RCPs", "Schedules"], vec![
            item("Room", "Name / Number / Area, centered", &[]),
            item("Door", "Circle, room number + suffix", &[]),
            item("Window", "Hexagon, alpha type mark", &[]),
            item("Wall Type", "Diamond, alphanumeric", &[]),
            item("Ceiling", "Rectangle — material / height AFF", &[]),
            item("Finish", "", &[]),
            item("Equipment", "Rounded rectangle, EQ-##", &[]),
            item("Casework", "", &[]),
            item("Keynote", "Square, CSI code", &[]),
        ]),
        cat("text", "Text", "TEXT", "ANNOTATION", &["All views", "Sheets"], vec![
            item("Font", "Barlow Semi Condensed", &[]),
            item("Heights", "Notes 3/32\" · Room names 1/8\" · View titles 3/16\" · Sheet title 1/4\"", &[]),
            item("Case", "ALL CAPS", &["ALL CAPS", "Mixed case"]),
            item("Title Hierarchy", "Sheet title › View title › Room name › Note", &[]),
            item("Note Formats", "General notes lettered, sheet notes numbered, keynotes by CSI", &[]),
            item("Abbreviations", "Office list on G-001; no undefined abbreviations", &[]),
        ]),
        cat("line", "Linework", "LINEWORK", "GRAPHICS", &["Object styles", "View templates"], vec![
            item("Pen Weights", "Pens 1–16, mapped by scale", &[]),
            item("Line Types", "Hidden, overhead, centerline, property, setback, demo, NIC", &[]),
            item("Line Hierarchy", "Cut 5 · Profile 4 · Projection 2 · Beyond 1 · Overhead dashed 1", &[]),
            item("Halftone & Underlays", "50% halftone for links and underlays", &[]),
        ]),
        cat("mat", "Material Graphics", "MATERIALS", "GRAPHICS", &["Walls", "Floors", "Details"], vec![
            item("Cut Hatches & Poché", "Concrete, CMU, brick, stud, insulation, earth, steel, glass", &[]),
            item("Surface / Elevation Patterns", "Brick coursing, siding, panels, roofing", &[]),
            item("Scale-Dependent Fill Rules", "Solid poché ≤ 1/8\", full pattern ≥ 1/4\"", &[]),
            item("Rated Wall Graphics", "", &[]),
        ]),
        cat("phase", "Phasing", "PHASING", "GRAPHICS", &["Demo plans", "Plans", "Elevations"], vec![
            item("Existing", "Halftone gray, thin", &[]),
            item("Demo", "Dashed, keynoted", &[]),
            item("New", "Solid black, full weight", &[]),
            item("Future", "Dash-dot, light", &[]),
            item("NIC", "Hidden line + \"NIC\" note", &[]),
            item("Historic Fabric", "", &[]),
            item("Treatment Graphics", "Repair · Replace in kind · Salvage · Reinstall", &[]),
        ]),
        cat("dim", "Dimensioning", "DIMENSIONS", "ANNOTATION", &["Plans", "Enlarged plans", "Details"], vec![
            item("Units", "Feet & fractional inches", &[]),
            item("Precision", "1/8\"", &["1/16\"", "1/8\"", "1/4\"", "1/2\""]),
            item("Reference Points", "New: face of stud · Existing: face of finish · Masonry: face", &[]),
            item("String Order", "Overall → grids → openings", &[]),
            item("Tick Style", "Diagonal tick", &["Diagonal tick", "Arrow", "Dot"]),
        ]),
        cat("num", "Numbering", "NUMBERING", "DATA", &["Tags", "Schedules"], vec![
            item("Rooms", "Level + sequence, clockwise from entry (101)", &[]),
            item("Doors", "Room number + suffix (101A)", &[]),
            item("Windows", "Alpha type (A, B, C)", &[]),
            item("Wall & Assembly Types", "Walls 4A · Floors F-1 · Roofs R-1 · Ceilings C-1", &[]),
            item("Grids", "Numbers E–W, letters N–S", &[]),
            item("Levels", "LEVEL 1, LEVEL 2, ROOF", &[]),
            item("Equipment", "", &[]),
        ]),
        cat("sched", "Schedules & Legends", "SCHEDULES", "DATA", &["Schedules", "Legends", "G-series"], vec![
            item("Door Schedule", "Mark, size, type, material, frame, hardware set, rating", &[]),
            item("Window Schedule", "Type, size, operation, glazing, head/sill", &[]),
            item("Hardware", "", &[]),
            item("Finish Schedule", "Room, floor, base, walls, ceiling, notes", &[]),
            item("Equipment Schedule", "", &[]),
            item("Wall Type Legend", "Enlarged assembly sections at 1-1/2\"", &[]),
            item("Symbol Legend", "On G-002", &[]),
            item("Material Legend", "On G-002", &[]),
        ]),
        cat("key", "Keynotes & Specs", "KEYNOTES", "DATA", &["Keynote tags", "Specs"], vec![
            item("Keynote System", "CSI MasterFormat (04 20 00.A1)", &["CSI MasterFormat (04 20 00.A1)", "Sheet-specific numbers"]),
            item("Spec Section References", "Section number only, no titles", &[]),
        ]),
        cat("scale", "Scales & Views", "SCALES", "VIEWS", &["View templates", "All views"], vec![
            item("Standard Scales", "Site 1\"=20' · Plans 1/8\" · Enlarged 1/4\" · Int elev 1/4\" · Wall sect 3/4\" · Details 1-1/2\", 3\"", &[]),
            item("Plan Cut Plane", "4'-0\" AFF, overhead dashed", &[]),
            item("RCP Conventions", "Reflected, fixtures + soffits + heights AFF", &[]),
            item("Plan Orientation", "Project north up", &["True north up", "Project north up"]),
        ]),
        cat("bim", "BIM", "BIM", "VIEWS", &["Project template", "Project browser"], vec![
            item("Project Template", "Rufplan Residential 2026", &[]),
            item("View Templates", "One per view type and scale", &[]),
            item("View Naming", "LEVEL – VIEW TYPE – AREA", &[]),
            item("Browser Organization", "Discipline › View type › Phase", &[]),
            item("Worksets", "", &[]),
            item("Phases", "Existing · Demo · New Construction", &[]),
            item("Coordinates", "Shared coordinates from survey point", &[]),
        ]),
        cat("issue", "Issuance", "ISSUANCE", "OUTPUT", &["Title block", "Revisions", "PDF export"], vec![
            item("Issue Names", "SD, DD, CD 50/90/100%, Permit, Bid, Addendum, ASI, CCD, Record", &[]),
            item("Revision Rules", "Delta per issue; clouds cleared on next issue", &[]),
            item("Date Format", "YYYY-MM-DD", &["YYYY-MM-DD", "MM/DD/YYYY", "DD MMM YYYY"]),
        ]),
    ]
}

/// A library preset's standards: the default, with a few values tailored.
pub fn library(name: &str) -> CoreResult<Standards> {
    let mut cats = default_categories();
    let mut set = |cat: &str, item: &str, value: &str| {
        if let Some(i) = cats
            .iter_mut()
            .find(|c| c.id == cat)
            .and_then(|c| c.items.iter_mut().find(|i| i.name == item))
        {
            i.value = value.into();
            i.done = !value.is_empty();
        }
    };
    match name {
        "Rufplan Default (NCS 6)" => {}
        // Houses: small sets at larger scales, rooms and finishes that matter to owners.
        "Residential" => {
            set("sheet", "Sheet Size", "ARCH C — 18\" × 24\"");
            set("sheet", "Discipline Order", "G, C, L, S, A, I, M, P, E");
            set(
                "sheet",
                "Key Plan & North Arrow",
                "North arrow on every plan; no key plan",
            );
            set("scale", "Standard Scales", "Site 1\"=10' · Plans 1/4\" · Int elev 1/2\" · Wall sect 3/4\" · Details 1-1/2\", 3\"");
            set("tags", "Finish", "Room finish code in room tag");
            set("tags", "Casework", "Hexagon, cabinet type");
            set(
                "sched",
                "Hardware",
                "Hardware set per door, in door schedule",
            );
            set("bim", "Project Template", "Rufplan Residential 2026");
            set("bim", "Worksets", "Not used (single user)");
        }
        // Historic work: existing and historic fabric first, treatments keyed.
        "Preservation" => {
            set(
                "phase",
                "Existing",
                "Solid, full weight — historic fabric is the drawing",
            );
            set(
                "phase",
                "Historic Fabric",
                "Period-coded hatch, keyed to historic structure report",
            );
            set(
                "phase",
                "Demo",
                "Dashed; historic fabric removed only as keynoted",
            );
            set("phase", "Treatment Graphics", "Preserve · Repair · Replace in kind · Salvage · Reinstall · Reconstruct (per SOI Standards)");
            set(
                "dim",
                "Reference Points",
                "Existing: as-found face of finish · New: face of stud",
            );
            set("key", "Keynote System", "Sheet-specific numbers");
            set("tags", "Finish", "Finish code + period (e.g. P-2 1910)");
            set(
                "bim",
                "Phases",
                "Historic · Existing · Demo · Rehabilitation",
            );
            set(
                "issue",
                "Issue Names",
                "HSR, SD, DD, SHPO Review, CD 100%, Permit, Bid, Record",
            );
        }
        other => return Err(CoreError::Invalid(format!("no standards library {other}"))),
    }
    Ok(Standards {
        library: name.into(),
        categories: cats,
    })
}

fn element(doc: &Document) -> Option<(ElementId, Standards)> {
    doc.of(Category::Standards).find_map(|e| match &e.data {
        ElementData::Standards(s) => Some((e.id, s.clone())),
        _ => None,
    })
}

/// The project's standards: saved ones, else the default library.
pub fn standards(doc: &Document) -> Standards {
    element(doc).map_or_else(
        || library(LIBRARIES[0]).unwrap_or_else(|_| empty()),
        |e| e.1,
    )
}

fn empty() -> Standards {
    Standards {
        library: String::new(),
        categories: vec![],
    }
}

fn save(doc: &mut Document, label: &str, s: Standards) -> CoreResult<()> {
    let existing = element(doc).map(|e| e.0);
    doc.transact(label, |tx| {
        match existing {
            Some(id) => tx.set(id, ElementData::Standards(s))?,
            None => {
                tx.insert(ElementData::Standards(s));
            }
        }
        Ok(())
    })
}

/// Edits one standard: its value (a non-empty value marks it defined; clearing it leaves
/// its status) and/or its status.
pub fn set_standard(
    doc: &mut Document,
    category: &str,
    index: usize,
    value: Option<&str>,
    done: Option<bool>,
) -> CoreResult<()> {
    let mut s = standards(doc);
    let item = s
        .categories
        .iter_mut()
        .find(|c| c.id == category)
        .and_then(|c| c.items.get_mut(index))
        .ok_or_else(|| CoreError::Invalid("no such standard".into()))?;
    if let Some(v) = value {
        item.value = v.to_owned();
        if !v.trim().is_empty() {
            item.done = true;
        }
    }
    if let Some(d) = done {
        item.done = d;
    }
    let label = if value.is_some() {
        "Edit standard"
    } else {
        "Mark standard"
    };
    save(doc, label, s)
}

/// Loads a library's values into the project's standards (the checklist stays the same).
pub fn load_library(doc: &mut Document, name: &str) -> CoreResult<()> {
    let s = library(name)?;
    save(doc, "Load standards library", s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_has_fourteen_categories_and_ninety_standards() {
        let s = library(LIBRARIES[0]).unwrap();
        assert_eq!(s.categories.len(), 14);
        let all: Vec<&Item> = s.categories.iter().flat_map(|c| &c.items).collect();
        assert_eq!(all.len(), 90);
        // Empty values start open.
        assert!(all.iter().all(|i| i.done == !i.value.is_empty()));
        assert_eq!(all.iter().filter(|i| !i.done).count(), 11);
    }

    #[test]
    fn editing_saves_in_the_project_and_undoes() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        assert_eq!(
            doc.count(Category::Standards),
            0,
            "nothing saved until edited"
        );
        // Key Plan & North Arrow (sheet, 9) is open; a value defines it.
        set_standard(&mut doc, "sheet", 9, Some("Key plan top right"), None).unwrap();
        let s = standards(&doc);
        let it = &s.categories[0].items[9];
        assert_eq!((it.value.as_str(), it.done), ("Key plan top right", true));
        // Clearing it keeps it defined; the status sets it open.
        set_standard(&mut doc, "sheet", 9, Some(""), None).unwrap();
        assert!(standards(&doc).categories[0].items[9].done);
        set_standard(&mut doc, "sheet", 9, None, Some(false)).unwrap();
        assert!(!standards(&doc).categories[0].items[9].done);
        doc.undo().unwrap();
        assert!(standards(&doc).categories[0].items[9].done);
        assert!(set_standard(&mut doc, "sheet", 99, None, Some(true)).is_err());
    }

    #[test]
    fn a_library_loads_its_values() {
        let mut doc = Document::new();
        load_library(&mut doc, "Preservation").unwrap();
        let s = standards(&doc);
        assert_eq!(s.library, "Preservation");
        let phase = s.categories.iter().find(|c| c.id == "phase").unwrap();
        let historic = phase
            .items
            .iter()
            .find(|i| i.name == "Historic Fabric")
            .unwrap();
        assert!(historic.done && historic.value.contains("Period"));
        assert_eq!(
            s.categories.iter().flat_map(|c| &c.items).count(),
            90,
            "same checklist"
        );
        assert!(load_library(&mut doc, "Nope").is_err());
    }
}
