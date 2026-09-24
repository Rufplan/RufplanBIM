//! .rfproj persistence (SQLite) and IFC4 export.
//!
//! A project file is a SQLite database: a `meta` table plus one row per element, with the
//! element payload stored as MessagePack. Schema 1 (M0) had only `meta`; opening such a
//! file yields an empty document, and saving upgrades it.
//!
//! Save strategy: the whole project is written to `<file>.tmp` inside one SQLite
//! transaction, then renamed over the target. A crash mid-save never leaves a
//! half-written project in place of the last good one.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use studio_core::{Document, Element, ElementData, ElementId};

/// File extension for project files, without the dot.
pub const EXTENSION: &str = "rfproj";

/// Schema version written by this build. Files with a higher version are refused.
pub const SCHEMA_VERSION: i64 = 2;

/// Forward-only migrations. Index `i` upgrades schema `i` to `i + 1`.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_init.sql"),
    include_str!("../migrations/0002_elements.sql"),
];

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
    #[error("element {id} could not be read: {msg}")]
    BadElement { id: String, msg: String },
    #[error("element could not be written: {0}")]
    Encode(String),
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
#[derive(Debug, Clone)]
pub struct Project {
    pub meta: ProjectMeta,
    pub doc: Document,
}

impl Project {
    /// A new, unsaved project holding `doc`.
    pub fn new(app_version: &str, doc: Document) -> Self {
        Self {
            meta: ProjectMeta {
                schema_version: SCHEMA_VERSION,
                app_version: app_version.to_owned(),
            },
            doc,
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
        let doc = if schema_version >= 2 {
            Document::from_elements(read_elements(&conn)?)
        } else {
            Document::new()
        };

        Ok(Self {
            meta: ProjectMeta {
                schema_version,
                app_version,
            },
            doc,
        })
    }

    /// Writes the project to `path` atomically (temp file + rename), stamping it with the
    /// current schema version and `app_version`.
    pub fn save(&mut self, path: &Path, app_version: &str) -> Result<()> {
        let tmp = temp_path(path);
        if tmp.exists() {
            std::fs::remove_file(&tmp)?;
        }
        let written = write_file(&tmp, app_version, &self.doc);
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        std::fs::rename(&tmp, path)?;
        self.meta.schema_version = SCHEMA_VERSION;
        self.meta.app_version = app_version.to_owned();
        self.doc.mark_saved();
        Ok(())
    }
}

fn write_file(path: &Path, app_version: &str, doc: &Document) -> Result<()> {
    let mut conn = Connection::open(path)?;
    let tx = conn.transaction()?;
    for sql in MIGRATIONS {
        tx.execute_batch(sql)?;
    }
    let mut put = tx.prepare("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)")?;
    put.execute((KEY_SCHEMA_VERSION, SCHEMA_VERSION.to_string()))?;
    put.execute((KEY_APP_VERSION, app_version))?;
    drop(put);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    let empty_params =
        rmp_serde::to_vec_named(&std::collections::BTreeMap::<String, String>::new())
            .map_err(|e| IoError::Encode(e.to_string()))?;
    let mut ins = tx.prepare(
        "INSERT INTO elements (id, category, type_id, level_id, data, params, rev, modified_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for el in doc.iter() {
        let data = rmp_serde::to_vec_named(&el.data).map_err(|e| IoError::Encode(e.to_string()))?;
        ins.execute((
            el.id.as_bytes().as_slice(),
            el.category().as_str(),
            el.data.type_id().map(|t| t.as_bytes().to_vec()),
            el.data.level().map(|l| l.as_bytes().to_vec()),
            data,
            &empty_params,
            el.rev as i64,
            now,
        ))?;
    }
    drop(ins);
    tx.commit()?;
    conn.close().map_err(|(_, e)| IoError::Sqlite(e))?;
    Ok(())
}

fn read_elements(conn: &Connection) -> Result<Vec<Element>> {
    let mut q = conn.prepare("SELECT id, data, rev FROM elements")?;
    let rows = q.query_map([], |r| {
        Ok((
            r.get::<_, Vec<u8>>(0)?,
            r.get::<_, Vec<u8>>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let mut out = vec![];
    for row in rows {
        let (id, data, rev) = row?;
        let bytes: [u8; 16] = id.as_slice().try_into().map_err(|_| IoError::BadElement {
            id: format!("{id:?}"),
            msg: "id is not 16 bytes".into(),
        })?;
        let id = ElementId::from_bytes(bytes);
        let data: ElementData = rmp_serde::from_slice(&data).map_err(|e| IoError::BadElement {
            id: id.to_string(),
            msg: e.to_string(),
        })?;
        out.push(Element {
            id,
            rev: rev.max(1) as u64,
            data,
        });
    }
    Ok(out)
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
        let mut project = Project::new("0.0.1", Document::new());
        project.save(&path, "0.0.1").unwrap();

        let reopened = Project::open(&path).unwrap();
        assert_eq!(reopened.meta, project.meta);
        assert_eq!(reopened.meta.schema_version, SCHEMA_VERSION);
        assert!(!temp_path(&path).exists(), "temp file must be renamed away");
    }

    #[test]
    fn save_overwrites_existing_file_and_updates_app_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        let mut project = Project::new("0.0.1", Document::new());
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
        Project::new("0.0.1", Document::new())
            .save(&path, "0.0.1")
            .unwrap();
        assert!(Project::open(&path).is_ok());
    }

    #[test]
    fn open_rejects_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        Project::new("0.0.1", Document::new())
            .save(&path, "0.0.1")
            .unwrap();
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
    fn elements_round_trip_exactly() {
        use studio_core::{ops, Category};
        use studio_geom::Pt;
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(1234.5, 678.9)).unwrap();
        ops::set_property(&mut doc, w, "base_offset", "6\"", 0).unwrap();
        ops::create_grid(&mut doc, Pt::new(0.0, -1000.0), Pt::new(0.0, 5000.0)).unwrap();
        let mut project = Project::new("0.0.1", doc);
        project.save(&path, "0.0.1").unwrap();
        assert!(!project.doc.is_dirty());

        let reopened = Project::open(&path).unwrap();
        let a: Vec<_> = project.doc.iter().cloned().collect();
        let b: Vec<_> = reopened.doc.iter().cloned().collect();
        assert_eq!(a, b);
        assert_eq!(reopened.doc.get(w).unwrap().rev, 2);
    }

    #[test]
    fn schema_1_file_opens_empty_and_upgrades_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.execute(
            "INSERT INTO meta VALUES ('schema_version', '1'), ('app_version', '0.0.1')",
            [],
        )
        .unwrap();
        drop(conn);
        let mut p = Project::open(&path).unwrap();
        assert_eq!(p.meta.schema_version, 1);
        assert!(p.doc.is_empty());
        p.save(&path, "0.0.2").unwrap();
        assert_eq!(
            Project::open(&path).unwrap().meta.schema_version,
            SCHEMA_VERSION
        );
    }

    #[test]
    fn open_rejects_garbage_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_path(&dir);
        Project::new("0.0.1", Document::new())
            .save(&path, "0.0.1")
            .unwrap();
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
