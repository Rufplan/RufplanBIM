//! Planting in views (ADR-064), after Enscape's assets:
//! - **Models.** Each species is grown from its spec. Broadleaf trees, pines and shrubs use a
//!   space-colonization skeleton (branches grow toward points scattered in the crown's
//!   envelope), with pipe-model branch thickness and bark tubes. Firs, spruces and cypresses
//!   are a leader with whorled, drooping branches. Palms have curved trunks with feather or
//!   fan fronds; there are also rosettes (agave, yucca, hosta), grass clumps, clipped hedges
//!   and ribbed cacti.
//! - **Foliage.** Leaves are alpha-cut cards textured with the species' foliage atlas
//!   ([`crate::foliage`]). Their normals point out of the crown, and the crown's occlusion
//!   is baked into their colour: SpeedTree's and Enscape's soft, volumetric crowns.
//! - **Proxies.** The working views (Shaded, Hidden Line) and picking use a low-poly proxy,
//!   as Enscape's assets appear in Revit. Realistic views and renders show the full model.
//! - **Plan and elevation.** Revit-style planting symbols in plan (a scalloped canopy with
//!   branches, a conifer star, palm fronds, a shrub cloud, a grass tuft) and silhouettes in
//!   elevations and sections.

use serde::Serialize;
use studio_core::planting::{CrownForm, Foliage, PlantGroup, PlantSpec};
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::Pt;
use ts_rs::TS;

use crate::{Builder, Dash, Mesh};

/// Model variants per species, so neighbouring trees differ.
pub const VARIANTS: u32 = 3;

type V = [f64; 3];

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V, k: f64) -> V {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length(a: V) -> f64 {
    dot(a, a).sqrt()
}
fn norm(a: V) -> V {
    let l = length(a);
    if l < 1e-12 {
        [0.0, 0.0, 1.0]
    } else {
        mul(a, 1.0 / l)
    }
}
fn lerp(a: V, b: V, t: f64) -> V {
    add(a, mul(sub(b, a), t))
}
/// Any unit vector perpendicular to `d`.
fn perp(d: V) -> V {
    let h = if d[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    norm(cross(d, h))
}

/// A small deterministic random source (SplitMix64).
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub(crate) fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub(crate) fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
    /// Uniform in [-1, 1).
    pub(crate) fn sym(&mut self) -> f64 {
        self.f() * 2.0 - 1.0
    }
    fn unit(&mut self) -> V {
        loop {
            let v = [self.sym(), self.sym(), self.sym()];
            let l = length(v);
            if l > 0.05 && l <= 1.0 {
                return mul(v, 1.0 / l);
            }
        }
    }
}

/// A seed for a species' variant: its botanical name, form and size.
pub fn seed_of(spec: &PlantSpec, variant: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: u8| {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    for b in spec.botanical.bytes() {
        eat(b);
    }
    for b in format!(
        "{:?}{:.0}{:.0}{variant}",
        spec.form, spec.height, spec.spread
    )
    .bytes()
    {
        eat(b);
    }
    h
}

// ---------------------------------------------------------------- envelope

/// A crown's radius profile, 0..1, at `t` up the crown (0 bottom, 1 top).
fn profile(form: CrownForm, t: f64) -> f64 {
    use CrownForm::*;
    if !(0.0..=1.0).contains(&t) {
        return 0.0;
    }
    let dome = (1.0 - (2.0 * t - 1.0).powi(2)).max(0.0).sqrt();
    let s = (std::f64::consts::PI * t).sin().max(0.0);
    match form {
        Round | Irregular | Weeping => dome,
        Oval => s.powf(0.8) * (1.0 - 0.15 * t),
        Vase => (0.3 + 0.7 * t.powf(0.6)) * (1.0 - t.powi(6)).max(0.0).sqrt(),
        Columnar | Upright => s.powf(0.4) * (1.0 - 0.2 * t),
        Pyramidal => (1.0 - t).powf(0.85) * (t * 6.0).min(1.0).sqrt(),
        Conical => (1.0 - t).powf(1.05) * (t * 12.0).min(1.0).sqrt(),
        Spreading => s.powf(0.55),
        Umbrella => t.powf(0.3) * (1.0 - t.powi(3)).max(0.0).sqrt(),
        Mound | Rosette => (1.0 - t * t).max(0.0).sqrt() * (0.7 + 0.3 * (t * 4.0).min(1.0)),
        Fountain => (0.35 + 0.65 * t.powf(0.5)) * (1.0 - t.powi(4)).max(0.0).sqrt(),
        PalmHead | Box | Cactus => dome,
    }
}

/// The volume the crown fills: its bottom and top (mm above the base), its half-spread and
/// a lumpy outline (lobes round the trunk, more for irregular crowns).
struct Envelope {
    form: CrownForm,
    z0: f64,
    z1: f64,
    r: f64,
    lobes: [(f64, f64); 4],
    rough: f64,
}

impl Envelope {
    fn new(spec: &PlantSpec, rng: &mut Rng) -> Envelope {
        use CrownForm::*;
        let h = spec.height;
        let z0 = match spec.form {
            Mound | Rosette | Fountain => 0.0,
            Upright | Box => h * 0.04,
            Weeping => (spec.trunk * 0.5).max(h * 0.1),
            _ if spec.group.is_tree() => spec.trunk.min(h * 0.8),
            _ => h * 0.05,
        };
        let rough = match spec.form {
            Irregular => 0.3,
            Umbrella | Spreading => 0.18,
            Conical | Columnar | Box => 0.06,
            _ => 0.12,
        };
        let mut lobes = [(0.0, 0.0); 4];
        for (k, l) in lobes.iter_mut().enumerate() {
            *l = (
                rng.range(0.0, std::f64::consts::TAU),
                rng.range(0.4, 1.0) / (k + 1) as f64,
            );
        }
        Envelope {
            form: spec.form,
            z0,
            z1: h,
            r: spec.spread / 2.0,
            lobes,
            rough,
        }
    }
    fn t(&self, z: f64) -> f64 {
        (z - self.z0) / (self.z1 - self.z0).max(1.0)
    }
    /// The lumpiness at angle `a` and height `t`, about 1.
    fn lump(&self, a: f64, t: f64) -> f64 {
        let mut m = 0.0;
        for (k, (ph, amp)) in self.lobes.iter().enumerate() {
            let n = (k + 2) as f64;
            m += amp * (a * n + ph + t * 3.0 * n).sin();
        }
        1.0 + self.rough * m / 1.6
    }
    fn radius(&self, z: f64, a: f64) -> f64 {
        let t = self.t(z);
        self.r * profile(self.form, t) * self.lump(a, t)
    }
    fn contains(&self, p: V) -> bool {
        let rr = p[0].hypot(p[1]);
        rr <= self.radius(p[2], p[1].atan2(p[0]))
    }
    fn center(&self) -> V {
        [0.0, 0.0, (self.z0 + self.z1) / 2.0]
    }
    /// How far out toward the envelope a point is (0 at the axis, 1 at the edge).
    fn outer(&self, p: V) -> f64 {
        let r = self.radius(p[2], p[1].atan2(p[0])).max(1.0);
        (p[0].hypot(p[1]) / r).clamp(0.0, 1.2)
    }
}

// ---------------------------------------------------------------- skeleton

struct Node {
    p: V,
    parent: Option<usize>,
    children: Vec<usize>,
    r: f64,
    /// Steps to the nearest tip.
    tip: u32,
    /// Part of a weeping curtain or a conifer branch (leaf cards along it, not just at
    /// its tips).
    leafy_along: bool,
}

#[derive(Default)]
struct Skeleton {
    nodes: Vec<Node>,
    roots: Vec<usize>,
}

impl Skeleton {
    fn push(&mut self, p: V, parent: Option<usize>) -> usize {
        let i = self.nodes.len();
        self.nodes.push(Node {
            p,
            parent,
            children: vec![],
            r: 0.0,
            tip: 0,
            leafy_along: false,
        });
        match parent {
            Some(q) => self.nodes[q].children.push(i),
            None => self.roots.push(i),
        }
        i
    }
    fn dir(&self, i: usize) -> V {
        match self.nodes[i].parent {
            Some(q) => norm(sub(self.nodes[i].p, self.nodes[q].p)),
            None => [0.0, 0.0, 1.0],
        }
    }
    /// Pipe-model radii (Leonardo's rule: a parent's cross-section carries its children's)
    /// scaled so each root is `root` thick, twigs no thinner than `min`; and steps to tips.
    fn thicken(&mut self, root: f64, min: f64) {
        let n = self.nodes.len();
        // Children always come after their parents.
        for i in (0..n).rev() {
            let kids = self.nodes[i].children.clone();
            if kids.is_empty() {
                self.nodes[i].r = 1.0;
                self.nodes[i].tip = 0;
            } else {
                let s: f64 = kids.iter().map(|&c| self.nodes[c].r.powf(2.4)).sum();
                self.nodes[i].r = s.powf(1.0 / 2.4);
                self.nodes[i].tip = kids.iter().map(|&c| self.nodes[c].tip).min().unwrap_or(0) + 1;
            }
        }
        let top = self
            .roots
            .iter()
            .map(|&r| self.nodes[r].r)
            .fold(1.0, f64::max);
        let k = root / top;
        for nd in &mut self.nodes {
            nd.r = (nd.r * k).max(min);
        }
    }
    /// Smooths kinks: every inner node moves toward the middle of its neighbours.
    fn smooth(&mut self, passes: usize) {
        for _ in 0..passes {
            let next: Vec<V> = (0..self.nodes.len())
                .map(|i| {
                    let nd = &self.nodes[i];
                    match (nd.parent, nd.children.first()) {
                        (Some(q), Some(&c)) => {
                            let mid = lerp(self.nodes[q].p, self.nodes[c].p, 0.5);
                            lerp(nd.p, mid, 0.5)
                        }
                        _ => nd.p,
                    }
                })
                .collect();
            for (nd, p) in self.nodes.iter_mut().zip(next) {
                nd.p = p;
            }
        }
    }
}

/// Grows a crown by space colonization (Runions et al.): attraction points fill the
/// envelope; the nearest branch tip grows a step toward each group of them, and points a
/// branch reaches are used up.
fn colonize(spec: &PlantSpec, env: &Envelope, rng: &mut Rng) -> Skeleton {
    use std::collections::HashMap;
    let h = spec.height;
    let big = h.max(spec.spread);
    // Narrow crowns need finer steps, or the trunk alone uses up their attraction points.
    let step = (big / 42.0).min(spec.spread / 7.0).clamp(45.0, 420.0);
    let reach = step * 7.0;
    let kill = step * 1.7;
    // Attraction points, as many as the crown's volume holds at this scale.
    let crown_h = (env.z1 - env.z0).max(step);
    let volume = std::f64::consts::PI * env.r * env.r * crown_h * 0.55;
    let want = (volume / step.powi(3) / 7.0 * spec.density.max(0.2)).clamp(150.0, 2400.0) as usize;
    let mut points: Vec<V> = vec![];
    let mut tries = 0;
    while points.len() < want && tries < want * 60 {
        tries += 1;
        let p = [
            rng.sym() * env.r * 1.35,
            rng.sym() * env.r * 1.35,
            rng.range(env.z0, env.z1),
        ];
        if env.contains(p) {
            // Favour the outer shell, where leaves are.
            if env.outer(p) < 0.45 && rng.f() < 0.5 {
                continue;
            }
            points.push(p);
        }
    }
    let mut sk = Skeleton::default();
    // Trunks: one, or several leaning apart (multi-stem).
    let stems = spec.stems.max(1) as usize;
    let trunk_top = match spec.form {
        CrownForm::Mound | CrownForm::Rosette | CrownForm::Fountain => step,
        _ => env.z0 + (env.z1 - env.z0) * if spec.group.is_tree() { 0.12 } else { 0.05 },
    };
    let mut tips = vec![];
    for s in 0..stems {
        let a =
            rng.range(0.0, std::f64::consts::TAU) + s as f64 * std::f64::consts::TAU / stems as f64;
        let lean = if stems > 1 {
            rng.range(0.12, 0.35) * if spec.group.is_tree() { 1.0 } else { 1.6 }
        } else {
            rng.range(0.0, 0.04)
        };
        let out = [a.cos(), a.sin(), 0.0];
        let base = mul(out, if stems > 1 { spec.caliper * 0.35 } else { 0.0 });
        let mut prev = sk.push([base[0], base[1], -step * 0.3], None);
        let mut p = sk.nodes[prev].p;
        let mut z = p[2];
        let mut wander = [0.0, 0.0, 0.0];
        while z < trunk_top {
            wander = add(mul(wander, 0.7), mul([rng.sym(), rng.sym(), 0.0], 0.04));
            let d = norm(add(add([0.0, 0.0, 1.0], mul(out, lean)), wander));
            p = add(p, mul(d, step));
            z = p[2];
            prev = sk.push(p, Some(prev));
        }
        tips.push(prev);
    }
    let up = match spec.form {
        CrownForm::Columnar | CrownForm::Upright | CrownForm::Pyramidal | CrownForm::Conical => {
            0.45
        }
        CrownForm::Weeping => -0.25,
        CrownForm::Spreading | CrownForm::Umbrella => 0.05,
        _ => 0.2,
    };
    let cell = |p: V| -> (i64, i64, i64) {
        (
            (p[0] / reach).floor() as i64,
            (p[1] / reach).floor() as i64,
            (p[2] / reach).floor() as i64,
        )
    };
    for _ in 0..260 {
        if points.is_empty() {
            break;
        }
        let mut grid: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
        for (i, nd) in sk.nodes.iter().enumerate() {
            grid.entry(cell(nd.p)).or_default().push(i);
        }
        let mut pull: HashMap<usize, V> = HashMap::new();
        let mut alive = vec![true; points.len()];
        for (k, a) in points.iter().enumerate() {
            let c = cell(*a);
            let mut best: Option<(f64, usize)> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(list) = grid.get(&(c.0 + dx, c.1 + dy, c.2 + dz)) {
                            for &i in list {
                                let d = length(sub(*a, sk.nodes[i].p));
                                if d < reach && best.is_none_or(|b| d < b.0) {
                                    best = Some((d, i));
                                }
                            }
                        }
                    }
                }
            }
            if let Some((d, i)) = best {
                if d < kill {
                    alive[k] = false;
                } else {
                    let e = pull.entry(i).or_insert([0.0; 3]);
                    *e = add(*e, norm(sub(*a, sk.nodes[i].p)));
                }
            }
        }
        let mut k = 0;
        points.retain(|_| {
            k += 1;
            alive[k - 1]
        });
        let mut grew = false;
        let mut order: Vec<(usize, V)> = pull.into_iter().collect();
        order.sort_by_key(|(i, _)| *i);
        for (i, v) in order {
            let d = norm(add(norm(v), [0.0, 0.0, up]));
            let q = add(sk.nodes[i].p, mul(d, step));
            // Don't grow the same way twice.
            if sk.nodes[i]
                .children
                .iter()
                .any(|&c| dot(norm(sub(sk.nodes[c].p, sk.nodes[i].p)), d) > 0.97)
            {
                continue;
            }
            sk.push(q, Some(i));
            grew = true;
        }
        if !grew {
            break;
        }
    }
    // A weeping tree's curtains: long twigs hanging from the upper crown's tips.
    if spec.form == CrownForm::Weeping {
        let tips: Vec<usize> = (0..sk.nodes.len())
            .filter(|&i| sk.nodes[i].children.is_empty() && sk.nodes[i].p[2] > env.z0)
            .collect();
        for i in tips {
            if rng.f() > 0.75 {
                continue;
            }
            let fall = rng.range(0.35, 0.65) * (sk.nodes[i].p[2] - h * 0.05);
            let out = norm([sk.nodes[i].p[0], sk.nodes[i].p[1], 0.0]);
            let mut prev = i;
            let mut p = sk.nodes[i].p;
            let n = (fall / step).ceil().max(2.0) as usize;
            for s in 0..n {
                let t = s as f64 / n as f64;
                let d = norm(add(mul(out, 0.35 * (1.0 - t)), [0.0, 0.0, -1.0]));
                p = add(p, mul(d, fall / n as f64));
                prev = sk.push(p, Some(prev));
                sk.nodes[prev].leafy_along = true;
            }
        }
    }
    let _ = tips;
    sk.smooth(2);
    let min = if spec.group.is_tree() { 3.0 } else { 1.5 };
    sk.thicken(spec.caliper / 2.0 / (stems as f64).sqrt().max(1.0), min);
    sk
}

