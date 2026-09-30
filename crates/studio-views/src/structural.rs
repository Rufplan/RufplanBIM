//! The structural overlay (ADR-080): the structural layer drawn over the greyed-out
//! architecture, per level in plans and as boxes in 3D, with picking for its hover/click
//! information. Colours and transparency are the canvas's; this gives geometry and kinds.
//!
//! Preliminary — not engineered. Requires review by a licensed structural engineer.

use serde::Serialize;
use studio_core::structural::{MemberKind, StructLayout, StructMember, DISCLAIMER};
use studio_core::units::format_ft_in;
use studio_core::{Category, Document, ElementData, ElementId, ViewKind};
use studio_geom::Pt;
use ts_rs::TS;

/// What an overlay piece is (its colour on the canvas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum OverlayKind {
    Grid,
    Column,
    Girder,
    Beam,
    BearingWall,
    ShearWall,
    BracedFrame,
    MomentFrame,
    Span,
    Transfer,
    /// Foundations (ADR-083).
    SpreadFooting,
    StripFooting,
    Mat,
    PileCap,
    FoundationWall,
    Flag,
    /// The MEPT layers (ADR-082): an item, coloured by its `mep` kind, and a zone.
    Mep,
    MepZone,
}

impl From<MemberKind> for OverlayKind {
    fn from(k: MemberKind) -> Self {
        match k {
            MemberKind::Column => OverlayKind::Column,
            MemberKind::Girder => OverlayKind::Girder,
            MemberKind::Beam => OverlayKind::Beam,
            MemberKind::BearingWall => OverlayKind::BearingWall,
            MemberKind::ShearWall => OverlayKind::ShearWall,
            MemberKind::BracedFrame => OverlayKind::BracedFrame,
            MemberKind::MomentFrame => OverlayKind::MomentFrame,
            MemberKind::Span => OverlayKind::Span,
            MemberKind::Transfer => OverlayKind::Transfer,
            MemberKind::SpreadFooting => OverlayKind::SpreadFooting,
            MemberKind::StripFooting => OverlayKind::StripFooting,
            MemberKind::Mat => OverlayKind::Mat,
            MemberKind::PileCap => OverlayKind::PileCap,
            MemberKind::FoundationWall => OverlayKind::FoundationWall,
        }
    }
}

/// One piece of the plan overlay, in model mm.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OverlayPrim {
    pub kind: OverlayKind,
    /// Index into the layout's members or flags.
    #[ts(optional)]
    pub member: Option<u32>,
    #[ts(optional)]
    pub flag: Option<u32>,
    #[ts(optional)]
    pub mep: Option<studio_core::mep::MepKind>,
    /// Filled areas.
    pub fill: Vec<Vec<[f64; 2]>>,
    /// Lines.
    pub lines: Vec<Vec<[f64; 2]>>,
    /// A label and where it goes, paper mm high.
    #[ts(optional)]
    pub label: Option<(String, [f64; 2])>,
}

/// A 3D piece: a triangle soup (9 floats per triangle, mm, z up).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OverlayMesh {
    pub kind: OverlayKind,
    #[ts(optional)]
    pub member: Option<u32>,
    #[ts(optional)]
    pub flag: Option<u32>,
    #[ts(optional)]
    pub mep: Option<studio_core::mep::MepKind>,
    pub positions: Vec<f32>,
}

/// What hovering or clicking an overlay piece shows.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OverlayInfo {
    pub kind: OverlayKind,
    #[ts(optional)]
    pub mep: Option<studio_core::mep::MepKind>,
    pub title: String,
    pub lines: Vec<String>,
}

/// The structural layer, if the project has one.
pub fn scheme(doc: &Document) -> Option<(ElementId, &StructLayout)> {
    doc.of(Category::StructuralScheme)
        .find_map(|e| match &e.data {
            ElementData::StructuralScheme { layout, .. } => Some((e.id, layout)),
            _ => None,
        })
}

pub(crate) fn p2(p: Pt) -> [f64; 2] {
    [p.x, p.y]
}

