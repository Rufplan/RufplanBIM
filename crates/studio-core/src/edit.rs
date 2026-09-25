//! Revit-style modify tools (ADR-017): Copy, Array, Rotate, Mirror, Trim/Extend, Offset,
//! Split, Flip, dragging wall and grid ends, and editing temporary dimensions. Each tool
//! is one transaction, so one undo.

use std::collections::{HashMap, HashSet};

use studio_geom::{line_intersection, project_to_segment, tol, Pt};

use crate::document::{CoreError, CoreResult, Document, Tx};
use crate::element::{Anchor, Category, CropBox, ElementData, ElementId, ViewKind};
use crate::ops::{ccw, next_grid_name, next_mark, tag_in_plans};

/// A 2D rigid motion or reflection: p' = M·p + t.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xform {
    m: [f64; 4],
    t: Pt,
}

impl Xform {
    pub fn translate(d: Pt) -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 1.0],
            t: d,
        }
    }

    /// Counter-clockwise rotation by `angle` radians about `c`.
    pub fn rotate(c: Pt, angle: f64) -> Self {
        let (s, co) = angle.sin_cos();
        let m = [co, -s, s, co];
        // t = c - M·c
        let mc = Pt::new(m[0] * c.x + m[1] * c.y, m[2] * c.x + m[3] * c.y);
        Self { m, t: c.sub(mc) }
    }

    /// Reflection across the line through `a` and `b`.
    pub fn mirror(a: Pt, b: Pt) -> Self {
        let d = b.sub(a).norm();
        let (xx, xy, yy) = (d.x * d.x, d.x * d.y, d.y * d.y);
        let m = [xx - yy, 2.0 * xy, 2.0 * xy, yy - xx];
        let ma = Pt::new(m[0] * a.x + m[1] * a.y, m[2] * a.x + m[3] * a.y);
        Self { m, t: a.sub(ma) }
    }

    pub fn apply(&self, p: Pt) -> Pt {
        Pt::new(
            self.m[0] * p.x + self.m[1] * p.y + self.t.x,
            self.m[2] * p.x + self.m[3] * p.y + self.t.y,
        )
    }

    pub fn apply_vec(&self, v: Pt) -> Pt {
        Pt::new(
            self.m[0] * v.x + self.m[1] * v.y,
            self.m[2] * v.x + self.m[3] * v.y,
        )
    }

    /// True for mirror images (orientation flips).
    pub fn is_reflection(&self) -> bool {
        self.m[0] * self.m[3] - self.m[1] * self.m[2] < 0.0
    }

    /// The pure translation, if this is one.
    fn translation(&self) -> Option<Pt> {
        (self.m == [1.0, 0.0, 0.0, 1.0]).then_some(self.t)
    }
}

fn wall_line(tx: &Tx<'_>, id: ElementId) -> Option<(Pt, Pt)> {
    match tx.data(id).ok()? {
        ElementData::Wall { start, end, .. } => Some((*start, *end)),
        _ => None,
    }
}

fn is_plan_view(tx: &Tx<'_>, view: ElementId) -> bool {
    matches!(
        tx.data(view),
        Ok(ElementData::View {
            kind: ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. },
            ..
        })
    )
}

/// Selected ids plus the doors and windows hosted by selected walls (they travel with
/// their host, as in Revit).
fn with_hosted(doc: &Document, ids: &[ElementId]) -> Vec<ElementId> {
    let set: HashSet<ElementId> = ids.iter().copied().collect();
    let mut out: Vec<ElementId> = ids.to_vec();
    for e in doc.of(Category::Door).chain(doc.of(Category::Window)) {
        if let Some(h) = e.data.host() {
            if set.contains(&h) && !set.contains(&e.id) {
                out.push(e.id);
            }
        }
    }
    // Hosts before hosted, so copies can find their new host.
    out.sort_by_key(|id| doc.data(*id).map(|d| d.host().is_some()).unwrap_or(false));
    out.dedup();
    out
}

