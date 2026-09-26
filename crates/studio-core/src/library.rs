//! The material library (ADR-029): V-Ray-style physical appearances and presets typical of
//! US residential, hospitality and multifamily buildings, from typical to high-end.
//!
//! Photo-real presets use Poly Haven's CC0 texture sets (colour, normal and roughness at
//! 2K), downloaded on first use; tiles, block, carpet, stone veining and brushed metal are
//! procedural; paints, metals and glass need no texture.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, CutPattern, ElementData, ElementId, SurfacePattern};

fn yes() -> bool {
    true
}
fn white() -> [u8; 3] {
    [255, 255, 255]
}
fn default_ior() -> f64 {
    1.5
}
fn default_scale() -> f64 {
    1000.0
}
fn default_roughness() -> f64 {
    0.8
}
fn default_reflection() -> f64 {
    0.5
}
fn default_bump() -> f64 {
    1.0
}

/// How a material renders, in V-Ray's terms (a material's Appearance tab).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Appearance {
    /// The library preset it was made from.
    #[serde(default)]
    pub preset: Option<String>,
    /// Reflection glossiness's complement: 0 mirror-sharp, 1 fully matte.
    #[serde(default = "default_roughness")]
    pub roughness: f64,
    /// 0 dielectric (paint, wood, stone), 1 metal.
    #[serde(default)]
    pub metalness: f64,
    /// Reflection amount (specular), 0–1.
    #[serde(default = "default_reflection")]
    pub reflection: f64,
    /// Refraction (transparency), 0–1: glass is 1.
    #[serde(default)]
    pub refraction: f64,
    /// Index of refraction.
    #[serde(default = "default_ior")]
    pub ior: f64,
    /// Bump (normal map) strength, 0–2.
    #[serde(default = "default_bump")]
    pub bump: f64,
    /// A texture: a Poly Haven set id, or `proc:` and a procedural pattern.
    #[serde(default)]
    pub texture: Option<String>,
    /// Real-world size of one texture tile (mm), as V-Ray's real-world mapping.
    #[serde(default = "default_scale")]
    pub scale: f64,
    /// Multiplies a photo texture's colour (white keeps it as photographed).
    #[serde(default = "white")]
    pub tint: [u8; 3],
    /// Use the texture's colour; off keeps only its relief and gloss, painted in the
    /// material's colour (painted brick, coloured plaster).
    #[serde(default = "yes")]
    pub texture_color: bool,
    /// Clear coat (lacquer, varnish, polish), 0–1.
    #[serde(default)]
    pub coat: f64,
    /// Fabric sheen, 0–1.
    #[serde(default)]
    pub sheen: f64,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            preset: None,
            roughness: default_roughness(),
            metalness: 0.0,
            reflection: default_reflection(),
            refraction: 0.0,
            ior: default_ior(),
            bump: default_bump(),
            texture: None,
            scale: default_scale(),
            tint: white(),
            texture_color: true,
            coat: 0.0,
            sheen: 0.0,
        }
    }
}

/// Price and finish level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum Tier {
    Typical,
    MidRange,
    HighEnd,
}

/// A library material.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub category: String,
    pub tier: Tier,
    pub residential: bool,
    pub hospitality: bool,
    pub multifamily: bool,
    /// What it is and where it's typically used.
    pub description: String,
    /// Shading colour for plans, elevations and the 3D view (sRGB).
    pub color: [u8; 3],
    #[ts(skip)]
    #[serde(skip)]
    pub cut: CutPattern,
    /// Surface pattern preset id for elevations.
    pub surface: String,
    pub appearance: Appearance,
}

/// Library categories, in the browser's order.
pub const CATEGORIES: &[&str] = &[
    "Wood",
    "Stone",
    "Tile",
    "Masonry",
    "Concrete",
    "Plaster & Paint",
    "Metal",
    "Glass",
    "Fabric & Leather",
    "Roofing",
    "Surfaces & Ceilings",
    "Site",
];

struct P {
    id: &'static str,
    name: &'static str,
    cat: &'static str,
    tier: Tier,
    /// "R", "H", "M" letters.
    sectors: &'static str,
    desc: &'static str,
    color: [u8; 3],
    cut: CutPattern,
    surface: &'static str,
    texture: Option<&'static str>,
    scale: f64,
    rough: f64,
    metal: f64,
    refr: f64,
    coat: f64,
    sheen: f64,
    tint: [u8; 3],
    texture_color: bool,
}