/// A fir, spruce or cypress: a leader with whorls of branches, drooping with their weight.
fn whorled(spec: &PlantSpec, env: &Envelope, rng: &mut Rng) -> Skeleton {
    let h = spec.height;
    let mut sk = Skeleton::default();
    let step = (h / 40.0).clamp(60.0, 500.0);
    let whorl = (h / 26.0).clamp(90.0, 900.0);
    let droop = match spec.foliage {
        Foliage::Scale => 0.15,
        _ if spec.form == CrownForm::Columnar => 0.1,
        _ => 0.55,
    };
    // The leader.
    let mut prev = sk.push([0.0, 0.0, -step * 0.3], None);
    let mut z = -step * 0.3;
    let mut leader = vec![];
    let lean = [rng.sym() * 0.015, rng.sym() * 0.015, 1.0];
    while z < h * 0.985 {
        let p = add(sk.nodes[prev].p, mul(norm(lean), step));
        z = p[2];
        prev = sk.push(p, Some(prev));
        leader.push(prev);
    }
    let golden = 2.399_963;
    let mut a0 = rng.range(0.0, std::f64::consts::TAU);
    let mut zw = env.z0.max(step * 2.0);
    while zw < h * 0.95 {
        let t = env.t(zw);
        let host = leader
            .iter()
            .copied()
            .min_by(|&a, &b| {
                (sk.nodes[a].p[2] - zw)
                    .abs()
                    .total_cmp(&(sk.nodes[b].p[2] - zw).abs())
            })
            .unwrap_or(prev);
        let count = 4 + (rng.f() * 3.0) as usize;
        for k in 0..count {
            let a = a0 + k as f64 * std::f64::consts::TAU / count as f64 + rng.sym() * 0.3;
            let len = env.radius(zw, a) * rng.range(0.85, 1.02);
            if len < step * 0.8 {
                continue;
            }
            let out = [a.cos(), a.sin(), 0.0];
            // Young top branches reach up; old lower ones sag.
            let pitch = 0.35 - droop * (1.0 - t) * 0.9;
            let mut p = sk.nodes[host].p;
            let mut from = host;
            let n = (len / step).ceil().max(2.0) as usize;
            for s in 0..n {
                let f = s as f64 / n as f64;
                let d = norm(add(out, [0.0, 0.0, pitch - droop * f * 0.8]));
                p = add(p, mul(d, len / n as f64));
                from = sk.push(p, Some(from));
                sk.nodes[from].leafy_along = true;
                // Side shoots every other step.
                if s % 2 == 1 && s + 1 < n {
                    for side in [-1.0, 1.0] {
                        let sd = norm(add(mul(out, 0.6), mul([-out[1], out[0], 0.0], side * 0.8)));
                        let sl = (len * (1.0 - f)) * 0.35;
                        let q = add(p, add(mul(sd, sl), [0.0, 0.0, -sl * droop * 0.3]));
                        let c = sk.push(q, Some(from));
                        sk.nodes[c].leafy_along = true;
                    }
                }
            }
        }
        a0 += golden;
        zw += whorl * rng.range(0.85, 1.15);
    }
    sk.smooth(1);
    sk.thicken(spec.caliper / 2.0, 2.0);
    sk
}

// ---------------------------------------------------------------- mesh parts

/// Indexed triangles with normals, texture coordinates and a colour tint per vertex.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PlantPart {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    /// Linear 0..1 (occlusion and variation baked in).
    pub colors: Vec<f32>,
    pub indices: Vec<u32>,
}

impl PlantPart {
    fn vert(&mut self, p: V, n: V, uv: [f64; 2], c: V) -> u32 {
        let i = (self.positions.len() / 3) as u32;
        self.positions.extend(p.map(|x| x as f32));
        self.normals.extend(norm(n).map(|x| x as f32));
        self.uvs.extend(uv.map(|x| x as f32));
        self.colors.extend(c.map(|x| x as f32));
        i
    }
    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.indices.extend([a, b, c, a, c, d]);
    }
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

/// A species' 3D model, base at the origin (mm, z up).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PlantModel {
    /// Trunk and branches, textured with the bark (u around, v along; 1 = one tile).
    pub bark: PlantPart,
    /// Leaf, needle, frond and flower cards: alpha-cut, textured with the foliage atlas
    /// (2 × 2 cells).
    pub leaves: PlantPart,
    /// Untextured solid parts (grass blades, agave leaves, cacti), coloured per vertex (sRGB
    /// colour times the tint).
    pub solid: PlantPart,
    pub height: f64,
    pub spread: f64,
}

/// The bark's texture repeats every this many mm round and along a trunk.
pub const BARK_TILE: f64 = 600.0;

