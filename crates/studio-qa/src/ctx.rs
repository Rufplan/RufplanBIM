//! What the checks share: the project's code basis, and the model digested once (walls
//! with their lengths, heights and layer text; openings placed in the world; rooms with
//! their rings; where views are placed).

use std::collections::{HashMap, HashSet};

use studio_core::element::{WallFunction, WallTop};
use studio_core::project::ProjectDetails;
use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::{Category, Document, ElementData, ElementId, ViewKind};
use studio_geom::{project_to_segment, Pt};
use studio_regen::Model;

use crate::Milestone;

pub struct WallInfo {
    pub id: ElementId,
    pub type_id: ElementId,
    pub type_name: String,
    pub level: ElementId,
    pub start: Pt,
    pub end: Pt,
    pub len: f64,
    pub height: f64,
    pub exterior: bool,
    /// The type's layer names, materials and functions, lower case ("" without layers).
    pub layers: String,
    pub has_layers: bool,
}

pub struct Opening {
    pub id: ElementId,
    pub door: bool,
    pub type_name: String,
    pub family: String,
    pub host: ElementId,
    pub offset: f64,
    /// Overall width (all mulled units) and height, mm.
    pub width: f64,
    pub height: f64,
    pub units: u32,
    /// A window's largest operable clear opening (width, height), mm; None when fixed.
    pub clear: Option<(f64, f64)>,
    pub sill: f64,
    pub at: Pt,
    pub mark: String,
}

pub struct RoomRec {
    pub id: ElementId,
    pub level: ElementId,
    pub name: String,
    pub number: String,
    pub ring: Option<Vec<Pt>>,
    pub area_sf: f64,
}

impl RoomRec {
    pub fn is(&self, words: &[&str]) -> bool {
        let n = self.name.to_lowercase();
        words.iter().any(|w| {
            n.split(|c: char| !c.is_alphanumeric()).any(|t| t == *w)
                || (w.len() > 4 && n.contains(w))
        })
    }
    /// Distance from `p` to the room's boundary (infinite when not enclosed).
    pub fn edge_dist(&self, p: Pt) -> f64 {
        let Some(r) = &self.ring else {
            return f64::INFINITY;
        };
        (0..r.len())
            .map(|i| project_to_segment(p, r[i], r[(i + 1) % r.len()]).1)
            .fold(f64::INFINITY, f64::min)
    }
    /// Least of its width and depth (bounding box), mm.
    pub fn min_dim(&self) -> f64 {
        self.ring
            .as_ref()
            .and_then(|r| studio_geom::bounds_of(r))
            .map_or(0.0, |(lo, hi)| (hi.x - lo.x).min(hi.y - lo.y))
    }
}

pub struct Ctx<'a> {
    pub doc: &'a Document,
    pub model: &'a Model,
    pub milestone: Milestone,
    pub details: ProjectDetails,
    pub project_name: String,
    pub project_number: String,
    pub residential: bool,
    pub california: bool,
    pub walls: Vec<WallInfo>,
    pub openings: Vec<Opening>,
    pub rooms: Vec<RoomRec>,
    pub level_elev: HashMap<ElementId, f64>,
    pub lowest_level: Option<ElementId>,
    /// View → the sheet it's placed on.
    pub on_sheet: HashMap<ElementId, ElementId>,
    /// Tagged elements (door, window and room tags).
    pub tagged: HashSet<ElementId>,
}

pub fn text_of(doc: &Document, layers: &[studio_core::element::WallLayer]) -> String {
    let mut t = String::new();
    for l in layers {
        t.push_str(&l.name.to_lowercase());
        t.push(' ');
        t.push_str(&format!("{:?} ", l.function).to_lowercase());
        if let Some(m) = l.material.and_then(|m| doc.data(m).ok()) {
            t.push_str(&m.name().to_lowercase());
            t.push(' ');
        }
    }
    t
}

/// A window type's largest operable clear opening (width, height), from its sashes: a
/// casement's whole sash, a hung or sliding sash's glass, half an awning's or hopper's.
pub fn clear_opening(d: &ElementData) -> Option<(f64, f64)> {
    use studio_core::windows::{layout, Operation, WindowStyle};
    let ElementData::WindowType { width, height, .. } = d else {
        return None;
    };
    let style = WindowStyle::of(d)?;
    let l = layout(style, *width, *height);
    l.lites
        .iter()
        .filter_map(|lite| {
            let [u0, z0, u1, z1] = lite.glass();
            let (w, h) = ((u1 - u0).max(0.0), (z1 - z0).max(0.0));
            match lite.operation {
                Operation::Fixed => None,
                Operation::Casement { .. } | Operation::Hung | Operation::Slide { .. } => {
                    Some((w, h))
                }
                Operation::Awning | Operation::Hopper => Some((w, h * 0.5)),
            }
        })
        .max_by(|a, b| (a.0 * a.1).total_cmp(&(b.0 * b.1)))
}

