//! Section box caps (ADR-044), as Revit draws them: where the box cuts through the model,
//! each element's cut is filled in its colour and outlined with a heavy cut line, and a
//! wall's cut shows the boundaries between its layers.

use serde::Serialize;
use studio_core::{Category, Document, ElementData, ElementId, SectionBox};
use studio_geom::{even_odd_in_rect, mesh_section, point_in_ring, triangulate, Poly, Pt};
use studio_regen::regenerate;
use ts_rs::TS;

use crate::{meshes_in_view, Mesh};

/// One element's cut on one face of the section box.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Cap {
    pub el: ElementId,
    pub category: Category,
    /// The element's shaded colour, when it has one (else by category, as its mesh).
    pub color: Option<[u8; 3]>,
    pub exterior: bool,
    /// The cut, filled: triangles, 9 floats each (mm, z-up), on the box face.
    pub positions: Vec<f32>,
    /// The cut's outline, 6 floats a segment.
    pub outline: Vec<f32>,
    /// Lines inside the cut (a wall's layer boundaries), 6 floats a segment.
    pub inner: Vec<f32>,
}

/// The caps of a 3D view's section box: every element it cuts, on each of its six faces.
pub fn section_caps(doc: &Document, view: ElementId) -> Vec<Cap> {
    let Ok(ElementData::View {
        section_box: Some(b),
        ..
    }) = doc.data(view)
    else {
        return vec![];
    };
    let b = *b;
    let mut meshes = meshes_in_view(doc, Some(view));
    let model = regenerate(doc);
    // The site closes into a block with its earth sides (ADR-045), so it caps too.
    if let Some(s) = &model.site {
        if let Some(low) = s.lowest() {
            let skirt = s.skirt(low - crate::terrain::TERRAIN_DEPTH);
            for m in meshes.iter_mut().filter(|m| m.category == Category::Site) {
                m.positions.extend_from_slice(&skirt);
            }
        }
    }
    let mut out = vec![];
    for axis in 0..3 {
        for at in [b.min[axis], b.max[axis]] {
            for m in &meshes {
                if let Some(cap) = cap_of(m, &b, axis, at, &model) {
                    out.push(cap);
                }
            }
        }
    }
    out
}

/// 3D point from a point in the plane's coordinates (the other two axes, in order).
fn lift(axis: usize, at: f64, p: Pt) -> [f64; 3] {
    match axis {
        0 => [at, p.x, p.y],
        1 => [p.x, at, p.y],
        _ => [p.x, p.y, at],
    }
}

