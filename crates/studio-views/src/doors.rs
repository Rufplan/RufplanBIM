//! Door graphics by family (ADR-033): plan symbols (swings, sliders, pockets, barn tracks,
//! bifolds, folding walls, overhead doors), elevation detail, the 3D frame, casing, leaves,
//! panels, muntins and glass, and thumbnails. Layouts come from `studio_core::doors`.

use studio_core::doors::{layout, DoorLayout, DoorOp, DoorStyle, Leaf, BAR, LEAF};
use studio_core::{DoorFamily, ElementId};
use studio_geom::{Poly, Prism, Pt};
use studio_regen::OpeningSolid;

use super::windows::{Parts, WindowLine, WindowPreview};
use super::{arc, Builder, Dash, FillKind};

const IN: f64 = 25.4;
/// Casing projection from the wall face.
const CASING_DEPTH: f64 = 0.75 * IN;
const GLASS: f64 = 0.25 * IN;

/// A door's own axes in plan: `u` runs from the opening's start along the wall, `w` across
/// it, + toward the swing (facing) side.
#[derive(Debug, Clone, Copy)]
struct Axes {
    origin: Pt,
    d: Pt,
    out: Pt,
}

impl Axes {
    fn of(o: &OpeningSolid) -> Self {
        let s = if o.flip_facing { -1.0 } else { 1.0 };
        Axes {
            origin: o.at(o.t0),
            d: o.dir,
            out: o.dir.perp().scale(s),
        }
    }
    fn at(&self, u: f64, w: f64) -> Pt {
        self.origin.add(self.d.scale(u)).add(self.out.scale(w))
    }
}

pub fn layout_of(o: &OpeningSolid, s: DoorStyle) -> DoorLayout {
    layout(s, o.width(), o.z1 - o.z0, !o.flip_hand)
}

// ---------------------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------------------

