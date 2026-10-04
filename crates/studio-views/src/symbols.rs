//! Drawing annotation symbols (ADR-048): spot elevations, north arrows and graphic scales,
//! each in the style its standard is set to. Sizes are paper mm, scaled by the view.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use studio_core::element::{ElementData, ElementId, ViewKind};
use studio_core::symbols::{scale_label, scale_unit, spot_text, NorthStyle, ScaleStyle, SpotStyle};
use studio_core::units::MM_PER_FT;
use studio_core::Document;
use studio_geom::Pt;
use studio_regen::Model;

use crate::{ring, Anchor, Builder, Dash, FillKind};

/// The site's angle to true north (radians counter-clockwise), 0 without a site.
pub fn true_north(doc: &Document) -> f64 {
    doc.iter()
        .find_map(|e| match &e.data {
            ElementData::Site { rotation, .. } => Some(*rotation),
            _ => None,
        })
        .unwrap_or(0.0)
}

/// The site's datum: the surveyed height (mm) of project 0, if there's a site.
fn survey_datum(doc: &Document) -> Option<f64> {
    doc.iter().find_map(|e| match &e.data {
        ElementData::Site { base_elevation, .. } => Some(*base_elevation),
        _ => None,
    })
}

/// The model's height (project mm) at `at` in `view`: in plans, the top of the floor there
/// (at or below the cut plane), else the ground, else the level; in ceiling plans, the
/// ceiling; in elevations and sections, the point's own height.
pub fn spot_height(doc: &Document, model: &Model, view: ElementId, at: Pt) -> Option<f64> {
    let Ok(ElementData::View { kind, .. }) = doc.data(view) else {
        return None;
    };
    let level_z = |level: ElementId| {
        model
            .levels
            .iter()
            .find(|l| l.id == level)
            .map(|l| l.elevation)
    };
    match kind {
        ViewKind::FloorPlan { level } => {
            let e = level_z(*level)?;
            // Revit's cut plane: 4'-0" above the level.
            let cut = e + 4.0 * MM_PER_FT;
            let floor = model
                .floors
                .iter()
                .filter(|f| f.top_at(at) <= cut + 1.0 && f.base.contains(at))
                .map(|f| f.top_at(at))
                .fold(None, |m: Option<f64>, z| Some(m.map_or(z, |m| m.max(z))));
            floor
                .or_else(|| model.site.as_ref().and_then(|s| s.ground_at(at)))
                .or(Some(e))
        }
        ViewKind::CeilingPlan { level } => {
            let e = level_z(*level)?;
            let ceiling = model
                .ceilings
                .iter()
                .filter(|c| c.z0 >= e - 1.0 && c.base.contains(at))
                .map(|c| c.z0)
                .fold(None, |m: Option<f64>, z| Some(m.map_or(z, |m| m.min(z))));
            ceiling.or(Some(e))
        }
        ViewKind::Elevation { .. }
        | ViewKind::Section { .. }
        | ViewKind::MarkerElevation { .. } => Some(at.y),
        ViewKind::Drafting
        | ViewKind::Schedule { .. }
        | ViewKind::ThreeD
        | ViewKind::Rendering { .. } => None,
    }
}

/// Spot elevations, north arrows and graphic scales owned by `view` (a view or a sheet).
pub fn draw_symbols(doc: &Document, b: &mut Builder, view: ElementId) {
    let mut model: Option<std::sync::Arc<Model>> = None;
    for e in doc.iter() {
        match &e.data {
            ElementData::SpotElevation {
                view: v,
                at,
                leader,
            } if *v == view => {
                let m = model.get_or_insert_with(|| studio_regen::regenerate(doc));
                let Some(z) = spot_height(doc, m, view, *at) else {
                    continue;
                };
                let style = SpotStyle::of(doc);
                let text = spot_text(style, z, survey_datum(doc));
                spot_elevation(b, Some(e.id), *at, *leader, &text, style);
            }
            ElementData::SpotSlope {
                view: v,
                at,
                format,
                triangle,
            } if *v == view => {
                let m = model.get_or_insert_with(|| studio_regen::regenerate(doc));
                let in_plan = matches!(
                    doc.data(view),
                    Ok(ElementData::View {
                        kind: ViewKind::FloorPlan { .. },
                        ..
                    })
                );
                match crate::slopes::slope_at(doc, m, view, *at, b.paper(3.0)) {
                    Some(hit) => crate::slopes::spot_slope(
                        b,
                        Some(e.id),
                        *at,
                        &hit,
                        *format,
                        *triangle,
                        in_plan,
                    ),
                    // The model changed under it: say so, so it can be moved or deleted.
                    None => b.text(Some(e.id), *at, "NO SLOPE".into(), 2.0, Anchor::Center),
                }
            }
            ElementData::NorthArrow { view: v, at } if *v == view => {
                north_arrow(
                    b,
                    Some(e.id),
                    *at,
                    7.0,
                    NorthStyle::of(doc),
                    true_north(doc),
                );
            }
            ElementData::GraphicScale { view: v, at } if *v == view => {
                graphic_scale(b, Some(e.id), *at, ScaleStyle::of(doc));
            }
            _ => {}
        }
    }
}

