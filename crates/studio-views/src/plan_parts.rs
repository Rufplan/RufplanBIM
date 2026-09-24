//! Plan graphics for stairs, columns, beams, railings and wall layer detail (ADR-019).

use super::{clip_line_convex, ring, Anchor, Builder, Dash, FillKind};
use studio_core::ElementId;
use studio_geom::{clip_half_plane, Pt};
use studio_regen::{Hatch, Model, WallSolid};

/// Stairs based on this level (treads up to the cut plane, a break line, and an UP
/// arrow) and stairs arriving from below (all treads and DN).
pub(crate) fn stairs_in_plan(b: &mut Builder, model: &Model, level: ElementId, cut: f64) {
    for s in &model.stairs {
        let up = s.base_level == level;
        let down = s.top_level == level;
        if !up && !down {
            continue;
        }
        let el = Some(s.id);
        for o in s.footprint() {
            b.fill(el, vec![ring(&o)], FillKind::Room);
        }
        // Distance along the walking line where the cut plane crosses, run by run.
        let mut walk: Vec<Pt> = vec![];
        let mut broke = false;
        let mut walk_done = false;
        for (i, r) in s.runs.iter().enumerate() {
            let run = s.tread * r.treads as f64;
            let n = r.dir.perp().scale(s.width / 2.0);
            let cut_at = if up {
                (((cut - r.z0) / s.riser).floor() * s.tread).clamp(0.0, run)
            } else {
                run
            };
            let at = |t: f64| r.start.add(r.dir.scale(t));
            for k in 0..=r.treads {
                let t = k as f64 * s.tread;
                let dash = if up && t > cut_at + 1.0 {
                    Dash::Dashed
                } else {
                    Dash::Solid
                };
                b.line(el, &[at(t).add(n), at(t).sub(n)], false, 1, dash);
            }
            for edge in [n, n.scale(-1.0)] {
                if cut_at > 0.0 {
                    b.line(
                        el,
                        &[at(0.0).add(edge), at(cut_at).add(edge)],
                        false,
                        2,
                        Dash::Solid,
                    );
                }
                if cut_at < run {
                    b.line(
                        el,
                        &[at(cut_at).add(edge), at(run).add(edge)],
                        false,
                        2,
                        Dash::Dashed,
                    );
                }
            }
            if up && !broke && cut_at > 0.0 && cut_at < run {
                // Diagonal break line across the run at the cut.
                let c = at(cut_at);
                let skew = r.dir.scale(s.tread * 0.8);
                b.line(
                    el,
                    &[c.add(n).add(skew), c.sub(n).sub(skew)],
                    false,
                    2,
                    Dash::Solid,
                );
                broke = true;
            }
            // Walking line: through each run's center and the landing's middle, stopping at
            // the cut plane going up.
            if walk_done {
                continue;
            }
            if i > 0 {
                if let Some((l, _)) = s.landings.get(i - 1) {
                    let c = l.iter().fold(Pt::default(), |a, p| a.add(*p));
                    walk.push(c.scale(1.0 / l.len() as f64));
                }
            }
            let first = if i == 0 { s.tread * 0.5 } else { 0.0 };
            let last = if i + 1 == s.runs.len() {
                run - s.tread * 0.5
            } else {
                run
            };
            walk.push(at(first));
            if up && (broke || cut_at < run) {
                walk.push(at((cut_at - s.tread * 0.5).max(first)));
                walk_done = true;
            } else {
                walk.push(at(last));
            }
        }
        for (l, z) in &s.landings {
            let dash = if up && *z > cut {
                Dash::Dashed
            } else {
                Dash::Solid
            };
            b.line(el, l, true, 2, dash);
        }
        if !up {
            walk.reverse();
        }
        walk.dedup_by(|a, b| a.dist(*b) < 1.0);
        if walk.len() < 2 {
            continue;
        }
        b.line(el, &walk, false, 1, Dash::Solid);
        let (to, prev) = (walk[walk.len() - 1], walk[walk.len() - 2]);
        let from = walk[0];
        let back = prev.sub(to).norm();
        let head = b.paper(2.0);
        let wing = back.perp().scale(head * 0.5);
        b.line(
            el,
            &[
                to.add(back.scale(head)).add(wing),
                to,
                to.add(back.scale(head)).sub(wing),
            ],
            false,
            1,
            Dash::Solid,
        );
        let lead = walk[1].sub(from).norm();
        b.text(
            el,
            from.sub(lead.scale(b.paper(3.0))),
            if up { "UP".into() } else { "DN".into() },
            2.5,
            Anchor::Center,
        );
    }
}

