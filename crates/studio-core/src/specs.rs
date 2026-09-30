//! The project manual (ADR-085): the specification book, kept in the project as one
//! SpecBook element and edited through transactions like any model change. Sections follow
//! CSI SectionFormat (PART 1 GENERAL, PART 2 PRODUCTS, PART 3 EXECUTION, articles and
//! paragraphs at levels A. 1. a.); numbering and page layout are the style's business
//! (studio-specs), so the book stores only structure and text. Sections are keyed by their
//! MasterFormat number, unique in a book.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData};

/// One paragraph: its level (0: A., 1: 1., 2: a., 3: 1)) and text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "SpecParagraph")]
pub struct Paragraph {
    pub level: u8,
    pub text: String,
}

impl Paragraph {
    pub fn new(level: u8, text: impl Into<String>) -> Self {
        Paragraph {
            level: level.min(MAX_LEVEL),
            text: text.into(),
        }
    }
    /// Reads the library's shorthand: one leading `>` per level ("Section Includes:",
    /// ">Gypsum board.").
    pub fn parse(s: &str) -> Self {
        let t = s.trim_start_matches('>');
        Paragraph::new((s.len() - t.len()) as u8, t.trim())
    }
    /// The shorthand back.
    pub fn shorthand(&self) -> String {
        format!("{}{}", ">".repeat(self.level as usize), self.text)
    }
}

pub const MAX_LEVEL: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "SpecArticle")]
pub struct Article {
    pub title: String,
    pub paragraphs: Vec<Paragraph>,
}

/// A part: "GENERAL", "PRODUCTS", "EXECUTION"; a document section has one untitled part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpecPart {
    pub title: String,
    pub articles: Vec<Article>,
}

/// What a section is. The generated kinds are written from the project when the book is
/// shown or exported (Project Info, the team, the sheets), so they are never stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SectionKind {
    ThreePart,
    /// A Division 00 document: articles without PART headings.
    Document,
    TitlePage,
    ProjectDirectory,
    SealsPage,
    Contents,
    DrawingList,
}

impl SectionKind {
    pub fn generated(self) -> bool {
        !matches!(self, SectionKind::ThreePart | SectionKind::Document)
    }
}

/// Where a section's text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SectionOrigin {
    #[default]
    Library,
    Custom,
    Claude,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpecSection {
    /// MasterFormat number, "09 29 00" (optionally ".11").
    pub number: String,
    /// Upper case, "GYPSUM BOARD".
    pub title: String,
    pub kind: SectionKind,
    pub parts: Vec<SpecPart>,
    /// Excluded sections stay in the book (their text kept) but aren't issued.
    pub included: bool,
    pub origin: SectionOrigin,
    /// Changed since it came from the library.
    pub edited: bool,
}

impl SpecSection {
    /// The two-digit division, "09".
    pub fn division(&self) -> String {
        division_of(&self.number)
    }
    pub fn paragraph_count(&self) -> usize {
        self.parts
            .iter()
            .flat_map(|p| &p.articles)
            .map(|a| a.paragraphs.len())
            .sum()
    }
    /// A new three-part section with the usual articles, to be filled in.
    pub fn blank(number: &str, title: &str) -> Self {
        let art = |t: &str, p: &[&str]| Article {
            title: t.into(),
            paragraphs: p.iter().map(|s| Paragraph::parse(s)).collect(),
        };
        SpecSection {
            number: number.trim().into(),
            title: title.trim().to_uppercase(),
            kind: SectionKind::ThreePart,
            parts: vec![
                SpecPart {
                    title: "GENERAL".into(),
                    articles: vec![
                        art("SUMMARY", &["Section Includes:", ">"]),
                        art("ACTION SUBMITTALS", &["Product Data: For each type of product."]),
                        art(
                            "QUALITY ASSURANCE",
                            &["Installer Qualifications: An experienced installer."],
                        ),
                    ],
                },
                SpecPart {
                    title: "PRODUCTS".into(),
                    articles: vec![art(
                        "MATERIALS",
                        &["Manufacturers: Subject to compliance with requirements, provide products by one of the following:", ">"],
                    )],
                },
                SpecPart {
                    title: "EXECUTION".into(),
                    articles: vec![
                        art("EXAMINATION", &["Examine substrates, with Installer present, for compliance with requirements. Proceed with installation only after unsatisfactory conditions have been corrected."]),
                        art("INSTALLATION", &["Comply with manufacturer's written installation instructions."]),
                    ],
                },
            ],
            included: true,
            origin: SectionOrigin::Custom,
            edited: false,
        }
    }
}

