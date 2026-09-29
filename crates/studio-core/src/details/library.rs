//! The typical details (ADR-069), drawn in inches with the exterior to the left (sections)
//! or up (plans). Light-frame US residential construction, as a starting point to edit.

use super::FillPattern::{
    Concrete, CrossHatch, Gravel, Masonry, RigidInsulation, Sand, Solid, Steel, Wood,
};
use super::D;
use crate::lines::LineStyle::{Beyond, Hidden, Medium, Thin, Wide};

pub(super) struct Entry {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub scale: u32,
    pub description: &'static str,
    pub draw: fn() -> D,
}

const FOUNDATIONS: &str = "Foundations";
const WALLS: &str = "Walls";
const OPENINGS: &str = "Openings";
const ROOFS: &str = "Roofs";
const FLOORS: &str = "Floors & Stairs";
const INTERIORS: &str = "Interiors";

pub(super) static DETAILS: &[Entry] = &[
    Entry {
        id: "slab-edge",
        name: "Thickened Slab Edge",
        category: FOUNDATIONS,
        scale: 16,
        description: "Slab-on-grade with a thickened edge under a 2x6 exterior wall.",
        draw: slab_edge,
    },
    Entry {
        id: "stem-wall",
        name: "Stem Wall at Crawlspace",
        category: FOUNDATIONS,
        scale: 16,
        description: "Concrete stem wall and footing carrying a framed floor over a crawlspace.",
        draw: stem_wall,
    },
    Entry {
        id: "basement-wall",
        name: "Basement Wall and Footing Drain",
        category: FOUNDATIONS,
        scale: 16,
        description: "Concrete foundation wall with exterior insulation, footing drain and slab.",
        draw: basement_wall,
    },
    Entry {
        id: "spread-footing",
        name: "Interior Spread Footing",
        category: FOUNDATIONS,
        scale: 12,
        description: "Isolated footing under a steel column, isolated from the slab.",
        draw: spread_footing,
    },
    Entry {
        id: "wall-plan",
        name: "Exterior Wall Assembly (Plan)",
        category: WALLS,
        scale: 4,
        description: "2x6 wall at 16\" o.c. with batts, sheathing, WRB and lap siding, in plan.",
        draw: wall_plan,
    },
    Entry {
        id: "brick-base",
        name: "Brick Veneer at Foundation",
        category: WALLS,
        scale: 8,
        description: "Brick veneer on a foundation ledge: through-wall flashing, weeps and ties.",
        draw: brick_base,
    },
    Entry {
        id: "cmu-slab",
        name: "CMU Wall at Slab on Grade",
        category: WALLS,
        scale: 8,
        description: "Grouted CMU on a footing, interior rigid insulation, isolated slab.",
        draw: cmu_slab,
    },
    Entry {
        id: "window-head",
        name: "Window Head - Wood Frame",
        category: OPENINGS,
        scale: 4,
        description: "Flanged window under an insulated header: drip cap, WRB lap and trim.",
        draw: window_head,
    },
    Entry {
        id: "window-sill",
        name: "Window Sill - Wood Frame",
        category: OPENINGS,
        scale: 4,
        description: "Sill pan flashing with back dam, sloped exterior sill, stool and apron.",
        draw: window_sill,
    },
    Entry {
        id: "window-jamb",
        name: "Window Jamb (Plan)",
        category: OPENINGS,
        scale: 4,
        description: "King and jack studs, window frame, flange, trim and casing, in plan.",
        draw: window_jamb,
    },
    Entry {
        id: "door-threshold",
        name: "Exterior Door Threshold",
        category: OPENINGS,
        scale: 2,
        description: "Aluminum saddle threshold on a slab, door bottom sweep and exterior walk.",
        draw: door_threshold,
    },
    Entry {
        id: "door-jamb",
        name: "Interior Door Jamb (Plan)",
        category: OPENINGS,
        scale: 4,
        description: "Wood jamb, stop and casings in a 2x4 partition, in plan.",
        draw: door_jamb,
    },
    Entry {
        id: "eave",
        name: "Eave with Gutter",
        category: ROOFS,
        scale: 8,
        description: "6:12 truss roof eave: vented soffit, baffle, fascia, drip edge and gutter.",
        draw: eave,
    },
    Entry {
        id: "rake",
        name: "Rake Edge",
        category: ROOFS,
        scale: 8,
        description: "Gable rake on ladder-framed outlookers with fascia and drip edge.",
        draw: rake,
    },
    Entry {
        id: "parapet",
        name: "Parapet with Metal Coping",
        category: ROOFS,
        scale: 8,
        description: "Low-slope membrane roof turned up a framed parapet under metal coping.",
        draw: parapet,
    },
    Entry {
        id: "ridge-vent",
        name: "Ridge Vent",
        category: ROOFS,
        scale: 8,
        description: "Shingled ridge vent over a slot cut in the sheathing each side.",
        draw: ridge_vent,
    },
    Entry {
        id: "rim-joist",
        name: "Floor at Exterior Wall (Rim)",
        category: FLOORS,
        scale: 8,
        description: "Floor framing bearing on a lower wall, rim joist sealed with spray foam.",
        draw: rim_joist,
    },
    Entry {
        id: "stair-tread",
        name: "Stair Tread and Riser",
        category: FLOORS,
        scale: 4,
        description: "Wood treads with nosing, risers and a cut stringer (7 3/4\" max rise).",
        draw: stair_tread,
    },
    Entry {
        id: "handrail",
        name: "Handrail at Wall",
        category: FLOORS,
        scale: 4,
        description: "Round handrail on a bracket into solid blocking, 1 1/2\" clear of the wall.",
        draw: handrail,
    },
    Entry {
        id: "deck-guard",
        name: "Guardrail at Deck Edge",
        category: FLOORS,
        scale: 8,
        description: "4x4 guard post through-bolted to the deck rim, 36\" rail and balusters.",
        draw: deck_guard,
    },
    Entry {
        id: "base-cabinet",
        name: "Base Cabinet and Countertop",
        category: INTERIORS,
        scale: 8,
        description: "Base cabinet with toe kick, countertop at 36\" and backsplash.",
        draw: base_cabinet,
    },
    Entry {
        id: "act-ceiling",
        name: "Acoustical Ceiling at Wall",
        category: INTERIORS,
        scale: 4,
        description: "Suspended ACT grid at a partition: wall angle, main runner and hanger wire.",
        draw: act_ceiling,
    },
    Entry {
        id: "partition-head",
        name: "Partition Head (Deflection)",
        category: INTERIORS,
        scale: 4,
        description: "Metal stud partition to structure with a slotted deflection track.",
        draw: partition_head,
    },
];

// ------------------------------------------------------------------------------------------
// Shared parts
// ------------------------------------------------------------------------------------------

/// Lap siding outside the face at `x` (toward -x), boards of 7" exposure from `y0` up.
fn siding(d: &mut D, x: f64, y0: f64, y1: f64) {
    let mut pts = vec![(x, y0)];
    let mut y = y0;
    while y < y1 - 0.1 {
        let top = (y + 7.0).min(y1);
        pts.push((x - 0.6, y));
        pts.push((x - 0.2, top));
        pts.push((x, top));
        y = top;
    }
    d.line(&pts, Medium);
}

/// A 2x6 wall in section from `y0` to `y1`: sheathing, WRB and siding outside (x < 0),
/// batts in the cavity, gypsum inside.
fn wall_2x6(d: &mut D, y0: f64, y1: f64, siding_from: f64) {
    d.sheet(-0.5, y0, 0.0, y1);
    d.line(&[(-0.6, siding_from), (-0.6, y1)], Hidden);
    siding(d, -0.6, siding_from, y1);
    d.batt((2.75, y0), (2.75, y1), 5.5);
    d.line(&[(0.0, y0), (0.0, y1)], Beyond);
    d.line(&[(5.5, y0), (5.5, y1)], Beyond);
    d.gyp(5.5, y0, 6.0, y1);
}

// ------------------------------------------------------------------------------------------
// Foundations
// ------------------------------------------------------------------------------------------