/// A band `w` wide along `a`–`b`.
pub(crate) fn band(a: Pt, b: Pt, w: f64) -> Vec<[f64; 2]> {
    let d = b.sub(a);
    let len = d.len();
    let n = if len > 1e-6 {
        d.scale(1.0 / len).perp().scale(w / 2.0)
    } else {
        Pt::new(0.0, w / 2.0)
    };
    vec![p2(a.add(n)), p2(b.add(n)), p2(b.sub(n)), p2(a.sub(n))]
}

/// A rectangle between corners `a` and `b`.
fn rect(a: Pt, b: Pt) -> Vec<[f64; 2]> {
    let (lo, hi) = (
        Pt::new(a.x.min(b.x), a.y.min(b.y)),
        Pt::new(a.x.max(b.x), a.y.max(b.y)),
    );
    vec![[lo.x, lo.y], [hi.x, lo.y], [hi.x, hi.y], [lo.x, hi.y]]
}

/// A ring as a closed polyline (its outline).
fn close(mut r: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    if let Some(f) = r.first().copied() {
        r.push(f);
    }
    r
}

pub(crate) fn square(c: Pt, w: f64) -> Vec<[f64; 2]> {
    let h = w / 2.0;
    vec![
        [c.x - h, c.y - h],
        [c.x + h, c.y - h],
        [c.x + h, c.y + h],
        [c.x - h, c.y + h],
    ]
}

pub(crate) fn ring(c: Pt, r: f64) -> Vec<[f64; 2]> {
    (0..=24)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 24.0;
            [c.x + r * a.cos(), c.y + r * a.sin()]
        })
        .collect()
}

/// A structural grid line's name, S-prefixed so it isn't taken for the architectural
/// grid: numbers for lines along y (at x), letters for lines along x.
fn grid_name(i: usize, along_y: bool) -> String {
    format!("S{}", grid_base(i, along_y))
}

fn grid_base(i: usize, along_y: bool) -> String {
    if along_y {
        (i + 1).to_string()
    } else {
        let letters: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ".chars().collect();
        let mut s = String::new();
        let mut n = i;
        loop {
            s.insert(0, letters[n % letters.len()]);
            if n < letters.len() {
                break;
            }
            n = n / letters.len() - 1;
        }
        s
    }
}

/// The level a plan view shows, and its elevation.
pub(crate) fn plan_level(doc: &Document, view: ElementId) -> Option<(ElementId, f64)> {
    let ElementData::View { kind, .. } = doc.data(view).ok()? else {
        return None;
    };
    let level = match kind {
        ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level } => *level,
        _ => return None,
    };
    Some((level, doc.level_elevation(level).ok()?))
}

/// Whether `m` shows in the plan of `level` at `elev`: what stands on the level, the floor
/// framing at it, and (when the level has no floor framing) the roof framing above it.
fn in_plan(l: &StructLayout, m: &StructMember, level: ElementId, elev: f64) -> bool {
    if m.level != level {
        return false;
    }
    match m.kind {
        MemberKind::Beam | MemberKind::Girder | MemberKind::Span | MemberKind::Transfer => {
            let floor = (m.top - elev).abs() < 1.0;
            let has_floor = l.members.iter().any(|o| {
                o.level == level
                    && matches!(
                        o.kind,
                        MemberKind::Beam | MemberKind::Girder | MemberKind::Span
                    )
                    && (o.top - elev).abs() < 1.0
            });
            floor || !has_floor
        }
        _ => true,
    }
}

