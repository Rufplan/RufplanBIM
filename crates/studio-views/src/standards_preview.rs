//! Preview drawings for the Standards tab's choices (ADR-048): what each choice of a
//! graphic standard looks like. The symbols the app draws (spot elevations, north arrows,
//! graphic scales) use their real drawing code; the rest are small samples. Paper mm.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use studio_core::symbols::{spot_text, NorthStyle, ScaleStyle, SpotStyle};
use studio_geom::Pt;

use crate::symbols::{graphic_scale, north_arrow, spot_elevation};
use crate::{ring, Anchor, Builder, Dash, DisplayList, FillKind, Prim, ViewType};

/// The preview of choice `index` of standard `item` in `category`; None for standards that
/// aren't graphic (the pop-up shows their text instead).
pub fn preview(category: &str, item: &str, index: usize) -> Option<DisplayList> {
    let mut b = Builder::new(1.0);
    let o = Pt::new(0.0, 0.0);
    match (category, item) {
        ("sym", "North Arrow") => {
            let style = *NorthStyle::ALL.get(index)?;
            north_arrow(&mut b, None, o, 7.0, style, 20f64.to_radians());
        }
        ("sym", "Graphic Scale") => {
            let style = *ScaleStyle::ALL.get(index)?;
            // At 1/8" = 1'-0", in model mm; shown on paper.
            b = Builder::new(96.0);
            graphic_scale(&mut b, None, o, style);
        }
        ("sym", "Spot Elevations") => {
            let style = *SpotStyle::ALL.get(index)?;
            b.line(
                None,
                &[Pt::new(-10.0, 0.0), Pt::new(10.0, 0.0)],
                false,
                3,
                Dash::Solid,
            );
            let text = spot_text(style, 3086.1, Some(125_577.6));
            spot_elevation(&mut b, None, o, Pt::new(6.0, 7.0), &text, style);
        }
        ("sheet", "Key Plan & North Arrow") => key_plan_strip(&mut b, index)?,
        ("sheet", "Sheet Size") => sheet_size(&mut b, index)?,
        ("dim", "Tick Style") => ticks(&mut b, index)?,
        ("sym", "Level Markers") => level(&mut b, index)?,
        ("sym", "Grid Bubbles") => grids(&mut b, index)?,
        ("sym", "Section Markers") => section(&mut b, index)?,
        ("sym", "Exterior Elevation Markers") => elevation_mark(&mut b, index)?,
        ("sym", "Interior Elevation Markers") => interior(&mut b, index)?,
        ("sym", "Detail Callouts") | ("sym", "Enlarged Callouts") => {
            callout(&mut b, item == "Enlarged Callouts", index)?
        }
        ("sym", "Revision Clouds & Deltas") => revision(&mut b, index)?,
        ("sym", "Break Lines") => break_line(&mut b, index)?,
        ("sym", "Matchlines") => matchline(&mut b, index)?,
        ("tags", _) => tag(&mut b, item, index)?,
        ("phase", "Existing" | "Demo" | "New" | "Future" | "NIC") => phase(&mut b, item, index)?,
        _ => return None,
    }
    finish(b)
}

fn finish(b: Builder) -> Option<DisplayList> {
    let mut pts: Vec<Pt> = vec![];
    for it in &b.items {
        match &it.prim {
            Prim::Line { pts: p, .. } => pts.extend(p.iter().map(|q| Pt::new(q[0], q[1]))),
            Prim::Fill { rings, .. } => {
                pts.extend(rings.iter().flatten().map(|q| Pt::new(q[0], q[1])))
            }
            Prim::Text { at, text, size, .. } => {
                // Roughly the text's extent, so labels aren't clipped.
                let w = text.chars().count() as f64 * size * 0.55;
                pts.push(Pt::new(at[0] - w, at[1] - size));
                pts.push(Pt::new(at[0] + w, at[1] + size));
            }
            Prim::Circle { c, r, .. } => {
                pts.push(Pt::new(c[0] - r, c[1] - r));
                pts.push(Pt::new(c[0] + r, c[1] + r));
            }
        }
    }
    let (lo, hi) = studio_regen::bounds(&pts)?;
    let pad = (hi.x - lo.x).max(hi.y - lo.y) * 0.08;
    Some(DisplayList {
        view_type: ViewType::Sheet,
        scale: b.scale.round() as u32,
        bounds: [lo.x - pad, lo.y - pad, hi.x + pad, hi.y + pad],
        items: b.items,
    })
}

