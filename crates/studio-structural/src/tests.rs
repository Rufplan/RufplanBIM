//! Fixtures (a 3-story wood box, a podium building, one with a non-stacking wall) and
//! tests for each stage.

use studio_core::structural::{
    FlagKind, LateralKind, MemberKind, SchemeKind, Seismic, SpanDir, DISCLAIMER,
};
use studio_core::units::MM_PER_FT;
use studio_core::{ops, Category, Document, ElementId};
use studio_geom::Pt;

use crate::*;

const FT: f64 = MM_PER_FT;

fn p(x: f64, y: f64) -> Pt {
    Pt::new(x * FT, y * FT)
}

struct Fixture {
    doc: Document,
    levels: Vec<ElementId>,
}

/// `stories` levels 10' apart, each a 60' x 40' box with a floor.
fn shell(stories: usize) -> Fixture {
    let mut doc = Document::new();
    ops::seed_default_project(&mut doc).unwrap();
    let mut levels: Vec<ElementId> = doc.levels().into_iter().map(|l| l.0).collect();
    while levels.len() < stories {
        let e = levels.len() as f64 * 10.0 * FT;
        levels.push(ops::create_level(&mut doc, e).unwrap());
    }
    levels.truncate(stories);
    let ext = ops::first_of(&doc, Category::WallType).unwrap();
    let ft = ops::first_of(&doc, Category::FloorType).unwrap();
    let corners = [p(0.0, 0.0), p(60.0, 0.0), p(60.0, 40.0), p(0.0, 40.0)];
    for &l in &levels {
        for i in 0..4 {
            ops::create_wall(&mut doc, ext, l, corners[i], corners[(i + 1) % 4]).unwrap();
        }
        ops::create_floor(&mut doc, ft, l, corners.to_vec()).unwrap();
    }
    Fixture { doc, levels }
}

fn interior(doc: &Document) -> ElementId {
    doc.of(Category::WallType)
        .find(|e| e.data.name().starts_with("Interior"))
        .unwrap()
        .id
}

fn room(doc: &mut Document, level: ElementId, at: Pt, name: &str) {
    let r = ops::create_room(doc, level, at).unwrap();
    ops::set_property(doc, r, "name", name, 0).unwrap();
}

/// Three residential stories: a corridor along x and demising walls every 20', stacking.
fn wood_box() -> Fixture {
    let mut f = shell(3);
    let it = interior(&f.doc);
    for &l in &f.levels.clone() {
        for y in [18.0, 22.0] {
            ops::create_wall(&mut f.doc, it, l, p(0.0, y), p(60.0, y)).unwrap();
        }
        for x in [20.0, 40.0] {
            ops::create_wall(&mut f.doc, it, l, p(x, 0.0), p(x, 18.0)).unwrap();
            ops::create_wall(&mut f.doc, it, l, p(x, 22.0), p(x, 40.0)).unwrap();
        }
        for (x, y, n) in [
            (10.0, 9.0, "Unit Living"),
            (30.0, 9.0, "Bedroom"),
            (50.0, 30.0, "Kitchen"),
        ] {
            room(&mut f.doc, l, p(x, y), n);
        }
    }
    f
}

/// Retail on an open ground floor under three residential stories.
fn podium_building() -> Fixture {
    let mut f = shell(4);
    let it = interior(&f.doc);
    let levels = f.levels.clone();
    room(&mut f.doc, levels[0], p(30.0, 20.0), "Retail");
    for &l in &levels[1..] {
        ops::create_wall(&mut f.doc, it, l, p(0.0, 20.0), p(60.0, 20.0)).unwrap();
        for x in [15.0, 30.0, 45.0] {
            ops::create_wall(&mut f.doc, it, l, p(x, 0.0), p(x, 20.0)).unwrap();
        }
        room(&mut f.doc, l, p(7.0, 10.0), "Unit Bedroom");
        room(&mut f.doc, l, p(22.0, 10.0), "Unit Living");
    }
    f
}

