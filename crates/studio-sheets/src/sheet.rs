//! Sheet layout in paper millimetres (origin at the sheet's bottom-left): the Rufplan title
//! block plus viewports, each a view's display list scaled from model mm to paper mm.

use studio_core::{ops, Category, Document, ElementData, ElementId, SheetSize, ViewKind};
use studio_geom::Pt;
use studio_views::{Anchor, Builder, Dash, DisplayList, FillKind, Item, Prim, ViewType};

use crate::schedule::{approx_width, schedule, table_items};

/// Border margins in paper mm: (left binding edge, other edges).
pub(crate) fn margins(size: SheetSize) -> (f64, f64) {
    match size {
        SheetSize::ArchD => (25.4, 12.7),
        SheetSize::Tabloid => (19.0, 9.5),
    }
}

/// Width of the title block strip on the right, paper mm.
pub fn title_block_width(size: SheetSize) -> f64 {
    match size {
        SheetSize::ArchD => 88.9,
        SheetSize::Tabloid => 63.5,
    }
}

/// Maps every point of `items` through `f`, scales sizes by `k` and retags them with `el`.
fn transform(items: Vec<Item>, f: impl Fn(Pt) -> Pt, k: f64, el: Option<ElementId>) -> Vec<Item> {
    let map = |p: [f64; 2]| {
        let q = f(Pt::new(p[0], p[1]));
        [q.x, q.y]
    };
    items
        .into_iter()
        .map(|it| {
            let prim = match it.prim {
                Prim::Line {
                    pts,
                    closed,
                    w,
                    dash,
                } => Prim::Line {
                    pts: pts.into_iter().map(map).collect(),
                    closed,
                    w,
                    dash,
                },
                Prim::Fill { rings, fill } => Prim::Fill {
                    rings: rings
                        .into_iter()
                        .map(|r| r.into_iter().map(map).collect())
                        .collect(),
                    fill,
                },
                Prim::Text {
                    at,
                    text,
                    size,
                    anchor,
                    angle,
                } => Prim::Text {
                    at: map(at),
                    text,
                    size: size * k,
                    anchor,
                    angle,
                },
                Prim::Circle { c, r, w, filled } => Prim::Circle {
                    c: map(c),
                    r: r * k,
                    w,
                    filled,
                },
            };
            Item {
                el: el.or(it.el),
                prim,
            }
        })
        .collect()
}

