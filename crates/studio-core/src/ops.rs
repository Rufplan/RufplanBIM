//! Model operations used by the app. Each is one undoable transaction.

use serde::Serialize;
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{
    Anchor, Category, Compass, CropBox, DoorFamily, ElementData, ElementId, LocationLine,
    RufplanLink, ScheduleKind, SheetSize, SlabBound, StageChange, ViewKind, WallFunction, WallTop,
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
    (8, "1 1/2\" = 1'-0\""),
    (4, "3\" = 1'-0\""),
    // Engineering scales for site plans (ADR-023).
    (120, "1\" = 10'-0\""),
    (240, "1\" = 20'-0\""),
    (360, "1\" = 30'-0\""),
    (480, "1\" = 40'-0\""),
    (600, "1\" = 50'-0\""),
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
                layers: crate::compound::default_layers(name),
            });
        }
        for (name, t) in [
            ("Concrete Slab - 6\"", 6.0),
            ("Wood Joist Floor - 12\"", 12.0),
        ] {
            tx.insert(ElementData::FloorType {
                name: name.into(),
                thickness: t * MM_PER_IN,
                layers: crate::compound::default_type_layers(name),
            });
        }
        for (name, t) in [("ACT 2x4 Ceiling", 1.0), ("GWB Ceiling - 5/8\"", 0.625)] {
            tx.insert(ElementData::CeilingType {
                name: name.into(),
                thickness: t * MM_PER_IN,
                layers: vec![],
            });
        }
        seed_opening_types(tx);
        crate::structure::seed_structure_types(tx);
        crate::material::seed_materials(tx);
        crate::detail::seed_mark_types(tx);
        for (facing, name) in [
            (Compass::North, "North"),
            (Compass::South, "South"),
            (Compass::East, "East"),
            (Compass::West, "West"),
        ] {
            tx.insert(ElementData::view(name, ViewKind::Elevation { facing }, 96));
        }
        tx.insert(ElementData::view("{3D}", ViewKind::ThreeD, 96));
        seed_schedules(tx);
        crate::build::seed_roof_types(tx);
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
            rufplan: None,
            param_defs: vec![],
        });
        Ok(())
    })
}

/// Built-in door and window types (inches: width × height, sill).
fn seed_opening_types(tx: &mut Tx<'_>) {
    for (name, family, w, h) in [
        (
            "Single Flush 36\" x 84\"",
            DoorFamily::SingleFlush,
            36.0,
            84.0,
        ),
        (
            "Single Flush 30\" x 80\"",
            DoorFamily::SingleFlush,
            30.0,
            80.0,
        ),
        (
            "Double Flush 72\" x 84\"",
            DoorFamily::DoubleFlush,
            72.0,
            84.0,
        ),
    ] {
        tx.insert(ElementData::DoorType {
            name: name.into(),
            family,
            width: w * MM_PER_IN,
            height: h * MM_PER_IN,
        });
    }
    // The common size of every window family (ADR-031); more load from the library.
    for spec in crate::windows::starter() {
        tx.insert(spec.data());
    }
}

/// Adds the built-in door and window types to a project that has none (files saved
/// before doors and windows existed).
pub fn ensure_opening_types(doc: &mut Document) -> CoreResult<()> {
    if doc.of(Category::DoorType).next().is_some() || doc.of(Category::WindowType).next().is_some()
    {
        return Ok(());
    }
    doc.transact("Add door and window types", |tx| {
        seed_opening_types(tx);
        Ok(())
    })
}

const SCHEDULES: &[(ScheduleKind, &str)] = &[
    (ScheduleKind::Doors, "Door Schedule"),
    (ScheduleKind::Windows, "Window Schedule"),
    (ScheduleKind::Rooms, "Room Schedule"),
    (ScheduleKind::Sheets, "Sheet Index"),
    (ScheduleKind::Columns, "Structural Column Schedule"),
    (ScheduleKind::Beams, "Structural Framing Schedule"),
    (ScheduleKind::MaterialTakeoff, "Material Takeoff"),
];

fn seed_schedules(tx: &mut Tx<'_>) {
    for (kind, name) in SCHEDULES {
        tx.insert(ElementData::view(
            *name,
            ViewKind::Schedule { kind: *kind },
            1,
        ));
    }
}

/// Adds any standard schedule the project doesn't have yet (files from before M4, and
/// the structure and material schedules of ADR-020).
pub fn ensure_schedules(doc: &mut Document) -> CoreResult<()> {
    let has: Vec<ScheduleKind> = doc
        .of(Category::View)
        .filter_map(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::Schedule { kind },
                ..
            } => Some(*kind),
            _ => None,
        })
        .collect();
    let missing: Vec<(ScheduleKind, &str)> = SCHEDULES
        .iter()
        .filter(|(k, _)| !has.contains(k))
        .copied()
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    doc.transact("Add schedules", |tx| {
        for (kind, name) in missing {
            tx.insert(ElementData::view(name, ViewKind::Schedule { kind }, 1));
        }
        Ok(())
    })
}

/// Default section depth (far clip beyond the cut), mm (30'-0").
pub const DEFAULT_SECTION_DEPTH: f64 = 30.0 * MM_PER_FT;

/// Creates a section view along `start` → `end`, named "Section N".
pub fn create_section(doc: &mut Document, start: Pt, end: Pt) -> CoreResult<ElementId> {
    if start.dist(end) < 300.0 {
        return Err(CoreError::Invalid("draw a longer section line".into()));
    }
    let n = doc
        .iter()
        .filter(|e| {
            matches!(
                &e.data,
                ElementData::View {
                    kind: ViewKind::Section { .. },
                    ..
                }
            )
        })
        .count()
        + 1;
    doc.transact("Create section", |tx| {
        Ok(tx.insert(ElementData::view(
            format!("Section {n}"),
            ViewKind::Section {
                start,
                end,
                depth: DEFAULT_SECTION_DEPTH,
            },
            48,
        )))
    })
}

/// Adds an aligned dimension to `view`.
pub fn create_dimension(
    doc: &mut Document,
    view: ElementId,
    a: Pt,
    b: Pt,
    offset: f64,
) -> CoreResult<ElementId> {
    if a.dist(b) < 1.0 {
        return Err(CoreError::Invalid("pick two different points".into()));
    }
    let (a_ref, b_ref) = (anchor_at(doc, view, a), anchor_at(doc, view, b));
    doc.transact("Place dimension", |tx| {
        Ok(tx.insert(ElementData::Dimension {
            view,
            a,
            b,
            offset,
            a_ref,
            b_ref,
        }))
    })
}

/// In plan views, the wall or grid a point sits on (within 1 mm of a wall's footprint,
/// faces and centerline included, or of a grid line), as an anchor that follows it.
pub fn anchor_at(doc: &Document, view: ElementId, p: Pt) -> Option<Anchor> {
    let is_plan = matches!(
        doc.data(view),
        Ok(ElementData::View {
            kind: ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. },
            ..
        })
    );
    if !is_plan {
        return None;
    }
    let slop = 1.0;
    let mut best: Option<(f64, Anchor)> = None;
    for e in doc.iter() {
        match &e.data {
            ElementData::Wall {
                type_id,
                start,
                end,
                ..
            } => {
                let h = match doc.data(*type_id) {
                    Ok(ElementData::WallType { thickness, .. }) => thickness / 2.0,
                    _ => continue,
                };
                let len = start.dist(*end);
                let dir = end.sub(*start).norm();
                let along = p.sub(*start).dot(dir);
                let side = p.sub(*start).dot(dir.perp());
                if side.abs() > h + slop || along < -h - slop || along > len + h + slop {
                    continue;
                }
                // Prefer the wall whose face or centerline the point is closest to.
                let dev = [-h, 0.0, h]
                    .iter()
                    .map(|f| (side - f).abs())
                    .fold(f64::INFINITY, f64::min);
                if best.as_ref().is_none_or(|b| dev < b.0) {
                    best = Some((
                        dev,
                        Anchor::Wall {
                            wall: e.id,
                            t: along / len,
                            side,
                        },
                    ));
                }
            }
            ElementData::Grid { start, end, .. } => {
                let (t, d) = studio_geom::project_to_segment(p, *start, *end);
                if d < slop && best.as_ref().is_none_or(|b| d < b.0) {
                    best = Some((d, Anchor::Grid { grid: e.id, t }));
                }
            }
            _ => {}
        }
    }
    best.map(|b| b.1)
}

