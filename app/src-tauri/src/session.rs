//! The open project and where it lives on disk, plus the UI snapshot built from it.
//! Kept free of Tauri types so it can be unit-tested directly.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde::Serialize;
use studio_core::{ops, Category, Document, ElementData, ElementId, ViewKind};
use studio_io::Project;
use studio_views::ViewType;
use ts_rs::TS;

/// What the UI needs to show about the open project file.
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
    /// Unsaved changes exist.
    pub dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ViewInfo {
    pub id: ElementId,
    pub name: String,
    pub view_type: ViewType,
    pub scale: u32,
    pub scale_label: String,
    pub level: Option<ElementId>,
    /// Sheet this view is placed on, if any (schedules may be on several; this is the first).
    pub on_sheet: Option<ElementId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct NamedItem {
    pub id: ElementId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct StageItem {
    pub id: ElementId,
    pub name: String,
    pub abbreviation: String,
}

/// Everything the UI shows outside the drawing canvases. Returned after every change.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppState {
    pub project: ProjectStatus,
    /// Bumps on every model change; canvases refetch when it changes.
    #[ts(type = "number")]
    pub revision: u64,
    pub views: Vec<ViewInfo>,
    pub levels: Vec<NamedItem>,
    pub wall_types: Vec<NamedItem>,
    pub floor_types: Vec<NamedItem>,
    pub ceiling_types: Vec<NamedItem>,
    pub door_types: Vec<NamedItem>,
    pub window_types: Vec<NamedItem>,
    pub stages: Vec<StageItem>,
    pub current_stage: Option<ElementId>,
    pub project_info: Option<ElementId>,
    pub project_name: String,
    pub undo: Option<String>,
    pub redo: Option<String>,
}

#[derive(Debug, Default)]
pub struct Session {
    project: Option<Project>,
    path: Option<PathBuf>,
    revision: u64,
}

impl Session {
    pub fn doc(&self) -> anyhow::Result<&Document> {
        self.project
            .as_ref()
            .map(|p| &p.doc)
            .context("no project is open")
    }

    /// Runs a model edit and bumps the revision.
    pub fn edit<T>(
        &mut self,
        f: impl FnOnce(&mut Document) -> studio_core::CoreResult<T>,
    ) -> anyhow::Result<T> {
        let p = self.project.as_mut().context("no project is open")?;
        let v = f(&mut p.doc)?;
        self.revision += 1;
        Ok(v)
    }

    pub fn is_dirty(&self) -> bool {
        self.project
            .as_ref()
            .is_some_and(|p| p.doc.is_dirty() || self.path.is_none())
    }

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
            dirty: project.doc.is_dirty(),
        })
    }

    pub fn state(&self) -> Option<AppState> {
        let project = self.status()?;
        let doc = self.doc().ok()?;
        let named = |cat: Category| -> Vec<NamedItem> {
            let mut v: Vec<NamedItem> = doc
                .of(cat)
                .map(|e| NamedItem {
                    id: e.id,
                    name: e.data.name(),
                })
                .collect();
            v.sort_by(|a, b| ops::natural_cmp(&a.name, &b.name));
            v
        };
        let levels = doc.levels();
        let mut placed = std::collections::HashMap::new();
        for e in doc.of(Category::Viewport) {
            if let ElementData::Viewport { sheet, view, .. } = &e.data {
                placed.entry(*view).or_insert(*sheet);
            }
        }
        let mut views: Vec<ViewInfo> = doc
            .iter()
            .filter(|e| matches!(e.category(), Category::View | Category::Sheet))
            .filter_map(|e| match &e.data {
                ElementData::View { name, kind, scale } => {
                    let (view_type, level) = match kind {
                        ViewKind::FloorPlan { level } => (ViewType::Plan, Some(*level)),
                        ViewKind::CeilingPlan { level } => (ViewType::CeilingPlan, Some(*level)),
                        ViewKind::Elevation { .. } => (ViewType::Elevation, None),
                        ViewKind::ThreeD => (ViewType::ThreeD, None),
                        ViewKind::Section { .. } => (ViewType::Section, None),
                        ViewKind::Schedule { .. } => (ViewType::Schedule, None),
                    };
                    Some(ViewInfo {
                        id: e.id,
                        name: name.clone(),
                        view_type,
                        scale: *scale,
                        scale_label: if matches!(kind, ViewKind::Schedule { .. }) {
                            String::new()
                        } else {
                            ops::scale_label(*scale)
                        },
                        level,
                        on_sheet: placed.get(&e.id).copied(),
                    })
                }
                ElementData::Sheet { .. } => Some(ViewInfo {
                    id: e.id,
                    name: e.data.name(),
                    view_type: ViewType::Sheet,
                    scale: 1,
                    scale_label: String::new(),
                    level: None,
                    on_sheet: None,
                }),
                _ => None,
            })
            .collect();
        let level_order = |l: Option<ElementId>| {
            l.and_then(|l| levels.iter().position(|x| x.0 == l))
                .unwrap_or(usize::MAX)
        };
        views.sort_by(|a, b| {
            (a.view_type as u8, level_order(a.level), &a.name).cmp(&(
                b.view_type as u8,
                level_order(b.level),
                &b.name,
            ))
        });
        let info = ops::project_info(doc);
        let (current_stage, project_name) = match info.and_then(|i| doc.data(i).ok()) {
            Some(ElementData::ProjectInfo {
                current_stage,
                name,
                ..
            }) => (*current_stage, name.clone()),
            _ => (None, String::new()),
        };
        Some(AppState {
            project,
            revision: self.revision,
            views,
            levels: levels
                .into_iter()
                .map(|(id, name, _)| NamedItem { id, name })
                .collect(),
            wall_types: named(Category::WallType),
            floor_types: named(Category::FloorType),
            ceiling_types: named(Category::CeilingType),
            door_types: named(Category::DoorType),
            window_types: named(Category::WindowType),
            stages: ops::stages(doc)
                .into_iter()
                .map(|(id, name, abbreviation)| StageItem {
                    id,
                    name,
                    abbreviation,
                })
                .collect(),
            current_stage,
            project_info: info,
            project_name,
            undo: doc.can_undo().map(str::to_owned),
            redo: doc.can_redo().map(str::to_owned),
        })
    }

    /// Replaces the open project with a new, unsaved one seeded with defaults.
    pub fn new_project(&mut self, app_version: &str) -> anyhow::Result<()> {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc)?;
        doc.clear_history();
        doc.mark_saved();
        self.project = Some(Project::new(app_version, doc));
        self.path = None;
        self.revision += 1;
        Ok(())
    }

    /// A new, unsaved sample project: a two-storey 40' × 30' house with grids, interior
    /// walls, floors and ceilings, for trying the tools.
    pub fn new_sample(&mut self, app_version: &str) -> anyhow::Result<()> {
        self.new_project(app_version)?;
        let p = self.project.as_mut().context("no project is open")?;
        build_sample(&mut p.doc)?;
        if let Some(info) = ops::project_info(&p.doc) {
            ops::set_property(&mut p.doc, info, "name", "Sample House", 0)?;
        }
        p.doc.clear_history();
        p.doc.mark_saved();
        self.revision += 1;
        Ok(())
    }

    /// Opens `path`. On failure the currently open project is left untouched.
    pub fn open(&mut self, path: &Path) -> anyhow::Result<()> {
        let mut project =
            Project::open(path).with_context(|| format!("could not open {}", path.display()))?;
        if project.doc.of(Category::View).next().is_none() {
            // Files from before element storage (schema 1) have no content yet.
            ops::seed_default_project(&mut project.doc)?;
            project.doc.clear_history();
        } else if project.doc.of(Category::DoorType).next().is_none() {
            // Saved before doors and windows existed: add the built-in types. They are
            // written on the next save; there is nothing for the user to review.
            ops::ensure_opening_types(&mut project.doc)?;
            project.doc.clear_history();
            project.doc.mark_saved();
        }
        if !project.doc.iter().any(|e| {
            matches!(
                &e.data,
                ElementData::View {
                    kind: ViewKind::Schedule { .. },
                    ..
                }
            )
        }) {
            // Saved before schedules existed.
            let dirty = project.doc.is_dirty();
            ops::ensure_schedules(&mut project.doc)?;
            project.doc.clear_history();
            if !dirty {
                project.doc.mark_saved();
            }
        }
        self.project = Some(project);
        self.path = Some(path.to_owned());
        self.revision += 1;
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
        self.revision += 1;
        Ok(())
    }

    /// Level of a plan view.
    pub fn view_level(&self, view: ElementId) -> anyhow::Result<ElementId> {
        match self.doc()?.data(view)? {
            ElementData::View {
                kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
                ..
            } => Ok(*level),
            _ => bail!("switch to a floor or ceiling plan to draw this"),
        }
    }
}