fn slab_edge() -> D {
    let mut d = D::new();
    d.cut(
        &[
            (0.0, 0.0),
            (48.0, 0.0),
            (48.0, -4.0),
            (20.0, -4.0),
            (12.0, -12.0),
            (0.0, -12.0),
        ],
        Concrete,
    );
    d.region(
        &[(16.0, -8.0), (20.0, -4.0), (48.0, -4.0), (48.0, -8.0)],
        Gravel,
    );
    d.earth(&[
        (-20.0, -6.0),
        (0.0, -6.0),
        (0.0, -12.0),
        (12.0, -12.0),
        (16.0, -8.0),
        (48.0, -8.0),
        (48.0, -24.0),
        (-20.0, -24.0),
    ]);
    d.line(&[(-20.0, -6.0), (0.0, -6.0)], Medium);
    d.line(&[(18.0, -4.3), (48.0, -4.3)], Hidden);
    d.rebar(4.0, -9.0, 0.6);
    d.rebar(8.0, -9.0, 0.6);
    d.lumber(0.0, 0.0, 5.5, 1.5);
    d.line(&[(0.0, 0.0), (5.5, 0.0)], Wide);
    wall_2x6(&mut d, 1.5, 30.0, 0.5);
    d.sheet(-0.5, -1.0, 0.0, 1.5);
    d.line(&[(2.75, 2.2), (2.75, -8.0), (4.5, -8.0)], Hidden);
    d.rect(1.5, 1.5, 4.0, 2.0, Medium);
    d.brk((-3.0, 30.0), (8.0, 30.0));
    d.brk((48.0, 2.0), (48.0, -10.0));
    d.note((1.0, 0.75), "2x6 P.T. SILL PLATE ON SILL SEALER");
    d.note(
        (2.75, -3.0),
        "1/2\" DIA. ANCHOR BOLT @ 6'-0\" O.C., 7\" MIN. EMBED.",
    );
    d.note((34.0, -2.0), "4\" CONC. SLAB W/ 6x6-W1.4xW1.4 W.W.F.");
    d.note((30.0, -4.3), "6 MIL VAPOR RETARDER");
    d.note((40.0, -6.0), "4\" COMPACTED GRAVEL");
    d.note((8.0, -9.0), "(2) #4 CONT., 3\" CLR.");
    d.note((3.0, 18.0), "R-21 BATT INSULATION");
    d.note((5.75, 24.0), "1/2\" GYP. BD.");
    d.note((-0.25, 24.0), "7/16\" OSB SHEATHING W/ WRB");
    d.note((-1.0, 12.0), "FIBER CEMENT LAP SIDING");
    d.note((-12.0, -6.0), "FINISH GRADE, SLOPE AWAY 5% MIN.");
    d
}

fn stem_wall() -> D {
    let mut d = D::new();
    d.cut(
        &[
            (0.0, 0.0),
            (8.0, 0.0),
            (8.0, -22.0),
            (14.0, -22.0),
            (14.0, -30.0),
            (-6.0, -30.0),
            (-6.0, -22.0),
            (0.0, -22.0),
        ],
        Concrete,
    );
    d.earth(&[
        (-24.0, -8.0),
        (0.0, -8.0),
        (0.0, -22.0),
        (-6.0, -22.0),
        (-6.0, -30.0),
        (14.0, -30.0),
        (14.0, -22.0),
        (8.0, -22.0),
        (8.0, -14.0),
        (40.0, -14.0),
        (40.0, -36.0),
        (-24.0, -36.0),
    ]);
    d.line(&[(-24.0, -8.0), (0.0, -8.0)], Medium);
    d.line(&[(8.0, -13.8), (40.0, -13.8)], Hidden);
    d.rebar(-1.0, -27.0, 0.6);
    d.rebar(9.0, -27.0, 0.6);
    d.lumber(0.0, 0.0, 5.5, 1.5);
    d.line(&[(2.75, 2.2), (2.75, -7.0), (4.5, -7.0)], Hidden);
    d.lumber(0.0, 1.5, 1.5, 10.75);
    d.board(1.5, 1.5, 40.0, 10.75);
    d.sheet(0.0, 10.75, 40.0, 11.5);
    d.lumber(0.0, 11.5, 5.5, 13.0);
    wall_2x6(&mut d, 13.0, 34.0, 13.0);
    d.sheet(-0.5, -0.5, 0.0, 13.0);
    siding(&mut d, -0.6, 0.0, 13.0);
    d.brk((-3.0, 34.0), (8.0, 34.0));
    d.brk((40.0, 13.0), (40.0, 0.0));
    d.note((1.0, 0.75), "2x6 P.T. SILL W/ 1/2\" A.B. @ 6'-0\" O.C.");
    d.note((0.75, 6.0), "2x10 RIM JOIST");
    d.note((24.0, 6.0), "2x10 FLOOR JOISTS @ 16\" O.C.");
    d.note((30.0, 11.1), "3/4\" T&G SUBFLOOR, GLUED & NAILED");
    d.note((4.0, -12.0), "8\" CONC. STEM WALL");
    d.note((12.0, -26.0), "20\"x8\" CONC. FOOTING W/ (2) #4 CONT.");
    d.note((30.0, -13.8), "6 MIL VAPOR RETARDER OVER CRAWLSPACE");
    d.note((-14.0, -8.0), "FINISH GRADE");
    d.note((3.0, 24.0), "2x6 STUDS @ 16\" O.C. W/ R-21 BATTS");
    d
}

fn basement_wall() -> D {
    let mut d = D::new();
    d.cut(
        &[
            (0.0, 12.0),
            (8.0, 12.0),
            (8.0, -24.0),
            (16.0, -24.0),
            (16.0, -34.0),
            (-8.0, -34.0),
            (-8.0, -24.0),
            (0.0, -24.0),
        ],
        Concrete,
    );
    d.cut(
        &[(8.0, -20.0), (40.0, -20.0), (40.0, -24.0), (8.0, -24.0)],
        Concrete,
    );
    d.region(
        &[(16.0, -28.0), (40.0, -28.0), (40.0, -24.0), (16.0, -24.0)],
        Gravel,
    );
    d.cut_rect(-2.0, -20.0, 0.0, 12.0, RigidInsulation);
    d.region(
        &[
            (-16.0, -34.0),
            (-8.0, -34.0),
            (-8.0, -24.0),
            (0.0, -24.0),
            (0.0, -20.0),
            (-2.0, -20.0),
            (-2.0, -10.0),
            (-16.0, -10.0),
        ],
        Gravel,
    );
    d.earth(&[
        (-30.0, 6.0),
        (-2.0, 6.0),
        (-2.0, -10.0),
        (-16.0, -10.0),
        (-16.0, -34.0),
        (16.0, -34.0),
        (16.0, -28.0),
        (40.0, -28.0),
        (40.0, -42.0),
        (-30.0, -42.0),
    ]);
    d.line(&[(-30.0, 6.0), (-2.0, 6.0)], Medium);
    d.line(&[(-0.15, -24.0), (-0.15, 12.0)], Medium);
    d.circle(-12.0, -30.0, 2.0, Medium);
    d.line(&[(-16.5, -10.0), (-16.5, -34.5), (-7.5, -34.5)], Hidden);
    d.line(&[(4.0, 10.0), (4.0, -31.0), (11.0, -31.0)], Hidden);
    d.rebar(-4.0, -31.0, 0.7);
    d.rebar(12.0, -31.0, 0.7);
    d.line(&[(16.0, -24.3), (40.0, -24.3)], Hidden);
    d.brk((-4.0, 12.0), (10.0, 12.0));
    d.brk((40.0, -18.0), (40.0, -30.0));
    d.note(
        (4.0, 2.0),
        "8\" CONC. FOUNDATION WALL W/ #5 @ 16\" O.C. VERT.",
    );
    d.note((-0.15, -5.0), "DAMPPROOFING");
    d.note((-1.0, 0.0), "2\" XPS RIGID INSULATION");
    d.note(
        (-12.0, -30.0),
        "4\" PERF. DRAIN IN GRAVEL, FILTER FABRIC WRAP",
    );
    d.note((12.0, -31.0), "CONC. FOOTING W/ (2) #5 CONT.");
    d.note((30.0, -22.0), "4\" CONC. SLAB");
    d.note((34.0, -24.3), "6 MIL VAPOR RETARDER ON 4\" GRAVEL");
    d.note((-20.0, 6.0), "FINISH GRADE");
    d
}

