//! Sheet layout in paper millimetres (origin at the sheet's bottom-left): the Rufplan title
//! block plus viewports, each a view's display list scaled from model mm to paper mm.

use studio_core::text::sizes;
use studio_core::{ops, Category, Document, ElementData, ElementId, SheetSize, ViewKind};
use studio_geom::Pt;
use studio_views::{Anchor, Builder, Dash, DisplayList, FillKind, Item, Prim, ViewType};

use crate::schedule::{approx_width, table_items};

/// Border margins in paper mm: (left binding edge, other edges).
pub fn margins(size: SheetSize) -> (f64, f64) {
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
                Prim::Image { image, min, max } => {
                    let (a, b) = (map(min), map(max));
                    Prim::Image {
                        image,
                        min: [a[0].min(b[0]), a[1].min(b[1])],
                        max: [a[0].max(b[0]), a[1].max(b[1])],
                    }
                }
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
        let on = match doc.data(viewport) {
            Ok(ElementData::Viewport { sheet, .. }) => Some(*sheet),
            _ => None,
        };
        let table = crate::schedule::schedule_on(doc, view, on)?;
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
        // A rendering has no scale (ADR-095).
        if matches!(kind, ViewKind::Rendering { .. }) {
            String::new()
        } else {
            ops::scale_label(*scale)
        },
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

    for (i, (vp, view, center, length, offset)) in viewports_on(doc, sheet).into_iter().enumerate()
    {
        let Some((items, _, _, title, scale)) = viewport_items(doc, vp, view, center) else {
            continue;
        };
        let at = title_at(&items, center, *size, offset);
        let length = length.or_else(|| full_width(&items, center, *size, offset));
        b.items.extend(items);
        if !title.is_empty() {
            view_title(&mut b, Some(vp), i + 1, &title, &scale, at, length);
        }
    }
    // Text notes placed on the sheet itself (e.g. a cover title), in paper mm.
    studio_views::annotations(doc, &mut b, sheet);
    maps(doc, &mut b, sheet);
    placed_key_plans(doc, &mut b, sheet);
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
            Prim::Image { min, max, .. } => {
                pts.push(Pt::new(min[0], min[1]));
                pts.push(Pt::new(max[0], max[1]));
            }
        }
    }
    studio_regen::bounds(&pts)
}

/// What the sheet's plans show (ADR-048): each plan viewport's crop box, or None for an
/// uncropped plan (the whole building).
fn shown_areas(doc: &Document, sheet: ElementId) -> Vec<Option<studio_core::CropBox>> {
    viewports_on(doc, sheet)
        .into_iter()
        .filter_map(|(_, view, ..)| match doc.data(view) {
            Ok(ElementData::View {
                kind: ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. },
                crop,
                ..
            }) => Some(*crop),
            _ => None,
        })
        .collect()
}

/// The building's outline on its lowest level, fitted into a `w` × `h` box at `origin`,
/// with the areas in `shade` toned (None: all of it).
fn key_plan(
    doc: &Document,
    b: &mut Builder,
    el: Option<ElementId>,
    origin: Pt,
    w: f64,
    h: f64,
    shade: &[Option<studio_core::CropBox>],
) {
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
    let all = shade.iter().any(Option::is_none);
    for r in &regions {
        let outline: Vec<Pt> = r.outer.iter().map(map).collect();
        let fill = if all {
            FillKind::PocheLight
        } else {
            FillKind::Slab
        };
        b.fill(el, vec![studio_views::ring(&outline)], fill);
    }
    for c in shade.iter().flatten() {
        let (a, z) = (
            Pt::new(c.min.x.max(lo.x), c.min.y.max(lo.y)),
            Pt::new(c.max.x.min(hi.x), c.max.y.min(hi.y)),
        );
        if z.x - a.x < 1.0 || z.y - a.y < 1.0 {
            continue;
        }
        let rect = [a, Pt::new(z.x, a.y), z, Pt::new(a.x, z.y)];
        let mapped: Vec<Pt> = rect.iter().map(map).collect();
        b.fill(el, vec![studio_views::ring(&mapped)], FillKind::PocheLight);
        b.line(el, &mapped, true, 1, Dash::Dashed);
    }
    for r in &regions {
        let outline: Vec<Pt> = r.outer.iter().map(map).collect();
        b.line(el, &outline, true, 3, Dash::Solid);
    }
}

