//! Planting textures (ADR-064): each species' foliage atlas and bark, drawn here.
//! - **Foliage atlas.** RGBA, 2 × 2 cells. Cells 0 and 1 hold a leafy twig (leaves, needles
//!   or scale sprays), a palm frond or a fan. Cells 2 and 3 hold the same with flowers, a
//!   grass's plume, or a palm's dead frond. A card's base is at the bottom of its cell.
//!   Leaves are drawn with their own colour, midrib, veins, edge shading and paler
//!   undersides. The transparent pixels carry the nearest leaf's colour, so mipmaps don't
//!   darken the edges.
//! - **Bark.** A seamless colour and normal pair per bark kind (smooth, furrowed, plated,
//!   papery, mottled, fibrous, palm rings, green), tinted with the species' bark colour.

use studio_core::planting::{Bark, CrownForm, Foliage, PlantGroup, PlantSpec};

use crate::plants::Rng;

/// A straight-alpha RGBA image, rows top to bottom.
pub struct Rgba {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

/// Premultiplied linear-ish canvas (sRGB values, 0..1) with coverage-based anti-aliasing.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

type C3 = [f64; 3];

fn srgb(c: [u8; 3]) -> C3 {
    c.map(|x| f64::from(x) / 255.0)
}
fn mix3(a: C3, b: C3, t: f64) -> C3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
fn scale3(a: C3, k: f64) -> C3 {
    a.map(|x| (x * k).clamp(0.0, 1.0))
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Canvas {
            w,
            h,
            px: vec![[0.0; 4]; w * h],
        }
    }
    /// Paints `c` at pixel (x, y) with coverage `a` over what's there.
    fn over(&mut self, x: i64, y: i64, c: C3, a: f64) {
        if a <= 0.0 || x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return;
        }
        let p = &mut self.px[y as usize * self.w + x as usize];
        let a = a.min(1.0) as f32;
        for k in 0..3 {
            p[k] = c[k] as f32 * a + p[k] * (1.0 - a);
        }
        p[3] = a + p[3] * (1.0 - a);
    }
    /// Fills a shape over its bounding box: `f(x, y)` gives (coverage, colour).
    fn shape(
        &mut self,
        lo: [f64; 2],
        hi: [f64; 2],
        mut f: impl FnMut(f64, f64) -> Option<(f64, C3)>,
        shadow: bool,
    ) {
        let (x0, y0) = (lo[0].floor().max(0.0) as i64, lo[1].floor().max(0.0) as i64);
        let (x1, y1) = (
            hi[0].ceil().min(self.w as f64 - 1.0) as i64,
            hi[1].ceil().min(self.h as f64 - 1.0) as i64,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                if let Some((a, c)) = f(x as f64 + 0.5, y as f64 + 0.5) {
                    if shadow {
                        self.darken(x, y, a * 0.42);
                    } else {
                        self.over(x, y, c, a);
                    }
                }
            }
        }
    }
    /// Darkens what is at (x, y) by `k` (a shadow), leaving its coverage.
    fn darken(&mut self, x: i64, y: i64, k: f64) {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return;
        }
        let p = &mut self.px[y as usize * self.w + x as usize];
        let m = (1.0 - k.clamp(0.0, 1.0)) as f32;
        for v in p.iter_mut().take(3) {
            *v *= m;
        }
    }
    /// A tapered stroke from `a` (radius `ra`) to `b` (radius `rb`).
    fn stroke(&mut self, a: [f64; 2], b: [f64; 2], ra: f64, rb: f64, c: C3, shade: bool) {
        let r = ra.max(rb) + 1.5;
        let lo = [a[0].min(b[0]) - r, a[1].min(b[1]) - r];
        let hi = [a[0].max(b[0]) + r, a[1].max(b[1]) + r];
        let d = [b[0] - a[0], b[1] - a[1]];
        let l2 = (d[0] * d[0] + d[1] * d[1]).max(1e-9);
        self.shape(
            lo,
            hi,
            |x, y| {
                let t = (((x - a[0]) * d[0] + (y - a[1]) * d[1]) / l2).clamp(0.0, 1.0);
                let (cx, cy) = (a[0] + d[0] * t, a[1] + d[1] * t);
                let rr = ra + (rb - ra) * t;
                let dist = (x - cx).hypot(y - cy);
                let cov = (rr - dist + 0.5).clamp(0.0, 1.0);
                if cov <= 0.0 {
                    return None;
                }
                // Round: lighter along the middle.
                let k = if shade {
                    0.75 + 0.35 * (1.0 - (dist / rr.max(0.5)).min(1.0))
                } else {
                    1.0
                };
                Some((cov, scale3(c, k)))
            },
            false,
        );
    }
    /// Spreads colour into transparent pixels (for mipmaps) and returns straight RGBA.
    fn finish(mut self) -> Rgba {
        for _ in 0..6 {
            let prev = self.px.clone();
            for y in 0..self.h {
                for x in 0..self.w {
                    let i = y * self.w + x;
                    if prev[i][3] > 0.02 {
                        continue;
                    }
                    let mut acc = [0.0f32; 3];
                    let mut n = 0.0f32;
                    for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if nx < 0 || ny < 0 || nx >= self.w as i64 || ny >= self.h as i64 {
                            continue;
                        }
                        let q = prev[ny as usize * self.w + nx as usize];
                        if q[3] > 0.02 {
                            for (v, c) in acc.iter_mut().zip(q) {
                                *v += c / q[3];
                            }
                            n += 1.0;
                        }
                    }
                    if n > 0.0 {
                        let a = self.px[i][3];
                        // Stored premultiplied with a tiny alpha, so it counts as filled.
                        for (v, c) in self.px[i].iter_mut().zip(acc) {
                            *v = c / n * a.max(0.001);
                        }
                        self.px[i][3] = a.max(0.001);
                    }
                }
            }
        }
        let mut data = vec![0u8; self.w * self.h * 4];
        for (i, p) in self.px.iter().enumerate() {
            let a = p[3].max(1e-6);
            for k in 0..3 {
                data[i * 4 + k] = ((p[k] / a).clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            data[i * 4 + 3] = if p[3] < 0.002 {
                0
            } else {
                (p[3].clamp(0.0, 1.0) * 255.0).round() as u8
            };
        }
        Rgba {
            width: self.w,
            height: self.h,
            data,
        }
    }
}