/// Two stories whose upper cross wall sits 6' off the one below.
fn offset_wall() -> Fixture {
    let mut f = shell(2);
    let it = interior(&f.doc);
    let l = f.levels.clone();
    ops::create_wall(&mut f.doc, it, l[0], p(30.0, 0.0), p(30.0, 40.0)).unwrap();
    ops::create_wall(&mut f.doc, it, l[1], p(36.0, 0.0), p(36.0, 40.0)).unwrap();
    room(&mut f.doc, l[0], p(10.0, 20.0), "Living");
    room(&mut f.doc, l[1], p(10.0, 20.0), "Bedroom");
    f
}

fn features(f: &Fixture) -> Features {
    let model = studio_regen::regenerate(&f.doc);
    extract(&f.doc, &model, &Rules::builtin())
}

#[test]
fn extraction_reads_levels_heights_outline_stacking_and_uses() {
    let f = wood_box();
    let ft = features(&f);
    assert_eq!(ft.story_count, 3);
    assert!(ft
        .levels
        .iter()
        .all(|l| (l.floor_to_floor - 10.0 * FT).abs() < 1.0));
    assert!(
        (ft.total_height - 30.0 * FT).abs() < 1.0,
        "{}",
        ft.total_height / FT
    );
    assert!((ft.footprint_area - 2400.0 * FT * FT).abs() < 1.0);
    assert!((ft.aspect_ratio - 1.5).abs() < 1e-9);
    assert_eq!(ft.stacking, 1.0);
    assert!(ft.levels.iter().all(|l| l.stacking_ratio == 1.0));
    assert_eq!(ft.uses[0], UseKind::Residential);
    // Rooms are 18' deep: the longest clear span.
    assert!(
        (ft.max_span - 18.0 * FT).abs() < 12.0 * 25.4,
        "{}",
        ft.max_span / FT
    );
    assert!(ft.discontinuities.is_empty(), "{:?}", ft.discontinuities);
    // Each level: 4 exterior walls of 200' and 6 interior pieces.
    let l1 = &ft.levels[0];
    assert!((l1.exterior_wall_length - 200.0 * FT).abs() < 1.0);
}

#[test]
fn openings_split_walls_into_solid_segments() {
    let mut f = shell(1);
    let wall = f.doc.of(Category::Wall).next().unwrap().id;
    let dt = ops::first_of(&f.doc, Category::DoorType).unwrap();
    ops::create_door(&mut f.doc, dt, wall, 20.0 * FT, false).unwrap();
    let ft = features(&f);
    let w = ft.walls.iter().find(|w| w.id == wall).unwrap();
    assert!(
        w.opening_length > 2.5 * FT && w.opening_length < 4.0 * FT,
        "{}",
        w.opening_length / FT
    );
    assert_eq!(w.solid.len(), 2);
    let total: f64 = w.solid.iter().map(|s| s.1 - s.0).sum();
    assert!((total + w.opening_length - w.length).abs() < 1.0);
}

#[test]
fn a_wall_that_doesnt_stack_is_a_discontinuity_and_needs_a_transfer() {
    let f = offset_wall();
    let ft = features(&f);
    assert!(ft.stacking < 1.0);
    assert!(ft
        .discontinuities
        .iter()
        .any(|d| d.kind == DiscontinuityKind::NonStackingWall && d.level == f.levels[1]));
    let prop = propose(&ft, &Rules::builtin(), Seismic::Moderate);
    let wood = prop
        .schemes
        .iter()
        .find(|s| s.kind == SchemeKind::LightWood)
        .unwrap();
    assert!(
        wood.red_flags
            .iter()
            .any(|r| r.starts_with("Transfer beams likely at Level 2")),
        "{:?}",
        wood.red_flags
    );
    let model = studio_regen::regenerate(&f.doc);
    let mut settings = wood.settings.clone();
    settings.span_dir = SpanDir::X;
    let lay = layout(&ft, &model, &Rules::builtin(), &settings);
    assert!(lay
        .members
        .iter()
        .any(|m| m.kind == MemberKind::Transfer && m.level == f.levels[1]));
    assert!(lay.flags.iter().any(|x| x.kind == FlagKind::Transfer));
}

