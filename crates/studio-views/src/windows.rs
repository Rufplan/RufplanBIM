//! Window graphics by family (ADR-031): plan symbols, elevation detail, the 3D frame,
//! sashes, muntins and glass, and the Window Library's thumbnails. The layouts themselves
//! come from `studio_core::windows`.

use serde::Serialize;
use studio_core::windows::{layout, Operation, WindowLayout, WindowStyle, BAR};
use studio_core::ElementId;
use studio_geom::{Poly, Prism, Pt};
use studio_regen::OpeningSolid;
use ts_rs::TS;

use super::{Builder, Dash};

/// Sash depth through the frame.
const SASH_DEPTH: f64 = 1.375 * 25.4;
/// Glass (insulating unit) thickness as modelled.
const GLASS: f64 = 0.25 * 25.4;
/// Muntin depth, straddling the glass.
const MUNTIN_DEPTH: f64 = 0.625 * 25.4;
/// Bay seat and head board thickness.
const BOARD: f64 = 1.5 * 25.4;

/// A window's own axes in plan. Seen from outside, `origin` is its left jamb on the wall's
/// centre line, `ax` runs left to right and `out` points outside (its facing side).
#[derive(Debug, Clone, Copy)]
pub struct Axes {
    pub origin: Pt,
    pub ax: Pt,
    pub out: Pt,
}

impl Axes {
    pub fn of(o: &OpeningSolid) -> Self {
        let n = o.dir.perp();
        if o.flip_facing {
            Axes {
                origin: o.at(o.t0),
                ax: o.dir,
                out: n.scale(-1.0),
            }
        } else {
            Axes {
                origin: o.at(o.t1),
                ax: o.dir.scale(-1.0),
                out: n,
            }
        }
    }
    /// The plan point `u` across the window and `w` outward from the centre line.
    pub fn at(&self, u: f64, w: f64) -> Pt {
        self.origin.add(self.ax.scale(u)).add(self.out.scale(w))
    }
}

/// The layout of an opening's window.
pub fn layout_of(o: &OpeningSolid, style: WindowStyle) -> WindowLayout {
    layout(style, o.width(), o.z1 - o.z0)
}

// ---------------------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------------------

