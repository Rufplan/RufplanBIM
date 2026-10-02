//! Planting (ADR-064), after Enscape's Asset Library and Revit's Planting category.
//! - A planting type is a species: its group (deciduous, conifer, palm, shrub…), crown form,
//!   foliage, size, trunk and colours. studio-views grows its 3D model from these (a
//!   branching skeleton with leaf cards), its plan symbol and its elevation outline.
//! - The Asset Library lists typical trees, shrubs, hedges, grasses, perennials and
//!   succulents for every climate. Like Enscape's, deciduous and flowering trees come in
//!   season variants: spring, summer, autumn colour and bare winter branches.
//! - A planting stands on its level, or on the topography where the level is at grade.
//! - The base ground (the topography, or the ground around the model) and sketched ground
//!   regions (Revit's subregions: drives, lawns, beds) are finished in materials, the Site &
//!   Landscape materials among them. Enscape's Grass material type grows 3D blades on them.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::lighting::{num, opts, parse_num};
use crate::ops::{choice, len, level_options, non_empty, parse_id, parse_len, positive, ro, text};
use crate::ops::{PropOption, Property};

const FT: f64 = 304.8;
const IN: f64 = 25.4;

/// The Asset Library's groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum PlantGroup {
    Deciduous,
    Flowering,
    Evergreen,
    Conifer,
    Palm,
    Shrub,
    Hedge,
    Grass,
    Perennial,
    Succulent,
    /// Boulders and stones, set among the plants (ADR-095).
    Rock,
}

impl PlantGroup {
    pub const ALL: [PlantGroup; 11] = [
        PlantGroup::Deciduous,
        PlantGroup::Flowering,
        PlantGroup::Evergreen,
        PlantGroup::Conifer,
        PlantGroup::Palm,
        PlantGroup::Shrub,
        PlantGroup::Hedge,
        PlantGroup::Grass,
        PlantGroup::Perennial,
        PlantGroup::Succulent,
        PlantGroup::Rock,
    ];
    pub fn label(self) -> &'static str {
        match self {
            PlantGroup::Deciduous => "Deciduous Trees",
            PlantGroup::Flowering => "Flowering Trees",
            PlantGroup::Evergreen => "Broadleaf Evergreens",
            PlantGroup::Conifer => "Conifers",
            PlantGroup::Palm => "Palms",
            PlantGroup::Shrub => "Shrubs & Bushes",
            PlantGroup::Hedge => "Hedges",
            PlantGroup::Grass => "Ornamental Grasses",
            PlantGroup::Perennial => "Flowers & Perennials",
            PlantGroup::Succulent => "Succulents & Cacti",
            PlantGroup::Rock => "Rocks & Boulders",
        }
    }
    /// Enscape's top-level vegetation categories.
    pub fn category(self) -> &'static str {
        match self {
            PlantGroup::Deciduous
            | PlantGroup::Flowering
            | PlantGroup::Evergreen
            | PlantGroup::Conifer
            | PlantGroup::Palm => "Trees",
            PlantGroup::Shrub | PlantGroup::Hedge => "Bushes",
            PlantGroup::Grass | PlantGroup::Perennial => "Grass & Flowers",
            PlantGroup::Succulent | PlantGroup::Rock => "Plants",
        }
    }
    pub fn is_tree(self) -> bool {
        self.category() == "Trees"
    }
    /// Whether it drops its leaves (and so has season variants).
    pub fn seasonal(self) -> bool {
        matches!(self, PlantGroup::Deciduous | PlantGroup::Flowering)
    }
}

/// The crown's shape (the envelope the branches fill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum CrownForm {
    Round,
    Oval,
    Vase,
    Columnar,
    Pyramidal,
    Conical,
    Weeping,
    Spreading,
    Umbrella,
    Irregular,
    PalmHead,
    Mound,
    Upright,
    Fountain,
    Rosette,
    Box,
    Cactus,
    /// Boulders: weathered stones, one or a cluster (`stems`), half set into the ground.
    Boulder,
}

impl CrownForm {
    pub const ALL: [CrownForm; 18] = [
        CrownForm::Round,
        CrownForm::Oval,
        CrownForm::Vase,
        CrownForm::Columnar,
        CrownForm::Pyramidal,
        CrownForm::Conical,
        CrownForm::Weeping,
        CrownForm::Spreading,
        CrownForm::Umbrella,
        CrownForm::Irregular,
        CrownForm::PalmHead,
        CrownForm::Mound,
        CrownForm::Upright,
        CrownForm::Fountain,
        CrownForm::Rosette,
        CrownForm::Box,
        CrownForm::Cactus,
        CrownForm::Boulder,
    ];
    pub fn label(self) -> &'static str {
        match self {
            CrownForm::Round => "Round",
            CrownForm::Oval => "Oval",
            CrownForm::Vase => "Vase",
            CrownForm::Columnar => "Columnar",
            CrownForm::Pyramidal => "Pyramidal",
            CrownForm::Conical => "Conical",
            CrownForm::Weeping => "Weeping",
            CrownForm::Spreading => "Spreading",
            CrownForm::Umbrella => "Umbrella",
            CrownForm::Irregular => "Irregular",
            CrownForm::PalmHead => "Palm",
            CrownForm::Mound => "Mounded",
            CrownForm::Upright => "Upright",
            CrownForm::Fountain => "Fountain",
            CrownForm::Rosette => "Rosette",
            CrownForm::Box => "Clipped (box)",
            CrownForm::Cactus => "Columnar cactus",
            CrownForm::Boulder => "Boulder",
        }
    }
}

/// The leaves, as the leaf cards' texture draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Foliage {
    Ovate,
    Lanceolate,
    Lobed,
    Palmate,
    Heart,
    Compound,
    Small,
    Rounded,
    Needle,
    Scale,
    Frond,
    Fan,
    Blade,
    Fleshy,
}

/// The bark's texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Bark {
    Smooth,
    Furrowed,
    Plated,
    Papery,
    Mottled,
    Fibrous,
    Palm,
    Green,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Season {
    Spring,
    #[default]
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn label(self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Autumn => "Autumn",
            Season::Winter => "Winter",
        }
    }
}

/// Where a species grows well, for the library's filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Climate {
    Cold,
    Temperate,
    Mediterranean,
    Tropical,
    Arid,
}

impl Climate {
    pub const ALL: [Climate; 5] = [
        Climate::Cold,
        Climate::Temperate,
        Climate::Mediterranean,
        Climate::Tropical,
        Climate::Arid,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Climate::Cold => "Cold / Northern",
            Climate::Temperate => "Temperate",
            Climate::Mediterranean => "Mediterranean / Warm",
            Climate::Tropical => "Tropical",
            Climate::Arid => "Arid / Desert",
        }
    }
}

/// A species and how it looks (mm, sRGB colours).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlantSpec {
    pub botanical: String,
    pub group: PlantGroup,
    pub form: CrownForm,
    pub foliage: Foliage,
    pub bark_kind: Bark,
    /// Overall height and crown spread.
    pub height: f64,
    pub spread: f64,
    /// A clipped hedge's depth across (its spread is its length); 0 otherwise.
    #[serde(default)]
    pub depth: f64,
    /// Clear trunk up to the lowest branches.
    pub trunk: f64,
    /// Trunk diameter at the base.
    pub caliper: f64,
    /// Trunks (multi-stem) or, for shrubs and grasses, main stems.
    pub stems: u32,
    pub leaf: [u8; 3],
    pub leaf_alt: [u8; 3],
    pub bark: [u8; 3],
    #[ts(optional)]
    pub flowers: Option<[u8; 3]>,
    #[ts(optional)]
    pub autumn: Option<[u8; 3]>,
    #[serde(default)]
    pub season: Season,
    /// How full the crown is, 0.2 (open) to 1 (dense).
    pub density: f64,
}

