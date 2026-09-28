//! Wall openings (ADR-058): Revit's Wall Opening, a hole of any sketched shape cut through
//! a wall.
//! - The sketch is drawn on the wall's face, in an elevation or section facing the wall or
//!   on the face picked in 3D.
//! - It's kept in the wall's own frame: `u` along the location line from the wall's start
//!   and `z` up from its base. So the opening moves and stretches with its wall, and goes
//!   when the wall is deleted.
//! - Each loop is a hole; loops can't nest.

use serde::Serialize;
use studio_geom::{point_in_ring, signed_area, Pt};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::ops::{ro, Property};
use crate::sketch::{self, SketchCurve, SketchError};
use crate::units::format_ft_in;

/// A wall face as a sketch's work plane (mm): the location line's start and direction,
/// the face's side and distance from the centerline, and the wall's base and length.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct WallFrame {
    pub wall: ElementId,
    pub start: Pt,
    pub dir: Pt,
    /// The face's outward normal in plan (the wall's left, `dir.perp()`, or its right).
    pub normal: Pt,
    /// The face's distance from the centerline (half the thickness).
    pub half: f64,
    pub base_z: f64,
    pub length: f64,
    pub height: f64,
}

impl WallFrame {
    /// A point of the face: `u` along the wall, `z` up from its base (absolute mm, z up).
    pub fn world(&self, u: f64, z: f64) -> [f64; 3] {
        let p = self
            .start
            .add(self.dir.scale(u))
            .add(self.normal.scale(self.half));
        [p.x, p.y, self.base_z + z]
    }
    /// `u` of a plan point.
    pub fn u_of(&self, p: Pt) -> f64 {
        p.sub(self.start).dot(self.dir)
    }
}

/// The frame of `wall`'s face on the side of `toward` (a plan direction from the wall
/// toward whoever is sketching).
pub fn frame(doc: &Document, wall: ElementId, toward: Pt) -> CoreResult<WallFrame> {
    let data = doc.data(wall)?;
    let ElementData::Wall {
        type_id,
        start,
        end,
        base_level,
        base_offset,
        ..
    } = data
    else {
        return Err(CoreError::Invalid(
            "pick a wall to cut the opening in".into(),
        ));
    };
    let length = start.dist(*end);
    if length < 1.0 {
        return Err(CoreError::Invalid("that wall has no length".into()));
    }
    let dir = end.sub(*start).norm();
    let left = dir.perp();
    let normal = if toward.dot(left) >= 0.0 {
        left
    } else {
        left.scale(-1.0)
    };
    let half = match doc.data(*type_id)? {
        ElementData::WallType { thickness, .. } => thickness / 2.0,
        _ => 0.0,
    };
    let get = |id| doc.data(id).ok();
    let height = crate::hosting::wall_height(&get, data).unwrap_or(0.0);
    Ok(WallFrame {
        wall,
        start: *start,
        dir,
        normal,
        half,
        base_z: doc.level_elevation(*base_level)? + base_offset,
        length,
        height,
    })
}

/// A loop as a polygon (its curves' points in order).
fn ring(l: &[SketchCurve]) -> Vec<Pt> {
    let mut out: Vec<Pt> = vec![];
    for c in l {
        let pts = c.points();
        for p in &pts[..pts.len() - 1] {
            if out.last().is_none_or(|q| q.dist(*p) > 1e-6) {
                out.push(*p);
            }
        }
    }
    out
}

/// The opening's holes in its wall's frame (`u`, `z` from the base), counter-clockwise.
pub fn rings(sketch: &[Vec<SketchCurve>]) -> Vec<Vec<Pt>> {
    sketch
        .iter()
        .map(|l| {
            let mut r = ring(l);
            if signed_area(&r) < 0.0 {
                r.reverse();
            }
            r
        })
        .filter(|r| r.len() >= 3)
        .collect()
}

/// Each wall opening cut in `wall`, with its holes.
pub fn openings_in(doc: &Document, wall: ElementId) -> Vec<(ElementId, Vec<Vec<Pt>>)> {
    doc.of(Category::WallOpening)
        .filter_map(|e| match &e.data {
            ElementData::WallOpening { host, sketch } if *host == wall => {
                Some((e.id, rings(sketch)))
            }
            _ => None,
        })
        .collect()
}

