//! What the model and Project Info say about the job, as the tags library sections are
//! picked by (`when` in the library files), and the placeholder values sections are filled
//! with. Rules of thumb on names and families: an architect reviews the picks.

use std::collections::{BTreeMap, BTreeSet};

use studio_core::element::{DoorFamily, WindowFamily};
use studio_core::lighting::LightMount;
use studio_core::units::MM_PER_FT;
use studio_core::{Category, Document, ElementData, ElementId};
use studio_regen::Model;

/// Every tag a library section may use.
pub const TAGS: &[&str] = &[
    "always",
    "optional",
    "bidding",
    "commercial",
    "residential",
    "renovation",
    "basement",
    "concrete",
    "masonry",
    "structural_steel",
    "cold_formed_framing",
    "wood_framing",
    "wood_trusses",
    "roof_shingle",
    "roof_metal",
    "roof_membrane",
    "siding_fiber_cement",
    "siding_wood",
    "siding_metal",
    "stucco",
    "doors",
    "doors_wood",
    "doors_stile_rail",
    "doors_hollow_metal",
    "doors_fiberglass",
    "doors_sliding_glass",
    "doors_overhead",
    "storefront",
    "windows",
    "windows_aluminum",
    "windows_vinyl",
    "windows_clad_wood",
    "windows_fiberglass",
    "stairs",
    "stairs_steel",
    "stairs_wood",
    "railings",
    "casework",
    "countertops",
    "gypsum",
    "tile",
    "ceilings_acoustical",
    "flooring_wood",
    "flooring_resilient",
    "flooring_carpet",
    "paint",
    "restrooms_commercial",
    "toilet_accessories",
    "appliances",
    "elevator",
    "sprinklers",
    "fire_alarm",
    "plumbing",
    "hvac",
    "electrical",
    "lighting",
    "exterior_lighting",
    "communications",
    "sitework",
    "paving",
    "planting",
    "termite",
];

/// The job's tags, why each was set, and the placeholder values.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub tags: BTreeSet<&'static str>,
    /// Tag → a short reason ("3 wood doors"), shown next to picked sections.
    pub why: BTreeMap<&'static str, String>,
    pub values: BTreeMap<&'static str, String>,
}