/// A copy (or the moved original) of `data` under `x`. `map` gives the new id of any
/// already-copied element. Returns None for elements the tool doesn't transform.
fn transformed(
    tx: &Tx<'_>,
    data: &ElementData,
    x: &Xform,
    map: &HashMap<ElementId, ElementId>,
    in_place: bool,
) -> Option<ElementData> {
    let mirror = x.is_reflection();
    let mut d = data.clone();
    match &mut d {
        ElementData::Wall { start, end, .. } => {
            let (s, e) = (x.apply(*start), x.apply(*end));
            // A mirrored wall keeps its exterior face outside by running the other way.
            (*start, *end) = if mirror { (e, s) } else { (s, e) };
        }
        ElementData::Grid { start, end, .. } => {
            *start = x.apply(*start);
            *end = x.apply(*end);
        }
        ElementData::Floor {
            boundary, sketch, ..
        }
        | ElementData::Ceiling {
            boundary, sketch, ..
        } => {
            *boundary = ccw(boundary.iter().map(|p| x.apply(*p)).collect());
            // Locked lines of copied walls follow the copies (ADR-021).
            for c in sketch.iter_mut().flatten() {
                *c = c.mapped(&|p| x.apply(p), mirror);
                if let crate::sketch::SketchCurve::Line { wall: Some(r), .. } = c {
                    if let Some(n) = map.get(&r.wall) {
                        r.wall = *n;
                    }
                }
            }
        }
        ElementData::Roof {
            boundary, sloped, ..
        } => {
            let mut pts: Vec<Pt> = boundary.iter().map(|p| x.apply(*p)).collect();
            if mirror {
                // Reversing the ring reverses edge order: edge k becomes edge n-2-k.
                pts.reverse();
                sloped.reverse();
                sloped.rotate_left(1);
            }
            *boundary = pts;
        }
        ElementData::Stair { start, end, .. } => {
            *start = x.apply(*start);
            *end = x.apply(*end);
        }
        ElementData::Room { point, .. } => *point = x.apply(*point),
        ElementData::ElevationMarker { at, .. } => *at = x.apply(*at),
        ElementData::RoomSeparator { start, end, .. } => {
            *start = x.apply(*start);
            *end = x.apply(*end);
        }
        ElementData::TextNote { view, at, .. } => {
            if !is_plan_view(tx, *view) {
                return None;
            }
            *at = x.apply(*at);
        }
        ElementData::Dimension {
            view,
            a,
            b,
            a_ref,
            b_ref,
            ..
        } => {
            if !is_plan_view(tx, *view) {
                return None;
            }
            *a = x.apply(*a);
            *b = x.apply(*b);
            for r in [&mut *a_ref, &mut *b_ref] {
                *r = match r.take() {
                    Some(Anchor::Wall { wall, t, side }) => map.get(&wall).map(|w| Anchor::Wall {
                        wall: *w,
                        t: if mirror { 1.0 - t } else { t },
                        side,
                    }),
                    Some(Anchor::Grid { grid, t }) => {
                        map.get(&grid).map(|g| Anchor::Grid { grid: *g, t })
                    }
                    None => None,
                };
            }
            if mirror {
                // Keep the dimension line on the same side of the measured points.
                std::mem::swap(a, b);
                std::mem::swap(a_ref, b_ref);
            }
        }
        ElementData::Door {
            host,
            offset,
            flip_hand,
            ..
        } => place_opening(tx, host, offset, Some(flip_hand), x, map, in_place)?,
        ElementData::Window { host, offset, .. } => {
            place_opening(tx, host, offset, None, x, map, in_place)?
        }
        _ => return None,
    }
    Some(d)
}

/// Where a transformed door or window goes: into its host's copy (mirrored copies measure
/// from the other end and change hand), or, copied alone, along the same wall.
fn place_opening(
    tx: &Tx<'_>,
    host: &mut ElementId,
    offset: &mut f64,
    flip_hand: Option<&mut bool>,
    x: &Xform,
    map: &HashMap<ElementId, ElementId>,
    in_place: bool,
) -> Option<()> {
    match (map.get(host).copied(), x.translation()) {
        (Some(h), _) => {
            if x.is_reflection() {
                let (s, e) = wall_line(tx, *host)?;
                *offset = s.dist(e) - *offset;
                if let Some(fh) = flip_hand {
                    *fh = !*fh;
                }
            }
            *host = h;
            Some(())
        }
        // A door copied without its wall slides along the same wall.
        (None, Some(delta)) if !in_place => {
            let (s, e) = wall_line(tx, *host)?;
            *offset += delta.dot(e.sub(s).norm());
            Some(())
        }
        _ => None,
    }
}

/// Copies `ids` once per transform (Copy is one transform, Array several). Hosted doors and
/// windows of copied walls are copied with them, and new marks, room numbers and grid
/// names continue the existing sequences. Returns the new elements.
pub fn copy_elements(
    doc: &mut Document,
    ids: &[ElementId],
    xforms: &[Xform],
    name: &str,
) -> CoreResult<Vec<ElementId>> {
    let order = with_hosted(doc, ids);
    let next_num = |cat: Category| next_mark(doc, cat).parse::<u32>().unwrap_or(1);
    let (mut door_no, mut win_no, mut room_no) = (
        next_num(Category::Door),
        next_num(Category::Window),
        next_num(Category::Room),
    );
    let mut grid_name = doc
        .of(Category::Grid)
        .filter_map(|e| match &e.data {
            ElementData::Grid { name, .. } => Some(name.clone()),
            _ => None,
        })
        .max_by(|a, b| crate::ops::natural_cmp(a, b));
    doc.transact(name, |tx| {
        let mut created = vec![];
        for x in xforms {
            let mut map: HashMap<ElementId, ElementId> = HashMap::new();
            for id in &order {
                let Some(el) = tx.get(*id).cloned() else {
                    continue;
                };
                let Some(mut d) = transformed(tx, &el.data, x, &map, false) else {
                    continue;
                };
                match &mut d {
                    ElementData::Door { mark, .. } => {
                        *mark = door_no.to_string();
                        door_no += 1;
                    }
                    ElementData::Window { mark, .. } => {
                        *mark = win_no.to_string();
                        win_no += 1;
                    }
                    ElementData::Room { number, .. } => {
                        *number = room_no.to_string();
                        room_no += 1;
                    }
                    ElementData::Grid { name, .. } => {
                        let n = next_grid_name(grid_name.as_deref());
                        *name = n.clone();
                        grid_name = Some(n);
                    }
                    _ => {}
                }
                let mut copy = el.clone();
                copy.id = ElementId::new();
                copy.rev = 1;
                copy.data = d;
                let new_id = tx.insert_element(copy);
                map.insert(*id, new_id);
                if matches!(
                    el.category(),
                    Category::Door | Category::Window | Category::Room
                ) {
                    tag_in_plans(tx, new_id);
                }
                if ids.contains(id) {
                    created.push(new_id);
                }
            }
        }
        if created.is_empty() {
            return Err(CoreError::Invalid(
                "nothing selected can be copied this way".into(),
            ));
        }
        Ok(created)
    })
}

