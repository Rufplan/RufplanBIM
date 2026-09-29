//! In-place elements in views (ADR-068): cut or seen in plans by their category, their
//! silhouettes and cuts in elevations and sections, and their meshes in 3D.

use studio_core::{Category, Document, ElementId};
use studio_geom::{bounds_of, even_odd_in_rect, mesh_section, signed_area, union_all, Poly, Pt};
use studio_regen::inplace::{solids, InPlaceSolid};

use super::{ring, Builder, Dash, FillKind, Mesh};

/// Structure is cut solid; everything else lighter, as Revit's cut patterns tend to be.
fn cut_fill(c: Category) -> FillKind {
    match c {
        Category::Wall
        | Category::Column
        | Category::Beam
        | Category::Floor
        | Category::Roof
        | Category::Stair => FillKind::Poche,
        _ => FillKind::PocheLight,
    }
}

/// Where a plane cuts closed meshes, given each one's loops: every piece's inside, joined.
fn cut_regions(pieces: impl Iterator<Item = Vec<Vec<Pt>>>) -> Vec<Poly> {
    let mut polys = vec![];
    for loops in pieces {
        let pts: Vec<Pt> = loops.concat();
        let Some((lo, hi)) = bounds_of(&pts) else {
            continue;
        };
        let m = Pt::new(10.0, 10.0);
        polys.extend(even_odd_in_rect(&loops, lo.sub(m), hi.add(m)));
    }
    if polys.len() > 1 {
        union_all(&polys)
    } else {
        polys
    }
}

/// The outline of flat polygons (triangles seen along an axis), joined.
fn silhouette(polys: impl Iterator<Item = Vec<Pt>>) -> Vec<Poly> {
    let parts: Vec<Poly> = polys
        .filter(|r| r.len() >= 3 && signed_area(r).abs() > 1.0)
        .map(|mut r| {
            if signed_area(&r) < 0.0 {
                r.reverse();
            }
            Poly::simple(r)
        })
        .collect();
    union_all(&parts)
}

fn tris(t: &[f32]) -> impl Iterator<Item = [[f64; 3]; 3]> + '_ {
    t.as_chunks::<9>().0.iter().map(|c| {
        let v = |k: usize| {
            [
                f64::from(c[3 * k]),
                f64::from(c[3 * k + 1]),
                f64::from(c[3 * k + 2]),
            ]
        };
        [v(0), v(1), v(2)]
    })
}

/// In a plan cut at `cut` on the level at `elev`: what the cut plane passes through (for
/// cuttable categories), in poché with a heavy outline; below it, the footprint in thin
/// lines. Ceiling plans show only the cut.
pub(crate) fn in_plan(doc: &Document, b: &mut Builder, elev: f64, cut: f64, ceiling: bool) {
    for s in solids(doc) {
        let el = Some(s.id);
        let through = s.z0 < cut && s.z1 > cut;
        if through && s.cuttable() {
            let regions = cut_regions(s.pieces.iter().map(|p| mesh_section(p, 2, cut)));
            for r in &regions {
                let mut rings = vec![ring(&r.outer)];
                rings.extend(r.holes.iter().map(|h| ring(h)));
                b.fill(el, rings, cut_fill(s.category));
                b.line(el, &r.outer, true, 4, Dash::Solid);
                for h in &r.holes {
                    b.line(el, h, true, 4, Dash::Solid);
                }
            }
            continue;
        }
        // Seen from above: anything of it below the cut, standing on this floor.
        if ceiling || s.z0 >= cut || s.z1 <= elev - 1.0 {
            continue;
        }
        footprint(b, &s, cut);
    }
}

/// The plan outline of what's below `cut`, pickable inside.
fn footprint(b: &mut Builder, s: &InPlaceSolid, cut: f64) {
    let el = Some(s.id);
    let outline = silhouette(s.pieces.iter().flat_map(|p| {
        tris(p)
            .filter(|t| t.iter().any(|v| v[2] < cut))
            .map(|t| t.iter().map(|v| Pt::new(v[0], v[1])).collect::<Vec<Pt>>())
            .collect::<Vec<_>>()
    }));
    for r in &outline {
        b.fill(el, vec![ring(&r.outer)], FillKind::Room);
        b.line(el, &r.outer, true, 2, Dash::Solid);
        for h in &r.holes {
            b.line(el, h, true, 2, Dash::Solid);
        }
    }
}

