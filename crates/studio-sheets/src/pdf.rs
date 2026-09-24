//! Vector PDF export of sheets at true scale: paper mm map 1:1 to PDF points × 72/25.4, pen
//! weights are real line widths, and Barlow Condensed is embedded (subset) for all text.

use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Transform};
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, LineCap, LineJoin, Stroke, StrokeDash};
use krilla::text::{Font, TextDirection};
use krilla::Document as Pdf;
use studio_core::{Document, ElementData, ElementId};
use studio_views::{Anchor, Dash, DisplayList, FillKind, Prim};

use crate::sheet::sheet_display_list;

static FONT: &[u8] = include_bytes!("../fonts/BarlowCondensed-SemiBold.ttf");

/// PDF points per paper millimetre.
pub const PT_PER_MM: f64 = 72.0 / 25.4;

/// Pen weights 1–6 as printed line widths in mm.
pub const PEN_MM: [f64; 7] = [0.0, 0.13, 0.18, 0.25, 0.35, 0.5, 0.7];

#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("{0} is not a sheet")]
    NotASheet(ElementId),
    #[error("the embedded font could not be loaded")]
    Font,
    #[error("PDF could not be written: {0}")]
    Write(String),
}

fn color(fill: FillKind) -> Option<rgb::Color> {
    Some(match fill {
        FillKind::Poche => rgb::Color::new(0x3b, 0x3b, 0x3b),
        FillKind::PocheLight => rgb::Color::new(0xb4, 0xb4, 0xae),
        FillKind::Paper => rgb::Color::new(0xff, 0xff, 0xff),
        FillKind::Slab => rgb::Color::new(0xef, 0xef, 0xeb),
        FillKind::Ceiling => rgb::Color::new(0xf2, 0xfb, 0xfe),
        FillKind::Ink => rgb::Color::new(0x0a, 0x0a, 0x0a),
        FillKind::Glass => rgb::Color::new(0xdf, 0xf5, 0xfd),
        FillKind::Accent => rgb::Color::new(0x3e, 0xcf, 0xf7),
        FillKind::Room => return None,
    })
}

/// Advance width of `text` in em units of the embedded font.
fn text_width_em(face: &ttf_parser::Face<'_>, text: &str) -> f64 {
    let upem = f64::from(face.units_per_em());
    text.chars()
        .map(|c| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_hor_advance(g))
                .map_or(0.5, |a| f64::from(a) / upem)
        })
        .sum()
}

/// Writes the given sheets, one page each, to PDF bytes. `date` goes in the title blocks.
pub fn export_pdf(doc: &Document, sheets: &[ElementId], date: &str) -> Result<Vec<u8>, PdfError> {
    let font = Font::new(FONT.to_vec().into(), 0).ok_or(PdfError::Font)?;
    let face = ttf_parser::Face::parse(FONT, 0).map_err(|_| PdfError::Font)?;
    let mut pdf = Pdf::new();
    for &sheet in sheets {
        if !matches!(doc.data(sheet), Ok(ElementData::Sheet { .. })) {
            return Err(PdfError::NotASheet(sheet));
        }
        let dl = sheet_display_list(doc, sheet, date).ok_or(PdfError::NotASheet(sheet))?;
        draw_page(&mut pdf, &dl, &font, &face)?;
    }
    pdf.finish().map_err(|e| PdfError::Write(format!("{e:?}")))
}