fn spread_footing() -> D {
    let mut d = D::new();
    // A 4'-0" square footing 12" deep, 2'-0" below the slab, and a steel column on it.
    d.cut(
        &[(-24.0, -36.0), (24.0, -36.0), (24.0, -24.0), (-24.0, -24.0)],
        Concrete,
    );
    d.cut(
        &[(-6.0, -24.0), (6.0, -24.0), (6.0, -4.0), (-6.0, -4.0)],
        Concrete,
    );
    d.cut(
        &[(-36.0, 0.0), (-6.5, 0.0), (-6.5, -4.0), (-36.0, -4.0)],
        Concrete,
    );
    d.cut(
        &[(6.5, 0.0), (36.0, 0.0), (36.0, -4.0), (6.5, -4.0)],
        Concrete,
    );
    d.region(
        &[(-36.0, -4.0), (-6.0, -4.0), (-6.0, -8.0), (-36.0, -8.0)],
        Gravel,
    );
    d.region(
        &[(6.0, -4.0), (36.0, -4.0), (36.0, -8.0), (6.0, -8.0)],
        Gravel,
    );
    d.earth(&[
        (-36.0, -8.0),
        (-6.0, -8.0),
        (-6.0, -24.0),
        (-24.0, -24.0),
        (-24.0, -36.0),
        (24.0, -36.0),
        (24.0, -24.0),
        (6.0, -24.0),
        (6.0, -8.0),
        (36.0, -8.0),
        (36.0, -44.0),
        (-36.0, -44.0),
    ]);
    // Base plate on grout, the column, anchor rods.
    d.cut_rect(-5.0, -3.5, 5.0, -2.5, Steel);
    d.region_rect(-5.0, -4.0, 5.0, -3.5, Sand);
    d.rect(-2.0, -2.5, 2.0, 18.0, Wide);
    d.line(&[(-2.0, -2.5), (2.0, -2.5)], Wide);
    for x in [-3.5, 3.5] {
        d.line(&[(x, -1.5), (x, -20.0), (x + 2.0, -20.0)], Hidden);
    }
    for x in [-18.0, -9.0, 0.0, 9.0, 18.0] {
        d.rebar(x, -33.0, 0.7);
    }
    d.brk((-4.0, 18.0), (4.0, 18.0));
    d.brk((-36.0, 1.0), (-36.0, -9.0));
    d.brk((36.0, 1.0), (36.0, -9.0));
    d.note((1.0, 10.0), "HSS 4x4 STEEL COLUMN");
    d.note((4.0, -3.0), "3/4\" BASE PLATE ON NON-SHRINK GROUT");
    d.note((-3.5, -12.0), "(4) 3/4\" ANCHOR RODS");
    d.note((-6.25, -2.0), "1/2\" ISOLATION JOINT");
    d.note(
        (20.0, -30.0),
        "4'-0\"x4'-0\"x12\" CONC. FOOTING W/ #5 @ 9\" E.W.",
    );
    d.note((-30.0, -2.0), "4\" CONC. SLAB ON 4\" GRAVEL");
    d
}

// ------------------------------------------------------------------------------------------
// Walls
// ------------------------------------------------------------------------------------------

fn wall_plan() -> D {
    let mut d = D::new();
    // Interior at the bottom (y 0), exterior up.
    d.gyp(-6.0, 0.0, 30.0, 0.5);
    for x in [0.0, 16.0] {
        d.lumber(x, 0.5, x + 1.5, 6.0);
    }
    d.batt((-6.0, 3.25), (0.0, 3.25), 5.5);
    d.batt((1.5, 3.25), (16.0, 3.25), 5.5);
    d.batt((17.5, 3.25), (30.0, 3.25), 5.5);
    d.sheet(-6.0, 6.0, 30.0, 6.4375);
    d.line(&[(-6.0, 6.55), (30.0, 6.55)], Hidden);
    d.rect(-6.0, 6.6, 30.0, 7.2, Medium);
    d.brk((-6.0, -1.0), (-6.0, 8.0));
    d.brk((30.0, -1.0), (30.0, 8.0));
    d.note((8.0, 0.25), "1/2\" GYP. BD., TAPE & FINISH");
    d.note((0.75, 2.0), "2x6 STUDS @ 16\" O.C.");
    d.note((9.0, 4.0), "R-21 BATT INSULATION");
    d.note((22.0, 6.2), "7/16\" OSB SHEATHING");
    d.note((12.0, 6.55), "WEATHER-RESISTIVE BARRIER");
    d.note((25.0, 6.9), "FIBER CEMENT LAP SIDING");
    d
}

fn brick_base() -> D {
    let mut d = D::new();
    d.cut(
        &[
            (-6.0, -4.0),
            (0.0, -4.0),
            (0.0, 0.0),
            (6.0, 0.0),
            (6.0, -24.0),
            (-6.0, -24.0),
        ],
        Concrete,
    );
    d.earth(&[(-20.0, -8.0), (-6.0, -8.0), (-6.0, -24.0), (-20.0, -24.0)]);
    d.line(&[(-20.0, -8.0), (-6.0, -8.0)], Medium);
    d.lumber(0.0, 0.0, 5.5, 1.5);
    wall_2x6(&mut d, 1.5, 32.0, 32.0);
    d.sheet(-0.5, -0.5, 0.0, 1.5);
    d.line(&[(-0.6, -0.5), (-0.6, 32.0)], Hidden);
    // Brick in courses (2 2/3" each) on the flashing, 1" air space behind.
    let (bx0, bx1, by0) = (-5.1, -1.5, -3.6);
    d.region(&[(bx0, by0), (bx1, by0), (bx1, 32.0), (bx0, 32.0)], Masonry);
    d.rect(bx0, by0, bx1, 32.0, Wide);
    let mut y = by0 + 2.667;
    while y < 32.0 {
        d.line(&[(bx0, y), (bx1, y)], Thin);
        y += 2.667;
    }
    d.flashing(&[(-0.6, 8.0), (-0.6, -3.7), (-5.6, -3.7), (-6.1, -4.3)]);
    for y in [7.0, 23.0] {
        d.line(&[(-0.6, y), (-3.3, y), (-3.3, y + 0.8)], Medium);
    }
    d.region_rect(-1.5, -3.6, -0.6, 2.0, Sand);
    d.brk((-7.0, 32.0), (7.0, 32.0));
    d.brk((6.0, -24.0), (-6.0, -24.0));
    d.note((-3.3, 20.0), "BRICK VENEER");
    d.note((-1.0, 14.0), "1\" AIR SPACE");
    d.note(
        (-2.0, 23.0),
        "GALV. CORRUGATED TIES @ 16\" O.C. VERT., 24\" O.C. HORIZ.",
    );
    d.note((-3.3, -2.3), "WEEP VENTS @ 24\" O.C. AT FIRST COURSE");
    d.note(
        (-5.6, -3.7),
        "THROUGH-WALL FLASHING W/ DRIP, LAP BEHIND WRB",
    );
    d.note((-1.0, 0.5), "MORTAR NET AT CAVITY BASE");
    d.note((2.0, 0.75), "2x6 P.T. SILL PLATE");
    d.note((3.0, -10.0), "CONC. FOUNDATION W/ BRICK LEDGE");
    d.note((3.0, 18.0), "2x6 STUDS W/ R-21 BATTS");
    d
}