/// A leaf's half-width at `s` along it (0 base, 1 tip), in leaf lengths.
fn half_width(f: Foliage, s: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let sn = (pi * s).sin().max(0.0);
    match f {
        Foliage::Ovate => 0.3 * sn.powf(0.85) * (1.15 - 0.4 * s),
        Foliage::Lanceolate => 0.12 * sn,
        Foliage::Heart => 0.38 * (pi * s.powf(0.8)).sin().max(0.0).powf(0.7) * (1.05 - 0.4 * s),
        Foliage::Rounded => 0.42 * (1.0 - (2.0 * s - 1.0).powi(2)).max(0.0).sqrt(),
        Foliage::Lobed => 0.27 * sn.powf(0.8) * (0.7 + 0.3 * (5.0 * pi * s).sin().abs()),
        Foliage::Small => 0.3 * sn.powf(0.8),
        Foliage::Blade => 0.045 * sn.powf(0.6),
        _ => 0.2 * sn,
    }
}

/// A leaf's outline at (s along, q across, in leaf lengths): how far outside it a point
/// is (negative inside), and its half-width there (for shading).
fn leaf_edge(f: Foliage, s: f64, q: f64, teeth: f64) -> Option<(f64, f64)> {
    if f == Foliage::Palmate {
        // A maple: five pointed lobes about the leaf's centre, the middle one longest.
        let (cs, cq) = (s - 0.42, q);
        let a = cq.atan2(cs);
        let rr = cs.hypot(cq);
        let k = (2.5 * a).cos().abs();
        let tip = k.powf(6.0);
        let lobe = 0.2 + 0.36 * tip + 0.06 * (a.cos() + 1.0);
        let serr = 0.012 * teeth * (a * 40.0).sin().abs();
        return Some((rr - lobe + serr, lobe));
    }
    if !(0.0..=1.0).contains(&s) {
        return None;
    }
    let hw = half_width(f, s);
    let serr = match f {
        Foliage::Ovate | Foliage::Heart | Foliage::Rounded | Foliage::Small => {
            0.01 * teeth * (s * 60.0).sin().abs()
        }
        _ => 0.0,
    };
    Some((q.abs() - hw + serr, hw))
}

/// One leaf from `base`, pointing `angle` (radians, 0 = up the cell), `len` px long, seen
/// `squash` edge-on (1 flat on, smaller tilted away), casting a soft shadow on what's
/// under it.
#[allow(clippy::too_many_arguments)]
fn leaf(
    cv: &mut Canvas,
    f: Foliage,
    base: [f64; 2],
    angle: f64,
    len: f64,
    c: C3,
    under: bool,
    rng: &mut Rng,
) {
    let squash = rng.range(0.55, 1.0);
    let (dx, dy) = (angle.sin(), -angle.cos());
    let (nx, ny) = (-dy, dx);
    let r = len * 0.7;
    let lo = [
        base[0] - r - 6.0 + dx.min(0.0) * len,
        base[1] - r - 6.0 + dy.min(0.0) * len,
    ];
    let hi = [
        base[0] + r + 6.0 + dx.max(0.0) * len,
        base[1] + r + 6.0 + dy.max(0.0) * len,
    ];
    let c = if under {
        mix3(c, [0.66, 0.72, 0.58], 0.3)
    } else {
        c
    };
    let curl = rng.sym() * 0.1;
    let veins = rng.range(7.0, 11.0);
    let teeth = if len > 40.0 { 1.0 } else { 0.0 };
    // The fold: one half of the blade catches more light.
    let fold = rng.sym() * 0.14;
    let local = |x: f64, y: f64| {
        let (px, py) = (x - base[0], y - base[1]);
        let s = (px * dx + py * dy) / len;
        let q0 = (px * nx + py * ny) / len / squash;
        (s, q0 - curl * s * s)
    };
    // Its shadow on the leaves behind: offset down-right and soft.
    let (ox, oy) = (len * 0.05 + 1.5, len * 0.07 + 2.0);
    cv.shape(
        lo,
        [hi[0] + ox, hi[1] + oy],
        |x, y| {
            let (s, q) = local(x - ox, y - oy);
            let (edge, _) = leaf_edge(f, s, q, 0.0)?;
            let soft = (-edge * len / (len * 0.06 + 2.0) + 0.5).clamp(0.0, 1.0);
            (soft > 0.0).then_some((soft, [0.0, 0.0, 0.0]))
        },
        true,
    );
    cv.shape(
        lo,
        hi,
        |x, y| {
            let (s, q) = local(x, y);
            let (edge, hw) = leaf_edge(f, s, q, teeth)?;
            let cov = (-edge * len + 0.5).clamp(0.0, 1.0);
            if cov <= 0.0 {
                return None;
            }
            let across = (q.abs() / hw.max(1e-6)).min(1.0);
            let mut k = 1.0 - 0.22 * across.powi(2) + 0.1 * s + fold * q.signum() * across;
            // The midrib, veins and the paler base.
            let rib = if f == Foliage::Palmate {
                // Veins radiating from the stalk to each lobe.
                let a = q.atan2(s - 0.05);
                ((2.5 * a).cos().abs() > 0.985) as i32 as f64
            } else {
                (q.abs() * len < 0.6 + len * 0.009) as i32 as f64
            };
            if rib > 0.0 {
                k += if under { 0.14 } else { 0.08 };
            } else if f != Foliage::Palmate {
                let v = (s * veins - q.abs() * 1.4).rem_euclid(1.0);
                if v < 0.07 {
                    k += 0.045;
                }
            }
            Some((cov, scale3(c, k)))
        },
        false,
    );
}

