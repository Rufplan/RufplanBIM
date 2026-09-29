//! Scoring the candidate schemes against the features, and the proposal's text. The text
//! only narrates the scores and features: templates, no invented geometry or sizes.

use studio_core::structural::{LateralKind, SchemeKind, SchemeSettings, Seismic, DISCLAIMER};
use studio_core::units::MM_PER_FT;

use crate::rules::Rules;
use crate::sizing;
use crate::types::*;

fn ft(mm: f64) -> String {
    format!("{:.0}'", mm / MM_PER_FT)
}

/// The lateral systems each scheme can use; the first is its default.
pub fn laterals(kind: SchemeKind) -> Vec<LateralKind> {
    match kind {
        SchemeKind::LightWood => vec![LateralKind::WoodShearWalls],
        SchemeKind::ColdFormedSteel => {
            vec![LateralKind::CfsShearWalls, LateralKind::BracedFrames]
        }
        SchemeKind::Podium => vec![LateralKind::WoodShearWalls, LateralKind::ConcreteShearWalls],
        SchemeKind::SteelFrame => vec![LateralKind::BracedFrames, LateralKind::MomentFrames],
        SchemeKind::ConcreteFlatPlate => vec![LateralKind::ConcreteShearWalls],
        SchemeKind::MassTimber => vec![LateralKind::BracedFrames, LateralKind::CltShearWalls],
    }
}

/// Whether the ground floor holds retail, parking or an open zone while floors above are
/// residential: the podium case.
fn podium_case(f: &Features) -> bool {
    let (Some(g), true) = (f.levels.first(), f.levels.len() >= 2) else {
        return false;
    };
    let ground_open = g
        .uses
        .iter()
        .any(|u| matches!(u, UseKind::Retail | UseKind::Parking | UseKind::Assembly))
        || f.open_zones.iter().any(|z| z.level == g.id);
    let upper_res = f
        .levels
        .iter()
        .skip(1)
        .any(|l| l.uses.contains(&UseKind::Residential));
    ground_open && upper_res
}