fn cmu_slab() -> D {
    let mut d = D::new();
    d.cut(
        &[(-4.0, -16.0), (12.0, -16.0), (12.0, -6.0), (-4.0, -6.0)],
        Concrete,
    );
    d.region(
        &[(0.0, -6.0), (7.625, -6.0), (7.625, 36.0), (0.0, 36.0)],
        CrossHatch,
    );
    d.rect(0.0, -6.0, 7.625, 36.0, Wide);
    let mut y = -6.0 + 8.0;
    while y < 36.0 {
        d.line(&[(0.0, y), (7.625, y)], Thin);
        y += 8.0;
    }
    d.line(&[(3.8, 34.0), (3.8, -13.0), (8.0, -13.0)], Hidden);
    d.cut(
        &[(8.1, 0.0), (40.0, 0.0), (40.0, -4.0), (8.1, -4.0)],
        Concrete,
    );
    d.region(
        &[(12.0, -8.0), (40.0, -8.0), (40.0, -4.0), (12.0, -4.0)],
        Gravel,
    );
    d.region(
        &[(7.625, -6.0), (12.0, -6.0), (12.0, -4.0), (7.625, -4.0)],
        Gravel,
    );
    d.cut_rect(7.625, 0.0, 9.625, 36.0, RigidInsulation);
    d.gyp(9.625, 0.0, 10.125, 36.0);
    d.earth(&[
        (-18.0, -2.0),
        (0.0, -2.0),
        (0.0, -6.0),
        (-4.0, -6.0),
        (-4.0, -16.0),
        (12.0, -16.0),
        (12.0, -8.0),
        (40.0, -8.0),
        (40.0, -22.0),
        (-18.0, -22.0),
    ]);
    d.line(&[(-18.0, -2.0), (0.0, -2.0)], Medium);
    d.rebar(0.0, -13.0, 0.6);
    d.rebar(8.0, -13.0, 0.6);
    d.brk((-2.0, 36.0), (12.0, 36.0));
    d.brk((40.0, 1.0), (40.0, -9.0));
    d.note((3.8, 24.0), "8\" CMU, #5 @ 32\" O.C. IN GROUTED CELLS");
    d.note((8.6, 20.0), "2\" POLYISO RIGID INSUL. W/ Z-FURRING");
    d.note((9.9, 28.0), "1/2\" GYP. BD.");
    d.note((7.9, -1.0), "1/2\" ISOLATION JOINT");
    d.note((30.0, -2.0), "4\" CONC. SLAB ON VAPOR RETARDER & GRAVEL");
    d.note((4.0, -11.0), "16\"x10\" CONC. FOOTING W/ (2) #4 CONT.");
    d.note((-12.0, -2.0), "FINISH GRADE");
    d
}

// ------------------------------------------------------------------------------------------
// Openings
// ------------------------------------------------------------------------------------------

/// A window's glass (an insulated unit) between `y0` and `y1`, at x 2.0 to 2.75.
fn glass(d: &mut D, y0: f64, y1: f64) {
    d.line(&[(2.0, y0), (2.0, y1)], Medium);
    d.line(&[(2.75, y0), (2.75, y1)], Medium);
}

fn window_head() -> D {
    let mut d = D::new();
    // Header: two 2x10s with rigid insulation; double top plate above.
    d.lumber(0.0, 8.0, 1.5, 17.25);
    d.lumber(1.5, 8.0, 3.0, 17.25);
    d.cut_rect(3.0, 8.0, 5.5, 17.25, RigidInsulation);
    d.lumber(0.0, 17.25, 5.5, 18.75);
    d.lumber(0.0, 18.75, 5.5, 20.25);
    d.sheet(-0.5, 8.0, 0.0, 22.0);
    d.gyp(5.5, 8.0, 6.0, 22.0);
    // The window: frame head, sash top rail and glass.
    d.poly(
        &[(0.25, 6.4), (4.75, 6.4), (4.75, 7.75), (0.25, 7.75)],
        Wide,
    );
    d.region_rect(0.25, 7.75, 4.75, 8.0, Sand);
    d.rect(1.25, 3.4, 3.5, 6.4, Medium);
    glass(&mut d, -4.0, 3.9);
    d.rect(-0.6, 7.75, -0.5, 10.0, Medium);
    // Exterior trim, drip cap and the WRB lapped over it.
    d.lumber(-1.6, 7.25, -0.6, 10.75);
    d.flashing(&[(-0.5, 12.5), (-0.5, 10.95), (-1.9, 10.95), (-1.9, 10.3)]);
    d.line(&[(-0.6, 22.0), (-0.6, 11.1)], Hidden);
    siding(&mut d, -0.6, 11.2, 22.0);
    // Interior: extension jamb and casing.
    d.board(4.75, 6.4, 6.0, 7.75);
    d.lumber(6.0, 5.9, 6.75, 9.4);
    d.brk((-3.0, 22.0), (8.0, 22.0));
    d.brk((-1.0, -4.0), (5.0, -4.0));
    d.note((1.5, 13.0), "(2) 2x10 HEADER W/ 2\" RIGID INSUL.");
    d.note((2.75, 19.5), "DOUBLE 2x6 TOP PLATE");
    d.note((-1.2, 10.95), "METAL DRIP CAP");
    d.note((-0.6, 16.0), "WRB LAPPED OVER DRIP CAP FLANGE");
    d.note((-0.55, 9.0), "WINDOW FLANGE, SEALED & TAPED TO SHEATHING");
    d.note((-1.1, 8.0), "5/4x4 EXTERIOR TRIM");
    d.note((3.0, 7.9), "LOW-EXPANSION FOAM, SHIM AS REQ'D");
    d.note((2.4, 0.0), "INSULATED GLASS UNIT");
    d.note((5.4, 7.0), "WOOD EXTENSION JAMB");
    d.note((6.4, 8.5), "INTERIOR CASING");
    d
}

fn window_sill() -> D {
    let mut d = D::new();
    d.lumber(0.0, -1.5, 5.5, 0.0);
    d.board(0.0, -12.0, 1.5, -1.5);
    d.sheet(-0.5, -12.0, 0.0, 0.0);
    d.gyp(5.5, -12.0, 6.0, 1.2);
    d.batt((3.5, -12.0), (3.5, -1.5), 4.0);
    d.flashing(&[(4.6, 1.0), (4.6, 0.12), (-0.5, 0.12), (-0.5, -2.0)]);
    d.poly(
        &[
            (0.25, 0.4),
            (4.75, 0.4),
            (4.75, 1.9),
            (2.4, 1.9),
            (0.25, 1.1),
        ],
        Wide,
    );
    d.rect(1.25, 1.9, 3.5, 4.4, Medium);
    glass(&mut d, 3.6, 12.0);
    // Sloped exterior sill with a drip, trim below.
    d.poly(
        &[(-0.6, 0.35), (-2.4, -0.2), (-2.4, -0.9), (-0.6, -0.5)],
        Medium,
    );
    d.circle(-0.2, 0.9, 0.3, Thin);
    d.lumber(-1.6, -4.5, -0.6, -0.9);
    d.line(&[(-0.6, -2.2), (-0.6, -12.0)], Hidden);
    siding(&mut d, -0.6, -12.0, -4.6);
    // Stool and apron.
    d.lumber(4.75, 1.2, 7.5, 1.95);
    d.lumber(6.0, -2.3, 6.75, 1.2);
    d.brk((-3.0, -12.0), (8.0, -12.0));
    d.brk((-1.0, 12.0), (5.0, 12.0));
    d.note((2.75, -0.75), "SINGLE 2x6 ROUGH SILL");
    d.note((2.0, 0.12), "SILL PAN FLASHING, SLOPED TO EXTERIOR");
    d.note((4.6, 0.8), "BACK DAM");
    d.note(
        (1.5, 1.2),
        "WINDOW SILL, SHIM & SEAL (LEAVE EXTERIOR EDGE OPEN)",
    );
    d.note((-1.5, -0.2), "SLOPED EXTERIOR SILL W/ DRIP");
    d.note((-0.2, 0.9), "SEALANT & BACKER ROD");
    d.note((6.5, 1.6), "WOOD STOOL");
    d.note((6.4, -1.0), "APRON");
    d.note((2.4, 8.0), "INSULATED GLASS UNIT");
    d
}

