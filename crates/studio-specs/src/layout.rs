//! The book as blocks: numbered, indented, styled paragraphs in reading order, with where
//! sections start (for per-section page numbers). The PDF and Word writers both set these,
//! so the two match.

use studio_core::specs::{Paragraph, SectionKind, SpecBook, SpecSection};

use crate::generate::{responsible_for, Front};
use crate::library::{division_title, subgroup};
use crate::style::{article_label, label, part_label, Content, Heading, Numbering, SpecStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Regular,
    Bold,
    Italic,
    /// The condensed display face (Rufplan) or bold.
    Display,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
}

/// A Word list level for a numbered paragraph: which list (one per section, so numbering
/// restarts) and its level (0 part, 1 article, 2–5 paragraphs A. 1. a. 1)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListRef {
    pub list: u32,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub label: String,
    pub text: String,
    /// Where the label and the text start, pt from the left margin.
    pub label_x: f32,
    pub x: f32,
    pub size: f32,
    pub face: Face,
    pub align: Align,
    pub before: f32,
    pub after: f32,
    /// Keep on the page with the next block (headings).
    pub keep_next: bool,
    pub new_page: bool,
    /// White text on a black band.
    pub banner: bool,
    /// A rule under the block.
    pub rule: bool,
    /// An empty box this tall (pt) under the text: a seal.
    pub boxed: f32,
    pub list: Option<ListRef>,
    /// Index into `Laid::sections`: whose footer this page shows.
    pub section: usize,
}

impl Block {
    fn new(text: impl Into<String>, size: f32, section: usize) -> Self {
        Block {
            label: String::new(),
            text: text.into(),
            label_x: 0.0,
            x: 0.0,
            size,
            face: Face::Regular,
            align: Align::Left,
            before: 0.0,
            after: 0.0,
            keep_next: false,
            new_page: false,
            banner: false,
            rule: false,
            boxed: 0.0,
            list: None,
            section,
        }
    }
    fn face(mut self, f: Face) -> Self {
        self.face = f;
        self
    }
    fn center(mut self) -> Self {
        self.align = Align::Center;
        self
    }
    fn space(mut self, before: f32, after: f32) -> Self {
        self.before = before;
        self.after = after;
        self
    }
    fn at(mut self, label: impl Into<String>, label_x: f32, x: f32) -> Self {
        self.label = label.into();
        self.label_x = label_x;
        self.x = x;
        self
    }
    fn keep(mut self) -> Self {
        self.keep_next = true;
        self
    }
}

/// Whose pages these are: number and title (empty number: the book's run of pages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub number: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Laid {
    pub blocks: Vec<Block>,
    pub sections: Vec<Run>,
    pub header_left: String,
    pub header_right: String,
    /// Per-section page numbers ("09 29 00 - 3"), else one run.
    pub per_section: bool,
}