/// Location and vicinity maps on the sheet (ADR-107): the image (fetched when shown or
/// printed, with Google's attribution in it), its frame, and the address on a white label
/// over its top left.
fn maps(doc: &Document, b: &mut Builder, sheet: ElementId) {
    for e in doc.of(Category::MapFrame) {
        let ElementData::MapFrame {
            sheet: s,
            min,
            max,
            label,
            ..
        } = &e.data
        else {
            continue;
        };
        if *s != sheet {
            continue;
        }
        let el = Some(e.id);
        b.items.push(Item {
            el,
            prim: Prim::Image {
                image: e.id,
                min: [min.x, min.y],
                max: [max.x, max.y],
            },
        });
        let frame = [*min, Pt::new(max.x, min.y), *max, Pt::new(min.x, max.y)];
        b.line(el, &frame, true, 2, Dash::Solid);
        if !label.trim().is_empty() {
            let size = 3.2;
            let lines =
                studio_core::text::wrap(&label.to_uppercase(), size, Some((max.x - min.x) * 0.7));
            let w = lines
                .iter()
                .map(|l| studio_core::text::text_width(l, size))
                .fold(0.0, f64::max)
                + 4.0;
            let h = lines.len() as f64 * size * studio_core::text::LINE + 2.0;
            let (x0, y1) = (min.x + 3.0, max.y - 3.0);
            let r = [
                Pt::new(x0, y1 - h),
                Pt::new(x0 + w, y1 - h),
                Pt::new(x0 + w, y1),
                Pt::new(x0, y1),
            ];
            b.fill(el, vec![studio_views::ring(&r)], FillKind::Paper);
            b.line(el, &r, true, 1, Dash::Solid);
            for (i, l) in lines.iter().enumerate() {
                let y = y1 - 1.0 - size * 0.6 - i as f64 * size * studio_core::text::LINE;
                b.text(el, Pt::new(x0 + 2.0, y), l.clone(), size, Anchor::Left);
            }
        }
    }
}

/// Key plans placed on `sheet` (ADR-048): the key plan, a north arrow and its label.
fn placed_key_plans(doc: &Document, b: &mut Builder, sheet: ElementId) {
    let shade = shown_areas(doc, sheet);
    let style = studio_core::symbols::NorthStyle::of(doc);
    let tn = studio_views::symbols::true_north(doc);
    for e in doc.of(Category::KeyPlan) {
        let ElementData::KeyPlan {
            sheet: s,
            at,
            width,
        } = &e.data
        else {
            continue;
        };
        if *s != sheet {
            continue;
        }
        let el = Some(e.id);
        let (w, h) = (*width, width * 0.7);
        let o = at.sub(Pt::new(w / 2.0, h / 2.0));
        let frame = [
            o,
            Pt::new(o.x + w, o.y),
            Pt::new(o.x + w, o.y + h),
            Pt::new(o.x, o.y + h),
        ];
        b.fill(el, vec![studio_views::ring(&frame)], FillKind::Paper);
        b.line(el, &frame, true, 1, Dash::Solid);
        let r = (w * 0.06).clamp(3.0, 6.0);
        let pad = 3.0;
        key_plan(
            doc,
            b,
            el,
            o.add(Pt::new(pad, pad + 5.0)),
            w - 2.0 * pad - 2.6 * r,
            h - 2.0 * pad - 5.0,
            &shade,
        );
        studio_views::symbols::north_arrow(
            b,
            el,
            Pt::new(o.x + w - pad - r, o.y + h - pad - r * 1.9),
            r,
            style,
            tn,
        );
        b.text(
            el,
            o.add(Pt::new(pad, pad + 1.2)),
            "KEY PLAN".into(),
            2.2,
            Anchor::Left,
        );
    }
}

