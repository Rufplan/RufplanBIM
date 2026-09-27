//! IPC for Edit Model with Claude (ADR-050): Claude turns the prompt into a structured edit,
//! studio-core checks and previews it; nothing changes until Apply.

use serde::Serialize;
use serde_json::{json, Value};
use studio_core::model_edit::{self, EditContext, EditPreview, ModelEdit};
use studio_core::{ElementData, ElementId, ViewKind};
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{edit, lock, CommandError, SessionState, StateResult};
use crate::generate_cmds::CLAUDE_KEY;
use crate::site_cmds::get;

type CommandResult<T> = Result<T, CommandError>;

/// Claude's proposal and what it would change (or why it can't be done).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EditPlan {
    pub edit: ModelEdit,
    pub preview: Option<EditPreview>,
    pub error: Option<String>,
}

fn context(doc: &studio_core::Document, view: ElementId, selection: Vec<ElementId>) -> EditContext {
    let (name, level, drawn) = match doc.data(view) {
        Ok(ElementData::View { name, kind, .. }) => {
            let level = match kind {
                ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level } => Some(*level),
                _ => None,
            };
            // What the view draws, for "in this view" (3D views show everything).
            let drawn = studio_views::display_list_shared(doc, view).map(|dl| {
                let mut v: Vec<ElementId> = dl.items.iter().filter_map(|i| i.el).collect();
                v.sort();
                v.dedup();
                // A door or window is in the view when its wall is.
                for e in doc.iter() {
                    if let ElementData::Door { host, .. } | ElementData::Window { host, .. } =
                        &e.data
                    {
                        if v.binary_search(host).is_ok() {
                            v.push(e.id);
                        }
                    }
                }
                v
            });
            (name.clone(), level, drawn)
        }
        _ => (String::new(), None, None),
    };
    EditContext {
        selection,
        view_level: level,
        view_ids: drawn,
        view_name: name,
    }
}

fn system_prompt() -> String {
    r#"You turn an architect's request into ONE change to a building model in Rufplan Studio (a Revit-like BIM app). Answer only by calling the edit_model tool.

Actions:
- set_parameter: set a property on the elements of one category. Use the category and the property names exactly as listed in the model summary (instance properties, or "type:" properties, which you name without the "type:" prefix). The app moves instances to a matching type for type properties (e.g. door Width), so "make all doors 3'-0"" is category Doors, parameter Width, value 3'-0". Choices take one of the listed values; "Type" takes a type name.
- resize_building: change the building's overall size, outside face to outside face. "50' wide" is axis east-west; "deep"/"long" north-south is axis north-south. anchor is the side that stays put: west (default for width), south (default for depth), east, north, or center when the request says so.
- none: the request isn't a model change you can make with these (say why in message, one short sentence, and suggest what would work).

Scope: model (default), level (with level = its name, e.g. "Level 1 doors"), selection ("these", "selected"), or view ("in this view"). typeFilter narrows to types whose name contains it ("the single doors" → "Single").
Values: US feet-inches like 3'-0", 7'-6 1/2", 50'-0". Only one change: if asked for several, do the first and say in summary that the rest need their own prompts.
summary: one line like: Width → 3'-0" on 7 doors."#
        .into()
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": { "type": "string", "enum": ["set_parameter", "resize_building", "none"] },
            "category": { "type": "string" },
            "parameter": { "type": "string" },
            "value": { "type": "string" },
            "scope": { "type": "string", "enum": ["model", "level", "selection", "view"] },
            "level": { "type": "string" },
            "typeFilter": { "type": "string" },
            "axis": { "type": "string", "enum": ["east-west", "north-south"] },
            "anchor": { "type": "string", "enum": ["west", "east", "south", "north", "center"] },
            "summary": { "type": "string" },
            "message": { "type": "string" }
        },
        "required": ["action", "summary"]
    })
}

