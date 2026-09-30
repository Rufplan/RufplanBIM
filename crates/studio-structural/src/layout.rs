//! The preliminary layout for a chosen scheme: grid, gravity system, lateral system,
//! sizing and flags. Deterministic; reads the features and model, never changes them.
//!
//! Preliminary — not engineered. Requires review by a licensed structural engineer.

use studio_core::structural::{
    FlagKind, LateralKind, MemberKind, SchemeKind, SchemeSettings, SpanDir, StructFlag,
    StructLayout, StructMember, DISCLAIMER,
};
use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::ElementId;
use studio_geom::Pt;
use studio_regen::Model;

use crate::extract::{centroid, inside, merge};
use crate::rules::Rules;
use crate::sizing::{material_at, size};
use crate::types::*;

const SQFT: f64 = MM_PER_FT * MM_PER_FT;

fn ft(mm: f64) -> String {
    format!("{:.0}'", mm / MM_PER_FT)
}

/// A level being framed.
struct Lv<'a> {
    i: usize,
    f: &'a LevelFeature,
    elev: f64,
    top: f64,
}

/// Picks grid lines from `lo` to `hi` about `target` apart, preferring candidate lines
/// (weighted positions of aligned walls) and filling evenly where there are none.
pub fn pick_lines(lo: f64, hi: f64, target: f64, cands: &[(f64, f64)]) -> Vec<f64> {
    if hi - lo < 1.0 || target < 1.0 {
        return vec![lo];
    }
    let near = |x: f64| {
        cands
            .iter()
            .filter(|c| (c.0 - x).abs() <= 600.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|c| c.0)
    };
    let (lo, hi) = (near(lo).unwrap_or(lo), near(hi).unwrap_or(hi));
    let mut out = vec![lo];
    let mut pos = lo;
    while hi - pos > target * 1.25 {
        let best = cands
            .iter()
            .filter(|c| {
                c.0 > pos + 0.6 * target && c.0 < pos + 1.25 * target && c.0 < hi - 0.4 * target
            })
            .max_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then(((b.0 - pos - target).abs()).total_cmp(&(a.0 - pos - target).abs()))
            })
            .map(|c| c.0);
        pos = match best {
            Some(c) => c,
            None => {
                let n = ((hi - pos) / target).ceil().max(1.0);
                pos + (hi - pos) / n
            }
        };
        out.push(pos);
    }
    out.push(hi);
    out
}

/// Grid candidates: the model's own grid lines first (the architect's intent), then wall
/// positions weighted by length, stacking and exterior/core: x of lines running along y
/// (`along_y`), or y of lines along x; clustered within 300 mm.
fn candidates(f: &Features, model: &Model, along_y: bool) -> Vec<(f64, f64)> {
    let grids = model.grids.iter().filter_map(|g| {
        let axis = crate::extract::axis_of(g.start, g.end);
        let c = if along_y {
            (axis == Axis::Y).then(|| (g.start.x + g.end.x) / 2.0)
        } else {
            (axis == Axis::X).then(|| (g.start.y + g.end.y) / 2.0)
        };
        c.map(|c| (c, 1.0e9))
    });
    let mut raw: Vec<(f64, f64)> = f
        .walls
        .iter()
        .filter(|w| w.axis == if along_y { Axis::Y } else { Axis::X })
        .map(|w| {
            let c = if along_y {
                (w.start.x + w.end.x) / 2.0
            } else {
                (w.start.y + w.end.y) / 2.0
            };
            let mut weight = w.length;
            if w.stacks {
                weight *= 1.5;
            }
            if w.exterior || w.at_core {
                weight *= 1.5;
            }
            (c, weight)
        })
        .chain(grids)
        .collect();
    raw.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = vec![];
    for (c, w) in raw {
        match out.last_mut() {
            Some(l) if (c - l.0).abs() <= 300.0 => {
                let tw = l.1 + w;
                l.0 = (l.0 * l.1 + c * w) / tw;
                l.1 = tw;
            }
            _ => out.push((c, w)),
        }
    }
    out
}

fn avg_gap(lines: &[f64]) -> f64 {
    if lines.len() < 2 {
        return 0.0;
    }
    (lines[lines.len() - 1] - lines[0]) / (lines.len() - 1) as f64
}

fn near_outline(outline: &[Pt], p: Pt, tol: f64) -> bool {
    if inside(outline, p) {
        return true;
    }
    let n = outline.len();
    (0..n).any(|i| studio_geom::project_to_segment(p, outline[i], outline[(i + 1) % n]).1 <= tol)
}

/// Distance from `p` to a wall's centerline, the point on it, and how far along it is.
fn on_wall(w: &WallFeature, p: Pt) -> (f64, Pt, f64) {
    let (t, d) = studio_geom::project_to_segment(p, w.start, w.end);
    let q = w.start.lerp(w.end, t);
    (d, q, t * w.length)
}

struct Builder<'a> {
    rules: &'a Rules,
    material: String,
    podium_levels: usize,
    members: Vec<StructMember>,
    flags: Vec<StructFlag>,
}