fn window_jamb() -> D {
    let mut d = D::new();
    // Plan: interior at the bottom, exterior up; the opening to the right of x 0.
    d.gyp(-14.0, 0.0, 0.25, 0.5);
    d.lumber(-1.5, 0.5, 0.0, 6.0);
    d.lumber(-3.0, 0.5, -1.5, 6.0);
    d.batt((-14.0, 3.25), (-3.0, 3.25), 5.5);
    d.sheet(-14.0, 6.0, 0.0, 6.5);
    d.line(&[(-14.0, 6.6), (-3.2, 6.6)], Hidden);
    d.rect(-14.0, 6.6, -3.25, 7.2, Medium);
    // Frame, sash stile and glass (running across the opening).
    d.poly(&[(0.25, 1.5), (1.6, 1.5), (1.6, 5.8), (0.25, 5.8)], Wide);
    d.region_rect(0.0, 1.5, 0.25, 5.8, Sand);
    d.rect(1.6, 2.4, 3.6, 4.6, Medium);
    d.line(&[(3.6, 3.1), (14.0, 3.1)], Medium);
    d.line(&[(3.6, 3.85), (14.0, 3.85)], Medium);
    d.rect(-1.25, 6.5, 0.25, 6.6, Medium);
    d.lumber(-3.25, 6.6, 0.25, 7.6);
    d.circle(0.5, 6.9, 0.3, Thin);
    d.board(0.25, 0.5, 1.6, 1.5);
    d.lumber(-2.5, -0.75, 0.9, 0.0);
    d.brk((-14.0, -1.0), (-14.0, 8.0));
    d.brk((14.0, 2.0), (14.0, 5.0));
    d.note((-2.25, 3.0), "KING STUD");
    d.note((-0.75, 2.0), "JACK STUD");
    d.note((0.9, 5.5), "WINDOW FRAME");
    d.note((-0.5, 6.55), "FLANGE TAPED TO SHEATHING");
    d.note((-1.5, 7.1), "5/4x4 EXTERIOR TRIM");
    d.note((0.5, 6.9), "SEALANT & BACKER ROD");
    d.note((0.1, 3.0), "LOW-EXPANSION FOAM");
    d.note((-0.8, -0.4), "INTERIOR CASING");
    d.note((9.0, 3.5), "INSULATED GLASS UNIT");
    d
}

fn door_threshold() -> D {
    let mut d = D::new();
    d.cut(
        &[(0.0, 0.0), (14.0, 0.0), (14.0, -4.0), (0.0, -4.0)],
        Concrete,
    );
    d.cut(
        &[(-14.0, -1.5), (-0.25, -1.25), (-0.25, -5.0), (-14.0, -5.0)],
        Concrete,
    );
    d.region_rect(-0.25, -5.0, 0.0, -1.25, Sand);
    // The saddle: a low aluminum profile set in sealant.
    d.cut(
        &[
            (-2.75, -1.3),
            (-2.75, -1.0),
            (-0.6, 0.55),
            (1.6, 0.55),
            (3.25, 0.1),
            (3.25, 0.0),
            (-0.25, 0.0),
            (-0.25, -1.3),
        ],
        Steel,
    );
    d.region_rect(-0.25, -0.1, 3.25, 0.0, Solid);
    // The door leaf and its bottom sweep.
    d.rect(0.25, 1.0, 2.0, 8.0, Medium);
    d.line(&[(0.25, 1.0), (0.25, 0.6), (2.0, 0.6), (2.0, 1.0)], Medium);
    d.line(&[(0.4, 0.6), (0.2, 0.55)], Thin);
    d.brk((-0.5, 8.0), (2.75, 8.0));
    d.brk((14.0, 1.0), (14.0, -5.0));
    d.brk((-14.0, -1.0), (-14.0, -6.0));
    d.note(
        (0.3, 0.3),
        "ALUMINUM SADDLE THRESHOLD SET IN SEALANT, 1/2\" MAX. HEIGHT",
    );
    d.note((1.2, 0.7), "DOOR BOTTOM SWEEP");
    d.note((1.1, 5.0), "1-3/4\" INSULATED DOOR");
    d.note((8.0, -2.0), "CONC. SLAB");
    d.note((-8.0, -2.5), "CONC. WALK, SLOPE 1/4\" PER FT. AWAY");
    d.note((-0.1, -3.0), "1/2\" EXPANSION JOINT W/ SEALANT");
    d
}

fn door_jamb() -> D {
    let mut d = D::new();
    // Plan: a 2x4 partition running left, the door opening to the right of x 0.
    d.gyp(-12.0, 0.0, 0.0, 0.5);
    d.gyp(-12.0, 4.0, 0.0, 4.5);
    d.lumber(-1.5, 0.5, 0.0, 4.0);
    d.lumber(-3.0, 0.5, -1.5, 4.0);
    d.line(&[(-12.0, 0.5), (-3.0, 0.5)], Beyond);
    d.line(&[(-12.0, 4.0), (-3.0, 4.0)], Beyond);
    // Jamb, stop, shims and casings.
    d.lumber(0.0, 0.0, 0.6875, 4.5625);
    d.region_rect(-0.1, 0.5, 0.0, 4.0, Sand);
    d.lumber(0.6875, 2.0, 1.1875, 3.375);
    d.lumber(-2.5, -0.6875, 0.25, 0.0);
    d.lumber(-2.5, 4.5625, 0.25, 5.25);
    // The door, 1 3/8" thick, closed against the stop.
    d.rect(0.8, 0.6, 12.0, 1.975, Medium);
    d.brk((-12.0, -1.0), (-12.0, 5.5));
    d.brk((12.0, 0.0), (12.0, 2.5));
    d.note((-2.25, 2.0), "DOUBLE 2x4 STUD AT OPENING");
    d.note((0.35, 3.5), "11/16\" WOOD JAMB");
    d.note((0.95, 2.7), "WOOD STOP");
    d.note((-0.05, 1.0), "SHIMS");
    d.note((-1.0, -0.35), "CASING");
    d.note((-1.0, 4.9), "CASING");
    d.note((6.0, 1.3), "1-3/8\" SOLID CORE DOOR");
    d.note((-8.0, 0.25), "1/2\" GYP. BD. EACH SIDE");
    d
}

// ------------------------------------------------------------------------------------------
// Roofs
// ------------------------------------------------------------------------------------------

fn eave() -> D {
    let mut d = D::new();
    // A 6:12 top chord: lower edge y = 4 + x/2, 3 1/2" deep (3.91" plumb).
    let lo = |x: f64| 4.0 + x * 0.5;
    let hi = |x: f64| lo(x) + 3.91;
    let (x0, x1) = (-18.0, 40.0);
    d.lumber(0.0, 0.0, 5.5, 1.5);
    d.lumber(0.0, 1.5, 5.5, 3.0);
    d.board(0.0, 3.0, x1, 6.5);
    d.poly(
        &[(x0, lo(x0)), (x1, lo(x1)), (x1, hi(x1)), (x0, hi(x0))],
        Medium,
    );
    // Sheathing and shingles on top, drip edge at the eave.
    let s = |x: f64| hi(x) + 0.56;
    d.poly(
        &[
            (x0 - 0.75, hi(x0 - 0.75)),
            (x1, hi(x1)),
            (x1, s(x1)),
            (x0 - 0.75, s(x0 - 0.75)),
        ],
        Medium,
    );
    d.line(&[(x0 - 1.0, s(x0 - 1.0) + 0.25), (x1, s(x1) + 0.4)], Wide);
    d.flashing(&[
        (x0 + 2.0, s(x0 + 2.0)),
        (x0 - 1.0, s(x0 - 1.0)),
        (x0 - 1.0, s(x0 - 1.0) - 1.4),
    ]);
    // Fascia, soffit and gutter.
    d.lumber(x0 - 0.75, lo(x0) - 1.5, x0, hi(x0));
    d.rect(x0, lo(x0) - 1.5, -0.6, lo(x0) - 1.1, Medium);
    d.line(&[(-8.0, lo(x0) - 1.1), (-6.0, lo(x0) - 1.1)], Thin);
    d.line(
        &[
            (x0 - 0.8, hi(x0) - 0.3),
            (x0 - 0.8, hi(x0) - 5.0),
            (x0 - 5.0, hi(x0) - 5.0),
            (x0 - 5.6, hi(x0) - 1.0),
            (x0 - 4.9, hi(x0) - 0.3),
        ],
        Medium,
    );
    // Baffle and attic insulation.
    d.line(&[(1.0, lo(1.0) - 1.5), (27.0, lo(27.0) - 1.5)], Medium);
    d.region(&[(8.0, 6.5), (40.0, 6.5), (40.0, 16.0), (27.0, 16.0)], Sand);
    d.gyp(5.5, 2.5, x1, 3.0);
    d.line(&[(1.0, 1.5), (1.0, 5.5), (3.0, 7.0)], Thin);
    wall_2x6(&mut d, -18.0, 0.0, -18.0);
    d.sheet(-0.5, 0.0, 0.0, lo(x0) + 11.0);
    d.brk((x1, 0.0), (x1, 26.0));
    d.brk((-3.0, -18.0), (8.0, -18.0));
    d.note(
        (20.0, 21.0),
        "ASPHALT SHINGLES ON UNDERLAYMENT, ICE BARRIER 24\" INSIDE WALL",
    );
    d.note((30.0, 23.5), "7/16\" OSB ROOF SHEATHING");
    d.note((25.0, 18.0), "ENGINEERED TRUSSES @ 24\" O.C.");
    d.note((x0 - 0.5, hi(x0) + 0.6), "METAL DRIP EDGE");
    d.note((x0 - 3.0, hi(x0) - 5.0), "5\" K-STYLE GUTTER");
    d.note((x0 - 0.4, lo(x0) - 0.5), "1x8 FASCIA ON 2x SUB-FASCIA");
    d.note((-7.0, lo(x0) - 1.3), "VENTED SOFFIT");
    d.note((14.0, lo(14.0) - 1.5), "VENT BAFFLE, 1-1/2\" CLR. AIRWAY");
    d.note((34.0, 12.0), "R-49 BLOWN-IN INSULATION");
    d.note((1.0, 3.5), "HURRICANE TIE EACH TRUSS");
    d.note((20.0, 2.75), "5/8\" GYP. BD. CEILING");
    d
}

