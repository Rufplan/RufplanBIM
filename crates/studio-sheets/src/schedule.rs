//! Schedules: tables built from element queries.

use serde::{Deserialize, Serialize};
use studio_core::params::ParamValue;
use studio_core::text::{sizes, TextFont};
use studio_core::units::{format_area_sf, format_ft_in};
use studio_core::{
    ops, Category, CoreError, CoreResult, Document, ElementData, ElementId, ScheduleKind, ViewKind,
};
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
    schedule_on(doc, view, None)
}

/// [`schedule`] as placed on `sheet`: a Keynote Legend then lists only that sheet's
/// keynotes, numbered as the sheet numbers them (ADR-081).
pub fn schedule_on(doc: &Document, view: ElementId, sheet: Option<ElementId>) -> Option<Table> {
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
            // The current design stage's set (every sheet when none are assigned), ADR-032.
            let stage = ops::project_info(doc).and_then(|i| match doc.data(i) {
                Ok(ElementData::ProjectInfo { current_stage, .. }) => *current_stage,
                _ => None,
            });
            for id in ops::stage_sheets(doc, stage) {
                if let Ok(ElementData::Sheet { number, name, .. }) = doc.data(id) {
                    rows.push((number.clone(), id, vec![number.clone(), name.clone()]));
                }
            }
            // Placeholder sheets (ADR-110): listed, though not in the project.
            if let Some(info) = ops::project_info(doc) {
                for p in placeholders(doc) {
                    rows.push((p.number.clone(), info, vec![p.number, p.name]));
                }
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
        ScheduleKind::Keynotes => {
            use studio_core::keynotes;
            let by_sheet = keynotes::table(doc).1 == keynotes::KeynoteNumbering::BySheet;
            let keys: Vec<(String, String)> = match sheet {
                Some(s) if by_sheet => keynotes::sheet_numbers(doc, s)
                    .into_iter()
                    .map(|(k, n)| (format!("{n:04}"), k))
                    .collect(),
                Some(s) => {
                    let views: Vec<ElementId> = doc
                        .of(Category::Viewport)
                        .filter_map(|e| match &e.data {
                            ElementData::Viewport {
                                sheet: vs, view, ..
                            } if *vs == s => Some(*view),
                            _ => None,
                        })
                        .collect();
                    keynotes::used_keys(doc, Some(&views))
                        .into_iter()
                        .map(|k| (k.clone(), k))
                        .collect()
                }
                None => keynotes::used_keys(doc, None)
                    .into_iter()
                    .map(|k| (k.clone(), k))
                    .collect(),
            };
            for (sort, key) in keys {
                let text = keynotes::text_of(doc, &key).unwrap_or_default();
                let shown = if by_sheet && sheet.is_some() {
                    sort.trim_start_matches('0').to_owned()
                } else {
                    key
                };
                rows.push((sort, view, vec![shown, text]));
            }
            vec![
                if by_sheet && sheet.is_some() {
                    "No."
                } else {
                    "Key Value"
                },
                "Keynote Text",
            ]
        }
    };
    // Sheets read in sheet-index (discipline) order; everything else naturally by key.
    if *kind == ScheduleKind::Sheets {
        // In the user's order where they've set one (ADR-113).
        let mut numbers: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
        crate::sheet_index::arrange(&mut numbers, &crate::sheet_index::order(doc));
        rows.sort_by_key(|r| numbers.iter().position(|n| *n == r.0));
    } else {
        rows.sort_by(|a, b| ops::natural_cmp(&a.0, &b.0));
    }
    Some(Table {
        title: name.to_uppercase(),
        columns: columns.into_iter().map(str::to_owned).collect(),
        ids: rows.iter().map(|r| r.1).collect(),
        rows: rows.into_iter().map(|r| r.2).collect(),
    })
}

/// How a schedule prints (ADR-110): its font, text sizes (paper mm) and row height, set per
/// schedule; the office's text types (ADR-109) when not set.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScheduleStyle {
    pub font: TextFont,
    pub title: f64,
    pub header: f64,
    pub body: f64,
    /// Body row height, paper mm.
    pub row: f64,
}

impl Default for ScheduleStyle {
    fn default() -> Self {
        ScheduleStyle {
            font: TextFont::Drafting,
            title: sizes::SCHEDULE_TITLE,
            header: sizes::SCHEDULE_HEADER,
            body: sizes::SCHEDULE_BODY,
            row: 6.0,
        }
    }
}

