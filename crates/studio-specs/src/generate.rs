//! Making the book from the project: the library sections the model calls for, their
//! placeholders filled, plus the front matter; and keeping it up to date as the model
//! changes, without touching what the architect has edited.

use serde::Serialize;
use studio_core::specs::{Paragraph, SpecBook, SpecSection};
use studio_core::{Category, Document, ElementData};
use ts_rs::TS;

use crate::features::{fill, Facts};
use crate::library::{library, LibSection};

/// A library section with its placeholders filled from the project.
pub fn from_library(lib: &LibSection, facts: &Facts) -> SpecSection {
    let mut s = lib.section.clone();
    for a in s.parts.iter_mut().flat_map(|p| p.articles.iter_mut()) {
        for p in &mut a.paragraphs {
            *p = Paragraph::new(p.level, fill(&p.text, facts));
        }
    }
    s
}

/// A new book: every section the project calls for, in `style`.
pub fn generate(facts: &Facts, style: &str, issue: &str, date: &str) -> SpecBook {
    SpecBook {
        style: style.into(),
        issue: issue.into(),
        date: date.into(),
        sections: library()
            .iter()
            .filter(|l| facts.picks(&l.when))
            .map(|l| from_library(l, facts))
            .collect(),
    }
}

/// What Update from Model did.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecUpdate {
    /// Sections the model now calls for, added.
    pub added: Vec<String>,
    /// Library sections in the book the model no longer calls for (left in, to review).
    pub unindicated: Vec<String>,
}

/// Adds the sections the project now calls for (never replacing one already in the book,
/// edited or not) and lists the ones it no longer does.
pub fn update(book: &SpecBook, facts: &Facts) -> (SpecBook, SpecUpdate) {
    let mut b = book.clone();
    let mut report = SpecUpdate::default();
    for l in library() {
        let n = &l.section.number;
        let in_book = b.section(n).is_some();
        let picks = facts.picks(&l.when);
        if picks && !in_book {
            b.sections.push(from_library(l, facts));
            report.added.push(n.clone());
        } else if !picks && in_book && b.section(n).is_some_and(|s| s.included) {
            report.unindicated.push(n.clone());
        }
    }
    b.sort();
    (b, report)
}

/// A person or firm on the project, for the directory and seals page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SpecParty")]
pub struct Party {
    pub role: String,
    pub company: String,
    pub name: String,
    pub address: String,
    pub phone: String,
    pub email: String,
}

/// What the generated front matter is written from.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SpecFront")]
pub struct Front {
    pub project_name: String,
    pub project_number: String,
    pub address: String,
    pub owner: Option<Party>,
    /// The architect, then the engaged consultants.
    pub team: Vec<Party>,
    /// Sheet number and name, in number order.
    pub sheets: Vec<(String, String)>,
}

pub fn front(doc: &Document) -> Front {
    let (id, d) = studio_core::project::get(doc).unwrap_or_default();
    let party = |role: &str, c: &studio_core::project::Contact| Party {
        role: role.into(),
        company: c.company.clone(),
        name: c.name.clone(),
        address: c.address.clone(),
        phone: c.phone.clone(),
        email: c.email.clone(),
    };
    let mut team: Vec<Party> = d
        .team
        .iter()
        .filter(|m| !m.contact.is_empty())
        .map(|m| party(&m.discipline, &m.contact))
        .collect();
    // The architect first.
    team.sort_by_key(|p| p.role.to_lowercase() != "architect");
    let mut sheets: Vec<(String, String)> = doc
        .of(Category::Sheet)
        .filter_map(|e| match &e.data {
            ElementData::Sheet { number, name, .. } => Some((number.clone(), name.clone())),
            _ => None,
        })
        .collect();
    sheets.sort_by_key(|a| natural(&a.0));
    let address = {
        let a = d.location.one_line();
        if a.is_empty() {
            id.address.clone()
        } else {
            a
        }
    };
    Front {
        project_name: id.name,
        project_number: id.number,
        address,
        owner: (!d.client.is_empty()).then(|| party("Owner", &d.client)),
        team,
        sheets,
    }
}

