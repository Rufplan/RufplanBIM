use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::{ops, Category as Cat, Document, ElementData, ElementId};
use studio_geom::Pt;

use crate::{review, Category, Milestone, Options, Severity};

/// A 30' x 20' house: four exterior walls, a wall between a bedroom and a living room, an
/// entry door, one window in the living room, and the rooms named.
fn house() -> (Document, ElementId, Vec<ElementId>) {
    let mut doc = Document::new();
    ops::seed_default_project(&mut doc).unwrap();
    let l1 = doc.levels()[0].0;
    let ext = doc
        .of(Cat::WallType)
        .find(|e| e.data.name().starts_with("Exterior"))
        .map(|e| e.id)
        .unwrap();
    let (w, h) = (30.0 * MM_PER_FT, 20.0 * MM_PER_FT);
    let c = [
        Pt::new(0.0, 0.0),
        Pt::new(w, 0.0),
        Pt::new(w, h),
        Pt::new(0.0, h),
    ];
    let mut walls: Vec<ElementId> = (0..4)
        .map(|i| ops::create_wall(&mut doc, ext, l1, c[i], c[(i + 1) % 4]).unwrap())
        .collect();
    let int = doc
        .of(Cat::WallType)
        .find(|e| e.data.name().starts_with("Interior"))
        .map(|e| e.id)
        .unwrap_or(ext);
    walls.push(
        ops::create_wall(
            &mut doc,
            int,
            l1,
            Pt::new(12.0 * MM_PER_FT, 0.0),
            Pt::new(12.0 * MM_PER_FT, h),
        )
        .unwrap(),
    );
    let dt = doc
        .of(Cat::DoorType)
        .find(|e| e.data.name().contains("36"))
        .map(|e| e.id)
        .unwrap_or_else(|| ops::first_of(&doc, Cat::DoorType).unwrap());
    // Entry door in the living room's south wall; a door between the rooms.
    ops::create_door(&mut doc, dt, walls[0], 20.0 * MM_PER_FT, false).unwrap();
    ops::create_door(&mut doc, dt, walls[4], 10.0 * MM_PER_FT, false).unwrap();
    let wt = ops::first_of(&doc, Cat::WindowType).unwrap();
    ops::create_window(&mut doc, wt, walls[1], 10.0 * MM_PER_FT, false).unwrap();
    for (name, x) in [("Bedroom", 6.0), ("Living", 20.0)] {
        let r = ops::create_room(&mut doc, l1, Pt::new(x * MM_PER_FT, 10.0 * MM_PER_FT)).unwrap();
        ops::set_property(&mut doc, r, "name", name, 0).unwrap();
    }
    (doc, l1, walls)
}

fn run(doc: &Document, m: Milestone) -> crate::Report {
    let model = studio_regen::regenerate(doc);
    review(
        doc,
        &model,
        &Options {
            milestone: m,
            categories: Category::ALL.to_vec(),
        },
    )
}

fn has(r: &crate::Report, rule: &str) -> bool {
    r.findings.iter().any(|f| f.rule == rule)
}

#[test]
fn a_bedroom_without_a_window_fails_emergency_escape_with_the_irc_section() {
    let (doc, _, _) = house();
    let r = run(&doc, Milestone::Cd90);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule == "egress-none")
        .expect("egress finding");
    assert_eq!(
        (f.severity, f.category),
        (Severity::Critical, Category::Code)
    );
    assert_eq!(f.reference, "IRC R310.1");
    assert!(f.title.contains("Bedroom"));
    // Critical first, and the score reflects it.
    assert_eq!(r.findings[0].severity, Severity::Critical);
    assert!(r.score <= 85);
}