/// Where an anchor is now, or None if its element is gone.
pub fn anchor_point(doc: &Document, anchor: &Anchor) -> Option<Pt> {
    match anchor {
        Anchor::Wall { wall, t, side } => match doc.data(*wall).ok()? {
            ElementData::Wall { start, end, .. } => {
                let dir = end.sub(*start).norm();
                Some(
                    start
                        .add(end.sub(*start).scale(*t))
                        .add(dir.perp().scale(*side)),
                )
            }
            _ => None,
        },
        Anchor::Grid { grid, t } => match doc.data(*grid).ok()? {
            ElementData::Grid { start, end, .. } => Some(start.lerp(*end, *t)),
            _ => None,
        },
    }
}

/// A dimension's current end points: anchored ends follow their elements.
pub fn dimension_ends(doc: &Document, data: &ElementData) -> Option<(Pt, Pt)> {
    let ElementData::Dimension {
        a, b, a_ref, b_ref, ..
    } = data
    else {
        return None;
    };
    let a = a_ref
        .as_ref()
        .and_then(|r| anchor_point(doc, r))
        .unwrap_or(*a);
    let b = b_ref
        .as_ref()
        .and_then(|r| anchor_point(doc, r))
        .unwrap_or(*b);
    Some((a, b))
}

/// Adds a text note to `view` ("TEXT" when `text` is blank).
pub fn create_text(
    doc: &mut Document,
    view: ElementId,
    at: Pt,
    text: &str,
) -> CoreResult<ElementId> {
    let text = if text.trim().is_empty() {
        "TEXT".to_owned()
    } else {
        text.trim().to_owned()
    };
    doc.transact("Place text", |tx| {
        Ok(tx.insert(ElementData::TextNote {
            view,
            at,
            text,
            size: 3.0,
        }))
    })
}

/// Sheets sorted by number, as (id, number, name).
pub fn sheets(doc: &Document) -> Vec<(ElementId, String, String)> {
    let mut v: Vec<_> = doc
        .of(Category::Sheet)
        .filter_map(|e| match &e.data {
            ElementData::Sheet { number, name, .. } => Some((e.id, number.clone(), name.clone())),
            _ => None,
        })
        .collect();
    v.sort_by(|a, b| natural_cmp(&a.1, &b.1));
    v
}

/// Next sheet number after the last one: A1.0 → A2.0, A101 → A102.
pub fn next_sheet_number(last: Option<&str>) -> String {
    let Some(last) = last else {
        return "A1.0".into();
    };
    if let Some((prefix, rest)) = last.split_once('.') {
        let digits: String = prefix
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if !digits.is_empty() {
            let head = &prefix[..prefix.len() - digits.len()];
            return format!("{head}{}.{rest}", digits.parse::<u32>().unwrap_or(0) + 1);
        }
    }
    next_grid_name(Some(last))
}

/// Sheets in `stage`'s deliverable set, in number order. When no sheet is assigned to
/// any stage yet, every sheet counts.
pub fn stage_sheets(doc: &Document, stage: Option<ElementId>) -> Vec<ElementId> {
    let all = sheets(doc);
    let Some(stage) = stage else {
        return all.into_iter().map(|s| s.0).collect();
    };
    let any_assigned = doc
        .iter()
        .any(|e| matches!(&e.data, ElementData::Sheet { stages, .. } if !stages.is_empty()));
    all.into_iter()
        .filter(|(id, _, _)| {
            !any_assigned || matches!(doc.data(*id), Ok(ElementData::Sheet { stages, .. }) if stages.contains(&stage))
        })
        .map(|s| s.0)
        .collect()
}

/// Records an issue of `sheets` named `name` in the current design stage.
pub fn create_issuance(
    doc: &mut Document,
    name: &str,
    date: &str,
    sheets: Vec<ElementId>,
) -> CoreResult<ElementId> {
    if sheets.is_empty() {
        return Err(CoreError::Invalid("there are no sheets to issue".into()));
    }
    let stage = project_info(doc).and_then(|i| match doc.data(i) {
        Ok(ElementData::ProjectInfo { current_stage, .. }) => *current_stage,
        _ => None,
    });
    let name = non_empty(name)?;
    let date = date.to_owned();
    doc.transact("Issue set", |tx| {
        Ok(tx.insert(ElementData::Issuance {
            name,
            stage,
            date,
            sheets,
        }))
    })
}

/// Links the project to a Rufplan.io project (or unlinks it with `None`).
pub fn link_rufplan(doc: &mut Document, link: Option<RufplanLink>) -> CoreResult<()> {
    let info = project_info(doc).ok_or_else(|| CoreError::Invalid("no project info".into()))?;
    let mut data = doc.data(info)?.clone();
    if let ElementData::ProjectInfo { rufplan, .. } = &mut data {
        *rufplan = link.clone();
    }
    let label = if link.is_some() {
        "Link to Rufplan"
    } else {
        "Unlink from Rufplan"
    };
    doc.transact(label, |tx| tx.set(info, data))
}

/// The linked Rufplan.io project, if any.
pub fn rufplan_link(doc: &Document) -> Option<RufplanLink> {
    project_info(doc).and_then(|i| match doc.data(i) {
        Ok(ElementData::ProjectInfo { rufplan, .. }) => rufplan.clone(),
        _ => None,
    })
}

/// Issuances that included `sheet`, oldest first, as (name, date, stage abbreviation).
pub fn sheet_issues(doc: &Document, sheet: ElementId) -> Vec<(String, String, String)> {
    let mut v: Vec<(ElementId, String, String, String)> = doc
        .of(Category::Issuance)
        .filter_map(|e| match &e.data {
            ElementData::Issuance {
                name,
                stage,
                date,
                sheets,
            } if sheets.contains(&sheet) => {
                let abbr = stage
                    .and_then(|s| doc.data(s).ok())
                    .and_then(|d| match d {
                        ElementData::Stage { abbreviation, .. } => Some(abbreviation.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                Some((e.id, name.clone(), date.clone(), abbr))
            }
            _ => None,
        })
        .collect();
    v.sort_by_key(|x| x.0);
    v.into_iter().map(|x| (x.1, x.2, x.3)).collect()
}

/// Text sizes offered for notes, as (paper mm, label).
pub const TEXT_SIZES: &[(f64, &str)] = &[
    (2.4, "3/32\""),
    (3.0, "1/8\""),
    (4.8, "3/16\""),
    (6.4, "1/4\""),
    (12.7, "1/2\""),
    (25.4, "1\""),
];

/// Creates a sheet with the next number.
pub fn create_sheet(doc: &mut Document, name: &str, size: SheetSize) -> CoreResult<ElementId> {
    let number = next_sheet_number(sheets(doc).last().map(|s| s.1.as_str()));
    let name = name.to_owned();
    doc.transact("Create sheet", |tx| {
        Ok(tx.insert(ElementData::Sheet {
            number,
            name,
            size,
            stages: vec![],
        }))
    })
}

/// Places `view` on `sheet` centered at `center` (paper mm). A drawing view can be on one
/// sheet only (like Revit); schedules can repeat. 3D views can't be placed yet.
pub fn place_view(
    doc: &mut Document,
    sheet: ElementId,
    view: ElementId,
    center: Pt,
) -> CoreResult<ElementId> {
    if !matches!(doc.data(sheet)?, ElementData::Sheet { .. }) {
        return Err(CoreError::Invalid("not a sheet".into()));
    }
    let kind = match doc.data(view)? {
        ElementData::View { kind, .. } => kind.clone(),
        _ => {
            return Err(CoreError::Invalid(
                "only views can be placed on sheets".into(),
            ))
        }
    };
    if matches!(kind, ViewKind::ThreeD) {
        return Err(CoreError::Invalid(
            "3D views can't be placed on sheets yet".into(),
        ));
    }
    if !matches!(kind, ViewKind::Schedule { .. }) {
        let placed = doc.iter().find_map(|e| match &e.data {
            ElementData::Viewport {
                sheet: s, view: v, ..
            } if *v == view => Some(*s),
            _ => None,
        });
        if let Some(s) = placed {
            let label = doc.data(s).map(|d| d.name()).unwrap_or_default();
            return Err(CoreError::Invalid(format!(
                "that view is already on sheet {label}"
            )));
        }
    }
    doc.transact("Place view on sheet", |tx| {
        Ok(tx.insert(ElementData::Viewport {
            sheet,
            view,
            center,
        }))
    })
}

fn add_level(tx: &mut Tx<'_>, name: &str, elevation: f64) -> ElementId {
    let id = tx.insert(ElementData::Level {
        name: name.into(),
        elevation,
    });
    tx.insert(ElementData::view(
        name,
        ViewKind::FloorPlan { level: id },
        48,
    ));
    tx.insert(ElementData::view(
        name,
        ViewKind::CeilingPlan { level: id },
        48,
    ));
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
            location: LocationLine::Centerline,
            attach_top: false,
        }))
    })
}