fn build_sample(doc: &mut Document) -> anyhow::Result<()> {
    use studio_core::units::MM_PER_FT;
    use studio_geom::Pt;
    let ft = |x: f64, y: f64| Pt::new(x * MM_PER_FT, y * MM_PER_FT);
    let levels = doc.levels();
    let (l1, l2) = (levels[0].0, levels[1].0);
    let wall_type = |doc: &Document, prefix: &str| {
        doc.of(Category::WallType)
            .find(|e| e.data.name().starts_with(prefix))
            .map(|e| e.id)
            .context("missing wall type")
    };
    let ext = wall_type(doc, "Exterior - 8")?;
    let int = wall_type(doc, "Interior - 4")?;

    // Grids 1–3 run north–south, A–C east–west.
    for x in [0.0, 16.0, 40.0] {
        ops::create_grid(doc, ft(x, -6.0), ft(x, 36.0))?;
    }
    let first = ops::create_grid(doc, ft(-6.0, 0.0), ft(46.0, 0.0))?;
    ops::set_property(doc, first, "name", "A", 0)?;
    for y in [12.0, 30.0] {
        ops::create_grid(doc, ft(-6.0, y), ft(46.0, y))?;
    }

    let corners = [ft(0.0, 0.0), ft(40.0, 0.0), ft(40.0, 30.0), ft(0.0, 30.0)];
    for level in [l1, l2] {
        for i in 0..4 {
            ops::create_wall(doc, ext, level, corners[i], corners[(i + 1) % 4])?;
        }
    }
    ops::create_wall(doc, int, l1, ft(16.0, 0.0), ft(16.0, 30.0))?;
    ops::create_wall(doc, int, l1, ft(16.0, 12.0), ft(40.0, 12.0))?;
    ops::create_wall(doc, int, l2, ft(24.0, 0.0), ft(24.0, 30.0))?;

    // Doors and windows (offsets are distances from each wall's start to the center).
    let named_type = |doc: &Document, cat: Category, prefix: &str| {
        doc.of(cat)
            .find(|e| e.data.name().starts_with(prefix))
            .map(|e| e.id)
            .context("missing door or window type")
    };
    let entry = named_type(doc, Category::DoorType, "Double Flush")?;
    let door = named_type(doc, Category::DoorType, "Single Flush 36")?;
    let small = named_type(doc, Category::DoorType, "Single Flush 30")?;
    let wide = named_type(doc, Category::WindowType, "Fixed 72")?;
    let casement = named_type(doc, Category::WindowType, "Casement")?;
    let wall_on = |doc: &Document, level: ElementId, a: Pt, b: Pt| {
        doc.of(Category::Wall)
            .find(|e| {
                matches!(&e.data, ElementData::Wall { start, end, base_level, .. }
                if *base_level == level && start.dist(a) < 1.0 && end.dist(b) < 1.0)
            })
            .map(|e| e.id)
            .context("missing sample wall")
    };
    let south1 = wall_on(doc, l1, corners[0], corners[1])?;
    let east1 = wall_on(doc, l1, corners[1], corners[2])?;
    let north1 = wall_on(doc, l1, corners[2], corners[3])?;
    let west1 = wall_on(doc, l1, corners[3], corners[0])?;
    let part_ns = wall_on(doc, l1, ft(16.0, 0.0), ft(16.0, 30.0))?;
    let part_ew = wall_on(doc, l1, ft(16.0, 12.0), ft(40.0, 12.0))?;
    let south2 = wall_on(doc, l2, corners[0], corners[1])?;
    let north2 = wall_on(doc, l2, corners[2], corners[3])?;
    let f = |x: f64| x * MM_PER_FT;
    ops::create_door(doc, entry, south1, f(28.0), false)?;
    ops::create_door(doc, door, part_ns, f(22.0), true)?;
    ops::create_door(doc, small, part_ew, f(6.0), false)?;
    ops::create_window(doc, wide, south1, f(8.0), true)?;
    ops::create_window(doc, casement, east1, f(6.0), true)?;
    ops::create_window(doc, casement, east1, f(22.0), true)?;
    ops::create_window(doc, wide, north1, f(12.0), true)?;
    ops::create_window(doc, casement, north1, f(30.0), true)?;
    ops::create_window(doc, casement, west1, f(15.0), true)?;
    for x in [8.0, 20.0, 32.0] {
        ops::create_window(doc, casement, south2, f(x), true)?;
        ops::create_window(doc, casement, north2, f(x), true)?;
    }

    let model = studio_regen::regenerate(doc);
    let slab = ops::first_of(doc, Category::FloorType).context("missing floor type")?;
    for level in [l1, l2] {
        if let Some(b) = studio_regen::outer_boundary(&model, level) {
            ops::create_floor(doc, slab, level, b)?;
        }
    }
    let act = doc
        .of(Category::CeilingType)
        .find(|e| e.data.name().contains("ACT"))
        .map(|e| e.id)
        .context("missing ceiling type")?;
    for p in [ft(8.0, 15.0), ft(28.0, 6.0), ft(28.0, 21.0)] {
        if let Some(room) = studio_regen::room_at(&model, l1, p) {
            ops::create_ceiling(doc, act, l1, room)?;
        }
    }
    for (level, x, y, name) in [
        (l1, 8.0, 15.0, "Living"),
        (l1, 28.0, 21.0, "Kitchen"),
        (l1, 28.0, 6.0, "Bedroom"),
        (l2, 12.0, 15.0, "Studio"),
        (l2, 32.0, 15.0, "Office"),
    ] {
        let r = ops::create_room(doc, level, ft(x, y))?;
        ops::set_property(doc, r, "name", name, 0)?;
    }
    build_sample_documents(doc, ft)
}

