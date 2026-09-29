//! Fixtures (a two-story house, a three-story office, a four-story apartment building) and
//! tests for each discipline's features, proposal and layout.

use studio_core::mep::{Climate, Discipline, MepKind, MepSettings};
use studio_core::units::MM_PER_FT;
use studio_core::{ops, Category, Document, ElementId};
use studio_geom::Pt;

use crate::*;

const FT: f64 = MM_PER_FT;

fn p(x: f64, y: f64) -> Pt {
    Pt::new(x * FT, y * FT)
}

/// `stories` levels 10' apart (12' when `tall`), each a `w` x `d` box split into a grid of
/// `cols` x `rows` rooms named by `name(level, col, row)`.
fn building(
    stories: usize,
    tall: bool,
    (w, d): (f64, f64),
    (cols, rows): (usize, usize),
    name: impl Fn(usize, usize, usize) -> String,
) -> (Document, Vec<ElementId>) {
    let mut doc = Document::new();
    ops::seed_default_project(&mut doc).unwrap();
    let h = if tall { 12.0 } else { 10.0 };
    let mut levels: Vec<ElementId> = doc.levels().into_iter().map(|l| l.0).collect();
    // Level 2 moves to the story height; more are added above.
    ops::set_property(&mut doc, levels[1], "elevation", &format!("{h}'"), 0).unwrap();
    while levels.len() < stories {
        let e = levels.len() as f64 * h * FT;
        levels.push(ops::create_level(&mut doc, e).unwrap());
    }
    levels.truncate(stories);
    let ext = ops::first_of(&doc, Category::WallType).unwrap();
    let int = doc
        .of(Category::WallType)
        .find(|e| e.data.name().starts_with("Interior"))
        .unwrap()
        .id;
    let ft_id = ops::first_of(&doc, Category::FloorType).unwrap();
    let dt = ops::first_of(&doc, Category::DoorType).unwrap();
    let corners = [p(0.0, 0.0), p(w, 0.0), p(w, d), p(0.0, d)];
    for (li, &l) in levels.iter().enumerate() {
        let mut ext_walls = vec![];
        for i in 0..4 {
            ext_walls.push(
                ops::create_wall(&mut doc, ext, l, corners[i], corners[(i + 1) % 4]).unwrap(),
            );
        }
        if li == 0 {
            ops::create_door(&mut doc, dt, ext_walls[0], w * FT / 2.0, false).unwrap();
        }
        for c in 1..cols {
            let x = w * c as f64 / cols as f64;
            ops::create_wall(&mut doc, int, l, p(x, 0.0), p(x, d)).unwrap();
        }
        for r in 1..rows {
            let y = d * r as f64 / rows as f64;
            ops::create_wall(&mut doc, int, l, p(0.0, y), p(w, y)).unwrap();
        }
        ops::create_floor(&mut doc, ft_id, l, corners.to_vec()).unwrap();
        for c in 0..cols {
            for r in 0..rows {
                let at = p(
                    w * (c as f64 + 0.5) / cols as f64,
                    d * (r as f64 + 0.5) / rows as f64,
                );
                let room = ops::create_room(&mut doc, l, at).unwrap();
                ops::set_property(&mut doc, room, "name", &name(li, c, r), 0).unwrap();
            }
        }
    }
    (doc, levels)
}

fn house() -> (Document, Vec<ElementId>) {
    building(2, false, (40.0, 30.0), (3, 2), |l, c, r| {
        match (l, c, r) {
            (0, 0, 0) => "Kitchen",
            (0, 1, 0) => "Powder Room",
            (0, 2, 0) => "Laundry",
            (0, _, _) => "Living",
            (1, 1, 0) => "Bath",
            (1, _, _) => "Bedroom",
            _ => "Room",
        }
        .into()
    })
}

fn office() -> (Document, Vec<ElementId>) {
    building(3, true, (150.0, 100.0), (5, 3), |l, c, r| {
        match (l, c, r) {
            (0, 0, 0) => "Mechanical",
            (0, 1, 0) => "Electrical",
            (_, 2, 0) => "Restroom",
            (_, 0, 1) => "Tel/Data",
            (_, 4, 2) => "Conference",
            (_, _, 1) => "Corridor",
            _ => "Open Office",
        }
        .into()
    })
}

fn apartments() -> (Document, Vec<ElementId>) {
    building(4, false, (120.0, 60.0), (4, 3), |l, c, r| match (c, r) {
        (_, 1) => "Corridor".into(),
        (0, _) => format!("Unit {l}{r} Kitchen"),
        (1, _) => format!("Unit {l}{r} Bath"),
        _ => format!("Unit {l}{r} Bedroom"),
    })
}

fn features_of(doc: &Document) -> Features {
    let model = studio_regen::regenerate(doc);
    extract(doc, &model, &Rules::builtin())
}

fn top(d: Discipline, f: &Features, c: Climate) -> MepProposal {
    propose(d, f, &Rules::builtin(), c)
}

