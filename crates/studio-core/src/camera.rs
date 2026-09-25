//! Cameras and the sun for renderings (ADR-027).
//!
//! A camera is a 3D view with a perspective eye and target, placed in a plan as Revit's
//! Camera tool does: click the eye, then the target, at an eye height above the plan's level.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, ViewKind};
use crate::units::MM_PER_FT;

/// Revit's default eye (and target) height above the level: 5'-6".
pub const DEFAULT_EYE_HEIGHT: f64 = 5.5 * MM_PER_FT;
/// Vertical field of view of a new camera, degrees.
pub const DEFAULT_FOV: f64 = 50.0;

/// A perspective camera on a 3D view. Heights are above `level`, so the camera moves with
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ViewCamera {
    pub level: ElementId,
    pub eye: Pt,
    pub eye_height: f64,
    pub target: Pt,
    pub target_height: f64,
    /// Vertical field of view, degrees.
    pub fov: f64,
}

/// A camera in model space (mm, z up), for the 3D view and the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CameraPose {
    pub eye: [f64; 3],
    pub target: [f64; 3],
    pub fov: f64,
}

/// The camera's eye and target in model space.
pub fn pose(doc: &Document, cam: &ViewCamera) -> CameraPose {
    let z = doc.level_elevation(cam.level).unwrap_or(0.0);
    CameraPose {
        eye: [cam.eye.x, cam.eye.y, z + cam.eye_height],
        target: [cam.target.x, cam.target.y, z + cam.target_height],
        fov: cam.fov,
    }
}

/// The camera of a view, if it's a camera view.
pub fn camera_of(doc: &Document, view: ElementId) -> Option<ViewCamera> {
    match doc.data(view).ok()? {
        ElementData::View { camera, .. } => *camera,
        _ => None,
    }
}

/// "3D View 1", "3D View 2"…: the next free camera view name, as Revit numbers them.
fn next_name(doc: &Document) -> String {
    let taken: Vec<String> = doc.of(Category::View).map(|e| e.data.name()).collect();
    (1..)
        .map(|n| format!("3D View {n}"))
        .find(|n| !taken.contains(n))
        .unwrap_or_default()
}

fn check_apart(eye: Pt, target: Pt) -> CoreResult<()> {
    if eye.dist(target) < 300.0 {
        return Err(CoreError::Invalid(
            "place the target at least 1' from the eye".into(),
        ));
    }
    Ok(())
}

/// Camera tool: a new perspective view looking from `eye` toward `target` on `level`, both
/// `height` above it.
pub fn create_camera(
    doc: &mut Document,
    level: ElementId,
    eye: Pt,
    target: Pt,
    height: f64,
) -> CoreResult<ElementId> {
    doc.level_elevation(level)?;
    check_apart(eye, target)?;
    let name = next_name(doc);
    doc.transact("Create camera", |tx| {
        let mut v = ElementData::view(name, ViewKind::ThreeD, 96);
        if let ElementData::View { camera, .. } = &mut v {
            *camera = Some(ViewCamera {
                level,
                eye,
                eye_height: height,
                target,
                target_height: height,
                fov: DEFAULT_FOV,
            });
        }
        Ok(tx.insert(v))
    })
}

/// Moves a camera to a pose in model space (after orbiting its view).
pub fn set_pose(doc: &mut Document, view: ElementId, p: &CameraPose) -> CoreResult<()> {
    let cam =
        camera_of(doc, view).ok_or_else(|| CoreError::Invalid("that view has no camera".into()))?;
    let z = doc.level_elevation(cam.level)?;
    let (eye, target) = (
        Pt::new(p.eye[0], p.eye[1]),
        Pt::new(p.target[0], p.target[1]),
    );
    let d = ((p.eye[0] - p.target[0]).powi(2)
        + (p.eye[1] - p.target[1]).powi(2)
        + (p.eye[2] - p.target[2]).powi(2))
    .sqrt();
    if d < 300.0 {
        return Err(CoreError::Invalid(
            "the target must be at least 1' from the eye".into(),
        ));
    }
    let next = ViewCamera {
        eye,
        eye_height: p.eye[2] - z,
        target,
        target_height: p.target[2] - z,
        fov: p.fov.clamp(10.0, 120.0),
        ..cam
    };
    doc.transact("Move camera", |tx| {
        tx.modify(view, |d| {
            if let ElementData::View { camera, .. } = d {
                *camera = Some(next);
            }
        })
    })
}

