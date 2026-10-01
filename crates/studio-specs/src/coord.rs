//! Coordination: the cross-references in the book (`Section 09 91 23 "Interior
//! Painting"`) that point at sections not in it, or excluded from it; the checks a spec
//! writer runs before issuing.

use serde::Serialize;
use studio_core::specs::{valid_number, SpecBook, SpecSection};
use ts_rs::TS;

use crate::library::entry;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecReference {
    /// The sections the reference is in.
    pub from: Vec<String>,
    /// The section it points to.
    pub to: String,
    /// Its title, if the library has it (it can then be added).
    pub title: Option<String>,
    /// In the book but excluded.
    pub excluded: bool,
}

/// The section numbers `text` refers to ("Section 07 92 00", "Sections 09 29 00 and
/// 09 91 23").
pub fn numbers_in(text: &str) -> Vec<String> {
    let mut out = vec![];
    let b = text.as_bytes();
    let mut i = 0;
    while i + 8 <= b.len() {
        let digit = |k: usize| b.get(k).is_some_and(|c| c.is_ascii_digit());
        let is_num = digit(i)
            && digit(i + 1)
            && b[i + 2] == b' '
            && digit(i + 3)
            && digit(i + 4)
            && b.get(i + 5) == Some(&b' ')
            && digit(i + 6)
            && digit(i + 7)
            && (i == 0 || !b[i - 1].is_ascii_alphanumeric())
            && !digit(i + 8);
        if is_num {
            let mut end = i + 8;
            if b.get(end) == Some(&b'.') && digit(end + 1) && digit(end + 2) {
                end += 3;
            }
            let n = &text[i..end];
            // Only as a "Section" reference: the word within a few words before.
            let before = &text[..i];
            let tail = before.rsplit(['.', ';', ':']).next().unwrap_or(before);
            if tail.contains("Section") && valid_number(n) {
                out.push(n.to_string());
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// Edits references to Section `to` out of `section` (ADR-094): a "Related Sections" item
/// that names only it goes; in a list ("Section A and Section B") it drops out; elsewhere it
/// becomes "the Contract Documents". Returns how many references were edited.
pub fn strip_reference(section: &mut SpecSection, to: &str) -> usize {
    let pat = format!("Section {to}");
    let mut edits = 0;
    for part in &mut section.parts {
        for article in &mut part.articles {
            let mut keep = vec![];
            for mut p in std::mem::take(&mut article.paragraphs) {
                let mut drop = false;
                for _ in 0..16 {
                    let Some(i) = p.text.find(&pat) else { break };
                    // The span: the number, and the quoted title after it.
                    let mut end = i + pat.len();
                    let rest = &p.text[end..];
                    let mut dotted = false;
                    if let Some(q) = rest.strip_prefix(" \"").or_else(|| rest.strip_prefix(" “"))
                    {
                        if let Some(close) = q.find(['"', '”']) {
                            dotted = q[..close].ends_with('.');
                            end += rest.len() - q.len()
                                + close
                                + q[close..].chars().next().map_or(1, char::len_utf8);
                        }
                    }
                    let before = &p.text[..i];
                    let after = p.text[end..].to_string();
                    let lead = before.trim_start_matches(['>', ' ']);
                    edits += 1;
                    if let Some(next) = after.strip_prefix(" and ") {
                        p.text = format!("{before}{next}");
                    } else if lead.is_empty() {
                        drop = true;
                        break;
                    } else if let Some(b) = [" and ", ", ", " or "]
                        .iter()
                        .find_map(|c| before.strip_suffix(c))
                        .filter(|b| b.ends_with('"') || b.ends_with('”'))
                    {
                        p.text = format!("{b}{after}");
                    } else {
                        let stop = if dotted { "." } else { "" };
                        p.text = format!("{before}the Contract Documents{stop}{after}");
                    }
                }
                if !drop {
                    keep.push(p);
                }
            }
            article.paragraphs = keep;
        }
    }
    edits
}

/// The sections missing from, or excluded from, the book that it refers to, each once
/// with the sections referring to it (excluded targets first).
pub fn missing_references(book: &SpecBook) -> Vec<SpecReference> {
    let mut out: Vec<SpecReference> = vec![];
    for s in book.sections.iter().filter(|s| s.included) {
        for a in s.parts.iter().flat_map(|p| &p.articles) {
            for p in &a.paragraphs {
                for n in numbers_in(&p.text) {
                    if n == s.number {
                        continue;
                    }
                    let target = book.section(&n);
                    if target.is_some_and(|t| t.included) {
                        continue;
                    }
                    if let Some(r) = out.iter_mut().find(|r| r.to == n) {
                        if !r.from.contains(&s.number) {
                            r.from.push(s.number.clone());
                        }
                        continue;
                    }
                    out.push(SpecReference {
                        from: vec![s.number.clone()],
                        to: n.clone(),
                        title: entry(&n).map(|l| l.section.title.clone()),
                        excluded: target.is_some(),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.excluded.cmp(&a.excluded).then(a.to.cmp(&b.to)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::specs::{Paragraph, SpecSection};

    #[test]
    fn references_are_edited_out_the_way_a_spec_writer_would() {
        let mut s = crate::generate::from_library(
            crate::library::entry("04 72 00").unwrap(),
            &crate::features::Facts::default(),
        );
        let mentions = |s: &SpecSection| {
            s.parts
                .iter()
                .flat_map(|p| &p.articles)
                .flat_map(|a| &a.paragraphs)
                .filter(|p| p.text.contains("04 20 00"))
                .count()
        };
        assert!(mentions(&s) >= 3);
        let n = strip_reference(&mut s, "04 20 00");
        assert!(n >= 3);
        assert_eq!(mentions(&s), 0);
        let text: Vec<&str> = s
            .parts
            .iter()
            .flat_map(|p| &p.articles)
            .flat_map(|a| &a.paragraphs)
            .map(|p| p.text.as_str())
            .collect();
        assert!(
            text.iter()
                .any(|t| t.contains("requirements in the Contract Documents.")),
            "{text:?}"
        );
        // A list keeps the other section.
        let mut list = SpecSection {
            parts: vec![studio_core::specs::SpecPart {
                title: "GENERAL".into(),
                articles: vec![studio_core::specs::Article {
                    title: "RELATED SECTIONS".into(),
                    paragraphs: vec![
                        Paragraph::new(0, "Section 06 41 13 \"Wood-Veneer-Faced Architectural Cabinets\" and Section 06 41 16 \"Plastic-Laminate-Faced Architectural Cabinets\" for casework."),
                        Paragraph::new(0, "Section 01 81 13 \"Sustainable Design Requirements\" for documentation."),
                    ],
                }],
            }],
            ..s.clone()
        };
        assert_eq!(strip_reference(&mut list, "06 41 16"), 1);
        assert_eq!(strip_reference(&mut list, "01 81 13"), 1);
        let a = &list.parts[0].articles[0].paragraphs;
        assert_eq!(a.len(), 1);
        assert_eq!(
            a[0].text,
            "Section 06 41 13 \"Wood-Veneer-Faced Architectural Cabinets\" for casework."
        );
    }

    #[test]
    fn section_numbers_are_found_only_as_references() {
        assert_eq!(
            numbers_in("Section 09 91 23 \"Interior Painting\" and Section 23 74 16.11 for units."),
            vec!["09 91 23", "23 74 16.11"]
        );
        assert_eq!(
            numbers_in("Sections 07 92 00 and 09 29 00."),
            vec!["07 92 00", "09 29 00"]
        );
        // Not a reference: a phone number, a standard or a date.
        assert!(numbers_in("Call 12 34 56 78. ASTM C 1396. 2026 10 01").is_empty());
    }

    #[test]
    fn missing_and_excluded_targets_are_listed_once() {
        let mut a = SpecSection::blank("09 29 00", "Gypsum Board");
        a.parts[0].articles[0].paragraphs = vec![
            Paragraph::new(1, "Section 09 91 23 \"Interior Painting\" for finishes."),
            Paragraph::new(
                1,
                "Section 09 91 23 again, and Section 07 92 00 \"Joint Sealants\".",
            ),
            Paragraph::new(1, "Section 06 10 00 \"Rough Carpentry\"."),
        ];
        let mut sealants = SpecSection::blank("07 92 00", "Joint Sealants");
        sealants.included = false;
        let carpentry = SpecSection::blank("06 10 00", "Rough Carpentry");
        let book = SpecBook {
            style: "csi-classic".into(),
            issue: String::new(),
            date: String::new(),
            sections: vec![carpentry, sealants, a],
        };
        let r = missing_references(&book);
        let got: Vec<(&str, bool)> = r.iter().map(|x| (x.to.as_str(), x.excluded)).collect();
        assert_eq!(got, vec![("07 92 00", true), ("09 91 23", false)]);
        assert_eq!(r[1].from, vec!["09 29 00".to_string()]);
    }
}

#[cfg(test)]
mod library_refs {
    /// Lists cross-references in the library to sections it doesn't have.
    #[test]
    #[ignore]
    fn print_library_refs() {
        let mut counts = std::collections::BTreeMap::new();
        for l in crate::library::library() {
            for a in l.section.parts.iter().flat_map(|p| &p.articles) {
                for p in &a.paragraphs {
                    for n in super::numbers_in(&p.text) {
                        if crate::library::entry(&n).is_none() {
                            let i = p.text.find(&n).unwrap_or(0);
                            let t: String = p.text[i..].chars().take(60).collect();
                            counts.entry(n).or_insert_with(Vec::new).push(t);
                        }
                    }
                }
            }
        }
        for (n, v) in &counts {
            println!("{n} x{}  {}", v.len(), v[0]);
        }
    }
}