impl PlantSpec {
    /// Whether it shows leaves (bare in winter).
    pub fn leafy(&self) -> bool {
        !(self.season == Season::Winter && self.group.seasonal())
    }
    /// The leaves' colours for its season.
    pub fn leaf_colors(&self) -> ([u8; 3], [u8; 3]) {
        match (self.season, self.autumn) {
            (Season::Autumn, Some(a)) => (a, mix(a, [214, 150, 48], 0.35)),
            (Season::Spring, _) if self.group.seasonal() => (
                mix(self.leaf, [150, 186, 84], 0.35),
                mix(self.leaf_alt, [176, 200, 100], 0.35),
            ),
            _ => (self.leaf, self.leaf_alt),
        }
    }
    /// The flowers it shows now (flowering trees bloom in spring, in their spring variant
    /// and the base asset; shrubs and perennials all season).
    pub fn flowers_now(&self) -> Option<[u8; 3]> {
        match (self.group, self.season) {
            (_, Season::Winter | Season::Autumn) if self.group.seasonal() => None,
            (PlantGroup::Flowering, Season::Summer) => None,
            _ => self.flowers,
        }
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

/// An Asset Library entry.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PlantPreset {
    pub name: String,
    /// The species' name without its season, grouping its variants.
    pub species: String,
    pub description: String,
    pub climates: Vec<Climate>,
    pub spec: PlantSpec,
}

// ---------------------------------------------------------------- the library

struct B(PlantPreset);

#[allow(clippy::too_many_arguments)]
fn plant(
    name: &str,
    botanical: &str,
    group: PlantGroup,
    form: CrownForm,
    foliage: Foliage,
    h_ft: f64,
    w_ft: f64,
    leaf: [u8; 3],
) -> B {
    use PlantGroup::*;
    let height = h_ft * FT;
    let tree = group.is_tree();
    let trunk = match group {
        Conifer => height * 0.06,
        Palm => height * 0.85,
        _ if tree => height * 0.3,
        _ => 0.0,
    };
    let caliper = match group {
        Palm => (height / 55.0).clamp(150.0, 900.0),
        _ if tree => (height / 42.0).clamp(60.0, 1800.0),
        _ => 18.0,
    };
    let stems = match group {
        Shrub | Hedge => 5,
        Grass | Perennial => 1,
        _ => 1,
    };
    let bark_kind = match group {
        Palm => Bark::Palm,
        Conifer => Bark::Plated,
        Succulent => Bark::Green,
        _ => Bark::Furrowed,
    };
    B(PlantPreset {
        name: name.into(),
        species: name.into(),
        description: String::new(),
        climates: vec![Climate::Temperate],
        spec: PlantSpec {
            botanical: botanical.into(),
            group,
            form,
            foliage,
            bark_kind,
            height,
            spread: w_ft * FT,
            depth: 0.0,
            trunk,
            caliper,
            stems,
            leaf,
            leaf_alt: mix(leaf, [168, 176, 96], 0.25),
            bark: [92, 82, 72],
            flowers: None,
            autumn: None,
            season: Season::Summer,
            density: 0.8,
        },
    })
}

impl B {
    fn d(mut self, s: &str) -> Self {
        self.0.description = s.into();
        self
    }
    fn alt(mut self, c: [u8; 3]) -> Self {
        self.0.spec.leaf_alt = c;
        self
    }
    fn bark(mut self, kind: Bark, c: [u8; 3]) -> Self {
        self.0.spec.bark_kind = kind;
        self.0.spec.bark = c;
        self
    }
    fn trunk(mut self, ft: f64) -> Self {
        self.0.spec.trunk = ft * FT;
        self
    }
    fn cal(mut self, inches: f64) -> Self {
        self.0.spec.caliper = inches * IN;
        self
    }
    fn stems(mut self, n: u32) -> Self {
        self.0.spec.stems = n;
        self
    }
    fn flowers(mut self, c: [u8; 3]) -> Self {
        self.0.spec.flowers = Some(c);
        self
    }
    fn autumn(mut self, c: [u8; 3]) -> Self {
        self.0.spec.autumn = Some(c);
        self
    }
    fn dense(mut self, d: f64) -> Self {
        self.0.spec.density = d;
        self
    }
    fn depth(mut self, ft: f64) -> Self {
        self.0.spec.depth = ft * FT;
        self
    }
    fn clim(mut self, c: &[Climate]) -> Self {
        self.0.climates = c.to_vec();
        self
    }
}

// Typical colours.
const GREEN: [u8; 3] = [72, 104, 48];
const GLOSSY: [u8; 3] = [40, 66, 34];
const SILVER: [u8; 3] = [120, 134, 104];
const SPRUCE: [u8; 3] = [40, 66, 46];
const RED_FALL: [u8; 3] = [178, 48, 30];
const ORANGE_FALL: [u8; 3] = [206, 106, 34];
const GOLD_FALL: [u8; 3] = [214, 170, 50];
const RUST_FALL: [u8; 3] = [150, 78, 38];
const WHITE: [u8; 3] = [246, 244, 238];
const GRAY_BARK: [u8; 3] = [118, 114, 106];
const DARK_BARK: [u8; 3] = [62, 54, 48];
const BIRCH: [u8; 3] = [226, 222, 212];

/// The species, each in its showcase look (flowering trees in bloom).
fn species() -> Vec<PlantPreset> {
    use Climate::*;
    use CrownForm::*;
    use Foliage::*;
    use PlantGroup::*;
    let warm = &[Mediterranean, Tropical][..];
    let mild = &[Temperate, Mediterranean][..];
    let cold = &[Cold, Temperate][..];
    let dry = &[Arid, Mediterranean][..];
    let v: Vec<B> = vec![
        // Deciduous trees
        plant(
            "Red Maple",
            "Acer rubrum",
            Deciduous,
            Oval,
            Palmate,
            50.0,
            35.0,
            GREEN,
        )
        .autumn(RED_FALL)
        .bark(Bark::Smooth, GRAY_BARK)
        .clim(cold)
        .d("Fast, adaptable street and yard tree; brilliant red in fall."),
        plant(
            "Sugar Maple",
            "Acer saccharum",
            Deciduous,
            Round,
            Palmate,
            65.0,
            45.0,
            [66, 102, 44],
        )
        .autumn(ORANGE_FALL)
        .clim(cold)
        .d("Dense, stately shade tree with orange-gold fall colour."),
        plant(
            "Norway Maple",
            "Acer platanoides",
            Deciduous,
            Round,
            Palmate,
            45.0,
            40.0,
            [58, 92, 42],
        )
        .autumn(GOLD_FALL)
        .dense(0.95)
        .clim(cold)
        .d("Very dense rounded crown; urban shade tree."),
        plant(
            "Japanese Maple, Red",
            "Acer palmatum 'Bloodgood'",
            Deciduous,
            Spreading,
            Palmate,
            18.0,
            20.0,
            [118, 34, 38],
        )
        .alt([150, 48, 46])
        .autumn([196, 40, 36])
        .trunk(3.0)
        .stems(3)
        .cal(5.0)
        .bark(Bark::Smooth, [96, 84, 84])
        .d("Layered specimen tree for courtyards and entries; deep red leaves."),
        plant(
            "Japanese Maple, Green",
            "Acer palmatum",
            Deciduous,
            Spreading,
            Palmate,
            18.0,
            20.0,
            [104, 146, 58],
        )
        .autumn([222, 92, 36])
        .trunk(3.0)
        .stems(3)
        .cal(5.0)
        .bark(Bark::Smooth, [110, 104, 96])
        .d("Delicate layered canopy, fine lacy leaves; orange-red in fall."),
        plant(
            "Paperbark Maple",
            "Acer griseum",
            Deciduous,
            Oval,
            Compound,
            25.0,
            18.0,
            [70, 100, 50],
        )
        .autumn([190, 58, 36])
        .bark(Bark::Papery, [160, 90, 60])
        .clim(cold)
        .d("Small tree with peeling cinnamon bark; four-season interest."),
        plant(
            "White Oak",
            "Quercus alba",
            Deciduous,
            Spreading,
            Lobed,
            70.0,
            75.0,
            [70, 100, 48],
        )
        .autumn(RUST_FALL)
        .bark(Bark::Furrowed, [128, 122, 110])
        .trunk(14.0)
        .clim(cold)
        .d("Broad, majestic oak with a massive spreading crown."),
        plant(
            "Northern Red Oak",
            "Quercus rubra",
            Deciduous,
            Round,
            Lobed,
            70.0,
            55.0,
            [64, 94, 44],
        )
        .autumn([150, 52, 34])
        .bark(Bark::Furrowed, DARK_BARK)
        .clim(cold)
        .d("Fast-growing rounded oak; russet-red fall colour."),
        plant(
            "Pin Oak",
            "Quercus palustris",
            Deciduous,
            Pyramidal,
            Lobed,
            60.0,
            40.0,
            [62, 96, 44],
        )
        .autumn([140, 46, 34])
        .trunk(8.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .d("Pyramidal oak with drooping lower branches."),
        plant(
            "Willow Oak",
            "Quercus phellos",
            Deciduous,
            Oval,
            Lanceolate,
            60.0,
            40.0,
            [76, 110, 52],
        )
        .autumn([184, 140, 60])
        .clim(mild)
        .d("Fine-textured oak with willow-like leaves; southern street tree."),
        plant(
            "Swamp White Oak",
            "Quercus bicolor",
            Deciduous,
            Round,
            Lobed,
            55.0,
            55.0,
            [60, 92, 44],
        )
        .autumn([170, 120, 50])
        .clim(cold)
        .d("Tough rounded oak for wet sites and parks."),
        plant(
            "River Birch",
            "Betula nigra",
            Deciduous,
            Oval,
            Ovate,
            40.0,
            30.0,
            [84, 118, 52],
        )
        .autumn(GOLD_FALL)
        .stems(3)
        .trunk(6.0)
        .cal(6.0)
        .bark(Bark::Papery, [170, 120, 96])
        .d("Multi-stem birch with peeling salmon bark; tolerates wet soil."),
        plant(
            "Paper Birch",
            "Betula papyrifera",
            Deciduous,
            Oval,
            Ovate,
            50.0,
            30.0,
            [88, 124, 54],
        )
        .autumn([226, 190, 70])
        .bark(Bark::Papery, BIRCH)
        .trunk(12.0)
        .clim(&[Cold])
        .d("Chalk-white bark, open oval crown; northern landscapes."),
        plant(
            "Silver Birch",
            "Betula pendula",
            Deciduous,
            Weeping,
            Small,
            45.0,
            25.0,
            [96, 130, 58],
        )
        .autumn([226, 196, 76])
        .bark(Bark::Papery, [230, 228, 220])
        .trunk(10.0)
        .dense(0.6)
        .clim(cold)
        .d("Graceful birch with drooping twigs and white bark."),
        plant(
            "Birch Clump",
            "Betula jacquemontii",
            Deciduous,
            Columnar,
            Ovate,
            35.0,
            18.0,
            [86, 122, 56],
        )
        .autumn([224, 188, 68])
        .stems(3)
        .trunk(8.0)
        .cal(5.0)
        .bark(Bark::Papery, [240, 238, 232])
        .dense(0.65)
        .clim(cold)
        .d("Brilliant white multi-stem birch; a modern landscape favourite."),
        plant(
            "Honey Locust",
            "Gleditsia triacanthos inermis",
            Deciduous,
            Spreading,
            Compound,
            50.0,
            40.0,
            [104, 138, 56],
        )
        .autumn([222, 196, 70])
        .dense(0.45)
        .trunk(10.0)
        .clim(cold)
        .d("Airy, filtered shade from tiny leaflets; tough plaza tree."),
        plant(
            "London Plane Tree",
            "Platanus × acerifolia",
            Deciduous,
            Round,
            Palmate,
            75.0,
            60.0,
            [80, 110, 52],
        )
        .autumn([170, 130, 60])
        .bark(Bark::Mottled, [172, 164, 138])
        .trunk(14.0)
        .d("Classic city tree with camouflage bark and a big rounded crown."),
        plant(
            "American Sycamore",
            "Platanus occidentalis",
            Deciduous,
            Irregular,
            Palmate,
            80.0,
            65.0,
            [86, 116, 56],
        )
        .autumn([164, 120, 56])
        .bark(Bark::Mottled, [210, 204, 184])
        .trunk(16.0)
        .d("Massive tree with white upper limbs and patchy bark."),
        plant(
            "American Elm",
            "Ulmus americana",
            Deciduous,
            Vase,
            Ovate,
            70.0,
            60.0,
            [70, 104, 46],
        )
        .autumn([200, 170, 60])
        .trunk(16.0)
        .bark(Bark::Furrowed, [100, 92, 82])
        .d("The classic arching vase-shaped avenue tree."),
        plant(
            "Lacebark Elm",
            "Ulmus parvifolia",
            Deciduous,
            Round,
            Small,
            45.0,
            40.0,
            [74, 106, 48],
        )
        .autumn([170, 120, 56])
        .bark(Bark::Mottled, [150, 128, 104])
        .clim(mild)
        .d("Fine-textured, tough elm with exfoliating bark."),
        plant(
            "Japanese Zelkova",
            "Zelkova serrata",
            Deciduous,
            Vase,
            Ovate,
            55.0,
            45.0,
            [74, 110, 50],
        )
        .autumn([180, 90, 40])
        .trunk(10.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .d("Upright vase form; a refined street tree."),
        plant(
            "Littleleaf Linden",
            "Tilia cordata",
            Deciduous,
            Pyramidal,
            Heart,
            50.0,
            35.0,
            [66, 100, 46],
        )
        .autumn([200, 176, 70])
        .dense(0.95)
        .trunk(8.0)
        .d("Dense, neat pyramidal crown; formal allées."),
        plant(
            "American Linden",
            "Tilia americana",
            Deciduous,
            Oval,
            Heart,
            70.0,
            45.0,
            [72, 106, 48],
        )
        .autumn([196, 170, 64])
        .clim(cold)
        .d("Large heart-leaved shade tree."),
        plant(
            "Ginkgo",
            "Ginkgo biloba",
            Deciduous,
            Pyramidal,
            Rounded,
            60.0,
            35.0,
            [100, 140, 58],
        )
        .autumn([232, 196, 58])
        .dense(0.6)
        .trunk(8.0)
        .bark(Bark::Furrowed, [110, 100, 88])
        .d("Ancient fan-leaved tree; clear gold in fall."),
        plant(
            "Tulip Tree",
            "Liriodendron tulipifera",
            Deciduous,
            Oval,
            Lobed,
            85.0,
            40.0,
            [86, 124, 54],
        )
        .autumn([222, 186, 60])
        .trunk(20.0)
        .d("Tall straight trunk, tulip-shaped leaves."),
        plant(
            "Sweetgum",
            "Liquidambar styraciflua",
            Deciduous,
            Pyramidal,
            Palmate,
            65.0,
            45.0,
            [64, 100, 44],
        )
        .autumn([160, 40, 60])
        .trunk(10.0)
        .d("Star-shaped leaves turning purple, red and orange."),
        plant(
            "Katsura Tree",
            "Cercidiphyllum japonicum",
            Deciduous,
            Round,
            Heart,
            45.0,
            40.0,
            [92, 124, 70],
        )
        .autumn([230, 170, 90])
        .stems(2)
        .d("Rounded tree with round heart-shaped leaves."),
        plant(
            "Black Tupelo",
            "Nyssa sylvatica",
            Deciduous,
            Pyramidal,
            Ovate,
            45.0,
            30.0,
            [48, 84, 40],
        )
        .autumn([190, 34, 34])
        .clim(cold)
        .d("Glossy leaves turning scarlet; native shade tree."),
        plant(
            "European Beech",
            "Fagus sylvatica",
            Deciduous,
            Round,
            Ovate,
            60.0,
            50.0,
            [62, 98, 44],
        )
        .autumn([170, 104, 44])
        .bark(Bark::Smooth, [142, 142, 136])
        .dense(1.0)
        .trunk(8.0)
        .d("Smooth gray trunk and a dense, low-branched crown."),
        plant(
            "Copper Beech",
            "Fagus sylvatica 'Purpurea'",
            Deciduous,
            Round,
            Ovate,
            55.0,
            45.0,
            [92, 44, 50],
        )
        .alt([118, 58, 58])
        .autumn([150, 74, 40])
        .bark(Bark::Smooth, [142, 142, 136])
        .dense(1.0)
        .trunk(8.0)
        .d("Deep purple-leaved specimen beech."),
        plant(
            "Weeping Willow",
            "Salix babylonica",
            Deciduous,
            Weeping,
            Lanceolate,
            40.0,
            40.0,
            [126, 152, 68],
        )
        .autumn([200, 190, 80])
        .trunk(6.0)
        .bark(Bark::Furrowed, [100, 88, 70])
        .dense(0.7)
        .d("Cascading curtains of fine branches; waterside tree."),
        plant(
            "Quaking Aspen",
            "Populus tremuloides",
            Deciduous,
            Columnar,
            Rounded,
            45.0,
            20.0,
            [100, 136, 60],
        )
        .autumn([236, 200, 52])
        .bark(Bark::Papery, [212, 214, 196])
        .trunk(15.0)
        .dense(0.6)
        .clim(&[Cold])
        .d("White-barked grove tree with fluttering round leaves."),
        plant(
            "Lombardy Poplar",
            "Populus nigra 'Italica'",
            Deciduous,
            Columnar,
            Heart,
            60.0,
            12.0,
            [82, 116, 52],
        )
        .autumn([220, 190, 60])
        .trunk(4.0)
        .d("Tall narrow column; windbreaks and allées."),
        plant(
            "Green Ash",
            "Fraxinus pennsylvanica",
            Deciduous,
            Oval,
            Compound,
            55.0,
            40.0,
            [70, 104, 46],
        )
        .autumn([214, 190, 70])
        .d("Upright oval shade tree with compound leaves."),
        plant(
            "Common Hackberry",
            "Celtis occidentalis",
            Deciduous,
            Round,
            Ovate,
            50.0,
            45.0,
            [76, 106, 50],
        )
        .autumn([200, 176, 76])
        .bark(Bark::Furrowed, [120, 112, 100])
        .clim(cold)
        .d("Tough, adaptable shade tree with warty bark."),
        plant(
            "Kentucky Coffeetree",
            "Gymnocladus dioicus",
            Deciduous,
            Oval,
            Compound,
            60.0,
            40.0,
            [70, 100, 64],
        )
        .autumn([200, 180, 90])
        .dense(0.45)
        .d("Coarse, open branching; very tough urban tree."),
        plant(
            "Horse Chestnut",
            "Aesculus hippocastanum",
            Deciduous,
            Oval,
            Palmate,
            60.0,
            45.0,
            [60, 94, 42],
        )
        .autumn([180, 110, 40])
        .dense(1.0)
        .d("Large dense tree with big palmate leaves."),
        plant(
            "Columnar Hornbeam",
            "Carpinus betulus 'Fastigiata'",
            Deciduous,
            Columnar,
            Ovate,
            35.0,
            20.0,
            [66, 102, 46],
        )
        .autumn([210, 170, 60])
        .trunk(4.0)
        .dense(1.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .d("Tight upright oval; screens and narrow spaces."),
        plant(
            "Chinese Pistache",
            "Pistacia chinensis",
            Deciduous,
            Round,
            Compound,
            35.0,
            35.0,
            [70, 104, 50],
        )
        .autumn([222, 80, 34])
        .clim(dry)
        .d("Drought-tolerant; fiery orange-red fall colour."),
        plant(
            "Chinese Elm, Weeping",
            "Ulmus parvifolia 'Drake'",
            Deciduous,
            Weeping,
            Small,
            35.0,
            35.0,
            [80, 112, 52],
        )
        .autumn([190, 150, 60])
        .clim(mild)
        .d("Pendulous fine-textured elm."),
        plant(
            "Black Walnut",
            "Juglans nigra",
            Deciduous,
            Round,
            Compound,
            70.0,
            60.0,
            [80, 112, 54],
        )
        .autumn([210, 190, 80])
        .dense(0.55)
        .bark(Bark::Furrowed, DARK_BARK)
        .d("Open rounded crown on a tall dark trunk."),
        plant(
            "Catalpa",
            "Catalpa speciosa",
            Deciduous,
            Oval,
            Heart,
            50.0,
            35.0,
            [90, 128, 58],
        )
        .flowers(WHITE)
        .autumn([200, 180, 80])
        .d("Huge heart-shaped leaves and white summer flowers."),
        plant(
            "Crabapple Tree, Fruiting",
            "Malus 'Prairifire'",
            Deciduous,
            Round,
            Ovate,
            20.0,
            20.0,
            [72, 90, 48],
        )
        .flowers([214, 70, 110])
        .autumn([180, 110, 50])
        .trunk(4.0)
        .cal(5.0)
        .d("Small rounded tree with deep pink blossoms and red fruit."),
        // Flowering trees
        plant(
            "Eastern Redbud",
            "Cercis canadensis",
            Flowering,
            Spreading,
            Heart,
            25.0,
            25.0,
            [84, 116, 52],
        )
        .flowers([196, 84, 160])
        .autumn([220, 190, 70])
        .stems(2)
        .trunk(4.0)
        .cal(5.0)
        .dense(0.6)
        .d("Magenta-pink blossoms on bare branches in early spring."),
        plant(
            "Flowering Dogwood",
            "Cornus florida",
            Flowering,
            Spreading,
            Ovate,
            25.0,
            25.0,
            [76, 108, 50],
        )
        .flowers(WHITE)
        .autumn([168, 44, 44])
        .trunk(4.0)
        .cal(6.0)
        .dense(0.65)
        .d("Layered branches covered in white bracts."),
        plant(
            "Kousa Dogwood",
            "Cornus kousa",
            Flowering,
            Round,
            Ovate,
            25.0,
            25.0,
            [70, 104, 48],
        )
        .flowers([242, 240, 226])
        .autumn([176, 56, 44])
        .stems(2)
        .trunk(3.0)
        .cal(5.0)
        .d("Star-shaped cream flowers in early summer."),
        plant(
            "Yoshino Cherry",
            "Prunus × yedoensis",
            Flowering,
            Spreading,
            Ovate,
            35.0,
            35.0,
            [80, 112, 52],
        )
        .flowers([246, 216, 224])
        .autumn([204, 110, 50])
        .trunk(5.0)
        .bark(Bark::Smooth, [96, 70, 62])
        .dense(0.75)
        .d("Clouds of pale pink blossom; the cherry of Washington's Tidal Basin."),
        plant(
            "Kwanzan Cherry",
            "Prunus serrulata 'Kwanzan'",
            Flowering,
            Vase,
            Ovate,
            30.0,
            25.0,
            [78, 104, 48],
        )
        .flowers([234, 150, 184])
        .autumn([196, 100, 44])
        .trunk(5.0)
        .bark(Bark::Smooth, [96, 70, 62])
        .d("Double pink blossoms on a vase-shaped tree."),
        plant(
            "Weeping Cherry",
            "Prunus subhirtella 'Pendula'",
            Flowering,
            Weeping,
            Ovate,
            25.0,
            25.0,
            [82, 112, 54],
        )
        .flowers([244, 206, 220])
        .autumn([200, 150, 60])
        .trunk(5.0)
        .bark(Bark::Smooth, [96, 70, 62])
        .d("Cascading branches of pink blossom."),
        plant(
            "Saucer Magnolia",
            "Magnolia × soulangeana",
            Flowering,
            Round,
            Ovate,
            25.0,
            25.0,
            [74, 106, 50],
        )
        .flowers([238, 196, 214])
        .autumn([180, 150, 70])
        .stems(3)
        .trunk(3.0)
        .cal(5.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .d("Big goblet flowers, pink outside and white within."),
        plant(
            "Star Magnolia",
            "Magnolia stellata",
            Flowering,
            Round,
            Lanceolate,
            18.0,
            15.0,
            [80, 112, 56],
        )
        .flowers(WHITE)
        .stems(4)
        .trunk(1.5)
        .cal(3.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .d("Starry white early-spring flowers; compact."),
        plant(
            "Crape Myrtle",
            "Lagerstroemia indica",
            Flowering,
            Vase,
            Small,
            22.0,
            15.0,
            [60, 96, 46],
        )
        .flowers([212, 72, 140])
        .autumn([200, 90, 50])
        .stems(4)
        .trunk(4.0)
        .cal(4.0)
        .bark(Bark::Mottled, [172, 142, 118])
        .clim(warm)
        .d("Long summer bloom, sculptural smooth trunks."),
        plant(
            "Crape Myrtle, White",
            "Lagerstroemia 'Natchez'",
            Flowering,
            Vase,
            Small,
            25.0,
            18.0,
            [62, 98, 48],
        )
        .flowers(WHITE)
        .autumn([200, 110, 50])
        .stems(4)
        .trunk(5.0)
        .cal(4.0)
        .bark(Bark::Mottled, [182, 146, 118])
        .clim(warm)
        .d("White panicles and cinnamon bark."),
        plant(
            "Serviceberry",
            "Amelanchier × grandiflora",
            Flowering,
            Oval,
            Ovate,
            22.0,
            18.0,
            [74, 106, 52],
        )
        .flowers(WHITE)
        .autumn([214, 90, 40])
        .stems(4)
        .trunk(3.0)
        .cal(3.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .d("Multi-stem native with white spring flowers."),
        plant(
            "Jacaranda",
            "Jacaranda mimosifolia",
            Flowering,
            Umbrella,
            Compound,
            35.0,
            35.0,
            [84, 122, 56],
        )
        .flowers([146, 128, 214])
        .dense(0.55)
        .trunk(8.0)
        .clim(warm)
        .d("Lavender-blue canopy of bloom; frost-free climates."),
        plant(
            "Royal Poinciana",
            "Delonix regia",
            Flowering,
            Umbrella,
            Compound,
            35.0,
            50.0,
            [76, 118, 50],
        )
        .flowers([226, 70, 38])
        .dense(0.6)
        .trunk(8.0)
        .clim(&[Tropical])
        .d("Flame tree: a wide umbrella of scarlet flowers."),
        plant(
            "Golden Rain Tree",
            "Koelreuteria paniculata",
            Flowering,
            Round,
            Compound,
            30.0,
            30.0,
            [74, 108, 50],
        )
        .flowers([240, 206, 60])
        .autumn([214, 180, 60])
        .dense(0.65)
        .d("Yellow summer panicles and papery pods."),
        plant(
            "Silk Tree",
            "Albizia julibrissin",
            Flowering,
            Umbrella,
            Compound,
            25.0,
            30.0,
            [86, 124, 60],
        )
        .flowers([240, 150, 186])
        .dense(0.5)
        .stems(2)
        .trunk(5.0)
        .clim(mild)
        .d("Flat-topped tree with pink powder-puff flowers."),
        plant(
            "Purple Leaf Plum",
            "Prunus cerasifera 'Krauter Vesuvius'",
            Flowering,
            Round,
            Ovate,
            20.0,
            18.0,
            [84, 40, 52],
        )
        .alt([108, 52, 62])
        .flowers([244, 214, 222])
        .trunk(4.0)
        .cal(4.0)
        .d("Dark purple foliage and pale pink spring flowers."),
        plant(
            "Cleveland Pear",
            "Pyrus calleryana 'Cleveland Select'",
            Flowering,
            Oval,
            Ovate,
            35.0,
            15.0,
            [62, 100, 46],
        )
        .flowers(WHITE)
        .autumn([176, 48, 40])
        .dense(0.95)
        .trunk(5.0)
        .d("Tight oval covered in white blossom."),
        plant(
            "Tabebuia",
            "Handroanthus chrysotrichus",
            Flowering,
            Round,
            Compound,
            25.0,
            25.0,
            [82, 118, 54],
        )
        .flowers([244, 200, 40])
        .dense(0.5)
        .clim(&[Tropical])
        .d("Golden trumpet tree; spring gold flowers."),
        // Broadleaf evergreens
        plant(
            "Southern Live Oak",
            "Quercus virginiana",
            Evergreen,
            Spreading,
            Small,
            50.0,
            80.0,
            [56, 80, 42],
        )
        .trunk(10.0)
        .cal(30.0)
        .bark(Bark::Furrowed, [80, 74, 66])
        .clim(warm)
        .d("Massive low sweeping limbs; the Southern landmark tree."),
        plant(
            "Coast Live Oak",
            "Quercus agrifolia",
            Evergreen,
            Spreading,
            Small,
            45.0,
            55.0,
            [50, 76, 40],
        )
        .trunk(8.0)
        .stems(2)
        .clim(&[Mediterranean])
        .d("Gnarled Californian native with holly-like leaves."),
        plant(
            "Holm Oak",
            "Quercus ilex",
            Evergreen,
            Round,
            Small,
            50.0,
            40.0,
            [44, 64, 38],
        )
        .alt([80, 92, 66])
        .dense(1.0)
        .clim(&[Mediterranean])
        .d("Dense dark evergreen oak of the Mediterranean."),
        plant(
            "Southern Magnolia",
            "Magnolia grandiflora",
            Evergreen,
            Pyramidal,
            Ovate,
            60.0,
            35.0,
            GLOSSY,
        )
        .flowers([246, 242, 226])
        .dense(1.0)
        .trunk(4.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .clim(warm)
        .d("Glossy leaves and huge fragrant white flowers."),
        plant(
            "Olive Tree",
            "Olea europaea",
            Evergreen,
            Irregular,
            Lanceolate,
            25.0,
            25.0,
            SILVER,
        )
        .alt([150, 160, 126])
        .stems(2)
        .trunk(5.0)
        .cal(12.0)
        .bark(Bark::Furrowed, [110, 104, 94])
        .dense(0.6)
        .clim(dry)
        .d("Silvery gnarled Mediterranean tree; courtyards and terraces."),
        plant(
            "Camphor Tree",
            "Cinnamomum camphora",
            Evergreen,
            Round,
            Ovate,
            50.0,
            60.0,
            [76, 110, 52],
        )
        .clim(warm)
        .d("Broad, dense, glossy shade tree."),
        plant(
            "River Red Gum",
            "Eucalyptus camaldulensis",
            Evergreen,
            Irregular,
            Lanceolate,
            80.0,
            50.0,
            [108, 126, 96],
        )
        .bark(Bark::Mottled, [206, 198, 176])
        .trunk(18.0)
        .dense(0.45)
        .clim(dry)
        .d("Tall eucalyptus with pale patchy trunks and drooping leaves."),
        plant(
            "Lemon-Scented Gum",
            "Corymbia citriodora",
            Evergreen,
            Oval,
            Lanceolate,
            70.0,
            30.0,
            [110, 130, 88],
        )
        .bark(Bark::Smooth, [222, 218, 206])
        .trunk(25.0)
        .dense(0.4)
        .clim(dry)
        .d("Elegant powder-white trunk and open crown."),
        plant(
            "Strawberry Tree",
            "Arbutus 'Marina'",
            Evergreen,
            Round,
            Ovate,
            25.0,
            20.0,
            [48, 76, 40],
        )
        .flowers([236, 200, 206])
        .stems(2)
        .bark(Bark::Smooth, [150, 64, 46])
        .clim(&[Mediterranean])
        .d("Red peeling bark, pink flowers and red fruit."),
        plant(
            "Bay Laurel",
            "Laurus nobilis",
            Evergreen,
            Oval,
            Lanceolate,
            30.0,
            20.0,
            [52, 80, 42],
        )
        .dense(1.0)
        .clim(&[Mediterranean])
        .d("Dense aromatic evergreen; clipped or natural."),
        plant(
            "Indian Laurel Fig",
            "Ficus microcarpa",
            Evergreen,
            Round,
            Small,
            50.0,
            50.0,
            [46, 84, 40],
        )
        .dense(1.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .clim(&[Tropical])
        .d("Dense glossy canopy; tropical street tree."),
        plant(
            "California Pepper Tree",
            "Schinus molle",
            Evergreen,
            Weeping,
            Compound,
            40.0,
            40.0,
            [100, 132, 70],
        )
        .bark(Bark::Furrowed, [120, 100, 84])
        .dense(0.55)
        .clim(dry)
        .d("Weeping fine foliage on a gnarled trunk."),
        plant(
            "Palo Verde",
            "Parkinsonia 'Desert Museum'",
            Evergreen,
            Spreading,
            Compound,
            25.0,
            25.0,
            [128, 150, 70],
        )
        .flowers([242, 212, 50])
        .bark(Bark::Green, [132, 160, 90])
        .stems(3)
        .trunk(3.0)
        .dense(0.35)
        .clim(&[Arid])
        .d("Green-barked desert tree with yellow spring flowers."),
        plant(
            "Honey Mesquite",
            "Prosopis glandulosa",
            Evergreen,
            Spreading,
            Compound,
            25.0,
            30.0,
            [100, 130, 66],
        )
        .stems(3)
        .trunk(3.0)
        .bark(Bark::Furrowed, [80, 66, 56])
        .dense(0.4)
        .clim(&[Arid])
        .d("Low spreading desert shade tree."),
        plant(
            "Umbrella Thorn Acacia",
            "Vachellia tortilis",
            Evergreen,
            Umbrella,
            Compound,
            30.0,
            40.0,
            [100, 124, 60],
        )
        .dense(0.5)
        .trunk(10.0)
        .clim(&[Arid])
        .d("The flat-topped savanna tree."),
        plant(
            "American Holly",
            "Ilex opaca",
            Evergreen,
            Pyramidal,
            Ovate,
            30.0,
            18.0,
            [40, 64, 36],
        )
        .flowers([176, 30, 30])
        .dense(1.0)
        .trunk(2.0)
        .bark(Bark::Smooth, GRAY_BARK)
        .clim(mild)
        .d("Spiny dark leaves with red winter berries."),
        plant(
            "Loquat",
            "Eriobotrya japonica",
            Evergreen,
            Round,
            Lanceolate,
            20.0,
            20.0,
            [54, 86, 42],
        )
        .clim(warm)
        .d("Bold, leathery leaves; small courtyard tree."),
        // Conifers
        plant(
            "Eastern White Pine",
            "Pinus strobus",
            Conifer,
            Irregular,
            Needle,
            80.0,
            40.0,
            [66, 100, 70],
        )
        .trunk(10.0)
        .bark(Bark::Furrowed, [82, 72, 64])
        .dense(0.65)
        .clim(cold)
        .d("Soft blue-green needles in tiers; a stately pine."),
        plant(
            "Scots Pine",
            "Pinus sylvestris",
            Conifer,
            Umbrella,
            Needle,
            60.0,
            30.0,
            [70, 98, 84],
        )
        .trunk(30.0)
        .bark(Bark::Plated, [168, 104, 64])
        .dense(0.6)
        .clim(cold)
        .d("Orange upper bark, flat irregular crown."),
        plant(
            "Ponderosa Pine",
            "Pinus ponderosa",
            Conifer,
            Oval,
            Needle,
            90.0,
            30.0,
            [74, 100, 58],
        )
        .trunk(25.0)
        .bark(Bark::Plated, [150, 100, 64])
        .dense(0.6)
        .clim(&[Cold, Arid])
        .d("Tall western pine with jigsaw-puzzle bark."),
        plant(
            "Italian Stone Pine",
            "Pinus pinea",
            Conifer,
            Umbrella,
            Needle,
            50.0,
            45.0,
            [70, 100, 56],
        )
        .trunk(25.0)
        .bark(Bark::Plated, [140, 100, 76])
        .clim(&[Mediterranean])
        .d("The umbrella pine of Rome."),
        plant(
            "Austrian Pine",
            "Pinus nigra",
            Conifer,
            Pyramidal,
            Needle,
            55.0,
            30.0,
            [40, 66, 44],
        )
        .bark(Bark::Plated, [80, 72, 66])
        .dense(0.9)
        .clim(cold)
        .d("Dark, dense pine for screens."),
        plant(
            "Japanese Black Pine",
            "Pinus thunbergii",
            Conifer,
            Irregular,
            Needle,
            30.0,
            25.0,
            [46, 74, 46],
        )
        .trunk(6.0)
        .bark(Bark::Plated, [70, 62, 58])
        .dense(0.6)
        .d("Sculptural, windswept pine for Japanese gardens."),
        plant(
            "Loblolly Pine",
            "Pinus taeda",
            Conifer,
            Oval,
            Needle,
            80.0,
            35.0,
            [78, 104, 56],
        )
        .trunk(35.0)
        .bark(Bark::Plated, [118, 90, 70])
        .dense(0.6)
        .clim(mild)
        .d("Tall straight southern pine with an open crown."),
        plant(
            "Norway Spruce",
            "Picea abies",
            Conifer,
            Pyramidal,
            Needle,
            70.0,
            30.0,
            SPRUCE,
        )
        .dense(0.9)
        .bark(Bark::Plated, [96, 70, 56])
        .clim(cold)
        .d("Classic dark spruce with drooping branchlets."),
        plant(
            "Colorado Blue Spruce",
            "Picea pungens 'Glauca'",
            Conifer,
            Conical,
            Needle,
            50.0,
            20.0,
            [104, 136, 144],
        )
        .alt([126, 154, 160])
        .dense(1.0)
        .clim(cold)
        .d("Silver-blue formal cone."),
        plant(
            "Serbian Spruce",
            "Picea omorika",
            Conifer,
            Conical,
            Needle,
            50.0,
            14.0,
            [42, 68, 52],
        )
        .dense(0.85)
        .clim(cold)
        .d("Slender spire with upturned branch tips."),
        plant(
            "Douglas Fir",
            "Pseudotsuga menziesii",
            Conifer,
            Pyramidal,
            Needle,
            90.0,
            25.0,
            [52, 84, 52],
        )
        .bark(Bark::Furrowed, [96, 70, 56])
        .trunk(8.0)
        .clim(cold)
        .d("Tall western timber tree."),
        plant(
            "Fraser Fir",
            "Abies fraseri",
            Conifer,
            Conical,
            Needle,
            40.0,
            20.0,
            [40, 70, 50],
        )
        .alt([72, 100, 84])
        .dense(1.0)
        .clim(&[Cold])
        .d("Neat dense cone; the Christmas tree."),
        plant(
            "White Fir",
            "Abies concolor",
            Conifer,
            Conical,
            Needle,
            50.0,
            25.0,
            [104, 132, 128],
        )
        .dense(0.9)
        .clim(cold)
        .d("Soft blue-green needles; a gentler blue spruce."),
        plant(
            "Eastern Hemlock",
            "Tsuga canadensis",
            Conifer,
            Pyramidal,
            Needle,
            60.0,
            30.0,
            [44, 76, 48],
        )
        .dense(0.9)
        .clim(cold)
        .d("Graceful, soft, nodding branches; shade tolerant."),
        plant(
            "Eastern Red Cedar",
            "Juniperus virginiana",
            Conifer,
            Conical,
            Scale,
            40.0,
            12.0,
            [58, 82, 52],
        )
        .bark(Bark::Fibrous, [120, 80, 60])
        .dense(1.0)
        .d("Dense narrow native juniper."),
        plant(
            "Italian Cypress",
            "Cupressus sempervirens",
            Conifer,
            Columnar,
            Scale,
            40.0,
            5.0,
            [40, 62, 40],
        )
        .dense(1.0)
        .trunk(0.5)
        .clim(&[Mediterranean])
        .d("Pencil-thin dark column of Tuscan landscapes."),
        plant(
            "Leyland Cypress",
            "× Cuprocyparis leylandii",
            Conifer,
            Conical,
            Scale,
            50.0,
            15.0,
            [56, 84, 54],
        )
        .dense(1.0)
        .clim(mild)
        .d("Fast feathery screen."),
        plant(
            "Monterey Cypress",
            "Cupressus macrocarpa",
            Conifer,
            Umbrella,
            Scale,
            40.0,
            40.0,
            [48, 76, 46],
        )
        .trunk(10.0)
        .stems(2)
        .bark(Bark::Fibrous, [120, 96, 80])
        .clim(&[Mediterranean])
        .d("Windswept flat-topped coastal cypress."),
        plant(
            "Deodar Cedar",
            "Cedrus deodara",
            Conifer,
            Pyramidal,
            Needle,
            60.0,
            40.0,
            [92, 120, 102],
        )
        .dense(0.8)
        .clim(mild)
        .d("Graceful pyramid with drooping tips; silver-green."),
        plant(
            "Blue Atlas Cedar",
            "Cedrus atlantica 'Glauca'",
            Conifer,
            Pyramidal,
            Needle,
            55.0,
            35.0,
            [112, 138, 142],
        )
        .dense(0.75)
        .clim(mild)
        .d("Silver-blue tiers; a striking specimen."),
        plant(
            "Giant Sequoia",
            "Sequoiadendron giganteum",
            Conifer,
            Conical,
            Scale,
            120.0,
            35.0,
            [62, 94, 60],
        )
        .cal(72.0)
        .bark(Bark::Fibrous, [150, 80, 56])
        .trunk(12.0)
        .clim(mild)
        .d("The largest tree on earth: massive red fibrous trunk."),
        plant(
            "Coast Redwood",
            "Sequoia sempervirens",
            Conifer,
            Conical,
            Needle,
            110.0,
            30.0,
            [50, 84, 52],
        )
        .cal(48.0)
        .bark(Bark::Fibrous, [140, 76, 54])
        .trunk(15.0)
        .clim(&[Mediterranean])
        .d("The tallest tree; soft sprays on a red trunk."),
        plant(
            "Dawn Redwood",
            "Metasequoia glyptostroboides",
            Conifer,
            Conical,
            Needle,
            70.0,
            25.0,
            [100, 138, 70],
        )
        .autumn([184, 100, 50])
        .bark(Bark::Fibrous, [140, 90, 66])
        .d("Deciduous conifer, feathery and fast; bronze in fall."),
        plant(
            "Bald Cypress",
            "Taxodium distichum",
            Conifer,
            Pyramidal,
            Needle,
            60.0,
            25.0,
            [92, 126, 64],
        )
        .autumn([170, 84, 44])
        .bark(Bark::Fibrous, [120, 96, 80])
        .clim(mild)
        .d("Deciduous conifer for wet ground; russet in fall."),
        plant(
            "Green Giant Arborvitae",
            "Thuja 'Green Giant'",
            Conifer,
            Conical,
            Scale,
            40.0,
            14.0,
            [62, 94, 50],
        )
        .dense(1.0)
        .trunk(1.0)
        .clim(cold)
        .d("Fast, dense privacy screen."),
        plant(
            "Emerald Arborvitae",
            "Thuja occidentalis 'Smaragd'",
            Conifer,
            Columnar,
            Scale,
            14.0,
            4.0,
            [60, 100, 50],
        )
        .dense(1.0)
        .trunk(0.5)
        .clim(cold)
        .d("Narrow bright green column; hedges and entries."),
        plant(
            "Japanese Cedar",
            "Cryptomeria japonica",
            Conifer,
            Conical,
            Needle,
            50.0,
            20.0,
            [58, 92, 56],
        )
        .bark(Bark::Fibrous, [130, 86, 64])
        .clim(mild)
        .d("Soft, dense cone with reddish bark."),
        plant(
            "Norfolk Island Pine",
            "Araucaria heterophylla",
            Conifer,
            Pyramidal,
            Needle,
            60.0,
            25.0,
            [60, 102, 56],
        )
        .dense(0.55)
        .clim(&[Tropical])
        .d("Symmetrical tiers of branches; coastal tropics."),
        plant(
            "Mugo Pine",
            "Pinus mugo",
            Shrub,
            Mound,
            Needle,
            5.0,
            8.0,
            [58, 88, 52],
        )
        .dense(1.0)
        .stems(6)
        .trunk(0.0)
        .cal(1.5)
        .clim(cold)
        .d("Dwarf mounding pine for rock gardens."),
        plant(
            "Dwarf Alberta Spruce",
            "Picea glauca 'Conica'",
            Shrub,
            Conical,
            Needle,
            8.0,
            3.5,
            [84, 124, 66],
        )
        .dense(1.0)
        .trunk(0.0)
        .clim(cold)
        .d("Tidy bright green cone for containers and entries."),
        // Palms
        plant(
            "Queen Palm",
            "Syagrus romanzoffiana",
            Palm,
            CrownForm::PalmHead,
            Frond,
            40.0,
            20.0,
            [72, 112, 50],
        )
        .bark(Bark::Palm, [150, 146, 138])
        .clim(warm)
        .d("Graceful arching feathery fronds on a smooth gray trunk."),
        plant(
            "Mexican Fan Palm",
            "Washingtonia robusta",
            Palm,
            CrownForm::PalmHead,
            Fan,
            70.0,
            12.0,
            [74, 110, 48],
        )
        .cal(14.0)
        .bark(Bark::Palm, [124, 108, 92])
        .clim(&[Mediterranean, Arid])
        .d("Tall skinny trunk with a tight fan crown; Los Angeles skyline."),
        plant(
            "California Fan Palm",
            "Washingtonia filifera",
            Palm,
            CrownForm::PalmHead,
            Fan,
            50.0,
            15.0,
            [96, 124, 64],
        )
        .cal(30.0)
        .bark(Bark::Palm, [134, 116, 94])
        .clim(&[Arid])
        .d("Stout trunk and gray-green fans with threads."),
        plant(
            "Canary Island Date Palm",
            "Phoenix canariensis",
            Palm,
            CrownForm::PalmHead,
            Frond,
            50.0,
            40.0,
            [66, 104, 46],
        )
        .cal(36.0)
        .bark(Bark::Palm, [138, 116, 90])
        .clim(warm)
        .d("Huge full crown on a massive diamond-patterned trunk."),
        plant(
            "Date Palm",
            "Phoenix dactylifera",
            Palm,
            CrownForm::PalmHead,
            Frond,
            60.0,
            25.0,
            [110, 130, 96],
        )
        .cal(18.0)
        .bark(Bark::Palm, [132, 112, 88])
        .clim(&[Arid])
        .d("Gray-green stiff fronds; desert oases."),
        plant(
            "Royal Palm",
            "Roystonea regia",
            Palm,
            CrownForm::PalmHead,
            Frond,
            60.0,
            25.0,
            [68, 110, 48],
        )
        .cal(22.0)
        .bark(Bark::Smooth, [176, 174, 166])
        .clim(&[Tropical])
        .d("Smooth concrete-gray trunk and a green crownshaft."),
        plant(
            "Coconut Palm",
            "Cocos nucifera",
            Palm,
            CrownForm::PalmHead,
            Frond,
            60.0,
            25.0,
            [86, 126, 52],
        )
        .cal(14.0)
        .bark(Bark::Palm, [140, 126, 104])
        .clim(&[Tropical])
        .d("Leaning curved trunk; the beach palm."),
        plant(
            "Sabal Palm",
            "Sabal palmetto",
            Palm,
            CrownForm::PalmHead,
            Fan,
            40.0,
            12.0,
            [70, 104, 50],
        )
        .cal(14.0)
        .bark(Bark::Palm, [132, 116, 96])
        .clim(warm)
        .d("Cabbage palm with costapalmate fans; hardy."),
        plant(
            "Windmill Palm",
            "Trachycarpus fortunei",
            Palm,
            CrownForm::PalmHead,
            Fan,
            25.0,
            8.0,
            [58, 94, 44],
        )
        .cal(9.0)
        .bark(Bark::Fibrous, [96, 78, 60])
        .clim(mild)
        .d("Cold-hardy fan palm with a hairy trunk."),
        plant(
            "Foxtail Palm",
            "Wodyetia bifurcata",
            Palm,
            CrownForm::PalmHead,
            Frond,
            30.0,
            15.0,
            [78, 120, 50],
        )
        .cal(10.0)
        .bark(Bark::Smooth, [170, 168, 158])
        .clim(&[Tropical])
        .d("Bushy plume-like fronds."),
        plant(
            "Bismarck Palm",
            "Bismarckia nobilis",
            Palm,
            CrownForm::PalmHead,
            Fan,
            40.0,
            20.0,
            [150, 172, 168],
        )
        .cal(18.0)
        .bark(Bark::Palm, [150, 140, 126])
        .clim(&[Tropical, Arid])
        .d("Huge silver-blue fans; a bold accent."),
        plant(
            "Pygmy Date Palm",
            "Phoenix roebelenii",
            Palm,
            CrownForm::PalmHead,
            Frond,
            8.0,
            7.0,
            [62, 100, 44],
        )
        .stems(3)
        .cal(4.0)
        .bark(Bark::Palm, [110, 96, 80])
        .clim(&[Tropical])
        .d("Small clustering palm for patios."),
        plant(
            "Sago Palm",
            "Cycas revoluta",
            Palm,
            Rosette,
            Frond,
            4.0,
            6.0,
            [44, 80, 36],
        )
        .trunk(1.0)
        .cal(8.0)
        .clim(warm)
        .d("Stiff glossy rosette of fronds; a cycad."),
        // Shrubs
        plant(
            "Boxwood, Round",
            "Buxus sempervirens",
            Shrub,
            Mound,
            Small,
            3.0,
            3.0,
            [50, 82, 38],
        )
        .dense(1.0)
        .d("Clipped evergreen globe."),
        plant(
            "Boxwood, Large",
            "Buxus 'Green Mountain'",
            Shrub,
            Oval,
            Small,
            5.0,
            3.5,
            [52, 84, 40],
        )
        .dense(1.0)
        .d("Upright evergreen for formal gardens."),
        plant(
            "Bigleaf Hydrangea",
            "Hydrangea macrophylla",
            Shrub,
            Mound,
            Ovate,
            5.0,
            5.0,
            [74, 110, 50],
        )
        .flowers([150, 160, 222])
        .d("Mophead blue flowers on a rounded shrub."),
        plant(
            "Panicle Hydrangea",
            "Hydrangea paniculata 'Limelight'",
            Shrub,
            Upright,
            Ovate,
            7.0,
            6.0,
            [70, 106, 48],
        )
        .flowers([232, 238, 204])
        .clim(cold)
        .d("Large lime-to-white cone-shaped flowers."),
        plant(
            "Azalea",
            "Rhododendron 'Encore'",
            Shrub,
            Mound,
            Small,
            4.0,
            4.0,
            [52, 84, 40],
        )
        .flowers([228, 80, 128])
        .clim(mild)
        .d("Mounded evergreen covered in pink flowers."),
        plant(
            "Rhododendron",
            "Rhododendron catawbiense",
            Shrub,
            Round,
            Lanceolate,
            8.0,
            8.0,
            [42, 70, 36],
        )
        .flowers([168, 100, 188])
        .d("Big leathery leaves and purple trusses."),
        plant(
            "Camellia",
            "Camellia japonica",
            Shrub,
            Upright,
            Ovate,
            10.0,
            6.0,
            GLOSSY,
        )
        .flowers([208, 56, 76])
        .clim(mild)
        .d("Glossy evergreen with rose-like red flowers."),
        plant(
            "Gardenia",
            "Gardenia jasminoides",
            Shrub,
            Mound,
            Ovate,
            4.0,
            4.0,
            [44, 74, 38],
        )
        .flowers(WHITE)
        .clim(warm)
        .d("Fragrant white flowers on glossy foliage."),
        plant(
            "English Lavender",
            "Lavandula angustifolia",
            Shrub,
            Mound,
            Blade,
            2.0,
            3.0,
            [128, 146, 120],
        )
        .flowers([164, 142, 212])
        .clim(dry)
        .d("Silver mound with purple flower spikes."),
        plant(
            "Rosemary",
            "Salvia rosmarinus",
            Shrub,
            Mound,
            Needle,
            4.0,
            4.0,
            [92, 112, 86],
        )
        .flowers([150, 164, 214])
        .clim(dry)
        .d("Aromatic gray-green evergreen."),
        plant(
            "Cherry Laurel",
            "Prunus laurocerasus 'Otto Luyken'",
            Shrub,
            Mound,
            Lanceolate,
            4.0,
            7.0,
            GLOSSY,
        )
        .flowers(WHITE)
        .d("Low wide glossy evergreen."),
        plant(
            "Inkberry Holly",
            "Ilex glabra",
            Shrub,
            Round,
            Small,
            6.0,
            6.0,
            [40, 66, 36],
        )
        .dense(1.0)
        .clim(cold)
        .d("Native evergreen, a boxwood alternative."),
        plant(
            "Hicks Yew",
            "Taxus × media 'Hicksii'",
            Shrub,
            Upright,
            Needle,
            12.0,
            4.0,
            [34, 60, 36],
        )
        .dense(1.0)
        .clim(cold)
        .d("Dark narrow upright evergreen."),
        plant(
            "Creeping Juniper",
            "Juniperus horizontalis",
            Shrub,
            Mound,
            Scale,
            1.0,
            6.0,
            [88, 116, 104],
        )
        .dense(1.0)
        .clim(cold)
        .d("Low spreading blue-green groundcover."),
        plant(
            "Blue Star Juniper",
            "Juniperus squamata 'Blue Star'",
            Shrub,
            Mound,
            Needle,
            2.5,
            4.0,
            [110, 138, 150],
        )
        .dense(1.0)
        .clim(cold)
        .d("Compact silver-blue mound."),
        plant(
            "Japanese Spirea",
            "Spiraea japonica",
            Shrub,
            Mound,
            Small,
            3.0,
            4.0,
            [96, 124, 56],
        )
        .flowers([226, 120, 170])
        .d("Pink flat-topped flowers over a tidy mound."),
        plant(
            "Crimson Barberry",
            "Berberis thunbergii 'Crimson Pygmy'",
            Shrub,
            Mound,
            Small,
            2.0,
            3.0,
            [110, 40, 42],
        )
        .alt([138, 54, 50])
        .dense(1.0)
        .d("Low burgundy-red mound."),
        plant(
            "Burning Bush",
            "Euonymus alatus 'Compactus'",
            Shrub,
            Round,
            Ovate,
            8.0,
            8.0,
            [72, 100, 48],
        )
        .dense(1.0)
        .d("Dense rounded shrub, scarlet in fall."),
        plant(
            "Forsythia",
            "Forsythia × intermedia",
            Shrub,
            Fountain,
            Lanceolate,
            8.0,
            8.0,
            [80, 114, 50],
        )
        .flowers([244, 208, 56])
        .d("Arching stems covered in yellow in early spring."),
        plant(
            "Common Lilac",
            "Syringa vulgaris",
            Shrub,
            Upright,
            Heart,
            12.0,
            8.0,
            [70, 102, 50],
        )
        .flowers([176, 136, 208])
        .clim(cold)
        .d("Fragrant purple spring panicles."),
        plant(
            "Doublefile Viburnum",
            "Viburnum plicatum",
            Shrub,
            Spreading,
            Ovate,
            8.0,
            10.0,
            [66, 100, 46],
        )
        .flowers(WHITE)
        .d("Tiered branches lined with white flowers."),
        plant(
            "Oleander",
            "Nerium oleander",
            Shrub,
            Upright,
            Lanceolate,
            10.0,
            8.0,
            [60, 92, 50],
        )
        .flowers([232, 110, 150])
        .clim(dry)
        .d("Tough flowering evergreen for hot climates."),
        plant(
            "Bougainvillea",
            "Bougainvillea glabra",
            Shrub,
            Irregular,
            Ovate,
            8.0,
            8.0,
            [62, 100, 44],
        )
        .flowers([212, 38, 140])
        .clim(warm)
        .d("Masses of magenta bracts."),
        plant(
            "Hibiscus",
            "Hibiscus rosa-sinensis",
            Shrub,
            Upright,
            Ovate,
            8.0,
            5.0,
            [52, 90, 40],
        )
        .flowers([224, 44, 50])
        .clim(&[Tropical])
        .d("Big red tropical flowers."),
        plant(
            "Rose Bush",
            "Rosa 'Knock Out'",
            Shrub,
            Round,
            Compound,
            4.0,
            4.0,
            [58, 92, 44],
        )
        .flowers([212, 40, 70])
        .d("Everblooming shrub rose, cherry red."),
        plant(
            "Pittosporum",
            "Pittosporum tobira",
            Shrub,
            Mound,
            Ovate,
            8.0,
            8.0,
            [56, 90, 44],
        )
        .flowers(WHITE)
        .clim(warm)
        .d("Dense glossy rosettes of leaves."),
        plant(
            "Hebe",
            "Hebe 'Autumn Glory'",
            Shrub,
            Mound,
            Small,
            2.0,
            2.5,
            [56, 94, 48],
        )
        .flowers([126, 90, 190])
        .clim(&[Mediterranean])
        .d("Compact mound with violet spikes."),
        plant(
            "Smoke Bush",
            "Cotinus coggygria 'Royal Purple'",
            Shrub,
            Round,
            Rounded,
            12.0,
            10.0,
            [96, 44, 60],
        )
        .flowers([200, 170, 176])
        .stems(4)
        .d("Purple leaves and smoky plumes."),
        plant(
            "Red Twig Dogwood",
            "Cornus sericea",
            Shrub,
            Upright,
            Ovate,
            7.0,
            8.0,
            [76, 108, 52],
        )
        .bark(Bark::Smooth, [170, 40, 40])
        .stems(9)
        .clim(cold)
        .d("Bright red stems stand out in winter."),
        plant(
            "Fern, Ostrich",
            "Matteuccia struthiopteris",
            Shrub,
            Fountain,
            Frond,
            4.0,
            4.0,
            [96, 138, 56],
        )
        .clim(cold)
        .d("Vase of feathery fronds for shade."),
        // Hedges
        plant(
            "Boxwood Hedge",
            "Buxus sempervirens",
            Hedge,
            Box,
            Small,
            3.0,
            6.0,
            [50, 82, 38],
        )
        .depth(2.0)
        .dense(1.0)
        .d("Low clipped formal edging, 6' module."),
        plant(
            "Privet Hedge",
            "Ligustrum vulgare",
            Hedge,
            Box,
            Small,
            6.0,
            8.0,
            [62, 96, 46],
        )
        .depth(2.5)
        .dense(1.0)
        .d("Classic clipped privacy hedge, 8' module."),
        plant(
            "English Laurel Hedge",
            "Prunus laurocerasus",
            Hedge,
            Box,
            Lanceolate,
            8.0,
            8.0,
            GLOSSY,
        )
        .depth(3.5)
        .dense(1.0)
        .d("Tall glossy privacy hedge."),
        plant(
            "Yew Hedge",
            "Taxus baccata",
            Hedge,
            Box,
            Needle,
            5.0,
            8.0,
            [34, 58, 34],
        )
        .depth(2.5)
        .dense(1.0)
        .d("Dark formal hedge."),
        plant(
            "Hornbeam Hedge",
            "Carpinus betulus",
            Hedge,
            Box,
            Ovate,
            7.0,
            8.0,
            [70, 104, 48],
        )
        .depth(2.5)
        .dense(1.0)
        .autumn([180, 140, 60])
        .d("Clipped hornbeam, bronze leaves hold in winter."),
        plant(
            "Arborvitae Hedge",
            "Thuja occidentalis",
            Hedge,
            Box,
            Scale,
            10.0,
            8.0,
            [60, 96, 50],
        )
        .depth(3.5)
        .dense(1.0)
        .clim(cold)
        .d("Tall evergreen screen."),
        // Ornamental grasses
        plant(
            "Feather Reed Grass",
            "Calamagrostis 'Karl Foerster'",
            Grass,
            Upright,
            Blade,
            5.0,
            2.0,
            [124, 136, 80],
        )
        .flowers([196, 174, 120])
        .d("Vertical tan plumes; the modern landscape staple."),
        plant(
            "Fountain Grass",
            "Pennisetum alopecuroides",
            Grass,
            Fountain,
            Blade,
            3.0,
            3.0,
            [106, 130, 66],
        )
        .flowers([210, 190, 170])
        .d("Arching mound with bottlebrush plumes."),
        plant(
            "Purple Fountain Grass",
            "Pennisetum 'Rubrum'",
            Grass,
            Fountain,
            Blade,
            4.0,
            3.0,
            [110, 52, 62],
        )
        .flowers([176, 110, 120])
        .clim(warm)
        .d("Burgundy blades and plumes."),
        plant(
            "Maiden Grass",
            "Miscanthus sinensis",
            Grass,
            Fountain,
            Blade,
            6.0,
            5.0,
            [100, 124, 70],
        )
        .flowers([214, 198, 170])
        .d("Tall graceful clumps with silky plumes."),
        plant(
            "Mexican Feather Grass",
            "Nassella tenuissima",
            Grass,
            Fountain,
            Blade,
            2.0,
            2.0,
            [168, 170, 106],
        )
        .clim(dry)
        .d("Fine, blonde, wind-tossed tufts."),
        plant(
            "Blue Fescue",
            "Festuca glauca",
            Grass,
            Mound,
            Blade,
            1.0,
            1.0,
            [120, 150, 162],
        )
        .d("Tiny silver-blue tufts for edging."),
        plant(
            "Switchgrass",
            "Panicum virgatum",
            Grass,
            Upright,
            Blade,
            5.0,
            3.0,
            [96, 122, 90],
        )
        .flowers([190, 170, 140])
        .autumn([196, 160, 80])
        .d("Native upright prairie grass."),
        plant(
            "Pampas Grass",
            "Cortaderia selloana",
            Grass,
            Fountain,
            Blade,
            10.0,
            8.0,
            [104, 124, 70],
        )
        .flowers([236, 226, 200])
        .clim(warm)
        .d("Giant clump with feathery cream plumes."),
        plant(
            "Liriope",
            "Liriope muscari",
            Grass,
            Mound,
            Blade,
            1.0,
            1.5,
            [44, 76, 38],
        )
        .flowers([130, 96, 180])
        .clim(mild)
        .d("Strappy evergreen border with purple spikes."),
        plant(
            "Clumping Bamboo",
            "Fargesia robusta",
            Grass,
            Upright,
            Lanceolate,
            15.0,
            6.0,
            [80, 122, 54],
        )
        .stems(12)
        .bark(Bark::Green, [120, 150, 70])
        .clim(mild)
        .d("Upright non-running bamboo screen."),
        plant(
            "Mondo Grass",
            "Ophiopogon japonicus",
            Grass,
            Mound,
            Blade,
            0.5,
            1.0,
            [34, 58, 32],
        )
        .d("Dark evergreen groundcover tufts."),
        // Flowers & perennials
        plant(
            "Hosta",
            "Hosta sieboldiana",
            Perennial,
            Rosette,
            Heart,
            2.0,
            3.0,
            [96, 128, 124],
        )
        .clim(cold)
        .d("Bold blue-green leaves for shade."),
        plant(
            "Daylily",
            "Hemerocallis",
            Perennial,
            Fountain,
            Blade,
            2.0,
            2.0,
            [86, 124, 52],
        )
        .flowers([238, 140, 40])
        .d("Strappy leaves with orange trumpet flowers."),
        plant(
            "Purple Coneflower",
            "Echinacea purpurea",
            Perennial,
            Upright,
            Lanceolate,
            3.0,
            2.0,
            [70, 104, 48],
        )
        .flowers([200, 100, 160])
        .d("Pink-purple daisies for pollinators."),
        plant(
            "Black-Eyed Susan",
            "Rudbeckia fulgida",
            Perennial,
            Mound,
            Lanceolate,
            2.0,
            2.0,
            [66, 100, 44],
        )
        .flowers([242, 180, 30])
        .d("Golden daisies late summer."),
        plant(
            "Salvia",
            "Salvia nemorosa 'May Night'",
            Perennial,
            Upright,
            Lanceolate,
            2.0,
            2.0,
            [70, 102, 56],
        )
        .flowers([136, 106, 200])
        .d("Deep violet spikes."),
        plant(
            "Agapanthus",
            "Agapanthus africanus",
            Perennial,
            Fountain,
            Blade,
            3.0,
            2.0,
            [64, 106, 50],
        )
        .flowers([110, 130, 216])
        .clim(warm)
        .d("Blue globes over strappy leaves."),
        plant(
            "Peony",
            "Paeonia lactiflora",
            Perennial,
            Mound,
            Compound,
            3.0,
            3.0,
            [64, 100, 50],
        )
        .flowers([236, 168, 190])
        .clim(cold)
        .d("Lush pink double blooms."),
        plant(
            "Russian Sage",
            "Salvia yangii",
            Perennial,
            Upright,
            Needle,
            4.0,
            3.0,
            [142, 156, 140],
        )
        .flowers([150, 140, 210])
        .clim(dry)
        .d("Airy silver stems with lavender haze."),
        plant(
            "Tulip Bed",
            "Tulipa",
            Perennial,
            Upright,
            Lanceolate,
            1.5,
            2.0,
            [86, 130, 90],
        )
        .flowers([226, 40, 50])
        .d("A clump of red tulips."),
        // Rocks & boulders (ADR-095)
        plant(
            "Boulder, Granite",
            "Granite",
            Rock,
            Boulder,
            Fleshy,
            2.5,
            3.5,
            [148, 144, 138],
        )
        .stems(1)
        .d("A weathered granite boulder, set a third into the ground."),
        plant(
            "Boulder Cluster, Fieldstone",
            "Fieldstone",
            Rock,
            Boulder,
            Fleshy,
            2.0,
            5.0,
            [156, 148, 136],
        )
        .stems(5)
        .d("A pile of rounded fieldstones of mixed sizes, as a landscaper sets them in a bed."),
        plant(
            "Stacked Ledge Stones",
            "Limestone",
            Rock,
            Boulder,
            Fleshy,
            1.8,
            5.0,
            [166, 156, 138],
        )
        .stems(5)
        .d("Flat, split limestone pieces stacked loosely: rock gardens and dry beds."),
        plant(
            "Boulder, Large Granite",
            "Granite",
            Rock,
            Boulder,
            Fleshy,
            4.0,
            6.0,
            [138, 134, 128],
        )
        .stems(1)
        .d("A big accent boulder for the corner of a bed or a lawn edge."),
        // Succulents & cacti
        plant(
            "Blue Agave",
            "Agave americana",
            Succulent,
            Rosette,
            Fleshy,
            5.0,
            8.0,
            [112, 146, 150],
        )
        .clim(&[Arid])
        .d("Huge silver-blue rosette of spiky leaves."),
        plant(
            "Spanish Dagger Yucca",
            "Yucca gloriosa",
            Succulent,
            Rosette,
            Fleshy,
            6.0,
            4.0,
            [72, 106, 76],
        )
        .trunk(2.0)
        .cal(5.0)
        .clim(dry)
        .d("Spiky rosette on a short trunk."),
        plant(
            "Aloe",
            "Aloe vera",
            Succulent,
            Rosette,
            Fleshy,
            2.0,
            2.0,
            [98, 132, 92],
        )
        .flowers([230, 110, 50])
        .clim(&[Arid])
        .d("Fleshy rosette with orange flower spikes."),
        plant(
            "Saguaro Cactus",
            "Carnegiea gigantea",
            Succulent,
            Cactus,
            Fleshy,
            30.0,
            8.0,
            [74, 106, 64],
        )
        .cal(20.0)
        .clim(&[Arid])
        .d("The arms-raised desert giant."),
        plant(
            "Golden Barrel Cactus",
            "Echinocactus grusonii",
            Succulent,
            Cactus,
            Fleshy,
            2.0,
            2.0,
            [104, 130, 70],
        )
        .cal(22.0)
        .stems(1)
        .clim(&[Arid])
        .d("Round ribbed cactus with golden spines."),
        plant(
            "Ocotillo",
            "Fouquieria splendens",
            Succulent,
            Fountain,
            Small,
            15.0,
            10.0,
            [96, 124, 60],
        )
        .flowers([224, 60, 40])
        .stems(14)
        .bark(Bark::Green, [110, 100, 80])
        .dense(0.3)
        .clim(&[Arid])
        .d("Tall whip-like canes tipped red."),
    ];
    v.into_iter().map(|b| b.0).collect()
}

/// The Asset Library: every species in its showcase look, and, like Enscape, deciduous and
/// flowering trees also in their other seasons.
pub fn catalog() -> Vec<PlantPreset> {
    let mut out = vec![];
    for p in species() {
        let seasonal = p.spec.group.seasonal() && p.spec.autumn.is_some();
        let base_season = if p.spec.group == PlantGroup::Flowering {
            Season::Spring
        } else {
            Season::Summer
        };
        let mut base = p.clone();
        base.spec.season = base_season;
        out.push(base);
        if !seasonal {
            continue;
        }
        for s in [
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ] {
            if s == base_season {
                continue;
            }
            let mut v = p.clone();
            v.name = format!("{} ({})", p.name, s.label());
            v.spec.season = s;
            out.push(v);
        }
    }
    out
}

/// The library's shape for the Asset Library window.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PlantLibrary {
    pub presets: Vec<PlantPreset>,
    /// (group, its label, its Enscape category).
    pub groups: Vec<(PlantGroup, String, String)>,
    pub climates: Vec<(Climate, String)>,
}

pub fn library() -> PlantLibrary {
    PlantLibrary {
        presets: catalog(),
        groups: PlantGroup::ALL
            .iter()
            .map(|g| (*g, g.label().into(), g.category().into()))
            .collect(),
        climates: Climate::ALL
            .iter()
            .map(|c| (*c, c.label().into()))
            .collect(),
    }
}

// ---------------------------------------------------------------- the model

/// Loads library plants by name (one undo step), reusing types already there by name.
pub fn load(doc: &mut Document, names: &[String]) -> CoreResult<Vec<ElementId>> {
    let all = catalog();
    let picked: Vec<PlantPreset> = names
        .iter()
        .map(|n| {
            all.iter()
                .find(|p| &p.name == n)
                .cloned()
                .ok_or_else(|| CoreError::Invalid(format!("no plant \"{n}\" in the library")))
        })
        .collect::<CoreResult<_>>()?;
    if picked.is_empty() {
        return Err(CoreError::Invalid("choose a plant to load".into()));
    }
    let label = if picked.len() == 1 {
        format!("Load {}", picked[0].name)
    } else {
        format!("Load {} plants", picked.len())
    };
    doc.transact(&label, |tx| {
        let mut out = vec![];
        for p in picked {
            let existing = tx
                .of(Category::PlantingType)
                .find(|e| e.data.name() == p.name)
                .map(|e| e.id);
            out.push(match existing {
                Some(id) => id,
                None => tx.insert(ElementData::PlantingType {
                    name: p.name,
                    spec: p.spec,
                }),
            });
        }
        Ok(out)
    })
}

/// A planting type's spec.
pub fn spec_of(doc: &Document, type_id: ElementId) -> Option<&PlantSpec> {
    match doc.data(type_id).ok()? {
        ElementData::PlantingType { spec, .. } => Some(spec),
        _ => None,
    }
}

/// A level within 5' of grade: plants on it stand on the topography.
pub fn at_grade(doc: &Document, level: ElementId) -> bool {
    doc.level_elevation(level)
        .is_ok_and(|z| z.abs() <= 5.0 * FT)
}

/// The ground's height (project z) at a project point from the site's topography.
pub fn ground_at(doc: &Document, p: Pt) -> Option<f64> {
    let e = doc.of(Category::Site).next()?;
    let ElementData::Site {
        offset,
        rotation,
        base_elevation,
        topo: Some(topo),
        ..
    } = &e.data
    else {
        return None;
    };
    let q = p.sub(*offset);
    let (s, c) = (-rotation).sin_cos();
    let local = Pt::new(q.x * c - q.y * s, q.x * s + q.y * c);
    topo.sample(local).map(|z| z - base_elevation)
}

/// Where a planting's base is (project z): on the topography under a level at grade, else
/// on its level; plus its offset.
pub fn base_z(doc: &Document, level: ElementId, at: Pt, offset: f64) -> f64 {
    let lz = doc.level_elevation(level).unwrap_or(0.0);
    let ground = at_grade(doc, level)
        .then(|| ground_at(doc, at))
        .flatten()
        .unwrap_or(lz);
    ground + offset
}

/// Places plants of `type_id` (Enscape's placement, one undo step): each at a point with its
/// rotation and size.
pub fn create_plants(
    doc: &mut Document,
    type_id: ElementId,
    level: ElementId,
    at: &[(Pt, f64, f64)],
) -> CoreResult<Vec<ElementId>> {
    if spec_of(doc, type_id).is_none() {
        return Err(CoreError::Invalid(
            "pick a plant from the Asset Library".into(),
        ));
    }
    doc.level_elevation(level)?;
    if at.is_empty() {
        return Ok(vec![]);
    }
    let label = if at.len() == 1 {
        "Place plant".to_string()
    } else {
        format!("Place {} plants", at.len())
    };
    doc.transact(&label, |tx| {
        Ok(at
            .iter()
            .map(|(p, rotation, scale)| {
                tx.insert(ElementData::Planting {
                    type_id,
                    level,
                    at: *p,
                    offset: 0.0,
                    rotation: *rotation,
                    scale: scale.clamp(0.1, 10.0),
                })
            })
            .collect())
    })
}

/// A planting's size and place: its spec, base point (project mm, z up), rotation and scale.
pub struct Placed<'a> {
    pub id: ElementId,
    pub type_id: ElementId,
    pub spec: &'a PlantSpec,
    pub at: [f64; 3],
    pub rotation: f64,
    pub scale: f64,
}

/// Every planting.
pub fn placed(doc: &Document) -> Vec<Placed<'_>> {
    doc.of(Category::Planting)
        .filter_map(|e| {
            let ElementData::Planting {
                type_id,
                level,
                at,
                offset,
                rotation,
                scale,
            } = &e.data
            else {
                return None;
            };
            let spec = spec_of(doc, *type_id)?;
            Some(Placed {
                id: e.id,
                type_id: *type_id,
                spec,
                at: [at.x, at.y, base_z(doc, *level, *at, *offset)],
                rotation: *rotation,
                scale: *scale,
            })
        })
        .collect()
}

// ---------------------------------------------------------------- ground

/// The Asset Library's ground materials: the library's Site & Landscape presets.
pub const GROUND_CATEGORY: &str = "Site & Landscape";

/// The base ground's material, if set (and still there).
pub fn ground(doc: &Document) -> Option<ElementId> {
    doc.of(Category::ProjectInfo).find_map(|e| match &e.data {
        ElementData::ProjectInfo { ground, .. } => ground.filter(|m| {
            doc.data(*m)
                .is_ok_and(|d| d.category() == Category::Material)
        }),
        _ => None,
    })
}

/// Sets the base ground's material (None: plain lawn), one undo step.
pub fn set_ground(doc: &mut Document, material: Option<ElementId>) -> CoreResult<()> {
    if let Some(m) = material {
        if doc.data(m)?.category() != Category::Material {
            return Err(CoreError::Invalid("pick a material for the ground".into()));
        }
    }
    let info = doc
        .of(Category::ProjectInfo)
        .next()
        .map(|e| e.id)
        .ok_or_else(|| CoreError::Invalid("no project information".into()))?;
    doc.transact("Base Ground", |tx| {
        tx.modify(info, |d| {
            if let ElementData::ProjectInfo { ground, .. } = d {
                *ground = material;
            }
        })
    })
}

/// A library material for a ground (the project's by name, else added), one undo step.
pub fn ground_material(doc: &mut Document, preset: &str) -> CoreResult<ElementId> {
    crate::library::add_preset(doc, preset)
}

// ---------------------------------------------------------------- properties

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn parse_hex(v: &str) -> CoreResult<[u8; 3]> {
    let s = v.trim().trim_start_matches('#');
    let bad = || CoreError::Invalid(format!("\"{v}\" is not a colour like #4a7a30"));
    if s.len() != 6 {
        return Err(bad());
    }
    let b = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| bad());
    Ok([b(0)?, b(2)?, b(4)?])
}

const SEASONS: [Season; 4] = [
    Season::Spring,
    Season::Summer,
    Season::Autumn,
    Season::Winter,
];

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(data) = doc.data(id) else { return };
    match data {
        ElementData::PlantingType { name, spec } => {
            const ID: &str = "Identity Data";
            const DIM: &str = "Dimensions";
            const LOOK: &str = "Appearance";
            props.push(text("name", "Type Name", ID, name));
            props.push(text("botanical", "Botanical Name", ID, &spec.botanical));
            props.push(ro("group", "Group", ID, spec.group.label().into()));
            props.push(len("height", "Height", DIM, spec.height));
            props.push(len("spread", "Crown Spread", DIM, spec.spread));
            if spec.form == CrownForm::Box {
                props.push(len("depth", "Depth", DIM, spec.depth));
            }
            if spec.group.is_tree() {
                props.push(len("trunk", "Clear Trunk Height", DIM, spec.trunk));
                props.push(len("caliper", "Trunk Caliper", DIM, spec.caliper));
            }
            props.push(choice(
                "form",
                "Crown Form",
                LOOK,
                format!("{:?}", spec.form),
                opts(&CrownForm::ALL, |f| format!("{f:?}"), CrownForm::label),
            ));
            if spec.group.seasonal() {
                props.push(choice(
                    "season",
                    "Season",
                    LOOK,
                    format!("{:?}", spec.season),
                    opts(&SEASONS, |s| format!("{s:?}"), Season::label),
                ));
            }
            props.push(num(
                "density",
                "Crown Density",
                LOOK,
                (spec.density * 100.0).round(),
                "%",
            ));
            props.push(text("leaf", "Leaf Colour", LOOK, &hex(spec.leaf)));
            if let Some(f) = spec.flowers {
                props.push(text("flowers", "Flower Colour", LOOK, &hex(f)));
            }
            props.push(text("bark", "Bark Colour", LOOK, &hex(spec.bark)));
        }
        ElementData::Planting {
            level,
            offset,
            rotation,
            scale,
            ..
        } => {
            const C: &str = "Constraints";
            props.push(choice(
                "level",
                "Level",
                C,
                level.to_string(),
                level_options(doc),
            ));
            let label = if at_grade(doc, *level) && ground_at(doc, Pt::default()).is_some() {
                "Offset from Ground"
            } else {
                "Offset from Level"
            };
            props.push(len("offset", label, C, *offset));
            props.push(text(
                "rotation",
                "Rotation (degrees)",
                C,
                &format!("{:.1}", rotation.to_degrees()),
            ));
            props.push(num(
                "scale",
                "Size",
                "Dimensions",
                (scale * 100.0).round(),
                "%",
            ));
        }
        ElementData::GroundRegion {
            level, material, ..
        } => {
            props.push(choice(
                "material",
                "Material",
                "Materials and Finishes",
                material.to_string(),
                crate::ops::options_of(doc, Category::Material),
            ));
            props.push(choice(
                "level",
                "Level",
                "Constraints",
                level.to_string(),
                level_options(doc),
            ));
            if let Ok(ElementData::GroundRegion { boundary, .. }) = doc.data(id) {
                let a = studio_geom::signed_area(boundary).abs();
                props.push(ro(
                    "area",
                    "Area",
                    "Dimensions",
                    format!("{:.1} SF", a / (FT * FT)),
                ));
            }
        }
        _ => {}
    }
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let mut d = doc.data(id)?.clone();
    let unknown = || CoreError::Invalid(format!("unknown property {key}"));
    let pick = |all: &[String], v: &str| all.iter().any(|a| a == v.trim());
    match &mut d {
        ElementData::PlantingType { name, spec } => match key {
            "name" => *name = non_empty(value)?,
            "botanical" => spec.botanical = value.trim().into(),
            "height" => spec.height = positive(parse_len(value)?)?,
            "spread" => spec.spread = positive(parse_len(value)?)?,
            "depth" => spec.depth = positive(parse_len(value)?)?,
            "trunk" => spec.trunk = parse_len(value)?.clamp(0.0, spec.height * 0.95),
            "caliper" => spec.caliper = positive(parse_len(value)?)?,
            "form" => {
                spec.form = *CrownForm::ALL
                    .iter()
                    .find(|f| format!("{f:?}") == value.trim())
                    .ok_or_else(unknown)?
            }
            "season" => {
                let all: Vec<String> = SEASONS.iter().map(|s| format!("{s:?}")).collect();
                if !pick(&all, value) {
                    return Err(unknown());
                }
                spec.season = *SEASONS
                    .iter()
                    .find(|s| format!("{s:?}") == value.trim())
                    .ok_or_else(unknown)?;
            }
            "density" => spec.density = (parse_num(value, "%")? / 100.0).clamp(0.1, 1.0),
            "leaf" => {
                spec.leaf = parse_hex(value)?;
                spec.leaf_alt = mix(spec.leaf, [168, 176, 96], 0.25);
            }
            "flowers" => spec.flowers = Some(parse_hex(value)?),
            "bark" => spec.bark = parse_hex(value)?,
            _ => return Err(unknown()),
        },
        ElementData::Planting {
            type_id,
            level,
            offset,
            rotation,
            scale,
            ..
        } => match key {
            "type" => {
                let t = parse_id(value)?;
                if spec_of(doc, t).is_none() {
                    return Err(CoreError::Invalid("not a planting type".into()));
                }
                *type_id = t;
            }
            "level" => {
                let l = parse_id(value)?;
                doc.level_elevation(l)?;
                *level = l;
            }
            "offset" => *offset = parse_len(value)?,
            "rotation" => {
                let deg: f64 = value
                    .trim()
                    .trim_end_matches('°')
                    .parse()
                    .map_err(|_| CoreError::Invalid(format!("\"{value}\" is not an angle")))?;
                *rotation = deg.to_radians();
            }
            "scale" => *scale = (parse_num(value, "%")? / 100.0).clamp(0.1, 10.0),
            _ => return Err(unknown()),
        },
        ElementData::GroundRegion {
            level, material, ..
        } => match key {
            "material" => {
                let m = parse_id(value)?;
                if doc.data(m)?.category() != Category::Material {
                    return Err(CoreError::Invalid("not a material".into()));
                }
                *material = m;
            }
            "level" => {
                let l = parse_id(value)?;
                doc.level_elevation(l)?;
                *level = l;
            }
            _ => return Err(unknown()),
        },
        _ => return Err(unknown()),
    }
    let label = format!("Change {}", key.replace('_', " "));
    doc.transact(&label, |tx| tx.set(id, d))
}

/// Options for choosing a planting type (the Properties type list).
pub fn type_options(doc: &Document) -> Vec<PropOption> {
    crate::ops::options_of(doc, Category::PlantingType)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    fn level_at(doc: &Document, z: f64) -> ElementId {
        doc.levels()
            .into_iter()
            .find(|(_, _, e)| (*e - z).abs() < 1.0)
            .unwrap()
            .0
    }

    #[test]
    fn the_library_is_extensive_and_trees_have_seasons() {
        let all = catalog();
        let base: Vec<_> = all.iter().filter(|p| p.name == p.species).collect();
        let trees = base.iter().filter(|p| p.spec.group.is_tree()).count();
        let bushes = base
            .iter()
            .filter(|p| p.spec.group.category() == "Bushes")
            .count();
        assert!(trees >= 90, "{trees} trees");
        assert!(bushes >= 30, "{bushes} bushes");
        assert!(base.len() >= 150, "{}", base.len());
        // Names are unique (types are found by name).
        let mut names: Vec<&str> = all.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n);
        // A red maple in four seasons: autumn is red, winter is bare.
        let maple: Vec<_> = all.iter().filter(|p| p.species == "Red Maple").collect();
        assert_eq!(maple.len(), 4);
        let autumn = maple
            .iter()
            .find(|p| p.spec.season == Season::Autumn)
            .unwrap();
        let (c, _) = autumn.spec.leaf_colors();
        assert!(c[0] > c[1] * 2, "{c:?}");
        let winter = maple
            .iter()
            .find(|p| p.spec.season == Season::Winter)
            .unwrap();
        assert!(!winter.spec.leafy());
        assert!(autumn.spec.leafy());
        // A flowering tree's base asset is in bloom (spring); its summer one isn't.
        let cherry: Vec<_> = all
            .iter()
            .filter(|p| p.species == "Yoshino Cherry")
            .collect();
        assert!(cherry[0].spec.flowers_now().is_some());
        let summer = cherry
            .iter()
            .find(|p| p.spec.season == Season::Summer)
            .unwrap();
        assert!(summer.spec.flowers_now().is_none());
        // Evergreens have no seasons.
        assert_eq!(
            all.iter().filter(|p| p.species == "Norway Spruce").count(),
            1
        );
        // Sizes are real: a sequoia is taller than a boxwood by far; trees have trunks.
        let h = |n: &str| all.iter().find(|p| p.name == n).unwrap().spec.height;
        assert!((h("Giant Sequoia") - 120.0 * FT).abs() < 1e-6);
        assert!((h("Boxwood, Round") - 3.0 * FT).abs() < 1e-6);
        for p in &base {
            assert!(p.spec.height > 0.0 && p.spec.spread > 0.0, "{}", p.name);
            if p.spec.group.is_tree() && p.spec.form != CrownForm::Rosette {
                assert!(p.spec.caliper > 50.0, "{}", p.name);
            }
            assert!(!p.description.is_empty(), "{}", p.name);
        }
    }

    #[test]
    fn plants_load_place_and_edit_in_one_undo_step_each() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = level_at(&doc, 0.0);
        let ids = load(&mut doc, &["Red Maple (Autumn)".into()]).unwrap();
        // Loading again reuses the type.
        assert_eq!(load(&mut doc, &["Red Maple (Autumn)".into()]).unwrap(), ids);
        let t = ids[0];
        assert_eq!(spec_of(&doc, t).unwrap().season, Season::Autumn);
        let placed_ids = create_plants(
            &mut doc,
            t,
            l1,
            &[
                (Pt::new(0.0, 0.0), 0.0, 1.0),
                (Pt::new(5000.0, 0.0), 1.0, 1.2),
            ],
        )
        .unwrap();
        assert_eq!(placed_ids.len(), 2);
        let all = placed(&doc);
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].at, [5000.0, 0.0, 0.0]);
        assert!((all[1].scale - 1.2).abs() < 1e-9);
        // Properties: size and offset.
        ops::set_property(&mut doc, placed_ids[0], "scale", "150%", 0).unwrap();
        ops::set_property(&mut doc, placed_ids[0], "offset", "2'", 0).unwrap();
        let p = placed(&doc);
        let first = p.iter().find(|x| x.id == placed_ids[0]).unwrap();
        assert!((first.scale - 1.5).abs() < 1e-9);
        assert!((first.at[2] - 2.0 * FT).abs() < 1e-6);
        ops::set_property(&mut doc, t, "height", "40'", 0).unwrap();
        assert!((spec_of(&doc, t).unwrap().height - 40.0 * FT).abs() < 1e-6);
        ops::set_property(&mut doc, t, "season", "Winter", 0).unwrap();
        assert!(!spec_of(&doc, t).unwrap().leafy());
        assert!(ops::set_property(&mut doc, t, "leaf", "green", 0).is_err());
        ops::set_property(&mut doc, t, "leaf", "#336622", 0).unwrap();
        assert_eq!(spec_of(&doc, t).unwrap().leaf, [0x33, 0x66, 0x22]);
    }

