//! The In-Place Editor (ADR-068): Model In-Place in a chosen category, its forms sketched
//! one by one, and Finish or Cancel Model. Thin wrappers over studio-core `inplace`.
//!
//! Each form is saved as it's finished; Finish Model folds everything since the editor
//! opened into one undo step, and Cancel Model undoes back to where it opened.

use anyhow::{bail, Context};
use serde::Serialize;
use studio_core::inplace::{self, FormKind};
use studio_core::sketch::{self, SketchKind};
use studio_core::{Category, Document, ElementData, ElementId};
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::{AppState, Session};
use crate::sketching::{FormDraft, SketchSession};

type CommandResult<T> = Result<T, CommandError>;
type StateResult = Result<Option<AppState>, CommandError>;

/// The editor's element, and the undo depth it opened at.
#[derive(Debug, Clone, Copy)]
pub struct InPlaceEdit {
    pub id: ElementId,
    pub mark: usize,
    /// Model In-Place (a new element) rather than Edit In-Place.
    pub new: bool,
}

/// The editor, for the UI.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InPlaceInfo {
    pub id: ElementId,
    pub name: String,
    pub category: Category,
    pub category_label: String,
    /// Its forms, numbered by kind ("Extrusion 1", "Void Extrusion 1").
    pub forms: Vec<String>,
    pub is_new: bool,
}

pub fn info(doc: &Document, e: &InPlaceEdit) -> Option<InPlaceInfo> {
    let ElementData::InPlace {
        name,
        category,
        forms,
        ..
    } = doc.data(e.id).ok()?
    else {
        return None;
    };
    Some(InPlaceInfo {
        id: e.id,
        name: name.clone(),
        category: *category,
        category_label: inplace::label(*category).unwrap_or_default().into(),
        forms: inplace::form_labels(forms),
        is_new: e.new,
    })
}

/// A choice in the Family Category and Parameters dialog.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct CategoryChoice {
    pub category: Category,
    pub label: String,
}

#[tauri::command]
pub fn in_place_categories() -> Vec<CategoryChoice> {
    inplace::CATEGORIES
        .iter()
        .map(|(c, l)| CategoryChoice {
            category: *c,
            label: (*l).into(),
        })
        .collect()
}

/// Which of `ids` are in-place elements (for Edit In-Place).
#[tauri::command]
pub fn in_place_of(
    ids: Vec<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<Vec<ElementId>> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    Ok(ids
        .into_iter()
        .filter(|id| matches!(doc.data(*id), Ok(ElementData::InPlace { .. })))
        .collect())
}

#[tauri::command]
pub fn in_place_default_name(
    category: Category,
    state: State<'_, SessionState>,
) -> CommandResult<String> {
    let session = lock(&state)?;
    Ok(inplace::default_name(session.doc()?, category))
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

/// Model In-Place: a new element of `category` on the view's level (or `level`, outside a
/// plan), and the editor open on it.
#[tauri::command]
pub fn in_place_begin(
    view: ElementId,
    category: Category,
    name: Option<String>,
    level: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        begin(s, view, category, name.as_deref(), level)
    })
}

pub(crate) fn begin(
    s: &mut Session,
    view: ElementId,
    category: Category,
    name: Option<&str>,
    level: Option<ElementId>,
) -> anyhow::Result<()> {
    if s.in_place.is_some() {
        bail!("Finish or cancel the model you're editing first.");
    }
    let level = match (s.view_level(view), level) {
        (Ok(l), _) | (Err(_), Some(l)) => l,
        (Err(_), None) => s
            .doc()?
            .levels()
            .first()
            .map(|l| l.0)
            .context("the project has no levels")?,
    };
    let mark = s.doc()?.undo_depth();
    let id = s.edit(|d| inplace::create(d, category, name, level))?;
    s.in_place = Some(InPlaceEdit {
        id,
        mark,
        new: true,
    });
    Ok(())
}

