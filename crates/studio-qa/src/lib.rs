//! QA/QC (ADR-088): a milestone review of the set, the way a firm's QC reviewer checks
//! drawings before they go out: coordination (marks, numbers, references, sheets),
//! completeness (tags, schedules, TBDs, keynotes), code and accessibility (egress,
//! emergency escape, stairs, door widths, ceiling heights, garage separation), waterproofing
//! (weather barriers, wet rooms, roofs, below-grade walls, slabs), drawing-to-spec
//! consistency (the project manual against the model and keynotes) and constructability
//! (walls, openings, rooms, consultant flags).
//!
//! Rule-based and preliminary: it finds what the model can show, cites the code section a
//! plan checker would, and never claims compliance. Professional QC review still applies.

use serde::{Deserialize, Serialize};
use studio_core::{Document, ElementId};
use studio_regen::Model;
use ts_rs::TS;

mod checks;
mod ctx;
pub mod fix;
pub mod report;

pub use checks::run_checks;

pub const DISCLAIMER: &str = "Automated preliminary review — not a substitute for professional QA/QC, plan check or code review. Verify every finding against the adopted codes and the full set.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export, rename = "QaSeverity")]
pub enum Severity {
    Critical,
    Major,
    Minor,
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Critical => "Critical",
            Severity::Major => "Major",
            Severity::Minor => "Minor",
            Severity::Info => "Info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export, rename = "QaCategory")]
pub enum Category {
    Coordination,
    Completeness,
    Code,
    Accessibility,
    Waterproofing,
    DrawingSpec,
    Constructability,
    Consultants,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Coordination,
        Category::Completeness,
        Category::Code,
        Category::Accessibility,
        Category::Waterproofing,
        Category::DrawingSpec,
        Category::Constructability,
        Category::Consultants,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Category::Coordination => "Coordination",
            Category::Completeness => "Completeness & Clarity",
            Category::Code => "Code Compliance",
            Category::Accessibility => "Accessibility",
            Category::Waterproofing => "Waterproofing & Envelope",
            Category::DrawingSpec => "Drawings ↔ Specifications",
            Category::Constructability => "Constructability",
            Category::Consultants => "Consultant Coordination",
        }
    }
}

/// The milestone the set is reviewed for: early sets skip what isn't expected yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export, rename = "QaMilestone")]
pub enum Milestone {
    SchematicDesign,
    DesignDevelopment,
    Cd50,
    Cd90,
    Cd100,
    Permit,
    Bid,
}

impl Milestone {
    pub fn label(self) -> &'static str {
        match self {
            Milestone::SchematicDesign => "Schematic Design",
            Milestone::DesignDevelopment => "Design Development",
            Milestone::Cd50 => "50% Construction Documents",
            Milestone::Cd90 => "90% Construction Documents",
            Milestone::Cd100 => "100% Construction Documents",
            Milestone::Permit => "Permit Submittal",
            Milestone::Bid => "Bid Set",
        }
    }
    /// Construction documents or later.
    pub fn cd(self) -> bool {
        self >= Milestone::Cd50
    }
    /// A set that leaves the office for an agency or bidders.
    pub fn issued(self) -> bool {
        self >= Milestone::Permit
    }
}

/// One thing the review found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "QaFinding")]
pub struct Finding {
    /// Stable key (rule and element), for marking it resolved.
    pub id: String,
    pub rule: String,
    pub category: Category,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    /// What to do about it.
    pub fix: String,
    /// The code section, standard or practice it's measured against.
    pub reference: String,
    pub elements: Vec<ElementId>,
    /// Where to look: a plan of the element's level, or a sheet.
    pub view: Option<ElementId>,
    /// "rules" or "claude".
    pub source: String,
}

/// What the review is asked to cover.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "QaOptions")]
pub struct Options {
    pub milestone: Milestone,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "QaCount")]
pub struct Count {
    pub category: Category,
    pub critical: usize,
    pub major: usize,
    pub minor: usize,
    pub info: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "QaReport")]
pub struct Report {
    pub milestone: Milestone,
    pub milestone_label: String,
    pub project: String,
    pub code_basis: String,
    pub findings: Vec<Finding>,
    pub counts: Vec<Count>,
    /// 0–100: 100 less 15 a critical, 5 a major, 1 a minor finding.
    pub score: u32,
    pub summary: String,
    /// Claude's overall review, when asked for.
    pub overview: Option<String>,
    pub checked: Vec<String>,
    pub disclaimer: String,
}

/// Reviews the set.
pub fn review(doc: &Document, model: &Model, opts: &Options) -> Report {
    let c = ctx::Ctx::new(doc, model, opts.milestone);
    let mut findings: Vec<Finding> = run_checks(&c)
        .into_iter()
        .filter(|f| opts.categories.contains(&f.category))
        .collect();
    findings.sort_by(|a, b| {
        (a.severity, a.category, &a.title).cmp(&(b.severity, b.category, &b.title))
    });
    report::build(&c, opts, findings)
}

#[cfg(test)]
mod tests;
