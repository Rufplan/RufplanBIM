//! The MEPT overlays (ADR-082): each discipline's layer drawn over the greyed-out
//! architecture, in plans per level and as boxes in 3D, with picking for hover/click
//! information. Built on the structural overlay's pieces; colours are the canvas's.
//!
//! Preliminary — not engineered. Requires review by a licensed engineer.

use studio_core::mep::{Discipline, MepKind, MepLayout, MepSettings};
use studio_core::units::format_ft_in;
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::Pt;

use crate::structural::{
    band, dist_to_poly, p2, plan_level, prism, square, OverlayInfo, OverlayKind, OverlayMesh,
    OverlayPrim,
};

/// The MEPT layers in the project, in discipline order.
pub fn layers(doc: &Document) -> Vec<(ElementId, &MepSettings, &MepLayout)> {
    let mut v: Vec<_> = doc
        .of(Category::MepScheme)
        .filter_map(|e| match &e.data {
            ElementData::MepScheme { settings, layout } => Some((e.id, settings, layout)),
            _ => None,
        })
        .collect();
    v.sort_by_key(|l| l.1.discipline);
    v
}

fn shown<'a>(
    doc: &'a Document,
    on: &[Discipline],
) -> Vec<(ElementId, &'a MepSettings, &'a MepLayout)> {
    layers(doc)
        .into_iter()
        .filter(|l| on.contains(&l.1.discipline))
        .collect()
}

/// Encodes a discipline and an index into the prim's member/flag fields.
fn code(d: Discipline, i: usize) -> u32 {
    (d as u32) << 24 | i as u32
}

fn decode(c: u32) -> (usize, usize) {
    ((c >> 24) as usize, (c & 0x00ff_ffff) as usize)
}