/// Columns through the cut plane in poché (lighter for architectural columns), columns
/// below it as outlines.
pub(crate) fn columns_in_plan(b: &mut Builder, model: &Model, elev: f64, cut: f64) {
    for c in &model.columns {
        let el = Some(c.id);
        if c.z0 <= cut && c.z1 > cut {
            let fill = if c.structural {
                FillKind::Poche
            } else {
                FillKind::PocheLight
            };
            b.fill(el, vec![ring(&c.base.outer)], fill);
            b.line(el, &c.base.outer, true, 4, Dash::Solid);
        } else if c.z1 <= cut && c.z1 > elev + 1.0 {
            b.fill(el, vec![ring(&c.base.outer)], FillKind::Room);
            b.line(el, &c.base.outer, true, 2, Dash::Solid);
        }
    }
}

/// Beams the plan cuts through (poché) and beams overhead up to the next level (dashed,
/// as structural framing is shown in a floor plan), with their centerlines.
pub(crate) fn beams_in_plan(b: &mut Builder, model: &Model, elev: f64, cut: f64) {
    let next = model
        .levels
        .iter()
        .map(|l| l.elevation)
        .filter(|z| *z > elev + 1.0)
        .fold(f64::INFINITY, f64::min);
    for m in &model.beams {
        let el = Some(m.id);
        let z0 = m.z_top - m.depth;
        let n = m.end.sub(m.start).norm().perp().scale(m.width / 2.0);
        let outline = [m.start.sub(n), m.end.sub(n), m.end.add(n), m.start.add(n)];
        if z0 <= cut && m.z_top > cut {
            b.fill(el, vec![ring(&outline)], FillKind::Poche);
            b.line(el, &outline, true, 4, Dash::Solid);
        } else if z0 > cut && z0 < next + 1.0 {
            b.fill(el, vec![ring(&outline)], FillKind::Room);
            b.line(el, &outline, true, 2, Dash::Dashed);
            b.line(el, &[m.start, m.end], false, 1, Dash::Center);
        } else if m.z_top <= cut && m.z_top > elev + 1.0 {
            b.fill(el, vec![ring(&outline)], FillKind::Room);
            b.line(el, &outline, true, 2, Dash::Solid);
        }
    }
}

/// Railings on this level (their top rail's outline) and a stair's handrails in the plans
/// of its base level.
pub(crate) fn railings_in_plan(b: &mut Builder, model: &Model, level: ElementId) {
    for r in model.railings.iter().filter(|r| r.level == level) {
        let el = Some(r.id);
        // Boxes come in (top rail, bottom rail) pairs per segment; the top rail reads.
        for bx in r.boxes.iter().step_by(2) {
            let o: Vec<Pt> = [0, 1, 5, 4]
                .iter()
                .map(|&i| Pt::new(bx[i][0], bx[i][1]))
                .collect();
            b.fill(el, vec![ring(&o)], FillKind::Room);
            b.line(el, &o, true, 2, Dash::Solid);
        }
    }
}

fn pieces_at_cut(w: &WallSolid, cut: f64) -> impl Iterator<Item = &Vec<Pt>> {
    w.pieces
        .iter()
        .filter(move |p| p.z0 <= cut && p.z1 > cut)
        .map(|p| &p.base.outer)
}