fn rake() -> D {
    let mut d = D::new();
    // Section across the rake: roof sheathing level, outlookers over the gable wall.
    d.lumber(0.0, -6.5, 5.5, -5.0);
    d.lumber(0.0, -5.0, 5.5, -3.5);
    d.board(-12.0, -3.5, 24.0, 0.0);
    d.sheet(-12.75, 0.0, 24.0, 0.5);
    d.line(&[(-13.5, 0.75), (24.0, 0.75)], Wide);
    d.lumber(-12.75, -5.5, -12.0, 0.0);
    d.flashing(&[(-10.0, 0.55), (-13.0, 0.55), (-13.0, -1.0)]);
    d.rect(-12.0, -3.9, -0.6, -3.5, Medium);
    wall_2x6(&mut d, -24.0, -6.5, -24.0);
    d.sheet(-0.5, -6.5, 0.0, -3.5);
    d.gyp(5.5, -6.5, 24.0, -6.0);
    d.brk((24.0, -7.0), (24.0, 1.5));
    d.brk((-3.0, -24.0), (8.0, -24.0));
    d.note((8.0, 0.8), "ASPHALT SHINGLES, 3/4\" OVERHANG AT RAKE");
    d.note((16.0, 0.25), "7/16\" OSB ROOF SHEATHING");
    d.note((-6.0, -1.75), "2x4 OUTLOOKERS @ 24\" O.C. (LADDER FRAME)");
    d.note((-12.4, -3.0), "1x6 RAKE FASCIA");
    d.note((-12.5, 0.55), "METAL DRIP EDGE");
    d.note((-5.0, -3.7), "RAKE SOFFIT");
    d.note((2.75, -5.0), "GABLE END TRUSS ON DOUBLE TOP PLATE");
    d
}

fn parapet() -> D {
    let mut d = D::new();
    // A 2x6 parapet 24" above the roof deck; the roof to the right.
    d.lumber(0.0, -12.0, 5.5, -10.5);
    d.batt((2.75, -10.5), (2.75, 22.0), 5.5);
    d.line(&[(0.0, -10.5), (0.0, 22.0)], Beyond);
    d.line(&[(5.5, -10.5), (5.5, 22.0)], Beyond);
    d.sheet(-0.5, -12.0, 0.0, 23.5);
    d.sheet(5.5, 0.0, 6.0, 23.5);
    d.lumber(0.0, 22.0, 5.5, 23.5);
    d.poly(
        &[(-0.5, 23.5), (6.0, 23.5), (6.0, 24.5), (-0.5, 25.0)],
        Medium,
    );
    // The coping: over the top, down both faces, drips out.
    d.flashing(&[
        (-1.5, 20.0),
        (-1.25, 20.4),
        (-1.25, 25.3),
        (6.9, 24.6),
        (6.9, 20.6),
        (7.2, 20.2),
    ]);
    // Roof: deck on joists, tapered insulation, cover board, membrane up the parapet.
    d.board(5.5, -10.5, 40.0, -1.25);
    d.sheet(6.0, -1.25, 40.0, -0.5);
    d.cut(
        &[(6.5, -0.5), (40.0, -0.5), (40.0, 4.0), (6.5, 3.0)],
        RigidInsulation,
    );
    d.poly(&[(6.5, 3.0), (40.0, 4.0), (40.0, 4.5), (6.5, 3.5)], Medium);
    d.poly(&[(6.5, 3.5), (6.5, 7.5), (10.5, 3.62)], Medium);
    d.line(
        &[
            (40.0, 4.6),
            (10.5, 3.72),
            (6.1, 8.0),
            (6.1, 23.6),
            (6.0, 23.7),
        ],
        Wide,
    );
    d.line(&[(-0.6, -12.0), (-0.6, 20.0)], Hidden);
    siding(&mut d, -0.6, -12.0, 19.5);
    d.brk((-3.0, -12.0), (8.0, -12.0));
    d.brk((40.0, -11.0), (40.0, 5.0));
    d.note((3.0, 25.0), "PREFINISHED METAL COPING ON CONT. CLEAT");
    d.note((3.0, 24.3), "SLOPED WOOD NAILER");
    d.note((6.1, 16.0), "MEMBRANE ROOFING, FULLY ADHERED UP PARAPET");
    d.note((8.0, 4.8), "WOOD CANT STRIP");
    d.note((30.0, 2.0), "TAPERED POLYISO INSULATION, 1/4\" PER FT.");
    d.note((34.0, 4.3), "1/2\" COVER BOARD");
    d.note((28.0, -0.9), "3/4\" PLYWOOD ROOF DECK");
    d.note((22.0, -6.0), "ROOF JOISTS PER STRUCT.");
    d.note((2.75, 10.0), "2x6 PARAPET FRAMING W/ BATTS");
    d.note((-1.0, 6.0), "SIDING OVER WRB");
    d
}

fn ridge_vent() -> D {
    let mut d = D::new();
    // A 6:12 ridge at x 0: top chords' upper edges y = -|x|/2.
    let up = |x: f64| -x.abs() * 0.5;
    for side in [-1.0_f64, 1.0] {
        let (a, b) = (side * 0.75, side * 30.0);
        d.poly(
            &[(a, up(a)), (b, up(b)), (b, up(b) - 3.91), (a, up(a) - 3.91)],
            Medium,
        );
        let (sa, sb) = (side * 1.5, side * 30.0);
        d.poly(
            &[
                (sa, up(sa)),
                (sb, up(sb)),
                (sb, up(sb) + 0.56),
                (sa, up(sa) + 0.56),
            ],
            Medium,
        );
        d.line(&[(side * 9.5, up(9.5) + 0.9), (sb, up(sb) + 0.9)], Wide);
    }
    d.rect(-2.5, -6.0, 2.5, -1.5, Medium);
    d.line(&[(0.0, -1.2), (0.0, -14.0)], Medium);
    // The vent: a low arch over the slot, capped with ridge shingles.
    d.poly(
        &[
            (-9.5, up(9.5) + 0.7),
            (0.0, 1.2),
            (9.5, up(9.5) + 0.7),
            (9.5, up(9.5) + 1.7),
            (0.0, 2.2),
            (-9.5, up(9.5) + 1.7),
        ],
        Wide,
    );
    d.region(
        &[
            (-9.5, up(9.5) + 0.7),
            (0.0, 1.2),
            (9.5, up(9.5) + 0.7),
            (0.0, 0.6),
        ],
        Sand,
    );
    d.brk((-30.0, -18.0), (-30.0, -12.0));
    d.brk((30.0, -18.0), (30.0, -12.0));
    d.note(
        (0.0, 2.0),
        "SHINGLE-OVER RIDGE VENT, CAPPED W/ RIDGE SHINGLES",
    );
    d.note(
        (-1.1, 0.2),
        "1-1/2\" SLOT EACH SIDE OF RIDGE, STOP 6\" FROM ENDS",
    );
    d.note((20.0, -9.0), "ASPHALT SHINGLES ON UNDERLAYMENT");
    d.note((-22.0, -10.7), "7/16\" OSB ROOF SHEATHING");
    d.note((1.5, -4.0), "TRUSS GUSSET PLATE");
    d.note((0.0, -12.0), "TRUSS WEB");
    d
}

