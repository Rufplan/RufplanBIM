//! 3D thumbnails of door and window types (ADR-033): the type in a short piece of wall, as
//! triangles for the type picker to render. The wall runs along +x, faces +y (the side the
//! thumbnail camera looks from), and its base is at z = 0.

use serde::Serialize;
use studio_core::{DoorFamily, ElementId};
use studio_geom::{Poly, Prism, Pt};
use studio_regen::{OpeningKind, OpeningSolid};
use ts_rs::TS;

const IN: f64 = 25.4;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct OpeningThumb {
    /// Frame, casing, sashes and leaves (9 floats a triangle, mm, z-up).
    pub frame: Vec<f32>,
    pub glass: Vec<f32>,
    pub wall: Vec<f32>,
    /// The finish colour.
    pub color: [u8; 3],
}

fn block(x0: f64, x1: f64, y0: f64, y1: f64, z0: f64, z1: f64) -> Vec<f32> {
    if x1 - x0 < 0.1 || z1 - z0 < 0.1 {
        return vec![];
    }
    Prism {
        base: Poly::simple(vec![
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ]),
        z0,
        z1,
    }
    .triangles()
}

pub fn thumb(kind: OpeningKind, width: f64, height: f64) -> OpeningThumb {
    let h = 3.0 * IN;
    let sill = match kind {
        OpeningKind::Window(_) => 24.0 * IN,
        OpeningKind::Door(_) => 0.0,
    };
    let o = OpeningSolid {
        id: ElementId::new(),
        host: ElementId::new(),
        kind,
        wall_start: Pt::new(0.0, 0.0),
        dir: Pt::new(1.0, 0.0),
        half_thickness: h,
        t0: 0.0,
        t1: width,
        z0: sill,
        z1: sill + height,
        flip_hand: false,
        flip_facing: false,
    };
    let (parts, color) = match kind {
        OpeningKind::Window(s) => (super::windows::parts(&o, s), s.finish.color()),
        OpeningKind::Door(s) => (super::doors::parts(&o, s), s.finish.color()),
    };
    // A foot and a half of wall each side (a barn door's track needs its leaf's width), and
    // a foot above.
    let side = 18.0 * IN;
    let right = match kind {
        OpeningKind::Door(s) if s.family == DoorFamily::Barn => width + 12.0 * IN,
        _ => side,
    };
    let top = sill + height + 12.0 * IN;
    let mut wall = block(-side, 0.0, -h, h, 0.0, top);
    wall.extend(block(width, width + right, -h, h, 0.0, top));
    wall.extend(block(0.0, width, -h, h, sill + height, top));
    wall.extend(block(0.0, width, -h, h, 0.0, sill));
    OpeningThumb {
        frame: parts.frame,
        glass: parts.glass,
        wall,
        color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::doors::DoorStyle;
    use studio_core::windows::WindowStyle;
    use studio_core::WindowFamily;

    #[test]
    fn thumbs_frame_the_type_in_a_wall() {
        let d = thumb(
            OpeningKind::Door(DoorStyle::new(DoorFamily::SlidingGlass)),
            1829.0,
            2032.0,
        );
        assert!(!d.frame.is_empty() && !d.glass.is_empty() && !d.wall.is_empty());
        // Two jambs, a head: no wall below a door.
        assert_eq!(d.wall.len() / 9, 36);
        let w = thumb(
            OpeningKind::Window(WindowStyle::new(WindowFamily::DoubleHung)),
            914.0,
            1524.0,
        );
        assert_eq!(w.wall.len() / 9, 48, "and a sill wall under a window");
    }
}
