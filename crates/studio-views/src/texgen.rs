//! Generated material textures (ADR-061): exterior wood sidings and modern roofing, and
//! (ADR-064) landscape ground covers: lawns, litter and mulch, gravels, paving and soils, made
//! here at any resolution (4096 px in the app) as seamless colour, normal and roughness
//! maps. Every pattern repeats exactly across its tile: noise lattices, board courses,
//! panel widths and joints all divide the tile's real-world size.

/// A generated texture set: `size` × `size` pixels, rows top to bottom.
pub struct GenTexture {
    pub size: usize,
    /// sRGB colour, 3 bytes a pixel.
    pub color: Vec<u8>,
    /// OpenGL tangent-space normals (+y up the tile), 3 bytes a pixel.
    pub normal: Vec<u8>,
    /// Roughness, 3 bytes a pixel (grey).
    pub rough: Vec<u8>,
}

/// The generated sets: (id, real-world tile size in mm, what it is).
pub const KINDS: &[(&str, f64, &str)] = &[
    ("cedar-bevel", FT8, "clear cedar bevel lap, 6\" exposure"),
    (
        "cedar-lap-stained",
        FT8,
        "cedar bevel lap under a warm semi-transparent stain, 6\" exposure",
    ),
    (
        "cedar-bevel-weathered",
        FT8,
        "weathered cedar bevel lap, 6\" exposure",
    ),
    ("shiplap8", FT8, "shiplap, 8\" boards (painted)"),
    ("dutch-lap6", FT8, "Dutch lap, 6\" exposure (painted)"),
    ("board-batten12", FT8, "board and batten, 12\" (painted)"),
    (
        "board-batten-cedar",
        FT8,
        "board and batten, 12\", natural cedar",
    ),
    (
        "cedar-vertical-tg",
        FT8,
        "vertical tongue and groove cedar, 6\"",
    ),
    ("thermo-ash", FT8, "horizontal thermally modified ash, 6\""),
    (
        "shou-sugi-ban",
        FT8,
        "charred cedar (shou sugi ban), vertical 6\"",
    ),
    ("cedar-shingles", FT8, "cedar shingles, 6\" exposure"),
    (
        "cedar-shingles-weathered",
        FT8,
        "weathered cedar shingles, 6\" exposure",
    ),
    ("channel-rustic", FT8, "channel rustic, 8\", stained"),
    ("ipe-rainscreen", FT8, "open-joint ipe rainscreen, 6\""),
    (
        "accoya-slats",
        FT8,
        "vertical Accoya slats, 1-1/2\" with 1/2\" gaps",
    ),
    ("barn-wood", FT8, "reclaimed barn wood, random widths"),
    (
        "asphalt-charcoal",
        SHINGLE_TILE,
        "architectural asphalt shingles, charcoal",
    ),
    (
        "asphalt-black",
        SHINGLE_TILE,
        "architectural asphalt shingles, black",
    ),
    (
        "asphalt-weathered-wood",
        SHINGLE_TILE,
        "architectural asphalt shingles, weathered wood",
    ),
    (
        "asphalt-pewter",
        SHINGLE_TILE,
        "architectural asphalt shingles, pewter gray",
    ),
    ("seam16", FT8, "standing seam, 16\" panels (painted)"),
    ("seam18", FT9, "standing seam, 18\" panels (painted)"),
    ("galvalume16", FT8, "standing seam, 16\" bare Galvalume"),
    ("epdm", FT10, "EPDM membrane with lap seams"),
    ("ballast", FT4, "river rock ballast"),
    ("sedum", FT4, "sedum green roof"),
    (
        "flat-concrete-tile",
        CONCRETE_TILE,
        "flat concrete roof tile (painted)",
    ),
    // Ground covers (ADR-064): lawns, litter and mulch, gravels, paving and soils.
    ("lawn", FT8, "manicured lawn, fine turf"),
    ("lawn-lush", FT8, "lush lawn, deep green"),
    ("lawn-dry", FT8, "summer-dry lawn with straw patches"),
    ("meadow", FT8, "meadow grasses with clover and wildflowers"),
    ("pine-straw", FT8, "long-leaf pine straw with fallen cones"),
    ("leaf-litter", FT4, "autumn leaf litter"),
    ("bark-mulch", FT4, "shredded hardwood bark mulch"),
    ("black-mulch", FT4, "shredded mulch, dyed black"),
    ("red-mulch", FT4, "shredded mulch, dyed red"),
    ("pea-gravel", FT4, "pea gravel, 3/8\" mixed pebbles"),
    ("crushed-stone", FT4, "crushed stone, 3/4\" gray"),
    ("river-rock", FT8, "river rock cobbles, 2\"-4\""),
    ("decomposed-granite", FT4, "decomposed granite, compacted"),
    ("asphalt", FT4, "asphalt paving"),
    (
        "asphalt-worn",
        FT8,
        "worn asphalt, sealed cracks and a patch",
    ),
    (
        "concrete-broom",
        FT10,
        "broom-finished concrete, 5' tooled joints",
    ),
    (
        "pavers-running",
        FT4,
        "concrete pavers 6\" x 12\", running bond",
    ),
    (
        "pavers-herringbone",
        FT4,
        "clay brick pavers 4\" x 8\", herringbone",
    ),
    ("flagstone", FT8, "irregular bluestone flagging, mortared"),
    (
        "bluestone-pattern",
        FT10,
        "thermal-finish bluestone pavers 24\" x 36\" and 24\" x 24\", 3/8\" joints",
    ),
    (
        "bluestone-slab",
        FT4,
        "cleft bluestone slab, no joints (stepping stones)",
    ),
    ("sand", FT4, "sand"),
    ("soil", FT4, "garden soil"),
    ("moss", FT4, "moss"),
    ("snow", FT4, "snow"),
];

const IN: f64 = 25.4;
const FT4: f64 = 48.0 * IN;
const FT8: f64 = 96.0 * IN;
const FT9: f64 = 108.0 * IN;
const FT10: f64 = 120.0 * IN;
/// Eight 5-5/8" courses.
const SHINGLE_TILE: f64 = 45.0 * IN;
/// Four 13" courses of three tiles.
const CONCRETE_TILE: f64 = 52.0 * IN;

/// A kind's tile size (mm).
pub fn tile_of(kind: &str) -> Option<f64> {
    KINDS.iter().find(|k| k.0 == kind).map(|k| k.1)
}

// ---------------------------------------------------------------- noise

fn hash(i: i64, j: i64, seed: u32) -> f64 {
    let mut h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ u64::from(seed).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise over the tile (u, v in tile units, wrapping), `nx` × `ny` cells.
fn vnoise(u: f64, v: f64, nx: i64, ny: i64, seed: u32) -> f64 {
    let (x, y) = (u * nx as f64, v * ny as f64);
    let (gx, gy) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - gx), smooth(y - gy));
    let (gx, gy) = (gx as i64, gy as i64);
    let c = |a: i64, b: i64| hash(a.rem_euclid(nx), b.rem_euclid(ny), seed);
    let top = c(gx, gy) * (1.0 - fx) + c(gx + 1, gy) * fx;
    let bot = c(gx, gy + 1) * (1.0 - fx) + c(gx + 1, gy + 1) * fx;
    top * (1.0 - fy) + bot * fy
}

/// Fractal noise, 0–1: octaves doubling the cells.
fn fbm(u: f64, v: f64, nx: i64, ny: i64, octaves: u32, seed: u32) -> f64 {
    let (mut s, mut a, mut w) = (0.0, 0.5, 0.0);
    for o in 0..octaves {
        let k = 1i64 << o;
        s += a * vnoise(u, v, nx * k, ny * k, seed.wrapping_add(o * 7919));
        w += a;
        a *= 0.5;
    }
    s / w
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    smooth(((x - a) / (b - a)).clamp(0.0, 1.0))
}

fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale3(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// sRGB 0–255 to linear.
fn lin(c: [u8; 3]) -> [f64; 3] {
    c.map(|v| (f64::from(v) / 255.0).powf(2.2))
}

/// One pixel of a pattern: linear colour, height (mm) and roughness (0–1).
#[derive(Clone, Copy)]
struct Px {
    c: [f64; 3],
    h: f64,
    r: f64,
}

// ---------------------------------------------------------------- wood

#[derive(Clone, Copy)]
struct Species {
    early: [u8; 3],
    late: [u8; 3],
    /// Ring (growth band) width, mm.
    ring: f64,
    /// How much boards differ in tone (0–0.3).
    spread: f64,
    /// Knots per metre of board.
    knots: f64,
    rough: f64,
    /// Silvered: grey, with dark streaks along the grain.
    weathered: bool,
    /// Board colours to pick from (earlywood, latewood); empty uses `early` and `late`.
    palette: &'static [[[u8; 3]; 2]],
}

/// Western red cedar heartwood as it's milled (ADR-062): honey, salmon, amber and
/// chocolate boards, and now and then pale sapwood.
const CEDAR_TONES: &[[[u8; 3]; 2]] = &[
    [[198, 134, 86], [136, 80, 46]],
    [[190, 122, 86], [128, 70, 46]],
    [[178, 108, 62], [118, 62, 34]],
    [[192, 130, 82], [134, 80, 46]],
    [[146, 86, 54], [96, 50, 30]],
    [[172, 106, 68], [112, 60, 38]],
    [[214, 176, 128], [170, 126, 84]],
];

const CEDAR: Species = Species {
    early: [200, 146, 100],
    late: [146, 92, 58],
    ring: 3.2,
    spread: 0.12,
    knots: 0.35,
    rough: 0.72,
    weathered: false,
    palette: CEDAR_TONES,
};
const WEATHERED: Species = Species {
    early: [168, 164, 156],
    late: [118, 114, 108],
    ring: 3.2,
    spread: 0.14,
    knots: 0.35,
    rough: 0.9,
    weathered: true,
    palette: CEDAR_TONES,
};
const THERMO_ASH: Species = Species {
    early: [132, 86, 54],
    late: [88, 54, 32],
    ring: 4.5,
    spread: 0.1,
    knots: 0.05,
    rough: 0.6,
    weathered: false,
    palette: &[],
};
const IPE: Species = Species {
    early: [124, 78, 50],
    late: [86, 50, 32],
    ring: 1.6,
    spread: 0.18,
    knots: 0.0,
    rough: 0.55,
    weathered: false,
    palette: &[],
};
const ACCOYA: Species = Species {
    early: [214, 188, 148],
    late: [178, 146, 104],
    ring: 5.0,
    spread: 0.08,
    knots: 0.0,
    rough: 0.7,
    weathered: false,
    palette: &[],
};
/// Semitransparent brown stain over cedar.
const STAINED: Species = Species {
    early: [120, 74, 44],
    late: [80, 46, 26],
    ring: 3.2,
    spread: 0.12,
    knots: 0.3,
    rough: 0.65,
    weathered: false,
    palette: &[],
};

/// Cedar under a warm semi-transparent stain (ADR-095): the stain evens the boards toward
/// one honey-brown, the grain still showing through softly.
const STAINED_CEDAR: Species = Species {
    early: [178, 112, 58],
    late: [150, 92, 48],
    ring: 6.0,
    spread: 0.16,
    knots: 0.08,
    rough: 0.62,
    weathered: false,
    palette: &[],
};

/// Wood at `along` (mm, with the grain) and `across` (mm) on board `seed`, tile-periodic.
fn wood(sp: &Species, along: f64, across: f64, seed: u32, tile: f64) -> Px {
    let (u, v) = (along / tile, across / tile);
    let rings_per_tile = (tile / sp.ring).round() as i64;
    let cells = |mm: f64| (tile / mm).round().max(1.0) as i64;
    // Growth rings run the board's length, bending gently (flat-sawn cathedrals): a slow
    // wander along the grain, a finer one across it, a couple of rings at most.
    let warp = fbm(u, v, cells(220.0), cells(90.0), 3, seed) * 2.6
        + fbm(u, v, cells(60.0), cells(14.0), 2, seed ^ 99) * 0.45;
    let phase = v * rings_per_tile as f64 + warp + hash(seed as i64, 7, 3) * 10.0;
    // Earlywood fades into latewood, which stops sharply at the next ring.
    let band = phase.rem_euclid(1.0);
    let late = smoothstep(0.62, 0.9, band) * (1.0 - smoothstep(0.955, 0.995, band));
    // Fibres: long fine streaks along the grain, and the open pores.
    let fibre = fbm(u, v, cells(70.0), cells(0.7), 2, seed ^ 7);
    let streak = fbm(u, v, cells(400.0), cells(4.0), 2, seed ^ 19);
    let pores = vnoise(u, v, cells(2.5), cells(0.3), seed ^ 31);
    // The board's own tone, drifting along its length.
    let drift = fbm(u, v, cells(600.0), cells(150.0), 2, seed ^ 23) - 0.5;
    let tone = 1.0 + (hash(seed as i64, 1, 11) - 0.5) * 2.0 * sp.spread + drift * 0.18;
    let warmth = (hash(seed as i64, 2, 13) - 0.5) * sp.spread;
    let (early, latec) = if sp.palette.is_empty() {
        (sp.early, sp.late)
    } else {
        // Pale sapwood only now and then.
        let k = hash(seed as i64, 6, 5);
        let n = sp.palette.len();
        let i = if k > 0.985 {
            n - 1
        } else {
            ((k / 0.985) * (n - 1) as f64) as usize
        };
        (sp.palette[i][0], sp.palette[i][1])
    };
    // Boards differ, but as one lot of wood: each pick pulled toward the lot's mean.
    let (mut e, mut l) = (lin(early), lin(latec));
    if !sp.palette.is_empty() {
        let mix = if sp.weathered { 0.8 } else { 0.62 };
        let n = sp.palette.len() as f64;
        let mean = |k: usize| -> [f64; 3] {
            let s = sp.palette.iter().fold([0.0; 3], |a, p| {
                let c = lin(p[k]);
                [a[0] + c[0], a[1] + c[1], a[2] + c[2]]
            });
            scale3(s, 1.0 / n)
        };
        e = lerp3(e, mean(0), mix);
        l = lerp3(l, mean(1), mix);
    }
    let mut c = lerp3(e, l, late + (1.0 - streak) * 0.3);
    c = [c[0] * (1.0 + warmth), c[1], c[2] * (1.0 - warmth)];
    c = scale3(c, tone * (0.84 + 0.32 * fibre) * (0.94 + 0.12 * pores));
    let mut h = -0.12 * late + 0.06 * fibre - 0.04 * (pores - 0.5);
    // Latewood is denser and a little glossier; open pores catch less light.
    let mut r = sp.rough - 0.08 * late + 0.06 * (fibre - 0.5) + 0.05 * (pores - 0.5);
    // Knots: dark ovals, the grain bending round them.
    if sp.knots > 0.0 {
        let per = (tile / 1000.0 * sp.knots).round().max(1.0) as i64;
        for k in 0..per {
            if hash(seed as i64, 100 + k, 17) > 0.6 {
                continue;
            }
            let kx = hash(seed as i64, 200 + k, 19) * tile;
            let dx = ((along - kx + tile / 2.0).rem_euclid(tile)) - tile / 2.0;
            let ky = (hash(seed as i64, 300 + k, 23) - 0.5) * 60.0;
            let rad = 7.0 + hash(seed as i64, 400 + k, 29) * 9.0;
            let d = ((dx / (rad * 1.4)).powi(2) + ((across - ky) / rad).powi(2)).sqrt();
            if d < 3.0 {
                let ring = 0.5 + 0.5 * (d * 9.0).sin();
                let halo = smoothstep(3.0, 0.8, d) * 0.35;
                c = scale3(c, 1.0 - halo * ring);
                if d < 1.0 {
                    let core = smoothstep(1.0, 0.6, d);
                    c = lerp3(c, scale3(lin(sp.late), 0.35), core);
                    h -= 0.2 * core;
                    r -= 0.1 * core;
                }
            }
        }
    }
    if sp.weathered {
        // Silvered: greyed, with dark water streaks along the grain.
        let streak = fbm(u, v, 3, (tile / 12.0) as i64, 3, seed ^ 55);
        let g = (c[0] + c[1] + c[2]) / 3.0;
        c = lerp3(c, [g; 3], 0.8);
        c = scale3(c, 0.8 + 0.35 * streak);
        // Water runs off each board's lower edge: a darker band there, blotched.
        let edge = (-(across.rem_euclid(tile)).min(60.0) / 18.0).exp();
        let blot = fbm(u, v, cells(80.0), cells(20.0), 2, seed ^ 77);
        c = scale3(c, 1.0 - 0.22 * edge * (0.6 + 0.8 * blot));
        h += 0.08 * (fibre - 0.5);
        r = 0.9;
    }
    Px { c, h, r }
}

/// Paint over wood: the paint's colour, the grain telegraphing through its relief.
fn painted(base: Px, paint: [f64; 3]) -> Px {
    let shade = (base.c[0] + base.c[1] + base.c[2]) / 3.0;
    let _ = shade;
    Px {
        c: paint,
        h: base.h * 0.5,
        r: 0.55,
    }
}

/// Where `x` falls in a repeat of `w`: (index, position within).
fn cell(x: f64, w: f64) -> (i64, f64) {
    let i = (x / w).floor();
    (i as i64, x - i * w)
}

/// Like [`cell`], the index wrapped to the repeats in a tile, so what it seeds (a board's
/// tone, a course's joints) is the same a tile over.
fn cellw(x: f64, w: f64, tile: f64) -> (i64, f64) {
    let (i, t) = cell(x, w);
    (i.rem_euclid((tile / w).round().max(1.0) as i64), t)
}

/// Board end joints along a course: `n` staggered joints per tile, the board index and
/// the distance to the nearest joint.
fn joints(x: f64, course: i64, n: i64, tile: f64, seed: u32) -> (i64, f64) {
    let step = tile / n as f64;
    let off = hash(course, 5, seed) * step;
    let (i, t) = cell(x - off, step);
    let seg = i.rem_euclid(n);
    (course * 17 + seg, t.min(step - t))
}

// ---------------------------------------------------------------- sidings

/// Horizontal lap boards: `exposure` courses; `profile(t)` the face's height (mm) at `t`
/// from the butt (0) to the top (1), and the shadow under each butt.
fn lap(
    x: f64,
    y: f64,
    tile: f64,
    exposure: f64,
    profile: impl Fn(f64) -> f64,
    board: impl Fn(f64, f64, u32) -> Px,
    joints_per_tile: i64,
) -> Px {
    let (course, dy) = cellw(y, exposure, tile);
    let t = dy / exposure;
    let (b, jd) = joints(x, course, joints_per_tile, tile, 41);
    let mut p = board(x, dy, b as u32);
    p.h += profile(t);
    // Each board cups a little across its width.
    p.h += 0.7 * (2.0 * t - 1.0).powi(2);
    // The shadow cast by the course above on this board's top.
    let shade = 1.0 - 0.72 * (-(1.0 - t) * exposure / 10.0).exp();
    p.c = scale3(p.c, shade);
    // Butt joints: a hairline, the end grain darker and rougher beside it.
    if jd < 1.2 {
        p.c = scale3(p.c, 0.35);
        p.h -= 1.0;
    } else if jd < 3.0 {
        p.c = scale3(p.c, 0.8);
        p.r = (p.r + 0.1).min(1.0);
    }
    nail(&mut p, x, dy, course, tile, 22.0);
    p
}

/// A siding nail's head 1" above the butt, every 16" (on the studs), a little off line:
/// stainless, catching the light; painted over on painted boards, weeping a rust tear on
/// weathered ones (`p` tells which by its roughness).
fn nail(p: &mut Px, x: f64, dy: f64, course: i64, tile: f64, above: f64) {
    let step = 16.0 * IN;
    let (k, _) = cell(x, step);
    let n = (tile / step).round() as i64;
    let jitter = (hash(k.rem_euclid(n), course, 211) - 0.5) * 18.0;
    let cx = (k as f64 + 0.5) * step + jitter;
    let cy = above + (hash(k.rem_euclid(n), course, 223) - 0.5) * 5.0;
    let d = ((x - cx).powi(2) + (dy - cy).powi(2)).sqrt();
    if d < 2.4 {
        let dome = (1.0 - (d / 2.4).powi(2)).max(0.0);
        p.h += 0.6 * dome;
        if p.r > 0.58 {
            // Bare wood: a stainless head.
            p.c = scale3(lin([150, 150, 152]), 0.75 + 0.35 * dome);
            p.r = 0.3;
        }
    } else if p.r > 0.85 && (x - cx).abs() < 1.6 && dy < cy && dy > cy - 40.0 {
        // Weathered: a faint tear stain below the head.
        let k = 1.0 - (cy - dy) / 40.0;
        p.c = scale3(p.c, 1.0 - 0.25 * k);
    }
}

fn bevel_profile(t: f64) -> f64 {
    // 3/4" butt tapering to 3/16".
    (1.0 - t) * 14.0
}

fn dutch_profile(t: f64) -> f64 {
    // A flat face, then the cove below the next course.
    if t < 0.62 {
        (1.0 - t) * 6.0 + 6.0
    } else {
        let s = (t - 0.62) / 0.38;
        6.0 + 2.3 - 8.0 * (s * std::f64::consts::PI).sin() * 0.6
    }
}

fn sample(kind: &str, x: f64, y: f64, tile: f64) -> Px {
    if let Some(p) = ground(kind, x, y, tile) {
        return p;
    }
    let paint = [0.62, 0.62, 0.62];
    match kind {
        "cedar-lap-stained" => {
            let e = 6.0 * IN;
            let mut p = lap(
                x,
                y,
                tile,
                e,
                bevel_profile,
                |a, c, s| wood(&STAINED_CEDAR, a, c, s, tile),
                1,
            );
            // The butt of the course above: a crisp dark line, then its soft shadow.
            let (_, dy) = cellw(y, e, tile);
            let from_top = e - dy;
            if from_top < 8.0 {
                p.c = scale3(p.c, 0.3);
                p.h -= 3.0;
            } else if from_top < 30.0 {
                p.c = scale3(p.c, 0.5 + 0.5 * (from_top - 8.0) / 22.0);
            }
            p
        }
        "cedar-bevel" | "cedar-bevel-weathered" => {
            let sp = if kind == "cedar-bevel" {
                CEDAR
            } else {
                WEATHERED
            };
            lap(
                x,
                y,
                tile,
                6.0 * IN,
                bevel_profile,
                |a, c, s| wood(&sp, a, c, s, tile),
                1,
            )
        }
        "shiplap8" => {
            let e = 8.0 * IN;
            let (course, dy) = cellw(y, e, tile);
            let (b, jd) = joints(x, course, 1, tile, 43);
            let mut p = painted(wood(&CEDAR, x, dy, b as u32, tile), paint);
            // The 3/8" gap under each board, in shadow.
            if dy < 9.5 {
                p.h -= 12.0;
                p.c = scale3(p.c, 0.18 + 0.02 * dy);
            } else if dy < 12.0 {
                p.c = scale3(p.c, 0.75);
            }
            if jd < 1.0 {
                p.c = scale3(p.c, 0.5);
            }
            nail(&mut p, x, dy, course, tile, 30.0);
            p
        }
        "dutch-lap6" => lap(
            x,
            y,
            tile,
            6.0 * IN,
            dutch_profile,
            |a, c, s| painted(wood(&CEDAR, a, c, s, tile), paint),
            1,
        ),
        "board-batten12" | "board-batten-cedar" => {
            let w = 12.0 * IN;
            let (b, dx) = cellw(x, w, tile);
            let cedar = kind == "board-batten-cedar";
            // Battens 2-1/2" wide over each joint, standing 3/4" proud.
            let from_joint = dx.min(w - dx);
            let batten_half = 1.25 * IN;
            let (mut p, on_batten) = if from_joint < batten_half {
                let bi = if dx < w / 2.0 {
                    b
                } else {
                    (b + 1).rem_euclid((tile / w).round() as i64)
                };
                (wood(&CEDAR, y, from_joint, 1000 + bi as u32, tile), true)
            } else {
                (wood(&CEDAR, y, dx, b as u32, tile), false)
            };
            if !cedar {
                p = painted(p, paint);
            }
            if on_batten {
                // Eased edges.
                let e = batten_half - from_joint;
                p.h += 19.0 - (2.0 - e.min(2.0)) * 2.0;
            } else {
                // Shade beside the battens.
                let d = from_joint - batten_half;
                p.c = scale3(p.c, 1.0 - 0.45 * (-d / 6.0).exp());
            }
            p
        }
        "cedar-vertical-tg" | "shou-sugi-ban" => {
            let w = 6.0 * IN;
            let (b, dx) = cellw(x, w, tile);
            let mut p = if kind == "shou-sugi-ban" {
                charred(x, y, dx, b, tile)
            } else {
                wood(
                    &Species {
                        knots: 0.15,
                        ..CEDAR
                    },
                    y,
                    dx,
                    b as u32,
                    tile,
                )
            };
            v_groove(&mut p, dx.min(w - dx));
            p
        }
        "thermo-ash" => {
            let e = 6.0 * IN;
            let (course, dy) = cellw(y, e, tile);
            let (b, jd) = joints(x, course, 2, tile, 47);
            let mut p = wood(&THERMO_ASH, x, dy, b as u32, tile);
            v_groove(&mut p, dy.min(e - dy));
            if jd < 1.0 {
                p.c = scale3(p.c, 0.4);
            }
            p
        }
        "cedar-shingles" | "cedar-shingles-weathered" => {
            let sp = if kind == "cedar-shingles" {
                CEDAR
            } else {
                WEATHERED
            };
            shingles(x, y, tile, &sp)
        }
        "channel-rustic" => {
            let e = 8.0 * IN;
            let (course, dy) = cellw(y, e, tile);
            let (b, jd) = joints(x, course, 1, tile, 53);
            let mut p = wood(&STAINED, x, dy, b as u32, tile);
            // The 1/2" channel along each board's bottom.
            if dy < 12.7 {
                p.h -= 10.0;
                p.c = scale3(p.c, 0.35 + 0.03 * dy);
            }
            p.c = scale3(p.c, 1.0 - 0.4 * (-(e - dy) / 5.0).exp());
            if jd < 1.0 {
                p.c = scale3(p.c, 0.4);
            }
            p
        }
        "ipe-rainscreen" => {
            // 5-5/8" boards, 3/8" open joints over a black membrane.
            let e = 6.0 * IN;
            let (course, dy) = cellw(y, e, tile);
            let gap = 0.375 * IN;
            if dy < gap {
                return membrane(x, y, tile);
            }
            let (b, jd) = joints(x, course, 2, tile, 59);
            let mut p = wood(&IPE, x, dy, b as u32, tile);
            let edge = (dy - gap).min(e - dy);
            if edge < 1.5 {
                p.h -= 1.5 - edge;
            }
            p.h += 25.0;
            if jd < 3.0 {
                return membrane(x, y, tile);
            }
            p
        }
        "accoya-slats" => {
            let w = 2.0 * IN;
            let (b, dx) = cellw(x, w, tile);
            let slat = 1.5 * IN;
            if dx > slat {
                return membrane(x, y, tile);
            }
            let mut p = wood(&ACCOYA, y, dx, b as u32, tile);
            let edge = dx.min(slat - dx);
            p.h += 38.0 - (1.5 - edge.min(1.5)) * 2.0;
            p
        }
        "barn-wood" => {
            // Random widths summing to 8'.
            const W: [f64; 16] = [
                4., 6., 8., 6., 4., 8., 6., 4., 6., 8., 4., 6., 8., 6., 4., 8.,
            ];
            let mut at = 0.0;
            let yy = y.rem_euclid(tile);
            let mut row = 0;
            for (i, w) in W.iter().enumerate() {
                if yy < at + w * IN {
                    row = i;
                    break;
                }
                at += w * IN;
            }
            let dy = yy - at;
            let w = W[row] * IN;
            let (b, jd) = joints(x, row as i64, 1, tile, 61);
            let tones = [
                [150, 146, 138],
                [120, 100, 82],
                [96, 90, 84],
                [140, 118, 92],
                [110, 108, 104],
            ];
            let pick = tones[(hash(b, 3, 67) * tones.len() as f64) as usize % tones.len()];
            let sp = Species {
                early: pick,
                late: pick.map(|c| (f64::from(c) * 0.72) as u8),
                ring: 4.0,
                spread: 0.1,
                knots: 0.6,
                rough: 0.9,
                weathered: false,
                palette: &[],
            };
            let mut p = wood(&sp, x, dy, b as u32, tile);
            // Saw marks, worn edges and the odd nail hole.
            let saw = 0.5
                + 0.5
                    * ((x / (tile / (tile / 9.0).round())
                        + fbm(x / tile, y / tile, 7, 7, 2, 71) * 3.0)
                        * std::f64::consts::TAU)
                        .sin();
            p.h += 0.15 * saw;
            let edge = dy.min(w - dy);
            if edge < 4.0 {
                p.h -= (4.0 - edge) * 0.8;
                p.c = scale3(p.c, 0.6 + 0.1 * edge);
            }
            for side in [0.35, 0.65] {
                let nx = (hash(b, 9, 73) * tile + side * tile).rem_euclid(tile);
                let d = ((x - nx).powi(2) + (dy - w * side).powi(2)).sqrt();
                if d < 2.2 {
                    p.c = scale3(p.c, 0.15);
                    p.h -= 1.0;
                }
            }
            if jd < 1.5 {
                p.c = scale3(p.c, 0.3);
            }
            p
        }
        k if k.starts_with("asphalt-") => asphalt(x, y, tile, k),
        "seam16" | "seam18" | "galvalume16" => {
            let w = if kind == "seam18" {
                18.0 * IN
            } else {
                16.0 * IN
            };
            seam(x, y, tile, w, kind == "galvalume16")
        }
        "epdm" => {
            let u = (x / tile, y / tile);
            let wr = fbm(u.0, u.1, 9, 9, 4, 81);
            let mut p = Px {
                c: scale3(lin([30, 30, 32]), 0.9 + 0.2 * wr),
                h: 0.4 * wr,
                r: 0.78,
            };
            // A 10' sheet: its lap seam with seam tape.
            let (_, dy) = cell(y, tile);
            if dy < 76.0 {
                p.c = scale3(p.c, 1.2);
                p.h += 0.8;
                p.r = 0.7;
            }
            p
        }
        "ballast" => ballast(x, y, tile),
        "sedum" => sedum(x, y, tile),
        "flat-concrete-tile" => {
            let e = 13.0 * IN;
            let wdt = tile / 3.0;
            let (course, dy) = cellw(y, e, tile);
            let off = if course.rem_euclid(2) == 1 {
                wdt / 2.0
            } else {
                0.0
            };
            let (ti, dx) = cellw(x + off, wdt, tile);
            let u = (x / tile, y / tile);
            let g = fbm(u.0, u.1, 60, 60, 3, 83);
            let tone = 0.9 + 0.2 * hash(ti, course, 89);
            let mut p = Px {
                c: scale3(paint, tone * (0.92 + 0.12 * g)),
                h: 0.2 * g + (1.0 - dy / e) * 8.0,
                r: 0.8,
            };
            let gap = dx.min(wdt - dx);
            if gap < 2.5 {
                p.c = scale3(p.c, 0.3);
                p.h -= 4.0;
            }
            p.c = scale3(p.c, 1.0 - 0.5 * (-(e - dy) / 8.0).exp());
            p
        }
        _ => Px {
            c: paint,
            h: 0.0,
            r: 0.8,
        },
    }
}

/// A V-groove between boards, `d` from the joint.
fn v_groove(p: &mut Px, d: f64) {
    if d < 4.0 {
        p.h -= (4.0 - d) * 1.2;
        p.c = scale3(p.c, 0.45 + 0.13 * d);
    }
}

/// The black weather membrane seen through open joints.
fn membrane(x: f64, y: f64, tile: f64) -> Px {
    let n = fbm(x / tile, y / tile, 30, 30, 2, 91);
    Px {
        c: scale3(lin([22, 22, 24]), 0.8 + 0.4 * n),
        h: 0.0,
        r: 0.85,
    }
}

/// Charred cedar: black, crazed into scales along the grain, a glint on the ridges.
fn charred(x: f64, y: f64, dx: f64, b: i64, tile: f64) -> Px {
    let base = wood(&CEDAR, y, dx, b as u32, tile);
    // Alligator scales: cells stretched along the grain.
    // Cell sizes that divide the tile, so the crazing repeats with it.
    let (cw, ch) = (tile / (tile / 9.0).round(), tile / (tile / 26.0).round());
    let (gx, gy) = ((x / cw).floor(), (y / ch).floor());
    let mut d1 = f64::MAX;
    let mut d2 = f64::MAX;
    for i in -1..=1 {
        for j in -1..=1 {
            let (cx, cy) = (gx + f64::from(i), gy + f64::from(j));
            let wrapx = (cx as i64).rem_euclid((tile / cw).round() as i64);
            let wrapy = (cy as i64).rem_euclid((tile / ch).round() as i64);
            let px = (cx + hash(wrapx, wrapy, 97)) * cw;
            let py = (cy + hash(wrapx, wrapy, 101)) * ch;
            let d = (((x - px) / cw).powi(2) + ((y - py) / ch).powi(2)).sqrt();
            if d < d1 {
                d2 = d1;
                d1 = d;
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    let crack = smoothstep(0.0, 0.12, d2 - d1);
    let lum = 0.018 + 0.02 * crack + 0.01 * (base.h + 0.1) * 10.0;
    Px {
        c: [lum, lum * 0.95, lum * 0.9],
        h: crack * 1.2 + base.h * 2.0,
        r: 0.55 + 0.35 * (1.0 - crack),
    }
}

/// Cedar shingles: 6" courses of random widths, keyways between, shadows under the butts.
fn shingles(x: f64, y: f64, tile: f64, sp: &Species) -> Px {
    let e = 6.0 * IN;
    let (course, dy) = cellw(y, e, tile);
    // Widths 3"–10" along the course, chosen so the course repeats across the tile.
    let n = 18;
    let mut edges = vec![0.0];
    let mut total = 0.0;
    for i in 0..n {
        total += 3.0 + hash(course, i, 103) * 7.0;
        edges.push(total);
    }
    let k = tile / total;
    let off = hash(course, 1, 107) * tile;
    let xx = (x + off).rem_euclid(tile) / k;
    let i = edges.iter().position(|e| *e > xx).unwrap_or(1).max(1) - 1;
    let (a, bb) = (edges[i], edges[i + 1]);
    let key = (xx - a).min(bb - xx) * k;
    let seed = (course * 31 + i as i64) as u32;
    let mut p = wood(sp, y, x, seed, tile);
    // Sawn face, a little thicker at the butt.
    p.h += (1.0 - dy / e) * 9.0;
    p.c = scale3(p.c, 1.0 - 0.6 * (-(e - dy) / 10.0).exp());
    if key < 3.0 + hash(seed as i64, 5, 109) * 3.0 {
        p.c = scale3(p.c, 0.12);
        p.h -= 8.0;
    }
    p
}

/// Architectural (laminated) asphalt shingles: 5-5/8" courses; each course's lower layer
/// shows where the random-width tabs of its top layer leave gaps ("dragon teeth"), the
/// shadow line under each course, and multicoloured granules.
fn asphalt(x: f64, y: f64, tile: f64, kind: &str) -> Px {
    let e = 5.625 * IN;
    let (course, dy) = cellw(y, e, tile);
    let t = dy / e;
    let palette: &[[u8; 3]] = match kind {
        "asphalt-black" => &[[26, 26, 28], [40, 40, 42], [18, 18, 20], [60, 60, 62]],
        "asphalt-weathered-wood" => &[
            [98, 86, 72],
            [70, 62, 54],
            [124, 110, 92],
            [52, 48, 44],
            [140, 124, 100],
        ],
        "asphalt-pewter" => &[[112, 112, 110], [86, 88, 88], [134, 132, 128], [64, 66, 68]],
        _ => &[
            [66, 68, 72],
            [46, 48, 52],
            [92, 94, 98],
            [34, 36, 38],
            [120, 120, 122],
        ],
    };
    // Tabs: widths 4"–13" across the course.
    let mut edges = vec![0.0];
    let mut total = 0.0;
    for i in 0..9 {
        total += 4.0 + hash(course, i, 113) * 9.0;
        edges.push(total);
    }
    let k = tile / total;
    let off = hash(course, 2, 127) * tile;
    let xx = (x + off).rem_euclid(tile) / k;
    let i = edges.iter().position(|e| *e > xx).unwrap_or(1).max(1) - 1;
    let tab = (course * 13 + i as i64) as u32;
    // Each tab stops short of the butt by its own amount (the dragon teeth's cut-outs).
    let tab_bottom = 0.08 + hash(tab as i64, 3, 131) * 0.28;
    let on_tab = t > tab_bottom;
    // Granules: ~1 mm grains of the palette.
    let g = tile / (tile / 1.1).round();
    let (gi, gj) = ((x / g).floor() as i64, (y / g).floor() as i64);
    let n = (tile / g).round() as i64;
    let pick = hash(gi.rem_euclid(n), gj.rem_euclid(n), 137);
    let mut col = lin(palette[(pick * palette.len() as f64) as usize % palette.len()]);
    // The tab's own blend (shingles are made in shades).
    let blend = 0.82 + 0.3 * hash(tab as i64, 5, 139);
    col = scale3(col, blend);
    let grain_h = hash(gi.rem_euclid(n), gj.rem_euclid(n), 149) * 0.35;
    let mut p = Px {
        c: col,
        h: grain_h + (1.0 - t) * 3.0,
        r: 0.92,
    };
    if on_tab {
        p.h += 3.0;
        // A thin shadow along the tab's cut edge below it.
    } else {
        // The lower layer, darker (the shadow band).
        p.c = scale3(p.c, 0.55);
    }
    // Shadow line under the course above.
    p.c = scale3(p.c, 1.0 - 0.6 * (-(1.0 - t) * e / 5.0).exp());
    // Tab edges.
    let ex = (xx - edges[i]).min(edges[i + 1] - xx) * k;
    if on_tab && ex < 1.5 {
        p.c = scale3(p.c, 0.5);
        p.h -= 1.0;
    }
    if (t - tab_bottom).abs() * e < 1.5 {
        p.c = scale3(p.c, 0.45);
    }
    p
}

/// Standing seam: panels `w` wide with a 1" seam standing 1-1/2", light striations in the
/// pans, faint oil-canning, and for bare Galvalume its spangle.
fn seam(x: f64, y: f64, tile: f64, w: f64, bare: bool) -> Px {
    let (panel, dx) = cellw(x, w, tile);
    let from = dx.min(w - dx);
    let u = (x / tile, y / tile);
    // Oil canning: a slow ripple in each pan.
    let canning = fbm(u.0, u.1, 12, 3, 2, 151 + panel as u32) * 0.5;
    // Striations every 2" across the pan.
    let stri = 0.5 + 0.5 * ((dx / (2.0 * IN)) * std::f64::consts::TAU).cos();
    let mut h = canning + 0.25 * smoothstep(0.85, 1.0, stri);
    let seam_half = 0.5 * IN;
    let mut shade = 1.0;
    if from < seam_half {
        // The seam: a rounded rib.
        let s = from / seam_half;
        h = 38.0 * (1.0 - s * s).sqrt().max(0.0) + 2.0;
    } else if from < seam_half + 6.0 {
        // The panel's bend up into the seam.
        let s = (from - seam_half) / 6.0;
        h += (1.0 - s) * 4.0;
        shade = 0.9;
    }
    let mut c = scale3([0.62; 3], shade);
    let mut r = 0.5;
    if bare {
        // Galvalume spangle: crystals a few mm across, each a slightly different sheen.
        let cs = tile / (tile / 6.0).round();
        let (gi, gj) = ((x / cs).floor() as i64, (y / cs).floor() as i64);
        let n = (tile / cs).round() as i64;
        let sp = hash(gi.rem_euclid(n), gj.rem_euclid(n), 157);
        c = scale3(lin([184, 188, 190]), 0.88 + 0.2 * sp);
        r = 0.28 + 0.14 * sp;
    }
    Px { c, h, r }
}

/// River rock ballast: rounded stones 3/4"–1-1/2".
fn ballast(x: f64, y: f64, tile: f64) -> Px {
    let cs = tile / (tile / 28.0).round();
    let n = (tile / cs).round() as i64;
    let (gx, gy) = ((x / cs).floor() as i64, (y / cs).floor() as i64);
    let mut best = (f64::MAX, 0i64, 0i64, 0.0);
    for i in -1..=1 {
        for j in -1..=1 {
            let (cx, cy) = (gx + i, gy + j);
            let (wx, wy) = (cx.rem_euclid(n), cy.rem_euclid(n));
            let px = (cx as f64 + 0.2 + 0.6 * hash(wx, wy, 163)) * cs;
            let py = (cy as f64 + 0.2 + 0.6 * hash(wx, wy, 167)) * cs;
            let rad = cs * (0.35 + 0.25 * hash(wx, wy, 173));
            let d = ((x - px).powi(2) + (y - py).powi(2)).sqrt() / rad;
            if d < best.0 {
                best = (d, wx, wy, rad);
            }
        }
    }
    let (d, wx, wy, rad) = best;
    let stones = [
        [150, 142, 130],
        [110, 104, 98],
        [176, 168, 154],
        [92, 86, 80],
        [130, 120, 104],
    ];
    let pick = stones[(hash(wx, wy, 179) * stones.len() as f64) as usize % stones.len()];
    if d < 1.0 {
        let dome = (1.0 - d * d).sqrt();
        let speck = vnoise(
            x / tile,
            y / tile,
            (tile / 1.5) as i64,
            (tile / 1.5) as i64,
            181,
        );
        Px {
            c: scale3(lin(pick), (0.75 + 0.3 * dome) * (0.9 + 0.2 * speck)),
            h: dome * rad * 0.6,
            r: 0.7 + 0.15 * speck,
        }
    } else {
        Px {
            c: scale3(lin([60, 56, 50]), 0.6),
            h: 0.0,
            r: 0.9,
        }
    }
}

/// Sedum: clumps of greens and reds over the growing medium.
fn sedum(x: f64, y: f64, tile: f64) -> Px {
    let u = (x / tile, y / tile);
    let cover = fbm(u.0, u.1, 24, 24, 4, 191);
    let hue = fbm(u.0, u.1, 10, 10, 3, 193);
    // Rosettes: little leaf bumps.
    let leaf = vnoise(u.0, u.1, (tile / 5.0) as i64, (tile / 5.0) as i64, 197);
    if cover < 0.36 {
        let soil = fbm(u.0, u.1, 200, 200, 2, 199);
        return Px {
            c: scale3(lin([88, 70, 52]), 0.7 + 0.3 * soil),
            h: soil * 2.0,
            r: 0.95,
        };
    }
    let greens = lerp3(lin([96, 128, 60]), lin([150, 162, 84]), hue);
    let c = lerp3(greens, lin([150, 70, 60]), smoothstep(0.62, 0.8, hue) * 0.6);
    Px {
        c: scale3(c, 0.7 + 0.45 * leaf),
        h: 8.0 * smoothstep(0.36, 0.6, cover) + 3.0 * leaf,
        r: 0.75,
    }
}

// ---------------------------------------------------------------- ground covers (ADR-064)

/// Whole cells of about `mm` across the tile.
fn cells_of(tile: f64, mm: f64) -> i64 {
    (tile / mm).round().max(1.0) as i64
}

/// Tile-periodic fractal noise, 0–1, cells of about `mm`.
fn gnoise(x: f64, y: f64, tile: f64, mm: f64, octaves: u32, seed: u32) -> f64 {
    let n = cells_of(tile, mm);
    fbm(x / tile, y / tile, n, n, octaves, seed)
}

/// One value per cell of about `mm`, unblended: grains, specks and granules.
fn speck(x: f64, y: f64, tile: f64, mm: f64, seed: u32) -> f64 {
    let n = cells_of(tile, mm);
    let g = tile / n as f64;
    hash(
        ((x / g).floor() as i64).rem_euclid(n),
        ((y / g).floor() as i64).rem_euclid(n),
        seed,
    )
}

/// Value noise in a feature's own frame, 0–1: it moves with the feature, so needs no wrap.
fn lnoise(u: f64, v: f64, seed: u32) -> f64 {
    let (gx, gy) = (u.floor(), v.floor());
    let (fx, fy) = (smooth(u - gx), smooth(v - gy));
    let (i, j) = (gx as i64, gy as i64);
    let top = hash(i, j, seed) * (1.0 - fx) + hash(i + 1, j, seed) * fx;
    let bot = hash(i, j + 1, seed) * (1.0 - fx) + hash(i + 1, j + 1, seed) * fx;
    top * (1.0 - fy) + bot * fy
}

/// A feature's direction (unit vector) from its hash.
fn dir_of(id: i64, k: i64, seed: u32) -> (f64, f64) {
    let (s, c) = (hash(id, k, seed) * std::f64::consts::TAU).sin_cos();
    (c, s)
}

/// One of `palette`, picked by `k` (0–1).
fn pick(palette: &[[u8; 3]], k: f64) -> [f64; 3] {
    lin(palette[(k * palette.len() as f64) as usize % palette.len()])
}

/// Features scattered over the tile: `per` in each of `n` × `n` cells, at hashed spots.
struct Grid {
    n: i64,
    cs: f64,
    per: i64,
    reach: i64,
    /// How far (0–1 of a cell) features stray from their cell's centre.
    jitter: f64,
}

impl Grid {
    /// Cells of about `cell` mm; features reach up to `extent` mm from their spot.
    fn new(tile: f64, cell: f64, per: i64, extent: f64) -> Grid {
        let n = cells_of(tile, cell);
        let cs = tile / n as f64;
        Grid {
            n,
            cs,
            per,
            reach: (extent / cs).ceil().max(1.0) as i64,
            jitter: 1.0,
        }
    }

    /// Calls `f` with the point's offset (mm) from each feature that may reach it, and the
    /// feature's id (the same a tile over).
    fn each(&self, x: f64, y: f64, seed: u32, mut f: impl FnMut(f64, f64, i64)) {
        let (gx, gy) = ((x / self.cs).floor() as i64, (y / self.cs).floor() as i64);
        for j in -self.reach..=self.reach {
            for i in -self.reach..=self.reach {
                let (cx, cy) = (gx + i, gy + j);
                let base = (cy.rem_euclid(self.n) * self.n + cx.rem_euclid(self.n)) * self.per;
                for k in 0..self.per {
                    let id = base + k;
                    let fx = (cx as f64 + 0.5 + (hash(id, 1, seed) - 0.5) * self.jitter) * self.cs;
                    let fy = (cy as f64 + 0.5 + (hash(id, 2, seed) - 0.5) * self.jitter) * self.cs;
                    f(x - fx, y - fy, id);
                }
            }
        }
    }
}

// ------------------------------------------------ grass

/// A grass sward seen from above.
struct Turf {
    greens: &'static [[u8; 3]],
    straw: [u8; 3],
    thatch: [u8; 3],
    /// Straw (dead) blades' share.
    dead: f64,
    /// Blade length seen from above and width (mm), least and most.
    len: (f64, f64),
    width: (f64, f64),
    /// 0 blunt (mown) tips … 1 pointed.
    point: f64,
    /// `per` blades in each cell of about `cell` mm.
    cell: f64,
    per: i64,
    /// Depth of the sward (mm of relief for the normals).
    depth: f64,
    /// Share of the ground in straw patches (summer-dry lawns).
    patches: f64,
    /// Clover and wildflowers among the grass.
    flowers: bool,
}

const LAWN: Turf = Turf {
    greens: &[
        [74, 104, 34],
        [62, 94, 30],
        [88, 116, 40],
        [56, 84, 28],
        [96, 122, 46],
        [70, 98, 42],
    ],
    straw: [170, 152, 100],
    thatch: [58, 52, 32],
    dead: 0.025,
    len: (4.5, 10.0),
    width: (1.5, 2.5),
    point: 0.15,
    cell: 3.4,
    per: 2,
    depth: 6.0,
    patches: 0.0,
    flowers: false,
};

const LAWN_LUSH: Turf = Turf {
    greens: &[
        [46, 88, 30],
        [38, 78, 26],
        [54, 98, 36],
        [42, 84, 34],
        [60, 104, 38],
        [50, 92, 40],
    ],
    dead: 0.012,
    per: 3,
    ..LAWN
};

const LAWN_DRY: Turf = Turf {
    greens: &[
        [100, 114, 48],
        [118, 122, 58],
        [88, 102, 42],
        [132, 128, 70],
        [110, 112, 56],
    ],
    straw: [188, 166, 112],
    thatch: [86, 72, 46],
    dead: 0.22,
    len: (5.0, 11.0),
    patches: 0.38,
    ..LAWN
};

const MEADOW: Turf = Turf {
    greens: &[
        [78, 104, 40],
        [96, 116, 50],
        [64, 90, 36],
        [110, 122, 60],
        [120, 118, 66],
        [86, 108, 52],
    ],
    straw: [176, 158, 108],
    thatch: [54, 50, 30],
    dead: 0.12,
    len: (10.0, 28.0),
    width: (2.0, 3.6),
    point: 0.9,
    cell: 9.5,
    per: 4,
    depth: 12.0,
    patches: 0.0,
    flowers: true,
};

fn turf(x: f64, y: f64, tile: f64, t: &Turf, seed: u32) -> Px {
    // Slow drift in colour (feeding, shade, wear), and straw patches on a dry lawn.
    let big = gnoise(x, y, tile, 420.0, 3, seed ^ 3);
    let mid = gnoise(x, y, tile, 70.0, 2, seed ^ 5);
    let dry = if t.patches > 0.0 {
        let k = gnoise(x, y, tile, 300.0, 5, seed ^ 7);
        smoothstep(0.58 - t.patches * 0.4, 0.78 - t.patches * 0.4, k)
    } else {
        0.0
    };
    // The sward's grain: blades lie a little one way, patch by patch.
    let (gs, gc) =
        (gnoise(x, y, tile, 300.0, 2, seed ^ 11) * std::f64::consts::TAU * 2.0).sin_cos();
    let grid = Grid::new(tile, t.cell, t.per, t.len.1);
    // Highest blade here: (height, id, along 0–1, across −1–1).
    let mut best = (f64::MIN, 0i64, 0.0, 0.0);
    grid.each(x, y, seed, |dx, dy, id| {
        let l = t.len.0 + (t.len.1 - t.len.0) * hash(id, 5, seed);
        if dx * dx + dy * dy > (l + 2.0) * (l + 2.0) {
            return;
        }
        let (rx, ry) = dir_of(id, 6, seed);
        let (ux, uy) = (rx + 0.45 * gc, ry + 0.45 * gs);
        let k = (ux * ux + uy * uy).sqrt().max(1e-6);
        let (ux, uy) = (ux / k, uy / k);
        let a = dx * ux + dy * uy;
        if a < 0.0 || a > l {
            return;
        }
        let tt = a / l;
        // Blades arch over as they lean.
        let bend = (hash(id, 7, seed) - 0.5) * 0.35;
        let c = -dx * uy + dy * ux - bend * a * tt;
        let w = t.width.0 + (t.width.1 - t.width.0) * hash(id, 8, seed);
        let hw = 0.5 * w * (1.0 - t.point * tt.powf(1.6)) * (1.0 - 0.25 * tt);
        if c.abs() >= hw {
            return;
        }
        let h = t.depth * (hash(id, 9, seed) + 0.45 * tt);
        if h > best.0 {
            best = (h, id, tt, c / hw);
        }
    });
    let mut p = if best.0 == f64::MIN {
        // Down in the sward: thatch and shadow.
        let n = gnoise(x, y, tile, 6.0, 2, seed ^ 13);
        Px {
            c: scale3(lin(t.thatch), 0.35 + 0.4 * n),
            h: -0.3 * t.depth,
            r: 0.95,
        }
    } else {
        let (h, id, tt, s) = best;
        let dead = hash(id, 10, seed) < t.dead + 0.6 * dry;
        let mut c = pick(t.greens, hash(id, 11, seed));
        c = scale3(c, 0.84 + 0.32 * big);
        // Some blades yellower, some bluer.
        c = lerp3(c, lin([132, 136, 58]), 0.28 * mid * mid);
        if dead {
            let s = lin(t.straw);
            c = scale3(lerp3(s, c, 0.2), 0.5 + 0.45 * hash(id, 12, seed));
        } else if t.point < 0.5 && tt > 0.9 {
            // Mown tips, cut and drying.
            c = lerp3(c, lin(t.straw), 0.3);
        }
        // The keel catches light; the edges roll away.
        c = scale3(c, 0.8 + 0.26 * (1.0 - s * s));
        // Blades deep in the sward get little light.
        let ao = 0.22 + 0.78 * smoothstep(0.0, 1.3 * t.depth, h).powf(0.75);
        c = scale3(c, ao);
        Px {
            c,
            h: h + 0.4 * (1.0 - s * s),
            r: if dead { 0.9 } else { 0.62 + 0.25 * (1.0 - ao) },
        }
    };
    if t.flowers {
        meadow_flowers(x, y, tile, seed, &mut p, t.depth);
    }
    p
}

/// White clover and wildflowers in a meadow, over or among the grass.
fn meadow_flowers(x: f64, y: f64, tile: f64, seed: u32, p: &mut Px, depth: f64) {
    // Clover grows in drifts.
    let drift = gnoise(x, y, tile, 350.0, 3, seed ^ 21);
    if drift > 0.52 {
        let grid = Grid::new(tile, 26.0, 1, 18.0);
        grid.each(x, y, seed ^ 23, |dx, dy, id| {
            if hash(id, 3, seed) > (drift - 0.52) * 4.0 {
                return;
            }
            let (ux, uy) = dir_of(id, 4, seed);
            // Three leaflets round the stalk.
            for k in 0..3 {
                let (s, c) = (f64::from(k) * 2.094 + 0.3).sin_cos();
                let (lx, ly) = (c * ux - s * uy, s * ux + c * uy);
                let (qx, qy) = (dx - lx * 6.5, dy - ly * 6.5);
                let d = (qx * qx + qy * qy).sqrt() / 6.0;
                if d < 1.0 {
                    let h = depth * (0.7 + 0.5 * hash(id, 5, seed)) + 1.5 * (1.0 - d * d);
                    if h > p.h {
                        // The pale chevron across each leaflet.
                        let along = (qx * lx + qy * ly) / 6.0;
                        let chevron = if (along + 0.25).abs() < 0.12 && d > 0.3 {
                            1.0
                        } else {
                            0.0
                        };
                        let base = lerp3(lin([54, 96, 40]), lin([150, 176, 130]), 0.5 * chevron);
                        p.c = scale3(base, 0.6 + 0.4 * (1.0 - d * d));
                        p.h = h;
                        p.r = 0.55;
                    }
                }
            }
        });
    }
    let grid = Grid::new(tile, 60.0, 1, 8.0);
    grid.each(x, y, seed ^ 29, |dx, dy, id| {
        if hash(id, 3, seed) > 0.4 {
            return;
        }
        let r = 3.0 + 3.5 * hash(id, 4, seed);
        let d2 = dx * dx + dy * dy;
        if d2 > r * r {
            return;
        }
        let d = d2.sqrt() / r;
        let kind = hash(id, 5, seed);
        // Five petals.
        let ang = dy.atan2(dx) + hash(id, 6, seed) * 6.0;
        let petal = 0.72 + 0.28 * (5.0 * ang).cos();
        if d > petal {
            return;
        }
        let (petals, eye) = if kind < 0.45 {
            ([240, 238, 226], [222, 176, 40])
        } else if kind < 0.75 {
            ([236, 196, 46], [200, 150, 30])
        } else {
            ([186, 120, 170], [150, 90, 140])
        };
        p.c = if d < 0.3 {
            lin(eye)
        } else {
            scale3(lin(petals), 0.8 + 0.2 * (1.0 - d))
        };
        p.h = depth * 1.6 + 1.0 * (1.0 - d);
        p.r = 0.6;
    });
}

// ------------------------------------------------ litter and mulch

/// Long-leaf pine straw: needles matted in layers, clumps lying their own ways, and the
/// odd fallen cone.
fn pine_straw(x: f64, y: f64, tile: f64) -> Px {
    const LAYERS: u32 = 12;
    let needles: &[[u8; 3]] = &[
        [138, 86, 50],
        [118, 74, 44],
        [156, 102, 62],
        [100, 64, 40],
        [130, 90, 58],
        [116, 94, 76],
        [146, 94, 56],
    ];
    // Best: (height, needle id, across −1–1, along −1–1, layer).
    let mut best = (f64::MIN, 0i64, 0.0, 0.0, 0u32);
    let grid = Grid::new(tile, 70.0, 1, 62.0);
    for layer in 0..LAYERS {
        let ls = 401 + layer * 31;
        grid.each(x, y, ls, |dx, dy, id| {
            let rad = 62.0;
            if dx * dx + dy * dy > rad * rad {
                return;
            }
            let (ux, uy) = dir_of(id, 3, ls);
            let a = dx * ux + dy * uy;
            // Needles curve gently.
            let curve = (hash(id, 4, ls) - 0.5) * 0.003;
            let c = -dx * uy + dy * ux - curve * a * a;
            let sp = 5.5;
            let k = (c / sp).floor();
            let nid = id * 97 + k as i64;
            if hash(nid, 5, ls) > 0.5 {
                return;
            }
            let rn = 0.6 + 0.25 * hash(nid, 6, ls);
            let centre = (k + 0.5) * sp + (hash(nid, 7, ls) - 0.5) * (sp - 2.0 * rn);
            let d = (c - centre) / rn;
            if d.abs() >= 1.0 {
                return;
            }
            let mid = (hash(nid, 8, ls) - 0.5) * 70.0;
            let half = 30.0 + 45.0 * hash(nid, 9, ls);
            let along = (a - mid) / half;
            if along.abs() >= 1.0 {
                return;
            }
            let h = f64::from(layer) * 1.1 + 0.7 * (1.0 - d * d).sqrt() + 0.3 * hash(nid, 10, ls);
            if h > best.0 {
                best = (h, nid, d, along, layer);
            }
        });
    }
    let mut p = if best.0 == f64::MIN {
        let n = gnoise(x, y, tile, 5.0, 2, 419);
        Px {
            c: scale3(lin([46, 32, 22]), 0.3 + 0.3 * n),
            h: -1.0,
            r: 0.95,
        }
    } else {
        let (h, nid, d, along, layer) = best;
        let mut c = pick(needles, hash(nid, 11, 421));
        c = scale3(c, 0.8 + 0.35 * hash(nid, 12, 421));
        // Each needle's tone drifts along it; the sheath end is dark.
        c = scale3(c, 0.9 + 0.14 * along * (hash(nid, 13, 421) - 0.5) * 2.0);
        if along < -0.95 {
            c = scale3(c, 0.5);
        }
        // Round needles: lit across the top, darker at the sides.
        c = scale3(c, 0.6 + 0.48 * (1.0 - d * d).sqrt());
        let ao = 0.14 + 0.86 * (f64::from(layer + 1) / f64::from(LAYERS)).powf(1.2);
        Px {
            c: scale3(c, ao),
            h,
            r: 0.5 + 0.3 * d * d,
        }
    };
    // Fallen cones, a few to the tile.
    let grid = Grid::new(tile, 480.0, 1, 70.0);
    grid.each(x, y, 431, |dx, dy, id| {
        if hash(id, 3, 431) > 0.3 {
            return;
        }
        let len = 50.0 + 20.0 * hash(id, 4, 431);
        if dx * dx + dy * dy > len * len {
            return;
        }
        let (ux, uy) = dir_of(id, 5, 431);
        let (a, c) = (dx * ux + dy * uy, -dx * uy + dy * ux);
        // Wider toward the open end.
        let wid = len * 0.62 * (0.8 + 0.2 * (a / len));
        let d = ((a / len).powi(2) + (c / wid).powi(2)).sqrt();
        if d >= 1.0 {
            return;
        }
        // Scales in two spirals: diamond-shaped plates, each with its raised boss, dark
        // clefts between.
        let (sp, sq) = ((a + c) / 12.0, (a - c) / 12.0);
        let (fp, fq) = (sp - sp.floor(), sq - sq.floor());
        let plate = 1.0 - 2.0 * (fp - 0.5).abs().max((fq - 0.5).abs());
        let dome = (1.0 - d * d).sqrt();
        let boss = smoothstep(0.55, 0.8, plate);
        let mut col = lerp3(
            lin([88, 54, 30]),
            lin([150, 108, 68]),
            smoothstep(0.1, 0.6, plate),
        );
        col = lerp3(col, lin([120, 104, 88]), boss * 0.6);
        let cleft = smoothstep(0.0, 0.12, plate);
        // Each scale lies over the one behind it: lit at its open lip, shadowed where the
        // next tucks under.
        let lip = ((fp + fq) / 2.0).powf(1.5);
        let own = 0.75 + 0.4 * hash(sp.floor() as i64, sq.floor() as i64, 433);
        p.c = scale3(
            col,
            own * (0.15 + 0.85 * cleft) * (0.35 + 0.65 * lip) * (0.2 + 0.8 * dome),
        );
        p.h = 14.0 + 26.0 * dome + 3.5 * plate + 3.0 * lip;
        p.r = 0.75;
    });
    p
}

/// Autumn leaf litter: oak, maple and willow-shaped leaves in drifts.
fn leaf_litter(x: f64, y: f64, tile: f64) -> Px {
    let colours: &[[u8; 3]] = &[
        [118, 80, 46],
        [140, 98, 58],
        [96, 66, 42],
        [160, 118, 72],
        [176, 140, 90],
        [156, 88, 42],
        [124, 62, 38],
        [184, 144, 72],
        [84, 58, 38],
        [132, 90, 50],
    ];
    let seed = 511;
    let best = top_leaf(x, y, tile, seed);
    if best.0 == f64::MIN {
        let n = gnoise(x, y, tile, 4.0, 2, seed ^ 3);
        return Px {
            c: scale3(lin([46, 34, 24]), 0.3 + 0.3 * n),
            h: -1.0,
            r: 0.95,
        };
    }
    // Leaves nearer the light (up and to the left) shade the ones beneath.
    let shade = [1.5, 3.5]
        .iter()
        .map(|o| {
            if top_leaf(x - o, y + o, tile, seed).0 > best.0 {
                0.72
            } else {
                1.0
            }
        })
        .product::<f64>();
    let (h0, id, t, s) = best;
    // Dry leaves curl up at their edges, and arch along the midrib.
    let curl = 2.5 * hash(id, 7, seed);
    let h = h0 + curl * s * s + 1.2 * (std::f64::consts::PI * t).sin();
    let mut c = pick(colours, hash(id, 8, seed));
    c = scale3(c, 0.85 + 0.25 * hash(id, 9, seed));
    // Spots of decay, and fading.
    let off = hash(id, 10, seed) * 50.0;
    let blot = lnoise(t * 16.0 + off, s * 6.0, seed);
    c = lerp3(
        c,
        scale3(lin([82, 58, 38]), 0.9),
        smoothstep(0.74, 0.84, blot) * 0.35,
    );
    let fleck = lnoise(t * 30.0 + off, s * 8.0, seed ^ 1);
    c = scale3(c, 0.92 + 0.14 * fleck);
    // Midrib and side veins angling out toward the tip; the rim a touch darker.
    let vein = (t * 8.0 - s.abs() * 1.2).rem_euclid(1.0);
    if s.abs() < 0.05 {
        c = scale3(c, 1.12);
    } else if vein < 0.05 || s.abs() > 0.96 {
        c = scale3(c, 0.88);
    }
    let ao = 0.25 + 0.75 * smoothstep(0.0, 11.0, h).powf(0.9);
    Px {
        c: scale3(c, ao * shade * (0.85 + 0.15 * s * s)),
        h: h - if s.abs() < 0.05 { 0.3 } else { 0.0 },
        r: 0.72 + 0.1 * blot,
    }
}

/// The topmost leaf at a point: (its order, id, along 0–1, across −1–1).
fn top_leaf(x: f64, y: f64, tile: f64, seed: u32) -> (f64, i64, f64, f64) {
    let grid = Grid::new(tile, 34.0, 2, 66.0);
    let mut best = (f64::MIN, 0i64, 0.0, 0.0);
    grid.each(x, y, seed, |dx, dy, id| {
        let size = 60.0 + 60.0 * hash(id, 3, seed);
        let hl = size / 2.0;
        if dx * dx + dy * dy > (hl + 10.0) * (hl + 10.0) {
            return;
        }
        let (ux, uy) = dir_of(id, 4, seed);
        let a = dx * ux + dy * uy;
        let c = -dx * uy + dy * ux;
        let shape = hash(id, 5, seed);
        let h0 = 8.0 * hash(id, 6, seed);
        // The stalk.
        if a < -hl && a > -hl - 10.0 && c.abs() < 0.9 {
            if h0 > best.0 {
                best = (h0, id, 0.5, 0.0);
            }
            return;
        }
        let t = (a / hl + 1.0) / 2.0;
        if !(0.0..1.0).contains(&t) {
            return;
        }
        let pi = std::f64::consts::PI;
        let (maxw, prof) = if shape < 0.4 {
            // Oak: rounded lobes.
            (0.34, 0.76 + 0.24 * (t * 4.5 * pi).sin().powi(2))
        } else if shape < 0.75 {
            // Maple-like: broad, three big lobes.
            (0.46, 0.62 + 0.38 * (t * 2.5 * pi).sin().powi(2))
        } else {
            // Beech and birch: plain ovals, pointed.
            (0.26, 1.0)
        };
        let w = size * maxw * (pi * t.powf(0.8)).sin().max(0.0).powf(0.75) * prof;
        let s = c / w.max(1e-6);
        if s.abs() >= 1.0 {
            return;
        }
        // Leaves lie one on another, in order.
        if h0 > best.0 {
            best = (h0, id, t, s);
        }
    });
    best
}

/// Shredded bark mulch in `palette`: frayed strands and chips lying every which way over
/// fines.
fn mulch(x: f64, y: f64, tile: f64, palette: &[[u8; 3]], seed: u32) -> Px {
    let grid = Grid::new(tile, 10.5, 2, 38.0);
    // Best: (height, id, along, across −1–1).
    let mut best = (f64::MIN, 0i64, 0.0, 0.0);
    grid.each(x, y, seed, |dx, dy, id| {
        let hl = 8.0 + 27.0 * hash(id, 3, seed).powf(1.3);
        let w = 3.0 + 8.0 * hash(id, 4, seed).powi(2);
        if dx * dx + dy * dy > (hl + w) * (hl + w) {
            return;
        }
        let (ux, uy) = dir_of(id, 5, seed);
        let a = dx * ux + dy * uy;
        if a.abs() >= hl {
            return;
        }
        let bend = (hash(id, 6, seed) - 0.5) * 0.02;
        let c = -dx * uy + dy * ux - bend * a * a;
        let off = hash(id, 7, seed) * 40.0;
        let half = 0.5 * w * (0.75 + 0.35 * lnoise(a / 5.0, off, seed));
        let s = c / half;
        if s.abs() >= 1.0 {
            return;
        }
        // Frayed: toward the edges and ends only separate fibres remain.
        let fray = (s.abs() - 0.5).max(0.0) * 2.0 + ((a.abs() / hl) - 0.6).max(0.0) * 2.5;
        if fray > 0.0 {
            let strand = lnoise(a / 12.0 + off, (s + 1.0) * 7.0, seed ^ 3);
            if strand < 0.35 + 0.5 * fray {
                return;
            }
        }
        let h = 7.0 * hash(id, 8, seed);
        if h > best.0 {
            best = (h, id, a, s);
        }
    });
    if best.0 == f64::MIN {
        // Fines and crumbs down between the strands.
        let n = gnoise(x, y, tile, 6.0, 2, seed ^ 5);
        let m = gnoise(x, y, tile, 1.8, 3, seed ^ 6);
        return Px {
            c: scale3(pick(palette, n), 0.12 + 0.25 * m),
            h: -1.5 + m,
            r: 0.95,
        };
    }
    let (h, id, a, s) = best;
    let mut c = pick(palette, hash(id, 9, seed));
    c = scale3(c, 0.78 + 0.4 * hash(id, 10, seed));
    // Fibres along the strand, coarse and fine.
    let off = hash(id, 11, seed) * 30.0;
    let fibre = lnoise(a / 10.0 + off, (s + 1.0) * 5.0, seed ^ 7);
    let fine = lnoise(a / 3.0 + off, (s + 1.0) * 14.0, seed ^ 9);
    c = scale3(c, 0.55 + 0.45 * fibre + 0.3 * fine);
    // Some strands show paler wood beside their bark.
    if hash(id, 12, seed) < 0.25 && s > 0.1 {
        c = scale3(c, 1.35);
    }
    let ao = 0.25 + 0.75 * smoothstep(0.0, 7.0, h);
    Px {
        c: scale3(c, ao * (0.75 + 0.25 * (1.0 - s * s))),
        h: h + 1.2 * (1.0 - s * s).sqrt() + 0.5 * fibre,
        r: 0.82 + 0.1 * fine,
    }
}

// ------------------------------------------------ stone

/// Stones scattered over the tile, one course of a bed.
struct Stones {
    /// `per` stones in each cell of about `cell` mm, `jitter` (0–1) off the cell's centre.
    cell: f64,
    per: i64,
    jitter: f64,
    /// Radius (mm), least and most.
    r: (f64, f64),
    /// How elongated they may be (0 round).
    elong: f64,
    /// Relief: height over radius.
    flat: f64,
    /// Angular (crushed) or rounded.
    angular: bool,
    /// How far they may sit down in the bed, over radius.
    sink: f64,
    /// The course's level (mm): lower courses show between the stones above.
    lift: f64,
}

const STONES: Stones = Stones {
    cell: 10.0,
    per: 1,
    jitter: 1.0,
    r: (3.0, 5.0),
    elong: 0.3,
    flat: 0.7,
    angular: false,
    sink: 0.2,
    lift: 0.0,
};

/// The highest stone at a point.
#[derive(Clone, Copy)]
struct Stone {
    h: f64,
    id: i64,
    /// 1 at the crown, 0 at the edge.
    top: f64,
    /// The point in the stone's frame (mm).
    a: f64,
    c: f64,
    /// Which course (0 the top one).
    course: usize,
    /// How far it stands above the next stone here (mm): small where stones touch.
    clear: f64,
    /// An angular stone's facet here (FACETS.len() on its crown).
    facet: usize,
}

/// Facet directions of an angular stone (unit vectors, irregular angles).
const FACETS: [(f64, f64); 6] = [
    (1.0, 0.0),
    (0.574, 0.819),
    (-0.423, 0.906),
    (-0.985, 0.174),
    (-0.342, -0.940),
    (0.643, -0.766),
];

/// The highest stone at a point over the courses of a bed.
fn stones(x: f64, y: f64, tile: f64, courses: &[Stones], seed: u32) -> Option<Stone> {
    let mut best: Option<Stone> = None;
    let mut second = f64::MIN;
    for (course, s) in courses.iter().enumerate() {
        let seed = seed.wrapping_add(course as u32 * 7919);
        let mut grid = Grid::new(tile, s.cell, s.per, s.r.1 * (1.0 + s.elong) * 1.3);
        grid.jitter = s.jitter;
        grid.each(x, y, seed, |dx, dy, id| {
            let r = s.r.0 + (s.r.1 - s.r.0) * hash(id, 3, seed).powf(1.4);
            let e = 1.0 + s.elong * hash(id, 4, seed);
            let bound = r * e * 1.3;
            if dx * dx + dy * dy > bound * bound {
                return;
            }
            let (ux, uy) = dir_of(id, 5, seed);
            let (a, c) = (dx * ux + dy * uy, -dx * uy + dy * ux);
            let (na, nc) = (a / (r * e.sqrt()), c * e.sqrt() / r);
            let mut facet = FACETS.len();
            let d = if s.angular {
                let mut d = 0.0f64;
                for (k, (fx, fy)) in FACETS.iter().enumerate() {
                    let apothem = 0.72 + 0.4 * hash(id, 10 + k as i64, seed);
                    let dk = (na * fx + nc * fy) / apothem;
                    if dk > d {
                        d = dk;
                        facet = k;
                    }
                }
                d
            } else {
                let wob = 1.0
                    + 0.14 * (lnoise(na * 1.6 + hash(id, 6, seed) * 40.0, nc * 1.6, seed) - 0.5);
                (na * na + nc * nc).sqrt() / wob
            };
            if d >= 1.0 {
                return;
            }
            let top = if s.angular {
                let t = (1.0 - d) / 0.3;
                if t >= 1.0 {
                    facet = FACETS.len();
                }
                t.min(1.0)
            } else {
                (1.0 - d * d).sqrt()
            };
            // Each stone lies a little tilted.
            let tilt = (na * (hash(id, 8, seed) - 0.5) + nc * (hash(id, 9, seed) - 0.5)) * 0.3;
            let h = s.lift + r * (s.flat * top - s.sink * hash(id, 7, seed) + tilt);
            match best {
                Some(b) if h <= b.h => second = second.max(h),
                _ => {
                    if let Some(b) = best {
                        second = second.max(b.h);
                    }
                    best = Some(Stone {
                        h,
                        id: id * 4 + course as i64,
                        top,
                        a,
                        c,
                        course,
                        clear: 0.0,
                        facet,
                    });
                }
            }
        });
    }
    best.map(|b| Stone {
        clear: b.h - second,
        ..b
    })
}

/// A stone's colour from `palette`: its own shade, mineral grain, and the shadow at its
/// edge and where it meets the next stone.
fn stone_colour(st: &Stone, palette: &[[u8; 3]], grain_mm: f64, seed: u32) -> [f64; 3] {
    let mut c = pick(palette, hash(st.id, 20, seed));
    c = scale3(c, 0.85 + 0.3 * hash(st.id, 21, seed));
    let off = hash(st.id, 22, seed) * 100.0;
    let g = lnoise(st.a / grain_mm + off, st.c / grain_mm, seed);
    let g2 = lnoise(
        st.a / (grain_mm * 4.0) + off,
        st.c / (grain_mm * 4.0),
        seed ^ 1,
    );
    c = scale3(c, (0.82 + 0.28 * g) * (0.86 + 0.28 * g2));
    let contact = 0.45 + 0.55 * smoothstep(0.0, 2.0 * grain_mm, st.clear);
    // Facets of broken stone catch the light differently.
    let facet = if st.facet < FACETS.len() {
        0.7 + 0.45 * hash(st.id, 40 + st.facet as i64, seed)
    } else {
        1.0
    };
    scale3(c, (0.28 + 0.72 * st.top.powf(0.7)) * contact * facet)
}

const PEA_GRAVEL: [Stones; 3] = [
    Stones {
        cell: 9.0,
        jitter: 0.55,
        r: (3.6, 5.2),
        elong: 0.45,
        ..STONES
    },
    Stones {
        cell: 6.0,
        jitter: 0.8,
        r: (2.4, 3.6),
        elong: 0.4,
        lift: -1.5,
        ..STONES
    },
    Stones {
        cell: 4.0,
        jitter: 1.0,
        r: (1.6, 2.4),
        lift: -3.0,
        ..STONES
    },
];
const PEA_TONES: &[[u8; 3]] = &[
    [168, 150, 124],
    [196, 184, 164],
    [140, 138, 134],
    [128, 104, 84],
    [158, 124, 96],
    [214, 208, 196],
    [98, 94, 90],
    [176, 150, 130],
    [150, 136, 116],
];

fn pea_gravel(x: f64, y: f64, tile: f64, seed: u32) -> Px {
    match stones(x, y, tile, &PEA_GRAVEL, seed) {
        Some(st) => Px {
            c: scale3(
                stone_colour(&st, PEA_TONES, 0.7, seed),
                1.0 - 0.2 * st.course as f64,
            ),
            h: st.h,
            r: 0.55 + 0.25 * hash(st.id, 30, seed),
        },
        None => Px {
            c: scale3(lin([52, 46, 40]), 0.3 + 0.2 * speck(x, y, tile, 1.0, seed)),
            h: -4.0,
            r: 0.95,
        },
    }
}

fn crushed_stone(x: f64, y: f64, tile: f64) -> Px {
    let s = [
        Stones {
            cell: 17.0,
            jitter: 0.6,
            r: (8.0, 11.0),
            elong: 0.5,
            flat: 0.5,
            angular: true,
            sink: 0.25,
            ..STONES
        },
        Stones {
            cell: 10.0,
            jitter: 0.9,
            r: (4.5, 6.5),
            elong: 0.5,
            flat: 0.5,
            angular: true,
            lift: -2.5,
            ..STONES
        },
    ];
    let grays: &[[u8; 3]] = &[
        [128, 128, 126],
        [150, 148, 144],
        [110, 110, 110],
        [166, 164, 160],
        [122, 118, 112],
        [96, 96, 98],
        [138, 134, 128],
    ];
    match stones(x, y, tile, &s, 611) {
        Some(st) => Px {
            c: scale3(
                stone_colour(&st, grays, 0.5, 611),
                1.0 - 0.25 * st.course as f64,
            ),
            h: st.h,
            r: 0.75 + 0.15 * hash(st.id, 30, 611),
        },
        None => Px {
            c: scale3(lin([62, 60, 58]), 0.3 + 0.25 * speck(x, y, tile, 1.5, 613)),
            h: -4.0,
            r: 0.95,
        },
    }
}

fn river_rock(x: f64, y: f64, tile: f64) -> Px {
    let s = [
        Stones {
            cell: 84.0,
            jitter: 0.5,
            r: (32.0, 46.0),
            elong: 0.6,
            flat: 0.6,
            sink: 0.15,
            ..STONES
        },
        Stones {
            cell: 46.0,
            jitter: 0.8,
            r: (17.0, 25.0),
            elong: 0.5,
            flat: 0.6,
            lift: -8.0,
            ..STONES
        },
    ];
    let tones: &[[u8; 3]] = &[
        [150, 140, 124],
        [122, 116, 108],
        [172, 162, 144],
        [100, 94, 88],
        [140, 120, 98],
        [194, 186, 172],
        [114, 100, 86],
        [80, 78, 76],
    ];
    match stones(x, y, tile, &s, 631) {
        Some(st) => {
            let mut c = stone_colour(&st, tones, 4.0, 631);
            c = scale3(c, 1.0 - 0.15 * st.course as f64);
            // Banding across some cobbles, and a quartz vein through a few.
            let off = hash(st.id, 33, 631) * 50.0;
            let band = (st.a / 7.0 + lnoise(st.a / 25.0 + off, st.c / 25.0, 633) * 3.0).sin();
            if hash(st.id, 31, 631) < 0.35 {
                c = scale3(c, 0.93 + 0.09 * band);
            }
            let vein = st.c + 8.0 * (lnoise(st.a / 30.0 + off, 0.5, 635) - 0.5);
            if hash(st.id, 32, 631) < 0.07 && vein.abs() < 2.0 {
                c = lerp3(c, scale3(lin([224, 220, 212]), 0.3 + 0.7 * st.top), 0.8);
            }
            Px {
                c,
                h: st.h,
                r: 0.45 + 0.2 * hash(st.id, 30, 631),
            }
        }
        // Pea gravel down between the cobbles.
        None => {
            let mut p = pea_gravel(x, y, tile, 637);
            p.c = scale3(p.c, 0.45);
            p.h -= 22.0;
            p
        }
    }
}

fn decomposed_granite(x: f64, y: f64, tile: f64) -> Px {
    let seed = 651;
    let tone = gnoise(x, y, tile, 300.0, 3, seed);
    let fine = gnoise(x, y, tile, 2.5, 2, seed ^ 1);
    let mut c = scale3(lin([184, 156, 118]), 0.82 + 0.25 * tone);
    c = scale3(c, 0.85 + 0.25 * fine);
    let mut h = 0.4 * fine + 1.5 * tone;
    let mut r = 0.92;
    // Grains: feldspar, quartz, mica and iron-stained bits.
    let grains = Stones {
        cell: 2.6,
        per: 1,
        r: (0.5, 1.2),
        elong: 0.3,
        flat: 0.4,
        angular: true,
        sink: 0.5,
        ..STONES
    };
    let tones: &[[u8; 3]] = &[
        [214, 196, 168],
        [228, 220, 204],
        [104, 92, 80],
        [170, 120, 84],
        [196, 170, 136],
        [150, 128, 104],
    ];
    if let Some(st) = stones(x, y, tile, &[grains], seed ^ 3) {
        if st.h > 0.0 {
            c = lerp3(c, stone_colour(&st, tones, 0.3, seed), 0.55);
            h += st.h;
            r = 0.8;
        }
    }
    // Small pebbles, a little paler or greyer than the fines.
    let pebbles = Stones {
        cell: 22.0,
        per: 1,
        r: (2.5, 6.0),
        elong: 0.4,
        flat: 0.5,
        angular: true,
        sink: 0.5,
        ..STONES
    };
    if let Some(st) = stones(x, y, tile, &[pebbles], seed ^ 5) {
        if st.h > 0.0 && hash(st.id, 40, seed) < 0.5 {
            let pale: &[[u8; 3]] = &[
                [196, 176, 146],
                [168, 150, 126],
                [150, 142, 132],
                [186, 160, 124],
            ];
            c = stone_colour(&st, pale, 0.8, seed);
            h += st.h;
            r = 0.7;
        }
    }
    Px { c, h, r }
}

// ------------------------------------------------ paving

/// Asphalt paving: black binder with its aggregate showing; worn, it greys, the stone
/// shows more, and it cracks, is sealed and patched.
fn asphalt_paving(x: f64, y: f64, tile: f64, worn: bool) -> Px {
    let seed = if worn { 701 } else { 711 };
    let tone = gnoise(x, y, tile, 500.0, 3, seed);
    let mottle = gnoise(x, y, tile, 40.0, 3, seed ^ 1);
    let sand = speck(x, y, tile, 0.7, seed ^ 2);
    // Patched: a newer rectangle of asphalt, its saw-cut seam sealed.
    let (mut patched, mut seam) = (false, f64::MAX);
    if worn {
        let (px0, py0, pw, ph) = (0.18 * tile, 0.56 * tile, 0.42 * tile, 0.24 * tile);
        let (qx, qy) = (x.rem_euclid(tile) - px0, y.rem_euclid(tile) - py0);
        if (0.0..pw).contains(&qx) && (0.0..ph).contains(&qy) {
            patched = true;
            seam = qx.min(pw - qx).min(qy).min(ph - qy);
        }
    }
    let aged = worn && !patched;
    let binder = if aged {
        scale3(lin([78, 76, 72]), 0.8 + 0.35 * tone)
    } else {
        scale3(lin([if worn { 40 } else { 30 }; 3]), 0.85 + 0.3 * tone)
    };
    let mut c = scale3(binder, 0.8 + 0.25 * mottle + 0.25 * (sand - 0.5));
    let mut h = 0.3 * sand + 0.8 * mottle;
    let mut r = 0.9 - 0.06 * mottle;
    // Aggregate: angular stones pressed into the mat, the tops worn clean.
    let agg = Stones {
        cell: 7.0,
        per: 2,
        r: (1.5, 4.5),
        elong: 0.45,
        flat: 0.5,
        angular: true,
        sink: 0.35,
        ..STONES
    };
    let tones: &[[u8; 3]] = &[
        [120, 118, 114],
        [96, 94, 92],
        [142, 138, 132],
        [110, 104, 96],
        [80, 80, 82],
    ];
    if let Some(st) = stones(x, y, tile, &[agg], seed ^ 3) {
        if st.h > 0.0 {
            let expose = if aged { 0.85 } else { 0.28 };
            let stone = stone_colour(&st, tones, 0.5, seed);
            let k = expose * smoothstep(0.0, 0.6, st.top);
            c = lerp3(c, stone, k);
            h += st.h;
            r = 0.9 - 0.15 * k;
        }
    }
    // Pits where fines have gone.
    if speck(x, y, tile, 1.4, seed ^ 4) > if aged { 0.93 } else { 0.965 } {
        c = scale3(c, 0.45);
        h -= 1.0;
    }
    if worn {
        // Oil drips and tyre polish.
        let stain = gnoise(x, y, tile, 250.0, 4, seed ^ 5);
        c = scale3(c, 1.0 - 0.35 * smoothstep(0.62, 0.8, stain));
        // Cracks: a meandering network, sealed with a band of black crack filler.
        // Warped so they run jagged, as cracks do.
        let n = |x: f64, y: f64| {
            let wx = x + (gnoise(x, y, tile, 60.0, 3, seed ^ 8) - 0.5) * 40.0;
            let wy = y + (gnoise(x, y, tile, 60.0, 3, seed ^ 9) - 0.5) * 40.0;
            gnoise(wx, wy, tile, 700.0, 5, seed ^ 6)
        };
        let e = 2.0;
        let v = n(x, y) - 0.5;
        let gx = (n(x + e, y) - n(x - e, y)) / (2.0 * e);
        let gy = (n(x, y + e) - n(x, y - e)) / (2.0 * e);
        let dist = v.abs() / (gx * gx + gy * gy).sqrt().max(1e-9);
        let wig = gnoise(x, y, tile, 30.0, 2, seed ^ 7);
        if dist < 2.0 + 7.0 * wig && !patched {
            c = lin([22, 22, 23]);
            h = 0.6;
            r = 0.55;
        }
        if seam < 8.0 {
            c = lin([20, 20, 21]);
            h = 0.5;
            r = 0.5;
        }
    }
    Px { c, h, r }
}

/// Broom-finished concrete paving: fine striations across the slab, 5' panels with tooled
/// joints and their smooth trowelled margins.
fn concrete_broom(x: f64, y: f64, tile: f64) -> Px {
    let seed = 731;
    let panel = 60.0 * IN;
    let (pi, dx) = cellw(x, panel, tile);
    let (pj, dy) = cellw(y, panel, tile);
    let from = dx.min(panel - dx).min(dy.min(panel - dy));
    let tone = gnoise(x, y, tile, 600.0, 3, seed);
    let blotch = gnoise(x, y, tile, 90.0, 3, seed ^ 1);
    let own = 0.94 + 0.1 * hash(pi, pj, seed);
    let mut c = scale3(
        lin([176, 172, 164]),
        own * (0.88 + 0.16 * tone) * (0.92 + 0.12 * blotch),
    );
    let grit = speck(x, y, tile, 0.6, seed ^ 2);
    c = scale3(c, 0.93 + 0.12 * grit);
    let mut h = 0.2 * grit;
    let mut r = 0.88;
    // The broom's bristle marks, running across the slab, wavering.
    let wav = gnoise(x, y, tile, 400.0, 2, seed ^ 3) * 6.0;
    let n = cells_of(tile, 2.2);
    let broom = fbm(
        x / tile,
        (y + wav) / tile,
        cells_of(tile, 60.0),
        n,
        2,
        seed ^ 4,
    );
    let margin = 1.5 * IN;
    if from > margin {
        h += 1.2 * broom;
        c = scale3(c, 0.84 + 0.28 * broom);
    } else {
        // Trowelled smooth by the edger.
        c = scale3(c, 1.04);
        r = 0.72;
    }
    // The joint: a 1/4" groove with rounded edges.
    if from < 3.2 {
        c = scale3(c, 0.4 + 0.1 * from);
        h -= 6.0 - from;
    } else if from < 6.0 {
        h -= (6.0 - from) * 0.8;
        c = scale3(c, 0.92);
    }
    Px { c, h, r }
}

/// A unit paver's face: speckled aggregate, chamfered edges, and the sand joint.
fn paver_face(base: [f64; 3], edge: f64, id: i64, x: f64, y: f64, tile: f64, seed: u32) -> Px {
    let joint = 1.6;
    let chamfer = 6.0;
    if edge < joint {
        let s = speck(x, y, tile, 0.6, seed ^ 1);
        return Px {
            c: scale3(lin([150, 138, 116]), 0.45 + 0.35 * s),
            h: -5.0 + s * 0.5,
            r: 0.95,
        };
    }
    let grit = speck(x, y, tile, 0.55, seed ^ 2);
    let wear = gnoise(x, y, tile, 60.0, 3, seed ^ 3);
    let mut c = scale3(
        base,
        (0.88 + 0.2 * hash(id, 3, seed)) * (0.9 + 0.2 * grit) * (0.92 + 0.14 * wear),
    );
    let mut h = 0.25 * grit + 0.3 * wear;
    let e = edge - joint;
    if e < chamfer {
        h -= (chamfer - e) * 0.6;
        c = scale3(c, 0.8 + 0.2 * e / chamfer);
    }
    // The odd dark aggregate fleck.
    if grit > 0.97 {
        c = scale3(c, 0.55);
    }
    Px {
        c,
        h,
        r: 0.82 + 0.1 * grit,
    }
}

/// Concrete pavers, 6" x 12", in running bond, a three-colour blend.
fn pavers_running(x: f64, y: f64, tile: f64) -> Px {
    let seed = 751;
    let (w, e) = (12.0 * IN, 6.0 * IN);
    let (row, dy) = cellw(y, e, tile);
    let off = if row.rem_euclid(2) == 1 { w / 2.0 } else { 0.0 };
    let (col, dx) = cellw(x + off, w, tile);
    let edge = dx.min(w - dx).min(dy.min(e - dy));
    let id = row * 97 + col;
    let blend: &[[u8; 3]] = &[
        [150, 144, 136],
        [128, 124, 118],
        [162, 152, 138],
        [112, 108, 104],
    ];
    paver_face(pick(blend, hash(id, 5, seed)), edge, id, x, y, tile, seed)
}

/// The herringbone brick holding unit cell (s, t): its first cell, whether it lies
/// horizontal, and which of its two cells this is. The motif: a horizontal brick on cells
/// (0,0),(1,0) and a vertical one on (0,1),(0,2), repeated over the lattice (1,1), (2,-2).
fn herringbone(s: i64, t: i64) -> (i64, i64, bool, i64) {
    const MOTIF: [(i64, i64, bool, i64); 4] = [
        (0, 0, true, 0),
        (1, 0, true, 1),
        (0, 1, false, 0),
        (0, 2, false, 1),
    ];
    for (mx, my, hz, k) in MOTIF {
        let (dx, dy) = (s - mx, t - my);
        if (dx + dy).rem_euclid(2) == 0 && (dx - dy).rem_euclid(4) == 0 {
            return if hz {
                (s - k, t, true, k)
            } else {
                (s, t - k, false, k)
            };
        }
    }
    // The motif covers every cell (see the test); a fallback all the same.
    (s, t, true, 0)
}

/// Clay brick pavers, 4" x 8", in 90° herringbone: each brick's cell in a lattice of
/// L-shaped pairs (a horizontal and a vertical brick) stepping one unit up the diagonal.
fn pavers_herringbone(x: f64, y: f64, tile: f64) -> Px {
    let seed = 761;
    let u = 4.0 * IN;
    let (s, fx) = cell(x, u);
    let (t, fy) = cell(y, u);
    let (ox, oy, horiz, part) = herringbone(s, t);
    // Position within the brick (mm): along its length and across it.
    let (along, across) = if horiz {
        (part as f64 * u + fx, fy)
    } else {
        (part as f64 * u + fy, fx)
    };
    let edge = along.min(2.0 * u - along).min(across.min(u - across));
    let n = (tile / u).round() as i64;
    let id = oy.rem_euclid(n) * n + ox.rem_euclid(n);
    let reds: &[[u8; 3]] = &[
        [146, 72, 54],
        [128, 62, 48],
        [160, 90, 66],
        [112, 56, 44],
        [150, 100, 76],
        [134, 78, 60],
    ];
    let mut p = paver_face(pick(reds, hash(id, 5, seed)), edge, id, x, y, tile, seed);
    // Kiln flashing: ends darker on some bricks.
    let flash = hash(id, 6, seed);
    if flash < 0.4 {
        let end = along.min(2.0 * u - along) / u;
        p.c = scale3(p.c, 0.8 + 0.2 * smoothstep(0.0, 0.6, end));
    }
    p
}

/// Irregular bluestone flagging, full-colour, with mortar joints.
/// Pattern-cut bluestone (ADR-095): rows 24" deep of 36" and 24" stones, alternating so
/// the joints break, with 3/8" sand joints; a thermal (flamed) face, fine and even, each
/// stone its own blue-grey.
fn bluestone_pattern(x: f64, y: f64, tile: f64) -> Px {
    let seed = 811;
    let d = 24.0 * IN;
    let (row, dy) = cellw(y, d, tile);
    // A 60" repeat of a 36" and a 24" stone, shifted every other row.
    let period = 60.0 * IN;
    let off = if row.rem_euclid(2) == 1 {
        18.0 * IN
    } else {
        0.0
    };
    let (rep, px) = cellw(x + off, period, tile);
    let (k, dx, w) = if px < 36.0 * IN {
        (0, px, 36.0 * IN)
    } else {
        (1, px - 36.0 * IN, 24.0 * IN)
    };
    let edge = dx.min(w - dx).min(dy.min(d - dy));
    let id = row * 131 + rep * 2 + k;
    let joint = 9.5;
    if edge < joint {
        let s = speck(x, y, tile, 0.5, seed ^ 1);
        return Px {
            c: scale3(lin([112, 108, 102]), 0.4 + 0.25 * s),
            h: -8.0 + 0.5 * s,
            r: 0.95,
        };
    }
    let tones: &[[u8; 3]] = &[
        [146, 142, 138],
        [136, 134, 132],
        [152, 148, 144],
        [130, 130, 130],
        [144, 140, 134],
        [138, 136, 134],
    ];
    let mut c = pick(tones, hash(id, 5, seed));
    let fine = gnoise(x, y, tile, 3.0, 3, seed ^ 2);
    let mottle = gnoise(x, y, tile, 140.0, 3, seed ^ 3 ^ id as u32);
    let grit = speck(x, y, tile, 0.7, seed ^ 4);
    // The odd rust or lilac cast through the bed.
    let cast = gnoise(x, y, tile, 320.0, 2, seed ^ 5);
    c = lerp3(c, lin([132, 114, 98]), 0.25 * smoothstep(0.62, 0.86, cast));
    c = scale3(
        c,
        (0.86 + 0.1 * mottle)
            * (0.9 + 0.16 * fine)
            * (0.94 + 0.1 * grit)
            * (0.92 + 0.16 * hash(id, 6, seed)),
    );
    // Arrised edges, a touch darker.
    let e = edge - joint;
    let arris = smoothstep(0.0, 5.0, e);
    Px {
        c: scale3(c, 0.82 + 0.18 * arris),
        h: 0.5 * fine + 0.6 * mottle - 2.0 * (1.0 - arris) + 1.5 * hash(id, 7, seed),
        r: 0.72 + 0.12 * fine,
    }
}

/// One cleft bluestone slab (ADR-095): the pattern-cut stone's face with no joints, for
/// stepping stones and treads, a little more relief from the cleft.
fn bluestone_slab(x: f64, y: f64, tile: f64) -> Px {
    let seed = 823;
    let fine = gnoise(x, y, tile, 3.0, 3, seed ^ 2);
    let mottle = gnoise(x, y, tile, 140.0, 3, seed ^ 3);
    let cleft = gnoise(x, y, tile, 420.0, 3, seed ^ 6);
    let grit = speck(x, y, tile, 0.7, seed ^ 4);
    let cast = gnoise(x, y, tile, 320.0, 2, seed ^ 5);
    let c = lerp3(
        lin([146, 142, 136]),
        lin([132, 114, 98]),
        0.25 * smoothstep(0.62, 0.86, cast),
    );
    Px {
        c: scale3(
            c,
            (0.86 + 0.12 * mottle)
                * (0.9 + 0.16 * fine)
                * (0.94 + 0.1 * grit)
                * (0.9 + 0.14 * cleft),
        ),
        h: 0.5 * fine + 0.6 * mottle + 2.5 * cleft,
        r: 0.74 + 0.12 * fine,
    }
}

fn flagstone(x: f64, y: f64, tile: f64) -> Px {
    let seed = 781;
    // Warp the Voronoi so the joints wander like hand-cut edges.
    let wx = x + (gnoise(x, y, tile, 250.0, 3, seed) - 0.5) * 70.0;
    let wy = y + (gnoise(x, y, tile, 250.0, 3, seed ^ 1) - 0.5) * 70.0;
    let n = cells_of(tile, 470.0);
    let cs = tile / n as f64;
    let (gx, gy) = ((wx / cs).floor() as i64, (wy / cs).floor() as i64);
    let mut near = [(f64::MAX, 0.0, 0.0, 0i64); 2];
    for j in -2..=2 {
        for i in -2..=2 {
            let (cx, cy) = (gx + i, gy + j);
            let id = cy.rem_euclid(n) * n + cx.rem_euclid(n);
            let px = (cx as f64 + 0.15 + 0.7 * hash(id, 1, seed)) * cs;
            let py = (cy as f64 + 0.15 + 0.7 * hash(id, 2, seed)) * cs;
            let d = (wx - px).powi(2) + (wy - py).powi(2);
            if d < near[0].0 {
                near[1] = near[0];
                near[0] = (d, px, py, id);
            } else if d < near[1].0 {
                near[1] = (d, px, py, id);
            }
        }
    }
    let (d1, p1x, p1y, id) = near[0];
    let (d2, p2x, p2y, _) = near[1];
    let gap = (d2 - d1) / (2.0 * ((p2x - p1x).powi(2) + (p2y - p1y).powi(2)).sqrt());
    let jw = 10.0 + 5.0 * gnoise(x, y, tile, 80.0, 2, seed ^ 2);
    if gap < jw {
        // Mortar: sandy grey, set down below the stone.
        let s = speck(x, y, tile, 0.7, seed ^ 3);
        let m = gnoise(x, y, tile, 20.0, 2, seed ^ 4);
        return Px {
            c: scale3(lin([146, 142, 134]), (0.7 + 0.2 * m) * (0.85 + 0.25 * s)),
            h: -8.0 + 2.0 * m,
            r: 0.95,
        };
    }
    let tones: &[[u8; 3]] = &[
        [104, 112, 120],
        [94, 100, 106],
        [114, 120, 124],
        [118, 110, 112],
        [130, 118, 104],
        [110, 106, 100],
        [100, 108, 114],
    ];
    let mut c = pick(tones, hash(id, 5, seed));
    // Cleft face: shallow terraces where the layers split.
    let cl = gnoise(x, y, tile, 90.0, 5, seed ^ 5 ^ id as u32);
    let steps = (cl * 6.0).floor() + smoothstep(0.0, 0.12, (cl * 6.0).fract());
    let fine = gnoise(x, y, tile, 4.0, 3, seed ^ 6);
    let grit = speck(x, y, tile, 0.8, seed ^ 8);
    // Rust and lilac drifts through full-colour stone.
    let drift = gnoise(x, y, tile, 200.0, 3, seed ^ 7);
    c = lerp3(c, lin([140, 116, 90]), 0.35 * smoothstep(0.6, 0.85, drift));
    c = scale3(
        c,
        (0.8 + 0.07 * steps)
            * (0.82 + 0.3 * fine)
            * (0.92 + 0.12 * grit)
            * (0.9 + 0.2 * hash(id, 6, seed)),
    );
    // Edges chipped a little lower.
    let edge = smoothstep(jw, jw + 12.0, gap);
    Px {
        c: scale3(c, 0.8 + 0.2 * edge),
        h: 1.2 * steps + 0.4 * fine - 3.0 * (1.0 - edge) + 4.0 * hash(id, 7, seed),
        r: 0.78 + 0.1 * fine,
    }
}

// ------------------------------------------------ soils

fn sand(x: f64, y: f64, tile: f64) -> Px {
    let seed = 801;
    let tone = gnoise(x, y, tile, 350.0, 3, seed);
    // Soft wind ripples across the tile.
    let k = cells_of(tile, 80.0) as f64;
    let warp = gnoise(x, y, tile, 300.0, 2, seed ^ 1) * 3.0;
    let rip = ((y / tile * k + warp) * std::f64::consts::TAU).sin();
    let grain = speck(x, y, tile, 0.35, seed ^ 2);
    let grain2 = speck(x, y, tile, 0.5, seed ^ 3);
    let mut c = scale3(lin([208, 186, 148]), 0.86 + 0.2 * tone);
    c = scale3(c, 0.84 + 0.25 * grain);
    if grain2 > 0.94 {
        c = lerp3(c, lin([90, 76, 62]), 0.6);
    } else if grain2 < 0.05 {
        c = lerp3(c, lin([240, 236, 226]), 0.6);
    }
    c = scale3(c, 0.95 + 0.06 * rip);
    Px {
        c,
        h: 1.5 * rip + 0.3 * grain + 2.0 * tone,
        r: 0.9,
    }
}

fn soil(x: f64, y: f64, tile: f64) -> Px {
    let seed = 821;
    let moist = gnoise(x, y, tile, 300.0, 3, seed);
    let dark = 0.75 + 0.35 * (1.0 - smoothstep(0.4, 0.75, moist));
    let clods = Stones {
        cell: 9.0,
        per: 2,
        r: (2.0, 7.0),
        elong: 0.5,
        flat: 0.6,
        angular: false,
        sink: 0.4,
        ..STONES
    };
    let tones: &[[u8; 3]] = &[[76, 56, 40], [64, 46, 34], [88, 66, 46], [56, 42, 32]];
    let mut p = match stones(x, y, tile, &[clods], seed ^ 1) {
        Some(st) => Px {
            c: stone_colour(&st, tones, 0.7, seed),
            h: st.h,
            r: 0.9,
        },
        None => Px {
            c: scale3(
                lin([40, 30, 22]),
                0.5 + 0.3 * speck(x, y, tile, 0.6, seed ^ 2),
            ),
            h: -2.0,
            r: 0.95,
        },
    };
    p.c = scale3(p.c, dark);
    // Crumbs, grit and bits of straw.
    let crumb = speck(x, y, tile, 0.8, seed ^ 3);
    if crumb > 0.985 {
        p.c = lin([150, 140, 124]);
        p.h += 0.8;
        p.r = 0.7;
    }
    let grid = Grid::new(tile, 60.0, 1, 12.0);
    grid.each(x, y, seed ^ 5, |dx, dy, id| {
        if hash(id, 3, seed) > 0.25 {
            return;
        }
        let (ux, uy) = dir_of(id, 4, seed);
        let a = dx * ux + dy * uy;
        let c = -dx * uy + dy * ux;
        if a.abs() < 5.0 + 6.0 * hash(id, 5, seed) && c.abs() < 0.6 {
            p.c = scale3(lin([120, 100, 70]), 0.6 + 0.4 * hash(id, 6, seed));
            p.h += 1.0;
        }
    });
    p
}

/// Cushion moss: soft mounds knit together, star-like fronds over them, dark crevices
/// between, browned where it's dried.
fn moss(x: f64, y: f64, tile: f64) -> Px {
    let seed = 841;
    // Mounds: rounded by squaring the noise's upper half.
    let mound = gnoise(x, y, tile, 45.0, 4, seed);
    let top = smoothstep(0.25, 0.72, mound);
    // Clumps, and fronds: tiny leafy tips a millimetre or two across.
    let clump = gnoise(x, y, tile, 9.0, 2, seed ^ 5);
    let frond = gnoise(x, y, tile, 2.5, 3, seed ^ 2) * 0.6 + clump * 0.4;
    let tip = gnoise(x, y, tile, 0.9, 1, seed ^ 1);
    let hue = gnoise(x, y, tile, 160.0, 3, seed ^ 3);
    let greens = lerp3(lin([62, 88, 30]), lin([118, 134, 46]), hue);
    let mut c = scale3(greens, (0.4 + 0.75 * frond) * (0.8 + 0.35 * tip));
    // Paler, yellower tips on the crowns.
    c = lerp3(
        c,
        lin([150, 156, 70]),
        0.25 * top * smoothstep(0.6, 0.8, frond),
    );
    // Browned patches.
    let brown = smoothstep(0.68, 0.8, gnoise(x, y, tile, 220.0, 3, seed ^ 4));
    c = lerp3(
        c,
        scale3(lin([112, 94, 52]), 0.6 + 0.5 * frond),
        brown * 0.7,
    );
    c = scale3(c, 0.6 + 0.4 * top.powf(0.6));
    Px {
        c,
        h: 9.0 * top + 1.0 * frond + 0.3 * tip,
        r: 0.82 + 0.1 * (1.0 - frond),
    }
}

fn snow(x: f64, y: f64, tile: f64) -> Px {
    let seed = 861;
    let drift = gnoise(x, y, tile, 400.0, 4, seed);
    let fine = gnoise(x, y, tile, 8.0, 3, seed ^ 1);
    let lumps = gnoise(x, y, tile, 60.0, 3, seed ^ 3);
    let crystal = speck(x, y, tile, 0.6, seed ^ 2);
    let hollow = 1.0 - smoothstep(0.3, 0.6, drift);
    let mut c = lerp3(lin([242, 244, 248]), lin([206, 216, 232]), hollow * 0.6);
    c = scale3(c, (0.95 + 0.05 * fine) * (0.93 + 0.07 * lumps));
    let mut r = 0.6 + 0.1 * fine;
    if crystal > 0.985 {
        // Glints off the crystals.
        c = lin([255, 255, 255]);
        r = 0.15;
    }
    Px {
        c,
        h: 10.0 * drift + 3.0 * lumps + 0.8 * fine + 0.2 * crystal,
        r,
    }
}

/// Ground covers (ADR-064), by kind; None for other kinds.
fn ground(kind: &str, x: f64, y: f64, tile: f64) -> Option<Px> {
    Some(match kind {
        "lawn" => turf(x, y, tile, &LAWN, 301),
        "lawn-lush" => turf(x, y, tile, &LAWN_LUSH, 311),
        "lawn-dry" => turf(x, y, tile, &LAWN_DRY, 321),
        "meadow" => turf(x, y, tile, &MEADOW, 331),
        "pine-straw" => pine_straw(x, y, tile),
        "leaf-litter" => leaf_litter(x, y, tile),
        "bark-mulch" => mulch(
            x,
            y,
            tile,
            // Freshly laid double-shredded hardwood (ADR-101): a deep, rich brown.
            &[
                [84, 52, 32],
                [104, 66, 40],
                [66, 42, 28],
                [122, 80, 50],
                [92, 58, 38],
            ],
            541,
        ),
        "black-mulch" => mulch(
            x,
            y,
            tile,
            &[[36, 32, 30], [46, 40, 36], [28, 26, 24], [56, 46, 40]],
            551,
        ),
        "red-mulch" => mulch(
            x,
            y,
            tile,
            &[
                [132, 52, 36],
                [112, 42, 30],
                [150, 66, 44],
                [96, 40, 30],
                [124, 58, 42],
            ],
            561,
        ),
        "pea-gravel" => pea_gravel(x, y, tile, 601),
        "crushed-stone" => crushed_stone(x, y, tile),
        "river-rock" => river_rock(x, y, tile),
        "decomposed-granite" => decomposed_granite(x, y, tile),
        "asphalt" => asphalt_paving(x, y, tile, false),
        "asphalt-worn" => asphalt_paving(x, y, tile, true),
        "concrete-broom" => concrete_broom(x, y, tile),
        "pavers-running" => pavers_running(x, y, tile),
        "pavers-herringbone" => pavers_herringbone(x, y, tile),
        "flagstone" => flagstone(x, y, tile),
        "bluestone-pattern" => bluestone_pattern(x, y, tile),
        "bluestone-slab" => bluestone_slab(x, y, tile),
        "sand" => sand(x, y, tile),
        "soil" => soil(x, y, tile),
        "moss" => moss(x, y, tile),
        "snow" => snow(x, y, tile),
        _ => return None,
    })
}

// ---------------------------------------------------------------- generation

fn to_srgb(v: f64) -> u8 {
    (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round() as u8
}

/// Generates `kind` at `size` × `size` pixels, over as many threads as there are cores.
pub fn generate(kind: &str, size: usize) -> Option<GenTexture> {
    let tile = tile_of(kind)?;
    let px = tile / size as f64;
    let n = size * size;
    let mut color = vec![0u8; n * 3];
    let mut rough = vec![0u8; n * 3];
    let mut height = vec![0f32; n];
    let threads = std::thread::available_parallelism()
        .map_or(4, |t| t.get())
        .min(32);
    let rows_per = size.div_ceil(threads);
    std::thread::scope(|s| {
        let bands = color
            .chunks_mut(rows_per * size * 3)
            .zip(rough.chunks_mut(rows_per * size * 3))
            .zip(height.chunks_mut(rows_per * size))
            .enumerate();
        for (band, ((c, r), h)) in bands {
            s.spawn(move || {
                let rows = h.len() / size;
                for k in 0..rows {
                    let row = band * rows_per + k;
                    // y up the tile: row 0 is the top.
                    let y = (size - 1 - row) as f64 * px + px / 2.0;
                    for col in 0..size {
                        let x = col as f64 * px + px / 2.0;
                        let p = sample(kind, x, y, tile);
                        let i = k * size + col;
                        c[i * 3] = to_srgb(p.c[0]);
                        c[i * 3 + 1] = to_srgb(p.c[1]);
                        c[i * 3 + 2] = to_srgb(p.c[2]);
                        let rv = (p.r.clamp(0.02, 1.0) * 255.0).round() as u8;
                        r[i * 3..i * 3 + 3].fill(rv);
                        h[i] = p.h as f32;
                    }
                }
            });
        }
    });
    // Normals from the height field (mm), wrapping so the tile repeats cleanly.
    let mut normal = vec![0u8; n * 3];
    let at = |row: usize, col: usize| f64::from(height[row * size + col]);
    for row in 0..size {
        let (up, down) = ((row + size - 1) % size, (row + 1) % size);
        for col in 0..size {
            let (l, rr) = ((col + size - 1) % size, (col + 1) % size);
            let dx = (at(row, rr) - at(row, l)) / (2.0 * px);
            let dy = (at(up, col) - at(down, col)) / (2.0 * px);
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let i = (row * size + col) * 3;
            normal[i] = ((-dx / len * 0.5 + 0.5) * 255.0).round() as u8;
            normal[i + 1] = ((-dy / len * 0.5 + 0.5) * 255.0).round() as u8;
            normal[i + 2] = ((1.0 / len * 0.5 + 0.5) * 255.0).round() as u8;
        }
    }
    Some(GenTexture {
        size,
        color,
        normal,
        rough,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_tiles_its_courses_and_panels() {
        // Courses, boards and panels divide each tile exactly, so the tile repeats.
        let courses = |kind: &str, e: f64| {
            let t = tile_of(kind).unwrap() / e;
            (t - t.round()).abs() < 1e-9
        };
        assert!(courses("cedar-bevel", 6.0 * IN));
        assert!(courses("shiplap8", 8.0 * IN));
        assert!(courses("board-batten12", 12.0 * IN));
        assert!(courses("cedar-vertical-tg", 6.0 * IN));
        assert!(courses("accoya-slats", 2.0 * IN));
        assert!(courses("cedar-shingles", 6.0 * IN));
        assert!(courses("asphalt-charcoal", 5.625 * IN));
        assert!(courses("seam16", 16.0 * IN));
        assert!(courses("seam18", 18.0 * IN));
        assert!(courses("flat-concrete-tile", 13.0 * IN));
        let barn: f64 = [
            4., 6., 8., 6., 4., 8., 6., 4., 6., 8., 4., 6., 8., 6., 4., 8.,
        ]
        .iter()
        .sum();
        assert!((barn * IN - FT8).abs() < 1e-9);
    }

    #[test]
    fn a_set_is_seamless_and_its_maps_make_sense() {
        for (kind, _, _) in KINDS {
            let g = generate(kind, 128).unwrap();
            assert_eq!(g.color.len(), 128 * 128 * 3);
            assert_eq!(g.normal.len(), g.color.len());
            // Normals point out of the surface.
            let blue: f64 = g
                .normal
                .iter()
                .skip(2)
                .step_by(3)
                .map(|v| f64::from(*v))
                .sum::<f64>()
                / (128.0 * 128.0);
            assert!(blue > 180.0, "{kind}: {blue}");
            // It repeats exactly: every sample equals the one a tile over, either way.
            let tile = tile_of(kind).unwrap();
            for k in 0..200 {
                let (x, y) = (hash(k, 1, 997) * tile, hash(k, 2, 991) * tile);
                let a = sample(kind, x, y, tile);
                for (bx, by) in [(x + tile, y), (x, y + tile), (x - tile, y - tile)] {
                    let b = sample(kind, bx, by, tile);
                    let d =
                        (0..3).map(|i| (a.c[i] - b.c[i]).abs()).sum::<f64>() + (a.h - b.h).abs();
                    assert!(d < 1e-6, "{kind} at ({x:.1}, {y:.1}): {d}");
                }
            }
        }
    }

    #[test]
    fn every_generated_preset_names_a_kind_at_its_tile_size() {
        let lib = studio_core::library::library();
        let gen: Vec<_> = lib
            .iter()
            .filter_map(|p| Some((p, p.appearance.texture.as_deref()?.strip_prefix("gen:")?)))
            .collect();
        assert!(gen.len() >= 30);
        for (p, kind) in gen {
            let tile = tile_of(kind).unwrap_or_else(|| panic!("{}: no kind {kind}", p.id));
            assert!((p.appearance.scale - tile).abs() < 1e-9, "{}", p.id);
        }
        let sidings = lib.iter().filter(|p| p.category == "Siding").count();
        assert!(sidings >= 15);
    }

    #[test]
    fn sidings_read_as_boards_and_roofs_as_courses() {
        // Bevel siding: darker right under each butt than mid-board.
        let tile = tile_of("cedar-bevel").unwrap();
        let lum = |p: Px| p.c[0] + p.c[1] + p.c[2];
        let e = 6.0 * IN;
        // Along the course, on average (joints and nails aside).
        let avg = |y: f64| {
            (0..48)
                .map(|k| lum(sample("cedar-bevel", 25.0 + k as f64 * 50.0, y, tile)))
                .sum::<f64>()
                / 48.0
        };
        let (under, mid) = (avg(3.0 * e - 2.0), avg(2.0 * e + e / 2.0));
        assert!(under < mid * 0.8);
        // A standing seam stands above its pan.
        let st = tile_of("seam16").unwrap();
        assert!(sample("seam16", 16.0 * IN, 500.0, st).h > 30.0);
        assert!(sample("seam16", 8.0 * IN, 500.0, st).h < 2.0);
        // Galvalume is glossier than the shingles.
        assert!(sample("galvalume16", 100.0, 100.0, st).r < 0.5);
        assert!(
            sample(
                "asphalt-charcoal",
                100.0,
                100.0,
                tile_of("asphalt-charcoal").unwrap()
            )
            .r > 0.9
        );
        assert!(generate("no-such-kind", 16).is_none());
    }

    /// Average linear colour and the spread of heights over a patch of a kind.
    fn survey(kind: &str) -> ([f64; 3], f64, f64) {
        let tile = tile_of(kind).unwrap();
        let (mut c, mut hs) = ([0.0; 3], vec![]);
        let n = 60;
        for i in 0..n {
            for j in 0..n {
                // A 300 mm patch at a fine pitch, plus spots across the tile.
                let (x, y) = if (i + j) % 2 == 0 {
                    (i as f64 * 5.0, j as f64 * 5.0)
                } else {
                    (hash(i, j, 3) * tile, hash(i, j, 5) * tile)
                };
                let p = sample(kind, x, y, tile);
                for (acc, v) in c.iter_mut().zip(p.c) {
                    *acc += v;
                }
                hs.push(p.h);
            }
        }
        let m = (n * n) as f64;
        let mean = hs.iter().sum::<f64>() / m;
        let sd = (hs.iter().map(|h| (h - mean).powi(2)).sum::<f64>() / m).sqrt();
        let r = (0..n * n)
            .map(|k| {
                let tile = tile_of(kind).unwrap();
                sample(kind, hash(k, 7, 11) * tile, hash(k, 8, 13) * tile, tile).r
            })
            .sum::<f64>()
            / m;
        (c.map(|v| v / m), sd, r)
    }

    #[test]
    fn ground_covers_read_as_what_they_are() {
        // Lawns are green, the lush one greener; dry lawn is yellower.
        let (lawn, lawn_h, _) = survey("lawn");
        assert!(
            lawn[1] > lawn[0] * 1.2 && lawn[1] > lawn[2] * 2.0,
            "{lawn:?}"
        );
        let (lush, _, _) = survey("lawn-lush");
        assert!(lush[1] / lush[0] > lawn[1] / lawn[0], "{lush:?}");
        let (dry, _, _) = survey("lawn-dry");
        assert!(dry[0] / dry[1] > lawn[0] / lawn[1], "{dry:?}");
        assert!(lawn_h > 1.0, "{lawn_h}");
        // Gravel has strong relief; decomposed granite is compacted flat.
        let (_, pea_h, _) = survey("pea-gravel");
        let (_, dg_h, _) = survey("decomposed-granite");
        assert!(pea_h > 1.0 && pea_h > dg_h * 1.5, "{pea_h} {dg_h}");
        let (_, rr_h, _) = survey("river-rock");
        assert!(rr_h > pea_h * 2.0, "{rr_h} {pea_h}");
        // Asphalt is dark and matte; worn asphalt greys; concrete is light.
        let (asph, _, asph_r) = survey("asphalt");
        let lum = |c: [f64; 3]| (c[0] + c[1] + c[2]) / 3.0;
        assert!(lum(asph) < 0.05, "{asph:?}");
        assert!((0.78..0.95).contains(&asph_r), "{asph_r}");
        let (worn, _, _) = survey("asphalt-worn");
        assert!(lum(worn) > lum(asph) * 1.5, "{worn:?}");
        let (conc, _, _) = survey("concrete-broom");
        assert!(lum(conc) > 0.3, "{conc:?}");
        // Red mulch is red, black mulch dark, pine straw brown; snow is white.
        let (red, _, _) = survey("red-mulch");
        assert!(red[0] > red[1] * 2.0, "{red:?}");
        let (black, _, _) = survey("black-mulch");
        assert!(lum(black) < 0.03, "{black:?}");
        let (pine, _, _) = survey("pine-straw");
        assert!(pine[0] > pine[1] && pine[1] > pine[2], "{pine:?}");
        let (sn, _, _) = survey("snow");
        assert!(lum(sn) > 0.75, "{sn:?}");
    }

    #[test]
    fn herringbone_bricks_are_whole_and_joints_line_up() {
        // Every 4" unit cell belongs to a brick of two cells, whose other cell agrees, and
        // the pattern repeats with the tile.
        let tile = tile_of("pavers-herringbone").unwrap();
        let u = 4.0 * IN;
        let n = (tile / u).round() as i64;
        assert_eq!(n % 4, 0);
        let mut horizontal = 0;
        for s in 0..n {
            for t in 0..n {
                let (ox, oy, h, k) = herringbone(s, t);
                let other = if h {
                    (ox + 1 - k, oy)
                } else {
                    (ox, oy + 1 - k)
                };
                assert_eq!(herringbone(other.0, other.1), (ox, oy, h, 1 - k), "{s},{t}");
                let (px, py, ph, pk) = herringbone(s + n, t - n);
                assert_eq!((px - n, py + n, ph, pk), (ox, oy, h, k));
                horizontal += i32::from(h);
            }
        }
        assert_eq!(horizontal, (n * n / 2) as i32);
        // A joint runs between two bricks: dark and low; the brick faces are high.
        let mid = sample("pavers-herringbone", u * 0.5, u * 0.5, tile);
        let joint = sample("pavers-herringbone", u * 2.0, u * 0.5, tile);
        assert!(joint.h < mid.h - 2.0, "{} {}", joint.h, mid.h);
    }

    /// Writes the ground covers at 1024 px to target/texgen-preview to look at:
    /// `cargo test -p studio-views texgen_preview -- --ignored`.
    #[test]
    #[ignore]
    fn texgen_preview() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/texgen-preview");
        std::fs::create_dir_all(&dir).unwrap();
        let only = std::env::var("KINDS").ok();
        let size = std::env::var("SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024);
        let start = if only.is_some() {
            0
        } else {
            KINDS.iter().position(|k| k.0 == "lawn").unwrap()
        };
        for (kind, _, _) in &KINDS[start..] {
            if only
                .as_deref()
                .is_some_and(|o| !o.split(',').any(|k| k == *kind))
            {
                continue;
            }
            let t = std::time::Instant::now();
            let g = generate(kind, size).unwrap();
            eprintln!("{kind}: {:?}", t.elapsed());
            // Past 1024 px, the top-left 1024 px: a close look at full detail.
            let crop = |m: &[u8]| -> Vec<u8> {
                let s = size.min(1024);
                (0..s)
                    .flat_map(|r| m[r * size * 3..(r * size + s) * 3].to_vec())
                    .collect()
            };
            let s = size.min(1024);
            std::fs::write(dir.join(format!("{kind}.png")), png(s, &crop(&g.color))).unwrap();
            std::fs::write(
                dir.join(format!("{kind}-normal.png")),
                png(s, &crop(&g.normal)),
            )
            .unwrap();
        }
    }

    /// A minimal PNG (RGB, stored deflate): enough to look at a texture.
    fn png(size: usize, rgb: &[u8]) -> Vec<u8> {
        fn crc(data: &[u8]) -> u32 {
            let mut c = 0xFFFF_FFFFu32;
            for b in data {
                c ^= u32::from(*b);
                for _ in 0..8 {
                    c = if c & 1 == 1 {
                        0xEDB8_8320 ^ (c >> 1)
                    } else {
                        c >> 1
                    };
                }
            }
            !c
        }
        fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
            out.extend((data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend(data);
            out.extend(&body);
            out.extend(crc(&body).to_be_bytes());
        }
        let mut raw = vec![];
        for row in rgb.chunks(size * 3) {
            raw.push(0);
            raw.extend(row);
        }
        let mut z = vec![0x78, 0x01];
        let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
        for (i, b) in blocks.iter().enumerate() {
            z.push(u8::from(i + 1 == blocks.len()));
            let n = b.len() as u16;
            z.extend(n.to_le_bytes());
            z.extend((!n).to_le_bytes());
            z.extend(*b);
        }
        let (mut a, mut bb) = (1u32, 0u32);
        for v in &raw {
            a = (a + u32::from(*v)) % 65521;
            bb = (bb + a) % 65521;
        }
        z.extend(((bb << 16) | a).to_be_bytes());
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = vec![];
        ihdr.extend((size as u32).to_be_bytes());
        ihdr.extend((size as u32).to_be_bytes());
        ihdr.extend([8, 2, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &z);
        chunk(&mut out, b"IEND", &[]);
        out
    }
}
