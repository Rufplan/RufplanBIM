//! Worksets (ADR-079), set up as Revit's are once worksharing is enabled: user-created
//! worksets that model elements, levels and grids sit on, plus the system worksets that
//! views, annotations and types belong to (read-only, as in Revit).
//!
//! A project starts with Architecture (the default, Revit's renamed Workset1), Shared
//! Levels and Grids, Structural, Interiors, Site, MEP and Linked Models. An element's
//! workset is its `rufplan.workset` parameter; without one it falls back to its category's
//! default, so no element kind changes. The active workset is the session's (per user, as
//! in Revit): new elements go on it unless it is the default one.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::ops::{choice, ro, PropOption, Property};
use crate::params::ParamValue;

/// The parameter holding an element's workset.
pub const KEY: &str = "rufplan.workset";

/// What a workset is for: which elements fall back to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WorksetRole {
    /// Architecture: model elements without another home.
    Default,
    /// Shared Levels and Grids.
    LevelsGrids,
    /// Structural: the structural layer (ADR-080) and what you put there.
    Structural,
    Other,
}

/// The worksets a project starts with, as a Revit office template sets them up.
pub const STANDARD: [(&str, WorksetRole); 7] = [
    ("Architecture", WorksetRole::Default),
    ("Shared Levels and Grids", WorksetRole::LevelsGrids),
    ("Structural", WorksetRole::Structural),
    ("Interiors", WorksetRole::Other),
    ("Site", WorksetRole::Other),
    ("MEP", WorksetRole::Other),
    ("Linked Models", WorksetRole::Other),
];

/// A workset for the Worksets dialog.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorksetInfo {
    pub id: ElementId,
    pub name: String,
    pub role: WorksetRole,
    pub visible_in_all_views: bool,
    /// Elements on it (explicitly or by default).
    pub count: usize,
}

/// Whether an element sits on a user-created workset: model elements, levels and grids.
/// Views, annotations, types and settings belong to Revit's system worksets instead.
pub fn carries_workset(data: &ElementData) -> bool {
    matches!(
        data,
        ElementData::Level { .. }
            | ElementData::Grid { .. }
            | ElementData::Wall { .. }
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
            | ElementData::Site { .. }
            | ElementData::ModelLine { .. }
            | ElementData::LightingFixture { .. }
            | ElementData::WallOpening { .. }
            | ElementData::Planting { .. }
            | ElementData::GroundRegion { .. }
            | ElementData::GrassPatch { .. }
            | ElementData::InPlace { .. }
            | ElementData::StructuralScheme { .. }
    )
}

/// The project's worksets, in the order they were made.
pub fn worksets(doc: &Document) -> Vec<(ElementId, String, WorksetRole, bool)> {
    let mut v: Vec<_> = doc
        .of(Category::Workset)
        .filter_map(|e| match &e.data {
            ElementData::Workset {
                name,
                role,
                visible_in_all_views,
            } => Some((e.id, name.clone(), *role, *visible_in_all_views)),
            _ => None,
        })
        .collect();
    // Standard ones first in their template order, then yours by name.
    let rank = |n: &str| STANDARD.iter().position(|s| s.0 == n).unwrap_or(99);
    v.sort_by(|a, b| (rank(&a.1), &a.1).cmp(&(rank(&b.1), &b.1)));
    v
}

fn with_role(doc: &Document, role: WorksetRole) -> Option<ElementId> {
    worksets(doc).into_iter().find(|w| w.2 == role).map(|w| w.0)
}

/// The default workset (Architecture).
pub fn default_workset(doc: &Document) -> Option<ElementId> {
    with_role(doc, WorksetRole::Default)
}

/// Where an element goes when nothing puts it elsewhere: levels and grids on Shared Levels
/// and Grids, the structural layer on Structural, the rest on the default.
fn default_for(doc: &Document, data: &ElementData) -> Option<ElementId> {
    match data {
        ElementData::Level { .. } | ElementData::Grid { .. } => {
            with_role(doc, WorksetRole::LevelsGrids).or_else(|| default_workset(doc))
        }
        ElementData::StructuralScheme { .. } => {
            with_role(doc, WorksetRole::Structural).or_else(|| default_workset(doc))
        }
        _ => default_workset(doc),
    }
}

/// The user-created workset `el` is on, or None for views, annotations and types.
pub fn workset_of(doc: &Document, el: ElementId) -> Option<ElementId> {
    let data = doc.data(el).ok()?;
    if !carries_workset(data) {
        return None;
    }
    if let Some(ParamValue::Text(s)) = doc.param(el, KEY) {
        if let Ok(id) = crate::ops::parse_id(s) {
            if matches!(doc.data(id), Ok(ElementData::Workset { .. })) {
                return Some(id);
            }
        }
    }
    default_for(doc, data)
}