/// Drags the camera's eye or target in a plan (its grips), keeping the heights.
pub fn drag(doc: &mut Document, view: ElementId, eye_end: bool, to: Pt) -> CoreResult<()> {
    let cam =
        camera_of(doc, view).ok_or_else(|| CoreError::Invalid("that view has no camera".into()))?;
    let next = if eye_end {
        ViewCamera { eye: to, ..cam }
    } else {
        ViewCamera { target: to, ..cam }
    };
    check_apart(next.eye, next.target)?;
    doc.transact(
        if eye_end {
            "Move camera eye"
        } else {
            "Move camera target"
        },
        |tx| {
            tx.modify(view, |d| {
                if let ElementData::View { camera, .. } = d {
                    *camera = Some(next);
                }
            })
        },
    )
}

/// The plan glyph of a camera: the view cone's two edges out to the target distance, as
/// (eye, left end, right end), for the width of the view at `aspect` (width / height).
pub fn cone(cam: &ViewCamera, aspect: f64) -> (Pt, Pt, Pt) {
    let d = cam.target.sub(cam.eye);
    let len = d.len();
    let half = ((cam.fov.to_radians() / 2.0).tan() * aspect).atan();
    let dir = d.norm();
    let rot = |a: f64| {
        let (s, c) = a.sin_cos();
        Pt::new(dir.x * c - dir.y * s, dir.x * s + dir.y * c)
    };
    let reach = len / half.cos();
    (
        cam.eye,
        cam.eye.add(rot(half).scale(reach)),
        cam.eye.add(rot(-half).scale(reach)),
    )
}

/// Where the sun is.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SunPosition {
    /// Unit vector toward the sun in model space (x east of project north's right, z up).
    pub dir: [f64; 3],
    /// Degrees above the horizon.
    pub altitude: f64,
    /// Degrees clockwise from true north.
    pub azimuth: f64,
}

fn day_of_year(month: u32, day: u32) -> f64 {
    const START: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    f64::from(START[(month.clamp(1, 12) - 1) as usize] + day.clamp(1, 31))
}

/// The sun at `lat`/`lon` (degrees) on a month and day at `hour` of local standard time
/// (the time zone taken from the longitude). NOAA's solar position approximation, good to
/// a fraction of a degree. `north` is the project's Angle to True North (radians).
pub fn sun_position(
    lat: f64,
    lon: f64,
    north: f64,
    month: u32,
    day: u32,
    hour: f64,
) -> SunPosition {
    use std::f64::consts::PI;
    let tz = (lon / 15.0).round();
    let g = 2.0 * PI / 365.0 * (day_of_year(month, day) - 1.0 + (hour - 12.0) / 24.0);
    let eqtime = 229.18
        * (0.000_075 + 0.001_868 * g.cos()
            - 0.032_077 * g.sin()
            - 0.014_615 * (2.0 * g).cos()
            - 0.040_849 * (2.0 * g).sin());
    let decl = 0.006_918 - 0.399_912 * g.cos() + 0.070_257 * g.sin() - 0.006_758 * (2.0 * g).cos()
        + 0.000_907 * (2.0 * g).sin()
        - 0.002_697 * (3.0 * g).cos()
        + 0.001_48 * (3.0 * g).sin();
    let true_minutes = hour * 60.0 + eqtime + 4.0 * lon - 60.0 * tz;
    let ha = (true_minutes / 4.0 - 180.0).to_radians();
    let phi = lat.to_radians();
    let cos_zen = (phi.sin() * decl.sin() + phi.cos() * decl.cos() * ha.cos()).clamp(-1.0, 1.0);
    let zen = cos_zen.acos();
    let altitude = 90.0 - zen.to_degrees();
    // Azimuth clockwise from north.
    let az = {
        let y = -ha.sin();
        let x = decl.tan() * phi.cos() - phi.sin() * ha.cos();
        let a = y.atan2(x);
        (a.to_degrees() + 360.0) % 360.0
    };
    // East/north/up, then turned into the project by the Angle to True North.
    let (sa, ca) = az.to_radians().sin_cos();
    let horiz = zen.sin();
    let (e, n) = (horiz * sa, horiz * ca);
    let (s, c) = north.sin_cos();
    SunPosition {
        dir: [e * c - n * s, e * s + n * c, cos_zen],
        altitude,
        azimuth: az,
    }
}

