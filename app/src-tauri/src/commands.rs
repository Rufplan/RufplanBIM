//! IPC commands: thin wrappers over the core crates. Payload types derive `TS` so
//! `cargo test` regenerates `app/src/bindings/*.ts` and the TypeScript side stays in sync.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use studio_core::{ops, ElementId};
use studio_geom::Pt;
use studio_views::{DimensionPreview, DisplayList, Mesh, OpeningPreview, RoomPreview, SnapResult};
use tauri::{AppHandle, Manager, State, WebviewWindow};
use ts_rs::TS;

use crate::session::{AppState, Session};

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

impl From<studio_core::CoreError> for CommandError {
    fn from(err: studio_core::CoreError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

type CommandResult<T> = Result<T, CommandError>;
type StateResult = CommandResult<Option<AppState>>;

/// Today's date as YYYY-MM-DD (UTC), for title blocks.
pub(crate) fn today() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The display list of a view or a sheet (views come from the document's cache).
fn any_display_list(
    doc: &studio_core::Document,
    id: ElementId,
) -> Option<std::sync::Arc<DisplayList>> {
    match doc.data(id).ok()? {
        studio_core::ElementData::Sheet { .. } => {
            studio_sheets::sheet_display_list_shared(doc, id, &today())
        }
        _ => studio_views::display_list_shared(doc, id),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

pub(crate) fn lock<'a>(
    state: &'a State<'_, SessionState>,
) -> CommandResult<MutexGuard<'a, Session>> {
    state
        .lock()
        .map_err(|_| anyhow::anyhow!("project state is unavailable after an earlier crash").into())
}

/// Updates the window title and returns the new state.
pub(crate) fn finish(window: &WebviewWindow, session: &Session) -> StateResult {
    let state = session.state();
    let title = match &state {
        Some(s) => format!(
            "{}{} — {APP_NAME}",
            s.project.name,
            if s.project.dirty { " *" } else { "" }
        ),
        None => APP_NAME.to_owned(),
    };
    window.set_title(&title).map_err(anyhow::Error::from)?;
    Ok(state)
}

/// Applies an edit and returns the new state.
fn edit<T>(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&mut Session) -> anyhow::Result<T>,
) -> StateResult {
    let mut session = lock(state)?;
    f(&mut session)?;
    finish(window, &session)
}

#[tauri::command]
pub fn core_version() -> CoreVersion {
    CoreVersion {
        app: APP_VERSION.to_owned(),
        core: studio_core::crate_version().to_owned(),
        io: studio_io::crate_version().to_owned(),
        schema_version: studio_io::SCHEMA_VERSION,
    }
}

/// Current state; also sets the window title (e.g. for a project opened at launch).
#[tauri::command]
pub fn app_state(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    let session = lock(&state)?;
    finish(&window, &session)
}

#[tauri::command]
pub fn project_new(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    edit(&window, &state, |s| s.new_project(APP_VERSION))
}

#[tauri::command]
pub fn project_sample(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    edit(&window, &state, |s| s.new_sample(APP_VERSION))
}

/// Opens a project passed on the command line (file association), or the sample with
/// `--sample`.
pub fn open_from_args(session: &mut Session) {
    let Some(arg) = std::env::args().nth(1) else {
        return;
    };
    let result = if arg == "--sample" {
        session.new_sample(APP_VERSION)
    } else {
        session.open(&PathBuf::from(&arg))
    };
    if let Err(e) = result {
        eprintln!("could not open {arg}: {e:#}");
    }
}

#[tauri::command]
pub fn project_open(
    path: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| s.open(&PathBuf::from(path)))
}

/// Saves to `path` when given (Save As), otherwise to the project's current path.
#[tauri::command]
pub fn project_save(
    path: Option<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.save(path.map(PathBuf::from).as_deref(), APP_VERSION)
    })
}

