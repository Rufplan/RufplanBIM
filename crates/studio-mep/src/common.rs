//! What the four disciplines share: scoring a system against the building, the proposal's
//! text, and layout helpers (trunks and branches, grids of devices, finding rooms).

use studio_core::mep::{Climate, Discipline, MepFlag, MepItem, MepKind, MepSettings};
use studio_core::units::MM_PER_FT;
use studio_core::ElementId;
use studio_geom::Pt;

use crate::features::{bbox, inside, SQFT};
use crate::rules::{Rules, SystemRules};
use crate::types::*;

pub(crate) fn ft(mm: f64) -> String {
    format!("{:.0}'", mm / MM_PER_FT)
}

pub(crate) fn sf(mm2: f64) -> String {
    let v = (mm2 / SQFT).round() as i64;
    let s = v.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    format!("{out} sf")
}

pub(crate) fn climate_label(c: Climate) -> &'static str {
    match c {
        Climate::Hot => "hot",
        Climate::Mixed => "mixed",
        Climate::Cold => "cold",
    }
}

/// Scores one system: size (area and stories, a hard story limit), use, climate when it
/// matters, and the discipline's own criteria in `extra`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn score(
    f: &Features,
    key: &str,
    sr: &SystemRules,
    discipline: Discipline,
    climate: Climate,
    extra: Vec<Criterion>,
    highlights: Vec<String>,
    mut red: Vec<String>,
) -> SystemProposal {
    let area_sf = f.area / SQFT;
    let ruled_out = f.stories > sr.max_stories;
    let mut crit = vec![];
    let size = if ruled_out {
        0.0
    } else if area_sf > sr.max_area_sf {
        (1.0 - (area_sf - sr.max_area_sf) / sr.max_area_sf).max(0.1)
    } else if area_sf < sr.min_area_sf {
        0.4 + 0.5 * (area_sf / sr.min_area_sf.max(1.0))
    } else {
        1.0
    };
    crit.push(Criterion {
        name: "Size".into(),
        score: size,
        note: if ruled_out {
            format!(
                "{} stories is beyond its usual {}.",
                f.stories, sr.max_stories
            )
        } else if area_sf > sr.max_area_sf {
            format!(
                "{} is past its usual {} sf.",
                crate::common::sf(f.area),
                sr.max_area_sf.round()
            )
        } else if area_sf < sr.min_area_sf {
            format!(
                "It usually pays off from about {} sf; this is {}.",
                sr.min_area_sf.round(),
                crate::common::sf(f.area)
            )
        } else {
            format!(
                "{} over {} stor{} fits it.",
                crate::common::sf(f.area),
                f.stories,
                if f.stories == 1 { "y" } else { "ies" }
            )
        },
    });
    let occ = if f.uses.is_empty() {
        0.6
    } else {
        let (mut s, mut w) = (0.0, 0.0);
        for (i, u) in f.uses.iter().take(3).enumerate() {
            let k = 1.0 / f64::from(1u32 << i);
            s += k * sr.uses.get(u).copied().unwrap_or(0.5);
            w += k;
        }
        s / w
    };
    crit.push(Criterion {
        name: "Use".into(),
        score: occ,
        note: if f.uses.is_empty() {
            "No uses read from room names; scored as neutral.".into()
        } else {
            format!("Uses by floor area: {}.", f.uses.join(", "))
        },
    });
    if let Some(c) = &sr.climate {
        crit.push(Criterion {
            name: "Climate".into(),
            score: c.get(climate),
            note: format!(
                "Suitability in a {} climate (your setting).",
                climate_label(climate)
            ),
        });
    }
    crit.extend(extra);
    let weight = |c: &Criterion| match c.name.as_str() {
        "Size" => 3.0,
        "Use" => 2.0,
        "Climate" => 1.0,
        _ => 2.0,
    };
    let total: f64 = crit.iter().map(|c| c.score * weight(c)).sum::<f64>()
        / crit.iter().map(weight).sum::<f64>()
        * 100.0;
    let total = if ruled_out {
        total.min(30.0) * 0.5
    } else {
        total
    };
    let good: Vec<String> = crit
        .iter()
        .filter(|c| c.score >= 0.85)
        .map(|c| c.name.to_lowercase())
        .collect();
    let bad: Vec<String> = crit
        .iter()
        .filter(|c| c.score < 0.6)
        .map(|c| c.note.clone())
        .collect();
    let mut rationale = format!("{} scores {:.0}/100.", sr.label, total);
    if !good.is_empty() {
        rationale.push_str(&format!(" It fits on {}.", good.join(", ")));
    }
    if !bad.is_empty() {
        rationale.push_str(&format!(" Concerns: {}", bad.join(" ")));
    }
    if ruled_out {
        red.insert(0, format!("Beyond its usual {} stories.", sr.max_stories));
    }
    SystemProposal {
        key: key.into(),
        label: sr.label.clone(),
        description: sr.description.clone(),
        score: (total * 10.0).round() / 10.0,
        ruled_out,
        criteria: crit,
        rationale,
        highlights,
        red_flags: red,
        settings: MepSettings {
            discipline,
            system: key.into(),
            climate,
        },
    }
}

