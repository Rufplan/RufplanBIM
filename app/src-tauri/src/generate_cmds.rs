//! Generate with Claude (ADR-030): the owner describes a building; Claude plans it as rooms
//! per story (a forced tool call, streamed so the page can show progress); studio-core
//! builds the model from the plan. The Claude API key lives in the OS credential store and
//! only goes to api.anthropic.com.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use studio_core::generate::{self, BuildReport, BuildingSpec};
use studio_core::{library, ElementData};
use tauri::{Emitter, State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;
use crate::site_cmds::{get, put};

type CommandResult<T> = Result<T, CommandError>;

const CLAUDE_KEY: &str = "anthropic-api-key";

#[tauri::command]
pub fn claude_key_set() -> bool {
    get(CLAUDE_KEY).is_some()
}

/// Saves (or, empty, removes) the Claude API key.
#[tauri::command]
pub fn claude_set_key(key: String) -> CommandResult<bool> {
    put(CLAUDE_KEY, &key)?;
    Ok(claude_key_set())
}

/// An image the owner attached as a reference.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReferenceImage {
    pub media_type: String,
    /// Base64.
    pub data: String,
}

/// What the owner asked for.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GenerateInputs {
    /// "Single-family house", "Garden-style apartments", "Boutique hotel"…
    pub building_type: String,
    pub stories: u32,
    /// Gross building area, square feet (optional).
    pub area: Option<f64>,
    /// Bedrooms (houses), units (multifamily) or keys (hotels) — see `building_type`.
    pub count: Option<u32>,
    pub bathrooms: Option<f64>,
    pub style: String,
    /// "auto", "hip", "gable" or "flat".
    pub roof: String,
    pub fit_lot: bool,
    /// Front, side and rear setbacks, feet.
    pub setbacks: [f64; 3],
    pub references: String,
    pub prompt: String,
    pub images: Vec<ReferenceImage>,
    /// "claude-opus-5-5" or "claude-sonnet-5".
    pub model: String,
}

