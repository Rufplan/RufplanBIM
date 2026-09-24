//! Snapping for drawing tools, computed in Rust so every tool snaps the same way.

use serde::Serialize;
use studio_core::units::{format_ft_in, MM_PER_IN};
use studio_core::{Document, ElementData, ElementId, ViewKind};
use studio_geom::{line_intersection, project_to_segment, Pt};
use studio_regen::regenerate;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export)]
pub enum SnapKind {
    Endpoint,
    Intersection,
    Midpoint,
    Perpendicular,
    Nearest,
    Angle,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SnapResult {
    pub pt: Pt,
    pub kind: SnapKind,
    /// Length (and angle) from the previous point, formatted for display.
    pub label: Option<String>,
}

/// Snaps `p` in `view`. `from` is the tool's previous point, `tol` the snap radius in mm.
pub fn snap(doc: &Document, view: ElementId, p: Pt, from: Option<Pt>, tol: f64) -> SnapResult {
    let is_plan = matches!(
        doc.data(view),
        Ok(ElementData::View {
            kind: ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. },
            ..
        })
    );
    if !is_plan {
        // Elevations: round the height to the nearest inch.
        let z = (p.y / MM_PER_IN).round() * MM_PER_IN;
        return SnapResult {
            pt: Pt::new(p.x, z),
            kind: SnapKind::None,
            label: Some(format_ft_in(z)),
        };
    }
    let model = regenerate(doc);
    let mut segs: Vec<(Pt, Pt)> = model.walls.iter().map(|w| (w.start, w.end)).collect();
    segs.extend(model.grids.iter().map(|g| (g.start, g.end)));

    let mut cands: Vec<(SnapKind, Pt)> = vec![];
    for (a, b) in &segs {
        cands.push((SnapKind::Endpoint, *a));
        cands.push((SnapKind::Endpoint, *b));
        cands.push((SnapKind::Midpoint, a.lerp(*b, 0.5)));
    }
    for s in model.floors.iter().chain(&model.ceilings) {
        cands.extend(s.base.outer.iter().map(|q| (SnapKind::Endpoint, *q)));
    }
    for (i, (a, b)) in segs.iter().enumerate() {
        for (c, d) in &segs[i + 1..] {
            if let Some(x) = line_intersection(*a, b.sub(*a), *c, d.sub(*c)) {
                let on = |s: Pt, e: Pt| project_to_segment(x, s, e).1 < 1.0;
                if on(*a, *b) && on(*c, *d) {
                    cands.push((SnapKind::Intersection, x));
                }
            }
        }
    }
    for (a, b) in &segs {
        if let Some(f) = from {
            let (t, _) = project_to_segment(f, *a, *b);
            if t > 0.0 && t < 1.0 {
                cands.push((SnapKind::Perpendicular, a.lerp(*b, t)));
            }
        }
        let (t, d) = project_to_segment(p, *a, *b);
        if d < tol * 0.6 {
            cands.push((SnapKind::Nearest, a.lerp(*b, t)));
        }
    }

    let best = cands
        .into_iter()
        .filter(|(_, q)| q.dist(p) <= tol)
        .filter(|(_, q)| from.is_none_or(|f| f.dist(*q) > 1.0))
        .min_by(|(k1, q1), (k2, q2)| k1.cmp(k2).then(q1.dist(p).total_cmp(&q2.dist(p))));

    let (kind, pt) = match (best, from) {
        (Some(b), _) => b,
        (None, Some(f)) => angle_snap(f, p),
        (None, None) => (SnapKind::None, p),
    };
    let label = from.map(|f| {
        let v = pt.sub(f);
        let deg = v.y.atan2(v.x).to_degrees().rem_euclid(360.0);
        format!("{}  ·  {:.0}°", format_ft_in(v.len()), deg)
    });
    SnapResult { pt, kind, label }
}

/// Locks to 15° increments within 3°, with length rounded to the nearest inch.
fn angle_snap(from: Pt, p: Pt) -> (SnapKind, Pt) {
    let v = p.sub(from);
    let len = v.len();
    if len < 1.0 {
        return (SnapKind::None, p);
    }
    let ang = v.y.atan2(v.x);
    let step = 15f64.to_radians();
    let snapped = (ang / step).round() * step;
    if (ang - snapped).abs() > 3f64.to_radians() {
        return (SnapKind::None, p);
    }
    let l = (len / MM_PER_IN).round() * MM_PER_IN;
    (
        SnapKind::Angle,
        from.add(Pt::new(snapped.cos(), snapped.sin()).scale(l)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::{ops, Category};

    fn plan_with_wall() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let v = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        (doc, v)
    }

    #[test]
    fn endpoint_beats_midpoint_and_nearest() {
        let (doc, v) = plan_with_wall();
        let r = snap(&doc, v, Pt::new(4050.0, 30.0), None, 200.0);
        assert_eq!(r.kind, SnapKind::Endpoint);
        assert_eq!(r.pt, Pt::new(4000.0, 0.0));
        let r = snap(&doc, v, Pt::new(2030.0, 40.0), None, 200.0);
        assert_eq!(r.kind, SnapKind::Midpoint);
        let r = snap(&doc, v, Pt::new(1000.0, 50.0), None, 200.0);
        assert_eq!(r.kind, SnapKind::Nearest);
        assert!((r.pt.y).abs() < 1e-9 && (r.pt.x - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn angle_snap_locks_to_orthogonal_and_rounds_length() {
        let (doc, v) = plan_with_wall();
        let from = Pt::new(0.0, 5000.0);
        let r = snap(&doc, v, Pt::new(3050.0, 5100.0), Some(from), 50.0);
        assert_eq!(r.kind, SnapKind::Angle);
        assert!((r.pt.y - 5000.0).abs() < 1e-9);
        assert!((r.pt.x / MM_PER_IN - (r.pt.x / MM_PER_IN).round()).abs() < 1e-9);
        assert!(r.label.unwrap().ends_with("0°"));
    }

    #[test]
    fn perpendicular_to_wall_from_previous_point() {
        let (doc, v) = plan_with_wall();
        let r = snap(
            &doc,
            v,
            Pt::new(1510.0, 60.0),
            Some(Pt::new(1500.0, 3000.0)),
            200.0,
        );
        assert_eq!(r.kind, SnapKind::Perpendicular);
        assert_eq!(r.pt, Pt::new(1500.0, 0.0));
    }
}
