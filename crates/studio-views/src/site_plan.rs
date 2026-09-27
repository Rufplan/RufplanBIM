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

/// Contours, as a survey draws them (ADR-046): minor ones thin, every fifth heavier, and
/// each labeled with its elevation along it, in a clear gap. With many contours only the
/// heavier ones are labeled.
pub(crate) fn contours(b: &mut Builder, s: &SiteSolid) {
    let el = Some(s.id);
    let all = s.contours();
    let label_all = all.iter().filter(|c| !c.2.is_empty()).count() <= 25;
    for (level, major, segs) in all {
        for seg in &segs {
            b.line(el, seg, false, if major { 3 } else { 1 }, Dash::Solid);
        }
        if !(major || label_all) {
            continue;
        }
        // On the longest run of this contour, read along it and kept upright.
        let Some([p, q]) = segs
            .iter()
            .max_by(|x, y| x[0].dist(x[1]).total_cmp(&y[0].dist(y[1])))
        else {
            continue;
        };
        let d = q.sub(*p).norm();
        let mut ang = d.y.atan2(d.x);
        let mut u = d;
        if ang.abs() > std::f64::consts::FRAC_PI_2 {
            ang += std::f64::consts::PI;
            u = u.scale(-1.0);
        }
        let text = crate::terrain::elevation_text(level);
        let size = 2.4;
        // A paper-white gap behind the label, so the line breaks around it.
        let w = b.paper(size * 0.62 * text.chars().count() as f64 + 1.6);
        let h = b.paper(size + 1.0);
        let c = p.lerp(*q, 0.5);
        let n = u.perp();
        let corners = [
            c.sub(u.scale(w / 2.0)).sub(n.scale(h / 2.0)),
            c.add(u.scale(w / 2.0)).sub(n.scale(h / 2.0)),
            c.add(u.scale(w / 2.0)).add(n.scale(h / 2.0)),
            c.sub(u.scale(w / 2.0)).add(n.scale(h / 2.0)),
        ];
        b.fill(el, vec![ring(&corners)], FillKind::Paper);
        b.text_rot(el, c, text, size, Anchor::Center, ang);
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
    fn a_site_plan_labels_each_contour_with_its_elevation() {
        use studio_core::site::{set_lot, set_topo, topo_request, GeoFrame, ParcelInfo};
        use studio_core::units::MM_PER_FT;
        use studio_core::{Category, Document, ElementData};
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let ring = [
            (37.7749, -122.4194),
            (37.7749, -122.41906),
            (37.77526, -122.41906),
            (37.77526, -122.4194),
        ];
        set_lot(&mut doc, &ring, ParcelInfo::default()).unwrap();
        let (nx, ny, origin, pts) = topo_request(&doc, 5.0 * MM_PER_FT, 10.0 * MM_PER_FT).unwrap();
        let frame = GeoFrame {
            lat0: ring.iter().map(|p| p.0).sum::<f64>() / 4.0,
            lon0: ring.iter().map(|p| p.1).sum::<f64>() / 4.0,
        };
        // Rises 1' for every 10' north.
        let m: Vec<Option<f64>> = pts
            .iter()
            .map(|(la, lo)| Some(30.0 + frame.to_local(*la, *lo).y / 10.0 / 1000.0))
            .collect();
        set_topo(&mut doc, (nx, ny, origin), 5.0 * MM_PER_FT, &m, 1.0).unwrap();
        let site = doc.of(Category::Site).next().unwrap().id;
        let view = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { site: true, .. }))
            .unwrap()
            .id;
        let dl = crate::display_list(&doc, view).unwrap();
        let labels: Vec<&str> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(site))
            .filter_map(|i| match &i.prim {
                // Contour elevations (the property line's distances have decimals).
                crate::Prim::Text { text, .. } if text.ends_with('\x27') && !text.contains('.') => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        let contours = regenerate_site_contours(&doc);
        assert!(contours > 3);
        // Few contours: every one is labeled, each in a paper gap.
        assert_eq!(labels.len(), contours, "{labels:?}");
        let gaps = dl
            .items
            .iter()
            .filter(|i| {
                i.el == Some(site)
                    && matches!(
                        i.prim,
                        crate::Prim::Fill {
                            fill: FillKind::Paper,
                            ..
                        }
                    )
            })
            .count();
        assert_eq!(gaps, contours);
    }

    fn regenerate_site_contours(doc: &studio_core::Document) -> usize {
        studio_regen::regenerate(doc)
            .site
            .as_ref()
            .unwrap()
            .contours()
            .iter()
            .filter(|c| !c.2.is_empty())
            .count()
    }

    #[test]
    fn bearings_read_like_a_survey() {
        assert_eq!(bearing(Pt::new(1.0, 1.0)), "N 45°00'00\" E");
        assert_eq!(bearing(Pt::new(0.0, -1.0)), "S 0°00'00\" E");
        assert_eq!(bearing(Pt::new(-1.0, 0.0)), "S 90°00'00\" W");
        let a = (12.5f64).to_radians();
        assert_eq!(bearing(Pt::new(-a.sin(), a.cos())), "N 12°30'00\" W");
    }
}
