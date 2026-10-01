//! IPC for Edit Model with Claude (ADR-050, ADR-051): Claude turns the prompt into a plan of
//! operations (create, change, delete, move, annotate…), studio-regen runs it on a copy to
//! preview it; nothing changes until Apply.

use serde::Serialize;
use serde_json::{json, Value};
use studio_core::model_edit::{self, EditContext, EditPreview};
use studio_core::{ElementData, ElementId, ViewKind};
use studio_regen::model_ops::{self, ModelPlan};
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
    pub edit: ModelPlan,
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
                // A building elevation draws the far facades too, behind the near ones, and
                // the side walls edge-on: "on this elevation" means the facade you see
                // face-on in front, and its openings.
                if let ViewKind::Elevation { facing } = kind {
                    let hidden = behind(doc, facing.look().scale(-1.0));
                    v.retain(|id| !hidden.contains(id));
                }
                v
            });
            // Claude reads "this view" better knowing what kind of view it is.
            let kind_name = match kind {
                ViewKind::FloorPlan { .. } => "floor plan",
                ViewKind::CeilingPlan { .. } => "ceiling plan",
                ViewKind::Elevation { .. } | ViewKind::MarkerElevation { .. } => "elevation",
                ViewKind::Section { .. } => "section",
                ViewKind::ThreeD => "3D view",
                ViewKind::Schedule { .. } => "schedule",
                ViewKind::Drafting => "drafting view",
                ViewKind::Rendering { .. } => "rendering",
            };
            (format!("{name} ({kind_name})"), level, drawn)
        }
        _ => (String::new(), None, None),
    };
    EditContext {
        selection,
        view_level: level,
        view_ids: drawn,
        view_name: name,
        view: Some(view),
    }
}

/// In an elevation looking along `look`, the walls not on its facade — seen edge-on, or
/// face-on behind a nearer face-on wall — and the doors and windows in them.
fn behind(doc: &studio_core::Document, look: studio_geom::Pt) -> Vec<ElementId> {
    let right = studio_geom::Pt::new(look.y, -look.x);
    // Face-on walls as (id, u-range, depth).
    let walls: Vec<(ElementId, f64, f64, f64)> = doc
        .of(studio_core::Category::Wall)
        .filter_map(|e| match &e.data {
            ElementData::Wall { start, end, .. } => {
                let d = end.sub(*start).norm();
                (d.dot(look).abs() < 0.3).then(|| {
                    let (u0, u1) = (start.dot(right), end.dot(right));
                    (
                        e.id,
                        u0.min(u1),
                        u0.max(u1),
                        start.lerp(*end, 0.5).dot(look),
                    )
                })
            }
            _ => None,
        })
        .collect();
    let mut out: Vec<ElementId> = walls
        .iter()
        .filter(|(_, a0, a1, depth)| {
            walls.iter().any(|(_, b0, b1, d2)| {
                let overlap = a1.min(*b1) - a0.max(*b0);
                *d2 < depth - 1.0 && overlap > 0.5 * (a1 - a0)
            })
        })
        .map(|w| w.0)
        .collect();
    out.extend(
        doc.of(studio_core::Category::Wall)
            .map(|e| e.id)
            .filter(|id| !walls.iter().any(|w| w.0 == *id)),
    );
    for e in doc.iter() {
        if let ElementData::Door { host, .. } | ElementData::Window { host, .. } = &e.data {
            if out.contains(host) {
                out.push(e.id);
            }
        }
    }
    out
}

fn system_prompt() -> String {
    format!(
        r#"You change a building model in Rufplan Studio (a Revit-like BIM app) as an architect asks: anything they ask — create, edit, move, copy, delete, annotate, add levels, views and sheets. Answer only by calling the edit_model tool with a plan: a list of operations run in order as one undoable change.

{ops}

How to plan:
- Use the ids, types, levels, views and coordinates in the model summary. Name new elements with "as" and refer to them later as "$name" (e.g. make a wall "as": "w1", then a door with "wall": "$w1").
- Walls are centerlines; a room is four walls meeting at shared corner points. Put doors and windows in walls by "offset" from the wall's start (its first point). Keep new work tied to the building: align to existing walls and grids, and join new walls to existing ones at their ends or along them.
- Changing something existing: set_parameter for "all/every X", set_property for particular elements (use the property names listed), move/copy/rotate/mirror/delete by id, resize_building for the building's overall size.
- Annotations go in the active view unless another is named. In plans, points are plan x, y; in elevations and sections give x, y of the spot on the building plus its height z.
- Lengths are US feet-inches strings like "3'-0\"" or "7'-6 1/2\""; coordinates, offsets and moves are numbers in feet.
- If the request truly can't be done with these operations, return no operations and say why in "message" (one or two sentences, with what would work instead).
- summary: one short line describing the whole change, e.g. "Adds a 12' x 10' office with a door and window"."#,
        ops = model_ops::OPERATIONS
    )
}