#[test]
fn a_small_window_is_measured_for_escape_and_a_big_one_passes() {
    let (mut doc, _, walls) = house();
    // A 36" x 60" double-hung in the bedroom's west wall: its lower sash opens about
    // 32" x 26" (~5.8 sf) with a 2'-6"-ish sill.
    let big = doc
        .of(Cat::WindowType)
        .find(|e| matches!(&e.data, ElementData::WindowType { width, height, .. } if (*width - 36.0 * MM_PER_IN).abs() < 1.0 && (*height - 60.0 * MM_PER_IN).abs() < 1.0))
        .map(|e| e.id);
    let small = doc
        .of(Cat::WindowType)
        .min_by(|a, b| {
            let area = |d: &ElementData| match d {
                ElementData::WindowType { width, height, .. } => width * height,
                _ => f64::MAX,
            };
            area(&a.data).total_cmp(&area(&b.data))
        })
        .map(|e| e.id)
        .unwrap();
    let w = ops::create_window(&mut doc, small, walls[3], 10.0 * MM_PER_FT, false).unwrap();
    let r = run(&doc, Milestone::Cd90);
    assert!(!has(&r, "egress-none"));
    assert!(
        has(&r, "egress-size"),
        "{:?}",
        r.findings.iter().map(|f| &f.title).collect::<Vec<_>>()
    );
    if let Some(big) = big {
        ops::delete(&mut doc, &[w]).unwrap();
        ops::create_window(&mut doc, big, walls[3], 10.0 * MM_PER_FT, false).unwrap();
        let r = run(&doc, Milestone::Cd90);
        assert!(!has(&r, "egress-size") && !has(&r, "egress-none"));
    }
}

#[test]
fn clear_openings_come_from_the_sashes() {
    // A 36" x 60" double-hung: the lower sash is half the height, less frame and rails.
    let d = ElementData::WindowType {
        name: "DH".into(),
        family: studio_core::element::WindowFamily::DoubleHung,
        width: 36.0 * MM_PER_IN,
        height: 60.0 * MM_PER_IN,
        sill: 30.0 * MM_PER_IN,
        units: 1,
        grille: Default::default(),
        finish: Default::default(),
    };
    let (w, h) = crate::ctx::clear_opening(&d).unwrap();
    assert!(
        w > 28.0 * MM_PER_IN && w < 33.0 * MM_PER_IN,
        "{}",
        w / MM_PER_IN
    );
    assert!(
        h > 20.0 * MM_PER_IN && h < 29.0 * MM_PER_IN,
        "{}",
        h / MM_PER_IN
    );
    // Fixed glass doesn't open.
    let mut fixed = d.clone();
    if let ElementData::WindowType { family, .. } = &mut fixed {
        *family = studio_core::element::WindowFamily::Fixed;
    }
    assert!(crate::ctx::clear_opening(&fixed).is_none());
}

#[test]
fn steep_risers_duplicate_marks_tbds_and_early_milestones() {
    let (mut doc, l1, _) = house();
    let l2 = ops::create_level(&mut doc, 10.0 * MM_PER_FT).unwrap();
    // 10' in 15 risers of at most 8": 8" risers break the IRC's 7-3/4".
    let _ = l2;
    let s = studio_core::build::create_stair(
        &mut doc,
        l1,
        Pt::new(2.0 * MM_PER_FT, 2.0 * MM_PER_FT),
        Pt::new(2.0 * MM_PER_FT, 14.0 * MM_PER_FT),
        36.0 * MM_PER_IN,
    )
    .unwrap();
    doc.transact("riser", |tx| {
        tx.modify(s, |d| {
            if let ElementData::Stair { max_riser, .. } = d {
                *max_riser = 8.0 * MM_PER_IN;
            }
        })
    })
    .unwrap();
    let doors: Vec<ElementId> = doc.of(Cat::Door).map(|e| e.id).collect();
    for d in &doors {
        ops::set_property(&mut doc, *d, "mark", "101", 0).unwrap();
    }
    let view = ops::first_of(&doc, Cat::View).unwrap();
    ops::create_text(&mut doc, view, Pt::new(0.0, 0.0), "FLASHING PER DETAIL TBD").unwrap();
    let r = run(&doc, Milestone::Permit);
    assert!(has(&r, "dup-door-mark"));
    let tbd = r.findings.iter().find(|f| f.rule == "tbd").unwrap();
    assert_eq!(
        tbd.severity,
        Severity::Critical,
        "a TBD is critical in a permit set"
    );
    // 10'-0" in 15 risers of 8" breaks the IRC's 7-3/4".
    let riser = r
        .findings
        .iter()
        .find(|f| f.rule == "stair-riser")
        .expect("riser finding");
    assert!(
        riser.title.contains("8\"") && riser.reference == "IRC R311.7.5.1",
        "{}",
        riser.title
    );
    // At SD the same TBD is only a note, and missing specs aren't flagged yet.
    let sd = run(&doc, Milestone::SchematicDesign);
    assert_eq!(
        sd.findings
            .iter()
            .find(|f| f.rule == "tbd")
            .unwrap()
            .severity,
        Severity::Info
    );
    assert!(!has(&sd, "no-specs") && has(&r, "no-specs"));
}

