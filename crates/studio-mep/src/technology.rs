//! Technology (low voltage): data drops by room type, Wi-Fi coverage, security at the
//! exterior doors, AV in meeting rooms, the telecom rooms each candidate uses, and the
//! 90 m (295') copper reach from a telecom room to every outlet.

use studio_core::mep::{Climate, Discipline, MepKind, MepLayout, MepSettings};
use studio_core::units::MM_PER_FT;
use studio_geom::Pt;

use crate::common::*;
use crate::features::SQFT;
use crate::rules::Rules;
use crate::types::*;

fn drops(rules: &Rules, s: &Space) -> usize {
    let r = rules.space(&s.kind);
    (r.data_drops as f64 + r.data_per_1000sf * s.area / SQFT / 1000.0).round() as usize
}

fn manhattan(a: Pt, b: Pt) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// Where the main telecom room goes.
fn mdf_at(f: &Features) -> Option<(usize, Pt, bool)> {
    let lowest = f.levels.first()?;
    let mid = middle(f);
    let room = room_of(
        f,
        Some(lowest.id),
        &["Telecom", "Electrical", "Mechanical", "Storage", "Laundry"],
        mid,
    );
    Some((0, room.map_or(mid, |r| r.center), room.is_some()))
}

pub fn propose(f: &Features, rules: &Rules, climate: Climate) -> MepProposal {
    let t = &rules.technology;
    let n_drops: usize = f.spaces.iter().map(|s| drops(rules, s)).sum();
    let ap_sf = if f.residential {
        t.ap_sf_residential
    } else {
        t.ap_sf_commercial
    };
    let aps: usize = f
        .levels
        .iter()
        .map(|l| (l.area / SQFT / ap_sf).ceil().max(1.0) as usize)
        .sum();
    let mdf = mdf_at(f).map_or(middle(f), |m| m.1);
    let rise = f.levels.last().map_or(0.0, |l| l.elevation)
        - f.levels.first().map_or(0.0, |l| l.elevation);
    let reach = f
        .spaces
        .iter()
        .map(|s| manhattan(mdf, s.center))
        .fold(0.0, f64::max);
    let max = t.max_cable_ft * MM_PER_FT;
    let mut v = vec![];
    for (key, sr) in &t.systems {
        let mut red = vec![];
        let (score_reach, note) = if sr.fiber {
            (
                1.0,
                "Fiber reaches kilometres: no reach limit in this building.".to_string(),
            )
        } else if sr.idf_per_floor {
            let ok = reach <= max;
            (
                if ok { 1.0 } else { 0.6 },
                format!(
                    "About {} across a floor from a closet at the core.",
                    ft(reach)
                ),
            )
        } else {
            let r = reach + rise;
            let ok = r <= max;
            if !ok {
                red.push(format!(
                    "Some outlets are over {:.0}' of cable from one room: add a closet.",
                    t.max_cable_ft
                ));
            }
            (
                if ok { 1.0 } else { 0.35 },
                format!("Farthest outlet about {} of cable from one room.", ft(r)),
            )
        };
        let extra = vec![Criterion {
            name: "Cable reach".into(),
            score: score_reach,
            note,
        }];
        if !sr.media_panel
            && !f.spaces.iter().any(|s| s.kind == "Telecom")
            && f.area / SQFT > 5000.0
        {
            red.push(
                "No telecom room in the model: allow about 10' x 12' (larger for an MDF).".into(),
            );
        }
        let rooms = if sr.media_panel {
            "structured media panel in a closet".to_string()
        } else if sr.idf_per_floor {
            format!(
                "MDF + {} IDF(s), fiber backbone",
                f.stories.saturating_sub(1)
            )
        } else if sr.fiber {
            "optical line terminal + fiber to zone terminals".into()
        } else {
            "one telecom room (MDF)".into()
        };
        let highlights = vec![
            format!("{n_drops} data drops, {aps} Wi-Fi access points"),
            rooms,
            format!(
                "{} camera(s) and door readers at exterior doors",
                f.exterior_doors.len()
            ),
        ];
        v.push(score(
            f,
            key,
            sr,
            Discipline::Technology,
            climate,
            extra,
            highlights,
            red,
        ));
    }
    let mut summary = summary(f);
    summary.push(format!(
        "{n_drops} data drops by room type; {aps} access points for coverage."
    ));
    MepProposal {
        discipline: Discipline::Technology,
        disclaimer: Discipline::Technology.disclaimer().into(),
        climate,
        systems: rank(v),
        assumptions: vec![
            "Data drops by room type and area (rules file).".into(),
            format!("One Wi-Fi access point per {:.0} sf; copper runs up to {:.0}' from their telecom room.", ap_sf, t.max_cable_ft),
            "Cameras and access readers at exterior doors; AV displays in conference rooms and classrooms.".into(),
        ],
        questions: vec![
            "Which carriers serve the site, and where does their service enter?".into(),
            "Is security (cameras, access control) part of the scope?".into(),
            "Are there special rooms: a server room, broadcast or AV studio?".into(),
        ],
        summary,
    }
}

