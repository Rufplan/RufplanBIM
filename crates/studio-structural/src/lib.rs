//! Suggest Structure (ADR-080): from the architectural model to a preliminary structural
//! scheme, in pure, deterministic stages, each testable on its own:
//! model → [`extract`] features → [`schemes`] proposal → [`layout`] (with [`sizing`]).
//!
//! Preliminary — not engineered. Requires review by a licensed structural engineer. Nothing
//! here claims code compliance, and nothing changes the architectural model.

pub mod extract;
pub mod foundation;
pub mod layout;
pub mod rules;
pub mod schemes;
pub mod sizing;
pub mod types;

pub use extract::extract;
pub use layout::layout;
pub use rules::Rules;
pub use schemes::propose;
pub use types::*;

#[cfg(test)]
mod tests;