/// The overlay for a plan view (empty for other views or without a layer). `scale` sizes
/// grid bubbles and arrows on paper.
pub fn overlay_2d(doc: &Document, view: ElementId) -> Vec<OverlayPrim> {
    let (Some((_, l)), Some((level, elev))) = (scheme(doc), plan_level(doc, view)) else {
        return vec![];
    };
    let scale = match doc.data(view) {
        Ok(ElementData::View { scale, .. }) => f64::from(*scale),
        _ => 48.0,
    };
    let paper = |mm: f64| mm * scale;
    let mut out = vec![];
    // Grid lines with bubbles.
    let (xs, ys) = (&l.grid_x, &l.grid_y);
    if let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) =
        (xs.first(), xs.last(), ys.first(), ys.last())
    {
        let ext = paper(10.0);
        let r = paper(4.0);
        for (i, &x) in xs.iter().enumerate() {
            let top = Pt::new(x, y1 + ext);
            out.push(OverlayPrim {
                kind: OverlayKind::Grid,
                member: None,
                flag: None,
                mep: None,
                fill: vec![],
                lines: vec![
                    vec![[x, y0 - ext], p2(top)],
                    ring(top.add(Pt::new(0.0, r)), r),
                ],
                label: Some((grid_name(i, true), p2(top.add(Pt::new(0.0, r))))),
            });
        }
        for (i, &y) in ys.iter().enumerate() {
            let left = Pt::new(x0 - ext, y);
            out.push(OverlayPrim {
                kind: OverlayKind::Grid,
                member: None,
                flag: None,
                mep: None,
                fill: vec![],
                lines: vec![
                    vec![p2(left), [x1 + ext, y]],
                    ring(left.sub(Pt::new(r, 0.0)), r),
                ],
                label: Some((grid_name(i, false), p2(left.sub(Pt::new(r, 0.0))))),
            });
        }
    }
    for (i, m) in l.members.iter().enumerate() {
        if !in_plan(l, m, level, elev) {
            continue;
        }
        let kind = OverlayKind::from(m.kind);
        let mid = m.start.add(m.end).scale(0.5);
        let (fill, lines, label) = match m.kind {
            MemberKind::Column => (vec![square(m.start, m.width.max(paper(1.5)))], vec![], None),
            // Foundations below the floor, as a foundation plan draws them.
            MemberKind::SpreadFooting | MemberKind::PileCap => {
                let sq = square(m.start, m.width);
                (vec![sq.clone()], vec![close(sq)], None)
            }
            MemberKind::StripFooting | MemberKind::FoundationWall => {
                let bd = band(m.start, m.end, m.width);
                (vec![bd.clone()], vec![close(bd)], None)
            }
            MemberKind::Mat => {
                let r = rect(m.start, m.end);
                (
                    vec![r.clone()],
                    vec![close(r)],
                    Some(("MAT".to_string(), p2(mid))),
                )
            }
            MemberKind::BearingWall | MemberKind::ShearWall => (
                vec![band(m.start, m.end, m.width.max(paper(1.0)))],
                vec![],
                (m.kind == MemberKind::ShearWall).then(|| ("SW".to_string(), p2(mid))),
            ),
            MemberKind::BracedFrame | MemberKind::MomentFrame => {
                let b = band(m.start, m.end, paper(2.0));
                let x = vec![vec![b[0], b[2]], vec![b[1], b[3]]];
                let mut lines = vec![vec![p2(m.start), p2(m.end)]];
                if m.kind == MemberKind::BracedFrame {
                    lines.extend(x);
                }
                (
                    vec![],
                    lines,
                    Some((
                        if m.kind == MemberKind::BracedFrame {
                            "BF"
                        } else {
                            "MF"
                        }
                        .into(),
                        p2(mid),
                    )),
                )
            }
            MemberKind::Span => {
                // A span arrow, with half-heads at both ends.
                let d = m.end.sub(m.start);
                let len = d.len().max(1.0);
                let u = d.scale(1.0 / len);
                let (h, n) = (paper(2.5).min(len / 4.0), u.perp().scale(paper(1.2)));
                let a = m.start.add(u.scale(paper(1.0)));
                let b = m.end.sub(u.scale(paper(1.0)));
                (
                    vec![],
                    vec![
                        vec![p2(a), p2(b)],
                        vec![p2(a.add(u.scale(h)).add(n)), p2(a)],
                        vec![p2(b.sub(u.scale(h)).sub(n)), p2(b)],
                    ],
                    Some((m.size.replace(" (prelim.)", ""), p2(mid.add(n.scale(1.5))))),
                )
            }
            _ => (
                vec![band(m.start, m.end, m.width.max(paper(0.8)))],
                vec![vec![p2(m.start), p2(m.end)]],
                None,
            ),
        };
        out.push(OverlayPrim {
            kind,
            member: Some(i as u32),
            flag: None,
            mep: None,
            fill,
            lines,
            label,
        });
    }
    for (i, f) in l.flags.iter().enumerate() {
        if f.level.is_some_and(|x| x != level) {
            continue;
        }
        let r = paper(3.0);
        let tri = vec![
            [f.at.x, f.at.y + r],
            [f.at.x + r * 0.87, f.at.y - r * 0.5],
            [f.at.x - r * 0.87, f.at.y - r * 0.5],
        ];
        out.push(OverlayPrim {
            kind: OverlayKind::Flag,
            member: None,
            flag: Some(i as u32),
            mep: None,
            fill: vec![tri],
            lines: vec![],
            label: Some(("!".into(), [f.at.x, f.at.y - r * 0.1])),
        });
    }
    out
}

