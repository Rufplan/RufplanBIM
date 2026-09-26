//! Deliverable sheet sets (ADR-032): the drawing sets US practice issues in each design
//! phase, by building type. Sheets are numbered to the US National CAD Standard (G-001,
//! A-101…), tagged with the design stages whose sets include them, and filled from the model:
//! plans, reflected ceiling plans, elevations, sections, callouts, interior elevations and
//! schedules are placed and scaled to fit. Drawings the model doesn't make yet (details, wall
//! sections) and consultants' sheets are titled placeholders, so each set's index is complete.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use studio_core::{
    ops, Category, CoreError, CoreResult, Document, ElementData, ElementId, ScheduleKind,
    SheetSize, ViewKind,
};
use studio_geom::Pt;
use ts_rs::TS;

use crate::sheet::{margins, title_block_width, viewport_items};

/// The building types sets are tailored to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum BuildingType {
    SingleFamily,
    Duplex,
    Townhouses,
    GardenApartments,
    MidRiseApartments,
    MixedUse,
    Hotel,
}

impl BuildingType {
    pub const ALL: [BuildingType; 7] = [
        BuildingType::SingleFamily,
        BuildingType::Duplex,
        BuildingType::Townhouses,
        BuildingType::GardenApartments,
        BuildingType::MidRiseApartments,
        BuildingType::MixedUse,
        BuildingType::Hotel,
    ];
    pub fn label(self) -> &'static str {
        match self {
            BuildingType::SingleFamily => "Single-family house",
            BuildingType::Duplex => "Duplex",
            BuildingType::Townhouses => "Townhouses",
            BuildingType::GardenApartments => "Garden-style apartments",
            BuildingType::MidRiseApartments => "Mid-rise apartments",
            BuildingType::MixedUse => "Mixed-use",
            BuildingType::Hotel => "Hotel",
        }
    }
    /// Built under the International Residential Code (one- and two-family dwellings and
    /// townhouses); everything else is IBC.
    fn irc(self) -> bool {
        matches!(
            self,
            BuildingType::SingleFamily | BuildingType::Duplex | BuildingType::Townhouses
        )
    }
    /// Plans at 1/4" for houses, 1/8" for larger buildings.
    fn plan_scale(self) -> u32 {
        if self.irc() {
            48
        } else {
            96
        }
    }
}

/// The design stages, by abbreviation, in order (ADR-010).
pub const PHASES: [&str; 6] = ["PD", "SD", "DD", "CD", "BN", "CA"];
const ALL: &[&str] = &PHASES;
const SD_ON: &[&str] = &["SD", "DD", "CD", "BN", "CA"];
const DD_ON: &[&str] = &["DD", "CD", "BN", "CA"];
const CD_ON: &[&str] = &["CD", "BN", "CA"];
const EARLY: &[&str] = &["PD", "SD"];
const SD_ONLY: &[&str] = &["SD"];

/// The deliverables a phase issues, as Rufplan catalog (id, name) (ADR-016).
pub fn deliverables_of(phase: &str) -> &'static [(&'static str, &'static str)] {
    match phase {
        "PD" => &[("pd-program", "Program & Site Analysis")],
        "SD" => &[("sd100", "100% Schematic Design")],
        "DD" => &[("dd100", "100% Design Development")],
        "CD" => &[
            ("cd100", "100% Construction Documents"),
            ("permit", "Permit Set"),
        ],
        "BN" => &[("bid", "Bid Set")],
        "CA" => &[("ifc", "IFC — Issued for Construction")],
        _ => &[],
    }
}