#[tauri::command]
pub fn view_display_list(
    view: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<Option<DisplayList>> {
    let session = lock(&state)?;
    Ok(any_display_list(session.doc()?, view).map(|d| (*d).clone()))
}

#[tauri::command]
pub fn view_meshes(
    view: Option<ElementId>,
    state: State<'_, SessionState>,
) -> CommandResult<Vec<Mesh>> {
    let session = lock(&state)?;
    Ok(studio_views::meshes_in_view(session.doc()?, view))
}

/// Element under `point` (display-list mm) within `tol` mm.
#[tauri::command]
pub fn pick(
    view: ElementId,
    point: Pt,
    tol: f64,
    state: State<'_, SessionState>,
) -> CommandResult<Option<ElementId>> {
    let session = lock(&state)?;
    let dl = any_display_list(session.doc()?, view);
    Ok(dl.and_then(|dl| studio_views::pick(&dl, point, tol)))
}

#[tauri::command]
pub fn snap(
    view: ElementId,
    point: Pt,
    from: Option<Pt>,
    tol: f64,
    only: Option<studio_views::SnapKind>,
    state: State<'_, SessionState>,
) -> CommandResult<SnapResult> {
    let session = lock(&state)?;
    let mut r = studio_views::snap_only(session.doc()?, view, point, from, tol, only);
    // In sketch mode, the sketch's own ends and midpoints snap first.
    if let Some(q) = crate::sketching::sketch_snap(&session, point, tol) {
        if from.is_none_or(|f| f.dist(q) > 1.0) {
            r.pt = q;
            r.kind = studio_views::snap::SnapKind::Endpoint;
        }
    }
    Ok(r)
}

#[tauri::command]
pub fn create_wall(
    view: ElementId,
    type_id: ElementId,
    start: Pt,
    end: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        let level = s.view_level(view)?;
        s.edit(|d| ops::create_wall(d, type_id, level, start, end))
    })
}

#[tauri::command]
pub fn create_grid(
    start: Pt,
    end: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| ops::create_grid(d, start, end))
    })
}

#[tauri::command]
pub fn create_level(
    elevation: f64,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| ops::create_level(d, elevation))
    })
}

/// Floor from a sketched boundary, or from the outer faces of the view level's walls
/// when `boundary` is empty.
#[tauri::command]
pub fn create_floor(
    view: ElementId,
    type_id: ElementId,
    boundary: Vec<Pt>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        let level = s.view_level(view)?;
        if boundary.is_empty() {
            // Pick Walls: the floor keeps following the walls (ADR-020).
            s.edit(|d| studio_regen::derived::create_floor_by_walls(d, type_id, level))
        } else {
            s.edit(|d| ops::create_floor(d, type_id, level, boundary))
        }
    })
}

/// Where a door or window of `type_id` would be placed for the cursor at `point`.
#[tauri::command]
pub fn opening_preview(
    view: ElementId,
    type_id: ElementId,
    point: Pt,
    tol: f64,
    state: State<'_, SessionState>,
) -> CommandResult<Option<OpeningPreview>> {
    let session = lock(&state)?;
    Ok(studio_views::opening_preview(
        session.doc()?,
        view,
        type_id,
        point,
        tol,
    ))
}

/// Places a door or window (by its type's category) in `host`.
#[tauri::command]
pub fn create_opening(
    type_id: ElementId,
    host: ElementId,
    offset: f64,
    flip_facing: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        let is_door = matches!(
            s.doc()?.data(type_id)?,
            studio_core::ElementData::DoorType { .. }
        );
        s.edit(|d| {
            if is_door {
                ops::create_door(d, type_id, host, offset, flip_facing)
            } else {
                ops::create_window(d, type_id, host, offset, flip_facing)
            }
        })
    })
}

/// Ceiling from a sketched boundary, or filling the room around `inside` when given.
#[tauri::command]
pub fn create_ceiling(
    view: ElementId,
    type_id: ElementId,
    boundary: Vec<Pt>,
    inside: Option<Pt>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        let level = s.view_level(view)?;
        match inside {
            // Auto Room: the ceiling keeps following the room's walls (ADR-020).
            Some(p) => {
                s.edit(|d| studio_regen::derived::create_ceiling_in_room(d, type_id, level, p))
            }
            None => s.edit(|d| ops::create_ceiling(d, type_id, level, boundary)),
        }
    })
}

#[tauri::command]
pub fn delete_elements(
    ids: Vec<ElementId>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| s.edit(|d| ops::delete(d, &ids)))
}

