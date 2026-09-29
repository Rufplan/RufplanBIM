//! Mechanical (HVAC): cooling loads from rules of thumb (sf per ton by space type), six
//! candidate systems, and a preliminary layout: zones, equipment, outdoor units, shafts,
//! ducts or refrigerant runs, diffusers and indoor units.

use studio_core::mep::{Climate, Discipline, MepKind, MepLayout, MepSettings, MepZone};
use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_geom::Pt;

use crate::common::*;
use crate::features::SQFT;
use crate::rules::Rules;
use crate::types::*;

/// Cooling tons of a space (0 when unconditioned).
fn tons(rules: &Rules, s: &Space) -> f64 {
    let r = rules.space(&s.kind);
    if r.sf_per_ton <= 0.0 {
        0.0
    } else {
        s.area / SQFT / r.sf_per_ton
    }
}

fn conditioned<'a>(f: &'a Features, rules: &Rules) -> Vec<&'a Space> {
    f.spaces.iter().filter(|s| tons(rules, s) > 0.0).collect()
}

fn total_tons(f: &Features, rules: &Rules) -> f64 {
    let t: f64 = f.spaces.iter().map(|s| tons(rules, s)).sum();
    if t > 0.0 {
        t
    } else {
        f.area / SQFT / 450.0
    }
}

pub fn propose(f: &Features, rules: &Rules, climate: Climate) -> MepProposal {
    let m = &rules.mechanical;
    let t = total_tons(f, rules);
    let rooms = conditioned(f, rules);
    let plenum = f
        .levels
        .iter()
        .map(|l| (l.top - l.elevation) / MM_PER_FT - ceiling_ft(rules, f))
        .fold(f64::INFINITY, f64::min);
    let habitable: Vec<&&Space> = rooms.iter().filter(|s| s.kind != "Corridor").collect();
    let outside = if habitable.is_empty() {
        1.0
    } else {
        habitable.iter().filter(|s| s.exterior_len > 0.0).count() as f64 / habitable.len() as f64
    };
    let has_mech = f.spaces.iter().any(|s| s.kind == "Mechanical");
    let mut v = vec![];
    for (key, sr) in &m.systems {
        let mut extra = vec![];
        let mut red = vec![];
        if sr.ducted {
            let ok = plenum >= m.min_plenum_ft || !plenum.is_finite();
            extra.push(Criterion {
                name: "Ceiling space".into(),
                score: if ok { 1.0 } else { 0.45 },
                note: if ok {
                    "Room above the ceilings for ducts.".into()
                } else {
                    format!(
                        "Only about {plenum:.1}' above a {:.0}' ceiling for ducts.",
                        ceiling_ft(rules, f)
                    )
                },
            });
            if !ok {
                red.push(
                    "Tight floor-to-floor for ducts: lower ceilings, soffits or bulkheads.".into(),
                );
            }
        }
        if sr.exterior {
            extra.push(Criterion {
                name: "Outside walls".into(),
                score: outside,
                note: format!(
                    "{:.0}% of rooms have an outside wall for a unit.",
                    100.0 * outside
                ),
            });
            let inner = habitable.len() - habitable.iter().filter(|s| s.exterior_len > 0.0).count();
            if inner > 0 {
                red.push(format!("{inner} interior room{} without an outside wall need another way to be conditioned.", if inner == 1 { "" } else { "s" }));
            }
        }
        if sr.rooftop && key == "packaged_rtu" && f.stories > 2 {
            extra.push(Criterion {
                name: "Roof".into(),
                score: 0.6,
                note: "Rooftop units reach lower floors only through shafts.".into(),
            });
            red.push("Lower floors need supply and return shafts from the roof.".into());
        }
        if sr.central {
            extra.push(Criterion {
                name: "Plant room".into(),
                score: if has_mech { 1.0 } else { 0.6 },
                note: if has_mech {
                    "A mechanical room is in the model.".into()
                } else {
                    "No room named mechanical: a plant room of about 3–5% of the floor area is needed.".into()
                },
            });
        }
        // Zoning: one ducted system serves many rooms from one thermostat.
        let zones = rooms.len().max(1) as f64;
        let zoning = if sr.per_room || key == "vrf_doas" || key == "central_vav" {
            1.0
        } else {
            (1.0 - ((zones / f.stories.max(1) as f64) - 8.0).max(0.0) * 0.05).max(0.5)
        };
        extra.push(Criterion {
            name: "Zoning".into(),
            score: zoning,
            note: format!("{} conditioned rooms.", rooms.len()),
        });
        let highlights = vec![
            format!(
                "Cooling about {t:.1} tons ({:.0} sf per ton)",
                f.area / SQFT / t.max(0.1)
            ),
            match key.as_str() {
                "furnace_split" => format!(
                    "{} furnace/air handler{} of up to {:.0} tons with outdoor condensers",
                    (t / m.max_split_tons).ceil().max(f.stories as f64),
                    if t > m.max_split_tons || f.stories > 1 {
                        "s"
                    } else {
                        ""
                    },
                    m.max_split_tons
                ),
                "ductless_minisplit" => format!(
                    "{} indoor heads on {} outdoor condensers",
                    habitable.len(),
                    habitable.len().div_ceil(m.heads_per_condenser.max(1))
                ),
                "packaged_rtu" => format!(
                    "{} rooftop unit(s) of up to {:.0} tons",
                    (t / m.max_rtu_tons).ceil().max(1.0),
                    m.max_rtu_tons
                ),
                "vrf_doas" => format!(
                    "{} VRF condenser(s), {} indoor units, 1 DOAS",
                    (t / m.max_vrf_tons).ceil().max(1.0),
                    habitable.len()
                ),
                "central_vav" => format!(
                    "Chiller plant about {:.0} tons, an air handler per floor, VAV per zone",
                    t * 1.1
                ),
                _ => format!("{} through-wall units", habitable.len()),
            },
        ];
        if climate == Climate::Cold && (sr.per_room || key == "vrf_doas") {
            red.push("Cold climate: choose cold-climate heat pumps or add backup heat.".into());
        }
        v.push(score(
            f,
            key,
            sr,
            Discipline::Mechanical,
            climate,
            extra,
            highlights,
            red,
        ));
    }
    let mut summary = summary(f);
    summary.push(format!("Cooling about {t:.1} tons by rule of thumb."));
    let mut questions = vec![];
    if climate == Climate::Mixed {
        questions.push("Is the site's climate hot, mixed or cold?".into());
    }
    if !f.spaces.iter().any(|s| s.kind == "Mechanical") && f.area / SQFT > 5000.0 {
        questions.push("Where can mechanical equipment go: a room, the roof or the site?".into());
    }
    questions.push("Is natural gas available, or should everything be electric?".into());
    questions.truncate(3);
    MepProposal {
        discipline: Discipline::Mechanical,
        disclaimer: Discipline::Mechanical.disclaimer().into(),
        climate,
        systems: rank(v),
        assumptions: vec![
            format!("Climate: {} (your setting; no weather data looked up).", climate_label(climate)),
            "Cooling loads by square feet per ton for each room type (rules file), not a load calculation.".into(),
            format!("Ceilings at {:.0}'; ducts run above them.", ceiling_ft(rules, f)),
            "The roof can take equipment where a system puts it there.".into(),
        ],
        questions,
        summary,
    }
}