pub fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

impl<'a> Ctx<'a> {
    pub fn new(doc: &'a Document, model: &'a Model, milestone: Milestone) -> Self {
        let (identity, details) = studio_core::project::get(doc).unwrap_or_default();
        let kind = format!(
            "{} {}",
            details.overview.project_type, details.codes.occupancy
        )
        .to_lowercase();
        let bed = model
            .rooms
            .iter()
            .any(|r| r.name.to_lowercase().contains("bed"));
        let residential = has_any(
            &kind,
            &["residential", "house", "home", "dwelling", "r-3", "r-2"],
        ) || (details.overview.project_type.trim().is_empty() && bed);
        let place = format!(
            "{} {} {}",
            details.codes.building_code, details.location.state, details.location.jurisdiction
        )
        .to_lowercase();
        let california = has_any(&place, &["cbc", "crc", "california", "calgreen"])
            || details.location.state.trim().eq_ignore_ascii_case("ca");
        let level_elev: HashMap<ElementId, f64> =
            doc.levels().into_iter().map(|l| (l.0, l.2)).collect();
        let lowest_level = doc.levels().first().map(|l| l.0);
        let elev = |l: &ElementId| level_elev.get(l).copied().unwrap_or(0.0);
        let mut walls = vec![];
        for e in doc.of(Category::Wall) {
            let ElementData::Wall {
                type_id,
                start,
                end,
                base_level,
                base_offset,
                top,
                ..
            } = &e.data
            else {
                continue;
            };
            let (type_name, exterior, layers, has_layers) = match doc.data(*type_id) {
                Ok(ElementData::WallType {
                    name,
                    function,
                    layers,
                    ..
                }) => (
                    name.clone(),
                    *function == WallFunction::Exterior,
                    format!("{} {}", name.to_lowercase(), text_of(doc, layers)),
                    !layers.is_empty(),
                ),
                _ => (String::new(), false, String::new(), false),
            };
            let base = elev(base_level) + base_offset;
            let height = match top {
                WallTop::UpToLevel { level, offset } => elev(level) + offset - base,
                WallTop::Unconnected { height } => *height,
            };
            walls.push(WallInfo {
                id: e.id,
                type_id: *type_id,
                type_name,
                level: *base_level,
                start: *start,
                end: *end,
                len: start.dist(*end),
                height,
                exterior,
                layers,
                has_layers,
            });
        }
        let wall_of = |id: ElementId| walls.iter().find(|w| w.id == id);
        let mut openings = vec![];
        for e in doc.of(Category::Door).chain(doc.of(Category::Window)) {
            let (door, type_id, host, offset, sill, mark) = match &e.data {
                ElementData::Door {
                    type_id,
                    host,
                    offset,
                    mark,
                    ..
                } => (true, *type_id, *host, *offset, 0.0, mark.clone()),
                ElementData::Window {
                    type_id,
                    host,
                    offset,
                    sill,
                    mark,
                    ..
                } => (false, *type_id, *host, *offset, *sill, mark.clone()),
                _ => continue,
            };
            let (type_name, family, width, height, units) = match doc.data(type_id) {
                Ok(ElementData::DoorType {
                    name,
                    family,
                    width,
                    height,
                    ..
                }) => (name.clone(), format!("{family:?}"), *width, *height, 1),
                Ok(ElementData::WindowType {
                    name,
                    family,
                    width,
                    height,
                    units,
                    ..
                }) => (
                    name.clone(),
                    format!("{family:?}"),
                    *width,
                    *height,
                    (*units).max(1),
                ),
                _ => (String::new(), String::new(), 0.0, 0.0, 1),
            };
            let at = wall_of(host).map_or(Pt::new(0.0, 0.0), |w| {
                let d = w.end.sub(w.start);
                let n = if w.len > 0.0 { d.scale(1.0 / w.len) } else { d };
                w.start.add(n.scale(offset))
            });
            let clear = if door {
                None
            } else {
                doc.data(type_id).ok().and_then(clear_opening)
            };
            openings.push(Opening {
                id: e.id,
                door,
                type_name,
                family,
                host,
                offset,
                width,
                height,
                units,
                clear,
                sill,
                at,
                mark,
            });
        }
        let rooms = model
            .rooms
            .iter()
            .map(|r| RoomRec {
                id: r.id,
                level: r.level,
                name: r.name.clone(),
                number: r.number.clone(),
                ring: r.boundary.clone(),
                area_sf: r.area() / (MM_PER_FT * MM_PER_FT),
            })
            .collect();
        let on_sheet = doc
            .of(Category::Viewport)
            .filter_map(|e| match &e.data {
                ElementData::Viewport { sheet, view, .. } => Some((*view, *sheet)),
                _ => None,
            })
            .collect();
        let tagged = doc
            .of(Category::Tag)
            .filter_map(|e| match &e.data {
                ElementData::Tag { target, .. } => Some(*target),
                _ => None,
            })
            .collect();
        Ctx {
            doc,
            model,
            milestone,
            details,
            project_name: identity.name,
            project_number: identity.number,
            residential,
            california,
            walls,
            openings,
            rooms,
            level_elev,
            lowest_level,
            on_sheet,
            tagged,
        }
    }