const fn p(id: &'static str, name: &'static str, cat: &'static str, tier: Tier) -> P {
    P {
        id,
        name,
        cat,
        tier,
        sectors: "RHM",
        desc: "",
        color: [200, 200, 200],
        cut: CutPattern::None,
        surface: "none",
        texture: None,
        scale: 1000.0,
        rough: 0.8,
        metal: 0.0,
        refr: 0.0,
        coat: 0.0,
        sheen: 0.0,
        tint: [255, 255, 255],
        texture_color: true,
    }
}

impl P {
    const fn sec(mut self, s: &'static str) -> P {
        self.sectors = s;
        self
    }
    const fn d(mut self, s: &'static str) -> P {
        self.desc = s;
        self
    }
    const fn c(mut self, c: [u8; 3]) -> P {
        self.color = c;
        self
    }
    const fn cut(mut self, c: CutPattern, surface: &'static str) -> P {
        self.cut = c;
        self.surface = surface;
        self
    }
    const fn tex(mut self, t: &'static str, scale: f64) -> P {
        self.texture = Some(t);
        self.scale = scale;
        self
    }
    const fn r(mut self, r: f64) -> P {
        self.rough = r;
        self
    }
    const fn metal(mut self, m: f64) -> P {
        self.metal = m;
        self
    }
    const fn glass(mut self) -> P {
        self.refr = 1.0;
        self.rough = 0.0;
        self
    }
    const fn coat(mut self, c: f64) -> P {
        self.coat = c;
        self
    }
    const fn sheen(mut self, s: f64) -> P {
        self.sheen = s;
        self
    }
    const fn tint(mut self, t: [u8; 3]) -> P {
        self.tint = t;
        self
    }
    /// Keep the texture's relief and gloss, painted in the material's colour.
    const fn painted(mut self) -> P {
        self.texture_color = false;
        self
    }
}

use Tier::{HighEnd as HI, MidRange as MID, Typical as TYP};

