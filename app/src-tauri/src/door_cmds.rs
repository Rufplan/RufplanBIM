//! IPC commands for the Door Library and the rendered type picker (ADR-033). Thin wrappers
//! over studio-core's door families and studio-views' thumbnails.

use serde::{Deserialize, Serialize};
use studio_core::doors::{self, DoorFinish, DoorSpec, DoorStyle, LeafStyle};
use studio_core::windows::{WindowSpec, WindowStyle};
use studio_core::{DoorFamily, ElementData, ElementId};
use studio_regen::OpeningKind;
use studio_views::thumbs::{thumb, OpeningThumb};
use studio_views::windows::WindowPreview;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::window_cmds::LoadedWindows;

type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DoorFamilyInfo {
    pub family: DoorFamily,
    pub label: String,
    pub description: String,
    /// Leaf styles it comes in, as (id, label); empty for glass and garage doors.
    pub leaves: Vec<LeafOption>,
    /// (min, max) of its panel count, when it has one.
    pub panels: Option<(u32, u32)>,
    pub panels_label: String,
    pub default_finish: DoorFinish,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LeafOption {
    pub id: LeafStyle,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DoorFinishOption {
    pub id: DoorFinish,
    pub label: String,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DoorPresetInfo {
    pub spec: DoorSpec,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DoorLibrary {
    pub families: Vec<DoorFamilyInfo>,
    pub presets: Vec<DoorPresetInfo>,
    pub finishes: Vec<DoorFinishOption>,
}

#[tauri::command]
pub fn door_library() -> DoorLibrary {
    DoorLibrary {
        families: doors::FAMILIES
            .iter()
            .map(|f| DoorFamilyInfo {
                family: f.family,
                label: f.label.into(),
                description: f.description.into(),
                leaves: f
                    .leaves
                    .iter()
                    .map(|l| LeafOption {
                        id: *l,
                        label: l.label().into(),
                    })
                    .collect(),
                panels: f.panels.map(|(lo, hi, _)| (lo, hi)),
                panels_label: f.panels_label.into(),
                default_finish: f.default_finish,
            })
            .collect(),
        presets: doors::CATALOG
            .iter()
            .map(|p| {
                let spec = DoorSpec::from(*p);
                DoorPresetInfo {
                    name: spec.name(),
                    spec,
                }
            })
            .collect(),
        finishes: DoorFinish::ALL
            .iter()
            .map(|f| DoorFinishOption {
                id: *f,
                label: f.label().into(),
                color: f.color(),
            })
            .collect(),
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DoorSpecPreview {
    pub name: String,
    pub preview: WindowPreview,
}

#[tauri::command]
pub fn door_preview(spec: DoorSpec) -> DoorSpecPreview {
    DoorSpecPreview {
        name: spec.name(),
        preview: studio_views::doors::preview(spec.style(), spec.width, spec.height),
    }
}

/// Loads door types (one undo step), reusing any already there by name.
#[tauri::command]
pub fn load_door_types(
    specs: Vec<DoorSpec>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<LoadedWindows> {
    let mut s = lock(&state)?;
    let ids = s.edit(|d| doors::load(d, &specs))?;
    let state = finish(&window, &s)?;
    Ok(LoadedWindows { state, ids })
}

/// What a thumbnail shows: a project type, or a door or window size to load.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub enum ThumbSource {
    Type(ElementId),
    Door(DoorSpec),
    Window(WindowSpec),
}

/// A door or window type as triangles in a short piece of wall, for its rendered thumbnail.
#[tauri::command]
pub fn opening_thumbnail(
    source: ThumbSource,
    state: State<'_, SessionState>,
) -> CommandResult<OpeningThumb> {
    let (kind, w, h) = match source {
        ThumbSource::Door(s) => (OpeningKind::Door(s.style()), s.width, s.height),
        ThumbSource::Window(s) => (OpeningKind::Window(s.style()), s.width, s.height),
        ThumbSource::Type(id) => {
            let s = lock(&state)?;
            let data = s.doc()?.data(id).map_err(anyhow::Error::from)?.clone();
            match &data {
                ElementData::DoorType { width, height, .. } => (
                    OpeningKind::Door(
                        DoorStyle::of(&data).ok_or_else(|| anyhow::anyhow!("not a door type"))?,
                    ),
                    *width,
                    *height,
                ),
                ElementData::WindowType { width, height, .. } => (
                    OpeningKind::Window(
                        WindowStyle::of(&data)
                            .ok_or_else(|| anyhow::anyhow!("not a window type"))?,
                    ),
                    *width,
                    *height,
                ),
                _ => return Err(anyhow::anyhow!("not a door or window type").into()),
            }
        }
    };
    Ok(thumb(kind, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_door_library_lists_families_and_sizes() {
        let lib = door_library();
        assert_eq!(lib.families.len(), doors::FAMILIES.len());
        assert_eq!(lib.presets.len(), doors::CATALOG.len());
        let garage = lib
            .families
            .iter()
            .find(|f| f.family == DoorFamily::Garage)
            .unwrap();
        assert!(garage.leaves.is_empty() && garage.panels.is_none());
        let p = door_preview(lib.presets[6].spec);
        assert_eq!(p.name, "Single Six-Panel 30\" x 80\"");
        assert!(p.preview.lines.len() > 6);
    }
}