#[test]
fn features_read_rooms_outside_walls_dwellings_and_doors() {
    let (doc, _) = house();
    let f = features_of(&doc);
    assert_eq!(f.stories, 2);
    assert!((f.area - 2.0 * 1200.0 * FT * FT).abs() < 1.0);
    assert!(f.residential);
    assert_eq!(f.dwelling_units, 1);
    assert_eq!(f.uses[0], "residential");
    let kinds: Vec<&str> = f.spaces.iter().map(|s| s.kind.as_str()).collect();
    for k in ["Kitchen", "Bathroom", "Laundry", "Living", "Bedroom"] {
        assert!(kinds.contains(&k), "{k} in {kinds:?}");
    }
    // Every room of a 3 x 2 grid touches the outside.
    assert!(f
        .spaces
        .iter()
        .all(|s| s.exterior_len > 0.0 && s.exterior_edge.is_some()));
    assert_eq!(f.exterior_doors.len(), 1);
    let (doc, _) = apartments();
    let f = features_of(&doc);
    assert_eq!(f.dwelling_units, 8, "one per kitchen");
}

#[test]
fn mechanical_suits_the_building_and_lays_out_zones_equipment_and_ducts() {
    let rules = Rules::builtin();
    let (doc, levels) = house();
    let f = features_of(&doc);
    let prop = top(Discipline::Mechanical, &f, Climate::Mixed);
    assert!(
        matches!(
            prop.systems[0].key.as_str(),
            "furnace_split" | "ductless_minisplit"
        ),
        "{:?}",
        prop.systems
            .iter()
            .map(|s| (&s.key, s.score))
            .collect::<Vec<_>>()
    );
    assert!(prop.disclaimer.starts_with("Preliminary — not engineered"));
    let s = |k: &str| MepSettings {
        discipline: Discipline::Mechanical,
        system: k.into(),
        climate: Climate::Mixed,
    };
    let lay = layout(&f, &rules, &s("furnace_split"));
    assert!(!lay.zones.is_empty());
    let count = |k: MepKind| lay.items.iter().filter(|i| i.kind == k).count();
    assert_eq!(count(MepKind::Equipment), 2, "an air handler per floor");
    assert!(count(MepKind::OutdoorUnit) >= 1 && count(MepKind::Diffuser) >= 12);
    assert!(count(MepKind::SupplyDuct) > 3);
    assert!(lay
        .items
        .iter()
        .all(|i| i.size.ends_with("(prelim.)") && !i.rule.is_empty()));
    // Mini-splits: a head in every room on its outside wall, condensers of up to 4 heads.
    let ms = layout(&f, &rules, &s("ductless_minisplit"));
    let heads = ms
        .items
        .iter()
        .filter(|i| i.kind == MepKind::IndoorUnit)
        .count();
    let cu = ms
        .items
        .iter()
        .filter(|i| i.kind == MepKind::OutdoorUnit)
        .count();
    assert_eq!(
        heads,
        f.spaces
            .iter()
            .filter(|x| rules.space(&x.kind).sf_per_ton > 0.0)
            .count()
    );
    assert!(cu >= heads.div_ceil(4));
    assert!(ms.flags.is_empty(), "every room has an outside wall");
    let _ = levels;
    // The office: a bigger system, and interior rooms flagged for PTACs.
    let (doc, _) = office();
    let f = features_of(&doc);
    let prop = top(Discipline::Mechanical, &f, Climate::Hot);
    assert!(
        matches!(prop.systems[0].key.as_str(), "vrf_doas" | "packaged_rtu"),
        "{:?}",
        prop.systems
            .iter()
            .map(|s| (&s.key, s.score))
            .collect::<Vec<_>>()
    );
    let ptac = layout(&f, &rules, &s("ptac"));
    assert!(ptac.flags.iter().any(|x| x.title == "Interior Room"));
    let vrf = layout(&f, &rules, &s("vrf_doas"));
    assert!(vrf.items.iter().any(|i| i.kind == MepKind::Shaft));
    assert!(vrf
        .items
        .iter()
        .any(|i| i.kind == MepKind::Equipment && i.size.starts_with("DOAS")));
    // Deterministic.
    assert_eq!(vrf, layout(&f, &rules, &s("vrf_doas")));
}

