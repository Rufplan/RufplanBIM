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
// The schedules' other fonts (ADR-110), the OFL ones the project manual bundles.
static SANS: &[u8] = include_bytes!("../../studio-specs/fonts/Carlito-Regular.ttf");
static SERIF: &[u8] = include_bytes!("../../studio-specs/fonts/Tinos-Regular.ttf");

/// The embedded fonts and their metrics, by [`TextFont`](studio_core::text::TextFont).
struct Fonts {
    list: Vec<(Font, ttf_parser::Face<'static>)>,
}

impl Fonts {
    fn load() -> Result<Fonts, PdfError> {
        let mut list = vec![];
        for bytes in [FONT, SANS, SERIF] {
            let font = Font::new(bytes.to_vec().into(), 0).ok_or(PdfError::Font)?;
            let face = ttf_parser::Face::parse(bytes, 0).map_err(|_| PdfError::Font)?;
            list.push((font, face));
        }
        Ok(Fonts { list })
    }
    fn get(&self, f: Option<studio_core::text::TextFont>) -> &(Font, ttf_parser::Face<'static>) {
        use studio_core::text::TextFont;
        let i = match f.unwrap_or_default() {
            TextFont::Drafting => 0,
            TextFont::Sans => 1,
            TextFont::Serif => 2,
        };
        &self.list[i]
    }
}

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
        FillKind::Ceiling => rgb::Color::new(0xff, 0xff, 0xff),
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
    export_pdf_with(doc, sheets, date, &Maps::new())
}

/// Map images by map frame (ADR-107): fetched by the caller just before printing (never
/// stored); a frame without one prints as a gray box.
pub type Maps = std::collections::HashMap<ElementId, Vec<u8>>;

/// Like [`export_pdf`], with the sheets' map images.
pub fn export_pdf_with(
    doc: &Document,
    sheets: &[ElementId],
    date: &str,
    maps: &Maps,
) -> Result<Vec<u8>, PdfError> {
    let fonts = Fonts::load()?;
    let mut pdf = Pdf::new();
    for &sheet in sheets {
        if !matches!(doc.data(sheet), Ok(ElementData::Sheet { .. })) {
            return Err(PdfError::NotASheet(sheet));
        }
        let dl = sheet_display_list(doc, sheet, date).ok_or(PdfError::NotASheet(sheet))?;
        draw_page(&mut pdf, doc, &dl, &fonts, maps)?;
    }
    pdf.finish().map_err(|e| PdfError::Write(format!("{e:?}")))
}

fn draw_page(
    pdf: &mut Pdf,
    doc: &Document,
    dl: &DisplayList,
    fonts: &Fonts,
    maps: &Maps,
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
            // A saved rendering (ADR-095), embedded as it was rendered.
            Prim::Image { image, min, max } => {
                use base64::Engine;
                let (bytes, png) = match doc.data(*image) {
                    Ok(ElementData::RenderImage { mime, data, .. }) => {
                        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data)
                        else {
                            continue;
                        };
                        (bytes, mime == "image/png")
                    }
                    Ok(ElementData::MapFrame { .. }) => match maps.get(image) {
                        Some(b) => (b.clone(), b.starts_with(&[0x89, b'P', b'N', b'G'])),
                        None => {
                            // No imagery (no key, or offline): a gray box where it goes.
                            let mut pb = PathBuilder::new();
                            pb.move_to(x(min[0]), y(min[1]));
                            pb.line_to(x(max[0]), y(min[1]));
                            pb.line_to(x(max[0]), y(max[1]));
                            pb.line_to(x(min[0]), y(max[1]));
                            pb.close();
                            if let Some(path) = pb.finish() {
                                surface.set_stroke(None);
                                surface.set_fill(Some(Fill {
                                    paint: rgb::Color::new(0xe8, 0xe6, 0xe1).into(),
                                    ..Default::default()
                                }));
                                surface.draw_path(&path);
                            }
                            continue;
                        }
                    },
                    _ => continue,
                };
                let img = if png {
                    krilla::image::Image::from_png(bytes.into(), true)
                } else {
                    krilla::image::Image::from_jpeg(bytes.into(), true)
                };
                let Ok(img) = img else { continue };
                let (w, h) = (
                    ((max[0] - min[0]) * PT_PER_MM) as f32,
                    ((max[1] - min[1]) * PT_PER_MM) as f32,
                );
                let Some(size) = krilla::geom::Size::from_wh(w, h) else {
                    continue;
                };
                surface.push_transform(&Transform::from_translate(x(min[0]), y(max[1])));
                surface.draw_image(img, size);
                surface.pop();
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
                font,
            } => {
                if text.is_empty() {
                    continue;
                }
                let (font, face) = fonts.get(*font);
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
