//! Schedules: tables built from element queries.

use serde::Serialize;
use studio_core::units::{format_area_sf, format_ft_in};
use studio_core::{ops, Category, Document, ElementData, ElementId, ScheduleKind, ViewKind};
use studio_geom::Pt;
use studio_views::{Anchor, Builder, Dash, FillKind, Item};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Table {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Element each row describes (for selecting from the schedule).
    pub ids: Vec<ElementId>,
}

fn name_of(doc: &Document, id: ElementId) -> String {
    doc.data(id).map(|d| d.name()).unwrap_or_default()
}

/// Type name of an instance.
fn type_of(doc: &Document, id: ElementId) -> String {
    doc.data(id)
        .ok()
        .and_then(|d| d.type_id())
        .map(|t| name_of(doc, t))
        .unwrap_or_default()
}

/// A volume in cubic feet, e.g. "12.35 CF".
pub fn format_volume_cf(mm3: f64) -> String {
    format!("{:.2} CF", mm3 / 304.8f64.powi(3))
}

/// Level name of a wall-hosted element.
fn host_level(doc: &Document, host: ElementId) -> String {
    match doc.data(host) {
        Ok(ElementData::Wall { base_level, .. }) => name_of(doc, *base_level),
        _ => String::new(),
    }
}

/// The table for a schedule view, or None if `view` isn't a schedule.
pub fn schedule(doc: &Document, view: ElementId) -> Option<Table> {
    let ElementData::View {
        name,
        kind: ViewKind::Schedule { kind },
        ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    let mut rows: Vec<(String, ElementId, Vec<String>)> = vec![];
    let columns: Vec<&str> = match kind {
        ScheduleKind::Doors => {
            for e in doc.of(Category::Door) {
                let ElementData::Door {
                    type_id,
                    host,
                    mark,
                    ..
                } = &e.data
                else {
                    continue;
                };
                let (w, h) = match doc.data(*type_id) {
                    Ok(ElementData::DoorType { width, height, .. }) => (*width, *height),
                    _ => continue,
                };
                rows.push((
                    mark.clone(),
                    e.id,
                    vec![
                        mark.clone(),
                        name_of(doc, *type_id),
                        format_ft_in(w),
                        format_ft_in(h),
                        host_level(doc, *host),
                    ],
                ));
            }
            vec!["Mark", "Type", "Width", "Height", "Level"]
        }
        ScheduleKind::Windows => {
            for e in doc.of(Category::Window) {
                let ElementData::Window {
                    type_id,
                    host,
                    mark,
                    sill,
                    ..
                } = &e.data
                else {
                    continue;
                };
                let (w, h) = match doc.data(*type_id) {
                    Ok(ElementData::WindowType { width, height, .. }) => (*width, *height),
                    _ => continue,
                };
                rows.push((
                    mark.clone(),
                    e.id,
                    vec![
                        mark.clone(),
                        name_of(doc, *type_id),
                        format_ft_in(w),
                        format_ft_in(h),
                        format_ft_in(*sill),
                        host_level(doc, *host),
                    ],
                ));
            }
            vec!["Mark", "Type", "Width", "Height", "Sill", "Level"]
        }
        ScheduleKind::Rooms => {
            let model = studio_regen::regenerate(doc);
            for r in &model.rooms {
                let area = if r.boundary.is_some() {
                    format_area_sf(r.area())
                } else {
                    "Not enclosed".into()
                };
                rows.push((
                    r.number.clone(),
                    r.id,
                    vec![
                        r.number.clone(),
                        r.name.clone(),
                        name_of(doc, r.level),
                        area,
                    ],
                ));
            }
            vec!["Number", "Name", "Level", "Area"]
        }
        ScheduleKind::Sheets => {
            for (id, number, name) in ops::sheets(doc) {
                rows.push((number.clone(), id, vec![number, name]));
            }
            vec!["Sheet Number", "Sheet Name"]
        }
        ScheduleKind::Columns => {
            let model = studio_regen::regenerate(doc);
            for c in &model.columns {
                let Ok(ElementData::Column { top, .. }) = doc.data(c.id) else {
                    continue;
                };
                let mark = studio_core::detail::column_mark(doc, c.at);
                let top_level = match top {
                    studio_core::WallTop::UpToLevel { level, .. } => name_of(doc, *level),
                    studio_core::WallTop::Unconnected { .. } => "Unconnected".into(),
                };
                rows.push((
                    mark.clone(),
                    c.id,
                    vec![
                        mark,
                        type_of(doc, c.id),
                        name_of(doc, c.level),
                        top_level,
                        format_ft_in(c.z1 - c.z0),
                    ],
                ));
            }
            vec![
                "Column Location Mark",
                "Type",
                "Base Level",
                "Top Level",
                "Length",
            ]
        }
        ScheduleKind::Beams => {
            let model = studio_regen::regenerate(doc);
            for m in &model.beams {
                let ty = type_of(doc, m.id);
                rows.push((
                    format!("{ty} {}", name_of(doc, m.level)),
                    m.id,
                    vec![ty, name_of(doc, m.level), format_ft_in(m.start.dist(m.end))],
                ));
            }
            vec!["Type", "Reference Level", "Length"]
        }
        ScheduleKind::MaterialTakeoff => {
            let model = studio_regen::regenerate(doc);
            for r in studio_regen::takeoff::material_takeoff(doc, &model) {
                let id = doc
                    .of(Category::Material)
                    .find(|e| e.data.name() == r.material)
                    .map_or(view, |e| e.id);
                rows.push((
                    r.material.clone(),
                    id,
                    vec![
                        r.material,
                        if r.area > 0.0 {
                            format_area_sf(r.area)
                        } else {
                            String::new()
                        },
                        format_volume_cf(r.volume),
                    ],
                ));
            }
            vec!["Material", "Area", "Volume"]
        }
    };
    rows.sort_by(|a, b| ops::natural_cmp(&a.0, &b.0));
    Some(Table {
        title: name.to_uppercase(),
        columns: columns.into_iter().map(str::to_owned).collect(),
        ids: rows.iter().map(|r| r.1).collect(),
        rows: rows.into_iter().map(|r| r.2).collect(),
    })
}

/// Text height used in printed schedules, paper mm.
const TEXT: f64 = 2.5;
const ROW: f64 = 6.0;
const TITLE_ROW: f64 = 8.0;

/// Approximate width of `s` in Barlow Condensed at `size` mm (0.43 em per character).
pub fn approx_width(s: &str, size: f64) -> f64 {
    s.chars().count() as f64 * size * 0.43
}

/// Lays out a table in paper mm with its top-left corner at `top_left`. Returns the items and
/// the table's width and height.
pub fn table_items(t: &Table, el: Option<ElementId>, top_left: Pt) -> (Vec<Item>, f64, f64) {
    let pad = 2.5;
    let widths: Vec<f64> = (0..t.columns.len())
        .map(|c| {
            let longest = std::iter::once(t.columns[c].to_uppercase())
                .chain(t.rows.iter().map(|r| r.get(c).cloned().unwrap_or_default()))
                .map(|s| approx_width(&s, TEXT))
                .fold(0.0, f64::max);
            (longest + pad * 2.0).max(14.0)
        })
        .collect();
    let width: f64 = widths
        .iter()
        .sum::<f64>()
        .max(approx_width(&t.title, 3.2) + pad * 2.0);
    let height = TITLE_ROW + ROW * (t.rows.len() as f64 + 1.0);
    let mut b = Builder::new(1.0);
    let (x0, y0) = (top_left.x, top_left.y);
    let rect = |x: f64, y: f64, w: f64, h: f64| {
        vec![
            Pt::new(x, y),
            Pt::new(x + w, y),
            Pt::new(x + w, y - h),
            Pt::new(x, y - h),
        ]
    };
    b.fill(
        el,
        vec![studio_views::ring(&rect(x0, y0, width, height))],
        FillKind::Paper,
    );
    b.text(
        el,
        Pt::new(x0 + width / 2.0, y0 - TITLE_ROW / 2.0),
        t.title.clone(),
        3.2,
        Anchor::Center,
    );
    // Header band.
    let hy = y0 - TITLE_ROW;
    b.fill(
        el,
        vec![studio_views::ring(&rect(x0, hy, width, ROW))],
        FillKind::Slab,
    );
    let mut x = x0;
    for (c, w) in widths.iter().enumerate() {
        b.text(
            el,
            Pt::new(x + pad, hy - ROW / 2.0),
            t.columns[c].to_uppercase(),
            TEXT,
            Anchor::Left,
        );
        for (r, row) in t.rows.iter().enumerate() {
            let y = hy - ROW * (r as f64 + 1.5);
            b.text(
                el,
                Pt::new(x + pad, y),
                row.get(c).cloned().unwrap_or_default(),
                TEXT,
                Anchor::Left,
            );
        }
        x += w;
    }
    // Grid: outer box heavy, header rule medium, rows and columns fine.
    b.line(el, &rect(x0, y0, width, height), true, 4, Dash::Solid);
    b.line(
        el,
        &[Pt::new(x0, hy), Pt::new(x0 + width, hy)],
        false,
        3,
        Dash::Solid,
    );
    b.line(
        el,
        &[Pt::new(x0, hy - ROW), Pt::new(x0 + width, hy - ROW)],
        false,
        3,
        Dash::Solid,
    );
    for r in 1..t.rows.len() {
        let y = hy - ROW * (r as f64 + 1.0);
        b.line(
            el,
            &[Pt::new(x0, y), Pt::new(x0 + width, y)],
            false,
            1,
            Dash::Solid,
        );
    }
    let mut x = x0;
    for w in &widths[..widths.len().saturating_sub(1)] {
        x += w;
        b.line(
            el,
            &[Pt::new(x, hy), Pt::new(x, y0 - height)],
            false,
            1,
            Dash::Solid,
        );
    }
    (b.items, width, height)
}
