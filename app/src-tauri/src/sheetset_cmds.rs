//! IPC commands for deliverable sheet sets (ADR-032). Thin wrappers over studio-sheets.

use serde::Serialize;
use studio_core::{ops, ElementData};
use studio_sheets::sets::{self, BuildingType, SetOptions, SetPlan, SetReport};
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, today, write_pdf, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

/// A building type as the dialog lists it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BuildingTypeOption {
    pub id: BuildingType,
    pub label: String,
}

#[tauri::command]
pub fn building_types() -> Vec<BuildingTypeOption> {
    BuildingType::ALL
        .iter()
        .map(|t| BuildingTypeOption {
            id: *t,
            label: t.label().into(),
        })
        .collect()
}

/// What Create Sheets would make.
#[tauri::command]
pub fn sheet_set_plan(
    options: SetOptions,
    state: State<'_, SessionState>,
) -> CommandResult<SetPlan> {
    let s = lock(&state)?;
    Ok(sets::plan(s.doc()?, &options))
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SetsCreated {
    pub state: Option<AppState>,
    pub report: SetReport,
}

/// Creates or updates the sets' sheets (one undo step).
#[tauri::command]
pub fn create_sheet_sets(
    options: SetOptions,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<SetsCreated> {
    let mut s = lock(&state)?;
    let report = s.edit(|d| sets::create(d, &options))?;
    let state = finish(&window, &s)?;
    Ok(SetsCreated { state, report })
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ExportedSet {
    pub name: String,
    pub path: String,
    pub sheets: usize,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SetsExported {
    pub state: Option<AppState>,
    pub files: Vec<ExportedSet>,
}

/// Keeps a file name to letters, digits and a few marks.
fn file_part(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_%&().".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Writes one PDF per deliverable of the chosen phases into `folder`, each printed as of
/// its own design stage; `record` also records each as an issuance.
#[tauri::command]
pub fn export_sheet_sets(
    phases: Vec<String>,
    folder: String,
    record: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<SetsExported> {
    let mut s = lock(&state)?;
    let doc = s.doc()?;
    let list = sets::deliverables(doc, &phases);
    if list.is_empty() {
        return Err(anyhow::anyhow!(
            "no sheets are in those phases' sets yet: create the sheet sets first"
        )
        .into());
    }
    let (number, project) = match ops::project_info(doc).and_then(|i| doc.data(i).ok()) {
        Some(ElementData::ProjectInfo { number, name, .. }) => (number.clone(), name.clone()),
        _ => (String::new(), String::new()),
    };
    let mut files = vec![];
    for (_, stage, _, name, sheets) in &list {
        let copy = sets::as_of_stage(doc, *stage)?;
        let stem = [number.as_str(), project.as_str(), name.as_str()]
            .iter()
            .filter(|p| !p.trim().is_empty())
            .map(|p| file_part(p))
            .collect::<Vec<_>>()
            .join(" - ");
        let path = std::path::Path::new(&folder).join(format!("{stem}.pdf"));
        let written = write_pdf(&copy, sheets, &path.to_string_lossy())?;
        files.push(ExportedSet {
            name: name.clone(),
            path: written.display().to_string(),
            sheets: sheets.len(),
        });
    }
    if record {
        let date = today();
        s.edit(|d| {
            d.transact("Issue deliverables", |tx| {
                for (_, stage, _, name, sheets) in &list {
                    tx.insert(ElementData::Issuance {
                        name: name.clone(),
                        stage: Some(*stage),
                        date: date.clone(),
                        sheets: sheets.clone(),
                    });
                }
                Ok(())
            })
        })?;
    }
    let state = finish(&window, &s)?;
    Ok(SetsExported { state, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_safe() {
        assert_eq!(
            file_part("IFC — Issued / Construction"),
            "IFC - Issued - Construction"
        );
        assert_eq!(building_types().len(), 7);
    }
}
