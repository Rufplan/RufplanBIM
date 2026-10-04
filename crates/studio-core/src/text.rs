//! Revit's text notes (ADR-070): leaders (one segment, two segments or curved, with a
//! filled arrowhead), paragraph alignment, a wrap width, and the box a note's lines fill.
//!
//! A note's `at` is where its first line sits: its left end, middle or right end (by the
//! alignment), at the line's vertical middle. Lengths here are model mm.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

/// Revit's leader shapes: one segment, two segments (with an elbow) or an arc.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Leader {
    /// The arrowhead's point.
    pub end: Pt,
    /// Two segments: the elbow between the text and the arrowhead.
    #[serde(default)]
    pub elbow: Option<Pt>,
    /// Curved (Revit's arc leader).
    #[serde(default)]
    pub arc: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// The office's text types (ADR-109): plotted heights for a full-size (24x36 or 30x42) set,
/// paper mm. 3/32" for anything read, 1/8" for labels scanned, 3/16"–1/4" for titles
/// navigated by; 3/32" is the floor, so the set reads at half size on 11x17.
pub mod sizes {
    const IN: f64 = 25.4;
    /// 3/32": the smallest text a set prints.
    pub const MIN: f64 = 3.0 / 32.0 * IN;
    /// 1/8".
    pub const EIGHTH: f64 = IN / 8.0;
    /// 3/16".
    pub const THREE_SIXTEENTHS: f64 = 3.0 / 16.0 * IN;

    // Title block.
    /// Project name on the cover: 3/4".
    pub const COVER_TITLE: f64 = 0.75 * IN;
    /// Project name in the title block: 1/4".
    pub const TB_PROJECT: f64 = IN / 4.0;
    /// Sheet number: 1/2".
    pub const SHEET_NUMBER: f64 = IN / 2.0;
    /// Sheet title: 3/16".
    pub const SHEET_TITLE: f64 = THREE_SIXTEENTHS;
    /// Firm info, dates, project number: 3/32".
    pub const TB_INFO: f64 = MIN;

    // Drawing titles.
    /// View title (FLOOR PLAN – LEVEL 1): 3/16".
    pub const VIEW_TITLE: f64 = THREE_SIXTEENTHS;
    /// View number in the title bubble: 1/8".
    pub const VIEW_NUMBER: f64 = EIGHTH;
    /// Scale under the view title: 3/32".
    pub const SCALE: f64 = MIN;

    // Plan annotation.
    /// Room names: 1/8".
    pub const ROOM_NAME: f64 = EIGHTH;
    /// Room numbers (and areas): 3/32".
    pub const ROOM_NUMBER: f64 = MIN;
    /// Dimensions: 3/32".
    pub const DIMENSION: f64 = MIN;
    /// Notes, leaders, keynote text: 3/32".
    pub const NOTE: f64 = MIN;
    /// Note and legend headers: 1/8".
    pub const NOTE_HEADER: f64 = EIGHTH;
    /// Door and window tags: 3/32".
    pub const TAG: f64 = MIN;
    /// Grid bubble text: 3/16", in a 1/2" bubble.
    pub const GRID: f64 = THREE_SIXTEENTHS;
    pub const GRID_BUBBLE: f64 = IN / 2.0;
    /// Section, elevation and detail callouts: 3/32".
    pub const CALLOUT: f64 = MIN;
    /// Levels and spot elevations: 3/32".
    pub const LEVEL: f64 = MIN;
    /// Match lines: 1/8".
    pub const MATCH_LINE: f64 = EIGHTH;
    /// The north arrow's "N": 1/8".
    pub const NORTH: f64 = EIGHTH;
    /// Revision tags: 3/32".
    pub const REVISION: f64 = MIN;

    // Schedules.
    /// Schedule title: 3/16".
    pub const SCHEDULE_TITLE: f64 = THREE_SIXTEENTHS;
    /// Column headers: 1/8".
    pub const SCHEDULE_HEADER: f64 = EIGHTH;
    /// Body: 3/32".
    pub const SCHEDULE_BODY: f64 = MIN;

    // Notes sheets.
    /// Section headers (GENERAL NOTES): 3/16".
    pub const NOTES_HEADER: f64 = THREE_SIXTEENTHS;
    /// Subheaders: 1/8".
    pub const NOTES_SUBHEADER: f64 = EIGHTH;
    /// Body: 3/32".
    pub const NOTES_BODY: f64 = MIN;
}
/// A character's advance, as a share of the text height (the drafting font is condensed).
pub const CHAR_W: f64 = 0.5;
/// Line spacing, as a multiple of the text height.
pub const LINE: f64 = 1.6;

/// Where a note's lines go.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    pub lines: Vec<String>,
    /// Each line's anchor point (left end, middle or right end by the alignment).
    pub line_at: Vec<Pt>,
    pub min: Pt,
    pub max: Pt,
    /// Text height (model mm).
    pub height: f64,
}

