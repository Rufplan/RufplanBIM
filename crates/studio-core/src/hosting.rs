//! Where hosted elements (doors, windows) sit in their host wall, and whether they fit.

use crate::document::{CoreError, CoreResult};
use crate::element::{ElementData, ElementId, WallTop};

/// An opening's extent in its host wall's local frame. Lengths in mm; `t` runs along the
/// location line from the wall start, `z` up from the wall base.
#[derive(Debug, Clone, PartialEq)]
pub struct OpeningFit {
    pub host: ElementId,
    pub t0: f64,
    pub t1: f64,
    pub z0: f64,
    pub z1: f64,
    pub wall_length: f64,
    pub wall_height: f64,
}

fn level_elevation<'a>(
    get: &impl Fn(ElementId) -> Option<&'a ElementData>,
    id: ElementId,
) -> Option<f64> {
    match get(id)? {
        ElementData::Level { elevation, .. } => Some(*elevation),
        _ => None,
    }
}

/// Height of a wall (mm) from its base to its top constraint.
pub fn wall_height<'a>(
    get: &impl Fn(ElementId) -> Option<&'a ElementData>,
    wall: &ElementData,
) -> Option<f64> {
    let ElementData::Wall {
        base_level,
        base_offset,
        top,
        ..
    } = wall
    else {
        return None;
    };
    let z0 = level_elevation(get, *base_level)? + base_offset;
    Some(match top {
        WallTop::UpToLevel { level, offset } => level_elevation(get, *level)? + offset - z0,
        WallTop::Unconnected { height } => *height,
    })
}

/// The extent of a door or window in its host, or None for other elements or missing refs.
pub fn opening_fit<'a>(
    get: &impl Fn(ElementId) -> Option<&'a ElementData>,
    data: &ElementData,
) -> Option<OpeningFit> {
    let (type_id, host, offset, sill) = match data {
        ElementData::Door {
            type_id,
            host,
            offset,
            ..
        } => (*type_id, *host, *offset, 0.0),
        ElementData::Window {
            type_id,
            host,
            offset,
            sill,
            ..
        } => (*type_id, *host, *offset, *sill),
        _ => return None,
    };
    let (width, height) = match get(type_id)? {
        ElementData::DoorType { width, height, .. }
        | ElementData::WindowType { width, height, .. } => (*width, *height),
        _ => return None,
    };
    let wall = get(host)?;
    let ElementData::Wall { start, end, .. } = wall else {
        return None;
    };
    Some(OpeningFit {
        host,
        t0: offset - width / 2.0,
        t1: offset + width / 2.0,
        z0: sill,
        z1: sill + height,
        wall_length: start.dist(*end),
        wall_height: wall_height(get, wall)?,
    })
}

/// Checks every door and window: inside its host's length and height, and not
/// overlapping another opening in the same wall.
pub fn validate_openings<'a>(
    all: impl Iterator<Item = (ElementId, &'a ElementData)>,
    get: &impl Fn(ElementId) -> Option<&'a ElementData>,
) -> CoreResult<()> {
    const SLOP: f64 = 0.5;
    let mut fits: Vec<(String, OpeningFit)> = vec![];
    for (_, data) in all {
        let Some(fit) = opening_fit(get, data) else {
            continue;
        };
        if !matches!(get(fit.host), Some(ElementData::Wall { .. })) {
            return Err(CoreError::Invalid(format!(
                "{} must be hosted by a wall",
                data.name()
            )));
        }
        if fit.t0 < -SLOP || fit.t1 > fit.wall_length + SLOP {
            return Err(CoreError::Invalid(format!(
                "{} no longer fits along its wall",
                data.name()
            )));
        }
        if fit.z0 < -SLOP || fit.z1 > fit.wall_height + SLOP {
            return Err(CoreError::Invalid(format!(
                "{} is taller than its wall",
                data.name()
            )));
        }
        fits.push((data.name(), fit));
    }
    fits.sort_by(|a, b| {
        (a.1.host, a.1.t0)
            .partial_cmp(&(b.1.host, b.1.t0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Openings may share a stretch of wall one above the other (a clerestory over a
    // window, windows on two floors of a tall wall), but not overlap (ADR-035).
    for (i, a) in fits.iter().enumerate() {
        for b in fits[i + 1..].iter() {
            if b.1.host != a.1.host || b.1.t0 >= a.1.t1 - SLOP {
                break;
            }
            if b.1.z0 < a.1.z1 - SLOP && a.1.z0 < b.1.z1 - SLOP {
                return Err(CoreError::Invalid(format!("{} overlaps {}", b.0, a.0)));
            }
        }
    }
    Ok(())
}

/// How far the finish faces of the walls a wall runs into sit from its start and from its
/// end, along it (mm): where an opening's distance to the wall it meets is measured from
/// (Revit's temporary dimensions to the face). 0 at a free end.
pub fn end_faces(doc: &crate::Document, host: ElementId) -> (f64, f64) {
    use studio_geom::Pt;
    let wall_of = |id: ElementId| -> Option<(Pt, Pt, f64, ElementId)> {
        match doc.data(id).ok()? {
            ElementData::Wall {
                type_id,
                start,
                end,
                base_level,
                ..
            } => match doc.data(*type_id).ok()? {
                ElementData::WallType { thickness, .. } => {
                    Some((*start, *end, *thickness, *base_level))
                }
                _ => None,
            },
            _ => None,
        }
    };
    let Some((s, e, _, level)) = wall_of(host) else {
        return (0.0, 0.0);
    };
    let len = s.dist(e);
    if len < 1.0 {
        return (0.0, 0.0);
    }
    // From point `p` heading `d`: the farthest exit from a wall whose band holds `p`.
    let face = |p: Pt, d: Pt| -> f64 {
        let mut best: f64 = 0.0;
        for w in doc.of(crate::Category::Wall) {
            if w.id == host {
                continue;
            }
            let Some((a, b, t, l)) = wall_of(w.id) else {
                continue;
            };
            let wl = a.dist(b);
            if l != level || wl < 1.0 {
                continue;
            }
            let u = b.sub(a).scale(1.0 / wl);
            let n = u.perp();
            let along = p.sub(a).dot(u);
            let d0 = p.sub(a).dot(n);
            let k = d.dot(n);
            // Inside the other wall's band (its ends wrapped by half its thickness), and
            // crossing it rather than running along it.
            if d0.abs() > t / 2.0 + 1.0 || along < -t / 2.0 - 1.0 || along > wl + t / 2.0 + 1.0 {
                continue;
            }
            if k.abs() < 0.2 {
                continue;
            }
            let exit = (k.signum() * t / 2.0 - d0) / k;
            if exit > 0.0 && exit < len / 2.0 {
                best = best.max(exit);
            }
        }
        best
    };
    let d = e.sub(s).scale(1.0 / len);
    (face(s, d), face(e, d.scale(-1.0)))
}