/// Ranks the proposals best first (ties by key, for determinism).
pub(crate) fn rank(mut v: Vec<SystemProposal>) -> Vec<SystemProposal> {
    v.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.key.cmp(&b.key)));
    v
}

/// The summary lines every discipline starts with.
pub(crate) fn summary(f: &Features) -> Vec<String> {
    let mut s = vec![format!(
        "{} stor{}, {} of floor.",
        f.stories,
        if f.stories == 1 { "y" } else { "ies" },
        sf(f.area)
    )];
    s.push(format!("{} rooms read from their names.", f.spaces.len()));
    if !f.uses.is_empty() {
        s.push(format!("Uses by area: {}.", f.uses.join(", ")));
    }
    if f.residential {
        s.push(format!(
            "{} dwelling unit{} (one per kitchen).",
            f.dwelling_units,
            if f.dwelling_units == 1 { "" } else { "s" }
        ));
    }
    s
}

/// A trunk along the level's long axis through `from`, and a branch square to it to each
/// target: how ducts, pipes and cable trays run to rooms.
pub(crate) fn trunk_and_branches(
    outline: &[Pt],
    from: Pt,
    targets: &[Pt],
) -> (Vec<Pt>, Vec<(Pt, Pt)>) {
    let Some((lo, hi)) = bbox(outline) else {
        return (vec![], vec![]);
    };
    let along_x = hi.x - lo.x >= hi.y - lo.y;
    let (a, b) = if along_x { (lo.x, hi.x) } else { (lo.y, hi.y) };
    let at = if along_x {
        from.y.clamp(lo.y, hi.y)
    } else {
        from.x.clamp(lo.x, hi.x)
    };
    let proj = |p: Pt| if along_x { p.x } else { p.y };
    let (mut t0, mut t1) = (proj(from), proj(from));
    for t in targets {
        t0 = t0.min(proj(*t));
        t1 = t1.max(proj(*t));
    }
    let (t0, t1) = (t0.max(a), t1.min(b));
    let on = |t: f64| {
        if along_x {
            Pt::new(t, at)
        } else {
            Pt::new(at, t)
        }
    };
    let mut trunk = vec![];
    if t1 - t0 > 1.0 {
        trunk = vec![on(t0), on(t1)];
    }
    // The source joins the trunk square to it.
    let join = on(proj(from).clamp(t0, t1));
    let mut branches = vec![];
    if from.dist(join) > 1.0 {
        branches.push((from, join));
    }
    for t in targets {
        let foot = on(proj(*t).clamp(t0, t1));
        if foot.dist(*t) > 1.0 {
            branches.push((foot, *t));
        }
    }
    (trunk, branches)
}

/// Points on a grid `spacing` apart inside `ring` (its centre if none fit).
pub(crate) fn grid_in(ring: &[Pt], spacing: f64) -> Vec<Pt> {
    let Some((lo, hi)) = bbox(ring) else {
        return vec![];
    };
    let (w, h) = (hi.x - lo.x, hi.y - lo.y);
    let nx = (w / spacing).round().max(1.0) as usize;
    let ny = (h / spacing).round().max(1.0) as usize;
    let mut out = vec![];
    for i in 0..nx {
        for j in 0..ny {
            let p = Pt::new(
                lo.x + w * (i as f64 + 0.5) / nx as f64,
                lo.y + h * (j as f64 + 0.5) / ny as f64,
            );
            if inside(ring, p) {
                out.push(p);
            }
        }
    }
    if out.is_empty() {
        out.push(crate::features::centroid(ring));
    }
    out
}

