//! Model plans for Edit Model with Claude (ADR-051): a list of operations — create, change,
//! delete, move, copy, annotate — that Claude writes and this module runs, each through the
//! same modelling functions as the app's tools. A whole plan previews on a copy of the
//! document and applies as one undo step; an operation that fails stops it, and nothing
//! changes.
//!
//! Coordinates are feet (x east, y north), as Claude is given the model. Lengths in
//! properties are the app's usual feet-inches. An element made by one operation can be named
//! with `as` and referred to later as `$name`.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use studio_core::element::{Category, ElementData, ElementId, SheetSize, ViewKind};
use studio_core::model_edit::{self, EditContext, EditPreview, ModelEdit};
use studio_core::units::{parse_length, MM_PER_FT, MM_PER_IN};
use studio_core::{build, detail, edit, modify, ops, slope, structure, symbols};
use studio_core::{CoreError, CoreResult, Document};
use studio_geom::Pt;
use ts_rs::TS;

/// A point in feet. In an elevation or section, a plan point with a height `z` lands where
/// the view shows it.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export, rename = "PlanPoint")]
pub struct P {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

/// A property to set, by the name Properties shows.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NameValue {
    pub name: String,
    pub value: String,
}

/// One operation. Only the fields its `op` uses matter.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename = "PlanOp")]
pub struct Op {
    pub op: String,
    /// A name for what this makes, to refer to later as `$name`.
    #[serde(rename = "as")]
    #[ts(rename = "as")]
    pub name_as: String,
    pub id: String,
    pub ids: Vec<String>,
    // set_parameter / resize_building (see model_edit::ModelEdit).
    pub category: String,
    pub parameter: String,
    pub value: String,
    pub scope: String,
    pub type_filter: String,
    pub axis: String,
    pub anchor: String,
    // Creation.
    pub level: String,
    pub top_level: String,
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_name: String,
    pub from: String,
    pub start: Option<P>,
    pub end: Option<P>,
    pub at: Option<P>,
    pub center: Option<P>,
    pub a: Option<P>,
    pub b: Option<P>,
    pub points: Vec<P>,
    pub wall: String,
    /// Feet along a wall from its start, a dimension's offset, a roof's overhang…
    pub offset: Option<f64>,
    pub flip: bool,
    pub height: String,
    pub width: String,
    pub elevation: String,
    pub slope: String,
    pub name: String,
    pub number: String,
    pub text: String,
    pub view: String,
    pub sheet: String,
    pub by_walls: bool,
    pub dx: f64,
    pub dy: f64,
    pub angle: f64,
    pub count: u32,
    /// A material by name (the project's, or the library's), for paint.
    pub material: String,
    /// A colour "#rrggbb" for a new material.
    pub color: String,
    pub properties: Vec<NameValue>,
}

/// Claude's plan: the operations, a one-line summary, or why it can't be done.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ModelPlan {
    pub operations: Vec<Op>,
    pub summary: String,
    pub message: String,
}

fn bad(msg: impl Into<String>) -> CoreError {
    CoreError::Invalid(msg.into())
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn mm(p: P) -> Pt {
    Pt::new(p.x * MM_PER_FT, p.y * MM_PER_FT)
}

fn need(p: Option<P>, what: &str, op: &str) -> CoreResult<Pt> {
    p.map(mm).ok_or_else(|| bad(format!("{op} needs {what}")))
}

fn length(s: &str, what: &str) -> CoreResult<f64> {
    parse_length(s).ok_or_else(|| bad(format!("can't read {what} \"{s}\"")))
}

/// Runs plans, keeping what they make by name.
struct Runner<'a> {
    ctx: &'a EditContext,
    refs: HashMap<String, Vec<ElementId>>,
}