/// Title case for contents lines: "Gypsum Board", "Thermoplastic-Polyolefin (TPO) Roofing".
pub fn title_case(s: &str) -> String {
    const SMALL: &[&str] = &[
        "and", "or", "of", "the", "for", "to", "in", "a", "an", "with", "by", "on", "at",
    ];
    const KEEP: &[&str] = &[
        "HVAC", "LED", "TPO", "PVC", "EPDM", "CMU", "EIFS", "AV", "MEP", "II", "III",
    ];
    s.split(' ')
        .enumerate()
        .map(|(i, w)| {
            let bare = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if KEEP.contains(&bare) {
                return w.to_string();
            }
            w.split('-')
                .enumerate()
                .map(|(k, part)| {
                    let lower = part.to_lowercase();
                    if (i > 0 || k > 0) && SMALL.contains(&lower.as_str()) {
                        lower
                    } else {
                        let mut c = lower.chars();
                        match c.next() {
                            Some('(') => {
                                let rest: String = c.collect();
                                let mut r = rest.chars();
                                match r.next() {
                                    Some(g) => format!("({}{}", g.to_uppercase(), r.as_str()),
                                    None => "(".into(),
                                }
                            }
                            Some(f) => format!("{}{}", f.to_uppercase(), c.as_str()),
                            None => String::new(),
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("-")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A long date: "2026-10-01" → "October 1, 2026" (other text as it is).
pub fn long_date(iso: &str) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let p: Vec<&str> = iso.split('-').collect();
    match (
        p.first(),
        p.get(1).and_then(|m| m.parse::<usize>().ok()),
        p.get(2).and_then(|d| d.parse::<u32>().ok()),
    ) {
        (Some(y), Some(m), Some(d)) if p.len() == 3 && (1..=12).contains(&m) => {
            format!("{} {d}, {y}", MONTHS[m - 1])
        }
        _ => iso.to_string(),
    }
}

/// Indents (label, text) in pt for article and paragraph levels.
pub(crate) fn indents(n: Numbering, level: Option<u8>) -> (f32, f32) {
    match (n, level) {
        (Numbering::Bullets, None) => (0.0, 0.0),
        (Numbering::Bullets, Some(l)) => (14.0 * l as f32, 14.0 * (l as f32 + 1.0)),
        (Numbering::Decimal, None) => (0.0, 40.0),
        (Numbering::Decimal, Some(l)) => {
            let at = [40.0, 94.0, 154.0, 220.0];
            let to = [94.0, 154.0, 220.0, 290.0];
            (at[l.min(3) as usize], to[l.min(3) as usize])
        }
        (_, None) => (0.0, 36.0),
        (_, Some(l)) => (36.0 * (l as f32 + 1.0), 36.0 * (l as f32 + 2.0)),
    }
}

struct Builder<'a> {
    style: &'a SpecStyle,
    out: Vec<Block>,
    sections: Vec<Run>,
    lists: u32,
    section: usize,
}

impl Builder<'_> {
    fn push(&mut self, b: Block) {
        self.out.push(b);
    }
    fn begin(&mut self, number: &str, title: &str, new_page: bool) {
        if self.style.page_per_section {
            self.sections.push(Run {
                number: number.into(),
                title: title_case(title),
            });
            self.section = self.sections.len() - 1;
        }
        let st = self.style;
        let s = self.section;
        let mut h = match st.heading {
            Heading::Centered => {
                Block::new(format!("SECTION {number} - {title}"), st.size + 1.0, s)
                    .face(Face::Bold)
                    .center()
                    .space(0.0, 14.0)
            }
            Heading::Ruled => {
                let mut b = Block::new(title, st.size + 2.5, s)
                    .face(Face::Bold)
                    .at(number, 0.0, 72.0)
                    .space(if new_page { 0.0 } else { 18.0 }, 12.0);
                b.rule = true;
                b
            }
            Heading::Banner => {
                let mut b = Block::new(title, st.size + 3.5, s)
                    .face(Face::Display)
                    .at(number, 8.0, 80.0)
                    .space(if new_page { 0.0 } else { 18.0 }, 14.0);
                b.banner = true;
                b
            }
        };
        h.new_page = new_page;
        h.keep_next = true;
        self.push(h);
    }
    fn end(&mut self, number: &str) {
        if self.style.content == Content::Full && self.style.page_per_section {
            let b = Block::new(
                format!("END OF SECTION {number}"),
                self.style.size,
                self.section,
            )
            .face(Face::Bold)
            .center()
            .space(18.0, 0.0);
            self.push(b);
        }
    }
    fn text(&mut self, text: impl Into<String>) -> Block {
        Block::new(text, self.style.size, self.section)
    }
}

/// Whether a paragraph survives the style's content filter.
fn keep_article(content: Content, part: usize, title: &str) -> bool {
    match content {
        Content::Full => true,
        Content::Short => match part {
            0 => title.contains("SUMMARY"),
            1 => true,
            _ => title.contains("INSTALL") || title.contains("APPLICATION"),
        },
        Content::Narrative => part == 1,
    }
}

/// Narrative: drops "Manufacturers:" lists and anything deeper than 1.
fn narrative(ps: &[Paragraph]) -> Vec<Paragraph> {
    let mut out = vec![];
    let mut skip_below: Option<u8> = None;
    for p in ps {
        if let Some(l) = skip_below {
            if p.level > l {
                continue;
            }
            skip_below = None;
        }
        if p.text.starts_with("Manufacturers") || p.text.starts_with("Basis-of-Design") {
            skip_below = Some(p.level);
            continue;
        }
        if p.level <= 1 {
            out.push(p.clone());
        }
    }
    out
}

fn section_body(b: &mut Builder<'_>, s: &SpecSection) {
    let st = b.style;
    let n = st.numbering;
    b.lists += 1;
    let list = b.lists;
    let document = s.kind == SectionKind::Document;
    for (pi, part) in s.parts.iter().enumerate() {
        let pn = pi as u32 + 1;
        let arts: Vec<_> = part
            .articles
            .iter()
            .filter(|a| document || keep_article(st.content, pi, &a.title))
            .collect();
        if arts.is_empty() {
            continue;
        }
        if !document && st.content != Content::Narrative {
            let mut h = b
                .text(part_label(n, pn, &part.title))
                .face(Face::Bold)
                .space(if pi == 0 { 0.0 } else { 14.0 }, 6.0)
                .keep();
            h.list = Some(ListRef { list, level: 0 });
            b.push(h);
        }
        for (ai, a) in arts.iter().enumerate() {
            let an = ai as u32 + 1;
            let (lx, tx) = indents(n, None);
            let paras = if st.content == Content::Narrative {
                narrative(&a.paragraphs)
            } else if st.content == Content::Short && pi == 1 {
                a.paragraphs
                    .iter()
                    .filter(|p| p.level <= 1)
                    .cloned()
                    .collect()
            } else {
                a.paragraphs.clone()
            };
            let not_used = paras.len() == 1
                && paras[0]
                    .text
                    .trim_end_matches('.')
                    .eq_ignore_ascii_case("not used");
            if paras.is_empty() || (not_used && st.content != Content::Full) {
                continue;
            }
            let title = if st.content == Content::Narrative {
                title_case(&a.title)
            } else {
                a.title.clone()
            };
            let mut h = b
                .text(title)
                .face(if st.bold_articles || st.content == Content::Narrative {
                    Face::Bold
                } else {
                    Face::Regular
                })
                .at(
                    article_label(n, pn, an),
                    lx,
                    if n == Numbering::Bullets { 0.0 } else { tx },
                )
                .space(st.gap + 4.0, st.gap * 0.5)
                .keep();
            h.list = Some(ListRef { list, level: 1 });
            b.push(h);
            // Counters per level, reset below a paragraph.
            let mut count = [0u32; 4];
            for p in &paras {
                let l = p.level.min(3) as usize;
                count[l] += 1;
                for c in count.iter_mut().skip(l + 1) {
                    *c = 0;
                }
                let mut path = vec![pn, an];
                path.extend(count.iter().take(l + 1).copied());
                let (px, pt) = indents(n, Some(l as u8));
                let mut blk = b
                    .text(p.text.clone())
                    .at(label(n, l as u8, count[l], &path), px, pt)
                    .space(if l == 0 { st.gap } else { st.gap * 0.6 }, 0.0);
                blk.list = Some(ListRef {
                    list,
                    level: 2 + l as u8,
                });
                if p.text.trim_end().ends_with(':') {
                    blk.keep_next = true;
                }
                b.push(blk);
            }
        }
    }
}

fn front_section(b: &mut Builder<'_>, s: &SpecSection, book: &SpecBook, f: &Front, only: bool) {
    let size = b.style.size;
    let bold = |b: &mut Builder<'_>, t: &str, before: f32| {
        let x = b.text(t).face(Face::Bold).space(before, 2.0).keep();
        b.push(x);
    };
    let line = |b: &mut Builder<'_>, t: &str| {
        if !t.trim().is_empty() {
            let x = b.text(t).space(0.0, 1.0);
            b.push(x);
        }
    };
    let party = |b: &mut Builder<'_>, p: &crate::generate::Party, before: f32| {
        bold(b, &p.role.to_uppercase(), before);
        line(b, &p.company);
        line(b, &p.name);
        line(b, &p.address);
        line(b, &p.phone);
        line(b, &p.email);
    };
    match s.kind {
        SectionKind::TitlePage => {
            let t = |b: &mut Builder<'_>, text: &str, sz: f32, face: Face, before: f32| {
                if !text.trim().is_empty() {
                    let x = Block::new(text, sz, b.section)
                        .face(face)
                        .center()
                        .space(before, 4.0);
                    b.push(x);
                }
            };
            t(b, "PROJECT MANUAL", size + 6.0, Face::Bold, 150.0);
            t(b, &f.project_name, size + 14.0, Face::Display, 18.0);
            t(b, &f.address, size + 2.0, Face::Regular, 6.0);
            if !f.project_number.is_empty() {
                t(
                    b,
                    &format!("Project No. {}", f.project_number),
                    size + 1.0,
                    Face::Regular,
                    4.0,
                );
            }
            if let Some(o) = &f.owner {
                t(b, "OWNER", size, Face::Bold, 48.0);
                t(
                    b,
                    if o.company.is_empty() {
                        &o.name
                    } else {
                        &o.company
                    },
                    size + 1.0,
                    Face::Regular,
                    0.0,
                );
            }
            if let Some(a) = f.team.iter().find(|p| p.role.to_lowercase() == "architect") {
                t(b, "ARCHITECT", size, Face::Bold, 24.0);
                t(
                    b,
                    if a.company.is_empty() {
                        &a.name
                    } else {
                        &a.company
                    },
                    size + 1.0,
                    Face::Regular,
                    0.0,
                );
            }
            t(b, &book.issue.to_uppercase(), size + 2.0, Face::Bold, 60.0);
            t(b, &long_date(&book.date), size + 1.0, Face::Regular, 0.0);
        }
        SectionKind::ProjectDirectory => {
            if let Some(o) = &f.owner {
                party(b, o, 0.0);
            }
            for p in &f.team {
                party(b, p, 12.0);
            }
            if f.owner.is_none() && f.team.is_empty() {
                line(
                    b,
                    "Fill in the client and consultants on the Project Info tab.",
                );
            }
        }
        SectionKind::SealsPage => {
            let x = b
                .text("The following design professionals are responsible for the portions of the Project Manual indicated.")
                .space(0.0, 8.0);
            b.push(x);
            for p in &f.team {
                bold(b, &p.role.to_uppercase(), 14.0);
                line(
                    b,
                    if p.company.is_empty() {
                        &p.name
                    } else {
                        &p.company
                    },
                );
                let r = responsible_for(&p.role);
                if !r.is_empty() {
                    let x = b.text(r).face(Face::Italic).space(0.0, 4.0);
                    b.push(x);
                }
                let mut sealbox = b.text("").space(2.0, 4.0);
                sealbox.boxed = 130.0;
                b.push(sealbox);
            }
        }
        SectionKind::Contents => {
            let mut group = "";
            let mut division = String::new();
            for sec in book
                .sections
                .iter()
                .filter(|x| (!only || x.included) && shown(b.style, x))
            {
                let d = sec.division();
                if subgroup(&d) != group {
                    group = subgroup(&d);
                    let x = b
                        .text(group)
                        .face(Face::Bold)
                        .center()
                        .space(16.0, 6.0)
                        .keep();
                    b.push(x);
                }
                if d != division {
                    division = d.clone();
                    let x = b
                        .text(format!("DIVISION {d} - {}", division_title(&d)))
                        .face(Face::Bold)
                        .space(10.0, 3.0)
                        .keep();
                    b.push(x);
                }
                let x = b
                    .text(title_case(&sec.title))
                    .at(sec.number.clone(), 18.0, 90.0)
                    .space(0.0, 1.0);
                b.push(x);
            }
        }
        SectionKind::DrawingList => {
            if f.sheets.is_empty() {
                line(b, "No sheets yet: sheets added in the project list here.");
            }
            for (num, name) in &f.sheets {
                let x = b
                    .text(name.clone())
                    .at(num.clone(), 0.0, 72.0)
                    .space(0.0, 1.5);
                b.push(x);
            }
        }
        SectionKind::ThreePart | SectionKind::Document => {}
    }
}