// ------------------------------------------------------------------------------------------
// Floors & stairs
// ------------------------------------------------------------------------------------------

fn rim_joist() -> D {
    let mut d = D::new();
    wall_2x6(&mut d, -18.0, 0.0, -18.0);
    d.lumber(0.0, 0.0, 5.5, 1.5);
    d.lumber(0.0, 1.5, 5.5, 3.0);
    d.lumber(0.0, 3.0, 1.5, 14.875);
    d.board(1.5, 3.0, 40.0, 14.875);
    d.region(
        &[(1.5, 3.0), (5.0, 3.0), (5.0, 14.875), (1.5, 14.875)],
        Sand,
    );
    d.sheet(0.0, 14.875, 40.0, 15.625);
    d.lumber(0.0, 15.625, 5.5, 17.125);
    wall_2x6(&mut d, 17.125, 34.0, 17.125);
    d.sheet(-0.5, -0.5, 0.0, 17.125);
    d.line(&[(-0.6, -0.5), (-0.6, 17.125)], Hidden);
    siding(&mut d, -0.6, 0.0, 17.125);
    d.gyp(6.0, 2.5, 40.0, 3.0);
    d.brk((-3.0, 34.0), (8.0, 34.0));
    d.brk((-3.0, -18.0), (8.0, -18.0));
    d.brk((40.0, 2.0), (40.0, 16.0));
    d.note((0.75, 9.0), "1-1/8\" RIM BOARD");
    d.note((3.2, 12.0), "CLOSED-CELL SPRAY FOAM AT RIM");
    d.note((24.0, 9.0), "11-7/8\" I-JOISTS @ 16\" O.C.");
    d.note((30.0, 15.3), "3/4\" T&G SUBFLOOR");
    d.note((2.75, 16.4), "2x6 SOLE PLATE");
    d.note((2.75, 2.2), "DOUBLE 2x6 TOP PLATE");
    d.note((-0.25, 10.0), "CONT. SHEATHING & WRB ACROSS RIM");
    d.note((24.0, 2.75), "5/8\" GYP. BD. CEILING");
    d
}

fn stair_tread() -> D {
    let mut d = D::new();
    let (rise, run) = (7.75, 10.0);
    // The floor at y -rise; treads (1 1/16", 1" nosing) with tops at 0, rise, 2 rise; 3/4"
    // risers under each nosing; a cut stringer, 9 1/4" deep below its notches, beyond.
    let t = 1.0625;
    let slope = rise / run;
    let x = |i: usize| i as f64 * run;
    let y = |i: usize| i as f64 * rise;
    let mut stringer = vec![(0.75, -rise), (0.75, -t)];
    for i in 0..3 {
        stringer.push((x(i + 1), y(i) - t));
        if i < 2 {
            stringer.push((x(i + 1), y(i + 1) - t));
        }
    }
    // 9 1/4" square to the pitch, measured plumb.
    let depth = 9.25 * (1.0 + slope * slope).sqrt();
    let bottom = |xx: f64| slope * xx - t - depth;
    let x_floor = (depth + t - rise) / slope;
    stringer.push((x(3), bottom(x(3))));
    stringer.push((x_floor, -rise));
    d.poly(&stringer, Beyond);
    d.line(&[(-8.0, -rise), (x(3), -rise)], Medium);
    // Cut along their length: wood grain, not the X of lumber cut across.
    let mut wood = |x0: f64, y0: f64, x1: f64, y1: f64| {
        d.region_rect(x0, y0, x1, y1, Wood);
        d.rect(x0, y0, x1, y1, Medium);
    };
    wood(0.0, -rise, 0.75, -t);
    for i in 0..3 {
        wood(x(i) - 1.0, y(i) - t, x(i + 1).min(x(3)) + 0.75, y(i));
        if i < 2 {
            wood(x(i + 1), y(i), x(i + 1) + 0.75, y(i + 1) - t);
        }
    }
    for i in 0..3 {
        d.line(
            &[(x(i) + 0.75, y(i) - t - 0.9), (x(i) + 1.65, y(i) - t)],
            Thin,
        );
    }
    d.brk((x(3), y(2) + 1.0), (x(3), bottom(x(3)) - 1.0));
    d.note((x(1) + 5.0, rise - 0.5), "1-1/16\" OAK TREAD W/ 1\" NOSING");
    d.note((x(1) + 0.4, rise / 2.0), "3/4\" RISER");
    d.note((x(1) + 1.2, y(1) - t - 0.4), "GLUE BLOCK");
    d.note(
        (x(2) + 5.0, bottom(x(2) + 5.0) + 2.0),
        "2x12 CUT STRINGER @ 16\" O.C. MAX.",
    );
    d.note((-0.5, -rise / 2.0), "7-3/4\" MAX. RISE");
    d.note((x(2) - 3.0, y(2) - 0.3), "10\" MIN. RUN");
    d.note((-4.0, -rise), "FINISH FLOOR");
    d
}

fn handrail() -> D {
    let mut d = D::new();
    // The wall face at x 0 (the stair to the right); a 1 1/2" rail 1 1/2" clear of it.
    d.gyp(-0.5, -10.0, 0.0, 14.0);
    d.board(-4.0, -10.0, -0.5, 14.0);
    d.lumber(-4.0, -2.0, -0.5, 3.5);
    d.circle(2.25, 4.0, 0.75, Wide);
    d.poly(&[(0.0, -1.0), (0.4, -1.0), (0.4, 1.5), (0.0, 1.5)], Medium);
    d.line(&[(0.4, 0.25), (2.1, 0.25), (2.25, 3.25)], Medium);
    d.line(&[(-3.2, -0.2), (0.0, -0.2)], Hidden);
    d.line(&[(-3.2, 0.9), (0.0, 0.9)], Hidden);
    d.line(&[(0.0, 4.0), (1.5, 4.0)], Thin);
    d.brk((-5.0, 14.0), (1.0, 14.0));
    d.brk((-5.0, -10.0), (1.0, -10.0));
    d.note(
        (2.25, 4.6),
        "1-1/2\" DIA. WOOD HANDRAIL, 34\"-38\" ABOVE NOSINGS",
    );
    d.note((1.2, 0.25), "METAL HANDRAIL BRACKET @ 48\" O.C. MAX.");
    d.note((-1.6, 0.35), "(2) #10 SCREWS INTO BLOCKING");
    d.note((-2.25, 2.5), "2x SOLID BLOCKING");
    d.note((0.75, 4.0), "1-1/2\" MIN. CLEAR TO WALL");
    d.note((-0.25, 10.0), "1/2\" GYP. BD.");
    d
}