/// Plan symbol of a window: the wall faces across the opening, each sash's glass in its
/// track, mullions, casement swings, and a bay's projecting outline.
pub(crate) fn plan_symbol(
    b: &mut Builder,
    el: Option<ElementId>,
    o: &OpeningSolid,
    s: WindowStyle,
) {
    let lay = layout_of(o, s);
    let x = Axes::of(o);
    let h = o.half_thickness;
    let w = lay.width;
    let g = h * 0.22;
    let seg = |b: &mut Builder, u0: f64, u1: f64, off: f64, weight: u8| {
        b.line(
            el,
            &[x.at(u0, off), x.at(u1, off)],
            false,
            weight,
            Dash::Solid,
        );
    };
    if lay.projection > 0.0 {
        // The interior face across the opening, then the bay's frame and glass lines.
        seg(b, 0.0, w, -h, 1);
        let mut us = vec![0.0];
        us.extend(lay.corners());
        us.push(w);
        for (off, weight) in [(lay.depth / 2.0, 1), (0.0, 2), (-lay.depth / 2.0, 1)] {
            let pts: Vec<Pt> = us
                .iter()
                .map(|u| x.at(*u, lay.bay_offset(*u, h) + off))
                .collect();
            b.line(el, &pts, false, weight, Dash::Solid);
        }
        return;
    }
    for off in [h, -h] {
        seg(b, 0.0, w, off, 1);
    }
    let edge = |u: f64| {
        if u <= lay.frame + 1.0 {
            0.0
        } else if u >= w - lay.frame - 1.0 {
            w
        } else {
            u
        }
    };
    // Lites stacked in one track show once (a picture over an awning); hung sashes in
    // their two tracks show both.
    let mut drawn: Vec<(f64, f64, f64)> = vec![];
    for l in &lay.lites {
        let (u0, u1) = (edge(l.rect[0]), edge(l.rect[2]));
        let key = (u0, u1, l.track);
        if drawn.iter().any(|d| {
            (d.0 - key.0).abs() < 1.0 && (d.1 - key.1).abs() < 1.0 && (d.2 - key.2).abs() < 0.1
        }) {
            continue;
        }
        drawn.push(key);
        if l.track == 0.0 {
            for off in [g, -g] {
                seg(b, u0, u1, off, 2);
            }
        } else {
            let c = g * l.track.signum();
            for off in [c + g / 2.0, c - g / 2.0] {
                seg(b, u0, u1, off, 2);
            }
        }
    }
    for m in &lay.mullions {
        if m[3] - m[1] > lay.height / 2.0 {
            b.line(
                el,
                &[
                    x.at(m[0], -g * 1.5),
                    x.at(m[2], -g * 1.5),
                    x.at(m[2], g * 1.5),
                    x.at(m[0], g * 1.5),
                ],
                true,
                1,
                Dash::Solid,
            );
        }
    }
    // Casements swing out: the sash drawn open 30° from its hinge.
    for l in &lay.lites {
        if let Operation::Casement { hinge_left } = l.operation {
            let (hinge, toward) = if hinge_left {
                (edge(l.rect[0]), 1.0)
            } else {
                (edge(l.rect[2]), -1.0)
            };
            let len = l.rect[2] - l.rect[0];
            let from = x.at(hinge, h);
            let tip = from
                .add(x.ax.scale(toward * len * 0.866))
                .add(x.out.scale(len * 0.5));
            b.line(el, &[from, tip], false, 1, Dash::Dashed);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Elevation
// ---------------------------------------------------------------------------------------

/// A line of a window's elevation, in (u, z) from its bottom-left seen from outside.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct WindowLine {
    pub pts: Vec<[f64; 2]>,
    pub closed: bool,
    pub dashed: bool,
    /// Pen weight, 1 finest.
    pub w: u8,
    /// The outline of a pane of glass (thumbnails fill it).
    pub glass: bool,
}

fn wl(pts: &[[f64; 2]], closed: bool, dashed: bool, w: u8) -> WindowLine {
    WindowLine {
        pts: pts.to_vec(),
        closed,
        dashed,
        w,
        glass: false,
    }
}

fn rect(r: [f64; 4]) -> [[f64; 2]; 4] {
    [[r[0], r[1]], [r[2], r[1]], [r[2], r[3]], [r[0], r[3]]]
}

/// The frame opening, sashes, glass, grilles and operation marks of a window seen from
/// outside: casement and awning marks point to the hinge, a slider's arrow the way it opens.
pub fn elevation_lines(lay: &WindowLayout) -> Vec<WindowLine> {
    let f = lay.frame;
    let mut out = vec![wl(
        &rect([f, f, lay.width - f, lay.height - f]),
        true,
        false,
        1,
    )];
    for l in &lay.lites {
        let [u0, z0, u1, z1] = l.rect;
        if l.rail > 0.0 {
            out.push(wl(&rect(l.rect), true, false, 1));
        }
        let gl = l.glass();
        out.push(WindowLine {
            glass: true,
            ..wl(&rect(gl), true, false, 1)
        });
        for bar in &l.bars {
            out.push(wl(&[[bar[0], bar[1]], [bar[2], bar[3]]], false, false, 1));
        }
        let (um, zm) = ((u0 + u1) / 2.0, (z0 + z1) / 2.0);
        match l.operation {
            Operation::Casement { hinge_left } => {
                let (hinge, free) = if hinge_left {
                    (gl[0], gl[2])
                } else {
                    (gl[2], gl[0])
                };
                out.push(wl(
                    &[[free, gl[1]], [hinge, zm], [free, gl[3]]],
                    false,
                    true,
                    1,
                ));
            }
            Operation::Awning => {
                out.push(wl(
                    &[[gl[0], gl[1]], [um, gl[3]], [gl[2], gl[1]]],
                    false,
                    true,
                    1,
                ));
            }
            Operation::Hopper => {
                out.push(wl(
                    &[[gl[0], gl[3]], [um, gl[1]], [gl[2], gl[3]]],
                    false,
                    true,
                    1,
                ));
            }
            Operation::Slide { to_right } => {
                let s = if to_right { 1.0 } else { -1.0 };
                let len = ((u1 - u0) * 0.25).min(8.0 * 25.4);
                let head = len * 0.3;
                let (a, t) = (um - s * len, um + s * len);
                out.push(wl(&[[a, zm], [t, zm]], false, false, 1));
                out.push(wl(
                    &[
                        [t - s * head, zm + head * 0.6],
                        [t, zm],
                        [t - s * head, zm - head * 0.6],
                    ],
                    false,
                    false,
                    1,
                ));
            }
            Operation::Fixed | Operation::Hung => {}
        }
    }
    out
}

/// Draws a window's elevation detail into the face spanning `u0`..`u1`, `z0`..`z1`;
/// `mirrored` when seen from inside, so its left is on the viewer's right.
pub(crate) fn elevation_detail(
    b: &mut Builder,
    el: ElementId,
    style: WindowStyle,
    (u0, u1, z0, z1): (f64, f64, f64, f64),
    mirrored: bool,
) {
    let lay = layout(style, u1 - u0, z1 - z0);
    for line in elevation_lines(&lay) {
        let pts: Vec<Pt> = line
            .pts
            .iter()
            .map(|p| {
                let u = if mirrored { u1 - p[0] } else { u0 + p[0] };
                Pt::new(u, z0 + p[1])
            })
            .collect();
        let dash = if line.dashed {
            Dash::Dashed
        } else {
            Dash::Solid
        };
        b.line(Some(el), &pts, line.closed, line.w, dash);
    }
}

/// A Window Library thumbnail: the outline and elevation lines of a style at a size.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct WindowPreview {
    pub width: f64,
    pub height: f64,
    pub lines: Vec<WindowLine>,
}

pub fn preview(style: WindowStyle, width: f64, height: f64) -> WindowPreview {
    let lay = layout(style, width, height);
    let mut lines = vec![wl(&rect([0.0, 0.0, width, height]), true, false, 2)];
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

/// A window's 3D parts: the frame, sashes, mullions and muntins (in the frame finish), and
/// the glass.
#[derive(Debug, Default)]
pub struct Parts {
    pub frame: Vec<f32>,
    pub glass: Vec<f32>,
}

struct Solid<'a> {
    x: Axes,
    lay: &'a WindowLayout,
    h: f64,
    z: f64,
}

impl Solid<'_> {
    /// The unit's centre line at `u`: the wall's centre plane, or a bay's projecting path.
    fn path(&self, u: f64) -> Pt {
        self.x.at(u, self.lay.bay_offset(u, self.h))
    }
    /// A box `u0`..`u1` by `z0`..`z1`, `depth` thick, centred `w` outward of the path;
    /// split at a bay's corners so each piece lies on one face.
    fn part(&self, out: &mut Vec<f32>, r: [f64; 4], w: f64, depth: f64) {
        let [u0, z0, u1, z1] = r;
        if u1 - u0 < 0.1 || z1 - z0 < 0.1 {
            return;
        }
        let mut cuts = vec![u0];
        cuts.extend(
            self.lay
                .corners()
                .into_iter()
                .filter(|c| *c > u0 && *c < u1),
        );
        cuts.push(u1);
        for s in cuts.windows(2) {
            let (a, b) = (self.path(s[0]), self.path(s[1]));
            let mut nn = b.sub(a).norm().perp();
            if nn.dot(self.x.out) < 0.0 {
                nn = nn.scale(-1.0);
            }
            let (lo, hi) = (nn.scale(w - depth / 2.0), nn.scale(w + depth / 2.0));
            let base = Poly::simple(vec![a.add(lo), b.add(lo), b.add(hi), a.add(hi)]);
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
}

pub fn parts(o: &OpeningSolid, style: WindowStyle) -> Parts {
    let lay = layout_of(o, style);
    let s = Solid {
        x: Axes::of(o),
        lay: &lay,
        h: o.half_thickness,
        z: o.z0,
    };
    let (w, ht, f) = (lay.width, lay.height, lay.frame);
    // Keep the frame inside thin walls.
    let depth = lay.depth.min(2.0 * o.half_thickness).max(GLASS * 2.0);
    let mut p = Parts::default();
    for r in [
        [0.0, 0.0, f, ht],
        [w - f, 0.0, w, ht],
        [f, ht - f, w - f, ht],
        [f, 0.0, w - f, f],
    ] {
        s.part(&mut p.frame, r, 0.0, depth);
    }
    for m in &lay.mullions {
        s.part(&mut p.frame, *m, 0.0, depth * 0.9);
    }
    for l in &lay.lites {
        let [u0, z0, u1, z1] = l.rect;
        let r = l.rail;
        let track = l.track.clamp(-depth / 3.0, depth / 3.0);
        if r > 0.0 {
            for piece in [
                [u0, z0, u0 + r, z1],
                [u1 - r, z0, u1, z1],
                [u0 + r, z0, u1 - r, z0 + r],
                [u0 + r, z1 - r, u1 - r, z1],
            ] {
                s.part(&mut p.frame, piece, track, SASH_DEPTH.min(depth));
            }
        }
        s.part(&mut p.glass, l.glass(), track, GLASS);
        for bar in &l.bars {
            let rb = if bar[0] == bar[2] {
                [bar[0] - BAR / 2.0, bar[1], bar[0] + BAR / 2.0, bar[3]]
            } else {
                [bar[0], bar[1] - BAR / 2.0, bar[2], bar[1] + BAR / 2.0]
            };
            s.part(&mut p.frame, rb, track, MUNTIN_DEPTH);
        }
    }
    if lay.projection > 0.0 {
        // Seat and head boards, from the wall's inside face out to the bay.
        let h = o.half_thickness;
        let mut us = vec![0.0];
        us.extend(lay.corners());
        us.push(w);
        let mut ring: Vec<Pt> = us
            .iter()
            .map(|u| s.x.at(*u, lay.bay_offset(*u, h) + depth / 2.0))
            .collect();
        ring.push(s.x.at(w, -h));
        ring.push(s.x.at(0.0, -h));
        let base = Poly::simple(ring);
        for (z0, z1) in [(-BOARD, 0.0), (ht, ht + BOARD)] {
            p.frame.extend(
                Prism {
                    base: base.clone(),
                    z0: o.z0 + z0,
                    z1: o.z0 + z1,
                }
                .triangles(),
            );
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::windows::{Grille, WindowStyle};
    use studio_core::WindowFamily as F;
    use studio_regen::OpeningKind;

    const IN: f64 = 25.4;

    fn opening(style: WindowStyle, width: f64, height: f64, flip: bool) -> OpeningSolid {
        OpeningSolid {
            id: ElementId::new(),
            host: ElementId::new(),
            kind: OpeningKind::Window(style),
            wall_start: Pt::new(0.0, 0.0),
            dir: Pt::new(1.0, 0.0),
            half_thickness: 4.0 * IN,
            t0: 1000.0,
            t1: 1000.0 + width,
            z0: 900.0,
            z1: 900.0 + height,
            flip_hand: false,
            flip_facing: flip,
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
    fn axes_put_the_left_jamb_on_the_left_seen_from_outside() {
        // A wall running +x faces +y (its left) unless flipped.
        let o = opening(WindowStyle::new(F::Fixed), 1000.0, 1000.0, false);
        let x = Axes::of(&o);
        assert_eq!((x.out.x, x.out.y), (0.0, 1.0));
        // Standing at +y looking toward -y, left is +x: the jamb at t1.
        assert_eq!((x.origin.x, x.ax.x), (2000.0, -1.0));
        let f = Axes::of(&opening(WindowStyle::new(F::Fixed), 1000.0, 1000.0, true));
        assert_eq!((f.origin.x, f.ax.x, f.out.y), (1000.0, 1.0, -1.0));
    }

    #[test]
    fn a_double_hung_has_a_frame_two_sashes_and_two_panes() {
        let o = opening(WindowStyle::new(F::DoubleHung), 36.0 * IN, 60.0 * IN, false);
        let p = parts(&o, WindowStyle::new(F::DoubleHung));
        // 12 triangles a box: 4 frame + 2 x 4 sash pieces; 2 panes.
        assert_eq!(p.frame.len() / 9, 12 * 12);
        assert_eq!(p.glass.len() / 9, 2 * 12);
        let g = bounds(&p.glass);
        // The glass fills the frame inside the stiles: 36 - 2 x 2 - 2 x 1.75 = 28.5" wide.
        assert!(((g[3] - g[0]) as f64 - 28.5 * IN).abs() < 0.01, "{g:?}");
        // The two sashes sit in separate tracks: panes 1.5" apart through the wall.
        assert!(
            ((g[4] - g[1]) as f64 - (1.5 * IN + GLASS)).abs() < 0.01,
            "{g:?}"
        );
        let f = bounds(&p.frame);
        assert!(
            (f[2] as f64 - 900.0).abs() < 0.01 && (f[5] as f64 - (900.0 + 60.0 * IN)).abs() < 0.01
        );
    }

    #[test]
    fn grilles_add_muntins() {
        let mut s = WindowStyle::new(F::DoubleHung);
        let plain = parts(&opening(s, 36.0 * IN, 60.0 * IN, false), s)
            .frame
            .len();
        s.grille = Grille::Colonial;
        let colonial = parts(&opening(s, 36.0 * IN, 60.0 * IN, false), s)
            .frame
            .len();
        // 6-over-6: 3 bars a sash.
        assert_eq!((colonial - plain) / 9, 6 * 12);
    }

    #[test]
    fn a_bay_projects_outside_the_wall() {
        let s = WindowStyle::new(F::Bay);
        let o = opening(s, 96.0 * IN, 60.0 * IN, false);
        let p = parts(&o, s);
        let f = bounds(&p.frame);
        // The wall's outside face is at y = 4"; the bay's centre line 18" past it, plus
        // half the frame depth. Its boards reach back to the inside face.
        let front = (4.0 + 18.0 + 3.25 / 2.0) * IN;
        assert!((f[4] as f64 - front).abs() < 0.5, "{f:?}");
        assert!((f[1] as f64 + 4.0 * IN).abs() < 0.5, "{f:?}");
        // Glass on all three faces: the centre pane is out at the projection.
        let g = bounds(&p.glass);
        assert!((g[4] as f64 - 22.125 * IN).abs() < 0.01, "{g:?}");
        // Seat board under the sill.
        assert!((f[2] as f64 - (900.0 - BOARD)).abs() < 0.01);
    }

    #[test]
    fn plan_symbols_by_family() {
        let count = |s: WindowStyle, w: f64| {
            let o = opening(s, w, 48.0 * IN, false);
            let mut b = Builder::new(48.0);
            plan_symbol(&mut b, None, &o, s);
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
        // Fixed: 2 wall faces + 2 glass lines.
        assert_eq!(count(WindowStyle::new(F::Fixed), 48.0 * IN), (4, 0));
        // Casement: plus its swing.
        assert_eq!(count(WindowStyle::new(F::Casement), 36.0 * IN), (5, 1));
        // Slider: two staggered sashes of two lines each.
        assert_eq!(count(WindowStyle::new(F::Slider), 60.0 * IN), (6, 0));
        // Picture + casements: 3 lites, 2 mullions, 2 swings.
        assert_eq!(
            count(WindowStyle::new(F::PictureCasement), 96.0 * IN),
            (2 + 6 + 2 + 2, 2)
        );
        // Bay: inside face and three polylines along the projection.
        assert_eq!(count(WindowStyle::new(F::Bay), 96.0 * IN), (4, 0));
    }

    #[test]
    fn elevation_marks_point_to_the_hinge() {
        let lay = layout(WindowStyle::new(F::Casement), 36.0 * IN, 48.0 * IN);
        let lines = elevation_lines(&lay);
        let mark = lines.iter().find(|l| l.dashed).unwrap();
        // Hinged on the left: the mark's apex is at the left of the glass, mid-height.
        let apex = mark.pts[1];
        assert!((apex[0] - (2.0 + 1.75) * IN).abs() < 1e-6 && (apex[1] - 24.0 * IN).abs() < 1e-6);
        let awning = elevation_lines(&layout(WindowStyle::new(F::Awning), 36.0 * IN, 24.0 * IN));
        let apex = awning.iter().find(|l| l.dashed).unwrap().pts[1];
        assert!(
            (apex[1] - (24.0 - 2.0 - 1.75) * IN).abs() < 1e-6,
            "awnings hinge at the top"
        );
        // Mirrored when seen from inside.
        let mut b = Builder::new(48.0);
        let s = WindowStyle::new(F::Casement);
        elevation_detail(
            &mut b,
            ElementId::new(),
            s,
            (0.0, 36.0 * IN, 0.0, 48.0 * IN),
            true,
        );
        let dashed = b
            .items
            .iter()
            .find_map(|i| match &i.prim {
                super::super::Prim::Line {
                    pts,
                    dash: Dash::Dashed,
                    ..
                } => Some(pts.clone()),
                _ => None,
            })
            .unwrap();
        assert!((dashed[1][0] - (36.0 - 3.75) * IN).abs() < 1e-6);
    }
}