/// Whether the style prints a section: narrative and short-form books leave out the
/// directory, seals and drawing list; narrative leaves out Divisions 00 and 01 (but the
/// title page and contents).
pub fn shown(style: &SpecStyle, s: &SpecSection) -> bool {
    match style.content {
        Content::Full => true,
        _ if matches!(s.kind, SectionKind::TitlePage | SectionKind::Contents) => true,
        _ if s.kind.generated() => false,
        Content::Short => true,
        Content::Narrative => !matches!(s.division().as_str(), "00" | "01"),
    }
}

/// Lays out the book (`only_included`: leaving out excluded sections, as issued).
pub fn layout(book: &SpecBook, style: &SpecStyle, front: &Front, only_included: bool) -> Laid {
    let mut b = Builder {
        style,
        out: vec![],
        sections: vec![Run {
            number: String::new(),
            title: String::new(),
        }],
        lists: 0,
        section: 0,
    };
    let mut division = String::new();
    let mut first = true;
    for s in book
        .sections
        .iter()
        .filter(|s| !only_included || s.included)
    {
        if !shown(style, s) {
            continue;
        }
        let d = s.division();
        let new_page = style.page_per_section || s.kind.generated() || first;
        if !style.page_per_section && d != division && !s.kind.generated() {
            division = d.clone();
            let mut x = b
                .text(format!("DIVISION {d} - {}", division_title(&d)))
                .face(Face::Display)
                .space(if first { 0.0 } else { 22.0 }, 6.0)
                .keep();
            x.size = style.size + 3.0;
            x.new_page = first;
            b.push(x);
            first = false;
            b.begin(&s.number, &s.title, false);
        } else if s.kind == SectionKind::TitlePage {
            // The title page has no heading.
            if style.page_per_section {
                b.sections.push(Run {
                    number: s.number.clone(),
                    title: title_case(&s.title),
                });
                b.section = b.sections.len() - 1;
            }
            let mut x = b.text("");
            x.new_page = true;
            b.push(x);
            first = false;
        } else {
            b.begin(&s.number, &s.title, new_page);
            first = false;
        }
        if s.kind.generated() {
            front_section(&mut b, s, book, front, only_included);
        } else {
            section_body(&mut b, s);
        }
        if !s.kind.generated() {
            b.end(&s.number);
        }
        // Run-on styles start the body on a new page after the front matter.
        if !style.page_per_section && s.kind.generated() {
            division.clear();
            first = true;
        }
    }
    Laid {
        blocks: b.out,
        sections: b.sections,
        header_left: front.project_name.clone(),
        header_right: [
            if front.project_number.is_empty() {
                String::new()
            } else {
                format!("Project No. {}", front.project_number)
            },
            book.issue.clone(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("  |  "),
        per_section: style.page_per_section,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::style;
    use studio_core::specs::{Article, SpecPart};

    fn one(title: &str) -> SpecBook {
        let mut s = SpecSection::blank("09 29 00", title);
        s.parts[0].articles = vec![Article {
            title: "SUMMARY".into(),
            paragraphs: vec![
                Paragraph::parse("Section Includes:"),
                Paragraph::parse(">Gypsum board."),
                Paragraph::parse(">Tile backing panels."),
                Paragraph::parse("Related Requirements:"),
                Paragraph::parse(">>Deep."),
            ],
        }];
        s.parts[1] = SpecPart {
            title: "PRODUCTS".into(),
            articles: vec![Article {
                title: "GYPSUM BOARD".into(),
                paragraphs: vec![
                    Paragraph::parse("Manufacturers:"),
                    Paragraph::parse(">USG."),
                    Paragraph::parse("Type X: 5/8 inch."),
                ],
            }],
        };
        SpecBook {
            style: "csi-classic".into(),
            issue: "Bid Set".into(),
            date: "2026-10-01".into(),
            sections: vec![s],
        }
    }

    #[test]
    fn csi_classic_numbers_parts_articles_and_paragraphs() {
        let l = layout(
            &one("GYPSUM BOARD"),
            &style("csi-classic"),
            &Front::default(),
            true,
        );
        let t: Vec<(String, String)> = l
            .blocks
            .iter()
            .map(|b| (b.label.clone(), b.text.clone()))
            .collect();
        assert_eq!(t[0].1, "SECTION 09 29 00 - GYPSUM BOARD");
        assert_eq!(t[1].1, "PART 1 - GENERAL");
        assert_eq!(t[2], ("1.01".into(), "SUMMARY".into()));
        assert_eq!(t[3], ("A.".into(), "Section Includes:".into()));
        assert_eq!(t[4], ("1.".into(), "Gypsum board.".into()));
        assert_eq!(t[5].0, "2.");
        assert_eq!(t[6].0, "B.");
        assert_eq!(t[7].0, "a.");
        assert_eq!(t.last().unwrap().1, "END OF SECTION 09 29 00");
        // Paragraph A. at 0.5 inch, its text at 1 inch; level 1 at 1 inch.
        assert_eq!(
            (l.blocks[3].label_x, l.blocks[3].x, l.blocks[4].label_x),
            (36.0, 72.0, 72.0)
        );
        assert!(l.blocks[0].new_page && l.per_section);
        assert_eq!(l.sections[1].title, "Gypsum Board");
    }

    #[test]
    fn decimal_and_narrative_styles() {
        let l = layout(
            &one("GYPSUM BOARD"),
            &style("decimal"),
            &Front::default(),
            true,
        );
        let labels: Vec<&str> = l.blocks.iter().map(|b| b.label.as_str()).collect();
        assert!(
            labels.contains(&"1.1.1") && labels.contains(&"1.1.1.1") && labels.contains(&"1.1.2")
        );
        // Narrative: products only, manufacturers dropped, bullets.
        let l = layout(
            &one("GYPSUM BOARD"),
            &style("narrative"),
            &Front::default(),
            true,
        );
        let texts: Vec<&str> = l.blocks.iter().map(|b| b.text.as_str()).collect();
        assert!(
            texts.contains(&"Type X: 5/8 inch.")
                && !texts.contains(&"USG.")
                && !texts.contains(&"Gypsum board.")
        );
        assert!(texts[0].starts_with("DIVISION 09"));
        assert!(!l.per_section);
    }

    #[test]
    fn words_and_dates() {
        assert_eq!(
            title_case("THERMOPLASTIC-POLYOLEFIN (TPO) ROOFING"),
            "Thermoplastic-Polyolefin (TPO) Roofing"
        );
        assert_eq!(
            title_case("TOILET, BATH, AND LAUNDRY ACCESSORIES"),
            "Toilet, Bath, and Laundry Accessories"
        );
        assert_eq!(
            title_case("COMMON WORK RESULTS FOR HVAC"),
            "Common Work Results for HVAC"
        );
        assert_eq!(
            title_case("CAST-IN-PLACE CONCRETE"),
            "Cast-in-Place Concrete"
        );
        assert_eq!(long_date("2026-10-01"), "October 1, 2026");
        assert_eq!(long_date("TBD"), "TBD");
    }
}
