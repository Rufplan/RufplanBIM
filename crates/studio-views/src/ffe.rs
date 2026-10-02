//! Furniture and equipment in views (ADR-090): each archetype as parts (boxes, cylinders,
//! round fronts) in the piece's own frame, drawn as plan outlines, 3D meshes in its
//! finishes, and picker thumbnails.

use serde::Serialize;
use studio_core::ffe::{FfeKind, FfeSpec};
use studio_core::{Category, Document, ElementData, ElementId};
use studio_geom::Pt;
use ts_rs::TS;

use crate::{ring, Builder, Dash, FillKind, Mesh};

const IN: f64 = 25.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Seat and back cushions (`FfeSpec::cushion`).
    Cushion,
    Main,
    Accent,
    Dark,
    Metal,
    Glass,
    Light,
}

/// One part in the piece's frame: x across its width (centered), y front (-) to back
/// (+), z up from its base.
#[derive(Debug, Clone)]
enum Part {
    Box {
        x0: f64,
        y0: f64,
        z0: f64,
        x1: f64,
        y1: f64,
        z1: f64,
        tone: Tone,
        plan: bool,
    },
    Cyl {
        cx: f64,
        cy: f64,
        r: f64,
        z0: f64,
        z1: f64,
        tone: Tone,
        plan: bool,
    },
    /// Free-form triangles in the piece's own frame (x, y, z per vertex), for shapes boxes
    /// and cylinders can't make: an umbrella's canopy (ADR-101). 3D only.
    Tris { tris: Vec<[f64; 3]>, tone: Tone },
    /// A disc on the front face (a washer's door), centered at (cx, cz), from y0 to y1.
    Front {
        cx: f64,
        cz: f64,
        r: f64,
        y0: f64,
        y1: f64,
        tone: Tone,
    },
}

struct P {
    parts: Vec<Part>,
}

impl P {
    #[allow(clippy::too_many_arguments)]
    fn b(&mut self, x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64, tone: Tone, plan: bool) {
        self.parts.push(Part::Box {
            x0: x0.min(x1),
            y0: y0.min(y1),
            z0: z0.min(z1),
            x1: x0.max(x1),
            y1: y0.max(y1),
            z1: z0.max(z1),
            tone,
            plan,
        });
    }
    #[allow(clippy::too_many_arguments)]
    fn c(&mut self, cx: f64, cy: f64, r: f64, z0: f64, z1: f64, tone: Tone, plan: bool) {
        self.parts.push(Part::Cyl {
            cx,
            cy,
            r,
            z0,
            z1,
            tone,
            plan,
        });
    }
    fn front(&mut self, cx: f64, cz: f64, r: f64, y0: f64, y1: f64, tone: Tone) {
        self.parts.push(Part::Front {
            cx,
            cz,
            r,
            y0,
            y1,
            tone,
        });
    }
    /// Four legs inset `i` from the corners of w x d, `t` square, `h` tall.
    fn legs(&mut self, w: f64, d: f64, i: f64, t: f64, h: f64, tone: Tone) {
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let (x, y) = (sx * (w / 2.0 - i - t / 2.0), sy * (d / 2.0 - i - t / 2.0));
            self.b(
                x - t / 2.0,
                y - t / 2.0,
                0.0,
                x + t / 2.0,
                y + t / 2.0,
                h,
                tone,
                false,
            );
        }
    }
}