/// Asks Claude for the edit and previews it. Nothing in the model changes.
#[tauri::command]
pub async fn model_edit_preview(
    prompt: String,
    view: ElementId,
    selection: Vec<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<EditPlan> {
    if prompt.trim().is_empty() {
        return Err(anyhow::anyhow!("describe a change to the model").into());
    }
    let key = get(CLAUDE_KEY).ok_or_else(|| {
        anyhow::anyhow!("add your Claude API key first (Architecture > Generate > Claude API key)")
    })?;
    let summary = {
        let s = lock(&state)?;
        let doc = s.doc()?;
        model_edit::describe(doc, &context(doc, view, selection.clone()))
    };
    let edit = tauri::async_runtime::spawn_blocking(move || ask(&key, &summary, &prompt))
        .await
        .map_err(anyhow::Error::from)??;
    let s = lock(&state)?;
    let doc = s.doc()?;
    Ok(plan(doc, edit, context(doc, view, selection)))
}

/// Asks Claude for one structured edit of the model described by `summary`.
fn ask(key: &str, summary: &str, prompt: &str) -> anyhow::Result<ModelEdit> {
    let request = studio_sync::claude::Request {
        model: "claude-opus-5-5".into(),
        system: system_prompt(),
        text: format!("The model:\n{summary}\n\nRequest: {}", prompt.trim()),
        images: vec![],
        tool_name: "edit_model".into(),
        tool_description: "Make one change to the building model.".into(),
        tool_schema: schema(),
        max_tokens: 2000,
    };
    let answer = studio_sync::claude::call(key, &request, &mut |_| {})?;
    serde_json::from_value(answer)
        .map_err(|e| anyhow::anyhow!("Claude's answer doesn't fit the model: {e}"))
}

fn plan(doc: &studio_core::Document, edit: ModelEdit, ctx: EditContext) -> EditPlan {
    match model_edit::preview(doc, &edit, &ctx) {
        Ok(p) => EditPlan {
            edit,
            preview: Some(p),
            error: None,
        },
        Err(e) => EditPlan {
            edit,
            preview: None,
            error: Some(e.to_string()),
        },
    }
}

/// Applies a previewed edit as one undo step.
#[tauri::command]
pub fn model_edit_apply(
    edit_plan: ModelEdit,
    view: ElementId,
    selection: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| {
            let ctx = context(d, view, selection);
            model_edit::apply(d, &edit_plan, &ctx).map(|_| ())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_house_previews_an_edit_without_changing() {
        let mut s = crate::session::Session::default();
        s.new_sample("0.0.1").unwrap();
        let doc = s.doc().unwrap();
        let plan_view = doc
            .of(studio_core::Category::View)
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
        let before = doc.stamp();
        let edit = ModelEdit {
            action: "set_parameter".into(),
            category: "Doors".into(),
            parameter: "Width".into(),
            value: "3'-0\"".into(),
            scope: "view".into(),
            ..Default::default()
        };
        let p = plan(doc, edit, context(doc, plan_view, vec![]));
        let preview = p.preview.expect("a preview");
        assert!(preview.count >= 1 && preview.to == "3'-0\"", "{preview:?}");
        assert!(preview.scope.starts_with("In "));
        assert_eq!(doc.stamp(), before, "previewing changes nothing");
        let d = model_edit::describe(doc, &context(doc, plan_view, vec![]));
        assert!(d.contains("Doors:") && d.contains("Active view: "), "{d}");
    }

    /// Live check (uses API credit):
    /// `cargo test -p rufplan-studio live_model_edit -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_model_edit() {
        let key = get(CLAUDE_KEY).expect("a Claude key in the credential store");
        let mut s = crate::session::Session::default();
        s.new_sample("0.0.1").unwrap();
        let doc = s.doc().unwrap();
        let view = doc
            .of(studio_core::Category::View)
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
        let ctx = context(doc, view, vec![]);
        let summary = model_edit::describe(doc, &ctx);
        for prompt in [
            "change all doors to be 3'-0\"",
            "update the building to be 50'-0\" wide",
            "set the Level 1 windows sill height to 2'-6\"",
            "paint the house blue",
        ] {
            let e = ask(&key, &summary, prompt).unwrap();
            let p = plan(doc, e.clone(), ctx.clone());
            println!(
                "{prompt}\n  -> {e:?}\n  -> {:?} {:?}",
                p.preview.map(|p| p.summary),
                p.error
            );
        }
    }
}
