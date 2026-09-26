//! IFC import (ADR-035): opens an IFC2x3 or IFC4 model (from Revit, ArchiCAD or Studio's own
//! export) as a Studio project of editable elements: storeys become levels, walls come in
//! with their thickness, height and type, doors and windows in their host walls, slabs as
//! floors or flat roofs, spaces as rooms, and grids.
//!
//! Geometry is read from each product's placement and its 'Axis' and 'Body' representations
//! (extrusions, directly or through boolean clippings and mapped items); anything else is
//! placed from the points it's made of. Curved walls come in straight.

use std::collections::{BTreeMap, HashMap};

use studio_core::doors::LeafStyle;
use studio_core::units::MM_PER_IN;
use studio_core::windows::{FrameFinish, Grille};
use studio_core::{
    ops, Category, CoreResult, Document, DoorFamily, ElementData, ElementId, LayerFunction,
    WallFunction, WallLayer, WallTop, WindowFamily,
};
use studio_geom::Pt;

use crate::step::{self, Entity, StepFile, V};

/// What came in, and what didn't.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct ImportReport {
    pub schema: String,
    /// The authoring tool, from the file header.
    pub application: String,
    pub project: String,
    pub levels: usize,
    pub walls: usize,
    pub doors: usize,
    pub windows: usize,
    pub floors: usize,
    pub roofs: usize,
    pub rooms: usize,
    pub grids: usize,
    pub columns: usize,
    /// Kinds of product not brought in, with counts (stairs, curtain walls, furniture…).
    pub skipped: Vec<(String, usize)>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------------------

type V3 = [f64; 3];

/// A rigid placement: origin and axes (x, y, z) as columns.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Tf {
    o: V3,
    x: V3,
    y: V3,
    z: V3,
}

const ID: Tf = Tf {
    o: [0.0; 3],
    x: [1.0, 0.0, 0.0],
    y: [0.0, 1.0, 0.0],
    z: [0.0, 0.0, 1.0],
};

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l < 1e-12 {
        a
    } else {
        scale(a, 1.0 / l)
    }
}

impl Tf {
    fn point(&self, p: V3) -> V3 {
        add(
            self.o,
            add(
                add(scale(self.x, p[0]), scale(self.y, p[1])),
                scale(self.z, p[2]),
            ),
        )
    }
    fn dir(&self, d: V3) -> V3 {
        add(
            add(scale(self.x, d[0]), scale(self.y, d[1])),
            scale(self.z, d[2]),
        )
    }
    /// self ∘ inner: inner's coordinates expressed in self's parent.
    fn then(&self, inner: &Tf) -> Tf {
        Tf {
            o: self.point(inner.o),
            x: self.dir(inner.x),
            y: self.dir(inner.y),
            z: self.dir(inner.z),
        }
    }
}

struct Reader<'a> {
    f: &'a StepFile,
    /// Model units to mm.
    unit: f64,
    /// Plane angle units to radians.
    angle: f64,
    placements: std::cell::RefCell<HashMap<u64, Tf>>,
}

/// A body's extent: its footprint (world plan points, mm) and z range.
#[derive(Debug, Clone, Default)]
struct Body {
    /// Outlines of its vertical extrusions, world mm.
    outlines: Vec<Vec<Pt>>,
    /// Every point reached (for extents), world mm.
    points: Vec<V3>,
    z0: f64,
    z1: f64,
}

impl Body {
    fn extend(&mut self, p: V3) {
        if self.points.is_empty() {
            self.z0 = p[2];
            self.z1 = p[2];
        }
        self.z0 = self.z0.min(p[2]);
        self.z1 = self.z1.max(p[2]);
        self.points.push(p);
    }
    fn plan(&self) -> Vec<Pt> {
        self.points.iter().map(|p| Pt::new(p[0], p[1])).collect()
    }
}