/// The parts of a piece of `spec`'s size.
fn parts(s: &FfeSpec) -> Vec<Part> {
    use FfeKind::*;
    use Tone::*;
    let (w, d, h) = (s.width, s.depth, s.height);
    let (hw, hd) = (w / 2.0, d / 2.0);
    let n = s.count;
    let mut p = P { parts: vec![] };
    let seat_h = (h * 0.5).min(18.0 * IN);
    // Cushions in their own fabric when the piece has one.
    let cushion = if s.cushion.is_some() {
        Tone::Cushion
    } else {
        Tone::Main
    };
    match s.kind {
        Sofa | Armchair => {
            let arm = (w * 0.12).clamp(4.0 * IN, 7.0 * IN);
            let back = (d * 0.22).max(6.0 * IN);
            let base = seat_h - 5.0 * IN;
            p.b(-hw, -hd, 3.0 * IN, hw, hd, base, Main, true);
            p.b(-hw, hd - back, base, hw, hd, h, Main, true);
            for sx in [-1.0, 1.0] {
                p.b(
                    sx * hw,
                    -hd,
                    base,
                    sx * (hw - arm),
                    hd,
                    h * 0.68,
                    Main,
                    true,
                );
            }
            let seats = n.max(1) as f64;
            let cw = (w - 2.0 * arm) / seats;
            for i in 0..n.max(1) {
                let x0 = -hw + arm + cw * i as f64;
                p.b(
                    x0 + 0.3 * IN,
                    -hd + 0.5 * IN,
                    base,
                    x0 + cw - 0.3 * IN,
                    hd - back,
                    seat_h,
                    cushion,
                    true,
                );
                // A back cushion against the frame, where the seat has its own fabric.
                if cushion == Cushion {
                    p.b(
                        x0 + 0.3 * IN,
                        hd - back - 4.0 * IN,
                        seat_h,
                        x0 + cw - 0.3 * IN,
                        hd - back,
                        h - 2.0 * IN,
                        Cushion,
                        false,
                    );
                }
            }
            p.legs(w, d, 2.0 * IN, 1.5 * IN, 3.0 * IN, Dark);
        }
        Sectional => {
            // A sofa across the back, and a return (or chaise) down the right side.
            let sd = 38.0 * IN.min(d);
            let back = 8.0 * IN;
            let base = seat_h - 5.0 * IN;
            p.b(-hw, hd - sd, 3.0 * IN, hw, hd, base, Main, true);
            p.b(-hw, hd - back, base, hw, hd, h, Main, true);
            let rw = 36.0 * IN;
            p.b(hw - rw, -hd, 3.0 * IN, hw, hd - sd, base, Main, true);
            if n >= 3 {
                p.b(hw - back, -hd, base, hw, hd - back, h, Main, true);
            }
            p.b(-hw, hd - sd, base, -hw + 6.0 * IN, hd, h * 0.68, Main, true);
            let seats = 3.0;
            let cw = (w - 6.0 * IN - rw) / seats;
            for i in 0..3 {
                let x0 = -hw + 6.0 * IN + cw * i as f64;
                p.b(
                    x0 + 0.3 * IN,
                    hd - sd + 0.5 * IN,
                    base,
                    x0 + cw - 0.3 * IN,
                    hd - back,
                    seat_h,
                    Main,
                    true,
                );
            }
            p.b(
                hw - rw + 0.3 * IN,
                -hd + 0.5 * IN,
                base,
                hw - if n >= 3 { back } else { 0.5 * IN },
                hd - back,
                seat_h,
                Main,
                true,
            );
        }
        Chaise => {
            // A long seat with a back rising at one end (+x).
            let base = (h * 0.45).min(seat_h);
            p.b(-hw, -hd, 3.0 * IN, hw, hd, base, Main, true);
            p.b(
                hw - 22.0 * IN.min(w * 0.3),
                -hd,
                base,
                hw,
                hd,
                h,
                Main,
                true,
            );
            p.legs(w, d, 1.5 * IN, 1.5 * IN, 3.0 * IN, Accent);
        }
        Chair | OfficeChair => {
            let sh = 18.0 * IN;
            if s.kind == OfficeChair {
                p.c(0.0, 0.0, hw.min(hd) * 0.95, 0.0, 2.0 * IN, Dark, true);
                p.c(0.0, 0.0, 1.0 * IN, 2.0 * IN, sh - 2.0 * IN, Metal, false);
            } else {
                p.legs(w, d, 1.0 * IN, 1.25 * IN, sh - 2.0 * IN, Accent);
            }
            p.b(
                -hw + 1.0 * IN,
                -hd + 1.0 * IN,
                sh - 2.0 * IN,
                hw - 1.0 * IN,
                hd - 2.0 * IN,
                sh,
                Main,
                true,
            );
            p.b(
                -hw + 1.0 * IN,
                hd - 2.5 * IN,
                sh,
                hw - 1.0 * IN,
                hd - 1.0 * IN,
                h,
                Main,
                true,
            );
            if n >= 1 {
                for sx in [-1.0, 1.0] {
                    p.b(
                        sx * (hw - 1.0 * IN),
                        -hd + 3.0 * IN,
                        sh,
                        sx * (hw - 2.5 * IN),
                        hd - 2.5 * IN,
                        sh + 8.0 * IN,
                        Main,
                        true,
                    );
                }
            }
        }
        Stool => {
            let r = hw.min(hd);
            p.c(0.0, 0.0, r, h - 2.0 * IN, h, Main, true);
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let (x, y) = (sx * r * 0.6, sy * r * 0.6);
                p.b(
                    x - 0.5 * IN,
                    y - 0.5 * IN,
                    0.0,
                    x + 0.5 * IN,
                    y + 0.5 * IN,
                    h - 2.0 * IN,
                    Accent,
                    false,
                );
            }
            p.b(
                -r * 0.6,
                -r * 0.6,
                9.0 * IN,
                r * 0.6,
                r * 0.6,
                9.75 * IN,
                Accent,
                false,
            );
            if n >= 1 {
                p.b(
                    -r * 0.8,
                    r * 0.7,
                    h,
                    r * 0.8,
                    r * 0.9,
                    h + 12.0 * IN,
                    Main,
                    true,
                );
            }
        }
        Ottoman => {
            if n >= 1 {
                p.c(0.0, 0.0, hw.min(hd), 0.0, h, Main, true);
            } else {
                p.b(-hw, -hd, 3.0 * IN, hw, hd, h, Main, true);
                p.legs(w, d, 1.5 * IN, 1.5 * IN, 3.0 * IN, Accent);
            }
        }
        Bench => {
            if n >= 1 {
                p.b(-hw, -hd, 2.0 * IN, hw, hd, h, Accent, true);
                p.b(
                    -hw + 0.5 * IN,
                    -hd + 0.5 * IN,
                    h,
                    hw - 0.5 * IN,
                    hd - 0.5 * IN,
                    h + 2.0 * IN,
                    Main,
                    true,
                );
            } else {
                p.b(-hw, -hd, h - 1.5 * IN, hw, hd, h, Main, true);
                p.legs(w, d, 1.5 * IN, 1.75 * IN, h - 1.5 * IN, Accent);
            }
        }
        Booth => {
            p.b(-hw, -hd, 0.0, hw, hd, seat_h, Main, true);
            p.b(-hw, hd - 5.0 * IN, seat_h, hw, hd, h, Main, true);
            p.b(-hw, -hd, 0.0, hw, -hd + 1.0 * IN, 4.0 * IN, Accent, false);
        }
        Table | Desk | PoolTable => {
            let top = if s.kind == PoolTable {
                6.0 * IN
            } else {
                1.25 * IN
            };
            p.b(-hw, -hd, h - top, hw, hd, h, Main, true);
            if s.kind == PoolTable {
                p.b(
                    -hw + 5.0 * IN,
                    -hd + 5.0 * IN,
                    h - 0.5 * IN,
                    hw - 5.0 * IN,
                    hd - 5.0 * IN,
                    h + 0.25 * IN,
                    Accent,
                    true,
                );
                p.legs(w, d, 6.0 * IN, 6.0 * IN, h - top, Main);
            } else if s.kind == Desk && n >= 1 {
                // Drawer pedestals; a reception desk's high transaction top.
                let pw = 16.0 * IN.min(w * 0.3);
                for i in 0..n.min(2) {
                    let side = if i == 0 { 1.0 } else { -1.0 };
                    p.b(
                        side * hw,
                        -hd + 1.0 * IN,
                        0.0,
                        side * (hw - pw),
                        hd - 1.0 * IN,
                        h - top,
                        Accent,
                        false,
                    );
                }
                if n >= 3 {
                    p.b(-hw, -hd, 0.0, hw, -hd + 1.0 * IN, h, Main, true);
                    p.b(
                        -hw,
                        -hd - 4.0 * IN,
                        h,
                        hw,
                        -hd + 8.0 * IN,
                        h + 1.0 * IN,
                        Accent,
                        true,
                    );
                } else {
                    p.legs(w, d, 1.0 * IN, 1.5 * IN, h - top, Accent);
                }
            } else {
                p.legs(w, d, 2.0 * IN, 2.0 * IN, h - top, Accent);
            }
        }
        RoundTable => {
            let r = hw.min(hd);
            p.c(0.0, 0.0, r, h - 1.25 * IN, h, Main, true);
            p.c(0.0, 0.0, 2.0 * IN, 1.0 * IN, h - 1.25 * IN, Accent, false);
            p.c(0.0, 0.0, r * 0.45, 0.0, 1.0 * IN, Accent, false);
        }
        Bed => {
            let (frame_h, mat_top) = (10.0 * IN, 24.0 * IN);
            let head = 3.0 * IN;
            p.b(-hw, -hd, 0.0, hw, hd - head, frame_h, Accent, true);
            p.b(
                -hw + 1.0 * IN,
                -hd + 1.0 * IN,
                frame_h,
                hw - 1.0 * IN,
                hd - head - 0.5 * IN,
                mat_top,
                Light,
                true,
            );
            // The duvet folded back, and the pillows at the head.
            p.b(
                -hw + 0.5 * IN,
                -hd + 0.5 * IN,
                mat_top - 6.0 * IN,
                hw - 0.5 * IN,
                hd - head - 22.0 * IN,
                mat_top + 1.0 * IN,
                Main,
                true,
            );
            let pillows = n.max(1);
            let pw = (w - 6.0 * IN) / pillows as f64;
            for i in 0..pillows {
                let x0 = -hw + 3.0 * IN + pw * i as f64;
                p.b(
                    x0 + 0.5 * IN,
                    hd - head - 20.0 * IN,
                    mat_top,
                    x0 + pw - 0.5 * IN,
                    hd - head - 2.0 * IN,
                    mat_top + 5.0 * IN,
                    Light,
                    true,
                );
            }
            p.b(-hw, hd - head, 0.0, hw, hd, h, Accent, true);
        }
        BunkBed => {
            for z in [8.0 * IN, h - 16.0 * IN] {
                p.b(
                    -hw + 2.0 * IN,
                    -hd + 2.0 * IN,
                    z,
                    hw - 2.0 * IN,
                    hd - 2.0 * IN,
                    z + 2.0 * IN,
                    Main,
                    false,
                );
                p.b(
                    -hw + 2.0 * IN,
                    -hd + 2.0 * IN,
                    z + 2.0 * IN,
                    hw - 2.0 * IN,
                    hd - 2.0 * IN,
                    z + 8.0 * IN,
                    Accent,
                    true,
                );
            }
            p.legs(w, d, 0.0, 3.0 * IN, h, Main);
            p.b(
                -hw,
                hd - 3.0 * IN,
                h - 6.0 * IN,
                hw,
                hd,
                h - 2.0 * IN,
                Main,
                false,
            );
        }
        Crib => {
            p.legs(w, d, 0.0, 2.0 * IN, h, Main);
            p.b(-hw, -hd, 12.0 * IN, hw, hd, 14.0 * IN, Main, true);
            p.b(
                -hw + 1.0 * IN,
                -hd + 1.0 * IN,
                14.0 * IN,
                hw - 1.0 * IN,
                hd - 1.0 * IN,
                19.0 * IN,
                Light,
                true,
            );
            for (y0, y1) in [(-hd, -hd + 1.5 * IN), (hd - 1.5 * IN, hd)] {
                p.b(-hw, y0, h - 2.0 * IN, hw, y1, h, Main, true);
            }
            for (x0, x1) in [(-hw, -hw + 1.5 * IN), (hw - 1.5 * IN, hw)] {
                p.b(x0, -hd, h - 2.0 * IN, x1, hd, h, Main, true);
            }
        }
        Casegood | Cabinet | Wardrobe => {
            let foot = if s.kind == Wardrobe {
                3.0 * IN
            } else {
                4.0 * IN
            };
            p.b(-hw, -hd + 0.75 * IN, foot, hw, hd, h, Main, true);
            p.b(
                -hw + 1.0 * IN,
                -hd + 1.0 * IN,
                0.0,
                hw - 1.0 * IN,
                hd - 1.0 * IN,
                foot,
                Accent,
                false,
            );
            // Fronts: drawers in rows (two columns when wide), or doors.
            let body = h - foot;
            if s.kind == Casegood {
                let rows = n.max(1);
                let cols = if w > 40.0 * IN { 2 } else { 1 };
                let rh = body / rows as f64;
                let cw = w / cols as f64;
                for r in 0..rows {
                    for c in 0..cols {
                        let x0 = -hw + cw * c as f64;
                        let z0 = foot + rh * r as f64;
                        p.b(
                            x0 + 0.4 * IN,
                            -hd,
                            z0 + 0.4 * IN,
                            x0 + cw - 0.4 * IN,
                            -hd + 0.75 * IN,
                            z0 + rh - 0.4 * IN,
                            Main,
                            false,
                        );
                        p.b(
                            x0 + cw / 2.0 - 2.0 * IN,
                            -hd - 0.6 * IN,
                            z0 + rh / 2.0 - 0.3 * IN,
                            x0 + cw / 2.0 + 2.0 * IN,
                            -hd,
                            z0 + rh / 2.0 + 0.3 * IN,
                            Accent,
                            false,
                        );
                    }
                }
            } else {
                let doors = n.max(1);
                let dw = w / doors as f64;
                for i in 0..doors {
                    let x0 = -hw + dw * i as f64;
                    p.b(
                        x0 + 0.4 * IN,
                        -hd,
                        foot + 0.4 * IN,
                        x0 + dw - 0.4 * IN,
                        -hd + 0.75 * IN,
                        h - 0.4 * IN,
                        Main,
                        false,
                    );
                    let hx = if i % 2 == 0 {
                        x0 + dw - 2.0 * IN
                    } else {
                        x0 + 2.0 * IN
                    };
                    let hz = foot + body * if s.kind == Wardrobe { 0.45 } else { 0.7 };
                    p.b(
                        hx - 0.3 * IN,
                        -hd - 0.6 * IN,
                        hz - 3.0 * IN,
                        hx + 0.3 * IN,
                        -hd,
                        hz + 3.0 * IN,
                        Accent,
                        false,
                    );
                }
            }
        }
        Bookcase => {
            let t = 0.75 * IN;
            for sx in [-1.0, 1.0] {
                p.b(sx * hw, -hd, 0.0, sx * (hw - t), hd, h, Main, true);
            }
            p.b(-hw, hd - 0.25 * IN, 0.0, hw, hd, h, Main, true);
            p.b(-hw, -hd, h - t, hw, hd, h, Main, true);
            let shelves = n.max(1) + 1;
            for i in 0..shelves {
                let z = 2.0 * IN + (h - 2.0 * IN - t) * i as f64 / shelves as f64;
                p.b(-hw + t, -hd, z, hw - t, hd, z + t, Main, false);
            }
        }
        Rug => {
            if n >= 1 {
                p.c(0.0, 0.0, hw.min(hd), 0.0, h, Main, true);
                p.c(
                    0.0,
                    0.0,
                    hw.min(hd) - 6.0 * IN,
                    h,
                    h + 0.05 * IN,
                    Accent,
                    true,
                );
            } else {
                p.b(-hw, -hd, 0.0, hw, hd, h, Main, true);
                p.b(
                    -hw + 6.0 * IN,
                    -hd + 6.0 * IN,
                    h,
                    hw - 6.0 * IN,
                    hd - 6.0 * IN,
                    h + 0.05 * IN,
                    Accent,
                    true,
                );
            }
        }
        LuggageRack => {
            p.b(-hw, -hd, h - 1.0 * IN, hw, hd, h, Accent, true);
            p.legs(w, d, 1.0 * IN, 1.0 * IN, h - 1.0 * IN, Main);
        }
        Umbrella => {
            // A market umbrella (ADR-101): an eight-rib canopy domed from its vent to the rib
            // tips, each panel sagging a little between its ribs, a valance at the edge, a
            // finial, a hub and a pole on a weighted base.
            let r = hw.min(hd);
            let apex = h - 3.0 * IN;
            let tip = h - 26.0 * IN;
            p.c(0.0, 0.0, 11.0 * IN, 0.0, 2.5 * IN, Dark, false);
            p.c(0.0, 0.0, 0.85 * IN, 2.5 * IN, apex, Accent, false);
            p.c(
                0.0,
                0.0,
                1.6 * IN,
                tip - 14.0 * IN,
                tip - 9.0 * IN,
                Accent,
                false,
            );
            p.c(0.0, 0.0, 1.3 * IN, apex, apex + 3.5 * IN, Accent, false);
            p.parts.push(Part::Tris {
                tris: canopy(r, apex, tip),
                tone: Main,
            });
            // The plan symbol: the canopy's outline.
            p.c(0.0, 0.0, r, tip, tip, Main, true);
        }
        // ---------- Equipment ----------
        Refrigerator | WineCooler | IceMachine | Vending => {
            p.b(-hw, -hd + 1.0 * IN, 0.0, hw, hd, h, Main, true);
            let fz = 1.0 * IN;
            match (s.kind, n) {
                (Refrigerator, 2) => {
                    door(&mut p, -hw, hw, h * 0.7, h, hd, fz);
                    door(&mut p, -hw, hw, 0.0, h * 0.7, hd, fz);
                }
                (Refrigerator, 3) => {
                    door(&mut p, -hw, 0.0, h * 0.35, h, hd, fz);
                    door(&mut p, 0.0, hw, h * 0.35, h, hd, fz);
                    door(&mut p, -hw, hw, 0.0, h * 0.35, hd, fz);
                }
                (Refrigerator, 4) => {
                    door(&mut p, -hw, hw, h * 0.35, h, hd, fz);
                    door(&mut p, -hw, hw, 0.0, h * 0.35, hd, fz);
                }
                (Refrigerator, 5) => {
                    door(&mut p, -hw, -hw + w * 0.4, 0.0, h, hd, fz);
                    door(&mut p, -hw + w * 0.4, hw, 0.0, h, hd, fz);
                }
                (WineCooler, _) | (Vending, _) => {
                    let glass_w = if s.kind == Vending {
                        w * 0.68
                    } else {
                        w - 3.0 * IN
                    };
                    p.b(
                        -hw + 1.5 * IN,
                        -hd + 0.2 * IN,
                        3.0 * IN,
                        -hw + 1.5 * IN + glass_w,
                        -hd + 1.0 * IN,
                        h - 3.0 * IN,
                        Glass,
                        false,
                    );
                    if s.kind == Vending {
                        p.b(
                            -hw + glass_w + 3.0 * IN,
                            -hd + 0.5 * IN,
                            h * 0.45,
                            hw - 2.0 * IN,
                            -hd + 1.0 * IN,
                            h * 0.7,
                            Dark,
                            false,
                        );
                    }
                }
                (IceMachine, _) => {
                    if n >= 1 {
                        p.b(-hw, -hd, 0.0, hw, hd, h * 0.55, Metal, false);
                        p.b(
                            -hw + 3.0 * IN,
                            -hd - 0.5 * IN,
                            h * 0.3,
                            hw - 3.0 * IN,
                            -hd,
                            h * 0.5,
                            Dark,
                            false,
                        );
                    }
                    p.b(
                        -hw + 2.0 * IN,
                        -hd - 0.4 * IN,
                        h * 0.62,
                        hw - 2.0 * IN,
                        -hd,
                        h * 0.95,
                        Accent,
                        false,
                    );
                }
                _ => door(&mut p, -hw, hw, 0.0, h, hd, fz),
            }
        }
        Range | Cooktop => {
            let top = if s.kind == Range { 36.0 * IN } else { h };
            if s.kind == Range {
                p.b(-hw, -hd + 1.0 * IN, 0.0, hw, hd, top, Main, true);
                p.b(
                    -hw + 3.0 * IN,
                    -hd + 0.2 * IN,
                    8.0 * IN,
                    hw - 3.0 * IN,
                    -hd + 1.0 * IN,
                    26.0 * IN,
                    Glass,
                    false,
                );
                p.b(
                    -hw,
                    -hd,
                    27.0 * IN,
                    hw,
                    -hd + 1.0 * IN,
                    28.0 * IN,
                    Metal,
                    false,
                );
                p.b(-hw, hd - 3.0 * IN, top, hw, hd, h, Main, true);
            } else {
                p.b(-hw, -hd, 0.0, hw, hd, top, Main, true);
            }
            burners(
                &mut p,
                w,
                d - if s.kind == Range { 4.0 * IN } else { 0.0 },
                top,
                n,
            );
        }
        WallOven => {
            p.b(-hw, -hd + 1.0 * IN, 0.0, hw, hd, h, Main, true);
            let ovens = n.max(1);
            let oh = h / ovens as f64;
            for i in 0..ovens {
                let z0 = oh * i as f64;
                p.b(
                    -hw + 3.0 * IN,
                    -hd + 0.2 * IN,
                    z0 + 3.0 * IN,
                    hw - 3.0 * IN,
                    -hd + 1.0 * IN,
                    z0 + oh - 5.0 * IN,
                    Glass,
                    false,
                );
                p.b(
                    -hw + 2.0 * IN,
                    -hd - 0.8 * IN,
                    z0 + oh - 3.5 * IN,
                    hw - 2.0 * IN,
                    -hd,
                    z0 + oh - 2.5 * IN,
                    Metal,
                    false,
                );
            }
        }
        Microwave | OtrMicrowave | CounterAppliance | Kiosk => {
            p.b(-hw, -hd + 0.5 * IN, 0.0, hw, hd, h, Main, true);
            match s.kind {
                Kiosk => p.b(
                    -hw + 2.0 * IN,
                    -hd,
                    h * 0.55,
                    hw - 2.0 * IN,
                    -hd + 0.5 * IN,
                    h * 0.92,
                    Glass,
                    false,
                ),
                CounterAppliance => p.b(
                    -hw + 1.0 * IN,
                    -hd,
                    h * 0.2,
                    hw - 1.0 * IN,
                    -hd + 0.5 * IN,
                    h * 0.6,
                    Accent,
                    false,
                ),
                _ => p.b(
                    -hw + 1.5 * IN,
                    -hd,
                    1.5 * IN,
                    hw - w * 0.32,
                    -hd + 0.5 * IN,
                    h - 1.5 * IN,
                    Glass,
                    false,
                ),
            }
        }
        Hood => {
            let canopy = 6.0 * IN.min(h);
            p.b(-hw, -hd, 0.0, hw, hd, canopy, Main, true);
            if n >= 1 {
                p.b(
                    -w * 0.18,
                    hd - d * 0.5,
                    canopy,
                    w * 0.18,
                    hd,
                    h,
                    Main,
                    false,
                );
            }
        }
        Dishwasher => {
            p.b(-hw, -hd + 1.0 * IN, 0.0, hw, hd, h, Main, true);
            p.b(
                -hw,
                -hd,
                4.0 * IN,
                hw,
                -hd + 1.0 * IN,
                h - 4.0 * IN,
                Main,
                false,
            );
            p.b(
                -hw + 2.0 * IN,
                -hd - 0.8 * IN,
                h - 7.0 * IN,
                hw - 2.0 * IN,
                -hd,
                h - 6.0 * IN,
                Metal,
                false,
            );
            p.b(-hw, -hd, h - 4.0 * IN, hw, -hd + 1.0 * IN, h, Dark, false);
        }
        Washer | Dryer | StackedLaundry => {
            let units = if s.kind == StackedLaundry { 2 } else { 1 };
            let uh = h / units as f64;
            for i in 0..units {
                let z0 = uh * i as f64;
                p.b(
                    -hw,
                    -hd + 0.5 * IN,
                    z0,
                    hw,
                    hd,
                    z0 + uh - 0.25 * IN,
                    Main,
                    i == 0,
                );
                p.b(
                    -hw + 0.2 * IN,
                    -hd + 0.2 * IN,
                    z0 + uh - 6.0 * IN,
                    hw - 0.2 * IN,
                    -hd + 1.0 * IN,
                    z0 + uh - 0.5 * IN,
                    Accent,
                    false,
                );
                if s.kind == Washer && n >= 1 {
                    p.c(0.0, 0.0, w * 0.32, z0 + uh - 0.2 * IN, z0 + uh, Dark, true);
                } else {
                    p.front(
                        0.0,
                        z0 + (uh - 6.0 * IN) * 0.5,
                        (w * 0.32).min(uh * 0.36),
                        -hd,
                        -hd + 0.5 * IN,
                        if s.kind == Dryer || i == 1 {
                            Dark
                        } else {
                            Glass
                        },
                    );
                }
            }
        }
        WaterHeater => {
            let r = hw.min(hd);
            p.c(
                0.0,
                0.0,
                r,
                0.0,
                h - if n >= 1 { 14.0 * IN } else { 0.0 },
                Main,
                true,
            );
            if n >= 1 {
                p.c(0.0, 0.0, r * 0.9, h - 14.0 * IN, h, Accent, false);
            }
            for sx in [-0.4, 0.4] {
                p.c(
                    r * sx,
                    0.0,
                    0.5 * IN,
                    h - 1.0 * IN,
                    h + 6.0 * IN,
                    Metal,
                    false,
                );
            }
        }
        WallBox | MiniSplit | Ptac => {
            p.b(-hw, -hd, 0.0, hw, hd, h, Main, true);
            match s.kind {
                MiniSplit => p.b(
                    -hw + 1.0 * IN,
                    -hd - 0.2 * IN,
                    1.0 * IN,
                    hw - 1.0 * IN,
                    -hd + 0.5 * IN,
                    3.0 * IN,
                    Accent,
                    false,
                ),
                Ptac => p.b(
                    -hw + 2.0 * IN,
                    -hd - 0.2 * IN,
                    h - 4.0 * IN,
                    hw - 2.0 * IN,
                    hd - 2.0 * IN,
                    h + 0.2 * IN,
                    Accent,
                    false,
                ),
                _ => p.b(
                    -hw + 1.0 * IN,
                    -hd - 0.3 * IN,
                    h * 0.55,
                    hw - 1.0 * IN,
                    -hd,
                    h * 0.9,
                    Accent,
                    false,
                ),
            }
        }
        Furnace => {
            p.b(-hw, -hd, 0.0, hw, hd, h, Main, true);
            for i in 0..4 {
                let z = h * 0.15 + i as f64 * 1.5 * IN;
                p.b(
                    -hw + 3.0 * IN,
                    -hd - 0.2 * IN,
                    z,
                    hw - 3.0 * IN,
                    -hd,
                    z + 0.5 * IN,
                    Accent,
                    false,
                );
            }
        }
        Condenser => {
            p.b(-hw, -hd, 0.0, hw, hd, h - 1.0 * IN, Main, true);
            p.c(0.0, 0.0, hw.min(hd) * 0.85, h - 1.0 * IN, h, Dark, true);
        }
        Tv => {
            p.b(-hw, -hd, 0.0, hw, hd, h, Dark, true);
            p.b(
                -hw + 0.3 * IN,
                -hd - 0.1 * IN,
                0.4 * IN,
                hw - 0.3 * IN,
                -hd,
                h - 0.3 * IN,
                Glass,
                false,
            );
        }
        ChestFreezer => {
            p.b(-hw, -hd, 0.0, hw, hd, h - 2.0 * IN, Main, true);
            p.b(-hw, -hd, h - 2.0 * IN, hw, hd, h, Main, true);
            p.b(
                -4.0 * IN,
                -hd - 0.8 * IN,
                h - 4.0 * IN,
                4.0 * IN,
                -hd,
                h - 3.0 * IN,
                Accent,
                false,
            );
        }
        Cart => {
            p.b(-hw, -hd, 6.0 * IN, hw, hd, 8.0 * IN, Main, true);
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                p.c(
                    sx * (hw - 3.0 * IN),
                    sy * (hd - 3.0 * IN),
                    2.5 * IN,
                    0.0,
                    5.0 * IN,
                    Dark,
                    false,
                );
            }
            if n >= 1 {
                // Housekeeping: shelves and a linen bag.
                p.b(-hw, -hd, 8.0 * IN, hw * 0.3, hd, h, Accent, true);
                p.b(hw * 0.3, -hd, 8.0 * IN, hw, hd, h * 0.8, Light, true);
            } else {
                for sx in [-1.0, 1.0] {
                    p.c(
                        sx * (hw - 2.0 * IN),
                        0.0,
                        1.0 * IN,
                        8.0 * IN,
                        h,
                        Main,
                        false,
                    );
                }
                p.b(
                    -hw + 1.0 * IN,
                    -IN,
                    h - 2.0 * IN,
                    hw - 1.0 * IN,
                    1.0 * IN,
                    h,
                    Main,
                    false,
                );
            }
        }
        Lockers | Mailboxes => {
            p.b(-hw, -hd + 0.5 * IN, 0.0, hw, hd, h, Main, true);
            let cols = n.max(1);
            let rows = if s.kind == Mailboxes { 4 } else { 3 };
            let (cw, rh) = (w / cols as f64, h / rows as f64);
            for c in 0..cols {
                for r in 0..rows {
                    let (x0, z0) = (-hw + cw * c as f64, rh * r as f64);
                    p.b(
                        x0 + 0.3 * IN,
                        -hd,
                        z0 + 0.3 * IN,
                        x0 + cw - 0.3 * IN,
                        -hd + 0.5 * IN,
                        z0 + rh - 0.3 * IN,
                        Accent,
                        false,
                    );
                }
            }
        }
        Treadmill => {
            p.b(-hd.min(hw), -hd, 0.0, hd.min(hw), hd, 0.0, Main, false);
            p.b(
                -hw + 12.0 * IN,
                -hd + 4.0 * IN,
                0.0,
                hw,
                hd - 4.0 * IN,
                8.0 * IN,
                Main,
                true,
            );
            p.b(
                -hw + 14.0 * IN,
                -hd + 7.0 * IN,
                8.0 * IN,
                hw - 2.0 * IN,
                hd - 7.0 * IN,
                8.5 * IN,
                Dark,
                true,
            );
            for sy in [-1.0, 1.0] {
                p.b(
                    -hw + 2.0 * IN,
                    sy * (hd - 3.0 * IN) - IN,
                    0.0,
                    -hw + 5.0 * IN,
                    sy * (hd - 3.0 * IN) + IN,
                    h - 6.0 * IN,
                    Accent,
                    false,
                );
            }
            p.b(
                -hw,
                -hd + 3.0 * IN,
                h - 8.0 * IN,
                -hw + 8.0 * IN,
                hd - 3.0 * IN,
                h,
                Dark,
                true,
            );
        }
        Elliptical | Bike => {
            p.b(-hw, -2.0 * IN, 0.0, hw, 2.0 * IN, 3.0 * IN, Main, true);
            p.c(
                -hw + 10.0 * IN,
                0.0,
                9.0 * IN,
                3.0 * IN,
                5.0 * IN,
                Dark,
                false,
            );
            p.b(
                -hw + 6.0 * IN,
                -1.5 * IN,
                3.0 * IN,
                -hw + 9.0 * IN,
                1.5 * IN,
                h - 8.0 * IN,
                Accent,
                false,
            );
            p.b(
                -hw + 2.0 * IN,
                -hd + 2.0 * IN,
                h - 8.0 * IN,
                -hw + 12.0 * IN,
                hd - 2.0 * IN,
                h,
                Dark,
                true,
            );
            if s.kind == Bike {
                p.b(
                    hw - 16.0 * IN,
                    -1.5 * IN,
                    3.0 * IN,
                    hw - 13.0 * IN,
                    1.5 * IN,
                    h - 14.0 * IN,
                    Accent,
                    false,
                );
                p.b(
                    hw - 20.0 * IN,
                    -4.0 * IN,
                    h - 14.0 * IN,
                    hw - 9.0 * IN,
                    4.0 * IN,
                    h - 12.0 * IN,
                    Main,
                    true,
                );
            } else {
                for sy in [-1.0, 1.0] {
                    p.b(
                        -hw + 20.0 * IN,
                        sy * 6.0 * IN - 3.0 * IN,
                        8.0 * IN,
                        hw - 6.0 * IN,
                        sy * 6.0 * IN + 3.0 * IN,
                        10.0 * IN,
                        Accent,
                        true,
                    );
                }
            }
        }
        WeightBench => {
            if n >= 1 {
                for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    let (x, y) = (sx * (hw - 2.0 * IN), sy * (hd - 2.0 * IN));
                    p.b(
                        x - 1.5 * IN,
                        y - 1.5 * IN,
                        0.0,
                        x + 1.5 * IN,
                        y + 1.5 * IN,
                        h,
                        Main,
                        true,
                    );
                }
                p.b(-hw, -hd, h - 3.0 * IN, hw, hd, h, Main, false);
                p.b(
                    -24.0 * IN,
                    -6.0 * IN,
                    0.0,
                    24.0 * IN,
                    6.0 * IN,
                    2.0 * IN,
                    Main,
                    false,
                );
                p.b(
                    -8.0 * IN,
                    -24.0 * IN,
                    2.0 * IN,
                    8.0 * IN,
                    24.0 * IN,
                    17.0 * IN,
                    Accent,
                    true,
                );
            } else {
                p.b(
                    -hw,
                    -hd + 5.0 * IN,
                    15.0 * IN,
                    hw,
                    hd - 5.0 * IN,
                    h,
                    Accent,
                    true,
                );
                p.legs(w, d, 3.0 * IN, 2.0 * IN, 15.0 * IN, Main);
            }
        }
        Rack => {
            p.legs(w, d, 1.0 * IN, 2.0 * IN, h, Main);
            for (z, y0) in [(h * 0.45, -hd), (h - 2.0 * IN, -hd + 6.0 * IN)] {
                p.b(-hw, y0, z, hw, hd, z + 2.0 * IN, Main, true);
                let k = 8;
                for i in 0..k {
                    let x = -hw + 4.0 * IN + (w - 8.0 * IN) * i as f64 / (k - 1) as f64;
                    p.c(
                        x,
                        (y0 + hd) / 2.0,
                        2.5 * IN,
                        z + 2.0 * IN,
                        z + 5.0 * IN,
                        Dark,
                        false,
                    );
                }
            }
        }
        Grill => {
            p.b(-hw, -hd, 0.0, hw, hd, 36.0 * IN, Accent, true);
            p.b(
                -hw + 2.0 * IN,
                -hd + 1.0 * IN,
                36.0 * IN,
                hw - 2.0 * IN,
                hd - 1.0 * IN,
                h - 2.0 * IN,
                Main,
                true,
            );
            p.b(
                -hw + 4.0 * IN,
                -hd - 1.0 * IN,
                h - 6.0 * IN,
                hw - 4.0 * IN,
                -hd + 1.0 * IN,
                h - 5.0 * IN,
                Metal,
                false,
            );
        }
    }
    p.parts
}

