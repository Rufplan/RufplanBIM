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