/// The schedule view's parameter holding its style.
pub const STYLE_KEY: &str = "rufplan.schedule.style";

/// A schedule's style: its own, else the default.
pub fn style_of(doc: &Document, view: ElementId) -> ScheduleStyle {
    match doc.param(view, STYLE_KEY) {
        Some(ParamValue::Text(s)) => serde_json::from_str(s).unwrap_or_default(),
        _ => ScheduleStyle::default(),
    }
}

/// Sets a schedule's style: text from 3/32" to 1", rows at least as tall as their text.
pub fn set_style(doc: &mut Document, view: ElementId, style: ScheduleStyle) -> CoreResult<()> {
    if !matches!(
        doc.data(view)?,
        ElementData::View {
            kind: ViewKind::Schedule { .. },
            ..
        }
    ) {
        return Err(CoreError::Invalid("that isn't a schedule".into()));
    }
    let size = |v: f64| v.clamp(sizes::MIN, 25.4);
    let style = ScheduleStyle {
        title: size(style.title),
        header: size(style.header),
        body: size(style.body),
        row: style.row.max(size(style.body) * 2.0).min(40.0),
        ..style
    };
    let json = serde_json::to_string(&style).map_err(|e| CoreError::Invalid(e.to_string()))?;
    doc.transact("Schedule appearance", |tx| {
        tx.set_param(view, STYLE_KEY, Some(ParamValue::Text(json.clone())))
    })
}

/// Approximate width of `s` in the drafting font at `size` mm (0.43 em per character).
pub fn approx_width(s: &str, size: f64) -> f64 {
    s.chars().count() as f64 * size * 0.43
}

/// Each column's width for a style, wide enough for its widest cell (paper mm).
fn column_widths(t: &Table, s: &ScheduleStyle) -> Vec<f64> {
    let pad = 2.5;
    let adv = s.font.advance();
    let w = |text: &str, size: f64| text.chars().count() as f64 * size * adv;
    (0..t.columns.len())
        .map(|c| {
            let longest = t
                .rows
                .iter()
                .map(|r| w(&r.get(c).cloned().unwrap_or_default(), s.body))
                .fold(w(&t.columns[c].to_uppercase(), s.header), f64::max);
            (longest + pad * 2.0).max(14.0)
        })
        .collect()
}

/// Lays out a table in paper mm with its top-left corner at `top_left`, in the default
/// style. Returns the items and the table's width and height.
pub fn table_items(t: &Table, el: Option<ElementId>, top_left: Pt) -> (Vec<Item>, f64, f64) {
    let s = ScheduleStyle::default();
    let widths = column_widths(t, &s);
    table_part(t, el, top_left, &s, &widths, 0..t.rows.len(), true)
}

