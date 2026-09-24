//! Element store, parameters, types, transactions and undo/redo.

pub mod document;
pub mod element;
pub mod hosting;
pub mod modify;
pub mod ops;
pub mod units;

pub use document::{ChangeSet, CoreError, CoreResult, Document, Tx};
pub use element::{
    Category, Compass, DoorFamily, Element, ElementData, ElementId, ScheduleKind, SheetSize,
    StageChange, ViewKind, WallFunction, WallTop, WindowFamily,
};

/// Version of this crate, from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn crate_version_is_set() {
        assert!(!super::crate_version().is_empty());
    }
}
