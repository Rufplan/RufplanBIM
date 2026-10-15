//! Window details (ADR-123): head, jamb and sill, and a hung window's meeting rail or a
//! slider's interlock, drawn by the door engine. A casement, awning or fixed sash closes
//! against an interior stop like an outswing leaf, a tilt-turn against an exterior one, and
//! hung and sliding sashes run in two tracks (`Door::shape`). The sill is the window's own:
//! a rough sill in the wall, the sill pan, the frame's sloped sill and, inside, a stool and
//! apron or a sill board.

use super::*;

pub(super) fn draw(d: &mut D, door: &Door, l: &Lay, part: Part) {
    let mut l = l.clone();
    if door.leaf <= 0.0 {
        // Glass set straight in the frame: centred in it.
        let c = (l.f0 + l.f1) / 2.0;
        l.leaves = vec![(c, c)];
    }
    let hung = matches!(door.op, Op::Window(Win::Hung { .. }));
    match part {
        Part::Head => {
            // A hung window's head holds only its upper (exterior) sash.
            let mut lh = l.clone();
            if hung {
                lh.leaves.truncate(1);
            }
            let mut t = T::head(d, &lh);
            framed_head_or_jamb(&mut t, door, &lh, false);
        }
        Part::Jamb => {
            let mut t = T::jamb(d, &l);
            framed_head_or_jamb(&mut t, door, &l, true);
        }
        Part::Sill => sill(d, door, &l),
        Part::Meet if hung => meeting_rail(d, door, &l),
        Part::Meet => meet(d, door, &l),
    }
}