fn push_quad(v: &mut Vec<f32>, a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) {
    for t in [a, b, c, a, c, d] {
        v.extend(t.iter().map(|x| *x as f32));
    }
}

/// A box on base `ring` (4 corners) from `z0` to `z1`.
pub(crate) fn prism(ring: &[[f64; 2]], z0: f64, z1: f64) -> Vec<f32> {
    let mut v = vec![];
    let n = ring.len();
    let at = |i: usize, z: f64| [ring[i % n][0], ring[i % n][1], z];
    for i in 0..n {
        push_quad(&mut v, at(i, z0), at(i + 1, z0), at(i + 1, z1), at(i, z1));
    }
    push_quad(&mut v, at(0, z1), at(1, z1), at(2, z1), at(3, z1));
    push_quad(&mut v, at(3, z0), at(2, z0), at(1, z0), at(0, z0));
    v
}

/// A slanted strut from `a` at `za` to `b` at `zb`, `w` square.
fn strut(a: Pt, za: f64, b: Pt, zb: f64, w: f64) -> Vec<f32> {
    let d = b.sub(a);
    let len = d.len().max(1.0);
    let n = d.scale(1.0 / len).perp().scale(w / 2.0);
    let h = w / 2.0;
    let q = |p: Pt, z: f64, s: f64, t: f64| [p.x + n.x * s, p.y + n.y * s, z + h * t];
    let mut v = vec![];
    let corners = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
    for i in 0..4 {
        let (s0, t0) = corners[i];
        let (s1, t1) = corners[(i + 1) % 4];
        push_quad(
            &mut v,
            q(a, za, s0, t0),
            q(b, zb, s0, t0),
            q(b, zb, s1, t1),
            q(a, za, s1, t1),
        );
    }
    v
}

/// The overlay in 3D: members as boxes, braces as struts, flags as markers.
pub fn overlay_3d(doc: &Document) -> Vec<OverlayMesh> {
    let Some((_, l)) = scheme(doc) else {
        return vec![];
    };
    let mut out = vec![];
    for (i, m) in l.members.iter().enumerate() {
        let kind = OverlayKind::from(m.kind);
        let positions = match m.kind {
            MemberKind::Column => prism(&square(m.start, m.width.max(150.0)), m.base, m.top),
            MemberKind::SpreadFooting | MemberKind::PileCap => {
                prism(&square(m.start, m.width.max(300.0)), m.base, m.top)
            }
            MemberKind::StripFooting | MemberKind::FoundationWall => {
                prism(&band(m.start, m.end, m.width.max(150.0)), m.base, m.top)
            }
            MemberKind::Mat => prism(&rect(m.start, m.end), m.base, m.top),
            MemberKind::BearingWall | MemberKind::ShearWall => {
                prism(&band(m.start, m.end, m.width.max(100.0)), m.base, m.top)
            }
            MemberKind::BracedFrame => {
                let w = 150.0;
                let mut v = strut(m.start, m.base, m.end, m.top, w);
                v.extend(strut(m.end, m.base, m.start, m.top, w));
                v
            }
            MemberKind::MomentFrame => prism(
                &band(m.start, m.end, m.width.max(200.0)),
                m.top - m.depth.max(450.0),
                m.top,
            ),
            MemberKind::Span => continue,
            _ => prism(
                &band(m.start, m.end, m.width.max(100.0)),
                m.top - m.depth.max(150.0),
                m.top,
            ),
        };
        out.push(OverlayMesh {
            kind,
            member: Some(i as u32),
            flag: None,
            mep: None,
            positions,
        });
    }
    for (i, f) in l.flags.iter().enumerate() {
        let z = f
            .level
            .and_then(|lv| doc.level_elevation(lv).ok())
            .unwrap_or(0.0)
            + 2400.0;
        let s = 450.0;
        out.push(OverlayMesh {
            kind: OverlayKind::Flag,
            member: None,
            flag: Some(i as u32),
            mep: None,
            positions: prism(&square(f.at, s), z, z + s),
        });
    }
    out
}

