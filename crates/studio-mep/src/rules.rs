//! The rules file (`mep_rules.toml`): space loads, fixtures, each discipline's systems and
//! sizing, tunable without rebuilding. The built-in copy is the default; the app keeps an
//! editable copy in its data folder.

use std::collections::BTreeMap;

use serde::Deserialize;
use studio_core::mep::{Climate, Discipline};

pub const DEFAULT_RULES: &str = include_str!("../mep_rules.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    pub general: General,
    pub space: Vec<SpaceRules>,
    pub fixtures: BTreeMap<String, (f64, f64, f64)>,
    pub mechanical: Mechanical,
    pub electrical: Electrical,
    pub plumbing: Plumbing,
    pub technology: Technology,
}

#[derive(Debug, Clone, Deserialize)]
pub struct General {
    pub exterior_tol_in: f64,
    pub ceiling_ft: f64,
    pub ceiling_ft_residential: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpaceRules {
    pub kind: String,
    pub keywords: Vec<String>,
    pub sf_per_ton: f64,
    pub light_w_sf: f64,
    pub recept_va_sf: f64,
    pub sf_per_light: f64,
    pub data_drops: u32,
    #[serde(default)]
    pub data_per_1000sf: f64,
    #[serde(default)]
    pub fixtures: Vec<String>,
    #[serde(default, rename = "use")]
    pub use_: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ByClimate {
    pub hot: f64,
    pub mixed: f64,
    pub cold: f64,
}

impl ByClimate {
    pub fn get(&self, c: Climate) -> f64 {
        match c {
            Climate::Hot => self.hot,
            Climate::Mixed => self.mixed,
            Climate::Cold => self.cold,
        }
    }
}

/// A candidate system of any discipline; each uses the fields that apply to it.
#[derive(Debug, Clone, Deserialize)]
pub struct SystemRules {
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub min_area_sf: f64,
    pub max_area_sf: f64,
    #[serde(default)]
    pub min_stories: usize,
    pub max_stories: usize,
    pub uses: BTreeMap<String, f64>,
    #[serde(default)]
    pub climate: Option<ByClimate>,
    // Mechanical
    #[serde(default)]
    pub ducted: bool,
    #[serde(default)]
    pub exterior: bool,
    #[serde(default)]
    pub rooftop: bool,
    #[serde(default)]
    pub central: bool,
    #[serde(default)]
    pub per_room: bool,
    // Electrical
    #[serde(default)]
    pub min_amps: f64,
    #[serde(default)]
    pub max_amps: f64,
    #[serde(default)]
    pub volts: f64,
    #[serde(default)]
    pub phases: u8,
    #[serde(default)]
    pub meters: bool,
    #[serde(default)]
    pub transformers: bool,
    // Plumbing
    #[serde(default)]
    pub heater: String,
    #[serde(default)]
    pub min_units: usize,
    #[serde(default)]
    pub max_units: usize,
    // Technology
    #[serde(default)]
    pub media_panel: bool,
    #[serde(default)]
    pub idf_per_floor: bool,
    #[serde(default)]
    pub fiber: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mechanical {
    pub supply_cfm_per_ton: f64,
    pub cfm_per_diffuser: f64,
    pub duct_velocity_fpm: f64,
    pub duct_depth_in: f64,
    pub max_split_tons: f64,
    pub max_rtu_tons: f64,
    pub max_vrf_tons: f64,
    pub heads_per_condenser: usize,
    pub min_plenum_ft: f64,
    pub systems: BTreeMap<String, SystemRules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Electrical {
    pub demand_factor: f64,
    pub hvac_va_sf: f64,
    pub dwelling_appliances_va: f64,
    pub service_sizes: Vec<f64>,
    pub receptacle_spacing_ft: f64,
    pub commercial_receptacle_spacing_ft: f64,
    pub max_feeder_ft: f64,
    pub systems: BTreeMap<String, SystemRules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Plumbing {
    pub tank_gallons_first_bath: f64,
    pub tank_gallons_per_bath: f64,
    pub recirc_ft: f64,
    pub stack_cluster_ft: f64,
    pub service_sizes: Vec<(f64, String)>,
    pub stack_sizes: Vec<(f64, String)>,
    pub drain_sizes: Vec<(f64, String)>,
    pub systems: BTreeMap<String, SystemRules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Technology {
    pub max_cable_ft: f64,
    pub ap_sf_residential: f64,
    pub ap_sf_commercial: f64,
    pub cameras_at_doors: bool,
    pub systems: BTreeMap<String, SystemRules>,
}

impl Rules {
    pub fn parse(text: &str) -> Result<Rules, String> {
        let r: Rules = toml::from_str(text).map_err(|e| format!("MEPT rules: {e}"))?;
        for d in Discipline::ALL {
            if r.systems(d).is_empty() {
                return Err(format!("MEPT rules: no [{}.systems] tables", d.key()));
            }
        }
        if !r.space.iter().any(|s| s.keywords.is_empty()) {
            return Err("MEPT rules: add a catch-all [[space]] with no keywords".into());
        }
        Ok(r)
    }

    pub fn builtin() -> Rules {
        #[allow(clippy::expect_used)]
        Rules::parse(DEFAULT_RULES).expect("the built-in MEPT rules parse")
    }

    pub fn systems(&self, d: Discipline) -> &BTreeMap<String, SystemRules> {
        match d {
            Discipline::Mechanical => &self.mechanical.systems,
            Discipline::Electrical => &self.electrical.systems,
            Discipline::Plumbing => &self.plumbing.systems,
            Discipline::Technology => &self.technology.systems,
        }
    }

    /// The rules of the space type called `kind` (the catch-all when unknown).
    pub fn space(&self, kind: &str) -> &SpaceRules {
        self.space
            .iter()
            .find(|s| s.kind == kind)
            .or_else(|| self.space.iter().find(|s| s.keywords.is_empty()))
            .unwrap_or(&self.space[0])
    }

    /// The space type a room name reads as: whole-word keyword matches, first type wins.
    pub fn kind_of(&self, name: &str) -> String {
        let n = name.to_lowercase();
        let words: Vec<&str> = n
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let joined = format!(" {} ", words.join(" "));
        for s in &self.space {
            if s.keywords.iter().any(|k| {
                let k = k.to_lowercase();
                joined.contains(&format!(" {k} "))
                    // Plurals and compounds: "bedrooms", "bathroom2".
                    || words.iter().any(|w| k.len() >= 4 && w.len() > k.len() && w.starts_with(&k))
            }) {
                return s.kind.clone();
            }
        }
        self.space
            .iter()
            .find(|s| s.keywords.is_empty())
            .map_or_else(|| "Other".into(), |s| s.kind.clone())
    }
}

/// The first row of a size table whose limit reaches `x`, else the last.
pub(crate) fn pick_size(table: &[(f64, String)], x: f64) -> String {
    table
        .iter()
        .find(|r| x <= r.0)
        .or(table.last())
        .map(|r| r.1.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_rules_parse_and_read_room_names_by_whole_words() {
        let r = Rules::builtin();
        assert_eq!(r.systems(Discipline::Mechanical).len(), 6);
        assert_eq!(r.kind_of("Master Bedroom"), "Bedroom");
        assert_eq!(r.kind_of("BEDROOMS 2"), "Bedroom");
        assert_eq!(r.kind_of("Powder Room"), "Bathroom");
        assert_eq!(r.kind_of("Mens Restroom"), "Restroom");
        // "Basement" doesn't read as a men's room, nor "Studio" as anything but living.
        assert_eq!(r.kind_of("Basement"), "Other");
        assert_eq!(r.kind_of("Break Room"), "Break Room");
        assert_eq!(r.kind_of("Tel / Data"), "Telecom");
        assert_eq!(pick_size(&r.plumbing.stack_sizes, 30.0), "4\"");
        assert_eq!(r.fixtures["Water Closet"].1, 3.0);
    }
}
