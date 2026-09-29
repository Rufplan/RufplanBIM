//! Electrical: connected load by rules of thumb (lighting, receptacles, HVAC, dwelling
//! appliances), the service each candidate needs, and a preliminary layout: service
//! entrance, panels, transformers, feeders, light fixtures and receptacles.

use studio_core::mep::{Climate, Discipline, MepKind, MepLayout, MepSettings};
use studio_core::units::MM_PER_FT;
use studio_geom::Pt;

use crate::common::*;
use crate::features::SQFT;
use crate::rules::{Rules, SystemRules};
use crate::types::*;

/// Connected load, VA: lighting, receptacles, HVAC, appliances.
fn loads(f: &Features, rules: &Rules) -> (f64, f64, f64, f64) {
    let e = &rules.electrical;
    let mut light = 0.0;
    let mut recept = 0.0;
    for s in &f.spaces {
        let r = rules.space(&s.kind);
        light += s.area / SQFT * r.light_w_sf;
        recept += s.area / SQFT * r.recept_va_sf;
    }
    if f.spaces.is_empty() {
        light = f.area / SQFT * 0.8;
        recept = f.area / SQFT * 1.5;
    }
    let hvac = f.area / SQFT * e.hvac_va_sf;
    let appl = f.dwelling_units as f64 * e.dwelling_appliances_va;
    (light, recept, hvac, appl)
}

fn demand(f: &Features, rules: &Rules) -> f64 {
    let (l, r, h, a) = loads(f, rules);
    (l + r + h + a) * rules.electrical.demand_factor
}

/// The service amps for `sr`: demand plus 25%, rounded up to a standard size.
fn service(rules: &Rules, sr: &SystemRules, demand_va: f64) -> f64 {
    let v = if sr.phases == 3 {
        sr.volts * 3f64.sqrt()
    } else {
        sr.volts
    };
    let need = demand_va * 1.25 / v.max(1.0);
    rules
        .electrical
        .service_sizes
        .iter()
        .copied()
        .find(|s| *s >= need)
        .unwrap_or(need.ceil())
}

fn volts_label(sr: &SystemRules) -> String {
    if sr.phases == 3 {
        if sr.volts >= 400.0 {
            "480Y/277 V 3-phase".into()
        } else {
            "208Y/120 V 3-phase".into()
        }
    } else {
        "120/240 V 1-phase".into()
    }
}

pub fn propose(f: &Features, rules: &Rules, climate: Climate) -> MepProposal {
    let d = demand(f, rules);
    let (l, r, h, a) = loads(f, rules);
    let has_room = f.spaces.iter().any(|s| s.kind == "Electrical");
    let mut v = vec![];
    for (key, sr) in &rules.electrical.systems {
        let amps = service(rules, sr, d);
        let mut extra = vec![];
        let mut red = vec![];
        let fit = if amps < sr.min_amps {
            0.6
        } else if amps > sr.max_amps {
            (1.0 - (amps - sr.max_amps) / sr.max_amps).max(0.2)
        } else {
            1.0
        };
        extra.push(Criterion {
            name: "Service size".into(),
            score: fit,
            note: format!(
                "About {amps:.0} A at {} ({:.0}–{:.0} A is its range).",
                volts_label(sr),
                sr.min_amps,
                sr.max_amps
            ),
        });
        let multi = f.dwelling_units >= 3;
        extra.push(Criterion {
            name: "Metering".into(),
            score: match (sr.meters, multi) {
                (true, true) | (false, false) => 1.0,
                (true, false) => 0.3,
                (false, true) => 0.4,
            },
            note: if multi {
                format!(
                    "{} dwellings, each wanting its own meter.",
                    f.dwelling_units
                )
            } else {
                "One meter for the building.".into()
            },
        });
        if amps > 800.0 {
            red.push("Over 800 A: a switchgear room and a utility transformer pad.".into());
        }
        if !has_room && f.area / SQFT > 10000.0 {
            red.push("No electrical room in the model for the switchboard and panels.".into());
        }
        let highlights = vec![
            format!(
                "Connected {:.0} kVA (lighting {:.0}, receptacles {:.0}, HVAC {:.0}, appliances {:.0}); demand about {:.0} kVA",
                (l + r + h + a) / 1000.0,
                l / 1000.0,
                r / 1000.0,
                h / 1000.0,
                a / 1000.0,
                d / 1000.0
            ),
            format!("{amps:.0} A service at {}", volts_label(sr)),
            if sr.meters {
                format!("Meter center + {} unit panels", f.dwelling_units.max(1))
            } else {
                if f.stories > 1 {
                    format!("Main panel + {} branch panel(s)", f.stories - 1)
                } else {
                    "Main panel".into()
                }
            },
        ];
        v.push(score(
            f,
            key,
            sr,
            Discipline::Electrical,
            climate,
            extra,
            highlights,
            red,
        ));
    }
    let mut summary = summary(f);
    summary.push(format!(
        "Demand load about {:.0} kVA by rule of thumb.",
        d / 1000.0
    ));
    let mut questions =
        vec!["Is the building all-electric, or is there gas for heating and cooking?".to_string()];
    if f.dwelling_units >= 2 {
        questions.push("Should each dwelling be metered separately?".into());
    }
    questions.push("Is standby power (a generator) wanted?".into());
    questions.truncate(3);
    MepProposal {
        discipline: Discipline::Electrical,
        disclaimer: Discipline::Electrical.disclaimer().into(),
        climate,
        systems: rank(v),
        assumptions: vec![
            "Loads by watts or VA per square foot for each room type, a flat HVAC allowance and appliances per dwelling (rules file), not a code load calculation.".into(),
            format!("A {:.0}% demand factor and 25% spare capacity.", rules.electrical.demand_factor * 100.0),
            "Utility service at the side of the building nearest the garage or utility rooms.".into(),
        ],
        questions,
        summary,
    }
}