fn arc(c: Pt, r: f64, a0: f64, a1: f64) -> Vec<Pt> {
    let n = 16;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f64 / n as f64;
            c.add(Pt::new(a.cos() * r, a.sin() * r))
        })
        .collect()
}

/// A regular polygon of `n` sides, radius `r`, first corner at angle `a0`.
fn polygon(c: Pt, r: f64, n: usize, a0: f64) -> Vec<Pt> {
    (0..n)
        .map(|i| {
            let a = a0 + TAU * i as f64 / n as f64;
            c.add(Pt::new(a.cos() * r, a.sin() * r))
        })
        .collect()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Pt> {
    vec![
        Pt::new(x0, y0),
        Pt::new(x1, y0),
        Pt::new(x1, y1),
        Pt::new(x0, y1),
    ]
}

fn text(b: &mut Builder, at: Pt, s: &str, size: f64) {
    b.text(None, at, s.into(), size, Anchor::Center);
}

fn key_plan_strip(b: &mut Builder, index: usize) -> Option<()> {
    if index > 3 {
        return None;
    }
    // A title block strip, 40 × 34 mm.
    b.line(None, &rect(0.0, 0.0, 40.0, 34.0), true, 2, Dash::Solid);
    let building = [
        Pt::new(4.0, 6.0),
        Pt::new(26.0, 6.0),
        Pt::new(26.0, 16.0),
        Pt::new(16.0, 16.0),
        Pt::new(16.0, 26.0),
        Pt::new(4.0, 26.0),
    ];
    let arrow = |b: &mut Builder, c: Pt| {
        north_arrow(b, None, c, 3.2, NorthStyle::Circle, 0.0);
    };
    match index {
        0 => {
            b.fill(None, vec![ring(&building)], FillKind::Slab);
            b.line(None, &building, true, 3, Dash::Solid);
            arrow(b, Pt::new(33.0, 22.0));
            b.text(
                None,
                Pt::new(3.0, 30.5),
                "KEY PLAN".into(),
                2.0,
                Anchor::Left,
            );
        }
        1 => {
            b.fill(None, vec![ring(&building)], FillKind::Slab);
            b.fill(
                None,
                vec![ring(&rect(4.0, 6.0, 16.0, 26.0))],
                FillKind::PocheLight,
            );
            b.line(None, &building, true, 3, Dash::Solid);
            arrow(b, Pt::new(33.0, 22.0));
            b.text(None, Pt::new(3.0, 30.5), "AREA A".into(), 2.0, Anchor::Left);
        }
        2 => {
            arrow(b, Pt::new(20.0, 16.0));
            b.text(None, Pt::new(3.0, 30.5), "NORTH".into(), 2.0, Anchor::Left);
        }
        _ => {
            b.line(
                None,
                &[Pt::new(8.0, 17.0), Pt::new(32.0, 17.0)],
                false,
                1,
                Dash::Dashed,
            );
        }
    }
    Some(())
}

/// A sheet drawn to scale against ARCH E (the largest), its title strip on the right.
fn sheet_size(b: &mut Builder, index: usize) -> Option<()> {
    let (w, h, name) = match index {
        0 => (36.0, 24.0, "24 × 36"),
        1 => (42.0, 30.0, "30 × 42"),
        2 => (48.0, 36.0, "36 × 48"),
        3 => (24.0, 18.0, "18 × 24"),
        4 => (17.0, 11.0, "11 × 17"),
        _ => return None,
    };
    // The largest sheet frames every preview, so sizes compare.
    b.line(None, &rect(0.0, 0.0, 48.0, 36.0), true, 1, Dash::Dashed);
    let (x0, y0) = ((48.0 - w) / 2.0, (36.0 - h) / 2.0);
    b.fill(
        None,
        vec![ring(&rect(x0, y0, x0 + w, y0 + h))],
        FillKind::Paper,
    );
    b.line(None, &rect(x0, y0, x0 + w, y0 + h), true, 3, Dash::Solid);
    let strip = x0 + w - w.min(h) * 0.12;
    b.line(
        None,
        &[Pt::new(strip, y0), Pt::new(strip, y0 + h)],
        false,
        1,
        Dash::Solid,
    );
    text(b, Pt::new(x0 + w / 2.0 - 1.5, y0 + h / 2.0), name, 3.2);
    Some(())
}

fn ticks(b: &mut Builder, index: usize) -> Option<()> {
    let (a, z) = (Pt::new(0.0, 0.0), Pt::new(30.0, 0.0));
    b.line(
        None,
        &[Pt::new(-2.0, 0.0), Pt::new(32.0, 0.0)],
        false,
        1,
        Dash::Solid,
    );
    for p in [a, z] {
        b.line(
            None,
            &[p.add(Pt::new(0.0, -4.0)), p.add(Pt::new(0.0, 2.0))],
            false,
            1,
            Dash::Solid,
        );
        match index {
            0 => b.line(
                None,
                &[p.add(Pt::new(-1.3, -1.3)), p.add(Pt::new(1.3, 1.3))],
                false,
                4,
                Dash::Solid,
            ),
            1 => {
                let d = if p.x > 0.0 { -1.0 } else { 1.0 };
                let tri = [
                    p,
                    p.add(Pt::new(d * 2.6, 0.8)),
                    p.add(Pt::new(d * 2.6, -0.8)),
                ];
                b.fill(None, vec![ring(&tri)], FillKind::Ink);
            }
            2 => b.circle(None, p, 0.7, 1, true),
            _ => return None,
        }
    }
    b.text(
        None,
        Pt::new(15.0, 2.2),
        "12'-6\"".into(),
        2.4,
        Anchor::Center,
    );
    Some(())
}

fn level(b: &mut Builder, index: usize) -> Option<()> {
    if index > 2 {
        return None;
    }
    let c = Pt::new(30.0, 0.0);
    b.line(
        None,
        &[Pt::new(0.0, 0.0), Pt::new(27.0, 0.0)],
        false,
        1,
        Dash::Center,
    );
    if index == 2 {
        let t = [Pt::new(27.0, 0.0), Pt::new(29.4, 2.4), Pt::new(29.4, -2.4)];
        b.fill(None, vec![ring(&t)], FillKind::Ink);
    } else {
        b.line(None, &arc(c, 2.4, 0.0, TAU), true, 2, Dash::Solid);
        for q in [0.0, PI] {
            let mut w = vec![c];
            w.extend(arc(c, 2.4, q, q + FRAC_PI_2));
            b.fill(None, vec![ring(&w)], FillKind::Ink);
        }
    }
    b.text(
        None,
        Pt::new(33.5, 1.6),
        "LEVEL 2".into(),
        2.4,
        Anchor::Left,
    );
    let elev = if index == 0 {
        "110' - 0\""
    } else {
        "10' - 0\""
    };
    b.text(None, Pt::new(33.5, -1.6), elev.into(), 2.4, Anchor::Left);
    Some(())
}

fn bubble(b: &mut Builder, c: Pt, label: &str) {
    b.circle(None, c, 3.2, 2, false);
    text(b, c, label, 2.8);
}

fn grids(b: &mut Builder, index: usize) -> Option<()> {
    let (v, h) = match index {
        0 | 2 => (["1", "2"], ["A", "B"]),
        1 => (["A", "B"], ["1", "2"]),
        _ => return None,
    };
    for (i, l) in v.iter().enumerate() {
        let x = i as f64 * 14.0;
        b.line(
            None,
            &[Pt::new(x, 0.0), Pt::new(x, 24.0)],
            false,
            1,
            Dash::Center,
        );
        bubble(b, Pt::new(x, 27.2), l);
        if index == 2 {
            bubble(b, Pt::new(x, -3.2), l);
        }
    }
    for (i, l) in h.iter().enumerate() {
        let y = 6.0 + i as f64 * 12.0;
        b.line(
            None,
            &[Pt::new(-6.0, y), Pt::new(20.0, y)],
            false,
            1,
            Dash::Center,
        );
        bubble(b, Pt::new(-9.2, y), l);
        if index == 2 {
            bubble(b, Pt::new(23.2, y), l);
        }
    }
    Some(())
}

fn section(b: &mut Builder, index: usize) -> Option<()> {
    let c = Pt::new(0.0, 0.0);
    b.line(
        None,
        &[Pt::new(4.5, 0.0), Pt::new(34.0, 0.0)],
        false,
        4,
        Dash::Center,
    );
    match index {
        0 => {
            b.circle(None, c, 4.5, 2, false);
            b.line(
                None,
                &[Pt::new(-4.5, 0.0), Pt::new(4.5, 0.0)],
                false,
                1,
                Dash::Solid,
            );
            text(b, Pt::new(0.0, 2.0), "3", 2.4);
            text(b, Pt::new(0.0, -2.0), "A-301", 1.8);
        }
        1 => {
            b.circle(None, c, 4.5, 2, false);
            let t = [Pt::new(-3.2, 3.2), Pt::new(0.0, 7.5), Pt::new(3.2, 3.2)];
            b.fill(None, vec![ring(&t)], FillKind::Ink);
            text(b, Pt::new(0.0, 1.0), "3", 2.4);
            text(b, Pt::new(0.0, -2.0), "A-301", 1.8);
        }
        2 => {
            let mut half = arc(c, 4.5, 0.0, PI);
            half.push(c);
            // The circle paints paper inside, so it goes first.
            b.circle(None, c, 4.5, 2, false);
            b.fill(None, vec![ring(&half)], FillKind::Ink);
            text(b, Pt::new(0.0, -2.0), "3 / A-301", 1.6);
        }
        _ => return None,
    }
    Some(())
}

fn elevation_mark(b: &mut Builder, index: usize) -> Option<()> {
    let c = Pt::new(0.0, 0.0);
    match index {
        0 => {
            b.circle(None, c, 4.0, 2, false);
            let t = [Pt::new(-4.0, 0.0), Pt::new(0.0, 6.5), Pt::new(4.0, 0.0)];
            b.fill(None, vec![ring(&t)], FillKind::Ink);
            text(b, Pt::new(0.0, -1.5), "1", 2.4);
        }
        1 => {
            b.line(None, &rect(-4.0, -4.0, 4.0, 4.0), true, 2, Dash::Solid);
            let t = [Pt::new(-4.0, 4.0), Pt::new(0.0, 8.0), Pt::new(4.0, 4.0)];
            b.fill(None, vec![ring(&t)], FillKind::Ink);
            text(b, c, "1", 2.4);
        }
        2 => {
            b.circle(None, c, 4.0, 2, false);
            let t = [Pt::new(-3.0, 2.6), Pt::new(0.0, 9.0), Pt::new(3.0, 2.6)];
            b.line(None, &t, true, 2, Dash::Solid);
            text(b, Pt::new(0.0, 5.0), "1", 2.0);
            text(b, Pt::new(0.0, -1.0), "A-201", 1.6);
        }
        _ => return None,
    }
    Some(())
}

fn interior(b: &mut Builder, index: usize) -> Option<()> {
    let n = match index {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 4,
        _ => return None,
    };
    let c = Pt::new(0.0, 0.0);
    b.line(None, &polygon(c, 5.0, 4, PI / 4.0), true, 2, Dash::Solid);
    let dirs = [
        (Pt::new(0.0, 1.0), "1"),
        (Pt::new(1.0, 0.0), "2"),
        (Pt::new(0.0, -1.0), "3"),
        (Pt::new(-1.0, 0.0), "4"),
    ];
    let pick: &[usize] = match n {
        1 => &[0],
        2 => &[0, 2],
        3 => &[0, 1, 2],
        _ => &[0, 1, 2, 3],
    };
    for &i in pick {
        let (d, l) = dirs[i];
        let side = d.perp().scale(3.0);
        let base = c.add(d.scale(3.53));
        let t = [base.add(side), c.add(d.scale(7.0)), base.sub(side)];
        b.fill(None, vec![ring(&t)], FillKind::Ink);
        text(b, c.add(d.scale(1.8)), l, 1.8);
    }
    Some(())
}

fn callout(b: &mut Builder, enlarged: bool, index: usize) -> Option<()> {
    let bubble_at = Pt::new(22.0, 14.0);
    match (enlarged, index) {
        (false, 0) => b.line(
            None,
            &arc(Pt::new(0.0, 0.0), 8.0, 0.0, TAU),
            true,
            2,
            Dash::Dashed,
        ),
        (false, 1) => {
            let mut r = arc(Pt::new(6.0, 4.0), 2.0, 0.0, FRAC_PI_2);
            r.extend(arc(Pt::new(-6.0, 4.0), 2.0, FRAC_PI_2, PI));
            r.extend(arc(Pt::new(-6.0, -4.0), 2.0, PI, 1.5 * PI));
            r.extend(arc(Pt::new(6.0, -4.0), 2.0, 1.5 * PI, TAU));
            b.line(None, &r, true, 2, Dash::Dashed);
        }
        (false, 2) => b.circle(None, Pt::new(0.0, 0.0), 0.8, 1, true),
        (true, 0) => {
            b.line(None, &rect(-10.0, -7.0, 10.0, 7.0), true, 2, Dash::Dashed);
            b.line(None, &rect(10.0, 7.0, 20.0, 12.0), true, 2, Dash::Solid);
            text(b, Pt::new(15.0, 9.5), "1/A-401", 1.8);
            return Some(());
        }
        (true, 1) => b.line(None, &rect(-10.0, -7.0, 10.0, 7.0), true, 2, Dash::Dashed),
        (true, 2) => {
            b.line(None, &rect(-10.0, -7.0, 10.0, 7.0), true, 1, Dash::Center);
            b.text(
                None,
                Pt::new(-10.0, 9.0),
                "ENLARGED PLAN".into(),
                1.8,
                Anchor::Left,
            );
            return Some(());
        }
        _ => return None,
    }
    b.line(
        None,
        &[Pt::new(5.7, 5.7), bubble_at.add(Pt::new(-3.0, -1.5))],
        false,
        1,
        Dash::Solid,
    );
    b.circle(None, bubble_at, 3.6, 2, false);
    b.line(
        None,
        &[
            bubble_at.add(Pt::new(-3.6, 0.0)),
            bubble_at.add(Pt::new(3.6, 0.0)),
        ],
        false,
        1,
        Dash::Solid,
    );
    text(b, bubble_at.add(Pt::new(0.0, 1.6)), "5", 2.0);
    text(b, bubble_at.add(Pt::new(0.0, -1.6)), "A-501", 1.4);
    Some(())
}

fn revision(b: &mut Builder, index: usize) -> Option<()> {
    // A cloud: bumps around a rounded box.
    let (w, h, r) = (24.0, 14.0, 2.0);
    let mut pts = vec![];
    let mut edge = |a: Pt, z: Pt| {
        let n = (a.dist(z) / (2.0 * r)).round().max(1.0) as usize;
        let d = z.sub(a).scale(1.0 / n as f64);
        let out = d.perp().norm().scale(-1.0);
        let ang = out.y.atan2(out.x);
        for k in 0..n {
            let c = a.add(d.scale(k as f64 + 0.5));
            pts.extend(
                arc(c, d.len() / 2.0, ang - FRAC_PI_2, ang + FRAC_PI_2)
                    .iter()
                    .rev(),
            );
        }
    };
    edge(Pt::new(0.0, 0.0), Pt::new(w, 0.0));
    edge(Pt::new(w, 0.0), Pt::new(w, h));
    edge(Pt::new(w, h), Pt::new(0.0, h));
    edge(Pt::new(0.0, h), Pt::new(0.0, 0.0));
    b.line(None, &pts, true, 2, Dash::Solid);
    let c = Pt::new(w + 6.0, h + 4.0);
    let (shape, label) = match index {
        0 => (polygon(c, 3.6, 3, FRAC_PI_2), "1"),
        1 => (polygon(c, 3.6, 3, FRAC_PI_2), "A"),
        2 => (polygon(c, 3.4, 6, 0.0), "1"),
        _ => return None,
    };
    b.line(None, &shape, true, 2, Dash::Solid);
    text(b, c.add(Pt::new(0.0, -0.4)), label, 2.2);
    Some(())
}

fn break_line(b: &mut Builder, index: usize) -> Option<()> {
    let zig = |x: f64| -> Vec<Pt> {
        vec![
            Pt::new(x, 0.0),
            Pt::new(x + 1.5, 3.0),
            Pt::new(x + 3.0, -3.0),
            Pt::new(x + 4.5, 0.0),
        ]
    };
    let mut line = vec![Pt::new(0.0, 0.0)];
    match index {
        0 => line.extend(zig(14.0)),
        1 => {
            line.extend(zig(8.0));
            line.extend(zig(20.0));
        }
        2 => {
            line.extend(arc(Pt::new(15.0, 0.0), 2.5, PI, TAU).iter().rev());
            line.extend(arc(Pt::new(20.0, 0.0), 2.5, PI, 0.0).iter().skip(1));
        }
        _ => return None,
    }
    line.push(Pt::new(34.0, 0.0));
    b.line(None, &line, false, 1, Dash::Solid);
    Some(())
}

fn matchline(b: &mut Builder, index: usize) -> Option<()> {
    match index {
        0 => b.line(
            None,
            &[Pt::new(0.0, 0.0), Pt::new(0.0, 30.0)],
            false,
            5,
            Dash::Center,
        ),
        1 => {
            b.fill(
                None,
                vec![ring(&rect(0.0, 0.0, 12.0, 30.0))],
                FillKind::PocheLight,
            );
            b.line(
                None,
                &[Pt::new(0.0, 0.0), Pt::new(0.0, 30.0)],
                false,
                5,
                Dash::Dashed,
            );
        }
        2 => b.line(
            None,
            &[Pt::new(0.0, 0.0), Pt::new(0.0, 30.0)],
            false,
            5,
            Dash::Solid,
        ),
        _ => return None,
    }
    b.text_rot(
        None,
        Pt::new(-2.2, 15.0),
        "MATCHLINE – SEE A-102".into(),
        1.8,
        Anchor::Center,
        FRAC_PI_2,
    );
    if index == 2 {
        b.text_rot(
            None,
            Pt::new(2.2, 15.0),
            "SEE A-101".into(),
            1.8,
            Anchor::Center,
            FRAC_PI_2,
        );
    }
    Some(())
}

fn tag(b: &mut Builder, item: &str, index: usize) -> Option<()> {
    let c = Pt::new(0.0, 0.0);
    let (shape, label): (Option<Vec<Pt>>, &str) = match (item, index) {
        ("Room", 0) => {
            text(b, Pt::new(0.0, 3.2), "LIVING", 2.6);
            text(b, Pt::new(0.0, 0.0), "101", 2.4);
            text(b, Pt::new(0.0, -3.0), "240 SF", 2.0);
            return Some(());
        }
        ("Room", 1) => {
            text(b, Pt::new(0.0, 1.6), "LIVING", 2.6);
            text(b, Pt::new(0.0, -1.6), "101", 2.4);
            return Some(());
        }
        ("Room", 2) => {
            text(b, Pt::new(0.0, 3.0), "LIVING", 2.6);
            b.line(None, &rect(-3.6, -2.4, 3.6, 1.2), true, 2, Dash::Solid);
            text(b, Pt::new(0.0, -0.6), "101", 2.2);
            return Some(());
        }
        ("Room", 3) => {
            text(b, Pt::new(0.0, 3.2), "LIVING", 2.6);
            text(b, Pt::new(0.0, 0.0), "101", 2.4);
            text(b, Pt::new(0.0, -3.0), "F-2 / B-1", 2.0);
            return Some(());
        }
        ("Door", 0) => (None, "101A"),
        ("Door", 1) => (Some(polygon(c, 3.6, 6, 0.0)), "7"),
        ("Door", 2) => (Some(rect(-4.0, -2.4, 4.0, 2.4)), "D2"),
        ("Window", 0) => (Some(polygon(c, 3.4, 6, 0.0)), "A"),
        ("Window", 1) => (Some(polygon(c, 3.6, 4, 0.0)), "A"),
        ("Window", 2) => (None, "3"),
        ("Wall Type", 0) => (Some(polygon(c, 3.8, 4, 0.0)), "4A"),
        ("Wall Type", 1) => (Some(polygon(c, 3.4, 6, 0.0)), "2"),
        ("Wall Type", 2) => (Some(rect(-4.2, -2.2, 4.2, 2.2)), "W-1"),
        ("Ceiling", 0) => (Some(rect(-6.0, -3.0, 6.0, 3.0)), "ACT-1 / 9'-0\""),
        ("Ceiling", 1) => (Some(ellipse(c, 7.5, 3.4)), "C-1 / 9'-0\""),
        ("Ceiling", 2) => (Some(vec![]), "9'-0\" AFF"),
        ("Finish", 0) => (Some(rect(-5.0, -2.4, 5.0, 2.4)), "F-2 W-1"),
        ("Finish", 1) => (Some(vec![]), "LIVING 101 · F-2"),
        ("Finish", 2) => (Some(rect(-5.5, -2.4, 5.5, 2.4)), "P-2 1910"),
        ("Finish", 3) => (Some(vec![]), "—"),
        ("Equipment", 0) => (Some(rounded(c, 5.0, 2.4, 1.2)), "EQ-01"),
        ("Equipment", 1) => (Some(polygon(c, 3.6, 6, 0.0)), "12"),
        ("Equipment", 2) => (None, "12"),
        ("Casework", 0) => (Some(polygon(c, 3.6, 6, 0.0)), "B3"),
        ("Casework", 1) => (Some(rect(-4.4, -2.2, 4.4, 2.2)), "CW-01"),
        ("Casework", 2) => (Some(polygon(c, 3.6, 4, 0.0)), "A"),
        ("Keynote", 0) => (Some(rect(-6.5, -2.2, 6.5, 2.2)), "04 20 00.A1"),
        ("Keynote", 1) => (None, "4"),
        ("Keynote", 2) => (Some(polygon(c, 3.4, 6, 0.0)), "4"),
        _ => return None,
    };
    match shape {
        None => b.circle(None, c, 3.6, 2, false),
        Some(s) if !s.is_empty() => b.line(None, &s, true, 2, Dash::Solid),
        Some(_) => {}
    }
    text(b, c, label, 2.0);
    Some(())
}

fn ellipse(c: Pt, rx: f64, ry: f64) -> Vec<Pt> {
    (0..32)
        .map(|i| {
            let a = TAU * i as f64 / 32.0;
            c.add(Pt::new(a.cos() * rx, a.sin() * ry))
        })
        .collect()
}

fn rounded(c: Pt, hw: f64, hh: f64, r: f64) -> Vec<Pt> {
    let mut v = arc(c.add(Pt::new(hw - r, hh - r)), r, 0.0, FRAC_PI_2);
    v.extend(arc(c.add(Pt::new(-hw + r, hh - r)), r, FRAC_PI_2, PI));
    v.extend(arc(c.add(Pt::new(-hw + r, -hh + r)), r, PI, 1.5 * PI));
    v.extend(arc(c.add(Pt::new(hw - r, -hh + r)), r, 1.5 * PI, TAU));
    v
}

/// A short wall drawn as the phase shows it.
fn phase(b: &mut Builder, item: &str, index: usize) -> Option<()> {
    let wall = rect(0.0, 0.0, 34.0, 3.0);
    let (fill, w, dash) = match (item, index) {
        ("Existing", 0) | ("Existing", 2) => (Some(FillKind::PocheLight), 1, Dash::Solid),
        ("Existing", 1) => (Some(FillKind::Poche), 4, Dash::Solid),
        ("Demo", 0) | ("Demo", 1) => (None, 2, Dash::Dashed),
        ("Demo", 2) => {
            b.line(None, &wall, true, 2, Dash::Dashed);
            for k in 0..6 {
                let x = k as f64 * 6.0;
                b.line(
                    None,
                    &[Pt::new(x, 0.0), Pt::new(x + 3.0, 3.0)],
                    false,
                    1,
                    Dash::Solid,
                );
                b.line(
                    None,
                    &[Pt::new(x, 3.0), Pt::new(x + 3.0, 0.0)],
                    false,
                    1,
                    Dash::Solid,
                );
            }
            return Some(());
        }
        ("New", 0) => (None, 4, Dash::Solid),
        ("New", 1) => (Some(FillKind::Poche), 4, Dash::Solid),
        ("Future", 0) => (None, 1, Dash::Center),
        ("Future", 1) => (Some(FillKind::PocheLight), 1, Dash::Dashed),
        ("Future", 2) => {
            b.text(
                None,
                Pt::new(17.0, 1.5),
                "(not shown)".into(),
                2.0,
                Anchor::Center,
            );
            return Some(());
        }
        ("NIC", 0) => (None, 1, Dash::Dashed),
        ("NIC", 1) => (Some(FillKind::PocheLight), 1, Dash::Solid),
        ("NIC", 2) => (None, 1, Dash::Center),
        _ => return None,
    };
    if let Some(f) = fill {
        b.fill(None, vec![ring(&wall)], f);
    }
    b.line(None, &wall, true, w, dash);
    if item == "NIC" {
        b.text(None, Pt::new(17.0, 5.5), "NIC".into(), 2.2, Anchor::Center);
    }
    if item == "Demo" && index == 0 {
        b.text(None, Pt::new(17.0, 5.5), "D1".into(), 2.2, Anchor::Center);
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::standards::{library, LIBRARIES};
    use studio_core::standards_catalog::choices;

    #[test]
    fn graphic_standards_preview_every_choice() {
        let s = library(LIBRARIES[0]).unwrap();
        let mut drawn = 0;
        for c in &s.categories {
            for it in &c.items {
                let n = choices(&c.id, &it.name).len();
                let shown: Vec<bool> = (0..n)
                    .map(|i| preview(&c.id, &it.name, i).is_some())
                    .collect();
                // A graphic standard previews every choice; others none.
                assert!(
                    shown.iter().all(|s| *s) || shown.iter().all(|s| !*s),
                    "{} / {}: {shown:?}",
                    c.id,
                    it.name
                );
                if shown.first() == Some(&true) {
                    drawn += 1;
                    assert!(preview(&c.id, &it.name, n).is_none(), "no choice {n}");
                }
            }
        }
        // 13 symbols, 9 tags, the tick style, sheet size, the key plan and five phases.
        assert_eq!(drawn, 30);
    }

    #[test]
    fn previews_fit_their_drawing() {
        let dl = preview("sym", "North Arrow", 0).unwrap();
        let [x0, y0, x1, y1] = dl.bounds;
        // A 7 mm arrow with its N above and the TN line leaning left: about 13 × 20 mm.
        assert!(x1 - x0 > 10.0 && x1 - x0 < 20.0, "{:?}", dl.bounds);
        assert!(y1 - y0 > 16.0 && y1 - y0 < 26.0, "{:?}", dl.bounds);
        let gs = preview("sym", "Graphic Scale", 0).unwrap();
        assert_eq!(gs.scale, 96);
        assert!(gs.bounds[2] - gs.bounds[0] > 16.0 * 304.8);
    }
}
