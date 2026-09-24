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
    for pair in fits.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if a.1.host == b.1.host && b.1.t0 < a.1.t1 - SLOP {
            return Err(CoreError::Invalid(format!("{} overlaps {}", b.0, a.0)));
        }
    }
    Ok(())
}