/// Points along a room's walls about `spacing` apart, `inset` inside them.
pub(crate) fn along_walls(ring: &[Pt], spacing: f64, inset: f64) -> Vec<Pt> {
    let c = crate::features::centroid(ring);
    let n = ring.len();
    let mut out = vec![];
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let len = a.dist(b);
        if len < spacing * 0.5 {
            continue;
        }
        let k = (len / spacing).floor().max(1.0) as usize;
        let d = b.sub(a).scale(1.0 / len);
        let mut nrm = d.perp();
        if c.sub(a).dot(nrm) < 0.0 {
            nrm = nrm.scale(-1.0);
        }
        for j in 0..k {
            let t = len * (j as f64 + 0.5) / k as f64;
            out.push(a.add(d.scale(t)).add(nrm.scale(inset)));
        }
    }
    out
}

/// The first room of these kinds on `level` (any level when None), nearest `to`.
pub(crate) fn room_of<'a>(
    f: &'a Features,
    level: Option<ElementId>,
    kinds: &[&str],
    to: Pt,
) -> Option<&'a Space> {
    kinds.iter().find_map(|k| {
        f.spaces
            .iter()
            .filter(|s| s.kind == *k && level.is_none_or(|l| s.level == l))
            .min_by(|a, b| a.center.dist(to).total_cmp(&b.center.dist(to)))
    })
}

/// The middle of the building's plan.
pub(crate) fn middle(f: &Features) -> Pt {
    f.min.add(f.max).scale(0.5)
}

/// A point just outside the exterior wall of `level` nearest `to`, and that wall's outward
/// direction.
pub(crate) fn outside_near(f: &Features, level: ElementId, to: Pt, off: f64) -> Option<(Pt, Pt)> {
    f.exterior_walls
        .iter()
        .filter(|w| w.level == level)
        .map(|w| {
            let (t, d) = studio_geom::project_to_segment(to, w.start, w.end);
            (
                d,
                w.start
                    .lerp(w.end, t)
                    .add(w.outward.scale(w.thickness / 2.0 + off)),
                w.outward,
            )
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p, n)| (p, n))
}

/// A point just inside the exterior wall nearest `to` (for wall units).
pub(crate) fn inside_near(f: &Features, level: ElementId, to: Pt, off: f64) -> Option<Pt> {
    f.exterior_walls
        .iter()
        .filter(|w| w.level == level)
        .map(|w| {
            let (t, d) = studio_geom::project_to_segment(to, w.start, w.end);
            (
                d,
                w.start
                    .lerp(w.end, t)
                    .sub(w.outward.scale(w.thickness / 2.0 + off)),
            )
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p)
}

/// A builder for items and flags.
pub(crate) struct Out {
    pub items: Vec<MepItem>,
    pub flags: Vec<MepFlag>,
}

impl Out {
    pub fn new() -> Self {
        Self {
            items: vec![],
            flags: vec![],
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn item(
        &mut self,
        kind: MepKind,
        level: &Level,
        pts: Vec<Pt>,
        z: (f64, f64),
        size: impl Into<String>,
        width: f64,
        rule: impl Into<String>,
    ) {
        if pts.is_empty() {
            return;
        }
        self.items.push(MepItem {
            kind,
            level: level.id,
            pts,
            base: z.0,
            top: z.1,
            size: format!("{} (prelim.)", size.into()),
            width,
            rule: rule.into(),
        });
    }
    pub fn flag(&mut self, title: &str, level: Option<ElementId>, at: Pt, message: String) {
        if !self
            .flags
            .iter()
            .any(|f| f.title == title && f.level == level && f.at.dist(at) < 1.0)
        {
            self.flags.push(MepFlag {
                title: title.into(),
                level,
                at,
                message,
            });
        }
    }
}

/// Heights: a device at the ceiling, a run just above it, equipment on the floor, of `lv`.
pub(crate) fn ceiling(rules: &Rules, f: &Features, lv: &Level) -> f64 {
    (lv.elevation + ceiling_ft(rules, f) * MM_PER_FT).min(lv.top - 150.0)
}

/// The assumed ceiling height, ft: lower in homes.
pub(crate) fn ceiling_ft(rules: &Rules, f: &Features) -> f64 {
    if f.residential {
        rules.general.ceiling_ft_residential
    } else {
        rules.general.ceiling_ft
    }
}