fn draw_page(
    pdf: &mut Pdf,
    dl: &DisplayList,
    font: &Font,
    face: &ttf_parser::Face<'_>,
) -> Result<(), PdfError> {
    let [_, _, w, h] = dl.bounds;
    let settings = PageSettings::from_wh((w * PT_PER_MM) as f32, (h * PT_PER_MM) as f32)
        .ok_or_else(|| PdfError::Write("page size".into()))?;
    let mut page = pdf.start_page_with(settings);
    let mut surface = page.surface();
    // Paper mm with y up → PDF points with y down.
    let x = |v: f64| (v * PT_PER_MM) as f32;
    let y = |v: f64| ((h - v) * PT_PER_MM) as f32;
    let ink = rgb::Color::new(0x0a, 0x0a, 0x0a);

    for item in &dl.items {
        match &item.prim {
            Prim::Fill { rings, fill } => {
                let Some(c) = color(*fill) else { continue };
                let mut pb = PathBuilder::new();
                for r in rings {
                    for (i, p) in r.iter().enumerate() {
                        if i == 0 {
                            pb.move_to(x(p[0]), y(p[1]));
                        } else {
                            pb.line_to(x(p[0]), y(p[1]));
                        }
                    }
                    pb.close();
                }
                if let Some(path) = pb.finish() {
                    surface.set_stroke(None);
                    surface.set_fill(Some(Fill {
                        paint: c.into(),
                        rule: FillRule::EvenOdd,
                        ..Default::default()
                    }));
                    surface.draw_path(&path);
                }
            }
            Prim::Line {
                pts,
                closed,
                w: pen,
                dash,
            } => {
                let mut pb = PathBuilder::new();
                for (i, p) in pts.iter().enumerate() {
                    if i == 0 {
                        pb.move_to(x(p[0]), y(p[1]));
                    } else {
                        pb.line_to(x(p[0]), y(p[1]));
                    }
                }
                if *closed {
                    pb.close();
                }
                let Some(path) = pb.finish() else { continue };
                let dash = match dash {
                    Dash::Solid => None,
                    Dash::Dashed => Some(vec![3.0, 1.5]),
                    Dash::Center => Some(vec![6.0, 1.5, 1.0, 1.5]),
                }
                .map(|a: Vec<f64>| StrokeDash {
                    array: a.into_iter().map(|v| (v * PT_PER_MM) as f32).collect(),
                    offset: 0.0,
                });
                surface.set_fill(None);
                surface.set_stroke(Some(Stroke {
                    paint: ink.into(),
                    width: (PEN_MM[usize::from(*pen).min(6)] * PT_PER_MM) as f32,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Miter,
                    dash,
                    ..Default::default()
                }));
                surface.draw_path(&path);
            }
            Prim::Circle {
                c,
                r,
                w: pen,
                filled,
            } => {
                // Four cubic Béziers; k is the standard circle-approximation constant.
                let (cx, cy, r) = (c[0], c[1], *r);
                let k = 0.552_284_75 * r;
                let mut pb = PathBuilder::new();
                pb.move_to(x(cx + r), y(cy));
                pb.cubic_to(x(cx + r), y(cy + k), x(cx + k), y(cy + r), x(cx), y(cy + r));
                pb.cubic_to(x(cx - k), y(cy + r), x(cx - r), y(cy + k), x(cx - r), y(cy));
                pb.cubic_to(x(cx - r), y(cy - k), x(cx - k), y(cy - r), x(cx), y(cy - r));
                pb.cubic_to(x(cx + k), y(cy - r), x(cx + r), y(cy - k), x(cx + r), y(cy));
                pb.close();
                let Some(path) = pb.finish() else { continue };
                let fill = if *filled {
                    ink
                } else {
                    rgb::Color::new(0xff, 0xff, 0xff)
                };
                surface.set_fill(Some(Fill {
                    paint: fill.into(),
                    ..Default::default()
                }));
                surface.set_stroke(Some(Stroke {
                    paint: ink.into(),
                    width: (PEN_MM[usize::from(*pen).min(6)] * PT_PER_MM) as f32,
                    ..Default::default()
                }));
                surface.draw_path(&path);
            }
            Prim::Text {
                at,
                text,
                size,
                anchor,
                angle,
            } => {
                if text.is_empty() {
                    continue;
                }
                let em = size * PT_PER_MM;
                let width = text_width_em(face, text) * em;
                let dx = match anchor {
                    Anchor::Left => 0.0,
                    Anchor::Center => -width / 2.0,
                    Anchor::Right => -width,
                };
                // Anchor points are the text's vertical middle; the baseline sits ~0.35 em below.
                let (ax, ay) = (x(at[0]), y(at[1]));
                surface.push_transform(&Transform::from_rotate_at(
                    (-angle.to_degrees()) as f32,
                    ax,
                    ay,
                ));
                surface.set_stroke(None);
                surface.set_fill(Some(Fill {
                    paint: ink.into(),
                    ..Default::default()
                }));
                surface.draw_text(
                    Point::from_xy(ax + dx as f32, ay + (em * 0.35) as f32),
                    font.clone(),
                    em as f32,
                    text,
                    false,
                    TextDirection::Auto,
                );
                surface.pop();
            }
        }
    }
    surface.finish();
    page.finish();
    Ok(())
}