/// `bounds` grown to take in the spot elevations, north arrows and graphic scales among
/// `items`, with `margin` around them.
pub fn grow_bounds(
    doc: &Document,
    items: &[crate::Item],
    bounds: [f64; 4],
    margin: f64,
) -> [f64; 4] {
    use studio_core::Category;
    let mut out = bounds;
    for it in items {
        let Some(el) = it.el else { continue };
        let is_symbol = doc.data(el).is_ok_and(|d| {
            matches!(
                d.category(),
                Category::SpotElevation
                    | Category::SpotSlope
                    | Category::NorthArrow
                    | Category::GraphicScale
            )
        });
        if !is_symbol {
            continue;
        }
        let mut take = |x: f64, y: f64, r: f64| {
            out[0] = out[0].min(x - r - margin);
            out[1] = out[1].min(y - r - margin);
            out[2] = out[2].max(x + r + margin);
            out[3] = out[3].max(y + r + margin);
        };
        match &it.prim {
            crate::Prim::Line { pts, .. } => pts.iter().for_each(|p| take(p[0], p[1], 0.0)),
            crate::Prim::Fill { rings, .. } => {
                rings.iter().flatten().for_each(|p| take(p[0], p[1], 0.0))
            }
            crate::Prim::Text { at, size, .. } => take(at[0], at[1], *size),
            crate::Prim::Circle { c, r, .. } => take(c[0], c[1], *r),
            crate::Prim::Image { min, max, .. } => {
                take(min[0], min[1], 0.0);
                take(max[0], max[1], 0.0);
            }
        }
    }
    out
}

/// Points on an arc of radius `r` about `c` from angle `a0` to `a1`.
fn arc(c: Pt, r: f64, a0: f64, a1: f64) -> Vec<Pt> {
    // About 7.5° a segment: round at any size on paper.
    let n = ((a1 - a0).abs() / TAU * 48.0).ceil().max(2.0) as usize;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f64 / n as f64;
            c.add(Pt::new(a.cos() * r, a.sin() * r))
        })
        .collect()
}

/// A spot elevation: its symbol on `at`, a leader to `leader` with a shoulder, the text.
pub fn spot_elevation(
    b: &mut Builder,
    el: Option<ElementId>,
    at: Pt,
    leader: Pt,
    text: &str,
    style: SpotStyle,
) {
    let s = b.paper(1.5);
    match style {
        SpotStyle::Triangle => {
            let tri = [
                at,
                at.add(Pt::new(-s, s * 1.6)),
                at.add(Pt::new(s, s * 1.6)),
            ];
            b.fill(el, vec![ring(&tri)], FillKind::Ink);
        }
        SpotStyle::Target => {
            b.line(el, &arc(at, s, 0.0, TAU), true, 2, Dash::Solid);
            for q in [0.0, PI] {
                let mut wedge = vec![at];
                wedge.extend(arc(at, s, q, q + FRAC_PI_2));
                b.fill(el, vec![ring(&wedge)], FillKind::Ink);
            }
        }
        SpotStyle::Cross => {
            b.line(
                el,
                &[at.add(Pt::new(-s, 0.0)), at.add(Pt::new(s, 0.0))],
                false,
                2,
                Dash::Solid,
            );
            b.line(
                el,
                &[at.add(Pt::new(0.0, -s)), at.add(Pt::new(0.0, s))],
                false,
                2,
                Dash::Solid,
            );
        }
        SpotStyle::TextOnly => {}
    }
    let lift = |p: Pt| {
        if style == SpotStyle::Triangle {
            p.add(Pt::new(0.0, s * 1.6))
        } else {
            p
        }
    };
    if at.dist(leader) > b.paper(0.5) {
        // Leader, then a short shoulder toward the side the text is on.
        let dir = if leader.x >= at.x { 1.0 } else { -1.0 };
        let start = lift(at);
        let end = leader.add(Pt::new(dir * b.paper(3.0), 0.0));
        b.line(el, &[start, leader, end], false, 1, Dash::Solid);
        let anchor = if dir > 0.0 {
            Anchor::Left
        } else {
            Anchor::Right
        };
        b.text(
            el,
            end.add(Pt::new(dir * b.paper(0.8), 0.0)),
            text.to_owned(),
            2.4,
            anchor,
        );
    } else {
        b.text(
            el,
            lift(at).add(Pt::new(b.paper(1.2), b.paper(1.4))),
            text.to_owned(),
            2.4,
            Anchor::Left,
        );
    }
}