#[test]
fn the_wood_box_proposes_light_frame_and_never_claims_engineering() {
    let f = wood_box();
    let ft = features(&f);
    let prop = propose(&ft, &Rules::builtin(), Seismic::Moderate);
    assert_eq!(prop.disclaimer, DISCLAIMER);
    assert_eq!(prop.schemes.len(), 6);
    let top = &prop.schemes[0];
    assert!(
        matches!(
            top.kind,
            SchemeKind::LightWood | SchemeKind::ColdFormedSteel
        ),
        "{:?}",
        prop.schemes
            .iter()
            .map(|s| (s.kind, s.score))
            .collect::<Vec<_>>()
    );
    let score = |k: SchemeKind| prop.schemes.iter().find(|s| s.kind == k).unwrap().score;
    assert!(score(SchemeKind::LightWood) > score(SchemeKind::SteelFrame));
    // A podium needs something to put under the housing.
    assert!(score(SchemeKind::LightWood) > score(SchemeKind::Podium));
    assert!(prop.questions.len() <= 3);
    assert!(!prop.assumptions.is_empty());
    for s in &prop.schemes {
        assert!(!s.rationale.is_empty() && !s.member_depths.is_empty());
        let text = format!("{} {}", s.rationale, s.red_flags.join(" ")).to_lowercase();
        assert!(!text.contains("complies") && !text.contains("code-compliant"));
    }
}

#[test]
fn the_podium_building_proposes_a_podium_with_a_transfer_slab() {
    let f = podium_building();
    let ft = features(&f);
    assert!(ft.open_zones.iter().any(|z| z.name == "Retail"));
    let prop = propose(&ft, &Rules::builtin(), Seismic::Moderate);
    assert_eq!(
        prop.schemes[0].kind,
        SchemeKind::Podium,
        "{:?}",
        prop.schemes
            .iter()
            .map(|s| (s.kind, s.score))
            .collect::<Vec<_>>()
    );
    assert!(prop.schemes[0]
        .red_flags
        .iter()
        .any(|r| r.contains("Transfer slab")));
    assert!(prop.questions[0].contains("ground floor"));
    let model = studio_regen::regenerate(&f.doc);
    let lay = layout(&ft, &model, &Rules::builtin(), &prop.schemes[0].settings);
    let l1 = f.levels[0];
    // Concrete columns in the podium, bearing walls above it.
    let cols: Vec<_> = lay
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Column)
        .collect();
    assert!(!cols.is_empty() && cols.iter().all(|m| m.level == l1));
    assert!(
        cols.iter().all(|m| m.size.contains("sq. column")),
        "{:?}",
        cols[0].size
    );
    assert!(lay
        .members
        .iter()
        .any(|m| m.kind == MemberKind::BearingWall && m.level == f.levels[2]));
    // The podium's lateral system is concrete; above it, wood shear walls.
    assert!(lay
        .members
        .iter()
        .any(|m| m.kind == MemberKind::ShearWall && m.level == l1 && m.size.contains("concrete")));
    assert!(lay.members.iter().any(|m| m.kind == MemberKind::ShearWall
        && m.level == f.levels[2]
        && m.size.contains("OSB")));
}