fn cap_of(
    m: &Mesh,
    b: &SectionBox,
    axis: usize,
    at: f64,
    model: &studio_regen::Model,
) -> Option<Cap> {
    if m.positions.is_empty() {
        return None;
    }
    // Only elements that reach across the plane.
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for c in m.positions.as_chunks::<3>().0 {
        let v = f64::from(c[axis]);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo >= at || hi <= at {
        return None;
    }
    let loops = mesh_section(&m.positions, axis, at);
    if loops.is_empty() {
        return None;
    }
    let (iu, iv) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let rect_lo = Pt::new(b.min[iu], b.min[iv]);
    let rect_hi = Pt::new(b.max[iu], b.max[iv]);
    let region = even_odd_in_rect(&loops, rect_lo, rect_hi);
    if region.is_empty() {
        return None;
    }
    let mut positions = vec![];
    let mut outline = vec![];
    let push_seg = |v: &mut Vec<f32>, a: Pt, c: Pt| {
        v.extend(
            lift(axis, at, a)
                .into_iter()
                .chain(lift(axis, at, c))
                .map(|x| x as f32),
        );
    };
    for p in &region {
        let (verts, tris) = triangulate(p);
        for t in tris {
            for i in t {
                positions.extend(lift(axis, at, verts[i]).map(|x| x as f32));
            }
        }
        for ring in std::iter::once(&p.outer).chain(&p.holes) {
            for i in 0..ring.len() {
                push_seg(&mut outline, ring[i], ring[(i + 1) % ring.len()]);
            }
        }
    }
    let mut inner = vec![];
    if m.category == Category::Wall {
        if let Some(w) = model.walls.iter().find(|w| w.id == m.el) {
            for line in layer_lines(w, axis, at) {
                for (a, c) in clip_to(&region, line.0, line.1) {
                    push_seg(&mut inner, a, c);
                }
            }
        }
    }
    Some(Cap {
        el: m.el,
        category: m.category,
        color: m.color,
        exterior: m.exterior,
        positions,
        outline,
        inner,
    })
}

/// A wall's layer boundaries where the plane cuts them, in the plane's coordinates.
fn layer_lines(w: &studio_regen::WallSolid, axis: usize, at: f64) -> Vec<(Pt, Pt)> {
    let n = w.dir().perp();
    let mut out = vec![];
    for off in &w.layers {
        let (a, c) = (w.start.add(n.scale(*off)), w.end.add(n.scale(*off)));
        match axis {
            // A level cut: the boundary itself, in plan.
            2 => out.push((a, c)),
            // A vertical cut: a vertical line where the boundary crosses it.
            _ => {
                let (ka, kc) = if axis == 0 { (a.x, c.x) } else { (a.y, c.y) };
                if (ka - at) * (kc - at) > 0.0 || (ka - kc).abs() < 1e-9 {
                    continue;
                }
                let s = (at - ka) / (kc - ka);
                let q = a.lerp(c, s);
                let u = if axis == 0 { q.y } else { q.x };
                out.push((Pt::new(u, w.z0), Pt::new(u, w.z1)));
            }
        }
    }
    out
}

/// The parts of segment a→c inside `region`.
fn clip_to(region: &[Poly], a: Pt, c: Pt) -> Vec<(Pt, Pt)> {
    let d = c.sub(a);
    let mut ts = vec![0.0, 1.0];
    for p in region {
        for ring in std::iter::once(&p.outer).chain(&p.holes) {
            for i in 0..ring.len() {
                let (p0, p1) = (ring[i], ring[(i + 1) % ring.len()]);
                let e = p1.sub(p0);
                let den = d.cross(e);
                if den.abs() < 1e-12 {
                    continue;
                }
                let t = p0.sub(a).cross(e) / den;
                let s = p0.sub(a).cross(d) / den;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&s) {
                    ts.push(t);
                }
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.windows(2)
        .filter(|w| w[1] - w[0] > 1e-9)
        .filter_map(|w| {
            let mid = a.add(d.scale((w[0] + w[1]) / 2.0));
            let inside = region.iter().any(|p| {
                point_in_ring(mid, &p.outer) && !p.holes.iter().any(|h| point_in_ring(mid, h))
            });
            inside.then(|| (a.add(d.scale(w[0])), a.add(d.scale(w[1]))))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ops;
    use studio_core::units::MM_PER_FT;
    use studio_core::ViewKind;

    #[test]
    fn a_section_box_caps_each_wall_it_cuts_with_its_layers() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let ft = MM_PER_FT;
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(40.0 * ft, 0.0),
            Pt::new(40.0 * ft, 30.0 * ft),
            Pt::new(0.0, 30.0 * ft),
        ];
        for i in 0..4 {
            ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap();
        }
        let v3d = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::ThreeD,
                        camera: None,
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        // No box, no caps.
        assert!(section_caps(&doc, v3d).is_empty());
        // A box whose top cuts the walls 4' up, and whose east face cuts through the
        // north and south walls at 30'.
        studio_regen::derived::set_section_box(
            &mut doc,
            v3d,
            Some(SectionBox {
                min: [-2000.0, -2000.0, -500.0],
                max: [30.0 * ft, 12000.0, 4.0 * ft],
            }),
        )
        .unwrap();
        let caps = section_caps(&doc, v3d);
        let top: Vec<&Cap> = caps
            .iter()
            .filter(|c| {
                c.positions
                    .chunks(3)
                    .all(|p| (f64::from(p[2]) - 4.0 * ft).abs() < 0.01)
            })
            .collect();
        // The top face cuts all four walls (the east wall is outside the box).
        assert_eq!(
            top.iter().filter(|c| c.category == Category::Wall).count(),
            3
        );
        let area = |c: &Cap| {
            c.positions
                .chunks(9)
                .map(|t| {
                    let (a, b, d) = (
                        Pt::new(t[0].into(), t[1].into()),
                        Pt::new(t[3].into(), t[4].into()),
                        Pt::new(t[6].into(), t[7].into()),
                    );
                    b.sub(a).cross(d.sub(a)).abs() / 2.0
                })
                .sum::<f64>()
        };
        // Each is a band 8" thick; the west wall's runs its whole length, mitered at the
        // corners (a trapezoid: 8" by 30' on average), all within the box.
        let west = top
            .iter()
            .find(|c| c.positions.chunks(3).all(|p| f64::from(p[0]) < 200.0))
            .unwrap();
        let t = 8.0 * 25.4;
        assert!((area(west) - t * 30.0 * ft).abs() < 1.0, "{}", area(west));
        // The cut line goes round it, and its layer boundaries show inside.
        assert!(!west.outline.is_empty());
        assert!(!west.inner.is_empty(), "layer lines");
        // The east face (x = 30') cuts the north and south walls: vertical caps.
        let east = caps
            .iter()
            .filter(|c| {
                c.positions
                    .chunks(3)
                    .all(|p| (f64::from(p[0]) - 30.0 * ft).abs() < 0.01)
            })
            .count();
        assert_eq!(east, 2);
    }
}
