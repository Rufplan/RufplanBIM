//! IPC commands for lighting fixtures, Artificial Lights and Sun Settings (ADR-057). Thin
//! wrappers over studio-core `lighting` and studio-views `lighting`.

use serde::{Deserialize, Serialize};
use studio_core::camera::SunPosition;
use studio_core::lighting::{self, BuildingUse, LightFamily, LightPreset, SunSettings};
use studio_core::{structure, Category, ElementId};
use studio_geom::Pt;
use studio_views::lighting::{FixtureThumb, LightInfo};
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;
use crate::window_cmds::LoadedWindows;

type CommandResult<T> = Result<T, CommandError>;
type StateResult = Result<Option<AppState>, CommandError>;

fn edit_state(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&mut crate::session::Session) -> anyhow::Result<()>,
) -> StateResult {
    let mut session = lock(state)?;
    f(&mut session)?;
    finish(window, &session)
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct UseOption {
    pub id: BuildingUse,
    pub label: String,
}

/// The lighting library: its fixtures, the picker's groups and the building types.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LightLibrary {
    pub presets: Vec<LightPreset>,
    pub groups: Vec<String>,
    pub uses: Vec<UseOption>,
}

#[tauri::command]
pub fn lighting_library() -> LightLibrary {
    LightLibrary {
        presets: lighting::catalog(),
        groups: LightFamily::GROUPS.iter().map(|g| (*g).into()).collect(),
        uses: BuildingUse::ALL
            .iter()
            .map(|u| UseOption {
                id: *u,
                label: u.label().into(),
            })
            .collect(),
    }
}

/// Loads library fixtures by name (one undo step), reusing any already there.
#[tauri::command]
pub fn load_lighting_types(
    names: Vec<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<LoadedWindows> {
    let mut s = lock(&state)?;
    let ids = s.edit(|d| lighting::load(d, &names))?;
    let state = finish(&window, &s)?;
    Ok(LoadedWindows { state, ids })
}

/// What a fixture thumbnail shows: a project type, or a library fixture by name.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub enum FixtureSource {
    Type(ElementId),
    Preset(String),
}

#[tauri::command]
pub fn fixture_thumbnail(
    source: FixtureSource,
    state: State<'_, SessionState>,
) -> CommandResult<FixtureThumb> {
    let spec = match source {
        FixtureSource::Preset(name) => lighting::catalog()
            .into_iter()
            .find(|p| p.name == name)
            .map(|p| p.spec)
            .ok_or_else(|| anyhow::anyhow!("no fixture \"{name}\" in the library"))?,
        FixtureSource::Type(id) => {
            let s = lock(&state)?;
            lighting::spec_of(s.doc()?, id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not a lighting fixture type"))?
        }
    };
    Ok(studio_views::lighting::thumb(&spec))
}

/// Places a fixture. In a plan or ceiling plan its level is the view's and its height
/// comes from its type (the ceiling, the wall height or the floor); a pick in 3D gives the
/// level and elevation.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_lighting_fixture(
    view: ElementId,
    type_id: Option<ElementId>,
    at: Pt,
    rotation: Option<f64>,
    level: Option<ElementId>,
    elevation: Option<f64>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        // In an elevation or section (ADR-059): on the wall face clicked, or a section's
        // cut plane; at the height clicked for wall fixtures, else at the type's own.
        let (level, at, elevation) = match level {
            Some(l) => (l, at, elevation),
            None if studio_views::view_frame(s.doc()?, view).is_some() => {
                let doc = s.doc()?;
                let (p, z, _) = studio_views::view_refs::model_point(doc, view, at, 10.0)
                    .ok_or_else(|| {
                        anyhow::anyhow!("click a wall (or, in a section, anywhere on the cut) to place the fixture")
                    })?;
                let levels = doc.levels();
                let (lid, _, lz) = levels
                    .iter()
                    .filter(|(_, _, e)| *e <= z + 1.0)
                    .max_by(|a, b| a.2.total_cmp(&b.2))
                    .or(levels.first())
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("the project has no levels"))?;
                let wall_mounted = type_id
                    .and_then(|t| lighting::spec_of(doc, t))
                    .is_some_and(|sp| sp.mount == lighting::LightMount::Wall);
                (lid, p, if wall_mounted { Some(z - lz) } else { None })
            }
            None => (s.view_level(view)?, at, elevation),
        };
        let t = type_id
            .or_else(|| {
                s.doc()
                    .ok()
                    .and_then(|d| structure::default_type(d, Category::LightingFixtureType))
            })
            .ok_or_else(|| anyhow::anyhow!("load a lighting fixture type first"))?;
        s.edit(|d| lighting::create_fixture(d, t, level, at, rotation.unwrap_or(0.0), elevation))?;
        Ok(())
    })
}

/// Artificial Lights: switches and dims fixtures (one undo step).
#[tauri::command]
pub fn set_lights(
    ids: Vec<ElementId>,
    on: Option<bool>,
    dimming: Option<f64>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| lighting::set_lights(d, &ids, on, dimming))?;
        Ok(())
    })
}

/// Every fixture's light, for renders and Artificial Lights; those hidden in `view` are
/// left out.
#[tauri::command]
pub fn lights(
    view: Option<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<Vec<LightInfo>> {
    let s = lock(&state)?;
    let doc = s.doc()?;
    let v = view.and_then(|v| doc.data(v).ok());
    Ok(studio_views::lighting::lights(doc)
        .into_iter()
        .filter(|l| v.is_none_or(|v| !studio_core::visibility::hidden_in(doc, v, l.el)))
        .collect())
}

#[tauri::command]
pub fn set_sun_settings(
    settings: SunSettings,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_state(&window, &state, |s| {
        s.edit(|d| lighting::set_sun_settings(d, settings))?;
        Ok(())
    })
}

/// The sun for the project's Sun Settings.
#[tauri::command]
pub fn sun_now(state: State<'_, SessionState>) -> CommandResult<SunPosition> {
    let s = lock(&state)?;
    Ok(lighting::project_sun_now(s.doc()?))
}