/// The architect's brief to Claude.
pub fn system_prompt() -> String {
    let mut materials = String::new();
    for p in library::library() {
        materials.push_str(&format!(
            "- {} — {} ({}, {:?})\n",
            p.id, p.name, p.category, p.tier
        ));
    }
    format!(
        "You are an experienced US architect producing a schematic design that Rufplan Studio will build as a BIM model. Always respond by calling the build_model tool exactly once, with no text before or after it; never reply in prose or ask questions. If the brief is unclear, make sensible assumptions and note them in the summary.

HOW THE MODEL IS BUILT FROM YOUR PLAN
- Each story is a set of axis-aligned rectangular rooms in feet: x runs east, y runs north, origin at the building's south-west corner. y = 0 is the front (street) unless the brief says otherwise.
- Rooms on a story must tile the story's footprint: no overlaps, no gaps, adjacent rooms share edges exactly. Round to 0.5 ft.
- Walls are generated on room edges: exterior walls around the outside, interior walls between rooms. Living, dining, kitchen and entry are open to each other (no wall between any two of them), so an open plan is those rooms side by side.
- Doors are placed automatically so every room is reachable from the entry (ground floor) or a stair (upper floors), preferring corridors, entries, lobbies and living rooms. A room needs a shared edge of at least 4 ft with the room it's entered from. Bathrooms, closets, laundry and storage are only entered, never passed through.
- Windows are placed automatically on exterior walls of bedrooms, living, dining, kitchens, offices, units, guest rooms, lobbies, retail and amenity spaces. Put those rooms on outside walls.
- Stairs: a room of kind stair on every story it serves, at the same x, y, width and depth on each (the top story's stair room is the landing). A straight stair needs about 4 x 18 ft for a 10 ft floor-to-floor; a U stair about 8 x 11 ft. Multifamily and hotels need at least two stairs, near opposite ends of the corridor.
- Upper stories stay within the story below (no cantilevers unless asked). Stack bathrooms and kitchens over each other where you can.

TYPICAL US SIZES
- Houses: bedroom 11 x 12 min, primary 13 x 15+, bath 5 x 8 min, primary bath 8 x 11+, walk-in closet 6 x 8, kitchen 12 x 14, living 14 x 18+, dining 11 x 13, hall 3.5-4 ft wide, two-car garage 22 x 22.
- Multifamily: double-loaded corridor 5-6 ft wide; unit depth 26-32 ft; studio 450-550 sf, one-bedroom 650-800 sf, two-bedroom 950-1,150 sf. For more than about a dozen units, model each unit as one room of kind unit (named like \"Unit 201 - 1BR\"); for smaller buildings you may divide units into rooms.
- Hotels: guest room (key) 12-13 ft wide x 28-30 ft deep as one room of kind guest_room; corridor 6 ft; lobby, front desk and amenity on the ground floor.
- Floor to floor: houses 10 ft (9-10 upper); multifamily 10 ft; hotel upper floors 10 ft; ground-floor lobby or retail 14-18 ft.
- Keep each story under about 60 rooms.

MATERIALS
Pick library materials by id for exterior walls, interior walls, floors and roof, to suit the style and finish level:
{materials}
Also choose roof (hip, gable or flat), pitch (rise per 12) and structure (wood, or masonry for concrete and CMU buildings).

Give the building a name and a two- to four-sentence summary for the owner (gross area, unit or bedroom count, key design moves)."
    )
}

/// The owner's request as a message.
pub fn user_prompt(i: &GenerateInputs, lot: Option<(f64, f64)>) -> String {
    let mut s = format!(
        "Design a {} with {} {}.",
        i.building_type.to_lowercase(),
        i.stories,
        if i.stories == 1 { "story" } else { "stories" }
    );
    if let Some(a) = i.area {
        s.push_str(&format!(" About {a:.0} sf gross."));
    }
    if let Some(n) = i.count {
        let t = i.building_type.to_lowercase();
        let what = if t.contains("hotel") {
            "guest rooms (keys)"
        } else if t.contains("apartment")
            || t.contains("multifamily")
            || t.contains("mixed")
            || t.contains("townhouse")
            || t.contains("duplex")
        {
            "units"
        } else {
            "bedrooms"
        };
        s.push_str(&format!(" {n} {what}."));
    }
    if let Some(b) = i.bathrooms {
        s.push_str(&format!(" {b} bathrooms."));
    }
    if !i.style.trim().is_empty() {
        s.push_str(&format!(" Style: {}.", i.style.trim()));
    }
    match i.roof.as_str() {
        "hip" | "gable" | "flat" => s.push_str(&format!(" Roof: {}.", i.roof)),
        _ => {}
    }
    if let (true, Some((w, d))) = (i.fit_lot, lot) {
        let [front, side, rear] = i.setbacks;
        s.push_str(&format!(
            " The lot is {w:.0} ft wide (east-west) by {d:.0} ft deep (north-south), street on the south. Setbacks: front {front:.0} ft, sides {side:.0} ft, rear {rear:.0} ft, so the building must fit within {:.0} x {:.0} ft.",
            (w - 2.0 * side).max(10.0),
            (d - front - rear).max(10.0)
        ));
    }
    if !i.references.trim().is_empty() {
        s.push_str(&format!("\n\nReferences: {}", i.references.trim()));
    }
    if !i.images.is_empty() {
        s.push_str(&format!(
            "\n\n{} reference image{} attached: take cues from their massing, style and plan.",
            i.images.len(),
            if i.images.len() == 1 { " is" } else { "s are" }
        ));
    }
    if !i.prompt.trim().is_empty() {
        s.push_str(&format!("\n\n{}", i.prompt.trim()));
    }
    s
}

/// The build_model tool's input schema: a [`BuildingSpec`].
pub fn spec_schema() -> Value {
    let kinds = [
        "living",
        "dining",
        "kitchen",
        "bedroom",
        "bathroom",
        "closet",
        "laundry",
        "garage",
        "corridor",
        "stair",
        "entry",
        "lobby",
        "office",
        "unit",
        "guest_room",
        "retail",
        "amenity",
        "mechanical",
        "storage",
        "other",
    ];
    let ids: Vec<String> = library::library().into_iter().map(|p| p.id).collect();
    json!({
        "type": "object",
        "required": ["name", "summary", "stories", "roof", "pitch", "structure", "materials"],
        "properties": {
            "name": { "type": "string" },
            "summary": { "type": "string" },
            "stories": {
                "type": "array",
                "description": "Ground floor first.",
                "items": {
                    "type": "object",
                    "required": ["height", "rooms"],
                    "properties": {
                        "height": { "type": "number", "description": "Floor to floor, feet." },
                        "rooms": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["name", "kind", "x", "y", "width", "depth"],
                                "properties": {
                                    "name": { "type": "string" },
                                    "kind": { "type": "string", "enum": kinds },
                                    "x": { "type": "number", "description": "West edge, feet." },
                                    "y": { "type": "number", "description": "South edge, feet." },
                                    "width": { "type": "number", "description": "East-west, feet." },
                                    "depth": { "type": "number", "description": "North-south, feet." }
                                }
                            }
                        }
                    }
                }
            },
            "roof": { "type": "string", "enum": ["hip", "gable", "flat"] },
            "pitch": { "type": "number", "description": "Rise per 12." },
            "structure": { "type": "string", "enum": ["wood", "masonry"] },
            "materials": {
                "type": "object",
                "properties": {
                    "exteriorWalls": { "type": "string", "enum": ids },
                    "interiorWalls": { "type": "string", "enum": ids },
                    "floors": { "type": "string", "enum": ids },
                    "roof": { "type": "string", "enum": ids }
                }
            }
        }
    })
}

