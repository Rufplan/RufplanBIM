//! Room separation lines, callouts (detail views) and column location marks (ADR-020).

use studio_geom::{line_intersection, project_to_segment, Pt};

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, Compass, CropBox, ElementData, ElementId, ViewKind};

/// Scale of a new callout: 1 1/2" = 1'-0", where layer wraps and cut patterns read.
pub const CALLOUT_SCALE: u32 = 8;

/// A room separation line on `level` (bounds rooms like a wall, for open plans).
pub fn create_room_separator(
    doc: &mut Document,
    level: ElementId,
    start: Pt,
    end: Pt,
) -> CoreResult<ElementId> {
    doc.transact("Create room separator", |tx| {
        if !matches!(tx.data(level)?, ElementData::Level { .. }) {
            return Err(CoreError::Invalid("room separators go on a level".into()));
        }
        Ok(tx.insert(ElementData::RoomSeparator { level, start, end }))
    })
}

/// A callout of `parent` (a plan, elevation or section): a new view of the region between
/// corners `a` and `b` (view coordinates) at 1 1/2" = 1'-0", with a marker in the parent.
pub fn create_callout(
    doc: &mut Document,
    parent: ElementId,
    a: Pt,
    b: Pt,
) -> CoreResult<ElementId> {
    let ElementData::View {
        name: parent_name,
        kind,
        ..
    } = doc.data(parent)?.clone()
    else {
        return Err(CoreError::Invalid("callouts are drawn in a view".into()));
    };
    if !matches!(
        kind,
        ViewKind::FloorPlan { .. }
            | ViewKind::CeilingPlan { .. }
            | ViewKind::Elevation { .. }
            | ViewKind::Section { .. }
    ) {
        return Err(CoreError::Invalid(
            "draw callouts in a plan, elevation or section".into(),
        ));
    }
    let (min, max) = (
        Pt::new(a.x.min(b.x), a.y.min(b.y)),
        Pt::new(a.x.max(b.x), a.y.max(b.y)),
    );
    if max.x - min.x < 150.0 || max.y - min.y < 150.0 {
        return Err(CoreError::Invalid("draw a larger callout".into()));
    }
    let base = format!("Callout of {parent_name}");
    let taken = |n: &str| doc.of(Category::View).any(|e| e.data.name() == n);
    let name = if taken(&base) {
        (2..)
            .map(|i| format!("{base} ({i})"))
            .find(|n| !taken(n))
            .unwrap_or(base)
    } else {
        base
    };
    doc.transact("Create callout", |tx| {
        let mut v = ElementData::view(name, kind, CALLOUT_SCALE);
        if let ElementData::View {
            crop, callout_of, ..
        } = &mut v
        {
            *crop = Some(CropBox { min, max });
            *callout_of = Some(parent);
        }
        Ok(tx.insert(v))
    })
}

/// Revit's Column Location Mark: the grids crossing nearest the column, letters first
/// ("B-2"). Empty when no grid intersection lies within 3'.
pub fn column_mark(doc: &Document, at: Pt) -> String {
    let grids: Vec<(String, Pt, Pt)> = doc
        .of(Category::Grid)
        .filter_map(|e| match &e.data {
            ElementData::Grid { name, start, end } => Some((name.clone(), *start, *end)),
            _ => None,
        })
        .collect();
    let mut best: Option<(f64, String)> = None;
    for i in 0..grids.len() {
        for j in (i + 1)..grids.len() {
            let (a, b) = (&grids[i], &grids[j]);
            let Some(x) = line_intersection(a.1, a.2.sub(a.1), b.1, b.2.sub(b.1)) else {
                continue;
            };
            let on = |g: &(String, Pt, Pt)| project_to_segment(x, g.1, g.2).1 < 1.0;
            let d = x.dist(at);
            if !on(a) || !on(b) || d > 914.4 || best.as_ref().is_some_and(|b| b.0 <= d) {
                continue;
            }
            let lettered = |n: &str| n.chars().next().is_some_and(char::is_alphabetic);
            let (p, q) = if lettered(&b.0) && !lettered(&a.0) {
                (&b.0, &a.0)
            } else {
                (&a.0, &b.0)
            };
            best = Some((d, format!("{p}-{q}")));
        }
    }
    best.map(|b| b.1).unwrap_or_default()
}

/// The four directions of an elevation marker, in Revit's order around the body.
pub const DIRECTIONS: [Compass; 4] = [Compass::North, Compass::East, Compass::South, Compass::West];

pub fn compass_name(c: Compass) -> &'static str {
    match c {
        Compass::North => "North",
        Compass::East => "East",
        Compass::South => "South",
        Compass::West => "West",
    }
}

/// The views of an elevation marker, by direction.
pub fn marker_views(doc: &Document, marker: ElementId) -> Vec<(Compass, ElementId)> {
    let mut v: Vec<(Compass, ElementId)> = doc
        .of(Category::View)
        .filter_map(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::MarkerElevation { marker: m, facing },
                ..
            } if *m == marker => Some((*facing, e.id)),
            _ => None,
        })
        .collect();
    v.sort_by_key(|(c, _)| DIRECTIONS.iter().position(|d| d == c));
    v
}