/// The plan overlay of the `on` disciplines for a plan view.
pub fn overlay_2d(doc: &Document, view: ElementId, on: &[Discipline]) -> Vec<OverlayPrim> {
    let Some((level, _)) = plan_level(doc, view) else {
        return vec![];
    };
    let scale = match doc.data(view) {
        Ok(ElementData::View { scale, .. }) => f64::from(*scale),
        _ => 48.0,
    };
    let paper = |mm: f64| mm * scale;
    let mut out = vec![];
    for (_, s, l) in shown(doc, on) {
        let d = s.discipline;
        for z in l.zones.iter().filter(|z| z.level == level) {
            let c = crate::structural::p2(centroid(&z.ring));
            out.push(OverlayPrim {
                kind: OverlayKind::MepZone,
                member: None,
                flag: None,
                mep: None,
                fill: vec![z.ring.iter().map(|p| p2(*p)).collect()],
                lines: vec![],
                label: Some((z.label.clone(), c)),
            });
        }
        for (i, it) in l
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| it.level == level)
        {
            let (fill, lines, label) = if it.pts.len() == 1 {
                let w = it.width.max(paper(2.0)).min(paper(6.0));
                let abbr = it.kind.abbr();
                (
                    vec![square(it.pts[0], w)],
                    vec![],
                    (!abbr.is_empty())
                        .then(|| (abbr.to_string(), p2(it.pts[0].add(Pt::new(0.0, w * 0.9))))),
                )
            } else {
                let mut fill = vec![];
                if it.width > 1.0 {
                    for seg in it.pts.windows(2) {
                        fill.push(band(seg[0], seg[1], it.width));
                    }
                }
                (fill, vec![it.pts.iter().map(|p| p2(*p)).collect()], None)
            };
            out.push(OverlayPrim {
                kind: OverlayKind::Mep,
                member: Some(code(d, i)),
                flag: None,
                mep: Some(it.kind),
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
            out.push(OverlayPrim {
                kind: OverlayKind::Flag,
                member: None,
                flag: Some(code(d, i)),
                mep: None,
                fill: vec![vec![
                    [f.at.x, f.at.y + r],
                    [f.at.x + r * 0.87, f.at.y - r * 0.5],
                    [f.at.x - r * 0.87, f.at.y - r * 0.5],
                ]],
                lines: vec![],
                label: Some(("!".into(), [f.at.x, f.at.y - r * 0.1])),
            });
        }
    }
    out
}

fn centroid(ring: &[Pt]) -> Pt {
    let n = ring.len().max(1) as f64;
    ring.iter()
        .fold(Pt::new(0.0, 0.0), |s, p| s.add(*p))
        .scale(1.0 / n)
}

/// The overlay in 3D: devices and equipment as boxes, runs as bars, risers as columns.
pub fn overlay_3d(doc: &Document, on: &[Discipline]) -> Vec<OverlayMesh> {
    let mut out = vec![];
    for (_, s, l) in shown(doc, on) {
        let d = s.discipline;
        for (i, it) in l.items.iter().enumerate() {
            let h = (it.top - it.base).max(60.0);
            let positions = if it.pts.len() == 1 {
                prism(
                    &square(it.pts[0], it.width.max(150.0)),
                    it.base,
                    it.base + h,
                )
            } else {
                let mut v = vec![];
                let w = it.width.max(if matches!(it.kind, MepKind::SupplyDuct) {
                    200.0
                } else {
                    60.0
                });
                let z1 = it.base + h.min(w.max(60.0));
                for seg in it.pts.windows(2) {
                    if seg[0].dist(seg[1]) > 1.0 {
                        v.extend(prism(&band(seg[0], seg[1], w), it.base, z1));
                    }
                }
                v
            };
            if positions.is_empty() {
                continue;
            }
            out.push(OverlayMesh {
                kind: OverlayKind::Mep,
                member: Some(code(d, i)),
                flag: None,
                mep: Some(it.kind),
                positions,
            });
        }
        for (i, f) in l.flags.iter().enumerate() {
            let z = f
                .level
                .and_then(|lv| doc.level_elevation(lv).ok())
                .unwrap_or(0.0)
                + 2400.0;
            out.push(OverlayMesh {
                kind: OverlayKind::Flag,
                member: None,
                flag: Some(code(d, i)),
                mep: None,
                positions: prism(&square(f.at, 450.0), z, z + 450.0),
            });
        }
    }
    out
}

/// The MEP piece under `at` in a plan, within `tol`.
pub fn pick(
    doc: &Document,
    view: ElementId,
    at: Pt,
    tol: f64,
    on: &[Discipline],
) -> Option<OverlayInfo> {
    let prims = overlay_2d(doc, view, on);
    let best = prims
        .iter()
        .filter(|p| p.kind != OverlayKind::MepZone)
        .map(|p| {
            let d = p
                .fill
                .iter()
                .map(|r| dist_to_poly(at, r, true))
                .chain(p.lines.iter().map(|l| dist_to_poly(at, l, false)))
                .fold(f64::INFINITY, f64::min);
            // Flags and point devices win over the runs they sit on.
            let bias = if p.kind == OverlayKind::Flag {
                -tol
            } else if p.lines.is_empty() {
                -tol / 2.0
            } else {
                0.0
            };
            (d + bias, p)
        })
        .filter(|(d, _)| *d <= tol)
        .min_by(|a, b| a.0.total_cmp(&b.0))?
        .1;
    info(doc, best.member, best.flag)
}

/// The hover/click text for an encoded item or flag.
pub fn info(doc: &Document, item: Option<u32>, flag: Option<u32>) -> Option<OverlayInfo> {
    let all = layers(doc);
    let level_name = |id: ElementId| doc.data(id).map(|d| d.name()).unwrap_or_default();
    if let Some(c) = flag {
        let (di, i) = decode(c);
        let (_, s, l) = all.iter().find(|l| l.1.discipline as usize == di)?;
        let f = l.flags.get(i)?;
        let mut lines = vec![f.message.clone()];
        if let Some(lv) = f.level {
            lines.push(format!("Level: {}", level_name(lv)));
        }
        lines.push(s.discipline.disclaimer().into());
        return Some(OverlayInfo {
            kind: OverlayKind::Flag,
            mep: None,
            title: format!("{} — {}", s.discipline.label(), f.title),
            lines,
        });
    }
    let (di, i) = decode(item?);
    let (_, s, l) = all.iter().find(|l| l.1.discipline as usize == di)?;
    let it = l.items.get(i)?;
    let mut lines = vec![];
    if it.pts.len() > 1 {
        let len: f64 = it.pts.windows(2).map(|w| w[0].dist(w[1])).sum();
        lines.push(format!("Length: {}", format_ft_in(len)));
    }
    lines.push(format!("Level: {}", level_name(it.level)));
    lines.push(format!("Rule: {}", it.rule));
    lines.push(s.discipline.disclaimer().into());
    Some(OverlayInfo {
        kind: OverlayKind::Mep,
        mep: Some(it.kind),
        title: format!("{} — {}", it.kind.label(), it.size),
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::mep::*;
    use studio_core::{ops, ViewKind};

    #[test]
    fn mep_layers_draw_per_level_and_discipline_and_explain_their_items() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let levels = doc.levels();
        let (l1, l2) = (levels[0].0, levels[1].0);
        let item = |kind, level, pts: Vec<Pt>| MepItem {
            kind,
            level,
            pts,
            base: 0.0,
            top: 300.0,
            size: "Test (prelim.)".into(),
            width: 300.0,
            rule: "Rule.".into(),
        };
        let mech = MepLayout {
            items: vec![
                item(MepKind::Diffuser, l1, vec![Pt::new(1000.0, 1000.0)]),
                item(
                    MepKind::SupplyDuct,
                    l1,
                    vec![Pt::new(0.0, 3000.0), Pt::new(6000.0, 3000.0)],
                ),
                item(MepKind::Diffuser, l2, vec![Pt::new(1000.0, 1000.0)]),
            ],
            zones: vec![MepZone {
                level: l1,
                ring: vec![
                    Pt::new(0.0, 0.0),
                    Pt::new(4000.0, 0.0),
                    Pt::new(4000.0, 4000.0),
                    Pt::new(0.0, 4000.0),
                ],
                label: "Z1".into(),
            }],
            flags: vec![MepFlag {
                title: "Interior Room".into(),
                level: Some(l1),
                at: Pt::new(5000.0, 5000.0),
                message: "No outside wall.".into(),
            }],
            notes: vec![],
        };
        let elec = MepLayout {
            items: vec![item(MepKind::Panel, l1, vec![Pt::new(8000.0, 1000.0)])],
            ..Default::default()
        };
        doc.transact("layers", |tx| {
            for (d, l) in [
                (Discipline::Mechanical, mech),
                (Discipline::Electrical, elec),
            ] {
                tx.insert(ElementData::MepScheme {
                    settings: MepSettings {
                        discipline: d,
                        system: "x".into(),
                        climate: Climate::Mixed,
                    },
                    layout: l,
                });
            }
            Ok(())
        })
        .unwrap();
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        let both = [Discipline::Mechanical, Discipline::Electrical];
        let o = overlay_2d(&doc, plan, &both);
        // Zone, diffuser and duct on Level 1 (not Level 2's diffuser), the flag, the panel.
        assert_eq!(
            o.iter().filter(|p| p.kind == OverlayKind::MepZone).count(),
            1
        );
        assert_eq!(
            o.iter()
                .filter(|p| p.mep == Some(MepKind::Diffuser))
                .count(),
            1
        );
        assert_eq!(
            o.iter()
                .filter(|p| p.mep == Some(MepKind::SupplyDuct))
                .count(),
            1
        );
        assert_eq!(o.iter().filter(|p| p.kind == OverlayKind::Flag).count(), 1);
        assert_eq!(
            o.iter().filter(|p| p.mep == Some(MepKind::Panel)).count(),
            1
        );
        // Only the disciplines turned on.
        let only = overlay_2d(&doc, plan, &[Discipline::Electrical]);
        assert!(only.iter().all(|p| p.mep == Some(MepKind::Panel)));
        let hit = pick(&doc, plan, Pt::new(8000.0, 1000.0), 300.0, &both).unwrap();
        assert_eq!(hit.title, "Panel — Test (prelim.)");
        assert_eq!(hit.mep, Some(MepKind::Panel));
        assert!(hit
            .lines
            .iter()
            .any(|l| l.contains("licensed electrical engineer")));
        let flag = pick(&doc, plan, Pt::new(5000.0, 5000.0), 300.0, &both).unwrap();
        assert_eq!(flag.title, "Mechanical — Interior Room");
        assert_eq!(overlay_3d(&doc, &both).len(), 5);
    }
}