#[test]
fn electrical_sizes_the_service_and_places_panels_lights_and_receptacles() {
    let rules = Rules::builtin();
    let (doc, _) = house();
    let f = features_of(&doc);
    let prop = top(Discipline::Electrical, &f, Climate::Mixed);
    assert_eq!(prop.systems[0].key, "res_single_phase");
    assert!(prop.systems[0]
        .highlights
        .iter()
        .any(|h| h.contains("A service at 120/240 V")));
    let lay = layout(&f, &rules, &prop.systems[0].settings);
    let count = |k: MepKind| lay.items.iter().filter(|i| i.kind == k).count();
    assert_eq!(count(MepKind::Service), 1);
    assert_eq!(count(MepKind::Panel), 2, "the main and a panel upstairs");
    assert!(count(MepKind::Light) >= f.spaces.len());
    assert!(count(MepKind::Receptacle) > 20);
    assert!(lay
        .items
        .iter()
        .any(|i| i.kind == MepKind::Receptacle && i.size.starts_with("GFCI")));
    // Apartments: a meter center and a panel per dwelling.
    let (doc, _) = apartments();
    let f = features_of(&doc);
    let prop = top(Discipline::Electrical, &f, Climate::Mixed);
    assert_eq!(prop.systems[0].key, "multifamily_meters");
    let lay = layout(&f, &rules, &prop.systems[0].settings);
    assert_eq!(
        lay.items
            .iter()
            .filter(|i| i.kind == MepKind::Panel && i.size.starts_with("100 A unit"))
            .count(),
        8
    );
    // The office: three-phase.
    let (doc, _) = office();
    let f = features_of(&doc);
    let prop = top(Discipline::Electrical, &f, Climate::Mixed);
    assert!(
        prop.systems[0].key.starts_with("three_phase"),
        "{:?}",
        prop.systems
            .iter()
            .map(|s| (&s.key, s.score))
            .collect::<Vec<_>>()
    );
}

#[test]
fn plumbing_counts_fixtures_stacks_wet_rooms_and_runs_drains_and_mains() {
    let rules = Rules::builtin();
    let (doc, _) = house();
    let f = features_of(&doc);
    let prop = top(Discipline::Plumbing, &f, Climate::Mixed);
    assert!(
        matches!(
            prop.systems[0].key.as_str(),
            "tank_heater" | "heat_pump_heater" | "tankless"
        ),
        "{:?}",
        prop.systems
            .iter()
            .map(|s| (&s.key, s.score))
            .collect::<Vec<_>>()
    );
    let lay = layout(&f, &rules, &prop.systems[0].settings);
    let count = |k: MepKind| lay.items.iter().filter(|i| i.kind == k).count();
    // Kitchen (2), powder room (3), laundry (1) downstairs; bath (3) upstairs.
    assert_eq!(count(MepKind::Fixture), 9);
    assert_eq!(count(MepKind::WaterHeater), 1);
    assert!(count(MepKind::Stack) >= 2 && count(MepKind::Vent) >= 1);
    assert!(count(MepKind::BuildingDrain) >= 1 && count(MepKind::WaterService) == 1);
    assert!(count(MepKind::HotWater) >= 1);
    // The upstairs bath sits over the powder room: its stack lines up with the one below.
    let stacks: Vec<_> = lay
        .items
        .iter()
        .filter(|i| i.kind == MepKind::Stack)
        .collect();
    assert!(stacks.iter().any(|a| stacks
        .iter()
        .any(|b| a.level != b.level && a.pts[0].dist(b.pts[0]) < 1.0)));
    // Apartments want a central plant.
    let (doc, _) = apartments();
    let f = features_of(&doc);
    let prop = top(Discipline::Plumbing, &f, Climate::Mixed);
    assert_eq!(
        prop.systems[0].key,
        "central_recirc",
        "{:?}",
        prop.systems
            .iter()
            .map(|s| (&s.key, s.score))
            .collect::<Vec<_>>()
    );
}

#[test]
fn technology_places_rooms_outlets_access_points_and_checks_cable_reach() {
    let rules = Rules::builtin();
    let (doc, _) = house();
    let f = features_of(&doc);
    let prop = top(Discipline::Technology, &f, Climate::Mixed);
    assert_eq!(prop.systems[0].key, "media_panel");
    let (doc, levels) = office();
    let f = features_of(&doc);
    let prop = top(Discipline::Technology, &f, Climate::Mixed);
    assert_eq!(
        prop.systems[0].key,
        "mdf_idf",
        "{:?}",
        prop.systems
            .iter()
            .map(|s| (&s.key, s.score))
            .collect::<Vec<_>>()
    );
    let lay = layout(&f, &rules, &prop.systems[0].settings);
    let count = |k: MepKind| lay.items.iter().filter(|i| i.kind == k).count();
    assert_eq!(count(MepKind::Mdf), 1);
    assert_eq!(count(MepKind::Idf), levels.len() - 1);
    assert_eq!(count(MepKind::Backbone), 1);
    // 15,000 sf a floor at 1,500 sf per access point.
    assert_eq!(count(MepKind::AccessPoint), 30);
    assert!(count(MepKind::DataOutlet) > 50);
    assert!(
        count(MepKind::AvDisplay) >= 3
            && count(MepKind::Camera) == 1
            && count(MepKind::AccessControl) == 1
    );
    // One room for everything: a 150' floor on three stories stays in reach? The corners
    // don't: the single-MDF layout flags them.
    let single = MepSettings {
        discipline: Discipline::Technology,
        system: "single_mdf".into(),
        climate: Climate::Mixed,
    };
    let s = layout(&f, &rules, &single);
    assert!(
        s.flags.iter().any(|x| x.title == "Cable Too Long")
            || s.items.iter().filter(|i| i.kind == MepKind::Idf).count() == 0
    );
}
