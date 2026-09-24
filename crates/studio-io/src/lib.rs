//! .rfproj persistence (SQLite) and IFC4 export.
//!
//! A project file is a SQLite database. M0 stores only the `meta` table; element tables
//! arrive with M1 as a new migration.
//!
//! Save strategy: the whole project is written to `<file>.tmp` inside one SQLite
//! transaction, then renamed over the target. A crash mid-save never leaves a
//! half-written project in place of the last good one.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};

/// File extension for project files, without the dot.
pub const EXTENSION: &str = "rfproj";

/// Schema version written by this build. Files with a higher version are refused.
pub const SCHEMA_VERSION: i64 = 1;

/// Forward-only migrations. Index `i` upgrades schema `i` to `i + 1`.
const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_init.sql")];

const KEY_SCHEMA_VERSION: &str = "schema_version";
const KEY_APP_VERSION: &str = "app_version";

/// Errors from reading or writing project files.
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("{path} is not a Rufplan Studio project")]
    NotAProject { path: PathBuf },
    #[error(
        "project was saved by a newer version (schema {found}, this build supports {supported})"
    )]
    NewerSchema { found: i64, supported: i64 },
    #[error("project metadata is invalid: {key} = {value:?}")]
    InvalidMeta { key: &'static str, value: String },
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, IoError>;

/// Project-level metadata stored in the `meta` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMeta {
    /// Schema version of the file as read; always [`SCHEMA_VERSION`] after a save.
    pub schema_version: i64,
    /// Version of the app that last saved the file.
    pub app_version: String,
}

/// An open project held in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub meta: ProjectMeta,
}

impl Project {
    /// A new, empty, unsaved project.
    pub fn new(app_version: &str) -> Self {
        Self {
            meta: ProjectMeta {
                schema_version: SCHEMA_VERSION,
                app_version: app_version.to_owned(),
            },
        }
    }

    /// Reads a project file into memory. The file is opened read-only and closed on return.
    pub fn open(path: &Path) -> Result<Self> {
        let not_a_project = || IoError::NotAProject {
            path: path.to_owned(),
        };
        if !path.is_file() {
            return Err(not_a_project());
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        let has_meta: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _)
                    if f.code == rusqlite::ErrorCode::NotADatabase =>
                {
                    not_a_project()
                }
                other => IoError::Sqlite(other),
            })?;
        if has_meta.is_none() {
            return Err(not_a_project());
        }

        let raw_version = read_meta(&conn, KEY_SCHEMA_VERSION)?.ok_or_else(not_a_project)?;
        let schema_version: i64 = raw_version.parse().map_err(|_| IoError::InvalidMeta {
            key: KEY_SCHEMA_VERSION,
            value: raw_version.clone(),
        })?;
        if schema_version > SCHEMA_VERSION {
            return Err(IoError::NewerSchema {
                found: schema_version,
                supported: SCHEMA_VERSION,
            });
        }
        if schema_version < 1 {
            return Err(IoError::InvalidMeta {
                key: KEY_SCHEMA_VERSION,
                value: raw_version,
            });
        }
        let app_version = read_meta(&conn, KEY_APP_VERSION)?.unwrap_or_default();

        Ok(Self {
            meta: ProjectMeta {
                schema_version,
                app_version,
            },
        })
    }

    /// Writes the project to `path` atomically (temp file + rename), stamping it with the
    /// current schema version and `app_version`.
    pub fn save(&mut self, path: &Path, app_version: &str) -> Result<()> {
        let tmp = temp_path(path);
        if tmp.exists() {
            std::fs::remove_file(&tmp)?;
        }
        let written = write_file(&tmp, app_version);
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        std::fs::rename(&tmp, path)?;
        self.meta.schema_version = SCHEMA_VERSION;
        self.meta.app_version = app_version.to_owned();
        Ok(())
    }
}

fn write_file(path: &Path, app_version: &str) -> Result<()> {
    let mut conn = Connection::open(path)?;
    let tx = conn.transaction()?;
    for sql in MIGRATIONS {
        tx.execute_batch(sql)?;
    }
    let mut put = tx.prepare("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)")?;
    put.execute((KEY_SCHEMA_VERSION, SCHEMA_VERSION.to_string()))?;
    put.execute((KEY_APP_VERSION, app_version))?;
    drop(put);
    tx.commit()?;
    conn.close().map_err(|(_, e)| IoError::Sqlite(e))?;
    Ok(())
}

fn read_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}

/// `project.rfproj` → `project.rfproj.tmp`, in the same directory so the rename is atomic.
fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

/// Version of this crate, from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join(format!("test.{EXTENSION}"))
    }

    #[test]
    fn crate_version_is_set() {
        assert!(!crate_version().is_empty());
    }

    #[test]
    fn new_save_open_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        let mut project = Project::new("0.0.1");
        project.save(&path, "0.0.1").unwrap();

        let reopened = Project::open(&path).unwrap();
        assert_eq!(reopened, project);
        assert_eq!(reopened.meta.schema_version, SCHEMA_VERSION);
        assert!(!temp_path(&path).exists(), "temp file must be renamed away");
    }

    #[test]
    fn save_overwrites_existing_file_and_updates_app_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        let mut project = Project::new("0.0.1");
        project.save(&path, "0.0.1").unwrap();
        project.save(&path, "0.0.2").unwrap();

        let reopened = Project::open(&path).unwrap();
        assert_eq!(reopened.meta.app_version, "0.0.2");
    }

    #[test]
    fn save_replaces_stale_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        std::fs::write(temp_path(&path), b"left over from a crash").unwrap();
        Project::new("0.0.1").save(&path, "0.0.1").unwrap();
        assert!(Project::open(&path).is_ok());
    }

    #[test]
    fn open_rejects_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        Project::new("0.0.1").save(&path, "0.0.1").unwrap();
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE meta SET value = ?1 WHERE key = 'schema_version'",
            [(SCHEMA_VERSION + 1).to_string()],
        )
        .unwrap();
        drop(conn);

        match Project::open(&path) {
            Err(IoError::NewerSchema { found, supported }) => {
                assert_eq!(found, SCHEMA_VERSION + 1);
                assert_eq!(supported, SCHEMA_VERSION);
            }
            other => panic!("expected NewerSchema, got {other:?}"),
        }
    }

    #[test]
    fn open_rejects_non_sqlite_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        std::fs::write(
            &path,
            b"this is not a database, just some text padding it out",
        )
        .unwrap();
        assert!(matches!(
            Project::open(&path),
            Err(IoError::NotAProject { .. })
        ));
    }

    #[test]
    fn open_rejects_sqlite_without_meta() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE other (x INTEGER)")
            .unwrap();
        assert!(matches!(
            Project::open(&path),
            Err(IoError::NotAProject { .. })
        ));
    }

    #[test]
    fn open_rejects_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            Project::open(&project_path(&dir)),
            Err(IoError::NotAProject { .. })
        ));
    }

    #[test]
    fn open_rejects_garbage_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        Project::new("0.0.1").save(&path, "0.0.1").unwrap();
        Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE meta SET value = 'abc' WHERE key = 'schema_version'",
                [],
            )
            .unwrap();
        assert!(matches!(
            Project::open(&path),
            Err(IoError::InvalidMeta { .. })
        ));
    }
}