pub fn division_of(number: &str) -> String {
    number.trim().chars().take(2).collect()
}

/// Checks a MasterFormat number: "NN NN NN" with an optional ".NN".
pub fn valid_number(number: &str) -> bool {
    let (main, suffix) = match number.split_once('.') {
        Some((m, s)) => (m, Some(s)),
        None => (number, None),
    };
    let pairs: Vec<&str> = main.split(' ').collect();
    pairs.len() == 3
        && pairs
            .iter()
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_digit()))
        && suffix.is_none_or(|s| s.len() == 2 && s.chars().all(|c| c.is_ascii_digit()))
}

/// The book: its sections in number order, the style it's issued in, and the issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpecBook {
    /// A studio-specs style id ("csi-classic"…).
    pub style: String,
    /// "Issued for Permit", "Bid Set"…
    pub issue: String,
    /// ISO date, or empty.
    pub date: String,
    pub sections: Vec<SpecSection>,
}

impl SpecBook {
    pub fn section(&self, number: &str) -> Option<&SpecSection> {
        self.sections.iter().find(|s| s.number == number)
    }
    /// Keeps the sections in MasterFormat order.
    pub fn sort(&mut self) {
        self.sections.sort_by(|a, b| a.number.cmp(&b.number));
    }
    fn check(&self) -> CoreResult<()> {
        let mut seen = std::collections::HashSet::new();
        for s in &self.sections {
            if !valid_number(&s.number) {
                return Err(CoreError::Invalid(format!(
                    "\"{}\" isn't a MasterFormat number (like 09 29 00)",
                    s.number
                )));
            }
            if s.title.trim().is_empty() {
                return Err(CoreError::Invalid(format!(
                    "section {} needs a title",
                    s.number
                )));
            }
            if !seen.insert(s.number.as_str()) {
                return Err(CoreError::Invalid(format!(
                    "the book already has a section {}",
                    s.number
                )));
            }
        }
        Ok(())
    }
}

/// The project's book, if it has one.
pub fn book(doc: &Document) -> Option<SpecBook> {
    doc.of(Category::SpecBook).find_map(|e| match &e.data {
        ElementData::SpecBook(b) => Some((**b).clone()),
        _ => None,
    })
}

/// Saves the whole book as one undoable step named `label` (made on the first save).
pub fn save(doc: &mut Document, label: &str, mut book: SpecBook) -> CoreResult<()> {
    book.sort();
    book.check()?;
    let existing = doc.of(Category::SpecBook).next().map(|e| e.id);
    doc.transact(label, |tx| {
        match existing {
            Some(id) => tx.modify(id, |d| {
                if let ElementData::SpecBook(b) = d {
                    **b = book.clone();
                }
            })?,
            None => {
                tx.insert(ElementData::SpecBook(Box::new(book.clone())));
            }
        }
        Ok(())
    })
}

/// Edits the book in place and saves it (errors if there is none yet).
pub fn change(
    doc: &mut Document,
    label: &str,
    f: impl FnOnce(&mut SpecBook) -> CoreResult<()>,
) -> CoreResult<()> {
    let mut b = book(doc).ok_or_else(|| {
        CoreError::Invalid("there's no project manual yet: generate one first".into())
    })?;
    f(&mut b)?;
    save(doc, label, b)
}

