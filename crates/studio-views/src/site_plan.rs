//! Site graphics (ADR-023): property lines with bearings and distances, topographic
//! contours and a north arrow in site plans, and the ground in sections and elevations.

use super::{ring, Anchor, Builder, Dash, FillKind};
use studio_geom::Pt;
use studio_regen::SiteSolid;

/// A surveyor's bearing for a direction in the local east/north frame: N 45°12'30" E.
pub fn bearing(d: Pt) -> String {
    let az = d.x.atan2(d.y).to_degrees().rem_euclid(360.0);
    let (ns, ew, a) = if az <= 90.0 {
        ('N', 'E', az)
    } else if az <= 180.0 {
        ('S', 'E', 180.0 - az)
    } else if az <= 270.0 {
        ('S', 'W', az - 180.0)
    } else {
        ('N', 'W', 360.0 - az)
    };
    let total = (a * 3600.0).round() as i64;
    let (deg, min, sec) = (total / 3600, (total / 60) % 60, total % 60);
    format!("{ns} {deg}°{min:02}'{sec:02}\" {ew}")
}

/// Decimal feet, as property line distances read: 120.45'.
fn feet(mm: f64) -> String {
    format!("{:.2}'", mm / 304.8)
}

/// The lot's property line (long dash, short dashes), with bearing and distance along each
/// side when `labels`.
pub(crate) fn property_line(b: &mut Builder, s: &SiteSolid, labels: bool) {
    let el = Some(s.id);
    b.fill(el, vec![ring(&s.boundary)], FillKind::Room);
    b.line(el, &s.boundary, true, 4, Dash::Center);
    if !labels {
        return;
    }
    let n = s.boundary.len();
    // Outward: the lot is counter-clockwise, so outside is to the right of each side.
    for i in 0..n {
        let (a, c) = (s.boundary[i], s.boundary[(i + 1) % n]);
        let (la, lc) = (s.boundary_local[i], s.boundary_local[(i + 1) % n]);
        let len = a.dist(c);
        if len < b.paper(12.0) {
            continue;
        }
        let d = c.sub(a).norm();
        let out = Pt::new(d.y, -d.x);
        let mid = a.lerp(c, 0.5);
        // Text reads left to right (or bottom to top).
        let mut ang = d.y.atan2(d.x);
        if ang > std::f64::consts::FRAC_PI_2 || ang < -std::f64::consts::FRAC_PI_2 {
            ang += std::f64::consts::PI;
        }
        b.text_rot(
            el,
            mid.add(out.scale(b.paper(2.5))),
            bearing(lc.sub(la)),
            2.2,
            Anchor::Center,
            ang,
        );
        b.text_rot(
            el,
            mid.sub(out.scale(b.paper(3.5))),
            feet(len),
            2.2,
            Anchor::Center,
            ang,
        );
    }
}

/// Contours: minor ones thin, every fifth heavier and labeled with its elevation.
pub(crate) fn contours(b: &mut Builder, s: &SiteSolid) {
    let el = Some(s.id);
    for (level, major, segs) in s.contours() {
        for seg in &segs {
            b.line(el, seg, false, if major { 2 } else { 1 }, Dash::Solid);
        }
        if major {
            // Label on the longest segment of this contour.
            if let Some([p, q]) = segs
                .iter()
                .max_by(|x, y| x[0].dist(x[1]).total_cmp(&y[0].dist(y[1])))
            {
                let d = q.sub(*p).norm();
                let mut ang = d.y.atan2(d.x);
                if ang.abs() > std::f64::consts::FRAC_PI_2 {
                    ang += std::f64::consts::PI;
                }
                b.text_rot(
                    el,
                    p.lerp(*q, 0.5),
                    format!("{:.0}", level / 304.8),
                    2.0,
                    Anchor::Center,
                    ang,
                );
            }
        }
    }
}

/// A north arrow (true north) beside the lot.
pub(crate) fn north_arrow(b: &mut Builder, s: &SiteSolid) {
    let Some((_, hi)) = studio_geom::bounds_of(&s.boundary) else {
        return;
    };
    let c = hi.add(Pt::new(b.paper(18.0), -b.paper(6.0)));
    let up = Pt::new(-s.rotation.sin(), s.rotation.cos());
    let side = up.perp().scale(-b.paper(3.0));
    let tip = c.add(up.scale(b.paper(8.0)));
    let tail = c.sub(up.scale(b.paper(8.0)));
    b.circle(None, c, 6.5, 2, false);
    b.fill(None, vec![ring(&[tip, c.add(side), tail])], FillKind::Ink);
    b.line(None, &[tip, c.sub(side), tail], false, 2, Dash::Solid);
    b.text(
        None,
        tip.add(up.scale(b.paper(3.2))),
        "N".into(),
        3.2,
        Anchor::Center,
    );
}

/// The ground where a section cuts it: its profile over the section's width, filled down
/// to below the lowest point. `u_of` and `along` map between plan points and the cut's
/// u coordinate.
pub(crate) fn ground_cut(s: &SiteSolid, origin: Pt, right: Pt, length: f64) -> Option<Vec<Pt>> {
    let step = s.topo.as_ref()?.spacing / 2.0;
    let n = ((length / step).ceil() as usize).clamp(2, 4000);
    let mut top = vec![];
    for k in 0..=n {
        let u = length * k as f64 / n as f64;
        if let Some(z) = s.ground_at(origin.add(right.scale(u))) {
            top.push(Pt::new(u, z));
        }
    }
    if top.len() < 2 {
        return None;
    }
    let low = top.iter().map(|p| p.y).fold(f64::INFINITY, f64::min) - 3.0 * 304.8;
    let (u0, u1) = (top[0].x, top[top.len() - 1].x);
    let mut poly = top;
    poly.push(Pt::new(u1, low));
    poly.push(Pt::new(u0, low));
    Some(poly)
}

/// The ground line of an elevation: the highest ground along each vertical of the view.
pub(crate) fn ground_line(s: &SiteSolid, u_of: &dyn Fn(Pt) -> f64) -> Vec<Pt> {
    let Some(t) = &s.topo else {
        return vec![];
    };
    let bin = t.spacing;
    let mut best: std::collections::BTreeMap<i64, f64> = Default::default();
    for j in 0..t.ny {
        for i in 0..t.nx {
            let p = s.place(t.node(i, j));
            let z = t.at(i, j) - s.datum;
            let k = (u_of(p) / bin).round() as i64;
            let e = best.entry(k).or_insert(f64::NEG_INFINITY);
            *e = e.max(z);
        }
    }
    best.into_iter()
        .map(|(k, z)| Pt::new(k as f64 * bin, z))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearings_read_like_a_survey() {
        assert_eq!(bearing(Pt::new(1.0, 1.0)), "N 45°00'00\" E");
        assert_eq!(bearing(Pt::new(0.0, -1.0)), "S 0°00'00\" E");
        assert_eq!(bearing(Pt::new(-1.0, 0.0)), "S 90°00'00\" W");
        let a = (12.5f64).to_radians();
        assert_eq!(bearing(Pt::new(-a.sin(), a.cos())), "N 12°30'00\" W");
    }
}