/// Bark tubes along the skeleton's chains: each branch continues through its thickest
/// child; the others start branches of their own.
fn bark_tubes(sk: &Skeleton, part: &mut PlantPart, env: &Envelope, tint: V) {
    // Chains.
    let mut chains: Vec<Vec<usize>> = vec![];
    let mut starts: Vec<usize> = sk.roots.clone();
    while let Some(s) = starts.pop() {
        let mut chain = vec![];
        if let Some(q) = sk.nodes[s].parent {
            chain.push(q);
        }
        let mut i = s;
        loop {
            chain.push(i);
            let kids = &sk.nodes[i].children;
            if kids.is_empty() {
                break;
            }
            let main = *kids
                .iter()
                .max_by(|&&a, &&b| sk.nodes[a].r.total_cmp(&sk.nodes[b].r))
                .unwrap_or(&kids[0]);
            for &k in kids {
                if k != main {
                    starts.push(k);
                }
            }
            i = main;
        }
        if chain.len() >= 2 {
            chains.push(chain);
        }
    }
    for chain in chains {
        let r0 = sk.nodes[chain[0]].r.max(sk.nodes[chain[1]].r);
        let sides = if r0 > 90.0 {
            12
        } else if r0 > 30.0 {
            8
        } else if r0 > 8.0 {
            5
        } else {
            3
        };
        let around = (std::f64::consts::TAU * r0 / BARK_TILE).round().max(1.0);
        let root = sk.nodes[chain[0]].parent.is_none() && sk.roots.contains(&chain[0]);
        let mut frame = perp(sub(sk.nodes[chain[1]].p, sk.nodes[chain[0]].p));
        let mut along = 0.0;
        let mut prev_ring: Option<u32> = None;
        for (k, &i) in chain.iter().enumerate() {
            let p = sk.nodes[i].p;
            let d = if k + 1 < chain.len() {
                norm(sub(sk.nodes[chain[k + 1]].p, p))
            } else {
                norm(sub(p, sk.nodes[chain[k - 1]].p))
            };
            // Parallel transport keeps the rings from twisting.
            frame = norm(sub(frame, mul(d, dot(frame, d))));
            if length(frame) < 0.5 {
                frame = perp(d);
            }
            let side = cross(d, frame);
            if k > 0 {
                along += length(sub(p, sk.nodes[chain[k - 1]].p));
            }
            let mut r = if k == 0 && !root {
                sk.nodes[chain[1]].r
            } else {
                sk.nodes[i].r
            };
            // The root flare.
            if root && p[2] < env.z1 * 0.08 {
                r *= 1.0 + 0.45 * (1.0 - (p[2].max(0.0) / (env.z1 * 0.08)).min(1.0)).powi(2);
            }
            let shade = 0.55
                + 0.45
                    * (1.0 - env.outer(p) * 0.5).clamp(0.0, 1.0)
                    * if p[2] < env.z0 { 1.0 } else { 0.8 };
            let c = mul(tint, shade);
            let first = (part.positions.len() / 3) as u32;
            for s in 0..=sides {
                let a = s as f64 / sides as f64 * std::f64::consts::TAU;
                let n = add(mul(frame, a.cos()), mul(side, a.sin()));
                part.vert(
                    add(p, mul(n, r)),
                    n,
                    [s as f64 / sides as f64 * around, along / BARK_TILE],
                    c,
                );
            }
            if let Some(pr) = prev_ring {
                for s in 0..sides as u32 {
                    part.quad(pr + s, pr + s + 1, first + s + 1, first + s);
                }
            }
            prev_ring = Some(first);
        }
    }
}

/// The foliage atlas cells: 0 and 1 leaves, 2 and 3 leaves with flowers (or, for palms,
/// 3 a dead frond).
fn cell_uv(cell: u32, u: f64, v: f64) -> [f64; 2] {
    let (cx, cy) = (f64::from(cell % 2), f64::from(cell / 2));
    [
        cx * 0.5 + u.clamp(0.0, 1.0) * 0.5,
        cy * 0.5 + v.clamp(0.0, 1.0) * 0.5,
    ]
}

/// One leaf card: a quad from `base` out along `along`, `w` wide and `l` long, its normal
/// bent toward `soft` (out of the crown), tinted `c`.
#[allow(clippy::too_many_arguments)]
fn card(
    part: &mut PlantPart,
    base: V,
    along: V,
    side: V,
    w: f64,
    l: f64,
    soft: V,
    c: V,
    cell: u32,
) {
    let face = norm(cross(side, along));
    let face = if dot(face, soft) < 0.0 {
        mul(face, -1.0)
    } else {
        face
    };
    let n = norm(add(mul(face, 0.35), mul(soft, 0.65)));
    let hw = mul(side, w / 2.0);
    let top = add(base, mul(along, l));
    let a = part.vert(sub(base, hw), n, cell_uv(cell, 0.0, 0.0), c);
    let b = part.vert(add(base, hw), n, cell_uv(cell, 1.0, 0.0), c);
    let d = part.vert(add(top, hw), n, cell_uv(cell, 1.0, 1.0), c);
    let e = part.vert(sub(top, hw), n, cell_uv(cell, 0.0, 1.0), c);
    part.quad(a, b, d, e);
}

/// The size of a leaf card for the species: clusters of leaves sized to the tree.
fn card_size(spec: &PlantSpec) -> f64 {
    let base = match spec.foliage {
        Foliage::Needle | Foliage::Scale => 1.25,
        Foliage::Small | Foliage::Compound => 0.9,
        _ => 1.0,
    };
    let s = (spec.height.max(spec.spread) / 1000.0).sqrt() * 190.0 * base;
    s.clamp(110.0, 820.0)
}

/// Leaf cards on a skeleton: at the tips and the twigs near them (and along conifer
/// branches and weeping curtains), sized to the crown, facing out of it.
fn leaf_cards(
    spec: &PlantSpec,
    sk: &Skeleton,
    env: &Envelope,
    rng: &mut Rng,
    part: &mut PlantPart,
) {
    let size = card_size(spec);
    let flowers = spec.flowers_now().is_some();
    let flowering_tree = flowers && spec.group == PlantGroup::Flowering;
    let conifer = matches!(spec.foliage, Foliage::Needle | Foliage::Scale);
    let spots: Vec<usize> = (0..sk.nodes.len())
        .filter(|&i| {
            let nd = &sk.nodes[i];
            nd.parent.is_some() && (nd.tip <= 3 || nd.leafy_along) && nd.p[2] > env.z0 * 0.6
        })
        .collect();
    if spots.is_empty() {
        return;
    }
    // Enough cards to clothe the crown's surface about twice over.
    let crown_h = (env.z1 - env.z0).max(size);
    let surface = std::f64::consts::PI * 2.0 * env.r.max(size) * crown_h * 0.9;
    // A narrow conifer's sprays shrink to its column (below), so it needs more of them.
    let each = if conifer {
        size.min(env.r * 0.9).max(size * 0.3)
    } else {
        size
    };
    let want = (surface / (each * each) * 3.3 * spec.density.max(0.25)).clamp(40.0, 6500.0);
    // Conifer sprays and shrubs pack their cards closer: they read as solid masses.
    let want = if conifer {
        want * if spec.foliage == Foliage::Scale {
            2.4
        } else {
            1.8
        }
    } else if !spec.group.is_tree() {
        want * 2.4
    } else {
        want
    };
    let per = want / spots.len() as f64;
    let mut debt = 0.0;
    let center = env.center();
    let stretch = ((env.z1 - env.z0) / (2.0 * env.r.max(1.0))).max(0.5);
    let (c1, _) = (spec.leaf, spec.leaf_alt);
    let _ = c1;
    for i in spots {
        debt += per;
        while debt >= 1.0 || rng.f() < debt {
            debt -= 1.0;
            let nd = &sk.nodes[i];
            let dir = sk.dir(i);
            let p = add(nd.p, mul(rng.unit(), size * 0.25));
            let mut out = sub(p, center);
            out[2] /= stretch;
            let out = norm(out);
            let along = if conifer {
                // Needle sprays lie along the branch, a little flattened.
                norm(add(
                    dir,
                    [rng.sym() * 0.3, rng.sym() * 0.3, rng.sym() * 0.15 - 0.05],
                ))
            } else {
                norm(add(
                    add(mul(dir, 0.45), mul(out, 0.45)),
                    add([0.0, 0.0, -0.15], mul(rng.unit(), 0.45)),
                ))
            };
            let side = if conifer {
                let s = norm(cross(along, [0.0, 0.0, 1.0]));
                norm(add(s, [0.0, 0.0, rng.sym() * 0.3]))
            } else {
                norm(cross(along, rng.unit()))
            };
            let o = env.outer(p);
            // Baked occlusion: darker inside and under the crown.
            let t = env.t(p[2]).clamp(0.0, 1.0);
            let ao = (0.38 + 0.62 * o.min(1.0).powf(0.9)) * (0.82 + 0.18 * t);
            let v = rng.range(0.86, 1.1);
            let warm = rng.sym() * 0.05;
            let c = [
                (ao * v * (1.0 + warm)).min(1.15),
                (ao * v).min(1.15),
                (ao * v * (1.0 - warm)).min(1.15),
            ];
            let cell = if flowering_tree {
                // In bloom: mostly flowers.
                if rng.f() < 0.75 {
                    2 + (rng.f() * 2.0) as u32
                } else {
                    (rng.f() * 2.0) as u32
                }
            } else if flowers && rng.f() < 0.3 {
                2 + (rng.f() * 2.0) as u32
            } else {
                (rng.f() * 2.0) as u32
            };
            let s = size * rng.range(0.8, 1.2);
            // A conifer narrows to its spire: its sprays no wider than the crown there.
            let s = if conifer {
                let rz = env.radius(p[2], p[1].atan2(p[0]));
                s.min((rz * 0.9).max(size * 0.3))
            } else {
                s
            };
            let (w, l) = if conifer { (s * 0.8, s) } else { (s, s) };
            card(
                part,
                sub(p, mul(along, l * 0.15)),
                along,
                side,
                w,
                l,
                out,
                c,
                cell,
            );
        }
    }
}

// ---------------------------------------------------------------- palms and others

/// A tapered strip along a curve (a frond, a grass blade, an agave leaf): `pts` the
/// spine, `width(t)` its half-width, `fold` how much the edges rise (a V-fold), textured
/// along one atlas cell or solid.
#[allow(clippy::too_many_arguments)]
fn strip(
    part: &mut PlantPart,
    pts: &[V],
    width: impl Fn(f64) -> f64,
    fold: f64,
    across: V,
    cell: Option<u32>,
    color: impl Fn(f64) -> V,
) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let mut prev: Option<[u32; 3]> = None;
    for (k, p) in pts.iter().enumerate() {
        let t = k as f64 / (n - 1) as f64;
        let d = if k + 1 < n {
            norm(sub(pts[k + 1], *p))
        } else {
            norm(sub(*p, pts[k - 1]))
        };
        let side = norm(sub(across, mul(d, dot(across, d))));
        let up = norm(cross(side, d));
        let w = width(t);
        let lift = mul(up, w * fold);
        let l = add(add(*p, mul(side, -w)), lift);
        let r = add(add(*p, mul(side, w)), lift);
        let c = color(t);
        let uv = |u: f64| match cell {
            Some(cl) => cell_uv(cl, u, t),
            None => [u, t],
        };
        let nl = norm(add(up, mul(side, -fold)));
        let nr = norm(add(up, mul(side, fold)));
        let ring = [
            part.vert(l, nl, uv(0.0), c),
            part.vert(*p, up, uv(0.5), c),
            part.vert(r, nr, uv(1.0), c),
        ];
        if let Some(pr) = prev {
            part.quad(pr[0], pr[1], ring[1], ring[0]);
            part.quad(pr[1], pr[2], ring[2], ring[1]);
        }
        prev = Some(ring);
    }
}

/// A curve from `from` heading `dir`, `len` long, bending down with `gravity`.
fn arc(from: V, dir: V, len: f64, gravity: f64, n: usize) -> Vec<V> {
    let mut out = vec![from];
    let mut p = from;
    let mut d = norm(dir);
    for _ in 0..n {
        p = add(p, mul(d, len / n as f64));
        out.push(p);
        d = norm(add(d, [0.0, 0.0, -gravity / n as f64]));
    }
    out
}

