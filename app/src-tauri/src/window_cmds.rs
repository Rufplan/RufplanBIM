//! IPC commands for the Window Library (ADR-031). Thin wrappers over studio-core's window
//! families and studio-views' elevation thumbnails.

use serde::Serialize;
use studio_core::windows::{self, FrameFinish, Grille, WindowSpec, WindowStyle};
use studio_core::{ElementId, WindowFamily};
use studio_views::windows::{preview, WindowPreview};
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WindowFamilyInfo {
    pub family: WindowFamily,
    pub label: String,
    pub description: String,
    pub mullable: bool,
    pub grilles: bool,
}

/// A standard size: its spec (mm, no grille, white) and the type name it loads as.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WindowPresetInfo {
    pub spec: WindowSpec,
    pub name: String,
    pub preview: WindowPreview,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GrilleOption {
    pub id: Grille,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FinishOption {
    pub id: FrameFinish,
    pub label: String,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct WindowLibrary {
    pub families: Vec<WindowFamilyInfo>,
    pub presets: Vec<WindowPresetInfo>,
    pub grilles: Vec<GrilleOption>,
    pub finishes: Vec<FinishOption>,
}

/// The families and their standard sizes, for the Window Library.
#[tauri::command]
pub fn window_library() -> WindowLibrary {
    WindowLibrary {
        families: windows::FAMILIES
            .iter()
            .map(|f| WindowFamilyInfo {
                family: f.family,
                label: f.label.into(),
                description: f.description.into(),
                mullable: f.mullable,
                grilles: f.grilles,
            })
            .collect(),
        presets: windows::CATALOG
            .iter()
            .map(|p| {
                let spec = WindowSpec::from(*p);
                WindowPresetInfo {
                    name: spec.name(),
                    preview: preview(spec.style(), spec.width, spec.height),
                    spec,
                }
            })
            .collect(),
        grilles: Grille::ALL
            .iter()
            .map(|g| GrilleOption {
                id: *g,
                label: g.label().into(),
            })
            .collect(),
        finishes: FrameFinish::ALL
            .iter()
            .map(|f| FinishOption {
                id: *f,
                label: f.label().into(),
                color: f.color(),
            })
            .collect(),
    }
}

/// A window's elevation thumbnail and the name it would load as.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SpecPreview {
    pub name: String,
    pub preview: WindowPreview,
}

#[tauri::command]
pub fn window_preview(spec: WindowSpec) -> SpecPreview {
    let style: WindowStyle = spec.style();
    SpecPreview {
        name: spec.name(),
        preview: preview(style, spec.width, spec.height),
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LoadedWindows {
    pub state: Option<AppState>,
    /// The type for each spec, in order.
    pub ids: Vec<ElementId>,
}

/// Loads window types into the project (one undo step), reusing any already there by name.
#[tauri::command]
pub fn load_window_types(
    specs: Vec<WindowSpec>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<LoadedWindows> {
    let mut s = lock(&state)?;
    let ids = s.edit(|d| windows::load(d, &specs))?;
    let state = finish(&window, &s)?;
    Ok(LoadedWindows { state, ids })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_lists_every_family_and_size() {
        let lib = window_library();
        assert_eq!(lib.families.len(), windows::FAMILIES.len());
        assert_eq!(lib.presets.len(), windows::CATALOG.len());
        assert!(lib.presets.iter().all(|p| p.preview.lines.len() >= 3));
        assert_eq!(lib.finishes.len(), 5);
        let p = window_preview(WindowSpec {
            grille: Grille::Colonial,
            ..lib.presets[6].spec
        });
        assert_eq!(p.name, "Double Hung 36\" x 60\" - Colonial");
        // Colonial bars add lines to the plain preview.
        assert!(p.preview.lines.len() > lib.presets[6].preview.lines.len());
    }
}