/// Places an elevation marker with views looking the given directions, named as given.
pub fn create_elevation_marker(
    doc: &mut Document,
    level: ElementId,
    at: Pt,
    interior: bool,
    views: &[(Compass, String)],
) -> CoreResult<ElementId> {
    doc.transact("Create elevation", |tx| {
        if !matches!(tx.data(level)?, ElementData::Level { .. }) {
            return Err(CoreError::Invalid(
                "elevations are placed on a level".into(),
            ));
        }
        let marker = tx.insert(ElementData::ElevationMarker {
            level,
            at,
            interior,
        });
        for (facing, name) in views {
            tx.insert(ElementData::view(
                name.clone(),
                ViewKind::MarkerElevation {
                    marker,
                    facing: *facing,
                },
                48,
            ));
        }
        Ok(marker)
    })
}

/// Turns one direction of a marker on (a new view named `name`) or off (deleting it).
pub fn set_marker_view(
    doc: &mut Document,
    marker: ElementId,
    facing: Compass,
    name: Option<String>,
) -> CoreResult<()> {
    if !matches!(doc.data(marker)?, ElementData::ElevationMarker { .. }) {
        return Err(CoreError::Invalid("that isn't an elevation marker".into()));
    }
    let existing = marker_views(doc, marker)
        .into_iter()
        .find(|(c, _)| *c == facing)
        .map(|x| x.1);
    match (name, existing) {
        (Some(n), None) => doc.transact("Add elevation view", |tx| {
            tx.insert(ElementData::view(
                n,
                ViewKind::MarkerElevation { marker, facing },
                48,
            ));
            Ok(())
        }),
        (None, Some(v)) => {
            crate::ops::delete(doc, &[v])?;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The direction to look from `at` toward the nearest wall on `level` (Revit points a new
/// marker at the nearest wall).
pub fn facing_nearest_wall(doc: &Document, level: ElementId, at: Pt) -> Compass {
    let nearest = doc
        .of(Category::Wall)
        .filter_map(|e| match &e.data {
            ElementData::Wall {
                base_level,
                start,
                end,
                ..
            } if *base_level == level => {
                let (t, d) = project_to_segment(at, *start, *end);
                Some((d, start.lerp(*end, t)))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let Some((_, q)) = nearest else {
        return Compass::North;
    };
    let v = q.sub(at);
    if v.x.abs() > v.y.abs() {
        if v.x > 0.0 {
            Compass::East
        } else {
            Compass::West
        }
    } else if v.y >= 0.0 {
        Compass::North
    } else {
        Compass::South
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use crate::units::MM_PER_FT;

    fn ft(x: f64, y: f64) -> Pt {
        Pt::new(x * MM_PER_FT, y * MM_PER_FT)
    }

    #[test]
    fn column_marks_come_from_the_nearest_grid_intersection() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        for x in [0.0, 20.0] {
            ops::create_grid(&mut doc, ft(x, -5.0), ft(x, 30.0)).unwrap();
        }
        let a = ops::create_grid(&mut doc, ft(-5.0, 0.0), ft(25.0, 0.0)).unwrap();
        ops::set_property(&mut doc, a, "name", "A", 0).unwrap();
        let b = ops::create_grid(&mut doc, ft(-5.0, 24.0), ft(25.0, 24.0)).unwrap();
        ops::set_property(&mut doc, b, "name", "B", 0).unwrap();
        assert_eq!(column_mark(&doc, ft(20.0, 24.0)), "B-2");
        assert_eq!(column_mark(&doc, ft(0.5, 0.2)), "A-1", "close enough");
        assert_eq!(column_mark(&doc, ft(10.0, 12.0)), "", "between grids");
    }

    #[test]
    fn callouts_crop_a_copy_of_the_parent_at_detail_scale() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let plan = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::FloorPlan { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        let c = create_callout(&mut doc, plan, ft(5.0, 5.0), ft(0.0, 0.0)).unwrap();
        let ElementData::View {
            name,
            kind,
            scale,
            crop,
            callout_of,
            ..
        } = doc.data(c).unwrap()
        else {
            panic!()
        };
        assert_eq!(name, "Callout of Level 1");
        assert!(matches!(kind, ViewKind::FloorPlan { .. }));
        assert_eq!(*scale, 8);
        assert_eq!(crop.unwrap().min, ft(0.0, 0.0));
        assert_eq!(*callout_of, Some(plan));
        let c2 = create_callout(&mut doc, plan, ft(5.0, 5.0), ft(10.0, 10.0)).unwrap();
        assert_eq!(doc.data(c2).unwrap().name(), "Callout of Level 1 (2)");
        assert!(create_callout(&mut doc, plan, ft(0.0, 0.0), ft(0.1, 5.0)).is_err());
        // Deleting the parent takes its callouts with it.
        let other = create_callout(&mut doc, c, ft(1.0, 1.0), ft(3.0, 3.0)).unwrap();
        ops::delete(&mut doc, &[plan]).unwrap();
        assert!(doc.get(c).is_none() && doc.get(c2).is_none() && doc.get(other).is_none());
    }

    #[test]
    fn separators_need_a_level_and_length() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        assert!(create_room_separator(&mut doc, l1, ft(0.0, 0.0), ft(10.0, 0.0)).is_ok());
        assert!(create_room_separator(&mut doc, l1, ft(0.0, 0.0), ft(0.0, 0.0)).is_err());
    }
}