pub fn text_width(s: &str, height: f64) -> f64 {
    s.chars().count() as f64 * CHAR_W * height
}

/// Splits `text` into lines: at its line breaks, and at word breaks to fit `width`.
pub fn wrap(text: &str, height: f64, width: Option<f64>) -> Vec<String> {
    let mut out = vec![];
    for para in text.split('\n') {
        let Some(w) = width.filter(|w| *w > height) else {
            out.push(para.to_owned());
            continue;
        };
        let mut line = String::new();
        for word in para.split(' ') {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && text_width(&candidate, height) > w {
                out.push(std::mem::take(&mut line));
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        out.push(line);
    }
    out
}

/// The note's box and line positions, its first line at `at`.
pub fn layout(at: Pt, text: &str, height: f64, width: Option<f64>, align: TextAlign) -> TextBox {
    let lines = wrap(text, height, width);
    let widest = lines
        .iter()
        .map(|l| text_width(l, height))
        .fold(0.0, f64::max);
    let w = width.unwrap_or(widest).max(height);
    let (x0, x1) = match align {
        TextAlign::Left => (at.x, at.x + w),
        TextAlign::Center => (at.x - w / 2.0, at.x + w / 2.0),
        TextAlign::Right => (at.x - w, at.x),
    };
    let step = height * LINE;
    let line_at: Vec<Pt> = (0..lines.len())
        .map(|i| Pt::new(at.x, at.y - i as f64 * step))
        .collect();
    let n = lines.len().max(1) as f64;
    TextBox {
        lines,
        line_at,
        min: Pt::new(x0, at.y - (n - 1.0) * step - height * 0.8),
        max: Pt::new(x1, at.y + height * 0.8),
        height,
    }
}

/// Where a leader leaves the text: the side nearer its arrowhead, at the first line.
pub fn attach(tb: &TextBox, toward: Pt) -> Pt {
    let gap = tb.height * 0.4;
    let mid = (tb.min.x + tb.max.x) / 2.0;
    let y = tb.max.y - tb.height * 0.8;
    if toward.x < mid {
        Pt::new(tb.min.x - gap, y)
    } else {
        Pt::new(tb.max.x + gap, y)
    }
}

/// A leader's polyline from the text to its arrowhead.
pub fn leader_points(tb: &TextBox, l: &Leader) -> Vec<Pt> {
    let first = l.elbow.unwrap_or(l.end);
    let a = attach(tb, first);
    if l.arc {
        // A quadratic curve leaving the text level and arriving at the arrowhead.
        let c = l.elbow.unwrap_or(Pt::new(l.end.x, a.y));
        return (0..=16)
            .map(|i| {
                let t = i as f64 / 16.0;
                let u = 1.0 - t;
                a.scale(u * u)
                    .add(c.scale(2.0 * u * t))
                    .add(l.end.scale(t * t))
            })
            .collect();
    }
    match l.elbow {
        Some(e) => vec![a, e, l.end],
        None => vec![a, l.end],
    }
}

/// `p` turned by `angle` (radians, counter-clockwise) about `about` (ADR-108).
pub fn turn(p: Pt, about: Pt, angle: f64) -> Pt {
    if angle == 0.0 {
        return p;
    }
    let (s, c) = angle.sin_cos();
    let d = p.sub(about);
    about.add(Pt::new(d.x * c - d.y * s, d.x * s + d.y * c))
}

/// A leader's polyline for a note turned by `angle` about `at`: worked out in the note's
/// own frame, so it leaves the text's side as it would unturned.
pub fn leader_points_turned(tb: &TextBox, l: &Leader, at: Pt, angle: f64) -> Vec<Pt> {
    if angle == 0.0 {
        return leader_points(tb, l);
    }
    let local = Leader {
        end: turn(l.end, at, -angle),
        elbow: l.elbow.map(|e| turn(e, at, -angle)),
        arc: l.arc,
    };
    leader_points(tb, &local)
        .into_iter()
        .map(|p| turn(p, at, angle))
        .collect()
}

/// The note's box in the model, turned with it: bottom left, bottom right, top right, top
/// left.
pub fn box_corners(tb: &TextBox, at: Pt, angle: f64) -> [Pt; 4] {
    [
        tb.min,
        Pt::new(tb.max.x, tb.min.y),
        tb.max,
        Pt::new(tb.min.x, tb.max.y),
    ]
    .map(|p| turn(p, at, angle))
}

/// How far outside the box a selected note's move and rotate grips sit, paper mm (ADR-108).
pub const GRIP_OFFSET: f64 = 3.0;

/// A selected note's box grips (ADR-108), model mm, turned with it: the move grip off the
/// box's top left corner, the rotate grip off its top right, and the width grips at the
/// middle of its left and right sides. `scale`: the view's.
pub fn frame_grips(tb: &TextBox, at: Pt, angle: f64, scale: f64) -> [(&'static str, Pt); 4] {
    let off = GRIP_OFFSET * scale;
    let mid = (tb.min.y + tb.max.y) / 2.0;
    [
        ("text_move_grip", Pt::new(tb.min.x - off, tb.max.y + off)),
        ("text_rotate", Pt::new(tb.max.x + off, tb.max.y + off)),
        ("text_width_left", Pt::new(tb.min.x, mid)),
        ("text_width", Pt::new(tb.max.x, mid)),
    ]
    .map(|(k, p)| (k, turn(p, at, angle)))
}
/// Revit's "Arrow Filled 30 Degree": a solid triangle at the leader's end, `len` long.
pub fn arrowhead(pts: &[Pt], len: f64) -> Option<[Pt; 3]> {
    let n = pts.len();
    if n < 2 {
        return None;
    }
    let (from, tip) = (pts[n - 2], pts[n - 1]);
    if from.dist(tip) < 1e-6 {
        return None;
    }
    let d = tip.sub(from).norm();
    let base = tip.sub(d.scale(len));
    let half = len * (15f64.to_radians()).tan();
    let n = d.perp();
    Some([tip, base.add(n.scale(half)), base.sub(n.scale(half))])
}

/// A new leader's default arrowhead for Add Leader, off the given side of the text.
pub fn default_leader(tb: &TextBox, left: bool) -> Leader {
    let h = tb.height;
    let y = tb.max.y - h * 0.8;
    let end = if left {
        Pt::new(tb.min.x - h * 6.0, y - h * 3.0)
    } else {
        Pt::new(tb.max.x + h * 6.0, y - h * 3.0)
    };
    Leader {
        end,
        elbow: None,
        arc: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_wraps_to_its_width_and_leaders_leave_the_near_side() {
        // 10 mm text, 0.5 advance: "TYPICAL" is 35 mm wide.
        assert_eq!(text_width("TYPICAL", 10.0), 35.0);
        // "1/2\" GYP. BD. ON" is 16 characters: exactly 80 mm, so it fits.
        let lines = wrap("1/2\" GYP. BD. ON 2x6 STUDS", 10.0, Some(80.0));
        assert_eq!(lines, vec!["1/2\" GYP. BD. ON", "2x6 STUDS"]);
        assert_eq!(
            wrap("ONE TWO THREE", 10.0, Some(30.0)),
            vec!["ONE", "TWO", "THREE"]
        );
        assert_eq!(wrap("A\nB C", 10.0, None), vec!["A", "B C"]);
        let tb = layout(
            Pt::new(0.0, 0.0),
            "LINE ONE\nTWO",
            10.0,
            None,
            TextAlign::Left,
        );
        assert_eq!(tb.lines.len(), 2);
        assert_eq!(tb.line_at[1], Pt::new(0.0, -16.0));
        assert!((tb.max.x - 40.0).abs() < 1e-9 && (tb.min.y + 24.0).abs() < 1e-9);
        // Right-aligned: the box ends at `at`.
        let r = layout(Pt::new(100.0, 0.0), "ABCD", 10.0, None, TextAlign::Right);
        assert_eq!((r.min.x, r.max.x), (80.0, 100.0));
        // A leader to the left leaves the left side at the first line; to the right, the right.
        let l = Leader {
            end: Pt::new(-100.0, -50.0),
            elbow: None,
            arc: false,
        };
        assert_eq!(leader_points(&tb, &l)[0], Pt::new(-4.0, 0.0));
        let r_end = Leader {
            end: Pt::new(200.0, 0.0),
            ..l
        };
        assert_eq!(leader_points(&tb, &r_end)[0], Pt::new(44.0, 0.0));
        // Two segments go through the elbow; an arc is smooth and ends at the arrowhead.
        let two = Leader {
            elbow: Some(Pt::new(-30.0, 0.0)),
            ..l
        };
        assert_eq!(leader_points(&tb, &two).len(), 3);
        let arc = Leader { arc: true, ..l };
        let pts = leader_points(&tb, &arc);
        assert_eq!(pts.len(), 17);
        assert_eq!(*pts.last().unwrap(), l.end);
        // The arrowhead points along the last segment, 30° wide.
        let a = arrowhead(&[Pt::new(0.0, 0.0), Pt::new(10.0, 0.0)], 3.0).unwrap();
        assert_eq!(a[0], Pt::new(10.0, 0.0));
        let half = 3.0 * 15f64.to_radians().tan();
        assert!((a[1].x - 7.0).abs() < 1e-9);
        assert!(((a[1].y - a[2].y).abs() - 2.0 * half).abs() < 1e-9);
    }
}
