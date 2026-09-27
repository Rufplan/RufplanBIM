//! Lighting fixtures in views (ADR-057): plan symbols (ceiling plans, and floor plans for
//! wall, floor and site fixtures), 3D bodies with glowing lenses, picker thumbnails, and
//! the lights a render places.

use serde::Serialize;
use studio_core::lighting::{
    kelvin_rgb, FixtureSpec, LightAim, LightDistribution, LightFamily, LightMount, LightShape,
};
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::Pt;
use ts_rs::TS;

use crate::{arc, ring, Builder, Dash, FillKind, Mesh};

const IN: f64 = 25.4;

/// Where a fixture is and which way it faces.
struct Frame<'a> {
    spec: &'a FixtureSpec,
    at: Pt,
    /// The level's elevation plus the fixture's: the ceiling, the wall height or the floor.
    z: f64,
    /// Facing (out of the wall) and across.
    u: Pt,
    v: Pt,
}

fn frame<'a>(doc: &'a Document, data: &ElementData) -> Option<Frame<'a>> {
    let ElementData::LightingFixture {
        type_id,
        level,
        at,
        elevation,
        rotation,
        ..
    } = data
    else {
        return None;
    };
    let spec = studio_core::lighting::spec_of(doc, *type_id)?;
    let z = doc.level_elevation(*level).ok()? + elevation;
    let u = Pt::new(rotation.cos(), rotation.sin());
    Some(Frame {
        spec,
        at: *at,
        z,
        u,
        v: u.perp(),
    })
}

/// A point in the fixture's frame: `f` out (facing), `s` across, height `z`.
fn local(fr: &Frame, f: f64, s: f64) -> Pt {
    fr.at.add(fr.u.scale(f)).add(fr.v.scale(s))
}

// ---------- Plan symbols ----------

/// The fixture symbols of `level` in a plan: all of them in a ceiling plan, the wall,
/// floor and site ones in a floor plan.
pub(crate) fn plan_symbols(doc: &Document, b: &mut Builder, level: ElementId, ceiling: bool) {
    for e in doc.of(Category::LightingFixture) {
        let ElementData::LightingFixture { level: l, .. } = &e.data else {
            continue;
        };
        if *l != level {
            continue;
        }
        let Some(fr) = frame(doc, &e.data) else {
            continue;
        };
        if !ceiling && !fr.spec.mount.in_floor_plan() {
            continue;
        }
        symbol(b, Some(e.id), &fr);
    }
}

fn rect(fr: &Frame, f0: f64, f1: f64, half: f64) -> Vec<Pt> {
    vec![
        local(fr, f0, -half),
        local(fr, f1, -half),
        local(fr, f1, half),
        local(fr, f0, half),
    ]
}