#[test]
fn categories_filter_and_counts_add_up() {
    let (doc, _, _) = house();
    let model = studio_regen::regenerate(&doc);
    let r = review(
        &doc,
        &model,
        &Options {
            milestone: Milestone::Cd100,
            categories: vec![Category::Waterproofing],
        },
    );
    assert!(r
        .findings
        .iter()
        .all(|f| f.category == Category::Waterproofing));
    let total: usize = r
        .counts
        .iter()
        .map(|c| c.critical + c.major + c.minor + c.info)
        .sum();
    assert_eq!(total, r.findings.len());
    assert!(r.code_basis.contains("IRC"));
    // The PDF writes.
    let pdf = crate::report::pdf(&run(&doc, Milestone::Cd100), &[]).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn california_projects_cite_the_crc() {
    let (mut doc, _, _) = house();
    let (_, mut d) = studio_core::project::get(&doc).unwrap();
    d.location.state = "CA".into();
    studio_core::project::set(&mut doc, "Oak House", "2601", d).unwrap();
    let r = run(&doc, Milestone::Cd90);
    let f = r.findings.iter().find(|f| f.rule == "egress-none").unwrap();
    assert_eq!(f.reference, "CRC R310.1");
}

#[test]
fn scores_and_summaries() {
    use crate::report::{score, summary};
    let mk = |s: Severity| crate::Finding {
        id: String::new(),
        rule: String::new(),
        category: Category::Code,
        severity: s,
        title: String::new(),
        detail: String::new(),
        fix: String::new(),
        reference: String::new(),
        elements: vec![],
        view: None,
        source: "rules".into(),
    };
    let fs = vec![
        mk(Severity::Critical),
        mk(Severity::Major),
        mk(Severity::Minor),
        mk(Severity::Info),
    ];
    assert_eq!(score(&fs), 100 - 15 - 5 - 1);
    assert!(summary(&fs).starts_with("1 critical, 1 major, 1 minor issues. Not ready"));
    assert_eq!(summary(&[]), "No issues found by the automated checks.");
}

#[test]
fn fixes_clear_what_they_can_in_one_undo_step() {
    let (mut doc, _, _) = house();
    let doors: Vec<ElementId> = doc.of(Cat::Door).map(|e| e.id).collect();
    for d in &doors {
        ops::set_property(&mut doc, *d, "mark", "101", 0).unwrap();
    }
    let before = run(&doc, Milestone::Cd90);
    for rule in ["egress-none", "dup-door-mark", "wrb"] {
        assert!(has(&before, rule), "{rule} before");
    }
    let model = studio_regen::regenerate(&doc);
    let plan = crate::fix::plan(&doc, &model, &before);
    // A window is added in the bedroom's exterior wall; that's a design change.
    let egress = plan
        .fixes
        .iter()
        .find(|f| matches!(f.action, crate::fix::Action::AddWindow { .. }))
        .expect("egress fix");
    assert!(egress.design_change);
    let depth = doc.undo_depth();
    let actions: Vec<_> = plan.fixes.iter().map(|f| f.action.clone()).collect();
    let (n, errors) = crate::fix::apply(&mut doc, &model, &actions, "QA/QC: Fix issues");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(n, actions.len());
    assert_eq!(doc.undo_depth(), depth + 1, "one undo step");
    let after = run(&doc, Milestone::Cd90);
    for rule in ["egress-none", "dup-door-mark", "wrb"] {
        assert!(
            !has(&after, rule),
            "{rule} after: {:?}",
            after.findings.iter().map(|f| &f.title).collect::<Vec<_>>()
        );
    }
    assert!(after.score > before.score);
    // Undo puts it all back.
    doc.undo().unwrap();
    assert!(has(&run(&doc, Milestone::Cd90), "egress-none"));
}
