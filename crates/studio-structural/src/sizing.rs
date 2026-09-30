//! Preliminary sizing: span/depth ratios and tributary-area tables from the rules file.
//! Rules of thumb only. Preliminary — not engineered. Requires review by a licensed
//! structural engineer.

use studio_core::structural::MemberKind;
use studio_core::units::{MM_PER_FT, MM_PER_IN};

use crate::rules::{pick, Rules};

/// A preliminary size: its name, depth and width (mm), and the rule that gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct Size {
    pub name: String,
    pub depth: f64,
    pub width: f64,
    pub rule: String,
}

fn inches(x: f64) -> String {
    let whole = x.floor();
    let frac = ((x - whole) * 8.0).round() as i64;
    let (whole, frac) = if frac == 8 {
        (whole + 1.0, 0)
    } else {
        (whole, frac)
    };
    let f = match frac {
        0 => String::new(),
        2 => "-1/4".into(),
        4 => "-1/2".into(),
        6 => "-3/4".into(),
        n => format!("-{n}/8"),
    };
    format!("{whole:.0}{f}\"")
}

/// Rounds up to the next `step` inches.
fn up(x: f64, step: f64) -> f64 {
    (x / step - 1e-9).ceil() * step
}

/// The sizing table a scheme's material uses for a member at `level_index` (0 lowest):
/// podiums are concrete for their podium levels and wood above.
pub fn material_at(
    rules: &Rules,
    material: &str,
    podium_levels: usize,
    level_index: usize,
) -> String {
    let _ = rules;
    if material == "podium" {
        if level_index < podium_levels.max(1) {
            "concrete".into()
        } else {
            "wood".into()
        }
    } else {
        material.into()
    }
}

