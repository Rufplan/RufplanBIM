//! Fix with Claude (ADR-094): the findings Fix Issues can't plan on its own go to Claude
//! with the project, the element ids involved, the types to choose from and the spec book.
//! Claude answers with advice and concrete operations from a small vocabulary; each one
//! is checked against the model and becomes an ordinary [`Fix`], applied (and undone) like
//! the planner's own.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use studio_core::specs::{Article, Paragraph, SectionKind, SectionOrigin, SpecPart, SpecSection};
use studio_core::{Category as Cat, Document, ElementData, ElementId};
use studio_regen::Model;
use ts_rs::TS;

use crate::fix::{Action, Fix};
use crate::Report;

/// Claude's answer for one finding: its advice, and the fixes made from its operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "QaSuggestion")]
pub struct Suggestion {
    pub finding: String,
    pub advice: String,
    pub fixes: Vec<Fix>,
    /// Operations that didn't check out against the model, in words.
    pub dropped: Vec<String>,
}

pub fn prompt() -> String {
    "You are an experienced architect doing QA/QC on a residential or commercial project \
     in Rufplan Studio, a BIM tool like Revit. Each finding below is one the automated \
     fixer couldn't resolve. For each one, say briefly how you'd fix it (advice), and give \
     the operations that make the fix in the model or the project manual.\n\
     Rules:\n\
     - Use only element and type ids given in the message. Never invent ids.\n\
     - Prefer the smallest change that resolves the finding and keeps the design intent.\n\
     - When the fix needs facts only the architect knows (a client's name, an address, a \
       consultant), give advice and no operations; never invent people, companies, \
       addresses or numbers.\n\
     - For a specification section that isn't in the manual, write it with write_spec_section: \
       CSI three-part format (PART 1 GENERAL, PART 2 PRODUCTS, PART 3 EXECUTION), concise \
       paragraphs suited to this project, standards cited by designation (ASTM, ANSI), no \
       manufacturer names unless as 'or equal'. Or, if the section isn't needed on this \
       project, edit the reference out with strip_spec_reference.\n\
     - roof_slope takes rise per 12 (0.25 for 1/4\":12).\n\
     - set_property sets a Properties value by its key (name, mark, number, sill, offset, \
       height, ...); lengths are written like 3'-0\" or 36\".\n\
     Answer with the fix_suggestions tool."
        .into()
}

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "suggestions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "finding": { "type": "string", "description": "The finding's id" },
                        "advice": { "type": "string" },
                        "ops": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "op": { "type": "string", "enum": [
                                        "set_property", "swap_type", "roof_slope", "delete",
                                        "add_library_section", "write_spec_section",
                                        "strip_spec_reference"
                                    ] },
                                    "id": { "type": "string" },
                                    "ids": { "type": "array", "items": { "type": "string" } },
                                    "key": { "type": "string" },
                                    "value": { "type": "string" },
                                    "type_id": { "type": "string" },
                                    "rise_per_12": { "type": "number" },
                                    "number": { "type": "string", "description": "MasterFormat number, \"09 93 00\"" },
                                    "title": { "type": "string" },
                                    "articles": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "part": { "type": "string", "enum": ["GENERAL", "PRODUCTS", "EXECUTION"] },
                                                "title": { "type": "string" },
                                                "paragraphs": { "type": "array", "items": { "type": "string" } }
                                            },
                                            "required": ["part", "title", "paragraphs"]
                                        }
                                    },
                                    "from": { "type": "array", "items": { "type": "string" }, "description": "strip_spec_reference: the sections whose text names the other" },
                                    "to": { "type": "string", "description": "strip_spec_reference: the section number they name" }
                                },
                                "required": ["op"]
                            }
                        }
                    },
                    "required": ["finding", "advice", "ops"]
                }
            }
        },
        "required": ["suggestions"]
    })
}