/// A viewport's drawing: the view scaled onto paper and centered at `center`, plus its
/// extent (w, h) in paper mm. Schedules are laid out as tables.
pub fn viewport_items(
    doc: &Document,
    viewport: ElementId,
    view: ElementId,
    center: Pt,
) -> Option<(Vec<Item>, f64, f64, String, String)> {
    let ElementData::View {
        name, kind, scale, ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    if matches!(kind, ViewKind::Schedule { .. }) {
        let table = schedule(doc, view)?;
        let (_, w, h) = table_items(&table, Some(viewport), Pt::default());
        let (items, _, _) = table_items(
            &table,
            Some(viewport),
            Pt::new(center.x - w / 2.0, center.y + h / 2.0),
        );
        return Some((items, w, h, String::new(), String::new()));
    }
    let dl = studio_views::display_list(doc, view)?;
    let s = f64::from(*scale);
    let [x0, y0, x1, y1] = dl.bounds;
    let c = Pt::new((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    // The crop boundary (drawn with the view as its element) and cameras never print.
    let drawn: Vec<Item> = dl
        .items
        .into_iter()
        .filter(|it| {
            it.el != Some(view)
                && !it
                    .el
                    .is_some_and(|e| studio_core::camera::camera_of(doc, e).is_some())
        })
        .collect();
    let items = transform(
        drawn,
        |p| center.add(p.sub(c).scale(1.0 / s)),
        1.0 / s,
        Some(viewport),
    );
    Some((
        items,
        (x1 - x0) / s,
        (y1 - y0) / s,
        name.clone(),
        ops::scale_label(*scale),
    ))
}

/// Sheet display lists by sheet, with the stamp and date they were made for.
#[derive(Default)]
struct SheetCache(std::collections::HashMap<ElementId, (u64, String, std::sync::Arc<DisplayList>)>);

/// Like [`sheet_display_list`], shared from the document's cache while the model is
/// unchanged (hovering over a sheet picks on every mouse move).
pub fn sheet_display_list_shared(
    doc: &Document,
    sheet: ElementId,
    date: &str,
) -> Option<std::sync::Arc<DisplayList>> {
    use std::sync::{Arc, Mutex};
    const KEY: &str = "studio-sheets";
    let cache = doc
        .derived()
        .get::<Mutex<SheetCache>>(KEY)
        .unwrap_or_else(|| {
            let c = Arc::new(Mutex::new(SheetCache::default()));
            doc.derived().put(KEY, c.clone());
            c
        });
    let stamp = doc.stamp();
    if let Some((s, d, dl)) = cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .0
        .get(&sheet)
    {
        if *s == stamp && d == date {
            return Some(dl.clone());
        }
    }
    let dl = Arc::new(sheet_display_list(doc, sheet, date)?);
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .0
        .insert(sheet, (stamp, date.to_owned(), dl.clone()));
    Some(dl)
}

/// The display list of a sheet, in paper mm. `date` is printed in the title block.
pub fn sheet_display_list(doc: &Document, sheet: ElementId, date: &str) -> Option<DisplayList> {
    let ElementData::Sheet {
        number, name, size, ..
    } = doc.data(sheet).ok()?
    else {
        return None;
    };
    let (w, h) = size.mm();
    let mut b = Builder::new(1.0);
    b.fill(
        None,
        vec![studio_views::ring(&[
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ])],
        FillKind::Paper,
    );

    let mut vps: Vec<(ElementId, ElementId, Pt)> = doc
        .of(Category::Viewport)
        .filter_map(|e| match &e.data {
            ElementData::Viewport {
                sheet: s,
                view,
                center,
            } if *s == sheet => Some((e.id, *view, *center)),
            _ => None,
        })
        .collect();
    vps.sort_by_key(|v| v.0);
    for (i, (vp, view, center)) in vps.iter().enumerate() {
        let Some((items, _, _, title, scale)) = viewport_items(doc, *vp, *view, *center) else {
            continue;
        };
        // Titles sit under what is actually drawn (not the view's padded bounds), kept inside
        // the border.
        let (lo, _) = extents(&items).unwrap_or((*center, *center));
        b.items.extend(items);
        if !title.is_empty() {
            view_title(
                &mut b,
                Some(*vp),
                i + 1,
                &title,
                &scale,
                Pt::new(lo.x.max(margins(*size).0 + 4.0), lo.y - 4.0),
            );
        }
    }
    // Text notes placed on the sheet itself (e.g. a cover title), in paper mm.
    studio_views::annotations(doc, &mut b, sheet);
    title_block(doc, &mut b, sheet, *size, number, name, date);
    Some(DisplayList {
        view_type: ViewType::Sheet,
        scale: 1,
        bounds: [0.0, 0.0, w, h],
        items: b.items,
    })
}

/// Bounding box (min, max) of every point in `items`.
fn extents(items: &[Item]) -> Option<(Pt, Pt)> {
    let mut pts = vec![];
    for it in items {
        match &it.prim {
            Prim::Line { pts: p, .. } => pts.extend(p.iter().map(|q| Pt::new(q[0], q[1]))),
            Prim::Fill { rings, .. } => {
                pts.extend(rings.iter().flatten().map(|q| Pt::new(q[0], q[1])))
            }
            Prim::Text { at, .. } => pts.push(Pt::new(at[0], at[1])),
            Prim::Circle { c, r, .. } => {
                pts.push(Pt::new(c[0] - r, c[1] - r));
                pts.push(Pt::new(c[0] + r, c[1] + r));
            }
        }
    }
    studio_regen::bounds(&pts)
}

/// The building's outline on its lowest level, fitted into a `w` × `h` box at `origin`.
fn key_plan(doc: &Document, b: &mut Builder, origin: Pt, w: f64, h: f64) {
    let model = studio_regen::regenerate(doc);
    let Some(level) = doc.levels().first().map(|l| l.0) else {
        return;
    };
    let regions = studio_regen::wall_regions(&model, level);
    let pts: Vec<Pt> = regions
        .iter()
        .flat_map(|r| r.outer.iter().copied())
        .collect();
    let Some((lo, hi)) = studio_regen::bounds(&pts) else {
        return;
    };
    let (bw, bh) = ((hi.x - lo.x).max(1.0), (hi.y - lo.y).max(1.0));
    let k = (w / bw).min(h / bh);
    let off = origin.add(Pt::new((w - bw * k) / 2.0, (h - bh * k) / 2.0));
    let map = |p: &Pt| off.add(p.sub(lo).scale(k));
    for r in &regions {
        let outline: Vec<Pt> = r.outer.iter().map(map).collect();
        b.fill(None, vec![studio_views::ring(&outline)], FillKind::Slab);
        b.line(None, &outline, true, 3, Dash::Solid);
    }
}

/// A north arrow (plan north is +y) at `c` with radius `r` paper mm.
fn north_arrow(b: &mut Builder, c: Pt, r: f64) {
    b.circle(None, c, r, 2, false);
    let tip = c.add(Pt::new(0.0, r * 0.95));
    b.fill(
        None,
        vec![studio_views::ring(&[
            tip,
            c.add(Pt::new(r * 0.45, -r * 0.6)),
            c,
            c.add(Pt::new(-r * 0.45, -r * 0.6)),
        ])],
        FillKind::Ink,
    );
    b.text(
        None,
        c.add(Pt::new(0.0, r + 3.0)),
        "N".into(),
        3.0,
        Anchor::Center,
    );
}

/// Revit-style view title: number bubble, name above a heavy rule, scale below it.
fn view_title(b: &mut Builder, el: Option<ElementId>, n: usize, name: &str, scale: &str, at: Pt) {
    let r = 4.0;
    let c = at.add(Pt::new(r, -r));
    b.circle(el, c, r, 3, false);
    b.text(el, c, n.to_string(), 3.2, Anchor::Center);
    let x0 = c.x + r + 2.0;
    let len = approx_width(name, 4.0).max(approx_width(scale, 2.6)) + 6.0;
    b.line(
        el,
        &[Pt::new(x0, c.y), Pt::new(x0 + len, c.y)],
        false,
        5,
        Dash::Solid,
    );
    b.text(
        el,
        Pt::new(x0, c.y + 2.8),
        name.to_uppercase(),
        4.0,
        Anchor::Left,
    );
    b.text(
        el,
        Pt::new(x0, c.y - 2.6),
        scale.to_owned(),
        2.6,
        Anchor::Left,
    );
}

fn title_block(
    doc: &Document,
    b: &mut Builder,
    sheet: ElementId,
    size: SheetSize,
    number: &str,
    name: &str,
    date: &str,
) {
    let (w, h) = size.mm();
    let (left, m) = margins(size);
    let k = if size == SheetSize::ArchD { 1.0 } else { 0.72 };
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ]
    };
    // Border.
    b.line(None, &rect(left, m, w - m, h - m), true, 5, Dash::Solid);
    let tb = title_block_width(size);
    let (x0, x1) = (w - m - tb, w - m);
    b.line(
        None,
        &[Pt::new(x0, m), Pt::new(x0, h - m)],
        false,
        5,
        Dash::Solid,
    );

    let (project, number_p, client, address, stage) =
        match ops::project_info(doc).and_then(|i| doc.data(i).ok()) {
            Some(ElementData::ProjectInfo {
                name,
                number,
                client,
                address,
                current_stage,
                ..
            }) => {
                let stage = current_stage
                    .and_then(|s| doc.data(s).ok())
                    .and_then(|s| match s {
                        ElementData::Stage {
                            name, abbreviation, ..
                        } => Some((name.clone(), abbreviation.clone())),
                        _ => None,
                    });
                (
                    name.clone(),
                    number.clone(),
                    client.clone(),
                    address.clone(),
                    stage,
                )
            }
            _ => (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                None,
            ),
        };

    let pad = 5.0 * k;
    let tx = x0 + pad;
    let mut y = h - m;
    // Cyan accent bar and the Rufplan wordmark.
    b.fill(
        None,
        vec![studio_views::ring(&rect(x0, y - 5.0 * k, x1, y))],
        FillKind::Accent,
    );
    y -= 5.0 * k + 11.0 * k;
    b.text(
        None,
        Pt::new(tx, y),
        "RUFPLAN".into(),
        11.0 * k,
        Anchor::Left,
    );
    y -= 7.0 * k;
    b.text(None, Pt::new(tx, y), "STUDIO".into(), 4.0 * k, Anchor::Left);
    y -= 7.0 * k;
    let rule = |b: &mut Builder, y: f64| {
        b.line(
            None,
            &[Pt::new(x0, y), Pt::new(x1, y)],
            false,
            3,
            Dash::Solid,
        )
    };
    rule(b, y);

    let label = |b: &mut Builder, y: &mut f64, text: &str| {
        *y -= 5.5 * k;
        b.text(
            None,
            Pt::new(tx, *y),
            text.to_owned(),
            2.2 * k,
            Anchor::Left,
        );
    };
    let value = |b: &mut Builder, y: &mut f64, text: &str, size: f64| {
        if text.is_empty() {
            return;
        }
        *y -= size * k * 1.35;
        b.text(
            None,
            Pt::new(tx, *y),
            text.to_owned(),
            size * k,
            Anchor::Left,
        );
    };
    label(b, &mut y, "PROJECT");
    value(b, &mut y, &project.to_uppercase(), 5.0);
    value(b, &mut y, &address, 3.0);
    value(b, &mut y, &client, 3.0);
    if !number_p.is_empty() {
        value(b, &mut y, &format!("PROJECT NO. {number_p}"), 3.0);
    }
    y -= 4.0 * k;
    rule(b, y);
    label(b, &mut y, "DESIGN STAGE");
    if let Some((stage_name, abbr)) = &stage {
        value(b, &mut y, &stage_name.to_uppercase(), 4.2);
        y -= 16.0 * k;
        b.text(None, Pt::new(tx, y), abbr.clone(), 16.0 * k, Anchor::Left);
        y -= 6.0 * k;
    }
    rule(b, y);
    label(b, &mut y, "DATE");
    value(b, &mut y, date, 3.4);
    y -= 4.0 * k;
    rule(b, y);

    // Issue block: every issued set that included this sheet (newest last).
    label(b, &mut y, "ISSUES");
    let issues = ops::sheet_issues(doc, sheet);
    if issues.is_empty() {
        value(b, &mut y, "—", 2.6);
    }
    for (issue, date, abbr) in issues.iter().rev().take(6).rev() {
        y -= 2.6 * k * 1.5;
        b.text(None, Pt::new(tx, y), date.clone(), 2.4 * k, Anchor::Left);
        b.text(
            None,
            Pt::new(tx + 18.0 * k, y),
            abbr.clone(),
            2.4 * k,
            Anchor::Left,
        );
        b.text(
            None,
            Pt::new(tx + 27.0 * k, y),
            issue.to_uppercase(),
            2.4 * k,
            Anchor::Left,
        );
    }
    y -= 4.0 * k;
    rule(b, y);

    // Key plan with north arrow, just above the sheet name block.
    let key_top = m + 42.0 * k + 62.0 * k;
    if y > key_top + 2.0 {
        b.line(
            None,
            &[Pt::new(x0, key_top), Pt::new(x1, key_top)],
            false,
            3,
            Dash::Solid,
        );
        b.text(
            None,
            Pt::new(tx, key_top - 5.5 * k),
            "KEY PLAN".into(),
            2.2 * k,
            Anchor::Left,
        );
        key_plan(
            doc,
            b,
            Pt::new(tx, m + 42.0 * k + 5.0 * k),
            x1 - tx - pad - 14.0 * k,
            44.0 * k,
        );
        north_arrow(
            b,
            Pt::new(x1 - pad - 5.0 * k, m + 42.0 * k + 12.0 * k),
            4.5 * k,
        );
    }

    // Sheet name and number at the bottom.
    let bottom = m;
    b.line(
        None,
        &[
            Pt::new(x0, bottom + 42.0 * k),
            Pt::new(x1, bottom + 42.0 * k),
        ],
        false,
        3,
        Dash::Solid,
    );
    b.text(
        None,
        Pt::new(tx, bottom + 37.0 * k),
        "SHEET".into(),
        2.2 * k,
        Anchor::Left,
    );
    b.text(
        None,
        Pt::new(tx, bottom + 30.0 * k),
        name.to_uppercase(),
        4.6 * k,
        Anchor::Left,
    );
    b.text(
        None,
        Pt::new(tx, bottom + 13.0 * k),
        number.to_owned(),
        17.0 * k,
        Anchor::Left,
    );
}