/// Finish (Revit's ✓): checks the sketch (closed loops that don't cross or nest, and on
/// the wall) and creates the opening, or changes `target`'s sketch; one undo step.
/// `curves` are in the wall's frame.
pub fn finish(
    doc: &mut Document,
    target: Option<ElementId>,
    wall: ElementId,
    curves: &[SketchCurve],
) -> Result<ElementId, SketchError> {
    let fail = |m: &str| SketchError {
        message: m.into(),
        bad: vec![],
    };
    let loops = sketch::loops(curves)?;
    let rs = rings(&loops);
    for (i, a) in rs.iter().enumerate() {
        for (j, b) in rs.iter().enumerate() {
            if i != j && point_in_ring(a[0], b) {
                return Err(fail(
                    "A wall opening's loops can't be inside one another. Delete the inner loop, or draw it as its own opening.",
                ));
            }
        }
    }
    let f = frame(doc, wall, Pt::new(0.0, 1.0)).map_err(|e| fail(&e.to_string()))?;
    let on_wall = rs.iter().any(|r| {
        let (lo, hi) = bounds(r);
        lo.x < f.length && hi.x > 0.0 && lo.y < f.height && hi.y > 0.0
    });
    if !on_wall {
        return Err(fail(
            "The opening doesn't cut the wall. Sketch it on the wall's face.",
        ));
    }
    let label = if target.is_some() {
        "Edit wall opening"
    } else {
        "Create wall opening"
    };
    doc.transact(label, |tx| {
        if let Some(id) = target {
            tx.modify(id, |d| {
                if let ElementData::WallOpening { sketch, .. } = d {
                    *sketch = loops.clone();
                }
            })?;
            return Ok(id);
        }
        Ok(tx.insert(ElementData::WallOpening {
            host: wall,
            sketch: loops.clone(),
        }))
    })
    .map_err(|e| fail(&e.to_string()))
}