/// Rotates or mirrors `ids` in place (with their hosted doors and windows).
pub fn transform_elements(
    doc: &mut Document,
    ids: &[ElementId],
    x: Xform,
    name: &str,
) -> CoreResult<()> {
    crate::visibility::ensure_unpinned(doc, ids)?;
    let order = with_hosted(doc, ids);
    doc.transact(name, |tx| {
        // In place, a host keeps its id, so the map is the identity.
        let map: HashMap<ElementId, ElementId> = order.iter().map(|id| (*id, *id)).collect();
        // Transform openings first: mirroring needs their host's length before it changes.
        let (hosted, rest): (Vec<ElementId>, Vec<ElementId>) = order
            .iter()
            .partition(|id| tx.data(**id).map(|d| d.host().is_some()).unwrap_or(false));
        let mut changed = 0;
        for id in hosted.iter().chain(&rest) {
            let Some(data) = tx.data(*id).ok().cloned() else {
                continue;
            };
            if let Some(d) = transformed(tx, &data, &x, &map, true) {
                tx.set(*id, d)?;
                changed += 1;
            }
        }
        if changed == 0 {
            return Err(CoreError::Invalid(
                "nothing selected can be rotated or mirrored".into(),
            ));
        }
        Ok(())
    })
}

fn hosted_in(tx: &Tx<'_>, wall: ElementId) -> Vec<ElementId> {
    tx.of(Category::Door)
        .chain(tx.of(Category::Window))
        .filter(|e| e.data.host() == Some(wall))
        .map(|e| e.id)
        .collect()
}

fn set_offset(tx: &mut Tx<'_>, id: ElementId, f: impl Fn(f64) -> f64) -> CoreResult<()> {
    tx.modify(id, |d| {
        if let ElementData::Door { offset, .. } | ElementData::Window { offset, .. } = d {
            *offset = f(*offset);
        }
    })
}

/// Sets a wall's ends, keeping its openings where they are in the world.
fn set_wall_ends(tx: &mut Tx<'_>, id: ElementId, s1: Pt, e1: Pt) -> CoreResult<()> {
    let (s0, e0) = wall_line(tx, id).ok_or(CoreError::NotFound(id))?;
    if (s0, e0) == (s1, e1) {
        return Ok(());
    }
    tx.modify(id, |d| {
        if let ElementData::Wall { start, end, .. } = d {
            *start = s1;
            *end = e1;
        }
    })?;
    let dir0 = e0.sub(s0).norm();
    let dir1 = e1.sub(s1).norm();
    for o in hosted_in(tx, id) {
        set_offset(tx, o, |off| {
            let world = s0.add(dir0.scale(off));
            world.sub(s1).dot(dir1)
        })?;
    }
    Ok(())
}

/// Revit's Trim/Extend to Corner: both walls are cut or extended to meet where their
/// location lines cross, keeping the part of each wall that was clicked.
pub fn trim_extend(
    doc: &mut Document,
    a: ElementId,
    a_pick: Pt,
    b: ElementId,
    b_pick: Pt,
) -> CoreResult<()> {
    crate::visibility::ensure_unpinned(doc, &[a, b])?;
    if a == b {
        return Err(CoreError::Invalid("pick two different walls".into()));
    }
    doc.transact("Trim/Extend", |tx| {
        let (sa, ea) = wall_line(tx, a).ok_or_else(|| CoreError::Invalid("pick walls".into()))?;
        let (sb, eb) = wall_line(tx, b).ok_or_else(|| CoreError::Invalid("pick walls".into()))?;
        let x = line_intersection(sa, ea.sub(sa), sb, eb.sub(sb))
            .ok_or_else(|| CoreError::Invalid("those walls are parallel".into()))?;
        for (id, s, e, pick) in [(a, sa, ea, a_pick), (b, sb, eb, b_pick)] {
            let dir = e.sub(s).norm();
            let tx_ = x.sub(s).dot(dir);
            let tp = pick.sub(s).dot(dir);
            let (s1, e1) = if tp < tx_ { (s, x) } else { (x, e) };
            set_wall_ends(tx, id, s1, e1)?;
        }
        Ok(())
    })
}