fn palm(spec: &PlantSpec, rng: &mut Rng, m: &mut PlantModel) {
    let h = spec.height;
    let frond = spec.spread / 2.0 * 1.05;
    let fan = spec.foliage == Foliage::Fan;
    let top_z = (h - frond * 0.35).max(h * 0.3);
    let stems = spec.stems.max(1);
    for _ in 0..stems {
        let a = rng.range(0.0, std::f64::consts::TAU);
        // Coconuts lean and curve; most palms stand nearly straight.
        let lean = if spec.botanical.starts_with("Cocos") {
            0.35
        } else if stems > 1 {
            0.3
        } else {
            rng.range(0.02, 0.07)
        };
        let out = [a.cos(), a.sin(), 0.0];
        let this_top = if stems > 1 {
            top_z * rng.range(0.6, 1.0)
        } else {
            top_z
        };
        let mut sk = Skeleton::default();
        let steps = 14;
        let mut prev = sk.push(
            mul(out, if stems > 1 { spec.caliper * 0.8 } else { 0.0 }),
            None,
        );
        for k in 1..=steps {
            let t = k as f64 / steps as f64;
            let base = sk.nodes[sk.roots[0]].p;
            let bend = lean * this_top * t * t;
            let p = add(base, add(mul(out, bend), [0.0, 0.0, this_top * t]));
            prev = sk.push(p, Some(prev));
        }
        for nd in &mut sk.nodes {
            let t = nd.p[2] / this_top.max(1.0);
            // A swollen base, and a crownshaft a little thicker at the top.
            nd.r = spec.caliper / 2.0 * (1.0 + 0.35 * (-t * 8.0).exp()) * (1.0 - 0.1 * t);
        }
        let env = Envelope {
            form: CrownForm::PalmHead,
            z0: this_top,
            z1: h,
            r: frond,
            lobes: [(0.0, 0.0); 4],
            rough: 0.0,
        };
        bark_tubes(&sk, &mut m.bark, &env, [1.0, 1.0, 1.0]);
        let crown = sk.nodes[prev].p;
        let count = if fan { 20 } else { 18 } + (rng.f() * 6.0) as usize;
        let golden = 2.399_963;
        for k in 0..count {
            let t = k as f64 / count as f64;
            let a = k as f64 * golden + rng.sym() * 0.2;
            let o = [a.cos(), a.sin(), 0.0];
            // Young fronds point up; old ones arch out and down.
            let rise = 1.3 - 1.9 * t + rng.sym() * 0.15;
            let len = frond * (0.75 + 0.35 * (1.0 - (t - 0.4).abs())) * rng.range(0.9, 1.05);
            let tint = {
                let v = 0.8 + 0.25 * (1.0 - t) + rng.sym() * 0.06;
                [v, v, v]
            };
            if fan {
                // A petiole, then a pleated fan facing out and up.
                let petiole = arc(crown, add(o, [0.0, 0.0, rise]), len * 0.55, 0.3, 4);
                strip(
                    &mut m.solid,
                    &petiole,
                    |_| 18.0,
                    0.0,
                    perp(o),
                    None,
                    |_| mul(tint, 0.85),
                );
                let c = *petiole.last().unwrap_or(&crown);
                let face = norm(add(o, [0.0, 0.0, rise.max(-0.2) + 0.6]));
                let r = len * 0.5;
                let u = norm(cross(face, [0.0, 0.0, 1.0]));
                let w = norm(cross(u, face));
                let center = m
                    .leaves
                    .vert(c, face, cell_uv(k as u32 % 2, 0.5, 0.5), tint);
                let segs = 16;
                let mut ring = vec![];
                for s in 0..=segs {
                    let ang = s as f64 / segs as f64 * std::f64::consts::TAU;
                    let (ca, sa) = (ang.cos(), ang.sin());
                    // Cupped: the rim a little forward.
                    let p = add(
                        c,
                        add(add(mul(u, ca * r), mul(w, sa * r)), mul(face, r * 0.18)),
                    );
                    let n = norm(add(face, mul(add(mul(u, ca), mul(w, sa)), -0.25)));
                    ring.push(m.leaves.vert(
                        p,
                        n,
                        cell_uv(k as u32 % 2, 0.5 + ca * 0.5, 0.5 + sa * 0.5),
                        tint,
                    ));
                }
                for s in 0..segs {
                    m.leaves.tri(center, ring[s], ring[s + 1]);
                }
            } else {
                let pts = arc(crown, add(o, [0.0, 0.0, rise]), len, 1.1, 10);
                strip(
                    &mut m.leaves,
                    &pts,
                    |t| len * 0.2 * (std::f64::consts::PI * (0.08 + t * 0.92)).sin().powf(0.6),
                    0.35,
                    perp(o),
                    Some(k as u32 % 2),
                    |_| tint,
                );
            }
        }
        // Old dead fronds hanging under a fan palm's crown (Washingtonia's skirt).
        if fan {
            for k in 0..14 {
                let a = rng.range(0.0, std::f64::consts::TAU);
                let o = [a.cos(), a.sin(), 0.0];
                let base = add(crown, [0.0, 0.0, -frond * 0.08 * rng.f()]);
                let pts = arc(
                    base,
                    add(mul(o, 0.35), [0.0, 0.0, -1.0]),
                    frond * 0.55,
                    0.0,
                    4,
                );
                strip(
                    &mut m.leaves,
                    &pts,
                    |t| frond * 0.14 * (1.0 - t * 0.4),
                    0.2,
                    perp(o),
                    Some(3),
                    |_| [0.8, 0.8, 0.8],
                );
                let _ = k;
            }
        }
    }
}

/// Blades from a clump (ornamental grass, daylily, liriope): solid tapered strips, darker
/// at the base, with seed plumes or flowers at some tips.
fn grass(spec: &PlantSpec, rng: &mut Rng, m: &mut PlantModel) {
    let h = spec.height;
    let r = spec.spread / 2.0;
    let count = ((spec.spread / 5.0) as usize).clamp(90, 420);
    let (lo, hi) = spec.leaf_colors();
    let srgb = |c: [u8; 3]| c.map(|x| f64::from(x) / 255.0);
    let (lo, hi) = (srgb(lo), srgb(hi));
    let flowers = spec.flowers_now();
    for k in 0..count {
        let a = rng.range(0.0, std::f64::consts::TAU);
        let base = [
            a.cos() * r * 0.18 * rng.f().sqrt(),
            a.sin() * r * 0.18 * rng.f().sqrt(),
            0.0,
        ];
        let o = [a.cos(), a.sin(), 0.0];
        let (tilt, grav) = match spec.form {
            CrownForm::Upright => (rng.range(0.05, 0.35), 0.15),
            CrownForm::Mound => (rng.range(0.4, 1.3), 0.9),
            _ => (rng.range(0.2, 1.0), 1.2),
        };
        let len = h
            * rng.range(0.55, 1.0)
            * if spec.form == CrownForm::Mound {
                1.2
            } else {
                1.0
            };
        let pts = arc(base, add(mul(o, tilt), [0.0, 0.0, 1.0]), len, grav, 6);
        let w = (len / 90.0).clamp(2.5, 11.0) * rng.range(0.8, 1.3);
        let mix = rng.f();
        let v = rng.range(0.85, 1.1);
        strip(
            &mut m.solid,
            &pts,
            |t| w * (1.0 - t).powf(0.6),
            0.3,
            perp(o),
            None,
            |t| {
                let c = lerp(lo, hi, mix * 0.6 + t * 0.4);
                mul(c, v * (0.45 + 0.55 * t.powf(0.5)))
            },
        );
        // Plumes above the leaves.
        if flowers.is_some() && k % 4 == 0 {
            let tip = *pts.last().unwrap_or(&base);
            let stalk_top = add(
                base,
                [
                    o[0] * r * 0.2 * tilt,
                    o[1] * r * 0.2 * tilt,
                    h * rng.range(0.95, 1.2),
                ],
            );
            let top = if spec.form == CrownForm::Upright {
                stalk_top
            } else {
                tip
            };
            let s = (h * 0.22).clamp(60.0, 700.0);
            let along = norm(sub(top, base));
            card(
                &mut m.leaves,
                sub(top, mul(along, s * 0.6)),
                along,
                perp(along),
                s * 0.45,
                s,
                norm(add(o, [0.0, 0.0, 0.5])),
                [1.0, 1.0, 1.0],
                2 + (k % 2) as u32,
            );
        }
    }
}

/// A rosette: fleshy tapered leaves (agave, aloe, yucca), fronds (sago) or leaf cards
/// (hosta) radiating from the crown of a short trunk.
fn rosette(spec: &PlantSpec, rng: &mut Rng, m: &mut PlantModel) {
    let h = spec.height;
    let r = spec.spread / 2.0;
    let trunk = spec.trunk.min(h * 0.6);
    if trunk > 50.0 {
        let mut sk = Skeleton::default();
        let a = sk.push([0.0, 0.0, -50.0], None);
        let b = sk.push([0.0, 0.0, trunk * 0.5], Some(a));
        let c = sk.push([rng.sym() * 40.0, rng.sym() * 40.0, trunk], Some(b));
        for i in [a, b, c] {
            sk.nodes[i].r = spec.caliper / 2.0;
        }
        let env = Envelope {
            form: CrownForm::Rosette,
            z0: trunk,
            z1: h,
            r,
            lobes: [(0.0, 0.0); 4],
            rough: 0.0,
        };
        bark_tubes(&sk, &mut m.bark, &env, [1.0, 1.0, 1.0]);
    }
    let center = [0.0, 0.0, trunk];
    let (lo, hi) = spec.leaf_colors();
    let srgb = |c: [u8; 3]| c.map(|x| f64::from(x) / 255.0);
    let (lo, hi) = (srgb(lo), srgb(hi));
    let (count, fleshy) = match spec.foliage {
        Foliage::Fleshy if spec.spread < spec.height * 0.8 => (70, true),
        Foliage::Fleshy => (28, true),
        Foliage::Frond => (22, false),
        _ => (18, false),
    };
    let golden = 2.399_963;
    for k in 0..count {
        let t = k as f64 / count as f64;
        let a = k as f64 * golden;
        let o = [a.cos(), a.sin(), 0.0];
        // Outer leaves splay; the inner ones stand up.
        let rise = 0.25 + 1.8 * t;
        let len = (r * (1.1 - 0.5 * t)).hypot((h - trunk) * t * 0.5) * rng.range(0.9, 1.1);
        match (fleshy, spec.foliage) {
            (true, _) => {
                let pts = arc(center, add(o, [0.0, 0.0, rise]), len, 0.4, 6);
                let w = if count > 40 { len * 0.035 } else { len * 0.1 };
                let v = rng.range(0.9, 1.08);
                strip(
                    &mut m.solid,
                    &pts,
                    |t| w * (1.0 - t).powf(0.8) * (0.6 + 0.4 * (t * 6.0).min(1.0)),
                    0.45,
                    perp(o),
                    None,
                    |tt| mul(lerp(lo, hi, tt * 0.6), v * (0.7 + 0.3 * tt)),
                );
            }
            (false, Foliage::Frond) => {
                let pts = arc(center, add(o, [0.0, 0.0, rise * 0.7]), len, 0.8, 8);
                strip(
                    &mut m.leaves,
                    &pts,
                    |t| len * 0.17 * (std::f64::consts::PI * (0.08 + t * 0.92)).sin().powf(0.6),
                    0.3,
                    perp(o),
                    Some(k as u32 % 2),
                    |_| [0.95, 0.95, 0.95],
                );
            }
            _ => {
                let along = norm(add(o, [0.0, 0.0, 0.6 + 2.0 * t]));
                let s = len.max(h * 0.8) * 0.9;
                card(
                    &mut m.leaves,
                    center,
                    along,
                    norm(cross(along, [0.0, 0.0, 1.0])),
                    s * 0.8,
                    s,
                    norm(add(o, [0.0, 0.0, 0.8])),
                    [0.9, 0.9, 0.9],
                    k as u32 % 2,
                );
            }
        }
    }
    // A flower spike (aloe, yucca).
    if let Some(f) = spec.flowers_now() {
        let _ = f;
        let top = [0.0, 0.0, h * 1.25];
        let along = [0.0, 0.0, 1.0];
        card(
            &mut m.leaves,
            [0.0, 0.0, h * 0.8],
            along,
            [1.0, 0.0, 0.0],
            h * 0.15,
            top[2] - h * 0.8,
            [0.0, -1.0, 0.3],
            [1.0, 1.0, 1.0],
            2,
        );
    }
}

