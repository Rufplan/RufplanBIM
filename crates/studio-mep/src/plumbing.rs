//! Plumbing: fixtures from room types, fixture units, wet rooms grouped into stacks that
//! line up floor to floor, the water heating each candidate uses, and a preliminary layout:
//! fixtures, waste/vent stacks, the building drain, water service, cold and hot mains.

use studio_core::mep::{Climate, Discipline, MepKind, MepLayout, MepSettings};
use studio_core::units::MM_PER_FT;
use studio_core::ElementId;
use studio_geom::Pt;

use crate::common::*;
use crate::rules::{pick_size, Rules};
use crate::types::*;

fn fixtures<'a>(rules: &'a Rules, s: &Space) -> &'a [String] {
    &rules.space(&s.kind).fixtures
}

fn wet<'a>(f: &'a Features, rules: &Rules) -> Vec<&'a Space> {
    f.spaces
        .iter()
        .filter(|s| !fixtures(rules, s).is_empty())
        .collect()
}

/// Water supply and drainage fixture units of the whole building.
fn units(f: &Features, rules: &Rules) -> (f64, f64, usize) {
    let (mut ws, mut df, mut n) = (0.0, 0.0, 0);
    for s in wet(f, rules) {
        for x in fixtures(rules, s) {
            if let Some(u) = rules.fixtures.get(x) {
                ws += u.0;
                df += u.1;
                n += 1;
            }
        }
    }
    (ws, df, n)
}

/// Wet rooms grouped per level (within the cluster distance), each group's centre.
fn clusters(f: &Features, rules: &Rules) -> Vec<(ElementId, Pt, Vec<ElementId>)> {
    let d = rules.plumbing.stack_cluster_ft * MM_PER_FT;
    let mut out: Vec<(ElementId, Pt, Vec<ElementId>)> = vec![];
    for s in wet(f, rules) {
        match out
            .iter_mut()
            .find(|c| c.0 == s.level && c.1.dist(s.center) <= d)
        {
            Some(c) => {
                let n = c.2.len() as f64;
                c.1 =
                    c.1.scale(n / (n + 1.0))
                        .add(s.center.scale(1.0 / (n + 1.0)));
                c.2.push(s.id);
            }
            None => out.push((s.level, s.center, vec![s.id])),
        }
    }
    out
}

/// Where the water heater goes for a heater kind (None for point-of-use).
fn heater_at(f: &Features, heater: &str) -> Option<(ElementId, Pt)> {
    let lowest = f.levels.first()?;
    let mid = middle(f);
    match heater {
        "pou" => None,
        "tankless" => {
            let k = room_of(
                f,
                Some(lowest.id),
                &["Kitchen", "Laundry", "Bathroom", "Restroom"],
                mid,
            );
            let to = k.map_or(mid, |k| k.center);
            Some((
                lowest.id,
                inside_near(f, lowest.id, to, 300.0).unwrap_or(to),
            ))
        }
        "central" => Some((
            lowest.id,
            room_of(
                f,
                Some(lowest.id),
                &["Mechanical", "Laundry", "Storage"],
                mid,
            )
            .map_or(mid, |r| r.center),
        )),
        _ => Some((
            lowest.id,
            room_of(
                f,
                Some(lowest.id),
                &["Mechanical", "Laundry", "Garage", "Storage"],
                mid,
            )
            .map_or(mid, |r| r.center),
        )),
    }
}

fn manhattan(a: Pt, b: Pt) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