const PRESETS: &[P] = &[
    // Wood
    p("wood-white-oak-floor", "White Oak Plank Flooring, Matte", "Wood", HI)
        .d("Wide-plank white oak with a matte oil finish; the high-end residential and boutique hotel floor.")
        .c([176, 140, 100]).tex("wood_floor", 1700.0).r(0.55).coat(0.15),
    p("wood-herringbone-oak", "Herringbone Oak Parquet", "Wood", HI).sec("RH")
        .d("Oak parquet in herringbone, satin finish: luxury residences, hotel suites and lobbies.")
        .c([170, 128, 86]).tex("herringbone_parquet", 3400.0).r(0.45).coat(0.3),
    p("wood-lvp-light-oak", "Luxury Vinyl Plank, Light Oak", "Wood", TYP).sec("RM")
        .d("Waterproof LVP in a light oak look; the standard multifamily unit floor.")
        .c([196, 168, 128]).tex("laminate_floor_02", 1700.0).r(0.4),
    p("wood-laminate-warm", "Laminate Flooring, Warm Oak", "Wood", TYP).sec("RM")
        .d("Laminate plank in a warm mid-tone oak: builder-grade residential and rentals.")
        .c([170, 120, 80]).tex("laminate_floor_03", 2080.0).r(0.35),
    p("wood-walnut-veneer", "Walnut Veneer Millwork", "Wood", HI).sec("RH")
        .d("Book-matched walnut veneer, satin lacquer: cabinetry, paneling and hotel casework.")
        .c([120, 86, 62]).tex("walnut_veneer_02", 1000.0).r(0.4).coat(0.4).tint([170, 128, 100]),
    p("wood-white-oak-veneer", "Rift White Oak Veneer", "Wood", MID)
        .d("Rift-cut white oak veneer, clear matte finish: kitchens, built-ins and wall paneling.")
        .c([196, 170, 132]).tex("white_oak_veneer", 500.0).r(0.5).coat(0.2),
    p("wood-espresso-lacquer", "Espresso Lacquered Wood", "Wood", HI).sec("RH")
        .d("Dark wood under a high-gloss lacquer: hotel furniture, reception desks, doors.")
        .c([70, 36, 26]).tex("lacquered_cherry_wood", 1000.0).r(0.15).coat(0.8),
    p("wood-teak", "Teak Veneer", "Wood", HI).sec("RH")
        .d("Warm teak veneer, oiled: mid-century millwork and resort interiors.")
        .c([184, 136, 90]).tex("teak_veneer", 1000.0).r(0.45).coat(0.3),
    p("wood-ipe-deck", "Ipe Decking", "Wood", HI).sec("RH")
        .d("Brazilian hardwood decking: rooftop amenity decks, pool decks and terraces.")
        .c([140, 70, 45]).tex("wood_floor_deck", 1800.0).r(0.6),
    p("wood-weathered-siding", "Weathered Cedar Siding", "Wood", MID).sec("R")
        .d("Silvered cedar boards: modern farmhouse and coastal cladding.")
        .c([130, 128, 122]).cut(CutPattern::None, "lap6").tex("wood_planks_grey", 1500.0).r(0.85),
    p("wood-plywood", "Birch Plywood", "Wood", TYP).sec("RM")
        .d("Clear-coated birch ply: casework, feature walls and ceilings.")
        .c([214, 190, 140]).tex("plywood", 500.0).r(0.6),
    // Stone
    p("stone-carrara", "Carrara Marble, Honed", "Stone", HI)
        .d("White Carrara with soft grey veining, honed: baths, countertops and lobbies.")
        .c([236, 236, 234]).tex("proc:carrara", 1200.0).r(0.25),
    p("stone-calacatta", "Calacatta Gold Marble, Polished", "Stone", HI).sec("RH")
        .d("Bold gold-grey veining on white, polished: luxury baths, bars and feature walls.")
        .c([240, 238, 232]).tex("proc:calacatta", 1500.0).r(0.06).coat(0.3),
    p("stone-travertine", "Travertine, Honed", "Stone", MID).sec("RH")
        .d("Warm beige travertine in large pavers: floors, spa walls and pool surrounds.")
        .c([214, 196, 164]).tex("marble_01", 1500.0).r(0.35),
    p("stone-granite-black", "Absolute Black Granite, Polished", "Stone", MID)
        .d("Near-black granite with fine speckle, polished: countertops and bar tops.")
        .c([30, 30, 32]).tex("proc:granite", 800.0).r(0.06).coat(0.4),
    p("stone-quartz-white", "White Quartz Countertop", "Stone", MID)
        .d("Engineered quartz, fine speckle, polished: the default multifamily and hotel countertop.")
        .c([238, 238, 236]).tex("proc:quartz", 600.0).r(0.1).coat(0.3),
    p("stone-terrazzo", "Terrazzo", "Stone", HI).sec("HM")
        .d("Poured terrazzo with stone chips: hotel lobbies, amenity spaces and corridors.")
        .c([180, 160, 140]).tex("terrazzo_tiles", 2000.0).r(0.2).coat(0.3),
    p("stone-limestone", "Limestone Block Cladding", "Stone", HI).sec("RH")
        .d("Buff limestone in ashlar courses: facades and entries.")
        .c([220, 206, 180]).cut(CutPattern::Masonry, "none").tex("white_sandstone_blocks_02", 2000.0).r(0.85),
    p("stone-ledgestone", "Ledgestone Veneer", "Stone", MID).sec("RH")
        .d("Stacked ledgestone veneer: fireplaces, entry walls and water tables.")
        .c([150, 132, 110]).cut(CutPattern::Masonry, "none").tex("rustic_stone_wall_02", 1500.0).r(0.9),
    p("stone-fieldstone", "Fieldstone Wall", "Stone", MID).sec("R")
        .d("Rounded fieldstone in mortar: rustic homes, garden walls.")
        .c([150, 146, 138]).cut(CutPattern::Masonry, "none").tex("stone_wall", 2000.0).r(0.9),
    p("stone-flagstone", "Flagstone Paving", "Stone", MID).sec("RH")
        .d("Irregular flagstone: patios, walks and terraces.")
        .c([170, 140, 110]).tex("pavement_02", 2120.0).r(0.85),
    // Tile
    p("tile-porcelain-gray", "Large-Format Porcelain, Gray", "Tile", MID)
        .d("24\" x 48\" porcelain in a concrete look: corridors, lobbies, units and baths.")
        .c([150, 150, 148]).tex("large_floor_tiles_02", 3000.0).r(0.3),
    p("tile-stone-charcoal", "Stone Tile, Charcoal", "Tile", MID).sec("HM")
        .d("Charcoal stone tile, honed: entries, bath floors and amenity spaces.")
        .c([90, 90, 92]).tex("granite_tile", 2300.0).r(0.4),
    p("tile-subway-white", "White Subway Tile 3\" x 6\", Gloss", "Tile", TYP).sec("RM")
        .d("Glazed ceramic subway tile, running bond with light grout: kitchen backsplashes and showers.")
        .c([245, 245, 242]).tex("proc:subway", 610.0).r(0.06).coat(0.5),
    p("tile-hex-mosaic", "Hex Mosaic, Matte White", "Tile", MID).sec("RH")
        .d("2\" hexagon porcelain mosaic: bath floors and showers.")
        .c([236, 236, 232]).tex("proc:hex", 300.0).r(0.45),
    p("tile-marble-mosaic", "Marble Mosaic", "Tile", HI).sec("H")
        .d("Small-format marble mosaic: spa and hotel bath floors.")
        .c([200, 196, 190]).tex("marble_tiles", 2000.0).r(0.25),
    // Masonry
    p("masonry-red-brick", "Red Brick, Running Bond", "Masonry", TYP)
        .d("Modular red face brick in running bond: facades and townhouses.")
        .c([150, 70, 52]).cut(CutPattern::Masonry, "brick").tex("red_brick_03", 1000.0).r(0.85),
    p("masonry-common-brick", "Common Brick, Tumbled", "Masonry", TYP).sec("RM")
        .d("Tumbled common brick with a softer, varied face: urban infill and loft conversions.")
        .c([160, 90, 70]).cut(CutPattern::Masonry, "brick").tex("red_brick", 1400.0).r(0.85),
    p("masonry-painted-brick", "Painted Brick, White", "Masonry", MID).sec("RH")
        .d("Brick painted white, keeping the brick's texture: modern farmhouse and interior feature walls.")
        .c([236, 234, 228]).cut(CutPattern::Masonry, "brick").tex("red_brick_03", 1000.0).r(0.7).painted(),
    p("masonry-cmu", "CMU Block, Gray", "Masonry", TYP).sec("M")
        .d("8\" x 16\" concrete masonry units with struck joints: podiums, stairs, back-of-house.")
        .c([170, 168, 162]).cut(CutPattern::Masonry, "block").tex("proc:cmu", 1219.2).r(0.9),
    // Concrete
    p("concrete-polished", "Polished Concrete Floor", "Concrete", MID)
        .d("Ground and polished slab: lofts, amenity spaces, retail and lobbies.")
        .c([122, 120, 116]).cut(CutPattern::Concrete, "none").tex("concrete_floor_02", 2000.0).r(0.25).coat(0.3).painted(),
    p("concrete-architectural", "Architectural Concrete, Smooth", "Concrete", TYP)
        .d("Smooth cast-in-place concrete: podiums, walls and columns.")
        .c([142, 140, 136]).cut(CutPattern::Concrete, "none").tex("concrete_floor_02", 2000.0).r(0.75).painted(),
    p("concrete-board-formed", "Board-Formed Concrete", "Concrete", HI).sec("RH")
        .d("Concrete cast against boards, showing their grain: feature walls and modern facades.")
        .c([160, 158, 152]).cut(CutPattern::Concrete, "none").tex("concrete_layers_02", 2000.0).r(0.85),
    p("concrete-exposed-aggregate", "Exposed Aggregate Paving", "Concrete", TYP).sec("HM")
        .d("Concrete with the aggregate exposed: plazas, pool decks and walks.")
        .c([150, 144, 134]).cut(CutPattern::Concrete, "none").tex("gravel_concrete_03", 2100.0).r(0.9),
    // Plaster & Paint
    p("plaster-stucco-white", "Stucco, Smooth White", "Plaster & Paint", TYP).sec("RM")
        .d("Three-coat stucco, smooth trowel finish: facades.")
        .c([238, 234, 226]).tex("painted_plaster_wall", 2000.0).r(0.85).painted(),
    p("plaster-stucco-sand", "Stucco, Sand Finish, Warm", "Plaster & Paint", TYP).sec("RM")
        .d("Sand-finish stucco in a warm off-white: Mediterranean and Southwest facades.")
        .c([226, 214, 194]).tex("painted_plaster_wall", 1200.0).r(0.9).painted(),
    p("plaster-venetian", "Venetian Plaster, Polished", "Plaster & Paint", HI).sec("RH")
        .d("Burnished lime plaster with depth and soft sheen: hotel lobbies and luxury interiors.")
        .c([232, 228, 220]).tex("painted_plaster_wall", 2000.0).r(0.25).coat(0.3).painted(),
    p("paint-warm-white", "Paint, Warm White, Eggshell", "Plaster & Paint", TYP)
        .d("Warm white eggshell on drywall: the standard wall finish.")
        .c([242, 240, 232]).r(0.7),
    p("paint-ceiling-white", "Paint, Ceiling White, Flat", "Plaster & Paint", TYP)
        .d("Flat bright white for ceilings.")
        .c([248, 248, 246]).r(0.95),
    p("paint-greige", "Paint, Greige, Eggshell", "Plaster & Paint", TYP)
        .d("Warm grey-beige: corridors, units and lobbies.")
        .c([205, 197, 184]).r(0.7),
    p("paint-sage", "Paint, Sage Green, Matte", "Plaster & Paint", MID).sec("RH")
        .d("Muted green: accent walls and hospitality interiors.")
        .c([170, 180, 160]).r(0.8),
    p("paint-navy", "Paint, Deep Navy, Satin", "Plaster & Paint", MID)
        .d("Deep blue satin: accent walls, cabinetry and doors.")
        .c([38, 52, 74]).r(0.5),
    p("paint-charcoal", "Paint, Charcoal, Satin", "Plaster & Paint", MID)
        .d("Charcoal satin: exterior trim, doors and accent walls.")
        .c([60, 62, 64]).r(0.5),
    p("paint-black-gloss", "Paint, Black, Semi-Gloss", "Plaster & Paint", MID)
        .d("Black semi-gloss: doors, trim, railings and windows.")
        .c([22, 22, 24]).r(0.25),
    // Metal
    p("metal-brushed-stainless", "Brushed Stainless Steel", "Metal", MID)
        .d("#4 brushed stainless: appliances, elevator cabs, railings and trim.")
        .c([218, 218, 220]).tex("proc:brushed", 300.0).metal(1.0).r(0.28),
    p("metal-polished-chrome", "Polished Chrome", "Metal", MID).sec("RH")
        .d("Mirror-bright chrome: plumbing fixtures and hardware.")
        .c([235, 235, 238]).metal(1.0).r(0.03),
    p("metal-brushed-nickel", "Brushed Nickel", "Metal", MID)
        .d("Warm brushed nickel: faucets, hardware and lighting.")
        .c([196, 188, 176]).tex("proc:brushed", 300.0).metal(1.0).r(0.3),
    p("metal-satin-brass", "Satin Brass", "Metal", HI).sec("RH")
        .d("Satin (unlacquered look) brass: hotel hardware, lighting and accents.")
        .c([212, 178, 110]).tex("proc:brushed", 300.0).metal(1.0).r(0.3),
    p("metal-polished-brass", "Polished Brass", "Metal", HI).sec("H")
        .d("Polished brass: luxury hotel accents, bars and railings.")
        .c([225, 190, 115]).metal(1.0).r(0.06),
    p("metal-oil-rubbed-bronze", "Oil-Rubbed Bronze", "Metal", MID).sec("RH")
        .d("Dark bronze with warm highlights: traditional hardware and lighting.")
        .c([70, 52, 40]).metal(0.9).r(0.45),
    p("metal-matte-black", "Matte Black Steel", "Metal", MID)
        .d("Powder-coated black steel: windows, railings, fixtures and hardware.")
        .c([28, 28, 30]).metal(0.6).r(0.55),
    p("metal-copper", "Copper", "Metal", HI).sec("RH")
        .d("Bright copper: accents, hoods and roofing details.")
        .c([196, 120, 80]).metal(1.0).r(0.25),
    p("metal-anodized-clear", "Clear Anodized Aluminum", "Metal", TYP).sec("HM")
        .d("Clear anodized aluminum: storefront, curtain wall and window frames.")
        .c([190, 192, 196]).metal(1.0).r(0.35),
    p("metal-anodized-bronze", "Dark Bronze Anodized Aluminum", "Metal", TYP)
        .d("Dark bronze anodized: storefront, windows and railings.")
        .c([60, 50, 42]).metal(1.0).r(0.4),
    p("metal-galvanized", "Galvanized Corrugated Steel", "Metal", MID).sec("RM")
        .d("Corrugated galvanized panels: industrial-modern siding and accents.")
        .c([160, 164, 166]).tex("corrugated_iron", 1120.0).metal(0.8).r(0.45),
    p("metal-corten", "Weathering Steel (Cor-ten)", "Metal", HI).sec("RH")
        .d("Rusted weathering steel: cladding, planters and landscape walls.")
        .c([130, 70, 40]).tex("rust_coarse_01", 2200.0).metal(0.2).r(0.85),
    p("metal-standing-seam", "Standing Seam Metal, Charcoal", "Metal", MID).sec("RM")
        .d("18\" standing seam panels, charcoal Kynar finish: roofs and siding.")
        .c([74, 77, 82]).tex("proc:seam", 457.2).metal(0.7).r(0.35),
    // Glass
    p("glass-clear", "Clear Glass", "Glass", TYP)
        .d("Clear float glass: windows, doors and partitions.")
        .c([245, 250, 248]).glass(),
    p("glass-lowe-gray", "Low-E Glass, Gray", "Glass", MID).sec("HM")
        .d("Low-E insulating glass with a neutral grey cast: curtain wall and storefront.")
        .c([160, 168, 170]).glass(),
    p("glass-lowe-bluegreen", "Low-E Glass, Blue-Green", "Glass", TYP).sec("HM")
        .d("Low-E glass with the familiar blue-green edge: typical commercial glazing.")
        .c([170, 200, 195]).glass(),
    p("glass-frosted", "Frosted Glass, Acid-Etched", "Glass", MID)
        .d("Acid-etched glass: shower enclosures, privacy partitions and doors.")
        .c([236, 242, 242]).glass().r(0.35),
    p("glass-mirror", "Mirror", "Glass", TYP)
        .d("Silvered mirror: baths, elevator lobbies and fitness rooms.")
        .c([240, 240, 240]).metal(1.0).r(0.0),
    p("glass-back-painted", "Back-Painted Glass, White", "Glass", MID).sec("RH")
        .d("Glass painted on the back: backsplashes and wall panels.")
        .c([240, 240, 236]).r(0.02).coat(1.0),
    // Fabric & Leather
    p("fabric-carpet-loop", "Loop Carpet, Charcoal", "Fabric & Leather", TYP).sec("HM")
        .d("Commercial loop-pile carpet tile: corridors and offices.")
        .c([80, 82, 86]).tex("proc:carpet", 300.0).r(0.95).sheen(0.3),
    p("fabric-carpet-hotel", "Patterned Hotel Carpet", "Fabric & Leather", MID).sec("H")
        .d("Broadloom with a geometric pattern: hotel corridors and ballrooms.")
        .c([120, 90, 70]).tex("proc:carpet-pattern", 900.0).r(0.95).sheen(0.3),
    p("fabric-linen", "Linen Upholstery, Natural", "Fabric & Leather", MID).sec("RH")
        .d("Natural linen weave: upholstery and drapery.")
        .c([200, 196, 180]).tex("leather_white", 300.0).r(0.9).sheen(0.4),
    p("fabric-velvet", "Velvet, Crimson", "Fabric & Leather", HI).sec("H")
        .d("Deep velvet: hotel lounge seating and banquettes.")
        .c([150, 30, 40]).tex("velour_velvet", 280.0).r(0.8).sheen(1.0),
    p("fabric-leather-brown", "Leather, Saddle Brown", "Fabric & Leather", HI).sec("RH")
        .d("Aniline leather: lounge seating and bar stools.")
        .c([110, 64, 40]).tex("brown_leather", 400.0).r(0.5).coat(0.1),
    // Roofing
    p("roof-asphalt-shingle", "Architectural Asphalt Shingles, Charcoal", "Roofing", TYP).sec("RM")
        .d("Laminated asphalt shingles: the standard pitched roof.")
        .c([80, 84, 90]).cut(CutPattern::None, "shingle5").tex("grey_roof_01", 2000.0).r(0.9),
    p("roof-cedar-shake", "Cedar Shake Roof", "Roofing", HI).sec("R")
        .d("Split cedar shakes: craftsman, shingle-style and coastal homes.")
        .c([140, 110, 80]).tex("roof_09", 1500.0).r(0.85),
    p("roof-clay-tile", "Clay Barrel Tile", "Roofing", MID).sec("RH")
        .d("Terracotta barrel (Spanish) tile: Mediterranean and Southwest roofs.")
        .c([180, 90, 50]).tex("clay_roof_tiles_02", 2500.0).r(0.7),
    p("roof-slate", "Slate Roof", "Roofing", HI).sec("R")
        .d("Natural slate shingles: traditional high-end roofs.")
        .c([70, 74, 80]).tex("roof_slates_02", 3000.0).r(0.6),
    p("roof-tpo-white", "TPO Membrane, White", "Roofing", TYP).sec("HM")
        .d("White single-ply membrane: low-slope multifamily and hotel roofs.")
        .c([232, 234, 234]).r(0.6),
    // Surfaces & Ceilings
    p("surface-solid-white", "Solid Surface, Glacier White", "Surfaces & Ceilings", MID).sec("HM")
        .d("Seamless acrylic solid surface: vanities, counters and reception desks.")
        .c([242, 242, 240]).r(0.2),
    p("surface-laminate-gray", "Plastic Laminate, Warm Gray", "Surfaces & Ceilings", TYP).sec("M")
        .d("High-pressure laminate: casework and countertops.")
        .c([122, 122, 118]).r(0.35),
    p("ceiling-act", "Acoustic Ceiling Tile, 2' x 2'", "Surfaces & Ceilings", TYP).sec("HM")
        .d("Mineral fiber tile in a white grid: corridors and back-of-house.")
        .c([238, 238, 234]).tex("proc:act", 609.6).r(0.95),
    // Site
    p("site-asphalt", "Asphalt Paving", "Site", TYP)
        .d("Asphalt: drives, parking and streets.")
        .c([70, 70, 70]).tex("asphalt_02", 3000.0).r(0.9),
    p("site-lawn", "Lawn", "Site", TYP)
        .d("Turf grass.")
        .c([70, 100, 40]).tex("proc:grass", 1000.0).r(0.95).sheen(0.3),
];

