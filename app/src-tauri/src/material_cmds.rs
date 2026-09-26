//! IPC commands for the material library (ADR-029). Thin wrappers over studio-core; texture
//! maps are downloaded (once) and cached by studio-sync.

use serde::Serialize;
use studio_core::library::{self, Appearance, Preset, TextureMap};
use studio_core::{Category, ElementData, ElementId};
use tauri::{Manager, State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type StateResult = Result<Option<AppState>, CommandError>;
type CommandResult<T> = Result<T, CommandError>;

/// The library, for the Material Browser.
#[tauri::command]
pub fn material_library() -> Vec<Preset> {
    library::library()
}

/// Adds a library material to the project (the new one is the last in the state's list).
#[tauri::command]
pub fn add_library_material(
    id: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    s.edit(|d| library::add_preset(d, &id))?;
    finish(&window, &s)
}

/// Applies a material to the outside finish of the selected elements' types.
#[tauri::command]
pub fn apply_material(
    ids: Vec<ElementId>,
    material: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    s.edit(|d| library::apply_to(d, &ids, material))?;
    finish(&window, &s)
}

/// A project material as the renderer needs it.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenderMaterial {
    pub id: ElementId,
    pub name: String,
    pub color: [u8; 3],
    pub appearance: Appearance,
}

#[tauri::command]
pub fn render_materials(state: State<'_, SessionState>) -> CommandResult<Vec<RenderMaterial>> {
    let s = lock(&state)?;
    Ok(s.doc()?
        .of(Category::Material)
        .filter_map(|e| match &e.data {
            ElementData::Material {
                name,
                color,
                appearance,
                ..
            } => Some(RenderMaterial {
                id: e.id,
                name: name.clone(),
                color: *color,
                appearance: appearance.clone(),
            }),
            _ => None,
        })
        .collect())
}

/// A library texture map's JPEG bytes: from this computer's cache, else downloaded once
/// from Poly Haven. Only the library's own textures can be fetched.
#[tauri::command]
pub async fn material_texture(
    set: String,
    map: TextureMap,
    app: tauri::AppHandle,
) -> CommandResult<tauri::ipc::Response> {
    let url = library::texture_url(&set, map)
        .ok_or_else(|| anyhow::anyhow!("{set} is not a library texture"))?;
    let cache = app
        .path()
        .app_local_data_dir()
        .map_err(anyhow::Error::from)?
        .join("textures");
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        studio_sync::textures::texture_map(&studio_sync::UreqHttp::default(), &cache, &set, &url)
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(anyhow::Error::from)?;
    Ok(tauri::ipc::Response::new(bytes))
}