/// Scores one scheme.
fn score(f: &Features, rules: &Rules, kind: SchemeKind, seismic: Seismic) -> SchemeProposal {
    let sr = rules.scheme(kind);
    let w = &rules.weights;
    let height_ft = f.total_height / MM_PER_FT;
    let stories = f.story_count.max(1);
    let mut crit = vec![];

    // Height and stories: a hard limit.
    let ruled_out = stories > sr.max_stories || height_ft > sr.max_height_ft + 0.5;
    let few = sr.min_stories > 0 && stories < sr.min_stories;
    let small = stories < sr.economical_min_stories;
    crit.push(Criterion {
        name: "Height".into(),
        score: if ruled_out {
            0.0
        } else if few {
            0.3
        } else if small {
            0.5
        } else {
            1.0
        },
        note: if ruled_out {
            format!(
                "{stories} stories, {height_ft:.0}' tall is beyond its typical {} stories / {:.0}'.",
                sr.max_stories, sr.max_height_ft
            )
        } else if few {
            format!(
                "At {stories} stories a podium rarely pays off (usually {}+).",
                sr.min_stories
            )
        } else if small {
            format!(
                "At {stories} stor{} it's usually uneconomical (from about {} stories).",
                if stories == 1 { "y" } else { "ies" },
                sr.economical_min_stories
            )
        } else {
            format!(
                "{stories} stories, {height_ft:.0}' is within its typical {} stories / {:.0}'.",
                sr.max_stories, sr.max_height_ft
            )
        },
    });

    // Spans: bearing walls must reach across rooms (above a podium, its upper rooms);
    // column grids only need to clear the rooms that must be column-free.
    let podium_n = if kind == SchemeKind::Podium {
        sr.podium_levels.max(1)
    } else {
        0
    };
    let room_span = f
        .levels
        .iter()
        .skip(podium_n)
        .map(|l| l.max_room_span)
        .fold(0.0, f64::max);
    let need_mm = if sr.bearing_walls {
        room_span.max(f.column_free_span)
    } else {
        f.column_free_span
    };
    let need = need_mm / MM_PER_FT;
    let span = if need <= sr.span_max_ft {
        1.0
    } else {
        (1.0 - (need - sr.span_max_ft) / sr.span_max_ft).max(0.0)
    };
    crit.push(Criterion {
        name: "Span".into(),
        score: span,
        note: if need_mm <= 0.0 {
            "No room needs to be column-free; columns fit the grid.".into()
        } else {
            format!(
                "Longest span it must reach about {need:.0}'; it typically spans {:.0}'–{:.0}'.",
                sr.span_min_ft, sr.span_max_ft
            )
        },
    });

    // Wall stacking: bearing-wall schemes need walls over walls.
    // A podium's first light-frame level sits on its transfer slab by design, so stacking
    // counts from the level above it; its own levels are concrete.
    let podium_ids: Vec<_> = f.levels.iter().take(podium_n + 1).map(|l| l.id).collect();
    let upper: Vec<f64> = f
        .levels
        .iter()
        .skip(podium_n + 1)
        .map(|l| l.stacking_ratio)
        .collect();
    let stacking = if kind == SchemeKind::Podium {
        if upper.is_empty() {
            1.0
        } else {
            upper.iter().sum::<f64>() / upper.len() as f64
        }
    } else if sr.bearing_walls {
        f.stacking
    } else if f.stacking >= 0.5 {
        1.0
    } else {
        0.85
    };
    crit.push(Criterion {
        name: "Wall stacking".into(),
        score: stacking,
        note: if sr.bearing_walls {
            format!(
                "{:.0}% of upper-floor walls stand on walls below; bearing walls need them to stack.",
                100.0 * f.stacking
            )
        } else {
            "Columns on a grid don't need the walls to stack.".into()
        },
    });

    // Use.
    let mut occ = if f.uses.is_empty() {
        0.6
    } else {
        let (mut sum, mut wsum) = (0.0, 0.0);
        for (i, u) in f.uses.iter().take(3).enumerate() {
            let weight = 1.0 / (1u32 << i) as f64;
            sum += weight * sr.uses.get(u.key()).copied().unwrap_or(0.5);
            wsum += weight;
        }
        sum / wsum
    };
    let podium = podium_case(f);
    if kind == SchemeKind::Podium && podium {
        occ = 1.0;
    }
    crit.push(Criterion {
        name: "Use".into(),
        score: occ,
        note: if f.uses.is_empty() {
            "No uses found in room names; scored as neutral.".into()
        } else if kind == SchemeKind::Podium && podium {
            "An open ground floor under residential floors is the podium's case.".into()
        } else {
            format!(
                "Uses from room names: {}.",
                f.uses
                    .iter()
                    .map(|u| u.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    });

    // Discontinuities.
    let count = |k: DiscontinuityKind| {
        f.discontinuities
            .iter()
            .filter(|d| d.kind == k)
            .filter(|d| {
                // A podium absorbs walls landing on its slab and its open ground floor.
                kind != SchemeKind::Podium
                    || !(podium_ids.contains(&d.level)
                        && matches!(
                            k,
                            DiscontinuityKind::NonStackingWall | DiscontinuityKind::SoftStory
                        ))
            })
            .count()
    };
    let (nonstack, soft, cant) = (
        count(DiscontinuityKind::NonStackingWall),
        count(DiscontinuityKind::SoftStory),
        count(DiscontinuityKind::Cantilever) + count(DiscontinuityKind::Setback),
    );
    // Non-stacking walls are scored under stacking; here only for frames, lightly.
    let per = if sr.bearing_walls { 0.0 } else { 0.03 };
    let disc = (1.0
        - per * nonstack as f64
        - if sr.bearing_walls { 0.2 } else { 0.1 } * soft as f64
        - 0.08 * cant as f64
        - 0.05 * f.reentrant_corners as f64)
        .max(0.0);
    crit.push(Criterion {
        name: "Discontinuities".into(),
        score: disc,
        note: format!(
            "{nonstack} non-stacking walls, {soft} possible soft stories, {cant} setbacks or cantilevers, {} re-entrant corners.",
            f.reentrant_corners
        ),
    });

    // Seismic.
    let sei = sr.seismic.get(seismic);
    crit.push(Criterion {
        name: "Seismic".into(),
        score: sei,
        note: format!(
            "Suitability in a {} seismic region (your setting).",
            seismic_label(seismic)
        ),
    });

    let weights = [
        w.height,
        w.span,
        w.stacking,
        w.occupancy,
        w.discontinuities,
        w.seismic,
    ];
    let total: f64 = crit
        .iter()
        .zip(weights)
        .map(|(c, w)| c.score * w)
        .sum::<f64>()
        / weights.iter().sum::<f64>()
        * 100.0;
    let total = if ruled_out {
        total.min(30.0) * 0.5
    } else {
        total
    };

    // Text.
    let strengths: Vec<&Criterion> = crit.iter().filter(|c| c.score >= 0.85).collect();
    let concerns: Vec<&Criterion> = crit.iter().filter(|c| c.score < 0.6).collect();
    let mut rationale = format!("{} scores {:.0}/100.", kind.label(), total);
    if !strengths.is_empty() {
        rationale.push_str(&format!(
            " It fits on {}.",
            strengths
                .iter()
                .map(|c| c.name.to_lowercase())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !concerns.is_empty() {
        rationale.push_str(&format!(
            " Concerns: {}",
            concerns
                .iter()
                .map(|c| c.note.clone())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    let grid = sr.grid_ft * MM_PER_FT;
    let grid_text = if sr.bearing_walls {
        format!(
            "Bearing walls about {} apart, joists spanning between them{}",
            ft(grid),
            if kind == SchemeKind::Podium {
                format!(
                    "; {} x {} column grid in the podium",
                    ft(grid * 1.25),
                    ft(grid * 1.25)
                )
            } else {
                String::new()
            }
        )
    } else {
        format!("{} x {} column grid", ft(grid), ft(grid))
    };

    let mut red = vec![];
    if ruled_out {
        red.push(format!(
            "Beyond its usual height: {} stories / {:.0}' typical maximum.",
            sr.max_stories, sr.max_height_ft
        ));
    }
    for z in f
        .open_zones
        .iter()
        .filter(|z| z.column_free || sr.bearing_walls)
    {
        let above_podium = f.levels.iter().position(|l| l.id == z.level).unwrap_or(0) >= podium_n;
        if z.span > sr.span_max_ft * MM_PER_FT && (z.column_free || above_podium) {
            red.push(format!(
                "{} spans about {}, beyond this system's {:.0}': long-span framing or a transfer.",
                z.name,
                ft(z.span),
                sr.span_max_ft
            ));
        }
    }
    if sr.bearing_walls || kind == SchemeKind::Podium {
        // Above a podium, its first level lands on the transfer slab (flagged below).
        for l in f.levels.iter().skip(1 + podium_n) {
            let n = f
                .walls
                .iter()
                .filter(|w| w.level == l.id && !w.stacks && w.length >= 1500.0)
                .count();
            if n > 0 {
                red.push(format!(
                    "Transfer beams likely at {} ({n} wall{} {} stack).",
                    l.name,
                    if n == 1 { "" } else { "s" },
                    if n == 1 { "doesn't" } else { "don't" }
                ));
            }
        }
    }
    if kind == SchemeKind::Podium && f.levels.len() > 1 {
        red.push(format!(
            "Transfer slab at the top of the podium ({}) under the light-frame walls.",
            f.levels[1].name
        ));
    }
    for d in &f.discontinuities {
        match d.kind {
            DiscontinuityKind::SoftStory
            | DiscontinuityKind::Cantilever
            | DiscontinuityKind::Setback => red.push(d.message.clone()),
            _ => {}
        }
    }
    if f.reentrant_corners > 0 {
        red.push(format!(
            "{} re-entrant corner{}: collectors or a seismic joint may be needed.",
            f.reentrant_corners,
            if f.reentrant_corners == 1 { "" } else { "s" }
        ));
    }
    red.dedup();

    let lat = laterals(kind);
    SchemeProposal {
        kind,
        label: kind.label().into(),
        score: (total * 10.0).round() / 10.0,
        ruled_out,
        rationale,
        grid: grid_text,
        member_depths: sizing::typical(rules, &sr.material, sr.podium_levels, grid),
        red_flags: red,
        settings: SchemeSettings {
            kind,
            seismic,
            grid_x: grid,
            grid_y: grid,
            lateral: sr.lateral,
            span_dir: default_span(),
        },
        laterals: lat,
        criteria: crit,
    }
}

pub fn seismic_label(s: Seismic) -> &'static str {
    match s {
        Seismic::Low => "low",
        Seismic::Moderate => "moderate",
        Seismic::High => "high",
    }
}

/// Suggest Structure: every scheme scored and ranked, best first, with the assumptions and
/// questions for the user.
pub fn propose(f: &Features, rules: &Rules, seismic: Seismic) -> StructuralProposal {
    let mut schemes: Vec<SchemeProposal> = SchemeKind::ALL
        .into_iter()
        .map(|k| score(f, rules, k, seismic))
        .collect();
    schemes.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then((a.kind as u8).cmp(&(b.kind as u8)))
    });

    let mut summary = vec![
        format!(
            "{} stor{}, {} to the top of the walls.",
            f.story_count,
            if f.story_count == 1 { "y" } else { "ies" },
            ft(f.total_height)
        ),
        format!(
            "Footprint about {:.0} sf, {} x {} (aspect {:.1}).",
            f.footprint_area / (MM_PER_FT * MM_PER_FT),
            ft(f.max.x - f.min.x),
            ft(f.max.y - f.min.y),
            f.aspect_ratio
        ),
        format!("Longest clear span about {}.", ft(f.max_span)),
    ];
    if f.levels.len() > 1 {
        summary.push(format!(
            "{:.0}% of upper-floor walls stack on walls below.",
            100.0 * f.stacking
        ));
    }
    if !f.uses.is_empty() {
        summary.push(format!(
            "Uses from room names: {}.",
            f.uses
                .iter()
                .map(|u| u.label())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !f.cores.is_empty() {
        summary.push(format!(
            "{} stair/elevator/shaft core zone(s).",
            f.cores.len()
        ));
    }
    if !f.discontinuities.is_empty() {
        summary.push(format!(
            "{} discontinuities found.",
            f.discontinuities.len()
        ));
    }

    let mut assumptions = vec![
        format!(
            "Seismic region: {} (your setting; no hazard data looked up).",
            seismic_label(seismic)
        ),
        "Typical gravity loads for the uses found; no unusual loads (roof gardens, pools, heavy equipment).".into(),
        "Floor-to-floor heights and outlines are the model's levels, floors and walls.".into(),
        "Foundations, wind and fire ratings are not evaluated.".into(),
    ];
    if f.uses.is_empty() {
        assumptions.push("No room names matched a use; the use score is neutral.".into());
    } else {
        assumptions.push(format!(
            "Uses are read from room names ({}).",
            f.uses
                .iter()
                .map(|u| u.label())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut questions = vec![];
    if podium_case(f)
        || f.levels
            .first()
            .is_some_and(|g| f.open_zones.iter().any(|z| z.level == g.id))
    {
        questions.push("Is the ground floor open-plan (retail, parking or assembly)?".into());
    }
    if f.uses.is_empty() {
        questions.push("What will the building be used for? The room names didn't say.".into());
    }
    if seismic == Seismic::Moderate {
        questions.push("Is the site in a low, moderate or high seismic region?".into());
    }
    if let Some(best) = schemes.first() {
        let sr = rules.scheme(best.kind);
        if f.story_count + 1 >= sr.max_stories {
            questions.push("Could the building grow taller later?".into());
        }
    }
    questions.truncate(3);

    StructuralProposal {
        disclaimer: DISCLAIMER.into(),
        seismic,
        schemes,
        assumptions,
        questions,
        summary,
    }
}