fn schema() -> Value {
    let pt = json!({
        "type": "object",
        "properties": { "x": { "type": "number" }, "y": { "type": "number" }, "z": { "type": "number" } },
        "required": ["x", "y"]
    });
    let mut op = serde_json::Map::new();
    for k in [
        "op",
        "as",
        "id",
        "category",
        "parameter",
        "value",
        "scope",
        "typeFilter",
        "axis",
        "anchor",
        "level",
        "topLevel",
        "type",
        "from",
        "wall",
        "height",
        "width",
        "elevation",
        "slope",
        "name",
        "number",
        "text",
        "view",
        "sheet",
        "material",
        "color",
    ] {
        op.insert(k.into(), json!({ "type": "string" }));
    }
    for k in ["offset", "dx", "dy", "angle", "count"] {
        op.insert(k.into(), json!({ "type": "number" }));
    }
    for k in ["start", "end", "at", "center", "a", "b"] {
        op.insert(k.into(), pt.clone());
    }
    op.insert("flip".into(), json!({ "type": "boolean" }));
    op.insert(
        "ids".into(),
        json!({ "type": "array", "items": { "type": "string" } }),
    );
    op.insert("points".into(), json!({ "type": "array", "items": pt }));
    op.insert(
        "properties".into(),
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": { "name": { "type": "string" }, "value": { "type": "string" } },
                "required": ["name", "value"]
            }
        }),
    );
    json!({
        "type": "object",
        "properties": {
            "operations": {
                "type": "array",
                "items": { "type": "object", "properties": op, "required": ["op"] }
            },
            "summary": { "type": "string" },
            "message": { "type": "string" }
        },
        "required": ["operations", "summary"]
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

/// Asks Claude for a plan for the model described by `summary`.
fn ask(key: &str, summary: &str, prompt: &str) -> anyhow::Result<ModelPlan> {
    let request = studio_sync::claude::Request {
        model: "claude-opus-5-5".into(),
        system: system_prompt(),
        text: format!("The model:\n{summary}\n\nRequest: {}", prompt.trim()),
        images: vec![],
        tool_name: "edit_model".into(),
        tool_description: "Change the building model: a list of operations run in order.".into(),
        tool_schema: schema(),
        max_tokens: 16_000,
    };
    let answer = studio_sync::claude::call(key, &request, &mut |_| {})?;
    serde_json::from_value(answer)
        .map_err(|e| anyhow::anyhow!("Claude's answer doesn't fit the model: {e}"))
}

fn plan(doc: &studio_core::Document, edit: ModelPlan, ctx: EditContext) -> EditPlan {
    match model_ops::preview(doc, &edit, &ctx) {
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
    edit_plan: ModelPlan,
    view: ElementId,
    selection: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| {
            let ctx = context(d, view, selection);
            model_ops::apply(d, &edit_plan, &ctx).map(|_| ())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plan of one bulk change.
    fn bulk(category: &str, parameter: &str, value: &str, scope: &str) -> ModelPlan {
        serde_json::from_value(json!({ "operations": [{
            "op": "set_parameter", "category": category, "parameter": parameter,
            "value": value, "scope": scope
        }]}))
        .unwrap()
    }

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
        let p = plan(
            doc,
            bulk("Doors", "Width", "3'-0\"", "view"),
            context(doc, plan_view, vec![]),
        );
        let preview = p.preview.expect("a preview");
        assert!(preview.count >= 1 && preview.to == "3'-0\"", "{preview:?}");
        assert!(preview.scope.starts_with("In "));
        assert_eq!(doc.stamp(), before, "previewing changes nothing");
        let d = model_edit::describe(doc, &context(doc, plan_view, vec![]));
        assert!(d.contains("Doors:") && d.contains("(floor plan)"), "{d}");
        // In an elevation, "view" scope is what the elevation shows (ADR-050).
        let elev = doc
            .of(studio_core::Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Elevation { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        let ctx = context(doc, elev, vec![]);
        assert!(ctx.view_name.ends_with("(elevation)") && ctx.view_level.is_none());
        let seen = plan(doc, bulk("Windows", "Sill Height", "2'-6\"", "view"), ctx)
            .preview
            .expect("windows in the elevation");
        let all = plan(
            doc,
            bulk("Windows", "Sill Height", "2'-6\"", "model"),
            context(doc, elev, vec![]),
        )
        .preview
        .unwrap();
        assert!(
            seen.count >= 1 && seen.count < all.count,
            "{} of {}",
            seen.count,
            all.count
        );
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
            "add a 12' x 10' office off the east side of the house with a door into it and a window on its east wall",
            "put a note that says VERIFY IN FIELD by the front door",
            "add a level called Roof Deck at 26'-0\"",
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
        // From an elevation: "on this elevation" is the facade it shows.
        let elev = doc
            .of(studio_core::Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: ViewKind::Elevation { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        let ctx = context(doc, elev, vec![]);
        let prompt = "make the windows on this elevation 2'-6\" sill height";
        let e = ask(&key, &model_edit::describe(doc, &ctx), prompt).unwrap();
        let p = plan(doc, e.clone(), ctx);
        println!(
            "{prompt}\n  -> {e:?}\n  -> {:?} {:?}",
            p.preview.map(|p| p.summary),
            p.error
        );
    }
}