/// The window's sill in section: y 0 the top of the rough sill.
fn sill(d: &mut D, door: &Door, l: &Lay) {
    let mut t = T::head(d, l);
    let (f0, f1, xs) = (l.f0, l.f1, l.xs);
    let ys = 0.375;
    let yi = ys
        + if door.frame_sill > 0.0 {
            door.frame_sill
        } else {
            door.jamb
        };
    let drop = (door.depth * 0.08).clamp(0.25, 0.5);
    let xn = f0 - 0.375;
    // The frame's sill: sloped to the exterior with a drip, its stop or tracks inside.
    let mut top = vec![(xn, yi - drop - 0.1), (f0, yi - drop)];
    let (x0, x1) = l.leaves[0];
    match door.shape() {
        Op::Swing { out: false, .. } => top.extend([
            (x0 - 0.625, yi - 0.05),
            (x0 - 0.625, yi + 0.625),
            (x0 - 0.125, yi + 0.625),
            (x0 - 0.125, yi),
            (f1, yi),
        ]),
        Op::Slide { .. } => {
            let n = l.leaves.len();
            for (i, &(a, b)) in l.leaves.iter().enumerate() {
                let s = yi - drop * (1.0 - (i as f64 + 1.0) / n as f64);
                top.extend([(a - 0.2, s), (b + 0.2, s)]);
                if i + 1 < n {
                    top.extend([(b + 0.2, s + 0.5), (b + 0.4, s + 0.5)]);
                }
            }
            top.extend([(f1 - 0.3, yi), (f1 - 0.3, yi + 0.625), (f1, yi + 0.625)]);
        }
        _ => {
            let s = x1.max(x0) + 0.125;
            top.extend([
                (s, yi),
                (s, yi + 0.625),
                (s + 0.5, yi + 0.625),
                (s + 0.5, yi),
                (f1, yi),
            ]);
        }
    }
    let mut pts = vec![
        (f1, ys),
        (xn + 0.15, ys),
        (xn + 0.15, ys - 0.35),
        (xn, ys - 0.35),
    ];
    pts.extend(top.iter().copied());
    member(&mut t, door.mat, &pts, f0, ys, f1, yi);
    if door.mat == Mat::CladWood {
        t.line(
            &[
                (xn - 0.05, ys - 0.35),
                (xn - 0.05, yi - drop - 0.05),
                (f0 + 0.75, yi - drop + 0.02),
            ],
            Wide,
        );
    }
    if door.mount == Mount::Fin {
        t.rect(xs - 0.0625, ys - 1.75, xs, ys + 0.25, Medium);
    }
    // The sashes' bottoms: a hung window's lower (interior) sash only.
    let hung = matches!(door.op, Op::Window(Win::Hung { .. }));
    let top_at = |x: f64| -> f64 {
        top.windows(2)
            .filter(|w| x >= w[0].0.min(w[1].0) && x <= w[0].0.max(w[1].0))
            .map(|w| {
                if (w[1].0 - w[0].0).abs() < 1e-9 {
                    w[0].1.max(w[1].1)
                } else {
                    w[0].1 + (w[1].1 - w[0].1) * (x - w[0].0) / (w[1].0 - w[0].0)
                }
            })
            .fold(ys, f64::max)
    };
    let sashes: Vec<(f64, f64)> = if hung {
        l.leaves.iter().skip(1).copied().collect()
    } else {
        l.leaves.clone()
    };
    for &(a, b) in &sashes {
        let edge = top_at((a + b) / 2.0) + 0.125;
        leaf(&mut t, door, a, b, edge, 1.0, door.bottom_rail, 14.0);
        if door.leaf > 0.0 {
            t.circle(b + 0.1, edge + 0.25, 0.1, Thin);
        }
    }
    let (sa, sb) = sashes.first().copied().unwrap_or((x0, x1));
    t.brk(
        (sa - 1.0, 14.0),
        (sashes.last().map_or(sb, |p| p.1) + 1.0, 14.0),
    );
    let commercial = matches!(door.wall, Wall::Cmu | Wall::MetalPanel);
    // The rough sill and the wall under it.
    match door.wall {
        Wall::Cmu => {
            t.cut_rect(0.0, -12.0, l.wd, -2.5, Masonry);
            t.cut_rect(1.5, -2.5, l.wd, 0.0, Concrete);
            t.cut(
                &[
                    (l.xo - 1.5, -2.75),
                    (l.xo - 1.5, -1.75),
                    (1.5, -0.4),
                    (1.5, -2.75),
                ],
                Concrete,
            );
            t.note((l.wd / 2.0, -7.0), "8\" CMU");
            t.note((l.xo - 0.5, -1.6), "PRECAST CONC. SILL, SLOPED, W/ DRIP");
        }
        Wall::MetalPanel => {
            t.flashing(&[(0.0, -1.25), (0.0, 0.0), (l.wd, 0.0), (l.wd, -1.25)]);
            t.batt((l.wd / 2.0, -1.25), (l.wd / 2.0, -12.0), l.wd);
            t.sheet(-0.625, -12.0, 0.0, 0.0);
            t.cut_rect(-2.125, -12.0, -0.625, -0.75, RigidInsulation);
            t.rect(l.xo, -12.0, -2.125, -1.25, Medium);
            t.flashing(&[(-0.6, -0.75), (l.xo - 0.6, -1.0), (l.xo - 0.6, -1.5)]);
            t.note((l.wd / 2.0, 0.0), "STEEL STUD SILL TRACK");
            t.note(((l.xo - 2.125) / 2.0, -6.0), "METAL PANEL ON 1-1/2\" CI");
            t.note((l.xo - 0.6, -1.25), "SILL FLASHING W/ DRIP");
        }
        _ => {
            t.lumber(0.0, -1.5, l.wd, 0.0);
            t.batt((l.wd / 2.0, -1.5), (l.wd / 2.0, -12.0), l.wd);
            t.sheet(xs.min(-0.5), -12.0, 0.0, 0.0);
            t.note((l.wd / 2.0, -0.75), "2x6 ROUGH SILL");
        }
    }
    // The sill pan (or a commercial window's subsill receptor) under the frame.
    let back = f1.min(l.xi - 0.25) + 0.06;
    if commercial {
        t.flashing(&[
            (l.xo - 0.75, -0.6),
            (l.xo - 0.75, -0.05),
            (back, 0.12),
            (back, ys + 0.75),
        ]);
        t.note(
            ((l.xo + back) / 2.0, 0.05),
            "ALUMINUM SUBSILL W/ END DAMS, SET IN SEALANT",
        );
    } else {
        t.flashing(&[
            (back, 1.0),
            (back, 0.03),
            (xs - 0.03, 0.03),
            (xs - 0.03, -2.0),
        ]);
        t.note(
            (xs + 0.5, 0.03),
            "SILL PAN FLASHING SLOPED TO EXTERIOR, END DAMS",
        );
        t.note((back, 0.7), "BACK DAM");
    }
    // Outside, under the frame's nosing.
    let under = ys - 0.4;
    match door.wall {
        Wall::Siding => {
            t.poly(
                &[
                    (xs - 0.1, under),
                    (xs - 1.9, under - 0.55),
                    (xs - 1.9, under - 1.25),
                    (xs - 0.1, under - 0.85),
                ],
                Medium,
            );
            t.line(&[(xs - 0.12, -2.0), (xs - 0.12, -12.0)], Hidden);
            super::siding_up(&mut t, xs, -12.0, under - 1.4);
            t.note((xs - 1.0, under - 0.6), "SLOPED EXTERIOR SILL W/ DRIP");
        }
        Wall::Stucco => {
            t.cut_rect(l.xo, -12.0, xs, under - 0.5, Sand);
            t.line(
                &[(l.xo, under - 0.5), (xs, under - 0.5), (xs, under - 1.5)],
                Wide,
            );
            t.note((l.xo + 0.4, -6.0), "7/8\" 3-COAT STUCCO ON 2 LAYERS WRB");
            t.note((xs - 0.1, under - 1.0), "CASING BEAD, 3/8\" JOINT");
        }
        Wall::Brick => {
            // A rowlock sill sloped out, on thru-wall flashing over the course below.
            let back_b = l.xo + 3.625;
            t.cut(
                &[
                    (l.xo - 1.25, under - 1.6),
                    (back_b, under - 0.3),
                    (back_b, under - 3.9),
                    (l.xo - 1.25, under - 4.2),
                ],
                Masonry,
            );
            t.flashing(&[
                (xs - 0.06, under - 0.5),
                (xs - 0.06, under - 4.0),
                (back_b, under - 4.25),
                (l.xo - 1.6, under - 4.45),
                (l.xo - 1.6, under - 4.8),
            ]);
            t.comp("brick-mod", (back_b, -12.0), (back_b, under - 4.6), false);
            t.line(&[(xs - 0.06, under), (xs - 0.06, -12.0)], Hidden);
            t.note(
                (l.xo + 1.0, under - 2.0),
                "BRICK ROWLOCK SILL, SLOPED 15 DEG.",
            );
            t.note(
                (back_b, under - 4.3),
                "THRU-WALL FLASHING W/ END DAMS & WEEPS",
            );
            t.note(((l.xo + back_b) / 2.0, -9.0), "BRICK VENEER, 1\" AIR SPACE");
        }
        _ => {}
    }
    t.seal("seal-14", (xn + 0.2, under), (-1.0, 0.0));
    // Inside: a stool and apron, or a sill board on a gypsum return.
    let x = l.xi.max(f1);
    if commercial
        || door.mat == Mat::Aluminum
        || door.mat == Mat::PlainAluminum
        || matches!(door.mat, Mat::Steel | Mat::SolidSteel)
    {
        if f1 < l.xi {
            t.cut_rect(f1 + 0.06, yi - 0.75, l.xi + 0.5, yi, Wood);
        }
        t.gyp(l.wd, -12.0, l.xi, yi - 0.75);
        t.note(
            ((f1 + l.xi) / 2.0, yi - 0.4),
            "SOLID-SURFACE OR WOOD SILL BOARD",
        );
    } else {
        t.cut_rect(f1.min(l.xi), yi - 1.0, x + 1.0, yi, Wood);
        t.lumber(x, yi - 1.0 - 3.5, x + 0.6875, yi - 1.0);
        t.gyp(l.wd, -12.0, l.xi, yi - 1.0);
        t.note((x + 0.5, yi - 0.5), "5/4 WOOD STOOL");
        t.note((x + 0.35, yi - 3.0), "APRON");
    }
    t.brk((l.xo - 2.0, -12.0), (l.xi + 2.0, -12.0));
    // The window's own notes.
    let mut frame_note = format!(
        "{} {} SILL, {} FRAME DEPTH",
        door.frame_note_or(),
        mat_note(door.mat),
        fr(door.depth)
    );
    if door.mount == Mount::Fin {
        frame_note.push_str(", FIN OVER PAN");
    }
    t.note(((f0 + f1) / 2.0, ys + 0.2), &frame_note);
    if !door.sill_note.is_empty() {
        t.note((xn + 0.2, yi - drop - 0.2), door.sill_note);
    }
    let (ca, cb) = sashes.first().copied().unwrap_or((x0, x1));
    let edge = top_at((ca + cb) / 2.0) + 0.125;
    if door.leaf > 0.0 {
        t.note(
            ((ca + cb) / 2.0, edge + door.bottom_rail / 2.0),
            &format!(
                "{}{} SASH, {} BOTTOM RAIL",
                leaf_label(door),
                fr(door.leaf),
                fr(door.bottom_rail)
            ),
        );
    } else {
        t.note(
            ((ca + cb) / 2.0, edge + 0.4),
            "GLASS SET IN FRAME W/ GASKETS & SNAP-IN STOPS",
        );
    }
    if door.glass > 0.0 {
        let note = if door.glass_note.is_empty() {
            format!("{} INSULATING GLASS", fr(door.glass))
        } else {
            door.glass_note.to_owned()
        };
        t.note(((ca + cb) / 2.0, 11.0), &note);
    }
    t.note(
        (xn + 0.2, under),
        "SEALANT & BACKER ROD; LEAVE THE PAN'S DRAINAGE OPEN",
    );
}