/// An in-place element as an elevation or section sees it, in the view's (u, z).
pub(crate) struct Seen {
    pub id: ElementId,
    pub fill: FillKind,
    pub silhouettes: Vec<Poly>,
    /// Nearest and middle depth, for the painter's order.
    pub near: f64,
    pub mid: f64,
    /// Where a section's plane cuts it.
    pub cut: Vec<Poly>,
}

/// Clips a polygon of (u, depth, z) points to `depth >= lo`.
fn clip_depth(poly: &[[f64; 3]], lo: f64, keep_above: bool) -> Vec<[f64; 3]> {
    let d = |v: &[f64; 3]| if keep_above { v[1] - lo } else { lo - v[1] };
    let n = poly.len();
    let mut out = vec![];
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (da, db) = (d(&a), d(&b));
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]);
        }
    }
    out
}

/// In-place elements seen from `origin` looking along `look` (`right` across): for a
/// section, `cut` = (its width, its depth).
pub(crate) fn in_view(
    doc: &Document,
    origin: Pt,
    right: Pt,
    look: Pt,
    cut: Option<(f64, f64)>,
) -> Vec<Seen> {
    let to_view = |v: [f64; 3]| {
        let p = Pt::new(v[0], v[1]).sub(origin);
        [p.dot(right), p.dot(look), v[2]]
    };
    let mut out = vec![];
    for s in solids(doc) {
        let pieces: Vec<Vec<[[f64; 3]; 3]>> = s
            .pieces
            .iter()
            .map(|p| tris(p).map(|t| t.map(to_view)).collect())
            .collect();
        let all = || pieces.iter().flatten().flatten();
        let dmin = all().map(|v| v[1]).fold(f64::INFINITY, f64::min);
        let dmax = all().map(|v| v[1]).fold(f64::NEG_INFINITY, f64::max);
        let (umin, umax) = all().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(v[0]), b.max(v[0]))
        });
        if let Some((width, depth)) = cut {
            if dmax <= 0.0 || dmin >= depth || umax <= 0.0 || umin >= width {
                continue;
            }
        }
        // What lies beyond the cut plane (and short of the far clip), flattened.
        let silhouettes = silhouette(all_tris(&pieces).filter_map(|t| {
            let mut poly = t.to_vec();
            if let Some((_, depth)) = cut {
                poly = clip_depth(&poly, 0.0, true);
                poly = clip_depth(&poly, depth, false);
            }
            (poly.len() >= 3).then(|| poly.iter().map(|v| Pt::new(v[0], v[2])).collect())
        }));
        let cut_polys = match cut {
            Some((width, _)) if dmin < 0.0 && dmax > 0.0 => {
                let regions = cut_regions(pieces.iter().map(|p| {
                    let flat: Vec<f32> = p.iter().flatten().flatten().map(|x| *x as f32).collect();
                    mesh_section(&flat, 1, 0.0)
                }));
                // Only the section's width.
                regions
                    .into_iter()
                    .flat_map(|r| {
                        let loops: Vec<Vec<Pt>> = std::iter::once(r.outer).chain(r.holes).collect();
                        even_odd_in_rect(&loops, Pt::new(0.0, -1.0e7), Pt::new(width, 1.0e7))
                    })
                    .collect()
            }
            _ => vec![],
        };
        out.push(Seen {
            id: s.id,
            fill: if s.category == Category::Window {
                FillKind::Glass
            } else {
                FillKind::Paper
            },
            silhouettes,
            near: dmin.max(0.0),
            mid: (dmin + dmax) / 2.0,
            cut: cut_polys,
        });
    }
    out
}

fn all_tris(pieces: &[Vec<[[f64; 3]; 3]>]) -> impl Iterator<Item = &[[f64; 3]; 3]> {
    pieces.iter().flatten()
}