#[tauri::command]
pub fn properties(
    id: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<ops::PropertySheet> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let mut sheet = ops::properties(doc, id)?;
    if matches!(
        sheet.category,
        studio_core::Category::Floor | studio_core::Category::Ceiling
    ) {
        // A floor or ceiling bound to walls takes its area from the model.
        let model = studio_regen::regenerate(doc);
        let area: f64 = model
            .floors
            .iter()
            .chain(&model.ceilings)
            .filter(|s| s.id == id)
            .map(|s| s.base.area())
            .sum();
        if let Some(row) = sheet.properties.iter_mut().find(|p| p.key == "area") {
            row.value = studio_core::units::format_area_sf(area);
        }
    }
    if sheet.category == studio_core::Category::Room {
        // Area and enclosure are derived from the walls, so they come from regeneration.
        let model = studio_regen::regenerate(doc);
        if let Some(r) = model.rooms.iter().find(|r| r.id == id) {
            let row = |key: &str, label: &str, value: String| ops::Property {
                key: key.into(),
                label: label.into(),
                group: "Dimensions".into(),
                value,
                kind: ops::PropKind::ReadOnly,
                options: vec![],
            };
            let enclosed = r.boundary.is_some();
            sheet.properties.push(row(
                "area",
                "Area",
                studio_core::units::format_area_sf(r.area()),
            ));
            sheet.properties.push(row(
                "perimeter",
                "Perimeter",
                studio_core::units::format_ft_in(r.perimeter()),
            ));
            sheet.properties.push(row(
                "status",
                "Status",
                if enclosed {
                    "Enclosed".into()
                } else {
                    "Not enclosed — close the walls around it".into()
                },
            ));
        }
    }
    Ok(sheet)
}

#[tauri::command]
pub fn create_section(
    start: Pt,
    end: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| ops::create_section(d, start, end))
    })
}

#[tauri::command]
pub fn dimension_preview(
    view: ElementId,
    a: Pt,
    b: Pt,
    cursor: Pt,
    state: State<'_, SessionState>,
) -> CommandResult<Option<DimensionPreview>> {
    let session = lock(&state)?;
    Ok(studio_views::dimension_preview(
        session.doc()?,
        view,
        a,
        b,
        cursor,
    ))
}

#[tauri::command]
pub fn create_dimension(
    view: ElementId,
    a: Pt,
    b: Pt,
    offset: f64,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| ops::create_dimension(d, view, a, b, offset))
    })
}

#[tauri::command]
pub fn create_text(
    view: ElementId,
    at: Pt,
    text: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| ops::create_text(d, view, at, &text))
    })
}

/// Creates a sheet with the next number.
#[tauri::command]
pub fn create_sheet(
    name: String,
    tabloid: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let size = if tabloid {
        studio_core::SheetSize::Tabloid
    } else {
        studio_core::SheetSize::ArchD
    };
    edit(&window, &state, |s| {
        s.edit(|d| ops::create_sheet(d, &name, size))
    })
}

/// Places `view` in the middle of `sheet`'s drawing area.
#[tauri::command]
pub fn place_view(
    sheet: ElementId,
    view: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        let size = match s.doc()?.data(sheet)? {
            studio_core::ElementData::Sheet { size, .. } => *size,
            _ => anyhow::bail!("open a sheet first"),
        };
        let (w, h) = size.mm();
        let center = Pt::new(
            (w - studio_sheets::sheet::title_block_width(size)) / 2.0,
            h / 2.0,
        );
        s.edit(|d| ops::place_view(d, sheet, view, center))
    })
}

#[tauri::command]
pub fn schedule_table(
    view: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<Option<studio_sheets::Table>> {
    let session = lock(&state)?;
    Ok(studio_sheets::schedule(session.doc()?, view))
}

#[tauri::command]
pub fn tag_all(
    view: ElementId,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| s.edit(|d| ops::tag_all(d, view)))
}

fn write_pdf(
    doc: &studio_core::Document,
    sheets: &[ElementId],
    path: &str,
) -> anyhow::Result<PathBuf> {
    let bytes = studio_sheets::export_pdf(doc, sheets, &today())?;
    let mut path = PathBuf::from(path);
    if path
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("pdf"))
    {
        path.set_extension("pdf");
    }
    std::fs::write(&path, bytes)
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", path.display()))?;
    Ok(path)
}

/// Issues the current design stage's sheet set: records an issuance named `name` (so the
/// title blocks list it) and writes the set to a PDF at `path`.
#[tauri::command]
pub fn issue_set(
    name: String,
    path: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        let doc = s.doc()?;
        let stage = ops::project_info(doc).and_then(|i| match doc.data(i) {
            Ok(studio_core::ElementData::ProjectInfo { current_stage, .. }) => *current_stage,
            _ => None,
        });
        let sheets = ops::stage_sheets(doc, stage);
        let date = today();
        s.edit(|d| ops::create_issuance(d, &name, &date, sheets.clone()))?;
        write_pdf(s.doc()?, &sheets, &path)?;
        Ok(())
    })
}

