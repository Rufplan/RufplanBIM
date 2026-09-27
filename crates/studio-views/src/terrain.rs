//! The site's terrain in 3D (ADR-045), as Revit's toposolid: the ground as a block cut out
//! of the earth (its sides down to a base below the lowest ground), and contour lines on
//! its surface with their elevations.

use serde::Serialize;
use studio_core::units::format_ft_in;
use studio_core::{Document, ElementId};
use studio_regen::regenerate;
use ts_rs::TS;

/// How far below the lowest ground the terrain's block goes (mm): 10'.
pub const TERRAIN_DEPTH: f64 = 3048.0;
/// Contour lines float this far over the ground so they don't flicker into it (mm).
const LIFT: f64 = 25.0;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TerrainContour {
    /// Absolute elevation, mm.
    pub elevation: f64,
    /// Every fifth interval, drawn heavier.
    pub major: bool,
    /// Segments on the ground, 6 floats each (project mm, z-up).
    pub segments: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ContourLabel {
    /// On the contour (project mm, z-up), at the middle of its longest run.
    pub at: [f64; 3],
    pub text: String,
    pub major: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Terrain {
    pub site: ElementId,
    /// The contour interval, mm (the Site's Contour Interval property).
    pub interval: f64,
    /// The block's sides and bottom, triangles (9 floats each).
    pub skirt: Vec<f32>,
    pub contours: Vec<TerrainContour>,
    pub labels: Vec<ContourLabel>,
}

/// An elevation for a contour label: whole feet as `412'`, else feet and inches.
fn elevation_text(mm: f64) -> String {
    let ft = mm / 304.8;
    if (ft - ft.round()).abs() < 1e-6 {
        format!("{:.0}'", ft.round())
    } else {
        format_ft_in(mm)
    }
}

/// The terrain of the project's site, when it has topography.
pub fn terrain(doc: &Document) -> Option<Terrain> {
    let m = regenerate(doc);
    let s = m.site.as_ref()?;
    s.topo.as_ref()?;
    let base = s.lowest()? - TERRAIN_DEPTH;
    let ground = |p: studio_geom::Pt, level: f64| {
        let z = s.ground_at(p).unwrap_or(level - s.datum);
        [p.x, p.y, z + LIFT]
    };
    let mut contours = vec![];
    let mut labels = vec![];
    for (level, major, segs) in s.contours() {
        if segs.is_empty() {
            continue;
        }
        let segments: Vec<f32> = segs
            .iter()
            .flat_map(|[a, b]| {
                ground(*a, level)
                    .into_iter()
                    .chain(ground(*b, level))
                    .map(|x| x as f32)
            })
            .collect();
        if let Some([a, b]) = segs
            .iter()
            .max_by(|x, y| x[0].dist(x[1]).total_cmp(&y[0].dist(y[1])))
        {
            labels.push(ContourLabel {
                at: ground(a.lerp(*b, 0.5), level),
                text: elevation_text(level),
                major,
            });
        }
        contours.push(TerrainContour {
            elevation: level,
            major,
            segments,
        });
    }
    Some(Terrain {
        site: s.id,
        interval: s.contour,
        skirt: s.skirt(base),
        contours,
        labels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_labels_read_as_elevations() {
        assert_eq!(elevation_text(412.0 * 304.8), "412'");
        assert_eq!(elevation_text(412.5 * 304.8), "412'-6\"");
    }
}
