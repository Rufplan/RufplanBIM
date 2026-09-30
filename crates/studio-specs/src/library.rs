//! The section library: MasterFormat sections in CSI SectionFormat, written as TOML files
//! under `library/` (one per group of divisions), plus the front-matter sections the app
//! writes from the project (title page, directory, seals, contents, drawing list).

use std::sync::OnceLock;

use serde::Deserialize;
use studio_core::specs::{Article, Paragraph, SectionKind, SectionOrigin, SpecPart, SpecSection};

/// The library files, in division order.
const SOURCES: &[(&str, &str)] = &[
    ("div00-01", include_str!("../library/div00-01.toml")),
    ("div02-06", include_str!("../library/div02-06.toml")),
    ("div07", include_str!("../library/div07.toml")),
    ("div08", include_str!("../library/div08.toml")),
    ("div09-10", include_str!("../library/div09-10.toml")),
    ("div11-22", include_str!("../library/div11-22.toml")),
    ("div23-28", include_str!("../library/div23-28.toml")),
    ("div31-33", include_str!("../library/div31-33.toml")),
];

/// MasterFormat divisions: number and title.
pub const DIVISIONS: &[(&str, &str)] = &[
    ("00", "PROCUREMENT AND CONTRACTING REQUIREMENTS"),
    ("01", "GENERAL REQUIREMENTS"),
    ("02", "EXISTING CONDITIONS"),
    ("03", "CONCRETE"),
    ("04", "MASONRY"),
    ("05", "METALS"),
    ("06", "WOOD, PLASTICS, AND COMPOSITES"),
    ("07", "THERMAL AND MOISTURE PROTECTION"),
    ("08", "OPENINGS"),
    ("09", "FINISHES"),
    ("10", "SPECIALTIES"),
    ("11", "EQUIPMENT"),
    ("12", "FURNISHINGS"),
    ("13", "SPECIAL CONSTRUCTION"),
    ("14", "CONVEYING EQUIPMENT"),
    ("21", "FIRE SUPPRESSION"),
    ("22", "PLUMBING"),
    ("23", "HEATING, VENTILATING, AND AIR CONDITIONING (HVAC)"),
    ("25", "INTEGRATED AUTOMATION"),
    ("26", "ELECTRICAL"),
    ("27", "COMMUNICATIONS"),
    ("28", "ELECTRONIC SAFETY AND SECURITY"),
    ("31", "EARTHWORK"),
    ("32", "EXTERIOR IMPROVEMENTS"),
    ("33", "UTILITIES"),
    ("34", "TRANSPORTATION"),
];

pub fn division_title(code: &str) -> &'static str {
    DIVISIONS.iter().find(|d| d.0 == code).map_or("", |d| d.1)
}

/// The groups the book is issued in (MasterFormat's subgroups).
pub fn subgroup(code: &str) -> &'static str {
    match code {
        "00" => "PROCUREMENT AND CONTRACTING REQUIREMENTS GROUP",
        "01" => "GENERAL REQUIREMENTS SUBGROUP",
        "02" | "03" | "04" | "05" | "06" | "07" | "08" | "09" | "10" | "11" | "12" | "13"
        | "14" => "FACILITY CONSTRUCTION SUBGROUP",
        "21" | "22" | "23" | "25" | "26" | "27" | "28" => "FACILITY SERVICES SUBGROUP",
        _ => "SITE AND INFRASTRUCTURE SUBGROUP",
    }
}

/// One library section: its text (placeholders not yet filled) and when it applies.
#[derive(Debug, Clone)]
pub struct LibSection {
    pub section: SpecSection,
    /// Auto-include tags: any entry matches; "a+b" needs both; "optional" never.
    pub when: Vec<String>,
}

#[derive(Deserialize)]
struct RawFile {
    section: Vec<RawSection>,
}

#[derive(Deserialize)]
struct RawArticle {
    title: String,
    items: Vec<String>,
}

#[derive(Deserialize)]
struct RawSection {
    number: String,
    title: String,
    when: Vec<String>,
    kind: String,
    #[serde(default)]
    part1: Vec<RawArticle>,
    #[serde(default)]
    part2: Vec<RawArticle>,
    #[serde(default)]
    part3: Vec<RawArticle>,
    #[serde(default)]
    articles: Vec<RawArticle>,
}

fn articles(raw: Vec<RawArticle>) -> Vec<Article> {
    raw.into_iter()
        .map(|a| Article {
            title: a.title.trim().to_uppercase(),
            paragraphs: a.items.iter().map(|s| Paragraph::parse(s)).collect(),
        })
        .collect()
}

