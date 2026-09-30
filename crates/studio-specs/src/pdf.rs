//! The book as a vector PDF on US Letter: the laid-out blocks wrapped with the style's
//! embedded fonts (subset), headers and footers, and pages numbered by section as CSI
//! PageFormat does ("09 29 00 - 3").

use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::text::{Font as PdfFont, TextDirection};
use krilla::Document as Pdf;

use crate::layout::{Align, Block, Face, Laid};
use crate::style::Font;

static TINOS: [&[u8]; 3] = [
    include_bytes!("../fonts/Tinos-Regular.ttf"),
    include_bytes!("../fonts/Tinos-Bold.ttf"),
    include_bytes!("../fonts/Tinos-Italic.ttf"),
];
static CARLITO: [&[u8]; 3] = [
    include_bytes!("../fonts/Carlito-Regular.ttf"),
    include_bytes!("../fonts/Carlito-Bold.ttf"),
    include_bytes!("../fonts/Carlito-Italic.ttf"),
];
static BARLOW: [&[u8]; 3] = [
    include_bytes!("../fonts/Barlow-Regular.ttf"),
    include_bytes!("../fonts/Barlow-SemiBold.ttf"),
    include_bytes!("../fonts/Barlow-Italic.ttf"),
];
static DISPLAY: &[u8] = include_bytes!("../../studio-sheets/fonts/BarlowCondensed-SemiBold.ttf");

const PAGE_W: f32 = 612.0;
const PAGE_H: f32 = 792.0;
const TOP: f32 = 80.0;
const BOTTOM: f32 = 72.0;
const LEADING: f32 = 1.22;

struct Face2<'a> {
    pdf: PdfFont,
    face: ttf_parser::Face<'a>,
}

struct Fonts<'a> {
    regular: Face2<'a>,
    bold: Face2<'a>,
    italic: Face2<'a>,
    display: Face2<'a>,
}

impl<'a> Fonts<'a> {
    fn new(font: Font) -> Result<Self, String> {
        let set = match font {
            Font::Serif => &TINOS,
            Font::Sans => &CARLITO,
            Font::Brand => &BARLOW,
        };
        let load = |bytes: &'a [u8]| -> Result<Face2<'a>, String> {
            Ok(Face2 {
                pdf: PdfFont::new(bytes.to_vec().into(), 0).ok_or("a font could not be loaded")?,
                face: ttf_parser::Face::parse(bytes, 0).map_err(|e| e.to_string())?,
            })
        };
        let display = if font == Font::Brand { DISPLAY } else { set[1] };
        Ok(Fonts {
            regular: load(set[0])?,
            bold: load(set[1])?,
            italic: load(set[2])?,
            display: load(display)?,
        })
    }
    fn of(&self, f: Face) -> &Face2<'a> {
        match f {
            Face::Regular => &self.regular,
            Face::Bold => &self.bold,
            Face::Italic => &self.italic,
            Face::Display => &self.display,
        }
    }
}

fn width(f: &Face2<'_>, text: &str, size: f32) -> f32 {
    let upem = f32::from(f.face.units_per_em());
    text.chars()
        .map(|c| {
            f.face
                .glyph_index(c)
                .and_then(|g| f.face.glyph_hor_advance(g))
                .map_or(0.5, |a| f32::from(a) / upem)
        })
        .sum::<f32>()
        * size
}

