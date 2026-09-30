//! Foundations (ADR-083): loads carried down by rule of thumb, footing sizes from an
//! assumed allowable soil bearing, and when a mat or deep foundations are likely.
//!
//! Preliminary — not engineered. The soil values are assumptions until a geotechnical
//! report gives the real ones.

use studio_core::structural::SchemeKind;
use studio_core::units::MM_PER_FT;

use crate::rules::Rules;
use crate::types::Features;

/// Whether a scheme's floors are heavy (steel, concrete, mass timber) at `level_index`.
pub(crate) fn heavy_at(rules: &Rules, kind: SchemeKind, level_index: usize) -> bool {
    let sr = rules.scheme(kind);
    match sr.material.as_str() {
        "podium" => level_index < sr.podium_levels.max(1),
        "wood" | "cfs" => false,
        _ => true,
    }
}

/// The load one square foot of plan brings to the foundation, psf: each floor above the
/// lowest and the roof.
pub(crate) fn psf_down(rules: &Rules, kind: SchemeKind, stories: usize) -> f64 {
    let fr = &rules.foundation;
    let floors: f64 = (1..stories.max(1))
        .map(|i| {
            if heavy_at(rules, kind, i - 1) {
                fr.floor_psf_heavy
            } else {
                fr.floor_psf_light
            }
        })
        .sum();
    let roof = if heavy_at(rules, kind, stories.saturating_sub(1)) {
        fr.roof_psf_heavy
    } else {
        fr.roof_psf_light
    };
    floors + roof
}

fn up(x: f64, step: f64) -> f64 {
    (x / step - 1e-9).ceil() * step
}

fn feet(ft: f64) -> String {
    let whole = ft.floor();
    let inch = ((ft - whole) * 12.0).round();
    if inch >= 12.0 {
        format!("{}'-0\"", whole + 1.0)
    } else {
        format!("{whole:.0}'-{inch:.0}\"")
    }
}

/// A spread footing for `load_lb`: its side (ft), depth (in), and whether it passes the
/// largest side (deep foundations then).
pub(crate) fn spread(rules: &Rules, load_lb: f64) -> (f64, f64, bool) {
    let fr = &rules.foundation;
    let side = up((load_lb / fr.soil_psf).sqrt(), 0.5).max(fr.spread_min_ft);
    let depth = fr.spread_depth_in.max(up(side * 2.0, 2.0));
    (side, depth, side > fr.spread_max_ft)
}

pub(crate) fn spread_name(side: f64, depth_in: f64) -> String {
    format!(
        "{} x {} x {} spread footing",
        feet(side),
        feet(side),
        feet(depth_in / 12.0)
    )
}

/// A strip footing's width (in) for `plf` pounds per foot.
pub(crate) fn strip(rules: &Rules, plf: f64, heavy: bool) -> f64 {
    let fr = &rules.foundation;
    let min = if heavy {
        fr.strip_min_width_heavy_in
    } else {
        fr.strip_min_width_in
    };
    up(plf / fr.soil_psf * 12.0, 2.0).max(min)
}

/// Piles for `load_lb`.
pub(crate) fn piles(rules: &Rules, load_lb: f64) -> usize {
    ((load_lb / 1000.0 / rules.foundation.pile_kips).ceil() as usize).max(2)
}

/// The proposal's foundation line for a scheme, and its red flags.
pub fn summary(f: &Features, rules: &Rules, kind: SchemeKind) -> (String, Vec<String>) {
    let fr = &rules.foundation;
    let sr = rules.scheme(kind);
    let psf = psf_down(rules, kind, f.story_count);
    let grid = sr.grid_ft;
    let mut red = vec![];
    let deep_by_height = f.story_count >= fr.deep_stories;
    let line = if sr.bearing_walls && kind != SchemeKind::Podium {
        let plf = grid * psf + fr.wall_psf * f.total_height / MM_PER_FT;
        let w = strip(rules, plf, false);
        format!(
            "Foundations: continuous strip footings about {w:.0}\" wide under the bearing walls, slab on grade (on {:.0} psf soil)",
            fr.soil_psf
        )
    } else {
        // The largest column carries a full bay, or half the building each way if smaller.
        let (w, d) = (
            (f.max.x - f.min.x) / MM_PER_FT,
            (f.max.y - f.min.y) / MM_PER_FT,
        );
        let load = grid.min(w / 2.0).max(1.0) * grid.min(d / 2.0).max(1.0) * psf;
        let (side, depth, too_big) = spread(rules, load);
        let footing_area =
            side * side * (f.footprint_area / (MM_PER_FT * MM_PER_FT) / (grid * grid)).max(1.0);
        let share = footing_area / (f.footprint_area / (MM_PER_FT * MM_PER_FT)).max(1.0);
        if deep_by_height || too_big {
            red.push(format!(
                "Deep foundations likely (drilled piers or piles, about {} per column): a geotechnical report decides.",
                piles(rules, load)
            ));
            format!("Foundations: pile caps on piles or drilled piers under the columns (about {:.0} kips a column)", load / 1000.0)
        } else if share > fr.mat_share {
            red.push(
                "Footings would cover most of the footprint: a mat foundation is likely.".into(),
            );
            format!(
                "Foundations: a {:.0}\" mat under the building",
                fr.mat_depth_in
            )
        } else {
            format!(
                "Foundations: {} under the columns (about {:.0} kips each), strip footings at the perimeter",
                spread_name(side, depth),
                load / 1000.0
            )
        }
    };
    let basement = f
        .levels
        .first()
        .is_some_and(|l| l.elevation <= -fr.basement_ft * MM_PER_FT);
    let line = if basement {
        red.push(
            "A level below grade: foundation walls retain the soil (waterproofing and drainage)."
                .into(),
        );
        format!(
            "{line}; {:.0}\" concrete foundation walls at the basement",
            fr.foundation_wall_in
        )
    } else {
        line
    };
    (line, red)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footings_size_from_the_load_and_the_soil() {
        let r = Rules::builtin();
        // 30' x 30' bay of a 4-story steel frame: 3 floors x 150 + roof 90 = 540 psf,
        // 900 sf x 540 = 486 kips; sqrt(486000 / 1500) = 18' > 12': deep.
        assert_eq!(psf_down(&r, SchemeKind::SteelFrame, 4), 540.0);
        let (side, _, deep) = spread(&r, 900.0 * 540.0);
        assert!(deep && side == 18.0);
        // A 20' bay, one floor of wood and a roof: 400 x 90 = 36 kips -> 5' square, 12".
        assert_eq!(psf_down(&r, SchemeKind::LightWood, 2), 90.0);
        let (side, depth, deep) = spread(&r, 400.0 * 90.0);
        assert_eq!((side, depth, deep), (5.0, 12.0, false));
        assert_eq!(
            spread_name(side, depth),
            "5'-0\" x 5'-0\" x 1'-0\" spread footing"
        );
        // Strips: 1,200 plf on 1,500 psf is 9.6" -> the 16" minimum; 4,000 plf -> 32".
        assert_eq!(strip(&r, 1200.0, false), 16.0);
        assert_eq!(strip(&r, 4000.0, true), 32.0);
        assert_eq!(piles(&r, 486_000.0), 7);
        // A podium's concrete level is heavy, the wood above light.
        assert!(heavy_at(&r, SchemeKind::Podium, 0) && !heavy_at(&r, SchemeKind::Podium, 1));
    }
}
