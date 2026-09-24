//! IPC commands. Payload types derive `TS` so `cargo test` regenerates
//! `app/src/bindings/*.ts` and the TypeScript side stays in sync.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::session::{ProjectStatus, Session};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "Rufplan Studio";

pub type SessionState = Mutex<Session>;

/// Versions of the running app and its core crates.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CoreVersion {
    pub app: String,
    pub core: String,
    pub io: String,
    /// Project file schema version this build writes.
    #[ts(type = "number")]
    pub schema_version: i64,
}

/// Error returned to the UI by any failing command.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CommandError {
    pub message: String,
}

impl From<anyhow::Error> for CommandError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            message: format!("{err:#}"),
        }
    }
}

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub fn core_version() -> CoreVersion {
    CoreVersion {
        app: APP_VERSION.to_owned(),
        core: studio_core::crate_version().to_owned(),
        io: studio_io::crate_version().to_owned(),
        schema_version: studio_io::SCHEMA_VERSION,
    }
}

#[tauri::command]
pub fn project_status(state: State<'_, SessionState>) -> CommandResult<Option<ProjectStatus>> {
    Ok(lock(&state)?.status())
}

#[tauri::command]
pub fn project_new(
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<ProjectStatus>> {
    let mut session = lock(&state)?;
    session.new_project(APP_VERSION);
    finish(&window, &session)
}

#[tauri::command]
pub fn project_open(
    path: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<ProjectStatus>> {
    let mut session = lock(&state)?;
    session.open(&PathBuf::from(path))?;
    finish(&window, &session)
}

/// Saves to `path` when given (Save As), otherwise to the project's current path.
#[tauri::command]
pub fn project_save(
    path: Option<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<ProjectStatus>> {
    let mut session = lock(&state)?;
    session.save(path.map(PathBuf::from).as_deref(), APP_VERSION)?;
    finish(&window, &session)
}

fn lock<'a>(
    state: &'a State<'_, SessionState>,
) -> CommandResult<std::sync::MutexGuard<'a, Session>> {
    state
        .lock()
        .map_err(|_| anyhow::anyhow!("project state is unavailable after an earlier crash").into())
}

/// Updates the window title and returns the new status.
fn finish(window: &WebviewWindow, session: &Session) -> CommandResult<Option<ProjectStatus>> {
    let status = session.status();
    let title = match &status {
        Some(s) => format!("{} — {APP_NAME}", s.name),
        None => APP_NAME.to_owned(),
    };
    window.set_title(&title).map_err(anyhow::Error::from)?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_version_reports_schema() {
        let v = core_version();
        assert_eq!(v.schema_version, studio_io::SCHEMA_VERSION);
        assert!(!v.app.is_empty());
    }

    #[test]
    fn command_error_keeps_context_chain() {
        let err = anyhow::anyhow!("root cause").context("could not open x");
        assert_eq!(
            CommandError::from(err).message,
            "could not open x: root cause"
        );
    }
}