fn symbol(b: &mut Builder, el: Option<ElementId>, fr: &Frame) {
    use LightFamily::*;
    let s = fr.spec;
    let (w, d) = (s.width, s.depth);
    let c = fr.at;
    let line = |b: &mut Builder, pts: &[Pt], closed: bool| b.line(el, pts, closed, 2, Dash::Solid);
    // A paper-white back so the symbol reads over ceiling grids.
    let back = |b: &mut Builder, pts: &[Pt]| b.fill(el, vec![ring(pts)], FillKind::Paper);
    match s.family {
        Downlight | Gimbal | WallWasher => {
            let r = w / 2.0;
            let o = arc(c, r, 0.0, std::f64::consts::TAU);
            back(b, &o);
            line(b, &o, true);
            if s.family == Gimbal {
                line(b, &arc(c, r * 0.45, 0.0, std::f64::consts::TAU), true);
            }
            if s.family == WallWasher {
                // The kick reflector toward the washed wall.
                let a = fr.u.y.atan2(fr.u.x);
                line(b, &arc(c, r * 1.35, a - 1.0, 2.0), false);
            }
        }
        Troffer | FlatPanel => {
            // Revit's lay-in troffer: the outline with its diagonals.
            let p = rect(fr, -d / 2.0, d / 2.0, w / 2.0);
            back(b, &p);
            line(b, &p, true);
            b.line(el, &[p[0], p[2]], false, 1, Dash::Solid);
            b.line(el, &[p[1], p[3]], false, 1, Dash::Solid);
        }
        LinearPendant | LinearSlot | StripLight | CoveLight | UnderCabinet | VanityLight => {
            let (long, across) = if matches!(s.family, CoveLight | UnderCabinet | VanityLight) {
                // Wall fixtures run along the wall.
                (w, d)
            } else {
                (d, w)
            };
            let p = if matches!(s.family, CoveLight | UnderCabinet | VanityLight) {
                rect_along(fr, long, across)
            } else {
                rect(fr, -long / 2.0, long / 2.0, across / 2.0)
            };
            back(b, &p);
            line(b, &p, true);
            let mid = |i: usize, j: usize| p[i].lerp(p[j], 0.5);
            b.line(el, &[mid(0, 3), mid(1, 2)], false, 1, Dash::Solid);
        }
        TrackLight => {
            b.line(
                el,
                &[
                    local(fr, -d.max(w) / 2.0, 0.0),
                    local(fr, d.max(w) / 2.0, 0.0),
                ],
                false,
                3,
                Dash::Solid,
            );
            let run = d.max(w);
            for k in [-1.0, 0.0, 1.0] {
                let h = local(fr, k * run / 3.0, 0.0);
                let o = arc(h, 1.5 * IN, 0.0, std::f64::consts::TAU);
                back(b, &o);
                line(b, &o, true);
            }
        }
        FlushMount | DrumPendant | GlobePendant | MiniPendant | HighBay | Chandelier
        | CeilingFan => {
            let r = w / 2.0;
            let o = arc(c, r, 0.0, std::f64::consts::TAU);
            back(b, &o);
            line(b, &o, true);
            match s.family {
                FlushMount => {
                    let k = r * std::f64::consts::FRAC_1_SQRT_2;
                    b.line(
                        el,
                        &[c.add(Pt::new(-k, -k)), c.add(Pt::new(k, k))],
                        false,
                        1,
                        Dash::Solid,
                    );
                    b.line(
                        el,
                        &[c.add(Pt::new(-k, k)), c.add(Pt::new(k, -k))],
                        false,
                        1,
                        Dash::Solid,
                    );
                }
                Chandelier => {
                    for i in 0..6 {
                        let a = f64::from(i) * std::f64::consts::TAU / 6.0;
                        let p = c.add(Pt::new(a.cos(), a.sin()).scale(r * 0.7));
                        line(b, &arc(p, r * 0.12, 0.0, std::f64::consts::TAU), true);
                    }
                }
                CeilingFan => {
                    for i in 0..5 {
                        let a = f64::from(i) * std::f64::consts::TAU / 5.0;
                        let dir = Pt::new(a.cos(), a.sin());
                        b.line(
                            el,
                            &[c.add(dir.scale(r * 0.2)), c.add(dir.scale(r))],
                            false,
                            1,
                            Dash::Solid,
                        );
                    }
                    line(b, &arc(c, r * 0.2, 0.0, std::f64::consts::TAU), true);
                }
                _ => {
                    // A pendant: the canopy, filled.
                    b.fill(
                        el,
                        vec![ring(&arc(c, r * 0.18, 0.0, std::f64::consts::TAU))],
                        FillKind::Ink,
                    );
                }
            }
        }
        WallSconce | Lantern | StepLight => {
            // Half round against the wall, bulging out of it.
            let a = fr.u.y.atan2(fr.u.x);
            let r = w.max(d) / 2.0;
            let mut half = arc(c, r, a - std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
            back(b, &half);
            half.push(half[0]);
            line(b, &half, false);
        }
        WallPack | FloodLight => {
            let p = rect(fr, 0.0, d, w / 2.0);
            back(b, &p);
            line(b, &p, true);
            // The throw.
            let t = [
                local(fr, d, -w / 3.0),
                local(fr, d + w / 2.0, 0.0),
                local(fr, d, w / 3.0),
            ];
            line(b, &t, false);
        }
        ExitSign | EmergencyLight => {
            let p = rect(fr, 0.0, d.max(2.0 * IN), w / 2.0);
            back(b, &p);
            line(b, &p, true);
            if s.family == ExitSign {
                // Revit's exit sign: the face half-filled toward the arrow side.
                let q = [p[1], p[2], local(fr, d.max(2.0 * IN), 0.0)];
                b.fill(el, vec![ring(&q)], FillKind::Ink);
            } else {
                for k in [-1.0, 1.0] {
                    let h = local(fr, d, k * w / 4.0);
                    line(b, &arc(h, 1.25 * IN, 0.0, std::f64::consts::TAU), true);
                }
            }
        }
        FloorLamp | TableLamp => {
            let r = w / 2.0;
            let o = arc(c, r, 0.0, std::f64::consts::TAU);
            back(b, &o);
            line(b, &o, true);
            line(b, &arc(c, r * 0.35, 0.0, std::f64::consts::TAU), true);
        }
        Bollard | InGrade | LandscapeSpot => {
            let r = w / 2.0;
            let o = arc(c, r, 0.0, std::f64::consts::TAU);
            if s.family == Bollard {
                b.fill(el, vec![ring(&o)], FillKind::Ink);
            } else {
                back(b, &o);
                line(b, &o, true);
                // Uplights: a star of rays.
                for i in 0..4 {
                    let a =
                        f64::from(i) * std::f64::consts::FRAC_PI_2 + std::f64::consts::FRAC_PI_4;
                    let dir = Pt::new(a.cos(), a.sin());
                    b.line(
                        el,
                        &[c.add(dir.scale(r)), c.add(dir.scale(r * 1.6))],
                        false,
                        1,
                        Dash::Solid,
                    );
                }
            }
        }
        AreaPole | PostTop => {
            // The pole, and the head (a shoebox out front, or a lantern on top).
            b.fill(
                el,
                vec![ring(&arc(c, 3.0 * IN, 0.0, std::f64::consts::TAU))],
                FillKind::Ink,
            );
            let p = if s.family == AreaPole {
                rect(fr, 6.0 * IN, 6.0 * IN + w, d / 2.0)
            } else {
                rect(fr, -w / 2.0, w / 2.0, w / 2.0)
            };
            line(b, &p, true);
        }
    }
}

/// A rectangle `long` along the wall and `across` out of it, from the wall face.
fn rect_along(fr: &Frame, long: f64, across: f64) -> Vec<Pt> {
    vec![
        local(fr, 0.0, -long / 2.0),
        local(fr, across, -long / 2.0),
        local(fr, across, long / 2.0),
        local(fr, 0.0, long / 2.0),
    ]
}

// ---------- 3D ----------

/// Triangles of a fixture's body and its lens.
#[derive(Default)]
struct Parts {
    body: Vec<f32>,
    lens: Vec<f32>,
}

fn tri(out: &mut Vec<f32>, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
    for p in [a, b, c] {
        out.extend(p.iter().map(|v| *v as f32));
    }
}

/// A convex prism: `ring` (counter-clockwise) from `z0` to `z1`, capped.
fn prism(out: &mut Vec<f32>, ring: &[Pt], z0: f64, z1: f64) {
    let n = ring.len();
    let p = |q: Pt, z: f64| [q.x, q.y, z];
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        tri(out, p(a, z0), p(b, z0), p(b, z1));
        tri(out, p(a, z0), p(b, z1), p(a, z1));
    }
    for i in 1..n.saturating_sub(1) {
        tri(out, p(ring[0], z1), p(ring[i], z1), p(ring[i + 1], z1));
        tri(out, p(ring[0], z0), p(ring[i + 1], z0), p(ring[i], z0));
    }
}

