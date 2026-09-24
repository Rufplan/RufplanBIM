//! IPC commands. Payload types derive `TS` so `cargo test` regenerates
//! `app/src/bindings/*.ts` and the TypeScript side stays in sync.

use serde::Serialize;
use ts_rs::TS;

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

impl CoreVersion {
    fn current() -> Self {
        Self {
            app: env!("CARGO_PKG_VERSION").to_owned(),
            core: studio_core::crate_version().to_owned(),
            io: studio_io::crate_version().to_owned(),
            schema_version: studio_io::SCHEMA_VERSION,
        }
    }
}

#[tauri::command]
pub fn core_version() -> CoreVersion {
    CoreVersion::current()
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
}