pub(crate) fn dist_to_poly(p: Pt, ring: &[[f64; 2]], closed: bool) -> f64 {
    let pts: Vec<Pt> = ring.iter().map(|q| Pt::new(q[0], q[1])).collect();
    let n = pts.len();
    if n == 0 {
        return f64::INFINITY;
    }
    if closed {
        let mut c = false;
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + n - 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                c = !c;
            }
        }
        if c {
            return 0.0;
        }
    }
    let segs = if closed { n } else { n - 1 };
    (0..segs)
        .map(|i| studio_geom::project_to_segment(p, pts[i], pts[(i + 1) % n]).1)
        .fold(f64::INFINITY, f64::min)
}

/// The overlay piece under `at` in a plan (members and flags, not grid lines), within `tol`.
pub fn pick(doc: &Document, view: ElementId, at: Pt, tol: f64) -> Option<OverlayInfo> {
    let prims = overlay_2d(doc, view);
    let best = prims
        .iter()
        .filter(|p| p.kind != OverlayKind::Grid)
        .map(|p| {
            let d = p
                .fill
                .iter()
                .map(|r| dist_to_poly(at, r, true))
                .chain(p.lines.iter().map(|l| dist_to_poly(at, l, false)))
                .fold(f64::INFINITY, f64::min);
            // Flags and columns win ties over what they sit on.
            let bias = match p.kind {
                OverlayKind::Flag => -tol,
                OverlayKind::Column => -tol / 2.0,
                _ => 0.0,
            };
            (d + bias, p)
        })
        .filter(|(d, _)| *d <= tol)
        .min_by(|a, b| a.0.total_cmp(&b.0))?
        .1;
    info(doc, best.member, best.flag)
}

