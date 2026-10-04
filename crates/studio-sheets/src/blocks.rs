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
            } if *v == sheet => bounds(doc, members).map(|(lo, hi)| (e.id, lo, hi)),
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
    let (lo, hi) = bounds(doc, &members).ok_or_else(|| CoreError::Invalid("empty".into()))?;
    studio_core::modify::move_elements(doc, &[group], to.sub(lo.lerp(hi, 0.5)))
}

/// Stretches a block's right (or left) edge to `x`: lines and regions on that edge move
/// with it, notes that span the block widen with it and wrap again, and everything under a
/// note that gains or loses lines moves down or up by as much.
pub fn stretch_block(doc: &mut Document, group: ElementId, right: bool, x: f64) -> CoreResult<()> {
    let members = members_of(doc, group)?;
    let (lo, hi) = bounds(doc, &members).ok_or_else(|| CoreError::Invalid("empty".into()))?;
    let old_w = hi.x - lo.x;
    let new_w = if right { x - lo.x } else { hi.x - x }.max(MIN_WIDTH);
    let d = new_w - old_w;
    if d.abs() < 0.05 {
        return Ok(());
    }
    // Stretched on the right; a left stretch is the same then moved left by as much.
    let shift_x = if right { 0.0 } else { -d };
    let edge = hi.x;
    let mid = (lo.x + hi.x) / 2.0;

    struct Note {
        id: ElementId,
        at: Pt,
        width: Option<f64>,
        growth: f64,
    }
    let mut notes: Vec<Note> = vec![];
    for m in &members {
        let Ok(ElementData::TextNote {
            at,
            text,
            size,
            width,
            align,
            ..
        }) = doc.data(*m)
        else {
            continue;
        };
        let mut at = *at;
        let mut width = *width;
        let mut growth = 0.0;
        match (align, width) {
            (TextAlign::Left, Some(w)) if at.x + w >= edge - NEAR_RIGHT => {
                let nw = (w + d).max(size * 4.0);
                let n0 = wrap(text, *size, Some(w)).len() as f64;
                let n1 = wrap(text, *size, Some(nw)).len() as f64;
                growth = (n1 - n0) * size * LINE;
                width = Some(nw);
            }
            (TextAlign::Center, _) if (at.x - mid).abs() < 2.0 => at.x += d / 2.0,
            (TextAlign::Right, _) if at.x >= edge - NEAR_RIGHT => at.x += d,
            _ => {}
        }
        notes.push(Note {
            id: *m,
            at,
            width,
            growth,
        });
    }
    // Rows of notes, top down: a row's growth moves whatever is under it.
    let mut rows: Vec<(f64, f64)> = vec![];
    for n in &notes {
        match rows.iter_mut().find(|r| (r.0 - n.at.y).abs() < 0.3) {
            Some(r) => r.1 = r.1.max(n.growth),
            None => rows.push((n.at.y, n.growth)),
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

    doc.transact("Stretch", |tx| {
        for n in &notes {
            let at = Pt::new(n.at.x + shift_x, n.at.y - above(n.at.y));
            tx.modify(n.id, |e| {
                if let ElementData::TextNote {
                    at: a, width: w, ..
                } = e
                {
                    *a = at;
                    *w = n.width;
                }
            })?;
        }
        for m in &members {
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
        if shift_x != 0.0 {
            tx.modify(group, |e| {
                if let ElementData::Group { origin, .. } = e {
                    origin.x += shift_x;
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

    #[test]
    fn narrowing_a_block_wraps_its_text_and_pushes_the_rest_down() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let (g, bottom, under, sheet) = boxed(&mut doc);
        let n0 = wrap(&"word ".repeat(24), 2.38125, Some(74.0)).len();
        // From 80 to 50 wide: the note is 44 wide and wraps to more lines.
        stretch_block(&mut doc, g, true, 150.0).unwrap();
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
        stretch_block(&mut doc, g, false, 70.0).unwrap();
        let (lo, hi) = (
            sheet_blocks(&doc, sheet)[0].1,
            sheet_blocks(&doc, sheet)[0].2,
        );
        assert!((lo.x - 70.0).abs() < 1e-9 && (hi.x - 150.0).abs() < 1e-9);
        let (at, w) = note_at(&doc, under);
        assert!((at.x - 73.0).abs() < 1e-9 && (w.unwrap() - 74.0).abs() < 1e-9);
        // Back at 80 wide the text is as it was.
        assert!((at.y - 175.0).abs() < 1e-9, "{at:?}");
        // Never narrower than 25 mm.
        stretch_block(&mut doc, g, true, 0.0).unwrap();
        let (lo, hi) = (
            sheet_blocks(&doc, sheet)[0].1,
            sheet_blocks(&doc, sheet)[0].2,
        );
        assert!((hi.x - lo.x - MIN_WIDTH).abs() < 1e-9);
    }

    #[test]
    fn a_block_moves_whole_to_where_its_centre_is_dragged() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let (g, bottom, _, sheet) = boxed(&mut doc);
        let (lo, hi) = (
            sheet_blocks(&doc, sheet)[0].1,
            sheet_blocks(&doc, sheet)[0].2,
        );
        let c = lo.lerp(hi, 0.5);
        move_block(&mut doc, g, c.add(Pt::new(10.0, -20.0))).unwrap();
        let (a, _) = line_y(&doc, bottom);
        assert!((a.x - 110.0).abs() < 1e-9 && (a.y - 140.0).abs() < 1e-9);
    }
}