    #[test]
    fn plants_stand_on_the_topography_at_grade() {
        use crate::site::Topo;
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = level_at(&doc, 0.0);
        let l2 = ops::create_level(&mut doc, 3000.0).unwrap();
        // Ground rising 1 m every 10 m eastward, datum 100 m.
        let topo = Topo {
            x0: -20_000.0,
            y0: -20_000.0,
            spacing: 10_000.0,
            nx: 5,
            ny: 5,
            z: (0..25)
                .map(|k| (100_000.0 + f64::from(k % 5) * 1000.0 - 2000.0) as f32)
                .collect(),
            resolution: 1.0,
        };
        doc.transact("site", |tx| {
            Ok(tx.insert(ElementData::Site {
                address: String::new(),
                lat: 0.0,
                lon: 0.0,
                boundary: vec![],
                parcel: Default::default(),
                offset: Pt::default(),
                rotation: 0.0,
                base_elevation: 100_000.0,
                contour: 1000.0,
                topo: Some(topo),
            }))
        })
        .unwrap();
        assert!((base_z(&doc, l1, Pt::new(5000.0, 0.0), 0.0) - 500.0).abs() < 1e-6);
        // Upper levels (a roof terrace) keep their own height.
        assert!((base_z(&doc, l2, Pt::new(5000.0, 0.0), 0.0) - 3000.0).abs() < 1e-6);
        // Off the topography: the level.
        assert!((base_z(&doc, l1, Pt::new(90_000.0, 0.0), 0.0)).abs() < 1e-6);
    }