/// Five-petal blossoms round `at`.
fn blossom(cv: &mut Canvas, at: [f64; 2], r: f64, c: C3, rng: &mut Rng) {
    let rot = rng.range(0.0, std::f64::consts::TAU);
    let petals = 5;
    let lo = [at[0] - r - 2.0, at[1] - r - 2.0];
    let hi = [at[0] + r + 2.0, at[1] + r + 2.0];
    let tone = rng.range(0.9, 1.08);
    cv.shape(
        lo,
        hi,
        |x, y| {
            let (px, py) = (x - at[0], y - at[1]);
            let rr = px.hypot(py);
            let a = py.atan2(px) - rot;
            let lobe = r * (0.55 + 0.45 * (f64::from(petals) * a / 2.0).cos().abs().powf(0.5));
            let cov = (lobe - rr + 0.5).clamp(0.0, 1.0);
            if cov <= 0.0 {
                return None;
            }
            let centre = rr < r * 0.2;
            let col = if centre {
                [0.92, 0.8, 0.35]
            } else {
                scale3(c, tone * (0.85 + 0.2 * (rr / r)))
            };
            Some((cov, col))
        },
        false,
    );
}

fn cell_origin(size: usize, cell: u32) -> ([f64; 2], f64) {
    // UV cell (cx, cy) with v up; the image's rows run down, so cy = 0 is the lower half.
    let half = size as f64 / 2.0;
    let cx = f64::from(cell % 2);
    let cy = f64::from(cell / 2);
    ([cx * half, (2.0 - cy) * half], half)
}

/// Needles along a twig from `a` to `b`, both sides, pointing forward.
#[allow(clippy::too_many_arguments)]
fn needle_run(
    cv: &mut Canvas,
    a: [f64; 2],
    b: [f64; 2],
    nl: f64,
    long: bool,
    lc: C3,
    ac: C3,
    w: f64,
    rng: &mut Rng,
) {
    let len = (b[0] - a[0]).hypot(b[1] - a[1]);
    let ang = (b[0] - a[0]).atan2(-(b[1] - a[1]));
    let count = (len / if long { 2.5 } else { 1.0 }) as usize;
    for k in 0..count {
        let t = k as f64 / count.max(1) as f64;
        let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
        let spread = if long {
            rng.range(0.2, 0.9)
        } else {
            rng.range(0.6, 1.05)
        };
        let na = ang + side * spread;
        let l = nl * rng.range(0.75, 1.1) * (1.0 - 0.35 * t);
        let c = scale3(mix3(lc, ac, rng.f()), rng.range(0.72, 1.12));
        cv.stroke(
            p,
            [p[0] + na.sin() * l, p[1] - na.cos() * l],
            w * 0.0048,
            w * 0.002,
            c,
            false,
        );
    }
}

