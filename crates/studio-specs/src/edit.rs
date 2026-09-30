//! Edit Specs with Claude (ADR-085): Claude answers a request with a list of operations on
//! the book (rewrite a section, add, remove, include or exclude, renumber); they're checked
//! and previewed as a paragraph diff, then applied as one undo step.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use studio_core::specs::{
    valid_number, Article, Paragraph, SectionKind, SectionOrigin, SpecBook, SpecPart, SpecSection,
};
use ts_rs::TS;

use crate::features::Facts;
use crate::generate::from_library;
use crate::library::{entry, library};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpecOpArticle {
    pub title: String,
    /// Paragraphs in the shorthand: one leading ">" per level.
    pub paragraphs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpecOpPart {
    pub title: String,
    pub articles: Vec<SpecOpArticle>,
}

/// One operation. `op`: "replace_section", "add_section", "remove_section", "include",
/// "exclude", "renumber".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpecOp {
    pub op: String,
    pub number: String,
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub new_number: Option<String>,
    /// The section's whole text (replace, or add a section not in the library).
    #[serde(default)]
    #[ts(optional)]
    pub parts: Option<Vec<SpecOpPart>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpecEdit {
    #[serde(default)]
    pub operations: Vec<SpecOp>,
    #[serde(default)]
    pub summary: String,
    /// Why nothing could be done, or a note for the architect.
    #[serde(default)]
    pub message: String,
}

fn to_parts(parts: &[SpecOpPart]) -> Vec<SpecPart> {
    parts
        .iter()
        .map(|p| SpecPart {
            title: p.title.trim().to_uppercase(),
            articles: p
                .articles
                .iter()
                .map(|a| Article {
                    title: a.title.trim().to_uppercase(),
                    paragraphs: a
                        .paragraphs
                        .iter()
                        .filter(|t| !t.trim_start_matches('>').trim().is_empty())
                        .map(|t| Paragraph::parse(t))
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// A three-part section needs its three parts; a document one part.
fn check_parts(kind: SectionKind, parts: &[SpecPart], number: &str) -> Result<(), String> {
    match kind {
        SectionKind::ThreePart if parts.len() != 3 => Err(format!(
            "section {number} needs PART 1 GENERAL, PART 2 PRODUCTS and PART 3 EXECUTION"
        )),
        _ if parts.iter().all(|p| p.articles.is_empty()) => {
            Err(format!("section {number} has no articles"))
        }
        _ => Ok(()),
    }
}

/// Applies the operations to a copy of the book.
pub fn apply(book: &SpecBook, edit: &SpecEdit, facts: &Facts) -> Result<SpecBook, String> {
    let mut b = book.clone();
    for op in &edit.operations {
        let n = op.number.trim();
        let find = |b: &SpecBook| b.sections.iter().position(|s| s.number == n);
        match op.op.as_str() {
            "replace_section" => {
                let i = find(&b).ok_or_else(|| format!("there's no section {n} to rewrite"))?;
                let s = &mut b.sections[i];
                if s.kind.generated() {
                    return Err(format!(
                        "{n} is written from the project; edit Project Info instead"
                    ));
                }
                if let Some(parts) = &op.parts {
                    let parts = to_parts(parts);
                    check_parts(s.kind, &parts, n)?;
                    s.parts = parts;
                }
                if let Some(t) = op.title.as_ref().filter(|t| !t.trim().is_empty()) {
                    s.title = t.trim().to_uppercase();
                }
                s.edited = true;
                s.origin = SectionOrigin::Claude;
            }
            "add_section" => {
                if !valid_number(n) {
                    return Err(format!("\"{n}\" isn't a MasterFormat number"));
                }
                if find(&b).is_some() {
                    return Err(format!("section {n} is already in the book"));
                }
                let s = match (&op.parts, entry(n)) {
                    (Some(parts), _) => {
                        let parts = to_parts(parts);
                        let kind = if parts.len() == 1 && parts[0].title.is_empty() {
                            SectionKind::Document
                        } else {
                            SectionKind::ThreePart
                        };
                        check_parts(kind, &parts, n)?;
                        SpecSection {
                            number: n.into(),
                            title: op
                                .title
                                .clone()
                                .or_else(|| entry(n).map(|l| l.section.title.clone()))
                                .unwrap_or_default()
                                .trim()
                                .to_uppercase(),
                            kind,
                            parts,
                            included: true,
                            origin: SectionOrigin::Claude,
                            edited: true,
                        }
                    }
                    (None, Some(l)) => from_library(l, facts),
                    (None, None) => {
                        return Err(format!("section {n} isn't in the library; give its text"))
                    }
                };
                if s.title.is_empty() {
                    return Err(format!("section {n} needs a title"));
                }
                b.sections.push(s);
            }
            "remove_section" => {
                let i = find(&b).ok_or_else(|| format!("there's no section {n} to remove"))?;
                b.sections.remove(i);
            }
            "include" | "exclude" => {
                let i = find(&b).ok_or_else(|| format!("there's no section {n}"))?;
                b.sections[i].included = op.op == "include";
            }
            "renumber" => {
                let to = op.new_number.as_deref().map(str::trim).unwrap_or_default();
                if !valid_number(to) {
                    return Err(format!("\"{to}\" isn't a MasterFormat number"));
                }
                let i = find(&b).ok_or_else(|| format!("there's no section {n}"))?;
                if b.sections.iter().any(|s| s.number == to) {
                    return Err(format!("section {to} is already in the book"));
                }
                b.sections[i].number = to.into();
                if let Some(t) = op.title.as_ref().filter(|t| !t.trim().is_empty()) {
                    b.sections[i].title = t.trim().to_uppercase();
                }
                b.sections[i].edited = true;
            }
            other => return Err(format!("unknown operation \"{other}\"")),
        }
    }
    b.sort();
    Ok(b)
}

/// A section as lines: headings marked, paragraphs in the shorthand (the diff's unit and
/// what Claude reads).
pub fn lines(s: &SpecSection) -> Vec<String> {
    let mut out = vec![];
    for p in &s.parts {
        if !p.title.is_empty() {
            out.push(format!("## PART: {}", p.title));
        }
        for a in &p.articles {
            out.push(format!("### {}", a.title));
            out.extend(a.paragraphs.iter().map(Paragraph::shorthand));
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SpecDiffLine {
    /// "+", "-" or " " (context).
    pub sign: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecChange {
    pub number: String,
    pub title: String,
    /// "added", "removed", "changed", "included", "excluded", "renumbered".
    pub kind: String,
    pub added: usize,
    pub removed: usize,
    /// The changed lines with a line of context, up to 80.
    pub lines: Vec<SpecDiffLine>,
}

/// Line diff (longest common subsequence).
pub fn diff(a: &[String], b: &[String]) -> Vec<SpecDiffLine> {
    let (n, m) = (a.len(), b.len());
    let mut l = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            l[i][j] = if a[i] == b[j] {
                l[i + 1][j + 1] + 1
            } else {
                l[i + 1][j].max(l[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = vec![];
    let line = |sign: &str, t: &String| SpecDiffLine {
        sign: sign.into(),
        text: t.clone(),
    };
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            out.push(line(" ", &a[i]));
            i += 1;
            j += 1;
        } else if j < m && (i == n || l[i][j + 1] >= l[i + 1][j]) {
            out.push(line("+", &b[j]));
            j += 1;
        } else {
            out.push(line("-", &a[i]));
            i += 1;
        }
    }
    out
}

/// Keeps the changed lines and one line of context around each run.
fn trimmed(d: Vec<SpecDiffLine>) -> Vec<SpecDiffLine> {
    let keep: Vec<bool> = (0..d.len())
        .map(|i| {
            let changed = |k: usize| d.get(k).is_some_and(|x| x.sign != " ");
            changed(i) || (i > 0 && changed(i - 1)) || changed(i + 1)
        })
        .collect();
    d.into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(x, _)| x)
        .take(80)
        .collect()
}

/// What an edit changes, section by section.
pub fn preview(before: &SpecBook, after: &SpecBook) -> Vec<SpecChange> {
    let mut out = vec![];
    let change = |s: &SpecSection, kind: &str, d: Vec<SpecDiffLine>| {
        let added = d.iter().filter(|x| x.sign == "+").count();
        let removed = d.iter().filter(|x| x.sign == "-").count();
        SpecChange {
            number: s.number.clone(),
            title: s.title.clone(),
            kind: kind.into(),
            added,
            removed,
            lines: trimmed(d),
        }
    };
    for s in &before.sections {
        match after.section(&s.number) {
            None => {
                // Renumbered, or removed.
                let moved = after.sections.iter().find(|x| {
                    !before.sections.iter().any(|y| y.number == x.number) && x.parts == s.parts
                });
                match moved {
                    Some(m) => {
                        let mut c = change(m, "renumbered", vec![]);
                        c.lines = vec![SpecDiffLine {
                            sign: " ".into(),
                            text: format!("{} → {}", s.number, m.number),
                        }];
                        out.push(c);
                    }
                    None => out.push(change(s, "removed", vec![])),
                }
            }
            Some(t) => {
                if t.parts != s.parts || t.title != s.title {
                    let mut d = diff(&lines(s), &lines(t));
                    if t.title != s.title {
                        d.insert(
                            0,
                            SpecDiffLine {
                                sign: "-".into(),
                                text: format!("# {}", s.title),
                            },
                        );
                        d.insert(
                            1,
                            SpecDiffLine {
                                sign: "+".into(),
                                text: format!("# {}", t.title),
                            },
                        );
                    }
                    out.push(change(t, "changed", d));
                }
                if t.included != s.included {
                    out.push(change(
                        t,
                        if t.included { "included" } else { "excluded" },
                        vec![],
                    ));
                }
            }
        }
    }
    for t in &after.sections {
        let renumbered = out
            .iter()
            .any(|c| c.kind == "renumbered" && c.number == t.number);
        if before.section(&t.number).is_none() && !renumbered {
            let d: Vec<SpecDiffLine> = lines(t)
                .into_iter()
                .map(|text| SpecDiffLine {
                    sign: "+".into(),
                    text,
                })
                .collect();
            out.push(change(t, "added", d));
        }
    }
    out.sort_by(|a, b| a.number.cmp(&b.number));
    out
}

/// What Claude is given: the outline of the book, the library sections not in it, and the
/// full text of the sections in focus (the selected one, and any the request names).
pub fn describe(book: &SpecBook, focus: Option<&str>, prompt: &str, facts: &Facts) -> String {
    let mut s = String::from("PROJECT MANUAL OUTLINE (number, title, [excluded], [edited]):\n");
    for x in &book.sections {
        s.push_str(&format!(
            "{} {}{}{}\n",
            x.number,
            x.title,
            if x.included { "" } else { " [excluded]" },
            if x.edited { " [edited]" } else { "" }
        ));
    }
    s.push_str("\nLIBRARY SECTIONS NOT IN THE BOOK (can be added by number, with no text):\n");
    for l in library()
        .iter()
        .filter(|l| book.section(&l.section.number).is_none())
    {
        s.push_str(&format!("{} {}\n", l.section.number, l.section.title));
    }
    s.push_str("\nPROJECT FACTS:\n");
    for (k, v) in &facts.values {
        if !v.is_empty() {
            s.push_str(&format!("{k}: {v}\n"));
        }
    }
    let tags: Vec<&str> = facts.tags.iter().copied().collect();
    s.push_str(&format!("model features: {}\n", tags.join(", ")));
    // Sections in focus: the selected one and any named in the request by number or title.
    let p = prompt.to_uppercase();
    let named: Vec<&SpecSection> = book
        .sections
        .iter()
        .filter(|x| {
            !x.kind.generated()
                && (Some(x.number.as_str()) == focus
                    || p.contains(&x.number)
                    || (x.title.len() > 4 && p.contains(&x.title)))
        })
        .take(6)
        .collect();
    for x in named {
        s.push_str(&format!(
            "\nFULL TEXT OF SECTION {} - {} ({:?}):\n{}\n",
            x.number,
            x.title,
            x.kind,
            lines(x).join("\n")
        ));
    }
    if let Some(f) = focus {
        s.push_str(&format!("\nThe architect has section {f} open.\n"));
    }
    s
}

pub fn system_prompt() -> String {
    r#"You edit the project manual (the specification book) of a US building project in Rufplan Studio, a Revit-like BIM app, as the architect asks. Sections are CSI MasterFormat 2020 numbers in SectionFormat: PART 1 - GENERAL, PART 2 - PRODUCTS, PART 3 - EXECUTION, each with articles (upper-case titles like SUMMARY, ACTION SUBMITTALS, INSTALLATION) of paragraphs. Answer only by calling the edit_specs tool.

Operations (run in order as one undoable change):
- replace_section: number, optional title, and parts = the WHOLE new text of the section (all three parts with all their articles, keeping everything you aren't asked to change word for word). Only for sections whose full text you were given.
- add_section: number and title; with parts to write a new section, or without parts to add a library section by number.
- remove_section, include, exclude: number.
- renumber: number and new_number (and optional title).

Paragraph shorthand: each paragraph is one string; its level is set by leading ">" characters: none = A., ">" = 1., ">>" = a., ">>>" = 1). Never type the numbers or letters. A paragraph ending in ":" introduces the subparagraphs under it. Cross-reference other sections as: Section 09 91 23 "Interior Painting".

Write like a careful US spec writer: imperative mood, the Contractor, Owner and Architect capitalized, imperial units, real standards by designation (ASTM, ANSI, UL, NFPA, BHMA…), real manufacturers under "Manufacturers: Subject to compliance with requirements, provide products by one of the following:". Keep edits tight: change what was asked, and keep the rest of a section as it was. When a change affects other sections (e.g. a product change that touches its related sections) and you have their text, update them too; otherwise mention them in message.
If the request can't be done with these operations, return no operations and say why in message.
summary: one short line describing the whole change, e.g. "Makes all gypsum board 5/8 inch Type X and adds a mold-resistant board for wet areas"."#
        .into()
}

pub fn schema() -> Value {
    let article = json!({
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "paragraphs": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["title", "paragraphs"]
    });
    let part = json!({
        "type": "object",
        "properties": {
            "title": { "type": "string", "description": "GENERAL, PRODUCTS or EXECUTION; empty for a Division 00 document" },
            "articles": { "type": "array", "items": article }
        },
        "required": ["title", "articles"]
    });
    json!({
        "type": "object",
        "properties": {
            "operations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "op": { "type": "string", "enum": ["replace_section", "add_section", "remove_section", "include", "exclude", "renumber"] },
                        "number": { "type": "string" },
                        "title": { "type": "string" },
                        "new_number": { "type": "string" },
                        "parts": { "type": "array", "items": part }
                    },
                    "required": ["op", "number"]
                }
            },
            "summary": { "type": "string" },
            "message": { "type": "string" }
        },
        "required": ["operations", "summary"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> SpecBook {
        SpecBook {
            style: "csi-classic".into(),
            issue: String::new(),
            date: String::new(),
            sections: vec![
                SpecSection::blank("09 29 00", "Gypsum Board"),
                SpecSection::blank("09 91 23", "Interior Painting"),
            ],
        }
    }

    fn part(title: &str, paras: &[&str]) -> SpecOpPart {
        SpecOpPart {
            title: title.into(),
            articles: vec![SpecOpArticle {
                title: "Summary".into(),
                paragraphs: paras.iter().map(|s| s.to_string()).collect(),
            }],
        }
    }

    #[test]
    fn a_rewrite_previews_as_a_diff_and_marks_the_section() {
        let b = book();
        let edit = SpecEdit {
            operations: vec![
                SpecOp {
                    op: "replace_section".into(),
                    number: "09 29 00".into(),
                    title: None,
                    new_number: None,
                    parts: Some(vec![
                        part("general", &["Section Includes:", ">Type X gypsum board."]),
                        part("PRODUCTS", &["Board: 5/8 inch Type X."]),
                        part("EXECUTION", &["Install."]),
                    ]),
                },
                SpecOp {
                    op: "exclude".into(),
                    number: "09 91 23".into(),
                    title: None,
                    new_number: None,
                    parts: None,
                },
                SpecOp {
                    op: "add_section".into(),
                    number: "07 92 00".into(),
                    title: None,
                    new_number: None,
                    parts: None,
                },
            ],
            summary: "Type X".into(),
            message: String::new(),
        };
        let after = apply(&b, &edit, &Facts::default()).unwrap();
        let s = after.section("09 29 00").unwrap();
        assert_eq!((s.origin, s.edited), (SectionOrigin::Claude, true));
        assert_eq!(s.parts[0].title, "GENERAL");
        assert_eq!(s.parts[0].articles[0].title, "SUMMARY");
        assert_eq!(
            s.parts[0].articles[0].paragraphs[1],
            Paragraph::new(1, "Type X gypsum board.")
        );
        assert!(!after.section("09 91 23").unwrap().included);
        let changes = preview(&b, &after);
        let kinds: Vec<(&str, &str)> = changes
            .iter()
            .map(|c| (c.number.as_str(), c.kind.as_str()))
            .collect();
        assert!(
            kinds.contains(&("09 29 00", "changed")) && kinds.contains(&("09 91 23", "excluded"))
        );
        let gyp = changes.iter().find(|c| c.number == "09 29 00").unwrap();
        assert!(gyp.added > 0 && gyp.removed > 0);
        assert!(gyp
            .lines
            .iter()
            .any(|l| l.sign == "+" && l.text == ">Type X gypsum board."));
        // Library sections added by number come with their text, when the library has it.
        if entry("07 92 00").is_some() {
            assert!(after.section("07 92 00").unwrap().paragraph_count() > 10);
        }
    }

    #[test]
    fn bad_operations_are_refused() {
        let b = book();
        let op = |op: &str, n: &str| SpecEdit {
            operations: vec![SpecOp {
                op: op.into(),
                number: n.into(),
                title: None,
                new_number: None,
                parts: None,
            }],
            summary: String::new(),
            message: String::new(),
        };
        assert!(apply(&b, &op("remove_section", "01 10 00"), &Facts::default()).is_err());
        assert!(apply(&b, &op("add_section", "09 29 00"), &Facts::default()).is_err());
        assert!(apply(&b, &op("add_section", "9 29"), &Facts::default()).is_err());
        assert!(apply(&b, &op("explode", "09 29 00"), &Facts::default()).is_err());
        // A rewrite has to keep three parts.
        let mut e = op("replace_section", "09 29 00");
        e.operations[0].parts = Some(vec![part("GENERAL", &["x"])]);
        assert!(apply(&b, &e, &Facts::default()).is_err());
    }

    #[test]
    fn diff_keeps_order() {
        let a: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let b: Vec<String> = ["a", "x", "c", "d"].iter().map(|s| s.to_string()).collect();
        let d: Vec<String> = diff(&a, &b)
            .iter()
            .map(|l| format!("{}{}", l.sign, l.text))
            .collect();
        assert_eq!(d, vec![" a", "+x", "-b", " c", "+d"]);
    }
}