/// What Revit shows as the workset of an element on a system workset: `View "Level 1"` for
/// a view and what's drawn in it, Project Standards for types and settings.
pub fn system_label(doc: &Document, el: ElementId) -> String {
    let Ok(data) = doc.data(el) else {
        return String::new();
    };
    let view = match data {
        ElementData::View { .. } | ElementData::Sheet { .. } => Some(el),
        _ => data.refs().into_iter().find(|r| {
            matches!(
                doc.data(*r),
                Ok(ElementData::View { .. } | ElementData::Sheet { .. })
            )
        }),
    };
    match view {
        Some(v) => format!(
            "View \"{}\"",
            doc.data(v).map(|d| d.name()).unwrap_or_default()
        ),
        None => "Project Standards".into(),
    }
}

/// Makes Revit's standard worksets if the project has none yet (enabling worksharing).
pub fn ensure_worksets(doc: &mut Document) -> CoreResult<()> {
    if doc.count(Category::Workset) > 0 {
        return Ok(());
    }
    doc.transact("Enable Worksharing", |tx| {
        for (name, role) in STANDARD {
            tx.insert(ElementData::Workset {
                name: name.into(),
                role,
                visible_in_all_views: true,
            });
        }
        Ok(())
    })
}

/// The worksets with how many elements each holds.
pub fn list(doc: &Document) -> Vec<WorksetInfo> {
    let mut counts: std::collections::HashMap<ElementId, usize> = Default::default();
    for e in doc.iter() {
        if let Some(w) = workset_of(doc, e.id) {
            *counts.entry(w).or_default() += 1;
        }
    }
    worksets(doc)
        .into_iter()
        .map(|(id, name, role, visible_in_all_views)| WorksetInfo {
            id,
            name,
            role,
            visible_in_all_views,
            count: counts.get(&id).copied().unwrap_or(0),
        })
        .collect()
}

fn check_name(doc: &Document, name: &str, except: Option<ElementId>) -> CoreResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("name the workset".into()));
    }
    if worksets(doc)
        .iter()
        .any(|w| w.1.eq_ignore_ascii_case(name) && Some(w.0) != except)
    {
        return Err(CoreError::Invalid(format!("a workset named {name} exists")));
    }
    Ok(name.to_owned())
}

/// Worksets > New.
pub fn create(doc: &mut Document, name: &str, visible_in_all_views: bool) -> CoreResult<ElementId> {
    let name = check_name(doc, name, None)?;
    doc.transact("New Workset", |tx| {
        Ok(tx.insert(ElementData::Workset {
            name,
            role: WorksetRole::Other,
            visible_in_all_views,
        }))
    })
}

fn modify(
    doc: &mut Document,
    ws: ElementId,
    what: &str,
    f: impl FnOnce(&mut String, &mut bool),
) -> CoreResult<()> {
    if !matches!(doc.data(ws)?, ElementData::Workset { .. }) {
        return Err(CoreError::Invalid("not a workset".into()));
    }
    doc.transact(what, |tx| {
        tx.modify(ws, |d| {
            if let ElementData::Workset {
                name,
                visible_in_all_views,
                ..
            } = d
            {
                f(name, visible_in_all_views);
            }
        })
    })
}

/// Worksets > Rename.
pub fn rename(doc: &mut Document, ws: ElementId, name: &str) -> CoreResult<()> {
    let name = check_name(doc, name, Some(ws))?;
    modify(doc, ws, "Rename Workset", |n, _| *n = name)
}

/// Worksets > Visible in all views.
pub fn set_visible_in_all_views(doc: &mut Document, ws: ElementId, on: bool) -> CoreResult<()> {
    modify(doc, ws, "Workset Visibility", |_, v| *v = on)
}