/// One viewport on a sheet: (viewport, view, center, title length, title offset).
type Placed = (ElementId, ElementId, Pt, Option<f64>, Option<Pt>);

/// A sheet's viewports in their numbering order.
fn viewports_on(doc: &Document, sheet: ElementId) -> Vec<Placed> {
    let mut vps: Vec<_> = doc
        .of(Category::Viewport)
        .filter_map(|e| match &e.data {
            ElementData::Viewport {
                sheet: s,
                view,
                center,
                title_length,
                title_offset,
            } if *s == sheet => Some((e.id, *view, *center, *title_length, *title_offset)),
            _ => None,
        })
        .collect();
    vps.sort_by_key(|v| v.0);
    vps
}

/// Where a viewport's title goes: under what is actually drawn (not the view's padded
/// bounds), kept inside the border, then wherever it was moved to (`offset`).
fn title_at(items: &[Item], center: Pt, size: SheetSize, offset: Option<Pt>) -> Pt {
    let (lo, _) = extents(items).unwrap_or((center, center));
    let at = Pt::new(lo.x.max(margins(size).0 + 4.0), lo.y - 4.0);
    offset.map_or(at, |o| at.add(o))
}

/// A title rule's default length (ADR-106): across the full width of its view, from the
/// end of the number bubble to the view's right edge. None for a view too narrow to need it.
fn full_width(items: &[Item], center: Pt, size: SheetSize, offset: Option<Pt>) -> Option<f64> {
    let (_, hi) = extents(items)?;
    let at = title_at(items, center, size, offset);
    let x0 = at.x + 2.0 * TITLE_BUBBLE + 2.0;
    let len = hi.x - x0;
    (len > MIN_TITLE_LENGTH).then_some(len)
}
/// Radius of the view title's number bubble, paper mm.
const TITLE_BUBBLE: f64 = 4.0;
/// The shortest a title's rule can be stretched to, paper mm.
pub const MIN_TITLE_LENGTH: f64 = 12.0;

/// The ends of a title's rule for a title at `at`: across the full width of the view
/// (ADR-106), at least as long as the name and scale, or `length` long when stretched.
fn title_rule(at: Pt, name: &str, scale: &str, length: Option<f64>) -> (Pt, Pt) {
    let c = at.add(Pt::new(TITLE_BUBBLE, -TITLE_BUBBLE));
    let x0 = c.x + TITLE_BUBBLE + 2.0;
    let len = length
        .unwrap_or_else(|| {
            approx_width(name, sizes::VIEW_TITLE).max(approx_width(scale, sizes::SCALE)) + 6.0
        })
        .max(MIN_TITLE_LENGTH);
    (Pt::new(x0, c.y), Pt::new(x0 + len, c.y))
}

/// A viewport's title rule on its sheet (paper mm), if it has a title.
pub fn title_line(doc: &Document, viewport: ElementId) -> Option<(Pt, Pt)> {
    let ElementData::Viewport {
        sheet,
        view,
        center,
        title_length,
        title_offset,
    } = doc.data(viewport).ok()?
    else {
        return None;
    };
    let ElementData::Sheet { size, .. } = doc.data(*sheet).ok()? else {
        return None;
    };
    let (items, _, _, title, scale) = viewport_items(doc, viewport, *view, *center)?;
    if title.is_empty() {
        return None;
    }
    Some(title_rule(
        title_at(&items, *center, *size, *title_offset),
        &title,
        &scale,
        title_length.or_else(|| full_width(&items, *center, *size, *title_offset)),
    ))
}