    pub fn wall(&self, id: ElementId) -> Option<&WallInfo> {
        self.walls.iter().find(|w| w.id == id)
    }

    /// The residential or building code family ("IRC"/"CRC", "IBC"/"CBC") and a section.
    pub fn code(&self, irc: &str, ibc: &str) -> String {
        let (fam, sec) = match (self.residential, self.california) {
            (true, false) => ("IRC", irc),
            (true, true) => ("CRC", irc),
            (false, false) => ("IBC", ibc),
            (false, true) => ("CBC", ibc),
        };
        if sec.is_empty() {
            String::new()
        } else {
            format!("{fam} {sec}")
        }
    }

    /// The accessibility reference: CBC Chapter 11B in California, else the 2010 ADA
    /// Standards and ICC A117.1.
    pub fn access(&self, section: &str) -> String {
        if self.california {
            format!("CBC 11B-{section}")
        } else {
            format!("2010 ADA Standards {section}; ICC A117.1 {section}")
        }
    }

    pub fn code_basis(&self) -> String {
        let c = self.details.codes.building_code.trim();
        let base = if !c.is_empty() {
            c.to_string()
        } else if self.residential {
            if self.california {
                "California Residential Code (CRC)".into()
            } else {
                "International Residential Code (IRC)".into()
            }
        } else if self.california {
            "California Building Code (CBC)".into()
        } else {
            "International Building Code (IBC)".into()
        };
        if self.residential {
            base
        } else if self.california {
            format!("{base}, CBC Chapter 11B accessibility")
        } else {
            format!("{base}, 2010 ADA Standards / ICC A117.1")
        }
    }

    /// A plan view of `level` (to look at a finding).
    pub fn plan_of(&self, level: ElementId) -> Option<ElementId> {
        self.doc.of(Category::View).find_map(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::FloorPlan { level: l },
                ..
            } if *l == level => Some(e.id),
            _ => None,
        })
    }

    pub fn level_of(&self, id: ElementId) -> Option<ElementId> {
        let d = self.doc.data(id).ok()?;
        d.level().or_else(|| {
            d.host()
                .and_then(|h| self.doc.data(h).ok())
                .and_then(|h| h.level())
        })
    }

    /// Where to look at an element: a plan of its level.
    pub fn view_for(&self, id: ElementId) -> Option<ElementId> {
        self.level_of(id).and_then(|l| self.plan_of(l))
    }

    /// The rooms an opening is in the boundary of.
    pub fn rooms_at(&self, o: &Opening) -> Vec<&RoomRec> {
        let level = self.wall(o.host).map(|w| w.level);
        self.rooms
            .iter()
            .filter(|r| Some(r.level) == level && r.edge_dist(o.at) < 12.0 * MM_PER_IN)
            .collect()
    }

    /// The rooms along a wall (its midpoint within 12" of their boundary).
    pub fn rooms_along(&self, w: &WallInfo) -> Vec<&RoomRec> {
        let mid = w.start.add(w.end).scale(0.5);
        self.rooms
            .iter()
            .filter(|r| r.level == w.level && r.edge_dist(mid) < 12.0 * MM_PER_IN)
            .collect()
    }
}