/// Worksets > Delete: its elements move to `move_to` (Revit asks where), one undo step.
/// The default workset can't be deleted.
pub fn delete(doc: &mut Document, ws: ElementId, move_to: ElementId) -> CoreResult<()> {
    let Ok(ElementData::Workset { role, .. }) = doc.data(ws) else {
        return Err(CoreError::Invalid("not a workset".into()));
    };
    if *role == WorksetRole::Default {
        return Err(CoreError::Invalid(
            "the default workset can't be deleted; rename it instead".into(),
        ));
    }
    if move_to == ws || !matches!(doc.data(move_to), Ok(ElementData::Workset { .. })) {
        return Err(CoreError::Invalid(
            "pick another workset to move its elements to".into(),
        ));
    }
    let on: Vec<ElementId> = doc
        .iter()
        .map(|e| e.id)
        .filter(|id| workset_of(doc, *id) == Some(ws))
        .collect();
    let target = ParamValue::Text(move_to.to_string());
    // Views that hid the deleted workset no longer list it.
    let views: Vec<ElementId> = doc
        .of(Category::View)
        .filter(|e| matches!(&e.data, ElementData::View { hidden_worksets, .. } if hidden_worksets.contains(&ws)))
        .map(|e| e.id)
        .collect();
    doc.transact("Delete Workset", |tx| {
        for id in &on {
            tx.set_param(*id, KEY, Some(target.clone()))?;
        }
        for v in &views {
            tx.modify(*v, |d| {
                if let ElementData::View {
                    hidden_worksets, ..
                } = d
                {
                    hidden_worksets.retain(|w| *w != ws);
                }
            })?;
        }
        tx.delete(ws)?;
        Ok(())
    })
}

/// Moves `ids` to workset `ws` (Properties > Workset), one undo step. Elements that sit on
/// system worksets are skipped.
pub fn set_workset(doc: &mut Document, ids: &[ElementId], ws: ElementId) -> CoreResult<usize> {
    if !matches!(doc.data(ws), Ok(ElementData::Workset { .. })) {
        return Err(CoreError::Invalid("pick a workset".into()));
    }
    let movable: Vec<ElementId> = ids
        .iter()
        .copied()
        .filter(|id| doc.data(*id).is_ok_and(carries_workset))
        .collect();
    if movable.is_empty() {
        return Err(CoreError::Invalid(
            "views, annotations and types stay on their system worksets".into(),
        ));
    }
    let v = ParamValue::Text(ws.to_string());
    doc.transact("Change Workset", |tx| {
        for id in &movable {
            tx.set_param(*id, KEY, Some(v.clone()))?;
        }
        Ok(movable.len())
    })
}

/// Visibility/Graphics > Worksets: shows or hides `ws` in `view`.
pub fn set_visible_in_view(
    doc: &mut Document,
    view: ElementId,
    ws: ElementId,
    visible: bool,
) -> CoreResult<()> {
    if !matches!(doc.data(ws), Ok(ElementData::Workset { .. })) {
        return Err(CoreError::Invalid("pick a workset".into()));
    }
    if !matches!(doc.data(view)?, ElementData::View { .. }) {
        return Err(CoreError::Invalid("not a view".into()));
    }
    doc.transact("Workset Visibility", |tx| {
        tx.modify(view, |d| {
            if let ElementData::View {
                hidden_worksets, ..
            } = d
            {
                hidden_worksets.retain(|w| *w != ws);
                if !visible {
                    hidden_worksets.push(ws);
                }
            }
        })
    })
}

/// Whether `el` is hidden in `view` by its workset: hidden in that view, or (unless the
/// view shows it) not visible in all views.
pub fn hidden_by_workset(doc: &Document, view: &ElementData, el: ElementId) -> bool {
    let Some(ws) = workset_of(doc, el) else {
        return false;
    };
    if let ElementData::View {
        hidden_worksets, ..
    } = view
    {
        if hidden_worksets.contains(&ws) {
            return true;
        }
    }
    matches!(
        doc.data(ws),
        Ok(ElementData::Workset {
            visible_in_all_views: false,
            ..
        })
    )
}

/// Whether any workset could hide something in `view` (so drawing can skip the check).
pub fn any_hidden(doc: &Document, view: &ElementData) -> bool {
    matches!(view, ElementData::View { hidden_worksets, .. } if !hidden_worksets.is_empty())
        || worksets(doc).iter().any(|w| !w.3)
}