/// A leafy twig filling a cell, its base at the cell's bottom middle.
#[allow(clippy::too_many_arguments)]
fn twig(cv: &mut Canvas, spec: &PlantSpec, cell: u32, flowers: bool, size: usize, rng: &mut Rng) {
    let (o, w) = cell_origin(size, cell);
    let (leaf_c, alt_c) = spec.leaf_colors();
    let (lc, ac) = (srgb(leaf_c), srgb(alt_c));
    let bark = scale3(srgb(spec.bark), 0.8);
    let flower = spec.flowers_now().map(srgb).unwrap_or([1.0, 1.0, 1.0]);
    let f = spec.foliage;
    let base = [o[0] + w * 0.5, o[1] - 2.0];
    // The twig and its side shoots: a main stem up the cell and alternate branchlets.
    let top = [o[0] + w * rng.range(0.42, 0.58), o[1] - w * 0.9];
    let mut shoots: Vec<([f64; 2], [f64; 2])> = vec![(base, top)];
    let conifer = matches!(f, Foliage::Needle | Foliage::Scale);
    let n = if conifer { 8 } else { 5 };
    for k in 1..=n {
        let t = k as f64 / (n + 1) as f64;
        let p = [
            base[0] + (top[0] - base[0]) * t,
            base[1] + (top[1] - base[1]) * t,
        ];
        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
        let len = w * if conifer { 0.46 } else { 0.4 } * (1.0 - t * 0.5) * rng.range(0.8, 1.1);
        let a = side * rng.range(0.6, 1.1);
        shoots.push((p, [p[0] + a.sin() * len, p[1] - a.cos() * len]));
    }
    let stem_r = if conifer { w * 0.008 } else { w * 0.006 };
    for (a, b) in &shoots {
        cv.stroke(*a, *b, stem_r, stem_r * 0.4, bark, true);
    }
    // What grows on the shoots, drawn back to front.
    let long_needles = conifer
        && f == Foliage::Needle
        && !matches!(
            spec.form,
            CrownForm::Pyramidal | CrownForm::Conical | CrownForm::Columnar
        );
    for (a, b) in &shoots {
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let ang = (b[0] - a[0]).atan2(-(b[1] - a[1]));
        match f {
            Foliage::Needle => {
                let nl = if long_needles { w * 0.16 } else { w * 0.05 };
                needle_run(cv, *a, *b, nl, long_needles, lc, ac, w, rng);
                if !long_needles {
                    // Fir and spruce sprays: side branchlets all along, needled too.
                    let step = w * 0.06;
                    let count = (len / step) as usize;
                    for k in 1..count {
                        let t = k as f64 / count as f64;
                        let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                        let sa = ang + side * rng.range(0.7, 1.0);
                        let sl = w * 0.15 * (1.0 - 0.6 * t) * rng.range(0.8, 1.1);
                        let q = [p[0] + sa.sin() * sl, p[1] - sa.cos() * sl];
                        needle_run(cv, p, q, nl * 0.85, false, lc, ac, w, rng);
                    }
                } else {
                    // Pine tufts at the tips.
                    for _ in 0..40 {
                        let na = ang + rng.sym() * 1.3;
                        let l = nl * rng.range(0.7, 1.05);
                        let c = scale3(mix3(lc, ac, rng.f()), rng.range(0.85, 1.15));
                        cv.stroke(
                            *b,
                            [b[0] + na.sin() * l, b[1] - na.cos() * l],
                            w * 0.0035,
                            w * 0.0015,
                            c,
                            false,
                        );
                    }
                }
            }
            Foliage::Scale => {
                // Flat sprays: branchlets of overlapping scales.
                let count = (len / (w * 0.05)) as usize + 2;
                for k in 0..count {
                    let t = (k as f64 + 0.5) / count as f64;
                    let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                    for side in [-1.0, 1.0] {
                        let na = ang + side * rng.range(0.5, 0.9);
                        let l = w * 0.09 * (1.0 - 0.4 * t) * rng.range(0.8, 1.1);
                        let q = [p[0] + na.sin() * l, p[1] - na.cos() * l];
                        let steps = 6;
                        for s in 0..steps {
                            let u = s as f64 / steps as f64;
                            let u2 = (s + 1) as f64 / steps as f64;
                            let c =
                                scale3(mix3(lc, ac, rng.f() * 0.5), 0.85 + 0.25 * ((s % 2) as f64));
                            let r = w * 0.014 * (1.0 - u * 0.6);
                            cv.stroke(
                                [p[0] + (q[0] - p[0]) * u, p[1] + (q[1] - p[1]) * u],
                                [p[0] + (q[0] - p[0]) * u2, p[1] + (q[1] - p[1]) * u2],
                                r,
                                r * 0.8,
                                c,
                                true,
                            );
                        }
                    }
                }
            }
            Foliage::Compound => {
                // Pinnate leaves: leaflets in pairs along a rachis.
                let count = (len / (w * 0.14)) as usize + 1;
                for k in 0..count {
                    let t = (k as f64 + 0.6) / (count as f64 + 0.5);
                    let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                    let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                    let la = ang + side * rng.range(0.7, 1.1);
                    let rl = w * rng.range(0.22, 0.3);
                    let tip = [p[0] + la.sin() * rl, p[1] - la.cos() * rl];
                    cv.stroke(p, tip, w * 0.0025, w * 0.0015, scale3(lc, 0.7), false);
                    let pairs = 5;
                    for j in 0..pairs {
                        let u = (j as f64 + 0.5) / pairs as f64;
                        let q = [p[0] + (tip[0] - p[0]) * u, p[1] + (tip[1] - p[1]) * u];
                        for s2 in [-1.0, 1.0] {
                            let c = scale3(mix3(lc, ac, rng.f()), rng.range(0.82, 1.1));
                            leaf(
                                cv,
                                Foliage::Lanceolate,
                                q,
                                la + s2 * 0.9,
                                w * 0.07,
                                c,
                                rng.f() < 0.2,
                                rng,
                            );
                        }
                    }
                    let c = scale3(mix3(lc, ac, rng.f()), rng.range(0.82, 1.1));
                    leaf(cv, Foliage::Lanceolate, tip, la, w * 0.07, c, false, rng);
                }
            }
            _ => {
                let leaf_len = match f {
                    Foliage::Small => w * 0.08,
                    Foliage::Lanceolate => w * 0.2,
                    Foliage::Palmate | Foliage::Lobed => w * 0.22,
                    Foliage::Heart | Foliage::Rounded => w * 0.16,
                    _ => w * 0.17,
                };
                // Small leaves crowd their twigs (boxwood, privet, holly).
                let spacing = if f == Foliage::Small { 0.28 } else { 0.45 };
                let count = (len / (leaf_len * spacing)).ceil() as usize + 1;
                for k in 0..count {
                    let t = (k as f64 + 0.5) / count as f64;
                    let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                    let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                    let la = ang + side * rng.range(0.5, 1.2);
                    // A short petiole.
                    let pl = leaf_len * 0.12;
                    let q = [p[0] + la.sin() * pl, p[1] - la.cos() * pl];
                    cv.stroke(p, q, w * 0.0025, w * 0.002, scale3(lc, 0.75), false);
                    let c = scale3(mix3(lc, ac, rng.f()), rng.range(0.8, 1.12));
                    let l = leaf_len * rng.range(0.8, 1.15) * (1.0 - 0.25 * t);
                    leaf(cv, f, q, la, l, c, rng.f() < 0.22, rng);
                }
                // A terminal leaf.
                let c = scale3(mix3(lc, ac, rng.f()), rng.range(0.85, 1.1));
                leaf(cv, f, *b, ang, leaf_len, c, false, rng);
            }
        }
    }
    if flowers {
        // Blossoms along the shoots and in clusters at the tips.
        let r = (w * 0.028).max(3.0);
        for (a, b) in &shoots {
            let count = 10;
            for k in 0..count {
                let t = rng.f();
                let p = [
                    a[0] + (b[0] - a[0]) * t + rng.sym() * w * 0.05,
                    a[1] + (b[1] - a[1]) * t + rng.sym() * w * 0.05,
                ];
                blossom(cv, p, r * rng.range(0.8, 1.2), flower, rng);
                let _ = k;
            }
            for _ in 0..6 {
                let p = [b[0] + rng.sym() * w * 0.06, b[1] + rng.sym() * w * 0.06];
                blossom(cv, p, r * rng.range(0.8, 1.2), flower, rng);
            }
        }
    }
}

