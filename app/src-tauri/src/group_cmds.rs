//! IPC for model and detail groups (ADR-087): thin wrappers over studio-core `groups`, and
//! Revit's Edit Group mode (an undo mark, Add/Remove, Finish repeats the edit in every
//! instance, Cancel undoes it).

use std::collections::HashSet;

use anyhow::bail;
use studio_core::groups::{self, GroupKind};
use studio_core::{Document, ElementData, ElementId, ViewKind};
use studio_geom::Pt;
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::{AppState, Session};

type StateResult = Result<Option<AppState>, CommandError>;

/// Edit Group in progress: the instance, the undo mark it opened at, and what existed then
/// (what's drawn while editing joins the group).
#[derive(Debug, Clone)]
pub struct GroupEdit {
    pub id: ElementId,
    pub mark: usize,
    pub before: HashSet<ElementId>,
}

fn run(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&mut Session) -> anyhow::Result<()>,
) -> StateResult {
    let mut session = lock(state)?;
    f(&mut session)?;
    finish(window, &session)
}

/// Create Group (GP): the selection as a new group named `name` (empty: "Group 1"…).
#[tauri::command]
pub fn group_create(
    ids: Vec<ElementId>,
    name: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<(Vec<ElementId>, Option<AppState>), CommandError> {
    let mut session = lock(&state)?;
    if session.group_edit.is_some() {
        return Err(anyhow::anyhow!("Finish or cancel the group you're editing first.").into());
    }
    let made = session.edit(|d| groups::create(d, &ids, &name))?;
    Ok((made, finish(&window, &session)?))
}

#[tauri::command]
pub fn group_ungroup(
    ids: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        s.edit(|d| groups::ungroup(d, &ids))?;
        Ok(())
    })
}

/// The level a plan view shows (model groups go on it).
fn view_level(doc: &Document, view: ElementId) -> Option<ElementId> {
    match doc.data(view) {
        Ok(ElementData::View {
            kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
            ..
        }) => Some(*level),
        _ => None,
    }
}

/// Place Group: an instance of `type_id` centered at `at` in `view` (a model group on the
/// view's level, a detail group in the view).
#[tauri::command]
pub fn group_place(
    type_id: ElementId,
    at: Pt,
    view: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        let doc = s.doc()?;
        let kind = match doc.data(type_id)? {
            ElementData::GroupType { kind, .. } => *kind,
            _ => bail!("Pick a group to place."),
        };
        let (level, v) = match kind {
            GroupKind::Model => (view_level(doc, view), None),
            GroupKind::Detail => (None, Some(view)),
        };
        s.edit(|d| groups::place(d, type_id, at, level, v))?;
        Ok(())
    })
}

/// Edit Group: opens the group for editing.
#[tauri::command]
pub fn group_edit(
    id: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        if s.group_edit.is_some() || s.in_place.is_some() {
            bail!("Finish what you're editing first.");
        }
        let doc = s.doc()?;
        if !matches!(doc.data(id)?, ElementData::Group { .. }) {
            bail!("Select a group to edit it.");
        }
        s.group_edit = Some(GroupEdit {
            id,
            mark: doc.undo_depth(),
            before: doc.iter().map(|e| e.id).collect(),
        });
        Ok(())
    })
}

fn editing(s: &Session) -> anyhow::Result<GroupEdit> {
    match &s.group_edit {
        Some(e) => Ok(e.clone()),
        None => bail!("Edit a group first."),
    }
}

#[tauri::command]
pub fn group_add(
    ids: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        let e = editing(s)?;
        s.edit(|d| groups::add_members(d, e.id, &ids))?;
        Ok(())
    })
}

#[tauri::command]
pub fn group_remove(
    ids: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        let e = editing(s)?;
        s.edit(|d| groups::remove_members(d, e.id, &ids))?;
        Ok(())
    })
}