/// How far along: planning (with what's arrived) or building.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GenerateProgress {
    pub phase: String,
    pub chars: usize,
    pub stories: usize,
    pub rooms: usize,
    /// The building's name, once Claude has written it.
    pub name: Option<String>,
}

/// Progress from the plan received so far.
pub fn progress_of(partial: &str) -> GenerateProgress {
    let name = partial.split("\"name\"").nth(1).and_then(|rest| {
        let start = rest.find('"')? + 1;
        let end = rest[start..].find('"')? + start;
        Some(rest[start..end].to_owned())
    });
    GenerateProgress {
        phase: "planning".into(),
        chars: partial.len(),
        stories: partial.matches("\"rooms\"").count(),
        rooms: partial.matches("\"kind\"").count(),
        name,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GenerateResult {
    pub state: Option<AppState>,
    pub report: BuildReport,
    pub summary: String,
}

/// Plans the building with Claude, then builds it (replacing the current building; one
/// undo step).
#[tauri::command]
pub async fn generate_building(
    inputs: GenerateInputs,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<GenerateResult> {
    let key = get(CLAUDE_KEY).ok_or_else(|| {
        anyhow::anyhow!("add your Claude API key first (Generate > Claude API key)")
    })?;
    if !(1..=20).contains(&inputs.stories) {
        return Err(anyhow::anyhow!("choose 1 to 20 stories").into());
    }
    // The lot, in project axes, when fitting to it.
    let lot = {
        let s = lock(&state)?;
        let doc = s.doc()?;
        studio_core::site::site_of(doc).and_then(|id| match doc.data(id).ok()? {
            ElementData::Site {
                boundary,
                offset,
                rotation,
                ..
            } => {
                let pts: Vec<_> = boundary
                    .iter()
                    .map(|p| studio_core::site::to_project(*offset, *rotation, *p))
                    .collect();
                let (lo, hi) = studio_geom::bounds_of(&pts)?;
                Some(((hi.x - lo.x) / 304.8, (hi.y - lo.y) / 304.8))
            }
            _ => None,
        })
    };
    let model = match inputs.model.as_str() {
        "claude-sonnet-5" => "claude-sonnet-5",
        _ => "claude-opus-5-5",
    };
    let request = studio_sync::claude::Request {
        model: model.into(),
        system: system_prompt(),
        text: user_prompt(&inputs, lot),
        images: inputs
            .images
            .iter()
            .take(5)
            .map(|i| studio_sync::claude::ImageInput {
                media_type: i.media_type.clone(),
                data: i.data.clone(),
            })
            .collect(),
        tool_name: "build_model".into(),
        tool_description: "Build the building in Rufplan Studio from this room plan.".into(),
        tool_schema: spec_schema(),
        max_tokens: 32_000,
    };
    let win = window.clone();
    let plan = tauri::async_runtime::spawn_blocking(move || {
        let mut last = 0usize;
        studio_sync::claude::call(&key, &request, &mut |partial| {
            // A few updates a second is plenty.
            if partial.len() >= last + 200 {
                last = partial.len();
                let _ = win.emit("generate-progress", progress_of(partial));
            }
        })
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(anyhow::Error::from)?;
    let spec: BuildingSpec = serde_json::from_value(plan)
        .map_err(|e| anyhow::anyhow!("Claude's plan doesn't fit the model: {e}"))?;
    let _ = window.emit(
        "generate-progress",
        GenerateProgress {
            phase: "building".into(),
            chars: 0,
            stories: spec.stories.len(),
            rooms: spec.stories.iter().map(|s| s.rooms.len()).sum(),
            name: Some(spec.name.clone()),
        },
    );
    let mut s = lock(&state)?;
    let report = s.edit(|d| generate::build(d, &spec))?;
    let state = finish(&window, &s)?;
    Ok(GenerateResult {
        state,
        report,
        summary: spec.summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> GenerateInputs {
        GenerateInputs {
            building_type: "Garden-style apartments".into(),
            stories: 3,
            area: Some(24_000.0),
            count: Some(24),
            bathrooms: None,
            style: "Modern farmhouse".into(),
            roof: "gable".into(),
            fit_lot: true,
            setbacks: [20.0, 10.0, 15.0],
            references: "Board and batten, black windows".into(),
            prompt: "Put the amenity room by the entry.".into(),
            images: vec![],
            model: "claude-opus-5-5".into(),
        }
    }

    #[test]
    fn the_brief_carries_every_input() {
        let p = user_prompt(&inputs(), Some((120.0, 200.0)));
        assert!(p.starts_with("Design a garden-style apartments with 3 stories."));
        for s in [
            "About 24000 sf gross.",
            "24 units.",
            "Style: Modern farmhouse.",
            "Roof: gable.",
            "must fit within 100 x 165 ft",
            "References: Board and batten",
            "amenity room by the entry",
        ] {
            assert!(p.contains(s), "{s}\n{p}");
        }
        let mut hotel = inputs();
        hotel.building_type = "Boutique hotel".into();
        hotel.fit_lot = false;
        hotel.roof = "auto".into();
        let p = user_prompt(&hotel, Some((120.0, 200.0)));
        assert!(
            p.contains("24 guest rooms (keys)") && !p.contains("Setbacks") && !p.contains("Roof:")
        );
        let sys = system_prompt();
        assert!(sys.contains("masonry-red-brick") && sys.contains("two stairs"));
    }

    #[test]
    fn the_schema_matches_the_spec() {
        let schema = spec_schema();
        let kinds = schema
            .pointer("/properties/stories/items/properties/rooms/items/properties/kind/enum")
            .unwrap();
        // Every kind in the schema parses as a room kind.
        for k in kinds.as_array().unwrap() {
            let v: studio_core::generate::RoomKind = serde_json::from_value(k.clone()).unwrap();
            let _ = v;
        }
        // A plan shaped like the schema parses into a building spec.
        let plan = json!({
            "name": "Maple Court", "summary": "Three stories.",
            "stories": [{ "height": 10, "rooms": [
                { "name": "Lobby", "kind": "lobby", "x": 0, "y": 0, "width": 20, "depth": 30 },
                { "name": "Unit 101", "kind": "unit", "x": 20, "y": 0, "width": 25, "depth": 30 }
            ]}],
            "roof": "gable", "pitch": 6, "structure": "wood",
            "materials": { "exteriorWalls": "plaster-stucco-white", "roof": "roof-asphalt-shingle" }
        });
        let spec: BuildingSpec = serde_json::from_value(plan).unwrap();
        assert_eq!(
            spec.materials.exterior_walls.as_deref(),
            Some("plaster-stucco-white")
        );
        assert_eq!(
            spec.stories[0].rooms[1].kind,
            studio_core::generate::RoomKind::Unit
        );
    }

    #[test]
    fn progress_counts_what_has_arrived() {
        let p = progress_of(
            r#"{"name": "Maple Court", "summary": "x", "stories": [{"height": 10, "rooms": [{"name": "Lobby", "kind": "lobby"}, {"name": "U", "kind": "unit""#,
        );
        assert_eq!(p.name.as_deref(), Some("Maple Court"));
        assert_eq!((p.stories, p.rooms), (1, 2));
    }

    /// Live: one Sonnet call with the saved key, built into a fresh document.
    /// `cargo test -p rufplan-studio live_generate -- --ignored --nocapture` (uses API credit).
    #[test]
    #[ignore]
    fn live_generate() {
        let key = get(CLAUDE_KEY).expect("no Claude key saved");
        let mut i = inputs();
        i.building_type = "Single-family house".into();
        i.stories = 1;
        i.area = Some(1400.0);
        i.count = Some(3);
        i.bathrooms = Some(2.0);
        i.fit_lot = false;
        i.prompt = String::new();
        let request = studio_sync::claude::Request {
            model: "claude-sonnet-5".into(),
            system: system_prompt(),
            text: user_prompt(&i, None),
            images: vec![],
            tool_name: "build_model".into(),
            tool_description: "Build the building in Rufplan Studio from this room plan.".into(),
            tool_schema: spec_schema(),
            max_tokens: 32_000,
        };
        let plan = studio_sync::claude::call(&key, &request, &mut |_| {}).unwrap();
        let spec: BuildingSpec = serde_json::from_value(plan).unwrap();
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let r = generate::build(&mut doc, &spec).unwrap();
        println!("{}: {:?}", spec.name, r);
    }
}