/// The sun for this project: at the site's location, or central USA without one.
pub fn project_sun(doc: &Document, month: u32, day: u32, hour: f64) -> SunPosition {
    match crate::site::site_of(doc).and_then(|id| doc.data(id).ok()) {
        Some(ElementData::Site {
            lat, lon, rotation, ..
        }) => sun_position(*lat, *lon, *rotation, month, day, hour),
        _ => sun_position(39.8, -98.6, 0.0, month, day, hour),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    #[test]
    fn camera_tool_makes_numbered_perspective_views() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let levels = doc.levels();
        let (l2, z2) = (levels[1].0, levels[1].2);
        let v = create_camera(
            &mut doc,
            l2,
            Pt::new(0.0, 0.0),
            Pt::new(10_000.0, 0.0),
            DEFAULT_EYE_HEIGHT,
        )
        .unwrap();
        assert_eq!(doc.data(v).unwrap().name(), "3D View 1");
        let cam = camera_of(&doc, v).unwrap();
        let p = pose(&doc, &cam);
        assert_eq!(p.eye, [0.0, 0.0, z2 + 1676.4]);
        assert_eq!(p.target, [10_000.0, 0.0, z2 + 1676.4]);
        let v2 = create_camera(
            &mut doc,
            l2,
            Pt::new(0.0, 0.0),
            Pt::new(0.0, 5000.0),
            1500.0,
        )
        .unwrap();
        assert_eq!(doc.data(v2).unwrap().name(), "3D View 2");
        assert!(
            create_camera(&mut doc, l2, Pt::new(0.0, 0.0), Pt::new(100.0, 0.0), 1500.0).is_err()
        );

        // Orbiting saves the pose; heights stay relative to the level.
        set_pose(
            &mut doc,
            v,
            &CameraPose {
                eye: [1000.0, 2000.0, z2 + 3000.0],
                target: [9000.0, 0.0, z2 + 1000.0],
                fov: 40.0,
            },
        )
        .unwrap();
        let cam = camera_of(&doc, v).unwrap();
        assert_eq!(cam.eye, Pt::new(1000.0, 2000.0));
        assert!(
            (cam.eye_height - 3000.0).abs() < 1e-9 && (cam.target_height - 1000.0).abs() < 1e-9
        );
        assert_eq!(cam.fov, 40.0);
        doc.undo().unwrap();
        assert_eq!(camera_of(&doc, v).unwrap().eye, Pt::new(0.0, 0.0));

        // Plan grips move the eye or target.
        drag(&mut doc, v, false, Pt::new(0.0, 8000.0)).unwrap();
        assert_eq!(camera_of(&doc, v).unwrap().target, Pt::new(0.0, 8000.0));
        assert!(drag(&mut doc, v, true, Pt::new(0.0, 7900.0)).is_err());
    }

    #[test]
    fn the_view_cone_spreads_by_the_field_of_view() {
        let cam = ViewCamera {
            level: ElementId::default(),
            eye: Pt::new(0.0, 0.0),
            eye_height: 0.0,
            target: Pt::new(10_000.0, 0.0),
            target_height: 0.0,
            fov: 90.0,
        };
        // Square view, 90°: the edges reach the target line 10 m either side.
        let (e, l, r) = cone(&cam, 1.0);
        assert_eq!(e, cam.eye);
        assert!((l.x - 10_000.0).abs() < 1e-6 && (l.y - 10_000.0).abs() < 1e-6);
        assert!((r.y + 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn sun_positions_match_known_values() {
        // Summer solstice, solar noon at 40° N on its time zone meridian (105° W): the sun
        // is due south, 90 - 40 + 23.44 = 73.4° up.
        let noon = 12.0 + 0.03; // equation of time on June 21 is about -1.8 min
        let s = sun_position(40.0, -105.0, 0.0, 6, 21, noon);
        assert!((s.altitude - 73.4).abs() < 0.5, "{}", s.altitude);
        assert!((s.azimuth - 180.0).abs() < 3.0, "{}", s.azimuth);
        assert!(s.dir[1] < 0.0 && s.dir[2] > 0.9);
        // Winter solstice noon: 90 - 40 - 23.44 = 26.6°.
        let w = sun_position(40.0, -105.0, 0.0, 12, 21, 12.0);
        assert!((w.altitude - 26.6).abs() < 0.6, "{}", w.altitude);
        // Mornings are in the east, afternoons in the west.
        assert!(sun_position(40.0, -105.0, 0.0, 6, 21, 8.0).dir[0] > 0.5);
        assert!(sun_position(40.0, -105.0, 0.0, 6, 21, 16.0).dir[0] < -0.5);
        // Night: below the horizon.
        assert!(sun_position(40.0, -105.0, 0.0, 6, 21, 0.5).altitude < 0.0);
        // Turning project north 90° turns the sun with it.
        let t = sun_position(40.0, -105.0, std::f64::consts::FRAC_PI_2, 6, 21, 8.0);
        let d = sun_position(40.0, -105.0, 0.0, 6, 21, 8.0).dir;
        assert!((t.dir[0] + d[1]).abs() < 1e-9 && (t.dir[1] - d[0]).abs() < 1e-9);
    }
}