/// A clipped hedge: leaf cards clothing a box (its spread along x, its depth across),
/// facing out, with stems inside.
fn hedge(spec: &PlantSpec, rng: &mut Rng, m: &mut PlantModel) {
    let (lx, ly, h) = (spec.spread / 2.0, spec.depth.max(300.0) / 2.0, spec.height);
    let s = (h.min(ly * 2.0) * 0.28).clamp(110.0, 320.0);
    let area = 2.0 * (lx * 2.0 + ly * 2.0) * h + 4.0 * lx * ly;
    let count = (area / (s * s) * 3.2).clamp(80.0, 6000.0) as usize;
    for _ in 0..count {
        // A point on the box's sides or top, pushed in a little.
        let pick = rng.f() * area;
        let side_a = 2.0 * lx * h;
        let end_a = 2.0 * ly * h;
        let (p, n) = if pick < 2.0 * side_a {
            let sgn = if pick < side_a { 1.0 } else { -1.0 };
            ([rng.sym() * lx, sgn * ly, rng.f() * h], [0.0, sgn, 0.0])
        } else if pick < 2.0 * side_a + 2.0 * end_a {
            let sgn = if rng.f() < 0.5 { 1.0 } else { -1.0 };
            ([sgn * lx, rng.sym() * ly, rng.f() * h], [sgn, 0.0, 0.0])
        } else {
            ([rng.sym() * lx, rng.sym() * ly, h], [0.0, 0.0, 1.0])
        };
        let depth = rng.f().powi(2) * s * 0.9;
        let p = sub(p, mul(n, depth));
        // Soft rounded edges.
        let soft = norm(add(
            n,
            mul([p[0] / lx, p[1] / ly, (p[2] / h - 0.5) * 2.0], 0.35),
        ));
        let along = norm(add(soft, mul(rng.unit(), 0.9)));
        let side = norm(cross(along, rng.unit()));
        let ao = 0.55 + 0.45 * (1.0 - depth / s).clamp(0.0, 1.0) * (0.75 + 0.25 * p[2] / h);
        let v = ao * rng.range(0.9, 1.08);
        card(
            &mut m.leaves,
            sub(p, mul(along, s * 0.4)),
            along,
            side,
            s,
            s,
            soft,
            [v, v, v],
            (rng.f() * 2.0) as u32
                + if spec.flowers_now().is_some() && rng.f() < 0.2 {
                    2
                } else {
                    0
                },
        );
    }
}

/// A columnar cactus with arms (saguaro) or a barrel: ribbed, solid.
fn cactus(spec: &PlantSpec, rng: &mut Rng, m: &mut PlantModel) {
    let (lo, hi) = spec.leaf_colors();
    let srgb = |c: [u8; 3]| c.map(|x| f64::from(x) / 255.0);
    let (lo, hi) = (srgb(lo), srgb(hi));
    let ribs = 16.0;
    let column = |m: &mut PlantModel, pts: &[V], r: f64, round_top: bool| {
        let segs = 32;
        let n = pts.len();
        let mut prev: Option<u32> = None;
        let mut frame = perp(sub(pts[1], pts[0]));
        for (k, p) in pts.iter().enumerate() {
            let d = if k + 1 < n {
                norm(sub(pts[k + 1], *p))
            } else {
                norm(sub(*p, pts[k - 1]))
            };
            frame = norm(sub(frame, mul(d, dot(frame, d))));
            let side = cross(d, frame);
            let t = k as f64 / (n - 1) as f64;
            let taper = if round_top {
                (1.0 - ((t - 0.85).max(0.0) / 0.15).powi(2))
                    .max(0.05)
                    .sqrt()
            } else {
                1.0
            };
            let first = (m.solid.positions.len() / 3) as u32;
            for s in 0..=segs {
                let a = s as f64 / segs as f64 * std::f64::consts::TAU;
                let rib = 1.0 - 0.09 * (0.5 - 0.5 * (a * ribs).cos());
                let nn = add(mul(frame, a.cos()), mul(side, a.sin()));
                let groove = 0.75 + 0.25 * (0.5 + 0.5 * (a * ribs).cos());
                let c = mul(lerp(lo, hi, 0.5 + 0.5 * (a * ribs).cos()), groove);
                m.solid
                    .vert(add(*p, mul(nn, r * rib * taper)), nn, [0.0, 0.0], c);
            }
            if let Some(pr) = prev {
                for s in 0..segs as u32 {
                    m.solid.quad(pr + s, pr + s + 1, first + s + 1, first + s);
                }
            }
            prev = Some(first);
        }
    };
    let h = spec.height;
    let r = spec.caliper / 2.0;
    if h < spec.spread * 1.5 {
        // A barrel: a squat ribbed ball.
        let pts: Vec<V> = (0..=12).map(|k| [0.0, 0.0, h * k as f64 / 12.0]).collect();
        let rr = spec.spread / 2.0;
        let segs = pts.len();
        let _ = segs;
        let ball: Vec<V> = pts;
        column(m, &ball, rr, true);
        // Squash the column into a ball: scale each ring by its height's circle.
        let n = m.solid.positions.len() / 3;
        for i in 0..n {
            let z = f64::from(m.solid.positions[i * 3 + 2]);
            let t = (z / h).clamp(0.0, 1.0);
            let k = (1.0 - (2.0 * t - 1.0).powi(2)).max(0.0).sqrt().max(0.25) * (0.75 + 0.25 * t);
            m.solid.positions[i * 3] *= k as f32;
            m.solid.positions[i * 3 + 1] *= k as f32;
        }
        return;
    }
    let main: Vec<V> = (0..=16)
        .map(|k| [0.0, 0.0, -100.0 + (h + 100.0) * k as f64 / 16.0])
        .collect();
    column(m, &main, r, true);
    let arms = 2 + (rng.f() * 3.0) as usize;
    for k in 0..arms {
        let a = rng.range(0.0, std::f64::consts::TAU) + k as f64 * 2.0;
        let o = [a.cos(), a.sin(), 0.0];
        let z0 = h * rng.range(0.35, 0.6);
        let out = r * rng.range(2.2, 3.0);
        let top = h * rng.range(0.6, 0.85);
        let mut pts = vec![];
        for s in 0..=5 {
            let t = s as f64 / 5.0;
            let ang = t * std::f64::consts::FRAC_PI_2;
            pts.push(add(
                mul(o, r * 0.5 + (out - r * 0.5) * (1.0 - ang.cos())),
                [0.0, 0.0, z0 + out * ang.sin() * 0.8],
            ));
        }
        let elbow = *pts.last().unwrap_or(&[0.0, 0.0, z0]);
        for s in 1..=6 {
            pts.push(add(
                elbow,
                [0.0, 0.0, (top - elbow[2]).max(r) * s as f64 / 6.0],
            ));
        }
        column(m, &pts, r * 0.62, true);
    }
}

/// Arching canes with leaves along them (forsythia, ocotillo, ferns' crowns).
fn canes(spec: &PlantSpec, rng: &mut Rng, m: &mut PlantModel) {
    let mut sk = Skeleton::default();
    let count = (spec.stems.max(8) as usize)
        .max((spec.spread / 250.0) as usize)
        .min(40);
    let h = spec.height;
    let r = spec.spread / 2.0;
    let stiff = spec.foliage == Foliage::Small && spec.density < 0.4;
    for _ in 0..count {
        let a = rng.range(0.0, std::f64::consts::TAU);
        let o = [a.cos(), a.sin(), 0.0];
        let tilt = rng.range(0.1, 0.6) * if stiff { 0.4 } else { 1.0 };
        let len = h.hypot(r * tilt) * rng.range(0.8, 1.1);
        let pts = arc(
            mul(o, rng.f() * r * 0.1),
            add(mul(o, tilt), [0.0, 0.0, 1.0]),
            len,
            if stiff { 0.1 } else { 1.3 },
            10,
        );
        let mut prev = sk.push(pts[0], None);
        for p in &pts[1..] {
            prev = sk.push(*p, Some(prev));
            sk.nodes[prev].leafy_along = true;
        }
    }
    sk.thicken(spec.caliper / 2.0, if stiff { 12.0 } else { 3.0 });
    let env = Envelope {
        form: spec.form,
        z0: 0.0,
        z1: h,
        r,
        lobes: [(0.0, 0.0); 4],
        rough: 0.0,
    };
    bark_tubes(&sk, &mut m.bark, &env, [1.0, 1.0, 1.0]);
    if spec.leafy() {
        leaf_cards(spec, &sk, &env, rng, &mut m.leaves);
    }
}