fn describe(doc: &Document, id: ElementId) -> String {
    let Ok(d) = doc.data(id) else {
        return format!("{id} (missing)");
    };
    let ty = d
        .type_id()
        .and_then(|t| doc.data(t).ok())
        .map(|t| format!(", type {}", t.name()))
        .unwrap_or_default();
    let level = d
        .level()
        .and_then(|l| doc.data(l).ok())
        .map(|l| format!(", {}", l.name()))
        .unwrap_or_default();
    format!("{id}: {:?} {}{ty}{level}", d.category(), d.name())
}

/// The message for Claude: the project, the findings with their elements, the types to
/// choose from and the manual's sections.
pub fn digest(doc: &Document, model: &Model, report: &Report, ids: &[String]) -> String {
    let mut s = crate::report::digest(doc, model, report);
    s.push_str("\nFINDINGS TO FIX:\n");
    for f in report.findings.iter().filter(|f| ids.contains(&f.id)) {
        s.push_str(&format!(
            "- id {} [{} {:?}] {}\n  {}\n  Suggested: {}\n",
            f.id, f.rule, f.severity, f.title, f.detail, f.fix
        ));
        for e in &f.elements {
            s.push_str(&format!("  element {}\n", describe(doc, *e)));
        }
    }
    s.push_str("\nTYPES (id: name):\n");
    for cat in [
        Cat::DoorType,
        Cat::WindowType,
        Cat::WallType,
        Cat::FloorType,
        Cat::CeilingType,
        Cat::RoofType,
        Cat::RailingType,
    ] {
        for e in doc.of(cat) {
            let layers = match &e.data {
                ElementData::WallType { layers, .. }
                | ElementData::FloorType { layers, .. }
                | ElementData::RoofType { layers, .. } => format!(
                    " [{}]",
                    layers
                        .iter()
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" / ")
                ),
                _ => String::new(),
            };
            s.push_str(&format!(
                "- {:?} {}: {}{layers}\n",
                cat,
                e.id,
                e.data.name()
            ));
        }
    }
    if let Some(book) = studio_core::specs::book(doc) {
        s.push_str("\nPROJECT MANUAL SECTIONS:\n");
        for sec in &book.sections {
            s.push_str(&format!(
                "- {} {}{}\n",
                sec.number,
                sec.title,
                if sec.included { "" } else { " (excluded)" }
            ));
        }
    }
    s
}

#[derive(Debug, Deserialize)]
struct Answer {
    suggestions: Vec<RawSuggestion>,
}

