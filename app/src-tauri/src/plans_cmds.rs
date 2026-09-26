//! Plans to 3D (ADR-036): the owner gives plan drawings (images, or PDF pages the page turns
//! into images); Claude reads each sheet's walls, openings, rooms and slabs in pixels with
//! its scale and a shared anchor; studio-core builds the model. Same key and streaming as
//! Generate (ADR-030).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use studio_core::plans::{self, PlanReport, PlanSet};
use tauri::{Emitter, State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;
use crate::site_cmds::get;

type CommandResult<T> = Result<T, CommandError>;

/// One plan sheet as sent: the image and what the owner says it is.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanImage {
    pub media_type: String,
    /// Base64.
    pub data: String,
    /// Pixel size of the image as sent.
    pub width: u32,
    pub height: u32,
    /// The owner's label ("First Floor"); may be empty.
    pub level: String,
    /// Floor elevation in feet if the owner knows it.
    pub elevation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlansInputs {
    pub name: String,
    pub sheets: Vec<PlanImage>,
    /// Scale, heights, anything that helps read the drawings.
    pub notes: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlansProgress {
    pub phase: String,
    pub chars: usize,
    pub sheets: usize,
    pub walls: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PlansResult {
    pub state: Option<AppState>,
    pub report: PlanReport,
    pub summary: String,
}

pub fn system_prompt() -> String {
    "You are an architect tracing floor plans into a BIM model. You are given plan drawings as images, one floor per image (sometimes two floors on one sheet: report each as its own sheet with the same image). Read them carefully and call read_plans exactly once, with no text before or after it; never reply in prose. Make sensible assumptions and note them in the summary.

COORDINATES
- Everything you trace is in the image's own pixels: x to the right, y DOWN, origin at the image's top-left. The pixel size of each image is given; use that coordinate range.
- pixelsPerFoot: the drawing's scale in this image. Measure it from a dimension string (a long one: find its two extension lines in pixels and divide by the dimension in feet) or from a graphic scale bar. Check it against a second dimension. Different images may have different scales.
- anchorPx / anchorFt: pick one feature that exists on every floor and stacks vertically (a building corner, a stair or a column, a chimney mass). Give its pixel position on each image and the same anchorFt on every sheet (for example [0, 0]), so the floors line up. If the floors don't share an obvious feature, use the corner of the main mass that you believe stacks.

WHAT TO TRACE, PER SHEET
- walls: every wall as its centerline from end a to end b, in pixels, straight segments (break a wall where it turns or changes thickness; approximate curves with short segments). thickness in inches (from the drawing: poché width / pixelsPerFoot × 12, rounded to 1/2\"). exterior: true for walls on the outside of the building. Trace walls through door and window openings as one continuous wall — the openings are cut later. Ignore furniture, fixtures, hatching, stone paving, site lines, rivers and grades, and dimension lines.
- doors: the center of each door opening in pixels (on the wall's centerline), width in inches (from its dimension or the swing radius), height if noted (else 80), and kind: swing, double, french (pairs of glass doors), sliding, pocket, bifold, garage or glass.
- windows: the center of each window in pixels, width in inches, height and sill in inches if you can tell (else 48 and 36), and kind: fixed, casement, double_hung, slider, awning, or storefront for floor-to-ceiling glazing.
- rooms: each room's name (as labelled) and a pixel point well inside it.
- slabs: the floor slab outline of this floor as polygons in pixels — include terraces, balconies and cantilevers as their own polygons (they are part of the floor structure).
- elevation: the floor's elevation in feet relative to the lowest floor (0), from level notes, sections or the owner's labels; wallHeight: the typical wall height on that floor, feet (floor to the floor above, or 9-10 ft if unknown).
- level: the floor's name, as labelled on the sheet or by the owner.

Also give the building a name, a two-sentence summary of what you read (and what you assumed), and roof: flat, hip or gable.".into()
}

pub fn user_prompt(i: &PlansInputs) -> String {
    let mut s = format!(
        "Trace these {} plan sheet{} into a model named \"{}\".\n",
        i.sheets.len(),
        if i.sheets.len() == 1 { "" } else { "s" },
        if i.name.trim().is_empty() {
            "Traced Building"
        } else {
            i.name.trim()
        }
    );
    for (k, sh) in i.sheets.iter().enumerate() {
        s.push_str(&format!(
            "\nImage {}: {} x {} px.",
            k + 1,
            sh.width,
            sh.height
        ));
        if !sh.level.trim().is_empty() {
            s.push_str(&format!(" The owner says this is {}.", sh.level.trim()));
        }
        if let Some(e) = sh.elevation {
            s.push_str(&format!(" Its floor is at {e} ft."));
        }
    }
    if !i.notes.trim().is_empty() {
        s.push_str(&format!("\n\nNotes from the owner: {}", i.notes.trim()));
    }
    s
}

pub fn plans_schema() -> Value {
    let pt =
        json!({ "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 });
    json!({
        "type": "object",
        "required": ["name", "summary", "sheets", "roof"],
        "properties": {
            "name": { "type": "string" },
            "summary": { "type": "string" },
            "roof": { "type": "string", "enum": ["flat", "hip", "gable"] },
            "sheets": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["image", "level", "elevation", "wallHeight", "pixelsPerFoot", "anchorPx", "anchorFt", "walls"],
                    "properties": {
                        "image": { "type": "integer", "description": "Which image, from 1." },
                        "level": { "type": "string" },
                        "elevation": { "type": "number", "description": "Feet above the lowest floor." },
                        "wallHeight": { "type": "number", "description": "Feet." },
                        "pixelsPerFoot": { "type": "number" },
                        "anchorPx": pt,
                        "anchorFt": pt,
                        "walls": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["a", "b", "thickness"],
                                "properties": {
                                    "a": pt, "b": pt,
                                    "thickness": { "type": "number", "description": "Inches." },
                                    "exterior": { "type": "boolean" }
                                }
                            }
                        },
                        "doors": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["at", "width"],
                                "properties": {
                                    "at": pt,
                                    "width": { "type": "number" },
                                    "height": { "type": "number" },
                                    "kind": { "type": "string", "enum": ["swing", "double", "french", "sliding", "pocket", "bifold", "garage", "glass"] }
                                }
                            }
                        },
                        "windows": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["at", "width"],
                                "properties": {
                                    "at": pt,
                                    "width": { "type": "number" },
                                    "height": { "type": "number" },
                                    "sill": { "type": "number" },
                                    "kind": { "type": "string", "enum": ["fixed", "casement", "double_hung", "slider", "awning", "storefront"] }
                                }
                            }
                        },
                        "rooms": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["name", "at"],
                                "properties": { "name": { "type": "string" }, "at": pt }
                            }
                        },
                        "slabs": { "type": "array", "items": { "type": "array", "items": pt } }
                    }
                }
            }
        }
    })
}

