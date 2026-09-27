//! IPC for the Standards tab (ADR-047): thin wrappers over studio-core's `standards`.

use studio_core::standards::{self, Standards, LIBRARIES};
use tauri::{State, WebviewWindow};

use crate::commands::{edit, lock, CommandError, SessionState, StateResult};

type CommandResult<T> = Result<T, CommandError>;

/// The project's standards (the default library until one is edited).
#[tauri::command]
pub fn standards_get(state: State<'_, SessionState>) -> CommandResult<Standards> {
    let session = lock(&state)?;
    Ok(standards::standards(session.doc()?))
}

/// The office libraries the Library menu offers.
#[tauri::command]
pub fn standards_libraries() -> Vec<String> {
    LIBRARIES.iter().map(|s| (*s).to_owned()).collect()
}

/// Edits one standard's value and/or status.
#[tauri::command]
pub fn standards_set(
    category: String,
    index: usize,
    value: Option<String>,
    done: Option<bool>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| standards::set_standard(d, &category, index, value.as_deref(), done))
    })
}

/// Loads a library's values into the project's standards.
#[tauri::command]
pub fn standards_load_library(
    name: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| standards::load_library(d, &name))
    })
}

/// One way a standard can be set, as the Standards pop-up shows it (ADR-048).
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct StandardChoice {
    pub label: String,
    pub detail: String,
    /// What it looks like, for graphic standards (paper mm).
    pub preview: Option<studio_views::DisplayList>,
}

/// The choices for one standard, with a preview drawing of each graphic one.
#[tauri::command]
pub fn standards_choices(category: String, index: usize) -> CommandResult<Vec<StandardChoice>> {
    let s = standards::library(LIBRARIES[0]).map_err(CommandError::from)?;
    let item = s
        .categories
        .iter()
        .find(|c| c.id == category)
        .and_then(|c| c.items.get(index))
        .ok_or_else(|| {
            CommandError::from(studio_core::CoreError::Invalid("no such standard".into()))
        })?;
    Ok(
        studio_core::standards_catalog::choices(&category, &item.name)
            .into_iter()
            .enumerate()
            .map(|(i, c)| StandardChoice {
                label: c.label.into(),
                detail: c.detail.into(),
                preview: studio_views::standards_preview::preview(&category, &item.name, i),
            })
            .collect(),
    )
}
