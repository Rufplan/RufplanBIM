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

/// Paints elements with a material (ADR-034), or removes their paint with none.
#[tauri::command]
pub fn paint_elements(
    ids: Vec<ElementId>,
    material: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    s.edit(|d| studio_core::paint::paint(d, &ids, material))?;
    finish(&window, &s)
}

/// Paints the face of `el` at a 3D hit `point` with face `normal` (z-up mm), as Revit's Paint
/// tool (ADR-096): only that face, the assembly unchanged. Columns and beams paint whole.
#[tauri::command]
pub fn paint_face(
    el: ElementId,
    point: [f64; 3],
    normal: [f64; 3],
    material: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    let face = {
        let m = studio_regen::regenerate(s.doc()?);
        studio_views::faces::Shape::of(&m, el).and_then(|sh| sh.face(point, normal))
    };
    paint_one(&mut s, el, face, material)?;
    finish(&window, &s)
}

/// Paints the face of `el` seen at `point` in a plan, elevation or section (ADR-096).
#[tauri::command]
pub fn paint_in_view(
    view: ElementId,
    el: ElementId,
    point: studio_geom::Pt,
    material: Option<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    let face = studio_views::faces::face_in_view(s.doc()?, view, el, point);
    paint_one(&mut s, el, face, material)?;
    finish(&window, &s)
}

fn paint_one(
    s: &mut crate::session::Session,
    el: ElementId,
    face: Option<String>,
    material: Option<ElementId>,
) -> anyhow::Result<()> {
    match face {
        Some(f) => s.edit(|d| studio_core::paint::paint_face(d, el, &f, material)),
        None => s.edit(|d| studio_core::paint::paint(d, &[el], material).map(|_| ())),
    }
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
    let cache = app
        .path()
        .app_local_data_dir()
        .map_err(anyhow::Error::from)?
        .join("textures");
    // Generated sets (ADR-061): made here at 4096 px on first use, then kept as PNG.
    if let Some(kind) = set.strip_prefix("gen:") {
        let kind = kind.to_owned();
        let bytes = tauri::async_runtime::spawn_blocking(move || {
            generated_map(&cache.join("generated"), &kind, map, GEN_SIZE)
        })
        .await
        .map_err(anyhow::Error::from)??;
        return Ok(tauri::ipc::Response::new(bytes));
    }
    let url = library::texture_url(&set, map)
        .ok_or_else(|| anyhow::anyhow!("{set} is not a library texture"))?;
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        studio_sync::textures::texture_map(&studio_sync::UreqHttp::default(), &cache, &set, &url)
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(anyhow::Error::from)?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// Generated textures' resolution (ADR-061).
const GEN_SIZE: usize = 4096;

/// One at a time: a render asks for a set's three maps at once, and it's made once.
static GENERATING: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn map_name(map: TextureMap) -> &'static str {
    match map {
        TextureMap::Color => "color",
        TextureMap::Normal => "normal",
        TextureMap::Roughness => "rough",
    }
}

/// A generated set's map as PNG bytes, from `dir` or made and written there first (all
/// three maps, each written whole under a temporary name, then renamed).
pub(crate) fn generated_map(
    dir: &std::path::Path,
    kind: &str,
    map: TextureMap,
    size: usize,
) -> anyhow::Result<Vec<u8>> {
    if studio_views::texgen::tile_of(kind).is_none() {
        anyhow::bail!("no generated texture {kind}");
    }
    let path = |m: TextureMap| dir.join(format!("{kind}-{size}-{}.png", map_name(m)));
    let _lock = GENERATING
        .lock()
        .map_err(|_| anyhow::anyhow!("texture generation failed"))?;
    if let Ok(b) = std::fs::read(path(map)) {
        return Ok(b);
    }
    let g = studio_views::texgen::generate(kind, size)
        .ok_or_else(|| anyhow::anyhow!("no generated texture {kind}"))?;
    std::fs::create_dir_all(dir)?;
    let mut wanted = vec![];
    for (m, data) in [
        (TextureMap::Color, &g.color),
        (TextureMap::Normal, &g.normal),
        (TextureMap::Roughness, &g.rough),
    ] {
        let bytes = encode_png(size, data)?;
        let tmp = path(m).with_extension("part");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, path(m))?;
        if m == map {
            wanted = bytes;
        }
    }
    Ok(wanted)
}

fn encode_png(size: usize, rgb: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut out = vec![];
    let mut enc = png::Encoder::new(&mut out, size as u32, size as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    let mut w = enc.write_header()?;
    w.write_image_data(rgb)?;
    w.finish()?;
    Ok(out)
}

#[cfg(test)]
mod gen_tests {
    use super::*;

    #[test]
    fn a_generated_set_is_made_once_and_cached_as_png() {
        let dir = tempfile::tempdir().unwrap();
        let a = generated_map(dir.path(), "seam16", TextureMap::Normal, 64).unwrap();
        assert_eq!(&a[..8], b"\x89PNG\r\n\x1a\n");
        // All three maps were written.
        for m in ["color", "normal", "rough"] {
            assert!(
                dir.path().join(format!("seam16-64-{m}.png")).exists(),
                "{m}"
            );
        }
        // The next ask reads the file back.
        let b = generated_map(dir.path(), "seam16", TextureMap::Normal, 64).unwrap();
        assert_eq!(a, b);
        assert!(generated_map(dir.path(), "../etc", TextureMap::Color, 64).is_err());
    }
}
