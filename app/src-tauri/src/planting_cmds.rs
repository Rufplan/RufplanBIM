//! IPC commands for the Vegetation tab (ADR-064): Enscape's Asset Library of plants,
//! placing them, their 3D models and textures, and the base ground. Thin wrappers over
//! studio-core `planting` and studio-views `plants` and `foliage`.

use serde::{Deserialize, Serialize};
use studio_core::planting::{self, PlantLibrary, PlantSpec};
use studio_core::{Category, ElementId};
use studio_geom::Pt;
use studio_views::plants::{PlantInstance, PlantModel};
use tauri::{Manager, State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;
use crate::window_cmds::LoadedWindows;

type CommandResult<T> = Result<T, CommandError>;
type StateResult = Result<Option<AppState>, CommandError>;

#[tauri::command]
pub fn planting_library() -> PlantLibrary {
    planting::library()
}

/// Loads library plants by name (one undo step), reusing any already there.
#[tauri::command]
pub fn load_planting_types(
    names: Vec<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<LoadedWindows> {
    let mut s = lock(&state)?;
    let ids = s.edit(|d| planting::load(d, &names))?;
    let state = finish(&window, &s)?;
    Ok(LoadedWindows { state, ids })
}

/// A plant to show: a project type, or a library asset by name.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub enum PlantSource {
    Type(ElementId),
    Preset(String),
}

fn spec_for(source: &PlantSource, state: &State<'_, SessionState>) -> CommandResult<PlantSpec> {
    Ok(match source {
        PlantSource::Preset(name) => planting::catalog()
            .into_iter()
            .find(|p| &p.name == name)
            .map(|p| p.spec)
            .ok_or_else(|| anyhow::anyhow!("no plant \"{name}\" in the library"))?,
        PlantSource::Type(id) => {
            let s = lock(state)?;
            planting::spec_of(s.doc()?, *id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not a planting type"))?
        }
    })
}

/// A plant's 3D model (variant 0-2).
#[tauri::command]
pub async fn plant_model(
    source: PlantSource,
    variant: u32,
    state: State<'_, SessionState>,
) -> CommandResult<PlantModel> {
    let spec = spec_for(&source, &state)?;
    let m = tauri::async_runtime::spawn_blocking(move || {
        studio_views::plants::model(&spec, variant % studio_views::plants::VARIANTS)
    })
    .await
    .map_err(anyhow::Error::from)?;
    Ok(m)
}

/// Which of a plant's textures.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export)]
pub enum PlantMap {
    Foliage,
    Bark,
    BarkNormal,
}

/// Foliage atlases are this many pixels square; bark tiles half that.
const FOLIAGE_SIZE: usize = 1024;

/// A plant's texture as PNG bytes: made on first use and kept in this computer's cache,
/// named by what it depends on.
#[tauri::command]
pub async fn plant_texture(
    source: PlantSource,
    map: PlantMap,
    size: Option<usize>,
    app: tauri::AppHandle,
    state: State<'_, SessionState>,
) -> CommandResult<tauri::ipc::Response> {
    let spec = spec_for(&source, &state)?;
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(anyhow::Error::from)?
        .join("textures")
        .join("plants");
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        plant_texture_bytes(
            &dir,
            &spec,
            map,
            size.unwrap_or(FOLIAGE_SIZE)
                .clamp(128, 2048)
                .next_power_of_two(),
        )
    })
    .await
    .map_err(anyhow::Error::from)??;
    Ok(tauri::ipc::Response::new(bytes))
}

/// A texture's cache key: a hash of what draws it.
fn key_of(spec: &PlantSpec, map: PlantMap) -> String {
    let what = match map {
        PlantMap::Foliage => format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}{}",
            spec.foliage,
            spec.form,
            spec.group,
            spec.leaf_colors(),
            spec.flowers_now(),
            spec.bark,
            spec.botanical
        ),
        PlantMap::Bark | PlantMap::BarkNormal => format!("{:?}{:?}", spec.bark_kind, spec.bark),
    };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in what.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

pub(crate) fn plant_texture_bytes(
    dir: &std::path::Path,
    spec: &PlantSpec,
    map: PlantMap,
    size: usize,
) -> anyhow::Result<Vec<u8>> {
    let name = match map {
        PlantMap::Foliage => "foliage",
        PlantMap::Bark => "bark",
        PlantMap::BarkNormal => "bark-normal",
    };
    let path = dir.join(format!("{}-{size}-{name}.png", key_of(spec, map)));
    if let Ok(b) = std::fs::read(&path) {
        return Ok(b);
    }
    let bytes = match map {
        PlantMap::Foliage => {
            let img = studio_views::foliage::atlas(spec, size);
            encode(img.width, img.height, &img.data, png::ColorType::Rgba)?
        }
        PlantMap::Bark | PlantMap::BarkNormal => {
            let s = size / 2;
            let (c, n) = studio_views::foliage::bark(spec.bark_kind, spec.bark, s);
            encode(
                s,
                s,
                if matches!(map, PlantMap::Bark) {
                    &c
                } else {
                    &n
                },
                png::ColorType::Rgb,
            )?
        }
    };
    std::fs::create_dir_all(dir)?;
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &path)?;
    Ok(bytes)
}

