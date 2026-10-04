//! IPC commands for General Notes (ADR-103). Thin wrappers over studio-core
//! `general_notes`; placement works out where the notes go in a view or on a sheet.

use serde::Serialize;
use studio_core::general_notes::{self, NotesBuilding, NotesDrawing};
use studio_core::{ElementData, ElementId};
use studio_geom::Pt;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

/// What the General Notes dialog offers: the building types and drawings, the defaults for
/// the view (from Project Info and the view itself) and the code edition.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotesOptions {
    pub buildings: Vec<(NotesBuilding, String)>,
    pub drawings: Vec<(NotesDrawing, String)>,
    pub building: NotesBuilding,
    pub drawing: NotesDrawing,
    pub code: String,
}

#[tauri::command]
pub fn general_notes_options(
    view: ElementId,
    state: State<'_, SessionState>,
) -> CommandResult<NotesOptions> {
    let s = lock(&state)?;
    let doc = s.doc()?;
    Ok(NotesOptions {
        buildings: NotesBuilding::ALL
            .iter()
            .map(|b| (*b, b.label().to_string()))
            .collect(),
        drawings: NotesDrawing::ALL
            .iter()
            .map(|d| (*d, d.label().to_string()))
            .collect(),
        building: general_notes::building_for(doc),
        drawing: general_notes::drawing_for(doc, view),
        code: general_notes::code_for(doc),
    })
}

/// The preset notes for a building type and drawing, and their heading.
#[tauri::command]
pub fn general_notes_preset(
    building: NotesBuilding,
    drawing: NotesDrawing,
    code: String,
) -> (String, Vec<String>) {
    (
        drawing.heading(),
        general_notes::notes(building, drawing, &code),
    )
}

/// Places notes in `view`: on a sheet, at the top of the column beside the title block; in
/// a drawing, just right of what's drawn, at its top.
#[tauri::command]
pub fn place_general_notes(
    view: ElementId,
    heading: String,
    notes: Vec<String>,
    width: f64,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut s = lock(&state)?;
    let at = {
        let doc = s.doc()?;
        match doc.data(view) {
            Ok(ElementData::Sheet { size, .. }) => {
                let (w, h) = size.mm();
                let (_, other) = studio_sheets::sheet::margins(*size);
                let tb = studio_sheets::sheet::title_block_width(*size);
                Pt::new(w - other - tb - width - 12.0, h - other - 16.0)
            }
            Ok(ElementData::View { scale, .. }) => {
                let k = f64::from(*scale);
                let [_, _, x1, y1] = studio_views::display_list(doc, view)
                    .map(|d| d.bounds)
                    .unwrap_or([0.0, 0.0, 0.0, 0.0]);
                Pt::new(x1 + 12.0 * k, y1)
            }
            _ => return Err(anyhow::anyhow!("open a view or sheet to place notes in").into()),
        }
    };
    s.edit(|d| general_notes::place(d, view, at, &heading, &notes, width))?;
    finish(&window, &s)
}