/// A north arrow of radius `r` (paper mm) at `c`; `true_north` is the site's angle to true
/// north (plan north is +y).
pub fn north_arrow(
    b: &mut Builder,
    el: Option<ElementId>,
    c: Pt,
    r: f64,
    style: NorthStyle,
    true_north: f64,
) {
    let r = b.paper(r);
    let project = Pt::new(0.0, 1.0);
    let tn = Pt::new(-true_north.sin(), true_north.cos());
    // A pointer along `up`: tip, the two barbs and the notch at the center.
    let pointer = |up: Pt, len: f64| {
        let side = up.perp().scale(len * 0.42);
        let tip = c.add(up.scale(len));
        let back = c.sub(up.scale(len * 0.55));
        (tip, back.add(side), c, back.sub(side))
    };
    let letter = |b: &mut Builder, up: Pt, text: &str, dist: f64| {
        b.text(
            el,
            c.add(up.scale(dist)),
            text.into(),
            studio_core::text::sizes::NORTH,
            Anchor::Center,
        );
    };
    match style {
        NorthStyle::ProjectAndTrue => {
            let (tip, l, n, rt) = pointer(project, r);
            b.fill(el, vec![ring(&[tip, l, n, rt])], FillKind::Ink);
            letter(b, project, "N", r + b.paper(2.6));
            if true_north.abs() > 1e-3 {
                // A thin line to true north with a half arrowhead, clear of the arrow.
                let end = c.add(tn.scale(r * 1.6));
                let barb = end.sub(tn.scale(r * 0.3)).add(tn.perp().scale(r * 0.16));
                b.line(el, &[c, end, barb], false, 2, Dash::Solid);
                letter(b, tn, "TN", r * 1.6 + b.paper(2.2));
            }
        }
        NorthStyle::Circle | NorthStyle::TrueOnly => {
            let up = if style == NorthStyle::TrueOnly {
                tn
            } else {
                project
            };
            b.line(el, &arc(c, r, 0.0, TAU), true, 2, Dash::Solid);
            let (tip, l, n, _) = pointer(up, r * 0.95);
            b.fill(el, vec![ring(&[tip, l, n])], FillKind::Ink);
            let (tip, _, n, rt) = pointer(up, r * 0.95);
            b.line(el, &[tip, rt, n], false, 2, Dash::Solid);
            letter(b, up, "N", r + b.paper(2.6));
        }
        NorthStyle::HalfArrow => {
            let (tip, l, n, rt) = pointer(project, r);
            b.fill(el, vec![ring(&[tip, l, n])], FillKind::Ink);
            b.line(el, &[tip, l, n, rt], true, 2, Dash::Solid);
            letter(b, project, "N", r + b.paper(2.6));
        }
        NorthStyle::Compass => {
            b.line(el, &arc(c, r * 0.62, 0.0, TAU), true, 1, Dash::Solid);
            for (k, name) in ["N", "W", "S", "E"].iter().enumerate() {
                let a = FRAC_PI_2 + k as f64 * FRAC_PI_2;
                let up = Pt::new(a.cos(), a.sin());
                let side = up.perp().scale(r * 0.18);
                let tip = c.add(up.scale(r));
                if k == 0 {
                    b.fill(
                        el,
                        vec![ring(&[tip, c.add(side), c, c.sub(side)])],
                        FillKind::Ink,
                    );
                } else {
                    b.line(
                        el,
                        &[tip, c.add(side), c, c.sub(side)],
                        true,
                        1,
                        Dash::Solid,
                    );
                }
                letter(b, up, name, r + b.paper(2.4));
            }
        }
    }
}