/// Edit In-Place: the editor open on an existing in-place element.
#[tauri::command]
pub fn in_place_edit(
    id: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        if s.in_place.is_some() {
            bail!("Finish or cancel the model you're editing first.");
        }
        let doc = s.doc()?;
        if !matches!(doc.data(id)?, ElementData::InPlace { .. }) {
            bail!("Select an in-place element to edit it.");
        }
        let mark = doc.undo_depth();
        s.in_place = Some(InPlaceEdit {
            id,
            mark,
            new: false,
        });
        Ok(())
    })
}

/// Finish Model (✓): one undo step for everything done in the editor.
#[tauri::command]
pub fn in_place_finish(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    run(&window, &state, finish_model)
}

pub(crate) fn finish_model(s: &mut Session) -> anyhow::Result<()> {
    let Some(e) = s.in_place else {
        return Ok(());
    };
    if s.sketch().is_some() {
        bail!("Finish or cancel the sketch first.");
    }
    let empty = match s.doc()?.data(e.id) {
        Ok(ElementData::InPlace { forms, .. }) => forms.is_empty(),
        // Undone past its creation: nothing left to finish.
        _ => {
            s.in_place = None;
            return Ok(());
        }
    };
    if empty {
        bail!("Add a form (Extrusion, Blend or Sweep) to the model, or Cancel Model.");
    }
    let name = if e.new {
        "Model In-Place"
    } else {
        "Edit In-Place"
    };
    s.edit(|d| {
        d.merge_undo(e.mark, name);
        Ok(())
    })?;
    s.in_place = None;
    Ok(())
}

/// Cancel Model (✗): undoes everything since the editor opened.
#[tauri::command]
pub fn in_place_cancel(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    run(&window, &state, cancel_model)
}

pub(crate) fn cancel_model(s: &mut Session) -> anyhow::Result<()> {
    let Some(e) = s.in_place else {
        return Ok(());
    };
    s.set_sketch(None);
    s.edit(|d| {
        while d.undo_depth() > e.mark {
            d.undo()?;
        }
        Ok(())
    })?;
    s.in_place = None;
    Ok(())
}

/// Starts sketching a form: `kind` Extrusion, VoidExtrusion, Blend or Sweep, or Edit Sketch
/// of form `index`.
#[tauri::command]
pub fn in_place_form_begin(
    view: ElementId,
    kind: String,
    index: Option<usize>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| form_begin(s, view, &kind, index))
}

pub(crate) fn form_begin(
    s: &mut Session,
    view: ElementId,
    kind: &str,
    index: Option<usize>,
) -> anyhow::Result<()> {
    let e = s.in_place.context("Open the In-Place Editor first.")?;
    let doc = s.doc()?;
    let ElementData::InPlace { level, forms, .. } = doc.data(e.id)? else {
        bail!("the in-place element is gone");
    };
    let level = *level;
    let (draft, curves) = match index {
        Some(i) => {
            let f = forms.get(i).context("no such form")?;
            (
                FormDraft {
                    kind: f.kind.clone(),
                    void: f.void,
                    index: Some(i),
                    top: false,
                    base: None,
                },
                f.sketch.concat(),
            )
        }
        None => (
            FormDraft {
                kind: inplace::default_kind(kind)
                    .with_context(|| format!("unknown form {kind}"))?,
                void: kind == "VoidExtrusion",
                index: None,
                top: false,
                base: None,
            },
            vec![],
        ),
    };
    // Forms are sketched in plan: this one, else the level's (from 3D, ADR-025).
    let view = if s.view_level(view).is_ok() {
        view
    } else {
        sketch::plan_for(doc, level, SketchKind::InPlace)
            .context("that level has no floor plan to sketch in")?
    };
    let z0 = match &draft.kind {
        FormKind::Extrusion { start, .. } => *start,
        FormKind::Blend { base, .. } => *base,
        FormKind::Sweep { elevation, .. } => *elevation,
    };
    let elevation = doc.level_elevation(level)? + z0;
    s.set_sketch(Some(SketchSession::for_form(
        view, level, e.id, elevation, curves, draft,
    )));
    Ok(())
}