/// Revit's Offset: a copy of a wall (or grid) `distance` mm away, on the side of `side`.
pub fn offset_element(
    doc: &mut Document,
    id: ElementId,
    distance: f64,
    side: Pt,
) -> CoreResult<ElementId> {
    let (s, e) = match doc.data(id)? {
        ElementData::Wall { start, end, .. } | ElementData::Grid { start, end, .. } => {
            (*start, *end)
        }
        _ => return Err(CoreError::Invalid("offset works on walls and grids".into())),
    };
    let n = e.sub(s).norm().perp();
    let sign = if side.sub(s).dot(n) >= 0.0 { 1.0 } else { -1.0 };
    let delta = n.scale(sign * distance.abs());
    // Only the element itself: an offset wall doesn't bring its doors along.
    let mut out = copy_only(doc, id, delta)?;
    out.pop().ok_or(CoreError::NotFound(id))
}

fn copy_only(doc: &mut Document, id: ElementId, delta: Pt) -> CoreResult<Vec<ElementId>> {
    let mut name = None;
    if let ElementData::Grid { .. } = doc.data(id)? {
        name = doc
            .of(Category::Grid)
            .filter_map(|e| match &e.data {
                ElementData::Grid { name, .. } => Some(name.clone()),
                _ => None,
            })
            .max_by(|a, b| crate::ops::natural_cmp(a, b));
    }
    doc.transact("Offset", |tx| {
        let mut el = tx.get(id).cloned().ok_or(CoreError::NotFound(id))?;
        match &mut el.data {
            ElementData::Wall { start, end, .. } => {
                *start = start.add(delta);
                *end = end.add(delta);
            }
            ElementData::Grid {
                start,
                end,
                name: n,
            } => {
                *start = start.add(delta);
                *end = end.add(delta);
                *n = next_grid_name(name.as_deref());
            }
            _ => {}
        }
        el.id = ElementId::new();
        el.rev = 1;
        Ok(vec![tx.insert_element(el)])
    })
}

/// Revit's Split: cuts a wall in two at the point nearest `at`. Doors and windows stay
/// with the part they're in; dimensions attached to the wall follow their spot.
pub fn split_wall(doc: &mut Document, wall: ElementId, at: Pt) -> CoreResult<ElementId> {
    let el = doc.get(wall).cloned().ok_or(CoreError::NotFound(wall))?;
    let ElementData::Wall { start, end, .. } = el.data else {
        return Err(CoreError::Invalid("split works on walls".into()));
    };
    let len = start.dist(end);
    let dir = end.sub(start).norm();
    let t = at.sub(start).dot(dir);
    if t < 10.0 || t > len - 10.0 {
        return Err(CoreError::Invalid(
            "split closer to the middle of the wall".into(),
        ));
    }
    let p = start.add(dir.scale(t));
    let get = |id: ElementId| doc.get(id).map(|e| &e.data);
    let mut to_second = vec![];
    for e in doc.of(Category::Door).chain(doc.of(Category::Window)) {
        if e.data.host() != Some(wall) {
            continue;
        }
        let Some(fit) = crate::hosting::opening_fit(&get, &e.data) else {
            continue;
        };
        if fit.t0 < t && fit.t1 > t {
            return Err(CoreError::Invalid(
                "that point is inside a door or window; split beside it".into(),
            ));
        }
        if fit.t0 >= t {
            to_second.push(e.id);
        }
    }
    let dims: Vec<ElementId> = doc
        .of(Category::Dimension)
        .filter(|e| matches!(&e.data, ElementData::Dimension { a_ref, b_ref, .. }
            if [a_ref, b_ref].iter().any(|r| matches!(r, Some(Anchor::Wall { wall: w, .. }) if *w == wall))))
        .map(|e| e.id)
        .collect();
    doc.transact("Split wall", |tx| {
        let mut second = el.clone();
        second.id = ElementId::new();
        second.rev = 1;
        if let ElementData::Wall { start: s, .. } = &mut second.data {
            *s = p;
        }
        let b = tx.insert_element(second);
        tx.modify(wall, |d| {
            if let ElementData::Wall { end: e, .. } = d {
                *e = p;
            }
        })?;
        for o in &to_second {
            tx.modify(*o, |d| {
                if let ElementData::Door { host, offset, .. }
                | ElementData::Window { host, offset, .. } = d
                {
                    *host = b;
                    *offset -= t;
                }
            })?;
        }
        for id in &dims {
            tx.modify(*id, |d| {
                if let ElementData::Dimension { a_ref, b_ref, .. } = d {
                    for r in [a_ref, b_ref] {
                        if let Some(Anchor::Wall {
                            wall: w,
                            t: f,
                            side,
                        }) = r
                        {
                            if *w == wall {
                                let along = *f * len;
                                *r = Some(if along > t {
                                    Anchor::Wall {
                                        wall: b,
                                        t: (along - t) / (len - t),
                                        side: *side,
                                    }
                                } else {
                                    Anchor::Wall {
                                        wall,
                                        t: along / t,
                                        side: *side,
                                    }
                                });
                            }
                        }
                    }
                }
            })?;
        }
        Ok(b)
    })
}