/// Exports every sheet, in number order, to a PDF at `path`. Returns the sheet count.
#[tauri::command]
pub fn export_pdf(path: String, state: State<'_, SessionState>) -> CommandResult<usize> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let sheets: Vec<ElementId> = ops::sheets(doc).into_iter().map(|s| s.0).collect();
    if sheets.is_empty() {
        return Err(anyhow::anyhow!("create a sheet and place views on it first").into());
    }
    write_pdf(doc, &sheets, &path)?;
    Ok(sheets.len())
}

/// Writes the model to an IFC4 file; returns a one-line summary of what it holds.
#[tauri::command]
pub fn export_ifc(path: String, state: State<'_, SessionState>) -> CommandResult<String> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let (path, sum) = write_ifc(doc, &path)?;
    Ok(format!(
        "{} walls, {} doors, {} windows, {} slabs, {} spaces to {}",
        sum.walls,
        sum.doors,
        sum.windows,
        sum.slabs,
        sum.spaces,
        path.display()
    ))
}

/// ISO 8601 UTC timestamp for file headers.
pub(crate) fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!(
        "{}T{:02}:{:02}:{:02}",
        today(),
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60
    )
}

pub(crate) fn write_ifc(
    doc: &studio_core::Document,
    path: &str,
) -> anyhow::Result<(PathBuf, studio_io::ifc::IfcSummary)> {
    let (text, sum) = studio_io::ifc::export_ifc(doc, env!("CARGO_PKG_VERSION"), &now_iso());
    let mut path = PathBuf::from(path);
    if path
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("ifc"))
    {
        path.set_extension("ifc");
    }
    std::fs::write(&path, text)
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", path.display()))?;
    Ok((path, sum))
}

/// What a room placed at `point` would fill (floor plans only).
#[tauri::command]
pub fn room_preview(
    view: ElementId,
    point: Pt,
    state: State<'_, SessionState>,
) -> CommandResult<Option<RoomPreview>> {
    let session = lock(&state)?;
    Ok(studio_views::room_preview(session.doc()?, view, point))
}

/// Places a room in the enclosed area around `point`.
#[tauri::command]
pub fn create_room(
    view: ElementId,
    point: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        let level = s.view_level(view)?;
        let model = studio_regen::regenerate(s.doc()?);
        if studio_regen::room_at(&model, level, point).is_none() {
            anyhow::bail!("click inside an area fully enclosed by walls");
        }
        if let Some(r) = studio_regen::room_occupying(&model, level, point) {
            anyhow::bail!("this area already has a room ({} {})", r.name, r.number);
        }
        s.edit(|d| ops::create_room(d, level, point))
    })
}

/// Moves elements by `delta` mm (walls stretch their joined neighbours, see modify.rs).
#[tauri::command]
pub fn move_elements(
    ids: Vec<ElementId>,
    delta: Pt,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| studio_core::modify::move_elements(d, &ids, delta))
    })
}

#[tauri::command]
pub fn set_property(
    id: ElementId,
    key: String,
    value: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        if studio_regen::derived::handles(&key, &value) {
            s.edit(|d| studio_regen::derived::set_property(d, id, &key, &value))
        } else {
            s.edit(|d| ops::set_property(d, id, &key, &value, now_ms()))
        }
    })
}

#[tauri::command]
pub fn undo(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    edit(&window, &state, |s| s.edit(|d| d.undo().map(|_| ())))
}

#[tauri::command]
pub fn redo(window: WebviewWindow, state: State<'_, SessionState>) -> StateResult {
    edit(&window, &state, |s| s.edit(|d| d.redo().map(|_| ())))
}

/// Closes the app. Without `force`, refuses while there are unsaved changes.
#[tauri::command]
pub fn app_exit(
    force: bool,
    app: AppHandle,
    state: State<'_, SessionState>,
) -> CommandResult<bool> {
    if !force && lock(&state)?.is_dirty() {
        return Ok(false);
    }
    if let Some(w) = app.get_webview_window("main") {
        w.destroy().map_err(anyhow::Error::from)?;
    }
    Ok(true)
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
    fn today_is_a_plausible_date() {
        let d = today();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"), "{d}");
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