/// One part of a table (ADR-110): rows `rows`, under the title when `title`, always under
/// the column headers, in `style` with the whole table's column `widths`.
fn table_part(
    t: &Table,
    el: Option<ElementId>,
    top_left: Pt,
    style: &ScheduleStyle,
    widths: &[f64],
    rows: std::ops::Range<usize>,
    title: bool,
) -> (Vec<Item>, f64, f64) {
    let pad = 2.5;
    let font = Some(style.font);
    let row = style.row.max(style.body * 2.0);
    let head_row = row.max(style.header * 2.0);
    let title_row = if title { style.title * 2.1 } else { 0.0 };
    let n = rows.len();
    let width: f64 = widths.iter().sum::<f64>().max(if title {
        t.title.chars().count() as f64 * style.title * style.font.advance() + pad * 2.0
    } else {
        0.0
    });
    let height = title_row + head_row + row * n as f64;
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
    if title {
        b.text_font(
            el,
            Pt::new(x0 + width / 2.0, y0 - title_row / 2.0),
            t.title.clone(),
            style.title,
            Anchor::Center,
            0.0,
            font,
        );
    }
    // Header band.
    let hy = y0 - title_row;
    b.fill(
        el,
        vec![studio_views::ring(&rect(x0, hy, width, head_row))],
        FillKind::Slab,
    );
    let mut x = x0;
    for (c, w) in widths.iter().enumerate() {
        b.text_font(
            el,
            Pt::new(x + pad, hy - head_row / 2.0),
            t.columns[c].to_uppercase(),
            style.header,
            Anchor::Left,
            0.0,
            font,
        );
        for (i, r) in rows.clone().enumerate() {
            let y = hy - head_row - row * (i as f64 + 0.5);
            b.text_font(
                el,
                Pt::new(x + pad, y),
                t.rows[r].get(c).cloned().unwrap_or_default(),
                style.body,
                Anchor::Left,
                0.0,
                font,
            );
        }
        x += w;
    }
    // Grid: outer box heavy, header rule medium, rows and columns fine.
    b.line(el, &rect(x0, y0, width, height), true, 4, Dash::Solid);
    if title {
        b.line(
            el,
            &[Pt::new(x0, hy), Pt::new(x0 + width, hy)],
            false,
            3,
            Dash::Solid,
        );
    }
    b.line(
        el,
        &[
            Pt::new(x0, hy - head_row),
            Pt::new(x0 + width, hy - head_row),
        ],
        false,
        3,
        Dash::Solid,
    );
    for i in 1..n {
        let y = hy - head_row - row * i as f64;
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

// ---------------------------------------------------------------------------------------
// Split schedules (ADR-110)
// ---------------------------------------------------------------------------------------

/// The viewport's parameter holding how its schedule is split.
pub const SPLIT_KEY: &str = "rufplan.schedule.split";

/// A schedule split on its sheet, as Revit's Split Schedule Table: the rows each later part
/// starts at, and where each later part's centre is (paper mm). The first part is where the
/// viewport is.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub breaks: Vec<usize>,
    pub centers: Vec<Pt>,
}

pub fn split_of(doc: &Document, viewport: ElementId) -> Split {
    match doc.param(viewport, SPLIT_KEY) {
        Some(ParamValue::Text(s)) => serde_json::from_str::<Split>(s)
            .ok()
            .filter(|s| s.breaks.len() == s.centers.len())
            .unwrap_or_default(),
        _ => Split::default(),
    }
}

fn set_split(doc: &mut Document, viewport: ElementId, split: &Split, what: &str) -> CoreResult<()> {
    let value = if split.breaks.is_empty() {
        None
    } else {
        Some(ParamValue::Text(
            serde_json::to_string(split).map_err(|e| CoreError::Invalid(e.to_string()))?,
        ))
    };
    doc.transact(what, |tx| tx.set_param(viewport, SPLIT_KEY, value.clone()))
}

/// A schedule's parts on its sheet: (items, min, max) of each, the first at the viewport,
/// in the schedule's style, its headers repeated, the title on the first only.
pub fn schedule_parts(
    doc: &Document,
    viewport: ElementId,
    view: ElementId,
    center: Pt,
) -> Option<Vec<(Vec<Item>, Pt, Pt)>> {
    let on = match doc.data(viewport) {
        Ok(ElementData::Viewport { sheet, .. }) => Some(*sheet),
        _ => None,
    };
    let table = schedule_on(doc, view, on)?;
    let style = style_of(doc, view);
    let widths = column_widths(&table, &style);
    let split = split_of(doc, viewport);
    let n = table.rows.len();
    let mut starts: Vec<usize> = std::iter::once(0)
        .chain(split.breaks.iter().copied().filter(|b| *b > 0 && *b < n))
        .collect();
    starts.dedup();
    let mut out = vec![];
    for (i, &s) in starts.iter().enumerate() {
        let e = starts.get(i + 1).copied().unwrap_or(n);
        let range = s..e;
        let (_, w, h) = table_part(
            &table,
            None,
            Pt::default(),
            &style,
            &widths,
            range.clone(),
            i == 0,
        );
        let c = if i == 0 {
            center
        } else {
            split.centers.get(i - 1).copied().unwrap_or(center)
        };
        let lo = Pt::new(c.x - w / 2.0, c.y - h / 2.0);
        let (items, _, _) = table_part(
            &table,
            Some(viewport),
            Pt::new(lo.x, lo.y + h),
            &style,
            &widths,
            range,
            i == 0,
        );
        out.push((items, lo, Pt::new(lo.x + w, lo.y + h)));
    }
    Some(out)
}

/// Split Schedule Table (ADR-110): halves the part with the most rows; the new part goes
/// just right of the rightmost part, its top level with the first's.
pub fn split_schedule(doc: &mut Document, viewport: ElementId) -> CoreResult<()> {
    let (view, center) = match doc.data(viewport)? {
        ElementData::Viewport { view, center, .. } => (*view, *center),
        _ => return Err(CoreError::Invalid("select a schedule on a sheet".into())),
    };
    let parts = schedule_parts(doc, viewport, view, center)
        .ok_or_else(|| CoreError::Invalid("select a schedule on a sheet".into()))?;
    let rows = schedule(doc, view).map_or(0, |t| t.rows.len());
    let split = split_of(doc, viewport);
    let mut starts: Vec<usize> = std::iter::once(0)
        .chain(split.breaks.iter().copied())
        .collect();
    starts.push(rows);
    let (k, len) = starts
        .windows(2)
        .enumerate()
        .map(|(i, w)| (i, w[1].saturating_sub(w[0])))
        .max_by_key(|x| x.1)
        .unwrap_or((0, 0));
    if len < 2 {
        return Err(CoreError::Invalid(
            "the schedule is too short to split".into(),
        ));
    }
    let at = starts[k] + len / 2;
    let right = parts.iter().map(|p| p.2.x).fold(f64::MIN, f64::max);
    let top = parts[0].2.y;
    // The new part's own size: its rows under the headers, no title.
    let sheet = match doc.data(viewport)? {
        ElementData::Viewport { sheet, .. } => Some(*sheet),
        _ => None,
    };
    let table = schedule_on(doc, view, sheet)
        .ok_or_else(|| CoreError::Invalid("select a schedule on a sheet".into()))?;
    let style = style_of(doc, view);
    let widths = column_widths(&table, &style);
    let (_, w, h) = table_part(
        &table,
        None,
        Pt::default(),
        &style,
        &widths,
        at..starts[k + 1],
        false,
    );
    let c = Pt::new(right + 8.0 + w / 2.0, top - h / 2.0);
    let mut pairs: Vec<(usize, Pt)> = split.breaks.into_iter().zip(split.centers).collect();
    pairs.push((at, c));
    pairs.sort_by_key(|p| p.0);
    let split = Split {
        breaks: pairs.iter().map(|p| p.0).collect(),
        centers: pairs.iter().map(|p| p.1).collect(),
    };
    // The first part keeps its top, as in Revit: the viewport moves down by what it lost.
    let first_end = split.breaks.first().copied().unwrap_or(rows);
    let (_, _, h0) = table_part(
        &table,
        None,
        Pt::default(),
        &style,
        &widths,
        0..first_end,
        true,
    );
    let new_center = Pt::new(center.x, top - h0 / 2.0);
    let json = serde_json::to_string(&split).map_err(|e| CoreError::Invalid(e.to_string()))?;
    doc.transact("Split schedule", |tx| {
        tx.modify(viewport, |d| {
            if let ElementData::Viewport { center: c, .. } = d {
                *c = new_center;
            }
        })?;
        tx.set_param(viewport, SPLIT_KEY, Some(ParamValue::Text(json.clone())))
    })
}

/// Joins a split schedule back into one.
pub fn join_schedule(doc: &mut Document, viewport: ElementId) -> CoreResult<()> {
    set_split(doc, viewport, &Split::default(), "Join schedule")
}

/// Drags part `i` (1 and on) of a split schedule to centre `to`.
pub fn move_part(doc: &mut Document, viewport: ElementId, i: usize, to: Pt) -> CoreResult<()> {
    let mut split = split_of(doc, viewport);
    let Some(c) = i.checked_sub(1).and_then(|j| split.centers.get_mut(j)) else {
        return Err(CoreError::Invalid("no such part".into()));
    };
    *c = to;
    set_split(doc, viewport, &split, "Move schedule part")
}

// ---------------------------------------------------------------------------------------
// Placeholder sheets (ADR-110)
// ---------------------------------------------------------------------------------------

/// Project Info's parameter listing the placeholder sheets.
pub const PLACEHOLDER_KEY: &str = "rufplan.placeholder_sheets";

/// A sheet listed in the sheet index that isn't in the project, as Revit's placeholder
/// sheets: a consultant's sheet, or one drawn elsewhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlaceholderSheet {
    pub number: String,
    pub name: String,
}