/// The sample drawing set: a section, dimensions and four ARCH D sheets.
fn build_sample_documents(
    doc: &mut Document,
    ft: impl Fn(f64, f64) -> studio_geom::Pt,
) -> anyhow::Result<()> {
    use studio_core::{ScheduleKind, SheetSize};
    let view_where = |doc: &Document, pred: &dyn Fn(&ViewKind, &str) -> bool| {
        doc.of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind, name, .. } if pred(kind, name)))
            .map(|e| e.id)
            .context("missing sample view")
    };
    let levels = doc.levels();
    let (l1, l2) = (levels[0].0, levels[1].0);
    let plan1 = view_where(
        doc,
        &|k, _| matches!(k, ViewKind::FloorPlan { level } if *level == l1),
    )?;
    let plan2 = view_where(
        doc,
        &|k, _| matches!(k, ViewKind::FloorPlan { level } if *level == l2),
    )?;
    // Overall dimensions on the Level 1 plan (outside faces: walls are 8" thick).
    let face = 4.0 / 12.0;
    ops::create_dimension(
        doc,
        plan1,
        ft(-face, -face),
        ft(40.0 + face, -face),
        -6.0 * studio_core::units::MM_PER_FT,
    )?;
    ops::create_dimension(
        doc,
        plan1,
        ft(-face, 30.0 + face),
        ft(-face, -face),
        -6.0 * studio_core::units::MM_PER_FT,
    )?;
    ops::create_dimension(
        doc,
        plan1,
        ft(0.0, 30.0 + face),
        ft(16.0, 30.0 + face),
        3.5 * studio_core::units::MM_PER_FT,
    )?;
    ops::create_dimension(
        doc,
        plan1,
        ft(16.0, 30.0 + face),
        ft(40.0, 30.0 + face),
        3.5 * studio_core::units::MM_PER_FT,
    )?;
    // A cross section through the Living room and Kitchen, looking north.
    let section = ops::create_section(doc, ft(-4.0, 18.0), ft(44.0, 18.0))?;

    let schedule = |doc: &Document, kind: ScheduleKind| {
        view_where(
            doc,
            &|k, _| matches!(k, ViewKind::Schedule { kind: s } if *s == kind),
        )
    };
    let elevation = |doc: &Document, name: &str| {
        view_where(doc, &|k, n| {
            matches!(k, ViewKind::Elevation { .. }) && n == name
        })
    };
    let p = studio_geom::Pt::new;

    let cover = ops::create_sheet(doc, "Cover Sheet", SheetSize::ArchD)?;
    ops::set_property(doc, cover, "number", "A0.0", 0)?;
    ops::place_view(
        doc,
        cover,
        schedule(doc, ScheduleKind::Sheets)?,
        p(260.0, 420.0),
    )?;
    ops::place_view(
        doc,
        cover,
        schedule(doc, ScheduleKind::Rooms)?,
        p(560.0, 420.0),
    )?;

    let plans = ops::create_sheet(doc, "Floor Plans", SheetSize::ArchD)?;
    ops::set_property(doc, plans, "number", "A1.0", 0)?;
    ops::place_view(doc, plans, plan1, p(225.0, 330.0))?;
    ops::place_view(doc, plans, plan2, p(595.0, 330.0))?;

    let elevs = ops::create_sheet(doc, "Exterior Elevations", SheetSize::ArchD)?;
    ops::set_property(doc, elevs, "number", "A2.0", 0)?;
    for (name, x, y) in [
        ("South", 225.0, 440.0),
        ("North", 595.0, 440.0),
        ("East", 225.0, 175.0),
        ("West", 595.0, 175.0),
    ] {
        ops::place_view(doc, elevs, elevation(doc, name)?, p(x, y))?;
    }

    let sections = ops::create_sheet(doc, "Sections and Schedules", SheetSize::ArchD)?;
    ops::set_property(doc, sections, "number", "A3.0", 0)?;
    ops::place_view(doc, sections, section, p(300.0, 380.0))?;
    ops::place_view(
        doc,
        sections,
        schedule(doc, ScheduleKind::Doors)?,
        p(650.0, 470.0),
    )?;
    ops::place_view(
        doc,
        sections,
        schedule(doc, ScheduleKind::Windows)?,
        p(650.0, 300.0),
    )?;
    Ok(())
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
    use studio_geom::Pt;

    #[test]
    fn starts_with_no_project() {
        assert_eq!(Session::default().status(), None);
        assert!(Session::default().state().is_none());
    }

    #[test]
    fn new_project_is_untitled_and_seeded() {
        let mut s = Session::default();
        s.new_project("0.0.1").unwrap();
        let state = s.state().unwrap();
        assert_eq!(state.project.name, "Untitled");
        assert_eq!(state.views.len(), 13);
        assert_eq!(state.views[0].view_type, ViewType::Plan);
        assert_eq!(state.levels.len(), 2);
        assert!(state.current_stage.is_some());
        assert_eq!(state.undo, None, "seeding is not undoable");
    }

    #[test]
    fn save_as_then_save_then_reopen_keeps_elements() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::default();
        s.new_project("0.0.1").unwrap();
        assert!(s.save(None, "0.0.1").is_err(), "untitled needs a path");
        let state = s.state().unwrap();
        let (wt, l1) = (state.wall_types[0].id, state.levels[0].id);
        s.edit(|d| ops::create_wall(d, wt, l1, Pt::new(0.0, 0.0), Pt::new(3000.0, 0.0)))
            .unwrap();
        assert!(s.status().unwrap().dirty);

        s.save(Some(&dir.path().join("House")), "0.0.1").unwrap();
        let expected = dir.path().join("House.rfproj");
        assert!(expected.is_file());
        assert!(!s.status().unwrap().dirty);

        let mut other = Session::default();
        other.open(&expected).unwrap();
        assert_eq!(other.doc().unwrap().of(Category::Wall).count(), 1);
    }

    /// Dev aid: `cargo test -p rufplan-studio write_sample_pdf -- --ignored` writes the
    /// sample drawing set to target/sample-drawing-set.pdf for visual review.
    #[test]
    #[ignore]
    fn write_sample_pdf() {
        let mut s = Session::default();
        s.new_sample("0.0.1").unwrap();
        let doc = s.doc().unwrap();
        let sheets: Vec<_> = ops::sheets(doc).into_iter().map(|x| x.0).collect();
        let pdf = studio_sheets::export_pdf(doc, &sheets, "2026-09-24").unwrap();
        let out = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/sample-drawing-set.pdf"
        );
        std::fs::write(out, pdf).unwrap();
    }

    #[test]
    fn sample_project_has_a_building() {
        let mut s = Session::default();
        s.new_sample("0.0.1").unwrap();
        let doc = s.doc().unwrap();
        assert_eq!(doc.of(Category::Wall).count(), 11);
        assert_eq!(doc.of(Category::Floor).count(), 2);
        assert_eq!(doc.of(Category::Ceiling).count(), 3);
        assert_eq!(doc.of(Category::Grid).count(), 6);
        assert_eq!(doc.levels().len(), 2);
        assert_eq!(doc.of(Category::Door).count(), 3);
        assert_eq!(doc.of(Category::Window).count(), 12);
        let rooms = studio_regen::regenerate(doc).rooms;
        assert_eq!(rooms.len(), 5);
        assert!(
            rooms.iter().all(|r| r.boundary.is_some()),
            "every sample room is enclosed"
        );
        assert_eq!(
            studio_core::ops::sheets(doc)
                .iter()
                .map(|s| s.1.as_str())
                .collect::<Vec<_>>(),
            ["A0.0", "A1.0", "A2.0", "A3.0"]
        );
        let pdf = studio_sheets::export_pdf(
            doc,
            &studio_core::ops::sheets(doc)
                .iter()
                .map(|s| s.0)
                .collect::<Vec<_>>(),
            "2026-09-24",
        )
        .unwrap();
        assert!(pdf.len() > 10_000);
        let state = s.state().unwrap();
        assert_eq!(state.project_name, "Sample House");
        assert_eq!(state.undo, None);
        assert!(!state.project.dirty);
    }

    #[test]
    fn failed_open_keeps_current_project() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::default();
        s.new_project("0.0.1").unwrap();
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