#[test]
fn a_steel_frame_puts_columns_on_the_grid_with_beams_girders_and_frames() {
    let f = wood_box();
    let ft = features(&f);
    let rules = Rules::builtin();
    let prop = propose(&ft, &rules, Seismic::High);
    let mut settings = prop
        .schemes
        .iter()
        .find(|s| s.kind == SchemeKind::SteelFrame)
        .unwrap()
        .settings
        .clone();
    settings.grid_x = 30.0 * FT;
    settings.grid_y = 20.0 * FT;
    settings.span_dir = SpanDir::Y;
    let model = studio_regen::regenerate(&f.doc);
    let lay = layout(&ft, &model, &rules, &settings);
    assert_eq!(lay.notes[0], DISCLAIMER);
    // Grid lines prefer walls: the demising walls at 20' and 40' beat an even 30' fill.
    let xs: Vec<f64> = lay.grid_x.iter().map(|x| (x / FT).round()).collect();
    assert_eq!(xs, [0.0, 20.0, 40.0, 60.0]);
    for g in [&lay.grid_x, &lay.grid_y] {
        for w in g.windows(2) {
            assert!(w[1] - w[0] <= 1.25 * 30.0 * FT + 1.0);
        }
    }
    let cols = lay
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Column)
        .count();
    assert_eq!(cols, 3 * lay.grid_x.len() * lay.grid_y.len());
    assert!(lay
        .members
        .iter()
        .any(|m| m.kind == MemberKind::Girder && m.size.starts_with('W')));
    assert!(lay.members.iter().any(|m| m.kind == MemberKind::Beam));
    assert!(lay
        .members
        .iter()
        .any(|m| m.kind == MemberKind::BracedFrame));
    assert!(lay
        .members
        .iter()
        .all(|m| m.size.ends_with("(prelim.)") && !m.rule.is_empty()));
    // Deterministic.
    assert_eq!(lay, layout(&ft, &model, &rules, &settings));
    // Moment frames run the whole perimeter.
    settings.lateral = LateralKind::MomentFrames;
    let mf = layout(&ft, &model, &rules, &settings);
    let per_level = mf
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::MomentFrame && m.level == f.levels[0])
        .count();
    assert_eq!(
        per_level,
        2 * (lay.grid_x.len() - 1) + 2 * (lay.grid_y.len() - 1)
    );
}

#[test]
fn grid_lines_prefer_walls_and_fill_evenly() {
    let t = 30.0 * FT;
    let lines = layout::pick_lines(0.0, 100.0 * FT, t, &[(32.0 * FT, 5000.0)]);
    assert_eq!(lines[0], 0.0);
    assert_eq!(lines[1], 32.0 * FT);
    assert_eq!(*lines.last().unwrap(), 100.0 * FT);
    for w in lines.windows(2) {
        assert!(w[1] - w[0] <= 1.25 * t);
    }
}

#[test]
fn a_lopsided_building_is_flagged_for_torsion_or_a_short_direction() {
    // One wall line along x, all on one side: lateral is short north–south and off-centre.
    let mut f = shell(1);
    let walls: Vec<ElementId> = f.doc.of(Category::Wall).map(|e| e.id).collect();
    // Punch the y-walls full of windows so they can't take shear.
    let wt = ops::first_of(&f.doc, Category::WindowType).unwrap();
    for w in walls {
        let d = f.doc.data(w).unwrap().clone();
        if let studio_core::ElementData::Wall { start, end, .. } = d {
            if (start.x - end.x).abs() < 1.0 {
                for k in 0..10 {
                    let _ =
                        ops::create_window(&mut f.doc, wt, w, (2.0 + 4.0 * k as f64) * FT, false);
                }
            }
        }
    }
    let ft = features(&f);
    let rules = Rules::builtin();
    let prop = propose(&ft, &rules, Seismic::High);
    let wood = prop
        .schemes
        .iter()
        .find(|s| s.kind == SchemeKind::LightWood)
        .unwrap();
    let model = studio_regen::regenerate(&f.doc);
    let lay = layout(&ft, &model, &rules, &wood.settings);
    assert!(
        lay.flags
            .iter()
            .any(|x| x.kind == FlagKind::LateralDirection),
        "{:?}",
        lay.flags
    );
}