/// Finish (✓): elements drawn while editing join the group, every other instance is
/// rebuilt from it, and it's all one undo step.
#[tauri::command]
pub fn group_finish(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    run(&window, &state, finish_group)
}

pub(crate) fn finish_group(s: &mut Session) -> anyhow::Result<()> {
    let Some(e) = s.group_edit.clone() else {
        return Ok(());
    };
    let doc = s.doc()?;
    if doc.get(e.id).is_none() {
        // Undone past its creation, or deleted.
        s.group_edit = None;
        return Ok(());
    }
    let info = groups::groups(doc).into_iter().find(|g| g.id == e.id);
    let new: Vec<ElementId> = match &info {
        Some(g) => doc
            .iter()
            .filter(|x| !e.before.contains(&x.id))
            .filter(|x| groups::kind_of(&x.data) == Some(g.kind))
            .filter(|x| match g.kind {
                GroupKind::Model => x.data.level() == g.level || x.data.level().is_none(),
                GroupKind::Detail => groups::view_of(&x.data) == g.view,
            })
            .filter(|x| groups::group_of(doc, x.id).is_none())
            .map(|x| x.id)
            .collect(),
        None => vec![],
    };
    s.edit(|d| {
        if !new.is_empty() {
            groups::add_members(d, e.id, &new)?;
        }
        groups::sync(d, e.id)?;
        d.merge_undo(e.mark, "Edit Group");
        Ok(())
    })?;
    s.group_edit = None;
    Ok(())
}

/// Cancel (✗): undoes everything since Edit Group.
#[tauri::command]
pub fn group_cancel(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    run(&window, &state, |s| {
        let Some(e) = s.group_edit.clone() else {
            return Ok(());
        };
        s.edit(|d| {
            while d.undo_depth() > e.mark {
                d.undo()?;
            }
            Ok(())
        })?;
        s.group_edit = None;
        Ok(())
    })
}

/// Deletes a group type and every instance of it (with their members).
#[tauri::command]
pub fn group_delete_type(
    type_id: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        let doc = s.doc()?;
        let mut ids: Vec<ElementId> = groups::groups(doc)
            .into_iter()
            .filter(|g| g.type_id == type_id)
            .map(|g| g.id)
            .collect();
        ids.push(type_id);
        s.edit(|d| studio_core::ops::delete(d, &ids).map(|_| ()))?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_group_finish_takes_new_walls_and_is_one_undo_step() {
        let mut s = Session::default();
        s.new_project("0").unwrap();
        let (l1, wt) = {
            let d = s.doc().unwrap();
            (
                d.levels()[0].0,
                studio_core::ops::first_of(d, studio_core::Category::WallType).unwrap(),
            )
        };
        let w = s
            .edit(|d| {
                studio_core::ops::create_wall(d, wt, l1, Pt::new(0.0, 0.0), Pt::new(3000.0, 0.0))
            })
            .unwrap();
        let g = s.edit(|d| groups::create(d, &[w], "A")).unwrap()[0];
        let t = groups::groups(s.doc().unwrap())[0].type_id;
        s.edit(|d| groups::place(d, t, Pt::new(1500.0, 5000.0), Some(l1), None))
            .unwrap();
        let depth = s.doc().unwrap().undo_depth();
        let mark = depth;
        s.group_edit = Some(GroupEdit {
            id: g,
            mark,
            before: s.doc().unwrap().iter().map(|e| e.id).collect(),
        });
        // A wall drawn while editing joins the group, and both instances get it.
        s.edit(|d| {
            studio_core::ops::create_wall(d, wt, l1, Pt::new(0.0, 0.0), Pt::new(0.0, 3000.0))
        })
        .unwrap();
        finish_group(&mut s).unwrap();
        let doc = s.doc().unwrap();
        assert!(groups::groups(doc).iter().all(|g| g.members.len() == 2));
        assert_eq!(doc.undo_depth(), depth + 1);
        assert_eq!(doc.can_undo(), Some("Edit Group"));
    }
}