/// Sorts "A2" before "A10".
fn natural(s: &str) -> Vec<(String, u64)> {
    let mut out = vec![];
    let mut text = String::new();
    let mut num = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else {
            if !num.is_empty() {
                out.push((std::mem::take(&mut text), num.parse().unwrap_or(0)));
                num.clear();
            }
            text.push(c);
        }
    }
    out.push((text, num.parse().unwrap_or(0)));
    out
}

/// Which divisions a discipline is responsible for, as the seals page lists them.
pub fn responsible_for(role: &str) -> &'static str {
    let r = role.to_lowercase();
    if r.contains("structural") {
        "Structural portions of Divisions 03, 04, 05 and 06"
    } else if r.contains("mechanical") || r.contains("mep") {
        "Divisions 21, 22 and 23"
    } else if r.contains("plumbing") {
        "Divisions 21 and 22"
    } else if r.contains("electrical") {
        "Divisions 26, 27 and 28"
    } else if r.contains("civil") {
        "Divisions 31 and 33"
    } else if r.contains("landscape") {
        "Sections 32 84 00, 32 92 00 and 32 93 00"
    } else if r.contains("architect") {
        "All sections not listed for others"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::facts;

    #[test]
    fn a_house_gets_its_sections_filled_and_update_only_adds() {
        let doc = crate::testkit::house();
        let model = studio_regen::regenerate(&doc);
        let f = facts(&doc, &model);
        let b = generate(&f, "csi-classic", "Bid Set", "2026-10-01");
        let has = |n: &str| b.section(n).is_some();
        // Front matter, Division 01, and what the house has.
        for n in [
            "00 01 01", "00 01 10", "01 10 00", "06 10 00", "08 71 00", "09 29 00", "09 91 23",
        ] {
            assert!(has(n), "{n}");
        }
        // Not a commercial building, and optional sections stay out.
        assert!(!has("28 46 21.11") && !has("10 21 13") && !has("00 31 32"));
        // Placeholders are filled.
        let text: String = b
            .sections
            .iter()
            .flat_map(|s| s.parts.iter().flat_map(|p| &p.articles))
            .flat_map(|a| &a.paragraphs)
            .map(|p| p.text.as_str())
            .collect();
        assert!(!text.contains("{project_name}") && !text.contains("{door_types}"));
        // Nothing changed: nothing to add. A section removed is added back; one the model no
        // longer calls for is listed, not removed.
        let (same, r) = update(&b, &f);
        assert_eq!((same.sections.len(), r.added.len()), (b.sections.len(), 0));
        let mut fewer = b.clone();
        fewer.sections.retain(|s| s.number != "09 29 00");
        let mut edited = b.section("09 91 23").unwrap().clone();
        edited.parts[0].articles[0].paragraphs[0].text = "Mine.".into();
        let mut more = fewer.clone();
        more.sections.retain(|s| s.number != "09 91 23");
        more.sections.push(edited.clone());
        let mut c = SpecSection::blank("10 21 13", "Toilet Compartments");
        c.origin = studio_core::specs::SectionOrigin::Library;
        more.sections.push(c);
        more.sort();
        let (u, r) = update(&more, &f);
        assert_eq!(r.added, vec!["09 29 00".to_string()]);
        assert_eq!(r.unindicated, vec!["10 21 13".to_string()]);
        assert_eq!(u.section("09 91 23"), Some(&edited));
    }

    #[test]
    fn sheets_sort_naturally() {
        let mut v = vec!["A10", "A2", "A1.01", "G001"];
        v.sort_by_key(|s| natural(s));
        assert_eq!(v, vec!["A1.01", "A2", "A10", "G001"]);
    }
}