/// 3D meshes, in the element's category and material.
pub(crate) fn meshes(doc: &Document, out: &mut Vec<Mesh>) {
    for s in solids(doc) {
        let color = studio_core::material::resolve_type(doc, s.material, "").color;
        out.push(Mesh {
            el: s.id,
            category: s.category,
            exterior: false,
            color: Some(color),
            material: s.material,
            level: Some(s.level),
            positions: s.triangles(),
            edges: vec![],
            glow: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{display_list, DisplayList, Prim};
    use studio_core::inplace::{add_form, create, Form, FormKind};
    use studio_core::sketch::SketchCurve;
    use studio_core::{ops, ElementData, ViewKind};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<SketchCurve> {
        let p = [
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ];
        (0..4)
            .map(|i| SketchCurve::line(p[i], p[(i + 1) % 4]))
            .collect()
    }

    fn fills(dl: &DisplayList, id: ElementId) -> Vec<FillKind> {
        dl.items
            .iter()
            .filter(|i| i.el == Some(id))
            .filter_map(|i| match &i.prim {
                Prim::Fill { fill, .. } => Some(*fill),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn plans_cut_or_show_in_place_elements_by_category() {
        let mut doc = studio_core::Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        // A 10' tall in-place wall and a 3' tall base cabinet.
        let make = |doc: &mut studio_core::Document, cat, r: Vec<SketchCurve>, end: f64| {
            let id = create(doc, cat, None, l1).unwrap();
            add_form(
                doc,
                id,
                Form {
                    kind: FormKind::Extrusion { start: 0.0, end },
                    sketch: vec![r],
                    void: false,
                },
            )
            .unwrap();
            id
        };
        let wall = make(
            &mut doc,
            Category::Wall,
            rect(0.0, 0.0, 4000.0, 200.0),
            3048.0,
        );
        let cab = make(
            &mut doc,
            Category::Casework,
            rect(0.0, 1000.0, 2000.0, 1600.0),
            914.0,
        );
        let dl = display_list(&doc, plan).unwrap();
        // The wall is cut (solid poché); the cabinet is below the cut, seen in outline.
        assert_eq!(fills(&dl, wall), vec![FillKind::Poche]);
        assert_eq!(fills(&dl, cab), vec![FillKind::Room]);
        // Its cut is the wall's rectangle.
        let area: f64 = dl
            .items
            .iter()
            .filter(|i| i.el == Some(wall))
            .find_map(|i| match &i.prim {
                Prim::Fill { rings, .. } => Some(
                    signed_area(
                        &rings[0]
                            .iter()
                            .map(|p| Pt::new(p[0], p[1]))
                            .collect::<Vec<_>>(),
                    )
                    .abs(),
                ),
                _ => None,
            })
            .unwrap();
        assert!((area - 4000.0 * 200.0).abs() < 100.0, "{area}");
        // Furniture isn't cut even when the plane passes through it.
        ops::set_property(&mut doc, wall, "category", "Furniture", 0).unwrap();
        let dl = display_list(&doc, plan).unwrap();
        assert_eq!(fills(&dl, wall), vec![FillKind::Room]);
        // 3D: a mesh in its category.
        let m = crate::meshes(&doc);
        assert!(m
            .iter()
            .any(|m| m.el == cab && m.category == Category::Casework));
    }

    #[test]
    fn sections_cut_and_elevations_see_in_place_elements() {
        let mut doc = studio_core::Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let id = create(&mut doc, Category::GenericModel, None, l1).unwrap();
        add_form(
            &mut doc,
            id,
            Form {
                kind: FormKind::Extrusion {
                    start: 0.0,
                    end: 1000.0,
                },
                sketch: vec![rect(0.0, 0.0, 2000.0, 1000.0)],
                void: false,
            },
        )
        .unwrap();
        let south = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Elevation {
                            facing: studio_core::Compass::South
                        },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        let dl = display_list(&doc, south).unwrap();
        assert!(fills(&dl, id).contains(&FillKind::Paper));
        // A section through its middle cuts a 1000 × 1000 square.
        let sec = ops::create_section(&mut doc, Pt::new(1000.0, 3000.0), Pt::new(1000.0, -3000.0))
            .unwrap();
        let dl = display_list(&doc, sec).unwrap();
        let cut: Vec<f64> = dl
            .items
            .iter()
            .filter(|i| i.el == Some(id))
            .filter_map(|i| match &i.prim {
                Prim::Fill {
                    rings,
                    fill: FillKind::Poche,
                } => Some(
                    signed_area(
                        &rings[0]
                            .iter()
                            .map(|p| Pt::new(p[0], p[1]))
                            .collect::<Vec<_>>(),
                    )
                    .abs(),
                ),
                _ => None,
            })
            .collect();
        assert_eq!(cut.len(), 1);
        assert!((cut[0] - 1.0e6).abs() < 1.0e3, "{cut:?}");
    }
}
