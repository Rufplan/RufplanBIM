//! Groups (ADR-087), as Revit has them: a named group type and its instances, each instance
//! owning real member elements (so views, schedules, regeneration and IFC see them as they
//! are). A model group holds model elements (walls, doors, floors, furniture…); a detail
//! group holds a view's annotation (detail lines, text, regions, components, dimensions).
//! Selecting a member selects its group; Move, Copy, Rotate, Mirror, Array and Delete act
//! on the whole group. Edit Group changes one instance, and Finish repeats the change in
//! every other instance of the type.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::edit::Xform;
use crate::element::{Category, ElementData, ElementId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum GroupKind {
    Model,
    Detail,
}

impl GroupKind {
    pub fn label(self) -> &'static str {
        match self {
            GroupKind::Model => "Model Group",
            GroupKind::Detail => "Detail Group",
        }
    }
}

/// The group an element can join: model elements a model group, a view's annotation a
/// detail group; None for what can't be grouped (views, levels, types, sheets, tags…).
pub fn kind_of(data: &ElementData) -> Option<GroupKind> {
    match data {
        ElementData::Wall { .. }
        | ElementData::Floor { .. }
        | ElementData::Ceiling { .. }
        | ElementData::Door { .. }
        | ElementData::Window { .. }
        | ElementData::Room { .. }
        | ElementData::Roof { .. }
        | ElementData::Stair { .. }
        | ElementData::Column { .. }
        | ElementData::Beam { .. }
        | ElementData::Railing { .. }
        | ElementData::RoomSeparator { .. }
        | ElementData::ModelLine { .. }
        | ElementData::LightingFixture { .. }
        | ElementData::Planting { .. }
        | ElementData::InPlace { .. } => Some(GroupKind::Model),
        ElementData::DetailLine { .. }
        | ElementData::TextNote { .. }
        | ElementData::FilledRegion { .. }
        | ElementData::DetailComponent { .. }
        | ElementData::Dimension { .. }
        | ElementData::AngularDimension { .. }
        | ElementData::SpotElevation { .. }
        | ElementData::SpotSlope { .. }
        | ElementData::KeynoteTag { .. } => Some(GroupKind::Detail),
        _ => None,
    }
}

/// The view a detail element is drawn in.
pub fn view_of(data: &ElementData) -> Option<ElementId> {
    match data {
        ElementData::DetailLine { view, .. }
        | ElementData::TextNote { view, .. }
        | ElementData::FilledRegion { view, .. }
        | ElementData::DetailComponent { view, .. }
        | ElementData::Dimension { view, .. }
        | ElementData::AngularDimension { view, .. }
        | ElementData::SpotElevation { view, .. }
        | ElementData::SpotSlope { view, .. }
        | ElementData::KeynoteTag { view, .. } => Some(*view),
        _ => None,
    }
}