fn encode(w: usize, h: usize, data: &[u8], color: png::ColorType) -> anyhow::Result<Vec<u8>> {
    let mut out = vec![];
    let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
    enc.set_color(color);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    let mut wr = enc.write_header()?;
    wr.write_image_data(data)?;
    wr.finish()?;
    Ok(out)
}

/// Every planting's placement, for the full models (those hidden in `view` left out).
#[tauri::command]
pub fn plant_instances(
    view: Option<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<Vec<PlantInstance>> {
    let s = lock(&state)?;
    let doc = s.doc()?;
    let v = view.and_then(|v| doc.data(v).ok());
    Ok(studio_views::plants::instances(doc)
        .into_iter()
        .filter(|p| v.is_none_or(|v| !studio_core::visibility::hidden_in(doc, v, p.el)))
        .collect())
}

/// One plant to place: where, turned how far and how big.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct PlantAt {
    pub at: Pt,
    pub rotation: f64,
    pub scale: f64,
}

/// Places plants (one undo step): on the plan's level, or the level given (from 3D).
#[tauri::command]
pub fn create_plants(
    view: ElementId,
    type_id: ElementId,
    at: Vec<PlantAt>,
    level: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    let level = match level {
        Some(l) => l,
        None => s.view_level(view).or_else(|_| {
            // Elevations, sections and 3D: the level at grade.
            let doc = s.doc()?;
            doc.levels()
                .into_iter()
                .min_by(|a, b| a.2.abs().total_cmp(&b.2.abs()))
                .map(|l| l.0)
                .ok_or_else(|| anyhow::anyhow!("the project has no levels"))
        })?,
    };
    let pts: Vec<(Pt, f64, f64)> = at.iter().map(|p| (p.at, p.rotation, p.scale)).collect();
    s.edit(|d| planting::create_plants(d, type_id, level, &pts))?;
    finish(&window, &s)
}

/// Sets the base ground to a project material, or a Site & Landscape library material by id
/// (loaded first), or plain lawn (neither).
#[tauri::command]
pub fn set_base_ground(
    material: Option<ElementId>,
    preset: Option<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    s.edit(|d| {
        let m = match (&preset, material) {
            (Some(p), _) => Some(planting::ground_material(d, p)?),
            (None, m) => m,
        };
        planting::set_ground(d, m)
    })?;
    finish(&window, &s)
}

/// The Site & Landscape library materials for the ground picker.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroundChoice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub color: [u8; 3],
    pub texture: Option<String>,
}

#[tauri::command]
pub fn ground_library() -> Vec<GroundChoice> {
    studio_core::library::library()
        .into_iter()
        .filter(|p| p.category == planting::GROUND_CATEGORY)
        .map(|p| GroundChoice {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            color: p.color,
            texture: p.appearance.texture.clone(),
        })
        .collect()
}

/// The planting types for a picker (Properties' type list).
#[tauri::command]
pub fn planting_types(state: State<'_, SessionState>) -> CommandResult<Vec<(ElementId, String)>> {
    let s = lock(&state)?;
    Ok(s.doc()?
        .of(Category::PlantingType)
        .map(|e| (e.id, e.data.name()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plant_textures_are_png_and_cached_by_what_draws_them() {
        let dir = tempfile::tempdir().unwrap();
        let maple = planting::catalog()
            .into_iter()
            .find(|p| p.name == "Red Maple")
            .unwrap()
            .spec;
        let a = plant_texture_bytes(dir.path(), &maple, PlantMap::Foliage, 64).unwrap();
        assert_eq!(&a[..8], b"\x89PNG\r\n\x1a\n");
        let b = plant_texture_bytes(dir.path(), &maple, PlantMap::Bark, 64).unwrap();
        assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        // An autumn maple's leaves are another texture; its bark is the same one.
        let mut fall = maple.clone();
        fall.season = planting::Season::Autumn;
        assert_ne!(
            key_of(&maple, PlantMap::Foliage),
            key_of(&fall, PlantMap::Foliage)
        );
        assert_eq!(
            key_of(&maple, PlantMap::Bark),
            key_of(&fall, PlantMap::Bark)
        );
    }
}

#[cfg(test)]
mod ground_previews {
    use super::*;

    /// Dev aid: writes the Base Ground picker's swatches (app/public/ground/<preset>.png,
    /// 256 px crops of each generated texture at 1024 px):
    /// `cargo test --release -p rufplan-studio write_ground_previews -- --ignored`.
    #[test]
    #[ignore]
    fn write_ground_previews() {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public/ground");
        std::fs::create_dir_all(&out).unwrap();
        for g in ground_library() {
            let Some(kind) = g.texture.as_deref().and_then(|t| t.strip_prefix("gen:")) else {
                continue;
            };
            let src = 1024;
            let tex = studio_views::texgen::generate(kind, src).unwrap();
            // A 256 px crop at full detail (2' of an 8' tile): blades and stones show.
            let n = 256;
            let mut px = vec![0u8; n * n * 3];
            for y in 0..n {
                let row = &tex.color[y * src * 3..y * src * 3 + n * 3];
                px[y * n * 3..(y + 1) * n * 3].copy_from_slice(row);
            }
            let bytes = encode(n, n, &px, png::ColorType::Rgb).unwrap();
            std::fs::write(out.join(format!("{}.png", g.id)), bytes).unwrap();
        }
    }
}
