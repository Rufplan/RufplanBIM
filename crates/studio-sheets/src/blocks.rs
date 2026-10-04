//! Blocks of sheet text (ADR-112): the detail groups the sets make of each box of notes
//! (ADR-111) drag on the sheet as the sheet index does, and stretch from their left or
//! right edge as a Revit text box does, the text inside wrapping to the new width and the
//! box growing or shrinking to fit.

use studio_core::text::{wrap, TextAlign, LINE};
use studio_core::{CoreError, CoreResult, Document, ElementData, ElementId};
use studio_geom::Pt;

/// How near an edge a point is to move with it (paper mm).
const ON_EDGE: f64 = 0.5;
/// A note whose right side is this near the block's moves its right side with the edge.
const NEAR_RIGHT: f64 = 6.0;
/// The narrowest a block stretches to (paper mm).
pub const MIN_WIDTH: f64 = 25.0;

/// The detail groups on a sheet with their extents.
pub fn sheet_blocks(doc: &Document, sheet: ElementId) -> Vec<(ElementId, Pt, Pt)> {
    doc.of(studio_core::Category::Group)
        .filter_map(|e| match &e.data {
            ElementData::Group {
                view: Some(v),
                members,
                ..
            } if *v == sheet => frame(doc, members).map(|(lo, hi)| (e.id, lo, hi)),
            _ => None,
        })
        .collect()
}