/// The library, in browser order.
pub fn library() -> Vec<Preset> {
    PRESETS
        .iter()
        .map(|p| Preset {
            id: p.id.into(),
            name: p.name.into(),
            category: p.cat.into(),
            tier: p.tier,
            residential: p.sectors.contains('R'),
            hospitality: p.sectors.contains('H'),
            multifamily: p.sectors.contains('M'),
            description: p.desc.into(),
            color: p.color,
            cut: p.cut,
            surface: p.surface.into(),
            appearance: Appearance {
                preset: Some(p.id.into()),
                roughness: p.rough,
                metalness: p.metal,
                reflection: if p.refr > 0.0 || p.metal > 0.0 {
                    1.0
                } else {
                    0.5
                },
                refraction: p.refr,
                ior: if p.refr > 0.0 { 1.52 } else { 1.5 },
                bump: 1.0,
                texture: p.texture.map(Into::into),
                scale: p.scale,
                tint: p.tint,
                texture_color: p.texture_color,
                coat: p.coat,
                sheen: p.sheen,
            },
        })
        .collect()
}

pub fn preset(id: &str) -> Option<Preset> {
    library().into_iter().find(|p| p.id == id)
}

/// Maps of a photo texture set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TextureMap {
    Color,
    Normal,
    Roughness,
}