fn circle(c: Pt, r: f64) -> Vec<Pt> {
    let n = 20;
    (0..n)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::TAU / f64::from(n);
            c.add(Pt::new(a.cos(), a.sin()).scale(r))
        })
        .collect()
}

fn cyl(out: &mut Vec<f32>, c: Pt, r: f64, z0: f64, z1: f64) {
    prism(out, &circle(c, r), z0, z1);
}

/// A box `f0..f1` out and `-half..half` across, in the fixture's frame.
fn boxed(out: &mut Vec<f32>, fr: &Frame, f0: f64, f1: f64, half: f64, z0: f64, z1: f64) {
    prism(out, &rect(fr, f0, f1, half), z0, z1);
}

fn parts(fr: &Frame) -> Parts {
    use LightFamily::*;
    let s = fr.spec;
    let (w, d, h) = (s.width, s.depth, s.height);
    let (c, z) = (fr.at, fr.z);
    let mut p = Parts::default();
    let cable =
        |p: &mut Parts, at: Pt, top: f64, bottom: f64| cyl(&mut p.body, at, 0.2 * IN, bottom, top);
    // Pendants hang `drop` below the ceiling; the body's top is there.
    let top = z - s.drop;
    match s.family {
        Downlight | Gimbal | WallWasher => {
            // A trim ring at the ceiling and the lens just inside it.
            cyl(&mut p.body, c, w / 2.0, z - 0.4 * IN, z);
            cyl(&mut p.lens, c, w * 0.36, z - 0.5 * IN, z - 0.4 * IN);
        }
        Troffer | FlatPanel | LinearSlot => {
            boxed(&mut p.body, fr, -d / 2.0, d / 2.0, w / 2.0, z - 0.5 * IN, z);
            let m = if s.family == LinearSlot { 0.3 } else { 1.0 } * IN;
            boxed(
                &mut p.lens,
                fr,
                -d / 2.0 + m,
                d / 2.0 - m,
                w / 2.0 - m,
                z - 0.6 * IN,
                z - 0.5 * IN,
            );
        }
        FlushMount => {
            cyl(&mut p.body, c, w / 2.0, z - h * 0.3, z);
            cyl(&mut p.lens, c, w * 0.44, z - h, z - h * 0.3);
        }
        CeilingFan => {
            cable(&mut p, c, z, top);
            cyl(&mut p.body, c, 4.0 * IN, top - h * 0.5, top);
            for i in 0..5 {
                let a = f64::from(i) * std::f64::consts::TAU / 5.0;
                let dir = Pt::new(a.cos(), a.sin());
                let bl = Frame {
                    u: dir,
                    v: dir.perp(),
                    ..clone_frame(fr)
                };
                boxed(
                    &mut p.body,
                    &bl,
                    4.0 * IN,
                    w / 2.0,
                    2.5 * IN,
                    top - h * 0.45,
                    top - h * 0.4,
                );
            }
            cyl(&mut p.lens, c, 5.0 * IN, top - h, top - h * 0.5);
        }
        TrackLight => {
            let run = d.max(w);
            boxed(&mut p.body, fr, -run / 2.0, run / 2.0, 0.75 * IN, z - IN, z);
            for k in [-1.0, 0.0, 1.0] {
                let at = local(fr, k * run / 3.0, 0.0);
                cyl(&mut p.body, at, 1.5 * IN, z - h, z - IN);
                cyl(&mut p.lens, at, 1.2 * IN, z - h - 0.1 * IN, z - h);
            }
        }
        DrumPendant | MiniPendant | HighBay => {
            cable(&mut p, c, z, top);
            cyl(&mut p.body, c, w / 2.0, top - h, top);
            cyl(&mut p.lens, c, w * 0.47, top - h - 0.1 * IN, top - h);
        }
        GlobePendant => {
            cable(&mut p, c, z, top);
            let r = w / 2.0;
            cyl(&mut p.lens, c, r * 0.7, top - h * 0.25, top);
            cyl(&mut p.lens, c, r, top - h * 0.75, top - h * 0.25);
            cyl(&mut p.lens, c, r * 0.7, top - h, top - h * 0.75);
        }
        Chandelier => {
            cable(&mut p, c, z, top);
            cyl(&mut p.body, c, 2.0 * IN, top - h, top);
            let r = w / 2.0 * 0.8;
            for i in 0..6 {
                let a = f64::from(i) * std::f64::consts::TAU / 6.0;
                let dir = Pt::new(a.cos(), a.sin());
                let arm = Frame {
                    u: dir,
                    v: dir.perp(),
                    ..clone_frame(fr)
                };
                boxed(
                    &mut p.body,
                    &arm,
                    0.0,
                    r,
                    0.4 * IN,
                    top - h * 0.7,
                    top - h * 0.65,
                );
                let at = c.add(dir.scale(r));
                cyl(&mut p.lens, at, 1.0 * IN, top - h * 0.65, top - h * 0.4);
            }
        }
        LinearPendant => {
            for k in [-1.0, 1.0] {
                cable(&mut p, local(fr, k * d * 0.4, 0.0), z, top);
            }
            boxed(&mut p.body, fr, -d / 2.0, d / 2.0, w / 2.0, top - h, top);
            boxed(
                &mut p.lens,
                fr,
                -d / 2.0 + IN,
                d / 2.0 - IN,
                w / 2.0 - 0.5 * IN,
                top - h - 0.1 * IN,
                top - h,
            );
        }
        StripLight => {
            boxed(&mut p.body, fr, -d / 2.0, d / 2.0, w / 2.0, z - h * 0.4, z);
            boxed(
                &mut p.lens,
                fr,
                -d / 2.0 + IN,
                d / 2.0 - IN,
                w / 2.0 * 0.8,
                z - h,
                z - h * 0.4,
            );
        }
        UnderCabinet | CoveLight | VanityLight => {
            // Along the wall face, centered at the mounting height.
            let long = rect_along(fr, w, d);
            prism(&mut p.body, &long, z - h / 2.0, z + h / 2.0);
            let face = rect_along(fr, w * 0.95, d * 1.05);
            if s.family == CoveLight {
                prism(&mut p.lens, &face, z + h / 2.0, z + h / 2.0 + 0.1 * IN);
            } else {
                prism(&mut p.lens, &face, z - h / 2.0 - 0.1 * IN, z - h / 2.0);
            }
        }
        WallSconce | StepLight | WallPack | FloodLight => {
            boxed(
                &mut p.body,
                fr,
                0.0,
                d * 0.5,
                w / 2.0,
                z - h / 2.0,
                z + h / 2.0,
            );
            boxed(
                &mut p.lens,
                fr,
                d * 0.5,
                d,
                w / 2.0 * 0.85,
                z - h / 2.0 * 0.85,
                z + h / 2.0 * 0.85,
            );
        }
        Lantern => {
            boxed(&mut p.body, fr, 0.0, IN, w / 2.0, z - h / 2.0, z + h / 2.0);
            boxed(
                &mut p.lens,
                fr,
                IN,
                d,
                w / 2.0 * 0.8,
                z - h / 2.0,
                z + h * 0.3,
            );
            boxed(&mut p.body, fr, IN, d, w / 2.0, z + h * 0.3, z + h / 2.0);
        }
        ExitSign => {
            boxed(&mut p.lens, fr, 0.0, d, w / 2.0, z - h / 2.0, z + h / 2.0);
        }
        EmergencyLight => {
            boxed(&mut p.body, fr, 0.0, d, w / 2.0, z - h / 2.0, z + h / 4.0);
            for k in [-1.0, 1.0] {
                cyl(
                    &mut p.lens,
                    local(fr, d, k * w / 4.0),
                    1.5 * IN,
                    z + h / 4.0,
                    z + h / 2.0,
                );
            }
        }
        FloorLamp | TableLamp => {
            let shade = 10.0 * IN;
            cyl(
                &mut p.body,
                c,
                if s.family == FloorLamp { 5.0 } else { 3.0 } * IN,
                z,
                z + IN,
            );
            cyl(&mut p.body, c, 0.5 * IN, z + IN, z + h - shade);
            cyl(&mut p.lens, c, w / 2.0, z + h - shade, z + h);
        }
        Bollard => {
            cyl(&mut p.body, c, w / 2.0, z, z + h - 6.0 * IN);
            cyl(&mut p.lens, c, w / 2.0 * 0.95, z + h - 6.0 * IN, z + h - IN);
            cyl(&mut p.body, c, w / 2.0, z + h - IN, z + h);
        }
        AreaPole => {
            cyl(&mut p.body, c, 2.5 * IN, z, z + h);
            boxed(
                &mut p.body,
                fr,
                0.0,
                6.0 * IN,
                IN,
                z + h - 3.0 * IN,
                z + h - IN,
            );
            boxed(
                &mut p.body,
                fr,
                6.0 * IN,
                6.0 * IN + w,
                d / 2.0,
                z + h - 5.0 * IN,
                z + h,
            );
            boxed(
                &mut p.lens,
                fr,
                7.0 * IN,
                5.0 * IN + w,
                d / 2.0 - IN,
                z + h - 5.1 * IN,
                z + h - 5.0 * IN,
            );
        }
        PostTop => {
            let lantern = 24.0 * IN;
            cyl(&mut p.body, c, 2.0 * IN, z, z + h - lantern);
            boxed(
                &mut p.lens,
                fr,
                -w / 2.0,
                w / 2.0,
                w / 2.0,
                z + h - lantern,
                z + h - 4.0 * IN,
            );
            boxed(
                &mut p.body,
                fr,
                -w / 2.0 - IN,
                w / 2.0 + IN,
                w / 2.0 + IN,
                z + h - 4.0 * IN,
                z + h,
            );
        }
        InGrade => {
            cyl(&mut p.body, c, w / 2.0, z, z + 0.3 * IN);
            cyl(&mut p.lens, c, w * 0.38, z + 0.3 * IN, z + 0.4 * IN);
        }
        LandscapeSpot => {
            cyl(&mut p.body, c, 0.4 * IN, z, z + h - 3.0 * IN);
            cyl(&mut p.body, c, w / 2.0, z + h - 3.0 * IN, z + h);
            cyl(&mut p.lens, c, w * 0.4, z + h, z + h + 0.1 * IN);
        }
    }
    p
}

