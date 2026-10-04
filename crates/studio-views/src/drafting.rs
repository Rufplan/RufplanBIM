//! Drafting views and filled regions (ADR-069): hatch patterns clipped to a region, drawn
//! at a paper size so they read the same at any scale, and library detail previews.

use studio_core::details::{self, FillPattern};
use studio_core::lines::LineStyle;
use studio_core::ElementId;
use studio_geom::{point_in_ring, Pt};

use super::{line_style, ring, Builder, DisplayList, FillKind, ViewType};

/// Most hatch lines or marks one region draws (a huge region stays drawable).
const MAX_MARKS: usize = 3000;

/// Inside the region: in the outer loop and none of the holes.
fn inside(rings: &[Vec<Pt>], p: Pt) -> bool {
    rings
        .first()
        .is_some_and(|o| point_in_ring(p, o) && !rings[1..].iter().any(|h| point_in_ring(p, h)))
}

/// Parallel lines at `angle` (radians) every `spacing` (model mm), clipped to the region
/// (even-odd across all its loops).
fn hatch_lines(rings: &[Vec<Pt>], angle: f64, spacing: f64) -> Vec<[Pt; 2]> {
    let (c, s) = (angle.cos(), angle.sin());
    // Into a frame where the hatch runs along x.
    let to = |p: Pt| Pt::new(p.x * c + p.y * s, -p.x * s + p.y * c);
    let back = |p: Pt| Pt::new(p.x * c - p.y * s, p.x * s + p.y * c);
    let rr: Vec<Vec<Pt>> = rings
        .iter()
        .map(|r| r.iter().map(|p| to(*p)).collect())
        .collect();
    let ys = rr.iter().flatten().map(|p| p.y);
    let (lo, hi) = ys.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), y| {
        (a.min(y), b.max(y))
    });
    if !lo.is_finite() || spacing <= 0.0 {
        return vec![];
    }
    let mut out = vec![];
    let mut y = (lo / spacing).floor() * spacing + spacing * 0.5;
    while y < hi && out.len() < MAX_MARKS {
        let mut xs: Vec<f64> = vec![];
        for r in &rr {
            let n = r.len();
            for i in 0..n {
                let (a, b) = (r[i], r[(i + 1) % n]);
                if (a.y <= y) != (b.y <= y) {
                    xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        for w in xs.as_chunks::<2>().0 {
            if w[1] - w[0] > 1e-6 {
                out.push([back(Pt::new(w[0], y)), back(Pt::new(w[1], y))]);
            }
        }
        y += spacing;
    }
    out
}

/// A repeatable jitter in [0, 1) for grid cell (i, j) and channel k.
fn jitter(i: i64, j: i64, k: u64) -> f64 {
    let mut h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ k.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Jittered points on a `step` grid inside the region.
fn scatter(rings: &[Vec<Pt>], step: f64) -> Vec<(Pt, f64)> {
    let Some((lo, hi)) = studio_geom::bounds_of(&rings.concat()) else {
        return vec![];
    };
    let mut out = vec![];
    let (i0, i1) = ((lo.x / step).floor() as i64, (hi.x / step).ceil() as i64);
    let (j0, j1) = ((lo.y / step).floor() as i64, (hi.y / step).ceil() as i64);
    for i in i0..=i1 {
        for j in j0..=j1 {
            let p = Pt::new(
                (i as f64 + jitter(i, j, 1)) * step,
                (j as f64 + jitter(i, j, 2)) * step,
            );
            if inside(rings, p) {
                out.push((p, jitter(i, j, 3)));
                if out.len() >= MAX_MARKS {
                    return out;
                }
            }
        }
    }
    out
}

/// Draws a filled region: its pattern (at paper sizes) and its outline, if any.
pub(crate) fn region(
    b: &mut Builder,
    el: Option<ElementId>,
    rings: &[Vec<Pt>],
    pattern: FillPattern,
    outline: Option<LineStyle>,
) {
    if rings.first().is_none_or(|r| r.len() < 3) {
        return;
    }
    let polys: Vec<Vec<[f64; 2]>> = rings.iter().map(|r| ring(r)).collect();
    let scale = b.scale;
    let paper = |mm: f64| mm * scale;
    let deg = |d: f64| d.to_radians();
    let fill = match pattern {
        FillPattern::Solid => FillKind::Ink,
        FillPattern::Masking => FillKind::Paper,
        FillPattern::Gray => FillKind::PocheLight,
        _ => FillKind::Paper,
    };
    b.fill(el, polys, fill);
    let mut lines: Vec<[Pt; 2]> = vec![];
    let families: &[(f64, f64)] = match pattern {
        FillPattern::Diagonal => &[(45.0, 1.6)],
        FillPattern::CrossHatch => &[(45.0, 1.6), (135.0, 1.6)],
        FillPattern::Earth => &[(45.0, 0.8), (135.0, 3.2)],
        FillPattern::Masonry => &[(45.0, 0.9)],
        FillPattern::RigidInsulation => &[(60.0, 1.8), (120.0, 1.8)],
        FillPattern::Wood => &[(0.0, 0.7)],
        FillPattern::Steel => &[(45.0, 0.5)],
        _ => &[],
    };
    for (a, s) in families {
        lines.extend(hatch_lines(rings, deg(*a), paper(*s)));
    }
    for [p, q] in &lines {
        b.line(el, &[*p, *q], false, 1, super::Dash::Solid);
    }
    match pattern {
        FillPattern::Concrete => {
            // Aggregate: small triangles and dots, scattered.
            for (p, r) in scatter(rings, paper(1.6)) {
                if r < 0.45 {
                    let (sz, rot) = (paper(0.28 + r * 0.3), r * 40.0);
                    let tri: Vec<Pt> = (0..3)
                        .map(|k| {
                            let a = rot + k as f64 * 2.094;
                            p.add(Pt::new(a.cos() * sz, a.sin() * sz))
                        })
                        .collect();
                    if tri.iter().all(|q| inside(rings, *q)) {
                        b.line(el, &tri, true, 1, super::Dash::Solid);
                    }
                } else {
                    b.circle(el, p, 0.08, 1, true);
                }
            }
        }
        FillPattern::Gravel => {
            for (p, r) in scatter(rings, paper(1.3)) {
                let rad = 0.25 + r * 0.25;
                if inside(rings, p.add(Pt::new(paper(rad), 0.0)))
                    && inside(rings, p.sub(Pt::new(paper(rad), 0.0)))
                {
                    b.circle(el, p, rad, 1, false);
                }
            }
        }
        FillPattern::Sand => {
            for (p, _) in scatter(rings, paper(0.8)) {
                b.circle(el, p, 0.06, 1, true);
            }
        }
        _ => {}
    }
    if let Some(style) = outline {
        let (w, dash) = line_style(style);
        for r in rings {
            b.line(el, r, true, w, dash);
        }
    }
}

/// A detail component's regions (masks and patterns) under its lines (ADR-071).
pub(crate) fn component(
    b: &mut Builder,
    el: Option<ElementId>,
    p: &studio_core::details::components::Parts,
) {
    for (rings, pattern) in &p.regions {
        region(b, el, rings, *pattern, None);
    }
    for l in &p.lines {
        let (w, dash) = line_style(l.style);
        b.line(el, &l.pts, l.closed, w, dash);
    }
}

/// A library detail drawn as its drafting view would be, for its thumbnail.
pub fn detail_preview(id: &str) -> Option<DisplayList> {
    let (scale, d) = details::drawing(id)?;
    let mut b = Builder::new(f64::from(scale));
    for (ring_pts, pattern) in &d.regions {
        region(&mut b, None, std::slice::from_ref(ring_pts), *pattern, None);
    }
    // Components among the lines in the order they were drafted, as inserted.
    let parts = d.component_parts();
    let comps = |b: &mut Builder, when: &dyn Fn(usize) -> bool| {
        for (c, p) in d.components.iter().zip(&parts) {
            if when(c.after) {
                component(b, None, p);
            }
        }
    };
    for (i, l) in d.lines.iter().enumerate() {
        comps(&mut b, &|after| after == i);
        let (w, dash) = line_style(l.style);
        b.line(None, &l.pts, l.closed, w, dash);
    }
    comps(&mut b, &|after| after >= d.lines.len());
    for n in &d.notes {
        let leader = studio_core::text::Leader {
            end: n.to,
            elbow: n.elbow,
            arc: false,
        };
        super::text_note(
            &mut b,
            None,
            (n.at, 0.0),
            &n.text,
            details::TEXT_SIZE,
            &[leader],
            n.align,
            None,
        );
    }
    let bounds = drafting_bounds(&b, 6.0);
    Some(DisplayList {
        view_type: ViewType::Drafting,
        scale,
        bounds,
        items: b.items,
    })
}

/// A drafting view's extent: what's drawn in it, with a paper margin.
pub(crate) fn drafting_bounds(b: &Builder, margin: f64) -> [f64; 4] {
    use super::Prim;
    let mut pts: Vec<Pt> = vec![];
    for it in &b.items {
        match &it.prim {
            Prim::Line { pts: p, .. } => pts.extend(p.iter().map(|q| Pt::new(q[0], q[1]))),
            Prim::Fill { rings, .. } => {
                pts.extend(rings.iter().flatten().map(|q| Pt::new(q[0], q[1])));
            }
            Prim::Text { at, text, size, .. } => {
                pts.push(Pt::new(at[0], at[1]));
                // Text runs right from its start.
                let w = text.chars().count() as f64 * size * 0.62;
                pts.push(Pt::new(at[0] + w, at[1] + size));
            }
            Prim::Image { min, max, .. } => {
                pts.push(Pt::new(min[0], min[1]));
                pts.push(Pt::new(max[0], max[1]));
            }
            Prim::Circle { c, r, .. } => {
                pts.push(Pt::new(c[0] - r, c[1] - r));
                pts.push(Pt::new(c[0] + r, c[1] + r));
            }
        }
    }
    let m = b.paper(margin);
    match studio_geom::bounds_of(&pts) {
        Some((lo, hi)) => [lo.x - m, lo.y - m, hi.x + m, hi.y + m],
        None => [-m * 10.0, -m * 10.0, m * 10.0, m * 10.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hatches_fill_the_region_and_skip_its_holes() {
        let sq = |x0: f64, y0: f64, s: f64| {
            vec![
                Pt::new(x0, y0),
                Pt::new(x0 + s, y0),
                Pt::new(x0 + s, y0 + s),
                Pt::new(x0, y0 + s),
            ]
        };
        // Horizontal lines every 10 across a 100 square: 10 lines of 100.
        let l = hatch_lines(&[sq(0.0, 0.0, 100.0)], 0.0, 10.0);
        assert_eq!(l.len(), 10);
        assert!(l.iter().all(|[a, b]| (a.dist(*b) - 100.0).abs() < 1e-6));
        // A hole in the middle splits the lines through it in two.
        let holed = [sq(0.0, 0.0, 100.0), sq(40.0, 40.0, 20.0)];
        let l = hatch_lines(&holed, 0.0, 10.0);
        assert_eq!(l.len(), 12);
        let total: f64 = l.iter().map(|[a, b]| a.dist(*b)).sum();
        assert!((total - (1000.0 - 2.0 * 20.0)).abs() < 1e-6);
        // Diagonal lines stay inside.
        for [a, b] in hatch_lines(&[sq(0.0, 0.0, 100.0)], 45f64.to_radians(), 7.0) {
            for p in [a, b] {
                assert!((-1e-6..=100.0 + 1e-6).contains(&p.x));
                assert!((-1e-6..=100.0 + 1e-6).contains(&p.y));
            }
        }
        // Stipples land inside only, and not in the hole.
        let pts = scatter(&holed, 5.0);
        assert!(pts.len() > 200);
        assert!(pts.iter().all(|(p, _)| inside(&holed, *p)));
    }

    #[test]
    fn an_inserted_detail_draws_in_its_drafting_view() {
        use crate::{display_list, Prim};
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let v = details::insert(&mut doc, "eave").unwrap();
        let dl = display_list(&doc, v).unwrap();
        assert_eq!(dl.view_type, ViewType::Drafting);
        assert_eq!(dl.scale, 8);
        let first_line = dl
            .items
            .iter()
            .position(|i| {
                matches!(i.prim, Prim::Line { .. })
                    && i.el.is_some_and(|e| {
                        matches!(doc.data(e), Ok(studio_core::ElementData::DetailLine { .. }))
                    })
            })
            .unwrap();
        let last_fill = dl
            .items
            .iter()
            .rposition(|i| {
                matches!(i.prim, Prim::Fill { .. })
                    && i.el.is_some_and(|e| {
                        matches!(
                            doc.data(e),
                            Ok(studio_core::ElementData::FilledRegion { .. })
                        )
                    })
            })
            .unwrap();
        assert!(last_fill < first_line, "regions under the detail lines");
        assert!(dl
            .items
            .iter()
            .any(|i| matches!(&i.prim, Prim::Text { text, .. } if text.contains("GUTTER"))));
        // Its extent holds the drawing and its notes: a few feet across at 1 1/2".
        let w = dl.bounds[2] - dl.bounds[0];
        assert!(w > 1500.0 && w < 12000.0, "{w}");
    }

    #[test]
    fn detail_components_draw_in_their_view_and_mirror_flips_them() {
        use crate::{display_list, Prim};
        use studio_core::ElementData;
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let v = details::create_drafting_view(&mut doc, "D", 4).unwrap();
        // A line under a break line: the break line's mask comes after it, covering it.
        doc.transact("line", |tx| {
            tx.insert(ElementData::DetailLine {
                view: v,
                curve: studio_core::sketch::SketchCurve::line(
                    Pt::new(0.0, 100.0),
                    Pt::new(1000.0, 100.0),
                ),
                style: studio_core::lines::LineStyle::Wide,
            });
            Ok(())
        })
        .unwrap();
        let brk = details::create_component(
            &mut doc,
            v,
            "break",
            Pt::new(0.0, 0.0),
            Pt::new(1000.0, 0.0),
            false,
        )
        .unwrap();
        let lum = details::create_component(
            &mut doc,
            v,
            "lum-2x6",
            Pt::new(2000.0, 0.0),
            Pt::new(2100.0, 0.0),
            false,
        )
        .unwrap();
        assert!(details::create_component(
            &mut doc,
            v,
            "break",
            Pt::new(0.0, 0.0),
            Pt::new(0.0, 0.0),
            false
        )
        .is_err());
        assert!(details::create_component(
            &mut doc,
            v,
            "nope",
            Pt::new(0.0, 0.0),
            Pt::new(9.0, 0.0),
            false
        )
        .is_err());
        let dl = display_list(&doc, v).unwrap();
        let mask = dl
            .items
            .iter()
            .position(|i| {
                i.el == Some(brk)
                    && matches!(
                        i.prim,
                        Prim::Fill {
                            fill: FillKind::Paper,
                            ..
                        }
                    )
            })
            .unwrap();
        let line = dl
            .items
            .iter()
            .position(|i| matches!(&i.prim, Prim::Line { w, .. } if *w >= 4) && i.el != Some(brk))
            .unwrap();
        assert!(line < mask, "the mask is drawn over the older line");
        assert!(dl.items.iter().filter(|i| i.el == Some(lum)).count() >= 4);
        // Mirrored, a line-based component flips to the other side of its line.
        let x = studio_core::edit::Xform::mirror(Pt::new(0.0, 500.0), Pt::new(1000.0, 500.0));
        studio_core::edit::transform_elements(&mut doc, &[brk], x, "Mirror").unwrap();
        assert!(matches!(
            doc.data(brk).unwrap(),
            ElementData::DetailComponent { flip: true, .. }
        ));
        // Properties: the type changes within its family only.
        studio_core::ops::set_property(&mut doc, lum, "type", "lum-2x10", 0).unwrap();
        assert!(studio_core::ops::set_property(&mut doc, lum, "type", "break", 0).is_err());
    }

    #[test]
    fn dimensions_in_drafting_views_follow_detail_lines_and_components() {
        use studio_core::ElementData;
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let v = details::create_drafting_view(&mut doc, "D", 4).unwrap();
        let line = |doc: &mut studio_core::Document, x: f64| {
            doc.transact("line", |tx| {
                Ok(tx.insert(ElementData::DetailLine {
                    view: v,
                    curve: studio_core::sketch::SketchCurve::line(
                        Pt::new(x, 0.0),
                        Pt::new(x, 1000.0),
                    ),
                    style: studio_core::lines::LineStyle::Medium,
                }))
            })
            .unwrap()
        };
        let (l1, l2) = (line(&mut doc, 0.0), line(&mut doc, 300.0));
        let refs_at =
            |doc: &studio_core::Document, p: Pt| crate::view_refs::references(doc, v, p, 20.0);
        let a = refs_at(&doc, Pt::new(2.0, 500.0));
        let b = refs_at(&doc, Pt::new(298.0, 500.0));
        assert!(
            matches!(a[0].anchor, Some(studio_core::Anchor::DetailLine { line, .. }) if line == l1)
        );
        let dim = studio_core::dimension::create_string(
            &mut doc,
            v,
            &[a[0].clone(), b[0].clone()],
            Pt::new(150.0, 800.0),
            studio_core::DimKind::Aligned,
        )
        .unwrap();
        let len = |doc: &studio_core::Document| {
            let (pts, u) =
                studio_core::dimension::string_points(doc, doc.data(dim).unwrap()).unwrap();
            pts[1].sub(pts[0]).dot(u).abs()
        };
        assert!((len(&doc) - 300.0).abs() < 1e-6);
        // Moving the second line 100 mm out stretches the dimension, as in Revit.
        studio_core::modify::move_elements(&mut doc, &[l2], Pt::new(100.0, 0.0)).unwrap();
        assert!((len(&doc) - 400.0).abs() < 1e-6, "{}", len(&doc));
        // A point on a detail component anchors to it and follows it.
        let c = details::create_component(
            &mut doc,
            v,
            "lum-2x6",
            Pt::new(1000.0, 0.0),
            Pt::new(1100.0, 0.0),
            false,
        )
        .unwrap();
        let corner = Pt::new(1000.0 + 0.75 * 25.4, 2.75 * 25.4);
        let anchor = studio_core::ops::anchor_at(&doc, v, corner).unwrap();
        assert!(
            matches!(anchor, studio_core::Anchor::Component { component, .. } if component == c)
        );
        studio_core::modify::move_elements(&mut doc, &[c], Pt::new(0.0, 50.0)).unwrap();
        let now = studio_core::ops::anchor_point(&doc, &anchor).unwrap();
        assert!(now.dist(corner.add(Pt::new(0.0, 50.0))) < 1e-6);
    }

    #[test]
    fn every_library_detail_has_a_preview() {
        for d in details::catalog() {
            let dl = detail_preview(&d.id).unwrap();
            assert_eq!(dl.view_type, ViewType::Drafting);
            assert_eq!(dl.scale, d.scale);
            assert!(dl.items.len() > 40, "{}", d.id);
            assert!(dl.bounds[2] > dl.bounds[0] && dl.bounds[3] > dl.bounds[1]);
        }
    }
}