pub fn placeholders(doc: &Document) -> Vec<PlaceholderSheet> {
    ops::project_info(doc)
        .and_then(|i| match doc.param(i, PLACEHOLDER_KEY) {
            Some(ParamValue::Text(s)) => serde_json::from_str(s).ok(),
            _ => None,
        })
        .unwrap_or_default()
}

/// Sets the placeholder sheets; numbers must be given and unique, and not a real sheet's.
pub fn set_placeholders(doc: &mut Document, list: Vec<PlaceholderSheet>) -> CoreResult<()> {
    let info = ops::project_info(doc)
        .ok_or_else(|| CoreError::Invalid("project information is missing".into()))?;
    let real: Vec<String> = ops::sheets(doc).into_iter().map(|s| s.1).collect();
    let mut seen = std::collections::BTreeSet::new();
    let list: Vec<PlaceholderSheet> = list
        .into_iter()
        .map(|p| PlaceholderSheet {
            number: p.number.trim().to_string(),
            name: p.name.trim().to_string(),
        })
        .collect();
    for p in &list {
        if p.number.is_empty() {
            return Err(CoreError::Invalid(
                "give each placeholder sheet a number".into(),
            ));
        }
        if real.contains(&p.number) || !seen.insert(p.number.clone()) {
            return Err(CoreError::Invalid(format!(
                "sheet {} already exists",
                p.number
            )));
        }
    }
    let value = if list.is_empty() {
        None
    } else {
        Some(ParamValue::Text(
            serde_json::to_string(&list).map_err(|e| CoreError::Invalid(e.to_string()))?,
        ))
    };
    doc.transact("Placeholder sheets", |tx| {
        tx.set_param(info, PLACEHOLDER_KEY, value.clone())
    })
}

