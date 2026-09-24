//! Modify operations that touch several elements at once.

use std::collections::{HashMap, HashSet};

use studio_geom::{line_intersection, project_to_segment, tol, Pt};

use crate::document::{CoreResult, Document, Tx};
use crate::element::{ElementData, ElementId};

/// Moves elements by `delta` (plan mm) in one transaction, like Revit's Move:
/// - Walls translate. Walls joined at a moved wall's endpoint stretch to follow it, and walls
///   T-joined into a moved wall keep their end on its location line.
/// - Doors and windows in stretched walls keep their real-world position; doors and
///   windows that are themselves selected slide along their host by the component of
///   `delta` along the wall.
/// - Grids, floors, ceilings and rooms translate. Levels, views and types are ignored.
///
/// Validation (openings must still fit their walls) applies as for any transaction.
pub fn move_elements(doc: &mut Document, ids: &[ElementId], delta: Pt) -> CoreResult<()> {
    if delta.len() < tol::LINEAR {
        return Ok(());
    }
    let selected: HashSet<ElementId> = ids.iter().copied().collect();
    doc.transact("Move", |tx| {
        // Moved walls' old location lines, for stretching joined walls afterwards.
        let mut moved_walls: Vec<(Pt, Pt)> = vec![];
        for id in ids {
            let Some(el) = tx.get(*id) else { continue };
            let mut data = el.data.clone();
            match &mut data {
                ElementData::Wall { start, end, .. } => {
                    moved_walls.push((*start, *end));
                    *start = start.add(delta);
                    *end = end.add(delta);
                }
                ElementData::Grid { start, end, .. } => {
                    *start = start.add(delta);
                    *end = end.add(delta);
                }
                ElementData::Floor { boundary, .. } | ElementData::Ceiling { boundary, .. } => {
                    for p in boundary.iter_mut() {
                        *p = p.add(delta);
                    }
                }
                ElementData::Room { point, .. } => *point = point.add(delta),
                ElementData::TextNote { at, .. } => *at = at.add(delta),
                ElementData::Viewport { center, .. } => *center = center.add(delta),
                ElementData::Dimension { a, b, .. } => {
                    *a = a.add(delta);
                    *b = b.add(delta);
                }
                ElementData::Door { host, offset, .. }
                | ElementData::Window { host, offset, .. } => {
                    if selected.contains(host) {
                        continue; // Moves with its host.
                    }
                    if let Ok(ElementData::Wall { start, end, .. }) = tx.data(*host) {
                        *offset += delta.dot(end.sub(*start).norm());
                    }
                }
                _ => continue,
            }
            tx.set(*id, data)?;
        }
        stretch_joined_walls(tx, &selected, &moved_walls, delta)
    })
}

