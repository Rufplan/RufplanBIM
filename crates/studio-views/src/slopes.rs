//! Spot slopes (ADR-049): the slope of the roof, sloped floor or ground at a point, read from
//! the model, and drawn as Revit draws it: an arrow pointing downhill with the slope beside
//! it, or in elevations and sections a slope triangle.

use studio_core::element::{ElementData, ElementId, SlopeFormat, ViewKind};
use studio_core::slope::{format_slope, SlopeSource};
use studio_core::Document;
use studio_geom::{point_in_ring, Pt};
use studio_regen::Model;

use crate::{ring, upright, Anchor, Builder, Dash, FillKind};

/// A slope found at a point: rise over run, the way down (a unit vector in the view's
/// coordinates) and what it was read from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlopeHit {
    pub tan: f64,
    pub down: Pt,
    pub source: SlopeSource,
}

/// A sloped plane: where it lies in plan, its height at a point and its uphill gradient.
struct Plane<'a> {
    outline: &'a [Pt],
    source: SlopeSource,
    height: Box<dyn Fn(Pt) -> f64 + 'a>,
    gradient: Pt,
}

fn planes(model: &Model) -> Vec<Plane<'_>> {
    let mut out: Vec<Plane> = vec![];
    for r in &model.roofs {
        for f in &r.faces {
            out.push(Plane {
                outline: &f.poly,
                source: SlopeSource::Roof,
                height: Box::new(move |p| r.face_top(f, p)),
                gradient: f.n.scale(r.slope.tan()),
            });
        }
    }
    for s in model.floors.iter().filter(|s| s.tilt.is_some()) {
        out.push(Plane {
            outline: &s.base.outer,
            source: SlopeSource::Floor,
            height: Box::new(move |p| s.top_at(p)),
            gradient: s.gradient(),
        });
    }
    out
}

/// The ground's uphill gradient at `p` (sampled either side), if the topography covers it.
fn ground_gradient(model: &Model, p: Pt) -> Option<Pt> {
    let s = model.site.as_ref()?;
    let h = s
        .topo
        .as_ref()
        .map_or(300.0, |t| (t.spacing / 2.0).max(50.0));
    let z = |q: Pt| s.ground_at(q);
    let gx = (z(p.add(Pt::new(h, 0.0)))? - z(p.sub(Pt::new(h, 0.0)))?) / (2.0 * h);
    let gy = (z(p.add(Pt::new(0.0, h)))? - z(p.sub(Pt::new(0.0, h)))?) / (2.0 * h);
    Some(Pt::new(gx, gy))
}