/// Wall layers at detail scales: finish layers wrap around free ends and opening jambs,
/// and hatched layers get their cut pattern.
pub(crate) fn wall_layer_detail(b: &mut Builder, walls: &[&WallSolid], cut: f64) {
    let solid_at = |p: Pt| {
        walls
            .iter()
            .any(|w| pieces_at_cut(w, cut).any(|r| studio_geom::point_in_ring(p, r)))
    };
    for w in walls.iter().filter(|w| !w.layers.is_empty()) {
        let el = Some(w.id);
        let (d, n) = (w.dir(), w.dir().perp());
        let h = w.thickness / 2.0;
        let (wo, wi) = w.wraps;
        let wrap = wo.max(wi);
        for piece in pieces_at_cut(w, cut) {
            let Some((s0, e0)) = clip_line_convex(w.start, d, piece) else {
                continue;
            };
            // A free end has no wall material just beyond it.
            let free_s = wrap > 0.0 && !solid_at(s0.sub(d.scale(2.0)));
            let free_e = wrap > 0.0 && !solid_at(e0.add(d.scale(2.0)));
            let (ts, te) = (s0.sub(w.start).dot(d), e0.sub(w.start).dot(d));
            for off in &w.layers {
                let a = w.start.add(n.scale(*off));
                let Some((mut s, mut e)) = clip_line_convex(a, d, piece) else {
                    continue;
                };
                if free_s && s.sub(w.start).dot(d) < ts + 1.0 {
                    s = s.add(d.scale(wrap));
                }
                if free_e && e.sub(w.start).dot(d) > te - 1.0 {
                    e = e.sub(d.scale(wrap));
                }
                if e.sub(s).dot(d) > 1.0 {
                    b.line(el, &[s, e], false, 1, Dash::Solid);
                }
            }
            // The finish returning across the end.
            let (hi, lo) = (h - wo, -h + wi);
            for (free, t, sign) in [(free_s, ts, 1.0), (free_e, te, -1.0)] {
                if free {
                    let c = w.start.add(d.scale(t + sign * wrap));
                    b.line(
                        el,
                        &[c.add(n.scale(hi)), c.add(n.scale(lo))],
                        false,
                        1,
                        Dash::Solid,
                    );
                }
            }
            for (o_hi, o_lo, kind) in &w.hatches {
                // The layer's band of this piece, pulled back from wrapped ends.
                let mut band = clip_half_plane(piece, w.start.add(n.scale(*o_hi)), n.scale(-1.0));
                band = clip_half_plane(&band, w.start.add(n.scale(*o_lo)), n);
                if free_s {
                    band = clip_half_plane(&band, w.start.add(d.scale(ts + wrap)), d);
                }
                if free_e {
                    band = clip_half_plane(&band, w.start.add(d.scale(te - wrap)), d.scale(-1.0));
                }
                if band.len() >= 3 {
                    hatch(b, el, &band, w.start, d, n, *o_hi, *o_lo, *kind);
                }
            }
        }
    }
}

/// Fills a convex layer band with its pattern. The band runs along `d` between offsets
/// `hi` and `lo` (along `n` from `origin`).
#[allow(clippy::too_many_arguments)]
fn hatch(
    b: &mut Builder,
    el: Option<ElementId>,
    band: &[Pt],
    origin: Pt,
    d: Pt,
    n: Pt,
    hi: f64,
    lo: f64,
    kind: Hatch,
) {
    let ts: Vec<f64> = band.iter().map(|p| p.sub(origin).dot(d)).collect();
    let (t0, t1) = (
        ts.iter().copied().fold(f64::INFINITY, f64::min),
        ts.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    let thick = (hi - lo).abs();
    let at = |t: f64, o: f64| origin.add(d.scale(t)).add(n.scale(o));
    match kind {
        Hatch::Insulation => {
            // Batt zigzag: one peak per layer thickness.
            let step = thick / 2.0;
            let mut pts = vec![];
            let mut t = t0;
            let mut k = 0;
            while t <= t1 + 0.1 {
                pts.push(at(t, if k % 2 == 0 { lo } else { hi }));
                t += step;
                k += 1;
            }
            if pts.len() >= 2 {
                b.line(el, &pts, false, 1, Dash::Solid);
            }
        }
        Hatch::Masonry | Hatch::Concrete => {
            let spacing = if kind == Hatch::Masonry {
                b.paper(1.2)
            } else {
                b.paper(2.4)
            };
            // 45° lines along the band, clipped to it.
            let diag = d.add(n).norm();
            let across = diag.perp();
            let (mut lo_s, mut hi_s) = (f64::INFINITY, f64::NEG_INFINITY);
            for p in band {
                let s = p.sub(origin).dot(across);
                lo_s = lo_s.min(s);
                hi_s = hi_s.max(s);
            }
            let mut s = (lo_s / spacing).ceil() * spacing;
            while s < hi_s {
                let a = origin.add(across.scale(s));
                if let Some((p, q)) = clip_line_convex(a, diag, band) {
                    let dash = if kind == Hatch::Concrete {
                        Dash::Dashed
                    } else {
                        Dash::Solid
                    };
                    b.line(el, &[p, q], false, 1, dash);
                }
                s += spacing;
            }
            if kind == Hatch::Concrete {
                // Aggregate: small triangles at a staggered interval.
                let r = thick.min(b.paper(1.6)) * 0.18;
                let mut t = t0 + b.paper(1.5);
                let mut k = 0;
                while t < t1 - r {
                    let o = lo + thick * if k % 2 == 0 { 0.3 } else { 0.7 };
                    let c = at(t, o);
                    b.line(
                        el,
                        &[
                            c.add(n.scale(r)),
                            c.sub(n.scale(r * 0.5)).add(d.scale(r)),
                            c.sub(n.scale(r * 0.5)).sub(d.scale(r)),
                        ],
                        true,
                        1,
                        Dash::Solid,
                    );
                    t += b.paper(2.2);
                    k += 1;
                }
            }
        }
    }
}