/// Flips walls end for end (their exterior face swaps sides) — the spacebar in Revit.
/// Doors and windows stay exactly where they are, as do attached dimensions.
pub fn flip_walls(doc: &mut Document, walls: &[ElementId]) -> CoreResult<()> {
    doc.transact("Flip wall", |tx| {
        let mut n = 0;
        for w in walls {
            let Some((s, e)) = wall_line(tx, *w) else {
                continue;
            };
            let len = s.dist(e);
            // Flip about the location line: a wall drawn by its exterior face keeps that
            // face where it was drawn.
            let shift = match tx.data(*w)? {
                ElementData::Wall {
                    type_id, location, ..
                } => match tx.data(*type_id)? {
                    ElementData::WallType {
                        thickness, layers, ..
                    } => {
                        let off = crate::compound::location_offset(layers, *thickness, *location);
                        e.sub(s).norm().perp().scale(2.0 * off)
                    }
                    _ => Pt::default(),
                },
                _ => Pt::default(),
            };
            tx.modify(*w, |d| {
                if let ElementData::Wall { start, end, .. } = d {
                    std::mem::swap(start, end);
                    *start = start.add(shift);
                    *end = end.add(shift);
                }
            })?;
            for o in hosted_in(tx, *w) {
                tx.modify(o, |d| match d {
                    ElementData::Door {
                        offset,
                        flip_hand,
                        flip_facing,
                        ..
                    } => {
                        *offset = len - *offset;
                        *flip_hand = !*flip_hand;
                        *flip_facing = !*flip_facing;
                    }
                    ElementData::Window {
                        offset,
                        flip_facing,
                        ..
                    } => {
                        *offset = len - *offset;
                        *flip_facing = !*flip_facing;
                    }
                    _ => {}
                })?;
            }
            let dims: Vec<ElementId> = tx.of(Category::Dimension).map(|e| e.id).collect();
            for id in dims {
                tx.modify(id, |d| {
                    if let ElementData::Dimension { a_ref, b_ref, .. } = d {
                        for r in [a_ref, b_ref] {
                            if let Some(Anchor::Wall { wall, t, side }) = r {
                                if wall == w {
                                    *t = 1.0 - *t;
                                    *side = -*side;
                                }
                            }
                        }
                    }
                })?;
            }
            n += 1;
        }
        if n == 0 {
            return Err(CoreError::Invalid("select walls to flip".into()));
        }
        Ok(())
    })
}

/// Drags one end of a wall to `to`. Walls corner-joined at that end follow it, walls
/// T-joined into the wall stay on its new line, and openings keep their world position.
pub fn move_wall_end(
    doc: &mut Document,
    wall: ElementId,
    at_start: bool,
    to: Pt,
) -> CoreResult<()> {
    doc.transact("Drag wall end", |tx| {
        move_wall_end_tx(tx, wall, at_start, to)
    })
}

fn move_wall_end_tx(tx: &mut Tx<'_>, wall: ElementId, at_start: bool, to: Pt) -> CoreResult<()> {
    let (s0, e0) = wall_line(tx, wall).ok_or_else(|| CoreError::Invalid("not a wall".into()))?;
    let old = if at_start { s0 } else { e0 };
    let (s1, e1) = if at_start { (to, e0) } else { (s0, to) };
    set_wall_ends(tx, wall, s1, e1)?;
    let others: Vec<(ElementId, Pt, Pt)> = tx
        .of(Category::Wall)
        .filter(|e| e.id != wall)
        .filter_map(|e| match &e.data {
            ElementData::Wall { start, end, .. } => Some((e.id, *start, *end)),
            _ => None,
        })
        .collect();
    for (id, s, e) in others {
        let follow = |p: Pt, other: Pt| -> Option<Pt> {
            if p.dist(old) < tol::JOIN {
                return Some(to);
            }
            let (t, d) = project_to_segment(p, s0, e0);
            if d < tol::JOIN && t > 0.0 && t < 1.0 {
                return line_intersection(other, p.sub(other), s1, e1.sub(s1));
            }
            None
        };
        let ns = follow(s, e).unwrap_or(s);
        let ne = follow(e, s).unwrap_or(e);
        if (ns, ne) != (s, e) {
            set_wall_ends(tx, id, ns, ne)?;
        }
    }
    Ok(())
}