impl Builder<'_> {
    #[allow(clippy::too_many_arguments)]
    fn member(
        &mut self,
        kind: MemberKind,
        lv: &Lv,
        start: Pt,
        end: Pt,
        base: f64,
        top: f64,
        span: f64,
        trib: f64,
        why: &str,
    ) {
        let mat = material_at(self.rules, &self.material, self.podium_levels, lv.i);
        let s = size(self.rules, &mat, kind, span, trib);
        self.members.push(StructMember {
            kind,
            level: lv.f.id,
            start,
            end,
            base,
            top,
            size: format!("{} (prelim.)", s.name),
            depth: s.depth,
            width: s.width,
            span,
            rule: format!("{why} {}", s.rule),
        });
    }
    fn flag(&mut self, kind: FlagKind, level: Option<ElementId>, at: Pt, message: String) {
        if !self
            .flags
            .iter()
            .any(|f| f.kind == kind && f.level == level && f.at.dist(at) < 1.0)
        {
            self.flags.push(StructFlag {
                kind,
                level,
                at,
                message,
            });
        }
    }
}

/// Plan area a column at `p` carries: half the bays either side each way, sf.
fn trib_sf(p: Pt, xs: &[f64], ys: &[f64]) -> f64 {
    let half = |lines: &[f64], v: f64| {
        let below = lines
            .iter()
            .copied()
            .filter(|l| *l < v - 1.0)
            .fold(f64::NAN, f64::max);
        let above = lines
            .iter()
            .copied()
            .filter(|l| *l > v + 1.0)
            .fold(f64::NAN, f64::min);
        (if below.is_finite() {
            (v - below) / 2.0
        } else {
            0.0
        }) + (if above.is_finite() {
            (above - v) / 2.0
        } else {
            0.0
        })
    };
    half(xs, p.x).max(600.0) * half(ys, p.y).max(600.0) / SQFT
}