pub fn layout(f: &Features, rules: &Rules, s: &MepSettings) -> MepLayout {
    let t = &rules.technology;
    let mut out = Out::new();
    let mut notes = vec![Discipline::Technology.disclaimer().to_string()];
    let Some(sr) = rules.systems(Discipline::Technology).get(&s.system) else {
        notes.push(format!("Unknown system {}.", s.system));
        return MepLayout {
            notes,
            ..Default::default()
        };
    };
    let Some((_, mdf, found)) = mdf_at(f) else {
        notes.push("Nothing to wire yet: add walls, floors and rooms.".into());
        return MepLayout {
            notes,
            ..Default::default()
        };
    };
    let lowest = &f.levels[0];
    let floor = |lv: &Level| (lv.elevation, lv.elevation + 2100.0);
    out.item(
        MepKind::Mdf,
        lowest,
        vec![mdf],
        floor(lowest),
        if sr.media_panel {
            "structured media panel"
        } else if sr.fiber {
            "optical line terminal (core room)"
        } else {
            "MDF: racks, patch panels, switches"
        },
        900.0,
        "In a telecom, electrical or utility room near the middle of the lowest floor.",
    );
    if !found && !sr.media_panel {
        out.flag(
            "No Telecom Room",
            Some(lowest.id),
            mdf,
            "No telecom or utility room found: the MDF needs a room (about 10' x 12').".into(),
        );
    }
    // A closet on each upper floor stacked over the MDF, with a backbone riser.
    let core = f
        .cores
        .first()
        .map(|c| Pt::new(c.1.x - 600.0, c.2.y + 600.0))
        .unwrap_or(mdf);
    let closets = sr.idf_per_floor || sr.fiber;
    if closets && f.levels.len() > 1 {
        out.item(
            MepKind::Backbone,
            lowest,
            vec![core],
            (
                lowest.elevation,
                f.levels.last().map_or(lowest.top, |l| l.top),
            ),
            if sr.fiber {
                "single-mode fiber riser"
            } else {
                "fiber + copper backbone riser"
            },
            300.0,
            "Stacked at the core from the MDF to each floor's closet.",
        );
        out.item(
            MepKind::Pathway,
            lowest,
            vec![mdf, Pt::new(mdf.x, core.y), core],
            (ceiling(rules, f, lowest), ceiling(rules, f, lowest) + 100.0),
            "12\" cable tray",
            300.0,
            "MDF to the riser.",
        );
    }
    let max = t.max_cable_ft * MM_PER_FT;
    let ap_sf = if f.residential {
        t.ap_sf_residential
    } else {
        t.ap_sf_commercial
    };
    let mut long = 0;
    for (li, lv) in f.levels.iter().enumerate() {
        let tr = if li == 0 {
            mdf
        } else if closets {
            out.item(
                MepKind::Idf,
                lv,
                vec![core],
                floor(lv),
                if sr.fiber {
                    "fiber zone terminal enclosure"
                } else {
                    "IDF: rack, patch panels, switch"
                },
                900.0,
                "Stacked over the one below at the core.",
            );
            core
        } else {
            mdf
        };
        let c = ceiling(rules, f, lv);
        let spaces: Vec<&Space> = f.spaces.iter().filter(|x| x.level == lv.id).collect();
        // Outlets along each room's walls.
        let mut room_pts = vec![];
        for sp in &spaces {
            let n = drops(rules, sp);
            if n == 0 {
                continue;
            }
            let perim: f64 = (0..sp.ring.len())
                .map(|i| sp.ring[i].dist(sp.ring[(i + 1) % sp.ring.len()]))
                .sum();
            let pts: Vec<Pt> = along_walls(&sp.ring, (perim / n as f64).max(600.0), 150.0)
                .into_iter()
                .take(n)
                .collect();
            for p in &pts {
                let run = manhattan(tr, *p)
                    + if li > 0 && !closets {
                        lv.elevation - lowest.elevation
                    } else {
                        0.0
                    }
                    + 3000.0;
                out.item(
                    MepKind::DataOutlet,
                    lv,
                    vec![*p],
                    (lv.elevation + 400.0, lv.elevation + 500.0),
                    if sr.fiber {
                        "fiber/copper outlet"
                    } else {
                        "2 x Cat6A outlet"
                    },
                    150.0,
                    format!(
                        "{} drop(s) for a {} (rules file).",
                        n,
                        sp.kind.to_lowercase()
                    ),
                );
                if run > max {
                    long += 1;
                    out.flag("Cable Too Long", Some(lv.id), *p, format!("About {} of cable from its telecom room: past {:.0}' — add a closet nearer.", ft(run), t.max_cable_ft));
                }
            }
            room_pts.push(sp.center);
            // AV in meeting and teaching rooms.
            if matches!(sp.kind.as_str(), "Conference" | "Classroom" | "Assembly") {
                let n = sp.ring.len();
                let (a, b) = (0..n)
                    .map(|i| (sp.ring[i], sp.ring[(i + 1) % n]))
                    .max_by(|x, y| x.0.dist(x.1).total_cmp(&y.0.dist(y.1)))
                    .unwrap_or((sp.center, sp.center));
                out.item(
                    MepKind::AvDisplay,
                    lv,
                    vec![a.add(b).scale(0.5)],
                    (lv.elevation + 1000.0, lv.elevation + 2000.0),
                    "display + conferencing bar",
                    600.0,
                    "On the room's longest wall.",
                );
            }
        }
        // Pathways from the telecom room to the rooms.
        let (trunk, branches) = trunk_and_branches(&lv.outline, tr, &room_pts);
        if trunk.len() >= 2 {
            out.item(
                MepKind::Pathway,
                lv,
                trunk,
                (c, c + 100.0),
                if sr.media_panel {
                    "conduit / J-hooks"
                } else {
                    "12\" cable tray"
                },
                if sr.media_panel { 0.0 } else { 300.0 },
                "Main pathway along the long axis.",
            );
        }
        for (a, b) in branches {
            out.item(
                MepKind::Pathway,
                lv,
                vec![a, b],
                (c, c + 50.0),
                "J-hooks",
                0.0,
                "Branch to a room.",
            );
        }
        // Wi-Fi coverage.
        let n = (lv.area / SQFT / ap_sf).ceil().max(1.0) as usize;
        let spacing = (lv.area / n as f64).sqrt();
        for mut p in grid_in(&lv.outline, spacing).into_iter().take(n) {
            // Clear of the telecom room's symbol.
            if p.dist(tr) < 1500.0 {
                p = p.add(Pt::new(2400.0, 0.0));
            }
            out.item(
                MepKind::AccessPoint,
                lv,
                vec![p],
                (c - 100.0, c),
                "Wi-Fi 6E access point (PoE)",
                300.0,
                format!("One per {ap_sf:.0} sf, spread over the floor."),
            );
        }
    }
    // Security at the exterior doors.
    if t.cameras_at_doors {
        for d in &f.exterior_doors {
            let Some(lv) = f.levels.iter().find(|l| l.id == d.level) else {
                continue;
            };
            out.item(
                MepKind::Camera,
                lv,
                vec![d.at.add(d.outward.scale(600.0))],
                (lv.elevation + 2700.0, lv.elevation + 2900.0),
                "exterior camera",
                250.0,
                "Covering an exterior door.",
            );
            if !f.residential {
                out.item(
                    MepKind::AccessControl,
                    lv,
                    vec![d.at.sub(d.outward.scale(400.0))],
                    (lv.elevation + 1000.0, lv.elevation + 1200.0),
                    "card reader + door contact",
                    200.0,
                    "Access control at an exterior door.",
                );
            }
        }
    }
    notes.push(format!(
        "{}: {} items, {} outlets beyond cable reach.",
        sr.label,
        out.items.len(),
        long
    ));
    MepLayout {
        items: out.items,
        zones: vec![],
        flags: out.flags,
        notes,
    }
}