/// Handles on a sheet for the selected viewports (ADR-039): a grip at each end of the
/// title's rule, to stretch it left or right (with Shift, to move the title), and the title
/// itself, to drag into place.
pub fn sheet_handles(
    doc: &Document,
    sheet: ElementId,
    ids: &[ElementId],
) -> studio_views::handles::Handles {
    let mut out = studio_views::handles::Handles::default();
    for id in ids {
        let on_sheet =
            matches!(doc.data(*id), Ok(ElementData::Viewport { sheet: s, .. }) if *s == sheet);
        if let (true, Some((a, b))) = (on_sheet, title_line(doc, *id)) {
            out.grips.push(studio_views::handles::Grip {
                id: *id,
                key: "title_start".into(),
                at: a,
                anchor: Some(b),
            });
            out.grips.push(studio_views::handles::Grip {
                id: *id,
                key: "title_end".into(),
                at: b,
                anchor: Some(a),
            });
            // The bubble, name and scale: from the bubble's left to the rule's end.
            let left = a.x - 2.0 - 2.0 * TITLE_BUBBLE;
            out.areas.push(studio_views::handles::DragArea {
                id: *id,
                key: "title_move".into(),
                min: Pt::new(left, a.y - 6.5),
                max: Pt::new(b.x, a.y + 7.5),
                at: b,
            });
        }
    }
    // Every view on the sheet drags as a whole by its drawing, selected or not (ADR-100),
    // as Revit's viewports do. Titles (above) come first, so a selected title still drags
    // on its own.
    let mut vps: Vec<(ElementId, ElementId, Pt)> = doc
        .of(Category::Viewport)
        .filter_map(|e| match &e.data {
            ElementData::Viewport {
                sheet: s,
                view,
                center,
                ..
            } if *s == sheet => Some((e.id, *view, *center)),
            _ => None,
        })
        .collect();
    // Selected views first, so one on top of another is the one that drags.
    vps.sort_by_key(|(id, _, _)| !ids.contains(id));
    for (id, view, center) in vps {
        // The box of what's drawn (the title sits under it).
        let Some((lo, hi)) =
            viewport_items(doc, id, view, center).and_then(|(items, ..)| extents(&items))
        else {
            continue;
        };
        out.areas.push(studio_views::handles::DragArea {
            id,
            key: "view_move".into(),
            min: lo,
            max: hi,
            at: center,
        });
    }
    out
}

/// Runs \`edit\` on view \`view\` (cropping it, say) keeping its drawing where it was on every
/// sheet it's placed on (ADR-100): a viewport is placed by its drawing's center, so when
/// the crop moves that center the viewport moves with it, and only the extent grows or
/// shrinks. One undo step with the edit.
pub fn keep_placed<T>(
    doc: &mut Document,
    view: ElementId,
    edit: impl FnOnce(&mut Document) -> studio_core::CoreResult<T>,
) -> studio_core::CoreResult<T> {
    let center_of = |doc: &Document| -> Option<(Pt, f64)> {
        let s = match doc.data(view).ok()? {
            ElementData::View { scale, .. } => f64::from(*scale),
            _ => return None,
        };
        let [x0, y0, x1, y1] = studio_views::display_list(doc, view)?.bounds;
        Some((Pt::new((x0 + x1) / 2.0, (y0 + y1) / 2.0), s))
    };
    let placed: Vec<ElementId> = doc
        .of(Category::Viewport)
        .filter(|e| matches!(&e.data, ElementData::Viewport { view: v, .. } if *v == view))
        .map(|e| e.id)
        .collect();
    let before = if placed.is_empty() {
        None
    } else {
        center_of(doc)
    };
    let mark = doc.undo_depth();
    let out = edit(doc)?;
    if let (Some((c0, s0)), Some((c1, s1))) = (before, center_of(doc)) {
        // Only when the scale holds (a new scale re-fits the view about its center).
        if (s0 - s1).abs() < 1e-9 && c0.dist(c1) > 1e-6 {
            let by = c1.sub(c0).scale(1.0 / s1);
            let name = doc.can_undo().unwrap_or("Crop view").to_string();
            doc.transact("Keep view placed", |tx| {
                for vp in &placed {
                    tx.modify(*vp, |d| {
                        if let ElementData::Viewport { center, .. } = d {
                            *center = center.add(by);
                        }
                    })?;
                }
                Ok(())
            })?;
            doc.merge_undo(mark, &name);
        }
    }
    Ok(out)
}

