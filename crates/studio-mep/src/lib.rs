//! MEPT suggestions (ADR-082): from the architectural model to preliminary mechanical,
//! electrical, plumbing and technology systems, in pure, deterministic stages like
//! Suggest Structure: model → [`features`] → a discipline's proposal → its layout.
//!
//! Preliminary — not engineered. Requires review by a licensed engineer. Nothing here
//! claims code compliance, and nothing changes the architectural model.

pub mod common;
pub mod electrical;
pub mod features;
pub mod mechanical;
pub mod plumbing;
pub mod rules;
pub mod technology;
pub mod types;

pub use features::extract;
pub use rules::Rules;
pub use types::*;

use studio_core::mep::{Climate, Discipline, MepLayout, MepSettings};

/// A discipline's Suggest: every candidate system scored and ranked.
pub fn propose(d: Discipline, f: &Features, rules: &Rules, climate: Climate) -> MepProposal {
    match d {
        Discipline::Mechanical => mechanical::propose(f, rules, climate),
        Discipline::Electrical => electrical::propose(f, rules, climate),
        Discipline::Plumbing => plumbing::propose(f, rules, climate),
        Discipline::Technology => technology::propose(f, rules, climate),
    }
}

/// The preliminary layout for the chosen system.
pub fn layout(f: &Features, rules: &Rules, s: &MepSettings) -> MepLayout {
    match s.discipline {
        Discipline::Mechanical => mechanical::layout(f, rules, s),
        Discipline::Electrical => electrical::layout(f, rules, s),
        Discipline::Plumbing => plumbing::layout(f, rules, s),
        Discipline::Technology => technology::layout(f, rules, s),
    }
}

#[cfg(test)]
mod tests;