/// A hung window's meeting rails in section: the upper sash (outside) down to the line,
/// the lower sash (inside) up to it, interlocked, with the sash lock on top.
fn meeting_rail(d: &mut D, door: &Door, l: &Lay) {
    let mut t = T::head(d, l);
    let mr = door.meet_rail;
    let (a, b) = (l.leaves[0], l.leaves[1.min(l.leaves.len() - 1)]);
    leaf(&mut t, door, a.0, a.1, -mr / 2.0, 1.0, mr, 14.0);
    leaf(&mut t, door, b.0, b.1, mr / 2.0, -1.0, mr, -14.0);
    // The interlock between the rails, and the lock on the lower sash.
    let mid = (a.1 + b.0) / 2.0;
    t.flashing(&[
        (a.1, -mr * 0.3),
        (mid + 0.12, -mr * 0.3),
        (mid + 0.12, mr * 0.1),
    ]);
    t.flashing(&[
        (b.0, mr * 0.3),
        (mid - 0.12, mr * 0.3),
        (mid - 0.12, -mr * 0.1),
    ]);
    t.rect(b.1 - 1.0, mr / 2.0, b.1 - 0.1, mr / 2.0 + 0.5, Medium);
    t.line(&[(l.f0, -14.0), (l.f0, 14.0)], Beyond);
    t.line(&[(l.f1, -14.0), (l.f1, 14.0)], Beyond);
    t.brk((a.0 - 1.0, 14.0), (a.1 + 1.0, 14.0));
    t.brk((b.0 - 1.0, -14.0), (b.1 + 1.0, -14.0));
    let upper = match door.op {
        Op::Window(Win::Hung { double: false }) => "FIXED UPPER SASH",
        _ => "UPPER SASH",
    };
    let mn = if door.meet_note.is_empty() {
        format!("INTERLOCKED MEETING RAILS, {} EACH", fr(mr))
    } else {
        door.meet_note.to_owned()
    };
    t.note((mid, 0.0), &mn);
    t.note(((a.0 + a.1) / 2.0, 7.0), upper);
    t.note(((b.0 + b.1) / 2.0, -7.0), "LOWER SASH");
    t.note((b.1 - 0.5, mr / 2.0 + 0.5), "SASH LOCK");
    t.note((l.f1, 10.0), "JAMB BEYOND");
    if door.glass > 0.0 {
        let note = if door.glass_note.is_empty() {
            format!("{} INSULATING GLASS", fr(door.glass))
        } else {
            door.glass_note.to_owned()
        };
        t.note(((b.0 + b.1) / 2.0, -11.0), &note);
    }
}