fn deck_guard() -> D {
    let mut d = D::new();
    // Deck boards (cut), joists seen, the rim; a 4x4 post on the rim's outside.
    let mut x = 0.0;
    while x < 34.0 {
        d.lumber(x, 0.0, x + 5.5, 1.0);
        x += 5.75;
    }
    d.board(1.5, -9.25, 34.0, 0.0);
    d.lumber(0.0, -9.25, 1.5, 0.0);
    d.lumber(-3.5, -9.0, 0.0, 36.0);
    for y in [-2.5, -7.0] {
        d.line(&[(-4.2, y), (1.8, y)], Hidden);
        d.rect(-4.4, y - 0.5, -3.9, y + 0.5, Medium);
    }
    d.lumber(-3.5, 36.0, 2.0, 37.5);
    d.board(-2.0, 32.0, 1.5, 33.5);
    d.board(-2.0, 3.5, 1.5, 5.0);
    d.board(-1.0, 5.0, 0.5, 32.0);
    d.brk((34.0, -10.0), (34.0, 2.0));
    d.brk((-5.0, -9.0), (2.0, -9.0));
    d.note((16.0, 0.5), "5/4x6 DECK BOARDS W/ 1/4\" GAPS");
    d.note((20.0, -5.0), "2x10 P.T. JOISTS @ 16\" O.C.");
    d.note((0.75, -5.0), "2x10 P.T. RIM JOIST");
    d.note((-1.75, 20.0), "4x4 P.T. POST @ 6'-0\" O.C. MAX.");
    d.note((-4.2, -2.5), "(2) 1/2\" THRU-BOLTS W/ WASHERS");
    d.note((-0.75, 37.0), "2x6 CAP RAIL, 36\" MIN. ABOVE DECK");
    d.note((-0.25, 18.0), "2x2 BALUSTERS, 4\" MAX. OPENING");
    d.note((-0.25, 4.2), "2x4 BOTTOM RAIL, 3-1/2\" MAX. ABOVE DECK");
    d
}

// ------------------------------------------------------------------------------------------
// Interiors
// ------------------------------------------------------------------------------------------

fn base_cabinet() -> D {
    let mut d = D::new();
    // The wall at x 24 (right), the cabinet's front at x 0.
    d.gyp(24.0, -2.0, 24.5, 44.0);
    d.board(24.5, -2.0, 28.0, 44.0);
    d.cut(
        &[(-4.0, -1.0), (30.0, -1.0), (30.0, 0.0), (-4.0, 0.0)],
        Wood,
    );
    d.poly(
        &[
            (3.0, 0.0),
            (23.25, 0.0),
            (23.25, 34.5),
            (0.0, 34.5),
            (0.0, 4.0),
            (3.0, 4.0),
        ],
        Medium,
    );
    d.rect(-0.75, 4.0, 0.0, 28.0, Medium);
    d.rect(-0.75, 28.25, 0.0, 34.25, Medium);
    d.line(&[(0.75, 16.0), (22.5, 16.0)], Hidden);
    d.line(&[(0.75, 28.0), (22.5, 28.0)], Medium);
    d.cut(
        &[(-1.0, 34.5), (24.0, 34.5), (24.0, 35.75), (-1.0, 35.75)],
        Sand,
    );
    d.cut(
        &[(23.25, 35.75), (24.0, 35.75), (24.0, 39.75), (23.25, 39.75)],
        Sand,
    );
    d.circle(-1.2, 31.25, 0.3, Medium);
    d.brk((-4.0, -1.0), (-4.0, 0.0));
    d.brk((22.0, 44.0), (29.0, 44.0));
    d.note((10.0, 35.2), "1-1/4\" QUARTZ COUNTERTOP, 36\" A.F.F.");
    d.note((23.6, 38.0), "4\" BACKSPLASH");
    d.note((-0.4, 16.0), "3/4\" DOOR");
    d.note((-0.4, 31.0), "DRAWER FRONT");
    d.note((12.0, 16.0), "ADJUSTABLE SHELF");
    d.note((1.5, 2.0), "4\"x3\" TOE KICK");
    d.note((12.0, 30.0), "BASE CABINET, 24\" DEEP");
    d.note((24.25, 20.0), "BLOCKING & 1/2\" GYP. BD. AT WALL");
    d.note((10.0, -0.5), "FINISH FLOOR");
    d
}

fn act_ceiling() -> D {
    let mut d = D::new();
    // The partition at x 0..4.5 (studs and gyp both faces), the ceiling to the right.
    d.gyp(0.0, -8.0, 0.5, 20.0);
    d.gyp(4.0, -8.0, 4.5, 20.0);
    d.board(0.5, -8.0, 4.0, 20.0);
    // Wall angle, tile resting on it, main runner.
    d.line(
        &[
            (4.5, 0.0),
            (5.4, 0.0),
            (5.4, 0.08),
            (4.58, 0.08),
            (4.58, 0.9),
            (4.5, 0.9),
        ],
        Wide,
    );
    d.cut(
        &[(4.6, 0.08), (22.0, 0.08), (22.0, 0.705), (4.6, 0.705)],
        Sand,
    );
    d.cut(
        &[(22.5, 0.08), (40.0, 0.08), (40.0, 0.705), (22.5, 0.705)],
        Sand,
    );
    d.line(
        &[
            (21.5, 0.0),
            (23.0, 0.0),
            (23.0, 0.08),
            (22.3, 0.08),
            (22.3, 1.5),
            (22.6, 1.5),
            (22.6, 1.8),
            (21.9, 1.8),
            (21.9, 1.5),
            (22.2, 1.5),
            (22.2, 0.08),
            (21.5, 0.08),
        ],
        Wide,
    );
    d.line(&[(22.25, 1.8), (22.25, 20.0)], Hidden);
    d.line(&[(22.1, 19.0), (22.25, 20.0), (22.4, 19.0)], Thin);
    d.brk((-1.0, 20.0), (5.5, 20.0));
    d.brk((-1.0, -8.0), (5.5, -8.0));
    d.brk((40.0, -1.0), (40.0, 2.0));
    d.note((5.0, 0.04), "7/8\" STEEL WALL ANGLE, SCREW @ 16\" O.C.");
    d.note((14.0, 0.4), "5/8\" x 24\"x24\" ACOUSTICAL CEILING TILE");
    d.note((22.3, 1.0), "15/16\" MAIN RUNNER @ 48\" O.C.");
    d.note((22.25, 12.0), "12 GA. HANGER WIRE @ 48\" O.C. TO STRUCTURE");
    d.note((2.25, 10.0), "PARTITION, 1/2\" GYP. BD. EACH SIDE");
    d
}

fn partition_head() -> D {
    let mut d = D::new();
    // Structure above: a concrete deck at y 0; the partition hangs from its deflection track.
    d.cut(
        &[(-12.0, 0.0), (18.0, 0.0), (18.0, 6.0), (-12.0, 6.0)],
        Concrete,
    );
    d.line(
        &[(-0.1, -2.5), (-0.1, -0.05), (3.725, -0.05), (3.725, -2.5)],
        Wide,
    );
    d.line(&[(0.0, -1.5), (0.0, -18.0)], Medium);
    d.line(&[(3.625, -1.5), (3.625, -18.0)], Medium);
    d.line(&[(0.0, -1.5), (3.625, -1.5)], Thin);
    d.line(&[(1.8, -0.05), (1.8, 1.5)], Hidden);
    d.gyp(-0.625, -18.0, 0.0, -0.75);
    d.gyp(3.625, -18.0, 4.25, -0.75);
    d.region_rect(-0.625, -0.75, 0.0, 0.0, Sand);
    d.region_rect(3.625, -0.75, 4.25, 0.0, Sand);
    d.circle(-0.3, -0.4, 0.25, Thin);
    d.circle(3.95, -0.4, 0.25, Thin);
    d.batt((1.8, -18.0), (1.8, -3.0), 3.4);
    d.brk((-2.0, -18.0), (6.0, -18.0));
    d.brk((-12.0, -1.0), (-12.0, 7.0));
    d.brk((18.0, -1.0), (18.0, 7.0));
    d.note((1.8, -0.5), "SLOTTED DEFLECTION TRACK, 1\" MIN. LEG GAP");
    d.note((1.8, 1.0), "POWDER-ACTUATED FASTENER @ 24\" O.C.");
    d.note(
        (0.0, -10.0),
        "3-5/8\" METAL STUDS @ 16\" O.C., NOT FASTENED TO TRACK",
    );
    d.note((4.0, -8.0), "5/8\" TYPE X GYP. BD., 3/4\" GAP AT TOP");
    d.note((-0.3, -0.4), "FIRE-RATED JOINT SEALANT");
    d.note((1.8, -12.0), "ACOUSTIC BATT INSULATION");
    d.note((10.0, 3.0), "CONC. DECK ABOVE");
    d
}