    #[test]
    fn a_ground_region_is_sketched_in_a_material_and_moves() {
        use crate::sketch::{finish, SketchCurve, SketchKind};
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = level_at(&doc, 0.0);
        let m = ground_material(&mut doc, &crate::library::library()[0].id).unwrap();
        let (a, b, c, d) = (
            Pt::new(0.0, 0.0),
            Pt::new(6000.0, 0.0),
            Pt::new(6000.0, 3000.0),
            Pt::new(0.0, 3000.0),
        );
        let curves = [
            SketchCurve::line(a, b),
            SketchCurve::line(b, c),
            SketchCurve::line(c, d),
            SketchCurve::line(d, a),
        ];
        // A floor type is not a ground material.
        let ft = doc.of(Category::FloorType).next().unwrap().id;
        assert!(finish(&mut doc, SketchKind::GroundRegion, None, ft, l1, &curves).is_err());
        let r = finish(&mut doc, SketchKind::GroundRegion, None, m, l1, &curves).unwrap();
        let Ok(ElementData::GroundRegion {
            material, boundary, ..
        }) = doc.data(r)
        else {
            panic!()
        };
        assert_eq!(*material, m);
        assert!((studio_geom::signed_area(boundary).abs() - 18e6).abs() < 1.0);
        crate::modify::move_elements(&mut doc, &[r], Pt::new(1000.0, 0.0)).unwrap();
        let Ok(ElementData::GroundRegion { boundary, .. }) = doc.data(r) else {
            panic!()
        };
        assert!(boundary.iter().any(|p| (p.x - 7000.0).abs() < 1e-6));
    }

    #[test]
    fn the_base_ground_is_a_material() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        assert_eq!(ground(&doc), None);
        let preset = crate::library::library().into_iter().next().unwrap().id;
        let m = ground_material(&mut doc, &preset).unwrap();
        set_ground(&mut doc, Some(m)).unwrap();
        assert_eq!(ground(&doc), Some(m));
        let l = doc.levels()[0].0;
        assert!(set_ground(&mut doc, Some(l)).is_err());
        doc.undo().unwrap();
        assert_eq!(ground(&doc), None);
    }
}