#[cfg(test)]
mod keynote_tests {
    use studio_core::keynotes::{self, KeynoteNumbering, KeynoteSource, KeynoteStyle};
    use studio_core::{ops, Category, Document, SheetSize};
    use studio_geom::Pt;

    #[test]
    fn the_keynote_legend_lists_the_keys_used_filtered_to_its_sheet() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let plans: Vec<_> = doc
            .of(Category::View)
            .filter(|e| matches!(e.data.name().as_str(), "Level 1" | "Level 2"))
            .map(|e| e.id)
            .collect();
        let user = |k: &str| KeynoteSource::User { key: k.into() };
        keynotes::create_tag(
            &mut doc,
            plans[0],
            user("09 29 00.A1"),
            None,
            Pt::new(0.0, 0.0),
            KeynoteStyle::Key,
        )
        .unwrap();
        keynotes::create_tag(
            &mut doc,
            plans[0],
            user("03 30 00.A1"),
            None,
            Pt::new(0.0, 900.0),
            KeynoteStyle::Key,
        )
        .unwrap();
        keynotes::create_tag(
            &mut doc,
            plans[1],
            user("07 21 00.A1"),
            None,
            Pt::new(0.0, 0.0),
            KeynoteStyle::Key,
        )
        .unwrap();
        let legend = keynotes::legend(&mut doc).unwrap();
        assert_eq!(
            keynotes::legend(&mut doc).unwrap(),
            legend,
            "one legend per project"
        );
        // Unplaced: every key used, in key order.
        let t = super::schedule(&doc, legend).unwrap();
        assert_eq!(t.columns, ["Key Value", "Keynote Text"]);
        let keys: Vec<&str> = t.rows.iter().map(|r| r[0].as_str()).collect();
        assert_eq!(keys, ["03 30 00.A1", "07 21 00.A1", "09 29 00.A1"]);
        assert_eq!(t.rows[0][1], "4\" concrete slab on grade");
        // On a sheet with Level 1 only: its two keys.
        let sheet = ops::create_sheet(&mut doc, "Plans", SheetSize::ArchD).unwrap();
        ops::place_view(&mut doc, sheet, plans[0], Pt::new(300.0, 300.0)).unwrap();
        let t = super::schedule_on(&doc, legend, Some(sheet)).unwrap();
        assert_eq!(t.rows.len(), 2);
        // By Sheet: numbered 1, 2.
        keynotes::set_numbering(&mut doc, KeynoteNumbering::BySheet).unwrap();
        let t = super::schedule_on(&doc, legend, Some(sheet)).unwrap();
        assert_eq!(t.columns[0], "No.");
        assert_eq!((t.rows[0][0].as_str(), t.rows[1][0].as_str()), ("1", "2"));
        assert_eq!(t.rows[1][1], "1/2\" gypsum board");
    }
}