pub fn layout(f: &Features, rules: &Rules, s: &MepSettings) -> MepLayout {
    let m = &rules.mechanical;
    let mut out = Out::new();
    let mut zones = vec![];
    let mut notes = vec![Discipline::Mechanical.disclaimer().to_string()];
    let key = s.system.as_str();
    let sr = rules.systems(Discipline::Mechanical).get(key);
    let label = sr.map_or(key, |r| r.label.as_str());
    if f.levels.is_empty() {
        notes.push("Nothing to condition yet: add walls, floors and rooms.".into());
        return MepLayout {
            notes,
            ..Default::default()
        };
    }
    let t = total_tons(f, rules);
    notes.push(format!("{label}: about {t:.1} tons of cooling."));
    let cfm = |tn: f64| tn * m.supply_cfm_per_ton;
    let duct = |c: f64| {
        let w = (c / m.duct_velocity_fpm * 144.0 / m.duct_depth_in / 2.0)
            .ceil()
            .max(3.0)
            * 2.0;
        (
            format!("{w:.0}x{:.0}\" supply duct, {c:.0} cfm", m.duct_depth_in),
            w * MM_PER_IN,
        )
    };
    let top = f.levels.last().map_or(0.0, |l| l.top);
    let lowest = &f.levels[0];
    let mid = middle(f);
    // The vertical route: a stair core, else the mechanical room, else the middle.
    let core = f
        .cores
        .first()
        .map(|c| Pt::new(c.1.x - 600.0, (c.1.y + c.2.y) / 2.0))
        .or_else(|| room_of(f, None, &["Mechanical"], mid).map(|r| r.center))
        .unwrap_or(mid);
    for (i, z) in conditioned(f, rules).iter().enumerate() {
        zones.push(MepZone {
            level: z.level,
            ring: z.ring.clone(),
            label: format!("Z{} {} — {:.1} t", i + 1, z.name, tons(rules, z)),
        });
    }
    let per_room = key == "ductless_minisplit" || key == "ptac";
    for (li, lv) in f.levels.iter().enumerate() {
        let spaces: Vec<&Space> = f
            .spaces
            .iter()
            .filter(|x| x.level == lv.id && tons(rules, x) > 0.0)
            .collect();
        let lt: f64 = spaces.iter().map(|x| tons(rules, x)).sum();
        let c = ceiling(rules, f, lv);
        let floor = (lv.elevation, lv.elevation + 1800.0);
        if per_room {
            // A unit in each room on its outside wall; interior rooms flagged.
            let mut heads: Vec<Pt> = vec![];
            for sp in &spaces {
                let tn = tons(rules, sp);
                match sp.exterior_edge {
                    Some((a, b)) => {
                        let p =
                            inside_near(f, lv.id, a.add(b).scale(0.5), 250.0).unwrap_or(sp.center);
                        let (what, z) = if key == "ptac" {
                            (
                                format!(
                                    "PTAC {:.0} Btu/h",
                                    (tn * 12000.0 / 3000.0).ceil() * 3000.0
                                ),
                                (lv.elevation, lv.elevation + 450.0),
                            )
                        } else {
                            (
                                format!(
                                    "wall head {:.0} Btu/h",
                                    (tn * 12000.0 / 3000.0).ceil() * 3000.0
                                ),
                                (c - 400.0, c),
                            )
                        };
                        out.item(
                            MepKind::IndoorUnit,
                            lv,
                            vec![p],
                            z,
                            what,
                            900.0,
                            format!("In {} on its outside wall; {:.1} t by sf/ton.", sp.name, tn),
                        );
                        heads.push(p);
                    }
                    None => {
                        out.item(
                            MepKind::IndoorUnit,
                            lv,
                            vec![sp.center],
                            (c - 300.0, c),
                            "ceiling cassette (ducted to an outside wall)",
                            600.0,
                            "Interior room: no outside wall for a wall unit.",
                        );
                        out.flag("Interior Room", Some(lv.id), sp.center, format!("{} has no outside wall for a {}: use a ceiling cassette, ducted unit, or relocate.", sp.name, if key == "ptac" { "PTAC" } else { "wall head" }));
                        heads.push(sp.center);
                    }
                }
            }
            if key == "ductless_minisplit" {
                for group in heads.chunks(m.heads_per_condenser.max(1)) {
                    let gc = group
                        .iter()
                        .fold(Pt::new(0.0, 0.0), |s, p| s.add(*p))
                        .scale(1.0 / group.len() as f64);
                    let Some((cu, _)) = outside_near(f, lowest.id, gc, 900.0) else {
                        continue;
                    };
                    out.item(MepKind::OutdoorUnit, lowest, vec![cu], (lowest.elevation, lowest.elevation + 900.0), format!("multi-zone condenser, {} heads", group.len()), 900.0, "Outside the nearest exterior wall, up to the heads per condenser in the rules.");
                    for h in group {
                        out.item(
                            MepKind::Refrigerant,
                            lv,
                            vec![*h, Pt::new(h.x, cu.y), cu],
                            (c, c + 100.0),
                            "3/8\" + 5/8\" line set",
                            0.0,
                            "Line set from the head to its condenser.",
                        );
                    }
                }
            }
            continue;
        }
        // Ducted and VRF systems.
        let src = match key {
            "furnace_split" => room_of(f, Some(lv.id), &["Mechanical", "Laundry", "Storage", "Garage"], mid)
                .map(|r| r.center)
                .unwrap_or_else(|| {
                    out.flag("No Mechanical Space", Some(lv.id), mid, format!("No closet, laundry or garage on {} for the furnace/air handler: allow about 3' x 4'.", lv.name));
                    mid
                }),
            "central_vav" => room_of(f, Some(lv.id), &["Mechanical"], core).map_or(core, |r| r.center),
            _ => core,
        };
        if key == "furnace_split" {
            let units = (lt / m.max_split_tons).ceil().max(1.0);
            out.item(MepKind::Equipment, lv, vec![src], floor, format!("{units:.0} x {:.1}-ton furnace/air handler", (lt / units * 2.0).ceil() / 2.0), 900.0, "In a closet, laundry or garage near the middle; split above the largest size in the rules.");
            if let Some((cu, _)) = outside_near(f, lowest.id, src, 900.0) {
                out.item(
                    MepKind::OutdoorUnit,
                    lowest,
                    vec![cu],
                    (lowest.elevation, lowest.elevation + 900.0),
                    format!("{:.1}-ton condenser", (lt / units * 2.0).ceil() / 2.0),
                    900.0,
                    "Outside the exterior wall nearest the air handler.",
                );
                out.item(
                    MepKind::Refrigerant,
                    lv,
                    vec![src, Pt::new(src.x, cu.y), cu],
                    (floor.1, floor.1 + 100.0),
                    "line set",
                    0.0,
                    "Refrigerant lines to the condenser.",
                );
            }
        }
        if key == "central_vav" {
            if li == 0 {
                out.item(
                    MepKind::Equipment,
                    lv,
                    vec![src],
                    floor,
                    format!("chiller/boiler plant, about {:.0} tons", t * 1.1),
                    2400.0,
                    "Plant in the lowest mechanical room.",
                );
                out.item(
                    MepKind::Piping,
                    lv,
                    vec![core],
                    (lv.elevation, top),
                    "chilled and hot water risers",
                    300.0,
                    "Risers up the core to each floor's air handler and the roof.",
                );
                if !f.spaces.iter().any(|x| x.kind == "Mechanical") {
                    out.flag("No Plant Room", Some(lv.id), src, "No mechanical room in the model: a central plant needs about 3–5% of the floor area.".into());
                }
            }
            out.item(
                MepKind::Equipment,
                lv,
                vec![src.add(Pt::new(1200.0, 0.0))],
                floor,
                format!("air handler, {:.0} cfm", cfm(lt)),
                1800.0,
                "An air handler on each floor, VAV boxes per zone.",
            );
        }
        if f.levels.len() > 1 && key != "furnace_split" {
            out.item(
                MepKind::Shaft,
                lv,
                vec![core],
                (lv.elevation, lv.top),
                "mechanical shaft",
                1200.0,
                "Vertical route at the core for ducts, refrigerant or piping.",
            );
        }
        // Diffusers or indoor units in each room, fed by a trunk and branches.
        let mut terminals: Vec<(Pt, f64, String)> = vec![];
        for sp in &spaces {
            let tn = tons(rules, sp);
            if key == "vrf_doas" {
                let n = (tn / 1.5).ceil().max(1.0) as usize;
                let spacing = (sp.area / n as f64).sqrt();
                for p in crate::common::grid_in(&sp.ring, spacing)
                    .into_iter()
                    .take(n)
                {
                    out.item(
                        MepKind::IndoorUnit,
                        lv,
                        vec![p],
                        (c - 300.0, c),
                        format!(
                            "VRF cassette {:.0} Btu/h",
                            (tn / n as f64 * 12000.0 / 3000.0).ceil() * 3000.0
                        ),
                        600.0,
                        format!("{} at {:.1} t.", sp.name, tn),
                    );
                    terminals.push((p, tn / n as f64, sp.name.clone()));
                }
            } else {
                let rc = cfm(tn);
                let n = (rc / m.cfm_per_diffuser).ceil().max(1.0) as usize;
                let spacing = (sp.area / n as f64).sqrt();
                for p in crate::common::grid_in(&sp.ring, spacing)
                    .into_iter()
                    .take(n)
                {
                    out.item(
                        MepKind::Diffuser,
                        lv,
                        vec![p],
                        (c - 50.0, c),
                        format!("{:.0} cfm diffuser", rc / n as f64),
                        600.0,
                        format!(
                            "{} needs about {rc:.0} cfm; {} cfm per diffuser.",
                            sp.name, m.cfm_per_diffuser
                        ),
                    );
                    terminals.push((p, tn / n as f64, sp.name.clone()));
                }
            }
        }
        if key == "packaged_rtu" && li + 1 < f.levels.len() {
            out.flag(
                "Shaft Needed",
                Some(lv.id),
                core,
                format!(
                    "{} is below the roof: its rooftop unit reaches it through the shaft.",
                    lv.name
                ),
            );
        }
        let pts: Vec<Pt> = terminals.iter().map(|x| x.0).collect();
        let (trunk, branches) = trunk_and_branches(&lv.outline, src, &pts);
        if key == "vrf_doas" {
            if trunk.len() >= 2 {
                out.item(
                    MepKind::Refrigerant,
                    lv,
                    trunk.clone(),
                    (c, c + 150.0),
                    "refrigerant main + branch selector",
                    0.0,
                    "VRF main along the long axis from the shaft.",
                );
                let (s, w) = duct(lt * 50.0);
                out.item(
                    MepKind::SupplyDuct,
                    lv,
                    trunk,
                    (c + 150.0, c + 150.0 + m.duct_depth_in * MM_PER_IN),
                    format!("ventilation {s}"),
                    w,
                    "Outdoor air from the DOAS down the shaft.",
                );
            }
            for (a, b) in branches {
                out.item(
                    MepKind::Refrigerant,
                    lv,
                    vec![a, b],
                    (c, c + 100.0),
                    "branch line set",
                    0.0,
                    "Branch to an indoor unit.",
                );
            }
        } else {
            if trunk.len() >= 2 {
                let (s, w) = duct(cfm(lt));
                out.item(
                    MepKind::SupplyDuct,
                    lv,
                    trunk,
                    (c, c + m.duct_depth_in * MM_PER_IN),
                    s,
                    w,
                    "Supply trunk along the long axis, sized at the rules' velocity.",
                );
            }
            for ((a, b), tt) in branches
                .into_iter()
                .zip(std::iter::once(lt).chain(terminals.iter().map(|x| x.1)))
            {
                let (s, w) = duct(cfm(tt));
                out.item(
                    MepKind::SupplyDuct,
                    lv,
                    vec![a, b],
                    (c, c + m.duct_depth_in * MM_PER_IN),
                    s,
                    w,
                    "Branch to a diffuser.",
                );
            }
            out.item(
                MepKind::ReturnGrille,
                lv,
                vec![src.add(Pt::new(0.0, 1500.0))],
                (c - 50.0, c),
                format!("{:.0} cfm return", cfm(lt)),
                900.0,
                "Central return near the equipment.",
            );
        }
    }
    // Roof equipment.
    let roof = f.levels.last().unwrap_or(lowest);
    let roof_z = (top, top + 1500.0);
    match key {
        "packaged_rtu" => {
            let n = (t / m.max_rtu_tons).ceil().max(1.0) as usize;
            let spacing = (roof.area / n as f64).sqrt();
            for p in crate::common::grid_in(&roof.outline, spacing)
                .into_iter()
                .take(n)
            {
                out.item(
                    MepKind::Equipment,
                    roof,
                    vec![p],
                    roof_z,
                    format!("{:.1}-ton rooftop unit", (t / n as f64 * 2.0).ceil() / 2.0),
                    2400.0,
                    "Spread over the roof, up to the largest RTU in the rules.",
                );
            }
        }
        "vrf_doas" => {
            let n = (t / m.max_vrf_tons).ceil().max(1.0) as usize;
            let spacing = (roof.area / (n + 1) as f64).sqrt();
            let pts = crate::common::grid_in(&roof.outline, spacing);
            for p in pts.iter().take(n) {
                out.item(
                    MepKind::OutdoorUnit,
                    roof,
                    vec![*p],
                    roof_z,
                    format!("{:.0}-ton VRF condenser", (t / n as f64).ceil()),
                    1500.0,
                    "On the roof, up to the largest VRF module in the rules.",
                );
            }
            let d = pts.get(n).copied().unwrap_or(core);
            out.item(
                MepKind::Equipment,
                roof,
                vec![d],
                roof_z,
                format!("DOAS, about {:.0} cfm outdoor air", t * 50.0),
                2400.0,
                "Dedicated outdoor air unit on the roof, ducted down the shaft.",
            );
        }
        "central_vav" => {
            out.item(
                MepKind::OutdoorUnit,
                roof,
                vec![core.add(Pt::new(3000.0, 0.0))],
                roof_z,
                format!("cooling tower, about {:.0} tons", t * 1.25),
                3000.0,
                "Cooling tower on the roof above the plant riser.",
            );
        }
        _ => {}
    }
    notes.push(format!(
        "{} items, {} zones, {} flags.",
        out.items.len(),
        zones.len(),
        out.flags.len()
    ));
    MepLayout {
        items: out.items,
        zones,
        flags: out.flags,
        notes,
    }
}