/// Breaks `text` into lines: the first `first` wide, the rest `rest` wide.
fn wrap(f: &Face2<'_>, text: &str, size: f32, first: f32, rest: f32) -> Vec<String> {
    let space = width(f, " ", size);
    let mut lines = vec![];
    let mut line = String::new();
    let mut line_w = 0.0;
    for word in text.split_whitespace() {
        let max = if lines.is_empty() { first } else { rest };
        let w = width(f, word, size);
        if line.is_empty() {
            line.push_str(word);
            line_w = w;
        } else if line_w + space + w <= max {
            line.push(' ');
            line.push_str(word);
            line_w += space + w;
        } else {
            lines.push(std::mem::take(&mut line));
            line.push_str(word);
            line_w = w;
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

enum Op {
    Text {
        x: f32,
        y: f32,
        face: Face,
        size: f32,
        text: String,
        white: bool,
    },
    Band {
        y: f32,
        h: f32,
    },
    Rule {
        x: f32,
        y: f32,
        w: f32,
    },
    Box {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
}

struct PageOut {
    ops: Vec<Op>,
    section: usize,
    number: u32,
}

/// One block set on lines: the ops relative to its top, and its height.
fn set_block(fonts: &Fonts<'_>, b: &Block, left: f32, width_pt: f32) -> (Vec<Op>, f32) {
    let f = fonts.of(b.face);
    let line_h = b.size * LEADING;
    let mut ops = vec![];
    let label_w = if b.label.is_empty() {
        0.0
    } else {
        width(f, &b.label, b.size)
    };
    let first_x = if b.label.is_empty() {
        b.x
    } else {
        b.x.max(b.label_x + label_w + 5.0)
    };
    let lines = if b.text.is_empty() {
        vec![]
    } else {
        wrap(f, &b.text, b.size, width_pt - first_x, width_pt - b.x)
    };
    let n = lines.len().max(usize::from(!b.label.is_empty()));
    let pad = if b.banner { 5.0 } else { 0.0 };
    let h = n as f32 * line_h + 2.0 * pad;
    if b.banner {
        ops.push(Op::Band { y: 0.0, h });
    }
    if !b.label.is_empty() {
        ops.push(Op::Text {
            x: left + b.label_x,
            y: pad + b.size,
            face: b.face,
            size: b.size,
            text: b.label.clone(),
            white: b.banner,
        });
    }
    for (i, l) in lines.iter().enumerate() {
        let x = match b.align {
            Align::Center => left + (width_pt - width(f, l, b.size)) / 2.0,
            Align::Left => left + if i == 0 { first_x } else { b.x },
        };
        ops.push(Op::Text {
            x,
            y: pad + b.size + i as f32 * line_h,
            face: b.face,
            size: b.size,
            text: l.clone(),
            white: b.banner,
        });
    }
    let mut total = h;
    if b.rule {
        ops.push(Op::Rule {
            x: left,
            y: h + 2.0,
            w: width_pt,
        });
        total += 4.0;
    }
    if b.boxed > 0.0 {
        ops.push(Op::Box {
            x: left + b.x,
            y: total + 2.0,
            w: 190.0,
            h: b.boxed,
        });
        total += b.boxed + 4.0;
    }
    (ops, total)
}

fn shift(ops: Vec<Op>, dy: f32) -> impl Iterator<Item = Op> {
    ops.into_iter().map(move |o| match o {
        Op::Text {
            x,
            y,
            face,
            size,
            text,
            white,
        } => Op::Text {
            x,
            y: y + dy,
            face,
            size,
            text,
            white,
        },
        Op::Band { y, h } => Op::Band { y: y + dy, h },
        Op::Rule { x, y, w } => Op::Rule { x, y: y + dy, w },
        Op::Box { x, y, w, h } => Op::Box { x, y: y + dy, w, h },
    })
}

/// Paginates the blocks.
fn paginate(fonts: &Fonts<'_>, laid: &Laid, margin: f32) -> Vec<PageOut> {
    let left = margin;
    let w = PAGE_W - 2.0 * margin;
    let bottom = PAGE_H - BOTTOM;
    let mut pages: Vec<PageOut> = vec![];
    let mut y = TOP;
    let mut number = 0;
    let new_page = |pages: &mut Vec<PageOut>, section: usize, number: &mut u32| {
        let restart = laid.per_section && pages.last().is_none_or(|p| p.section != section);
        *number = if restart { 1 } else { *number + 1 };
        pages.push(PageOut {
            ops: vec![],
            section,
            number: *number,
        });
    };
    for (i, b) in laid.blocks.iter().enumerate() {
        if pages.is_empty() || b.new_page {
            new_page(&mut pages, b.section, &mut number);
            y = TOP;
        }
        let (ops, h) = set_block(fonts, b, left, w);
        let before = if y <= TOP + 0.1 { 0.0 } else { b.before };
        // Headings keep with the next block's first line.
        let need = if b.keep_next {
            laid.blocks
                .get(i + 1)
                .map_or(0.0, |n| n.before + n.size * LEADING * 2.0)
        } else {
            0.0
        };
        if y + before + h + need > bottom && y > TOP + 0.1 {
            new_page(&mut pages, b.section, &mut number);
            y = TOP;
        } else {
            y += before;
        }
        let page = pages.last_mut().map(|p| &mut p.ops);
        if let Some(p) = page {
            p.extend(shift(ops, y));
        }
        y += h + b.after;
    }
    pages
}

/// Writes the book's PDF.
pub fn export(laid: &Laid, font: Font, margin_in: f32) -> Result<Vec<u8>, String> {
    let fonts = Fonts::new(font)?;
    let margin = margin_in * 72.0;
    let pages = paginate(&fonts, laid, margin);
    let ink = rgb::Color::new(0x0a, 0x0a, 0x0a);
    let white = rgb::Color::new(0xff, 0xff, 0xff);
    let grey = rgb::Color::new(0x55, 0x55, 0x55);
    let mut pdf = Pdf::new();
    for p in &pages {
        let settings = PageSettings::from_wh(PAGE_W, PAGE_H).ok_or("page size")?;
        let mut page = pdf.start_page_with(settings);
        let mut s = page.surface();
        let text = |s: &mut krilla::surface::Surface<'_>,
                    x: f32,
                    y: f32,
                    face: Face,
                    size: f32,
                    t: &str,
                    c: rgb::Color| {
            s.set_stroke(None);
            s.set_fill(Some(Fill {
                paint: c.into(),
                ..Default::default()
            }));
            s.draw_text(
                Point::from_xy(x, y),
                fonts.of(face).pdf.clone(),
                size,
                t,
                false,
                TextDirection::Auto,
            );
        };
        let rect = |x: f32, y: f32, w: f32, h: f32| {
            let mut pb = PathBuilder::new();
            pb.move_to(x, y);
            pb.line_to(x + w, y);
            pb.line_to(x + w, y + h);
            pb.line_to(x, y + h);
            pb.close();
            pb.finish()
        };
        for op in &p.ops {
            match op {
                Op::Text {
                    x,
                    y,
                    face,
                    size,
                    text: t,
                    white: w,
                } => text(
                    &mut s,
                    *x,
                    *y,
                    *face,
                    *size,
                    t,
                    if *w { white } else { ink },
                ),
                Op::Band { y, h } => {
                    if let Some(path) = rect(margin - 6.0, *y, PAGE_W - 2.0 * margin + 12.0, *h) {
                        s.set_stroke(None);
                        s.set_fill(Some(Fill {
                            paint: ink.into(),
                            ..Default::default()
                        }));
                        s.draw_path(&path);
                    }
                }
                Op::Rule { x, y, w } => {
                    let mut pb = PathBuilder::new();
                    pb.move_to(*x, *y);
                    pb.line_to(x + w, *y);
                    if let Some(path) = pb.finish() {
                        s.set_fill(None);
                        s.set_stroke(Some(Stroke {
                            paint: ink.into(),
                            width: 0.75,
                            ..Default::default()
                        }));
                        s.draw_path(&path);
                    }
                }
                Op::Box { x, y, w, h } => {
                    if let Some(path) = rect(*x, *y, *w, *h) {
                        s.set_fill(None);
                        s.set_stroke(Some(Stroke {
                            paint: grey.into(),
                            width: 0.6,
                            ..Default::default()
                        }));
                        s.draw_path(&path);
                    }
                }
            }
        }
        // Header and footer.
        let small = 8.0;
        let right = |s: &mut krilla::surface::Surface<'_>, t: &str, y: f32| {
            let w = width(&fonts.regular, t, small);
            text(s, PAGE_W - margin - w, y, Face::Regular, small, t, grey);
        };
        text(
            &mut s,
            margin,
            44.0,
            Face::Regular,
            small,
            &laid.header_left,
            grey,
        );
        right(&mut s, &laid.header_right, 44.0);
        let run = laid.sections.get(p.section);
        let (title, num) = match run {
            Some(r) if laid.per_section && !r.number.is_empty() => {
                (r.title.clone(), format!("{} - {}", r.number, p.number))
            }
            _ => (laid.header_left.clone(), p.number.to_string()),
        };
        text(
            &mut s,
            margin,
            PAGE_H - 40.0,
            Face::Regular,
            small,
            &title,
            grey,
        );
        right(&mut s, &num, PAGE_H - 40.0);
        s.finish();
        page.finish();
    }
    pdf.finish()
        .map_err(|e| format!("the PDF could not be written: {e:?}"))
}

/// How many pages the book runs to (for the status bar).
pub fn page_count(laid: &Laid, font: Font, margin_in: f32) -> usize {
    Fonts::new(font).map_or(0, |f| paginate(&f, laid, margin_in * 72.0).len())
}

/// The pages as SVG (the PDF's own layout, with the fonts named for the browser), for
/// looking at pages without a PDF viewer.
pub fn svg_pages(laid: &Laid, font: Font, margin_in: f32, which: &[usize]) -> Vec<String> {
    let Ok(fonts) = Fonts::new(font) else {
        return vec![];
    };
    let margin = margin_in * 72.0;
    let family = match font {
        Font::Serif => "Times New Roman, Tinos, serif",
        Font::Sans => "Calibri, Carlito, sans-serif",
        Font::Brand => "Barlow, sans-serif",
    };
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;");
    let pages = paginate(&fonts, laid, margin);
    which
        .iter()
        .filter_map(|&i| pages.get(i))
        .map(|p| {
            let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="612" height="792" viewBox="0 0 612 792" style="background:white;font-family:{family}"><rect width="612" height="792" fill="white"/>"#);
            for op in &p.ops {
                match op {
                    Op::Text { x, y, face, size, text, white } => {
                        let (w, st, fam) = match face {
                            Face::Bold => ("700", "normal", family),
                            Face::Italic => ("400", "italic", family),
                            Face::Display => ("600", "normal", if font == Font::Brand { "Barlow Condensed, sans-serif" } else { family }),
                            Face::Regular => ("400", "normal", family),
                        };
                        s.push_str(&format!(r#"<text x="{x:.1}" y="{y:.1}" font-size="{size}" font-weight="{w}" font-style="{st}" font-family="{fam}" fill="{}">{}</text>"#, if *white { "white" } else { "rgb(10,10,10)" }, esc(text)));
                    }
                    Op::Band { y, h } => s.push_str(&format!(r#"<rect x="{:.1}" y="{y:.1}" width="{:.1}" height="{h:.1}" fill="rgb(10,10,10)"/>"#, margin - 6.0, 612.0 - 2.0 * margin + 12.0)),
                    Op::Rule { x, y, w } => s.push_str(&format!(r#"<line x1="{x}" y1="{y}" x2="{}" y2="{y}" stroke="rgb(10,10,10)" stroke-width="0.75"/>"#, x + w)),
                    Op::Box { x, y, w, h } => s.push_str(&format!(r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="none" stroke="rgb(85,85,85)" stroke-width="0.6"/>"#)),
                }
            }
            let run = laid.sections.get(p.section);
            let (title, num) = match run {
                Some(r) if laid.per_section && !r.number.is_empty() => (r.title.clone(), format!("{} - {}", r.number, p.number)),
                _ => (laid.header_left.clone(), p.number.to_string()),
            };
            s.push_str(&format!(r#"<g font-size="8" fill="rgb(85,85,85)"><text x="{margin}" y="44">{}</text><text x="{r}" y="44" text-anchor="end">{}</text><text x="{margin}" y="752">{}</text><text x="{r}" y="752" text-anchor="end">{}</text></g></svg>"#, esc(&laid.header_left), esc(&laid.header_right), esc(&title), esc(&num), r = 612.0 - margin));
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_wrap_to_the_measure() {
        let f = Fonts::new(Font::Serif).unwrap();
        let text = "Install gypsum board to comply with ASTM C840 and manufacturer's written instructions.";
        let lines = wrap(&f.regular, text, 10.0, 150.0, 200.0);
        assert!(lines.len() >= 2);
        assert!(width(&f.regular, &lines[0], 10.0) <= 150.0);
        assert_eq!(lines.join(" "), text);
        // A word longer than the measure still goes on a line of its own.
        assert_eq!(
            wrap(&f.regular, "Supercalifragilistic", 10.0, 5.0, 5.0).len(),
            1
        );
    }
}