/// Sets the clear distance from a door or window's edge to its host wall's start
/// (`from_start`) or end — what its temporary dimensions show.
pub fn set_opening_gap(
    doc: &mut Document,
    id: ElementId,
    from_start: bool,
    gap: f64,
) -> CoreResult<()> {
    let get = |id: ElementId| doc.get(id).map(|e| &e.data);
    let data = doc.data(id)?;
    let fit = crate::hosting::opening_fit(&get, data)
        .ok_or_else(|| CoreError::Invalid("select a door or window".into()))?;
    let (s, e) = match doc.data(fit.host)? {
        ElementData::Wall { start, end, .. } => (*start, *end),
        _ => return Err(CoreError::NotFound(fit.host)),
    };
    let width = fit.t1 - fit.t0;
    let offset = if from_start {
        gap + width / 2.0
    } else {
        s.dist(e) - gap - width / 2.0
    };
    doc.transact("Move opening", |tx| set_offset(tx, id, |_| offset))
}

/// Sets a wall's length by moving its end (joined walls follow), from its temporary
/// dimension.
pub fn set_wall_length(doc: &mut Document, wall: ElementId, length: f64) -> CoreResult<()> {
    let (s, e) = match doc.data(wall)? {
        ElementData::Wall { start, end, .. } => (*start, *end),
        _ => return Err(CoreError::Invalid("not a wall".into())),
    };
    if length < 1.0 {
        return Err(CoreError::Invalid("a wall needs some length".into()));
    }
    move_wall_end(doc, wall, false, s.add(e.sub(s).norm().scale(length)))
}

/// Drags a handle (see `studio_views::handles`): wall and grid ends, a dimension's line,
/// and the edges of a view's crop region.
pub fn drag_handle(doc: &mut Document, id: ElementId, key: &str, to: Pt) -> CoreResult<()> {
    crate::visibility::ensure_unpinned(doc, &[id])?;
    let bad = || CoreError::Invalid(format!("unknown handle {key}"));
    match (doc.data(id)?.clone(), key) {
        (ElementData::Wall { .. }, "start" | "end") => move_wall_end(doc, id, key == "start", to),
        (ElementData::Grid { .. }, "start" | "end") => doc.transact("Drag grid end", |tx| {
            tx.modify(id, |d| {
                if let ElementData::Grid { start, end, .. } = d {
                    if key == "start" {
                        *start = to;
                    } else {
                        *end = to;
                    }
                }
            })
        }),
        (d @ ElementData::Dimension { .. }, "line") => {
            let (a, b) = crate::ops::dimension_ends(doc, &d).ok_or_else(bad)?;
            let n = b.sub(a).norm().perp();
            let off = to.sub(a).dot(n);
            doc.transact("Move dimension line", |tx| {
                tx.modify(id, |d| {
                    if let ElementData::Dimension { offset, .. } = d {
                        *offset = off;
                    }
                })
            })
        }
        (ElementData::View { crop: Some(c), .. }, k) if k.starts_with("crop:") => {
            let mut c: CropBox = c;
            match k {
                "crop:left" => c.min.x = to.x.min(c.max.x - 10.0),
                "crop:right" => c.max.x = to.x.max(c.min.x + 10.0),
                "crop:bottom" => c.min.y = to.y.min(c.max.y - 10.0),
                "crop:top" => c.max.y = to.y.max(c.min.y + 10.0),
                _ => return Err(bad()),
            }
            set_crop(doc, id, Some(c))
        }
        _ => Err(bad()),
    }
}