/// Where a library photo texture's 2K map is published (Poly Haven, CC0). None for
/// procedural textures and for ids the library doesn't use.
pub fn texture_url(texture: &str, map: TextureMap) -> Option<String> {
    let used = PRESETS.iter().any(|p| p.texture == Some(texture));
    if !used
        || texture.starts_with("proc:")
        || !texture
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let suffix = match map {
        TextureMap::Color if texture == "brown_leather" => "albedo",
        TextureMap::Color => "diff",
        TextureMap::Normal => "nor_gl",
        TextureMap::Roughness => "rough",
    };
    Some(format!(
        "https://dl.polyhaven.org/file/ph-assets/Textures/jpg/2k/{texture}/{texture}_{suffix}_2k.jpg"
    ))
}

fn unique_name(doc: &Document, base: &str) -> String {
    let taken = |n: &str| doc.of(Category::Material).any(|e| e.data.name() == n);
    if !taken(base) {
        return base.to_owned();
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| base.to_owned())
}

/// Adds a library material to the project.
pub fn add_preset(doc: &mut Document, id: &str) -> CoreResult<ElementId> {
    let p = preset(id).ok_or_else(|| CoreError::Invalid(format!("no library material {id}")))?;
    let name = unique_name(doc, &p.name);
    let surface = SurfacePattern::presets()
        .into_iter()
        .find(|(s, _, _)| *s == p.surface)
        .map_or(SurfacePattern::None, |(_, _, s)| s);
    doc.transact("Add material from library", |tx| {
        Ok(tx.insert(ElementData::Material {
            name,
            cut: p.cut,
            surface,
            color: p.color,
            appearance: p.appearance.clone(),
        }))
    })
}