/// A door panel on the front face from x0 to x1 and z0 to z1, with a bar handle.
fn door(p: &mut P, x0: f64, x1: f64, z0: f64, z1: f64, hd: f64, t: f64) {
    p.b(
        x0 + 0.25 * IN,
        -hd,
        z0 + 0.25 * IN,
        x1 - 0.25 * IN,
        -hd + t,
        z1 - 0.25 * IN,
        Tone::Main,
        false,
    );
    // The handle toward the middle: a left door's at its right edge.
    let hx = if x1 > 0.0 {
        x0 + 2.0 * IN
    } else {
        x1 - 2.0 * IN
    };
    let (hz0, hz1) = (z0 + (z1 - z0) * 0.2, z0 + (z1 - z0) * 0.8);
    p.b(
        hx - 0.4 * IN,
        -hd - 1.5 * IN,
        hz0,
        hx + 0.4 * IN,
        -hd,
        hz1,
        Tone::Metal,
        false,
    );
}

/// Burners on a cooktop surface: two rows.
fn burners(p: &mut P, w: f64, d: f64, top: f64, n: u32) {
    let n = n.max(2);
    let cols = n.div_ceil(2);
    let (cw, rd) = (w / cols as f64, d / 2.0);
    for i in 0..n {
        let (c, r) = (i / 2, i % 2);
        let x = -w / 2.0 + cw * (c as f64 + 0.5);
        let y = -d / 2.0 + rd * (r as f64 + 0.5) - 2.0 * IN;
        p.c(
            x,
            y,
            (cw.min(rd) * 0.32).max(2.5 * IN),
            top,
            top + 0.4 * IN,
            Tone::Dark,
            true,
        );
    }
}