fn clone_frame<'a>(fr: &Frame<'a>) -> Frame<'a> {
    Frame {
        spec: fr.spec,
        at: fr.at,
        z: fr.z,
        u: fr.u,
        v: fr.v,
    }
}

/// Body color: white fixtures indoors, dark bronze outside.
fn body_color(spec: &FixtureSpec) -> [u8; 3] {
    match spec.family.group() {
        "Site & Exterior" => [58, 52, 46],
        _ if spec.family == LightFamily::Chandelier => [176, 141, 87],
        _ => [236, 236, 232],
    }
}

/// The lens glows in its light's color (warm white for most, red for exit signs); an
/// off fixture's lens is plain.
fn lens_glow(spec: &FixtureSpec, on: bool) -> Option<[u8; 3]> {
    if !on {
        return None;
    }
    Some(if spec.family == LightFamily::ExitSign {
        [230, 40, 30]
    } else {
        let k = kelvin_rgb(spec.kelvin);
        // Toward white: a lit lens reads as bright, slightly tinted.
        k.map(|c| (f64::from(c) * 0.4 + 255.0 * 0.6).round() as u8)
    })
}

/// 3D meshes of the fixtures: the body, and the lens with its glow.
pub(crate) fn meshes(doc: &Document, out: &mut Vec<Mesh>) {
    for e in doc.of(Category::LightingFixture) {
        let ElementData::LightingFixture { level, on, .. } = &e.data else {
            continue;
        };
        let Some(fr) = frame(doc, &e.data) else {
            continue;
        };
        let p = parts(&fr);
        let mesh = |positions: Vec<f32>, color: [u8; 3], glow: Option<[u8; 3]>| Mesh {
            el: e.id,
            category: Category::LightingFixture,
            exterior: false,
            color: Some(color),
            material: None,
            level: Some(*level),
            positions,
            edges: vec![],
            glow,
        };
        if !p.body.is_empty() {
            out.push(mesh(p.body, body_color(fr.spec), None));
        }
        if !p.lens.is_empty() {
            out.push(mesh(p.lens, [250, 250, 246], lens_glow(fr.spec, *on)));
        }
    }
}