pub fn propose(f: &Features, rules: &Rules, climate: Climate) -> MepProposal {
    let p = &rules.plumbing;
    let (ws, df, n) = units(f, rules);
    let baths = f
        .spaces
        .iter()
        .filter(|s| matches!(s.kind.as_str(), "Bathroom" | "Restroom"))
        .count();
    let cl = clusters(f, rules);
    let unstacked = cl
        .iter()
        .filter(|c| {
            let li = f.levels.iter().position(|l| l.id == c.0).unwrap_or(0);
            li > 0
                && !cl.iter().any(|b| {
                    Some(b.0) == f.levels.get(li - 1).map(|l| l.id)
                        && b.1.dist(c.1) <= p.stack_cluster_ft * MM_PER_FT
                })
        })
        .count();
    let mut v = vec![];
    for (key, sr) in &p.systems {
        let mut extra = vec![];
        let mut red = vec![];
        let units_ok = f.dwelling_units.max(1) >= sr.min_units.max(1)
            && (sr.max_units == 0 || f.dwelling_units.max(1) <= sr.max_units);
        extra.push(Criterion {
            name: "Dwellings".into(),
            score: if units_ok { 1.0 } else { 0.4 },
            note: if f.residential {
                format!(
                    "{} dwelling(s); it suits {}.",
                    f.dwelling_units,
                    match (sr.min_units, sr.max_units) {
                        (0 | 1, 0) => "any number".to_string(),
                        (0 | 1, m) => format!("up to {m}"),
                        (a, 0) => format!("{a} or more"),
                        (a, b) => format!("{a}–{b}"),
                    }
                )
            } else {
                "Not residential.".into()
            },
        });
        let reach = heater_at(f, &sr.heater)
            .map(|(_, h)| {
                wet(f, rules)
                    .iter()
                    .map(|s| manhattan(h, s.center))
                    .fold(0.0, f64::max)
            })
            .unwrap_or(0.0)
            + (f.levels.last().map_or(0.0, |l| l.elevation)
                - f.levels.first().map_or(0.0, |l| l.elevation));
        let long = reach > p.recirc_ft * MM_PER_FT;
        extra.push(Criterion {
            name: "Hot water reach".into(),
            score: if sr.heater == "pou" || sr.heater == "central" || !long {
                1.0
            } else {
                0.7
            },
            note: if sr.heater == "pou" {
                "Heaters at each fixture group: hot water at once.".into()
            } else {
                format!(
                    "Farthest fixture about {} of pipe from the heater.",
                    ft(reach)
                )
            },
        });
        if long && sr.heater != "pou" && sr.heater != "central" {
            red.push(format!(
                "Hot water runs past {:.0}': a recirculation loop or a second heater.",
                p.recirc_ft
            ));
        }
        if unstacked > 0 {
            red.push(format!("{unstacked} upper-floor wet group(s) don't stack over one below: horizontal drain offsets."));
        }
        let heater = match sr.heater.as_str() {
            "tank" | "heatpump" => format!(
                "{:.0}-gallon {} water heater",
                p.tank_gallons_first_bath + p.tank_gallons_per_bath * (baths.max(1) - 1) as f64,
                if sr.heater == "tank" {
                    "tank"
                } else {
                    "heat pump"
                }
            ),
            "tankless" => format!(
                "{} tankless heater(s), about {:.0} gpm",
                baths.div_ceil(3).max(1),
                2.5 * (baths.clamp(1, 3) as f64)
            ),
            "central" => format!(
                "central plant, about {:.0} gallons storage",
                20.0 * f.dwelling_units.max(1) as f64 + 40.0
            ),
            _ => format!("{} point-of-use heater(s)", cl.len().max(1)),
        };
        let highlights = vec![
            format!("{n} fixtures: {ws:.0} supply fixture units, {df:.0} drainage fixture units"),
            format!(
                "{} water service, {} building drain",
                pick_size(&p.service_sizes, ws),
                pick_size(&p.drain_sizes, df)
            ),
            heater,
        ];
        v.push(score(
            f,
            key,
            sr,
            Discipline::Plumbing,
            climate,
            extra,
            highlights,
            red,
        ));
    }
    let mut summary = summary(f);
    summary.push(format!(
        "{} wet rooms in {} group(s); {n} fixtures.",
        wet(f, rules).len(),
        cl.len()
    ));
    let mut questions = vec!["Is natural gas available for water heating?".to_string()];
    questions.push("Which side of the building does the sewer and water main come from?".into());
    if f.residential {
        questions.push("Should each dwelling have its own water heater?".into());
    }
    questions.truncate(3);
    MepProposal {
        discipline: Discipline::Plumbing,
        disclaimer: Discipline::Plumbing.disclaimer().into(),
        climate,
        systems: rank(v),
        assumptions: vec![
            "Fixtures by room type (rules file): each bathroom a water closet, lavatory and shower; each kitchen a sink and dishwasher…".into(),
            "Fixture units and pipe sizes from simplified tables in the rules file.".into(),
            "The sewer and water service come from the south side (lowest y) of the building.".into(),
        ],
        questions,
        summary,
    }
}