pub(crate) fn plan_symbol(b: &mut Builder, el: Option<ElementId>, o: &OpeningSolid, s: DoorStyle) {
    let lay = layout_of(o, s);
    let x = Axes::of(o);
    let h = o.half_thickness;
    let w = lay.width;
    let line =
        |b: &mut Builder, pts: &[Pt], weight: u8, dash: Dash| b.line(el, pts, false, weight, dash);
    let rect = |b: &mut Builder, u0: f64, u1: f64, w0: f64, w1: f64, weight: u8, dash: Dash| {
        b.line(
            el,
            &[x.at(u0, w0), x.at(u1, w0), x.at(u1, w1), x.at(u0, w1)],
            true,
            weight,
            dash,
        );
    };
    match s.family {
        DoorFamily::SlidingGlass => {
            for off in [h, -h] {
                line(b, &[x.at(0.0, off), x.at(w, off)], 1, Dash::Solid);
            }
            for l in &lay.leaves {
                rect(
                    b,
                    l.rect[0],
                    l.rect[2],
                    l.track - 0.5 * IN,
                    l.track + 0.5 * IN,
                    2,
                    Dash::Solid,
                );
            }
        }
        DoorFamily::FoldingWall => {
            for off in [h, -h] {
                line(b, &[x.at(0.0, off), x.at(w, off)], 1, Dash::Solid);
            }
            // Panels drawn part-folded, zigzagging out from the stacking jamb.
            let n = lay.leaves.len().max(1) as f64;
            let pw = (w - 2.0 * lay.frame) / n;
            let (c, sn) = (0.85_f64, 0.527_f64);
            let mut pts = vec![x.at(lay.frame, h)];
            for i in 0..lay.leaves.len() {
                let u = lay.frame + pw * c * (i as f64 + 1.0);
                let off = if i % 2 == 0 { h + pw * sn } else { h };
                pts.push(x.at(u, off));
            }
            line(b, &pts, 2, Dash::Solid);
        }
        DoorFamily::Bifold => {
            // Each pair folds out from its jamb.
            let mut pairs: Vec<(f64, f64, bool)> = vec![];
            for l in &lay.leaves {
                let DoorOp::Fold { hinge_left } = l.op else {
                    continue;
                };
                match pairs.last_mut() {
                    Some(p) if p.2 == hinge_left => {
                        p.0 = p.0.min(l.rect[0]);
                        p.1 = p.1.max(l.rect[2]);
                    }
                    _ => pairs.push((l.rect[0], l.rect[2], hinge_left)),
                }
            }
            for (a, bb, left) in pairs {
                let pw = (bb - a) / 2.0;
                let (start, dir) = if left { (a, 1.0) } else { (bb, -1.0) };
                let pts = [
                    x.at(start, h),
                    x.at(start + dir * pw * 0.7, h + pw * 0.7),
                    x.at(start + dir * pw * 1.4, h),
                ];
                line(b, &pts, 2, Dash::Solid);
            }
        }
        DoorFamily::Pocket => {
            for l in &lay.leaves {
                let DoorOp::Pocket { into_left } = l.op else {
                    continue;
                };
                let lw = l.rect[2] - l.rect[0];
                let t = LEAF / 2.0;
                // The pocket in the wall, dashed, and the leaf half open.
                let (p0, p1) = if into_left {
                    (l.rect[0] - lw, l.rect[0])
                } else {
                    (l.rect[2], l.rect[2] + lw)
                };
                rect(b, p0, p1, -t - 0.5 * IN, t + 0.5 * IN, 1, Dash::Dashed);
                let shift = if into_left { -lw / 2.0 } else { lw / 2.0 };
                rect(
                    b,
                    l.rect[0] + shift,
                    l.rect[2] + shift,
                    -t,
                    t,
                    2,
                    Dash::Solid,
                );
            }
        }
        DoorFamily::Barn => {
            for l in &lay.leaves {
                let DoorOp::Barn { to_right } = l.op else {
                    continue;
                };
                let face = h + 0.5 * IN;
                rect(b, l.rect[0], l.rect[2], face, face + LEAF, 2, Dash::Solid);
                // The track, running as far again as the leaf slides.
                let lw = l.rect[2] - l.rect[0];
                let (a, e) = if to_right {
                    (l.rect[0], l.rect[2] + lw)
                } else {
                    (l.rect[0] - lw, l.rect[2])
                };
                line(
                    b,
                    &[x.at(a, face - 0.25 * IN), x.at(e, face - 0.25 * IN)],
                    1,
                    Dash::Dashed,
                );
            }
        }
        DoorFamily::Garage => {
            // The door closed, and open overhead (dashed) inside.
            line(
                b,
                &[x.at(0.0, -h + LEAF), x.at(w, -h + LEAF)],
                2,
                Dash::Solid,
            );
            let depth = lay.height;
            rect(b, 0.0, w, -h, -h - depth, 1, Dash::Dashed);
        }
        _ => {
            // Swing doors (single, pair, entry, storefront): leaf open 90° and its arc.
            for l in &lay.leaves {
                match l.op {
                    DoorOp::Swing { hinge_left } => {
                        let (hinge, other) = if hinge_left {
                            (l.rect[0], l.rect[2])
                        } else {
                            (l.rect[2], l.rect[0])
                        };
                        let r = l.rect[2] - l.rect[0];
                        let hf = x.at(hinge, h);
                        let tip = hf.add(x.out.scale(r));
                        line(b, &[hf, tip], 3, Dash::Solid);
                        let closing = x.at(other, h).sub(hf).norm();
                        let a0 = x.out.y.atan2(x.out.x);
                        let a1 = closing.y.atan2(closing.x);
                        let mut sweep = a1 - a0;
                        while sweep > std::f64::consts::PI {
                            sweep -= std::f64::consts::TAU;
                        }
                        while sweep < -std::f64::consts::PI {
                            sweep += std::f64::consts::TAU;
                        }
                        line(b, &arc(hf, r, a0, sweep), 1, Dash::Solid);
                    }
                    DoorOp::Fixed => {
                        // Sidelite glass.
                        let g = h * 0.22;
                        for off in [g, -g] {
                            line(
                                b,
                                &[x.at(l.rect[0], off), x.at(l.rect[2], off)],
                                2,
                                Dash::Solid,
                            );
                        }
                    }
                    _ => {}
                }
            }
            for m in &lay.mullions {
                rect(b, m[0], m[2], -h, h, 1, Dash::Solid);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// Elevation
// ---------------------------------------------------------------------------------------

fn wl(pts: &[[f64; 2]], closed: bool, dashed: bool, w: u8, glass: bool) -> WindowLine {
    WindowLine {
        pts: pts.to_vec(),
        closed,
        dashed,
        w,
        glass,
    }
}

fn r4(r: [f64; 4]) -> [[f64; 2]; 4] {
    [[r[0], r[1]], [r[2], r[1]], [r[2], r[3]], [r[0], r[3]]]
}

/// A door's elevation lines in (u, z): frame, leaves, panels (with a bevel line), lites,
/// muntins, louvers and braces, and slide arrows.
pub fn elevation_lines(lay: &DoorLayout) -> Vec<WindowLine> {
    let f = lay.frame;
    let mut out = vec![];
    if f > 0.0 {
        out.push(wl(
            &[
                [f, 0.0],
                [f, lay.height - f],
                [lay.width - f, lay.height - f],
                [lay.width - f, 0.0],
            ],
            false,
            false,
            1,
            false,
        ));
    }
    for l in &lay.leaves {
        out.push(wl(&r4(l.rect), true, false, 1, false));
        for g in &l.glass {
            out.push(wl(&r4(*g), true, false, 1, true));
        }
        for p in &l.panels {
            out.push(wl(&r4(*p), true, false, 1, false));
            if !l.louvered && l.op != DoorOp::Overhead {
                let i = 1.25 * IN;
                if p[2] - p[0] > 3.0 * i && p[3] - p[1] > 3.0 * i {
                    out.push(wl(
                        &r4([p[0] + i, p[1] + i, p[2] - i, p[3] - i]),
                        true,
                        false,
                        1,
                        false,
                    ));
                }
            }
        }
        for bar in &l.bars {
            out.push(wl(
                &[[bar[0], bar[1]], [bar[2], bar[3]]],
                false,
                false,
                1,
                false,
            ));
        }
        if let DoorOp::Slide { to_right } = l.op {
            let s = if to_right { 1.0 } else { -1.0 };
            let (um, zm) = ((l.rect[0] + l.rect[2]) / 2.0, 40.0 * IN);
            let len = ((l.rect[2] - l.rect[0]) * 0.2).min(8.0 * IN);
            let (a, t) = (um - s * len, um + s * len);
            out.push(wl(&[[a, zm], [t, zm]], false, false, 1, false));
            let hd = len * 0.3;
            out.push(wl(
                &[
                    [t - s * hd, zm + hd * 0.6],
                    [t, zm],
                    [t - s * hd, zm - hd * 0.6],
                ],
                false,
                false,
                1,
                false,
            ));
        }
    }
    for m in &lay.mullions {
        out.push(wl(&r4(*m), true, false, 1, false));
    }
    out
}

/// Draws a door's elevation detail into its face `u0`..`u1`, `z0`..`z1`; `mirrored` when
/// the view looks at it with the wall's start on the right.
pub(crate) fn elevation_detail(
    b: &mut Builder,
    el: ElementId,
    style: DoorStyle,
    flip_hand: bool,
    (u0, u1, z0, z1): (f64, f64, f64, f64),
    mirrored: bool,
) {
    let lay = layout(style, u1 - u0, z1 - z0, !flip_hand);
    let map = |p: &[f64; 2]| {
        let u = if mirrored { u1 - p[0] } else { u0 + p[0] };
        Pt::new(u, z0 + p[1])
    };
    for line in elevation_lines(&lay) {
        let pts: Vec<Pt> = line.pts.iter().map(map).collect();
        if line.glass {
            b.fill(Some(el), vec![super::ring(&pts)], FillKind::Glass);
        }
        let dash = if line.dashed {
            Dash::Dashed
        } else {
            Dash::Solid
        };
        b.line(Some(el), &pts, line.closed, line.w, dash);
    }
}

pub fn preview(style: DoorStyle, width: f64, height: f64) -> WindowPreview {
    let lay = layout(style, width, height, true);
    let mut lines = vec![wl(&r4([0.0, 0.0, width, height]), true, false, 2, false)];
    lines.extend(elevation_lines(&lay));
    WindowPreview {
        width,
        height,
        lines,
    }
}

// ---------------------------------------------------------------------------------------
// 3D
// ---------------------------------------------------------------------------------------

struct Solid {
    x: Axes,
    z: f64,
}

impl Solid {
    /// A box `u0..u1` along the wall, `w0..w1` across it, `z0..z1` up from the sill.
    fn part(
        &self,
        out: &mut Vec<f32>,
        (u0, u1): (f64, f64),
        (w0, w1): (f64, f64),
        (z0, z1): (f64, f64),
    ) {
        if u1 - u0 < 0.1 || (w1 - w0).abs() < 0.1 || z1 - z0 < 0.1 {
            return;
        }
        let (w0, w1) = (w0.min(w1), w0.max(w1));
        let base = Poly::simple(vec![
            self.x.at(u0, w0),
            self.x.at(u1, w0),
            self.x.at(u1, w1),
            self.x.at(u0, w1),
        ]);
        out.extend(
            Prism {
                base,
                z0: self.z + z0,
                z1: self.z + z1,
            }
            .triangles(),
        );
    }
}

/// `rect` less `holes`, as rectangles (a grid of the holes' edges, merged along rows).
fn solid_cells(rect: [f64; 4], holes: &[[f64; 4]]) -> Vec<[f64; 4]> {
    let mut us = vec![rect[0], rect[2]];
    let mut zs = vec![rect[1], rect[3]];
    for h in holes {
        us.extend([h[0].clamp(rect[0], rect[2]), h[2].clamp(rect[0], rect[2])]);
        zs.extend([h[1].clamp(rect[1], rect[3]), h[3].clamp(rect[1], rect[3])]);
    }
    us.sort_by(f64::total_cmp);
    us.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    let mut out = vec![];
    for zw in zs.windows(2) {
        let mut run: Option<(f64, f64)> = None;
        for uw in us.windows(2) {
            let c = ((uw[0] + uw[1]) / 2.0, (zw[0] + zw[1]) / 2.0);
            let open = holes
                .iter()
                .any(|h| c.0 > h[0] && c.0 < h[2] && c.1 > h[1] && c.1 < h[3]);
            match (open, run) {
                (false, Some((a, _))) => run = Some((a, uw[1])),
                (false, None) => run = Some((uw[0], uw[1])),
                (true, Some((a, e))) => {
                    out.push([a, zw[0], e, zw[1]]);
                    run = None;
                }
                (true, None) => {}
            }
        }
        if let Some((a, e)) = run {
            out.push([a, zw[0], e, zw[1]]);
        }
    }
    out
}

fn leaf_parts(s: &Solid, p: &mut Parts, l: &Leaf, center: f64, raised: bool) {
    let t = if l.op == DoorOp::Overhead {
        2.0 * 25.4
    } else {
        LEAF
    };
    let (w0, w1) = (center - t / 2.0, center + t / 2.0);
    let mut holes: Vec<[f64; 4]> = l.glass.clone();
    if !raised {
        holes.extend(l.panels.iter().copied());
    }
    for c in solid_cells(l.rect, &holes) {
        s.part(&mut p.frame, (c[0], c[2]), (w0, w1), (c[1], c[3]));
    }
    if raised {
        // Garage panels stand proud of the outside face.
        for q in &l.panels {
            s.part(
                &mut p.frame,
                (q[0], q[2]),
                (w1, w1 + 0.3 * IN),
                (q[1], q[3]),
            );
        }
    } else if l.louvered {
        for bar in &l.bars {
            s.part(
                &mut p.frame,
                (bar[0], bar[2]),
                (center - t * 0.3, center + t * 0.3),
                (bar[1] - 0.5 * IN, bar[1] + 0.5 * IN),
            );
        }
    } else {
        // Recessed panels, thinner than the stiles and rails.
        for q in &l.panels {
            s.part(
                &mut p.frame,
                (q[0], q[2]),
                (center - t * 0.28, center + t * 0.28),
                (q[1], q[3]),
            );
        }
    }
    for g in &l.glass {
        s.part(
            &mut p.glass,
            (g[0], g[2]),
            (center - GLASS / 2.0, center + GLASS / 2.0),
            (g[1], g[3]),
        );
    }
    if !l.louvered {
        for bar in &l.bars {
            let d = (center - t * 0.35, center + t * 0.35);
            if (bar[0] - bar[2]).abs() < 0.01 {
                s.part(
                    &mut p.frame,
                    (bar[0] - BAR / 2.0, bar[0] + BAR / 2.0),
                    d,
                    (bar[1].min(bar[3]), bar[1].max(bar[3])),
                );
            } else if (bar[1] - bar[3]).abs() < 0.01 {
                s.part(
                    &mut p.frame,
                    (bar[0], bar[2]),
                    d,
                    (bar[1] - BAR / 2.0, bar[1] + BAR / 2.0),
                );
            } else if l.op != DoorOp::Overhead {
                // A diagonal brace, as steps on the outside face.
                let n = 16;
                let d = (center + t / 2.0, center + t / 2.0 + 0.5 * IN);
                for k in 0..n {
                    let f0 = k as f64 / n as f64;
                    let f1 = (k + 1) as f64 / n as f64;
                    let (ua, ub) = (
                        bar[0] + (bar[2] - bar[0]) * f0,
                        bar[0] + (bar[2] - bar[0]) * f1,
                    );
                    let (za, zb) = (
                        bar[1] + (bar[3] - bar[1]) * f0,
                        bar[1] + (bar[3] - bar[1]) * f1,
                    );
                    let hw = 1.75 * IN;
                    s.part(
                        &mut p.frame,
                        (ua.min(ub) - hw, ua.max(ub) + hw),
                        d,
                        (za.min(zb) - hw, za.max(zb) + hw),
                    );
                }
            }
        }
    }
}

pub fn parts(o: &OpeningSolid, style: DoorStyle) -> Parts {
    let lay = layout_of(o, style);
    let s = Solid {
        x: Axes::of(o),
        z: o.z0,
    };
    let h = o.half_thickness;
    let (w, ht, f) = (lay.width, lay.height, lay.frame);
    let mut p = Parts::default();
    // Frame: jambs and head through the wall, or a 4 1/2" glass-door frame and threshold.
    let depth = if lay.casing > 0.0 {
        (-h, h)
    } else {
        (-(2.25 * IN).min(h), (2.25 * IN).min(h))
    };
    if f > 0.0 {
        s.part(&mut p.frame, (0.0, f), depth, (0.0, ht));
        s.part(&mut p.frame, (w - f, w), depth, (0.0, ht));
        s.part(&mut p.frame, (f, w - f), depth, (ht - f, ht));
        if lay.casing == 0.0 {
            s.part(&mut p.frame, (f, w - f), depth, (0.0, 0.75 * IN));
        }
    }
    if lay.casing > 0.0 && style.family != DoorFamily::Barn {
        let c = lay.casing;
        let over = c - f;
        for face in [(h, h + CASING_DEPTH), (-h - CASING_DEPTH, -h)] {
            s.part(&mut p.frame, (-over, f), face, (0.0, ht - f + c));
            s.part(&mut p.frame, (w - f, w + over), face, (0.0, ht - f + c));
            s.part(&mut p.frame, (f, w - f), face, (ht - f, ht - f + c));
        }
    }
    for l in &lay.leaves {
        let center = match l.op {
            DoorOp::Barn { .. } => h + 0.5 * IN + LEAF / 2.0,
            DoorOp::Overhead => -h + LEAF,
            _ => l
                .track
                .clamp(-(h - LEAF / 2.0).max(0.0), (h - LEAF / 2.0).max(0.0)),
        };
        leaf_parts(&s, &mut p, l, center, l.op == DoorOp::Overhead);
        if let DoorOp::Barn { to_right } = l.op {
            // The track above, as long again as the leaf.
            let lw = l.rect[2] - l.rect[0];
            let (a, e) = if to_right {
                (l.rect[0], l.rect[2] + lw)
            } else {
                (l.rect[0] - lw, l.rect[2])
            };
            s.part(
                &mut p.frame,
                (a, e),
                (h, h + 1.0 * IN),
                (l.rect[3], l.rect[3] + 1.5 * IN),
            );
        }
    }
    for m in &lay.mullions {
        s.part(&mut p.frame, (m[0], m[2]), depth, (m[1], m[3]));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::doors::{DoorFinish, LeafStyle};
    use studio_regen::OpeningKind;

    fn opening(style: DoorStyle, width: f64, height: f64) -> OpeningSolid {
        OpeningSolid {
            id: ElementId::new(),
            host: ElementId::new(),
            kind: OpeningKind::Door(style),
            wall_start: Pt::new(0.0, 0.0),
            dir: Pt::new(1.0, 0.0),
            half_thickness: 3.0 * IN,
            t0: 1000.0,
            t1: 1000.0 + width,
            z0: 0.0,
            z1: height,
            flip_hand: false,
            flip_facing: false,
        }
    }

    fn bounds(tris: &[f32]) -> [f32; 6] {
        let mut b = [f32::MAX, f32::MAX, f32::MAX, f32::MIN, f32::MIN, f32::MIN];
        for v in tris.chunks(3) {
            for i in 0..3 {
                b[i] = b[i].min(v[i]);
                b[i + 3] = b[i + 3].max(v[i]);
            }
        }
        b
    }

    #[test]
    fn cells_leave_the_holes_open() {
        let c = solid_cells([0.0, 0.0, 10.0, 10.0], &[[2.0, 2.0, 8.0, 8.0]]);
        let area: f64 = c.iter().map(|r| (r[2] - r[0]) * (r[3] - r[1])).sum();
        assert!((area - 64.0).abs() < 1e-9, "{c:?}");
        assert_eq!(c.len(), 4, "bottom, two sides, top");
    }

    #[test]
    fn a_full_lite_door_has_glass_and_casings() {
        let s = DoorStyle::resolve(DoorFamily::SingleFlush, LeafStyle::FullLite, 0, None);
        let p = parts(&opening(s, 36.0 * IN, 80.0 * IN), s);
        let g = bounds(&p.glass);
        // Glass inside the jambs and 4 1/2" stiles: 36 - 3 - 9 = 24" wide.
        assert!(((g[3] - g[0]) as f64 - 24.0 * IN).abs() < 0.01, "{g:?}");
        let f = bounds(&p.frame);
        // Casings stand 3/4" off both 3" wall faces and run 2" past the jambs.
        assert!((f[4] as f64 - 3.75 * IN).abs() < 0.01 && (f[1] as f64 + 3.75 * IN).abs() < 0.01);
        assert!((f[0] as f64 - (1000.0 - 2.0 * IN)).abs() < 0.01, "{f:?}");
        let flush = DoorStyle::new(DoorFamily::SingleFlush);
        assert!(parts(&opening(flush, 36.0 * IN, 80.0 * IN), flush)
            .glass
            .is_empty());
    }

    #[test]
    fn barn_and_garage_doors_sit_on_the_wall_faces() {
        let barn = DoorStyle::resolve(DoorFamily::Barn, LeafStyle::BarnX, 1, None);
        let f = bounds(&parts(&opening(barn, 36.0 * IN, 84.0 * IN), barn).frame);
        // Outside the 3" face, and the track runs a leaf's width past the opening.
        assert!(f[1] as f64 >= -3.0 * IN - 0.01 && f[4] as f64 > 5.0 * IN);
        assert!(f[3] as f64 > 1000.0 + 70.0 * IN);
        let g = DoorStyle::new(DoorFamily::Garage);
        let gp = parts(&opening(g, 192.0 * IN, 84.0 * IN), g);
        assert!(gp.glass.is_empty() && !gp.frame.is_empty());
    }

    #[test]
    fn plan_symbols_by_family() {
        let count = |s: DoorStyle, w: f64| {
            let mut b = Builder::new(48.0);
            plan_symbol(&mut b, None, &opening(s, w, 80.0 * IN), s);
            let dashed = b
                .items
                .iter()
                .filter(|i| {
                    matches!(
                        i.prim,
                        super::super::Prim::Line {
                            dash: Dash::Dashed,
                            ..
                        }
                    )
                })
                .count();
            (b.items.len(), dashed)
        };
        // Leaf + arc; a pair twice.
        assert_eq!(
            count(DoorStyle::new(DoorFamily::SingleFlush), 36.0 * IN),
            (2, 0)
        );
        assert_eq!(
            count(DoorStyle::new(DoorFamily::DoubleFlush), 72.0 * IN),
            (4, 0)
        );
        // Entry: leaf, arc, two sidelites of two glass lines, two mullions.
        assert_eq!(
            count(DoorStyle::new(DoorFamily::Sidelites), 64.0 * IN),
            (2 + 4 + 2, 0)
        );
        // Sliders: two faces and two panels.
        assert_eq!(
            count(DoorStyle::new(DoorFamily::SlidingGlass), 72.0 * IN),
            (4, 0)
        );
        // Pocket: dashed pocket and the leaf; barn: leaf and dashed track.
        assert_eq!(count(DoorStyle::new(DoorFamily::Pocket), 30.0 * IN), (2, 1));
        assert_eq!(count(DoorStyle::new(DoorFamily::Barn), 36.0 * IN), (2, 1));
        // Bifold 4: two folded pairs; garage: closed line and dashed open door.
        assert_eq!(
            count(
                DoorStyle::resolve(DoorFamily::Bifold, LeafStyle::Flush, 4, None),
                48.0 * IN
            ),
            (2, 0)
        );
        assert_eq!(
            count(DoorStyle::new(DoorFamily::Garage), 108.0 * IN),
            (2, 1)
        );
    }

    #[test]
    fn elevations_fill_glass_and_bevel_panels() {
        let s = DoorStyle::resolve(
            DoorFamily::SingleFlush,
            LeafStyle::SixPanel,
            0,
            Some(DoorFinish::Walnut),
        );
        let lines = elevation_lines(&layout(s, 32.0 * IN, 80.0 * IN, true));
        // Frame, leaf, six panels each with a bevel line.
        assert_eq!(lines.len(), 2 + 12);
        let f = DoorStyle::resolve(DoorFamily::DoubleFlush, LeafStyle::FifteenLite, 0, None);
        let lines = elevation_lines(&layout(f, 60.0 * IN, 80.0 * IN, true));
        assert_eq!(lines.iter().filter(|l| l.glass).count(), 2);
    }
}