// ---------- Placement ----------

struct Frame {
    at: Pt,
    z: f64,
    u: Pt,
    v: Pt,
}

impl Frame {
    fn world(&self, x: f64, y: f64) -> Pt {
        self.at.add(self.u.scale(x)).add(self.v.scale(y))
    }
}

fn frame(doc: &Document, data: &ElementData) -> Option<(Frame, FfeSpec, ElementId)> {
    let ElementData::Ffe {
        type_id,
        level,
        at,
        rotation,
        offset,
        ..
    } = data
    else {
        return None;
    };
    let spec = studio_core::ffe::spec_of(doc, *type_id)?.clone();
    let z = doc.level_elevation(*level).ok()? + offset;
    let u = Pt::new(rotation.cos(), rotation.sin());
    Some((
        Frame {
            at: *at,
            z,
            u,
            v: u.perp(),
        },
        spec,
        *level,
    ))
}

fn circle(c: (f64, f64), r: f64, n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / n as f64;
            (c.0 + r * a.cos(), c.1 + r * a.sin())
        })
        .collect()
}

// ---------- Plan ----------

/// The furniture and equipment of `level` in a floor plan: each piece's plan parts, thin,
/// over a paper fill (so they read over floor patterns). Wall pieces above the 4' cut plane
/// draw dashed, as overhead.
pub(crate) fn plan_symbols(doc: &Document, b: &mut Builder, level: ElementId) {
    for e in doc
        .of(Category::Furniture)
        .chain(doc.of(Category::SpecialtyEquipment))
    {
        let Some((fr, spec, l)) = frame(doc, &e.data) else {
            continue;
        };
        if l != level {
            continue;
        }
        let overhead = fr.z - doc.level_elevation(l).unwrap_or(0.0) > 48.0 * IN;
        let dash = if overhead { Dash::Dashed } else { Dash::Solid };
        let el = Some(e.id);
        let (hw, hd) = (spec.width / 2.0, spec.depth / 2.0);
        let outline = vec![
            fr.world(-hw, -hd),
            fr.world(hw, -hd),
            fr.world(hw, hd),
            fr.world(-hw, hd),
        ];
        let mut first = true;
        for part in parts(&spec) {
            let pts: Vec<Pt> = match part {
                Part::Box {
                    x0,
                    y0,
                    x1,
                    y1,
                    plan: true,
                    ..
                } => vec![
                    fr.world(x0, y0),
                    fr.world(x1, y0),
                    fr.world(x1, y1),
                    fr.world(x0, y1),
                ],
                Part::Cyl {
                    cx,
                    cy,
                    r,
                    plan: true,
                    ..
                } => circle((cx, cy), r, 24)
                    .into_iter()
                    .map(|(x, y)| fr.world(x, y))
                    .collect(),
                _ => continue,
            };
            if first && !overhead {
                b.fill(el, vec![ring(&outline)], FillKind::Paper);
                first = false;
            }
            b.line(el, &pts, true, 1, dash);
        }
    }
}