/// Every plan point in an element's data (its serialized `{x, y}` pairs): for the group's
/// origin, the center of its members.
fn points(data: &ElementData) -> Vec<Pt> {
    fn walk(v: &serde_json::Value, out: &mut Vec<Pt>) {
        match v {
            serde_json::Value::Object(m) => {
                if let (Some(x), Some(y), 2) = (
                    m.get("x").and_then(|x| x.as_f64()),
                    m.get("y").and_then(|y| y.as_f64()),
                    m.len(),
                ) {
                    out.push(Pt::new(x, y));
                } else {
                    for v in m.values() {
                        walk(v, out);
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = vec![];
    if let Ok(v) = serde_json::to_value(data) {
        walk(&v, &mut out);
    }
    out
}

/// A copy of `data` with every reference to an element in `map` replaced (a member moved
/// to another level or view keeps its type, host and everything else).
fn remapped(data: &ElementData, map: &HashMap<String, String>) -> ElementData {
    fn walk(v: &mut serde_json::Value, map: &HashMap<String, String>) {
        match v {
            serde_json::Value::String(s) => {
                if let Some(n) = map.get(s.as_str()) {
                    *s = n.clone();
                }
            }
            serde_json::Value::Object(m) => m.values_mut().for_each(|v| walk(v, map)),
            serde_json::Value::Array(a) => a.iter_mut().for_each(|v| walk(v, map)),
            _ => {}
        }
    }
    let Ok(mut v) = serde_json::to_value(data) else {
        return data.clone();
    };
    walk(&mut v, map);
    serde_json::from_value(v).unwrap_or_else(|_| data.clone())
}

fn id_str(id: ElementId) -> String {
    id.0.to_string()
}

/// One group instance, as the app shows it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GroupInfo {
    pub id: ElementId,
    pub type_id: ElementId,
    pub name: String,
    pub kind: GroupKind,
    pub members: Vec<ElementId>,
    pub origin: Pt,
    pub level: Option<ElementId>,
    pub view: Option<ElementId>,
}

/// A group type and how many instances it has.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GroupTypeInfo {
    pub id: ElementId,
    pub name: String,
    pub kind: GroupKind,
    pub instances: usize,
}

pub fn groups(doc: &Document) -> Vec<GroupInfo> {
    let mut out: Vec<GroupInfo> = doc
        .of(Category::Group)
        .filter_map(|e| match &e.data {
            ElementData::Group {
                type_id,
                origin,
                level,
                view,
                members,
                ..
            } => {
                let (name, kind) = match doc.data(*type_id) {
                    Ok(ElementData::GroupType { name, kind }) => (name.clone(), *kind),
                    _ => return None,
                };
                Some(GroupInfo {
                    id: e.id,
                    type_id: *type_id,
                    name,
                    kind,
                    members: members
                        .iter()
                        .copied()
                        .filter(|m| doc.get(*m).is_some())
                        .collect(),
                    origin: *origin,
                    level: *level,
                    view: *view,
                })
            }
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn types(doc: &Document) -> Vec<GroupTypeInfo> {
    let mut out: Vec<GroupTypeInfo> = doc
        .of(Category::GroupType)
        .filter_map(|e| match &e.data {
            ElementData::GroupType { name, kind } => Some(GroupTypeInfo {
                id: e.id,
                name: name.clone(),
                kind: *kind,
                instances: doc
                    .of(Category::Group)
                    .filter(|g| matches!(&g.data, ElementData::Group { type_id, .. } if *type_id == e.id))
                    .count(),
            }),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| (a.kind as u8, &a.name).cmp(&(b.kind as u8, &b.name)));
    out
}

/// The group instance an element belongs to.
pub fn group_of(doc: &Document, id: ElementId) -> Option<ElementId> {
    doc.of(Category::Group).find_map(|e| match &e.data {
        ElementData::Group { members, .. } if members.contains(&id) => Some(e.id),
        _ => None,
    })
}

/// A group instance: type, origin, angle, mirrored, level, view and its members.
type Instance = (
    ElementId,
    Pt,
    f64,
    bool,
    Option<ElementId>,
    Option<ElementId>,
    Vec<ElementId>,
);

fn instance(doc: &Document, id: ElementId) -> CoreResult<Instance> {
    match doc.data(id)? {
        ElementData::Group {
            type_id,
            origin,
            angle,
            mirrored,
            level,
            view,
            members,
        } => Ok((
            *type_id,
            *origin,
            *angle,
            *mirrored,
            *level,
            *view,
            members
                .iter()
                .copied()
                .filter(|m| doc.get(*m).is_some())
                .collect(),
        )),
        _ => Err(CoreError::Invalid("that isn't a group".into())),
    }
}

/// `ids` with each group replaced by its members followed by the group itself (so tools
/// move, copy and delete a group whole). Order is kept; nothing repeats.
pub fn expand(doc: &Document, ids: &[ElementId]) -> Vec<ElementId> {
    let mut out: Vec<ElementId> = vec![];
    let mut seen = HashSet::new();
    for id in ids {
        if let Ok(ElementData::Group { members, .. }) = doc.data(*id) {
            for m in members {
                if doc.get(*m).is_some() && seen.insert(*m) {
                    out.push(*m);
                }
            }
        }
        if seen.insert(*id) {
            out.push(*id);
        }
    }
    // Members before their groups, so a copied group finds its copied members.
    out.sort_by_key(|id| matches!(doc.data(*id), Ok(ElementData::Group { .. })));
    out
}

fn next_name(doc: &Document, kind: GroupKind) -> String {
    let taken: HashSet<String> = types(doc).into_iter().map(|t| t.name).collect();
    let base = match kind {
        GroupKind::Model => "Group",
        GroupKind::Detail => "Detail Group",
    };
    (1..)
        .map(|n| format!("{base} {n}"))
        .find(|n| !taken.contains(n))
        .unwrap_or_else(|| base.into())
}

fn center(doc: &Document, members: &[ElementId]) -> Pt {
    let pts: Vec<Pt> = members
        .iter()
        .filter_map(|m| doc.data(*m).ok())
        .flat_map(points)
        .collect();
    match studio_geom::bounds_of(&pts) {
        Some((lo, hi)) => lo.add(hi).scale(0.5),
        None => Pt::new(0.0, 0.0),
    }
}

/// Revit's Create Group: `ids` (with the doors and windows of any walls) become a new group
/// type and its first instance. Model and detail elements chosen together make a model
/// group and a detail group, as Revit makes a model group with an attached detail group.
/// Returns the new instances.
pub fn create(doc: &mut Document, ids: &[ElementId], name: &str) -> CoreResult<Vec<ElementId>> {
    let ids = crate::edit::with_hosted(doc, ids);
    let mut model = vec![];
    let mut detail: Vec<ElementId> = vec![];
    for id in &ids {
        let d = doc.data(*id)?;
        if let Some(g) = group_of(doc, *id) {
            return Err(CoreError::Invalid(format!(
                "{} is already in {}; ungroup it first",
                d.name(),
                doc.data(g).map(|x| x.name()).unwrap_or_default()
            )));
        }
        match kind_of(d) {
            Some(GroupKind::Model) => model.push(*id),
            Some(GroupKind::Detail) => detail.push(*id),
            None if matches!(d, ElementData::Group { .. }) => {
                return Err(CoreError::Invalid(
                    "groups can't be nested here: ungroup it first".into(),
                ))
            }
            None => {}
        }
    }
    if model.is_empty() && detail.is_empty() {
        return Err(CoreError::Invalid(
            "select model elements (walls, doors, floors…) or a view's annotation to group".into(),
        ));
    }
    let views: HashSet<ElementId> = detail
        .iter()
        .filter_map(|m| doc.data(*m).ok().and_then(view_of))
        .collect();
    if views.len() > 1 {
        return Err(CoreError::Invalid(
            "a detail group's elements have to be in one view".into(),
        ));
    }
    let taken: HashSet<String> = types(doc).into_iter().map(|t| t.name).collect();
    let name = name.trim();
    let mut plan = vec![];
    for (kind, members) in [(GroupKind::Model, model), (GroupKind::Detail, detail)] {
        if members.is_empty() {
            continue;
        }
        let mut n = if name.is_empty() {
            next_name(doc, kind)
        } else {
            name.to_string()
        };
        if kind == GroupKind::Detail && plan.iter().any(|(k, _, _)| *k == GroupKind::Model) {
            n = format!("{n} (Detail)");
        }
        if taken.contains(&n) {
            return Err(CoreError::Invalid(format!(
                "there's already a group named {n}"
            )));
        }
        plan.push((kind, n, members));
    }
    let lowest = |ms: &[ElementId]| {
        let levels = doc.levels();
        ms.iter()
            .filter_map(|m| doc.data(*m).ok().and_then(|d| d.level()))
            .min_by(|a, b| {
                let e = |l: &ElementId| levels.iter().find(|x| x.0 == *l).map_or(0.0, |x| x.2);
                e(a).total_cmp(&e(b))
            })
    };
    let rows: Vec<_> = plan
        .into_iter()
        .map(|(kind, n, members)| {
            let origin = center(doc, &members);
            let level = if kind == GroupKind::Model {
                lowest(&members)
            } else {
                None
            };
            let view = if kind == GroupKind::Detail {
                views.iter().next().copied()
            } else {
                None
            };
            (kind, n, members, origin, level, view)
        })
        .collect();
    doc.transact("Create Group", |tx| {
        let mut out = vec![];
        for (kind, n, members, origin, level, view) in rows {
            let t = tx.insert(ElementData::GroupType { name: n, kind });
            out.push(tx.insert(ElementData::Group {
                type_id: t,
                origin,
                angle: 0.0,
                mirrored: false,
                level,
                view,
                members,
            }));
        }
        Ok(out)
    })
}

/// Ungroup: the members stay, the instance goes (the type stays, as in Revit).
pub fn ungroup(doc: &mut Document, groups: &[ElementId]) -> CoreResult<()> {
    for g in groups {
        instance(doc, *g)?;
    }
    doc.transact("Ungroup", |tx| {
        for g in groups {
            tx.delete(*g)?;
        }
        Ok(())
    })
}

/// Maps level ids when a model group moves from one level to another: each level the
/// members use goes to the level as many steps up or down.
fn level_map(
    doc: &Document,
    from: Option<ElementId>,
    to: Option<ElementId>,
) -> HashMap<String, String> {
    let (Some(from), Some(to)) = (from, to) else {
        return HashMap::new();
    };
    if from == to {
        return HashMap::new();
    }
    let mut levels = doc.levels();
    levels.sort_by(|a, b| a.2.total_cmp(&b.2));
    let idx = |l: ElementId| levels.iter().position(|x| x.0 == l);
    let (Some(a), Some(b)) = (idx(from), idx(to)) else {
        return HashMap::new();
    };
    let shift = b as i64 - a as i64;
    levels
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let j = i as i64 + shift;
            (0..levels.len() as i64)
                .contains(&j)
                .then(|| (id_str(l.0), id_str(levels[j as usize].0)))
        })
        .collect()
}

/// Copies `members` under `x` into the document, moving them to another level or view by
/// `ids` (a map of level or view ids). Returns old → new member ids.
fn copy_members(
    doc: &mut Document,
    members: &[ElementId],
    x: Xform,
    ids: &HashMap<String, String>,
    label: &str,
) -> CoreResult<Vec<ElementId>> {
    let created = crate::edit::copy_elements(doc, members, &[x], label)?;
    if !ids.is_empty() {
        doc.transact(label, |tx| {
            for id in &created {
                let d = tx.data(*id)?.clone();
                tx.set(*id, remapped(&d, ids))?;
            }
            Ok(())
        })?;
    }
    Ok(created)
}

/// The motion from instance A to instance B: undo A's placement, then do B's.
fn between(a: (Pt, f64, bool), b: (Pt, f64, bool)) -> Xform {
    let mut x =
        Xform::translate(Pt::new(-a.0.x, -a.0.y)).then(Xform::rotate(Pt::new(0.0, 0.0), -a.1));
    if a.2 != b.2 {
        x = x.then(Xform::mirror(Pt::new(0.0, 0.0), Pt::new(0.0, 1.0)));
    }
    x.then(Xform::rotate(Pt::new(0.0, 0.0), b.1))
        .then(Xform::translate(b.0))
}

/// Revit's Place Group: a new instance of `type_id` with its center at `at`, on `level`
/// (model groups) or in `view` (detail groups). Copies an existing instance.
pub fn place(
    doc: &mut Document,
    type_id: ElementId,
    at: Pt,
    level: Option<ElementId>,
    view: Option<ElementId>,
) -> CoreResult<ElementId> {
    let src = groups(doc)
        .into_iter()
        .find(|g| g.type_id == type_id && !g.members.is_empty())
        .ok_or_else(|| {
            CoreError::Invalid(
                "the group has no instance left to copy: place one from an undo or recreate it"
                    .into(),
            )
        })?;
    let (_, _, angle, mirrored, _, _, _) = instance(doc, src.id)?;
    let mark = doc.undo_depth();
    let mut ids = level_map(doc, src.level, level);
    if let (Some(a), Some(b)) = (src.view, view) {
        if a != b {
            ids.insert(id_str(a), id_str(b));
        }
    }
    let x = Xform::translate(at.sub(src.origin));
    let copy = copy_members(doc, &src.members, x, &ids, "Place Group");
    let members = match copy {
        Ok(m) => m,
        Err(e) => {
            while doc.undo_depth() > mark {
                doc.undo()?;
            }
            return Err(e);
        }
    };
    let id = doc.transact("Place Group", |tx| {
        Ok(tx.insert(ElementData::Group {
            type_id,
            origin: at,
            angle,
            mirrored,
            level: level.or(src.level),
            view: view.or(src.view),
            members,
        }))
    })?;
    doc.merge_undo(mark, "Place Group");
    Ok(id)
}

/// Adds elements to a group (while editing it).
pub fn add_members(doc: &mut Document, group: ElementId, ids: &[ElementId]) -> CoreResult<()> {
    let (type_id, ..) = instance(doc, group)?;
    let kind = match doc.data(type_id)? {
        ElementData::GroupType { kind, .. } => *kind,
        _ => return Err(CoreError::Invalid("the group has no type".into())),
    };
    let ids = crate::edit::with_hosted(doc, ids);
    for id in &ids {
        if kind_of(doc.data(*id)?) != Some(kind) {
            return Err(CoreError::Invalid(format!(
                "{} can't go in a {}",
                doc.data(*id)?.name(),
                kind.label().to_lowercase()
            )));
        }
        if group_of(doc, *id).is_some_and(|g| g != group) {
            return Err(CoreError::Invalid(format!(
                "{} is in another group",
                doc.data(*id)?.name()
            )));
        }
    }
    doc.transact("Add to Group", |tx| {
        tx.modify(group, |d| {
            if let ElementData::Group { members, .. } = d {
                for id in &ids {
                    if !members.contains(id) {
                        members.push(*id);
                    }
                }
            }
        })
    })
}

/// Takes elements out of a group (they stay in the model).
pub fn remove_members(doc: &mut Document, group: ElementId, ids: &[ElementId]) -> CoreResult<()> {
    instance(doc, group)?;
    doc.transact("Remove from Group", |tx| {
        tx.modify(group, |d| {
            if let ElementData::Group { members, .. } = d {
                members.retain(|m| !ids.contains(m));
            }
        })
    })
}

/// Finish (Edit Group): every other instance of the edited group's type is rebuilt from
/// it, placed as that instance is. One transaction each; the caller merges them.
pub fn sync(doc: &mut Document, edited: ElementId) -> CoreResult<usize> {
    let (type_id, origin, angle, mirrored, level, view, members) = instance(doc, edited)?;
    let others: Vec<GroupInfo> = groups(doc)
        .into_iter()
        .filter(|g| g.type_id == type_id && g.id != edited)
        .collect();
    for o in &others {
        let (_, o_origin, o_angle, o_mirrored, o_level, o_view, o_members) = instance(doc, o.id)?;
        let x = between((origin, angle, mirrored), (o_origin, o_angle, o_mirrored));
        let mut ids = level_map(doc, level, o_level);
        if let (Some(a), Some(b)) = (view, o_view) {
            if a != b {
                ids.insert(id_str(a), id_str(b));
            }
        }
        doc.transact("Update Group", |tx| {
            for m in &o_members {
                if tx.get(*m).is_some() {
                    tx.delete(*m)?;
                }
            }
            Ok(())
        })?;
        let fresh = if members.is_empty() {
            vec![]
        } else {
            copy_members(doc, &members, x, &ids, "Update Group")?
        };
        doc.transact("Update Group", |tx| {
            tx.modify(o.id, |d| {
                if let ElementData::Group { members, .. } = d {
                    *members = fresh.clone();
                }
            })
        })?;
    }
    Ok(others.len())
}

/// Renames a group type.
pub fn rename_type(doc: &mut Document, type_id: ElementId, name: &str) -> CoreResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("a group needs a name".into()));
    }
    if types(doc).iter().any(|t| t.name == name && t.id != type_id) {
        return Err(CoreError::Invalid(format!(
            "there's already a group named {name}"
        )));
    }
    doc.transact("Rename Group", |tx| {
        tx.modify(type_id, |d| {
            if let ElementData::GroupType { name: n, .. } = d {
                *n = name.into();
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use crate::units::MM_PER_FT;

    fn room(doc: &mut Document, l1: ElementId) -> Vec<ElementId> {
        let wt = ops::first_of(doc, Category::WallType).unwrap();
        let (w, h) = (10.0 * MM_PER_FT, 8.0 * MM_PER_FT);
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ];
        let mut ids: Vec<ElementId> = (0..4)
            .map(|i| ops::create_wall(doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
            .collect();
        let dt = ops::first_of(doc, Category::DoorType).unwrap();
        ids.push(ops::create_door(doc, dt, ids[0], 4.0 * MM_PER_FT, false).unwrap());
        ids
    }

    fn setup() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        (doc, l1)
    }

    #[test]
    fn create_groups_walls_with_their_doors_about_the_center() {
        let (mut doc, l1) = setup();
        let r = room(&mut doc, l1);
        // Walls only: the door comes along.
        let g = create(&mut doc, &r[..4], "Unit A").unwrap();
        assert_eq!(g.len(), 1);
        let info = &groups(&doc)[0];
        assert_eq!(
            (info.name.as_str(), info.kind, info.members.len()),
            ("Unit A", GroupKind::Model, 5)
        );
        // The center of a 10' x 8' room.
        assert!(info.origin.dist(Pt::new(5.0 * MM_PER_FT, 4.0 * MM_PER_FT)) < 1.0);
        assert_eq!(info.level, Some(l1));
        assert_eq!(group_of(&doc, r[2]), Some(g[0]));
        // Already grouped: refused; the same name: refused.
        assert!(create(&mut doc, &r[..1], "B").is_err());
        let r2 = room(&mut doc, l1);
        assert!(create(&mut doc, &r2[..1], "Unit A").is_err());
        assert_eq!(types(&doc)[0].instances, 1);
    }

    #[test]
    fn place_copies_the_members_and_moving_a_group_moves_them_all() {
        let (mut doc, l1) = setup();
        let r = room(&mut doc, l1);
        let g = create(&mut doc, &r, "Unit A").unwrap()[0];
        let t = groups(&doc)[0].type_id;
        let depth = doc.undo_depth();
        let at = Pt::new(30.0 * MM_PER_FT, 4.0 * MM_PER_FT);
        let g2 = place(&mut doc, t, at, Some(l1), None).unwrap();
        assert_eq!(doc.undo_depth(), depth + 1, "one undo step");
        let i2 = groups(&doc).into_iter().find(|x| x.id == g2).unwrap();
        assert_eq!(i2.members.len(), 5);
        assert!(i2.members.iter().all(|m| !r.contains(m)));
        // Its first wall starts 25' to the right of the original's.
        fn start(doc: &Document, id: ElementId) -> Pt {
            match doc.data(id).unwrap() {
                ElementData::Wall { start, .. } => *start,
                _ => panic!(),
            }
        }
        let walls2: Vec<ElementId> = i2
            .members
            .iter()
            .copied()
            .filter(|m| matches!(doc.data(*m), Ok(ElementData::Wall { .. })))
            .collect();
        assert!(walls2
            .iter()
            .any(|w| start(&doc, *w).dist(Pt::new(25.0 * MM_PER_FT, 0.0)) < 1.0));
        // Moving the group moves its members and its origin.
        crate::modify::move_elements(&mut doc, &[g], Pt::new(0.0, 1000.0)).unwrap();
        assert!(start(&doc, r[0]).dist(Pt::new(0.0, 1000.0)) < 1e-6);
        assert!(
            groups(&doc)
                .iter()
                .find(|x| x.id == g)
                .unwrap()
                .origin
                .dist(Pt::new(5.0 * MM_PER_FT, 4.0 * MM_PER_FT + 1000.0))
                < 1.0
        );
        // Copying it makes a third instance; deleting one deletes its members.
        crate::edit::copy_elements(
            &mut doc,
            &[g2],
            &[Xform::translate(Pt::new(0.0, 20.0 * MM_PER_FT))],
            "Copy",
        )
        .unwrap();
        assert_eq!(types(&doc)[0].instances, 3);
        let before = doc.of(Category::Wall).count();
        ops::delete(&mut doc, &[g2]).unwrap();
        assert_eq!(doc.of(Category::Wall).count(), before - 4);
        assert_eq!(types(&doc)[0].instances, 2);
    }

    #[test]
    fn finishing_an_edit_repeats_it_in_every_instance() {
        let (mut doc, l1) = setup();
        let r = room(&mut doc, l1);
        let g = create(&mut doc, &r, "Unit A").unwrap()[0];
        let t = groups(&doc)[0].type_id;
        let g2 = place(
            &mut doc,
            t,
            Pt::new(30.0 * MM_PER_FT, 4.0 * MM_PER_FT),
            Some(l1),
            None,
        )
        .unwrap();
        // Edit the first: add a wall down the middle.
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let w = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(5.0 * MM_PER_FT, 0.0),
            Pt::new(5.0 * MM_PER_FT, 8.0 * MM_PER_FT),
        )
        .unwrap();
        add_members(&mut doc, g, &[w]).unwrap();
        assert_eq!(sync(&mut doc, g).unwrap(), 1);
        let i2 = groups(&doc).into_iter().find(|x| x.id == g2).unwrap();
        assert_eq!(i2.members.len(), 6);
        // The new wall lands at x = 30' in the copy.
        assert!(i2.members.iter().any(|m| matches!(doc.data(*m), Ok(ElementData::Wall { start, .. }) if start.dist(Pt::new(30.0 * MM_PER_FT, 0.0)) < 1.0)));
        // Removing a member takes it out, and ungrouping keeps the members.
        remove_members(&mut doc, g, &[w]).unwrap();
        assert_eq!(
            groups(&doc)
                .iter()
                .find(|x| x.id == g)
                .unwrap()
                .members
                .len(),
            5
        );
        let walls = doc.of(Category::Wall).count();
        ungroup(&mut doc, &[g]).unwrap();
        assert_eq!(doc.of(Category::Wall).count(), walls);
        assert_eq!(groups(&doc).len(), 1);
    }

    #[test]
    fn a_rotated_instance_gets_the_edit_rotated() {
        let a = (Pt::new(0.0, 0.0), 0.0, false);
        let b = (Pt::new(100.0, 0.0), std::f64::consts::FRAC_PI_2, false);
        let x = between(a, b);
        // A point 10 east of A's origin lands 10 north of B's.
        assert!(x.apply(Pt::new(10.0, 0.0)).dist(Pt::new(100.0, 10.0)) < 1e-9);
    }

    #[test]
    fn detail_elements_make_a_detail_group_in_their_view() {
        let (mut doc, _) = setup();
        let view = ops::first_of(&doc, Category::View).unwrap();
        let t = crate::ops::create_text(&mut doc, view, Pt::new(0.0, 0.0), "NOTE").unwrap();
        let g = create(&mut doc, &[t], "").unwrap();
        let info = groups(&doc).into_iter().find(|x| x.id == g[0]).unwrap();
        assert_eq!(
            (info.kind, info.view, info.name.as_str()),
            (GroupKind::Detail, Some(view), "Detail Group 1")
        );
    }
}