/// Foundations under the lowest level: a spread footing (or pile cap) under each column,
/// strip footings under exterior, bearing and shear walls, a mat when footings would cover
/// most of the footprint, and foundation walls around a basement.
fn foundations(
    f: &Features,
    rules: &Rules,
    settings: &SchemeSettings,
    lvs: &[Lv],
    xs: &[f64],
    ys: &[f64],
    b: &mut Builder,
) {
    use crate::foundation::{heavy_at, piles, psf_down, spread, spread_name, strip};
    let Some(lv0) = lvs.first() else { return };
    let fr = &rules.foundation;
    let n = lvs.len();
    let psf = psf_down(rules, settings.kind, n);
    let sr = rules.scheme(settings.kind);
    let heavy = heavy_at(rules, settings.kind, 0) || !sr.bearing_walls;
    let top = lv0.elev - 100.0;
    let frost = lv0.elev - fr.frost_depth_in * MM_PER_IN;
    let height_ft = (lvs.last().map_or(lv0.top, |l| l.top) - lv0.elev) / MM_PER_FT;
    let deep = n >= fr.deep_stories;
    let mut feet: Vec<StructMember> = vec![];
    let mut area = 0.0;
    let mut deep_hit = deep;
    let member =
        |kind, a: Pt, e: Pt, base: f64, width: f64, depth: f64, size: String, rule: String| {
            StructMember {
                kind,
                level: lv0.f.id,
                start: a,
                end: e,
                base,
                top,
                size: format!("{size} (prelim.)"),
                depth,
                width,
                span: if a.dist(e) > 1.0 { a.dist(e) } else { width },
                rule,
            }
        };
    // Columns standing on the lowest level.
    let cols: Vec<Pt> = b
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Column && m.level == lv0.f.id)
        .map(|m| m.start)
        .collect();
    for p in cols {
        let trib = trib_sf(p, xs, ys);
        let load = trib * psf;
        let (side, depth_in, too_big) = spread(rules, load);
        if deep || too_big {
            deep_hit = true;
            let np = piles(rules, load);
            let cap = ((np as f64).sqrt().ceil() * 3.0 + 1.5) * MM_PER_FT;
            let d = 36.0 * MM_PER_IN;
            feet.push(member(
                MemberKind::PileCap,
                p,
                p,
                top - d,
                cap,
                d,
                format!("pile cap on {np} piles ({:.0} kips)", load / 1000.0),
                format!(
                    "{trib:.0} sf x {psf:.0} psf = {:.0} kips; {:.0} kips a pile.",
                    load / 1000.0,
                    fr.pile_kips
                ),
            ));
        } else {
            area += side * side;
            let d = depth_in * MM_PER_IN;
            feet.push(member(
                MemberKind::SpreadFooting,
                p,
                p,
                (top - d).min(if is_edge(p, &lv0.f.outline) {
                    frost
                } else {
                    top - d
                }),
                side * MM_PER_FT,
                d,
                spread_name(side, depth_in),
                format!(
                    "{trib:.0} sf x {psf:.0} psf = {:.0} kips on {:.0} psf soil.",
                    load / 1000.0,
                    fr.soil_psf
                ),
            ));
        }
    }
    // Strips: exterior walls, then bearing and shear walls not already on one.
    let mut lines: Vec<(Pt, Pt, bool, bool)> = f
        .walls
        .iter()
        .filter(|w| w.level == lv0.f.id && w.exterior)
        .map(|w| (w.start, w.end, false, true))
        .collect();
    for m in b.members.iter().filter(|m| {
        m.level == lv0.f.id && matches!(m.kind, MemberKind::BearingWall | MemberKind::ShearWall)
    }) {
        let bearing = m.kind == MemberKind::BearingWall;
        let on = lines.iter_mut().find(|l| {
            studio_geom::project_to_segment(m.start, l.0, l.1).1 < 300.0
                && studio_geom::project_to_segment(m.end, l.0, l.1).1 < 300.0
        });
        match on {
            Some(l) => l.2 |= bearing,
            None => lines.push((m.start, m.end, bearing, false)),
        }
    }
    let grid_ft = settings.grid_x.min(settings.grid_y) / MM_PER_FT;
    for (a, e, bearing, exterior) in lines {
        let trib = if !bearing {
            0.0
        } else if exterior {
            grid_ft / 2.0
        } else {
            grid_ft
        };
        let plf = trib * psf + fr.wall_psf * height_ft;
        let w_in = strip(rules, plf, heavy);
        let d = fr.strip_depth_in * MM_PER_IN;
        area += a.dist(e) / MM_PER_FT * w_in / 12.0;
        feet.push(member(
            MemberKind::StripFooting,
            a,
            e,
            if exterior {
                (top - d).min(frost)
            } else {
                top - d
            },
            w_in * MM_PER_IN,
            d,
            format!("{w_in:.0}\" x {:.0}\" strip footing", fr.strip_depth_in),
            format!(
                "{plf:.0} plf ({}wall weight) on {:.0} psf soil{}.",
                if bearing {
                    format!("{trib:.0}' of floors + ")
                } else {
                    String::new()
                },
                fr.soil_psf,
                if exterior { ", below frost" } else { "" }
            ),
        ));
    }
    // A mat when footings would cover most of the footprint.
    let footprint = lv0.f.area / SQFT;
    if !deep_hit && footprint > 0.0 && area > fr.mat_share * footprint {
        if let Some((lo, hi)) = studio_geom::bounds_of(&lv0.f.outline) {
            let d = fr.mat_depth_in * MM_PER_IN;
            feet.retain(|m| m.kind == MemberKind::FoundationWall);
            feet.push(member(
                MemberKind::Mat,
                lo,
                hi,
                top - d,
                0.0,
                d,
                format!("{:.0}\" mat foundation", fr.mat_depth_in),
                format!(
                    "Footings would cover {:.0}% of the footprint (over {:.0}%): one mat instead.",
                    100.0 * area / footprint,
                    100.0 * fr.mat_share
                ),
            ));
            b.flag(
                FlagKind::Foundation,
                Some(lv0.f.id),
                lo.add(hi).scale(0.5),
                "Footings would cover most of the footprint: a mat foundation is likely.".into(),
            );
        }
    }
    if deep_hit {
        let at = studio_geom::bounds_of(&lv0.f.outline)
            .map_or(Pt::new(0.0, 0.0), |(lo, hi)| lo.add(hi).scale(0.5));
        b.flag(
            FlagKind::Foundation,
            Some(lv0.f.id),
            at,
            "Deep foundations likely (piles or drilled piers under pile caps): a geotechnical report decides the system.".into(),
        );
    }
    // Foundation walls around a level below grade.
    if lv0.elev <= -fr.basement_ft * MM_PER_FT {
        let w = fr.foundation_wall_in * MM_PER_IN;
        for wall in f.walls.iter().filter(|w| w.level == lv0.f.id && w.exterior) {
            feet.push(StructMember {
                kind: MemberKind::FoundationWall,
                level: lv0.f.id,
                start: wall.start,
                end: wall.end,
                base: lv0.elev,
                top: 0.0,
                size: format!(
                    "{:.0}\" concrete foundation wall (prelim.)",
                    fr.foundation_wall_in
                ),
                depth: w,
                width: w,
                span: -lv0.elev,
                rule: "Retains the soil around the basement, up to grade.".into(),
            });
        }
    }
    b.members.extend(feet);
}

/// Whether `p` is at the edge of an outline (within 600 mm), where footings bear below frost.
fn is_edge(p: Pt, outline: &[Pt]) -> bool {
    let n = outline.len();
    (0..n).any(|i| studio_geom::project_to_segment(p, outline[i], outline[(i + 1) % n]).1 <= 600.0)
}

/// The span direction to use: the setting, or (Auto) the shorter way between supports.
fn span_dir(setting: SpanDir, gap_x: f64, gap_y: f64) -> SpanDir {
    match setting {
        SpanDir::Auto if gap_x > 0.0 && (gap_y == 0.0 || gap_x <= gap_y) => SpanDir::X,
        SpanDir::Auto => SpanDir::Y,
        s => s,
    }
}

