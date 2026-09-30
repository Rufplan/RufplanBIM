//! Project information (ADR-084): everything about the job besides the model — overview,
//! location, client, the consultant team, budget, milestones, codes and zoning, and notes.
//! Kept on the ProjectInfo element (`details`), so it is saved, undone and synced with the
//! model. The title block's name, number, client and address stay the element's own fields;
//! `set` keeps them in step with the details.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::ElementData;

/// A person or firm and how to reach them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectContact")]
#[serde(default)]
pub struct Contact {
    pub company: String,
    pub name: String,
    pub title: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub website: String,
}

impl Contact {
    pub fn is_empty(&self) -> bool {
        [
            &self.company,
            &self.name,
            &self.title,
            &self.email,
            &self.phone,
            &self.address,
            &self.website,
        ]
        .iter()
        .all(|s| s.trim().is_empty())
    }
}

/// One discipline on the team: its consultant, their scope and fee.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectTeamMember")]
#[serde(default)]
pub struct TeamMember {
    /// "Architect", "Structural Engineer", "Civil Engineer"…
    pub discipline: String,
    pub contact: Contact,
    pub scope: String,
    /// Fee, in the budget's currency (0: not set).
    pub fee: f64,
    pub notes: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectOverview")]
#[serde(default)]
pub struct Overview {
    /// "Active", "On Hold", "Complete"…
    pub status: String,
    /// "Single-Family Residential", "Office", "Mixed-Use"…
    pub project_type: String,
    /// "New Construction", "Addition", "Renovation"…
    pub work_type: String,
    /// "Design-Bid-Build", "CM at Risk", "Design-Build"…
    pub delivery: String,
    pub description: String,
    /// The program's target gross area, sf (0: not set).
    pub target_area_sf: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectLocation")]
#[serde(default)]
pub struct Location {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub county: String,
    pub country: String,
    /// Assessor's parcel number.
    pub apn: String,
    pub legal: String,
    /// The authority having jurisdiction (the building department).
    pub jurisdiction: String,
}

impl Location {
    /// One line, as a title block shows it: "12 Oak St, Austin, TX 78701".
    pub fn one_line(&self) -> String {
        let state_zip = [self.state.trim(), self.zip.trim()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        [self.street.trim(), self.city.trim(), state_zip.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A budget line. `hard` lines are construction cost; the rest are soft costs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectBudgetLine")]
#[serde(default)]
pub struct BudgetLine {
    pub name: String,
    pub hard: bool,
    pub amount: f64,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectBudget")]
#[serde(default)]
pub struct Budget {
    pub currency: String,
    pub lines: Vec<BudgetLine>,
    /// Design and construction contingency, percent of the lines.
    pub contingency_pct: f64,
    /// Escalation to the midpoint of construction, percent of the hard cost.
    pub escalation_pct: f64,
    /// The owner's target construction cost per sf (0: not set).
    pub target_cost_sf: f64,
}

impl Default for Budget {
    fn default() -> Self {
        let line = |name: &str, hard| BudgetLine {
            name: name.into(),
            hard,
            ..Default::default()
        };
        Budget {
            currency: "USD".into(),
            lines: vec![
                line("Sitework", true),
                line("Building construction", true),
                line("General conditions & fee", true),
                line("Design & engineering fees", false),
                line("Permits & impact fees", false),
                line("Furniture, fixtures & equipment", false),
                line("Testing, inspections & surveys", false),
            ],
            contingency_pct: 10.0,
            escalation_pct: 0.0,
            target_cost_sf: 0.0,
        }
    }
}

/// The budget's sums.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectBudgetTotals")]
pub struct BudgetTotals {
    pub hard: f64,
    pub soft: f64,
    pub escalation: f64,
    pub contingency: f64,
    /// Hard + soft + escalation + contingency.
    pub total: f64,
    /// Hard cost (with escalation) per gross sf; 0 without an area.
    pub hard_per_sf: f64,
    /// The owner's target construction cost per sf times the gross area (0: not set).
    pub target_hard: f64,
}

impl Budget {
    /// The sums, and the hard cost per sf of `gross_sf`.
    pub fn totals(&self, gross_sf: f64) -> BudgetTotals {
        let hard: f64 = self.lines.iter().filter(|l| l.hard).map(|l| l.amount).sum();
        let soft: f64 = self
            .lines
            .iter()
            .filter(|l| !l.hard)
            .map(|l| l.amount)
            .sum();
        let escalation = hard * self.escalation_pct / 100.0;
        let contingency = (hard + soft + escalation) * self.contingency_pct / 100.0;
        BudgetTotals {
            hard,
            soft,
            escalation,
            contingency,
            total: hard + soft + escalation + contingency,
            hard_per_sf: if gross_sf > 0.0 {
                (hard + escalation) / gross_sf
            } else {
                0.0
            },
            target_hard: self.target_cost_sf * gross_sf.max(0.0),
        }
    }
}

/// A key date: "Permit submission", "Bid date", "Substantial completion"…
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectMilestone")]
#[serde(default)]
pub struct Milestone {
    pub name: String,
    /// ISO date (YYYY-MM-DD), or empty.
    pub date: String,
    pub done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectCodes")]
#[serde(default)]
pub struct Codes {
    /// "2021 IBC", "2021 IRC"…
    pub building_code: String,
    pub energy_code: String,
    /// IBC occupancy group: "R-3", "B", "A-2"…
    pub occupancy: String,
    /// IBC construction type: "V-B", "III-A"…
    pub construction_type: String,
    pub sprinklered: String,
    pub zoning_district: String,
    pub lot_area: String,
    pub far: String,
    pub max_height: String,
    pub setbacks: String,
    pub lot_coverage: String,
    pub parking: String,
    pub notes: String,
}

/// Everything about the job, beside the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(default)]
pub struct ProjectDetails {
    pub overview: Overview,
    pub location: Location,
    pub client: Contact,
    /// The client's representative, when not the client.
    pub owner_rep: Contact,
    pub team: Vec<TeamMember>,
    pub budget: Budget,
    pub milestones: Vec<Milestone>,
    pub codes: Codes,
    pub notes: String,
}

/// The disciplines a new project lists, in order (consultants are filled in as hired).
pub const DISCIPLINES: &[&str] = &[
    "Architect",
    "Structural Engineer",
    "Mechanical Engineer",
    "Electrical Engineer",
    "Plumbing Engineer",
    "Civil Engineer",
    "Landscape Architect",
    "Interior Designer",
    "Geotechnical Engineer",
    "Surveyor",
    "General Contractor",
];

/// More disciplines the Add menu offers.
pub const MORE_DISCIPLINES: &[&str] = &[
    "MEP Engineer",
    "Fire Protection Engineer",
    "Lighting Designer",
    "Acoustical Consultant",
    "Code Consultant",
    "Envelope Consultant",
    "Sustainability / LEED Consultant",
    "Cost Estimator",
    "Specifications Writer",
    "Technology / AV Consultant",
    "Kitchen Consultant",
    "Accessibility Consultant",
    "Traffic Engineer",
    "Environmental Consultant",
    "Owner's Representative",
    "Construction Manager",
    "Lender",
    "Attorney",
];

pub const MILESTONES: &[&str] = &[
    "Kickoff",
    "Schematic Design complete",
    "Design Development complete",
    "Permit submission",
    "Permit issued",
    "Bid date",
    "Construction start",
    "Substantial completion",
    "Certificate of occupancy",
];

impl Default for ProjectDetails {
    fn default() -> Self {
        ProjectDetails {
            overview: Overview {
                status: "Active".into(),
                ..Default::default()
            },
            location: Location {
                country: "United States".into(),
                ..Default::default()
            },
            client: Contact::default(),
            owner_rep: Contact::default(),
            team: DISCIPLINES
                .iter()
                .map(|d| TeamMember {
                    discipline: (*d).into(),
                    ..Default::default()
                })
                .collect(),
            budget: Budget::default(),
            milestones: MILESTONES
                .iter()
                .map(|m| Milestone {
                    name: (*m).into(),
                    ..Default::default()
                })
                .collect(),
            codes: Codes::default(),
            notes: String::new(),
        }
    }
}

/// The title block's fields, kept on the ProjectInfo element.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ProjectIdentity")]
pub struct Identity {
    pub name: String,
    pub number: String,
    pub client: String,
    pub address: String,
}

/// The project's identity and details. Files from before ADR-084 have the defaults, with
/// the old client and address strings moved in.
pub fn get(doc: &Document) -> CoreResult<(Identity, ProjectDetails)> {
    let info = crate::ops::project_info(doc)
        .ok_or_else(|| CoreError::Invalid("project information is missing".into()))?;
    match doc.data(info)? {
        ElementData::ProjectInfo {
            name,
            number,
            client,
            address,
            details,
            ..
        } => {
            let mut d = (**details).clone();
            if d.client.is_empty() && !client.trim().is_empty() {
                d.client.company = client.clone();
            }
            if d.location.one_line().is_empty() && !address.trim().is_empty() {
                d.location.street = address.clone();
            }
            Ok((
                Identity {
                    name: name.clone(),
                    number: number.clone(),
                    client: client.clone(),
                    address: address.clone(),
                },
                d,
            ))
        }
        _ => Err(CoreError::Invalid("not project information".into())),
    }
}

/// Saves the name, number and details in one undoable step. The title block's client is
/// the client's company (or name) and its address the location on one line.
pub fn set(doc: &mut Document, name: &str, number: &str, d: ProjectDetails) -> CoreResult<()> {
    let info = crate::ops::project_info(doc)
        .ok_or_else(|| CoreError::Invalid("project information is missing".into()))?;
    if name.trim().is_empty() {
        return Err(CoreError::Invalid("the project needs a name".into()));
    }
    if d.budget
        .lines
        .iter()
        .any(|l| !l.amount.is_finite() || l.amount < 0.0)
        || !(0.0..=100.0).contains(&d.budget.contingency_pct)
        || !(0.0..=100.0).contains(&d.budget.escalation_pct)
    {
        return Err(CoreError::Invalid(
            "budget amounts can't be negative, and percentages are 0–100".into(),
        ));
    }
    let client_line = if d.client.company.trim().is_empty() {
        d.client.name.trim().to_string()
    } else {
        d.client.company.trim().to_string()
    };
    let address_line = d.location.one_line();
    doc.transact("Edit Project Information", |tx| {
        tx.modify(info, |e| {
            if let ElementData::ProjectInfo {
                name: n,
                number: no,
                client,
                address,
                details,
                ..
            } = e
            {
                *n = name.trim().into();
                *no = number.trim().into();
                *client = client_line.clone();
                *address = address_line.clone();
                **details = d.clone();
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_totals_add_escalation_and_contingency() {
        let mut b = Budget::default();
        b.lines[0].amount = 100_000.0; // sitework (hard)
        b.lines[1].amount = 900_000.0; // building (hard)
        b.lines[3].amount = 120_000.0; // fees (soft)
        b.escalation_pct = 5.0;
        b.contingency_pct = 10.0;
        let t = b.totals(2_000.0);
        assert_eq!(
            (t.hard, t.soft, t.escalation),
            (1_000_000.0, 120_000.0, 50_000.0)
        );
        // (1,000,000 + 120,000 + 50,000) x 10%
        assert!((t.contingency - 117_000.0).abs() < 1e-6);
        assert!((t.total - 1_287_000.0).abs() < 1e-6);
        // 1,050,000 / 2,000 sf
        assert!((t.hard_per_sf - 525.0).abs() < 1e-9);
        assert_eq!(b.totals(0.0).hard_per_sf, 0.0);
        b.target_cost_sf = 450.0;
        assert_eq!(b.totals(2_000.0).target_hard, 900_000.0);
    }

    #[test]
    fn location_reads_as_one_line() {
        let l = Location {
            street: "12 Oak St".into(),
            city: "Austin".into(),
            state: "TX".into(),
            zip: "78701".into(),
            ..Default::default()
        };
        assert_eq!(l.one_line(), "12 Oak St, Austin, TX 78701");
        assert_eq!(Location::default().one_line(), "");
    }

    #[test]
    fn set_keeps_the_title_block_in_step_and_undoes() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let (id, d) = get(&doc).unwrap();
        assert_eq!(d.team.len(), DISCIPLINES.len());
        assert_eq!(d.budget.lines.len(), 7);
        let mut d2 = d.clone();
        d2.client.company = "Oak Holdings LLC".into();
        d2.location.street = "12 Oak St".into();
        d2.location.city = "Austin".into();
        d2.team[1].contact.company = "Beam & Co".into();
        set(&mut doc, "Oak House", "2601", d2.clone()).unwrap();
        let (id2, got) = get(&doc).unwrap();
        assert_eq!(
            id2,
            Identity {
                name: "Oak House".into(),
                number: "2601".into(),
                client: "Oak Holdings LLC".into(),
                address: "12 Oak St, Austin".into(),
            }
        );
        assert_eq!(got, d2);
        doc.undo().unwrap();
        assert_eq!(get(&doc).unwrap().0, id);
        // Bad values are refused.
        let mut bad = d.clone();
        bad.budget.lines[0].amount = -1.0;
        assert!(set(&mut doc, "Oak House", "2601", bad).is_err());
        assert!(set(&mut doc, " ", "2601", d).is_err());
    }

    #[test]
    fn old_client_and_address_move_into_the_details() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let info = crate::ops::project_info(&doc).unwrap();
        crate::ops::set_property(&mut doc, info, "client", "Ms. Lee", 0).unwrap();
        crate::ops::set_property(&mut doc, info, "address", "4 Elm Rd", 0).unwrap();
        let (_, d) = get(&doc).unwrap();
        assert_eq!(
            (d.client.company.as_str(), d.location.street.as_str()),
            ("Ms. Lee", "4 Elm Rd")
        );
    }
}
