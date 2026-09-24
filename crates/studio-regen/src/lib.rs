//! Dependency graph, dirty tracking and incremental regeneration.

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