impl Runner<'_> {
    /// Element ids from an id, a `$name`, or (for walls, rooms, grids, views, sheets,
    /// levels) a unique name.
    fn ids(&self, doc: &Document, s: &str) -> CoreResult<Vec<ElementId>> {
        let s = s.trim().trim_start_matches('[').trim_end_matches(']');
        if let Some(r) = s.strip_prefix('$') {
            return self
                .refs
                .get(r)
                .cloned()
                .ok_or_else(|| bad(format!("nothing was made as ${r}")));
        }
        if let Ok(id) = s.parse::<ElementId>() {
            doc.data(id)?;
            return Ok(vec![id]);
        }
        let n = norm(s);
        let hits: Vec<ElementId> = doc
            .iter()
            .filter(|e| !n.is_empty() && norm(&e.data.name()) == n)
            .map(|e| e.id)
            .collect();
        match hits.len() {
            1 => Ok(hits),
            0 => Err(bad(format!("no element \"{s}\""))),
            _ => Err(bad(format!(
                "more than one element is called \"{s}\"; use its id"
            ))),
        }
    }

    fn one(&self, doc: &Document, s: &str) -> CoreResult<ElementId> {
        let v = self.ids(doc, s)?;
        v.first()
            .copied()
            .ok_or_else(|| bad(format!("nothing is \"{s}\"")))
    }

    fn many(&self, doc: &Document, op: &Op) -> CoreResult<Vec<ElementId>> {
        let mut out = vec![];
        for s in op.ids.iter().chain((!op.id.is_empty()).then_some(&op.id)) {
            out.extend(self.ids(doc, s)?);
        }
        if out.is_empty() {
            return Err(bad(format!("{} needs ids", op.op)));
        }
        Ok(out)
    }

    /// A level by id or name; blank is the active view's level, else the first level.
    fn level(&self, doc: &Document, s: &str) -> CoreResult<ElementId> {
        if s.trim().is_empty() {
            return self
                .ctx
                .view_level
                .or_else(|| doc.levels().first().map(|l| l.0))
                .ok_or_else(|| bad("the project has no levels"));
        }
        if let Ok(id) = self.one(doc, s) {
            if matches!(doc.data(id), Ok(ElementData::Level { .. })) {
                return Ok(id);
            }
        }
        let n = norm(s);
        doc.levels()
            .into_iter()
            .find(|l| norm(&l.1) == n)
            .map(|l| l.0)
            .ok_or_else(|| bad(format!("no level \"{s}\"")))
    }

    /// A type of `cat` by id or name (exact, then the one name that contains it); blank is
    /// the first.
    fn type_of(&self, doc: &Document, cat: Category, s: &str) -> CoreResult<ElementId> {
        if s.trim().is_empty() {
            return ops::first_of(doc, cat)
                .ok_or_else(|| bad(format!("the project has no {}", ops::category_label(cat))));
        }
        if let Ok(id) = s.trim().parse::<ElementId>() {
            if doc.data(id).is_ok_and(|d| d.category() == cat) {
                return Ok(id);
            }
        }
        let n = norm(s);
        let all: Vec<(ElementId, String)> =
            doc.of(cat).map(|e| (e.id, norm(&e.data.name()))).collect();
        if let Some((id, _)) = all.iter().find(|(_, name)| *name == n) {
            return Ok(*id);
        }
        let hits: Vec<ElementId> = all
            .iter()
            .filter(|(_, name)| name.contains(&n))
            .map(|x| x.0)
            .collect();
        match hits.len() {
            0 => Err(bad(format!(
                "no {} called \"{s}\" (there are {})",
                ops::category_label(cat).to_lowercase(),
                doc.of(cat)
                    .map(|e| e.data.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
            _ => Ok(hits[0]),
        }
    }

    /// A view by id or name; blank is the active view.
    fn view(&self, doc: &Document, s: &str) -> CoreResult<ElementId> {
        if s.trim().is_empty() {
            return self.ctx.view.ok_or_else(|| bad("open a view to annotate"));
        }
        self.one(doc, s)
    }

    fn keep(&mut self, op: &Op, ids: Vec<ElementId>) -> Vec<ElementId> {
        if !op.name_as.trim().is_empty() {
            self.refs.insert(
                op.name_as.trim().trim_start_matches('$').to_owned(),
                ids.clone(),
            );
        }
        ids
    }

    /// Sets `op.properties` on what it made, in order (Top Constraint before Unconnected
    /// Height, say).
    fn props(&self, doc: &mut Document, ids: &[ElementId], op: &Op) -> CoreResult<()> {
        for id in ids {
            for p in &op.properties {
                model_edit::set_element(doc, *id, &p.name, &p.value)?;
            }
        }
        Ok(())
    }

    fn run(&mut self, doc: &mut Document, op: &Op) -> CoreResult<String> {
        let o = op.op.as_str();
        let made = match norm(o).as_str() {
            "setparameter" | "resizebuilding" => {
                let e = ModelEdit {
                    action: op.op.clone(),
                    category: op.category.clone(),
                    parameter: op.parameter.clone(),
                    value: op.value.clone(),
                    scope: op.scope.clone(),
                    level: op.level.clone(),
                    type_filter: op.type_filter.clone(),
                    axis: op.axis.clone(),
                    anchor: op.anchor.clone(),
                    ..Default::default()
                };
                return Ok(model_edit::run(doc, &e, self.ctx)?.summary);
            }
            "setproperty" => {
                let ids = self.many(doc, op)?;
                let mut label = String::new();
                for id in &ids {
                    label = model_edit::set_element(doc, *id, &op.parameter, &op.value)?;
                }
                return Ok(format!(
                    "{label} → {} on {} element(s)",
                    op.value,
                    ids.len()
                ));
            }
            "delete" => {
                let ids = self.many(doc, op)?;
                let n = ops::delete(doc, &ids)?;
                return Ok(format!("Deleted {n} element(s)"));
            }
            "move" => {
                let ids = self.many(doc, op)?;
                let d = Pt::new(op.dx * MM_PER_FT, op.dy * MM_PER_FT);
                modify::move_elements(doc, &ids, d)?;
                return Ok(format!(
                    "Moved {} element(s) {}′, {}′",
                    ids.len(),
                    op.dx,
                    op.dy
                ));
            }
            "copy" => {
                let ids = self.many(doc, op)?;
                let n = op.count.max(1);
                let xs: Vec<edit::Xform> = (1..=n)
                    .map(|k| {
                        edit::Xform::translate(Pt::new(
                            op.dx * MM_PER_FT * f64::from(k),
                            op.dy * MM_PER_FT * f64::from(k),
                        ))
                    })
                    .collect();
                let made = edit::copy_elements(doc, &ids, &xs, "Copy")?;
                self.keep(op, made)
            }
            "rotate" => {
                let ids = self.many(doc, op)?;
                let c = need(op.center, "a center", o)?;
                edit::transform_elements(
                    doc,
                    &ids,
                    edit::Xform::rotate(c, op.angle.to_radians()),
                    "Rotate",
                )?;
                return Ok(format!("Rotated {} element(s) {}°", ids.len(), op.angle));
            }
            "mirror" => {
                let ids = self.many(doc, op)?;
                let (a, b) = (
                    need(op.a, "an axis start a", o)?,
                    need(op.b, "an axis end b", o)?,
                );
                edit::transform_elements(doc, &ids, edit::Xform::mirror(a, b), "Mirror")?;
                return Ok(format!("Mirrored {} element(s)", ids.len()));
            }
            "createlevel" => {
                let z = length(&op.elevation, "the elevation")?;
                let id = ops::create_level(doc, z)?;
                if !op.name.trim().is_empty() {
                    ops::set_property(doc, id, "name", &op.name, 0)?;
                }
                self.keep(op, vec![id])
            }
            "creategrid" => {
                let id =
                    ops::create_grid(doc, need(op.start, "start", o)?, need(op.end, "end", o)?)?;
                if !op.name.trim().is_empty() {
                    ops::set_property(doc, id, "name", &op.name, 0)?;
                }
                self.keep(op, vec![id])
            }
            "createwall" => {
                let t = self.type_of(doc, Category::WallType, &op.type_name)?;
                let l = self.level(doc, &op.level)?;
                let id = ops::create_wall(
                    doc,
                    t,
                    l,
                    need(op.start, "start", o)?,
                    need(op.end, "end", o)?,
                )?;
                if !op.top_level.trim().is_empty() {
                    let top = self.level(doc, &op.top_level)?;
                    ops::set_property(doc, id, "top", &top.to_string(), 0)?;
                } else if !op.height.trim().is_empty() {
                    ops::set_property(doc, id, "top", "unconnected", 0)?;
                    ops::set_property(doc, id, "height", &op.height, 0)?;
                }
                self.keep(op, vec![id])
            }
            "createdoor" | "createwindow" => {
                let door = norm(o) == "createdoor";
                let wall = self.one(doc, &op.wall)?;
                let Ok(ElementData::Wall { start, end, .. }) = doc.data(wall) else {
                    return Err(bad(format!("{} isn't a wall", op.wall)));
                };
                let (s, e) = (*start, *end);
                // Along the wall: given, or where `at` falls on it, else its middle.
                let along = match (op.offset, op.at) {
                    (Some(f), _) => f * MM_PER_FT,
                    (None, Some(p)) => mm(p).sub(s).dot(e.sub(s).norm()),
                    _ => s.dist(e) / 2.0,
                };
                let cat = if door {
                    Category::DoorType
                } else {
                    Category::WindowType
                };
                let t = self.type_of(doc, cat, &op.type_name)?;
                let id = if door {
                    ops::create_door(doc, t, wall, along, op.flip)?
                } else {
                    ops::create_window(doc, t, wall, along, op.flip)?
                };
                self.keep(op, vec![id])
            }
            "createfloor" | "createceiling" => {
                let floor = norm(o) == "createfloor";
                let cat = if floor {
                    Category::FloorType
                } else {
                    Category::CeilingType
                };
                let t = self.type_of(doc, cat, &op.type_name)?;
                let l = self.level(doc, &op.level)?;
                let id = if op.points.len() >= 3 {
                    let pts: Vec<Pt> = op.points.iter().map(|p| mm(*p)).collect();
                    if floor {
                        ops::create_floor(doc, t, l, pts)?
                    } else {
                        ops::create_ceiling(doc, t, l, pts)?
                    }
                } else if floor {
                    crate::derived::create_floor_by_walls(doc, t, l)?
                } else {
                    let at = need(op.at, "points or a point in a room", o)?;
                    crate::derived::create_ceiling_in_room(doc, t, l, at)?
                };
                self.keep(op, vec![id])
            }
            "createroof" => {
                let t = self.type_of(doc, Category::RoofType, &op.type_name)?;
                let l = self.level(doc, &op.level)?;
                let slope = if op.slope.trim().is_empty() {
                    build::DEFAULT_ROOF_SLOPE
                } else {
                    build::parse_slope(&op.slope).ok_or_else(|| {
                        bad(format!(
                            "can't read the slope \"{}\" (try 6/12 or 30°)",
                            op.slope
                        ))
                    })?
                };
                let overhang = op.offset.map_or(build::DEFAULT_OVERHANG, |f| f * MM_PER_FT);
                if op.points.len() >= 3 {
                    let pts: Vec<Pt> = op.points.iter().map(|p| mm(*p)).collect();
                    // It bears on the tops of the level's walls under it (or `height` above
                    // the level), not on the floor.
                    let base = if op.height.trim().is_empty() {
                        let model = crate::regenerate(doc);
                        let (lo, hi) = studio_geom::bounds_of(&pts).unwrap_or_default();
                        let m = 2.0 * MM_PER_FT;
                        let inside = |p: Pt| {
                            p.x >= lo.x - m && p.x <= hi.x + m && p.y >= lo.y - m && p.y <= hi.y + m
                        };
                        let top = model
                            .walls
                            .iter()
                            .filter(|w| w.level == l && inside(w.start.lerp(w.end, 0.5)))
                            .map(|w| w.z1)
                            .fold(f64::NEG_INFINITY, f64::max);
                        if top.is_finite() {
                            top - doc.level_elevation(l)?
                        } else {
                            0.0
                        }
                    } else {
                        length(&op.height, "the roof's height")?
                    };
                    let id = build::create_roof(doc, t, l, base, pts, slope)?;
                    self.keep(op, vec![id])
                } else {
                    // Over the walls of `level`, bearing on their tops (the Roof tool).
                    let model = crate::regenerate(doc);
                    let outer = crate::outer_boundary(&model, l).ok_or_else(|| {
                        bad("draw walls on that level first, or give the roof's points")
                    })?;
                    let top = model
                        .walls
                        .iter()
                        .filter(|w| w.level == l)
                        .map(|w| w.z1)
                        .fold(f64::NEG_INFINITY, f64::max);
                    let (roof_level, base) = doc
                        .levels()
                        .into_iter()
                        .find(|x| (x.2 - top).abs() < 1.0)
                        .map_or_else(
                            || (l, top - doc.level_elevation(l).unwrap_or(0.0)),
                            |x| (x.0, 0.0),
                        );
                    let ids = build::create_roofs_by_footprint(
                        doc, t, roof_level, base, &outer, overhang, slope,
                    )?;
                    self.keep(op, ids)
                }
            }
            "createroom" => {
                let l = self.level(doc, &op.level)?;
                let id = ops::create_room(doc, l, need(op.at, "a point inside the room", o)?)?;
                if !op.name.trim().is_empty() {
                    ops::set_property(doc, id, "name", &op.name, 0)?;
                }
                if !op.number.trim().is_empty() {
                    ops::set_property(doc, id, "number", &op.number, 0)?;
                }
                self.keep(op, vec![id])
            }
            "createroomseparator" => {
                let l = self.level(doc, &op.level)?;
                let id = detail::create_room_separator(
                    doc,
                    l,
                    need(op.start, "start", o)?,
                    need(op.end, "end", o)?,
                )?;
                self.keep(op, vec![id])
            }
            "createcolumn" => {
                let t = self.type_of(doc, Category::ColumnType, &op.type_name)?;
                let l = self.level(doc, &op.level)?;
                let id = structure::create_column(
                    doc,
                    t,
                    l,
                    need(op.at, "a point", o)?,
                    op.angle.to_radians(),
                )?;
                self.keep(op, vec![id])
            }
            "createbeam" => {
                let t = self.type_of(doc, Category::BeamType, &op.type_name)?;
                let l = self.level(doc, &op.level)?;
                let id = structure::create_beam(
                    doc,
                    t,
                    l,
                    need(op.start, "start", o)?,
                    need(op.end, "end", o)?,
                )?;
                self.keep(op, vec![id])
            }
            "createrailing" => {
                let t = self.type_of(doc, Category::RailingType, &op.type_name)?;
                let l = self.level(doc, &op.level)?;
                let pts: Vec<Pt> = op.points.iter().map(|p| mm(*p)).collect();
                if pts.len() < 2 {
                    return Err(bad("a railing needs two or more points"));
                }
                let id = structure::create_railing(doc, t, l, pts)?;
                self.keep(op, vec![id])
            }
            "createstair" => {
                let l = self.level(doc, &op.level)?;
                let w = if op.width.trim().is_empty() {
                    3.0 * MM_PER_FT
                } else {
                    length(&op.width, "the width")?
                };
                let id = build::create_stair(
                    doc,
                    l,
                    need(op.start, "start", o)?,
                    need(op.end, "end (the direction it climbs)", o)?,
                    w,
                )?;
                self.keep(op, vec![id])
            }
            "createtext" => {
                let v = self.view(doc, &op.view)?;
                let at = self.in_view(doc, v, op.at.ok_or_else(|| bad("create_text needs at"))?);
                let id = ops::create_text(doc, v, at, &op.text)?;
                self.keep(op, vec![id])
            }
            "createdimension" => {
                let v = self.view(doc, &op.view)?;
                let (a, b) = (
                    self.in_view(doc, v, op.a.ok_or_else(|| bad("create_dimension needs a"))?),
                    self.in_view(doc, v, op.b.ok_or_else(|| bad("create_dimension needs b"))?),
                );
                let off = op.offset.map_or(3.0 * MM_PER_FT, |f| f * MM_PER_FT);
                let id = ops::create_dimension(doc, v, a, b, off)?;
                self.keep(op, vec![id])
            }
            "tagall" => {
                let v = self.view(doc, &op.view)?;
                let n = ops::tag_all(doc, v)?;
                return Ok(format!("Tagged {n} element(s)"));
            }
            "createspotelevation" => {
                let v = self.view(doc, &op.view)?;
                let at = self.in_view(
                    doc,
                    v,
                    op.at.ok_or_else(|| bad("create_spot_elevation needs at"))?,
                );
                let leader = op.b.map_or(at, |p| self.in_view(doc, v, p));
                let id = symbols::create_spot_elevation(doc, v, at, leader)?;
                self.keep(op, vec![id])
            }
            "createspotslope" => {
                let v = self.view(doc, &op.view)?;
                let at = self.in_view(
                    doc,
                    v,
                    op.at.ok_or_else(|| bad("create_spot_slope needs at"))?,
                );
                let id = slope::create_spot_slope(doc, v, at)?;
                self.keep(op, vec![id])
            }
            "createnortharrow" | "creategraphicscale" | "createkeyplan" => {
                let target = if norm(o) == "createkeyplan" {
                    self.one(doc, &op.sheet)?
                } else {
                    self.view(doc, &op.view)?
                };
                let at = need(op.at, "a point", o)?;
                let id = match norm(o).as_str() {
                    "createnortharrow" => {
                        symbols::create_north_arrow(doc, target, self.on(doc, target, at))?
                    }
                    "creategraphicscale" => {
                        symbols::create_graphic_scale(doc, target, self.on(doc, target, at))?
                    }
                    _ => {
                        symbols::create_key_plan(doc, target, sheet_pt(op.at.unwrap_or_default()))?
                    }
                };
                self.keep(op, vec![id])
            }
            "createsection" => {
                let id =
                    ops::create_section(doc, need(op.start, "start", o)?, need(op.end, "end", o)?)?;
                if !op.name.trim().is_empty() {
                    ops::set_property(doc, id, "name", &op.name, 0)?;
                }
                self.keep(op, vec![id])
            }
            "createelevationmarker" => {
                let l = self.level(doc, &op.level)?;
                let id = crate::derived::create_elevation_marker(
                    doc,
                    l,
                    need(op.at, "a point", o)?,
                    false,
                )?;
                self.keep(op, vec![id])
            }
            "createsheet" => {
                let name = if op.name.trim().is_empty() {
                    "Unnamed"
                } else {
                    op.name.trim()
                };
                let id = ops::create_sheet(doc, name, SheetSize::ArchD)?;
                if !op.number.trim().is_empty() {
                    ops::set_property(doc, id, "number", &op.number, 0)?;
                }
                self.keep(op, vec![id])
            }
            "placeview" => {
                let sheet = self.one(doc, &op.sheet)?;
                let view = self.one(doc, &op.view)?;
                let at = op.at.map_or(Pt::new(400.0, 300.0), sheet_pt);
                let id = ops::place_view(doc, sheet, view, at)?;
                self.keep(op, vec![id])
            }
            "createtype" => {
                let cat = type_category(&op.category)?;
                let from = self.type_of(doc, cat, &op.from)?;
                let data = doc.data(from)?.clone();
                let id = doc.transact("Duplicate type", |tx| Ok(tx.insert(data)))?;
                let name = if op.name.trim().is_empty() {
                    format!("{} 2", doc.data(from)?.name())
                } else {
                    op.name.trim().to_owned()
                };
                ops::set_property(doc, id, "name", &name, 0)?;
                self.keep(op, vec![id])
            }
            // Lines (ADR-054): a polyline through points (or start to end), in a style.
            "createdetailline" | "createmodelline" => {
                let detail = norm(o) == "createdetailline";
                let pts: Vec<Pt> = if op.points.len() >= 2 {
                    op.points.iter().map(|p| mm(*p)).collect()
                } else {
                    vec![need(op.start, "start", o)?, need(op.end, "end", o)?]
                };
                let on = if detail {
                    let v = self.view(doc, &op.view)?;
                    studio_core::lines::LinesOn::View(v)
                } else {
                    studio_core::lines::LinesOn::Level(self.level(doc, &op.level)?)
                };
                let style = if op.slope.is_empty() && op.value.is_empty() {
                    studio_core::lines::LineStyle::Thin
                } else {
                    let s = if op.value.is_empty() {
                        &op.slope
                    } else {
                        &op.value
                    };
                    studio_core::lines::LineStyle::parse(s)
                        .ok_or_else(|| bad(format!("no line style \"{s}\"")))?
                };
                let mut made = vec![];
                for w in pts.windows(2) {
                    made.extend(studio_core::lines::create_lines(
                        doc,
                        on,
                        studio_core::sketch::DrawTool::Line,
                        &[w[0], w[1]],
                        &studio_core::sketch::DrawOptions::default(),
                        style,
                    )?);
                }
                self.keep(op, made)
            }
            "paint" => {
                let m = self.material(doc, op)?;
                let ids = if op.ids.is_empty() && op.id.is_empty() {
                    self.of_category(doc, op)?
                } else {
                    self.many(doc, op)?
                };
                let n = studio_core::paint::paint(doc, &ids, Some(m))?;
                return Ok(format!("Painted {n} element(s) {}", doc.data(m)?.name()));
            }
            "creatematerial" => {
                let id = self.material(doc, op)?;
                self.keep(op, vec![id])
            }
            "none" => return Err(bad("nothing to do")),
            other => return Err(bad(format!("unknown operation \"{other}\""))),
        };
        self.props(doc, &made, op)?;
        let what: Vec<String> = made
            .iter()
            .filter_map(|id| doc.data(*id).ok().map(|d| d.name()))
            .take(3)
            .collect();
        Ok(format!(
            "Created {} {}",
            made.len(),
            if what.is_empty() {
                o.trim_start_matches("create_").to_owned()
            } else {
                what.join(", ")
            }
        ))
    }

    /// A material: the project's by name, else a library preset by name, else a new one
    /// with `color`.
    fn material(&self, doc: &mut Document, op: &Op) -> CoreResult<ElementId> {
        let want = if op.material.trim().is_empty() {
            op.name.trim()
        } else {
            op.material.trim()
        };
        let n = norm(want);
        if let Ok(id) = want.parse::<ElementId>() {
            if matches!(doc.data(id), Ok(ElementData::Material { .. })) {
                return Ok(id);
            }
        }
        if !n.is_empty() && op.color.trim().is_empty() {
            if let Some(e) = doc
                .of(Category::Material)
                .find(|e| norm(&e.data.name()) == n)
                .or_else(|| {
                    doc.of(Category::Material)
                        .find(|e| norm(&e.data.name()).contains(&n))
                })
            {
                return Ok(e.id);
            }
            if let Some(p) = studio_core::library::library()
                .into_iter()
                .find(|p| norm(&p.name) == n || norm(&p.id) == n)
                .or_else(|| {
                    studio_core::library::library()
                        .into_iter()
                        .find(|p| norm(&p.name).contains(&n))
                })
            {
                return studio_core::library::add_preset(doc, &p.id);
            }
        }
        if op.color.trim().is_empty() {
            return Err(bad(format!(
                "no material \"{want}\"; give a color (\"#3a5f8f\") to make one"
            )));
        }
        let id = studio_core::material::create_material(doc, None)?;
        let name = if want.is_empty() {
            format!("Paint {}", op.color.trim())
        } else {
            want.to_owned()
        };
        ops::set_property(doc, id, "name", &name, 0)?;
        ops::set_property(doc, id, "color", op.color.trim(), 0)?;
        Ok(id)
    }

    /// Every element of `op.category`, on `op.level` when given.
    fn of_category(&self, doc: &Document, op: &Op) -> CoreResult<Vec<ElementId>> {
        let n = norm(&op.category);
        let cat = model_edit::CATEGORIES
            .iter()
            .find(|(l, c)| {
                norm(l) == n || norm(l).trim_end_matches('s') == n || norm(c.as_str()) == n
            })
            .map(|x| x.1)
            .ok_or_else(|| {
                bad(format!(
                    "paint needs ids or a category (not \"{}\")",
                    op.category
                ))
            })?;
        let level = if op.level.trim().is_empty() {
            None
        } else {
            Some(self.level(doc, &op.level)?)
        };
        Ok(doc
            .of(cat)
            .filter(|e| level.is_none() || e.data.level() == level)
            .map(|e| e.id)
            .collect())
    }

    /// A plan point (feet → mm) in `view`'s own coordinates: plans are plan coordinates;
    /// elevations and sections read x as along the view and y as height.
    fn in_view(&self, doc: &Document, view: ElementId, p: P) -> Pt {
        let q = mm(p);
        let Some(z) = p.z else { return q };
        let frame = match doc.data(view) {
            Ok(ElementData::View {
                kind: ViewKind::Elevation { facing },
                ..
            }) => Some((Pt::default(), facing.look().scale(-1.0))),
            Ok(ElementData::View {
                kind: ViewKind::Section { start, end, .. },
                ..
            }) => Some((*start, end.sub(*start).norm().perp())),
            _ => None,
        };
        match frame {
            Some((origin, look)) => {
                let right = Pt::new(look.y, -look.x);
                Pt::new(q.sub(origin).dot(right), z * MM_PER_FT)
            }
            None => q,
        }
    }

    /// A point on a sheet (inches from its lower left) or in a view (feet).
    fn on(&self, doc: &Document, target: ElementId, p: Pt) -> Pt {
        if matches!(doc.data(target), Ok(ElementData::Sheet { .. })) {
            Pt::new(p.x / MM_PER_FT * MM_PER_IN, p.y / MM_PER_FT * MM_PER_IN)
        } else {
            p
        }
    }
}

/// Sheet points are given in inches from the sheet's lower left.
fn sheet_pt(p: P) -> Pt {
    Pt::new(p.x * MM_PER_IN, p.y * MM_PER_IN)
}

fn type_category(s: &str) -> CoreResult<Category> {
    let n = norm(s);
    let n = n
        .trim_end_matches("types")
        .trim_end_matches("type")
        .trim_end_matches('s');
    Ok(match n {
        "wall" => Category::WallType,
        "floor" => Category::FloorType,
        "ceiling" => Category::CeilingType,
        "door" => Category::DoorType,
        "window" => Category::WindowType,
        "roof" => Category::RoofType,
        "column" => Category::ColumnType,
        "beam" => Category::BeamType,
        "railing" => Category::RailingType,
        _ => return Err(bad(format!("no type category \"{s}\""))),
    })
}

/// What changed between two documents, by category: made, changed, deleted.
fn diff(
    before: &Document,
    after: &Document,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<ElementId>) {
    let ids_before: HashSet<ElementId> = before.iter().map(|e| e.id).collect();
    let mut made: BTreeMap<String, usize> = BTreeMap::new();
    let mut changed: BTreeMap<String, usize> = BTreeMap::new();
    let mut gone: BTreeMap<String, usize> = BTreeMap::new();
    let mut lit = vec![];
    let label = |c: Category| ops::category_label(c).to_owned();
    for e in after.iter() {
        match before.get(e.id) {
            None => *made.entry(label(e.data.category())).or_default() += 1,
            Some(b) if b.data != e.data => {
                *changed.entry(label(e.data.category())).or_default() += 1;
                lit.push(e.id);
            }
            _ => {}
        }
    }
    for e in before.iter().filter(|e| after.get(e.id).is_none()) {
        *gone.entry(label(e.data.category())).or_default() += 1;
        lit.push(e.id);
    }
    let _ = ids_before;
    let list =
        |m: BTreeMap<String, usize>| m.into_iter().map(|(k, n)| format!("{n} {k}")).collect();
    (list(made), list(changed), list(gone), lit)
}

fn run(doc: &mut Document, plan: &ModelPlan, ctx: &EditContext) -> CoreResult<Vec<String>> {
    if plan.operations.is_empty() {
        return Err(bad(if plan.message.trim().is_empty() {
            "Claude couldn't turn that into a model change".to_owned()
        } else {
            plan.message.clone()
        }));
    }
    let mut r = Runner {
        ctx,
        refs: HashMap::new(),
    };
    let mut steps = vec![];
    for (i, op) in plan.operations.iter().enumerate() {
        let s = r
            .run(doc, op)
            .map_err(|e| bad(format!("step {} ({}): {e}", i + 1, op.op)))?;
        steps.push(s);
    }
    Ok(steps)
}

/// What `plan` would do, worked out on a copy of the document.
pub fn preview(doc: &Document, plan: &ModelPlan, ctx: &EditContext) -> CoreResult<EditPreview> {
    let mut copy = doc.clone();
    let steps = run(&mut copy, plan, ctx)?;
    Ok(outcome(doc, &copy, plan, steps, ctx))
}

fn outcome(
    before: &Document,
    after: &Document,
    plan: &ModelPlan,
    steps: Vec<String>,
    ctx: &EditContext,
) -> EditPreview {
    let (created, changed, deleted, ids) = diff(before, after);
    // One bulk change reads as the parameter card; anything else as the steps.
    let single = (plan.operations.len() == 1 && norm(&plan.operations[0].op) == "setparameter")
        .then(|| {
            let op = &plan.operations[0];
            let e = ModelEdit {
                action: op.op.clone(),
                category: op.category.clone(),
                parameter: op.parameter.clone(),
                value: op.value.clone(),
                scope: op.scope.clone(),
                level: op.level.clone(),
                type_filter: op.type_filter.clone(),
                ..Default::default()
            };
            model_edit::run(&mut before.clone(), &e, ctx).ok()
        })
        .flatten();
    let summary = if plan.summary.trim().is_empty() {
        steps.join("; ")
    } else {
        plan.summary.trim().to_owned()
    };
    let count = ids.len()
        + created
            .iter()
            .filter_map(|s| s.split(' ').next()?.parse::<usize>().ok())
            .sum::<usize>();
    match single {
        Some(p) => EditPreview {
            ids,
            steps,
            created,
            changed,
            deleted,
            ..p
        },
        None => EditPreview {
            summary,
            count,
            ids,
            steps,
            created,
            changed,
            deleted,
            ..Default::default()
        },
    }
}

/// Runs `plan` as one undo step; if any step fails, nothing changes.
pub fn apply(doc: &mut Document, plan: &ModelPlan, ctx: &EditContext) -> CoreResult<EditPreview> {
    let before = doc.clone();
    let mark = doc.undo_depth();
    match run(doc, plan, ctx) {
        Ok(steps) => {
            let p = outcome(&before, doc, plan, steps, ctx);
            doc.merge_undo(mark, &model_edit::undo_label(&p));
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

/// The operations Claude may use, for its instructions.
pub const OPERATIONS: &str = r##"Operations (fields besides "op"; points are {"x","y"} in feet; "as" names what an operation makes so later ones can use "$name" where an id goes):
- set_parameter: category, parameter, value, scope (model|level|selection|view), level, typeFilter — one property on many elements of a category.
- set_property: id or ids, parameter, value — one property on specific elements (type properties like a door's Width switch it to a type of that size).
- resize_building: axis (east-west|north-south), value (overall length), anchor (west|east|south|north|center).
- delete: ids. move: ids, dx, dy (feet). copy: ids, dx, dy, count, as. rotate: ids, center, angle (degrees, counter-clockwise). mirror: ids, a, b (the axis).
- create_level: elevation (e.g. "20'-0\""), name.
- create_grid: start, end, name.
- create_wall: level, type, start, end, and optionally topLevel or height ("9'-0\"") — walls are centerlines; join walls end to end at shared points.
- create_door / create_window: wall (id or $name), offset (feet from the wall's start to the opening's center) or at (a point on the wall), type, flip.
- create_floor / create_ceiling: level, type, points (outline) — or omit points: a floor fills the level's walls, a ceiling fills the room at "at".
- create_roof: level (the walls' level), type, slope ("6/12"; "0" for flat), offset (overhang, feet) — omit points to roof over ALL of that level's walls; or give points (the eave outline, overhang included) for a roof over part of it, e.g. an addition: it sits on the tops of the walls under it (or give height, the eave's height above the level).
- create_room: level, at (a point inside walls), name, number. create_room_separator: level, start, end.
- create_column: level, at, type, angle. create_beam: level, start, end, type. create_railing: level, points, type. create_stair: level, start, end (the direction it climbs), width.
- create_text: view (blank = active view), at, text. In an elevation or section, give points as the plan x, y of the spot plus its height z in feet (e.g. {"x":10,"y":0,"z":8}). create_dimension: view, a, b, offset (feet from the measured line). tag_all: view.
- create_spot_elevation: view, at, b (where the text goes). create_spot_slope: view, at. create_north_arrow / create_graphic_scale: view, at. create_key_plan: sheet, at (inches on the sheet).
- create_section: start, end, name. create_elevation_marker: level, at. create_sheet: name, number. place_view: sheet, view, at (inches from the sheet's lower left; default centered).
- paint: ids, or category (+ level), and material (a project or library material by name, e.g. "Brick, Running Bond") — or color "#rrggbb" (with name) for a new one. create_material: name, color, or material (a library material to add).
- create_detail_line: view (blank = active), points (a polyline) or start/end, value (line style: Thin Lines, Medium Lines, Wide Lines, Hidden, Centerline, Overhead, Demolished, Beyond) — 2D, in that view only. create_model_line: level, points or start/end, value (line style) — on the level, seen in plans, elevations and 3D.
- create_type: category (Wall|Floor|Ceiling|Door|Window|Roof|Column|Beam|Railing), from (a type to copy), name, properties.
Every create operation also takes "properties": [{"name","value"}] set afterwards, by the names Properties shows (e.g. {"name":"Sill Height","value":"3'-0\""}, {"name":"Mark","value":"101A"}, {"name":"Width","value":"3'-6\""})."##;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn project() -> Document {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        doc
    }

    fn plan(v: serde_json::Value) -> ModelPlan {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn a_plan_builds_a_room_with_a_door_window_floor_and_label() {
        let mut doc = project();
        let ctx = EditContext::default();
        let p = plan(json!({
            "summary": "A 12' x 10' room",
            "operations": [
                { "op": "create_wall", "as": "s", "start": {"x": 0, "y": 0}, "end": {"x": 12, "y": 0} },
                { "op": "create_wall", "start": {"x": 12, "y": 0}, "end": {"x": 12, "y": 10} },
                { "op": "create_wall", "start": {"x": 12, "y": 10}, "end": {"x": 0, "y": 10}, "height": "9'-0\"" },
                { "op": "create_wall", "start": {"x": 0, "y": 10}, "end": {"x": 0, "y": 0} },
                { "op": "create_door", "wall": "$s", "offset": 6, "type": "Single Flush 36" },
                { "op": "create_window", "wall": "$s", "at": {"x": 3, "y": 0},
                  "properties": [{ "name": "Sill Height", "value": "3'-0\"" }] },
                { "op": "create_floor" },
                { "op": "create_room", "at": {"x": 6, "y": 5}, "name": "Office", "number": "101" }
            ]
        }));
        let before = doc.stamp();
        let pv = preview(&doc, &p, &ctx).unwrap();
        assert_eq!(doc.stamp(), before, "a preview changes nothing");
        assert_eq!(pv.steps.len(), 8);
        assert!(
            pv.created.contains(&"4 Walls".to_owned()),
            "{:?}",
            pv.created
        );
        assert!(
            pv.created.iter().any(|c| c.ends_with("Doors"))
                && pv.created.iter().any(|c| c.ends_with("Rooms"))
        );
        let depth = doc.undo_depth();
        let applied = apply(&mut doc, &p, &ctx).unwrap();
        assert_eq!(doc.undo_depth(), depth + 1, "one undo step");
        assert_eq!(
            doc.can_undo(),
            Some(model_edit::undo_label(&applied).as_str())
        );
        assert_eq!(doc.count(Category::Wall), 4);
        assert_eq!(doc.count(Category::Door), 1);
        assert_eq!(doc.count(Category::Floor), 1);
        let room = doc.of(Category::Room).next().unwrap();
        assert_eq!(room.data.name(), "Office 101");
        let sill = doc
            .of(Category::Window)
            .find_map(|e| match &e.data {
                ElementData::Window { sill, offset, .. } => Some((*sill, *offset)),
                _ => None,
            })
            .unwrap();
        assert!((sill.0 - 3.0 * MM_PER_FT).abs() < 1e-6 && (sill.1 - 3.0 * MM_PER_FT).abs() < 1e-6);
        doc.undo().unwrap();
        assert_eq!(doc.count(Category::Wall), 0);
    }

    #[test]
    fn edits_annotations_and_refusals() {
        let mut doc = project();
        let plan_view = doc
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
        let ctx = EditContext {
            view: Some(plan_view),
            ..Default::default()
        };
        let p = plan(json!({ "operations": [
            { "op": "create_wall", "as": "w", "start": {"x": 0, "y": 0}, "end": {"x": 20, "y": 0} },
            { "op": "move", "ids": ["$w"], "dx": 0, "dy": 5 },
            { "op": "copy", "ids": ["$w"], "dx": 0, "dy": 10, "count": 2, "as": "copies" },
            { "op": "create_text", "at": {"x": 2, "y": 2}, "text": "NEW WING" },
            { "op": "create_dimension", "a": {"x": 0, "y": 5}, "b": {"x": 20, "y": 5}, "offset": 3 },
            { "op": "create_level", "elevation": "30'-0\"", "name": "Roof Deck" },
            { "op": "set_property", "ids": ["$copies"], "parameter": "Length", "value": "10'" }
        ]}));
        apply(&mut doc, &p, &ctx).unwrap();
        assert_eq!(doc.count(Category::Wall), 3);
        assert_eq!(doc.count(Category::TextNote), 1);
        assert_eq!(doc.count(Category::Dimension), 1);
        assert!(doc.levels().iter().any(|l| l.1 == "Roof Deck"));
        let ys: Vec<(f64, f64)> = doc
            .of(Category::Wall)
            .filter_map(|e| match &e.data {
                ElementData::Wall { start, end, .. } => {
                    Some((start.y / MM_PER_FT, start.dist(*end) / MM_PER_FT))
                }
                _ => None,
            })
            .collect();
        assert!(
            ys.iter()
                .any(|(y, l)| (*y - 5.0).abs() < 1e-6 && (*l - 20.0).abs() < 1e-6),
            "{ys:?}"
        );
        assert!(
            ys.iter()
                .any(|(y, l)| (*y - 25.0).abs() < 1e-6 && (*l - 10.0).abs() < 1e-6),
            "{ys:?}"
        );
        // A failing step stops the plan and leaves nothing behind.
        let walls = doc.count(Category::Wall);
        let depth = doc.undo_depth();
        let bad_plan = plan(json!({ "operations": [
            { "op": "create_wall", "start": {"x": 0, "y": 40}, "end": {"x": 10, "y": 40} },
            { "op": "create_door", "wall": "$nothing" }
        ]}));
        let err = apply(&mut doc, &bad_plan, &ctx).unwrap_err().to_string();
        assert!(err.contains("step 2") && err.contains("$nothing"), "{err}");
        assert_eq!(
            (doc.count(Category::Wall), doc.undo_depth()),
            (walls, depth)
        );
        // An empty plan says why.
        let none = plan(json!({ "operations": [], "message": "Can't paint yet" }));
        assert_eq!(
            preview(&doc, &none, &ctx).unwrap_err().to_string(),
            "Can't paint yet"
        );
        // Delete by id.
        let id = doc.of(Category::TextNote).next().unwrap().id;
        apply(
            &mut doc,
            &plan(json!({ "operations": [{ "op": "delete", "ids": [id.to_string()] }]})),
            &ctx,
        )
        .unwrap();
        assert_eq!(doc.count(Category::TextNote), 0);
    }

    #[test]
    fn plans_draw_detail_and_model_lines() {
        let mut doc = project();
        let plan_view = doc
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
        let ctx = EditContext {
            view: Some(plan_view),
            ..Default::default()
        };
        apply(
            &mut doc,
            &plan(json!({ "operations": [
                { "op": "create_detail_line", "value": "Hidden",
                  "points": [{"x": 0, "y": 0}, {"x": 10, "y": 0}, {"x": 10, "y": 5}] },
                { "op": "create_model_line", "start": {"x": 0, "y": 20}, "end": {"x": 30, "y": 20} }
            ]})),
            &ctx,
        )
        .unwrap();
        assert_eq!(doc.count(Category::DetailLine), 2);
        assert_eq!(doc.count(Category::ModelLine), 1);
        assert!(doc.of(Category::DetailLine).all(|e| matches!(&e.data,
            ElementData::DetailLine { style: studio_core::lines::LineStyle::Hidden, view, .. } if *view == plan_view)));
    }

    #[test]
    fn a_roof_over_part_of_a_level_sits_on_its_walls() {
        let mut doc = project();
        let ctx = EditContext::default();
        apply(&mut doc, &plan(json!({ "operations": [
            { "op": "create_wall", "start": {"x": 0, "y": 0}, "end": {"x": 12, "y": 0}, "height": "9'-0\"" },
            { "op": "create_wall", "start": {"x": 12, "y": 0}, "end": {"x": 12, "y": 10}, "height": "9'-0\"" },
            { "op": "create_wall", "start": {"x": 12, "y": 10}, "end": {"x": 0, "y": 10}, "height": "9'-0\"" },
            { "op": "create_wall", "start": {"x": 0, "y": 10}, "end": {"x": 0, "y": 0}, "height": "9'-0\"" },
            { "op": "create_roof", "slope": "4/12", "points": [
                {"x": -1, "y": -1}, {"x": 13, "y": -1}, {"x": 13, "y": 11}, {"x": -1, "y": 11}] }
        ]})), &ctx).unwrap();
        let offset = doc
            .of(Category::Roof)
            .find_map(|e| match &e.data {
                ElementData::Roof { offset, .. } => Some(*offset),
                _ => None,
            })
            .unwrap();
        assert!((offset - 9.0 * MM_PER_FT).abs() < 1e-6, "{offset}");
    }

    #[test]
    fn paint_with_a_new_colour_or_a_library_material() {
        let mut doc = project();
        let ctx = EditContext::default();
        apply(&mut doc, &plan(json!({ "operations": [
            { "op": "create_wall", "as": "w", "start": {"x": 0, "y": 0}, "end": {"x": 20, "y": 0} },
            { "op": "paint", "category": "Walls", "material": "Sky Blue", "color": "#5a8fc8" }
        ]})), &ctx).unwrap();
        let w = doc.of(Category::Wall).next().unwrap().id;
        let m = studio_core::paint::paint_of(&doc, w).expect("painted");
        assert_eq!(doc.data(m).unwrap().name(), "Sky Blue");
        let preset = studio_core::library::library().into_iter().next().unwrap();
        apply(
            &mut doc,
            &plan(json!({ "operations": [
                { "op": "paint", "ids": [w.to_string()], "material": preset.name }
            ]})),
            &ctx,
        )
        .unwrap();
        let m = studio_core::paint::paint_of(&doc, w).unwrap();
        assert_eq!(doc.data(m).unwrap().name(), preset.name);
    }

    #[test]
    fn a_single_bulk_change_reads_as_the_parameter_card() {
        let mut doc = project();
        let ctx = EditContext::default();
        apply(&mut doc, &plan(json!({ "operations": [
            { "op": "create_wall", "as": "w", "start": {"x": 0, "y": 0}, "end": {"x": 20, "y": 0} },
            { "op": "create_door", "wall": "$w", "offset": 5, "type": "Single Six-Panel 32" }
        ]})), &ctx).unwrap();
        let p = preview(&doc, &plan(json!({ "operations": [
            { "op": "set_parameter", "category": "Doors", "parameter": "Width", "value": "3'-0\"" }
        ]})), &ctx).unwrap();
        assert_eq!(
            (p.parameter.as_str(), p.from.as_str(), p.to.as_str()),
            ("Width", "2'-8\"", "3'-0\"")
        );
        assert_eq!(p.summary, "Width → 3'-0\" on 1 door");
    }
}
