//! IPC for the Project Info tab (ADR-084): thin wrappers over studio-core `project`, plus
//! what the model says about the job (areas, levels, stages, the site).

use serde::Serialize;
use studio_core::project::{
    self, BudgetTotals, Identity, ProjectDetails, DISCIPLINES, MILESTONES, MORE_DISCIPLINES,
};
use studio_core::units::MM_PER_FT;
use studio_core::{Category, ElementData};
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

/// A design stage, as the Schedule section lists it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectStageRow {
    pub name: String,
    pub abbreviation: String,
    pub start: String,
    pub target: String,
    pub current: bool,
}

/// The lot, from the Site tab (ADR-023), when there is one.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectSiteInfo {
    pub address: String,
    pub lat: f64,
    pub lon: f64,
    pub apn: String,
    pub owner: String,
    pub acres: f64,
}

/// Everything the Project Info tab shows.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectInfoState {
    pub identity: Identity,
    pub details: ProjectDetails,
    /// From the model: gross area (outside faces of walls) and room area, sf.
    pub gross_sf: f64,
    pub room_sf: f64,
    /// The budget's sums, with the hard cost per gross sf.
    pub totals: BudgetTotals,
    pub levels: usize,
    pub rooms: usize,
    pub stages: Vec<ProjectStageRow>,
    pub site: Option<ProjectSiteInfo>,
    /// The linked Rufplan project's name (ADR-016).
    pub rufplan: Option<String>,
    pub disciplines: Vec<String>,
    pub milestone_names: Vec<String>,
}

fn sf(mm2: f64) -> f64 {
    mm2 / (MM_PER_FT * MM_PER_FT)
}

#[tauri::command]
pub fn project_info_get(state: State<'_, SessionState>) -> Result<ProjectInfoState, CommandError> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let (identity, details) = project::get(doc)?;
    let model = studio_regen::regenerate(doc);
    let (current, rufplan) =
        match studio_core::ops::project_info(doc).and_then(|i| doc.data(i).ok()) {
            Some(ElementData::ProjectInfo {
                current_stage,
                rufplan,
                ..
            }) => (*current_stage, rufplan.as_ref().map(|r| r.name.clone())),
            _ => (None, None),
        };
    let mut stages: Vec<(i32, ProjectStageRow)> = doc
        .of(Category::Stage)
        .filter_map(|e| match &e.data {
            ElementData::Stage {
                name,
                abbreviation,
                order,
                start,
                target,
            } => Some((
                *order,
                ProjectStageRow {
                    name: name.clone(),
                    abbreviation: abbreviation.clone(),
                    start: start.clone(),
                    target: target.clone(),
                    current: current == Some(e.id),
                },
            )),
            _ => None,
        })
        .collect();
    stages.sort_by_key(|s| s.0);
    let site = doc.of(Category::Site).find_map(|e| match &e.data {
        ElementData::Site {
            address,
            lat,
            lon,
            parcel,
            ..
        } => Some(ProjectSiteInfo {
            address: address.clone(),
            lat: *lat,
            lon: *lon,
            apn: parcel.apn.clone(),
            owner: parcel.owner.clone(),
            acres: parcel.acres,
        }),
        _ => None,
    });
    let gross_sf = sf(model.gross_area());
    Ok(ProjectInfoState {
        totals: details.budget.totals(gross_sf),
        identity,
        details,
        gross_sf,
        room_sf: sf(model.rooms.iter().map(|r| r.area()).sum()),
        levels: model.levels.len(),
        rooms: model.rooms.len(),
        stages: stages.into_iter().map(|s| s.1).collect(),
        site,
        rufplan,
        disciplines: DISCIPLINES
            .iter()
            .chain(MORE_DISCIPLINES)
            .map(|s| (*s).to_owned())
            .collect(),
        milestone_names: MILESTONES.iter().map(|s| (*s).to_owned()).collect(),
    })
}

/// Saves the project's name, number and details (one undoable step).
#[tauri::command]
pub fn project_info_set(
    name: String,
    number: String,
    details: ProjectDetails,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<Option<AppState>, CommandError> {
    let mut session = lock(&state)?;
    session.edit(|d| project::set(d, &name, &number, details))?;
    finish(&window, &session)
}