impl Facts {
    fn set(&mut self, tag: &'static str, why: impl Into<String>) {
        if self.tags.insert(tag) {
            self.why.insert(tag, why.into());
        }
    }
    pub fn has(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }
    /// Whether a section's `when` picks it: any entry, each "a+b" entry needing all.
    pub fn picks(&self, when: &[String]) -> bool {
        when.iter()
            .any(|w| w != "optional" && w.split('+').all(|t| self.has(t)))
    }
    /// Why a section was picked: the reason of the first matching entry.
    pub fn reason(&self, when: &[String]) -> Option<String> {
        let w = when
            .iter()
            .find(|w| *w != "optional" && w.split('+').all(|t| self.has(t)))?;
        Some(
            w.split('+')
                .filter_map(|t| self.why.get(t).cloned())
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

/// "A, B, and C".
pub fn series(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        _ => format!(
            "{}, and {}",
            items[..items.len() - 1].join(", "),
            items[items.len() - 1]
        ),
    }
}

fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

/// Type names in use and the text of their layers and materials, lower case.
struct Used {
    names: Vec<String>,
    text: String,
}

fn used(doc: &Document, of: Category) -> Used {
    let mut ids: Vec<ElementId> = doc
        .of(of)
        .filter_map(|e| e.data.type_id())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ids.sort();
    let mut names = vec![];
    let mut text = String::new();
    for id in ids {
        let Ok(d) = doc.data(id) else { continue };
        names.push(d.name());
        text.push_str(&d.name().to_lowercase());
        text.push(' ');
        let layers = match d {
            ElementData::WallType { layers, .. }
            | ElementData::FloorType { layers, .. }
            | ElementData::CeilingType { layers, .. }
            | ElementData::RoofType { layers, .. } => layers.as_slice(),
            _ => &[],
        };
        for l in layers {
            text.push_str(&l.name.to_lowercase());
            text.push(' ');
            if let Some(m) = l.material.and_then(|m| doc.data(m).ok()) {
                text.push_str(&m.name().to_lowercase());
                text.push(' ');
            }
        }
        if let ElementData::ColumnType { material, .. } | ElementData::BeamType { material, .. } = d
        {
            if let Some(m) = material.and_then(|m| doc.data(m).ok()) {
                text.push_str(&m.name().to_lowercase());
                text.push(' ');
            }
        }
    }
    Used { names, text }
}

/// What the project says, from the model, the regenerated model and Project Info.
pub fn facts(doc: &Document, model: &Model) -> Facts {
    let mut f = Facts::default();
    f.set("always", "every project");
    f.set("bidding", "bid documents");
    f.set("concrete", "foundations and slabs");

    let (identity, details) = studio_core::project::get(doc).unwrap_or_default();
    let o = &details.overview;
    let kind = format!("{} {}", o.project_type, details.codes.occupancy).to_lowercase();
    let rooms: Vec<String> = model.rooms.iter().map(|r| r.name.to_lowercase()).collect();
    let room = |words: &[&str]| rooms.iter().filter(|r| has_any(r, words)).count();
    let residential = has_any(
        &kind,
        &[
            "residential",
            "house",
            "home",
            "dwelling",
            "r-3",
            "r-2",
            "r-1",
        ],
    ) || (o.project_type.trim().is_empty() && room(&["bed"]) > 0);
    if residential {
        f.set(
            "residential",
            if o.project_type.is_empty() {
                "bedrooms in the model".to_string()
            } else {
                o.project_type.clone()
            },
        );
    } else {
        f.set(
            "commercial",
            if o.project_type.is_empty() {
                "not a residence".to_string()
            } else {
                o.project_type.clone()
            },
        );
    }
    let work = o.work_type.to_lowercase();
    if has_any(
        &work,
        &[
            "renovation",
            "addition",
            "improvement",
            "reuse",
            "restoration",
            "remodel",
        ],
    ) {
        f.set("renovation", o.work_type.clone());
    }
    let lowest = model
        .levels
        .iter()
        .map(|l| l.elevation)
        .fold(f64::INFINITY, f64::min);
    if lowest <= -3.0 * MM_PER_FT {
        f.set("basement", "a level below grade");
    }

    // Construction, from the types in use.
    let walls = used(doc, Category::Wall);
    let floors = used(doc, Category::Floor);
    let ceilings = used(doc, Category::Ceiling);
    let roofs = used(doc, Category::Roof);
    let frame = format!(
        "{} {}",
        used(doc, Category::Column).text,
        used(doc, Category::Beam).text
    );
    let scheme = doc
        .of(Category::StructuralScheme)
        .find_map(|e| match &e.data {
            ElementData::StructuralScheme { settings, .. } => Some(format!("{:?}", settings.kind)),
            _ => None,
        });
    let scheme = scheme.unwrap_or_default();
    if has_any(
        &walls.text,
        &["brick", "cmu", "masonry", "concrete block", "stone veneer"],
    ) {
        f.set("masonry", "masonry wall types");
    }
    if has_any(&frame, &["steel"]) || scheme.contains("Steel") && !scheme.contains("Cold") {
        f.set(
            "structural_steel",
            if scheme.is_empty() {
                "steel columns or beams".into()
            } else {
                format!("{scheme} scheme")
            },
        );
    }
    if has_any(
        &walls.text,
        &[
            "metal stud",
            "steel stud",
            "cfs",
            "cold-formed",
            "cold formed",
        ],
    ) || scheme == "ColdFormedSteel"
    {
        f.set("cold_formed_framing", "metal stud walls");
    }
    let wood_walls = has_any(&walls.text, &["wood stud", "2x", "wood frame", "timber"]);
    if wood_walls
        || scheme == "LightWood"
        || scheme == "MassTimber"
        || scheme == "Podium"
        || (residential && !f.has("cold_formed_framing") && !f.has("masonry"))
    {
        f.set(
            "wood_framing",
            if wood_walls {
                "wood stud walls"
            } else {
                "wood-framed residence"
            },
        );
    }
    let sloped = doc.of(Category::Roof).any(|e| match &e.data {
        ElementData::Roof { sloped, slope, .. } => sloped.iter().any(|s| *s) && *slope >= 0.25,
        _ => false,
    });
    let low = doc.of(Category::Roof).any(|e| match &e.data {
        ElementData::Roof { sloped, slope, .. } => !sloped.iter().any(|s| *s) || *slope < 0.25,
        _ => false,
    });
    if sloped && f.has("wood_framing") {
        f.set("wood_trusses", "sloped roof over wood framing");
    }
    let r = &roofs.text;
    if has_any(r, &["metal", "standing seam"]) {
        f.set("roof_metal", "metal roof type");
    }
    if has_any(
        r,
        &[
            "tpo",
            "membrane",
            "epdm",
            "built-up",
            "modified bitumen",
            "pvc",
        ],
    ) || low
    {
        f.set("roof_membrane", "low-slope roof");
    }
    if has_any(r, &["shingle", "asphalt"]) || (sloped && !f.has("roof_metal")) {
        f.set("roof_shingle", "sloped roof");
    }
    let w = &walls.text;
    if has_any(
        w,
        &[
            "fiber cement",
            "fibre cement",
            "hardie",
            "cement board siding",
            "lap siding",
        ],
    ) {
        f.set("siding_fiber_cement", "fiber-cement siding");
    }
    if has_any(
        w,
        &[
            "wood siding",
            "cedar",
            "shiplap",
            "board and batten",
            "board & batten",
        ],
    ) {
        f.set("siding_wood", "wood siding");
    }
    if has_any(
        w,
        &["metal panel", "metal siding", "acm", "composite panel"],
    ) {
        f.set("siding_metal", "metal wall panels");
    }
    if has_any(w, &["stucco", "plaster", "eifs"]) {
        f.set("stucco", "stucco walls");
    }
    if !walls.names.is_empty() {
        f.set("paint", "painted walls");
        if has_any(w, &["gypsum", "gwb", "drywall", "wallboard"]) || !walls.names.is_empty() {
            f.set("gypsum", "gypsum board walls");
        }
        f.set("plumbing", "a building");
        f.set("hvac", "a building");
        f.set("electrical", "a building");
        f.set("lighting", "a building");
    }

    // Openings.
    let mut door_types = BTreeSet::new();
    for e in doc.of(Category::Door) {
        let Some(t) = e.data.type_id() else { continue };
        let Ok(ElementData::DoorType {
            name, family, leaf, ..
        }) = doc.data(t)
        else {
            continue;
        };
        door_types.insert(name.clone());
        let n = name.to_lowercase();
        f.set("doors", "doors in the model");
        match family {
            DoorFamily::SlidingGlass | DoorFamily::FoldingWall => {
                f.set("doors_sliding_glass", "sliding or folding glass doors")
            }
            DoorFamily::Garage => f.set("doors_overhead", "garage door"),
            DoorFamily::Storefront => f.set("storefront", "storefront doors"),
            _ => {}
        }
        if has_any(&n, &["hollow metal", "hm ", "steel"])
            || (!residential && matches!(family, DoorFamily::SingleFlush | DoorFamily::DoubleFlush))
        {
            f.set("doors_hollow_metal", "hollow metal doors and frames");
        }
        if has_any(&n, &["fiberglass", "fibreglass"])
            || (residential && *family == DoorFamily::Sidelites)
        {
            f.set("doors_fiberglass", "an entry door");
        }
        if matches!(
            family,
            DoorFamily::SingleFlush
                | DoorFamily::DoubleFlush
                | DoorFamily::Pocket
                | DoorFamily::Bifold
                | DoorFamily::Barn
        ) {
            if *leaf == studio_core::doors::LeafStyle::Flush {
                f.set("doors_wood", "flush doors");
            } else {
                f.set("doors_stile_rail", "panel doors");
            }
        }
    }
    let mut window_types = BTreeSet::new();
    for e in doc.of(Category::Window) {
        let Some(t) = e.data.type_id() else { continue };
        let Ok(ElementData::WindowType {
            name,
            family,
            finish,
            ..
        }) = doc.data(t)
        else {
            continue;
        };
        window_types.insert(name.clone());
        f.set("windows", "windows in the model");
        let n = name.to_lowercase();
        if *family == WindowFamily::Storefront {
            f.set("storefront", "storefront windows");
        } else if n.contains("vinyl") {
            f.set("windows_vinyl", "vinyl windows");
        } else if has_any(&n, &["fiberglass", "fibreglass"]) {
            f.set("windows_fiberglass", "fiberglass windows");
        } else if n.contains("aluminum") || n.contains("aluminium") {
            f.set("windows_aluminum", "aluminum windows");
        } else if *finish == studio_core::windows::FrameFinish::Wood
            || has_any(&n, &["wood", "clad"])
        {
            f.set("windows_clad_wood", "clad-wood windows");
        } else if residential {
            f.set("windows_vinyl", "residential windows");
        } else {
            f.set("windows_aluminum", "commercial windows");
        }
    }

    // Stairs, railings, interiors.
    if doc.of(Category::Stair).next().is_some() {
        f.set("stairs", "stairs in the model");
        if residential {
            f.set("stairs_wood", "residential stair");
        } else {
            f.set("stairs_steel", "commercial stair");
        }
    }
    let stair_rails = doc
        .of(Category::Stair)
        .any(|e| matches!(&e.data, ElementData::Stair { railings: true, .. }));
    if doc.of(Category::Railing).next().is_some() || stair_rails {
        f.set("railings", "railings");
    }
    let kitchens = room(&["kitchen", "pantry", "break"]);
    let baths = room(&[
        "bath", "restroom", "toilet", "wc", "powder", "lavatory", "shower", "men", "women",
    ]);
    if doc.of(Category::Casework).next().is_some() || kitchens + baths > 0 {
        f.set("casework", "kitchen or bath casework");
    }
    if kitchens + baths > 0 {
        f.set("countertops", "kitchen or bath counters");
        f.set("toilet_accessories", "bathrooms");
    }
    let wet = baths + room(&["laundry", "mud"]);
    let fl = &floors.text;
    if wet > 0 || has_any(fl, &["tile", "porcelain", "ceramic"]) {
        f.set("tile", if wet > 0 { "wet rooms" } else { "tile floors" });
    }
    if has_any(
        &ceilings.text,
        &["act", "acoustic", "lay-in", "suspended", "grid"],
    ) {
        f.set("ceilings_acoustical", "acoustical ceilings");
    }
    if has_any(fl, &["wood", "hardwood", "oak", "maple", "plank"]) {
        f.set("flooring_wood", "wood floors");
    }
    if has_any(
        fl,
        &["lvt", "vinyl", "resilient", "vct", "linoleum", "rubber"],
    ) {
        f.set("flooring_resilient", "resilient floors");
    }
    if fl.contains("carpet") {
        f.set("flooring_carpet", "carpet");
    }
    if !(f.has("flooring_wood") || f.has("flooring_resilient") || f.has("flooring_carpet"))
        && !rooms.is_empty()
    {
        if residential {
            f.set("flooring_wood", "residential floors");
        } else {
            f.set("flooring_resilient", "commercial floors");
            f.set("flooring_carpet", "commercial floors");
        }
    }
    if !residential && baths > 0 {
        f.set("restrooms_commercial", "restrooms");
    }
    if residential && kitchens > 0 {
        f.set("appliances", "a kitchen");
    }
    let above = model.levels.iter().filter(|l| l.elevation > -1.0).count();
    if (!residential && above >= 3) || room(&["elevator"]) > 0 {
        f.set("elevator", "three or more stories");
    }
    let spr = details.codes.sprinklered.to_lowercase();
    if spr.contains("13") || (!residential && !spr.contains("not")) {
        f.set(
            "sprinklers",
            if spr.is_empty() {
                "commercial occupancy".to_string()
            } else {
                details.codes.sprinklered.clone()
            },
        );
    }
    if !residential {
        f.set("fire_alarm", "commercial occupancy");
        f.set("communications", "commercial occupancy");
    }
    let outdoor_lights = doc
        .of(Category::LightingFixtureType)
        .any(|e| match &e.data {
            ElementData::LightingFixtureType { spec, name } => {
                spec.mount == LightMount::Ground
                    || has_any(
                        &name.to_lowercase(),
                        &[
                            "exterior",
                            "outdoor",
                            "site",
                            "bollard",
                            "pole",
                            "landscape",
                        ],
                    )
            }
            _ => false,
        });
    if outdoor_lights || !residential {
        f.set("exterior_lighting", "exterior lighting");
    }
    if !work.contains("tenant") {
        f.set("sitework", "site work");
    }
    if !residential && f.has("sitework") {
        f.set("paving", "parking and drives");
    }
    if doc.of(Category::Planting).next().is_some() || doc.of(Category::GrassPatch).next().is_some()
    {
        f.set("planting", "plantings in the model");
    }
    if f.has("wood_framing") && residential {
        f.set("termite", "wood-framed residence");
    }

    // Placeholders.
    let architect = details
        .team
        .iter()
        .find(|m| m.discipline.to_lowercase() == "architect")
        .map(|m| {
            if m.contact.company.is_empty() {
                m.contact.name.clone()
            } else {
                m.contact.company.clone()
            }
        })
        .unwrap_or_default();
    let address = {
        let a = details.location.one_line();
        if a.is_empty() {
            identity.address.clone()
        } else {
            a
        }
    };
    let owner = if details.client.company.is_empty() {
        details.client.name.clone()
    } else {
        details.client.company.clone()
    };
    let list = |v: Vec<String>| series(&v);
    let vals: [(&'static str, String); 13] = [
        ("project_name", identity.name.clone()),
        ("project_number", identity.number.clone()),
        ("project_address", address),
        ("owner", owner),
        ("architect", architect),
        ("jurisdiction", details.location.jurisdiction.clone()),
        ("building_code", details.codes.building_code.clone()),
        ("door_types", list(door_types.into_iter().collect())),
        ("window_types", list(window_types.into_iter().collect())),
        ("wall_types", list(walls.names)),
        ("roof_types", list(roofs.names)),
        ("floor_types", list(floors.names)),
        ("ceiling_types", list(ceilings.names)),
    ];
    for (k, v) in vals {
        f.values.insert(k, v);
    }
    f
}

/// Fills `{placeholders}`; one without a value becomes "[to be determined]".
pub fn fill(text: &str, facts: &Facts) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j)
                if after[..j]
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_')
                    && j > 0 =>
            {
                let key = &after[..j];
                match facts.values.get(key).filter(|v| !v.trim().is_empty()) {
                    Some(v) => out.push_str(v),
                    None if facts.values.contains_key(key) => out.push_str("[to be determined]"),
                    None => out.push_str(&rest[i..i + j + 2]),
                }
                rest = &after[j + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_fill_or_say_to_be_determined() {
        let mut f = Facts::default();
        f.values.insert("project_name", "Oak House".into());
        f.values.insert("owner", String::new());
        assert_eq!(
            fill("{project_name} for {owner}; {unknown} {x", &f),
            "Oak House for [to be determined]; {unknown} {x"
        );
        assert_eq!(series(&["A".into(), "B".into(), "C".into()]), "A, B, and C");
        assert_eq!(series(&["A".into(), "B".into()]), "A and B");
    }

    #[test]
    fn picks_need_every_joined_tag_and_never_optional() {
        let mut f = Facts::default();
        f.set("casework", "kitchen");
        f.set("residential", "house");
        assert!(f.picks(&["casework+residential".into()]));
        assert!(!f.picks(&["casework+commercial".into()]));
        assert!(!f.picks(&["optional".into()]));
        assert_eq!(
            f.reason(&["casework+residential".into()]).as_deref(),
            Some("kitchen; house")
        );
    }

    #[test]
    fn a_small_house_is_a_wood_framed_residence_with_doors_and_windows() {
        let doc = crate::testkit::house();
        let model = studio_regen::regenerate(&doc);
        let f = facts(&doc, &model);
        for t in [
            "residential",
            "doors",
            "windows",
            "gypsum",
            "paint",
            "plumbing",
            "wood_framing",
        ] {
            assert!(f.has(t), "{t}: {:?}", f.tags);
        }
        assert!(!f.has("commercial") && !f.has("fire_alarm"));
        assert!(!f.values["door_types"].is_empty());
    }
}