/// Generates the layout for `settings`.
pub fn layout(
    f: &Features,
    model: &Model,
    rules: &Rules,
    settings: &SchemeSettings,
) -> StructLayout {
    let sr = rules.scheme(settings.kind);
    let mut out = StructLayout {
        notes: vec![DISCLAIMER.into()],
        ..Default::default()
    };
    if f.levels.is_empty() || f.max.x - f.min.x < 1.0 {
        out.notes
            .push("Nothing to frame yet: model walls and floors first.".into());
        return out;
    }
    let lvs: Vec<Lv> = f
        .levels
        .iter()
        .enumerate()
        .map(|(i, l)| Lv {
            i,
            f: l,
            elev: l.elevation,
            top: l.elevation + l.floor_to_floor,
        })
        .collect();
    let bearing = sr.bearing_walls;
    let podium = settings.kind == SchemeKind::Podium;
    let podium_levels = if podium {
        sr.podium_levels.max(1).min(lvs.len())
    } else {
        0
    };
    let mut b = Builder {
        rules,
        material: sr.material.clone(),
        podium_levels,
        members: vec![],
        flags: vec![],
    };

    // Grid lines from aligned walls, filled to the target spacing.
    let xs = pick_lines(
        f.min.x,
        f.max.x,
        settings.grid_x,
        &candidates(f, model, true),
    );
    let ys = pick_lines(
        f.min.y,
        f.max.y,
        settings.grid_y,
        &candidates(f, model, false),
    );
    out.grid_x = xs.clone();
    out.grid_y = ys.clone();
    let dir = span_dir(settings.span_dir, avg_gap(&xs), avg_gap(&ys));
    out.notes.push(format!(
        "{}; joists/deck span {}.",
        settings.kind.label(),
        if dir == SpanDir::X {
            "east–west (x)"
        } else {
            "north–south (y)"
        }
    ));

    let frame_level = |lv: &Lv| !bearing || lv.i < podium_levels;
    let span_max = sr.span_max_ft * MM_PER_FT;
    let snap = rules.general.column_snap_ft * MM_PER_FT;

    // ---- Gravity: columns, beams, girders and deck where the scheme frames ----
    let n = lvs.len();
    for lv in &lvs {
        if !frame_level(lv) {
            continue;
        }
        let rooms: Vec<_> = model
            .rooms
            .iter()
            .filter(|r| r.level == lv.f.id && r.boundary.is_some())
            .collect();
        let walls: Vec<&WallFeature> = f.walls.iter().filter(|w| w.level == lv.f.id).collect();
        let floors_above = (if podium { podium_levels } else { n }) - lv.i;
        for (ix, &x) in xs.iter().enumerate() {
            for (iy, &y) in ys.iter().enumerate() {
                let mut p = Pt::new(x, y);
                if !near_outline(&lv.f.outline, p, 450.0) {
                    continue;
                }
                let mut why = "At a grid intersection.".to_string();
                // In a room: onto a nearby wall if there is one, clear of its openings.
                let room = rooms.iter().find(|r| {
                    let bd = r.boundary.as_deref().unwrap_or(&[]);
                    inside(bd, p)
                        && (0..bd.len()).all(|k| {
                            studio_geom::project_to_segment(p, bd[k], bd[(k + 1) % bd.len()]).1
                                > 450.0
                        })
                });
                if let Some(r) = room {
                    let best = walls
                        .iter()
                        .map(|w| (w, on_wall(w, p)))
                        .filter(|(_, (d, _, _))| *d <= snap)
                        .min_by(|a, b| a.1 .0.total_cmp(&b.1 .0));
                    match best {
                        Some((w, (_, q, t))) => {
                            let solid = w.solid.iter().find(|s| t >= s.0 && t <= s.1);
                            let t2 = match solid {
                                Some(_) => t,
                                None => w
                                    .solid
                                    .iter()
                                    .flat_map(|s| [s.0 + 150.0, s.1 - 150.0])
                                    .min_by(|a, c| (a - t).abs().total_cmp(&(c - t).abs()))
                                    .unwrap_or(t),
                            };
                            let u = w.end.sub(w.start).scale(1.0 / w.length.max(1.0));
                            p = if (t2 - t).abs() > 1.0 { w.start.add(u.scale(t2)) } else { q };
                            why = format!("Moved {} onto a wall, out of {}.", ft(p.dist(Pt::new(x, y))), r.name);
                        }
                        None => b.flag(
                            FlagKind::ColumnInRoom,
                            Some(lv.f.id),
                            p,
                            format!(
                                "A column lands inside {} on {} with no wall nearby: shift the grid, or transfer it.",
                                r.name, lv.f.name
                            ),
                        ),
                    }
                }
                let gx = |i: usize| xs.get(i).copied();
                let gy = |i: usize| ys.get(i).copied();
                let half = |a: Option<f64>, c: f64| a.map_or(0.0, |a| (a - c).abs() / 2.0);
                let wx = half(ix.checked_sub(1).and_then(gx), x) + half(gx(ix + 1), x);
                let wy = half(iy.checked_sub(1).and_then(gy), y) + half(gy(iy + 1), y);
                let trib = wx * wy / SQFT * floors_above.max(1) as f64;
                b.member(
                    MemberKind::Column,
                    lv,
                    p,
                    p,
                    lv.elev,
                    lv.top,
                    lv.top - lv.elev,
                    trib,
                    &why,
                );
            }
        }
    }

    // Framing at each floor above the lowest (the lowest is on grade) and at the roof.
    let mut floors: Vec<(usize, f64, bool)> = lvs
        .iter()
        .filter(|lv| lv.i > 0)
        .map(|lv| (lv.i, lv.elev, false))
        .collect();
    if let Some(top) = lvs.last() {
        floors.push((top.i, top.top, true));
    }
    let (along_x_lines, cross_lines) = if dir == SpanDir::X {
        // Deck spans x: beams run y; girders run x on the y-lines.
        (&ys, &xs)
    } else {
        (&xs, &ys)
    };
    for &(i, elev, roof) in &floors {
        // The framing belongs to the level whose floor it carries; the roof's to the top
        // level. Its material is the level below it (what it sits on).
        let host = &lvs[i];
        let below = if roof { i } else { i.saturating_sub(1) };
        // Frames frame every floor; a podium frames the floors on its columns (its top is
        // the transfer slab); light frame uses joists (below).
        let framed = if !bearing {
            true
        } else {
            podium && !roof && i <= podium_levels
        };
        let tag = if roof {
            "Roof framing."
        } else {
            "Floor framing."
        };
        if framed {
            let mat_level = Lv {
                i: below,
                f: host.f,
                elev: host.elev,
                top: host.top,
            };
            let infill = match sr.material.as_str() {
                "steel" => rules.sizing.steel.beam_spacing_ft * MM_PER_FT,
                "timber" => 12.0 * MM_PER_FT,
                _ => 0.0,
            };
            let concrete = infill == 0.0;
            for w in cross_lines.windows(2) {
                for g in along_x_lines.windows(2) {
                    let (c0, c1, g0, g1) = (w[0], w[1], g[0], g[1]);
                    let (a, bpt) = if dir == SpanDir::X {
                        (Pt::new(c0, g0), Pt::new(c1, g1))
                    } else {
                        (Pt::new(g0, c0), Pt::new(g1, c1))
                    };
                    let mid = a.add(bpt).scale(0.5);
                    if !near_outline(&host.f.outline, mid, 0.0) {
                        continue;
                    }
                    let (bay_span, bay_other) = (c1 - c0, g1 - g0);
                    if concrete {
                        // Two-way flat plate: the bay's longer span.
                        let (s, e) = if dir == SpanDir::X {
                            (Pt::new(c0, mid.y), Pt::new(c1, mid.y))
                        } else {
                            (Pt::new(mid.x, c0), Pt::new(mid.x, c1))
                        };
                        let long = bay_span.max(bay_other);
                        b.member(
                            MemberKind::Span,
                            &mat_level,
                            s,
                            e,
                            elev,
                            elev,
                            long,
                            0.0,
                            &format!("{tag} Two-way flat plate over the bay."),
                        );
                        if long > span_max {
                            b.flag(FlagKind::SpanOutOfRange, Some(host.f.id), mid, format!("A {} bay exceeds the flat plate's {:.0}' range: drop panels or beams.", ft(long), sr.span_max_ft));
                        }
                        continue;
                    }
                    // Infill beams across the bay, the deck spanning between them.
                    let k = ((bay_span / infill).ceil() as usize).max(1);
                    for j in 1..k {
                        let t = c0 + bay_span * j as f64 / k as f64;
                        let (s, e) = if dir == SpanDir::X {
                            (Pt::new(t, g0), Pt::new(t, g1))
                        } else {
                            (Pt::new(g0, t), Pt::new(g1, t))
                        };
                        b.member(
                            MemberKind::Beam,
                            &mat_level,
                            s,
                            e,
                            elev,
                            elev,
                            bay_other,
                            0.0,
                            &format!("{tag} Infill beam, {} o.c.", ft(bay_span / k as f64)),
                        );
                    }
                    let (s, e) = if dir == SpanDir::X {
                        (Pt::new(c0, mid.y), Pt::new(c1, mid.y))
                    } else {
                        (Pt::new(mid.x, c0), Pt::new(mid.x, c1))
                    };
                    b.member(
                        MemberKind::Span,
                        &mat_level,
                        s,
                        e,
                        elev,
                        elev,
                        bay_span / k as f64,
                        0.0,
                        &format!("{tag} Deck/panel between beams."),
                    );
                    if bay_span.max(bay_other) > span_max {
                        b.flag(
                            FlagKind::SpanOutOfRange,
                            Some(host.f.id),
                            mid,
                            format!(
                                "A {} bay exceeds this system's {:.0}' typical span.",
                                ft(bay_span.max(bay_other)),
                                sr.span_max_ft
                            ),
                        );
                    }
                }
            }
            // Girders on the lines along the deck span, beams on the lines across it.
            for &g in along_x_lines.iter() {
                for w in cross_lines.windows(2) {
                    let (s, e) = if dir == SpanDir::X {
                        (Pt::new(w[0], g), Pt::new(w[1], g))
                    } else {
                        (Pt::new(g, w[0]), Pt::new(g, w[1]))
                    };
                    if concrete || !near_outline(&host.f.outline, s.add(e).scale(0.5), 300.0) {
                        continue;
                    }
                    b.member(
                        MemberKind::Girder,
                        &mat_level,
                        s,
                        e,
                        elev,
                        elev,
                        w[1] - w[0],
                        0.0,
                        &format!("{tag} Girder on a grid line, column to column."),
                    );
                }
            }
            for &c in cross_lines.iter() {
                for g in along_x_lines.windows(2) {
                    let (s, e) = if dir == SpanDir::X {
                        (Pt::new(c, g[0]), Pt::new(c, g[1]))
                    } else {
                        (Pt::new(g[0], c), Pt::new(g[1], c))
                    };
                    if concrete || !near_outline(&host.f.outline, s.add(e).scale(0.5), 300.0) {
                        continue;
                    }
                    b.member(
                        MemberKind::Beam,
                        &mat_level,
                        s,
                        e,
                        elev,
                        elev,
                        g[1] - g[0],
                        0.0,
                        &format!("{tag} Beam on a grid line, column to column."),
                    );
                }
            }
        }
    }

    // ---- Bearing walls and joists (light frame, and above a podium) ----
    if bearing {
        let wall_axis = if dir == SpanDir::X { Axis::Y } else { Axis::X };
        let joist_max = match sr.material.as_str() {
            "cfs" => rules.sizing.cfs.joists.last().map_or(24.0, |j| j.0),
            _ => rules.sizing.wood.joists.last().map_or(26.0, |j| j.0),
        } * MM_PER_FT;
        for lv in lvs.iter().filter(|lv| lv.i >= podium_levels) {
            let chosen: Vec<&WallFeature> = f
                .walls
                .iter()
                .filter(|w| w.level == lv.f.id && w.axis == wall_axis && w.length >= 1200.0)
                .collect();
            let mut lines: Vec<f64> = vec![];
            for w in &chosen {
                b.member(
                    MemberKind::BearingWall,
                    lv,
                    w.start,
                    w.end,
                    lv.elev,
                    lv.top,
                    lv.top - lv.elev,
                    0.0,
                    if w.exterior {
                        "Exterior wall bearing the joists."
                    } else {
                        "Interior wall bearing the joists."
                    },
                );
                lines.push(if wall_axis == Axis::Y {
                    (w.start.x + w.end.x) / 2.0
                } else {
                    (w.start.y + w.end.y) / 2.0
                });
                if !w.stacks {
                    let mid = w.start.add(w.end).scale(0.5);
                    b.member(
                        MemberKind::Transfer,
                        lv,
                        w.start,
                        w.end,
                        lv.elev,
                        lv.elev,
                        w.length,
                        0.0,
                        "Beam under a bearing wall that doesn't stack.",
                    );
                    b.flag(FlagKind::Transfer, Some(lv.f.id), mid, format!("A bearing wall on {} has no wall below: a transfer beam and posts carry it.", lv.f.name));
                }
                if podium && lv.i == podium_levels {
                    let on_grid = if wall_axis == Axis::Y {
                        xs.iter()
                            .any(|x| (x - (w.start.x + w.end.x) / 2.0).abs() < 600.0)
                    } else {
                        ys.iter()
                            .any(|y| (y - (w.start.y + w.end.y) / 2.0).abs() < 600.0)
                    };
                    if !on_grid {
                        b.flag(FlagKind::Transfer, Some(lv.f.id), w.start.add(w.end).scale(0.5), format!("This wall lands between podium columns: the podium's transfer slab carries it at {}.", lv.f.name));
                    }
                }
            }
            lines.sort_by(|a, c| a.total_cmp(c));
            lines.dedup_by(|a, c| (*a - *c).abs() < 300.0);
            // Joists across each bay between bearing lines, at this floor and the roof.
            let mut at = vec![(lv.elev, "Floor joists.")];
            if lv.i == n - 1 {
                at.push((lv.top, "Roof joists/rafters."));
            }
            for (elev, tag) in at {
                if elev == lv.elev && lv.i == 0 {
                    continue;
                }
                for g in lines.windows(2) {
                    let gap = g[1] - g[0];
                    if gap < 600.0 {
                        continue;
                    }
                    let mid_c = (g[0] + g[1]) / 2.0;
                    let other = if wall_axis == Axis::Y {
                        (f.min.y + f.max.y) / 2.0
                    } else {
                        (f.min.x + f.max.x) / 2.0
                    };
                    let (s, e) = if wall_axis == Axis::Y {
                        (Pt::new(g[0], other), Pt::new(g[1], other))
                    } else {
                        (Pt::new(other, g[0]), Pt::new(other, g[1]))
                    };
                    let mid = if wall_axis == Axis::Y {
                        Pt::new(mid_c, other)
                    } else {
                        Pt::new(other, mid_c)
                    };
                    if !near_outline(&lv.f.outline, mid, 0.0) {
                        continue;
                    }
                    if gap > joist_max {
                        // A beam line mid-bay halves the joist span.
                        let (bs, be) = if wall_axis == Axis::Y {
                            (Pt::new(mid_c, f.min.y), Pt::new(mid_c, f.max.y))
                        } else {
                            (Pt::new(f.min.x, mid_c), Pt::new(f.max.x, mid_c))
                        };
                        b.member(
                            MemberKind::Girder,
                            lv,
                            bs,
                            be,
                            elev,
                            elev,
                            settings.grid_x.min(settings.grid_y),
                            0.0,
                            "Beam line halving a bay too long for joists.",
                        );
                        b.flag(FlagKind::SpanOutOfRange, Some(lv.f.id), mid, format!("A {} bay between bearing walls is beyond joist range: a beam line and posts mid-bay.", ft(gap)));
                        b.member(
                            MemberKind::Span,
                            lv,
                            s,
                            mid,
                            elev,
                            elev,
                            gap / 2.0,
                            0.0,
                            tag,
                        );
                        b.member(
                            MemberKind::Span,
                            lv,
                            mid,
                            e,
                            elev,
                            elev,
                            gap / 2.0,
                            0.0,
                            tag,
                        );
                    } else {
                        b.member(MemberKind::Span, lv, s, e, elev, elev, gap, 0.0, tag);
                    }
                }
            }
        }
    }

    // ---- Lateral ----
    let ratio = sr.lateral_ratio.get(settings.seismic);
    let dims = (f.max.x - f.min.x, f.max.y - f.min.y);
    let frames = matches!(
        settings.lateral,
        LateralKind::BracedFrames | LateralKind::MomentFrames
    );
    let mut below_sel: Vec<(Pt, Pt)> = vec![];
    for lv in &lvs {
        let lateral = if podium && lv.i < podium_levels {
            LateralKind::ConcreteShearWalls
        } else {
            settings.lateral
        };
        let frames_here = frames && !(podium && lv.i < podium_levels);
        let mut sel: Vec<(Pt, Pt, bool)> = vec![]; // start, end, along x
        for along_x in [true, false] {
            let dim = if along_x { dims.0 } else { dims.1 };
            let want = ratio * dim;
            let center = if along_x {
                (f.min.y + f.max.y) / 2.0
            } else {
                (f.min.x + f.max.x) / 2.0
            };
            let mut got = 0.0;
            if frames_here {
                // Frames in grid bays: perimeter lines first, bays inside solid walls first.
                let (lines, cross) = if along_x { (&ys, &xs) } else { (&xs, &ys) };
                let mut bays: Vec<(f64, Pt, Pt)> = vec![];
                for (li, &l) in lines.iter().enumerate() {
                    let perimeter = li == 0 || li + 1 == lines.len();
                    for w in cross.windows(2) {
                        let (s, e) = if along_x {
                            (Pt::new(w[0], l), Pt::new(w[1], l))
                        } else {
                            (Pt::new(l, w[0]), Pt::new(l, w[1]))
                        };
                        if !near_outline(&lv.f.outline, s.add(e).scale(0.5), 300.0) {
                            continue;
                        }
                        let in_wall = f.walls.iter().any(|wf| {
                            wf.level == lv.f.id
                                && on_wall(wf, s).0 < 300.0
                                && on_wall(wf, e).0 < 300.0
                                && wf.opening_length < 1.0
                        });
                        let score =
                            if perimeter { 2.0 } else { 0.0 } + if in_wall { 1.0 } else { 0.0 };
                        bays.push((score, s, e));
                    }
                }
                bays.sort_by(|a, c| c.0.total_cmp(&a.0));
                let all_perimeter = lateral == LateralKind::MomentFrames;
                let (mut lo_side, mut hi_side) = (0.0, 0.0);
                for (score, s, e) in bays {
                    let side = if along_x { s.y } else { s.x };
                    let lower = side < center;
                    let enough = got >= want && lo_side > 0.0 && hi_side > 0.0;
                    if all_perimeter {
                        if score < 2.0 {
                            continue;
                        }
                    } else if enough || (lower && lo_side > hi_side + 1.0 && hi_side < want / 2.0) {
                        continue;
                    }
                    let len = s.dist(e);
                    got += len;
                    if lower {
                        lo_side += len
                    } else {
                        hi_side += len
                    }
                    sel.push((s, e, along_x));
                }
            } else {
                // Shear walls in solid, stacking wall segments.
                let min_len = (lv.top - lv.elev) / sr.shear_aspect;
                let mut segs: Vec<(f64, Pt, Pt)> = vec![];
                for w in f.walls.iter().filter(|w| {
                    w.level == lv.f.id && w.axis == if along_x { Axis::X } else { Axis::Y }
                }) {
                    if !w.stacks && lv.i > 0 {
                        continue;
                    }
                    let u = w.end.sub(w.start).scale(1.0 / w.length.max(1.0));
                    for (a, c) in &w.solid {
                        if c - a < min_len {
                            continue;
                        }
                        let (s, e) = (w.start.add(u.scale(*a)), w.start.add(u.scale(*c)));
                        let over_below = below_sel.iter().any(|(bs, be)| {
                            studio_geom::project_to_segment(s.add(e).scale(0.5), *bs, *be).1 < 600.0
                        });
                        let pri = 3.0 * f64::from(u8::from(over_below))
                            + 2.0
                                * f64::from(u8::from(
                                    w.at_core && lateral == LateralKind::ConcreteShearWalls,
                                ))
                            + 1.5 * f64::from(u8::from(w.exterior))
                            + (c - a) / 10_000.0;
                        segs.push((pri, s, e));
                    }
                }
                segs.sort_by(|a, c| c.0.total_cmp(&a.0));
                let (mut lo_side, mut hi_side) = (0.0, 0.0);
                for (_, s, e) in segs {
                    if got >= want && lo_side > 0.0 && hi_side > 0.0 {
                        break;
                    }
                    let side = if along_x { s.y } else { s.x };
                    let lower = side < center;
                    // Keep both sides going for symmetry.
                    if got >= want && ((lower && lo_side > 0.0) || (!lower && hi_side > 0.0)) {
                        continue;
                    }
                    let len = s.dist(e);
                    got += len;
                    if lower {
                        lo_side += len
                    } else {
                        hi_side += len
                    }
                    sel.push((s, e, along_x));
                }
            }
            if got < want {
                let at = if along_x {
                    Pt::new((f.min.x + f.max.x) / 2.0, f.min.y)
                } else {
                    Pt::new(f.min.x, (f.min.y + f.max.y) / 2.0)
                };
                b.flag(
                    FlagKind::LateralDirection,
                    Some(lv.f.id),
                    at,
                    format!(
                        "{}: {} of {} in the {} direction, about {} wanted ({:.0}% of {}).",
                        lv.f.name,
                        lateral.label().to_lowercase(),
                        ft(got),
                        if along_x {
                            "east–west"
                        } else {
                            "north–south"
                        },
                        ft(want),
                        ratio * 100.0,
                        ft(dim)
                    ),
                );
            }
        }
        // Members, stacking and balance.
        let kind = match lateral {
            LateralKind::BracedFrames if frames_here => MemberKind::BracedFrame,
            LateralKind::MomentFrames if frames_here => MemberKind::MomentFrame,
            _ => MemberKind::ShearWall,
        };
        let (mut sx, mut lx, mut sy, mut ly) = (0.0, 0.0, 0.0, 0.0);
        let saved_mat = b.material.clone();
        if podium && lv.i < podium_levels {
            b.material = "concrete".into();
        }
        for (s, e, along_x) in &sel {
            let len = s.dist(*e);
            let mid = s.add(*e).scale(0.5);
            b.member(kind, lv, *s, *e, lv.elev, lv.top, lv.top - lv.elev, 0.0, "Lateral: chosen for a solid, stacking segment, perimeter and core first, balanced side to side.");
            if *along_x {
                sy += len * mid.y;
                ly += len;
            } else {
                sx += len * mid.x;
                lx += len;
            }
            if lv.i > 0
                && !below_sel
                    .iter()
                    .any(|(bs, be)| studio_geom::project_to_segment(mid, *bs, *be).1 < 600.0)
            {
                b.flag(FlagKind::NonStackingLateral, Some(lv.f.id), mid, format!("This lateral element on {} has none below it: its load needs a path down (collector or transfer).", lv.f.name));
            }
        }
        b.material = saved_mat;
        let cm = centroid(&lv.f.outline);
        let share = rules.general.torsion_share;
        if lx > 0.0 && ly > 0.0 {
            let cr = Pt::new(sx / lx, sy / ly);
            if (cr.x - cm.x).abs() > share * dims.0 || (cr.y - cm.y).abs() > share * dims.1 {
                b.flag(FlagKind::Torsion, Some(lv.f.id), cr, format!("On {}, the lateral elements' centre ({}, {} from the centre of mass) is off-centre: expect torsion; balance the sides.", lv.f.name, ft((cr.x - cm.x).abs()), ft((cr.y - cm.y).abs())));
            }
        }
        out.notes.push(format!(
            "{}: {} of {} east–west and {} north–south.",
            lv.f.name,
            ft(ly),
            lateral.label().to_lowercase(),
            ft(lx)
        ));
        below_sel = sel.iter().map(|(s, e, _)| (*s, *e)).collect();
    }

    // ---- Foundations (ADR-083) ----
    foundations(f, rules, settings, &lvs, &xs, &ys, &mut b);

    // The model's discontinuities, as flags.
    for d in &f.discontinuities {
        let kind = match d.kind {
            DiscontinuityKind::NonStackingWall if !bearing => continue,
            DiscontinuityKind::NonStackingWall => continue,
            _ => FlagKind::Discontinuity,
        };
        b.flag(kind, Some(d.level), d.at, d.message.clone());
    }
    let _ = merge;
    out.members = b.members;
    out.flags = b.flags;
    out
}