/// A feather frond (palms, sago, ferns) up a cell: a rachis with leaflets both sides.
fn frond(cv: &mut Canvas, cell: u32, c: C3, alt: C3, dead: bool, size: usize, rng: &mut Rng) {
    let (o, w) = cell_origin(size, cell);
    let x = o[0] + w * 0.5;
    let (y0, y1) = (o[1] - 1.0, o[1] - w + 1.0);
    let rachis = if dead {
        [0.45, 0.36, 0.24]
    } else {
        scale3(c, 0.8)
    };
    cv.stroke([x, y0], [x, y1], w * 0.012, w * 0.003, rachis, true);
    let count = 40;
    for k in 0..count {
        let t = (k as f64 + 0.5) / count as f64;
        let y = y0 + (y1 - y0) * t;
        let reach = w * 0.47 * (std::f64::consts::PI * (0.06 + 0.94 * t)).sin().powf(0.55);
        for side in [-1.0, 1.0] {
            if dead && rng.f() < 0.35 {
                continue;
            }
            let droop = rng.range(0.1, 0.35);
            let tip = [x + side * reach, y - reach * (0.55 - droop)];
            let col = if dead {
                scale3([0.58, 0.46, 0.3], rng.range(0.7, 1.05))
            } else {
                scale3(mix3(c, alt, rng.f()), rng.range(0.82, 1.12))
            };
            let ang = (tip[0] - x).atan2(-(tip[1] - y));
            let len = (tip[0] - x).hypot(tip[1] - y);
            leaf(cv, Foliage::Blade, [x, y], ang, len, col, false, rng);
        }
    }
}

/// A palmate fan leaf filling a cell about its centre.
fn fan(cv: &mut Canvas, cell: u32, c: C3, alt: C3, size: usize, rng: &mut Rng) {
    let (o, w) = cell_origin(size, cell);
    let centre = [o[0] + w * 0.5, o[1] - w * 0.5];
    let fingers = 64;
    for k in 0..fingers {
        let a = (k as f64 / fingers as f64) * std::f64::consts::TAU;
        let len = w * 0.47 * rng.range(0.9, 1.0);
        let col = scale3(mix3(c, alt, rng.f()), if k % 2 == 0 { 0.9 } else { 1.05 });
        let start = [
            centre[0] + a.sin() * w * 0.05,
            centre[1] - a.cos() * w * 0.05,
        ];
        leaf(cv, Foliage::Blade, start, a, len, col, false, rng);
    }
    cv.stroke(centre, centre, w * 0.05, w * 0.05, scale3(c, 0.85), true);
}