impl Reader<'_> {
    fn e(&self, v: &V) -> Option<&Entity> {
        self.f.at(v)
    }
    fn v3(&self, v: &V) -> Option<V3> {
        let e = self.e(v)?;
        let c = e.arg(0).list();
        Some([
            c.first()?.num()?,
            c.get(1).and_then(V::num).unwrap_or(0.0),
            c.get(2).and_then(V::num).unwrap_or(0.0),
        ])
    }
    /// A point in mm.
    fn point(&self, v: &V) -> Option<V3> {
        self.v3(v).map(|p| scale(p, self.unit))
    }
    fn direction(&self, v: &V) -> Option<V3> {
        self.v3(v).map(unit)
    }
    fn axis2(&self, v: &V) -> Tf {
        let Some(e) = self.e(v) else { return ID };
        let o = self.point(e.arg(0)).unwrap_or([0.0; 3]);
        match e.name.as_str() {
            "IFCAXIS2PLACEMENT2D" => {
                let x = self.direction(e.arg(1)).unwrap_or([1.0, 0.0, 0.0]);
                let x = unit([x[0], x[1], 0.0]);
                Tf {
                    o,
                    x,
                    y: [-x[1], x[0], 0.0],
                    z: [0.0, 0.0, 1.0],
                }
            }
            _ => {
                let z = self.direction(e.arg(1)).unwrap_or([0.0, 0.0, 1.0]);
                let xr = self.direction(e.arg(2)).unwrap_or(if z[2].abs() > 0.9 {
                    [1.0, 0.0, 0.0]
                } else {
                    [0.0, 0.0, 1.0]
                });
                // Orthonormalise x against z.
                let x = unit(add(xr, scale(z, -dot(xr, z))));
                Tf {
                    o,
                    x,
                    y: cross(z, x),
                    z,
                }
            }
        }
    }
    /// An object placement in world mm.
    fn placement(&self, v: &V) -> Tf {
        let Some(id) = v.r() else { return ID };
        if let Some(t) = self.placements.borrow().get(&id) {
            return *t;
        }
        let t = match self.f.get(id) {
            Some(e) if e.name == "IFCLOCALPLACEMENT" => {
                let parent = if e.arg(0).r().is_some() {
                    self.placement(e.arg(0))
                } else {
                    ID
                };
                parent.then(&self.axis2(e.arg(1)))
            }
            _ => ID,
        };
        self.placements.borrow_mut().insert(id, t);
        t
    }
    /// A curve's points, local mm (arcs sampled).
    fn curve(&self, v: &V) -> Vec<V3> {
        let Some(e) = self.e(v) else { return vec![] };
        match e.name.as_str() {
            "IFCPOLYLINE" => e
                .arg(0)
                .list()
                .iter()
                .filter_map(|p| self.point(p))
                .collect(),
            "IFCINDEXEDPOLYCURVE" => {
                let Some(list) = self.e(e.arg(0)) else {
                    return vec![];
                };
                list.arg(0)
                    .list()
                    .iter()
                    .map(|c| {
                        let c = c.list();
                        scale(
                            [
                                c.first().and_then(V::num).unwrap_or(0.0),
                                c.get(1).and_then(V::num).unwrap_or(0.0),
                                c.get(2).and_then(V::num).unwrap_or(0.0),
                            ],
                            self.unit,
                        )
                    })
                    .collect()
            }
            "IFCCOMPOSITECURVE" => {
                let mut out: Vec<V3> = vec![];
                for seg in e.arg(0).list() {
                    let Some(s) = self.e(seg) else { continue };
                    let pts = self.curve(s.arg(2));
                    let pts: Vec<V3> = if s.arg(1).enumv() == Some("F") {
                        pts.into_iter().rev().collect()
                    } else {
                        pts
                    };
                    for p in pts {
                        if out.last().is_none_or(|q| {
                            dot(add(*q, scale(p, -1.0)), add(*q, scale(p, -1.0))) > 1e-6
                        }) {
                            out.push(p);
                        }
                    }
                }
                out
            }
            "IFCTRIMMEDCURVE" => self.trimmed(e),
            "IFCLINE" => {
                let p = self.point(e.arg(0)).unwrap_or([0.0; 3]);
                vec![p]
            }
            _ => vec![],
        }
    }
    fn trimmed(&self, e: &Entity) -> Vec<V3> {
        let trim_point = |t: &V| t.list().iter().find_map(|x| self.point(x));
        let trim_param = |t: &V| {
            t.list().iter().find_map(|x| match x {
                V::Typed(n, a) if n == "IFCPARAMETERVALUE" => a.first()?.num(),
                _ => None,
            })
        };
        let (a, b) = (e.arg(1), e.arg(2));
        let sense = e.arg(3).enumv() != Some("F");
        let Some(basis) = self.e(e.arg(0)) else {
            return vec![];
        };
        match basis.name.as_str() {
            "IFCCIRCLE" => {
                let tf = self.axis2(basis.arg(0));
                let r = basis.arg(1).num().unwrap_or(0.0) * self.unit;
                let angle_of = |t: &V| -> Option<f64> {
                    if let Some(p) = trim_point(t) {
                        let d = add(p, scale(tf.o, -1.0));
                        return Some(dot(d, tf.y).atan2(dot(d, tf.x)));
                    }
                    trim_param(t).map(|x| x * self.angle)
                };
                let (Some(mut t0), Some(mut t1)) = (angle_of(a), angle_of(b)) else {
                    return vec![];
                };
                if !sense {
                    std::mem::swap(&mut t0, &mut t1);
                }
                while t1 < t0 {
                    t1 += std::f64::consts::TAU;
                }
                let n = (((t1 - t0) / 0.2).ceil() as usize).clamp(2, 64);
                let mut pts: Vec<V3> = (0..=n)
                    .map(|i| {
                        let t = t0 + (t1 - t0) * i as f64 / n as f64;
                        tf.point([r * t.cos(), r * t.sin(), 0.0])
                    })
                    .collect();
                if !sense {
                    pts.reverse();
                }
                pts
            }
            _ => {
                // A line: its trim points (or the base point and parameters).
                match (trim_point(a), trim_point(b)) {
                    (Some(p), Some(q)) => {
                        if sense {
                            vec![p, q]
                        } else {
                            vec![q, p]
                        }
                    }
                    _ => {
                        let p0 = self.point(basis.arg(0)).unwrap_or([0.0; 3]);
                        let dir = self
                            .e(basis.arg(1))
                            .map(|vec| {
                                let d = self.direction(vec.arg(0)).unwrap_or([1.0, 0.0, 0.0]);
                                scale(d, vec.arg(1).num().unwrap_or(1.0) * self.unit)
                            })
                            .unwrap_or([self.unit, 0.0, 0.0]);
                        let (ta, tb) = (trim_param(a).unwrap_or(0.0), trim_param(b).unwrap_or(1.0));
                        vec![add(p0, scale(dir, ta)), add(p0, scale(dir, tb))]
                    }
                }
            }
        }
    }
    /// A profile's outer loop, local mm.
    fn profile(&self, v: &V) -> Vec<V3> {
        let Some(e) = self.e(v) else { return vec![] };
        match e.name.as_str() {
            "IFCRECTANGLEPROFILEDEF"
            | "IFCRECTANGLEHOLLOWPROFILEDEF"
            | "IFCROUNDEDRECTANGLEPROFILEDEF" => {
                let tf = self.axis2(e.arg(2));
                let (w, h) = (
                    e.arg(3).num().unwrap_or(0.0) * self.unit / 2.0,
                    e.arg(4).num().unwrap_or(0.0) * self.unit / 2.0,
                );
                [[-w, -h], [w, -h], [w, h], [-w, h]]
                    .iter()
                    .map(|c| tf.point([c[0], c[1], 0.0]))
                    .collect()
            }
            "IFCCIRCLEPROFILEDEF" | "IFCCIRCLEHOLLOWPROFILEDEF" => {
                let tf = self.axis2(e.arg(2));
                let r = e.arg(3).num().unwrap_or(0.0) * self.unit;
                (0..16)
                    .map(|i| {
                        let t = std::f64::consts::TAU * f64::from(i) / 16.0;
                        tf.point([r * t.cos(), r * t.sin(), 0.0])
                    })
                    .collect()
            }
            "IFCARBITRARYCLOSEDPROFILEDEF" | "IFCARBITRARYPROFILEDEFWITHVOIDS" => {
                let mut pts = self.curve(e.arg(2));
                if pts.len() > 1 && pts.first() == pts.last() {
                    pts.pop();
                }
                pts
            }
            // I, L, T, C shapes and the rest: their bounding rectangle is enough to place them.
            _ => {
                let tf = self.axis2(e.arg(2));
                let w = e.arg(3).num().unwrap_or(0.0) * self.unit / 2.0;
                let h = e.arg(4).num().unwrap_or(0.0) * self.unit / 2.0;
                [[-w, -h], [w, -h], [w, h], [-w, h]]
                    .iter()
                    .map(|c| tf.point([c[0], c[1], 0.0]))
                    .collect()
            }
        }
    }
    /// Collects a representation item's geometry into `body` under `tf`.
    fn item(&self, v: &V, tf: &Tf, body: &mut Body, depth: usize) {
        let Some(e) = self.e(v) else { return };
        if depth > 8 {
            return;
        }
        match e.name.as_str() {
            "IFCEXTRUDEDAREASOLID" => {
                let pos = tf.then(&self.axis2(e.arg(1)));
                let d = self.direction(e.arg(2)).unwrap_or([0.0, 0.0, 1.0]);
                let depth_mm = e.arg(3).num().unwrap_or(0.0) * self.unit;
                let prof = self.profile(e.arg(0));
                let dw = pos.dir(scale(d, depth_mm));
                let base: Vec<V3> = prof.iter().map(|p| pos.point(*p)).collect();
                for p in &base {
                    body.extend(*p);
                    body.extend(add(*p, dw));
                }
                // A vertical extrusion's footprint.
                if dw[2].abs() > 1.0
                    && (dw[0].abs() + dw[1].abs()) < dw[2].abs() * 0.01
                    && base.len() >= 3
                {
                    body.outlines
                        .push(base.iter().map(|p| Pt::new(p[0], p[1])).collect());
                }
            }
            "IFCBOOLEANCLIPPINGRESULT" | "IFCBOOLEANRESULT" => {
                self.item(e.arg(1), tf, body, depth + 1);
            }
            "IFCMAPPEDITEM" => {
                let Some(map) = self.e(e.arg(0)) else { return };
                let origin = self.axis2(map.arg(0));
                let target = self.operator(e.arg(1));
                let t = tf.then(&target).then(&origin);
                if let Some(rep) = self.e(map.arg(1)) {
                    for it in rep.arg(3).list() {
                        self.item(it, &t, body, depth + 1);
                    }
                }
            }
            _ => {
                // Breps, tessellations and the rest: every point they use.
                let mut stack = vec![v.clone()];
                let mut seen = std::collections::HashSet::new();
                while let Some(x) = stack.pop() {
                    let Some(id) = x.r() else { continue };
                    if !seen.insert(id) || seen.len() > 50_000 {
                        continue;
                    }
                    let Some(en) = self.f.get(id) else { continue };
                    match en.name.as_str() {
                        "IFCCARTESIANPOINT" => {
                            if let Some(p) = self.point(&x) {
                                body.extend(tf.point(p));
                            }
                        }
                        "IFCCARTESIANPOINTLIST3D" => {
                            for c in en.arg(0).list() {
                                let c = c.list();
                                let p = scale(
                                    [
                                        c.first().and_then(V::num).unwrap_or(0.0),
                                        c.get(1).and_then(V::num).unwrap_or(0.0),
                                        c.get(2).and_then(V::num).unwrap_or(0.0),
                                    ],
                                    self.unit,
                                );
                                body.extend(tf.point(p));
                            }
                        }
                        _ => {
                            for a in &en.args {
                                push_refs(a, &mut stack);
                            }
                        }
                    }
                }
            }
        }
    }
    fn operator(&self, v: &V) -> Tf {
        let Some(e) = self.e(v) else { return ID };
        // IfcCartesianTransformationOperator3D(Axis1, Axis2, LocalOrigin, Scale, Axis3).
        let x = self.direction(e.arg(0)).unwrap_or([1.0, 0.0, 0.0]);
        let yv = self.direction(e.arg(1));
        let z = self.direction(e.arg(4)).unwrap_or([0.0, 0.0, 1.0]);
        let o = self.point(e.arg(2)).unwrap_or([0.0; 3]);
        let y = yv.unwrap_or_else(|| cross(z, x));
        Tf { o, x, y, z }
    }
    /// A product's world placement and representations by identifier ('Axis', 'Body'…).
    fn shape(&self, e: &Entity, placement: usize, rep: usize) -> (Tf, Vec<(String, Vec<V>)>) {
        let tf = self.placement(e.arg(placement));
        let reps = self
            .e(e.arg(rep))
            .map(|pds| {
                pds.arg(2)
                    .list()
                    .iter()
                    .filter_map(|r| {
                        let r = self.e(r)?;
                        Some((
                            r.arg(1).str().unwrap_or("").to_owned(),
                            r.arg(3).list().to_vec(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        (tf, reps)
    }
    fn body(&self, e: &Entity) -> Body {
        let (tf, reps) = self.shape(e, 5, 6);
        let mut body = Body::default();
        for (name, items) in &reps {
            if name == "Body" || name.is_empty() {
                for it in items {
                    self.item(it, &tf, &mut body, 0);
                }
            }
        }
        body
    }
    fn axis(&self, e: &Entity) -> Option<(Pt, Pt)> {
        let (tf, reps) = self.shape(e, 5, 6);
        let (_, items) = reps.iter().find(|(n, _)| n == "Axis")?;
        let pts: Vec<V3> = items.iter().flat_map(|it| self.curve(it)).collect();
        let (a, b) = (pts.first()?, pts.last()?);
        let (a, b) = (tf.point(*a), tf.point(*b));
        Some((Pt::new(a[0], a[1]), Pt::new(b[0], b[1])))
    }
}

fn push_refs(v: &V, stack: &mut Vec<V>) {
    match v {
        V::Ref(_) => stack.push(v.clone()),
        V::List(l) | V::Typed(_, l) => {
            for x in l {
                push_refs(x, stack);
            }
        }
        _ => {}
    }
}

/// Model length units to mm, and plane angle units to radians.
fn units(f: &StepFile) -> (f64, f64) {
    let mut length = 1000.0;
    let mut angle = 1.0;
    let si = |e: &Entity| -> f64 {
        let prefix = match e.arg(2).enumv() {
            Some("MILLI") => 1e-3,
            Some("CENTI") => 1e-2,
            Some("DECI") => 1e-1,
            Some("KILO") => 1e3,
            _ => 1.0,
        };
        prefix
    };
    for id in f.all("IFCUNITASSIGNMENT") {
        let Some(ua) = f.get(id) else { continue };
        for u in ua.arg(0).list() {
            let Some(e) = f.at(u) else { continue };
            let kind = match e.name.as_str() {
                "IFCSIUNIT" => e.arg(1).enumv(),
                "IFCCONVERSIONBASEDUNIT" => e.arg(1).enumv(),
                _ => None,
            };
            match (e.name.as_str(), kind) {
                ("IFCSIUNIT", Some("LENGTHUNIT")) => length = si(e) * 1000.0,
                ("IFCSIUNIT", Some("PLANEANGLEUNIT")) => angle = 1.0,
                ("IFCCONVERSIONBASEDUNIT", Some("LENGTHUNIT")) => {
                    // FOOT and INCH (or any conversion to metres).
                    let factor = f
                        .at(e.arg(3))
                        .map(|m| {
                            let k = m.arg(0).num().unwrap_or(1.0);
                            let base = f.at(m.arg(1)).map_or(1.0, si);
                            k * base * 1000.0
                        })
                        .unwrap_or(304.8);
                    length = factor;
                }
                ("IFCCONVERSIONBASEDUNIT", Some("PLANEANGLEUNIT")) => {
                    angle = f
                        .at(e.arg(3))
                        .and_then(|m| m.arg(0).num())
                        .unwrap_or(std::f64::consts::PI / 180.0);
                }
                _ => {}
            }
        }
    }
    (length, angle)
}

// ---------------------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------------------

/// The line through a footprint's long direction: (start, end, thickness), where `dir`
/// (if given) sets the direction.
fn centerline(pts: &[Pt], dir: Option<Pt>) -> Option<(Pt, Pt, f64)> {
    if pts.len() < 2 {
        return None;
    }
    let d = match dir {
        Some(d) if d.len() > 1e-6 => d.norm(),
        _ => {
            // The longest edge of the outline's convex hull.
            let hull = studio_geom::convex_hull(pts);
            let n = hull.len();
            let mut best = (0.0, Pt::new(1.0, 0.0));
            for i in 0..n {
                let e = hull[(i + 1) % n].sub(hull[i]);
                if e.len() > best.0 {
                    best = (e.len(), e.norm());
                }
            }
            best.1
        }
    };
    let nrm = d.perp();
    let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in pts {
        let (a, b) = (p.dot(d), p.dot(nrm));
        a0 = a0.min(a);
        a1 = a1.max(a);
        b0 = b0.min(b);
        b1 = b1.max(b);
    }
    let mid = (b0 + b1) / 2.0;
    let at = |a: f64| d.scale(a).add(nrm.scale(mid));
    Some((at(a0), at(a1), b1 - b0))
}

/// The type part of a Revit name ("Basic Wall:Exterior - Brick:12345" → "Exterior - Brick").
fn type_part(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.len() {
        0 | 1 => name.trim().to_owned(),
        _ => parts[1].trim().to_owned(),
    }
}

/// The family part ("M_Single-Flush:…" → "M_Single-Flush").
fn family_part(name: &str) -> String {
    name.split(':').next().unwrap_or("").trim().to_owned()
}

/// Imports an IFC file's text as a new project.
pub fn import(text: &str) -> Result<(Document, ImportReport), String> {
    let f = step::parse(text)?;
    let (unit, angle) = units(&f);
    let r = Reader {
        f: &f,
        unit,
        angle,
        placements: Default::default(),
    };
    let mut report = ImportReport {
        schema: f.schema.clone(),
        application: text
            .find("FILE_NAME")
            .and_then(|i| {
                let line = &text[i..text[i..].find(';').map_or(text.len(), |e| i + e)];
                line.split('\'').nth(9).map(str::to_owned)
            })
            .unwrap_or_default(),
        ..Default::default()
    };
    let mut doc = Document::new();
    ops::seed_default_project(&mut doc).map_err(|e| e.to_string())?;
    build(&r, &mut doc, &mut report).map_err(|e| e.to_string())?;
    ops::ensure_tags(&mut doc).map_err(|e| e.to_string())?;
    doc.clear_history();
    doc.mark_saved();
    Ok((doc, report))
}

fn build(r: &Reader<'_>, doc: &mut Document, report: &mut ImportReport) -> CoreResult<()> {
    let f = r.f;
    if let Some(p) = f.all("IFCPROJECT").first().and_then(|id| f.get(*id)) {
        report.project = p.arg(7).str().or(p.arg(2).str()).unwrap_or("").to_owned();
        if report.project.is_empty() {
            report.project = p.arg(2).str().unwrap_or("").to_owned();
        }
    }

    // Containment: element → storey.
    let mut storey_of: HashMap<u64, u64> = HashMap::new();
    for id in f.all("IFCRELCONTAINEDINSPATIALSTRUCTURE") {
        let Some(rel) = f.get(id) else { continue };
        let Some(s) = rel.arg(5).r() else { continue };
        for e in rel.arg(4).refs() {
            storey_of.insert(e, s);
        }
    }
    // Aggregates (roof → slabs, stair → flights): parts take their whole's storey.
    for id in f.all("IFCRELAGGREGATES") {
        let Some(rel) = f.get(id) else { continue };
        let Some(whole) = rel.arg(4).r() else {
            continue;
        };
        for part in rel.arg(5).refs() {
            if let Some(s) = storey_of.get(&whole).copied() {
                storey_of.entry(part).or_insert(s);
            }
        }
    }

    // Storeys → levels, by elevation.
    let mut storeys: Vec<(u64, String, f64)> = f
        .all("IFCBUILDINGSTOREY")
        .into_iter()
        .filter_map(|id| {
            let e = f.get(id)?;
            let z = r.placement(e.arg(5)).o[2];
            let elev = e.arg(9).num().map(|x| x * r.unit).unwrap_or(z);
            let name = e.arg(2).str().unwrap_or("Level").to_owned();
            Some((id, name, if z.abs() > 1e-6 { z } else { elev }))
        })
        .collect();
    storeys.sort_by(|a, b| a.2.total_cmp(&b.2));
    if storeys.is_empty() {
        storeys.push((0, "Level 1".into(), 0.0));
        report
            .warnings
            .push("The file has no storeys: everything is on Level 1".into());
    }
    let seeded: Vec<ElementId> = doc.levels().into_iter().map(|l| l.0).collect();
    let mut level_of: HashMap<u64, (ElementId, f64)> = HashMap::new();
    let mut levels: Vec<(ElementId, f64)> = vec![];
    for (i, (sid, name, elev)) in storeys.iter().enumerate() {
        let id = match seeded.get(i) {
            Some(l) => *l,
            None => ops::create_level(doc, *elev)?,
        };
        let (name, elev) = (name.clone(), *elev);
        doc.transact("Import level", |tx| {
            tx.modify(id, |d| {
                if let ElementData::Level { name: n, elevation } = d {
                    *n = name.clone();
                    *elevation = elev;
                }
            })?;
            // Its plans take the storey's name too.
            let views: Vec<ElementId> = tx
                .of(Category::View)
                .filter(|v| matches!(&v.data, ElementData::View { kind: studio_core::ViewKind::FloorPlan { level } | studio_core::ViewKind::CeilingPlan { level }, .. } if *level == id))
                .map(|v| v.id)
                .collect();
            for v in views {
                tx.modify(v, |d| {
                    if let ElementData::View { name: n, .. } = d {
                        *n = name.clone();
                    }
                })?;
            }
            Ok(())
        })?;
        level_of.insert(*sid, (id, elev));
        levels.push((id, elev));
    }
    // Seeded levels the file doesn't need.
    for extra in seeded.iter().skip(storeys.len()) {
        doc.transact("Remove level", |tx| tx.delete(*extra).map(|_| ()))?;
    }
    report.levels = storeys.len();
    let level_for = |el: u64, z: f64| -> (ElementId, f64) {
        if let Some(l) = storey_of.get(&el).and_then(|s| level_of.get(s)) {
            return *l;
        }
        // The highest level at or below z.
        levels
            .iter()
            .rev()
            .find(|l| l.1 <= z + 1.0)
            .copied()
            .unwrap_or(levels[0])
    };

    // Recentre far-off models near the origin (site coordinates), by the walls' extent.
    let walls: Vec<u64> = ["IFCWALL", "IFCWALLSTANDARDCASE", "IFCWALLELEMENTEDCASE"]
        .iter()
        .flat_map(|n| f.all(n))
        .collect();
    let mut bodies: HashMap<u64, Body> = HashMap::new();
    for id in &walls {
        if let Some(e) = f.get(*id) {
            bodies.insert(*id, r.body(e));
        }
    }
    let all_pts: Vec<Pt> = bodies.values().flat_map(Body::plan).collect();
    let shift = match studio_geom::bounds_of(&all_pts) {
        Some((lo, hi))
            if lo.x.abs().max(hi.x.abs()).max(lo.y.abs()).max(hi.y.abs()) > 1_000_000.0 =>
        {
            let c = Pt::new(
                ((lo.x + hi.x) / 2000.0).round() * 1000.0,
                ((lo.y + hi.y) / 2000.0).round() * 1000.0,
            );
            report.warnings.push(format!(
                "The model sits {:.1} km from its origin; it was moved to the project origin",
                c.len() / 1e6
            ));
            c
        }
        _ => Pt::new(0.0, 0.0),
    };
    let sh = |p: Pt| p.sub(shift);

    // Psets: IsExternal.
    let mut external: HashMap<u64, bool> = HashMap::new();
    for id in f.all("IFCRELDEFINESBYPROPERTIES") {
        let Some(rel) = f.get(id) else { continue };
        let Some(ps) = f.at(rel.arg(5)) else { continue };
        for p in ps.arg(4).list() {
            let Some(prop) = f.at(p) else { continue };
            if prop.name == "IFCPROPERTYSINGLEVALUE" && prop.arg(0).str() == Some("IsExternal") {
                let v = matches!(prop.arg(2), V::Typed(_, a) if a.first().and_then(V::enumv) == Some("T"));
                for o in rel.arg(4).refs() {
                    external.insert(o, v);
                }
            }
        }
    }

    // Walls.
    let mut wall_types: BTreeMap<(String, i64), ElementId> = BTreeMap::new();
    let mut made_walls: HashMap<u64, ElementId> = HashMap::new();
    let mut curved = 0;
    for id in &walls {
        let Some(e) = f.get(*id) else { continue };
        let body = bodies.remove(id).unwrap_or_default();
        let foot: Vec<Pt> = body
            .outlines
            .first()
            .cloned()
            .unwrap_or_else(|| body.plan());
        let axis = r.axis(e);
        let dir = axis.map(|(a, b)| b.sub(a));
        let Some((a, b, t)) = centerline(&foot, dir) else {
            continue;
        };
        if foot.len() > 8 && body.outlines.first().is_some_and(|o| o.len() > 8) {
            curved += 1;
        }
        let (start, end) = (sh(a), sh(b));
        if start.dist(end) < 50.0 || t.is_nan() || t <= 10.0 {
            continue;
        }
        let (level, elev) = level_for(*id, body.z0);
        let name = type_part(e.arg(4).str().or(e.arg(2).str()).unwrap_or("Wall"));
        let t_key = (t / (MM_PER_IN / 8.0)).round() as i64;
        let exterior = external
            .get(id)
            .copied()
            .unwrap_or_else(|| name.to_lowercase().contains("exterior"));
        let wt = match wall_types.get(&(name.clone(), t_key)) {
            Some(w) => *w,
            None => {
                let tn = format!("{name} ({})", studio_core::units::format_ft_in(t));
                let w = doc.transact("Import wall type", |tx| {
                    Ok(tx.insert(ElementData::WallType {
                        name: tn.clone(),
                        thickness: t,
                        function: if exterior {
                            WallFunction::Exterior
                        } else {
                            WallFunction::Interior
                        },
                        layers: vec![WallLayer {
                            name: tn.clone(),
                            thickness: t,
                            function: LayerFunction::Structure,
                            material: None,
                        }],
                    }))
                })?;
                wall_types.insert((name.clone(), t_key), w);
                w
            }
        };
        let height = (body.z1 - body.z0).max(300.0);
        let base = body.z0 - elev;
        let made = doc.transact("Import wall", |tx| {
            Ok(tx.insert(ElementData::Wall {
                type_id: wt,
                start,
                end,
                base_level: level,
                base_offset: base,
                top: WallTop::Unconnected { height },
                location: Default::default(),
                attach_top: false,
            }))
        });
        if let Ok(w) = made {
            made_walls.insert(*id, w);
            report.walls += 1;
        }
    }
    if curved > 0 {
        report
            .warnings
            .push(format!("{curved} curved walls came in straight"));
    }

    // Openings: filler → (opening, host wall).
    let mut host_of_opening: HashMap<u64, u64> = HashMap::new();
    for id in f.all("IFCRELVOIDSELEMENT") {
        let Some(rel) = f.get(id) else { continue };
        if let (Some(w), Some(o)) = (rel.arg(4).r(), rel.arg(5).r()) {
            host_of_opening.insert(o, w);
        }
    }
    let mut opening_of: HashMap<u64, u64> = HashMap::new();
    for id in f.all("IFCRELFILLSELEMENT") {
        let Some(rel) = f.get(id) else { continue };
        if let (Some(o), Some(fill)) = (rel.arg(4).r(), rel.arg(5).r()) {
            opening_of.insert(fill, o);
        }
    }
    // Door and window styles/types: operation and partitioning.
    let mut type_of: HashMap<u64, u64> = HashMap::new();
    for id in f.all("IFCRELDEFINESBYTYPE") {
        let Some(rel) = f.get(id) else { continue };
        let Some(t) = rel.arg(5).r() else { continue };
        for o in rel.arg(4).refs() {
            type_of.insert(o, t);
        }
    }
    let mut door_types: BTreeMap<String, ElementId> = BTreeMap::new();
    let mut window_types: BTreeMap<String, ElementId> = BTreeMap::new();
    let mut marks = (0usize, 0usize);
    // Doors and windows with no wall to go in (skylights, curtain wall doors), and ones
    // Studio refused.
    let mut unhosted = 0usize;
    let mut refused: Vec<String> = vec![];
    for (cat, name) in [
        (0, "IFCDOOR"),
        (0, "IFCDOORSTANDARDCASE"),
        (1, "IFCWINDOW"),
        (1, "IFCWINDOWSTANDARDCASE"),
    ] {
        for id in f.all(name) {
            let Some(e) = f.get(id) else { continue };
            let Some(host) = opening_of
                .get(&id)
                .and_then(|o| host_of_opening.get(o))
                .and_then(|w| made_walls.get(w))
                .copied()
            else {
                unhosted += 1;
                continue;
            };
            let ElementData::Wall {
                start,
                end,
                base_level,
                base_offset,
                ..
            } = doc.data(host)?.clone()
            else {
                continue;
            };
            // Where: the opening's (or the element's own) body, projected on the host wall.
            let ob = opening_of
                .get(&id)
                .and_then(|o| f.get(*o))
                .map(|o| r.body(o))
                .filter(|b| !b.points.is_empty())
                .unwrap_or_else(|| r.body(e));
            if ob.points.is_empty() {
                continue;
            }
            let dir = end.sub(start).norm();
            let us: Vec<f64> = ob
                .plan()
                .iter()
                .map(|p| sh(*p).sub(start).dot(dir))
                .collect();
            let (u0, u1) = (
                us.iter().copied().fold(f64::MAX, f64::min),
                us.iter().copied().fold(f64::MIN, f64::max),
            );
            let width = e
                .arg(9)
                .num()
                .map(|w| w * r.unit)
                .filter(|w| *w > 100.0)
                .unwrap_or(u1 - u0);
            let height = e
                .arg(8)
                .num()
                .map(|h| h * r.unit)
                .filter(|h| *h > 100.0)
                .unwrap_or(ob.z1 - ob.z0);
            let offset = (u0 + u1) / 2.0;
            let len = start.dist(end);
            if width < 100.0 || offset - width / 2.0 < -1.0 || offset + width / 2.0 > len + 1.0 {
                report.warnings.push(format!(
                    "{} {} doesn't fit its wall and was left out",
                    if cat == 0 { "Door" } else { "Window" },
                    e.arg(7).str().unwrap_or("")
                ));
                continue;
            }
            // Which side it faces: its own y axis against the wall's left.
            let tf = r.placement(e.arg(5));
            let facing = Pt::new(tf.y[0], tf.y[1]).dot(dir.perp()) < 0.0;
            let full = e.arg(2).str().unwrap_or("");
            let fam_name = family_part(full);
            let type_name = type_part(e.arg(4).str().unwrap_or(full));
            let op = type_of
                .get(&id)
                .and_then(|t| f.get(*t))
                .map(|t| {
                    // IfcDoorStyle(…, OperationType at 8) / IfcDoorType(…, OperationType at 10).
                    [8usize, 9, 10, 11]
                        .iter()
                        .find_map(|i| t.arg(*i).enumv().map(str::to_owned))
                        .unwrap_or_default()
                })
                .or_else(|| e.arg(11).enumv().map(str::to_owned))
                .unwrap_or_default();
            let lower = format!("{fam_name} {type_name}").to_lowercase();
            let level_elev = doc.level_elevation(base_level).unwrap_or(0.0);
            let sill = (ob.z0 - level_elev - base_offset).max(0.0);
            if cat == 0 {
                let family = if op.starts_with("DOUBLE") || lower.contains("double") {
                    DoorFamily::DoubleFlush
                } else if op.starts_with("SLIDING") || lower.contains("slid") {
                    DoorFamily::SlidingGlass
                } else if op.starts_with("FOLDING") || lower.contains("bifold") {
                    DoorFamily::Bifold
                } else if lower.contains("overhead") || lower.contains("garage") {
                    DoorFamily::Garage
                } else {
                    DoorFamily::SingleFlush
                };
                let leaf = if lower.contains("glass") || lower.contains("glaz") {
                    LeafStyle::FullLite
                } else {
                    LeafStyle::Flush
                };
                let key = format!("{fam_name}: {type_name} {:.0}x{:.0}", width, height);
                let ty = match door_types.get(&key) {
                    Some(t) => *t,
                    None => {
                        let name = if type_name.is_empty() {
                            key.clone()
                        } else {
                            format!("{fam_name}: {type_name}")
                        };
                        let t = doc.transact("Import door type", |tx| {
                            Ok(tx.insert(ElementData::DoorType {
                                name: unique(tx.doc_names(Category::DoorType), &name),
                                family,
                                width,
                                height,
                                leaf,
                                panels: 0,
                                finish: None,
                            }))
                        })?;
                        door_types.insert(key, t);
                        t
                    }
                };
                marks.0 += 1;
                let mark = e
                    .arg(7)
                    .str()
                    .filter(|m| !m.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| marks.0.to_string());
                let made = doc.transact("Import door", |tx| {
                    Ok(tx.insert(ElementData::Door {
                        type_id: ty,
                        host,
                        offset,
                        flip_hand: op.ends_with("RIGHT"),
                        flip_facing: facing,
                        mark: mark.clone(),
                    }))
                });
                match made {
                    Ok(_) => report.doors += 1,
                    Err(err) => refused.push(format!("door {mark}: {err}")),
                }
            } else {
                let family = if lower.contains("casement") {
                    WindowFamily::Casement
                } else if lower.contains("double hung") || lower.contains("double-hung") {
                    WindowFamily::DoubleHung
                } else if lower.contains("single hung") {
                    WindowFamily::SingleHung
                } else if lower.contains("slid") {
                    WindowFamily::Slider
                } else if lower.contains("awning") {
                    WindowFamily::Awning
                } else {
                    WindowFamily::Fixed
                };
                let key = format!("{fam_name}: {type_name} {:.0}x{:.0}", width, height);
                let ty = match window_types.get(&key) {
                    Some(t) => *t,
                    None => {
                        let name = if type_name.is_empty() {
                            key.clone()
                        } else {
                            format!("{fam_name}: {type_name}")
                        };
                        let t = doc.transact("Import window type", |tx| {
                            Ok(tx.insert(ElementData::WindowType {
                                name: unique(tx.doc_names(Category::WindowType), &name),
                                family,
                                width,
                                height,
                                sill,
                                units: 1,
                                grille: Grille::None,
                                finish: FrameFinish::White,
                            }))
                        })?;
                        window_types.insert(key, t);
                        t
                    }
                };
                marks.1 += 1;
                let mark = e
                    .arg(7)
                    .str()
                    .filter(|m| !m.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| marks.1.to_string());
                let made = doc.transact("Import window", |tx| {
                    Ok(tx.insert(ElementData::Window {
                        type_id: ty,
                        host,
                        offset,
                        sill,
                        flip_facing: facing,
                        mark: mark.clone(),
                    }))
                });
                match made {
                    Ok(_) => report.windows += 1,
                    Err(err) => refused.push(format!("window {mark}: {err}")),
                }
            }
        }
    }

    if unhosted > 0 {
        report.warnings.push(format!(
            "{unhosted} doors and windows have no wall to go in (skylights, curtain wall doors) and were left out"
        ));
    }
    if !refused.is_empty() {
        let n = refused.len();
        refused.truncate(5);
        report.warnings.push(format!(
            "{n} doors and windows were refused: {}",
            refused.join("; ")
        ));
    }

    // Slabs: floors and flat roofs.
    let mut floor_types: BTreeMap<i64, ElementId> = BTreeMap::new();
    let roof_type = ops::first_of(doc, Category::RoofType);
    for id in f
        .all("IFCSLAB")
        .into_iter()
        .chain(f.all("IFCSLABSTANDARDCASE"))
    {
        let Some(e) = f.get(id) else { continue };
        let body = r.body(e);
        if body.points.is_empty() {
            continue;
        }
        let kind = e.arg(8).enumv().unwrap_or("FLOOR").to_owned();
        let outline: Vec<Pt> = match body.outlines.first() {
            Some(o) if kind != "ROOF" => o.iter().map(|p| sh(*p)).collect(),
            _ => studio_geom::convex_hull(&body.plan().into_iter().map(sh).collect::<Vec<_>>()),
        };
        if outline.len() < 3 || studio_geom::signed_area(&outline).abs() < 100_000.0 {
            continue;
        }
        let (level, elev) = level_for(id, body.z1 - 1.0);
        if kind == "ROOF" {
            let Some(rt) = roof_type else { continue };
            let n = outline.len();
            let off = body.z0 - elev;
            let made = doc.transact("Import roof", |tx| {
                Ok(tx.insert(ElementData::Roof {
                    type_id: rt,
                    level,
                    offset: off,
                    boundary: outline.clone(),
                    slope: 0.0,
                    sloped: vec![false; n],
                }))
            });
            if made.is_ok() {
                report.roofs += 1;
            }
            continue;
        }
        let t = (body.z1 - body.z0).clamp(25.0, 2000.0);
        let key = (t / (MM_PER_IN / 4.0)).round() as i64;
        let ft = match floor_types.get(&key) {
            Some(x) => *x,
            None => {
                let name = format!("Imported Floor {}", studio_core::units::format_ft_in(t));
                let x = doc.transact("Import floor type", |tx| {
                    Ok(tx.insert(ElementData::FloorType {
                        name: name.clone(),
                        thickness: t,
                        layers: vec![],
                    }))
                })?;
                floor_types.insert(key, x);
                x
            }
        };
        let off = body.z1 - elev;
        let made = doc.transact("Import floor", |tx| {
            Ok(tx.insert(ElementData::Floor {
                type_id: ft,
                level,
                offset: off,
                boundary: outline.clone(),
                bound: Default::default(),
                sketch: vec![],
            }))
        });
        if made.is_ok() {
            report.floors += 1;
        }
    }

    // Spaces → rooms.
    for id in f.all("IFCSPACE") {
        let Some(e) = f.get(id) else { continue };
        let body = r.body(e);
        let pts = body.plan();
        let at = if pts.is_empty() {
            let o = r.placement(e.arg(5)).o;
            Pt::new(o[0], o[1])
        } else {
            let foot = body.outlines.first().cloned().unwrap_or(pts);
            centroid(&foot)
        };
        let (level, _) = level_for(id, body.z0 + 1.0);
        let number = e.arg(2).str().unwrap_or("").to_owned();
        let name = e
            .arg(7)
            .str()
            .filter(|s| !s.is_empty())
            .unwrap_or("Room")
            .to_owned();
        let p = sh(at);
        let made = doc.transact("Import room", |tx| {
            Ok(tx.insert(ElementData::Room {
                level,
                point: p,
                name: name.clone(),
                number: if number.is_empty() {
                    "0".into()
                } else {
                    number.clone()
                },
            }))
        });
        if made.is_ok() {
            report.rooms += 1;
        }
    }

    // Grids.
    for id in f.all("IFCGRID") {
        let Some(g) = f.get(id) else { continue };
        let tf = r.placement(g.arg(5));
        for axes in [g.arg(7), g.arg(8), g.arg(9)] {
            for a in axes.list() {
                let Some(ax) = f.at(a) else { continue };
                let pts = r.curve(ax.arg(1));
                let (Some(p), Some(q)) = (pts.first(), pts.last()) else {
                    continue;
                };
                let (p, q) = (tf.point(*p), tf.point(*q));
                let (p, q) = (sh(Pt::new(p[0], p[1])), sh(Pt::new(q[0], q[1])));
                if p.dist(q) < 100.0 {
                    continue;
                }
                let tag = ax.arg(0).str().unwrap_or("").to_owned();
                let made = doc.transact("Import grid", |tx| {
                    Ok(tx.insert(ElementData::Grid {
                        name: if tag.is_empty() {
                            format!("{}", report.grids + 1)
                        } else {
                            tag.clone()
                        },
                        start: p,
                        end: q,
                    }))
                });
                if made.is_ok() {
                    report.grids += 1;
                }
            }
        }
    }

    // Columns.
    let col_type = ops::first_of(doc, Category::ColumnType);
    for id in f
        .all("IFCCOLUMN")
        .into_iter()
        .chain(f.all("IFCCOLUMNSTANDARDCASE"))
    {
        let (Some(e), Some(ct)) = (f.get(id), col_type) else {
            continue;
        };
        let body = r.body(e);
        if body.points.is_empty() {
            continue;
        }
        let foot = body
            .outlines
            .first()
            .cloned()
            .unwrap_or_else(|| body.plan());
        let at = sh(centroid(&foot));
        let (level, _) = level_for(id, body.z0 + 1.0);
        if studio_core::structure::create_column(doc, ct, level, at, 0.0).is_ok() {
            report.columns += 1;
        }
    }

    // What wasn't brought in.
    let handled = [
        "IFCWALL",
        "IFCWALLSTANDARDCASE",
        "IFCWALLELEMENTEDCASE",
        "IFCDOOR",
        "IFCDOORSTANDARDCASE",
        "IFCWINDOW",
        "IFCWINDOWSTANDARDCASE",
        "IFCSLAB",
        "IFCSLABSTANDARDCASE",
        "IFCSPACE",
        "IFCCOLUMN",
        "IFCCOLUMNSTANDARDCASE",
        "IFCROOF",
        "IFCBUILDINGSTOREY",
        "IFCBUILDING",
        "IFCSITE",
        "IFCOPENINGELEMENT",
        "IFCGRID",
        "IFCANNOTATION",
    ];
    let products = [
        "IFCSTAIR",
        "IFCSTAIRFLIGHT",
        "IFCRAILING",
        "IFCCURTAINWALL",
        "IFCPLATE",
        "IFCMEMBER",
        "IFCBEAM",
        "IFCFURNISHINGELEMENT",
        "IFCFLOWTERMINAL",
        "IFCBUILDINGELEMENTPROXY",
        "IFCCOVERING",
        "IFCRAMP",
        "IFCRAMPFLIGHT",
        "IFCFOOTING",
    ];
    for p in products {
        if handled.contains(&p) {
            continue;
        }
        let n = f.all(p).len();
        if n > 0 {
            let label = p.trim_start_matches("IFC").to_lowercase();
            report.skipped.push((label, n));
        }
    }
    Ok(())
}

/// A polygon's area centroid (the mean of its points when it has no area).
fn centroid(pts: &[Pt]) -> Pt {
    let a = studio_geom::signed_area(pts);
    if a.abs() < 1.0 {
        let n = pts.len().max(1) as f64;
        return Pt::new(
            pts.iter().map(|p| p.x).sum::<f64>() / n,
            pts.iter().map(|p| p.y).sum::<f64>() / n,
        );
    }
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 0..pts.len() {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        let c = p.x * q.y - q.x * p.y;
        cx += (p.x + q.x) * c;
        cy += (p.y + q.y) * c;
    }
    Pt::new(cx / (6.0 * a), cy / (6.0 * a))
}

/// `name`, or `name (2)`… when taken.
fn unique(taken: Vec<String>, name: &str) -> String {
    if !taken.iter().any(|t| t == name) {
        return name.to_owned();
    }
    (2..)
        .map(|i| format!("{name} ({i})"))
        .find(|n| !taken.contains(n))
        .unwrap_or_else(|| name.to_owned())
}

trait Names {
    fn doc_names(&self, cat: Category) -> Vec<String>;
}

impl Names for studio_core::Tx<'_> {
    fn doc_names(&self, cat: Category) -> Vec<String> {
        self.of(cat).map(|e| e.data.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centerlines_and_names() {
        let foot = [
            Pt::new(0.0, 0.0),
            Pt::new(4000.0, 0.0),
            Pt::new(4000.0, 200.0),
            Pt::new(0.0, 200.0),
        ];
        let (a, b, t) = centerline(&foot, None).unwrap();
        assert!((t - 200.0).abs() < 1e-9);
        assert!((a.y - 100.0).abs() < 1e-9 && (b.y - 100.0).abs() < 1e-9);
        assert!((a.dist(b) - 4000.0).abs() < 1e-9);
        assert_eq!(
            type_part("Basic Wall:Exterior - Brick on CMU:128360"),
            "Exterior - Brick on CMU"
        );
        assert_eq!(
            family_part("M_Single-Flush:0915 x 2134mm:146596"),
            "M_Single-Flush"
        );
        assert_eq!(unique(vec!["A".into()], "A"), "A (2)");
    }

    /// Dev aid: `IFC_IN=path cargo test -p studio-io import_file -- --ignored --nocapture`
    /// imports a file and prints what came in.
    #[test]
    #[ignore]
    fn import_file() {
        let path = std::env::var("IFC_IN").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let t0 = std::time::Instant::now();
        let (doc, rep) = import(&text).unwrap();
        println!("{path}: {:?} in {:?}", rep, t0.elapsed());
        println!("levels: {:?}", doc.levels());
        if let Ok(out) = std::env::var("IFC_OUT") {
            crate::Project::new("test", doc.clone())
                .save(std::path::Path::new(&out), "test")
                .unwrap();
        }
    }

    /// Studio's own IFC export comes back as the same building.
    #[test]
    fn round_trips_studio_ifc() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let ft = 304.8;
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(40.0 * ft, 0.0),
            Pt::new(40.0 * ft, 30.0 * ft),
            Pt::new(0.0, 30.0 * ft),
        ];
        let walls: Vec<ElementId> = (0..4)
            .map(|i| ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
            .collect();
        let dt = ops::first_of(&doc, Category::DoorType).unwrap();
        let wn = ops::first_of(&doc, Category::WindowType).unwrap();
        ops::create_door(&mut doc, dt, walls[0], 10.0 * ft, false).unwrap();
        ops::create_window(&mut doc, wn, walls[1], 12.0 * ft, false).unwrap();
        let m = studio_regen::regenerate(&doc);
        let ftype = ops::first_of(&doc, Category::FloorType).unwrap();
        ops::create_floor(
            &mut doc,
            ftype,
            l1,
            studio_regen::outer_boundary(&m, l1).unwrap(),
        )
        .unwrap();
        ops::create_room(&mut doc, l1, Pt::new(3000.0, 3000.0)).unwrap();
        let (ifc, _) = crate::ifc::export_ifc(&doc, "test", "2026-09-26T00:00:00");
        let (back, rep) = import(&ifc).unwrap();
        assert_eq!(rep.schema, "IFC4");
        assert_eq!(
            (rep.walls, rep.doors, rep.windows, rep.floors, rep.rooms),
            (4, 1, 1, 1, 1),
            "{rep:?}"
        );
        // Wall lines, thickness and height survive within a millimetre.
        let lens: Vec<f64> = back
            .of(Category::Wall)
            .filter_map(|e| match &e.data {
                ElementData::Wall { start, end, .. } => Some(start.dist(*end)),
                _ => None,
            })
            .collect();
        let mut sorted = lens.clone();
        sorted.sort_by(f64::total_cmp);
        // Exported footprints are joined at corners; walls come back from end to end.
        assert!(
            sorted.iter().all(|l| *l > 29.0 * ft && *l < 41.0 * ft),
            "{sorted:?}"
        );
        let thick: Vec<f64> = back
            .of(Category::WallType)
            .filter(|e| e.data.name().contains("Exterior - 8"))
            .filter_map(|e| match &e.data {
                ElementData::WallType { thickness, .. } => Some(*thickness),
                _ => None,
            })
            .collect();
        assert!(
            thick.iter().any(|t| (t - 8.0 * MM_PER_IN).abs() < 1.0),
            "{thick:?}"
        );
        // The door is in its wall, 10' along it (walls come back to their joined corners,
        // so measure where it is, not its offset).
        let door = back.of(Category::Door).next().unwrap();
        let ElementData::Door { offset, host, .. } = &door.data else {
            panic!()
        };
        let ElementData::Wall { start, end, .. } = back.data(*host).unwrap() else {
            panic!()
        };
        let at = start.add(end.sub(*start).norm().scale(*offset));
        assert!((at.x - 10.0 * ft).abs() < 1.0 && at.y.abs() < 1.0, "{at:?}");
        assert!(!back.can_undo().is_some(), "a fresh project");
    }
}
