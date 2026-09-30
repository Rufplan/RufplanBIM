//! The rules file (`structural_rules.toml`): every threshold, weight and sizing table, so
//! they can be tuned without rebuilding. The built-in copy is the default; the app keeps
//! an editable copy in its data folder.

use std::collections::BTreeMap;

use serde::Deserialize;
use studio_core::structural::{LateralKind, SchemeKind, Seismic};

/// The built-in rules.
pub const DEFAULT_RULES: &str = include_str!("../structural_rules.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    pub general: General,
    pub occupancy: BTreeMap<String, Vec<String>>,
    pub weights: Weights,
    pub schemes: BTreeMap<String, SchemeRules>,
    pub sizing: Sizing,
    pub foundation: Foundation,
}

/// Foundation rules of thumb (ADR-083).
#[derive(Debug, Clone, Deserialize)]
pub struct Foundation {
    pub soil_psf: f64,
    pub frost_depth_in: f64,
    pub floor_psf_light: f64,
    pub floor_psf_heavy: f64,
    pub roof_psf_light: f64,
    pub roof_psf_heavy: f64,
    pub wall_psf: f64,
    pub spread_min_ft: f64,
    pub spread_max_ft: f64,
    pub spread_depth_in: f64,
    pub strip_min_width_in: f64,
    pub strip_min_width_heavy_in: f64,
    pub strip_depth_in: f64,
    pub mat_share: f64,
    pub mat_depth_in: f64,
    pub deep_stories: usize,
    pub pile_kips: f64,
    pub basement_ft: f64,
    pub foundation_wall_in: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct General {
    pub open_zone_keywords: Vec<String>,
    pub open_zone_min_area_sf: f64,
    pub column_free_keywords: Vec<String>,
    pub stair_keywords: Vec<String>,
    pub elevator_keywords: Vec<String>,
    pub shaft_keywords: Vec<String>,
    pub stack_tolerance_in: f64,
    pub stack_overlap: f64,
    pub soft_story_ratio: f64,
    pub setback_ratio: f64,
    pub cantilever_ft: f64,
    pub reentrant_share: f64,
    pub column_snap_ft: f64,
    pub torsion_share: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Weights {
    pub height: f64,
    pub span: f64,
    pub stacking: f64,
    pub occupancy: f64,
    pub discontinuities: f64,
    pub seismic: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BySeismic {
    pub low: f64,
    pub moderate: f64,
    pub high: f64,
}

impl BySeismic {
    pub fn get(&self, s: Seismic) -> f64 {
        match s {
            Seismic::Low => self.low,
            Seismic::Moderate => self.moderate,
            Seismic::High => self.high,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchemeRules {
    pub max_stories: usize,
    pub max_height_ft: f64,
    pub span_min_ft: f64,
    pub span_max_ft: f64,
    pub grid_ft: f64,
    pub bearing_walls: bool,
    pub lateral: LateralKind,
    /// Which sizing table: wood, cfs, podium, steel, concrete, timber.
    pub material: String,
    #[serde(default)]
    pub podium_levels: usize,
    #[serde(default)]
    pub min_stories: usize,
    /// Below this many stories it's usually uneconomical.
    #[serde(default)]
    pub economical_min_stories: usize,
    pub uses: BTreeMap<String, f64>,
    pub seismic: BySeismic,
    pub lateral_ratio: BySeismic,
    pub shear_aspect: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightFrame {
    pub joists: Vec<(f64, String, f64)>,
    pub beam_span_depth: f64,
    pub beam_width_in: f64,
    pub beam_name: String,
    pub posts: Vec<(f64, String, f64)>,
    pub shear_wall: String,
    pub bearing_wall: String,
    pub wall_width_in: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Steel {
    pub beam_span_depth: f64,
    pub girder_span_depth: f64,
    pub beam_spacing_ft: f64,
    pub shapes: Vec<(String, f64, f64)>,
    pub columns: Vec<(f64, String, f64)>,
    pub deck: String,
    pub deck_depth_in: f64,
    pub brace: String,
    pub moment_frame: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Concrete {
    pub slab_span_depth: f64,
    pub slab_min_in: f64,
    pub columns: Vec<(f64, String, f64)>,
    pub shear_wall_in: f64,
    pub transfer_span_depth: f64,
    pub transfer_width_in: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Timber {
    pub panels: Vec<(f64, String, f64)>,
    pub beam_span_depth: f64,
    pub beam_width_in: f64,
    pub columns: Vec<(f64, String, f64)>,
    pub brace: String,
    pub clt_wall: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sizing {
    pub wood: LightFrame,
    pub cfs: LightFrame,
    pub steel: Steel,
    pub concrete: Concrete,
    pub timber: Timber,
}

impl Rules {
    /// Parses a rules file.
    pub fn parse(text: &str) -> Result<Rules, String> {
        let r: Rules = toml::from_str(text).map_err(|e| format!("structural rules: {e}"))?;
        for k in SchemeKind::ALL {
            if !r.schemes.contains_key(k.key()) {
                return Err(format!("structural rules: no [schemes.{}] table", k.key()));
            }
        }
        Ok(r)
    }

    /// The built-in rules.
    pub fn builtin() -> Rules {
        #[allow(clippy::expect_used)]
        Rules::parse(DEFAULT_RULES).expect("the built-in rules parse")
    }

    pub fn scheme(&self, k: SchemeKind) -> &SchemeRules {
        // `parse` checked every scheme has a table.
        #[allow(clippy::expect_used)]
        self.schemes.get(k.key()).expect("scheme rules")
    }
}

/// The first row of a table whose limit reaches `x`, else the last.
pub(crate) fn pick(table: &[(f64, String, f64)], x: f64) -> Option<&(f64, String, f64)> {
    table.iter().find(|r| x <= r.0).or(table.last())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_rules_parse_and_cover_every_scheme() {
        let r = Rules::builtin();
        assert_eq!(r.schemes.len(), 6);
        assert_eq!(r.scheme(SchemeKind::Podium).podium_levels, 1);
        assert_eq!(
            r.scheme(SchemeKind::SteelFrame).lateral,
            LateralKind::BracedFrames
        );
        assert_eq!(pick(&r.sizing.wood.joists, 15.0).unwrap().2, 11.875);
        assert_eq!(pick(&r.sizing.wood.joists, 99.0).unwrap().2, 18.0);
        assert!(Rules::parse("[general]").is_err());
    }
}