/// NCS disciplines, in sheet-index order (as `ops::sheet_order`).
const DISCIPLINES: &[(&str, &str)] = &[
    ("G", "General"),
    ("C", "Civil"),
    ("L", "Landscape"),
    ("S", "Structural"),
    ("A", "Architectural"),
    ("I", "Interiors"),
    ("F", "Fire Protection"),
    ("P", "Plumbing"),
    ("M", "Mechanical"),
    ("E", "Electrical"),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetOptions {
    pub building_type: BuildingType,
    /// Stage abbreviations to build sets for (PD, SD, DD, CD, BN, CA).
    pub phases: Vec<String>,
    pub size: SheetSize,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlannedSheet {
    pub number: String,
    pub name: String,
    pub discipline: String,
    /// What goes on it: the views and their scale, or who provides it.
    pub contents: String,
    pub placeholder: bool,
    /// Stage abbreviations whose sets include it.
    pub phases: Vec<String>,
    /// A sheet with this number is already in the project.
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlannedDeliverable {
    pub phase: String,
    pub stage: String,
    /// Rufplan catalog id (sd100, permit, bid…).
    pub id: String,
    pub name: String,
    pub sheets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SetPlan {
    pub deliverables: Vec<PlannedDeliverable>,
    pub sheets: Vec<PlannedSheet>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SetReport {
    pub created: usize,
    pub updated: usize,
    pub views: usize,
    pub sections: usize,
    pub placeholders: usize,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------------------

/// The drawing area of a sheet, paper mm: inside the border, left of the title block.
#[derive(Debug, Clone, Copy)]
struct Area {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

fn area(size: SheetSize) -> Area {
    let (w, h) = size.mm();
    let (left, other) = margins(size);
    Area {
        x0: left + 6.0,
        y0: other + 6.0,
        x1: w - other - title_block_width(size) - 6.0,
        y1: h - other - 6.0,
    }
}

impl Area {
    fn center(&self) -> Pt {
        Pt::new((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }
}

/// Space between views, and under each for its title.
const GAP: f64 = 18.0;
const TITLE: f64 = 16.0;

/// Drawing scales to try, largest drawing first (1/4" → 1/16").
const ARCH_SCALES: &[u32] = &[24, 32, 48, 64, 96, 192];
/// Site plans: 1/8", then engineering scales.
const SITE_SCALES: &[u32] = &[96, 120, 192, 240, 360, 480, 600];

/// Packs boxes (paper w, h) in rows, centred in the area; their centres, or None.
fn pack(boxes: &[(f64, f64)], a: &Area) -> Option<Vec<Pt>> {
    let (aw, ah) = (a.x1 - a.x0, a.y1 - a.y0);
    let mut rows: Vec<Vec<usize>> = vec![];
    let mut row_w = 0.0;
    for (i, b) in boxes.iter().enumerate() {
        if b.0 > aw || b.1 + TITLE > ah {
            return None;
        }
        match rows.last_mut() {
            Some(r) if row_w + GAP + b.0 <= aw => {
                r.push(i);
                row_w += GAP + b.0;
            }
            _ => {
                rows.push(vec![i]);
                row_w = b.0;
            }
        }
    }
    let heights: Vec<f64> = rows
        .iter()
        .map(|r| r.iter().map(|i| boxes[*i].1).fold(0.0, f64::max) + TITLE)
        .collect();
    let total = heights.iter().sum::<f64>() + GAP * (rows.len().max(1) - 1) as f64;
    if total > ah {
        return None;
    }
    let mut out = vec![Pt::default(); boxes.len()];
    let mut y = a.y1 - (ah - total) / 2.0;
    for (r, h) in rows.iter().zip(&heights) {
        let rw = r.iter().map(|i| boxes[*i].0).sum::<f64>() + GAP * (r.len() - 1) as f64;
        let mut x = a.x0 + (aw - rw) / 2.0;
        for i in r {
            let (w, bh) = boxes[*i];
            out[*i] = Pt::new(x + w / 2.0, y - bh / 2.0);
            x += w + GAP;
        }
        y -= h + GAP;
    }
    Some(out)
}

/// The largest scale, no larger than `preferred`, at which the views (model mm) fit.
fn fit(sizes: &[(f64, f64)], preferred: u32, scales: &[u32], a: &Area) -> Option<(u32, Vec<Pt>)> {
    scales.iter().filter(|s| **s >= preferred).find_map(|s| {
        let d = f64::from(*s);
        let boxes: Vec<(f64, f64)> = sizes.iter().map(|(w, h)| (w / d, h / d)).collect();
        pack(&boxes, a).map(|c| (*s, c))
    })
}

/// A group of views on one sheet at one scale, with their centres.
#[derive(Debug, Clone)]
struct Placed {
    views: Vec<(ElementId, Pt)>,
    scale: u32,
    too_big: bool,
}

/// Views flowed onto as few sheets as keep them at `preferred` scale, or one scale smaller
/// when that puts two or more on a sheet instead of one; a view too big for either gets a
/// sheet of its own at a smaller scale.
fn flow(
    views: &[(ElementId, (f64, f64))],
    preferred: u32,
    scales: &[u32],
    a: &Area,
) -> Vec<Placed> {
    let mut out = vec![];
    let mut rest = views;
    while !rest.is_empty() {
        let at = |m: usize, s: u32| {
            let sizes: Vec<(f64, f64)> = rest[..m].iter().map(|v| v.1).collect();
            fit(&sizes, s, &[s], a)
        };
        let most = |s: u32| {
            (1..=rest.len())
                .rev()
                .find_map(|m| at(m, s).map(|(s, c)| (m, s, c, false)))
        };
        let next = scales.iter().copied().find(|s| *s > preferred);
        let taken = match (most(preferred), next) {
            (Some(p), Some(n)) if p.0 == 1 && rest.len() > 1 => {
                most(n).filter(|q| q.0 > 1).or(Some(p))
            }
            (Some(p), _) => Some(p),
            (None, Some(n)) => most(n).filter(|q| q.0 > 1),
            (None, None) => None,
        };
        let (m, scale, centres, too_big) =
            taken.unwrap_or_else(|| match fit(&[rest[0].1], preferred, scales, a) {
                Some((s, c)) => (1, s, c, false),
                None => (1, *scales.last().unwrap_or(&192), vec![a.center()], true),
            });
        out.push(Placed {
            views: rest[..m]
                .iter()
                .zip(centres)
                .map(|(v, c)| (v.0, c))
                .collect(),
            scale,
            too_big,
        });
        rest = &rest[m..];
    }
    out
}

/// Schedules (paper size already) flowed onto sheets.
fn flow_schedules(tables: &[(ElementId, (f64, f64))], a: &Area) -> Vec<Vec<(ElementId, Pt)>> {
    let mut out = vec![];
    let mut rest = tables;
    while !rest.is_empty() {
        let m = (1..=rest.len())
            .rev()
            .find(|m| pack(&rest[..*m].iter().map(|t| t.1).collect::<Vec<_>>(), a).is_some())
            .unwrap_or(1);
        let boxes: Vec<(f64, f64)> = rest[..m].iter().map(|t| t.1).collect();
        let centres = pack(&boxes, a).unwrap_or_else(|| vec![a.center(); m]);
        out.push(
            rest[..m]
                .iter()
                .zip(centres)
                .map(|(t, c)| (t.0, c))
                .collect(),
        );
        rest = &rest[m..];
    }
    out
}

// ---------------------------------------------------------------------------------------
// The sets
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Content {
    Cover {
        index: Option<ElementId>,
    },
    Views(Placed),
    Schedules(Vec<(ElementId, Pt)>),
    /// Who provides the sheet, and a line on what it holds.
    Placeholder {
        by: String,
        note: String,
    },
}

#[derive(Debug, Clone)]
struct Draft {
    number: String,
    name: String,
    content: Content,
    phases: Vec<&'static str>,
}

fn num(d: &str, n: usize) -> String {
    format!("{d}-{n:03}")
}

fn view_of(doc: &Document, want: impl Fn(&ViewKind, &ElementData) -> bool) -> Option<ElementId> {
    doc.of(Category::View)
        .find(|e| match &e.data {
            ElementData::View { kind, .. } => want(kind, &e.data),
            _ => false,
        })
        .map(|e| e.id)
}

fn plain(data: &ElementData) -> bool {
    matches!(
        data,
        ElementData::View {
            callout_of: None,
            site: false,
            ..
        }
    )
}

fn view_name(doc: &Document, id: ElementId) -> String {
    doc.data(id).map(|d| d.name()).unwrap_or_default()
}

/// A view's drawn extent, model mm.
fn model_size(doc: &Document, view: ElementId) -> Option<(f64, f64)> {
    let dl = studio_views::display_list(doc, view)?;
    let [x0, y0, x1, y1] = dl.bounds;
    Some((x1 - x0, y1 - y0))
}

/// A schedule's printed size, paper mm.
fn table_size(doc: &Document, view: ElementId) -> Option<(f64, f64)> {
    viewport_items(doc, ElementId::new(), view, Pt::default()).map(|(_, w, h, _, _)| (w, h))
}

fn schedule(doc: &Document, kind: ScheduleKind) -> Option<ElementId> {
    view_of(
        doc,
        |k, _| matches!(k, ViewKind::Schedule { kind: s } if *s == kind),
    )
}

/// Levels that have walls (all of them, in a model with none), and the level above the top
/// one, which takes the roof plan.
/// A level and its name.
type Named = (ElementId, String);

fn plan_levels(doc: &Document) -> (Vec<Named>, Option<Named>) {
    let levels = doc.levels();
    let walled: BTreeSet<ElementId> = doc
        .of(Category::Wall)
        .filter_map(|e| e.data.level())
        .collect();
    let floors: Vec<(ElementId, String)> = if walled.is_empty() {
        levels.iter().map(|l| (l.0, l.1.clone())).collect()
    } else {
        levels
            .iter()
            .filter(|l| walled.contains(&l.0))
            .map(|l| (l.0, l.1.clone()))
            .collect()
    };
    let top = floors
        .last()
        .and_then(|f| levels.iter().position(|l| l.0 == f.0));
    let roof = match (walled.is_empty(), top) {
        (false, Some(i)) => levels.get(i + 1).map(|l| (l.0, l.1.clone())),
        _ => None,
    };
    (floors, roof)
}

/// Floors grouped for consultants' plans: each floor up to four, else the ground floor,
/// the typical floors between, and the top floor.
fn floor_groups(floors: &[(ElementId, String)]) -> Vec<String> {
    let names: Vec<String> = floors.iter().map(|f| f.1.clone()).collect();
    if names.len() <= 4 {
        return names;
    }
    let (a, b) = (&names[1], &names[names.len() - 2]);
    let number = |s: &str| s.strip_prefix("Level ").map(str::to_owned);
    let typical = match (number(a), number(b)) {
        (Some(x), Some(y)) => format!("Typical Levels {x}–{y}"),
        _ => format!("Typical Floors ({a} – {b})"),
    };
    vec![names[0].clone(), typical, names[names.len() - 1].clone()]
}

fn placeholder(by: &str, note: &str) -> Content {
    Content::Placeholder {
        by: by.into(),
        note: note.into(),
    }
}

/// Every sheet the building type's sets can hold, before choosing phases.
fn drafts(
    doc: &Document,
    t: BuildingType,
    size: SheetSize,
    warnings: &mut Vec<String>,
) -> Vec<Draft> {
    let a = area(size);
    let mut v: Vec<Draft> = vec![];
    let mut add =
        |number: String, name: &str, content: Content, phases: &'static [&'static str]| {
            v.push(Draft {
                number,
                name: name.into(),
                content,
                phases: phases.to_vec(),
            })
        };
    let irc = t.irc();
    let multifamily = matches!(
        t,
        BuildingType::GardenApartments | BuildingType::MidRiseApartments | BuildingType::MixedUse
    );
    let (floors, roof) = plan_levels(doc);
    let groups = floor_groups(&floors);
    let pref = t.plan_scale();

    // General.
    add(
        num("G", 1),
        "Cover Sheet & Sheet Index",
        Content::Cover {
            index: schedule(doc, ScheduleKind::Sheets),
        },
        ALL,
    );
    if irc {
        add(num("G", 2), "Code Summary (IRC)", placeholder("the architect", "Code edition, occupancy, construction type, fire separation, egress windows, smoke alarms."), DD_ON);
    } else {
        add(num("G", 2), "Code Analysis (IBC)", placeholder("the architect", "Occupancy, construction type, allowable height and area, fire-resistance ratings, occupant loads and egress."), DD_ON);
        add(num("G", 3), "Life Safety Plans", placeholder("the architect", "Exits, exit access travel distances, common path, fire-rated walls and doors, per floor."), CD_ON);
        let access = if multifamily {
            "Fair Housing Act and ICC A117.1 Type A and Type B unit plans and details."
        } else {
            "ADA and ICC A117.1 accessible guest rooms, routes, clearances and details."
        };
        add(
            num("G", 4),
            "Accessibility Plans & Details",
            placeholder("the architect", access),
            CD_ON,
        );
    }
    add(
        num("G", 5),
        "Energy Code Compliance",
        placeholder(
            "the architect",
            if irc {
                "IECC residential compliance: REScheck, envelope U-values, air sealing."
            } else {
                "IECC commercial or ASHRAE 90.1 compliance: COMcheck envelope and lighting."
            },
        ),
        CD_ON,
    );

    // Civil and landscape.
    let civil = "the civil engineer";
    if matches!(t, BuildingType::SingleFamily | BuildingType::Duplex) {
        add(
            num("C", 101),
            "Grading & Drainage Plan",
            placeholder(
                civil,
                "Finish grades, drainage away from the foundation, erosion control.",
            ),
            CD_ON,
        );
    } else {
        add(
            num("C", 1),
            "Civil General Notes",
            placeholder(civil, "Notes, legend and abbreviations."),
            CD_ON,
        );
        add(
            num("C", 101),
            "Site & Grading Plan",
            placeholder(civil, "Site layout, grading, drainage and stormwater."),
            DD_ON,
        );
        add(
            num("C", 201),
            "Utility Plan",
            placeholder(
                civil,
                "Water, sewer, storm, fire service and dry utilities.",
            ),
            CD_ON,
        );
        add(
            num("C", 301),
            "Erosion & Sediment Control Plan",
            placeholder(civil, "SWPPP measures and details."),
            CD_ON,
        );
        let land = "the landscape architect";
        add(
            num("L", 101),
            "Landscape Plan",
            placeholder(land, "Planting, hardscape and irrigation layout."),
            DD_ON,
        );
        add(
            num("L", 501),
            "Landscape Details",
            placeholder(land, "Planting and hardscape details."),
            CD_ON,
        );
    }

    // Structural.
    let se = "the structural engineer";
    add(
        num("S", 1),
        "Structural General Notes",
        placeholder(
            se,
            "Design criteria, loads, materials and special inspections.",
        ),
        CD_ON,
    );
    add(
        num("S", 101),
        "Foundation Plan",
        placeholder(se, "Footings, slab, hold-downs and reinforcing."),
        DD_ON,
    );
    let mut s = 102;
    for g in groups.iter().skip(1) {
        add(
            num("S", s),
            &format!("{g} Framing Plan"),
            placeholder(se, "Floor framing, beams, shear walls and headers."),
            DD_ON,
        );
        s += 1;
    }
    add(
        num("S", s),
        "Roof Framing Plan",
        placeholder(se, "Rafters or trusses, beams and uplift connections."),
        DD_ON,
    );
    add(
        num("S", 501),
        "Structural Details",
        placeholder(se, "Foundation, framing and connection details."),
        CD_ON,
    );

    // Architectural.
    if let Some(rooms) = schedule(doc, ScheduleKind::Rooms) {
        let p = table_size(doc, rooms)
            .and_then(|b| pack(&[b], &a))
            .unwrap_or_else(|| vec![a.center()]);
        add(
            num("A", 1),
            "Program & Area Summary",
            Content::Schedules(vec![(rooms, p[0])]),
            EARLY,
        );
    }
    match view_of(doc, |_, d| matches!(d, ElementData::View { site: true, .. })) {
        Some(site) => {
            let sized: Vec<_> = model_size(doc, site).map(|z| (site, z)).into_iter().collect();
            let placed = flow(&sized, 120, SITE_SCALES, &a);
            if let Some(p) = placed.into_iter().next() {
                add(num("A", 100), "Architectural Site Plan", Content::Views(p), ALL);
            }
        }
        None => add(
            num("A", 100),
            "Architectural Site Plan",
            placeholder("the architect", "Find the lot on the Site tab, then create the sheets again to draw the site plan here."),
            ALL,
        ),
    }
    // One floor plan a sheet, then the roof plan.
    let mut n = 101;
    let plan_view = |level: ElementId| {
        view_of(doc, |k, d| {
            plain(d) && matches!(k, ViewKind::FloorPlan { level: l } if *l == level)
        })
    };
    let mut one =
        |v: ElementId,
         name: String,
         n: &mut usize,
         disc_phases: &'static [&'static str],
         out: &mut Vec<(String, String, Content, &'static [&'static str])>| {
            let Some(z) = model_size(doc, v) else { return };
            if let Some(p) = flow(&[(v, z)], pref, ARCH_SCALES, &a).into_iter().next() {
                if p.too_big {
                    warnings.push(format!(
                        "{name} is too large for one sheet at 1/16\"; it's centred on {}",
                        num("A", *n)
                    ));
                }
                out.push((num("A", *n), name, Content::Views(p), disc_phases));
                *n += 1;
            }
        };
    let mut arch: Vec<(String, String, Content, &'static [&'static str])> = vec![];
    for (level, name) in &floors {
        if let Some(v) = plan_view(*level) {
            one(v, format!("{name} Floor Plan"), &mut n, SD_ON, &mut arch);
        }
    }
    if let Some((level, _)) = &roof {
        if let Some(v) = plan_view(*level) {
            one(v, "Roof Plan".into(), &mut n, SD_ON, &mut arch);
        }
    }
    let mut n = 151;
    for (level, name) in &floors {
        let rcp = view_of(doc, |k, d| {
            plain(d) && matches!(k, ViewKind::CeilingPlan { level: l } if *l == *level)
        });
        if let Some(v) = rcp {
            one(
                v,
                format!("{name} Reflected Ceiling Plan"),
                &mut n,
                DD_ON,
                &mut arch,
            );
        }
    }
    for (number, name, content, phases) in arch {
        add(number, &name, content, phases);
    }
    // Elevations, sections, then enlarged views, flowed onto as many sheets as they need.
    let sized = |kinds: &dyn Fn(&ViewKind, &ElementData) -> bool| -> Vec<(ElementId, (f64, f64))> {
        doc.of(Category::View)
            .filter(|e| matches!(&e.data, ElementData::View { kind, .. } if kinds(kind, &e.data)))
            .filter_map(|e| model_size(doc, e.id).map(|z| (e.id, z)))
            .collect()
    };
    let mut elevations = sized(&|k, d| plain(d) && matches!(k, ViewKind::Elevation { .. }));
    // South, north, east, west, the order elevation sheets read in.
    let order = |id: ElementId| match doc.data(id) {
        Ok(ElementData::View { name, .. }) => ["South", "North", "East", "West"]
            .iter()
            .position(|n| name == n)
            .unwrap_or(4),
        _ => 4,
    };
    elevations.sort_by_key(|v| order(v.0));
    for (i, p) in flow(&elevations, pref, ARCH_SCALES, &a)
        .into_iter()
        .enumerate()
    {
        add(
            num("A", 201 + i),
            "Exterior Elevations",
            Content::Views(p),
            SD_ON,
        );
    }
    let sections = sized(&|k, d| plain(d) && matches!(k, ViewKind::Section { .. }));
    for (i, p) in flow(&sections, pref, ARCH_SCALES, &a)
        .into_iter()
        .enumerate()
    {
        add(
            num("A", 301 + i),
            "Building Sections",
            Content::Views(p),
            SD_ON,
        );
    }
    add(num("A", 311), "Wall Sections", placeholder("the architect", "Each wall type cut from footing to roof at 3/4\" = 1'-0\": layers, flashing, insulation."), DD_ON);
    let enlarged = match t {
        BuildingType::SingleFamily | BuildingType::Duplex => "Enlarged Kitchen & Bath Plans",
        BuildingType::Townhouses => "Enlarged Unit Plans",
        BuildingType::GardenApartments | BuildingType::MidRiseApartments => "Enlarged Unit Plans",
        BuildingType::MixedUse => "Enlarged Unit & Retail Plans",
        BuildingType::Hotel => "Typical Guest Room Plans",
    };
    let callouts: Vec<(ElementId, (f64, f64))> = doc
        .of(Category::View)
        .filter(|e| {
            matches!(
                &e.data,
                ElementData::View {
                    callout_of: Some(_),
                    kind: ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. },
                    ..
                }
            )
        })
        .filter_map(|e| model_size(doc, e.id).map(|z| (e.id, z)))
        .collect();
    if callouts.is_empty() {
        add(
            num("A", 401),
            enlarged,
            placeholder(
                "the architect",
                "Enlarged plans at 1/4\" = 1'-0\": make callouts of the rooms to show them here.",
            ),
            DD_ON,
        );
    } else {
        for (i, p) in flow(&callouts, 24, ARCH_SCALES, &a).into_iter().enumerate() {
            add(num("A", 401 + i), enlarged, Content::Views(p), DD_ON);
        }
    }
    if !irc {
        add(
            num("A", 421),
            "Enlarged Stair Plans & Sections",
            placeholder(
                "the architect",
                "Stairs at 1/4\" = 1'-0\": treads, risers, handrails and guards.",
            ),
            CD_ON,
        );
    }
    let interior: Vec<(ElementId, (f64, f64))> = doc
        .of(Category::View)
        .filter(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::MarkerElevation { marker, .. },
                ..
            } => matches!(
                doc.data(*marker),
                Ok(ElementData::ElevationMarker { interior: true, .. })
            ),
            _ => false,
        })
        .filter_map(|e| model_size(doc, e.id).map(|z| (e.id, z)))
        .collect();
    if interior.is_empty() {
        add(
            num("A", 451),
            "Interior Elevations",
            placeholder(
                "the architect",
                "Place interior elevation marks in kitchens and baths to show them here.",
            ),
            CD_ON,
        );
    } else {
        for (i, p) in flow(&interior, 48, ARCH_SCALES, &a).into_iter().enumerate() {
            add(
                num("A", 451 + i),
                "Interior Elevations",
                Content::Views(p),
                CD_ON,
            );
        }
    }
    add(
        num("A", 501),
        "Architectural Details",
        placeholder(
            "the architect",
            "Envelope, opening, roof and interior details.",
        ),
        CD_ON,
    );
    let tables = |kinds: &[ScheduleKind]| -> Vec<(ElementId, (f64, f64))> {
        kinds
            .iter()
            .filter_map(|k| schedule(doc, *k))
            .filter_map(|v| table_size(doc, v).map(|z| (v, z)))
            .collect()
    };
    let mut n = 601;
    for group in flow_schedules(&tables(&[ScheduleKind::Doors, ScheduleKind::Windows]), &a) {
        add(
            num("A", n),
            "Door & Window Schedules",
            Content::Schedules(group),
            DD_ON,
        );
        n += 1;
    }
    for group in flow_schedules(&tables(&[ScheduleKind::Rooms]), &a) {
        add(
            num("A", n),
            "Room Finish Schedule",
            Content::Schedules(group),
            DD_ON,
        );
        n += 1;
    }
    add(
        num("A", 901),
        "3D Views & Renderings",
        placeholder(
            "the architect",
            "Render camera views on the Rendering tab for the presentation.",
        ),
        SD_ONLY,
    );

    // Interiors, for hotels.
    if t == BuildingType::Hotel {
        let id = "the interior designer";
        add(
            num("I", 101),
            "Interior Finish Plans",
            placeholder(
                id,
                "Floor, wall and ceiling finishes of public spaces and guest rooms.",
            ),
            CD_ON,
        );
        add(
            num("I", 601),
            "Interior Finish Schedule",
            placeholder(id, "Finishes, FF&E and specifications."),
            CD_ON,
        );
    }

    // Fire protection, plumbing, mechanical and electrical.
    if !irc {
        let fp = "the fire protection engineer";
        add(
            num("F", 1),
            "Fire Protection Notes",
            placeholder(
                fp,
                if t == BuildingType::GardenApartments {
                    "NFPA 13R sprinkler system (deferred submittal)."
                } else {
                    "NFPA 13 sprinkler and standpipe systems (deferred submittal)."
                },
            ),
            CD_ON,
        );
        for (i, g) in groups.iter().enumerate() {
            add(
                num("F", 101 + i),
                &format!("{g} Fire Protection Plan"),
                placeholder(fp, "Sprinkler heads, mains and risers."),
                CD_ON,
            );
        }
    }
    let mep = "the MEP engineer";
    for (d, label, what) in [
        ("P", "Plumbing", "Fixtures, waste, vent and water piping."),
        (
            "M",
            "Mechanical",
            "HVAC equipment, ductwork and ventilation.",
        ),
        ("E", "Electrical", "Power, lighting, panels and devices."),
    ] {
        if irc {
            add(
                num(d, 101),
                &format!("{label} Plans"),
                placeholder(mep, what),
                CD_ON,
            );
            continue;
        }
        add(
            num(d, 1),
            &format!("{label} Notes & Schedules"),
            placeholder(mep, "Notes, legend and equipment schedules."),
            CD_ON,
        );
        for (i, g) in groups.iter().enumerate() {
            add(
                num(d, 101 + i),
                &format!("{g} {label} Plan"),
                placeholder(mep, what),
                DD_ON,
            );
        }
        if d == "M" {
            add(
                num(d, 101 + groups.len()),
                "Roof Mechanical Plan",
                placeholder(mep, "Rooftop equipment, curbs and exhausts."),
                CD_ON,
            );
        }
    }
    v
}

/// The project's design stages by abbreviation, as (id, name).
fn stage_map(doc: &Document) -> BTreeMap<String, (ElementId, String)> {
    ops::stages(doc)
        .into_iter()
        .map(|(id, name, abbr)| (abbr, (id, name)))
        .collect()
}

/// The drafts in the chosen phases, in sheet-index order.
fn chosen(doc: &Document, o: &SetOptions, warnings: &mut Vec<String>) -> Vec<Draft> {
    let stages = stage_map(doc);
    let mut phases: Vec<&'static str> = vec![];
    for p in PHASES {
        if !o.phases.iter().any(|x| x == p) {
            continue;
        }
        if stages.contains_key(p) {
            phases.push(p);
        } else {
            warnings.push(format!(
                "The project has no {p} design stage, so its set is skipped (add it in Project Information)"
            ));
        }
    }
    let mut out: Vec<Draft> = drafts(doc, o.building_type, o.size, warnings)
        .into_iter()
        .filter_map(|mut d| {
            d.phases.retain(|p| phases.contains(p));
            (!d.phases.is_empty()).then_some(d)
        })
        .collect();
    out.sort_by(|a, b| ops::sheet_cmp(&a.number, &b.number));
    out
}

fn contents_label(doc: &Document, c: &Content) -> (String, bool) {
    match c {
        Content::Cover { .. } => ("Project title and the sheet index".into(), false),
        Content::Views(p) => (
            format!(
                "{} at {}",
                p.views
                    .iter()
                    .map(|v| view_name(doc, v.0))
                    .collect::<Vec<_>>()
                    .join(", "),
                ops::scale_label(p.scale)
            ),
            false,
        ),
        Content::Schedules(s) => (
            s.iter()
                .map(|v| view_name(doc, v.0))
                .collect::<Vec<_>>()
                .join(", "),
            false,
        ),
        Content::Placeholder { by, note } => (format!("Placeholder by {by}: {note}"), true),
    }
}

fn discipline_of(number: &str) -> String {
    let d = number.split_once('-').map_or("", |x| x.0);
    DISCIPLINES
        .iter()
        .find(|x| x.0 == d)
        .map_or("Other", |x| x.1)
        .into()
}

fn existing_sheets(doc: &Document) -> BTreeMap<String, ElementId> {
    ops::sheets(doc).into_iter().map(|s| (s.1, s.0)).collect()
}

/// What creating the sets would make, for the preview. Sections the project lacks are
/// planned as they would be created.
pub fn plan(doc: &Document, o: &SetOptions) -> SetPlan {
    let mut preview = doc.clone();
    let mut warnings = vec![];
    if let Err(e) = ensure_sections(&mut preview) {
        warnings.push(format!("Building sections: {e}"));
    }
    let doc = &preview;
    let drafts = chosen(doc, o, &mut warnings);
    let existing = existing_sheets(doc);
    let stages = stage_map(doc);
    let sheets: Vec<PlannedSheet> = drafts
        .iter()
        .map(|d| {
            let (contents, placeholder) = contents_label(doc, &d.content);
            PlannedSheet {
                number: d.number.clone(),
                name: d.name.clone(),
                discipline: discipline_of(&d.number),
                contents,
                placeholder,
                phases: d.phases.iter().map(|p| (*p).to_owned()).collect(),
                exists: existing.contains_key(&d.number),
            }
        })
        .collect();
    let mut deliverables = vec![];
    for p in PHASES {
        let Some((_, stage)) = stages.get(p) else {
            continue;
        };
        let in_set: Vec<String> = drafts
            .iter()
            .filter(|d| d.phases.contains(&p))
            .map(|d| d.number.clone())
            .collect();
        if in_set.is_empty() {
            continue;
        }
        for (id, name) in deliverables_of(p) {
            deliverables.push(PlannedDeliverable {
                phase: p.into(),
                stage: stage.clone(),
                id: (*id).into(),
                name: (*name).into(),
                sheets: in_set.clone(),
            });
        }
    }
    SetPlan {
        deliverables,
        sheets,
        warnings,
    }
}

/// Two building sections, when the project has walls and no sections: a longitudinal one
/// looking north through the middle, and a transverse one looking east. Returns how many
/// were made.
fn ensure_sections(doc: &mut Document) -> CoreResult<usize> {
    let has = doc.of(Category::View).any(|e| {
        matches!(
            &e.data,
            ElementData::View {
                kind: ViewKind::Section { .. },
                callout_of: None,
                ..
            }
        )
    });
    if has {
        return Ok(0);
    }
    let model = studio_regen::regenerate(doc);
    let pts: Vec<Pt> = model
        .walls
        .iter()
        .flat_map(|w| w.footprint.outer.iter().copied())
        .collect();
    let Some((lo, hi)) = studio_geom::bounds_of(&pts) else {
        return Ok(0);
    };
    let pad = 3.0 * studio_core::units::MM_PER_FT;
    let c = Pt::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let long = ops::create_section(doc, Pt::new(lo.x - pad, c.y), Pt::new(hi.x + pad, c.y))?;
    let cross = ops::create_section(doc, Pt::new(c.x, hi.y + pad), Pt::new(c.x, lo.y - pad))?;
    // Deep enough to see the whole building beyond the cut.
    let (dy, dx) = (hi.y - c.y + pad, hi.x - c.x + pad);
    doc.transact("Building sections", |tx| {
        for (id, depth, name) in [
            (long, dy, "Building Section 1"),
            (cross, dx, "Building Section 2"),
        ] {
            tx.modify(id, |d| {
                if let ElementData::View {
                    kind: ViewKind::Section { depth: dd, .. },
                    name: n,
                    ..
                } = d
                {
                    *dd = depth;
                    *n = name.into();
                }
            })?;
        }
        Ok(())
    })?;
    Ok(2)
}

/// Creates (or updates) the sheets of the chosen phases' sets, one undo step.
pub fn create(doc: &mut Document, o: &SetOptions) -> CoreResult<SetReport> {
    if o.phases.is_empty() {
        return Err(CoreError::Invalid("choose at least one phase".into()));
    }
    let mark = doc.undo_depth();
    let mut report = SetReport::default();
    match create_steps(doc, o, &mut report) {
        Ok(()) => {
            doc.merge_undo(mark, "Create sheet sets");
            Ok(report)
        }
        Err(e) => {
            while doc.undo_depth() > mark {
                doc.undo()?;
            }
            Err(e)
        }
    }
}

fn create_steps(doc: &mut Document, o: &SetOptions, report: &mut SetReport) -> CoreResult<()> {
    report.sections = ensure_sections(doc)?;
    let drafts = chosen(doc, o, &mut report.warnings);
    let stages = stage_map(doc);
    let existing = existing_sheets(doc);
    let generated: BTreeSet<&str> = drafts.iter().map(|d| d.number.as_str()).collect();
    let selected: Vec<ElementId> = o
        .phases
        .iter()
        .filter_map(|p| stages.get(p.as_str()).map(|s| s.0))
        .collect();
    // Where each drawing view is placed now: (viewport, sheet number).
    let placed: BTreeMap<ElementId, (ElementId, String)> = doc
        .of(Category::Viewport)
        .filter_map(|e| match &e.data {
            ElementData::Viewport { sheet, view, .. } => {
                let number = match doc.data(*sheet) {
                    Ok(ElementData::Sheet { number, .. }) => number.clone(),
                    _ => String::new(),
                };
                Some((*view, (e.id, number)))
            }
            _ => None,
        })
        .collect();
    let has_content: BTreeSet<ElementId> = doc
        .iter()
        .filter_map(|e| match &e.data {
            ElementData::Viewport { sheet, .. } => Some(*sheet),
            ElementData::TextNote { view, .. } => Some(*view),
            _ => None,
        })
        .collect();
    let (project, address) = match ops::project_info(doc).and_then(|i| doc.data(i).ok()) {
        Some(ElementData::ProjectInfo { name, address, .. }) => (name.clone(), address.clone()),
        _ => (String::new(), String::new()),
    };
    let mut cover_index: Option<ElementId> = None;
    let a = area(o.size);
    let size = o.size;
    let t = o.building_type;

    doc.transact("Create sheet sets", |tx| {
        for d in &drafts {
            let flags: Vec<ElementId> = d
                .phases
                .iter()
                .filter_map(|p| stages.get(*p).map(|s| s.0))
                .collect();
            let (sheet, fill) = match existing.get(&d.number) {
                Some(id) => {
                    let id = *id;
                    tx.modify(id, |data| {
                        if let ElementData::Sheet { name, stages, .. } = data {
                            *name = d.name.clone();
                            // The chosen phases follow the plan; others stay as they were.
                            stages.retain(|s| !selected.contains(s));
                            stages.extend(flags.iter().copied());
                        }
                    })?;
                    report.updated += 1;
                    (id, !has_content.contains(&id))
                }
                None => {
                    let id = tx.insert(ElementData::Sheet {
                        number: d.number.clone(),
                        name: d.name.clone(),
                        size,
                        stages: flags.clone(),
                    });
                    report.created += 1;
                    (id, true)
                }
            };
            if !fill {
                continue;
            }
            let note = |tx: &mut studio_core::Tx<'_>, at: Pt, text: String, size: f64| {
                tx.insert(ElementData::TextNote {
                    view: sheet,
                    at,
                    text,
                    size,
                });
            };
            match &d.content {
                Content::Cover { index } => {
                    let title = if project.trim().is_empty() {
                        "PROJECT".to_owned()
                    } else {
                        project.to_uppercase()
                    };
                    note(tx, Pt::new(a.x0 + 20.0, a.y1 - 60.0), title, 12.7);
                    let sub = [t.label().to_uppercase(), address.to_uppercase()]
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join("  ·  ");
                    note(tx, Pt::new(a.x0 + 21.0, a.y1 - 80.0), sub, 4.8);
                    if let Some(index) = index {
                        // Positioned once every sheet exists (below).
                        cover_index = Some(tx.insert(ElementData::Viewport {
                            sheet,
                            view: *index,
                            center: a.center(),
                        }));
                    }
                }
                Content::Views(p) => {
                    for (view, center) in &p.views {
                        tx.modify(*view, |data| {
                            if let ElementData::View { scale, .. } = data {
                                *scale = p.scale;
                            }
                        })?;
                        match placed.get(view) {
                            Some((vp, number)) if !generated.contains(number.as_str()) => {
                                // On a sheet outside the sets: move it here.
                                let (vp, center) = (*vp, *center);
                                tx.modify(vp, |data| {
                                    if let ElementData::Viewport {
                                        sheet: s,
                                        center: c,
                                        ..
                                    } = data
                                    {
                                        *s = sheet;
                                        *c = center;
                                    }
                                })?;
                                report.warnings.push(format!(
                                    "Moved {} from sheet {number} to {}",
                                    tx.data(*view).map(|v| v.name()).unwrap_or_default(),
                                    d.number
                                ));
                                report.views += 1;
                            }
                            Some(_) => {}
                            None => {
                                tx.insert(ElementData::Viewport {
                                    sheet,
                                    view: *view,
                                    center: *center,
                                });
                                report.views += 1;
                            }
                        }
                    }
                }
                Content::Schedules(list) => {
                    for (view, center) in list {
                        tx.insert(ElementData::Viewport {
                            sheet,
                            view: *view,
                            center: *center,
                        });
                        report.views += 1;
                    }
                }
                Content::Placeholder { by, note: what } => {
                    let c = a.center();
                    let x = (c.x - 160.0).max(a.x0 + 20.0);
                    note(tx, Pt::new(x, c.y + 30.0), d.name.to_uppercase(), 9.5);
                    note(
                        tx,
                        Pt::new(x + 1.0, c.y + 8.0),
                        format!("PLACEHOLDER — BY {}", by.to_uppercase()),
                        4.8,
                    );
                    note(tx, Pt::new(x + 1.0, c.y - 8.0), what.clone(), 3.0);
                    note(
                        tx,
                        Pt::new(x + 1.0, c.y - 20.0),
                        "Replace this sheet with the final drawings before the set is issued."
                            .into(),
                        3.0,
                    );
                    report.placeholders += 1;
                }
            }
        }
        Ok(())
    })?;

    // The sheet index lists the printing stage's set: top-align it for the largest one.
    if let (Some(vp), Some(index)) = (cover_index, schedule(doc, ScheduleKind::Sheets)) {
        let mut size = (0.0f64, 0.0f64);
        for s in &selected {
            if let Some((w, h)) = as_of_stage(doc, *s)
                .ok()
                .and_then(|c| table_size(&c, index))
            {
                size = (size.0.max(w), size.1.max(h));
            }
        }
        let center = Pt::new(a.x1 - size.0 / 2.0 - 10.0, a.y1 - size.1 / 2.0 - 10.0);
        doc.transact("Place the sheet index", |tx| {
            tx.modify(vp, |d| {
                if let ElementData::Viewport { center: c, .. } = d {
                    *c = center;
                }
            })
        })?;
    }
    // Older sheets left empty by the move leave the chosen phases' sets.
    let now_empty: Vec<(ElementId, String)> = doc
        .of(Category::Sheet)
        .filter(|e| has_content.contains(&e.id))
        .filter(|e| {
            !doc.iter().any(|x| {
                matches!(&x.data, ElementData::Viewport { sheet, .. } | ElementData::TextNote { view: sheet, .. } if *sheet == e.id)
            })
        })
        .filter_map(|e| match &e.data {
            ElementData::Sheet { number, name, .. } if !generated.contains(number.as_str()) => {
                Some((e.id, format!("{number} {name}")))
            }
            _ => None,
        })
        .collect();
    if !now_empty.is_empty() {
        doc.transact("Leave empty sheets out of the sets", |tx| {
            for (id, _) in &now_empty {
                tx.modify(*id, |d| {
                    if let ElementData::Sheet { stages, .. } = d {
                        stages.retain(|s| !selected.contains(s));
                    }
                })?;
            }
            Ok(())
        })?;
        let one = now_empty.len() == 1;
        report.warnings.push(format!(
            "{} {} left empty and out of the sets; delete {} if you don't need {}",
            now_empty
                .iter()
                .map(|x| x.1.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            if one { "is" } else { "are" },
            if one { "it" } else { "them" },
            if one { "it" } else { "them" },
        ));
    }
    Ok(())
}

/// The deliverables of each phase that has sheets, with its sheets in index order:
/// (phase, stage, Rufplan id, name, sheets).
pub fn deliverables(
    doc: &Document,
    phases: &[String],
) -> Vec<(String, ElementId, String, String, Vec<ElementId>)> {
    let stages = stage_map(doc);
    let mut out = vec![];
    for p in PHASES {
        if !phases.iter().any(|x| x == p) {
            continue;
        }
        let Some((stage, _)) = stages.get(p) else {
            continue;
        };
        let sheets: Vec<ElementId> = ops::sheets(doc)
            .into_iter()
            .filter(|(id, _, _)| {
                matches!(doc.data(*id), Ok(ElementData::Sheet { stages, .. }) if stages.contains(stage))
            })
            .map(|s| s.0)
            .collect();
        if sheets.is_empty() {
            continue;
        }
        for (id, name) in deliverables_of(p) {
            out.push((
                p.into(),
                *stage,
                (*id).into(),
                (*name).into(),
                sheets.clone(),
            ));
        }
    }
    out
}

/// A copy of the project with `stage` current, so a set prints its own stage in the title
/// block and its own sheets in the index.
pub fn as_of_stage(doc: &Document, stage: ElementId) -> CoreResult<Document> {
    let mut copy = doc.clone();
    if let Some(info) = ops::project_info(&copy) {
        copy.transact("Stage for printing", |tx| {
            tx.modify(info, |d| {
                if let ElementData::ProjectInfo { current_stage, .. } = d {
                    *current_stage = Some(stage);
                }
            })
        })?;
    }
    Ok(copy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::units::MM_PER_FT;

    fn house(stories: usize) -> Document {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        // Levels 3.. for taller buildings.
        for i in 2..stories {
            ops::create_level(&mut doc, i as f64 * 10.0 * MM_PER_FT).unwrap();
        }
        let levels = doc.levels();
        let (w, h) = (40.0 * MM_PER_FT, 30.0 * MM_PER_FT);
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ];
        for l in levels.iter().take(stories) {
            for i in 0..4 {
                ops::create_wall(&mut doc, wt, l.0, c[i], c[(i + 1) % 4]).unwrap();
            }
        }
        doc
    }

    fn opts(t: BuildingType, phases: &[&str]) -> SetOptions {
        SetOptions {
            building_type: t,
            phases: phases.iter().map(|p| (*p).to_owned()).collect(),
            size: SheetSize::ArchD,
        }
    }

    #[test]
    fn packing_centres_rows_and_refuses_what_wont_fit() {
        let a = Area {
            x0: 0.0,
            y0: 0.0,
            x1: 400.0,
            y1: 300.0,
        };
        let c = pack(&[(100.0, 50.0), (100.0, 50.0)], &a).unwrap();
        // One row 218 wide, centred: starts at 91; 66 tall with its title, centred at 150.
        assert!((c[0].x - 141.0).abs() < 1e-9 && (c[1].x - 259.0).abs() < 1e-9);
        assert!((c[0].y - (300.0 - 117.0 - 25.0)).abs() < 1e-9, "{c:?}");
        assert!(pack(&[(500.0, 10.0)], &a).is_none());
        // Three 180-wide boxes: two rows.
        let c = pack(&[(180.0, 50.0); 3], &a).unwrap();
        assert!(c[2].y < c[0].y && (c[0].y - c[1].y).abs() < 1e-9);
        // A 40' plan on ARCH D: 1/4" fits (120 mm wide), a 400' one needs 1/16".
        let d = area(SheetSize::ArchD);
        let ft = MM_PER_FT;
        assert_eq!(
            fit(&[(40.0 * ft, 30.0 * ft)], 48, ARCH_SCALES, &d)
                .unwrap()
                .0,
            48
        );
        assert_eq!(
            fit(&[(400.0 * ft, 100.0 * ft)], 48, ARCH_SCALES, &d)
                .unwrap()
                .0,
            192
        );
    }

    #[test]
    fn a_house_permit_set_follows_the_ncs() {
        let doc = house(2);
        let p = plan(&doc, &opts(BuildingType::SingleFamily, &PHASES));
        let numbers: Vec<&str> = p.sheets.iter().map(|s| s.number.as_str()).collect();
        // Discipline order, then number order.
        assert_eq!(numbers[0], "G-001");
        let pos = |n: &str| numbers.iter().position(|x| *x == n).unwrap();
        assert!(pos("G-005") < pos("C-101") && pos("C-101") < pos("S-001"));
        assert!(pos("S-501") < pos("A-001") && pos("A-901") < pos("P-101"));
        assert!(pos("P-101") < pos("M-101") && pos("M-101") < pos("E-101"));
        // Two floor plans, a roof plan isn't there (no level above Level 2), RCPs from 151.
        let name = |n: &str| p.sheets.iter().find(|s| s.number == n).unwrap();
        assert_eq!(name("A-101").name, "Level 1 Floor Plan");
        assert_eq!(name("A-102").name, "Level 2 Floor Plan");
        assert!(name("A-101").contents.contains("1/4\" = 1'-0\""));
        assert_eq!(name("A-151").name, "Level 1 Reflected Ceiling Plan");
        // A 40' x 30' house at 1/4": south and north on A-201, east and west on A-202.
        assert_eq!(name("A-201").contents, "South, North at 1/4\" = 1'-0\"");
        assert_eq!(name("A-202").contents, "East, West at 1/4\" = 1'-0\"");
        assert!(name("A-301")
            .contents
            .contains("Building Section 1, Building Section 2"));
        // IRC: no life safety or accessibility sheets, one sheet per MEP discipline, no
        // landscape.
        assert!(!numbers.contains(&"G-003") && !numbers.contains(&"L-101"));
        assert!(!numbers.iter().any(|n| n.starts_with("F-")));
        assert!(name("S-101").placeholder && !name("A-101").placeholder);
        // Phases: plans in SD on, RCPs from DD, details from CD, renderings in SD only.
        assert_eq!(name("A-101").phases, ["SD", "DD", "CD", "BN", "CA"]);
        assert_eq!(name("A-151").phases, ["DD", "CD", "BN", "CA"]);
        assert_eq!(name("A-501").phases, ["CD", "BN", "CA"]);
        assert_eq!(name("A-901").phases, ["SD"]);
        assert_eq!(name("G-001").phases, PHASES);
        // Deliverables: CD issues both 100% CDs and the permit set.
        let ids: Vec<&str> = p.deliverables.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "pd-program",
                "sd100",
                "dd100",
                "cd100",
                "permit",
                "bid",
                "ifc"
            ]
        );
        let sd = &p.deliverables[1];
        assert!(
            sd.sheets.contains(&"A-101".to_owned()) && !sd.sheets.contains(&"A-501".to_owned())
        );
    }

    #[test]
    fn taller_multifamily_groups_consultant_floors() {
        let doc = house(6);
        let p = plan(&doc, &opts(BuildingType::MidRiseApartments, &["CD"]));
        let has = |n: &str, name: &str| p.sheets.iter().any(|s| s.number == n && s.name == name);
        assert!(has("G-003", "Life Safety Plans"));
        assert!(has("G-004", "Accessibility Plans & Details"));
        assert!(has("A-106", "Level 6 Floor Plan"));
        assert!(has("M-101", "Level 1 Mechanical Plan"));
        assert!(has("M-102", "Typical Levels 2–5 Mechanical Plan"));
        assert!(has("M-103", "Level 6 Mechanical Plan"));
        assert!(has("M-104", "Roof Mechanical Plan"));
        assert!(has("F-102", "Typical Levels 2–5 Fire Protection Plan"));
        assert!(has("A-401", "Enlarged Unit Plans"));
        assert!(has("L-101", "Landscape Plan"));
        // CD only: no program or renderings sheets.
        assert!(!p
            .sheets
            .iter()
            .any(|s| s.number == "A-001" || s.number == "A-901"));
        assert!(p.sheets.iter().all(|s| s.phases == ["CD"]));
        // 1/8" plans for larger buildings.
        let a101 = p.sheets.iter().find(|s| s.number == "A-101").unwrap();
        assert!(a101.contents.contains("1/8\""), "{}", a101.contents);
    }

    #[test]
    fn creating_places_views_tags_stages_and_undoes_as_one() {
        let mut doc = house(2);
        let before = doc.undo_depth();
        let r = create(&mut doc, &opts(BuildingType::SingleFamily, &["SD", "CD"])).unwrap();
        assert_eq!(doc.undo_depth(), before + 1);
        assert_eq!(r.sections, 2);
        assert_eq!(r.updated, 0);
        assert!(r.created >= 20, "{}", r.created);
        assert!(r.placeholders >= 8);
        let sheet = |n: &str| {
            ops::sheets(&doc)
                .into_iter()
                .find(|s| s.1 == n)
                .map(|s| s.0)
        };
        let a101 = sheet("A-101").unwrap();
        let vps: Vec<ElementId> = doc
            .of(Category::Viewport)
            .filter_map(|e| match &e.data {
                ElementData::Viewport { sheet, view, .. } if *sheet == a101 => Some(*view),
                _ => None,
            })
            .collect();
        assert_eq!(vps.len(), 1);
        assert_eq!(view_name(&doc, vps[0]), "Level 1");
        // Stage flags: SD's set holds plans but not details; CD's holds both.
        let stages = stage_map(&doc);
        let (sd, cd) = (stages["SD"].0, stages["CD"].0);
        let sd_set = ops::stage_sheets(&doc, Some(sd));
        let cd_set = ops::stage_sheets(&doc, Some(cd));
        let a501 = sheet("A-501").unwrap();
        assert!(sd_set.contains(&a101) && !sd_set.contains(&a501));
        assert!(cd_set.contains(&a101) && cd_set.contains(&a501));
        assert!(!cd_set.contains(&sheet("A-901").unwrap()));
        // Running again updates rather than duplicates.
        let count = doc.count(Category::Sheet);
        let r2 = create(&mut doc, &opts(BuildingType::SingleFamily, &["SD", "CD"])).unwrap();
        assert_eq!((r2.created, r2.sections), (0, 0));
        assert_eq!(doc.count(Category::Sheet), count);
        // Undo the first creation entirely.
        doc.undo().unwrap();
        doc.undo().unwrap();
        assert_eq!(doc.count(Category::Sheet), 0);
        // Deliverables to print, each set as of its own stage.
        create(&mut doc, &opts(BuildingType::SingleFamily, &["SD", "CD"])).unwrap();
        let d = deliverables(&doc, &["SD".into(), "CD".into()]);
        assert_eq!(
            d.iter().map(|x| x.2.as_str()).collect::<Vec<_>>(),
            ["sd100", "cd100", "permit"]
        );
        assert!(d[0].4.len() < d[1].4.len());
        let copy = as_of_stage(&doc, sd).unwrap();
        assert!(matches!(
            copy.data(ops::project_info(&copy).unwrap()),
            Ok(ElementData::ProjectInfo { current_stage: Some(s), .. }) if *s == sd
        ));
    }

    #[test]
    fn views_on_older_sheets_move_into_the_set() {
        let mut doc = house(2);
        let old = ops::create_sheet(&mut doc, "Plans", SheetSize::ArchD).unwrap();
        let l1 = view_of(&doc, |k, d| {
            plain(d) && matches!(k, ViewKind::FloorPlan { .. })
        })
        .unwrap();
        ops::place_view(&mut doc, old, l1, Pt::new(200.0, 300.0)).unwrap();
        let r = create(&mut doc, &opts(BuildingType::SingleFamily, &["SD"])).unwrap();
        assert!(
            r.warnings
                .iter()
                .any(|w| w.starts_with("Moved Level 1 from sheet A1.0")),
            "{:?}",
            r.warnings
        );
        let on_old = doc
            .of(Category::Viewport)
            .any(|e| matches!(&e.data, ElementData::Viewport { sheet, .. } if *sheet == old));
        assert!(!on_old);
    }

    #[test]
    fn missing_stages_are_reported() {
        let mut doc = house(1);
        let bn = stage_map(&doc)["BN"].0;
        doc.transact("drop", |tx| tx.delete(bn).map(|_| ()))
            .unwrap();
        let p = plan(&doc, &opts(BuildingType::SingleFamily, &["CD", "BN"]));
        assert!(p.warnings.iter().any(|w| w.contains("no BN design stage")));
        assert!(p.deliverables.iter().all(|d| d.phase == "CD"));
        assert!(create(&mut doc, &opts(BuildingType::SingleFamily, &[])).is_err());
    }
}
