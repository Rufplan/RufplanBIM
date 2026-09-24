//! The open project and where it lives on disk. Kept free of Tauri types so it can be
//! unit-tested directly.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde::Serialize;
use studio_io::Project;
use ts_rs::TS;

/// What the UI needs to show about the open project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectStatus {
    /// File name without extension, or "Untitled" before the first save.
    pub name: String,
    /// Absolute path, or `null` before the first save.
    pub path: Option<String>,
    #[ts(type = "number")]
    pub schema_version: i64,
    /// App version that last saved the file.
    pub app_version: String,
}

#[derive(Debug, Default)]
pub struct Session {
    project: Option<Project>,
    path: Option<PathBuf>,
}

impl Session {
    pub fn status(&self) -> Option<ProjectStatus> {
        let project = self.project.as_ref()?;
        let name = self.path.as_deref().and_then(Path::file_stem).map_or_else(
            || "Untitled".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        Some(ProjectStatus {
            name,
            path: self.path.as_ref().map(|p| p.display().to_string()),
            schema_version: project.meta.schema_version,
            app_version: project.meta.app_version.clone(),
        })
    }

    /// Replaces the open project with a new, unsaved one.
    pub fn new_project(&mut self, app_version: &str) {
        self.project = Some(Project::new(app_version));
        self.path = None;
    }

    /// Opens `path`. On failure the currently open project is left untouched.
    pub fn open(&mut self, path: &Path) -> anyhow::Result<()> {
        let project =
            Project::open(path).with_context(|| format!("could not open {}", path.display()))?;
        self.project = Some(project);
        self.path = Some(path.to_owned());
        Ok(())
    }

    /// Saves to `path` (Save As) or to the current path (Save). Adds the `.rfproj`
    /// extension if missing.
    pub fn save(&mut self, path: Option<&Path>, app_version: &str) -> anyhow::Result<()> {
        let Some(project) = self.project.as_mut() else {
            bail!("no project is open");
        };
        let target = match (path, &self.path) {
            (Some(p), _) => with_project_extension(p),
            (None, Some(current)) => current.clone(),
            (None, None) => bail!("choose where to save the project first"),
        };
        project
            .save(&target, app_version)
            .with_context(|| format!("could not save {}", target.display()))?;
        self.path = Some(target);
        Ok(())
    }
}

fn with_project_extension(path: &Path) -> PathBuf {
    let has_ext = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(studio_io::EXTENSION));
    if has_ext {
        return path.to_owned();
    }
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(studio_io::EXTENSION);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_no_project() {
        assert_eq!(Session::default().status(), None);
    }

    #[test]
    fn new_project_is_untitled() {
        let mut s = Session::default();
        s.new_project("0.0.1");
        let status = s.status().unwrap();
        assert_eq!(status.name, "Untitled");
        assert_eq!(status.path, None);
    }

    #[test]
    fn save_as_then_save_then_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::default();
        s.new_project("0.0.1");
        assert!(s.save(None, "0.0.1").is_err(), "untitled needs a path");

        s.save(Some(&dir.path().join("House")), "0.0.1").unwrap();
        let expected = dir.path().join("House.rfproj");
        assert!(expected.is_file());
        assert_eq!(s.status().unwrap().name, "House");

        s.save(None, "0.0.2").unwrap();
        let mut other = Session::default();
        other.open(&expected).unwrap();
        assert_eq!(other.status().unwrap().app_version, "0.0.2");
    }

    #[test]
    fn failed_open_keeps_current_project() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::default();
        s.new_project("0.0.1");
        assert!(s.open(&dir.path().join("missing.rfproj")).is_err());
        assert_eq!(s.status().unwrap().name, "Untitled");
    }

    #[test]
    fn extension_is_added_only_when_missing() {
        assert_eq!(
            with_project_extension(Path::new("a/My.Project")),
            PathBuf::from("a/My.Project.rfproj")
        );
        assert_eq!(
            with_project_extension(Path::new("a/b.RFPROJ")),
            PathBuf::from("a/b.RFPROJ")
        );
    }
}