/// Grows a species' model (variant 0, 1 or 2).
pub fn model(spec: &PlantSpec, variant: u32) -> PlantModel {
    let mut rng = Rng::new(seed_of(spec, variant));
    let mut m = PlantModel {
        bark: PlantPart::default(),
        leaves: PlantPart::default(),
        solid: PlantPart::default(),
        height: spec.height,
        spread: spec.spread,
    };
    match (spec.form, spec.foliage) {
        (CrownForm::PalmHead, _) => palm(spec, &mut rng, &mut m),
        (CrownForm::Box, _) => {
            hedge(spec, &mut rng, &mut m);
        }
        (CrownForm::Cactus, _) => cactus(spec, &mut rng, &mut m),
        (CrownForm::Rosette, _) => rosette(spec, &mut rng, &mut m),
        (_, Foliage::Blade) if !spec.group.is_tree() && spec.group != PlantGroup::Shrub => {
            grass(spec, &mut rng, &mut m)
        }
        (CrownForm::Fountain, f) if f != Foliage::Blade => canes(spec, &mut rng, &mut m),
        _ => {
            let env = Envelope::new(spec, &mut rng);
            let whorls = spec.group == PlantGroup::Conifer
                && matches!(spec.form, CrownForm::Pyramidal | CrownForm::Conical);
            let sk = if whorls {
                whorled(spec, &env, &mut rng)
            } else {
                let mut sk = colonize(spec, &env, &mut rng);
                // Columnar conifers (cypress, arborvitae) are clothed to the ground: foliage
                // all along their branches, not just at the tips.
                if matches!(spec.foliage, Foliage::Needle | Foliage::Scale) {
                    let thin = spec.caliper / 2.0 * 0.4;
                    for nd in &mut sk.nodes {
                        nd.leafy_along |= nd.r < thin;
                    }
                }
                sk
            };
            bark_tubes(&sk, &mut m.bark, &env, [1.0, 1.0, 1.0]);
            if spec.leafy() {
                leaf_cards(spec, &sk, &env, &mut rng, &mut m.leaves);
            }
        }
    }
    m
}

// ---------------------------------------------------------------- proxies

/// Enscape's proxy in the working views: a low-poly trunk and crown (or a hedge's box, a
/// palm's star, a grass tuft) placed and turned like the planting.
pub fn proxy(spec: &PlantSpec, at: [f64; 3], rotation: f64, scale: f64) -> Vec<f32> {
    let mut out: Vec<f32> = vec![];
    let (s, c) = rotation.sin_cos();
    let place = |p: V| -> [f32; 3] {
        let (x, y) = (p[0] * scale, p[1] * scale);
        [
            (at[0] + x * c - y * s) as f32,
            (at[1] + x * s + y * c) as f32,
            (at[2] + p[2] * scale) as f32,
        ]
    };
    let mut tri = |a: V, b: V, d: V| {
        for p in [a, b, d] {
            out.extend(place(p));
        }
    };
    let h = spec.height;
    let r = spec.spread / 2.0;
    let mut rng = Rng::new(seed_of(spec, 0));
    // A lathe of rings: (z, radius) from bottom to top, `n` sides, lumpy by `lump`.
    let lathe = |tri: &mut dyn FnMut(V, V, V), rings: &[(f64, f64)], n: usize, lumps: &[f64]| {
        let pt = |k: usize, s: usize| -> V {
            let (z, rr) = rings[k];
            let a = s as f64 / n as f64 * std::f64::consts::TAU;
            let l = lumps[s % lumps.len()];
            [a.cos() * rr * l, a.sin() * rr * l, z]
        };
        for k in 0..rings.len() - 1 {
            for s in 0..n {
                let (a, b) = (pt(k, s), pt(k, (s + 1) % n));
                let (d, e) = (pt(k + 1, (s + 1) % n), pt(k + 1, s));
                tri(a, b, d);
                tri(a, d, e);
            }
        }
        // Caps.
        let (zb, _) = rings[0];
        let (zt, _) = rings[rings.len() - 1];
        for s in 0..n {
            tri([0.0, 0.0, zb], pt(0, (s + 1) % n), pt(0, s));
            tri(
                [0.0, 0.0, zt],
                pt(rings.len() - 1, s),
                pt(rings.len() - 1, (s + 1) % n),
            );
        }
    };
    let lumps: Vec<f64> = (0..10).map(|_| rng.range(0.88, 1.1)).collect();
    match spec.form {
        CrownForm::Box => {
            let (lx, ly) = (r, spec.depth.max(300.0) / 2.0);
            let corners = studio_geom::box_corners([-lx, 0.0, 0.0], [lx, 0.0, 0.0], ly * 2.0, h);
            let tris = studio_geom::box_triangles(&corners);
            for t in tris.chunks(9) {
                let v = |k: usize| [f64::from(t[k]), f64::from(t[k + 1]), f64::from(t[k + 2])];
                tri(v(0), v(3), v(6));
            }
        }
        CrownForm::PalmHead => {
            let top = h - r * 0.35;
            let tr = spec.caliper / 2.0;
            let rings: Vec<(f64, f64)> = vec![(0.0, tr * 1.3), (top * 0.2, tr), (top, tr * 0.9)];
            lathe(&mut tri, &rings, 6, &[1.0]);
            // Fronds: a star of drooping blades.
            for k in 0..9 {
                let a = k as f64 / 9.0 * std::f64::consts::TAU;
                let (o, q) = ([a.cos(), a.sin(), 0.0], [-a.sin(), a.cos(), 0.0]);
                let c = [0.0, 0.0, top];
                let mid = add(c, add(mul(o, r * 0.6), [0.0, 0.0, r * 0.25]));
                let tip = add(c, add(mul(o, r), [0.0, 0.0, -r * 0.25]));
                let w = r * 0.14;
                tri(c, add(mid, mul(q, w)), sub(mid, mul(q, w)));
                tri(sub(mid, mul(q, w)), add(mid, mul(q, w)), tip);
            }
        }
        _ if matches!(spec.foliage, Foliage::Blade) && !spec.group.is_tree() => {
            // A tuft: an upturned cone.
            let rings = vec![(0.0, r * 0.2), (h * 0.5, r * 0.75), (h, r)];
            lathe(&mut tri, &rings, 8, &lumps);
        }
        _ => {
            let mut e = Envelope::new(spec, &mut rng);
            e.rough = 0.0;
            if spec.group.is_tree() && e.z0 > 1.0 {
                let tr = spec.caliper / 2.0;
                let trunk = vec![(0.0, tr * 1.2), (e.z0 + (h - e.z0) * 0.15, tr * 0.8)];
                lathe(&mut tri, &trunk, 6, &[1.0]);
            }
            let n = 8;
            let mut rings: Vec<(f64, f64)> = (0..=n)
                .map(|k| {
                    let t = k as f64 / n as f64;
                    let z = e.z0 + (e.z1 - e.z0) * t;
                    (z, (r * profile(spec.form, t)).max(r * 0.04))
                })
                .collect();
            if let Some(f) = rings.first_mut() {
                f.1 *= 0.6;
            }
            lathe(&mut tri, &rings, 10, &lumps);
        }
    }
    out
}

/// Proxies for the 3D view: every planting (category Planting), coloured by its leaves.
pub(crate) fn meshes(doc: &Document, out: &mut Vec<Mesh>) {
    for p in studio_core::planting::placed(doc) {
        let (leaf, _) = p.spec.leaf_colors();
        let color = if p.spec.leafy() { leaf } else { p.spec.bark };
        out.push(Mesh {
            el: p.id,
            category: Category::Planting,
            exterior: false,
            color: Some(color),
            material: None,
            level: doc.data(p.id).ok().and_then(|d| d.level()),
            positions: proxy(p.spec, p.at, p.rotation, p.scale),
            edges: vec![],
            glow: None,
        });
    }
}

/// Where each planting is, for drawing the full models (Realistic views and renders).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PlantInstance {
    pub el: ElementId,
    pub type_id: ElementId,
    pub variant: u32,
    /// A hash of its type's spec: a changed type is grown again.
    pub spec_key: String,
    /// The base (project mm, z up).
    pub at: [f64; 3],
    pub rotation: f64,
    pub scale: f64,
}

fn spec_hash(spec: &PlantSpec) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in format!("{spec:?}").bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Every planting's placement; its variant from its id, so it stays put.
pub fn instances(doc: &Document) -> Vec<PlantInstance> {
    studio_core::planting::placed(doc)
        .into_iter()
        .map(|p| {
            let bytes = p.id.to_string();
            let h = bytes
                .bytes()
                .fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(u32::from(b)));
            PlantInstance {
                el: p.id,
                type_id: p.type_id,
                variant: h % VARIANTS,
                spec_key: format!("{:016x}", seed_of(p.spec, 0) ^ spec_hash(p.spec)),
                at: p.at,
                rotation: p.rotation,
                scale: p.scale,
            }
        })
        .collect()
}

// ---------------------------------------------------------------- plan and elevation

/// A canopy outline in plan: a circle of radius `r` about `c`, scalloped (`scallops` arcs
/// bulging out) and lumpy with `rng`.
fn scalloped(c: Pt, r: f64, scallops: usize, rng: &mut Rng) -> Vec<Pt> {
    let mut pts = vec![];
    let per = 6;
    let jitter: Vec<f64> = (0..scallops).map(|_| rng.range(0.9, 1.06)).collect();
    for (k, j) in jitter.iter().enumerate() {
        for s in 0..per {
            let t = s as f64 / per as f64;
            let a = (k as f64 + t) / scallops as f64 * std::f64::consts::TAU;
            let bulge = 1.0 + 0.09 * (std::f64::consts::PI * t).sin();
            let rr = r * j * bulge;
            pts.push(Pt::new(c.x + a.cos() * rr, c.y + a.sin() * rr));
        }
    }
    pts
}