/// A fixture by itself for the picker (at the origin; ceiling ones hung from z = 0).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FixtureThumb {
    pub body: Vec<f32>,
    pub lens: Vec<f32>,
    pub color: [u8; 3],
    pub glow: [u8; 3],
}

pub fn thumb(spec: &FixtureSpec) -> FixtureThumb {
    let z = 0.0;
    // Wall fixtures face the camera's side (-y), floor ones sit at z = 0.
    let rot = if spec.mount == LightMount::Wall {
        -std::f64::consts::FRAC_PI_2
    } else {
        0.0
    };
    let u = Pt::new(rot.cos(), rot.sin());
    let fr = Frame {
        spec,
        at: Pt::default(),
        z,
        u,
        v: u.perp(),
    };
    let p = parts(&fr);
    FixtureThumb {
        body: p.body,
        lens: p.lens,
        color: body_color(spec),
        glow: lens_glow(spec, true).unwrap_or([255, 255, 255]),
    }
}

// ---------- Lights for renders ----------

/// One fixture's light, for renders (mm, z up).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LightInfo {
    pub el: ElementId,
    pub type_id: ElementId,
    pub level: ElementId,
    /// Where the light leaves the fixture (the lens), and which way it points.
    pub at: [f64; 3],
    pub dir: [f64; 3],
    /// The emitting shape's long axis in plan, and its size (width, length) in mm.
    pub axis: [f64; 2],
    pub size: [f64; 2],
    pub shape: LightShape,
    pub distribution: LightDistribution,
    /// Initial intensity times the dimming.
    pub lumens: f64,
    pub color: [u8; 3],
    /// Spot beam (full) angle, degrees.
    pub beam: f64,
    pub on: bool,
    pub dimming: f64,
    pub exterior: bool,
}