fn bounds(r: &[Pt]) -> (Pt, Pt) {
    let mut lo = Pt::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in r {
        lo = Pt::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Pt::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

/// The curves to edit for an existing opening, and its wall.
pub fn curves_of(doc: &Document, id: ElementId) -> CoreResult<(ElementId, Vec<SketchCurve>)> {
    match doc.data(id)? {
        ElementData::WallOpening { host, sketch } => {
            Ok((*host, sketch.iter().flatten().cloned().collect()))
        }
        _ => Err(CoreError::Invalid("select a wall opening".into())),
    }
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(ElementData::WallOpening { host, sketch }) = doc.data(id) else {
        return;
    };
    let rs = rings(sketch);
    let (lo, hi) = rs.iter().fold(
        (
            Pt::new(f64::INFINITY, f64::INFINITY),
            Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), r| {
            let (a, b) = bounds(r);
            (
                Pt::new(lo.x.min(a.x), lo.y.min(a.y)),
                Pt::new(hi.x.max(b.x), hi.y.max(b.y)),
            )
        },
    );
    let host_name = doc
        .data(*host)
        .ok()
        .and_then(|w| w.type_id())
        .and_then(|t| doc.data(t).ok())
        .map(|t| format!("Wall: {}", t.name()))
        .unwrap_or_else(|| "Wall".into());
    const G: &str = "Dimensions";
    props.push(ro("host", "Host", "Constraints", host_name));
    if rs.is_empty() {
        return;
    }
    props.push(ro("width", "Width", G, format_ft_in(hi.x - lo.x)));
    props.push(ro("height", "Height", G, format_ft_in(hi.y - lo.y)));
    props.push(ro(
        "sill",
        "Base Offset (bottom)",
        "Constraints",
        format_ft_in(lo.y),
    ));
    let area: f64 = rs.iter().map(|r| signed_area(r).abs()).sum();
    props.push(ro(
        "area",
        "Area",
        G,
        format!("{:.2} SF", area / (304.8 * 304.8)),
    ));
    props.push(ro("loops", "Holes", G, rs.len().to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;
    use crate::sketch::{draw, DrawOptions, DrawTool};

    fn dr(tool: DrawTool, pts: &[Pt], o: &DrawOptions) -> Vec<SketchCurve> {
        draw(tool, pts, o).unwrap()
    }

    fn wall() -> (Document, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let w =
            ops::create_wall(&mut doc, wt, l1, Pt::new(0.0, 0.0), Pt::new(6000.0, 0.0)).unwrap();
        (doc, w)
    }

    #[test]
    fn a_wall_face_frame_maps_the_sketch_onto_the_wall() {
        let (doc, w) = wall();
        // Seen from the south (−y), the face is the wall's right side.
        let f = frame(&doc, w, Pt::new(0.0, -1.0)).unwrap();
        assert_eq!(f.normal, Pt::new(0.0, -1.0));
        let p = f.world(1000.0, 500.0);
        assert!(
            (p[0] - 1000.0).abs() < 1e-9
                && (p[1] + f.half).abs() < 1e-9
                && (p[2] - 500.0).abs() < 1e-9
        );
        assert!((f.length - 6000.0).abs() < 1e-9 && f.height > 2000.0);
        assert!((f.u_of(Pt::new(2500.0, 300.0)) - 2500.0).abs() < 1e-9);
    }

    #[test]
    fn a_circle_and_a_rectangle_finish_as_one_opening_with_two_holes() {
        let (mut doc, w) = wall();
        let mut curves = dr(
            DrawTool::Circle,
            &[Pt::new(1500.0, 1200.0), Pt::new(1900.0, 1200.0)],
            &DrawOptions::default(),
        );
        curves.extend(dr(
            DrawTool::Rectangle,
            &[Pt::new(3000.0, 300.0), Pt::new(4000.0, 2000.0)],
            &DrawOptions::default(),
        ));
        let id = finish(&mut doc, None, w, &curves).unwrap();
        let holes = openings_in(&doc, w);
        assert_eq!(holes.len(), 1);
        assert_eq!(holes[0].0, id);
        assert_eq!(holes[0].1.len(), 2);
        for r in &holes[0].1 {
            assert!(signed_area(r) > 0.0, "counter-clockwise");
        }
        let circle = holes[0].1.iter().find(|r| r.len() > 4).unwrap();
        let a = signed_area(circle);
        assert!((a - std::f64::consts::PI * 400.0 * 400.0).abs() / a < 0.01);
        let sheet = ops::properties(&doc, id).unwrap();
        let get = |k: &str| {
            sheet
                .properties
                .iter()
                .find(|p| p.key == k)
                .unwrap()
                .value
                .clone()
        };
        assert_eq!(get("loops"), "2");
        // Deleting the wall takes its opening.
        ops::delete(&mut doc, &[w]).unwrap();
        assert!(doc.data(id).is_err());
    }

    #[test]
    fn finish_refuses_nested_open_and_off_wall_sketches() {
        let (mut doc, w) = wall();
        let mut nested = dr(
            DrawTool::Rectangle,
            &[Pt::new(1000.0, 500.0), Pt::new(3000.0, 2000.0)],
            &DrawOptions::default(),
        );
        nested.extend(dr(
            DrawTool::Rectangle,
            &[Pt::new(1500.0, 800.0), Pt::new(2000.0, 1500.0)],
            &DrawOptions::default(),
        ));
        assert!(finish(&mut doc, None, w, &nested)
            .unwrap_err()
            .message
            .contains("inside one another"));
        let open = vec![SketchCurve::line(
            Pt::new(0.0, 0.0),
            Pt::new(1000.0, 1000.0),
        )];
        assert!(finish(&mut doc, None, w, &open)
            .unwrap_err()
            .message
            .contains("closed loops"));
        let off = dr(
            DrawTool::Rectangle,
            &[Pt::new(9000.0, 500.0), Pt::new(9500.0, 1000.0)],
            &DrawOptions::default(),
        );
        assert!(finish(&mut doc, None, w, &off)
            .unwrap_err()
            .message
            .contains("doesn't cut the wall"));
        assert_eq!(doc.count(Category::WallOpening), 0);
    }

    #[test]
    fn editing_an_opening_changes_its_sketch_in_one_step() {
        let (mut doc, w) = wall();
        let sq = dr(
            DrawTool::Rectangle,
            &[Pt::new(1000.0, 500.0), Pt::new(2000.0, 1500.0)],
            &DrawOptions::default(),
        );
        let id = finish(&mut doc, None, w, &sq).unwrap();
        let (host, curves) = curves_of(&doc, id).unwrap();
        assert_eq!((host, curves.len()), (w, 4));
        let tri = dr(
            DrawTool::InscribedPolygon,
            &[Pt::new(1500.0, 1000.0), Pt::new(1500.0, 1600.0)],
            &DrawOptions {
                sides: 3,
                ..DrawOptions::default()
            },
        );
        assert_eq!(finish(&mut doc, Some(id), w, &tri).unwrap(), id);
        assert_eq!(openings_in(&doc, w)[0].1[0].len(), 3);
        doc.undo().unwrap();
        assert_eq!(openings_in(&doc, w)[0].1[0].len(), 4);
    }
}
