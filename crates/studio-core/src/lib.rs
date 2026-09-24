//! Element store, parameters, types, transactions and undo/redo.

pub mod build;
pub mod compound;
pub mod document;
pub mod edit;
pub mod element;
pub mod hosting;
pub mod modify;
pub mod ops;
pub mod params;
pub mod units;

pub use document::{ChangeSet, CoreError, CoreResult, DerivedCache, Document, Tx};
pub use element::{
    Anchor, Category, Compass, CropBox, DoorFamily, Element, ElementData, ElementId, LayerFunction,
    RufplanLink, ScheduleKind, SheetSize, StageChange, ViewKind, WallFunction, WallLayer, WallTop,
    WindowFamily,
};
pub use params::{ParamDef, ParamKind, ParamScope, ParamValue};

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