/// Sizes a member of `kind` spanning `span` mm (height for columns and walls), carrying
/// `trib_floors` square feet (tributary area times the floors above) for columns.
pub fn size(rules: &Rules, material: &str, kind: MemberKind, span: f64, trib_floors: f64) -> Size {
    let s = &rules.sizing;
    let span_ft = span / MM_PER_FT;
    let span_in = span / MM_PER_IN;
    let size = |name: String, depth_in: f64, width_in: f64, rule: String| Size {
        name,
        depth: depth_in * MM_PER_IN,
        width: width_in * MM_PER_IN,
        rule,
    };
    let lf = match material {
        "cfs" => Some(&s.cfs),
        "wood" => Some(&s.wood),
        _ => None,
    };
    match (material, kind) {
        (_, MemberKind::Span) => match material {
            "wood" | "cfs" => {
                let lf = lf.unwrap_or(&s.wood);
                let j = pick(&lf.joists, span_ft);
                let (name, d) = j.map_or(("joists".into(), 12.0), |j| (j.1.clone(), j.2));
                size(
                    name,
                    d,
                    1.5,
                    format!("Joist table: the first joist rated for a {span_ft:.0}' span."),
                )
            }
            "steel" => size(
                s.steel.deck.clone(),
                s.steel.deck_depth_in,
                0.0,
                format!(
                    "Composite deck spanning {:.0}' between beams at {:.0}' o.c.",
                    s.steel.beam_spacing_ft, s.steel.beam_spacing_ft
                ),
            ),
            "timber" => {
                let p = pick(&s.timber.panels, span_ft);
                let (name, d) = p.map_or(("CLT".into(), 7.0), |p| (p.1.clone(), p.2));
                size(
                    name,
                    d,
                    0.0,
                    format!("CLT table: panel rated for a {span_ft:.0}' span."),
                )
            }
            _ => {
                let t = up(
                    (span_in / s.concrete.slab_span_depth).max(s.concrete.slab_min_in),
                    0.5,
                );
                size(
                    format!("{} flat plate", inches(t)),
                    t,
                    0.0,
                    format!(
                        "Slab: span/{:.0} = {}, {} min.",
                        s.concrete.slab_span_depth,
                        inches(span_in / s.concrete.slab_span_depth),
                        inches(s.concrete.slab_min_in)
                    ),
                )
            }
        },
        (_, MemberKind::Column) => {
            let table = match material {
                "steel" => &s.steel.columns,
                "concrete" => &s.concrete.columns,
                "timber" => &s.timber.columns,
                _ => &lf.unwrap_or(&s.wood).posts,
            };
            let row = pick(table, trib_floors);
            let (name, w) = row.map_or(("column".into(), 12.0), |r| (r.1.clone(), r.2));
            size(
                name,
                w,
                w,
                format!(
                    "Column table: {trib_floors:.0} sf of floor (tributary area × floors carried)."
                ),
            )
        }
        // In light frame and steel, a transfer is a deeper beam of the same material.
        ("wood" | "cfs" | "steel" | "timber", MemberKind::Transfer) => {
            let mut s = crate::sizing::size(rules, material, MemberKind::Girder, span, trib_floors);
            s.name = format!("{} transfer beam", s.name);
            s.rule = format!("Transfer beam under a wall that doesn't stack. {}", s.rule);
            s
        }
        (_, MemberKind::Transfer) => {
            let d = up(span_in / s.concrete.transfer_span_depth, 2.0);
            size(
                format!(
                    "{} x {} transfer girder",
                    inches(s.concrete.transfer_width_in),
                    inches(d)
                ),
                d,
                s.concrete.transfer_width_in,
                format!(
                    "Transfer girder: span/{:.0} for walls or columns landing on it.",
                    s.concrete.transfer_span_depth
                ),
            )
        }
        ("steel", MemberKind::Girder | MemberKind::Beam | MemberKind::MomentFrame) => {
            let ratio = if kind == MemberKind::Beam {
                s.steel.beam_span_depth
            } else {
                s.steel.girder_span_depth
            };
            let need = span_in / ratio;
            let shape = s
                .steel
                .shapes
                .iter()
                .find(|x| x.1 >= need)
                .or(s.steel.shapes.last());
            let (name, d, w) =
                shape.map_or(("W-shape".into(), need, 6.0), |x| (x.0.clone(), x.1, x.2));
            size(
                if kind == MemberKind::MomentFrame {
                    format!("{name} ({})", s.steel.moment_frame)
                } else {
                    name
                },
                d,
                w,
                format!(
                    "Span/{ratio:.0}: {span_ft:.0}' needs about {}.",
                    inches(need)
                ),
            )
        }
        ("timber", MemberKind::Girder | MemberKind::Beam) => {
            let d = up(span_in / s.timber.beam_span_depth, 1.5);
            size(
                format!("{} x {} glulam", inches(s.timber.beam_width_in), inches(d)),
                d,
                s.timber.beam_width_in,
                format!(
                    "Glulam: span/{:.0}, in 1-1/2\" laminations.",
                    s.timber.beam_span_depth
                ),
            )
        }
        ("concrete", MemberKind::Girder | MemberKind::Beam) => {
            let d = up(span_in / 14.0, 2.0);
            size(
                format!("{} x {} concrete beam", inches(18.0), inches(d)),
                d,
                18.0,
                "Concrete beam: span/14.".into(),
            )
        }
        (_, MemberKind::Girder | MemberKind::Beam) => {
            let lf = lf.unwrap_or(&s.wood);
            let d = up(span_in / lf.beam_span_depth, 0.25).max(9.25);
            size(
                format!(
                    "{} x {} {}",
                    inches(lf.beam_width_in),
                    inches(d),
                    lf.beam_name
                ),
                d,
                lf.beam_width_in,
                format!("Span/{:.0}.", lf.beam_span_depth),
            )
        }
        (_, MemberKind::BearingWall) => {
            let lf = lf.unwrap_or(&s.wood);
            size(
                lf.bearing_wall.clone(),
                lf.wall_width_in,
                lf.wall_width_in,
                "Stacked bearing wall carrying the joists.".into(),
            )
        }
        (_, MemberKind::ShearWall) => match material {
            "concrete" => size(
                format!("{} concrete shear wall", inches(s.concrete.shear_wall_in)),
                s.concrete.shear_wall_in,
                s.concrete.shear_wall_in,
                "Concrete shear wall, rule-of-thumb thickness.".into(),
            ),
            "timber" => size(
                s.timber.clt_wall.clone(),
                6.875,
                6.875,
                "CLT shear wall panel.".into(),
            ),
            _ => {
                let lf = lf.unwrap_or(&s.wood);
                size(
                    lf.shear_wall.clone(),
                    lf.wall_width_in,
                    lf.wall_width_in,
                    "Sheathed shear wall in a solid, stacking wall segment.".into(),
                )
            }
        },
        (_, MemberKind::BracedFrame) => size(
            if material == "timber" {
                s.timber.brace.clone()
            } else {
                s.steel.brace.clone()
            },
            6.0,
            6.0,
            "Braced frame bay.".into(),
        ),
        // Foundations are sized in `foundation`, from their loads.
        (
            _,
            MemberKind::SpreadFooting
            | MemberKind::StripFooting
            | MemberKind::Mat
            | MemberKind::PileCap
            | MemberKind::FoundationWall,
        ) => size(
            "foundation".into(),
            12.0,
            24.0,
            "Sized from its load (foundation rules).".into(),
        ),
        (_, MemberKind::MomentFrame) => size(
            s.steel.moment_frame.clone(),
            18.0,
            8.0,
            "Moment frame line.".into(),
        ),
    }
}