/// A wall drawn along `a` → `b` by its `location` line (e.g. its exterior finish face).
/// The wall is stored by its centerline, shifted from the drawn line accordingly.
pub fn create_wall_located(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    a: Pt,
    b: Pt,
    location: LocationLine,
) -> CoreResult<ElementId> {
    let (width, layers) = match doc.data(type_id)? {
        ElementData::WallType {
            thickness, layers, ..
        } => (*thickness, layers.clone()),
        _ => return Err(CoreError::Invalid("pick a wall type".into())),
    };
    let off = crate::compound::location_offset(&layers, width, location);
    // The exterior is on the left of a → b; the centerline is `off` to the right of the
    // location line.
    let shift = b.sub(a).norm().perp().scale(-off);
    let id = create_wall(doc, type_id, level, a.add(shift), b.add(shift))?;
    if location != LocationLine::Centerline {
        doc.transact("Create wall", |tx| {
            tx.modify(id, |d| {
                if let ElementData::Wall { location: l, .. } = d {
                    *l = location;
                }
            })
        })?;
    }
    Ok(id)
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
            bound: SlabBound::Sketch,
            sketch: vec![],
        }))
    })
}

/// A floor whose boundary follows the outer faces of its level's walls (Floor: Pick
/// Walls). `boundary` is their current outline, kept in case the walls go away.
pub fn create_floor_by_walls(
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
            bound: SlabBound::Walls,
            sketch: vec![],
        }))
    })
}

/// A ceiling that follows the room enclosing `inside` (Ceiling: Auto Room).
pub fn create_ceiling_in_room(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    boundary: Vec<Pt>,
    inside: Pt,
) -> CoreResult<ElementId> {
    doc.transact("Create ceiling", |tx| {
        Ok(tx.insert(ElementData::Ceiling {
            type_id,
            level,
            height: DEFAULT_CEILING_HEIGHT,
            boundary: ccw(boundary),
            bound: SlabBound::Room { point: inside },
            sketch: vec![],
        }))
    })
}

/// Sets where a floor's or ceiling's boundary comes from. Detaching (`Sketch`) passes the
/// shape it currently has, so it stays put.
pub fn set_slab_bound(
    doc: &mut Document,
    id: ElementId,
    to: SlabBound,
    boundary: Option<Vec<Pt>>,
) -> CoreResult<()> {
    doc.transact("Change boundary", |tx| {
        tx.modify(id, |d| match d {
            ElementData::Floor {
                bound, boundary: b, ..
            }
            | ElementData::Ceiling {
                bound, boundary: b, ..
            } => {
                *bound = to;
                if let Some(nb) = boundary {
                    *b = ccw(nb);
                }
            }
            _ => {}
        })
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
            bound: SlabBound::Sketch,
            sketch: vec![],
        }))
    })
}

/// Next free numeric mark in a category ("1", "2", …).
pub(crate) fn next_mark(doc: &Document, cat: Category) -> String {
    let max = doc
        .of(cat)
        .filter_map(|e| match &e.data {
            ElementData::Door { mark, .. } | ElementData::Window { mark, .. } => {
                mark.parse::<u32>().ok()
            }
            ElementData::Room { number, .. } => number.parse::<u32>().ok(),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    (max + 1).to_string()
}

/// Places a door in `host` with its center `offset` mm from the wall start.
pub fn create_door(
    doc: &mut Document,
    type_id: ElementId,
    host: ElementId,
    offset: f64,
    flip_facing: bool,
) -> CoreResult<ElementId> {
    if !matches!(doc.data(type_id)?, ElementData::DoorType { .. }) {
        return Err(CoreError::Invalid("pick a door type".into()));
    }
    let mark = next_mark(doc, Category::Door);
    doc.transact("Place door", |tx| {
        let id = tx.insert(ElementData::Door {
            type_id,
            host,
            offset,
            flip_hand: false,
            flip_facing,
            mark,
        });
        tag_in_plans(tx, id);
        Ok(id)
    })
}

/// Floor plan views of the level an element belongs to (a hosted element's host level).
fn target_level(tx: &Tx<'_>, target: ElementId) -> Option<ElementId> {
    match tx.data(target).ok()? {
        ElementData::Door { host, .. } | ElementData::Window { host, .. } => {
            tx.data(*host).ok()?.level()
        }
        ElementData::Room { level, .. } => Some(*level),
        ElementData::Column { base_level, .. } => Some(*base_level),
        // Beams frame the floor above the plan they're tagged in (seen overhead there).
        ElementData::Beam { level, .. } => {
            let z = tx.data(*level).ok().and_then(|d| match d {
                ElementData::Level { elevation, .. } => Some(*elevation),
                _ => None,
            })?;
            tx.of(Category::Level)
                .filter_map(|e| match &e.data {
                    ElementData::Level { elevation, .. } if *elevation < z - 1.0 => {
                        Some((*elevation, e.id))
                    }
                    _ => None,
                })
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map_or(Some(*level), |l| Some(l.1))
        }
        _ => None,
    }
}

fn plan_views_of(tx: &Tx<'_>, level: ElementId) -> Vec<ElementId> {
    tx.iter()
        .filter(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level: l }, callout_of: None, .. } if *l == level))
        .map(|e| e.id)
        .collect()
}

/// Revit-style "tag on placement": a tag in every floor plan of the target's level.
pub(crate) fn tag_in_plans(tx: &mut Tx<'_>, target: ElementId) {
    let Some(level) = target_level(tx, target) else {
        return;
    };
    for view in plan_views_of(tx, level) {
        tx.insert(ElementData::Tag {
            view,
            target,
            offset: Pt::default(),
        });
    }
}

/// Tags every untagged door, window and room shown in `view` (a floor plan). Returns how
/// many tags were added.
pub fn tag_all(doc: &mut Document, view: ElementId) -> CoreResult<usize> {
    let ElementData::View {
        kind: ViewKind::FloorPlan { level },
        ..
    } = doc.data(view)?
    else {
        return Err(CoreError::Invalid("open a floor plan to tag".into()));
    };
    let level = *level;
    doc.transact("Tag all", |tx| {
        let tagged: std::collections::HashSet<ElementId> = tx
            .iter()
            .filter_map(|e| match &e.data {
                ElementData::Tag {
                    view: v, target, ..
                } if *v == view => Some(*target),
                _ => None,
            })
            .collect();
        let targets: Vec<ElementId> = tx
            .iter()
            .filter(|e| {
                matches!(
                    e.category(),
                    Category::Door
                        | Category::Window
                        | Category::Room
                        | Category::Column
                        | Category::Beam
                )
            })
            .map(|e| e.id)
            .filter(|id| !tagged.contains(id))
            .collect();
        let mut n = 0;
        for target in targets {
            if target_level(tx, target) == Some(level) {
                tx.insert(ElementData::Tag {
                    view,
                    target,
                    offset: Pt::default(),
                });
                n += 1;
            }
        }
        Ok(n)
    })
}

/// Tags everything in every floor plan, for files saved before tags were elements.
pub fn ensure_tags(doc: &mut Document) -> CoreResult<()> {
    if doc.of(Category::Tag).next().is_some() {
        return Ok(());
    }
    let plans: Vec<ElementId> = doc
        .iter()
        .filter(|e| {
            matches!(
                &e.data,
                ElementData::View {
                    kind: ViewKind::FloorPlan { .. },
                    ..
                }
            )
        })
        .map(|e| e.id)
        .collect();
    for v in plans {
        tag_all(doc, v)?;
    }
    Ok(())
}

/// Places a room at `point` on `level`, named "Room" with the next free number. The
/// caller checks that the point is enclosed (that needs derived geometry).
pub fn create_room(doc: &mut Document, level: ElementId, point: Pt) -> CoreResult<ElementId> {
    let number = next_mark(doc, Category::Room);
    doc.transact("Place room", |tx| {
        let id = tx.insert(ElementData::Room {
            level,
            point,
            name: "Room".into(),
            number,
        });
        tag_in_plans(tx, id);
        Ok(id)
    })
}

/// Places a window in `host` at the type's default sill height.
pub fn create_window(
    doc: &mut Document,
    type_id: ElementId,
    host: ElementId,
    offset: f64,
    flip_facing: bool,
) -> CoreResult<ElementId> {
    let ElementData::WindowType { sill, .. } = doc.data(type_id)? else {
        return Err(CoreError::Invalid("pick a window type".into()));
    };
    let sill = *sill;
    let mark = next_mark(doc, Category::Window);
    doc.transact("Place window", |tx| {
        let id = tx.insert(ElementData::Window {
            type_id,
            host,
            offset,
            sill,
            flip_facing,
            mark,
        });
        tag_in_plans(tx, id);
        Ok(id)
    })
}