/// The Workset row of an element's properties: a choice for model elements, the system
/// workset's name (read-only) for the rest.
pub fn property(doc: &Document, el: ElementId) -> Option<Property> {
    if doc.count(Category::Workset) == 0 {
        return None;
    }
    let data = doc.data(el).ok()?;
    if matches!(data, ElementData::Workset { .. }) {
        return None;
    }
    if !carries_workset(data) {
        return Some(ro(
            "workset",
            "Workset",
            "Identity Data",
            system_label(doc, el),
        ));
    }
    let options = worksets(doc)
        .into_iter()
        .map(|w| PropOption {
            id: w.0.to_string(),
            label: w.1,
        })
        .collect();
    Some(choice(
        "workset",
        "Workset",
        "Identity Data",
        workset_of(doc, el)
            .map(|w| w.to_string())
            .unwrap_or_default(),
        options,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use studio_geom::Pt;

    fn doc() -> Document {
        let mut d = Document::new();
        ops::seed_default_project(&mut d).unwrap();
        d
    }

    fn named(d: &Document, n: &str) -> ElementId {
        worksets(d).into_iter().find(|w| w.1 == n).unwrap().0
    }

    #[test]
    fn projects_start_with_revits_worksets_and_elements_fall_back_by_category() {
        let mut d = doc();
        let names: Vec<String> = worksets(&d).into_iter().map(|w| w.1).collect();
        assert_eq!(
            names,
            [
                "Architecture",
                "Shared Levels and Grids",
                "Structural",
                "Interiors",
                "Site",
                "MEP",
                "Linked Models"
            ]
        );
        let l1 = d.levels()[0].0;
        let wt = ops::first_of(&d, Category::WallType).unwrap();
        let wall =
            ops::create_wall(&mut d, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let arch = named(&d, "Architecture");
        assert_eq!(default_workset(&d), Some(arch));
        assert_eq!(workset_of(&d, wall), Some(arch));
        assert_eq!(
            workset_of(&d, l1),
            Some(named(&d, "Shared Levels and Grids"))
        );
        // Types and views sit on system worksets.
        assert_eq!(workset_of(&d, wt), None);
        assert_eq!(system_label(&d, wt), "Project Standards");
        let plan = d
            .of(Category::View)
            .find(|e| e.data.name() == "Level 1")
            .unwrap()
            .id;
        assert_eq!(system_label(&d, plan), "View \"Level 1\"");
        // Ensuring again adds nothing.
        ensure_worksets(&mut d).unwrap();
        assert_eq!(d.count(Category::Workset), 7);
    }

    #[test]
    fn the_active_workset_takes_new_elements_and_moving_is_one_undo() {
        let mut d = doc();
        let l1 = d.levels()[0].0;
        let wt = ops::first_of(&d, Category::WallType).unwrap();
        let interiors = named(&d, "Interiors");
        d.set_active_workset(Some(interiors));
        let wall =
            ops::create_wall(&mut d, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        assert_eq!(workset_of(&d, wall), Some(interiors));
        // Undo removes the wall with its workset in the one step.
        d.undo().unwrap();
        assert!(d.data(wall).is_err());
        // With the default active, levels still go to Shared Levels and Grids.
        d.set_active_workset(default_workset(&d));
        let l2 = ops::create_level(&mut d, 3000.0).unwrap();
        assert_eq!(
            workset_of(&d, l2),
            Some(named(&d, "Shared Levels and Grids"))
        );
        let w2 = ops::create_wall(&mut d, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let s = named(&d, "Structural");
        assert_eq!(set_workset(&mut d, &[w2, wt], s).unwrap(), 1);
        assert_eq!(workset_of(&d, w2), Some(s));
        assert!(set_workset(&mut d, &[wt], s).is_err());
    }

    #[test]
    fn visibility_rename_new_and_delete_move_elements() {
        let mut d = doc();
        let l1 = d.levels()[0].0;
        let wt = ops::first_of(&d, Category::WallType).unwrap();
        let wall =
            ops::create_wall(&mut d, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let plan = d
            .of(Category::View)
            .find(|e| e.data.name() == "Level 1")
            .unwrap()
            .id;
        let arch = named(&d, "Architecture");
        let v = |d: &Document| d.data(plan).unwrap().clone();
        assert!(!hidden_by_workset(&d, &v(&d), wall));
        set_visible_in_view(&mut d, plan, arch, false).unwrap();
        assert!(hidden_by_workset(&d, &v(&d), wall) && any_hidden(&d, &v(&d)));
        set_visible_in_view(&mut d, plan, arch, true).unwrap();
        set_visible_in_all_views(&mut d, arch, false).unwrap();
        assert!(hidden_by_workset(&d, &v(&d), wall));
        set_visible_in_all_views(&mut d, arch, true).unwrap();

        let fx = create(&mut d, "Furniture", true).unwrap();
        assert!(create(&mut d, "furniture", true).is_err());
        rename(&mut d, fx, "FF&E").unwrap();
        set_workset(&mut d, &[wall], fx).unwrap();
        assert_eq!(list(&d).iter().find(|w| w.id == fx).unwrap().count, 1);
        assert!(delete(&mut d, arch, fx).is_err());
        delete(&mut d, fx, arch).unwrap();
        assert_eq!(workset_of(&d, wall), Some(arch));
        d.undo().unwrap();
        assert_eq!(workset_of(&d, wall), Some(fx));

        // Properties: a choice for the wall, read-only for its type.
        let p = property(&d, wall).unwrap();
        assert_eq!(p.value, fx.to_string());
        assert_eq!(p.options.len(), 8);
        assert_eq!(property(&d, wt).unwrap().value, "Project Standards");
    }
}