/// The material an element shows on its outside face: a wall's first (exterior) layer, a
/// floor's, ceiling's or roof's top layer, a column's or beam's type material.
pub fn finish_of(doc: &Document, el: ElementId) -> Option<ElementId> {
    let type_id = doc.data(el).ok()?.type_id()?;
    let data = doc.data(type_id).ok()?;
    match data {
        ElementData::WallType { layers, .. }
        | ElementData::FloorType { layers, .. }
        | ElementData::CeilingType { layers, .. }
        | ElementData::RoofType { layers, .. } => crate::material::resolve(doc, layers.first()?).id,
        ElementData::ColumnType { material, name, .. }
        | ElementData::BeamType { material, name, .. } => {
            crate::material::resolve_type(doc, *material, name).id
        }
        _ => None,
    }
}

/// Applies `material` to the outside finish of each element's type (so every element of
/// that type changes, as in Revit). Returns how many types changed.
pub fn apply_to(
    doc: &mut Document,
    elements: &[ElementId],
    material: ElementId,
) -> CoreResult<usize> {
    if !matches!(doc.data(material)?, ElementData::Material { .. }) {
        return Err(CoreError::Invalid("pick a material to apply".into()));
    }
    let mut types: Vec<ElementId> = elements
        .iter()
        .filter_map(|e| doc.data(*e).ok()?.type_id())
        .collect();
    types.sort();
    types.dedup();
    let name = doc.data(material)?.name();
    let changed = doc.transact("Apply material", |tx| {
        let mut n = 0;
        for t in &types {
            let mut hit = false;
            tx.modify(*t, |d| match d {
                ElementData::WallType { layers, .. }
                | ElementData::FloorType { layers, .. }
                | ElementData::CeilingType { layers, .. }
                | ElementData::RoofType { layers, .. } => {
                    if let Some(l) = layers.first_mut() {
                        l.material = Some(material);
                        l.name = name.clone();
                        hit = true;
                    }
                }
                ElementData::ColumnType { material: m, .. }
                | ElementData::BeamType { material: m, .. } => {
                    *m = Some(material);
                    hit = true;
                }
                _ => {}
            })?;
            n += usize::from(hit);
        }
        Ok(n)
    })?;
    if changed == 0 {
        return Err(CoreError::Invalid(
            "select walls, floors, ceilings, roofs, columns or beams to apply a material to".into(),
        ));
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    #[test]
    fn the_library_covers_us_building_types_and_tiers() {
        let lib = library();
        assert!(lib.len() >= 75, "{}", lib.len());
        let mut ids: Vec<&str> = lib.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), lib.len(), "ids are unique");
        for cat in CATEGORIES {
            assert!(lib.iter().any(|p| p.category == *cat), "{cat}");
        }
        for tier in [Tier::Typical, Tier::MidRange, Tier::HighEnd] {
            assert!(
                lib.iter().filter(|p| p.tier == tier).count() >= 15,
                "{tier:?}"
            );
        }
        assert!(lib.iter().filter(|p| p.hospitality).count() >= 40);
        assert!(lib.iter().filter(|p| p.multifamily).count() >= 40);
        assert!(lib.iter().filter(|p| p.residential).count() >= 50);
        // Every photo texture is downloadable; procedural ones aren't fetched.
        for p in &lib {
            match p.appearance.texture.as_deref() {
                Some(t) if t.starts_with("proc:") => {
                    assert!(texture_url(t, TextureMap::Color).is_none())
                }
                Some(t) => {
                    let u = texture_url(t, TextureMap::Normal).unwrap();
                    assert!(
                        u.starts_with("https://dl.polyhaven.org/") && u.ends_with("_nor_gl_2k.jpg")
                    );
                }
                None => {}
            }
            assert!(p.appearance.scale > 100.0);
            assert!((0.0..=1.0).contains(&p.appearance.roughness));
        }
        let glass = preset("glass-clear").unwrap().appearance;
        assert_eq!((glass.refraction, glass.ior), (1.0, 1.52));
        assert!(
            preset("metal-polished-chrome")
                .unwrap()
                .appearance
                .metalness
                == 1.0
        );
        assert_eq!(
            texture_url("brown_leather", TextureMap::Color).unwrap(),
            "https://dl.polyhaven.org/file/ph-assets/Textures/jpg/2k/brown_leather/brown_leather_albedo_2k.jpg"
        );
        assert!(texture_url("../../etc", TextureMap::Color).is_none());
        assert!(texture_url("not_in_library", TextureMap::Color).is_none());
    }

    #[test]
    fn presets_become_project_materials_and_apply_to_types() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let oak = add_preset(&mut doc, "wood-white-oak-floor").unwrap();
        let again = add_preset(&mut doc, "wood-white-oak-floor").unwrap();
        assert_eq!(
            doc.data(again).unwrap().name(),
            "White Oak Plank Flooring, Matte 2"
        );
        let ElementData::Material {
            appearance, color, ..
        } = doc.data(oak).unwrap()
        else {
            panic!()
        };
        assert_eq!(appearance.preset.as_deref(), Some("wood-white-oak-floor"));
        assert_eq!(appearance.texture.as_deref(), Some("wood_floor"));
        assert_eq!(*color, [176, 140, 100]);
        assert!(add_preset(&mut doc, "nope").is_err());

        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let w = ops::create_wall(
            &mut doc,
            wt,
            l1,
            studio_geom::Pt::new(0.0, 0.0),
            studio_geom::Pt::new(4000.0, 0.0),
        )
        .unwrap();
        let brick = add_preset(&mut doc, "masonry-red-brick").unwrap();
        assert_eq!(apply_to(&mut doc, &[w], brick).unwrap(), 1);
        assert_eq!(finish_of(&doc, w), Some(brick));
        // The level (no type) can't take one.
        assert!(apply_to(&mut doc, &[l1], brick).is_err());
        doc.undo().unwrap();
        assert_ne!(finish_of(&doc, w), Some(brick));
    }

    #[test]
    fn older_materials_get_default_appearance() {
        let a = Appearance::default();
        assert_eq!(
            (a.roughness, a.refraction, a.scale, a.tint),
            (0.8, 0.0, 1000.0, [255, 255, 255])
        );
        assert!(a.texture_color && a.texture.is_none());
    }
}