#[tauri::command]
pub fn in_place_delete_form(
    index: usize,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    run(&window, &state, |s| {
        let e = s.in_place.context("Open the In-Place Editor first.")?;
        s.edit(|d| inplace::delete_form(d, e.id, index))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::sketch::SketchCurve;
    use studio_core::ViewKind;
    use studio_geom::Pt;

    fn square(c: Pt, h: f64) -> Vec<SketchCurve> {
        let p = [
            Pt::new(c.x - h, c.y - h),
            Pt::new(c.x + h, c.y - h),
            Pt::new(c.x + h, c.y + h),
            Pt::new(c.x - h, c.y + h),
        ];
        (0..4)
            .map(|i| SketchCurve::line(p[i], p[(i + 1) % 4]))
            .collect()
    }

    fn setup() -> (Session, ElementId) {
        let mut s = Session::default();
        s.new_project("test").unwrap();
        let doc = s.doc().unwrap();
        let l1 = doc.levels()[0].0;
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        (s, plan)
    }

    fn sketch(s: &mut Session, curves: Vec<SketchCurve>) {
        let (_, sk) = s.doc_and_sketch().unwrap();
        sk.curves = curves;
    }

    #[test]
    fn model_in_place_adds_forms_and_finishes_as_one_undo_step() {
        let (mut s, plan) = setup();
        let depth = s.doc().unwrap().undo_depth();
        begin(&mut s, plan, Category::Column, Some("Tapered Column"), None).unwrap();
        let id = s.in_place.unwrap().id;
        // Finish needs a form.
        assert!(finish_model(&mut s).is_err());
        // A blend: its base, then its top.
        form_begin(&mut s, plan, "Blend", None).unwrap();
        sketch(&mut s, square(Pt::new(0.0, 0.0), 300.0));
        crate::sketching::finish_form_for_test(&mut s).unwrap();
        let sk = s.sketch().unwrap();
        assert!(sk.form.as_ref().unwrap().top && sk.curves.is_empty());
        sketch(&mut s, square(Pt::new(0.0, 0.0), 150.0));
        crate::sketching::finish_form_for_test(&mut s).unwrap();
        assert!(s.sketch().is_none());
        // A sweep along an L, then finish.
        form_begin(&mut s, plan, "Sweep", None).unwrap();
        sketch(
            &mut s,
            vec![
                SketchCurve::line(Pt::new(1000.0, 0.0), Pt::new(3000.0, 0.0)),
                SketchCurve::line(Pt::new(3000.0, 0.0), Pt::new(3000.0, 2000.0)),
            ],
        );
        crate::sketching::finish_form_for_test(&mut s).unwrap();
        let state = s.state().unwrap();
        assert_eq!(
            state.in_place.as_ref().unwrap().forms,
            vec!["Blend 1", "Sweep 1"]
        );
        finish_model(&mut s).unwrap();
        assert!(s.in_place.is_none());
        let doc = s.doc().unwrap();
        assert_eq!(doc.undo_depth(), depth + 1);
        assert_eq!(doc.can_undo(), Some("Model In-Place"));
        assert_eq!(doc.data(id).unwrap().category(), Category::Column);
        assert!(studio_regen::inplace::solid(doc, id).is_some());
    }

    #[test]
    fn cancel_model_leaves_nothing_and_a_bad_sketch_says_why() {
        let (mut s, plan) = setup();
        let before = s.doc().unwrap().len();
        begin(&mut s, plan, Category::GenericModel, None, None).unwrap();
        form_begin(&mut s, plan, "Extrusion", None).unwrap();
        // An open chain isn't a loop: the sketch stays open with Revit's message.
        sketch(
            &mut s,
            vec![SketchCurve::line(Pt::new(0.0, 0.0), Pt::new(1000.0, 0.0))],
        );
        crate::sketching::finish_form_for_test(&mut s).unwrap();
        assert!(s.sketch().unwrap().error.is_some());
        sketch(&mut s, square(Pt::new(0.0, 0.0), 500.0));
        crate::sketching::finish_form_for_test(&mut s).unwrap();
        assert!(s.sketch().is_none());
        cancel_model(&mut s).unwrap();
        assert!(s.in_place.is_none());
        assert_eq!(s.doc().unwrap().len(), before);
    }
}