/// The extent of a block's notes, lines and regions.
pub fn bounds(doc: &Document, members: &[ElementId]) -> Option<(Pt, Pt)> {
    let mut pts: Vec<Pt> = vec![];
    for m in members {
        match doc.data(*m) {
            Ok(ElementData::TextNote { .. }) => {
                if let Ok((tb, ..)) = studio_core::ops::text_note_box(doc, *m) {
                    pts.extend([tb.min, tb.max]);
                }
            }
            Ok(ElementData::DetailLine {
                curve: studio_core::sketch::SketchCurve::Line { a, b, .. },
                ..
            }) => pts.extend([*a, *b]),
            Ok(ElementData::FilledRegion { boundary, .. }) => {
                pts.extend(boundary.iter().flatten().copied())
            }
            _ => {}
        }
    }
    let first = *pts.first()?;
    Some(pts.iter().fold((first, first), |(lo, hi), p| {
        (
            Pt::new(lo.x.min(p.x), lo.y.min(p.y)),
            Pt::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}

fn members_of(doc: &Document, group: ElementId) -> CoreResult<Vec<ElementId>> {
    match doc.data(group)? {
        ElementData::Group { members, .. } => Ok(members.clone()),
        _ => Err(CoreError::Invalid(
            "that isn't a block of sheet text".into(),
        )),
    }
}

/// Moves a block so its extent's centre is at `to`.
pub fn move_block(doc: &mut Document, group: ElementId, to: Pt) -> CoreResult<()> {
    let members = members_of(doc, group)?;
    let (lo, hi) = frame(doc, &members).ok_or_else(|| CoreError::Invalid("empty".into()))?;
    studio_core::modify::move_elements(doc, &[group], to.sub(lo.lerp(hi, 0.5)))
}

/// Which edge of a block is dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    /// The side a `block_<side>` drag key names.
    pub fn from_key(key: &str) -> Option<Side> {
        match key.strip_prefix("block_")? {
            "left" => Some(Side::Left),
            "right" => Some(Side::Right),
            "top" => Some(Side::Top),
            "bottom" => Some(Side::Bottom),
            _ => None,
        }
    }
}

/// The block's frame: its rules' and regions' extent where it has them (a ruled box), else
/// its notes'. A title wider than its box doesn't move the box's edges.
pub fn frame(doc: &Document, members: &[ElementId]) -> Option<(Pt, Pt)> {
    let ruled: Vec<ElementId> = members
        .iter()
        .copied()
        .filter(|m| {
            matches!(
                doc.data(*m),
                Ok(ElementData::DetailLine { .. } | ElementData::FilledRegion { .. })
            )
        })
        .collect();
    bounds(doc, &ruled).or_else(|| bounds(doc, members))
}

struct NoteAt {
    id: ElementId,
    at: Pt,
    text: String,
    size: f64,
    width: Option<f64>,
    align: TextAlign,
    /// Its text box (min, max).
    min: Pt,
    max: Pt,
}

fn notes_of(doc: &Document, members: &[ElementId]) -> Vec<NoteAt> {
    members
        .iter()
        .filter_map(|m| match doc.data(*m) {
            Ok(ElementData::TextNote {
                at,
                text,
                size,
                width,
                align,
                ..
            }) => {
                let (tb, ..) = studio_core::ops::text_note_box(doc, *m).ok()?;
                Some(NoteAt {
                    id: *m,
                    at: *at,
                    text: text.clone(),
                    size: *size,
                    width: *width,
                    align: *align,
                    min: tb.min,
                    max: tb.max,
                })
            }
            _ => None,
        })
        .collect()
}

/// A wrapping note that spans to the block's right edge `edge`.
fn spans(n: &NoteAt, edge: f64) -> Option<f64> {
    match (n.align, n.width) {
        (TextAlign::Left, Some(w)) if n.at.x + w >= edge - NEAR_RIGHT => Some(w),
        _ => None,
    }
}

/// Stretches a block's edge to `to` (its x for the left and right edges, y for the top and
/// bottom). Left and right: lines and regions on that edge move with it, notes that span
/// the block widen or narrow and wrap again, and everything under a note that gains or
/// loses lines moves down or up by as much; never so narrow that a title would stick out.
/// Top and bottom: the edge moves, never into the text.
pub fn stretch_block(doc: &mut Document, group: ElementId, side: Side, to: Pt) -> CoreResult<()> {
    let members = members_of(doc, group)?;
    let (lo, hi) = frame(doc, &members).ok_or_else(|| CoreError::Invalid("empty".into()))?;
    let notes = notes_of(doc, &members);
    match side {
        Side::Left | Side::Right => {
            let right = side == Side::Right;
            let edge = hi.x;
            // As narrow as the titles and the shortest wrapping width allow.
            let least = notes
                .iter()
                .map(|n| match spans(n, edge) {
                    Some(w) => n.at.x - lo.x + n.size * 4.0 + (edge - (n.at.x + w)),
                    None if n.align == TextAlign::Left && n.width.is_none() => n.max.x - lo.x + 1.0,
                    None => 0.0,
                })
                .fold(MIN_WIDTH, f64::max);
            let new_w = if right { to.x - lo.x } else { hi.x - to.x }.max(least);
            stretch_x(doc, group, &members, &notes, (lo, hi), new_w, right)
        }
        Side::Top | Side::Bottom => {
            // Nothing above the text's top or under its bottom but the edge itself.
            let mut inner: Vec<f64> = notes.iter().flat_map(|n| [n.min.y, n.max.y]).collect();
            for m in &members {
                if let Ok(ElementData::DetailLine {
                    curve: studio_core::sketch::SketchCurve::Line { a, b, .. },
                    ..
                }) = doc.data(*m)
                {
                    for p in [a, b] {
                        if (p.y - lo.y).abs() > ON_EDGE && (p.y - hi.y).abs() > ON_EDGE {
                            inner.push(p.y);
                        }
                    }
                }
            }
            let bottom = side == Side::Bottom;
            // A box's title bar stays with its top: a top stretch grows the bottom, then
            // moves the block up by as much.
            let content = inner.iter().copied().fold(hi.y, f64::min);
            let least = (hi.y - content + 1.5).max(4.0);
            let new_h = if bottom { hi.y - to.y } else { to.y - lo.y }.max(least);
            let d = new_h - (hi.y - lo.y);
            if d.abs() < 0.05 {
                return Ok(());
            }
            let lift = if bottom { 0.0 } else { d };
            let place = |p: Pt| -> Pt {
                let y = if (p.y - lo.y).abs() < ON_EDGE {
                    p.y - d
                } else {
                    p.y
                };
                Pt::new(p.x, y + lift)
            };
            let shift = Pt::new(0.0, lift);
            transform(
                doc,
                group,
                &members,
                &place,
                &|n: &NoteAt| (n.at.add(shift), n.width),
                shift,
            )
        }
    }
}

fn stretch_x(
    doc: &mut Document,
    group: ElementId,
    members: &[ElementId],
    notes: &[NoteAt],
    (lo, hi): (Pt, Pt),
    new_w: f64,
    right: bool,
) -> CoreResult<()> {
    let d = new_w - (hi.x - lo.x);
    if d.abs() < 0.05 {
        return Ok(());
    }
    // Stretched on the right; a left stretch is the same then moved left by as much.
    let shift_x = if right { 0.0 } else { -d };
    let edge = hi.x;
    let mid = (lo.x + hi.x) / 2.0;
    let mut moved: Vec<(ElementId, Pt, Option<f64>, f64)> = vec![];
    for n in notes {
        let (mut at, mut width, mut growth) = (n.at, n.width, 0.0);
        if let Some(w) = spans(n, edge) {
            let nw = (w + d).max(n.size * 4.0);
            let n0 = wrap(&n.text, n.size, Some(w)).len() as f64;
            let n1 = wrap(&n.text, n.size, Some(nw)).len() as f64;
            growth = (n1 - n0) * n.size * LINE;
            width = Some(nw);
        } else if n.align == TextAlign::Center && (at.x - mid).abs() < 2.0 {
            at.x += d / 2.0;
        } else if n.align == TextAlign::Right && at.x >= edge - NEAR_RIGHT {
            at.x += d;
        }
        moved.push((n.id, at, width, growth));
    }
    // Rows of notes, top down: a row's growth moves whatever is under it.
    let mut rows: Vec<(f64, f64)> = vec![];
    for (_, at, _, g) in &moved {
        match rows.iter_mut().find(|r| (r.0 - at.y).abs() < 0.3) {
            Some(r) => r.1 = r.1.max(*g),
            None => rows.push((at.y, *g)),
        }
    }
    let above = |y: f64| -> f64 {
        rows.iter()
            .filter(|r| r.0 > y + 0.3)
            .map(|r| r.1)
            .sum::<f64>()
    };
    let place = |p: Pt| -> Pt {
        let x = if (p.x - edge).abs() < ON_EDGE {
            p.x + d
        } else {
            p.x
        };
        Pt::new(x + shift_x, p.y - above(p.y))
    };
    let note = |n: &NoteAt| {
        let (_, at, width, _) = moved
            .iter()
            .find(|m| m.0 == n.id)
            .copied()
            .unwrap_or((n.id, n.at, n.width, 0.0));
        (Pt::new(at.x + shift_x, at.y - above(at.y)), width)
    };
    transform(doc, group, members, &place, &note, Pt::new(shift_x, 0.0))
}

/// Applies a stretch in one transaction: `place` for line and region points, `note` for
/// each note's anchor and width, `shift` for the group's origin.
fn transform(
    doc: &mut Document,
    group: ElementId,
    members: &[ElementId],
    place: &dyn Fn(Pt) -> Pt,
    note: &dyn Fn(&NoteAt) -> (Pt, Option<f64>),
    shift: Pt,
) -> CoreResult<()> {
    let notes = notes_of(doc, members);
    doc.transact("Stretch", |tx| {
        for n in &notes {
            let (at, width) = note(n);
            tx.modify(n.id, |e| {
                if let ElementData::TextNote {
                    at: a, width: w, ..
                } = e
                {
                    *a = at;
                    *w = width;
                }
            })?;
        }
        for m in members {
            match tx.data(*m)?.clone() {
                ElementData::DetailLine {
                    curve: studio_core::sketch::SketchCurve::Line { .. },
                    ..
                } => tx.modify(*m, |e| {
                    if let ElementData::DetailLine {
                        curve: studio_core::sketch::SketchCurve::Line { a, b, .. },
                        ..
                    } = e
                    {
                        *a = place(*a);
                        *b = place(*b);
                    }
                })?,
                ElementData::FilledRegion { .. } => tx.modify(*m, |e| {
                    if let ElementData::FilledRegion { boundary, .. } = e {
                        for p in boundary.iter_mut().flatten() {
                            *p = place(*p);
                        }
                    }
                })?,
                _ => {}
            }
        }
        if shift.len() > 0.0 {
            tx.modify(group, |e| {
                if let ElementData::Group { origin, .. } = e {
                    *origin = origin.add(shift);
                }
            })?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::lines::LineStyle;
    use studio_core::{ops, SheetSize};

    /// A ruled box at (100, 200) 80 wide: a title bar, a 3/32" note filling it, and a line
    /// of text under it.
    fn boxed(doc: &mut Document) -> (ElementId, ElementId, ElementId, ElementId) {
        let mut ids = (vec![], None, None, None);
        doc.transact("Box", |tx| {
            let sheet = tx.insert(ElementData::Sheet {
                number: "G-002".into(),
                name: "Notes".into(),
                size: SheetSize::ArchD,
                stages: vec![],
            });
            let line = |tx: &mut studio_core::Tx<'_>, a: Pt, b: Pt| {
                tx.insert(ElementData::DetailLine {
                    view: sheet,
                    curve: studio_core::sketch::SketchCurve::Line { a, b, wall: None },
                    style: LineStyle::Medium,
                })
            };
            let (x0, x1, top, bot) = (100.0, 180.0, 200.0, 160.0);
            let right = line(tx, Pt::new(x1, top), Pt::new(x1, bot));
            let bottom = line(tx, Pt::new(x0, bot), Pt::new(x1, bot));
            ids.0.extend([
                line(tx, Pt::new(x0, top), Pt::new(x1, top)),
                line(tx, Pt::new(x0, top), Pt::new(x0, bot)),
                right,
                bottom,
            ]);
            let note = |tx: &mut studio_core::Tx<'_>, y: f64, text: &str| {
                tx.insert(ElementData::TextNote {
                    view: sheet,
                    at: Pt::new(x0 + 3.0, y),
                    text: text.into(),
                    size: 2.38125,
                    leaders: vec![],
                    align: TextAlign::Left,
                    width: Some(74.0),
                    angle: 0.0,
                })
            };
            // About 120 characters: two lines at 74 mm (1.19 mm a character).
            let long = note(tx, 190.0, &"word ".repeat(24));
            let under = note(tx, 175.0, "Last line.");
            ids.0.extend([long, under]);
            ids.1 = Some(sheet);
            ids.2 = Some(bottom);
            ids.3 = Some(under);
            Ok(())
        })
        .unwrap();
        let sheet = ids.1.unwrap();
        studio_core::groups::create(doc, &ids.0, "G-002 BOX").unwrap();
        let g = sheet_blocks(doc, sheet)[0].0;
        (g, ids.2.unwrap(), ids.3.unwrap(), sheet)
    }

    fn line_y(doc: &Document, id: ElementId) -> (Pt, Pt) {
        match doc.data(id) {
            Ok(ElementData::DetailLine {
                curve: studio_core::sketch::SketchCurve::Line { a, b, .. },
                ..
            }) => (*a, *b),
            _ => panic!(),
        }
    }
    fn note_at(doc: &Document, id: ElementId) -> (Pt, Option<f64>) {
        match doc.data(id) {
            Ok(ElementData::TextNote { at, width, .. }) => (*at, *width),
            _ => panic!(),
        }
    }

    fn extent(doc: &Document, sheet: ElementId) -> (Pt, Pt) {
        let b = &sheet_blocks(doc, sheet)[0];
        (b.1, b.2)
    }
    fn new_doc() -> Document {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        doc
    }
    const R: fn(f64) -> Pt = |x| Pt::new(x, 0.0);

    #[test]
    fn narrowing_a_block_wraps_its_text_and_pushes_the_rest_down() {
        let mut doc = new_doc();
        let (g, bottom, under, sheet) = boxed(&mut doc);
        let n0 = wrap(&"word ".repeat(24), 2.38125, Some(74.0)).len();
        // From 80 to 50 wide: the note is 44 wide and wraps to more lines.
        stretch_block(&mut doc, g, Side::Right, R(150.0)).unwrap();
        let n1 = wrap(&"word ".repeat(24), 2.38125, Some(44.0)).len();
        assert!(n1 > n0);
        let grow = (n1 - n0) as f64 * 2.38125 * LINE;
        let (a, b) = line_y(&doc, bottom);
        assert!((a.x - 100.0).abs() < 1e-9 && (b.x - 150.0).abs() < 1e-9);
        assert!((a.y - (160.0 - grow)).abs() < 1e-9, "{a:?}");
        let (at, w) = note_at(&doc, under);
        assert!((at.y - (175.0 - grow)).abs() < 1e-9);
        assert!((w.unwrap() - 44.0).abs() < 1e-9);
        // The left edge: the box widens to the left and its notes move with it.
        stretch_block(&mut doc, g, Side::Left, R(70.0)).unwrap();
        let (lo, hi) = extent(&doc, sheet);
        assert!((lo.x - 70.0).abs() < 1e-9 && (hi.x - 150.0).abs() < 1e-9);
        let (at, w) = note_at(&doc, under);
        assert!((at.x - 73.0).abs() < 1e-9 && (w.unwrap() - 74.0).abs() < 1e-9);
        // Back at 80 wide the text is as it was.
        assert!((at.y - 175.0).abs() < 1e-9, "{at:?}");
    }

    #[test]
    fn a_compacted_block_stretches_back_out() {
        let mut doc = new_doc();
        let (g, _, under, sheet) = boxed(&mut doc);
        stretch_block(&mut doc, g, Side::Right, R(0.0)).unwrap();
        let (lo, hi) = extent(&doc, sheet);
        assert!((hi.x - lo.x - MIN_WIDTH).abs() < 1e-9);
        stretch_block(&mut doc, g, Side::Right, R(200.0)).unwrap();
        let (_, hi) = extent(&doc, sheet);
        assert!((hi.x - 200.0).abs() < 1e-9, "{hi:?}");
        assert!((note_at(&doc, under).1.unwrap() - 94.0).abs() < 1e-9);
        assert!((note_at(&doc, under).0.y - 175.0).abs() < 1e-9);
    }

    #[test]
    fn a_title_that_doesnt_wrap_keeps_the_block_wide_enough() {
        let mut doc = new_doc();
        let (g, _, _, sheet) = boxed(&mut doc);
        let mut title = None;
        doc.transact("Title", |tx| {
            title = Some(tx.insert(ElementData::TextNote {
                view: sheet,
                at: Pt::new(103.0, 196.0),
                text: "A VERY LONG UNWRAPPED TITLE".into(),
                size: 3.0,
                leaders: vec![],
                align: TextAlign::Left,
                width: None,
                angle: 0.0,
            }));
            Ok(())
        })
        .unwrap();
        studio_core::groups::add_members(&mut doc, g, &[title.unwrap()]).unwrap();
        stretch_block(&mut doc, g, Side::Right, R(0.0)).unwrap();
        let (lo, hi) = extent(&doc, sheet);
        let wide = 3.0 + studio_core::text::text_width("A VERY LONG UNWRAPPED TITLE", 3.0) + 1.0;
        assert!((hi.x - lo.x - wide).abs() < 1e-9, "{}", hi.x - lo.x);
        stretch_block(&mut doc, g, Side::Right, R(190.0)).unwrap();
        assert!((extent(&doc, sheet).1.x - 190.0).abs() < 1e-9);
    }

    #[test]
    fn the_bottom_and_top_stretch_but_never_into_the_text() {
        let mut doc = new_doc();
        let (g, bottom, under, sheet) = boxed(&mut doc);
        stretch_block(&mut doc, g, Side::Bottom, Pt::new(0.0, 140.0)).unwrap();
        let (a, b) = line_y(&doc, bottom);
        assert!((a.y - 140.0).abs() < 1e-9 && (b.y - 140.0).abs() < 1e-9);
        assert!((extent(&doc, sheet).0.y - 140.0).abs() < 1e-9);
        // Up past the last line: it stops just under it.
        stretch_block(&mut doc, g, Side::Bottom, Pt::new(0.0, 199.0)).unwrap();
        let text_bottom = 175.0 - 2.38125 * 0.8;
        assert!((line_y(&doc, bottom).0.y - (text_bottom - 1.5)).abs() < 1e-9);
        // The top: the block grows upward, the text and title bar with it.
        stretch_block(&mut doc, g, Side::Bottom, Pt::new(0.0, 160.0)).unwrap();
        stretch_block(&mut doc, g, Side::Top, Pt::new(0.0, 220.0)).unwrap();
        let (lo, hi) = extent(&doc, sheet);
        assert!((hi.y - 220.0).abs() < 1e-9 && (lo.y - 160.0).abs() < 1e-9);
        assert!((note_at(&doc, under).0.y - 195.0).abs() < 1e-9);
        assert_eq!(Side::from_key("block_top"), Some(Side::Top));
    }

    #[test]
    fn a_block_moves_whole_to_where_its_centre_is_dragged() {
        let mut doc = new_doc();
        let (g, bottom, _, sheet) = boxed(&mut doc);
        let (lo, hi) = extent(&doc, sheet);
        move_block(&mut doc, g, lo.lerp(hi, 0.5).add(Pt::new(10.0, -20.0))).unwrap();
        let (a, _) = line_y(&doc, bottom);
        assert!((a.x - 110.0).abs() < 1e-9 && (a.y - 140.0).abs() < 1e-9);
    }
}