/// Revit's planting symbols in plan, at the plant's size.
pub fn plan_symbol(
    b: &mut Builder,
    el: Option<ElementId>,
    spec: &PlantSpec,
    at: Pt,
    rotation: f64,
    scale: f64,
) {
    let r = spec.spread / 2.0 * scale;
    let mut rng = Rng::new(seed_of(spec, 1));
    let ring = |r: f64, n: usize| -> Vec<Pt> {
        (0..n)
            .map(|k| {
                let a = k as f64 / n as f64 * std::f64::consts::TAU;
                Pt::new(at.x + a.cos() * r, at.y + a.sin() * r)
            })
            .collect()
    };
    let rot = |a: f64| a + rotation;
    match spec.form {
        CrownForm::Box => {
            let (lx, ly) = (r, spec.depth.max(300.0) / 2.0 * scale);
            let (s, c) = rotation.sin_cos();
            let p = |x: f64, y: f64| Pt::new(at.x + x * c - y * s, at.y + x * s + y * c);
            // Scalloped long sides.
            let mut outline = vec![];
            let n = ((lx * 2.0) / (ly * 0.9)).ceil().max(2.0) as usize;
            for k in 0..=n * 4 {
                let t = k as f64 / (n * 4) as f64;
                let bump = (t * n as f64 * std::f64::consts::PI).sin().abs() * ly * 0.12;
                outline.push(p(-lx + 2.0 * lx * t, -ly - bump));
            }
            for k in 0..=n * 4 {
                let t = k as f64 / (n * 4) as f64;
                let bump = (t * n as f64 * std::f64::consts::PI).sin().abs() * ly * 0.12;
                outline.push(p(lx - 2.0 * lx * t, ly + bump));
            }
            b.line(el, &outline, true, 2, Dash::Solid);
        }
        CrownForm::PalmHead => {
            // Fronds: a ring of leaf shapes from the trunk.
            let n = 9;
            for k in 0..n {
                let a = rot(k as f64 / n as f64 * std::f64::consts::TAU);
                let (o, q) = (Pt::new(a.cos(), a.sin()), Pt::new(-a.sin(), a.cos()));
                let pt = |f: f64, s: f64| at.add(o.scale(r * f)).add(q.scale(r * s));
                let leaf = vec![
                    pt(0.08, 0.0),
                    pt(0.45, 0.11),
                    pt(0.8, 0.07),
                    pt(1.0, 0.0),
                    pt(0.8, -0.07),
                    pt(0.45, -0.11),
                ];
                b.line(el, &leaf, true, 1, Dash::Solid);
                b.line(el, &[pt(0.08, 0.0), pt(1.0, 0.0)], false, 1, Dash::Solid);
            }
            b.line(
                el,
                &ring(spec.caliper / 2.0 * scale, 12),
                true,
                2,
                Dash::Solid,
            );
        }
        _ if spec.group == PlantGroup::Conifer
            || matches!(spec.foliage, Foliage::Needle | Foliage::Scale) =>
        {
            // A star: spiky outline with branches to each point.
            let n = 18;
            let mut outline = vec![];
            for k in 0..n * 2 {
                let a = rot(k as f64 / (n * 2) as f64 * std::f64::consts::TAU);
                let rr = if k % 2 == 0 { r } else { r * 0.72 } * rng.range(0.94, 1.04);
                outline.push(Pt::new(at.x + a.cos() * rr, at.y + a.sin() * rr));
            }
            b.line(el, &outline, true, 2, Dash::Solid);
            for k in (0..n * 2).step_by(2) {
                b.line(el, &[at, outline[k]], false, 1, Dash::Solid);
            }
        }
        _ if matches!(spec.foliage, Foliage::Blade | Foliage::Fleshy) => {
            // A tuft or a rosette: short strokes from the centre.
            let n = if spec.foliage == Foliage::Blade {
                16
            } else {
                12
            };
            for k in 0..n {
                let a = rot(k as f64 / n as f64 * std::f64::consts::TAU + rng.sym() * 0.1);
                let rr = r * rng.range(0.75, 1.0);
                b.line(
                    el,
                    &[
                        Pt::new(at.x + a.cos() * r * 0.15, at.y + a.sin() * r * 0.15),
                        Pt::new(at.x + a.cos() * rr, at.y + a.sin() * rr),
                    ],
                    false,
                    1,
                    Dash::Solid,
                );
            }
        }
        _ if !spec.group.is_tree() => {
            // A shrub: a cloud with a centre mark.
            let outline = scalloped(at, r * 0.92, 9, &mut rng);
            b.line(el, &outline, true, 2, Dash::Solid);
            b.line(el, &ring(r * 0.06, 8), true, 1, Dash::Solid);
        }
        _ => {
            // A canopy tree: scalloped crown, branches from the trunk, the trunk.
            let outline = scalloped(at, r * 0.94, 12, &mut rng);
            b.line(el, &outline, true, 2, Dash::Solid);
            let n = 7;
            for k in 0..n {
                let a = rot(k as f64 / n as f64 * std::f64::consts::TAU + rng.sym() * 0.2);
                let (o, q) = (Pt::new(a.cos(), a.sin()), Pt::new(-a.sin(), a.cos()));
                let mid = at.add(o.scale(r * 0.4)).add(q.scale(r * 0.05 * rng.sym()));
                let end = at.add(o.scale(r * rng.range(0.55, 0.7)));
                b.line(el, &[at, mid, end], false, 1, Dash::Solid);
                let twig = mid.add(o.scale(r * 0.15)).add(q.scale(r * 0.12));
                b.line(el, &[mid, twig], false, 1, Dash::Solid);
            }
            b.line(
                el,
                &ring((spec.caliper / 2.0 * scale).max(40.0), 12),
                true,
                2,
                Dash::Solid,
            );
        }
    }
}

/// Plantings in a plan of `level` (a site plan shows every one at grade).
pub(crate) fn plan_symbols(doc: &Document, b: &mut Builder, level: ElementId, site: bool) {
    for e in doc.of(Category::Planting) {
        let ElementData::Planting {
            type_id,
            level: l,
            at,
            rotation,
            scale,
            ..
        } = &e.data
        else {
            continue;
        };
        let here = *l == level || (site && studio_core::planting::at_grade(doc, *l));
        if !here {
            continue;
        }
        if let Some(spec) = studio_core::planting::spec_of(doc, *type_id) {
            plan_symbol(b, Some(e.id), spec, *at, *rotation, *scale);
        }
    }
}

/// A planting's silhouette seen from the side, in (across, up) mm about its base: the
/// crown's outline and the trunk's two sides.
pub fn silhouette(spec: &PlantSpec, scale: f64) -> (Vec<Pt>, Vec<[Pt; 2]>) {
    let mut rng = Rng::new(seed_of(spec, 2));
    let h = spec.height * scale;
    let r = spec.spread / 2.0 * scale;
    let mut lines = vec![];
    match spec.form {
        CrownForm::Box => (
            vec![
                Pt::new(-r, 0.0),
                Pt::new(r, 0.0),
                Pt::new(r, h * 0.94),
                Pt::new(r * 0.96, h),
                Pt::new(-r * 0.96, h),
                Pt::new(-r, h * 0.94),
            ],
            lines,
        ),
        CrownForm::PalmHead => {
            let top = h - r * 0.35;
            let tr = spec.caliper / 2.0 * scale;
            lines.push([Pt::new(-tr, 0.0), Pt::new(-tr * 0.9, top)]);
            lines.push([Pt::new(tr, 0.0), Pt::new(tr * 0.9, top)]);
            // Fronds as arcs out and down.
            let mut outline = vec![Pt::new(-tr, top)];
            for k in 0..=12 {
                let t = k as f64 / 12.0;
                let a = std::f64::consts::PI * (1.0 - t);
                let y = top + r * 0.45 * a.sin() - r * 0.3 * (a.cos().abs()).powi(3);
                outline.push(Pt::new(a.cos() * r, y));
            }
            outline.push(Pt::new(tr, top));
            for k in 0..6 {
                let a = std::f64::consts::PI * (k as f64 + 0.5) / 6.0;
                lines.push([
                    Pt::new(0.0, top),
                    Pt::new(a.cos() * r * 0.9, top + r * 0.3 * a.sin()),
                ]);
            }
            (outline, lines)
        }
        _ => {
            let mut e = Envelope::new(
                &PlantSpec {
                    height: h,
                    spread: r * 2.0,
                    trunk: spec.trunk * scale,
                    ..spec.clone()
                },
                &mut rng,
            );
            e.rough = e.rough.max(0.1);
            let n = 40;
            let mut right = vec![];
            let mut left = vec![];
            for k in 0..=n {
                let t = k as f64 / n as f64;
                let z = e.z0 + (e.z1 - e.z0) * t;
                let wobble = |a: f64| e.lump(a, t) * (1.0 + 0.05 * (t * 37.0 + a).sin());
                let pr = r * profile(spec.form, t) * wobble(0.0);
                let pl = r * profile(spec.form, t) * wobble(std::f64::consts::PI);
                right.push(Pt::new(pr, z));
                left.push(Pt::new(-pl, z));
            }
            left.reverse();
            let mut outline = right;
            outline.extend(left);
            if spec.group.is_tree() && e.z0 > 0.0 {
                let tr = (spec.caliper / 2.0 * scale).max(30.0);
                lines.push([
                    Pt::new(-tr, 0.0),
                    Pt::new(-tr * 0.7, e.z0 + (e.z1 - e.z0) * 0.08),
                ]);
                lines.push([
                    Pt::new(tr, 0.0),
                    Pt::new(tr * 0.7, e.z0 + (e.z1 - e.z0) * 0.08),
                ]);
            }
            (outline, lines)
        }
    }
}

// ---------------------------------------------------------------- ground regions

/// A ground region's areas (its sketch's loops, or its boundary).
fn region_polys(
    doc: &Document,
    boundary: &[Pt],
    sketch: &[Vec<studio_core::sketch::SketchCurve>],
) -> Vec<studio_geom::Poly> {
    let polys = if sketch.is_empty() {
        vec![]
    } else {
        studio_core::sketch::polygons(doc, sketch)
    };
    if polys.is_empty() && boundary.len() >= 3 {
        vec![studio_geom::Poly::simple(boundary.to_vec())]
    } else {
        polys
    }
}

/// A ground region laid over the ground: its area triangulated and split finely enough
/// to follow the topography, a little above it (mm, z up; 9 floats a triangle).
pub fn region_mesh(doc: &Document, id: ElementId) -> Vec<f32> {
    let Ok(ElementData::GroundRegion {
        level,
        boundary,
        sketch,
        ..
    }) = doc.data(id)
    else {
        return vec![];
    };
    let lz = doc.level_elevation(*level).unwrap_or(0.0);
    let draped = studio_core::planting::at_grade(doc, *level);
    let z_at = |p: Pt| {
        let g = if draped {
            studio_core::planting::ground_at(doc, p)
        } else {
            None
        };
        g.unwrap_or(lz) + 30.0
    };
    let fine = draped
        && studio_core::planting::ground_at(doc, boundary.first().copied().unwrap_or_default())
            .is_some();
    let mut out = vec![];
    for poly in region_polys(doc, boundary, sketch) {
        let (pts, tris) = studio_geom::triangulate(&poly);
        let mut stack: Vec<[Pt; 3]> = tris
            .iter()
            .map(|t| [pts[t[0]], pts[t[1]], pts[t[2]]])
            .collect();
        let mut guard = 0;
        while let Some(tri) = stack.pop() {
            guard += 1;
            let longest = (0..3)
                .max_by(|&a, &b| {
                    tri[a]
                        .dist(tri[(a + 1) % 3])
                        .total_cmp(&tri[b].dist(tri[(b + 1) % 3]))
                })
                .unwrap_or(0);
            let (a, b, c) = (tri[longest], tri[(longest + 1) % 3], tri[(longest + 2) % 3]);
            if fine && a.dist(b) > 1500.0 && guard < 200_000 {
                let m = a.lerp(b, 0.5);
                stack.push([a, m, c]);
                stack.push([m, b, c]);
                continue;
            }
            for p in [tri[0], tri[1], tri[2]] {
                out.extend([p.x as f32, p.y as f32, z_at(p) as f32]);
            }
        }
    }
    out
}

/// Triangulates areas and lays them over the ground: split finely (under 1.5 m a side)
/// when `fine`, so they follow the topography, heights from `z_at`.
fn drape(polys: &[studio_geom::Poly], z_at: impl Fn(Pt) -> f64, fine: bool) -> Vec<f32> {
    let mut out = vec![];
    for poly in polys {
        let (pts, tris) = studio_geom::triangulate(poly);
        let mut stack: Vec<[Pt; 3]> = tris
            .iter()
            .map(|t| [pts[t[0]], pts[t[1]], pts[t[2]]])
            .collect();
        let mut guard = 0;
        while let Some(tri) = stack.pop() {
            guard += 1;
            let longest = (0..3)
                .max_by(|&a, &b| {
                    tri[a]
                        .dist(tri[(a + 1) % 3])
                        .total_cmp(&tri[b].dist(tri[(b + 1) % 3]))
                })
                .unwrap_or(0);
            let (a, b, c) = (tri[longest], tri[(longest + 1) % 3], tri[(longest + 2) % 3]);
            if fine && a.dist(b) > 1500.0 && guard < 200_000 {
                let m = a.lerp(b, 0.5);
                stack.push([a, m, c]);
                stack.push([m, b, c]);
                continue;
            }
            for p in tri {
                out.extend([p.x as f32, p.y as f32, z_at(p) as f32]);
            }
        }
    }
    out
}