#[cfg(test)]
mod split_tests {
    use super::*;
    use studio_core::SheetSize;

    /// A project with ten sheets and the sheet index placed on the first.
    fn indexed() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        for i in 0..10 {
            ops::create_sheet(&mut doc, &format!("Sheet {i}"), SheetSize::ArchD).unwrap();
        }
        let sheet = ops::sheets(&doc)[0].0;
        ops::ensure_schedules(&mut doc).unwrap();
        let index = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Schedule {
                            kind: ScheduleKind::Sheets
                        },
                        ..
                    }
                )
            })
            .map(|e| e.id)
            .unwrap();
        let vp = ops::place_view(&mut doc, sheet, index, Pt::new(300.0, 300.0)).unwrap();
        (doc, index, vp)
    }

    #[test]
    fn schedules_split_into_parts_that_move_on_their_own_and_join() {
        let (mut doc, index, vp) = indexed();
        let parts = |doc: &Document| {
            let c = match doc.data(vp).unwrap() {
                ElementData::Viewport { center, .. } => *center,
                _ => unreachable!(),
            };
            schedule_parts(doc, vp, index, c).unwrap()
        };
        assert_eq!(parts(&doc).len(), 1);
        split_schedule(&mut doc, vp).unwrap();
        let p = parts(&doc);
        assert_eq!(p.len(), 2);
        // The new part sits right of the first, tops level, headers repeated (no title).
        assert!(p[1].1.x > p[0].2.x);
        assert!((p[1].2.y - p[0].2.y).abs() < 1.0);
        let text = |items: &[Item]| {
            items
                .iter()
                .filter_map(|i| match &i.prim {
                    studio_views::Prim::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(text(&p[1].0).contains(&"SHEET NUMBER".to_string()));
        assert!(!text(&p[1].0).contains(&"SHEET INDEX".to_string()));
        // Every row once across the parts.
        let rows = schedule(&doc, index).unwrap().rows.len();
        let numbers: usize = p
            .iter()
            .map(|x| text(&x.0).iter().filter(|t| t.starts_with("A")).count())
            .sum();
        assert!(numbers >= rows - 1, "{numbers} of {rows}");
        // The second part drags on its own.
        move_part(&mut doc, vp, 1, Pt::new(500.0, 200.0)).unwrap();
        let c = parts(&doc)[1].1.lerp(parts(&doc)[1].2, 0.5);
        assert!(c.dist(Pt::new(500.0, 200.0)) < 1e-6);
        join_schedule(&mut doc, vp).unwrap();
        assert_eq!(parts(&doc).len(), 1);
    }

    #[test]
    fn a_schedules_style_sets_its_text_and_placeholders_join_the_sheet_index() {
        let (mut doc, index, vp) = indexed();
        let style = ScheduleStyle {
            font: TextFont::Sans,
            title: 6.35,
            header: 1.0,
            ..ScheduleStyle::default()
        };
        set_style(&mut doc, index, style).unwrap();
        let s = style_of(&doc, index);
        assert_eq!(s.font, TextFont::Sans);
        assert!((s.header - sizes::MIN).abs() < 1e-9, "never under 3/32\"");
        let p = schedule_parts(&doc, vp, index, Pt::new(300.0, 300.0)).unwrap();
        assert!(p[0].0.iter().any(|i| matches!(&i.prim,
            studio_views::Prim::Text { font: Some(TextFont::Sans), size, .. } if (size - 6.35).abs() < 1e-9)));
        // Placeholder sheets list in the index, in sheet order, and can't take a real number.
        set_placeholders(
            &mut doc,
            vec![PlaceholderSheet {
                number: "S-101".into(),
                name: "Foundation Plan (by others)".into(),
            }],
        )
        .unwrap();
        let t = schedule(&doc, index).unwrap();
        assert!(t.rows.iter().any(|r| r[0] == "S-101"));
        let real = ops::sheets(&doc)[0].1.clone();
        assert!(set_placeholders(
            &mut doc,
            vec![PlaceholderSheet {
                number: real,
                name: "x".into()
            }]
        )
        .is_err());
    }
}