/// Moves a view on its sheet so its center is at \`to\` (paper mm); its title goes with it
/// (ADR-100).
pub fn move_viewport(
    doc: &mut Document,
    viewport: ElementId,
    to: Pt,
) -> studio_core::CoreResult<()> {
    if !matches!(doc.data(viewport)?, ElementData::Viewport { .. }) {
        return Err(studio_core::CoreError::Invalid(
            "that isn't a view on a sheet".into(),
        ));
    }
    doc.transact("Move view", |tx| {
        tx.modify(viewport, |d| {
            if let ElementData::Viewport { center, .. } = d {
                *center = to;
            }
        })
    })
}

/// Stretches a viewport's title rule so it ends at `to` (paper mm), as far as
/// [`MIN_TITLE_LENGTH`].
pub fn drag_title(doc: &mut Document, viewport: ElementId, to: Pt) -> studio_core::CoreResult<()> {
    let (a, _) = title_line(doc, viewport)
        .ok_or_else(|| studio_core::CoreError::Invalid("that view has no title".into()))?;
    let len = (to.x - a.x).max(MIN_TITLE_LENGTH);
    doc.transact("Stretch view title", |tx| {
        tx.modify(viewport, |d| {
            if let ElementData::Viewport { title_length, .. } = d {
                *title_length = Some(len);
            }
        })
    })
}

/// Stretches a viewport's title rule from its left end: the rule starts at `to` (paper mm,
/// its x only) and still ends where it did; the bubble, name and scale move with the start.
pub fn drag_title_start(
    doc: &mut Document,
    viewport: ElementId,
    to: Pt,
) -> studio_core::CoreResult<()> {
    let (a, b) = title_line(doc, viewport)
        .ok_or_else(|| studio_core::CoreError::Invalid("that view has no title".into()))?;
    let len = (b.x - to.x).max(MIN_TITLE_LENGTH);
    let dx = (b.x - len) - a.x;
    doc.transact("Stretch view title", |tx| {
        tx.modify(viewport, |d| {
            if let ElementData::Viewport {
                title_length,
                title_offset,
                ..
            } = d
            {
                *title_length = Some(len);
                *title_offset = Some(title_offset.unwrap_or_default().add(Pt::new(dx, 0.0)));
            }
        })
    })
}

/// Moves a viewport's title (dragging the title, or Shift + drag of a grip) so its rule ends at `to`, keeping its
/// length.
pub fn move_title(doc: &mut Document, viewport: ElementId, to: Pt) -> studio_core::CoreResult<()> {
    let (_, end) = title_line(doc, viewport)
        .ok_or_else(|| studio_core::CoreError::Invalid("that view has no title".into()))?;
    let by = to.sub(end);
    doc.transact("Move view title", |tx| {
        tx.modify(viewport, |d| {
            if let ElementData::Viewport { title_offset, .. } = d {
                *title_offset = Some(title_offset.unwrap_or_default().add(by));
            }
        })
    })
}