pub fn layout(f: &Features, rules: &Rules, s: &MepSettings) -> MepLayout {
    let p = &rules.plumbing;
    let mut out = Out::new();
    let mut notes = vec![Discipline::Plumbing.disclaimer().to_string()];
    let Some(sr) = rules.systems(Discipline::Plumbing).get(&s.system) else {
        notes.push(format!("Unknown system {}.", s.system));
        return MepLayout {
            notes,
            ..Default::default()
        };
    };
    if f.levels.is_empty() || wet(f, rules).is_empty() {
        notes.push(
            "No bathrooms, kitchens or other wet rooms to plumb yet (rooms are read by name)."
                .into(),
        );
        return MepLayout {
            notes,
            ..Default::default()
        };
    }
    let (ws, df, _) = units(f, rules);
    notes.push(format!(
        "{}: {ws:.0} WSFU, {df:.0} DFU; {} water service, {} building drain.",
        sr.label,
        pick_size(&p.service_sizes, ws),
        pick_size(&p.drain_sizes, df)
    ));
    let level = |id: ElementId| f.levels.iter().find(|l| l.id == id);
    // Fixtures along each wet room's inside wall nearest its group's centre.
    let cl = clusters(f, rules);
    // Each wet room's fixture wall, where its stack goes.
    let mut wall_at: std::collections::HashMap<ElementId, Pt> = Default::default();
    for sp in wet(f, rules) {
        let Some(lv) = level(sp.level) else { continue };
        let centre = cl
            .iter()
            .find(|c| c.2.contains(&sp.id))
            .map_or(sp.center, |c| c.1);
        let n = sp.ring.len();
        let edge = (0..n)
            .map(|i| (sp.ring[i], sp.ring[(i + 1) % n]))
            .filter(|(a, b)| a.dist(*b) > 600.0)
            .min_by(|x, y| {
                x.0.add(x.1)
                    .scale(0.5)
                    .dist(centre)
                    .total_cmp(&y.0.add(y.1).scale(0.5).dist(centre))
            });
        let Some((a, b)) = edge else { continue };
        let list = fixtures(rules, sp);
        let d = b.sub(a).scale(1.0 / a.dist(b));
        let mut nrm = d.perp();
        if sp.center.sub(a).dot(nrm) < 0.0 {
            nrm = nrm.scale(-1.0);
        }
        wall_at.insert(sp.id, a.add(b).scale(0.5).add(nrm.scale(150.0)));
        let step = a.dist(b) / (list.len() as f64 + 1.0);
        for (k, name) in list.iter().enumerate() {
            let at = a
                .add(d.scale(step * (k as f64 + 1.0)))
                .add(nrm.scale(350.0));
            let u = rules.fixtures.get(name).copied().unwrap_or((1.0, 1.0, 1.0));
            out.item(
                MepKind::Fixture,
                lv,
                vec![at],
                (lv.elevation, lv.elevation + 900.0),
                format!("{name} ({:.1} WSFU, {:.0} DFU)", u.0, u.1),
                500.0,
                format!("Fixtures for a {} (rules file).", sp.kind.to_lowercase()),
            );
        }
    }
    // Stacks: each group of wet rooms, lined up with the group below when there is one.
    let mut stacks: Vec<(ElementId, Pt, f64)> = vec![];
    for (li, lv) in f.levels.iter().enumerate() {
        for c in cl.iter().filter(|c| c.0 == lv.id) {
            let below = li
                .checked_sub(1)
                .and_then(|i| f.levels.get(i))
                .and_then(|b| {
                    stacks
                        .iter()
                        .filter(|s| s.0 == b.id)
                        .find(|s| s.1.dist(c.1) <= p.stack_cluster_ft * MM_PER_FT)
                });
            let wall =
                c.2.iter()
                    .find_map(|id| wall_at.get(id).copied())
                    .unwrap_or(c.1);
            let at = below.map_or(wall, |b| b.1);
            if li > 0 && below.is_none() {
                out.flag("Stack Offset", Some(lv.id), c.1, format!("The wet rooms here on {} have none below: the drain runs horizontally to a stack.", lv.name));
            }
            let dfu: f64 =
                c.2.iter()
                    .filter_map(|id| f.spaces.iter().find(|x| x.id == *id))
                    .flat_map(|x| fixtures(rules, x).iter())
                    .filter_map(|n| rules.fixtures.get(n).map(|u| u.1))
                    .sum();
            stacks.push((lv.id, at, dfu));
        }
    }
    let top = f.levels.last().map_or(0.0, |l| l.top);
    for (lid, at, dfu) in &stacks {
        let Some(lv) = level(*lid) else { continue };
        out.item(
            MepKind::Stack,
            lv,
            vec![*at],
            (lv.elevation, lv.top),
            format!("{} waste/vent stack", pick_size(&p.stack_sizes, *dfu)),
            150.0,
            format!("{dfu:.0} DFU on this floor; sized by the stack table."),
        );
        let is_top = !stacks
            .iter()
            .any(|s| s.1.dist(*at) < 1.0 && level(s.0).is_some_and(|l| l.elevation > lv.elevation));
        if is_top {
            out.item(
                MepKind::Vent,
                lv,
                vec![*at],
                (lv.top, top + 600.0),
                "vent through roof",
                100.0,
                "The stack vents through the roof.",
            );
        }
    }
    // The building drain and water service leave from the south side.
    let lowest = &f.levels[0];
    let street = f
        .exterior_walls
        .iter()
        .filter(|w| w.level == lowest.id)
        .min_by(|a, b| (a.start.y + a.end.y).total_cmp(&(b.start.y + b.end.y)));
    let exit_y = street.map_or(f.min.y, |w| w.start.y.min(w.end.y)) - 1500.0;
    let base = (lowest.elevation - 600.0, lowest.elevation - 300.0);
    for (lid, at, _) in stacks.iter().filter(|s| s.0 == lowest.id) {
        let _ = lid;
        out.item(
            MepKind::BuildingDrain,
            lowest,
            vec![*at, Pt::new(at.x, exit_y)],
            base,
            format!("{} building drain, 1/4\"/ft", pick_size(&p.drain_sizes, df)),
            150.0,
            "From the stack out to the sewer on the street side.",
        );
    }
    let heater = heater_at(f, &sr.heater);
    let entry_to = heater.map_or(middle(f), |h| h.1);
    let entry = Pt::new(entry_to.x, exit_y);
    out.item(
        MepKind::WaterService,
        lowest,
        vec![entry],
        (lowest.elevation - 900.0, lowest.elevation),
        format!("{} water service + meter", pick_size(&p.service_sizes, ws)),
        400.0,
        "Enters on the street side.",
    );
    // Water heating.
    let mut hot_sources: Vec<(ElementId, Pt)> = vec![];
    match heater {
        Some((lid, at)) => {
            let what = match sr.heater.as_str() {
                "tank" => "tank water heater",
                "heatpump" => "heat pump water heater",
                "tankless" => "tankless water heater (on outside wall)",
                _ => "central water heating plant + recirculation pump",
            };
            if let Some(lv) = level(lid) {
                out.item(
                    MepKind::WaterHeater,
                    lv,
                    vec![at],
                    (lv.elevation, lv.elevation + 1600.0),
                    what,
                    700.0,
                    "In a utility space on the lowest floor.",
                );
                out.item(
                    MepKind::ColdWater,
                    lv,
                    vec![entry, Pt::new(entry.x, at.y), at],
                    (lv.elevation + 2400.0, lv.elevation + 2450.0),
                    "cold water main",
                    0.0,
                    "From the service to the heater.",
                );
            }
            hot_sources.push((lid, at));
        }
        None => {
            for c in &cl {
                if let Some(lv) = level(c.0) {
                    let at = c.1.add(Pt::new(600.0, 0.0));
                    out.item(
                        MepKind::WaterHeater,
                        lv,
                        vec![at],
                        (lv.elevation, lv.elevation + 600.0),
                        "point-of-use heater",
                        400.0,
                        "At the fixture group.",
                    );
                    hot_sources.push((c.0, at));
                }
            }
        }
    }
    // Mains on each floor from the riser to its groups.
    let riser = hot_sources.first().map_or(middle(f), |h| h.1);
    for lv in &f.levels {
        let targets: Vec<Pt> = stacks
            .iter()
            .filter(|s| s.0 == lv.id)
            .map(|s| s.1)
            .collect();
        if targets.is_empty() {
            continue;
        }
        let src = hot_sources
            .iter()
            .find(|h| h.0 == lv.id)
            .map_or(riser, |h| h.1);
        let (trunk, branches) = trunk_and_branches(&lv.outline, src, &targets);
        let z = ceiling(rules, f, lv);
        let mut runs = vec![];
        if trunk.len() >= 2 {
            runs.push(trunk);
        }
        runs.extend(branches.into_iter().map(|(a, b)| vec![a, b]));
        for r in runs {
            let off: Vec<Pt> = r.iter().map(|q| q.add(Pt::new(150.0, 150.0))).collect();
            out.item(
                MepKind::ColdWater,
                lv,
                r,
                (z, z + 50.0),
                format!("{} cold water", pick_size(&p.service_sizes, ws * 0.5)),
                0.0,
                "Trunk and branches to the fixture groups.",
            );
            if sr.heater != "pou" {
                out.item(
                    MepKind::HotWater,
                    lv,
                    off,
                    (z, z + 50.0),
                    "3/4\" hot water",
                    0.0,
                    "Alongside the cold water.",
                );
            }
        }
        if sr.heater != "pou" {
            for t in &targets {
                let run = manhattan(riser, *t) + (lv.elevation - lowest.elevation).abs();
                if run > p.recirc_ft * MM_PER_FT && sr.heater != "central" {
                    out.flag("Long Hot Water Run", Some(lv.id), *t, format!("About {} of pipe from the heater: add recirculation or a heater nearer.", ft(run)));
                }
            }
        }
    }
    notes.push(format!(
        "{} stacks, {} items, {} flags.",
        stacks.len(),
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