/// Parses one library file.
pub fn parse(src: &str) -> Result<Vec<LibSection>, String> {
    let f: RawFile = toml::from_str(src).map_err(|e| e.to_string())?;
    f.section
        .into_iter()
        .map(|r| {
            let (kind, parts) = match r.kind.as_str() {
                "three-part" => (
                    SectionKind::ThreePart,
                    vec![
                        SpecPart {
                            title: "GENERAL".into(),
                            articles: articles(r.part1),
                        },
                        SpecPart {
                            title: "PRODUCTS".into(),
                            articles: articles(r.part2),
                        },
                        SpecPart {
                            title: "EXECUTION".into(),
                            articles: articles(r.part3),
                        },
                    ],
                ),
                "document" => (
                    SectionKind::Document,
                    vec![SpecPart {
                        title: String::new(),
                        articles: articles(r.articles),
                    }],
                ),
                k => return Err(format!("section {}: unknown kind {k:?}", r.number)),
            };
            Ok(LibSection {
                section: SpecSection {
                    number: r.number.trim().into(),
                    title: r.title.trim().to_uppercase(),
                    kind,
                    parts,
                    included: true,
                    origin: SectionOrigin::Library,
                    edited: false,
                },
                when: r.when,
            })
        })
        .collect()
}

fn generated(number: &str, title: &str, kind: SectionKind) -> LibSection {
    LibSection {
        section: SpecSection {
            number: number.into(),
            title: title.into(),
            kind,
            parts: vec![],
            included: true,
            origin: SectionOrigin::Library,
            edited: false,
        },
        when: vec!["always".into()],
    }
}

/// The whole library in number order: the front matter, then the files' sections. A file
/// that doesn't parse is left out (the tests keep them all parsing).
pub fn library() -> &'static [LibSection] {
    static LIB: OnceLock<Vec<LibSection>> = OnceLock::new();
    LIB.get_or_init(|| {
        let mut all = vec![
            generated("00 01 01", "PROJECT TITLE PAGE", SectionKind::TitlePage),
            generated(
                "00 01 03",
                "PROJECT DIRECTORY",
                SectionKind::ProjectDirectory,
            ),
            generated("00 01 07", "SEALS PAGE", SectionKind::SealsPage),
            generated("00 01 10", "TABLE OF CONTENTS", SectionKind::Contents),
            generated(
                "00 01 15",
                "LIST OF DRAWING SHEETS",
                SectionKind::DrawingList,
            ),
        ];
        for (_, src) in SOURCES {
            if let Ok(s) = parse(src) {
                all.extend(s);
            }
        }
        all.sort_by(|a, b| a.section.number.cmp(&b.section.number));
        all.dedup_by(|a, b| a.section.number == b.section.number);
        all
    })
}

pub fn entry(number: &str) -> Option<&'static LibSection> {
    library().iter().find(|l| l.section.number == number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_library_file_parses_with_known_tags_and_levels() {
        let mut n = 0;
        for (name, src) in SOURCES {
            let sections = parse(src).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!sections.is_empty(), "{name} is empty");
            for s in &sections {
                let sec = &s.section;
                assert!(
                    studio_core::specs::valid_number(&sec.number),
                    "{}",
                    sec.number
                );
                assert!(!s.when.is_empty(), "{}", sec.number);
                for w in &s.when {
                    for t in w.split('+') {
                        assert!(
                            crate::features::TAGS.contains(&t),
                            "{}: tag {t}",
                            sec.number
                        );
                    }
                }
                assert!(sec.paragraph_count() > 5, "{} is too short", sec.number);
                for a in sec.parts.iter().flat_map(|p| &p.articles) {
                    assert!(!a.paragraphs.is_empty(), "{} {}", sec.number, a.title);
                }
            }
            n += sections.len();
        }
        // The front matter plus every file's sections, none repeated.
        assert_eq!(library().len(), n + 5);
        assert!(n >= 120, "{n} sections");
        assert_eq!(
            entry("09 29 00").map(|l| l.section.title.as_str()),
            Some("GYPSUM BOARD")
        );
        assert_eq!(
            entry("00 01 10").map(|l| l.section.kind),
            Some(SectionKind::Contents)
        );
    }

    #[test]
    fn a_three_part_section_reads_its_parts_and_levels() {
        let src = r#"
[[section]]
number = "09 29 00"
title = "Gypsum Board"
when = ["gypsum"]
kind = "three-part"
[[section.part1]]
title = "summary"
items = ["Section Includes:", ">Interior gypsum board."]
[[section.part2]]
title = "GYPSUM BOARD"
items = ["Type X: 5/8 inch.", ">>Deep."]
[[section.part3]]
title = "INSTALLATION"
items = ["Install."]
"#;
        let s = &parse(src).unwrap()[0].section;
        assert_eq!(s.title, "GYPSUM BOARD");
        assert_eq!(s.parts.len(), 3);
        assert_eq!(s.parts[0].articles[0].title, "SUMMARY");
        assert_eq!(s.parts[0].articles[0].paragraphs[1].level, 1);
        assert_eq!(s.parts[1].articles[0].paragraphs[1].level, 2);
        assert!(parse("[[section]]\nnumber = \"x\"").is_err());
    }
}
