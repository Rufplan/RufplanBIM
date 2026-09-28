//! Element store, parameters, types, transactions and undo/redo.

pub mod build;
pub mod camera;
pub mod compound;
pub mod detail;
pub mod dimension;
pub mod document;
pub mod doors;
pub mod edit;
pub mod element;
pub mod generate;
pub mod grass;
pub mod hosting;
pub mod library;
pub mod lighting;
pub mod lines;
pub mod material;
pub mod model_edit;
pub mod modify;
pub mod ops;
pub mod paint;
pub mod params;
pub mod plans;
pub mod planting;
pub mod site;
pub mod sketch;
pub mod slope;
pub mod standards;
pub mod standards_catalog;
pub mod structure;
pub mod symbols;
pub mod units;
pub mod visibility;
pub mod wall_opening;
pub mod windows;

pub use document::{ChangeSet, CoreError, CoreResult, DerivedCache, Document, Tx};
pub use element::{
    Anchor, BeamShape, Category, ColumnShape, Compass, CropBox, CutPattern, DimKind, DimRef,
    DoorFamily, Element, ElementData, ElementId, FloorSlope, LayerFunction, LevelEnds,
    LocationLine, MarkStyle, RufplanLink, ScheduleKind, SectionBox, SheetSize, SlabBound,
    SlopeFormat, StageChange, StairShape, SurfacePattern, ViewKind, WallFunction, WallLayer,
    WallTop, WindowFamily,
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