pub(crate) fn ccw(mut ring: Vec<Pt>) -> Vec<Pt> {
    if studio_geom::signed_area(&ring) < 0.0 {
        ring.reverse();
    }
    ring
}

/// Deletes elements (and their dependents) in one transaction.
pub fn delete(doc: &mut Document, ids: &[ElementId]) -> CoreResult<usize> {
    crate::visibility::ensure_unpinned(doc, ids)?;
    for id in ids {
        if matches!(doc.data(*id)?, ElementData::ProjectInfo { .. }) {
            return Err(CoreError::Invalid(
                "project information can't be deleted".into(),
            ));
        }
        if matches!(doc.data(*id)?, ElementData::Material { .. }) {
            let n = crate::material::uses(doc, *id);
            if n > 0 {
                return Err(CoreError::Invalid(format!(
                    "{} is used by {n} type{}; pick another material for its layers first",
                    doc.data(*id)?.name(),
                    if n == 1 { "" } else { "s" }
                )));
            }
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
    /// A button: setting the property (to any value) performs the action.
    Action,
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

pub(crate) fn p(key: &str, label: &str, group: &str, value: String, kind: PropKind) -> Property {
    Property {
        key: key.into(),
        label: label.into(),
        group: group.into(),
        value,
        kind,
        options: vec![],
    }
}

pub(crate) fn len(key: &str, label: &str, group: &str, mm: f64) -> Property {
    p(key, label, group, format_ft_in(mm), PropKind::Length)
}

pub(crate) fn text(key: &str, label: &str, group: &str, v: &str) -> Property {
    p(key, label, group, v.to_owned(), PropKind::Text)
}

pub(crate) fn ro(key: &str, label: &str, group: &str, v: String) -> Property {
    p(key, label, group, v, PropKind::ReadOnly)
}

pub(crate) fn choice(
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

pub(crate) fn options_of(doc: &Document, cat: Category) -> Vec<PropOption> {
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

pub(crate) fn level_options(doc: &Document) -> Vec<PropOption> {
    doc.levels()
        .into_iter()
        .map(|(id, name, _)| PropOption {
            id: id.to_string(),
            label: name,
        })
        .collect()
}

/// Properties of an element for the properties panel.
fn bound_row(bound: SlabBound, attached: &str, label: &str) -> Property {
    choice(
        "bound",
        "Boundary",
        "Constraints",
        match bound {
            SlabBound::Sketch => "sketch",
            SlabBound::Walls => "walls",
            SlabBound::Room { .. } => "room",
        }
        .into(),
        vec![
            PropOption {
                id: attached.into(),
                label: label.into(),
            },
            PropOption {
                id: "sketch".into(),
                label: "Sketched".into(),
            },
        ],
    )
}

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
            layers,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(len("thickness", "Width", "Construction", *thickness));
            crate::compound::layer_properties(layers, &crate::material::options(doc), &mut props);
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
            location,
            attach_top,
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
            props.push(choice(
                "location",
                "Location Line",
                "Constraints",
                location.label().into(),
                LocationLine::ALL
                    .iter()
                    .map(|l| PropOption {
                        id: l.label().into(),
                        label: l.label().into(),
                    })
                    .collect(),
            ));
            props.push(flag(
                "attach_top",
                "Top Attached to Roof",
                "Constraints",
                *attach_top,
            ));
        }
        ElementData::FloorType {
            name,
            thickness,
            layers,
        }
        | ElementData::CeilingType {
            name,
            thickness,
            layers,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(len("thickness", "Thickness", "Construction", *thickness));
            crate::compound::layer_properties_in(
                layers,
                &crate::material::options(doc),
                &mut props,
                crate::compound::GROUP_TOP_DOWN,
            );
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
            if let ElementData::Floor { bound, sketch, .. } = &el.data {
                // A sketch with lines locked to walls follows them (ADR-021).
                let b = if sketch.is_empty() {
                    *bound
                } else if crate::sketch::has_locked(sketch) {
                    SlabBound::Walls
                } else {
                    SlabBound::Sketch
                };
                props.push(bound_row(b, "walls", "Follows Walls"));
            }
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
            if let ElementData::Ceiling { bound, .. } = &el.data {
                props.push(bound_row(*bound, "room", "Follows Room"));
            }
            props.push(ro(
                "area",
                "Area",
                "Dimensions",
                format_area_sf(studio_geom::signed_area(boundary).abs()),
            ));
        }
        ElementData::View {
            name,
            kind,
            scale,
            crop,
            show_crop,
            section_box,
            callout_of,
            mark_type,
            camera,
            ..
        } => {
            props.push(text("name", "View Name", "Identity Data", name));
            if let Some(c) = camera {
                // Revit's camera parameters (ADR-027).
                props.push(len(
                    "eye_elevation",
                    "Eye Elevation",
                    "Camera",
                    c.eye_height,
                ));
                props.push(len(
                    "target_elevation",
                    "Target Elevation",
                    "Camera",
                    c.target_height,
                ));
                props.push(text(
                    "fov",
                    "Field of View (°)",
                    "Camera",
                    &format!("{:.0}", c.fov),
                ));
                props.push(ro(
                    "camera_level",
                    "Reference Level",
                    "Camera",
                    doc.data(c.level).map(|d| d.name()).unwrap_or_default(),
                ));
            }
            if matches!(kind, ViewKind::Elevation { .. }) {
                // The mark drawn for this building elevation in plans (ADR-022).
                let current = mark_type.or_else(|| crate::detail::default_mark_type(doc, false));
                props.push(choice(
                    "mark_type",
                    "Elevation Mark",
                    "Graphics",
                    current.map(|t| t.to_string()).unwrap_or_default(),
                    crate::detail::mark_type_options(doc),
                ));
            }
            if let Some(parent) = callout_of {
                props.push(ro(
                    "callout_of",
                    "Callout Of",
                    "Identity Data",
                    doc.data(*parent).map(|d| d.name()).unwrap_or_default(),
                ));
            }
            if matches!(kind, ViewKind::ThreeD) {
                props.push(flag(
                    "section_box",
                    "Section Box",
                    "Extents",
                    section_box.is_some(),
                ));
                if let Some(b) = section_box {
                    for (i, (lo, hi)) in [("West", "East"), ("South", "North"), ("Bottom", "Top")]
                        .iter()
                        .enumerate()
                    {
                        props.push(len(&format!("box_min_{i}"), lo, "Extents", b.min[i]));
                        props.push(len(&format!("box_max_{i}"), hi, "Extents", b.max[i]));
                    }
                }
            }
            // Schedules have no drawing scale.
            if !matches!(kind, ViewKind::Schedule { .. }) {
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
            }
            let kind_label = match kind {
                ViewKind::FloorPlan { .. } => "Floor Plan",
                ViewKind::CeilingPlan { .. } => "Ceiling Plan",
                ViewKind::Elevation { .. } => "Elevation",
                ViewKind::ThreeD => "3D View",
                ViewKind::Section { .. } => "Section",
                ViewKind::Schedule { .. } => "Schedule",
                ViewKind::MarkerElevation { .. } => "Elevation",
            };
            if let ViewKind::Section { depth, .. } = kind {
                props.push(len("depth", "Far Clip Offset", "Extents", *depth));
            }
            if matches!(
                kind,
                ViewKind::FloorPlan { .. }
                    | ViewKind::CeilingPlan { .. }
                    | ViewKind::Elevation { .. }
                    | ViewKind::Section { .. }
            ) {
                props.push(flag("crop", "Crop View", "Extents", crop.is_some()));
                props.push(flag(
                    "show_crop",
                    "Crop Region Visible",
                    "Extents",
                    *show_crop,
                ));
            }
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
            rufplan,
            param_defs,
        } => {
            props.push(text("name", "Project Name", "Project", name));
            for d in param_defs {
                props.push(ro(
                    &format!("def:{}", d.key),
                    &d.label,
                    "Project Parameters",
                    format!(
                        "{} · {} · {}",
                        d.kind.label(),
                        if d.scope == crate::params::ParamScope::Type {
                            "Type"
                        } else {
                            "Instance"
                        },
                        d.categories
                            .iter()
                            .map(|c| category_label(*c))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
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
            props.push(ro(
                "rufplan",
                "Linked Project",
                "Rufplan",
                rufplan
                    .as_ref()
                    .map_or_else(|| "Not linked".into(), |l| l.name.clone()),
            ));
        }
        ElementData::Dimension {
            a,
            b,
            offset,
            a_ref,
            b_ref,
            ..
        } => {
            let (a, b) = dimension_ends(doc, &el.data).unwrap_or((*a, *b));
            props.push(ro("value", "Value", "Dimensions", format_ft_in(a.dist(b))));
            let attached = [a_ref, b_ref].iter().filter(|r| r.is_some()).count();
            props.push(ro(
                "attached",
                "Follows Model",
                "Dimensions",
                format!("{attached} of 2 ends"),
            ));
            props.push(len("offset", "Offset from Points", "Graphics", *offset));
        }
        ElementData::TextNote { text: t, size, .. } => {
            props.push(text("text", "Text", "Text", t));
            let current = TEXT_SIZES
                .iter()
                .min_by(|a, b| (a.0 - size).abs().total_cmp(&(b.0 - size).abs()))
                .map_or(3.0, |s| s.0);
            props.push(choice(
                "size",
                "Text Size",
                "Text",
                current.to_string(),
                TEXT_SIZES
                    .iter()
                    .map(|(mm, l)| PropOption {
                        id: mm.to_string(),
                        label: (*l).into(),
                    })
                    .collect(),
            ));
        }
        ElementData::Tag { target, .. } => {
            props.push(ro(
                "target",
                "Tags",
                "Identity Data",
                doc.data(*target).map(|d| d.name()).unwrap_or_default(),
            ));
        }
        ElementData::Issuance {
            name,
            date,
            sheets,
            stage,
        } => {
            props.push(ro("name", "Issue", "Identity Data", name.clone()));
            props.push(ro("date", "Date", "Identity Data", date.clone()));
            props.push(ro(
                "stage",
                "Design Stage",
                "Identity Data",
                stage
                    .and_then(|s| doc.data(s).ok())
                    .map(|d| d.name())
                    .unwrap_or_default(),
            ));
            props.push(ro(
                "sheets",
                "Sheets",
                "Identity Data",
                sheets.len().to_string(),
            ));
        }
        ElementData::Sheet {
            number,
            name,
            size,
            stages: in_stages,
        } => {
            props.push(text("number", "Sheet Number", "Identity Data", number));
            props.push(text("name", "Sheet Name", "Identity Data", name));
            props.push(choice(
                "size",
                "Size",
                "Identity Data",
                format!("{size:?}"),
                [SheetSize::ArchD, SheetSize::Tabloid]
                    .iter()
                    .map(|s| PropOption {
                        id: format!("{s:?}"),
                        label: s.label().into(),
                    })
                    .collect(),
            ));
            for (sid, sname, abbr) in stages(doc) {
                props.push(choice(
                    &format!("stage:{sid}"),
                    &format!("{abbr} — {sname}"),
                    "Stage Sets",
                    yes_no(in_stages.contains(&sid)),
                    yes_no_options(),
                ));
            }
        }
        ElementData::Viewport { sheet, view, .. } => {
            props.push(ro(
                "view",
                "View",
                "Identity Data",
                doc.data(*view).map(|d| d.name()).unwrap_or_default(),
            ));
            props.push(ro(
                "sheet",
                "Sheet",
                "Identity Data",
                doc.data(*sheet).map(|d| d.name()).unwrap_or_default(),
            ));
            if let Ok(ElementData::View { scale, .. }) = doc.data(*view) {
                props.push(ro("scale", "View Scale", "Graphics", scale_label(*scale)));
            }
        }
        ElementData::Room {
            level,
            name,
            number,
            ..
        } => {
            props.push(text("name", "Name", "Identity Data", name));
            props.push(text("number", "Number", "Identity Data", number));
            props.push(ro(
                "level",
                "Level",
                "Constraints",
                doc.data(*level).map(|d| d.name()).unwrap_or_default(),
            ));
        }
        ElementData::DoorType {
            name,
            family,
            width,
            height,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(ro(
                "family",
                "Family",
                "Identity Data",
                match family {
                    DoorFamily::SingleFlush => "Single Flush".into(),
                    DoorFamily::DoubleFlush => "Double Flush".into(),
                },
            ));
            props.push(len("width", "Width", "Dimensions", *width));
            props.push(len("height", "Height", "Dimensions", *height));
        }
        ElementData::WindowType {
            name,
            family,
            width,
            height,
            sill,
            units,
            grille,
            finish,
        } => {
            let fam = crate::windows::info(*family);
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(ro("family", "Family", "Identity Data", fam.label.into()));
            props.push(len("width", "Width", "Dimensions", *width));
            props.push(len("height", "Height", "Dimensions", *height));
            props.push(len("sill", "Default Sill Height", "Dimensions", *sill));
            if fam.mullable {
                props.push(choice(
                    "units",
                    "Units Mulled",
                    "Construction",
                    units.to_string(),
                    (1..=4)
                        .map(|n| PropOption {
                            id: n.to_string(),
                            label: match n {
                                1 => "1 (single)".into(),
                                2 => "2 (twin)".into(),
                                3 => "3 (triple)".into(),
                                n => n.to_string(),
                            },
                        })
                        .collect(),
                ));
            }
            if fam.grilles {
                props.push(choice(
                    "grille",
                    "Grille Pattern",
                    "Construction",
                    format!("{grille:?}"),
                    crate::windows::Grille::ALL
                        .iter()
                        .map(|g| PropOption {
                            id: format!("{g:?}"),
                            label: g.label().into(),
                        })
                        .collect(),
                ));
            }
            props.push(choice(
                "finish",
                "Frame Finish",
                "Materials and Finishes",
                format!("{finish:?}"),
                crate::windows::FrameFinish::ALL
                    .iter()
                    .map(|f| PropOption {
                        id: format!("{f:?}"),
                        label: f.label().into(),
                    })
                    .collect(),
            ));
        }
        ElementData::Door {
            host,
            offset,
            flip_hand,
            flip_facing,
            mark,
            ..
        } => {
            props.push(text("mark", "Mark", "Identity Data", mark));
            props.push(ro("host", "Host", "Constraints", host_label(doc, *host)));
            props.push(len(
                "offset",
                "Offset from Wall Start",
                "Constraints",
                *offset,
            ));
            props.push(choice(
                "flip_hand",
                "Flip Hand",
                "Graphics",
                yes_no(*flip_hand),
                yes_no_options(),
            ));
            props.push(choice(
                "flip_facing",
                "Flip Facing",
                "Graphics",
                yes_no(*flip_facing),
                yes_no_options(),
            ));
        }
        ElementData::Window {
            host,
            offset,
            sill,
            flip_facing,
            mark,
            ..
        } => {
            props.push(text("mark", "Mark", "Identity Data", mark));
            props.push(ro("host", "Host", "Constraints", host_label(doc, *host)));
            props.push(len(
                "offset",
                "Offset from Wall Start",
                "Constraints",
                *offset,
            ));
            props.push(len("sill", "Sill Height", "Constraints", *sill));
            props.push(choice(
                "flip_facing",
                "Flip Facing",
                "Graphics",
                yes_no(*flip_facing),
                yes_no_options(),
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
        ElementData::Roof { .. } | ElementData::RoofType { .. } | ElementData::Stair { .. } => {
            crate::build::properties(doc, id, &mut props);
        }
        ElementData::Column { .. }
        | ElementData::ColumnType { .. }
        | ElementData::Beam { .. }
        | ElementData::BeamType { .. }
        | ElementData::Railing { .. }
        | ElementData::RailingType { .. } => {
            crate::structure::properties(doc, id, &mut props);
        }
        ElementData::Material { .. } => crate::material::properties(doc, id, &mut props),
        ElementData::Site { .. } => crate::site::properties(doc, id, &mut props),
        ElementData::ElevationMarkerType {
            name,
            interior,
            style,
            size,
        } => {
            props.push(text("name", "Type Name", "Identity Data", name));
            props.push(flag(
                "interior",
                "Interior (crops to the room)",
                "Constraints",
                *interior,
            ));
            props.push(choice(
                "style",
                "Symbol",
                "Graphics",
                format!("{style:?}"),
                crate::element::MarkStyle::ALL
                    .iter()
                    .map(|s| PropOption {
                        id: format!("{s:?}"),
                        label: s.label().into(),
                    })
                    .collect(),
            ));
            props.push(text(
                "size",
                "Body Radius (paper mm)",
                "Graphics",
                &format!("{size}"),
            ));
        }
        ElementData::ElevationMarker {
            level,
            interior,
            type_id,
            ..
        } => {
            // The family type is chosen in the type selector above (ADR-022).
            let _ = (type_id, interior);
            props.push(ro(
                "level",
                "Level",
                "Constraints",
                doc.data(*level).map(|d| d.name()).unwrap_or_default(),
            ));
            // Revit's check boxes around the marker: one view per direction.
            let views = crate::detail::marker_views(doc, id);
            for c in crate::detail::DIRECTIONS {
                let name = crate::detail::compass_name(c);
                props.push(flag(
                    &format!("view_{name}"),
                    &format!("{name} View"),
                    "Views",
                    views.iter().any(|v| v.0 == c),
                ));
            }
        }
        ElementData::RoomSeparator { level, start, end } => {
            props.push(choice(
                "level",
                "Level",
                "Constraints",
                level.to_string(),
                level_options(doc),
            ));
            props.push(ro(
                "length",
                "Length",
                "Dimensions",
                format_ft_in(start.dist(*end)),
            ));
        }
    }
    crate::params::param_properties(doc, id, &mut props);
    if crate::visibility::is_pinned(doc, id) {
        props.push(ro(
            "pinned",
            "Pinned",
            "Identity Data",
            "Yes (UP to unpin)".into(),
        ));
    }
    Ok(PropertySheet {
        id,
        category: el.category(),
        title: el.data.name(),
        type_id: el.data.type_id(),
        properties: props,
    })
}

pub(crate) fn parse_len(v: &str) -> CoreResult<f64> {
    parse_length(v)
        .ok_or_else(|| CoreError::Invalid(format!("\"{v}\" is not a length (try 10'-6\")")))
}

pub(crate) fn parse_id(v: &str) -> CoreResult<ElementId> {
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
    if let Some(pkey) = key.strip_prefix("param:") {
        return crate::params::set_value(doc, id, pkey, value);
    }
    if matches!(
        data,
        ElementData::Roof { .. } | ElementData::RoofType { .. } | ElementData::Stair { .. }
    ) {
        return crate::build::set_property(doc, id, key, value);
    }
    if matches!(data, ElementData::Material { .. }) {
        return crate::material::set_property(doc, id, key, value);
    }
    if matches!(data, ElementData::Site { .. }) {
        return crate::site::set_property(doc, id, key, value);
    }
    if let (ElementData::ElevationMarker { .. }, "type") = (&data, key) {
        let t = parse_id(value)?;
        let ElementData::ElevationMarkerType { interior: to, .. } = doc.data(t)? else {
            return Err(CoreError::Invalid("pick an elevation mark type".into()));
        };
        let to = *to;
        return doc.transact("Change type", |tx| {
            tx.modify(id, |d| {
                if let ElementData::ElevationMarker {
                    type_id, interior, ..
                } = d
                {
                    *type_id = Some(t);
                    // Interior marks crop their views to the room; building marks don't.
                    *interior = to;
                }
            })
        });
    }
    if let ElementData::ElevationMarkerType { .. } = &data {
        return doc.transact("Change mark type", |tx| {
            let mut d = tx.data(id)?.clone();
            if let ElementData::ElevationMarkerType {
                name,
                interior,
                style,
                size,
            } = &mut d
            {
                match key {
                    "name" => *name = non_empty(value)?,
                    "interior" => *interior = value == "yes",
                    "style" => {
                        *style = crate::element::MarkStyle::ALL
                            .into_iter()
                            .find(|s| format!("{s:?}") == value || s.label() == value)
                            .ok_or_else(|| CoreError::Invalid(format!("unknown symbol {value}")))?
                    }
                    "size" => {
                        let v: f64 = value
                            .trim()
                            .trim_end_matches("mm")
                            .trim()
                            .parse()
                            .map_err(|_| CoreError::Invalid("size is paper mm, e.g. 5".into()))?;
                        if !(1.0..=20.0).contains(&v) {
                            return Err(CoreError::Invalid("size is 1 to 20 mm".into()));
                        }
                        *size = v;
                    }
                    _ => return Err(CoreError::Invalid(format!("unknown property {key}"))),
                }
            }
            tx.set(id, d)
        });
    }
    if let (ElementData::View { .. }, "mark_type") = (&data, key) {
        let t = parse_id(value)?;
        return doc.transact("Change elevation mark", |tx| {
            tx.modify(id, |d| {
                if let ElementData::View { mark_type, .. } = d {
                    *mark_type = Some(t);
                }
            })
        });
    }
    if let (ElementData::ElevationMarker { .. }, Some(dir)) = (&data, key.strip_prefix("view_")) {
        let facing = crate::detail::DIRECTIONS
            .into_iter()
            .find(|c| crate::detail::compass_name(*c) == dir)
            .ok_or_else(|| CoreError::Invalid(format!("unknown direction {dir}")))?;
        // Adding a view needs its room's name: see studio_regen::derived.
        let name = (value == "yes").then(|| format!("Elevation - {dir}"));
        return crate::detail::set_marker_view(doc, id, facing, name);
    }
    if matches!(
        data,
        ElementData::Column { .. }
            | ElementData::ColumnType { .. }
            | ElementData::Beam { .. }
            | ElementData::BeamType { .. }
            | ElementData::Railing { .. }
            | ElementData::RailingType { .. }
    ) {
        return crate::structure::set_property(doc, id, key, value);
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
            layers,
        } => match key {
            k if k.starts_with("layer") => {
                crate::compound::set_layer_property(layers, *thickness, k, value, &|m| {
                    crate::material::name_of(doc, m)
                })?;
                if !layers.is_empty() {
                    *thickness = layers.iter().map(|l| l.thickness).sum();
                }
            }
            "name" => *name = non_empty(value)?,
            "thickness" => {
                let w = positive(parse_len(value)?)?;
                crate::compound::resize_structure(layers, w)?;
                *thickness = w;
            }
            "function" => {
                *function = if value == "Exterior" {
                    WallFunction::Exterior
                } else {
                    WallFunction::Interior
                }
            }
            _ => return Err(unknown()),
        },
        ElementData::FloorType {
            name,
            thickness,
            layers,
        }
        | ElementData::CeilingType {
            name,
            thickness,
            layers,
        } => match key {
            k if k.starts_with("layer") => {
                crate::compound::set_layer_property(layers, *thickness, k, value, &|m| {
                    crate::material::name_of(doc, m)
                })?;
                if !layers.is_empty() {
                    *thickness = layers.iter().map(|l| l.thickness).sum();
                }
            }
            "name" => *name = non_empty(value)?,
            "thickness" => {
                let w = positive(parse_len(value)?)?;
                crate::compound::resize_structure(layers, w)?;
                *thickness = w;
            }
            _ => return Err(unknown()),
        },
        ElementData::Wall {
            type_id,
            start,
            end,
            base_level,
            base_offset,
            top,
            location,
            attach_top,
        } => match key {
            "location" => {
                *location = LocationLine::parse(value)
                    .ok_or_else(|| CoreError::Invalid(format!("unknown location line {value}")))?
            }
            "attach_top" => *attach_top = value == "yes",
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
            bound,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "level" => *level = parse_id(value)?,
            "offset" => *offset = parse_len(value)?,
            // Detaching needs the current outline: see studio_regen::derived.
            "bound" if value == "walls" => *bound = SlabBound::Walls,
            _ => return Err(unknown()),
        },
        ElementData::Ceiling {
            type_id,
            level,
            height,
            bound,
            boundary,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "level" => *level = parse_id(value)?,
            "height" => *height = parse_len(value)?,
            "bound" if value == "room" => {
                let n = boundary.len().max(1) as f64;
                let c = boundary.iter().fold(Pt::default(), |a, p| a.add(*p));
                *bound = SlabBound::Room {
                    point: c.scale(1.0 / n),
                }
            }
            _ => return Err(unknown()),
        },
        ElementData::View {
            name,
            scale,
            kind,
            crop,
            show_crop,
            section_box,
            camera,
            ..
        } => match key {
            "name" => *name = non_empty(value)?,
            "eye_elevation" | "target_elevation" | "fov" => {
                let c = camera
                    .as_mut()
                    .ok_or_else(|| CoreError::Invalid("that view has no camera".into()))?;
                match key {
                    "eye_elevation" => c.eye_height = parse_len(value)?,
                    "target_elevation" => c.target_height = parse_len(value)?,
                    _ => {
                        let v: f64 =
                            value.trim().trim_end_matches('°').parse().map_err(|_| {
                                CoreError::Invalid("enter an angle in degrees".into())
                            })?;
                        if !(10.0..=120.0).contains(&v) {
                            return Err(CoreError::Invalid(
                                "use a field of view from 10° to 120°".into(),
                            ));
                        }
                        c.fov = v;
                    }
                }
            }
            // Turning the box on needs the model's extents: see studio_regen::derived.
            "section_box" if value != "yes" => *section_box = None,
            k if k.starts_with("box_") => {
                let b = section_box
                    .as_mut()
                    .ok_or_else(|| CoreError::Invalid("turn on the section box first".into()))?;
                let v = parse_len(value)?;
                let (end, axis) = k[4..].split_once('_').ok_or_else(unknown)?;
                let i: usize = axis.parse().map_err(|_| unknown())?;
                match (end, i) {
                    ("min", 0..=2) => b.min[i] = v,
                    ("max", 0..=2) => b.max[i] = v,
                    _ => return Err(unknown()),
                }
            }
            "crop" => {
                *crop = if value == "yes" {
                    Some(crop.unwrap_or(CropBox {
                        min: Pt::new(-1.0e5, -1.0e5),
                        max: Pt::new(1.0e5, 1.0e5),
                    }))
                } else {
                    None
                };
            }
            "show_crop" => *show_crop = value == "yes",
            "depth" => {
                if let ViewKind::Section { depth, .. } = kind {
                    *depth = positive(parse_len(value)?)?;
                }
            }
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
        ElementData::Dimension { offset, .. } => match key {
            "offset" => *offset = parse_len(value)?,
            _ => return Err(unknown()),
        },
        ElementData::TextNote { text, size, .. } => match key {
            "text" => *text = non_empty(value)?,
            "size" => {
                *size = value
                    .parse()
                    .map_err(|_| CoreError::Invalid("bad text size".into()))?
            }
            _ => return Err(unknown()),
        },
        ElementData::Tag { .. } | ElementData::Issuance { .. } => return Err(unknown()),
        ElementData::Sheet {
            number,
            name,
            size,
            stages,
        } => match key {
            k if k.starts_with("stage:") => {
                let sid = parse_id(&k["stage:".len()..])?;
                stages.retain(|s| *s != sid);
                if value == "yes" {
                    stages.push(sid);
                }
            }
            "number" => *number = non_empty(value)?,
            "name" => *name = non_empty(value)?,
            "size" => {
                *size = if value == "Tabloid" {
                    SheetSize::Tabloid
                } else {
                    SheetSize::ArchD
                }
            }
            _ => return Err(unknown()),
        },
        ElementData::Viewport { .. } => return Err(unknown()),
        ElementData::Room { name, number, .. } => match key {
            "name" => *name = non_empty(value)?,
            "number" => *number = non_empty(value)?,
            _ => return Err(unknown()),
        },
        ElementData::DoorType {
            name,
            width,
            height,
            ..
        } => match key {
            "name" => *name = non_empty(value)?,
            "width" => *width = positive(parse_len(value)?)?,
            "height" => *height = positive(parse_len(value)?)?,
            _ => return Err(unknown()),
        },
        ElementData::WindowType {
            name,
            width,
            height,
            sill,
            units,
            grille,
            finish,
            ..
        } => match key {
            "name" => *name = non_empty(value)?,
            "width" => *width = positive(parse_len(value)?)?,
            "height" => *height = positive(parse_len(value)?)?,
            "sill" => *sill = parse_len(value)?,
            "units" => {
                *units = value
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=4).contains(n))
                    .ok_or_else(|| CoreError::Invalid("mull 1 to 4 units".into()))?
            }
            "grille" => {
                *grille = crate::windows::Grille::ALL
                    .into_iter()
                    .find(|g| format!("{g:?}") == value)
                    .ok_or_else(unknown)?
            }
            "finish" => {
                *finish = crate::windows::FrameFinish::ALL
                    .into_iter()
                    .find(|f| format!("{f:?}") == value)
                    .ok_or_else(unknown)?
            }
            _ => return Err(unknown()),
        },
        ElementData::Door {
            type_id,
            offset,
            flip_hand,
            flip_facing,
            mark,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "offset" => *offset = parse_len(value)?,
            "flip_hand" => *flip_hand = value == "yes",
            "flip_facing" => *flip_facing = value == "yes",
            "mark" => *mark = non_empty(value)?,
            _ => return Err(unknown()),
        },
        ElementData::Window {
            type_id,
            offset,
            sill,
            flip_facing,
            mark,
            ..
        } => match key {
            "type" => *type_id = parse_id(value)?,
            "offset" => *offset = parse_len(value)?,
            "sill" => *sill = parse_len(value)?,
            "flip_facing" => *flip_facing = value == "yes",
            "mark" => *mark = non_empty(value)?,
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
        ElementData::Roof { .. }
        | ElementData::RoofType { .. }
        | ElementData::Stair { .. }
        | ElementData::Column { .. }
        | ElementData::ColumnType { .. }
        | ElementData::Beam { .. }
        | ElementData::BeamType { .. }
        | ElementData::Railing { .. }
        | ElementData::RailingType { .. }
        | ElementData::Material { .. }
        | ElementData::RoomSeparator { .. }
        | ElementData::ElevationMarker { .. }
        | ElementData::ElevationMarkerType { .. }
        | ElementData::Site { .. } => return Err(unknown()),
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
        ElementData::Door { .. } => Category::DoorType,
        ElementData::Window { .. } => Category::WindowType,
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

fn yes_no(b: bool) -> String {
    if b {
        "yes".into()
    } else {
        "no".into()
    }
}

/// A Yes/No property.
pub(crate) fn flag(key: &str, label: &str, group: &str, b: bool) -> Property {
    choice(key, label, group, yes_no(b), yes_no_options())
}

/// Plural display name of an instance category, for parameter and filter lists.
pub fn category_label(c: Category) -> &'static str {
    match c {
        Category::Wall => "Walls",
        Category::Door => "Doors",
        Category::Window => "Windows",
        Category::Room => "Rooms",
        Category::Floor => "Floors",
        Category::Ceiling => "Ceilings",
        Category::Roof => "Roofs",
        Category::Stair => "Stairs",
        Category::Sheet => "Sheets",
        Category::Grid => "Grids",
        Category::Level => "Levels",
        other => other.as_str(),
    }
}

/// Instance categories that project parameters can apply to.
pub const PARAM_CATEGORIES: [Category; 9] = [
    Category::Wall,
    Category::Door,
    Category::Window,
    Category::Room,
    Category::Floor,
    Category::Ceiling,
    Category::Roof,
    Category::Stair,
    Category::Sheet,
];

pub(crate) fn yes_no_options() -> Vec<PropOption> {
    vec![
        PropOption {
            id: "no".into(),
            label: "No".into(),
        },
        PropOption {
            id: "yes".into(),
            label: "Yes".into(),
        },
    ]
}

fn host_label(doc: &Document, host: ElementId) -> String {
    let len = match doc.data(host) {
        Ok(ElementData::Wall { start, end, .. }) => {
            format!(" ({})", format_ft_in(start.dist(*end)))
        }
        _ => String::new(),
    };
    let ty = doc
        .data(host)
        .ok()
        .and_then(|d| d.type_id())
        .and_then(|t| doc.data(t).ok())
        .map(|t| t.name())
        .unwrap_or_else(|| "Wall".into());
    format!("{ty}{len}")
}

pub(crate) fn non_empty(v: &str) -> CoreResult<String> {
    let t = v.trim();
    if t.is_empty() {
        Err(CoreError::Invalid("name can't be empty".into()))
    } else {
        Ok(t.to_owned())
    }
}

pub(crate) fn positive(v: f64) -> CoreResult<f64> {
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
        // 2 levels × (plan + RCP) + 4 elevations + 3D + 7 schedules.
        assert_eq!(doc.of(Category::View).count(), 16);
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
        assert_eq!(doc.of(Category::View).count(), 18);
        delete(&mut doc, &[l3]).unwrap();
        assert_eq!(doc.of(Category::View).count(), 16);
    }

    fn wall_and_types(doc: &mut Document) -> (ElementId, ElementId, ElementId) {
        let wt = first_of(doc, Category::WallType).unwrap();
        let l0 = doc.levels()[0].0;
        let w = create_wall(doc, wt, l0, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let wn = doc
            .of(Category::WindowType)
            .find(|e| e.data.name().starts_with("Fixed 48"))
            .unwrap()
            .id;
        (w, dt, wn)
    }

    #[test]
    fn doors_and_windows_get_marks_and_default_sill() {
        let mut doc = seeded();
        let (w, dt, wn) = wall_and_types(&mut doc);
        let d1 = create_door(&mut doc, dt, w, 800.0, false).unwrap();
        let d2 = create_door(&mut doc, dt, w, 2000.0, true).unwrap();
        let win = create_window(&mut doc, wn, w, 3300.0, false).unwrap();
        assert_eq!(doc.data(d1).unwrap().name(), "Door 1");
        assert_eq!(doc.data(d2).unwrap().name(), "Door 2");
        match doc.data(win).unwrap() {
            ElementData::Window { sill, mark, .. } => {
                assert!((sill - 36.0 * MM_PER_IN).abs() < 1e-9);
                assert_eq!(mark, "1");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn openings_must_fit_and_not_overlap() {
        let mut doc = seeded();
        let (w, dt, wn) = wall_and_types(&mut doc);
        // 36" door centered 300 mm from the start would stick out of the wall.
        assert!(create_door(&mut doc, dt, w, 300.0, false).is_err());
        create_door(&mut doc, dt, w, 800.0, false).unwrap();
        // A window overlapping that door is rejected.
        assert!(create_window(&mut doc, wn, w, 1200.0, false).is_err());
        // Shortening the wall under the door is rejected and leaves the wall unchanged.
        assert!(set_property(&mut doc, w, "length", "3'", 0).is_err());
        assert!(
            matches!(doc.data(w).unwrap(), ElementData::Wall { end, .. } if (end.x - 4000.0).abs() < 1e-9)
        );
        // A sill so high the window pokes out of the 10' wall is rejected.
        let win = create_window(&mut doc, wn, w, 3000.0, false).unwrap();
        assert!(set_property(&mut doc, win, "sill", "7'", 0).is_err());
        // Doors can only take door types.
        assert!(set_property(&mut doc, win, "type", &dt.to_string(), 0).is_err());
    }

    #[test]
    fn deleting_the_wall_deletes_its_openings_and_undo_restores_them() {
        let mut doc = seeded();
        let (w, dt, _) = wall_and_types(&mut doc);
        let d = create_door(&mut doc, dt, w, 800.0, false).unwrap();
        assert_eq!(delete(&mut doc, &[w]).unwrap(), 3, "wall, door and its tag");
        assert!(doc.get(d).is_none());
        doc.undo().unwrap();
        assert!(doc.get(d).is_some());
    }

    #[test]
    fn sheet_numbers_increment_and_views_place_once() {
        assert_eq!(next_sheet_number(None), "A1.0");
        assert_eq!(next_sheet_number(Some("A1.0")), "A2.0");
        assert_eq!(next_sheet_number(Some("A9.0")), "A10.0");
        assert_eq!(next_sheet_number(Some("A101")), "A102");
        let mut doc = seeded();
        let s1 = create_sheet(&mut doc, "Plans", SheetSize::ArchD).unwrap();
        let s2 = create_sheet(&mut doc, "More", SheetSize::Tabloid).unwrap();
        assert_eq!(doc.data(s2).unwrap().name(), "A2.0 - More");
        let plan = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::FloorPlan { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        place_view(&mut doc, s1, plan, Pt::new(300.0, 300.0)).unwrap();
        assert!(
            place_view(&mut doc, s2, plan, Pt::new(300.0, 300.0)).is_err(),
            "one sheet per view"
        );
        let sched = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Schedule { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        place_view(&mut doc, s1, sched, Pt::new(700.0, 300.0)).unwrap();
        place_view(&mut doc, s2, sched, Pt::new(200.0, 200.0)).unwrap();
        // Deleting the sheet removes its viewports but not the views.
        delete(&mut doc, &[s1]).unwrap();
        assert_eq!(doc.of(Category::Viewport).count(), 1);
        assert!(doc.get(plan).is_some());
    }

    #[test]
    fn sections_dimensions_and_text() {
        let mut doc = seeded();
        assert_eq!(
            doc.iter()
                .filter(|e| matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Schedule { .. },
                        ..
                    }
                ))
                .count(),
            7
        );
        let s = create_section(&mut doc, Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)).unwrap();
        assert_eq!(doc.data(s).unwrap().name(), "Section 1");
        set_property(&mut doc, s, "depth", "10'", 0).unwrap();
        let d =
            create_dimension(&mut doc, s, Pt::new(0.0, 0.0), Pt::new(3048.0, 0.0), 500.0).unwrap();
        let sheet = properties(&doc, d).unwrap();
        assert_eq!(sheet.properties[0].value, "10'-0\"");
        let t = create_text(&mut doc, s, Pt::new(100.0, 100.0), "  ").unwrap();
        assert_eq!(doc.data(t).unwrap().name(), "TEXT");
        set_property(&mut doc, t, "text", "VERIFY IN FIELD", 0).unwrap();
        assert_eq!(doc.data(t).unwrap().name(), "VERIFY IN FIELD");
        // Annotations belong to their view.
        delete(&mut doc, &[s]).unwrap();
        assert!(doc.get(d).is_none() && doc.get(t).is_none());
    }

    #[test]
    fn placing_tags_in_plans_and_tag_all() {
        let mut doc = seeded();
        let (w, dt, _) = wall_and_types(&mut doc);
        let d = create_door(&mut doc, dt, w, 800.0, false).unwrap();
        let tags: Vec<_> = doc
            .of(Category::Tag)
            .filter(|e| matches!(&e.data, ElementData::Tag { target, .. } if *target == d))
            .map(|e| e.id)
            .collect();
        assert_eq!(tags.len(), 1, "one tag in the Level 1 floor plan");
        delete(&mut doc, &tags).unwrap();
        assert!(doc.get(d).is_some(), "deleting a tag keeps the door");
        let l0 = doc.levels()[0].0;
        let plan = doc
            .iter()
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l0))
            .unwrap()
            .id;
        assert_eq!(tag_all(&mut doc, plan).unwrap(), 1);
        assert_eq!(tag_all(&mut doc, plan).unwrap(), 0);
        delete(&mut doc, &[d]).unwrap();
        assert_eq!(doc.of(Category::Tag).count(), 0, "tags go with their door");
    }

    #[test]
    fn dimensions_attach_to_walls_and_follow_them() {
        let mut doc = seeded();
        let (w, _, _) = wall_and_types(&mut doc);
        let plan = doc
            .iter()
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::FloorPlan { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        let d = create_dimension(
            &mut doc,
            plan,
            Pt::new(0.0, 0.0),
            Pt::new(4000.0, 0.0),
            500.0,
        )
        .unwrap();
        let data = doc.data(d).unwrap().clone();
        assert!(matches!(
            &data,
            ElementData::Dimension {
                a_ref: Some(_),
                b_ref: Some(_),
                ..
            }
        ));
        set_property(&mut doc, w, "length", "20'", 0).unwrap();
        let (a, b) = dimension_ends(&doc, doc.data(d).unwrap()).unwrap();
        assert!(
            (a.dist(b) - 20.0 * MM_PER_FT).abs() < 1e-6,
            "the dimension follows the wall's end"
        );
        delete(&mut doc, &[w]).unwrap();
        let (a, b) = dimension_ends(&doc, doc.data(d).unwrap()).unwrap();
        assert!(
            (a.dist(b) - 4000.0).abs() < 1e-6,
            "falls back to its stored points"
        );
    }

    #[test]
    fn stage_sets_and_issuances() {
        let mut doc = seeded();
        let a = create_sheet(&mut doc, "Plans", SheetSize::ArchD).unwrap();
        let b = create_sheet(&mut doc, "Details", SheetSize::ArchD).unwrap();
        let sd = stages(&doc).into_iter().find(|s| s.2 == "SD").unwrap().0;
        let cd = stages(&doc).into_iter().find(|s| s.2 == "CD").unwrap().0;
        assert_eq!(
            stage_sheets(&doc, Some(sd)),
            vec![a, b],
            "no sets assigned yet: everything"
        );
        set_property(&mut doc, a, &format!("stage:{sd}"), "yes", 0).unwrap();
        set_property(&mut doc, b, &format!("stage:{cd}"), "yes", 0).unwrap();
        assert_eq!(stage_sheets(&doc, Some(sd)), vec![a]);
        assert_eq!(stage_sheets(&doc, Some(cd)), vec![b]);
        let issue = create_issuance(&mut doc, "SD Review Set", "2026-09-24", vec![a]).unwrap();
        assert_eq!(
            doc.data(issue).unwrap().name(),
            "SD Review Set (2026-09-24)"
        );
        assert_eq!(
            sheet_issues(&doc, a),
            vec![(
                "SD Review Set".to_string(),
                "2026-09-24".to_string(),
                "SD".to_string()
            )]
        );
        assert!(sheet_issues(&doc, b).is_empty());
        assert!(create_issuance(&mut doc, "Empty", "2026-09-24", vec![]).is_err());
    }

    #[test]
    fn linking_to_rufplan_is_undoable() {
        let mut doc = seeded();
        let link = RufplanLink {
            id: "p1".into(),
            name: "House".into(),
            slug: "house".into(),
        };
        link_rufplan(&mut doc, Some(link.clone())).unwrap();
        assert_eq!(rufplan_link(&doc), Some(link));
        doc.undo().unwrap();
        assert_eq!(rufplan_link(&doc), None);
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