/// A grass plume (feathery seed head) up a cell.
fn plume(cv: &mut Canvas, cell: u32, c: C3, size: usize, rng: &mut Rng) {
    let (o, w) = cell_origin(size, cell);
    let x = o[0] + w * 0.5;
    let (y0, y1) = (o[1] - 1.0, o[1] - w + 2.0);
    cv.stroke(
        [x, y0],
        [x, y1],
        w * 0.006,
        w * 0.003,
        scale3(c, 0.7),
        false,
    );
    for _ in 0..900 {
        let t = rng.range(0.35, 1.0);
        let y = y0 + (y1 - y0) * t;
        let reach = w * 0.13 * (std::f64::consts::PI * (t - 0.35) / 0.65).sin().max(0.1);
        let a = rng.sym() * 1.2;
        let l = reach * rng.range(0.4, 1.0);
        let col = scale3(c, rng.range(0.8, 1.15));
        cv.stroke(
            [x, y],
            [x + a.sin() * l, y - a.cos().abs() * l * 0.6],
            w * 0.003,
            w * 0.0012,
            col,
            false,
        );
    }
}

/// The species' foliage atlas, `size` px square.
pub fn atlas(spec: &PlantSpec, size: usize) -> Rgba {
    let mut rng = Rng::new(crate::plants::seed_of(spec, 9));
    let mut cv = Canvas::new(size, size);
    let (leaf_c, alt_c) = spec.leaf_colors();
    let (lc, ac) = (srgb(leaf_c), srgb(alt_c));
    let flowers = spec.flowers_now().is_some();
    match (spec.form, spec.foliage) {
        (_, Foliage::Frond) => {
            frond(&mut cv, 0, lc, ac, false, size, &mut rng);
            frond(&mut cv, 1, lc, ac, false, size, &mut rng);
            frond(&mut cv, 2, lc, ac, false, size, &mut rng);
            frond(&mut cv, 3, lc, ac, true, size, &mut rng);
        }
        (_, Foliage::Fan) => {
            fan(&mut cv, 0, lc, ac, size, &mut rng);
            fan(&mut cv, 1, lc, ac, size, &mut rng);
            fan(&mut cv, 2, lc, ac, size, &mut rng);
            frond(&mut cv, 3, lc, ac, true, size, &mut rng);
        }
        (_, Foliage::Blade) => {
            let f = spec.flowers_now().map(srgb).unwrap_or(lc);
            // Blade foliage is solid geometry; the atlas holds its plumes (or, for a shrub
            // with strap leaves, lanceolate twigs).
            twig(
                &mut cv,
                &PlantSpec {
                    foliage: Foliage::Lanceolate,
                    ..spec.clone()
                },
                0,
                false,
                size,
                &mut rng,
            );
            twig(
                &mut cv,
                &PlantSpec {
                    foliage: Foliage::Lanceolate,
                    ..spec.clone()
                },
                1,
                false,
                size,
                &mut rng,
            );
            if spec.group == PlantGroup::Grass {
                plume(&mut cv, 2, f, size, &mut rng);
                plume(&mut cv, 3, f, size, &mut rng);
            } else {
                twig(
                    &mut cv,
                    &PlantSpec {
                        foliage: Foliage::Lanceolate,
                        ..spec.clone()
                    },
                    2,
                    flowers,
                    size,
                    &mut rng,
                );
                twig(
                    &mut cv,
                    &PlantSpec {
                        foliage: Foliage::Lanceolate,
                        ..spec.clone()
                    },
                    3,
                    flowers,
                    size,
                    &mut rng,
                );
            }
        }
        _ => {
            for cell in 0..4 {
                twig(&mut cv, spec, cell, flowers && cell >= 2, size, &mut rng);
            }
        }
    }
    cv.finish()
}

// ---------------------------------------------------------------- bark

fn hash(x: i64, y: i64, seed: u64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B1)
        ^ (y as u64).wrapping_mul(0x85EB_CA77)
        ^ seed.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFFFF_FFFF) as f64 / 4_294_967_295.0
}

/// Periodic value noise: lattice `nx` × `ny` over the unit square.
fn vnoise(u: f64, v: f64, nx: i64, ny: i64, seed: u64) -> f64 {
    let (x, y) = (u * nx as f64, v * ny as f64);
    let (i, j) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = (x - i as f64, y - j as f64);
    let s = |t: f64| t * t * (3.0 - 2.0 * t);
    let g = |a: i64, b: i64| hash(a.rem_euclid(nx), b.rem_euclid(ny), seed);
    let (a, b, c, d) = (g(i, j), g(i + 1, j), g(i, j + 1), g(i + 1, j + 1));
    let (sx, sy) = (s(fx), s(fy));
    (a * (1.0 - sx) + b * sx) * (1.0 - sy) + (c * (1.0 - sx) + d * sx) * sy
}

fn fbm(u: f64, v: f64, nx: i64, ny: i64, oct: u32, seed: u64) -> f64 {
    let (mut sum, mut amp, mut norm) = (0.0, 0.5, 0.0);
    for o in 0..oct {
        let k = 1 << o;
        sum += amp * vnoise(u, v, nx * k, ny * k, seed + u64::from(o));
        norm += amp;
        amp *= 0.5;
    }
    sum / norm
}