/// Typical member depths for a scheme at a `grid` mm grid, for the proposal.
pub fn typical(rules: &Rules, material: &str, podium_levels: usize, grid: f64) -> Vec<String> {
    let g = grid / MM_PER_FT;
    let line = |label: &str, s: Size| format!("{label}: {} ({})", s.name, s.rule);
    match material {
        "podium" => {
            let mut v = typical(rules, "wood", 0, (grid * 0.6).min(16.0 * MM_PER_FT));
            v.insert(
                0,
                format!(
                    "Podium ({} level{}): {}",
                    podium_levels.max(1),
                    if podium_levels > 1 { "s" } else { "" },
                    size(rules, "concrete", MemberKind::Span, grid, 0.0).name
                ),
            );
            v.insert(
                1,
                line(
                    "Transfer slab/girders",
                    size(rules, "concrete", MemberKind::Transfer, grid, 0.0),
                ),
            );
            v
        }
        "wood" | "cfs" => vec![
            line("Joists", size(rules, material, MemberKind::Span, grid, 0.0)),
            line(
                "Beams/headers",
                size(rules, material, MemberKind::Beam, grid * 0.75, 0.0),
            ),
            format!(
                "Bearing walls: {}",
                size(rules, material, MemberKind::BearingWall, 0.0, 0.0).name
            ),
        ],
        "steel" => vec![
            line("Deck", size(rules, "steel", MemberKind::Span, grid, 0.0)),
            line("Beams", size(rules, "steel", MemberKind::Beam, grid, 0.0)),
            line(
                "Girders",
                size(rules, "steel", MemberKind::Girder, grid, 0.0),
            ),
            format!(
                "Columns: {} to {} typical",
                size(rules, "steel", MemberKind::Column, 0.0, g * g * 2.0).name,
                size(rules, "steel", MemberKind::Column, 0.0, g * g * 6.0).name
            ),
        ],
        "timber" => vec![
            line(
                "CLT floor",
                size(rules, "timber", MemberKind::Span, grid / 2.0, 0.0),
            ),
            line(
                "Glulam beams",
                size(rules, "timber", MemberKind::Beam, grid, 0.0),
            ),
            format!(
                "Columns: {}",
                size(rules, "timber", MemberKind::Column, 0.0, g * g * 3.0).name
            ),
        ],
        _ => vec![
            line("Slab", size(rules, "concrete", MemberKind::Span, grid, 0.0)),
            format!(
                "Columns: {} to {} typical",
                size(rules, "concrete", MemberKind::Column, 0.0, g * g * 2.0).name,
                size(rules, "concrete", MemberKind::Column, 0.0, g * g * 8.0).name
            ),
            line(
                "Shear walls/core",
                size(rules, "concrete", MemberKind::ShearWall, 0.0, 0.0),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_follow_the_rules_tables() {
        let r = Rules::builtin();
        let ft = MM_PER_FT;
        // Steel beam, 30': span/24 = 15" -> W16x31.
        assert_eq!(
            size(&r, "steel", MemberKind::Beam, 30.0 * ft, 0.0).name,
            "W16x31"
        );
        // Girder, 30': span/20 = 18" -> W21x44 (the W18x35 is 17.7").
        assert_eq!(
            size(&r, "steel", MemberKind::Girder, 30.0 * ft, 0.0).name,
            "W21x44"
        );
        // Flat plate at 24': 288/30 = 9.6" -> 10".
        let slab = size(&r, "concrete", MemberKind::Span, 24.0 * ft, 0.0);
        assert_eq!(slab.name, "10\" flat plate");
        assert!((slab.depth - 10.0 * MM_PER_IN).abs() < 1e-6);
        // Wood joists at 15' -> 11-7/8" I-joists.
        assert!(size(&r, "wood", MemberKind::Span, 15.0 * ft, 0.0)
            .name
            .starts_with("11-7/8\""));
        // Columns by tributary area x floors.
        assert_eq!(
            size(&r, "steel", MemberKind::Column, 0.0, 2000.0).name,
            "W10x49"
        );
        assert_eq!(
            size(&r, "concrete", MemberKind::Column, 0.0, 12000.0).name,
            "30\" sq. column"
        );
        // Glulam at 24': 288/16 = 18" -> 18" (12 laminations).
        assert_eq!(
            size(&r, "timber", MemberKind::Beam, 24.0 * ft, 0.0).name,
            "8-3/4\" x 18\" glulam"
        );
        assert_eq!(material_at(&r, "podium", 1, 0), "concrete");
        assert_eq!(material_at(&r, "podium", 1, 2), "wood");
        assert_eq!(inches(11.875), "11-7/8\"");
    }
}
