//! Dimensions on a generated building's plans (ADR-099), as a CD set dimensions them: each
//! face of the outline, outside face to outside face, and the overall length and width.

use studio_geom::{offset_ring, signed_area, union_all, Poly, Pt};

use super::StoryPlan;
use crate::document::{CoreResult, Document};
use crate::element::{Category, CropBox, ElementData, ElementId, ViewKind};
use crate::units::MM_PER_FT;

/// Dimensions every floor plan of a level built from `plans`: a string along each face of
/// its outline (`face` mm outside the room edges, the walls' outer faces) 4' out, and its
/// overall length and width 10' out. Returns how many.
pub(super) fn dimension_plans(
    doc: &mut Document,
    plans: &[StoryPlan],
    face: f64,
) -> CoreResult<usize> {
    let mut n = 0;
    for p in plans {
        let views: Vec<ElementId> = doc
            .of(Category::View)
            .filter(|e| {
                matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == p.level)
            })
            .map(|e| e.id)
            .collect();
        if views.is_empty() || p.indoor.is_empty() {
            continue;
        }
        let outline = union_all(&p.indoor.iter().map(|r| r.1.poly()).collect::<Vec<Poly>>());
        let mut lo = Pt::new(f64::MAX, f64::MAX);
        let mut hi = Pt::new(f64::MIN, f64::MIN);
        let mut faces: Vec<(Pt, Pt)> = vec![];
        for poly in &outline {
            let mut ring = poly.outer.clone();
            if signed_area(&ring) < 0.0 {
                ring.reverse();
            }
            let ring = offset_ring(&ring, face);
            for (i, a) in ring.iter().enumerate() {
                let b = ring[(i + 1) % ring.len()];
                lo = Pt::new(lo.x.min(a.x), lo.y.min(a.y));
                hi = Pt::new(hi.x.max(a.x), hi.y.max(a.y));
                if a.dist(b) >= 2.0 * MM_PER_FT {
                    faces.push((*a, b));
                }
            }
        }
        for v in views {
            // Counter-clockwise, the outside is to the right: a negative offset.
            for (a, b) in &faces {
                if crate::ops::create_dimension(doc, v, *a, *b, -4.0 * MM_PER_FT).is_ok() {
                    n += 1;
                }
            }
            let overall = [
                (Pt::new(lo.x, lo.y), Pt::new(hi.x, lo.y)),
                (Pt::new(hi.x, lo.y), Pt::new(hi.x, hi.y)),
                (Pt::new(hi.x, hi.y), Pt::new(lo.x, hi.y)),
                (Pt::new(lo.x, hi.y), Pt::new(lo.x, lo.y)),
            ];
            for (a, b) in overall {
                if crate::ops::create_dimension(doc, v, a, b, -10.0 * MM_PER_FT).is_ok() {
                    n += 1;
                }
            }
        }
    }
    Ok(n)
}

/// Crops the building's plans and ceiling plans to it, its porches and terraces with
/// `margin` round them (room for the dimensions), so the trees round the lot stay off
/// the sheets.
pub(super) fn crop_plans(doc: &mut Document, plans: &[StoryPlan], margin: f64) -> CoreResult<()> {
    let mut lo = Pt::new(f64::MAX, f64::MAX);
    let mut hi = Pt::new(f64::MIN, f64::MIN);
    for p in plans {
        for r in p
            .indoor
            .iter()
            .map(|r| &r.1)
            .chain(p.outdoor.iter().map(|o| &o.1))
        {
            lo = Pt::new(lo.x.min(r.x0), lo.y.min(r.y0));
            hi = Pt::new(hi.x.max(r.x1), hi.y.max(r.y1));
        }
    }
    if lo.x > hi.x {
        return Ok(());
    }
    let levels: Vec<ElementId> = plans.iter().map(|p| p.level).collect();
    let views: Vec<ElementId> = doc
        .of(Category::View)
        .filter(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
                ..
            } => levels.contains(level),
            _ => false,
        })
        .map(|e| e.id)
        .collect();
    let crop = CropBox {
        min: Pt::new(lo.x - margin, lo.y - margin),
        max: Pt::new(hi.x + margin, hi.y + margin),
    };
    for v in views {
        crate::edit::set_crop(doc, v, Some(crop))?;
    }
    Ok(())
}