/// Every fixture's light (off ones too, for Artificial Lights).
pub fn lights(doc: &Document) -> Vec<LightInfo> {
    let mut out = vec![];
    for e in doc.of(Category::LightingFixture) {
        let ElementData::LightingFixture {
            type_id,
            level,
            on,
            dimming,
            ..
        } = &e.data
        else {
            continue;
        };
        let Some(fr) = frame(doc, &e.data) else {
            continue;
        };
        let s = fr.spec;
        let p = parts(&fr);
        let aim = s.family.aim();
        // The lens's extent: its lowest point for down lights, highest for up lights, its
        // front for wall ones.
        let lens = if p.lens.is_empty() { &p.body } else { &p.lens };
        let pts: Vec<[f64; 3]> = lens
            .as_chunks::<3>()
            .0
            .iter()
            .map(|q| [f64::from(q[0]), f64::from(q[1]), f64::from(q[2])])
            .collect();
        let n = pts.len().max(1) as f64;
        let mut at = [0.0; 3];
        for q in &pts {
            for k in 0..3 {
                at[k] += q[k] / n;
            }
        }
        let zs = pts.iter().map(|q| q[2]);
        let (lo, hi) = zs.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), z| {
            (a.min(z), b.max(z))
        });
        let out_dir = [fr.u.x, fr.u.y, 0.0];
        let dir = match aim {
            LightAim::Down => {
                at[2] = lo - 1.0;
                [0.0, 0.0, -1.0]
            }
            LightAim::Up => {
                at[2] = hi + 1.0;
                [0.0, 0.0, 1.0]
            }
            LightAim::Out => {
                // Out of the wall and 30° down.
                let (c, sn) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
                [out_dir[0] * c, out_dir[1] * c, -sn]
            }
        };
        if aim == LightAim::Out {
            at[0] += fr.u.x * 1.0;
            at[1] += fr.u.y * 1.0;
        }
        let long_is_depth = matches!(
            s.family,
            LightFamily::Troffer
                | LightFamily::FlatPanel
                | LightFamily::LinearSlot
                | LightFamily::LinearPendant
                | LightFamily::StripLight
                | LightFamily::TrackLight
        );
        let (axis, size) = if long_is_depth {
            ([fr.u.x, fr.u.y], [s.width, s.depth])
        } else {
            ([fr.v.x, fr.v.y], [s.depth, s.width])
        };
        out.push(LightInfo {
            el: e.id,
            type_id: *type_id,
            level: *level,
            at,
            dir,
            axis,
            size,
            shape: s.shape,
            distribution: s.distribution,
            lumens: s.lumens * dimming,
            color: if s.family == LightFamily::ExitSign {
                [230, 40, 30]
            } else {
                kelvin_rgb(s.kelvin)
            },
            beam: s.beam,
            on: *on,
            dimming: *dimming,
            exterior: s.family.group() == "Site & Exterior",
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::lighting::{create_fixture, load};
    use studio_core::ops;

    fn project() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        (doc, l1)
    }

    fn place(doc: &mut Document, l1: ElementId, name: &str, at: Pt) -> ElementId {
        let t = load(doc, &[name.to_string()]).unwrap()[0];
        create_fixture(doc, t, l1, at, 0.0, None).unwrap()
    }

    fn bounds(v: &[f32]) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for q in v.as_chunks::<3>().0 {
            for k in 0..3 {
                lo[k] = lo[k].min(f64::from(q[k]));
                hi[k] = hi[k].max(f64::from(q[k]));
            }
        }
        (lo, hi)
    }

    #[test]
    fn every_library_fixture_has_a_body_or_lens_and_a_light() {
        let (mut doc, l1) = project();
        for (i, p) in studio_core::lighting::catalog().iter().enumerate() {
            let at = Pt::new(f64::from(i as u32) * 3000.0, 0.0);
            let id = place(&mut doc, l1, &p.name, at);
            let t = thumb(&p.spec);
            assert!(!t.lens.is_empty(), "{} has a lens", p.name);
            assert_eq!(t.lens.len() % 9, 0);
            let l = lights(&doc).into_iter().find(|l| l.el == id).unwrap();
            assert!(l.lumens > 0.0 && l.on, "{}", p.name);
        }
        let mut m = vec![];
        meshes(&doc, &mut m);
        assert!(m.iter().any(|m| m.glow.is_some()));
    }

    #[test]
    fn a_troffer_hangs_from_the_ceiling_and_lights_straight_down() {
        let (mut doc, l1) = project();
        let id = place(&mut doc, l1, "2x4 LED Troffer", Pt::new(1000.0, 2000.0));
        let mut m = vec![];
        meshes(&doc, &mut m);
        let body = m.iter().find(|m| m.el == id && m.glow.is_none()).unwrap();
        let (lo, hi) = bounds(&body.positions);
        // 9'-0" ceiling, the 2'x4' body 1/2" deep under it; 4' along x at rotation 0.
        assert!((hi[2] - 2743.2).abs() < 0.01 && (lo[2] - (2743.2 - 12.7)).abs() < 0.01);
        assert!((hi[0] - lo[0] - 1219.2).abs() < 0.01 && (hi[1] - lo[1] - 609.6).abs() < 0.01);
        let l = &lights(&doc)[0];
        assert_eq!(l.dir, [0.0, 0.0, -1.0]);
        assert!(l.at[2] < lo[2]);
        assert!((l.size[0] - 609.6).abs() < 1e-6 && (l.size[1] - 1219.2).abs() < 1e-6);
        assert_eq!(l.lumens, 4000.0);
        // Dimmed to half: half the lumens; off: the lens stops glowing.
        ops::set_property(&mut doc, id, "dimming", "50", 0).unwrap();
        ops::set_property(&mut doc, id, "on", "no", 0).unwrap();
        let l = &lights(&doc)[0];
        assert!((l.lumens - 2000.0).abs() < 1e-9 && !l.on);
        let mut m = vec![];
        meshes(&doc, &mut m);
        assert!(m.iter().all(|m| m.glow.is_none()));
    }

    #[test]
    fn ceiling_plans_draw_every_fixture_and_floor_plans_the_wall_and_floor_ones() {
        let (mut doc, l1) = project();
        let down = place(&mut doc, l1, "6\" LED Downlight", Pt::new(0.0, 0.0));
        let lamp = place(&mut doc, l1, "Floor Lamp", Pt::new(3000.0, 0.0));
        let drawn = |ceiling: bool| {
            let mut b = Builder::new(48.0);
            plan_symbols(&doc, &mut b, l1, ceiling);
            let ids: Vec<ElementId> = b.items.iter().filter_map(|i| i.el).collect();
            (ids.contains(&down), ids.contains(&lamp))
        };
        assert_eq!(drawn(true), (true, true));
        assert_eq!(drawn(false), (false, true));
        // The downlight's circle is its 7-1/2" trim.
        let mut b = Builder::new(48.0);
        plan_symbols(&doc, &mut b, l1, true);
        let far = b
            .items
            .iter()
            .filter(|i| i.el == Some(down))
            .filter_map(|i| match &i.prim {
                crate::Prim::Line { pts, .. } => Some(
                    pts.iter()
                        .map(|p| Pt::new(p[0], p[1]).len())
                        .fold(0.0, f64::max),
                ),
                _ => None,
            })
            .fold(0.0, f64::max);
        assert!((far - 7.5 * IN / 2.0).abs() < 1e-6);
    }
}