fn stretch_joined_walls(
    tx: &mut Tx<'_>,
    selected: &HashSet<ElementId>,
    moved: &[(Pt, Pt)],
    delta: Pt,
) -> CoreResult<()> {
    if moved.is_empty() {
        return Ok(());
    }
    // New location lines of the moved walls.
    let moved_new: Vec<(Pt, Pt)> = moved
        .iter()
        .map(|(a, b)| (a.add(delta), b.add(delta)))
        .collect();
    let others: Vec<(ElementId, Pt, Pt)> = tx
        .iter()
        .filter(|e| !selected.contains(&e.id))
        .filter_map(|e| match &e.data {
            ElementData::Wall { start, end, .. } => Some((e.id, *start, *end)),
            _ => None,
        })
        .collect();
    let mut stretched: HashMap<ElementId, (Pt, Pt, Pt, Pt)> = HashMap::new();
    for (id, s0, e0) in others {
        let follow = |p: Pt, other_end: Pt| -> Option<Pt> {
            for (i, (a, b)) in moved.iter().enumerate() {
                // Corner join: shares an endpoint with a moved wall.
                if p.dist(*a) < tol::JOIN || p.dist(*b) < tol::JOIN {
                    return Some(p.add(delta));
                }
                // T join: ends on a moved wall's location line.
                let (t, d) = project_to_segment(p, *a, *b);
                if d < tol::JOIN && t > 0.0 && t < 1.0 {
                    let (na, nb) = moved_new[i];
                    return line_intersection(other_end, p.sub(other_end), na, nb.sub(na));
                }
            }
            None
        };
        let s1 = follow(s0, e0).unwrap_or(s0);
        let e1 = follow(e0, s0).unwrap_or(e0);
        if s1 != s0 || e1 != e0 {
            tx.modify(id, |d| {
                if let ElementData::Wall { start, end, .. } = d {
                    *start = s1;
                    *end = e1;
                }
            })?;
            stretched.insert(id, (s0, e0, s1, e1));
        }
    }
    // Openings in stretched walls keep their real-world position.
    let hosted: Vec<(ElementId, ElementId, f64)> =
        tx.iter()
            .filter(|e| !selected.contains(&e.id))
            .filter_map(|e| match &e.data {
                ElementData::Door { host, offset, .. }
                | ElementData::Window { host, offset, .. } => Some((e.id, *host, *offset)),
                _ => None,
            })
            .collect();
    for (id, host, offset) in hosted {
        let Some((s0, e0, s1, e1)) = stretched.get(&host) else {
            continue;
        };
        let world = s0.add(e0.sub(*s0).norm().scale(offset));
        let new_offset = world.sub(*s1).dot(e1.sub(*s1).norm());
        tx.modify(id, |d| {
            if let ElementData::Door { offset, .. } | ElementData::Window { offset, .. } = d {
                *offset = new_offset;
            }
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Category;
    use crate::ops;
    use crate::units::{MM_PER_FT, MM_PER_IN};

    const EPS: f64 = 1e-6;

    fn wall_ends(doc: &Document, id: ElementId) -> (Pt, Pt) {
        match doc.data(id).unwrap() {
            ElementData::Wall { start, end, .. } => (*start, *end),
            _ => unreachable!(),
        }
    }

    fn offset_of(doc: &Document, id: ElementId) -> f64 {
        match doc.data(id).unwrap() {
            ElementData::Door { offset, .. } | ElementData::Window { offset, .. } => *offset,
            _ => unreachable!(),
        }
    }

    /// 40' × 30' rectangle (walls s, e, n, w) with a partition T-joined into s and n.
    fn building() -> (Document, [ElementId; 5]) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let (w, h) = (40.0 * MM_PER_FT, 30.0 * MM_PER_FT);
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ];
        let mut ids = [ElementId::new(); 5];
        for i in 0..4 {
            ids[i] = ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap();
        }
        ids[4] = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(16.0 * MM_PER_FT, 0.0),
            Pt::new(16.0 * MM_PER_FT, h),
        )
        .unwrap();
        (doc, ids)
    }

    #[test]
    fn moving_a_wall_stretches_corner_joined_walls() {
        let (mut doc, [s, e, n, _, _]) = building();
        let d = Pt::new(5.0 * MM_PER_FT, 0.0);
        move_elements(&mut doc, &[e], d).unwrap();
        assert!((wall_ends(&doc, e).0.x - 45.0 * MM_PER_FT).abs() < EPS);
        assert!(
            (wall_ends(&doc, s).1.x - 45.0 * MM_PER_FT).abs() < EPS,
            "south wall's end follows"
        );
        assert!(
            (wall_ends(&doc, n).0.x - 45.0 * MM_PER_FT).abs() < EPS,
            "north wall's start follows"
        );
        assert!(wall_ends(&doc, s).0.x.abs() < EPS, "far ends stay put");
        doc.undo().unwrap();
        assert!((wall_ends(&doc, s).1.x - 40.0 * MM_PER_FT).abs() < EPS);
    }

    #[test]
    fn t_joined_partition_stays_attached() {
        let (mut doc, [s, _, _, _, p]) = building();
        move_elements(&mut doc, &[s], Pt::new(0.0, -3.0 * MM_PER_FT)).unwrap();
        let (ps, pe) = wall_ends(&doc, p);
        assert!(
            (ps.y + 3.0 * MM_PER_FT).abs() < EPS,
            "partition end follows the moved wall's line"
        );
        assert!((ps.x - 16.0 * MM_PER_FT).abs() < EPS);
        assert!((pe.y - 30.0 * MM_PER_FT).abs() < EPS);
    }

    #[test]
    fn openings_in_stretched_walls_keep_their_world_position() {
        let (mut doc, [s, _, _, w, _]) = building();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let door = ops::create_door(&mut doc, dt, s, 25.0 * MM_PER_FT, false).unwrap();
        // Moving the west wall moves the south wall's start 4' west.
        move_elements(&mut doc, &[w], Pt::new(-4.0 * MM_PER_FT, 0.0)).unwrap();
        assert!((offset_of(&doc, door) - 29.0 * MM_PER_FT).abs() < EPS);
    }

    #[test]
    fn selected_door_slides_along_its_wall() {
        let (mut doc, [s, _, _, _, _]) = building();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let door = ops::create_door(&mut doc, dt, s, 25.0 * MM_PER_FT, false).unwrap();
        move_elements(&mut doc, &[door], Pt::new(24.0 * MM_PER_IN, 500.0)).unwrap();
        assert!(
            (offset_of(&doc, door) - 27.0 * MM_PER_FT).abs() < EPS,
            "only the along-wall part counts"
        );
        // Sliding past the wall end is rejected.
        assert!(move_elements(&mut doc, &[door], Pt::new(20.0 * MM_PER_FT, 0.0)).is_err());
    }

    #[test]
    fn a_door_moves_with_its_selected_host() {
        let (mut doc, [s, _, _, _, _]) = building();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let door = ops::create_door(&mut doc, dt, s, 25.0 * MM_PER_FT, false).unwrap();
        move_elements(&mut doc, &[s, door], Pt::new(0.0, -1000.0)).unwrap();
        assert!((offset_of(&doc, door) - 25.0 * MM_PER_FT).abs() < EPS);
    }
}