#[test]
fn foundations_go_under_walls_and_columns_below_frost() {
    let rules = Rules::builtin();
    let frost = rules.foundation.frost_depth_in * 25.4;
    // The wood box: strip footings under its exterior and bearing walls, none under columns.
    let f = wood_box();
    let ft = features(&f);
    let model = studio_regen::regenerate(&f.doc);
    let prop = propose(&ft, &rules, Seismic::Moderate);
    let wood = prop
        .schemes
        .iter()
        .find(|s| s.kind == SchemeKind::LightWood)
        .unwrap();
    assert!(wood
        .member_depths
        .iter()
        .any(|d| d.starts_with("Foundations: continuous strip footings")));
    let lay = layout(&ft, &model, &rules, &wood.settings);
    let strips: Vec<_> = lay
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::StripFooting)
        .collect();
    assert!(strips.len() >= 4, "{}", strips.len());
    assert!(lay
        .members
        .iter()
        .all(|m| m.kind != MemberKind::SpreadFooting));
    for s in &strips {
        assert_eq!(s.level, f.levels[0]);
        assert!(s.width >= 16.0 * 25.4 - 1.0 && s.size.contains("strip footing"));
        assert!(s.top < 0.0 && s.base < s.top);
    }
    // Exterior strips bear below frost.
    let perimeter = strips
        .iter()
        .filter(|s| s.start.y.abs() < 1.0 && s.end.y.abs() < 1.0)
        .collect::<Vec<_>>();
    assert!(!perimeter.is_empty() && perimeter.iter().all(|s| s.base <= -frost + 1.0));
    // The same box in steel: a spread footing under every ground-floor column.
    let mut steel = prop
        .schemes
        .iter()
        .find(|s| s.kind == SchemeKind::SteelFrame)
        .unwrap()
        .settings
        .clone();
    steel.grid_x = 30.0 * FT;
    steel.grid_y = 20.0 * FT;
    let lay = layout(&ft, &model, &rules, &steel);
    let cols = lay
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Column && m.level == f.levels[0])
        .count();
    let pads: Vec<_> = lay
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::SpreadFooting)
        .collect();
    assert_eq!(pads.len(), cols);
    assert!(pads
        .iter()
        .all(|p| p.size.contains("spread footing") && p.width >= 2.0 * FT - 1.0));
    // An interior column carries more than a corner one.
    let biggest = pads.iter().map(|p| p.width).fold(0.0, f64::max);
    let smallest = pads.iter().map(|p| p.width).fold(f64::INFINITY, f64::min);
    assert!(biggest > smallest);
}

#[test]
fn tall_buildings_go_deep_crowded_footings_become_a_mat_and_basements_get_walls() {
    let mut rules = Rules::builtin();
    // Twelve stories of steel: pile caps and a foundation flag.
    let f = shell(12);
    let ft = features(&f);
    let model = studio_regen::regenerate(&f.doc);
    let prop = propose(&ft, &rules, Seismic::Moderate);
    let steel = prop
        .schemes
        .iter()
        .find(|s| s.kind == SchemeKind::SteelFrame)
        .unwrap();
    assert!(steel
        .red_flags
        .iter()
        .any(|r| r.starts_with("Deep foundations likely")));
    let lay = layout(&ft, &model, &rules, &steel.settings);
    assert!(lay
        .members
        .iter()
        .any(|m| m.kind == MemberKind::PileCap && m.size.contains("piles")));
    assert!(lay.flags.iter().any(|x| x.kind == FlagKind::Foundation));
    // Footings covering over the rules' share of the footprint: one mat.
    rules.foundation.mat_share = 0.01;
    let f = wood_box();
    let ft = features(&f);
    let model = studio_regen::regenerate(&f.doc);
    let wood = propose(&ft, &rules, Seismic::Moderate)
        .schemes
        .into_iter()
        .find(|s| s.kind == SchemeKind::LightWood)
        .unwrap();
    let lay = layout(&ft, &model, &rules, &wood.settings);
    let mats: Vec<_> = lay
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Mat)
        .collect();
    assert_eq!(mats.len(), 1);
    assert!(lay
        .members
        .iter()
        .all(|m| m.kind != MemberKind::StripFooting));
    assert!((mats[0].end.x - mats[0].start.x - 60.0 * FT).abs() < 600.0);
    // A basement: foundation walls up to grade.
    let rules = Rules::builtin();
    let mut f = shell(2);
    ops::set_property(&mut f.doc, f.levels[0], "elevation", "-10'", 0).unwrap();
    let ft = features(&f);
    let model = studio_regen::regenerate(&f.doc);
    let prop = propose(&ft, &rules, Seismic::Moderate);
    assert!(prop.schemes[0]
        .red_flags
        .iter()
        .any(|r| r.contains("below grade")));
    let lay = layout(&ft, &model, &rules, &prop.schemes[0].settings);
    let walls: Vec<_> = lay
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::FoundationWall)
        .collect();
    assert_eq!(walls.len(), 4);
    assert!(walls
        .iter()
        .all(|w| (w.top - 0.0).abs() < 1.0 && w.base < -3000.0));
}