/// A grass patch's painted area: its dabs' circles merged.
fn patch_polys(dabs: &[studio_core::grass::Dab]) -> Vec<studio_geom::Poly> {
    let circles: Vec<studio_geom::Poly> = studio_core::grass::dab_rings(dabs)
        .into_iter()
        .map(studio_geom::Poly::simple)
        .collect();
    studio_geom::union_all(&circles)
}

/// A grass patch's area laid on what was painted: the topography under a level at grade,
/// else the painted surface's height (the nearest dab's).
pub fn grass_patch_mesh(doc: &Document, id: ElementId) -> Vec<f32> {
    let Ok(ElementData::GrassPatch { level, dabs, .. }) = doc.data(id) else {
        return vec![];
    };
    let draped = studio_core::planting::at_grade(doc, *level);
    let first = dabs
        .first()
        .map(|d| Pt::new(d[0], d[1]))
        .unwrap_or_default();
    let fine = draped && studio_core::planting::ground_at(doc, first).is_some();
    let nearest = |p: Pt| {
        dabs.iter()
            .min_by(|a, b| {
                (a[0] - p.x)
                    .hypot(a[1] - p.y)
                    .total_cmp(&(b[0] - p.x).hypot(b[1] - p.y))
            })
            .map_or(0.0, |d| d[2])
    };
    let z_at = |p: Pt| {
        let g = if draped {
            studio_core::planting::ground_at(doc, p)
        } else {
            None
        };
        g.unwrap_or_else(|| nearest(p)) + 12.0
    };
    drape(&patch_polys(dabs), z_at, fine)
}

/// Painted grass for the 3D view: its area in its colour (Realistic grows blades on it).
pub(crate) fn grass_meshes(doc: &Document, out: &mut Vec<Mesh>) {
    for e in doc.of(Category::GrassPatch) {
        let ElementData::GrassPatch { level, spec, .. } = &e.data else {
            continue;
        };
        out.push(Mesh {
            el: e.id,
            category: Category::GrassPatch,
            exterior: false,
            color: Some(spec.color),
            material: None,
            level: Some(*level),
            positions: grass_patch_mesh(doc, e.id),
            edges: vec![],
            glow: None,
        });
    }
}

/// Painted grass in a plan of its level (a site plan shows that at grade): its outline.
pub(crate) fn plan_grass(doc: &Document, b: &mut Builder, level: ElementId, site: bool) {
    for e in doc.of(Category::GrassPatch) {
        let ElementData::GrassPatch { level: l, dabs, .. } = &e.data else {
            continue;
        };
        if *l != level && !(site && studio_core::planting::at_grade(doc, *l)) {
            continue;
        }
        for poly in patch_polys(dabs) {
            b.fill(
                Some(e.id),
                vec![crate::ring(&poly.outer)],
                crate::FillKind::Room,
            );
            b.line(Some(e.id), &poly.outer, true, 1, Dash::Dashed);
            for h in &poly.holes {
                b.line(Some(e.id), h, true, 1, Dash::Dashed);
            }
        }
    }
}

/// Ground regions for the 3D view, in their materials.
pub(crate) fn region_meshes(doc: &Document, out: &mut Vec<Mesh>) {
    for e in doc.of(Category::GroundRegion) {
        let ElementData::GroundRegion {
            level, material, ..
        } = &e.data
        else {
            continue;
        };
        let color = match doc.data(*material) {
            Ok(ElementData::Material { color, .. }) => Some(*color),
            _ => Some([150, 160, 120]),
        };
        out.push(Mesh {
            el: e.id,
            category: Category::GroundRegion,
            exterior: false,
            color,
            material: Some(*material),
            level: Some(*level),
            positions: region_mesh(doc, e.id),
            edges: vec![],
            glow: None,
        });
    }
}

/// Ground regions' outlines in a plan of their level (a site plan shows those at grade).
pub(crate) fn plan_regions(doc: &Document, b: &mut Builder, level: ElementId, site: bool) {
    for e in doc.of(Category::GroundRegion) {
        let ElementData::GroundRegion {
            level: l,
            boundary,
            sketch,
            ..
        } = &e.data
        else {
            continue;
        };
        if *l != level && !(site && studio_core::planting::at_grade(doc, *l)) {
            continue;
        }
        for poly in region_polys(doc, boundary, sketch) {
            b.fill(
                Some(e.id),
                vec![crate::ring(&poly.outer)],
                crate::FillKind::Room,
            );
            b.line(Some(e.id), &poly.outer, true, 1, Dash::Solid);
            for h in &poly.holes {
                b.line(Some(e.id), h, true, 1, Dash::Solid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::planting::catalog;

    fn preset(name: &str) -> PlantSpec {
        catalog()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .spec
    }

    fn bounds(part: &PlantPart) -> (V, V) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in part.positions.chunks(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(f64::from(p[k]));
                hi[k] = hi[k].max(f64::from(p[k]));
            }
        }
        (lo, hi)
    }

    #[test]
    fn a_tree_fills_its_size_with_a_trunk_branches_and_leaves() {
        let spec = preset("Red Maple");
        let m = model(&spec, 0);
        assert!(m.bark.triangles() > 500, "{}", m.bark.triangles());
        assert!(m.leaves.triangles() > 1000, "{}", m.leaves.triangles());
        let (lo, hi) = bounds(&m.leaves);
        // Leaves reach near the top and out to the spread, not below the clear trunk.
        assert!(
            hi[2] > spec.height * 0.88 && hi[2] < spec.height * 1.12,
            "{hi:?}"
        );
        let width = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        assert!(
            width > spec.spread * 0.75 && width < spec.spread * 1.3,
            "{width}"
        );
        assert!(lo[2] > spec.trunk * 0.5, "{lo:?}");
        // The trunk is its caliper at the base (flared a little) and stands at the origin.
        let (blo, _) = bounds(&m.bark);
        assert!(blo[2] < 0.0);
        let base_r = m
            .bark
            .positions
            .chunks(3)
            .filter(|p| f64::from(p[2]) < blo[2] + 1.0)
            .map(|p| f64::from(p[0]).hypot(f64::from(p[1])))
            .fold(0.0, f64::max);
        assert!(
            base_r > spec.caliper * 0.45 && base_r < spec.caliper * 1.0,
            "{base_r}"
        );
        // Every buffer lines up.
        for part in [&m.bark, &m.leaves] {
            let n = part.positions.len() / 3;
            assert_eq!(part.normals.len(), n * 3);
            assert_eq!(part.uvs.len(), n * 2);
            assert_eq!(part.colors.len(), n * 3);
            assert!(part.indices.iter().all(|&i| (i as usize) < n));
        }
        // Variants differ; the same variant is the same.
        assert_eq!(model(&spec, 0), m);
        assert_ne!(model(&spec, 1).leaves.positions, m.leaves.positions);
    }

    #[test]
    fn inner_leaves_are_darker_than_outer_ones() {
        let m = model(&preset("Sugar Maple"), 0);
        let (mut inner, mut outer) = ((0.0, 0), (0.0, 0));
        for (p, c) in m.leaves.positions.chunks(3).zip(m.leaves.colors.chunks(3)) {
            let r = f64::from(p[0]).hypot(f64::from(p[1]));
            if r < 1500.0 {
                inner = (inner.0 + f64::from(c[1]), inner.1 + 1);
            } else if r > 5000.0 {
                outer = (outer.0 + f64::from(c[1]), outer.1 + 1);
            }
        }
        assert!(inner.1 > 0 && outer.1 > 0);
        assert!(inner.0 / f64::from(inner.1) < outer.0 / f64::from(outer.1) * 0.85);
    }

    #[test]
    fn winter_trees_are_bare_and_every_species_grows() {
        let bare = model(&preset("Red Maple (Winter)"), 0);
        assert_eq!(bare.leaves.triangles(), 0);
        assert!(bare.bark.triangles() > 500);
        for p in catalog().iter().filter(|p| p.name == p.species) {
            let m = model(&p.spec, 0);
            let n = m.bark.triangles() + m.leaves.triangles() + m.solid.triangles();
            assert!(n > 20, "{}: {n}", p.name);
            assert!(n < 120_000, "{}: {n}", p.name);
            let all: Vec<f32> = [&m.bark, &m.leaves, &m.solid]
                .iter()
                .flat_map(|x| x.positions.iter().copied())
                .collect();
            assert!(all.iter().all(|v| v.is_finite()), "{}", p.name);
            let top = all.chunks(3).map(|q| f64::from(q[2])).fold(0.0, f64::max);
            assert!(
                top > p.spec.height * 0.6 && top < p.spec.height * 1.4 + 300.0,
                "{}: {top}",
                p.name
            );
        }
    }

    #[test]
    fn conifers_are_whorled_cones_and_palms_have_fronds_on_a_tall_trunk() {
        let spruce = model(&preset("Colorado Blue Spruce"), 0);
        let (lo, hi) = bounds(&spruce.leaves);
        // Wide at the bottom, narrow at the top.
        let width_at = |z0: f64, z1: f64| {
            spruce
                .leaves
                .positions
                .chunks(3)
                .filter(|p| f64::from(p[2]) >= z0 && f64::from(p[2]) < z1)
                .map(|p| f64::from(p[0]).hypot(f64::from(p[1])))
                .fold(0.0, f64::max)
        };
        let h = hi[2];
        assert!(width_at(h * 0.15, h * 0.3) > width_at(h * 0.75, h * 0.9) * 1.8);
        assert!(lo[2] < h * 0.15);
        let palm = model(&preset("Mexican Fan Palm"), 0);
        let (plo, _) = bounds(&palm.leaves);
        assert!(plo[2] > palm.height * 0.6, "{plo:?}");
        assert!(palm.bark.triangles() > 100);
    }

    #[test]
    fn proxies_and_symbols_follow_the_plant() {
        let spec = preset("White Oak");
        let tris = proxy(&spec, [1000.0, 2000.0, 300.0], 0.5, 1.0);
        assert!(!tris.is_empty() && tris.len().is_multiple_of(9));
        let zmax = tris.chunks(3).map(|p| f64::from(p[2])).fold(0.0, f64::max);
        assert!(
            (zmax - (300.0 + spec.height)).abs() < spec.height * 0.05,
            "{zmax}"
        );
        let mut b = Builder::new(96.0);
        plan_symbol(&mut b, None, &spec, Pt::new(0.0, 0.0), 0.0, 1.0);
        assert!(b.items.len() > 5);
        let (outline, trunk) = silhouette(&spec, 1.0);
        let top = outline.iter().map(|p| p.y).fold(0.0, f64::max);
        let wide = outline.iter().map(|p| p.x.abs()).fold(0.0, f64::max);
        assert!((top - spec.height).abs() < spec.height * 0.05);
        assert!(wide > spec.spread * 0.4 && wide < spec.spread * 0.65);
        assert_eq!(trunk.len(), 2);
    }
}
