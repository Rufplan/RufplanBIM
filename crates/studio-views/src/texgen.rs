//! Generated material textures (ADR-061): exterior wood sidings and modern roofing, made
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
}

const CEDAR: Species = Species {
    early: [200, 146, 100],
    late: [146, 92, 58],
    ring: 3.2,
    spread: 0.22,
    knots: 0.35,
    rough: 0.72,
    weathered: false,
};
const WEATHERED: Species = Species {
    early: [168, 164, 156],
    late: [118, 114, 108],
    ring: 3.2,
    spread: 0.14,
    knots: 0.35,
    rough: 0.9,
    weathered: true,
};
const THERMO_ASH: Species = Species {
    early: [132, 86, 54],
    late: [88, 54, 32],
    ring: 4.5,
    spread: 0.1,
    knots: 0.05,
    rough: 0.6,
    weathered: false,
};
const IPE: Species = Species {
    early: [124, 78, 50],
    late: [86, 50, 32],
    ring: 1.6,
    spread: 0.18,
    knots: 0.0,
    rough: 0.55,
    weathered: false,
};
const ACCOYA: Species = Species {
    early: [214, 188, 148],
    late: [178, 146, 104],
    ring: 5.0,
    spread: 0.08,
    knots: 0.0,
    rough: 0.7,
    weathered: false,
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
    let band = 0.5 + 0.5 * (phase * std::f64::consts::TAU).sin();
    // Thin, darker latewood lines.
    let late = smoothstep(0.72, 0.97, band);
    // Fibres: long fine streaks along the grain, and the open pores.
    let fibre = fbm(u, v, cells(70.0), cells(0.7), 2, seed ^ 7);
    let streak = fbm(u, v, cells(400.0), cells(4.0), 2, seed ^ 19);
    let pores = vnoise(u, v, cells(2.5), cells(0.3), seed ^ 31);
    // The board's own tone, drifting along its length.
    let drift = fbm(u, v, cells(600.0), cells(150.0), 2, seed ^ 23) - 0.5;
    let tone = 1.0 + (hash(seed as i64, 1, 11) - 0.5) * 2.0 * sp.spread + drift * 0.18;
    let warmth = (hash(seed as i64, 2, 13) - 0.5) * sp.spread;
    let mut c = lerp3(
        lin(sp.early),
        lin(sp.late),
        late * 0.85 + (1.0 - streak) * 0.2,
    );
    c = [c[0] * (1.0 + warmth), c[1], c[2] * (1.0 - warmth)];
    c = scale3(c, tone * (0.88 + 0.24 * fibre) * (0.95 + 0.1 * pores));
    let mut h = -0.12 * late + 0.06 * fibre - 0.04 * (pores - 0.5);
    let mut r = sp.rough + 0.06 * (fibre - 0.5);
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
    let (course, dy) = cell(y, exposure);
    let t = dy / exposure;
    let (b, jd) = joints(x, course, joints_per_tile, tile, 41);
    let mut p = board(x, dy, b as u32);
    p.h += profile(t);
    // The shadow cast by the course above on this board's top.
    let shade = 1.0 - 0.55 * (-(1.0 - t) * exposure / 7.0).exp();
    p.c = scale3(p.c, shade);
    // Butt joints: a hairline.
    if jd < 1.2 {
        p.c = scale3(p.c, 0.35);
        p.h -= 1.0;
    }
    p
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

/// Vertical boards `w` wide: (board index, x within it).
fn vboards(x: f64, w: f64) -> (i64, f64) {
    cell(x, w)
}

fn sample(kind: &str, x: f64, y: f64, tile: f64) -> Px {
    let paint = [0.62, 0.62, 0.62];
    match kind {
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
                2,
            )
        }
        "shiplap8" => {
            let e = 8.0 * IN;
            let (course, dy) = cell(y, e);
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
            let (b, dx) = vboards(x, w);
            let cedar = kind == "board-batten-cedar";
            // Battens 2-1/2" wide over each joint, standing 3/4" proud.
            let from_joint = dx.min(w - dx);
            let batten_half = 1.25 * IN;
            let (mut p, on_batten) = if from_joint < batten_half {
                let bi = if dx < w / 2.0 { b } else { b + 1 };
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
            let (b, dx) = vboards(x, w);
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
            let (course, dy) = cell(y, e);
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
            let (course, dy) = cell(y, e);
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
            let (course, dy) = cell(y, e);
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
            let (b, dx) = vboards(x, w);
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
            };
            let mut p = wood(&sp, x, dy, b as u32, tile);
            // Saw marks, worn edges and the odd nail hole.
            let saw = 0.5
                + 0.5
                    * ((x / 9.0 + fbm(x / tile, y / tile, 7, 7, 2, 71) * 3.0)
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
            let (course, dy) = cell(y, e);
            let off = if course.rem_euclid(2) == 1 {
                wdt / 2.0
            } else {
                0.0
            };
            let (ti, dx) = cell(x + off, wdt);
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
    let (cw, ch) = (9.0, 26.0);
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
    let (course, dy) = cell(y, e);
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
    let (course, dy) = cell(y, e);
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
    let g = 1.1;
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
    let (panel, dx) = cell(x, w);
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
        let cs = 6.0;
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
    let cs = 28.0;
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
            // The first and last columns differ no more than neighbouring columns do (it
            // wraps), on average.
            let col_diff = |a: usize, b: usize| -> f64 {
                (0..128)
                    .map(|r| {
                        (0..3)
                            .map(|k| {
                                (f64::from(g.color[(r * 128 + a) * 3 + k])
                                    - f64::from(g.color[(r * 128 + b) * 3 + k]))
                                .abs()
                            })
                            .sum::<f64>()
                    })
                    .sum::<f64>()
                    / 128.0
            };
            let wrap = col_diff(127, 0);
            let inner = (1..127).map(|c| col_diff(c - 1, c)).sum::<f64>() / 126.0;
            assert!(wrap <= inner * 3.0 + 12.0, "{kind}: wrap {wrap} vs {inner}");
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
        let under = lum(sample("cedar-bevel", 300.0, 3.0 * e - 2.0, tile));
        let mid = lum(sample("cedar-bevel", 300.0, 2.0 * e + e / 2.0, tile));
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
}