/// Sets (or clears) a view's crop region.
pub fn set_crop(doc: &mut Document, view: ElementId, crop: Option<CropBox>) -> CoreResult<()> {
    doc.transact("Crop view", |tx| {
        tx.modify(view, |d| {
            if let ElementData::View { crop: c, .. } = d {
                *c = crop;
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use crate::units::MM_PER_FT;

    const EPS: f64 = 1e-6;

    fn ends(doc: &Document, id: ElementId) -> (Pt, Pt) {
        match doc.data(id).unwrap() {
            ElementData::Wall { start, end, .. } | ElementData::Grid { start, end, .. } => {
                (*start, *end)
            }
            _ => unreachable!(),
        }
    }

    fn close(a: Pt, b: Pt) -> bool {
        a.dist(b) < 1e-6
    }

    /// Clockwise 40' × 30' rectangle (exterior outside), with a door in the south wall.
    fn building() -> (Document, Vec<ElementId>, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let (w, h) = (40.0 * MM_PER_FT, 30.0 * MM_PER_FT);
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(0.0, h),
            Pt::new(w, h),
            Pt::new(w, 0.0),
        ];
        let walls: Vec<ElementId> = (0..4)
            .map(|i| ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
            .collect();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        // South wall runs east → west (from (40,0) to (0,0)).
        let door = ops::create_door(&mut doc, dt, walls[3], 10.0 * MM_PER_FT, false).unwrap();
        (doc, walls, door)
    }

    #[test]
    fn transforms() {
        let r = Xform::rotate(Pt::new(1.0, 1.0), std::f64::consts::FRAC_PI_2);
        assert!(close(r.apply(Pt::new(2.0, 1.0)), Pt::new(1.0, 2.0)));
        assert!(!r.is_reflection());
        let m = Xform::mirror(Pt::new(0.0, 0.0), Pt::new(0.0, 5.0));
        assert!(close(m.apply(Pt::new(3.0, 4.0)), Pt::new(-3.0, 4.0)));
        assert!(m.is_reflection());
        let m45 = Xform::mirror(Pt::new(0.0, 0.0), Pt::new(1.0, 1.0));
        assert!(close(m45.apply(Pt::new(2.0, 0.0)), Pt::new(0.0, 2.0)));
    }

    #[test]
    fn copy_brings_hosted_doors_with_new_marks_and_tags() {
        let (mut doc, walls, door) = building();
        let before_tags = doc.count(Category::Tag);
        let d = Pt::new(0.0, -20.0 * MM_PER_FT);
        let made = copy_elements(&mut doc, &[walls[3]], &[Xform::translate(d)], "Copy").unwrap();
        assert_eq!(made.len(), 1);
        assert_eq!(doc.count(Category::Door), 2);
        let copy = doc
            .of(Category::Door)
            .find(|e| e.id != door)
            .unwrap()
            .data
            .clone();
        match copy {
            ElementData::Door {
                host, mark, offset, ..
            } => {
                assert_eq!(host, made[0]);
                assert_eq!(mark, "2");
                assert!((offset - 10.0 * MM_PER_FT).abs() < EPS);
            }
            _ => unreachable!(),
        }
        assert_eq!(doc.count(Category::Tag), before_tags + 1);
        doc.undo().unwrap();
        assert_eq!(doc.count(Category::Door), 1);
    }

    #[test]
    fn array_makes_evenly_spaced_copies() {
        let (mut doc, walls, _) = building();
        let xs: Vec<Xform> = (1..4)
            .map(|k| Xform::translate(Pt::new(0.0, -(k as f64) * 1000.0)))
            .collect();
        let made = copy_elements(&mut doc, &[walls[3]], &xs, "Array").unwrap();
        assert_eq!(made.len(), 3);
        let (s, _) = ends(&doc, made[2]);
        assert!((s.y + 3000.0).abs() < EPS);
    }

    #[test]
    fn copying_a_door_alone_slides_it_along_its_wall() {
        let (mut doc, _, door) = building();
        let made = copy_elements(
            &mut doc,
            &[door],
            &[Xform::translate(Pt::new(-5.0 * MM_PER_FT, 300.0))],
            "Copy",
        )
        .unwrap();
        match doc.data(made[0]).unwrap() {
            // The wall runs west, so moving 5' west adds 5' of offset.
            ElementData::Door { offset, .. } => assert!((offset - 15.0 * MM_PER_FT).abs() < EPS),
            _ => unreachable!(),
        }
    }

    #[test]
    fn mirror_keeps_exterior_out_and_mirrors_door_hand() {
        let (mut doc, walls, door) = building();
        let axis = Xform::mirror(
            Pt::new(50.0 * MM_PER_FT, 0.0),
            Pt::new(50.0 * MM_PER_FT, 1.0),
        );
        let made = copy_elements(&mut doc, &walls, &[axis], "Mirror").unwrap();
        assert_eq!(made.len(), 4);
        // Every mirrored wall still has its exterior (left) face outside.
        let pts: Vec<Pt> = made.iter().map(|w| ends(&doc, *w).0).collect();
        let c = pts.iter().fold(Pt::default(), |a, p| a.add(p.scale(0.25)));
        for w in &made {
            let (s, e) = ends(&doc, *w);
            let left = e.sub(s).norm().perp();
            assert!(left.dot(s.lerp(e, 0.5).sub(c)) > 0.0, "exterior faces out");
        }
        let copy = doc.of(Category::Door).find(|e| e.id != door).unwrap();
        match &copy.data {
            ElementData::Door {
                flip_hand,
                host,
                offset,
                ..
            } => {
                assert!(*flip_hand, "a mirrored door has the other hand");
                let (s, e) = ends(&doc, *host);
                // Same distance from the mirrored wall's far end.
                assert!((s.dist(e) - offset - 10.0 * MM_PER_FT).abs() < EPS);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn rotate_in_place() {
        let (mut doc, walls, _) = building();
        transform_elements(
            &mut doc,
            &[walls[0]],
            Xform::rotate(Pt::new(0.0, 0.0), -std::f64::consts::FRAC_PI_2),
            "Rotate",
        )
        .unwrap();
        let (s, e) = ends(&doc, walls[0]);
        assert!(close(s, Pt::new(0.0, 0.0)));
        assert!(close(e, Pt::new(30.0 * MM_PER_FT, 0.0)));
    }

    #[test]
    fn trim_extend_to_corner() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        // An overshooting wall and a short one that doesn't reach.
        let a =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(6000.0, 0.0)).unwrap();
        let b = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(4000.0, 1000.0),
            Pt::new(4000.0, 3000.0),
        )
        .unwrap();
        trim_extend(
            &mut doc,
            a,
            Pt::new(1000.0, 0.0),
            b,
            Pt::new(4000.0, 2500.0),
        )
        .unwrap();
        assert!(close(ends(&doc, a).1, Pt::new(4000.0, 0.0)), "a trimmed");
        assert!(close(ends(&doc, b).0, Pt::new(4000.0, 0.0)), "b extended");
        let c = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 5000.0),
            Pt::new(6000.0, 5000.0),
        )
        .unwrap();
        assert!(trim_extend(&mut doc, a, Pt::new(1.0, 0.0), c, Pt::new(1.0, 5000.0)).is_err());
    }

    #[test]
    fn offset_split_and_flip() {
        let (mut doc, walls, door) = building();
        let n =
            offset_element(&mut doc, walls[0], 2.0 * MM_PER_FT, Pt::new(5000.0, 5000.0)).unwrap();
        let (s, _) = ends(&doc, n);
        assert!(
            (s.x - 2.0 * MM_PER_FT).abs() < EPS,
            "offset to the clicked side"
        );
        // Split the south wall (east → west) 30' from its start: the door at 10' stays.
        let b = split_wall(&mut doc, walls[3], Pt::new(10.0 * MM_PER_FT, 0.0)).unwrap();
        assert!(close(
            ends(&doc, walls[3]).1,
            Pt::new(10.0 * MM_PER_FT, 0.0)
        ));
        assert!(close(ends(&doc, b).0, Pt::new(10.0 * MM_PER_FT, 0.0)));
        assert_eq!(doc.data(door).unwrap().host(), Some(walls[3]));
        assert!(split_wall(&mut doc, walls[3], Pt::new(30.0 * MM_PER_FT, 0.0)).is_err());
        // Flip keeps the door in place and mirrors its flags.
        flip_walls(&mut doc, &[walls[3]]).unwrap();
        match doc.data(door).unwrap() {
            ElementData::Door {
                offset,
                flip_hand,
                flip_facing,
                ..
            } => {
                assert!((offset - 20.0 * MM_PER_FT).abs() < EPS);
                assert!(*flip_hand && *flip_facing);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn dragging_a_joined_wall_end_moves_the_corner() {
        let (mut doc, walls, door) = building();
        // Drag the NE corner (end of the north wall) out by 5'.
        let (_, ne) = ends(&doc, walls[1]);
        let to = ne.add(Pt::new(5.0 * MM_PER_FT, 0.0));
        drag_handle(&mut doc, walls[1], "end", to).unwrap();
        assert!(
            close(ends(&doc, walls[2]).0, to),
            "the east wall's start follows"
        );
        // The south wall's start is not at that corner and doesn't move.
        assert!(close(
            ends(&doc, walls[3]).0,
            Pt::new(40.0 * MM_PER_FT, 0.0)
        ));
        let _ = door;
        set_wall_length(&mut doc, walls[0], 20.0 * MM_PER_FT).unwrap();
        assert!(close(
            ends(&doc, walls[0]).1,
            Pt::new(0.0, 20.0 * MM_PER_FT)
        ));
        assert!(close(
            ends(&doc, walls[1]).0,
            Pt::new(0.0, 20.0 * MM_PER_FT)
        ));
    }

    #[test]
    fn opening_gaps_and_crop_handles() {
        let (mut doc, _, door) = building();
        set_opening_gap(&mut doc, door, true, 2.0 * MM_PER_FT).unwrap();
        match doc.data(door).unwrap() {
            // 2' clear to the edge of a 36" door: center at 3'-6".
            ElementData::Door { offset, .. } => assert!((offset - 3.5 * MM_PER_FT).abs() < EPS),
            _ => unreachable!(),
        }
        let plan = doc
            .of(Category::View)
            .find(|e| {
                matches!(
                    e.data,
                    ElementData::View {
                        kind: ViewKind::FloorPlan { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        set_crop(
            &mut doc,
            plan,
            Some(CropBox {
                min: Pt::new(0.0, 0.0),
                max: Pt::new(1000.0, 1000.0),
            }),
        )
        .unwrap();
        drag_handle(&mut doc, plan, "crop:right", Pt::new(5000.0, 0.0)).unwrap();
        match doc.data(plan).unwrap() {
            ElementData::View { crop: Some(c), .. } => assert!((c.max.x - 5000.0).abs() < EPS),
            _ => unreachable!(),
        }
        assert!(drag_handle(&mut doc, plan, "crop:nope", Pt::new(0.0, 0.0)).is_err());
    }

    #[test]
    fn flip_keeps_the_location_line() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        // Drawn by its exterior face along y = 0, exterior to the left (north).
        let w = ops::create_wall_located(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 0.0),
            Pt::new(5000.0, 0.0),
            crate::element::LocationLine::FinishExterior,
        )
        .unwrap();
        let h = 4.0 * crate::units::MM_PER_IN;
        assert!(
            (ends(&doc, w).0.y + h).abs() < 1e-9,
            "centerline 4\" to the south"
        );
        flip_walls(&mut doc, &[w]).unwrap();
        // Now the exterior is south: the face stays on y = 0 and the centerline moves north.
        let (s, e) = ends(&doc, w);
        assert!((s.y - h).abs() < 1e-9 && (e.y - h).abs() < 1e-9);
        assert!(s.x > e.x, "runs the other way");
    }
}