// ---------- 3D ----------

fn tri(out: &mut Vec<f32>, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
    for p in [a, b, c] {
        out.extend(p.iter().map(|v| *v as f32));
    }
}

/// A prism over `ring` (counter-clockwise, world plan) from z0 to z1.
/// A market umbrella's canopy (ADR-101), radius `r` to its eight rib tips, from its apex
/// down to the tips (mm, its own frame): the canvas's top and underside, the panels
/// sagging between ribs, and a 6" valance hanging at the edge. Triangle soup, each face
/// wound outward.
fn canopy(r: f64, apex: f64, tip: f64) -> Vec<[f64; 3]> {
    use std::f64::consts::{PI, TAU};
    const RIBS: usize = 8;
    const RINGS: usize = 6;
    const STEPS: usize = 4;
    let vent = 7.0 * IN;
    let thick = 0.35 * IN;
    // A point on the canvas: `rho` 0 at the vent to 1 at the edge, `phi` around.
    let at = |rho: f64, phi: f64| -> [f64; 3] {
        let k = phi / (TAU / RIBS as f64);
        let f = k - k.floor();
        // Between ribs the fabric falls short of the rib line and sags.
        let pull = 1.0 - 0.055 * (PI * f).sin() * rho;
        let rr = (vent + (r - vent) * rho) * pull;
        let drop = (apex - tip) * rho.powf(1.35) + 1.6 * IN * (PI * f).sin() * rho;
        [rr * phi.cos(), rr * phi.sin(), apex - drop]
    };
    let mut out: Vec<[f64; 3]> = vec![];
    let normal = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| -> [f64; 3] {
        let (u, v) = (
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    // Wound so the face's normal points up (`up`) or down.
    let tri = |out: &mut Vec<[f64; 3]>, a: [f64; 3], b: [f64; 3], c: [f64; 3], up: bool| {
        if (normal(a, b, c)[2] >= 0.0) == up {
            out.extend([a, b, c]);
        } else {
            out.extend([a, c, b]);
        }
    };
    let n = RIBS * STEPS;
    let phi = |j: usize| j as f64 / n as f64 * TAU;
    let lo = |q: [f64; 3]| [q[0], q[1], q[2] - thick];
    for i in 0..RINGS {
        let (r0, r1) = (i as f64 / RINGS as f64, (i + 1) as f64 / RINGS as f64);
        for j in 0..n {
            let (p0, p1) = (phi(j), phi(j + 1));
            let (a, b, c, d) = (at(r0, p0), at(r0, p1), at(r1, p1), at(r1, p0));
            tri(&mut out, a, b, c, true);
            tri(&mut out, a, c, d, true);
            tri(&mut out, lo(a), lo(b), lo(c), false);
            tri(&mut out, lo(a), lo(c), lo(d), false);
        }
    }
    // The vent's top, closing the canvas at the apex.
    for j in 0..n {
        let top = [0.0, 0.0, apex + 0.6 * IN];
        tri(&mut out, top, at(0.0, phi(j)), at(0.0, phi(j + 1)), true);
    }
    // The valance: a flap hanging from the edge, facing out.
    let flap = 6.0 * IN;
    for j in 0..n {
        let (a, b) = (at(1.0, phi(j)), at(1.0, phi(j + 1)));
        let (c, d) = ([b[0], b[1], b[2] - flap], [a[0], a[1], a[2] - flap]);
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        for (x, y, z) in [(a, b, c), (a, c, d)] {
            let nn = normal(x, y, z);
            if nn[0] * mid[0] + nn[1] * mid[1] >= 0.0 {
                out.extend([x, y, z]);
            } else {
                out.extend([x, z, y]);
            }
        }
    }
    out
}

fn prism(out: &mut Vec<f32>, ring: &[Pt], z0: f64, z1: f64) {
    let n = ring.len();
    let p = |q: Pt, z: f64| [q.x, q.y, z];
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        tri(out, p(a, z0), p(b, z0), p(b, z1));
        tri(out, p(a, z0), p(b, z1), p(a, z1));
    }
    for i in 1..n.saturating_sub(1) {
        tri(out, p(ring[0], z1), p(ring[i], z1), p(ring[i + 1], z1));
        tri(out, p(ring[0], z0), p(ring[i + 1], z0), p(ring[i], z0));
    }
}

fn tone_color(spec: &FfeSpec, t: Tone) -> [u8; 3] {
    match t {
        Tone::Main => spec.color,
        Tone::Cushion => spec.cushion.unwrap_or(spec.color),
        Tone::Accent => spec.accent,
        Tone::Dark => [38, 38, 40],
        Tone::Metal => [170, 174, 178],
        Tone::Glass => [42, 54, 66],
        Tone::Light => [244, 243, 238],
    }
}

/// Triangles of a piece by tone.
fn triangles(fr: &Frame, spec: &FfeSpec) -> Vec<(Tone, Vec<f32>)> {
    let mut by: Vec<(Tone, Vec<f32>)> = vec![];
    let get = |by: &mut Vec<(Tone, Vec<f32>)>, t: Tone| -> usize {
        match by.iter().position(|x| x.0 == t) {
            Some(i) => i,
            None => {
                by.push((t, vec![]));
                by.len() - 1
            }
        }
    };
    for part in parts(spec) {
        match part {
            Part::Box {
                x0,
                y0,
                z0,
                x1,
                y1,
                z1,
                tone,
                ..
            } => {
                if z1 - z0 <= 0.0 {
                    continue;
                }
                let i = get(&mut by, tone);
                let ring = vec![
                    fr.world(x0, y0),
                    fr.world(x1, y0),
                    fr.world(x1, y1),
                    fr.world(x0, y1),
                ];
                prism(&mut by[i].1, &ring, fr.z + z0, fr.z + z1);
            }
            Part::Tris { tris, tone } => {
                let i = get(&mut by, tone);
                for v in tris {
                    let q = fr.world(v[0], v[1]);
                    by[i]
                        .1
                        .extend([q.x as f32, q.y as f32, (fr.z + v[2]) as f32]);
                }
            }
            Part::Cyl {
                cx,
                cy,
                r,
                z0,
                z1,
                tone,
                ..
            } => {
                // A flat one is only a plan symbol.
                if z1 - z0 <= 0.0 {
                    continue;
                }
                let i = get(&mut by, tone);
                let ring: Vec<Pt> = circle((cx, cy), r, 20)
                    .into_iter()
                    .map(|(x, y)| fr.world(x, y))
                    .collect();
                prism(&mut by[i].1, &ring, fr.z + z0, fr.z + z1);
            }
            Part::Front {
                cx,
                cz,
                r,
                y0,
                y1,
                tone,
            } => {
                // A disc facing front: the ring in (x, z), swept along y.
                let i = get(&mut by, tone);
                let ring = circle((cx, cz), r, 20);
                let n = ring.len();
                let w = |x: f64, y: f64, z: f64| {
                    let p = fr.world(x, y);
                    [p.x, p.y, fr.z + z]
                };
                for k in 0..n {
                    let (a, b) = (ring[k], ring[(k + 1) % n]);
                    tri(
                        &mut by[i].1,
                        w(a.0, y0, a.1),
                        w(b.0, y0, b.1),
                        w(b.0, y1, b.1),
                    );
                    tri(
                        &mut by[i].1,
                        w(a.0, y0, a.1),
                        w(b.0, y1, b.1),
                        w(a.0, y1, a.1),
                    );
                }
                for k in 1..n - 1 {
                    tri(
                        &mut by[i].1,
                        w(ring[0].0, y0, ring[0].1),
                        w(ring[k + 1].0, y0, ring[k + 1].1),
                        w(ring[k].0, y0, ring[k].1),
                    );
                }
            }
        }
    }
    by
}

/// 3D meshes of the furniture and equipment, one per finish.
pub(crate) fn meshes(doc: &Document, out: &mut Vec<Mesh>) {
    for e in doc
        .of(Category::Furniture)
        .chain(doc.of(Category::SpecialtyEquipment))
    {
        let Some((fr, spec, level)) = frame(doc, &e.data) else {
            continue;
        };
        let name = e
            .data
            .type_id()
            .and_then(|t| doc.data(t).ok())
            .map(|t| t.name())
            .unwrap_or_default();
        for (tone, positions) in triangles(&fr, &spec) {
            if positions.is_empty() {
                continue;
            }
            out.push(Mesh {
                finish: finish_of(spec.kind, tone, &name),
                el: e.id,
                category: spec.class.category(),
                exterior: false,
                color: Some(tone_color(&spec, tone)),
                material: None,
                level: Some(level),
                positions,
                edges: vec![],
                glow: None,
            });
        }
    }
}

/// How a part renders (ADR-095): cushions as fabric, frames as wicker or wood, appliance
/// bodies as stainless, by the piece and the part's tone. `name` is its type's, which says
/// whether it's outdoor furniture.
pub fn finish_of(kind: FfeKind, tone: Tone, name: &str) -> Option<crate::Finish> {
    use crate::Finish as F;
    use FfeKind::*;
    let n = name.to_lowercase();
    let outdoor = [
        "outdoor",
        "pool",
        "adirondack",
        "patio",
        "fire table",
        "grill",
    ]
    .iter()
    .any(|w| n.contains(w));
    let soft = n.contains("upholstered") || n.contains("lounge") || n.contains("club");
    Some(match tone {
        Tone::Glass => F::Glass,
        Tone::Metal => F::Stainless,
        Tone::Dark => F::PowderCoat,
        Tone::Light | Tone::Cushion => F::Fabric,
        Tone::Main | Tone::Accent => {
            let main = tone == Tone::Main;
            match kind {
                Sofa | Armchair if main && outdoor => F::Wicker,
                Sofa | Sectional | Armchair | Chaise | Ottoman | Booth | Bench if main => {
                    if kind == Bench && !soft {
                        F::Wood
                    } else {
                        F::Fabric
                    }
                }
                Sofa | Sectional | Armchair | Chaise | Ottoman | Booth | Bench => {
                    if outdoor {
                        F::Wicker
                    } else {
                        F::Wood
                    }
                }
                Chair | Stool | OfficeChair if main && soft => F::Fabric,
                Chair | Stool | Table | RoundTable | Desk | Bookcase | LuggageRack | BunkBed
                | Crib => {
                    if kind == Table && n.contains("fire") {
                        F::Stone
                    } else {
                        F::Wood
                    }
                }
                Bed if main => F::Fabric,
                Bed => F::Wood,
                Rug => F::Fabric,
                Umbrella if main => F::Canvas,
                Umbrella => F::Wood,
                PoolTable if main => F::Wood,
                PoolTable => F::Fabric,
                Casegood | Cabinet | Wardrobe if main => F::Lacquer,
                Casegood | Cabinet | Wardrobe => F::PowderCoat,
                Grill if main => F::Stainless,
                Grill => F::Wicker,
                Refrigerator | Range | Cooktop | WallOven | Microwave | OtrMicrowave | Hood
                | Dishwasher | WineCooler | IceMachine | ChestFreezer
                    if main =>
                {
                    F::Stainless
                }
                Tv => F::PowderCoat,
                _ if main => F::Lacquer,
                _ => F::PowderCoat,
            }
        }
    })
}

/// A piece by itself, for the picker: its triangles by color.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FfeThumb {
    pub parts: Vec<FfeThumbPart>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FfeThumbPart {
    pub positions: Vec<f32>,
    pub color: [u8; 3],
}

pub fn thumb(spec: &FfeSpec) -> FfeThumb {
    let fr = Frame {
        at: Pt::default(),
        z: 0.0,
        u: Pt::new(1.0, 0.0),
        v: Pt::new(0.0, 1.0),
    };
    FfeThumb {
        parts: triangles(&fr, spec)
            .into_iter()
            .filter(|(_, p)| !p.is_empty())
            .map(|(t, positions)| FfeThumbPart {
                positions,
                color: tone_color(spec, t),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::ffe::{catalog, FfeClass};

    #[test]
    fn every_library_piece_builds_within_its_box() {
        for class in [FfeClass::Furniture, FfeClass::Equipment] {
            for p in catalog(class) {
                let s = &p.spec;
                let parts = parts(s);
                assert!(!parts.is_empty(), "{}", p.name);
                assert!(
                    parts.iter().any(|x| matches!(
                        x,
                        Part::Box { plan: true, .. } | Part::Cyl { plan: true, .. }
                    )),
                    "{} has a plan symbol",
                    p.name
                );
                let t = thumb(s);
                let tris: usize = t.parts.iter().map(|x| x.positions.len() / 9).sum();
                assert!(tris >= 12, "{}: {tris} triangles", p.name);
                // Nothing sticks out of the type's footprint by more than a handle's depth.
                for part in &t.parts {
                    for xyz in part.positions.chunks(3) {
                        let (x, y) = (f64::from(xyz[0]), f64::from(xyz[1]));
                        assert!(
                            x.abs() <= s.width / 2.0 + 2.0 * IN
                                && y.abs() <= s.depth / 2.0 + 5.0 * IN,
                            "{} part at {x:.0},{y:.0}",
                            p.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_queen_bed_draws_its_mattress_and_two_pillows_in_plan() {
        let q = catalog(FfeClass::Furniture)
            .into_iter()
            .find(|p| p.name == "Queen Bed")
            .unwrap();
        let plan = parts(&q.spec)
            .iter()
            .filter(|x| matches!(x, Part::Box { plan: true, .. }))
            .count();
        // Frame, mattress, duvet, two pillows, headboard.
        assert_eq!(plan, 6);
        let r = catalog(FfeClass::Equipment)
            .into_iter()
            .find(|p| p.name == "Range 30\"")
            .unwrap();
        let burners = parts(&r.spec)
            .iter()
            .filter(|x| matches!(x, Part::Cyl { plan: true, .. }))
            .count();
        assert_eq!(burners, 4);
    }

    #[test]
    fn placed_pieces_draw_in_floor_plans_and_3d_in_their_category() {
        use studio_core::ffe::{create, load};
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let bed = load(&mut doc, &["Queen Bed".to_string()]).unwrap()[0];
        let range = load(&mut doc, &["Range 30\"".to_string()]).unwrap()[0];
        let b1 = create(&mut doc, bed, l1, Pt::new(2000.0, 2000.0), 0.0).unwrap();
        let r1 = create(&mut doc, range, l1, Pt::new(5000.0, 2000.0), 0.0).unwrap();
        let mut b = Builder::new(48.0);
        plan_symbols(&doc, &mut b, l1);
        for id in [b1, r1] {
            assert!(b
                .items
                .iter()
                .any(|i| i.el == Some(id) && matches!(i.prim, crate::Prim::Line { .. })));
        }
        let mut m = vec![];
        meshes(&doc, &mut m);
        let bed_mesh: Vec<&Mesh> = m.iter().filter(|x| x.el == b1).collect();
        assert!(bed_mesh.len() >= 3 && bed_mesh.iter().all(|x| x.category == Category::Furniture));
        assert!(m
            .iter()
            .filter(|x| x.el == r1)
            .all(|x| x.category == Category::SpecialtyEquipment));
        // The queen frame is 64" wide (a 60" mattress), on the floor.
        let zs: Vec<f32> = bed_mesh
            .iter()
            .flat_map(|x| x.positions.chunks(3).map(|q| q[2]))
            .collect();
        assert!(zs.iter().cloned().fold(f32::INFINITY, f32::min).abs() < 0.01);
        let xs: Vec<f32> = bed_mesh
            .iter()
            .flat_map(|x| x.positions.chunks(3).map(|q| q[0]))
            .collect();
        let span = xs.iter().cloned().fold(f32::MIN, f32::max)
            - xs.iter().cloned().fold(f32::MAX, f32::min);
        assert!((f64::from(span) - 64.0 * IN).abs() < 1.0, "{span}");
    }
}