pub fn layout(f: &Features, rules: &Rules, s: &MepSettings) -> MepLayout {
    let e = &rules.electrical;
    let mut out = Out::new();
    let mut notes = vec![Discipline::Electrical.disclaimer().to_string()];
    let Some(sr) = rules.systems(Discipline::Electrical).get(&s.system) else {
        notes.push(format!("Unknown system {}.", s.system));
        return MepLayout {
            notes,
            ..Default::default()
        };
    };
    if f.levels.is_empty() {
        notes.push("Nothing to power yet: add walls, floors and rooms.".into());
        return MepLayout {
            notes,
            ..Default::default()
        };
    }
    let d = demand(f, rules);
    let amps = service(rules, sr, d);
    notes.push(format!(
        "{}: {amps:.0} A at {}, demand about {:.0} kVA.",
        sr.label,
        volts_label(sr),
        d / 1000.0
    ));
    let lowest = &f.levels[0];
    let mid = middle(f);
    // Service: outside the wall nearest the garage, utility or electrical room.
    let near = room_of(
        f,
        Some(lowest.id),
        &["Electrical", "Garage", "Laundry", "Mechanical", "Storage"],
        mid,
    );
    let target = near.map_or(mid, |r| r.center);
    let Some((svc, _)) = outside_near(f, lowest.id, target, 300.0) else {
        notes.push("No exterior walls to bring the service in through.".into());
        return MepLayout {
            notes,
            ..Default::default()
        };
    };
    let floor = |lv: &Level| (lv.elevation + 900.0, lv.elevation + 1900.0);
    out.item(
        MepKind::Service,
        lowest,
        vec![svc],
        floor(lowest),
        if sr.meters {
            format!("{amps:.0} A meter center, {}", volts_label(sr))
        } else {
            format!("{amps:.0} A meter/service, {}", volts_label(sr))
        },
        600.0,
        "At the exterior wall nearest the garage, utility or electrical room.",
    );
    let main = inside_near(f, lowest.id, svc, 450.0).unwrap_or(svc);
    out.item(
        MepKind::Panel,
        lowest,
        vec![main],
        floor(lowest),
        if amps > 400.0 {
            format!("{amps:.0} A switchboard")
        } else {
            format!("{amps:.0} A main panel")
        },
        600.0,
        "Just inside from the service, with working clearance in front.",
    );
    if sr.transformers {
        out.item(
            MepKind::Transformer,
            lowest,
            vec![main.add(Pt::new(1500.0, 0.0))],
            (lowest.elevation, lowest.elevation + 1500.0),
            format!(
                "{:.0} kVA 480–208Y/120 V transformer",
                (f.area / SQFT * 2.5 / 1000.0 / 15.0).ceil() * 15.0
            ),
            1200.0,
            "Steps 480 V down for receptacles.",
        );
    }
    if amps > 800.0 {
        out.flag(
            "Switchgear Room",
            Some(lowest.id),
            main,
            format!(
                "{amps:.0} A service: a dedicated switchgear room and a transformer pad outside."
            ),
        );
    }
    if !f.spaces.iter().any(|x| x.kind == "Electrical") && f.area / SQFT > 10000.0 {
        out.flag(
            "No Electrical Room",
            Some(lowest.id),
            main,
            "No room named electrical: panels need rooms with clearances on each floor.".into(),
        );
    }
    let core = f
        .cores
        .first()
        .map(|c| Pt::new(c.2.x + 600.0, (c.1.y + c.2.y) / 2.0))
        .unwrap_or(mid);
    // Panels: a unit panel per dwelling, or a branch panel per upper floor.
    if sr.meters {
        for k in f.spaces.iter().filter(|x| x.kind == "Kitchen") {
            let Some(lv) = f.levels.iter().find(|l| l.id == k.level) else {
                continue;
            };
            let p = k.center;
            out.item(
                MepKind::Panel,
                lv,
                vec![p],
                floor(lv),
                "100 A unit panel",
                400.0,
                format!("One per dwelling, at its kitchen ({}).", k.name),
            );
            out.item(
                MepKind::Feeder,
                lv,
                vec![
                    if lv.id == lowest.id { svc } else { core },
                    Pt::new(p.x, core.y),
                    p,
                ],
                floor(lv),
                "unit feeder",
                0.0,
                "From the meter center (up the riser) to the unit panel.",
            );
        }
    } else {
        for lv in f.levels.iter().skip(1) {
            let p = room_of(f, Some(lv.id), &["Electrical", "Storage", "Laundry"], core)
                .map_or(core, |r| r.center);
            let lt: f64 = f
                .spaces
                .iter()
                .filter(|x| x.level == lv.id)
                .map(|x| x.area)
                .sum();
            out.item(
                MepKind::Panel,
                lv,
                vec![p],
                floor(lv),
                format!(
                    "{:.0} A branch panel",
                    ((lt / SQFT * 6.0 / 208.0) / 25.0).ceil().max(4.0) * 25.0
                ),
                400.0,
                format!("A branch panel on {}.", lv.name),
            );
            out.item(
                MepKind::Feeder,
                lv,
                vec![core, p],
                floor(lv),
                "feeder",
                0.0,
                "Up the riser at the core, then to the panel.",
            );
            let run = main.dist(core) + (lv.elevation - lowest.elevation).abs() + core.dist(p);
            if run > e.max_feeder_ft * MM_PER_FT {
                out.flag(
                    "Long Feeder",
                    Some(lv.id),
                    p,
                    format!(
                        "About {} of feeder to this panel: check voltage drop or upsize.",
                        ft(run)
                    ),
                );
            }
        }
        if f.levels.len() > 1 {
            out.item(
                MepKind::Feeder,
                lowest,
                vec![main, Pt::new(main.x, core.y), core],
                floor(lowest),
                "feeder riser",
                0.0,
                "From the main panel to the riser at the core.",
            );
        }
    }
    // Lights and receptacles, room by room.
    for sp in &f.spaces {
        let Some(lv) = f.levels.iter().find(|l| l.id == sp.level) else {
            continue;
        };
        let r = rules.space(&sp.kind);
        let c = ceiling(rules, f, lv);
        let n = (sp.area / SQFT / r.sf_per_light).ceil().max(1.0) as usize;
        let spacing = (sp.area / n as f64).sqrt();
        for p in grid_in(&sp.ring, spacing).into_iter().take(n) {
            out.item(
                MepKind::Light,
                lv,
                vec![p],
                (c - 100.0, c),
                format!(
                    "LED fixture, {:.0} W",
                    sp.area / SQFT * r.light_w_sf / n as f64
                ),
                400.0,
                format!(
                    "{} W/sf in {} ({:.0} sf per fixture).",
                    r.light_w_sf,
                    sp.kind.to_lowercase(),
                    r.sf_per_light
                ),
            );
        }
        if r.recept_va_sf <= 0.0 {
            continue;
        }
        let spacing = if f.residential {
            e.receptacle_spacing_ft
        } else {
            e.commercial_receptacle_spacing_ft
        } * MM_PER_FT;
        let wet = matches!(
            sp.kind.as_str(),
            "Bathroom" | "Restroom" | "Kitchen" | "Laundry" | "Garage" | "Break Room"
        );
        for p in along_walls(&sp.ring, spacing, 150.0) {
            out.item(
                MepKind::Receptacle,
                lv,
                vec![p],
                (lv.elevation + 400.0, lv.elevation + 500.0),
                if wet {
                    "GFCI duplex receptacle"
                } else {
                    "duplex receptacle"
                },
                150.0,
                if wet {
                    "Wet location: GFCI protected."
                } else {
                    "Along the walls, spaced per the rules."
                },
            );
        }
    }
    notes.push(format!(
        "{} items, {} flags.",
        out.items.len(),
        out.flags.len()
    ));
    MepLayout {
        items: out.items,
        zones: vec![],
        flags: out.flags,
        notes,
    }
}