pub fn progress_of(partial: &str) -> PlansProgress {
    PlansProgress {
        phase: "reading".into(),
        chars: partial.len(),
        sheets: partial.matches("\"pixelsPerFoot\"").count(),
        walls: partial.matches("\"thickness\"").count(),
    }
}

/// Reads the plans with Claude, then builds the model (replacing the current building; one
/// undo step).
#[tauri::command]
pub async fn plans_to_model(
    inputs: PlansInputs,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<PlansResult> {
    let key = get(crate::generate_cmds::CLAUDE_KEY).ok_or_else(|| {
        anyhow::anyhow!("add your Claude API key first (Generate > Claude API key)")
    })?;
    if inputs.sheets.is_empty() {
        return Err(anyhow::anyhow!("add at least one plan").into());
    }
    if inputs.sheets.len() > 12 {
        return Err(anyhow::anyhow!("send up to 12 plan sheets at a time").into());
    }
    let model = match inputs.model.as_str() {
        "claude-sonnet-5" => "claude-sonnet-5",
        _ => "claude-opus-5-5",
    };
    let request = studio_sync::claude::Request {
        model: model.into(),
        system: system_prompt(),
        text: user_prompt(&inputs),
        images: inputs
            .sheets
            .iter()
            .map(|i| studio_sync::claude::ImageInput {
                media_type: i.media_type.clone(),
                data: i.data.clone(),
            })
            .collect(),
        tool_name: "read_plans".into(),
        tool_description: "Build the model in Rufplan Studio from the traced plans.".into(),
        tool_schema: plans_schema(),
        max_tokens: 64_000,
    };
    let win = window.clone();
    let plan = tauri::async_runtime::spawn_blocking(move || {
        let mut last = 0usize;
        studio_sync::claude::call(&key, &request, &mut |partial| {
            if partial.len() >= last + 400 {
                last = partial.len();
                let _ = win.emit("plans-progress", progress_of(partial));
            }
        })
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(anyhow::Error::from)?;
    let set: PlanSet = serde_json::from_value(plan)
        .map_err(|e| anyhow::anyhow!("Claude's reading doesn't fit the model: {e}"))?;
    let _ = window.emit(
        "plans-progress",
        PlansProgress {
            phase: "building".into(),
            chars: 0,
            sheets: set.sheets.len(),
            walls: set.sheets.iter().map(|s| s.walls.len()).sum(),
        },
    );
    let mut s = lock(&state)?;
    let report = s.edit(|d| plans::build(d, &set))?;
    let state = finish(&window, &s)?;
    Ok(PlansResult {
        state,
        report,
        summary: set.summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live: `PLANS_IN=inputs.json PLANS_OUT=reading.json cargo test -p rufplan-studio
    /// live_plans -- --ignored --nocapture` reads the plans with Claude (uses API credit)
    /// and saves the reading.
    #[test]
    #[ignore]
    fn live_plans() {
        let key = get(crate::generate_cmds::CLAUDE_KEY).expect("no Claude key saved");
        let text = std::fs::read_to_string(std::env::var("PLANS_IN").unwrap()).unwrap();
        let inputs: PlansInputs =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
        let request = studio_sync::claude::Request {
            model: inputs.model.clone(),
            system: system_prompt(),
            text: user_prompt(&inputs),
            images: inputs
                .sheets
                .iter()
                .map(|i| studio_sync::claude::ImageInput {
                    media_type: i.media_type.clone(),
                    data: i.data.clone(),
                })
                .collect(),
            tool_name: "read_plans".into(),
            tool_description: "Build the model in Rufplan Studio from the traced plans.".into(),
            tool_schema: plans_schema(),
            max_tokens: 64_000,
        };
        let t0 = std::time::Instant::now();
        let plan = studio_sync::claude::call(&key, &request, &mut |_| {}).unwrap();
        println!("read in {:?}", t0.elapsed());
        std::fs::write(
            std::env::var("PLANS_OUT").unwrap(),
            serde_json::to_string_pretty(&plan).unwrap(),
        )
        .unwrap();
        let set: PlanSet = serde_json::from_value(plan).unwrap();
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let r = plans::build(&mut doc, &set).unwrap();
        println!("{r:?}\n{}", set.summary);
    }

    #[test]
    fn a_reading_parses_as_a_plan_set() {
        let reading = json!({
            "name": "Box", "summary": "A box.", "roof": "flat",
            "sheets": [{
                "image": 1, "level": "First Floor", "elevation": 0, "wallHeight": 9,
                "pixelsPerFoot": 10, "anchorPx": [100, 400], "anchorFt": [0, 0],
                "walls": [{ "a": [100, 400], "b": [500, 400], "thickness": 8, "exterior": true }],
                "doors": [{ "at": [200, 400], "width": 36, "kind": "swing" }],
                "rooms": [{ "name": "Living", "at": [200, 300] }]
            }]
        });
        let set: PlanSet = serde_json::from_value(reading).unwrap();
        assert_eq!(set.sheets[0].walls[0].thickness, 8.0);
        assert_eq!(set.sheets[0].doors[0].kind, "swing");
        let i = PlansInputs {
            name: "Fallingwater".into(),
            sheets: vec![PlanImage {
                media_type: "image/jpeg".into(),
                data: String::new(),
                width: 1568,
                height: 1209,
                level: "First Floor".into(),
                elevation: Some(0.0),
            }],
            notes: "1/4\" = 1'-0\"".into(),
            model: "claude-opus-5-5".into(),
        };
        let p = user_prompt(&i);
        assert!(
            p.contains("Image 1: 1568 x 1209 px.")
                && p.contains("First Floor")
                && p.contains("1/4")
        );
        assert!(system_prompt().contains("read_plans"));
        let schema = plans_schema();
        assert!(schema
            .pointer("/properties/sheets/items/properties/walls")
            .is_some());
        let pr = progress_of(
            "{\"sheets\":[{\"pixelsPerFoot\":10,\"walls\":[{\"thickness\":8},{\"thickness\":4}]",
        );
        assert_eq!((pr.sheets, pr.walls), (1, 2));
    }
}