#[derive(Debug, Deserialize)]
struct RawSuggestion {
    finding: String,
    advice: String,
    #[serde(default)]
    ops: Vec<RawOp>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawOp {
    op: String,
    id: Option<String>,
    ids: Vec<String>,
    key: Option<String>,
    value: Option<String>,
    type_id: Option<String>,
    rise_per_12: Option<f64>,
    number: Option<String>,
    title: Option<String>,
    articles: Vec<RawArticle>,
    from: Vec<String>,
    to: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawArticle {
    part: String,
    title: String,
    paragraphs: Vec<String>,
}

/// The type category an element's type must be.
fn type_category(c: Cat) -> Option<Cat> {
    Some(match c {
        Cat::Door => Cat::DoorType,
        Cat::Window => Cat::WindowType,
        Cat::Wall => Cat::WallType,
        Cat::Floor => Cat::FloorType,
        _ => return None,
    })
}

fn element(doc: &Document, s: Option<&String>) -> Result<ElementId, String> {
    let s = s.ok_or("no element id")?;
    let id: ElementId = s
        .parse()
        .map_err(|_| format!("\"{s}\" isn't an element id"))?;
    doc.data(id)
        .map_err(|_| format!("there's no element {s}"))?;
    Ok(id)
}

/// One operation as a fix, checked against the model.
fn to_fix(doc: &Document, finding: &str, op: RawOp) -> Result<Fix, String> {
    let fix = |title: String, change: String, design: bool, action: Action| Fix {
        finding: finding.into(),
        title,
        change,
        design_change: design,
        action,
    };
    match op.op.as_str() {
        "set_property" => {
            let id = element(doc, op.id.as_ref())?;
            let key = op.key.ok_or("set_property needs a key")?;
            let value = op.value.ok_or("set_property needs a value")?;
            let what = doc.data(id).map(|d| d.name()).unwrap_or_default();
            Ok(fix(
                format!("Set {what}'s {} to {value}", key.replace('_', " ")),
                format!("{key} → {value}"),
                false,
                Action::SetText { id, key, value },
            ))
        }
        "swap_type" => {
            let t = element(doc, op.type_id.as_ref())?;
            let tc = doc
                .data(t)
                .map(|d| d.category())
                .map_err(|e| e.to_string())?;
            let mut ids = vec![];
            for s in &op.ids {
                let id = element(doc, Some(s))?;
                let c = doc
                    .data(id)
                    .map(|d| d.category())
                    .map_err(|e| e.to_string())?;
                if type_category(c) != Some(tc) {
                    return Err(format!("{s} can't take a {tc:?}"));
                }
                ids.push(id);
            }
            if ids.is_empty() {
                return Err("swap_type needs elements".into());
            }
            let name = doc.data(t).map(|d| d.name()).unwrap_or_default();
            Ok(fix(
                format!(
                    "Change {} element{} to {name}",
                    ids.len(),
                    if ids.len() == 1 { "" } else { "s" }
                ),
                format!("type → {name}"),
                tc == Cat::WallType,
                Action::SwapType { ids, type_id: t },
            ))
        }
        "roof_slope" => {
            let id = element(doc, op.id.as_ref())?;
            if !matches!(doc.data(id), Ok(ElementData::Roof { .. })) {
                return Err("roof_slope is for roofs".into());
            }
            let r = op.rise_per_12.ok_or("roof_slope needs rise_per_12")?;
            if !(0.0..=24.0).contains(&r) {
                return Err(format!("{r}:12 isn't a roof slope"));
            }
            Ok(fix(
                format!("Slope the roof {r}:12"),
                format!("roof slope → {r}:12"),
                true,
                Action::RoofSlope {
                    id,
                    slope: r / 12.0,
                },
            ))
        }
        "delete" => {
            let ids = op
                .ids
                .iter()
                .map(|s| element(doc, Some(s)))
                .collect::<Result<Vec<_>, _>>()?;
            if ids.is_empty() {
                return Err("delete needs elements".into());
            }
            Ok(fix(
                format!(
                    "Delete {} element{}",
                    ids.len(),
                    if ids.len() == 1 { "" } else { "s" }
                ),
                "deleted".into(),
                true,
                Action::Delete { ids },
            ))
        }
        "add_library_section" => {
            let n = op.number.ok_or("add_library_section needs a number")?;
            let lib = studio_specs::library::entry(&n)
                .ok_or_else(|| format!("{n} isn't in the library"))?;
            Ok(fix(
                format!("Add Section {n} {} from the library", lib.section.title),
                format!("{n} added"),
                false,
                Action::AddSpecs { numbers: vec![n] },
            ))
        }
        "write_spec_section" => {
            let n = op.number.ok_or("write_spec_section needs a number")?;
            if !studio_core::specs::valid_number(&n) {
                return Err(format!("\"{n}\" isn't a MasterFormat number"));
            }
            let title = op.title.unwrap_or_default().trim().to_uppercase();
            if title.is_empty() || op.articles.is_empty() {
                return Err(format!("Section {n} needs a title and text"));
            }
            let mut parts: Vec<SpecPart> = ["GENERAL", "PRODUCTS", "EXECUTION"]
                .iter()
                .map(|p| SpecPart {
                    title: (*p).into(),
                    articles: vec![],
                })
                .collect();
            let mut count = 0;
            for a in op.articles {
                let i = match a.part.to_uppercase().as_str() {
                    "PRODUCTS" => 1,
                    "EXECUTION" => 2,
                    _ => 0,
                };
                count += a.paragraphs.len();
                parts[i].articles.push(Article {
                    title: a.title.trim().to_uppercase(),
                    paragraphs: a
                        .paragraphs
                        .into_iter()
                        .map(|t| Paragraph::new(0, t.trim()))
                        .collect(),
                });
            }
            parts.retain(|p| !p.articles.is_empty());
            Ok(fix(
                format!("Write Section {n} {title}"),
                format!("{count} paragraphs written by Claude, added to the manual"),
                false,
                Action::AddSection {
                    section: SpecSection {
                        number: n,
                        title,
                        kind: SectionKind::ThreePart,
                        parts,
                        included: true,
                        origin: SectionOrigin::Claude,
                        edited: false,
                    },
                },
            ))
        }
        "strip_spec_reference" => {
            // "to" is the section named; Claude sometimes puts it in "number".
            let to = op
                .to
                .or(op.number)
                .ok_or("strip_spec_reference needs the section it names")?;
            if op.from.is_empty() {
                return Err("strip_spec_reference needs the sections to edit".into());
            }
            Ok(fix(
                format!("Edit the reference to {to} out of {}", op.from.join(", ")),
                "the reference edited out of the section text".into(),
                false,
                Action::StripSpecRef { from: op.from, to },
            ))
        }
        other => Err(format!("unknown operation {other}")),
    }
}

/// Claude's answer as suggestions, each operation checked against the model.
pub fn suggestions(doc: &Document, answer: Value) -> Result<Vec<Suggestion>, String> {
    let a: Answer =
        serde_json::from_value(answer).map_err(|e| format!("Claude's answer doesn't fit: {e}"))?;
    Ok(a.suggestions
        .into_iter()
        .map(|s| {
            let mut fixes = vec![];
            let mut dropped = vec![];
            for op in s.ops {
                let name = op.op.clone();
                match to_fix(doc, &s.finding, op) {
                    Ok(f) => fixes.push(f),
                    Err(e) => dropped.push(format!("{name}: {e}")),
                }
            }
            Suggestion {
                finding: s.finding,
                advice: s.advice,
                fixes,
                dropped,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claudes_operations_are_checked_against_the_model() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = studio_core::ops::first_of(&doc, Cat::WallType).unwrap();
        let wall = studio_core::ops::create_wall(
            &mut doc,
            wt,
            l1,
            studio_geom::Pt::new(0.0, 0.0),
            studio_geom::Pt::new(5000.0, 0.0),
        )
        .unwrap();
        let door_type = studio_core::ops::first_of(&doc, Cat::DoorType).unwrap();
        let answer = json!({ "suggestions": [{
            "finding": "f1",
            "advice": "Write the section, and fix the wall.",
            "ops": [
                { "op": "write_spec_section", "number": "09 93 00", "title": "Staining and Transparent Finishing",
                  "articles": [
                    { "part": "GENERAL", "title": "Summary", "paragraphs": ["Section includes stains."] },
                    { "part": "EXECUTION", "title": "Application", "paragraphs": ["Apply two coats.", "Wipe off excess."] }
                  ] },
                { "op": "swap_type", "ids": [wall.to_string()], "type_id": door_type.to_string() },
                { "op": "set_property", "id": "not-an-id", "key": "name", "value": "x" },
                { "op": "strip_spec_reference", "from": ["01 73 00"], "to": "01 91 13" }
            ]
        }]});
        let s = suggestions(&doc, answer).unwrap();
        assert_eq!(s.len(), 1);
        // The wall can't take a door type, and the id is made up: both dropped.
        assert_eq!(s[0].dropped.len(), 2, "{:?}", s[0].dropped);
        assert_eq!(s[0].fixes.len(), 2);
        match &s[0].fixes[0].action {
            Action::AddSection { section } => {
                assert_eq!(section.title, "STAINING AND TRANSPARENT FINISHING");
                assert_eq!(section.parts.len(), 2);
                assert_eq!(section.origin, SectionOrigin::Claude);
            }
            a => panic!("{a:?}"),
        }
    }
}