/// A graphic scale with its left end at `at`, divided 0 · u · 2u · 4u for the view's scale.
pub fn graphic_scale(b: &mut Builder, el: Option<ElementId>, at: Pt, style: ScaleStyle) {
    let u = scale_unit(b.scale);
    let marks = [0.0, u, 2.0 * u, 4.0 * u];
    let x = |ft: f64| at.x + ft * MM_PER_FT;
    let h = b.paper(1.4);
    let rect = |x0: f64, x1: f64, y0: f64, y1: f64| {
        ring(&[
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ])
    };
    let bar = |b: &mut Builder, y0: f64, first_dark: bool| {
        for i in 0..3 {
            let dark = (i % 2 == 0) == first_dark;
            let r = rect(x(marks[i]), x(marks[i + 1]), y0, y0 + h);
            b.fill(
                el,
                vec![r.clone()],
                if dark { FillKind::Ink } else { FillKind::Paper },
            );
        }
        let outline = [
            Pt::new(x(0.0), y0),
            Pt::new(x(marks[3]), y0),
            Pt::new(x(marks[3]), y0 + h),
            Pt::new(x(0.0), y0 + h),
        ];
        b.line(el, &outline, true, 1, Dash::Solid);
    };
    let top = match style {
        ScaleStyle::Alternating | ScaleStyle::WithText => {
            bar(b, at.y, true);
            at.y + h
        }
        ScaleStyle::Checkered => {
            bar(b, at.y, true);
            bar(b, at.y + h, false);
            at.y + 2.0 * h
        }
        ScaleStyle::Ticks => {
            b.line(
                el,
                &[Pt::new(x(0.0), at.y), Pt::new(x(marks[3]), at.y)],
                false,
                2,
                Dash::Solid,
            );
            for (i, m) in marks.iter().enumerate() {
                let t = if i == 0 || i == 3 { h * 1.4 } else { h };
                b.line(
                    el,
                    &[Pt::new(x(*m), at.y), Pt::new(x(*m), at.y + t)],
                    false,
                    2,
                    Dash::Solid,
                );
            }
            at.y + h * 1.4
        }
    };
    for m in marks {
        b.text(
            el,
            Pt::new(x(m), top + b.paper(2.0)),
            scale_label(m),
            2.0,
            Anchor::Center,
        );
    }
    if style == ScaleStyle::WithText {
        b.text(
            el,
            Pt::new(x(marks[3] / 2.0), at.y - b.paper(2.2)),
            studio_core::ops::scale_label(b.scale.round() as u32),
            2.0,
            Anchor::Center,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Prim;

    fn texts(b: &Builder) -> Vec<String> {
        b.items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn graphic_scale_marks_follow_the_view_scale() {
        // 1/8" = 1'-0": 0, 4, 8, 16' along 16' of model.
        let mut b = Builder::new(96.0);
        graphic_scale(&mut b, None, Pt::new(0.0, 0.0), ScaleStyle::Alternating);
        assert_eq!(texts(&b), ["0", "4'", "8'", "16'"]);
        let xs: Vec<f64> = b
            .items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Text { at, .. } => Some(at[0]),
                _ => None,
            })
            .collect();
        assert!((xs[3] - 16.0 * 304.8).abs() < 1e-6);
        // Three segments, alternating ink and paper.
        let fills: Vec<FillKind> = b
            .items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Fill { fill, .. } => Some(*fill),
                _ => None,
            })
            .collect();
        assert_eq!(fills, [FillKind::Ink, FillKind::Paper, FillKind::Ink]);
        // 1/4" = 1'-0" with the scale written below.
        let mut b = Builder::new(48.0);
        graphic_scale(&mut b, None, Pt::new(0.0, 0.0), ScaleStyle::WithText);
        assert_eq!(texts(&b), ["0", "2'", "4'", "8'", "1/4\" = 1'-0\""]);
    }

    #[test]
    fn north_arrow_points_to_true_north_when_asked() {
        let tip = |style, tn| {
            let mut b = Builder::new(1.0);
            north_arrow(&mut b, None, Pt::new(0.0, 0.0), 7.0, style, tn);
            b.items
                .iter()
                .find_map(|i| match &i.prim {
                    Prim::Fill { rings, .. } => Some(rings[0][0]),
                    _ => None,
                })
                .unwrap()
        };
        // Project north: straight up.
        let t = tip(NorthStyle::Circle, 0.3);
        assert!(t[0].abs() < 1e-9 && t[1] > 6.0);
        // True north 30° counter-clockwise: the tip leans left.
        let a = 30f64.to_radians();
        let t = tip(NorthStyle::TrueOnly, a);
        assert!((t[0] - (-a.sin() * 7.0 * 0.95)).abs() < 1e-9);
        assert!((t[1] - a.cos() * 7.0 * 0.95).abs() < 1e-9);
    }

    #[test]
    fn spot_heights_read_the_model() {
        use studio_core::{ops, Category};
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let (l1, _, _) = doc.levels()[0].clone();
        let ft = doc.of(Category::FloorType).next().unwrap().id;
        let sq = vec![
            Pt::new(0.0, 0.0),
            Pt::new(3000.0, 0.0),
            Pt::new(3000.0, 3000.0),
            Pt::new(0.0, 3000.0),
        ];
        let floor = ops::create_floor(&mut doc, ft, l1, sq).unwrap();
        ops::set_property(&mut doc, floor, "offset", "6\"", 0).unwrap();
        let view = |doc: &Document, f: fn(&ViewKind) -> bool| {
            doc.of(Category::View)
                .find(|e| matches!(&e.data, ElementData::View { kind, .. } if f(kind)))
                .unwrap()
                .id
        };
        let plan = view(&doc, |k| matches!(k, ViewKind::FloorPlan { .. }));
        let elev = view(&doc, |k| matches!(k, ViewKind::Elevation { .. }));
        let model = studio_regen::regenerate(&doc);
        // On the floor: its top, 6" up.
        let z = spot_height(&doc, &model, plan, Pt::new(1500.0, 1500.0)).unwrap();
        assert!((z - 152.4).abs() < 1e-6, "{z}");
        // Off it: the level.
        let z = spot_height(&doc, &model, plan, Pt::new(9000.0, 9000.0)).unwrap();
        assert!(z.abs() < 1e-6);
        // In an elevation, the point's own height.
        assert_eq!(
            spot_height(&doc, &model, elev, Pt::new(10.0, 2743.2)),
            Some(2743.2)
        );
        // Drawn in the plan with its text.
        studio_core::symbols::create_spot_elevation(
            &mut doc,
            plan,
            Pt::new(1500.0, 1500.0),
            Pt::new(2500.0, 2500.0),
        )
        .unwrap();
        let dl = crate::display_list(&doc, plan).unwrap();
        assert!(dl
            .items
            .iter()
            .any(|i| matches!(&i.prim, Prim::Text { text, .. } if text == "0' - 6\"")));
        // A graphic scale placed far off is part of the view's extent (Zoom to Fit).
        studio_core::symbols::create_graphic_scale(&mut doc, plan, Pt::new(-20_000.0, -20_000.0))
            .unwrap();
        let dl = crate::display_list(&doc, plan).unwrap();
        assert!(
            dl.bounds[0] < -20_000.0 && dl.bounds[1] < -20_000.0,
            "{:?}",
            dl.bounds
        );
    }

    #[test]
    fn spot_elevation_text_goes_beyond_the_shoulder() {
        let mut b = Builder::new(48.0);
        let at = Pt::new(0.0, 0.0);
        let leader = Pt::new(1000.0, 1000.0);
        spot_elevation(&mut b, None, at, leader, "10' - 0\"", SpotStyle::Triangle);
        let text_at = b
            .items
            .iter()
            .find_map(|i| match &i.prim {
                Prim::Text { at, .. } => Some(*at),
                _ => None,
            })
            .unwrap();
        // Shoulder 3 mm + gap 0.8 mm on paper at 1:48, to the right of the leader's end.
        assert!((text_at[0] - (1000.0 + 3.8 * 48.0)).abs() < 1e-6);
        assert!((text_at[1] - 1000.0).abs() < 1e-6);
    }
}