/// Replaces section `number` (which may be renumbered) with `section`, marked edited.
pub fn set_section(doc: &mut Document, number: &str, mut section: SpecSection) -> CoreResult<()> {
    change(doc, &format!("Edit Section {}", section.number), |b| {
        let i = b
            .sections
            .iter()
            .position(|s| s.number == number)
            .ok_or_else(|| CoreError::Invalid(format!("no section {number}")))?;
        section.title = section.title.trim().to_uppercase();
        if b.sections[i] != section {
            section.edited = true;
        }
        b.sections[i] = section;
        Ok(())
    })
}

/// Includes or excludes sections.
pub fn set_included(doc: &mut Document, numbers: &[String], included: bool) -> CoreResult<()> {
    let label = if included {
        "Include Sections"
    } else {
        "Exclude Sections"
    };
    change(doc, label, |b| {
        for s in &mut b.sections {
            if numbers.contains(&s.number) {
                s.included = included;
            }
        }
        Ok(())
    })
}

/// Adds sections (new numbers only).
pub fn add_sections(doc: &mut Document, sections: Vec<SpecSection>) -> CoreResult<()> {
    let label = match sections.as_slice() {
        [one] => format!("Add Section {}", one.number),
        _ => format!("Add {} Sections", sections.len()),
    };
    change(doc, &label, |b| {
        b.sections.extend(sections);
        Ok(())
    })
}

pub fn remove_sections(doc: &mut Document, numbers: &[String]) -> CoreResult<()> {
    change(doc, "Remove Sections", |b| {
        b.sections.retain(|s| !numbers.contains(&s.number));
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_of(numbers: &[&str]) -> SpecBook {
        SpecBook {
            style: "csi-classic".into(),
            issue: "Bid Set".into(),
            date: String::new(),
            sections: numbers
                .iter()
                .map(|n| SpecSection::blank(n, "Test Section"))
                .collect(),
        }
    }

    #[test]
    fn paragraphs_read_and_write_the_shorthand() {
        let p = Paragraph::parse(">>Type X, 5/8 inch thick.");
        assert_eq!((p.level, p.text.as_str()), (2, "Type X, 5/8 inch thick."));
        assert_eq!(p.shorthand(), ">>Type X, 5/8 inch thick.");
        assert_eq!(Paragraph::parse(">>>>>deep").level, MAX_LEVEL);
    }

    #[test]
    fn numbers_are_masterformat() {
        assert!(valid_number("09 29 00") && valid_number("23 74 16.11"));
        for bad in ["092900", "09 29", "9 29 00", "09 29 00.1", "AB 29 00"] {
            assert!(!valid_number(bad), "{bad}");
        }
        assert_eq!(division_of("09 29 00"), "09");
    }

    #[test]
    fn the_book_saves_sorted_and_undoes() {
        let mut doc = Document::new();
        assert!(book(&doc).is_none());
        save(
            &mut doc,
            "Generate Project Manual",
            book_of(&["09 29 00", "01 10 00"]),
        )
        .unwrap();
        let b = book(&doc).unwrap();
        assert_eq!(b.sections[0].number, "01 10 00");
        // Editing a section marks it edited; renumbering keeps it in order.
        let mut s = b.sections[1].clone();
        s.number = "00 73 00".into();
        s.parts[0].articles[0]
            .paragraphs
            .push(Paragraph::new(0, "More."));
        set_section(&mut doc, "09 29 00", s).unwrap();
        let b = book(&doc).unwrap();
        assert_eq!(b.sections[0].number, "00 73 00");
        assert!(b.sections[0].edited && !b.sections[1].edited);
        doc.undo().unwrap();
        assert_eq!(book(&doc).unwrap().sections[1].number, "09 29 00");
        // Duplicates and bad numbers are refused.
        assert!(add_sections(&mut doc, vec![SpecSection::blank("01 10 00", "Again")]).is_err());
        assert!(add_sections(&mut doc, vec![SpecSection::blank("1 10", "Bad")]).is_err());
        set_included(&mut doc, &["01 10 00".into()], false).unwrap();
        assert!(!book(&doc).unwrap().sections[0].included);
        remove_sections(&mut doc, &["01 10 00".into()]).unwrap();
        assert_eq!(book(&doc).unwrap().sections.len(), 1);
    }
}