/// The hover/click text for member `member` or flag `flag`.
pub fn info(doc: &Document, member: Option<u32>, flag: Option<u32>) -> Option<OverlayInfo> {
    let (_, l) = scheme(doc)?;
    let level_name = |id: ElementId| doc.data(id).map(|d| d.name()).unwrap_or_default();
    if let Some(i) = flag {
        let f = l.flags.get(i as usize)?;
        let mut lines = vec![f.message.clone()];
        if let Some(lv) = f.level {
            lines.push(format!("Level: {}", level_name(lv)));
        }
        lines.push(DISCLAIMER.into());
        return Some(OverlayInfo {
            kind: OverlayKind::Flag,
            mep: None,
            title: f.kind.label().into(),
            lines,
        });
    }
    let m = l.members.get(member? as usize)?;
    let span_label = match m.kind {
        MemberKind::Column
        | MemberKind::BearingWall
        | MemberKind::ShearWall
        | MemberKind::BracedFrame
        | MemberKind::MomentFrame
        | MemberKind::FoundationWall => "Height",
        MemberKind::SpreadFooting | MemberKind::PileCap | MemberKind::StripFooting => "Width",
        MemberKind::Mat => "Length",
        _ => "Span",
    };
    Some(OverlayInfo {
        kind: m.kind.into(),
        mep: None,
        title: format!("{} — {}", m.kind.label(), m.size),
        lines: vec![
            format!("{span_label}: {}", format_ft_in(m.span)),
            format!("Level: {}", level_name(m.level)),
            format!("Rule: {}", m.rule),
            DISCLAIMER.into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ops;
    use studio_core::structural::*;

    fn doc_with_layer() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let levels = doc.levels();
        let (l1, l2) = (levels[0].0, levels[1].0);
        let (e1, e2) = (levels[0].2, levels[1].2);
        let m = |kind, level, a: Pt, b: Pt, base, top| StructMember {
            kind,
            level,
            start: a,
            end: b,
            base,
            top,
            size: "W12x26 (prelim.)".into(),
            depth: 300.0,
            width: 160.0,
            span: a.dist(b).max(top - base),
            rule: "Test rule.".into(),
        };
        let layout = StructLayout {
            grid_x: vec![0.0, 9000.0],
            grid_y: vec![0.0, 6000.0],
            members: vec![
                m(
                    MemberKind::Column,
                    l1,
                    Pt::new(0.0, 0.0),
                    Pt::new(0.0, 0.0),
                    e1,
                    e2,
                ),
                m(
                    MemberKind::Girder,
                    l2,
                    Pt::new(0.0, 0.0),
                    Pt::new(9000.0, 0.0),
                    e2 - 300.0,
                    e2,
                ),
                m(
                    MemberKind::BracedFrame,
                    l1,
                    Pt::new(0.0, 6000.0),
                    Pt::new(9000.0, 6000.0),
                    e1,
                    e2,
                ),
                m(
                    MemberKind::SpreadFooting,
                    l1,
                    Pt::new(0.0, 0.0),
                    Pt::new(0.0, 0.0),
                    e1 - 800.0,
                    e1 - 100.0,
                ),
            ],
            flags: vec![StructFlag {
                kind: FlagKind::Transfer,
                level: Some(l1),
                at: Pt::new(4500.0, 3000.0),
                message: "Transfer here.".into(),
            }],
            notes: vec![DISCLAIMER.into()],
        };
        doc.transact("layer", |tx| {
            Ok(tx.insert(ElementData::StructuralScheme {
                settings: SchemeSettings {
                    kind: SchemeKind::SteelFrame,
                    seismic: Seismic::Moderate,
                    grid_x: 9000.0,
                    grid_y: 6000.0,
                    lateral: LateralKind::BracedFrames,
                    span_dir: SpanDir::Auto,
                },
                layout,
            }))
        })
        .unwrap();
        let plan = |l| {
            doc.of(Category::View)
                .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l))
                .unwrap()
                .id
        };
        let (p1, p2) = (plan(l1), plan(l2));
        (doc, p1, p2)
    }

    #[test]
    fn plans_show_their_level_and_picking_explains_it() {
        let (doc, p1, p2) = doc_with_layer();
        let o1 = overlay_2d(&doc, p1);
        let kinds: Vec<OverlayKind> = o1.iter().map(|p| p.kind).collect();
        // Two grid lines each way, the column and frame on Level 1, its flag; not the girder.
        assert_eq!(kinds.iter().filter(|k| **k == OverlayKind::Grid).count(), 4);
        assert!(kinds.contains(&OverlayKind::Column) && kinds.contains(&OverlayKind::BracedFrame));
        assert!(kinds.contains(&OverlayKind::Flag) && !kinds.contains(&OverlayKind::Girder));
        let o2 = overlay_2d(&doc, p2);
        // The footing shows in the lowest plan, filled and outlined.
        let pad = o1
            .iter()
            .find(|p| p.kind == OverlayKind::SpreadFooting)
            .unwrap();
        assert_eq!((pad.fill.len(), pad.lines.len()), (1, 1));
        assert!(!o2.iter().any(|p| p.kind == OverlayKind::SpreadFooting));
        assert!(o2.iter().any(|p| p.kind == OverlayKind::Girder));
        // Grid bubbles: 1, 2 and A, B.
        let labels: Vec<String> = o1
            .iter()
            .filter_map(|p| p.label.clone())
            .map(|l| l.0)
            .collect();
        for n in ["S1", "S2", "SA", "SB"] {
            assert!(labels.contains(&n.to_string()));
        }
        let hit = pick(&doc, p1, Pt::new(4500.0, 3000.0), 300.0).unwrap();
        assert_eq!(hit.title, "Transfer Condition");
        let col = pick(&doc, p1, Pt::new(20.0, 20.0), 300.0).unwrap();
        assert!(col.title.starts_with("Column — W12x26"));
        assert!(col.lines.iter().any(|l| l == "Rule: Test rule."));
        assert!(col.lines.contains(&DISCLAIMER.to_string()));
        // 3D: boxes for the column and girder, two struts, a flag marker.
        let m3 = overlay_3d(&doc);
        assert_eq!(m3.len(), 5);
        assert!(m3
            .iter()
            .all(|m| m.positions.len() % 9 == 0 && !m.positions.is_empty()));
        assert_eq!(grid_name(25, false), "SAB");
    }
}