/// Revit-style view title: number bubble, name above a heavy rule, scale below it. The rule
/// is fitted to the text unless stretched to `length`.
fn view_title(
    b: &mut Builder,
    el: Option<ElementId>,
    n: usize,
    name: &str,
    scale: &str,
    at: Pt,
    length: Option<f64>,
) {
    let r = TITLE_BUBBLE;
    let c = at.add(Pt::new(r, -r));
    b.circle(el, c, r, 3, false);
    b.text(el, c, n.to_string(), sizes::VIEW_NUMBER, Anchor::Center);
    let (start, end) = title_rule(at, name, scale, length);
    let x0 = start.x;
    let len = end.x - start.x;
    b.line(
        el,
        &[Pt::new(x0, c.y), Pt::new(x0 + len, c.y)],
        false,
        5,
        Dash::Solid,
    );
    b.text(
        el,
        Pt::new(x0, c.y + 3.4),
        name.to_uppercase(),
        sizes::VIEW_TITLE,
        Anchor::Left,
    );
    b.text(
        el,
        Pt::new(x0, c.y - 2.6),
        scale.to_owned(),
        sizes::SCALE,
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

    // The title block, top to bottom (ADR-109), as US sets carry it: the architect's firm,
    // the consultants, the project, the architect's stamp (with license and renewal, as
    // California's B&P Code 5536.1 asks of every sheet) and the agency's approval space,
    // the phase, date, scale and drawn/checked, the issues, the instruments-of-service
    // notice, the key plan, then the sheet title and number. Text to the office's types.
    let pad = 5.0 * k;
    let tx = x0 + pad;
    let inner = tb - 2.0 * pad;
    let (ident, details) =
        studio_core::project::get(doc).unwrap_or_else(|_| (Default::default(), Default::default()));
    let _ = &ident;
    let mut y = h - m;
    let txt = |b: &mut Builder, x: f64, y: f64, s: String, size: f64| {
        b.text(None, Pt::new(x, y), s, size * k, Anchor::Left);
    };
    // Lines of `text` wrapped to the block, from `y` down; returns the new y.
    let wrapped = |b: &mut Builder, y: f64, text: &str, size: f64| -> f64 {
        let mut y = y;
        for l in studio_core::text::wrap(text, size * k, Some(inner)) {
            y -= size * k * 1.45;
            b.text(None, Pt::new(tx, y), l, size * k, Anchor::Left);
        }
        y
    };
    let rule = |b: &mut Builder, y: f64| {
        b.line(
            None,
            &[Pt::new(x0, y), Pt::new(x1, y)],
            false,
            3,
            Dash::Solid,
        )
    };
    let label = |b: &mut Builder, y: &mut f64, text: &str| {
        *y -= 5.0 * k;
        b.text(
            None,
            Pt::new(tx, *y),
            text.to_owned(),
            sizes::TB_INFO * k,
            Anchor::Left,
        );
    };
    // Cyan accent bar, then the firm.
    b.fill(
        None,
        vec![studio_views::ring(&rect(x0, y - 5.0 * k, x1, y))],
        FillKind::Accent,
    );
    y -= 5.0 * k;
    let architect = details.team.iter().find(|t| {
        t.discipline.to_lowercase().contains("architect")
            && !t.discipline.to_lowercase().contains("landscape")
    });
    let firm = architect
        .map(|a| a.contact.company.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "RUFPLAN STUDIO".into());
    y -= sizes::TB_PROJECT * k * 1.6;
    txt(b, tx, y, firm.to_uppercase(), sizes::TB_PROJECT);
    if let Some(a) = architect {
        let c = &a.contact;
        for line in [
            c.address.trim().to_string(),
            [c.phone.trim(), c.email.trim()]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join("  ·  "),
            c.website.trim().to_string(),
        ] {
            if !line.is_empty() {
                y = wrapped(b, y, &line, sizes::TB_INFO);
            }
        }
    }
    y -= 3.0 * k;
    rule(b, y);
    // Consultants.
    let consultants: Vec<String> = details
        .team
        .iter()
        .filter(|t| !std::ptr::eq(*t, architect.map_or(std::ptr::null(), |a| a as *const _)))
        .filter(|t| !t.contact.company.trim().is_empty())
        .take(6)
        .map(|t| {
            format!(
                "{}: {}",
                t.discipline.to_uppercase(),
                t.contact.company.trim()
            )
        })
        .collect();
    if !consultants.is_empty() {
        label(b, &mut y, "CONSULTANTS");
        for c in &consultants {
            y = wrapped(b, y, c, sizes::TB_INFO);
        }
        y -= 3.0 * k;
        rule(b, y);
    }
    // The project.
    label(b, &mut y, "PROJECT");
    if !project.is_empty() {
        y = wrapped(b, y, &project.to_uppercase(), sizes::TB_PROJECT);
    }
    for line in [
        address.clone(),
        if client.is_empty() {
            String::new()
        } else {
            format!("OWNER: {client}")
        },
        if number_p.is_empty() {
            String::new()
        } else {
            format!("PROJECT NO. {number_p}")
        },
    ] {
        if !line.is_empty() {
            y = wrapped(b, y, &line, sizes::TB_INFO);
        }
    }
    y -= 3.0 * k;
    rule(b, y);
    // The architect's stamp, and the license lines under it.
    label(b, &mut y, "ARCHITECT'S STAMP");
    let stamp = 48.0 * k;
    y -= 2.0 * k;
    b.line(
        None,
        &rect(
            tx + (inner - stamp) / 2.0,
            y - stamp,
            tx + (inner + stamp) / 2.0,
            y,
        ),
        true,
        1,
        Dash::Dashed,
    );
    y -= stamp;
    y = wrapped(
        b,
        y,
        "LICENSE NO. __________   RENEWS __________",
        sizes::TB_INFO,
    );
    y -= 3.0 * k;
    rule(b, y);
    // The agency's approval space.
    label(b, &mut y, "AGENCY APPROVAL");
    y -= 2.0 * k;
    b.line(
        None,
        &rect(tx, y - 34.0 * k, tx + inner, y),
        true,
        1,
        Dash::Dashed,
    );
    y -= 34.0 * k;
    y -= 3.0 * k;
    rule(b, y);
    // Phase and status.
    label(b, &mut y, "DESIGN STAGE");
    if let Some((stage_name, abbr)) = &stage {
        y -= sizes::EIGHTH * k * 1.6;
        txt(b, tx, y, stage_name.to_uppercase(), sizes::EIGHTH);
        y -= sizes::SHEET_NUMBER * 0.75 * k * 1.25;
        txt(b, tx, y, abbr.clone(), sizes::SHEET_NUMBER * 0.75);
        if matches!(abbr.as_str(), "PD" | "SD" | "DD") {
            y -= (sizes::SHEET_NUMBER * 0.75 * 0.5 + sizes::TB_INFO * 1.6) * k;
            txt(b, tx, y, "NOT FOR CONSTRUCTION".into(), sizes::TB_INFO);
        }
    }
    y -= 3.0 * k;
    rule(b, y);
    // Date, scale, drawn and checked.
    let scales: std::collections::BTreeSet<String> = viewports_on(doc, sheet)
        .into_iter()
        .filter_map(|(_, view, ..)| match doc.data(view) {
            Ok(ElementData::View { scale, kind, .. })
                if !matches!(kind, ViewKind::Schedule { .. } | ViewKind::Rendering { .. }) =>
            {
                Some(ops::scale_label(*scale))
            }
            _ => None,
        })
        .collect();
    let scale = match scales.len() {
        0 => "NTS".to_string(),
        1 => scales.into_iter().next().unwrap_or_default(),
        _ => "AS INDICATED".into(),
    };
    let half = inner / 2.0;
    for (a, b_) in [
        (("DATE", date.to_string()), ("SCALE", scale)),
        (
            ("DRAWN BY", "—".to_string()),
            ("CHECKED BY", "—".to_string()),
        ),
    ] {
        y -= 4.5 * k;
        txt(b, tx, y, a.0.into(), sizes::TB_INFO);
        txt(b, tx + half, y, b_.0.into(), sizes::TB_INFO);
        y -= 3.8 * k;
        txt(b, tx, y, a.1, sizes::TB_INFO);
        txt(b, tx + half, y, b_.1, sizes::TB_INFO);
    }
    y -= 3.0 * k;
    rule(b, y);
    // Issues: every issued set that included this sheet (newest last).
    label(b, &mut y, "ISSUES");
    let issues = ops::sheet_issues(doc, sheet);
    if issues.is_empty() {
        y -= 4.0 * k;
        txt(b, tx, y, "—".into(), sizes::TB_INFO);
    }
    for (issue, date, abbr) in issues.iter().rev().take(6).rev() {
        y -= sizes::TB_INFO * k * 1.6;
        txt(b, tx, y, date.clone(), sizes::TB_INFO);
        txt(b, tx + 20.0 * k, y, abbr.clone(), sizes::TB_INFO);
        txt(b, tx + 29.0 * k, y, issue.to_uppercase(), sizes::TB_INFO);
    }
    y -= 3.0 * k;
    rule(b, y);
    // Instruments of service.
    let notice = format!(
        "These drawings are instruments of service and the property of {firm}; do not use, copy or scale them without its written consent."
    );
    let _ = wrapped(b, y - 1.0 * k, &notice, sizes::TB_INFO);
    // Key plan with north arrow, just above the sheet name block, as the Key Plan & North
    // Arrow standard says (ADR-048).
    let key_top = m + 42.0 * k + 62.0 * k;
    let kp = studio_core::symbols::KeyPlanStyle::of(doc);
    let show = matches!(
        kp,
        studio_core::symbols::KeyPlanStyle::TitleBlock
            | studio_core::symbols::KeyPlanStyle::ArrowOnly
    );
    if y > key_top + 2.0 && show {
        b.line(
            None,
            &[Pt::new(x0, key_top), Pt::new(x1, key_top)],
            false,
            3,
            Dash::Solid,
        );
        let arrow_only = kp == studio_core::symbols::KeyPlanStyle::ArrowOnly;
        b.text(
            None,
            Pt::new(tx, key_top - 5.5 * k),
            if arrow_only { "NORTH" } else { "KEY PLAN" }.into(),
            2.2 * k,
            Anchor::Left,
        );
        if !arrow_only {
            key_plan(
                doc,
                b,
                None,
                Pt::new(tx, m + 42.0 * k + 5.0 * k),
                x1 - tx - pad - 14.0 * k,
                44.0 * k,
                &shown_areas(doc, sheet),
            );
        }
        let c = if arrow_only {
            Pt::new((tx + x1) / 2.0, m + 42.0 * k + 26.0 * k)
        } else {
            Pt::new(x1 - pad - 5.0 * k, m + 42.0 * k + 12.0 * k)
        };
        studio_views::symbols::north_arrow(
            b,
            None,
            c,
            4.5 * k,
            studio_core::symbols::NorthStyle::of(doc),
            studio_views::symbols::true_north(doc),
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
        sizes::TB_INFO * k,
        Anchor::Left,
    );
    // The sheet title, wrapped to two lines when long.
    let lines = studio_core::text::wrap(&name.to_uppercase(), sizes::SHEET_TITLE * k, Some(inner));
    let two = lines.len() > 1;
    for (i, l) in lines.into_iter().take(2).enumerate() {
        let y =
            bottom + if two { 31.5 } else { 29.0 } * k - i as f64 * sizes::SHEET_TITLE * k * 1.3;
        b.text(
            None,
            Pt::new(tx, y),
            l,
            sizes::SHEET_TITLE * k,
            Anchor::Left,
        );
    }
    b.text(
        None,
        Pt::new(tx, bottom + 11.0 * k),
        number.to_owned(),
        sizes::SHEET_NUMBER * k,
        Anchor::Left,
    );
}