/// Periodic cellular noise: distance to the nearest of `n` × `n` jittered points, and the
/// second nearest.
fn cells(u: f64, v: f64, nx: i64, ny: i64, seed: u64) -> (f64, f64, f64) {
    let (x, y) = (u * nx as f64, v * ny as f64);
    let (i, j) = (x.floor() as i64, y.floor() as i64);
    let (mut d1, mut d2, mut id) = (9.0f64, 9.0f64, 0.0);
    for dj in -1..=1 {
        for di in -1..=1 {
            let (ci, cj) = (i + di, j + dj);
            let (wi, wj) = (ci.rem_euclid(nx), cj.rem_euclid(ny));
            let px = ci as f64 + hash(wi, wj, seed);
            let py = cj as f64 + hash(wi, wj, seed + 7);
            let d = (x - px).hypot(y - py);
            if d < d1 {
                d2 = d1;
                d1 = d;
                id = hash(wi, wj, seed + 13);
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (d1, d2, id)
}

/// A bark kind's colour and height at (u round, v along), both 0..1 over one tile.
fn bark_at(kind: Bark, base: C3, u: f64, v: f64) -> (C3, f64) {
    match kind {
        Bark::Smooth | Bark::Green => {
            let n = fbm(u, v, 4, 4, 5, 11);
            // Horizontal lenticels.
            let len = vnoise(u, v, 24, 90, 12);
            let dash = if len > 0.82 { 0.75 } else { 1.0 };
            (
                scale3(base, (0.85 + 0.3 * n) * dash),
                n * 0.6 + (1.0 - dash) * -0.5,
            )
        }
        Bark::Furrowed => {
            // Vertical ridges broken into blocks.
            let ridge = fbm(u, v, 10, 1, 4, 21);
            let warp = fbm(u, v, 3, 6, 3, 22) * 0.2;
            let r = ((u + warp) * 12.0 * std::f64::consts::TAU).sin() * 0.5 + 0.5;
            let h = (r * 0.7 + ridge * 0.6).powf(1.4);
            let crack = (1.0 - h).powi(3);
            (
                scale3(base, 0.45 + 0.75 * h + 0.1 * vnoise(u, v, 60, 60, 23)),
                h * 3.0 - crack,
            )
        }
        Bark::Plated => {
            let (d1, d2, id) = cells(u, v, 6, 8, 31);
            let edge = ((d2 - d1) * 3.0).clamp(0.0, 1.0);
            let plate = 0.75 + 0.35 * id + 0.15 * fbm(u, v, 12, 12, 3, 32);
            (scale3(base, plate * (0.35 + 0.65 * edge)), edge * 2.5)
        }
        Bark::Papery => {
            let n = fbm(u, v, 5, 5, 4, 41);
            // Dark horizontal lenticel dashes and peeling curls.
            let dash = vnoise(u, v, 16, 70, 42);
            let peel = vnoise(u, v, 6, 20, 43);
            let dark = if dash > 0.8 { 0.35 } else { 1.0 };
            let curl = if peel > 0.78 { 0.8 } else { 1.0 };
            (
                scale3(base, (0.88 + 0.14 * n) * dark * curl),
                n * 0.3 - (1.0 - dark) * 0.8,
            )
        }
        Bark::Mottled => {
            let (_, _, id) = cells(u, v, 5, 7, 51);
            let (_, _, id2) = cells(u + 0.13, v + 0.3, 7, 5, 52);
            let patch = mix3(base, mix3(base, [0.9, 0.88, 0.8], 0.5), id);
            let patch = mix3(patch, scale3(base, 0.7), (id2 - 0.5).max(0.0));
            let n = fbm(u, v, 8, 8, 3, 53);
            (scale3(patch, 0.9 + 0.2 * n), n * 0.5 + id * 0.3)
        }
        Bark::Fibrous => {
            let f = fbm(u, v, 40, 1, 4, 61);
            let w = fbm(u, v, 4, 4, 2, 62);
            let h = (f * 0.8 + w * 0.3).powf(1.2);
            (scale3(base, 0.55 + 0.6 * h), h * 2.0)
        }
        Bark::Palm => {
            // Horizontal leaf-scar rings with a diamond weave.
            let ring = ((v * 10.0).rem_euclid(1.0) - 0.5).abs() * 2.0;
            let weave = (((u * 8.0 + v * 10.0).rem_euclid(1.0) - 0.5).abs()
                + ((u * 8.0 - v * 10.0).rem_euclid(1.0) - 0.5).abs())
                * 0.5;
            let n = fbm(u, v, 8, 8, 3, 71);
            let h = (1.0 - ring).powf(3.0) * 0.6 + weave * 0.4 + n * 0.2;
            (scale3(base, 0.55 + 0.5 * h + 0.1 * n), h * 2.0)
        }
    }
}

/// A bark texture: `size` px square, one tile of [`crate::plants::BARK_TILE`] mm; its
/// colour and normal map (RGB, rows top to bottom).
pub fn bark(kind: Bark, color: [u8; 3], size: usize) -> (Vec<u8>, Vec<u8>) {
    let base = srgb(color);
    let mut col = vec![0u8; size * size * 3];
    let mut hgt = vec![0f64; size * size];
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (
                (x as f64 + 0.5) / size as f64,
                1.0 - (y as f64 + 0.5) / size as f64,
            );
            let (c, h) = bark_at(kind, base, u, v);
            let i = y * size + x;
            for k in 0..3 {
                col[i * 3 + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            hgt[i] = h;
        }
    }
    let mut nor = vec![0u8; size * size * 3];
    let strength = size as f64 / 64.0;
    for y in 0..size {
        for x in 0..size {
            let at = |xx: usize, yy: usize| hgt[yy * size + xx];
            let (l, r) = ((x + size - 1) % size, (x + 1) % size);
            let (u, d) = ((y + size - 1) % size, (y + 1) % size);
            let dx = (at(r, y) - at(l, y)) * strength / 2.0;
            let dy = (at(x, u) - at(x, d)) * strength / 2.0;
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let i = (y * size + x) * 3;
            nor[i] = ((-dx / len * 0.5 + 0.5) * 255.0).round() as u8;
            nor[i + 1] = ((-dy / len * 0.5 + 0.5) * 255.0).round() as u8;
            nor[i + 2] = ((1.0 / len * 0.5 + 0.5) * 255.0).round() as u8;
        }
    }
    (col, nor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::planting::catalog;

    fn spec(name: &str) -> PlantSpec {
        catalog().into_iter().find(|p| p.name == name).unwrap().spec
    }

    fn coverage(img: &Rgba, cell: u32) -> f64 {
        let half = img.width / 2;
        let (cx, cy) = ((cell % 2) as usize, (cell / 2) as usize);
        let y0 = if cy == 0 { half } else { 0 };
        let mut n = 0;
        for y in y0..y0 + half {
            for x in cx * half..cx * half + half {
                if img.data[(y * img.width + x) * 4 + 3] > 128 {
                    n += 1;
                }
            }
        }
        n as f64 / (half * half) as f64
    }

    #[test]
    fn an_atlas_has_leaves_in_every_cell_with_the_species_colours() {
        let img = atlas(&spec("Red Maple"), 256);
        assert_eq!(img.data.len(), 256 * 256 * 4);
        for cell in 0..4 {
            let c = coverage(&img, cell);
            assert!(c > 0.08 && c < 0.9, "cell {cell}: {c}");
        }
        // Opaque pixels are green; autumn's are red.
        let avg = |img: &Rgba| {
            let mut s = [0.0; 3];
            let mut n = 0.0;
            for p in img.data.chunks(4).filter(|p| p[3] > 200) {
                for k in 0..3 {
                    s[k] += f64::from(p[k]);
                }
                n += 1.0;
            }
            s.map(|x| x / n)
        };
        let g = avg(&img);
        assert!(g[1] > g[0] && g[1] > g[2], "{g:?}");
        let r = avg(&atlas(&spec("Red Maple (Autumn)"), 256));
        assert!(r[0] > r[1] * 1.3, "{r:?}");
        // Transparent pixels carry leaf colour (no black fringes in mipmaps).
        let dark = img
            .data
            .chunks(4)
            .filter(|p| p[3] == 0 && p[0] < 5 && p[1] < 5 && p[2] < 5)
            .count();
        assert!(dark < img.data.len() / 4, "{dark}");
        // Blossoms: a cherry in bloom is mostly pale pink in its flower cells.
        let cherry = atlas(&spec("Yoshino Cherry"), 256);
        let pink = cherry
            .data
            .chunks(4)
            .filter(|p| p[3] > 200 && p[0] > 200 && p[2] > 170)
            .count();
        assert!(pink > 500, "{pink}");
    }

    #[test]
    fn palms_conifers_and_grasses_draw_their_own_foliage() {
        for name in [
            "Queen Palm",
            "Mexican Fan Palm",
            "Norway Spruce",
            "Italian Cypress",
            "Fountain Grass",
            "White Oak",
            "Honey Locust",
        ] {
            let img = atlas(&spec(name), 256);
            let c = coverage(&img, 0);
            assert!(c > 0.05, "{name}: {c}");
        }
    }

    #[test]
    fn bark_tiles_seamlessly() {
        for kind in [
            Bark::Smooth,
            Bark::Furrowed,
            Bark::Plated,
            Bark::Papery,
            Bark::Mottled,
            Bark::Fibrous,
            Bark::Palm,
            Bark::Green,
        ] {
            for k in 0..50 {
                let (u, v) = (hash(k, 1, 5), hash(k, 2, 5));
                let (a, ha) = bark_at(kind, [0.4, 0.35, 0.3], u, v);
                for (bu, bv) in [(u + 1.0, v), (u, v + 1.0)] {
                    let (b, hb) = bark_at(kind, [0.4, 0.35, 0.3], bu, bv);
                    let d = (0..3).map(|i| (a[i] - b[i]).abs()).sum::<f64>() + (ha - hb).abs();
                    assert!(d < 1e-6, "{kind:?} at ({u}, {v}): {d}");
                }
            }
            let (c, n) = bark(kind, [120, 100, 80], 32);
            assert_eq!(c.len(), 32 * 32 * 3);
            assert_eq!(n.len(), c.len());
        }
    }
}
