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

pub(crate) const CLAUDE_KEY: &str = "anthropic-api-key";

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
    /// "claude-opus-5-5" or "claude-sonnet-5-5".
    pub model: String,
    /// An architect or work to design after (ADR-099), from the precedents or typed.
    #[serde(default)]
    pub precedent: String,
    /// Landscape the site and furnish the rooms.
    #[serde(default)]
    pub landscape: bool,
    /// Lay out the CD set: plans, ceiling plans, elevations, sections, schedules.
    #[serde(default)]
    pub cd_set: bool,
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
    let fascias = studio_core::fascia::catalog()
        .into_iter()
        .map(|f| format!("\"{}\"", f.name))
        .collect::<Vec<_>>()
        .join(", ");
    // Stair rooms from the stair the model builds (7" risers, 11" treads) for 10 ft.
    let (_, _, run) = studio_core::build::stair_layout(
        10.0 * 304.8,
        studio_core::build::DEFAULT_TREAD,
        studio_core::build::DEFAULT_MAX_RISER,
    );
    let ft = |mm: f64| (mm / 304.8 * 2.0).ceil() / 2.0;
    let straight = format!("4 x {} ft", ft(run + 304.8 + 900.0));
    let u_stair = format!("8 x {} ft", ft(run / 2.0 + 3.0 * 304.8 + 900.0 + 304.8));
    let mut precedents = String::new();
    for p in crate::precedents::all() {
        precedents.push_str(&format!("- {}: {}\n", p.name, p.moves));
    }
    format!(
        "You are an award-winning US residential and multifamily architect producing a schematic design that Rufplan Studio will build as a BIM model and document as a construction set. Always respond by calling the build_model tool exactly once, with no text before or after it; never reply in prose or ask questions. If the brief is unclear, make sensible assumptions and note them in the summary.

DESIGN LIKE AN ARCHITECT, NOT A BUILDER
A single box under one roof is a failure. Compose the building the way a published house is composed:
- Massing: two to four volumes and the spaces between them. A one-story wing beside a two-story volume; an upper story that steps back from, or cantilevers 4-12 ft past, the one below (often in a different direction); plan offsets of 2-8 ft between volumes; an L, T, U, pinwheel, bar or courtyard plan rather than a plain rectangle. Each story's rooms can extend beyond the story below.
- Outdoor rooms are rooms (kinds porch, terrace, deck, courtyard): a covered porch at the entry (columns go at its free corners), a terrace at grade off the living, dining and kitchen, a deck off the primary bedroom (an upper-story deck sits on the roof of the story below or cantilevers; railings are added), a courtyard open to the sky inside a U or O plan. Glass doors are added automatically wherever an indoor room shares an edge with a porch, terrace or deck.
- Roofs: each story gets a roof over the part of it nothing above covers, so wings, porches and set-backs all get roofs. Set a roof per story: flat (crisp, with a fascia), shed (one plane falling to lowSide), butterfly (two planes falling to a middle valley), gable or hip. A hip or gable wing against a taller volume leans to it as a shed. Overhang 0-6 ft.
- Materials by volume: give each story its own cladding (story cladding) - a stucco, stone, brick or board-formed concrete base under a cedar, metal or board-and-batten upper volume, or one material wrapping a monolithic form. Choose soffit, paving and deck materials too.
- Glazing by room (room glazing): window_wall or sliding_doors/folding_doors where rooms face terraces, views and the garden (living, dining, kitchen, primary bedroom); ribbon for horizontal bands (kitchens, studies, baths high up, Prairie and International Style); punched or large for traditional bedrooms; none for a solid street face, garages and service rooms. Glazing is placed on the room's exterior walls only.
- Fascia (fascia): one of {fascias}. Modern flat and shed roofs suit \"Modern Stepped Band 12\\\"\" or a thin metal edge; sloped traditional roofs a wood fascia.
- For houses set landscape and furnish true when the brief asks for them, and choose trees and shrubs (planting) suited to the region and style, by Asset Library name.

PRECEDENTS
When the brief names an architect, a building or a style, design in that manner: take its plan type, massing, roofs, materials, glazing and outdoor rooms - the moves below, in this plan's terms - for a new house that fits this brief. Don't copy the building. Put the precedent's name in precedent, and say in the summary which of its moves you took.
{precedents}
For an architect or work not listed, use what you know of their work in the same way.

HOW THE MODEL IS BUILT FROM YOUR PLAN
- Each story is a set of axis-aligned rectangular rooms in feet: x runs east, y runs north, origin at the building's south-west corner. y = 0 is the front (street) unless the brief says otherwise.
- Rooms on a story must not overlap; adjacent rooms share edges exactly; indoor rooms tile the story's enclosed area without gaps (outdoor rooms and courtyards may sit in notches and gaps). Round to 0.5 ft.
- Walls are generated on room edges: exterior walls round the indoor rooms (outdoor rooms have none of their own), interior walls between rooms. Living, dining, kitchen and entry are open to each other (no wall between any two of them).
- Doors are placed automatically so every room is reachable from the entry (ground floor) or a stair (upper floors), preferring corridors, entries, lobbies and living rooms. A room needs a shared edge of at least 4 ft with the room it's entered from. Bathrooms, closets, laundry and storage are only entered, never passed through.
- Windows follow each room's glazing (by its kind when not given). Put rooms that want light and view on outside walls.
- Stairs: a room of kind stair on every story it serves, at the same x, y, width and depth on each (the top story's stair room is the landing). For a 10 ft floor-to-floor a straight stair needs a room at least {straight}; a U stair at least {u_stair}. Bigger is fine; smaller isn't. Multifamily and hotels need at least two stairs, near opposite ends of the corridor.
- Stack bathrooms and kitchens where you can. Set cantileverColumns true to put posts under a ground-floor cantilever's outer corners (leave it false for a dramatic cantilever up to about 10 ft).

TYPICAL US SIZES
- Houses: bedroom 11 x 12 min, primary 13 x 15+, bath 5 x 8 min, primary bath 8 x 11+, walk-in closet 6 x 8, kitchen 12 x 14, living 14 x 18+, dining 11 x 13, hall 3.5-4 ft wide, two-car garage 22 x 22, porch 8 ft deep min, terrace 12 x 16+, deck 8 ft deep min.
- Multifamily: double-loaded corridor 5-6 ft wide; unit depth 26-32 ft; studio 450-550 sf, one-bedroom 650-800 sf, two-bedroom 950-1,150 sf. For more than about a dozen units, model each unit as one room of kind unit (named like \"Unit 201 - 1BR\") with a deck (balcony) on the outside wall; for smaller buildings you may divide units into rooms. Vary the massing: step the top story back, set bays forward, change the cladding by story.
- Hotels: guest room (key) 12-13 ft wide x 28-30 ft deep as one room of kind guest_room; corridor 6 ft; lobby, front desk and amenity on the ground floor, with a terrace.
- Floor to floor: houses 10 ft (9-10 upper); multifamily 10 ft; hotel upper floors 10 ft; ground-floor lobby or retail 14-18 ft.
- Keep each story under about 60 rooms.

MATERIALS
Pick library materials by id for exterior walls (the building's default; story cladding overrides it), interior walls, floors, roof, soffit, paving and deck, to suit the style and finish level:
{materials}
Also choose the building's roof (hip, gable, flat, shed or butterfly; each story's roof overrides it), pitch (rise per 12) and structure (wood, or masonry for concrete and CMU buildings).

WINDOWS
Choose the window family to suit the style: DoubleHung for colonial, Georgian, craftsman and other traditional houses; Casement for modern, farmhouse and contemporary; SingleHung for builder-grade houses and apartments; Slider for budget and mid-century; Awning or PictureAwning for modern multifamily and hotels. Bedrooms, kitchens and offices get that family; living spaces and units its twin; window walls are storefront. Grille: Colonial for colonial and Georgian, Craftsman for craftsman and bungalows, Prairie for prairie style, otherwise None. Finish: Black for modern farmhouse and modern, Bronze for contemporary and commercial, White for traditional, Wood for rustic and lodge, Almond for Mediterranean and Spanish.

Give the building a name and a three- to five-sentence summary for the owner (gross area, unit or bedroom count, the precedent if any, and the key design moves: massing, outdoor rooms, roofs, materials)."
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
    if !i.precedent.trim().is_empty() {
        let p = i.precedent.trim();
        match crate::precedents::moves_of(p) {
            Some(m) => s.push_str(&format!("\n\nDesign it in the manner of {p}: {m}")),
            None => s.push_str(&format!(
                "\n\nDesign it in the manner of {p}: take that architect's or building's plan type, massing, roofs, materials, glazing and outdoor rooms."
            )),
        }
    }
    if i.landscape {
        s.push_str("\n\nLandscape the site and furnish the rooms (landscape and furnish true).");
    } else {
        s.push_str("\n\nLeave the site and the furniture (landscape and furnish false).");
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
        "porch",
        "terrace",
        "deck",
        "courtyard",
    ];
    let ids: Vec<String> = library::library().into_iter().map(|p| p.id).collect();
    let fascias: Vec<String> = studio_core::fascia::catalog()
        .into_iter()
        .map(|f| f.name)
        .collect();
    let plants = studio_core::planting::catalog();
    let trees: Vec<String> = plants
        .iter()
        .filter(|p| p.spec.group.is_tree())
        .map(|p| p.name.clone())
        .collect();
    let shrubs: Vec<String> = plants
        .iter()
        .filter(|p| !p.spec.group.is_tree())
        .map(|p| p.name.clone())
        .collect();
    let roof_kinds = ["hip", "gable", "flat", "shed", "butterfly"];
    let glazing = [
        "none",
        "punched",
        "large",
        "ribbon",
        "window_wall",
        "sliding_doors",
        "folding_doors",
    ];
    json!({
        "type": "object",
        "required": ["name", "summary", "precedent", "stories", "roof", "pitch", "structure", "materials", "windows", "fascia", "overhang", "cantileverColumns", "landscape", "furnish", "planting"],
        "properties": {
            "name": { "type": "string" },
            "summary": { "type": "string" },
            "precedent": { "type": "string", "description": "The architect or work it's designed after, or empty." },
            "fascia": { "type": "string", "enum": fascias },
            "overhang": { "type": "number", "description": "Roof overhang, feet (0-6)." },
            "cantileverColumns": { "type": "boolean", "description": "Posts under a ground-floor cantilever's outer corners." },
            "landscape": { "type": "boolean" },
            "furnish": { "type": "boolean" },
            "planting": {
                "type": "object",
                "properties": {
                    "trees": { "type": "array", "items": { "type": "string", "enum": trees }, "description": "3-6 tree species." },
                    "shrubs": { "type": "array", "items": { "type": "string", "enum": shrubs }, "description": "3-5 shrubs, perennials and grasses for the beds." }
                }
            },
            "stories": {
                "type": "array",
                "description": "Ground floor first.",
                "items": {
                    "type": "object",
                    "required": ["height", "rooms"],
                    "properties": {
                        "height": { "type": "number", "description": "Floor to floor, feet." },
                        "cladding": { "type": "string", "enum": ids, "description": "This story's exterior wall material." },
                        "roof": {
                            "type": "object",
                            "description": "The roof over what nothing above covers.",
                            "required": ["kind"],
                            "properties": {
                                "kind": { "type": "string", "enum": roof_kinds },
                                "pitch": { "type": "number", "description": "Rise per 12." },
                                "lowSide": { "type": "string", "enum": ["north", "south", "east", "west"], "description": "A shed roof's low edge." }
                            }
                        },
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
                                    "depth": { "type": "number", "description": "North-south, feet." },
                                    "glazing": { "type": "string", "enum": glazing }
                                }
                            }
                        }
                    }
                }
            },
            "roof": { "type": "string", "enum": roof_kinds },
            "pitch": { "type": "number", "description": "Rise per 12." },
            "structure": { "type": "string", "enum": ["wood", "masonry"] },
            "materials": {
                "type": "object",
                "properties": {
                    "exteriorWalls": { "type": "string", "enum": ids },
                    "interiorWalls": { "type": "string", "enum": ids },
                    "floors": { "type": "string", "enum": ids },
                    "roof": { "type": "string", "enum": ids },
                    "soffit": { "type": "string", "enum": ids },
                    "paving": { "type": "string", "enum": ids },
                    "deck": { "type": "string", "enum": ids }
                }
            },
            "windows": {
                "type": "object",
                "properties": {
                    "family": {
                        "type": "string",
                        "enum": ["DoubleHung", "SingleHung", "Casement", "Awning", "Slider", "Slider3", "PictureAwning", "PictureCasement", "Fixed"]
                    },
                    "grille": { "type": "string", "enum": ["None", "Colonial", "Prairie", "Craftsman"] },
                    "finish": { "type": "string", "enum": ["White", "Almond", "Bronze", "Black", "Wood"] }
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
        "claude-sonnet-5-5" | "claude-sonnet-5" => "claude-sonnet-5-5",
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
    let cd = inputs.cd_set.then(|| set_type(&inputs.building_type));
    let report = s.edit(|d| build_with_set(d, &spec, cd))?;
    let state = finish(&window, &s)?;
    Ok(GenerateResult {
        state,
        report,
        summary: spec.summary,
    })
}

/// The sheet set's building type for a Generate building type.
pub fn set_type(label: &str) -> studio_sheets::sets::BuildingType {
    use studio_sheets::sets::BuildingType as B;
    let l = label.to_lowercase();
    if l.contains("duplex") {
        B::Duplex
    } else if l.contains("townhouse") {
        B::Townhouses
    } else if l.contains("garden") {
        B::GardenApartments
    } else if l.contains("mid-rise") || l.contains("apartment") {
        B::MidRiseApartments
    } else if l.contains("mixed") {
        B::MixedUse
    } else if l.contains("hotel") {
        B::Hotel
    } else {
        B::SingleFamily
    }
}

/// Builds the plan and, with \`cd\`, lays out its CD set (ADR-099): one undo step.
pub fn build_with_set(
    doc: &mut studio_core::Document,
    spec: &BuildingSpec,
    cd: Option<studio_sheets::sets::BuildingType>,
) -> studio_core::CoreResult<BuildReport> {
    let mark = doc.undo_depth();
    let mut report = generate::build(doc, spec)?;
    if let Some(building_type) = cd {
        let o = studio_sheets::sets::SetOptions {
            building_type,
            phases: vec!["CD".into()],
            size: studio_core::SheetSize::ArchD,
        };
        // Elevations and sections show the building, not the trees in front of it: hidden
        // before the set is laid out (the sections it cuts too, then laid out again), so
        // each drawing is scaled to the building.
        hide_planting_in_drawings(doc)?;
        match studio_sheets::sets::create(doc, &o) {
            Ok(_) => {
                hide_planting_in_drawings(doc)?;
                match studio_sheets::sets::create(doc, &o) {
                    Ok(r) => report.sheets = r.created + r.updated,
                    Err(e) => report.warnings.push(format!("CD set: {e}")),
                }
            }
            Err(e) => report.warnings.push(format!("CD set: {e}")),
        }
        doc.merge_undo(mark, &format!("Generate {}", spec.name));
    }
    Ok(report)
}

/// Hides planting in every elevation and section.
fn hide_planting_in_drawings(doc: &mut studio_core::Document) -> studio_core::CoreResult<()> {
    use studio_core::{Category, ElementData, ViewKind};
    let drawings: Vec<studio_core::ElementId> = doc
        .of(Category::View)
        .filter(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::Elevation { .. } | ViewKind::Section { .. },
                hidden_categories,
                ..
            } => !hidden_categories.contains(&Category::Planting),
            _ => false,
        })
        .map(|e| e.id)
        .collect();
    for v in drawings {
        studio_core::visibility::set_category_visible(doc, v, &[Category::Planting], false)?;
    }
    Ok(())
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
            precedent: "Richard Neutra".into(),
            landscape: true,
            cd_set: true,
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
            "Design it in the manner of Richard Neutra: A pinwheel",
            "landscape and furnish true",
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
        assert!(sys.contains("Fallingwater") && sys.contains("Modern Stepped Band 12"));
        assert!(!sys.contains("no cantilevers"));
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

    /// A plan in the schema's new terms (ADR-099) parses, builds and lays out its CD set.
    #[test]
    fn a_designed_plan_builds_with_its_cd_set() {
        let plan = json!({
            "name": "Canyon House", "summary": "x", "precedent": "Richard Neutra",
            "stories": [
                { "height": 10, "cladding": "plaster-stucco-white", "roof": { "kind": "flat" }, "rooms": [
                    { "name": "Garage", "kind": "garage", "x": 0, "y": 0, "width": 22, "depth": 24, "glazing": "none" },
                    { "name": "Entry", "kind": "entry", "x": 22, "y": 0, "width": 12, "depth": 10 },
                    { "name": "Stair", "kind": "stair", "x": 22, "y": 10, "width": 12, "depth": 14 },
                    { "name": "Living", "kind": "living", "x": 34, "y": 0, "width": 24, "depth": 20, "glazing": "window_wall" },
                    { "name": "Kitchen", "kind": "kitchen", "x": 34, "y": 20, "width": 24, "depth": 14, "glazing": "ribbon" },
                    { "name": "Porch", "kind": "porch", "x": 22, "y": -8, "width": 12, "depth": 8 },
                    { "name": "Terrace", "kind": "terrace", "x": 34, "y": 34, "width": 24, "depth": 14 }
                ]},
                { "height": 10, "cladding": "siding-cedar-vertical", "roof": { "kind": "shed", "pitch": 2, "lowSide": "north" }, "rooms": [
                    { "name": "Stair", "kind": "stair", "x": 22, "y": 10, "width": 12, "depth": 14 },
                    { "name": "Hall", "kind": "corridor", "x": 34, "y": 16, "width": 26, "depth": 4 },
                    { "name": "Primary Bedroom", "kind": "bedroom", "x": 34, "y": 4, "width": 26, "depth": 12, "glazing": "sliding_doors" },
                    { "name": "Bath", "kind": "bathroom", "x": 34, "y": 20, "width": 12, "depth": 10 },
                    { "name": "Bedroom 2", "kind": "bedroom", "x": 46, "y": 20, "width": 14, "depth": 10 },
                    { "name": "Deck", "kind": "deck", "x": 60, "y": 4, "width": 10, "depth": 12 }
                ]}
            ],
            "roof": "flat", "pitch": 2, "structure": "wood",
            "materials": { "soffit": "siding-cedar-vertical", "paving": "stone-travertine", "deck": "wood-ipe-deck" },
            "windows": { "family": "Casement", "finish": "Black" },
            "fascia": "Modern Stepped Band 12\"", "overhang": 3, "cantileverColumns": false,
            "landscape": true, "furnish": true,
            "planting": { "trees": ["Palo Verde", "Honey Mesquite"], "shrubs": ["Blue Agave", "Fountain Grass"] }
        });
        let spec: BuildingSpec = serde_json::from_value(plan).unwrap();
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let depth = doc.undo_depth();
        let r = build_with_set(&mut doc, &spec, Some(set_type("Single-family house"))).unwrap();
        assert!(
            r.warnings.iter().all(|w| !w.starts_with("CD set")),
            "{:?}",
            r.warnings
        );
        assert!(r.sheets >= 5, "{r:?}");
        let numbers: Vec<String> = studio_core::ops::sheets(&doc)
            .into_iter()
            .map(|s| s.1)
            .collect();
        assert!(numbers.iter().any(|n| n.starts_with("A-1")), "{numbers:?}");
        assert!(r.dimensions > 0 && r.plants > 0 && r.furniture > 0, "{r:?}");
        // Building and sheets undo together.
        doc.undo().unwrap();
        assert_eq!(doc.undo_depth(), depth);
        // Every new enum in the schema parses.
        let schema = spec_schema();
        for path in [
            "/properties/stories/items/properties/rooms/items/properties/glazing/enum",
            "/properties/stories/items/properties/roof/properties/kind/enum",
            "/properties/roof/enum",
        ] {
            for v in schema.pointer(path).unwrap().as_array().unwrap() {
                let ok = serde_json::from_value::<studio_core::generate::GlazingKind>(v.clone())
                    .is_ok()
                    || serde_json::from_value::<studio_core::generate::RoofKind>(v.clone()).is_ok();
                assert!(ok, "{v}");
            }
        }
    }

    #[test]
    fn progress_counts_what_has_arrived() {
        let p = progress_of(
            r#"{"name": "Maple Court", "summary": "x", "stories": [{"height": 10, "rooms": [{"name": "Lobby", "kind": "lobby"}, {"name": "U", "kind": "unit""#,
        );
        assert_eq!(p.name.as_deref(), Some("Maple Court"));
        assert_eq!((p.stories, p.rooms), (1, 2));
    }

    /// Live: one Claude call with the saved key, built (with its CD set) into a fresh
    /// document (uses API credit):
    /// `GEN_PRECEDENT="Richard Neutra" GEN_OUT=target/generated cargo test -p rufplan-studio
    /// live_generate -- --ignored --nocapture`. GEN_OUT writes <out>.json (the plan) and
    /// <out>.ruf; GEN_MODEL picks the model (Opus by default).
    #[test]
    #[ignore]
    fn live_generate() {
        let key = get(CLAUDE_KEY).expect("no Claude key saved");
        let mut i = inputs();
        i.building_type = "Single-family house".into();
        i.stories = 2;
        i.area = Some(3200.0);
        i.count = Some(4);
        i.bathrooms = Some(3.5);
        i.style = "Modern".into();
        i.roof = "auto".into();
        i.fit_lot = false;
        i.references = String::new();
        i.prompt = "A family house on a sloping wooded lot, open to a back garden.".into();
        i.precedent = std::env::var("GEN_PRECEDENT").unwrap_or_default();
        let request = studio_sync::claude::Request {
            model: std::env::var("GEN_MODEL").unwrap_or_else(|_| "claude-opus-5-5".into()),
            system: system_prompt(),
            text: user_prompt(&i, None),
            images: vec![],
            tool_name: "build_model".into(),
            tool_description: "Build the building in Rufplan Studio from this room plan.".into(),
            tool_schema: spec_schema(),
            max_tokens: 32_000,
        };
        let plan = studio_sync::claude::call(&key, &request, &mut |_| {}).unwrap();
        let out = std::env::var("GEN_OUT").ok();
        if let Some(o) = &out {
            std::fs::write(
                format!("{o}.json"),
                serde_json::to_string_pretty(&plan).unwrap(),
            )
            .unwrap();
        }
        let spec: BuildingSpec = serde_json::from_value(plan).unwrap();
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let r = build_with_set(&mut doc, &spec, Some(set_type(&i.building_type))).unwrap();
        println!("{}\n{}\n{:?}", spec.name, spec.summary, r);
        if let Some(o) = &out {
            studio_io::Project::new("0.0.1", doc)
                .save(std::path::Path::new(&format!("{o}.ruf")), "0.0.1")
                .unwrap();
        }
    }

    /// Dev aid: rebuilds a plan saved by live_generate, without calling Claude:
    /// `GEN_PLAN=target/generated/neutra.json GEN_OUT=target/generated/neutra cargo test -p
    /// rufplan-studio build_saved_plan -- --ignored --nocapture` writes <out>.ruf and its
    /// sheets as <out>.pdf.
    #[test]
    #[ignore]
    fn build_saved_plan() {
        let plan: Value = serde_json::from_str(
            &std::fs::read_to_string(std::env::var("GEN_PLAN").unwrap()).unwrap(),
        )
        .unwrap();
        let spec: BuildingSpec = serde_json::from_value(plan).unwrap();
        let mut doc = studio_core::Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let r = build_with_set(&mut doc, &spec, Some(set_type("Single-family house"))).unwrap();
        println!("{r:?}");
        let out = std::env::var("GEN_OUT").unwrap();
        // The sheets as a PDF, to look over.
        // GEN_SHEETS (numbers, comma-separated) picks some.
        let only = std::env::var("GEN_SHEETS").unwrap_or_default();
        let sheets: Vec<_> = studio_core::ops::sheets(&doc)
            .into_iter()
            .filter(|x| only.is_empty() || only.split(',').any(|n| n == x.1))
            .map(|x| x.0)
            .collect();
        println!(
            "{:?}",
            studio_core::ops::sheets(&doc)
                .into_iter()
                .map(|x| x.1)
                .collect::<Vec<_>>()
        );
        let pdf = studio_sheets::export_pdf(&doc, &sheets, "2026-10-01").unwrap();
        std::fs::write(format!("{out}.pdf"), pdf).unwrap();
        studio_io::Project::new("0.0.1", doc)
            .save(std::path::Path::new(&format!("{out}.ruf")), "0.0.1")
            .unwrap();
    }
}