/// A projected view's frame: its origin, right and look directions, and a section's depth.
fn frame(doc: &Document, model: &Model, view: ElementId) -> Option<(Pt, Pt, Pt, Option<f64>)> {
    let ElementData::View {
        kind, callout_of, ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    let parent = callout_of.and_then(|p| match doc.data(p) {
        Ok(ElementData::View {
            kind: k @ (ViewKind::Section { .. } | ViewKind::Elevation { .. }),
            ..
        }) => Some(k.clone()),
        _ => None,
    });
    let right_of = |look: Pt| Pt::new(look.y, -look.x);
    match parent.as_ref().unwrap_or(kind) {
        ViewKind::Section { start, end, depth } => {
            let look = end.sub(*start).norm().perp();
            Some((*start, right_of(look), look, Some(*depth)))
        }
        ViewKind::Elevation { facing } => {
            let look = facing.look().scale(-1.0);
            Some((Pt::default(), right_of(look), look, None))
        }
        ViewKind::MarkerElevation { marker, facing } => {
            let look = facing.look();
            match doc.data(*marker) {
                Ok(ElementData::ElevationMarker {
                    level,
                    at,
                    interior: true,
                    ..
                }) => {
                    let (cut, _) = crate::interior_cut(doc, model, *level, *at, look);
                    Some((cut.origin, right_of(look), look, Some(cut.depth)))
                }
                _ => Some((Pt::default(), right_of(look), look, None)),
            }
        }
        _ => None,
    }
}

/// The slope at `at` (view coordinates) in `view`, within `tol` (mm) of an edge in
/// elevations and sections. None where the surface there is level.
pub fn slope_at(
    doc: &Document,
    model: &Model,
    view: ElementId,
    at: Pt,
    tol: f64,
) -> Option<SlopeHit> {
    let ElementData::View { kind, .. } = doc.data(view).ok()? else {
        return None;
    };
    if let ViewKind::FloorPlan { level } = kind {
        return plan_slope(model, *level, at);
    }
    let (origin, right, look, depth) = frame(doc, model, view)?;
    let base = origin.add(right.scale(at.x));
    // The nearest sloped surface whose edge passes through the point, as (depth, slope).
    let mut best: Option<(f64, f64, SlopeSource)> = None;
    let mut take = |d: f64, m: f64, source| {
        if m.abs() > 1e-4 && best.is_none_or(|b| d < b.0) {
            best = Some((d, m, source));
        }
    };
    for pl in planes(model) {
        let m = pl.gradient.dot(right);
        let along = pl.gradient.dot(look);
        // Cut by a section's plane: its profile is a line there.
        if depth.is_some()
            && point_in_ring(base, pl.outline)
            && ((pl.height)(base) - at.y).abs() < tol
        {
            take(0.0, m, pl.source);
            continue;
        }
        // Otherwise only surfaces seen nearly edge-on read as a line.
        if along.abs() > 0.27 * pl.gradient.len() {
            continue;
        }
        let ds: Vec<f64> = pl.outline.iter().map(|p| p.sub(origin).dot(look)).collect();
        let (d0, d1) = (
            // Sections see only what's beyond the cut; elevations see everything.
            ds.iter()
                .copied()
                .fold(f64::INFINITY, f64::min)
                .max(if depth.is_some() {
                    0.0
                } else {
                    f64::NEG_INFINITY
                }),
            ds.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .min(depth.unwrap_or(f64::INFINITY)),
        );
        for k in 0..=64 {
            let d = d0 + (d1 - d0) * f64::from(k) / 64.0;
            let p = base.add(look.scale(d));
            if point_in_ring(p, pl.outline) && ((pl.height)(p) - at.y).abs() < tol {
                take(d, m, pl.source);
                break;
            }
        }
    }
    // The ground where a section cuts it.
    if let (Some(_), Some(s)) = (depth, model.site.as_ref()) {
        if s.ground_at(base).is_some_and(|z| (z - at.y).abs() < tol) {
            if let Some(g) = ground_gradient(model, base) {
                take(0.0, g.dot(right), SlopeSource::Ground);
            }
        }
    }
    let (_, m, source) = best?;
    Some(SlopeHit {
        tan: m.abs(),
        down: Pt::new(-m.signum(), -m.abs()).norm(),
        source,
    })
}

/// In a plan: the highest surface at the point (roofs on this level, floors at or below
/// the cut plane), else the ground.
fn plan_slope(model: &Model, level: ElementId, at: Pt) -> Option<SlopeHit> {
    let e = model.levels.iter().find(|l| l.id == level)?.elevation;
    let cut = e + 4.0 * studio_core::units::MM_PER_FT;
    let mut best: Option<(f64, Pt, SlopeSource)> = None;
    let mut take = |z: f64, g: Pt, source| {
        if best.is_none_or(|b| z > b.0) {
            best = Some((z, g, source));
        }
    };
    for r in model
        .roofs
        .iter()
        .filter(|r| r.level == level || (r.base >= e - 1.0 && r.base <= cut))
    {
        for f in r.faces.iter().filter(|f| point_in_ring(at, &f.poly)) {
            take(
                r.face_top(f, at),
                f.n.scale(r.slope.tan()),
                SlopeSource::Roof,
            );
        }
    }
    for s in &model.floors {
        let z = s.top_at(at);
        if z <= cut + 1.0 && s.base.contains(at) {
            take(z, s.gradient(), SlopeSource::Floor);
        }
    }
    let (g, source) = match best {
        Some((_, g, source)) => (g, source),
        None => (ground_gradient(model, at)?, SlopeSource::Ground),
    };
    let tan = g.len();
    (tan > 1e-4).then(|| SlopeHit {
        tan,
        down: g.scale(-1.0 / tan),
        source,
    })
}

/// The leg labels of a slope triangle: (run, rise), or None to write the slope beside it.
fn legs(tan: f64, format: SlopeFormat, source: SlopeSource) -> Option<(String, String)> {
    let text = format_slope(tan, format, source);
    if text.ends_with("/ 12\"") {
        let rise = text.trim_end_matches("/ 12\"").trim().trim_end_matches('"');
        return Some(("12".into(), rise.into()));
    }
    if let Some(p) = text.strip_suffix('%') {
        return Some(("100".into(), p.into()));
    }
    text.strip_prefix("1:").map(|run| (run.into(), "1".into()))
}

/// A spot slope at `at`: an arrow pointing down `hit.down` with the slope written along it,
/// or (in elevations and sections, `triangle`) a slope triangle above the edge.
#[allow(clippy::too_many_arguments)]
pub fn spot_slope(
    b: &mut Builder,
    el: Option<ElementId>,
    at: Pt,
    hit: &SlopeHit,
    format: SlopeFormat,
    triangle: bool,
    in_plan: bool,
) {
    let text = format_slope(hit.tan, format, hit.source);
    let d = hit.down;
    if triangle && !in_plan {
        // Legs level and plumb, the long side parallel to the slope, lifted clear of it.
        let run = b.paper(9.0);
        let side = -d.x.signum();
        let a = at.add(Pt::new(0.0, b.paper(2.0)));
        let bb = a.add(Pt::new(side * run, 0.0));
        let c = bb.add(Pt::new(0.0, run * hit.tan));
        b.line(el, &[a, bb, c], true, 1, Dash::Solid);
        match legs(hit.tan, format, hit.source) {
            Some((r, rise)) => {
                b.text(
                    el,
                    a.lerp(bb, 0.5).sub(Pt::new(0.0, b.paper(1.6))),
                    r,
                    2.2,
                    Anchor::Center,
                );
                let anchor = if side > 0.0 {
                    Anchor::Left
                } else {
                    Anchor::Right
                };
                b.text(
                    el,
                    bb.lerp(c, 0.5).add(Pt::new(side * b.paper(1.0), 0.0)),
                    rise,
                    2.2,
                    anchor,
                );
            }
            None => {
                let anchor = if side > 0.0 {
                    Anchor::Left
                } else {
                    Anchor::Right
                };
                b.text(
                    el,
                    c.add(Pt::new(side * b.paper(1.0), 0.0)),
                    text,
                    2.2,
                    anchor,
                );
            }
        }
        return;
    }
    // In elevations and sections the arrow sits just above the edge it measures.
    let lift = if in_plan {
        Pt::new(0.0, 0.0)
    } else {
        let n = d.perp();
        let up = if n.y >= 0.0 { n } else { n.scale(-1.0) };
        up.scale(b.paper(2.2))
    };
    let half = d.scale(b.paper(7.0));
    let (tail, head) = (at.add(lift).sub(half), at.add(lift).add(half));
    b.line(el, &[tail, head], false, 1, Dash::Solid);
    let back = head.sub(d.scale(b.paper(2.2)));
    let w = d.perp().scale(b.paper(0.75));
    b.fill(
        el,
        vec![ring(&[head, back.add(w), back.sub(w)])],
        FillKind::Ink,
    );
    let (angle, above) = upright(d);
    b.text_rot(
        el,
        at.add(lift).add(above.scale(b.paper(1.8))),
        text,
        2.4,
        Anchor::Center,
        angle,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Prim;
    use studio_core::{ops, Category};

    fn view(doc: &Document, f: fn(&ViewKind) -> bool) -> ElementId {
        doc.of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind, .. } if f(kind)))
            .unwrap()
            .id
    }

    /// A 10' × 5' floor on Level 1 falling 1:12 toward the east.
    fn ramp() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let ft = ops::first_of(&doc, Category::FloorType).unwrap();
        let sq = vec![
            Pt::new(0.0, 0.0),
            Pt::new(3048.0, 0.0),
            Pt::new(3048.0, 1524.0),
            Pt::new(0.0, 1524.0),
        ];
        let f = ops::create_floor(&mut doc, ft, l1, sq).unwrap();
        ops::set_property(&mut doc, f, "slope", "1:12", 0).unwrap();
        ops::set_property(&mut doc, f, "slope_dir", "0", 0).unwrap();
        (doc, f)
    }

    #[test]
    fn a_sloped_floor_falls_from_its_high_edge() {
        let (doc, f) = ramp();
        let m = studio_regen::regenerate(&doc);
        let s = m.floors.iter().find(|s| s.id == f).unwrap();
        // Level at the west edge, 10' / 12 = 10" lower at the east edge.
        assert!((s.top_at(Pt::new(0.0, 700.0)) - 0.0).abs() < 1e-6);
        assert!((s.top_at(Pt::new(3048.0, 700.0)) + 254.0).abs() < 1e-6);
        // Its 3D triangles are sheared to match.
        let tri = s.triangles();
        let zs: Vec<f32> = tri.as_chunks::<3>().0.iter().map(|v| v[2]).collect();
        let lowest = zs.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            (f64::from(lowest) - (s.z0 - 254.0)).abs() < 0.01,
            "{lowest}"
        );
    }

    #[test]
    fn spot_slopes_read_floors_in_plan_and_section() {
        let (mut doc, _) = ramp();
        let model = studio_regen::regenerate(&doc);
        let plan = view(&doc, |k| matches!(k, ViewKind::FloorPlan { .. }));
        let hit = slope_at(&doc, &model, plan, Pt::new(1500.0, 700.0), 50.0).unwrap();
        assert!((hit.tan - 1.0 / 12.0).abs() < 1e-9);
        assert!(hit.down.x > 0.999, "falls toward the east");
        assert_eq!(hit.source, SlopeSource::Floor);
        // Off the ramp there's nothing sloped.
        assert!(slope_at(&doc, &model, plan, Pt::new(9000.0, 9000.0), 50.0).is_none());
        // A section cutting the ramp east–west, looking north: its profile falls to the right.
        let sec =
            ops::create_section(&mut doc, Pt::new(-1000.0, 700.0), Pt::new(5000.0, 700.0)).unwrap();
        let model = studio_regen::regenerate(&doc);
        // u is measured from the section's start: the ramp's middle is 2524 mm along, where
        // its top is 1524 / 12 = 127 mm down.
        let hit = slope_at(&doc, &model, sec, Pt::new(2524.0, -127.0), 50.0).unwrap();
        assert!((hit.tan - 1.0 / 12.0).abs() < 1e-9);
        assert!(hit.down.x > 0.0 && hit.down.y < 0.0);
        assert!(slope_at(&doc, &model, sec, Pt::new(2524.0, 2000.0), 50.0).is_none());
    }

    #[test]
    fn spot_slopes_read_a_hip_roof_in_plan_and_elevation() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let rt = ops::first_of(&doc, Category::RoofType).unwrap();
        let outline = vec![
            Pt::new(0.0, 0.0),
            Pt::new(6096.0, 0.0),
            Pt::new(6096.0, 4572.0),
            Pt::new(0.0, 4572.0),
        ];
        // 6" in 12": a hip roof on Level 1, so its plan shows it.
        studio_core::build::create_roof(&mut doc, rt, l1, 0.0, outline, 0.5f64.atan()).unwrap();
        let model = studio_regen::regenerate(&doc);
        let plan = view(&doc, |k| matches!(k, ViewKind::FloorPlan { .. }));
        // Near the south eave the roof falls south.
        let hit = slope_at(&doc, &model, plan, Pt::new(3048.0, 300.0), 50.0).unwrap();
        assert!((hit.tan - 0.5).abs() < 1e-9 && hit.down.y < -0.999);
        assert_eq!(hit.source, SlopeSource::Roof);
        assert_eq!(
            format_slope(hit.tan, SlopeFormat::Auto, hit.source),
            "6\" / 12\""
        );
        // In an elevation looking north or south, the east hip face is seen edge-on.
        let elev = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Elevation { .. },
                        ..
                    }
                ) && frame(&doc, &model, e.id).is_some_and(|f| f.2.y.abs() > 0.9)
            })
            .unwrap()
            .id;
        let (origin, right, ..) = frame(&doc, &model, elev).unwrap();
        let roof = &model.roofs[0];
        let east = roof.faces.iter().find(|f| f.n.x < -0.9).unwrap();
        let p = Pt::new(5800.0, 2286.0);
        let at = Pt::new(p.sub(origin).dot(right), roof.face_top(east, p));
        let hit = slope_at(&doc, &model, elev, at, 50.0).unwrap();
        assert!((hit.tan - 0.5).abs() < 1e-9, "{hit:?}");
        // Down is toward the eave: east, which is right or left as the view looks.
        assert!((hit.down.x.signum() - right.x.signum()).abs() < 1e-9 && hit.down.y < 0.0);
    }

    #[test]
    fn spot_slope_arrow_and_triangle() {
        let hit = SlopeHit {
            tan: 0.5,
            down: Pt::new(-1.0, -0.5).norm(),
            source: SlopeSource::Roof,
        };
        let texts = |b: &Builder| -> Vec<String> {
            b.items
                .iter()
                .filter_map(|i| match &i.prim {
                    Prim::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        let mut b = Builder::new(48.0);
        spot_slope(
            &mut b,
            None,
            Pt::new(0.0, 0.0),
            &hit,
            SlopeFormat::Auto,
            false,
            false,
        );
        assert_eq!(texts(&b), ["6\" / 12\""]);
        // A roof rising to the right: the triangle's legs read 12 and 6.
        let mut b = Builder::new(48.0);
        spot_slope(
            &mut b,
            None,
            Pt::new(0.0, 0.0),
            &hit,
            SlopeFormat::Auto,
            true,
            false,
        );
        assert_eq!(texts(&b), ["12", "6"]);
        let tri = b
            .items
            .iter()
            .find_map(|i| match &i.prim {
                Prim::Line { pts, .. } => Some(pts.clone()),
                _ => None,
            })
            .unwrap();
        // 9 mm of run on paper at 1:48, and half that of rise.
        assert!((tri[1][0] - tri[0][0] - 9.0 * 48.0).abs() < 1e-6);
        assert!((tri[2][1] - tri[1][1] - 4.5 * 48.0).abs() < 1e-6);
        let mut b = Builder::new(48.0);
        spot_slope(
            &mut b,
            None,
            Pt::new(0.0, 0.0),
            &hit,
            SlopeFormat::Percent,
            true,
            false,
        );
        assert_eq!(texts(&b), ["100", "50.00"]);
    }
}
