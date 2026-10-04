//! Editing the sheet index (ADR-113): opened from the sheet index on a sheet, its rows are
//! the project's sheets and the placeholder sheets (ADR-110) in index order. Numbers and
//! names edit the sheets themselves; rows can be added (a new sheet or a placeholder) and
//! put in any order, kept as Project Info's `rufplan.sheet_index.order` (no file-format
//! change). Without an order of the user's, the index reads in sheet-index order.

use serde::{Deserialize, Serialize};
use studio_core::params::ParamValue;
use studio_core::{ops, CoreError, CoreResult, Document, ElementData, ElementId, SheetSize};
use ts_rs::TS;

use crate::schedule::{placeholders, set_placeholders, PlaceholderSheet};

/// Project Info's parameter with the sheet numbers in the user's order.
pub const ORDER_KEY: &str = "rufplan.sheet_index.order";

/// A row of the sheet index: a sheet (`sheet` set), a new sheet to make (neither set), or a
/// placeholder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexRow {
    pub sheet: Option<ElementId>,
    pub number: String,
    pub name: String,
    pub placeholder: bool,
}

/// The user's order of sheet numbers, if any.
pub fn order(doc: &Document) -> Vec<String> {
    ops::project_info(doc)
        .and_then(|i| match doc.param(i, ORDER_KEY) {
            Some(ParamValue::Text(s)) => serde_json::from_str(s).ok(),
            _ => None,
        })
        .unwrap_or_default()
}

/// Puts `numbers` in the user's order: those it lists where it lists them, any others
/// (sheets made since) after the nearest listed number that comes before them in
/// sheet-index order.
pub fn arrange(numbers: &mut Vec<String>, order: &[String]) {
    if order.is_empty() {
        numbers.sort_by(|a, b| ops::sheet_cmp(a, b));
        return;
    }
    let mut out: Vec<String> = order
        .iter()
        .filter(|n| numbers.contains(n))
        .cloned()
        .collect();
    let mut rest: Vec<String> = numbers
        .iter()
        .filter(|n| !order.contains(n))
        .cloned()
        .collect();
    rest.sort_by(|a, b| ops::sheet_cmp(a, b));
    for n in rest {
        // After the last of its own series before it (A-101 for A-102), else the last
        // number before it.
        let series = |s: &str| s.split(['-', '.']).next().unwrap_or("").to_string();
        let before = |o: &String| ops::sheet_cmp(o, &n).is_lt();
        let at = out
            .iter()
            .rposition(|o| before(o) && series(o) == series(&n))
            .or_else(|| out.iter().rposition(before))
            .map_or(0, |i| i + 1);
        out.insert(at, n);
    }
    *numbers = out;
}

/// The sheet index's rows: the current stage's sheets (every sheet when none are assigned)
/// and the placeholders, in index order.
pub fn rows(doc: &Document) -> Vec<IndexRow> {
    rows_for(doc, current_stage(doc))
}

/// [`rows`] for the set of `stage` (every sheet for None).
pub fn rows_for(doc: &Document, stage: Option<ElementId>) -> Vec<IndexRow> {
    let mut all: Vec<IndexRow> = ops::stage_sheets(doc, stage)
        .into_iter()
        .filter_map(|id| match doc.data(id) {
            Ok(ElementData::Sheet { number, name, .. }) => Some(IndexRow {
                sheet: Some(id),
                number: number.clone(),
                name: name.clone(),
                placeholder: false,
            }),
            _ => None,
        })
        .collect();
    all.extend(placeholders(doc).into_iter().map(|p| IndexRow {
        sheet: None,
        number: p.number,
        name: p.name,
        placeholder: true,
    }));
    let mut numbers: Vec<String> = all.iter().map(|r| r.number.clone()).collect();
    arrange(&mut numbers, &order(doc));
    numbers
        .iter()
        .filter_map(|n| all.iter().find(|r| &r.number == n).cloned())
        .collect()
}

pub fn current_stage(doc: &Document) -> Option<ElementId> {
    ops::project_info(doc).and_then(|i| match doc.data(i) {
        Ok(ElementData::ProjectInfo { current_stage, .. }) => *current_stage,
        _ => None,
    })
}

/// Saves the edited index in one undo step: renumbers and renames its sheets, makes its new
/// sheets (at the project's sheet size), sets the placeholders to its placeholder rows and
/// keeps its order. With a `stage` (ADR-116), its sheets are that stage's set: the listed
/// sheets join it and the others leave it (with no stages assigned yet, every sheet is first
/// put in every stage, as it was shown). Without one, new sheets go in the current stage's
/// set and the rest stay as they are.
pub fn set_rows(doc: &mut Document, rows: &[IndexRow], stage: Option<ElementId>) -> CoreResult<()> {
    let info = ops::project_info(doc)
        .ok_or_else(|| CoreError::Invalid("project information is missing".into()))?;
    let rows: Vec<IndexRow> = rows
        .iter()
        .map(|r| IndexRow {
            number: r.number.trim().to_string(),
            name: r.name.trim().to_string(),
            ..r.clone()
        })
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    for r in &rows {
        if r.number.is_empty() {
            return Err(CoreError::Invalid("give each sheet a number".into()));
        }
        if !seen.insert(r.number.clone()) {
            return Err(CoreError::Invalid(format!(
                "sheet {} is listed twice",
                r.number
            )));
        }
    }
    // Sheets not in the rows keep their numbers, which the rows can't take.
    let listed: Vec<ElementId> = rows.iter().filter_map(|r| r.sheet).collect();
    for (id, number, _) in ops::sheets(doc) {
        if !listed.contains(&id) && seen.contains(&number) {
            return Err(CoreError::Invalid(format!("sheet {number} already exists")));
        }
    }
    let all_stages: Vec<ElementId> = ops::stages(doc).into_iter().map(|s| s.0).collect();
    let any_assigned = doc
        .of(studio_core::Category::Sheet)
        .any(|e| matches!(&e.data, ElementData::Sheet { stages, .. } if !stages.is_empty()));
    let sheet_ids: Vec<ElementId> = ops::sheets(doc).into_iter().map(|s| s.0).collect();
    // A new sheet's stages: the chosen stage, else the current one when the sets use stages.
    let staged: Vec<ElementId> = match stage {
        Some(s) => vec![s],
        None => current_stage(doc)
            .filter(|s| {
                doc.of(studio_core::Category::Sheet).any(
                    |e| matches!(&e.data, ElementData::Sheet { stages, .. } if stages.contains(s)),
                )
            })
            .into_iter()
            .collect(),
    };
    let size = doc
        .of(studio_core::Category::Sheet)
        .find_map(|e| match &e.data {
            ElementData::Sheet { size, .. } => Some(*size),
            _ => None,
        })
        .unwrap_or(SheetSize::ArchD);
    let order: Vec<String> = rows.iter().map(|r| r.number.clone()).collect();
    let mut default = order.clone();
    arrange(&mut default, &[]);
    let order_value = (default != order)
        .then(|| serde_json::to_string(&order).map(ParamValue::Text))
        .transpose()
        .map_err(|e| CoreError::Invalid(e.to_string()))?;

    let mark = doc.undo_depth();
    let done = (|| -> CoreResult<()> {
        doc.transact("Edit Sheet Index", |tx| {
            for r in rows.iter().filter(|r| !r.placeholder) {
                match r.sheet {
                    Some(id) => tx.modify(id, |e| {
                        if let ElementData::Sheet { number, name, .. } = e {
                            *number = r.number.clone();
                            *name = r.name.clone();
                        }
                    })?,
                    None => {
                        tx.insert(ElementData::Sheet {
                            number: r.number.clone(),
                            name: r.name.clone(),
                            size,
                            stages: staged.clone(),
                        });
                    }
                }
            }
            // The stage's set is the listed sheets.
            if let Some(s) = stage {
                for id in &sheet_ids {
                    let listed = listed.contains(id);
                    tx.modify(*id, |e| {
                        if let ElementData::Sheet { stages, .. } = e {
                            if !any_assigned {
                                *stages = all_stages.clone();
                            }
                            stages.retain(|x| *x != s);
                            if listed {
                                stages.push(s);
                            }
                        }
                    })?;
                }
            }
            tx.set_param(info, ORDER_KEY, order_value.clone())
        })?;
        set_placeholders(
            doc,
            rows.iter()
                .filter(|r| r.placeholder)
                .map(|r| PlaceholderSheet {
                    number: r.number.clone(),
                    name: r.name.clone(),
                })
                .collect(),
        )
    })();
    if let Err(e) = done {
        while doc.undo_depth() > mark {
            let _ = doc.undo();
        }
        return Err(e);
    }
    doc.merge_undo(mark, "Edit Sheet Index");
    Ok(())
}

/// An issue the index can show the sheet list of (ADR-116): a phase's deliverable, with the
/// stage whose set it is. The CD phase issues at 50%, 90% and 100% and for permit, all from
/// the CD stage's set.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexIssue {
    pub key: String,
    pub label: String,
    /// The stage's abbreviation (SD, DD, CD…).
    pub phase: String,
    pub stage: ElementId,
}

/// The project's issues, in stage order.
pub fn issues(doc: &Document) -> Vec<IndexIssue> {
    let mut out = vec![];
    for (stage, name, abbr) in ops::stages(doc) {
        let labels: Vec<String> = match abbr.as_str() {
            "CD" => [
                "50% Construction Documents",
                "90% Construction Documents",
                "100% Construction Documents",
                "Permit Set",
            ]
            .map(String::from)
            .to_vec(),
            a => {
                let d = crate::sets::deliverables_of(a);
                if d.is_empty() {
                    vec![name.clone()]
                } else {
                    d.iter().map(|x| x.1.to_string()).collect()
                }
            }
        };
        for (i, label) in labels.into_iter().enumerate() {
            out.push(IndexIssue {
                key: format!("{abbr}:{i}"),
                label,
                phase: abbr.clone(),
                stage,
            });
        }
    }
    out
}

/// A stage's typical sheet list: the sheets in its set, and the sheets the Sheet Sets would
/// give that phase for `building` (ADR-032) that it doesn't have yet: a sheet already in the
/// project as itself, else as a new row. In index order.
pub fn typical(
    doc: &Document,
    stage: ElementId,
    building: crate::sets::BuildingType,
) -> Vec<IndexRow> {
    let mut rows = rows_for(doc, Some(stage));
    let Some(abbr) = ops::stages(doc)
        .into_iter()
        .find(|s| s.0 == stage)
        .map(|s| s.2)
    else {
        return rows;
    };
    let size = doc
        .of(studio_core::Category::Sheet)
        .find_map(|e| match &e.data {
            ElementData::Sheet { size, .. } => Some(*size),
            _ => None,
        })
        .unwrap_or(SheetSize::ArchD);
    let plan = crate::sets::plan(
        doc,
        &crate::sets::SetOptions {
            building_type: building,
            phases: vec![abbr.clone()],
            size,
            jurisdiction: None,
            consultants: false,
        },
    );
    let sheets = ops::sheets(doc);
    for p in plan.sheets.iter().filter(|p| p.phases.contains(&abbr)) {
        if rows.iter().any(|r| r.number == p.number) {
            continue;
        }
        let have = sheets.iter().find(|s| s.1 == p.number);
        rows.push(IndexRow {
            sheet: have.map(|s| s.0),
            number: p.number.clone(),
            name: have.map_or_else(|| p.name.clone(), |s| s.2.clone()),
            placeholder: false,
        });
    }
    let mut numbers: Vec<String> = rows.iter().map(|r| r.number.clone()).collect();
    arrange(&mut numbers, &order(doc));
    numbers
        .iter()
        .filter_map(|n| rows.iter().find(|r| &r.number == n).cloned())
        .collect()
}

/// The next sheet number after `after` in its series (A-101 → A-102), not yet taken by any
/// of `taken`.
pub fn next_number(after: &str, taken: &[String]) -> String {
    let mut n = ops::next_sheet_number(Some(after));
    while taken.contains(&n) {
        n = ops::next_sheet_number(Some(&n));
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with(numbers: &[&str]) -> Document {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        doc.transact("Sheets", |tx| {
            for n in numbers {
                tx.insert(ElementData::Sheet {
                    number: (*n).into(),
                    name: format!("Sheet {n}"),
                    size: SheetSize::ArchD,
                    stages: vec![],
                });
            }
            Ok(())
        })
        .unwrap();
        doc
    }
    fn numbers(doc: &Document) -> Vec<String> {
        rows(doc).into_iter().map(|r| r.number).collect()
    }

    #[test]
    fn the_index_edits_sheets_adds_rows_and_keeps_its_order() {
        let mut doc = doc_with(&["A-101", "G-001", "A-201"]);
        assert_eq!(numbers(&doc), ["G-001", "A-101", "A-201"]);
        let mut r = rows(&doc);
        // Rename one, add a sheet and a placeholder, put the elevations before the plans.
        r[1].name = "First Floor Plan".into();
        r.swap(1, 2);
        r.insert(
            1,
            IndexRow {
                sheet: None,
                number: "G-002".into(),
                name: "Code Analysis".into(),
                placeholder: false,
            },
        );
        r.push(IndexRow {
            sheet: None,
            number: "S-101".into(),
            name: "Foundation Plan".into(),
            placeholder: true,
        });
        let depth = doc.undo_depth();
        set_rows(&mut doc, &r, None).unwrap();
        assert_eq!(doc.undo_depth(), depth + 1);
        assert_eq!(numbers(&doc), ["G-001", "G-002", "A-201", "A-101", "S-101"]);
        assert_eq!(rows(&doc)[3].name, "First Floor Plan");
        assert_eq!(doc.count(studio_core::Category::Sheet), 4);
        // The sheet index schedule reads the same way.
        let view = doc
            .of(studio_core::Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    ElementData::View {
                        kind: studio_core::ViewKind::Schedule {
                            kind: studio_core::ScheduleKind::Sheets
                        },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        let index: Vec<String> = crate::schedule::schedule(&doc, view)
            .unwrap()
            .rows
            .into_iter()
            .map(|r| r[0].clone())
            .collect();
        assert_eq!(index, ["G-001", "G-002", "A-201", "A-101", "S-101"]);
        // A sheet made later goes after its neighbour in the series.
        doc.transact("A-102", |tx| {
            tx.insert(ElementData::Sheet {
                number: "A-102".into(),
                name: "Second Floor Plan".into(),
                size: SheetSize::ArchD,
                stages: vec![],
            });
            Ok(())
        })
        .unwrap();
        assert_eq!(
            numbers(&doc),
            ["G-001", "G-002", "A-201", "A-101", "A-102", "S-101"]
        );
        // One undo takes the edit back.
        doc.undo().unwrap();
        doc.undo().unwrap();
        assert_eq!(numbers(&doc), ["G-001", "A-101", "A-201"]);
    }

    #[test]
    fn each_phase_has_its_own_sheet_list_and_typical_sheets() {
        let mut doc = doc_with(&["G-001", "A-101", "A-501"]);
        let stage =
            |doc: &Document, a: &str| ops::stages(doc).into_iter().find(|s| s.2 == a).unwrap().0;
        let (sd, dd, cd) = (stage(&doc, "SD"), stage(&doc, "DD"), stage(&doc, "CD"));
        let list = issues(&doc);
        let cd90 = list
            .iter()
            .find(|i| i.label == "90% Construction Documents")
            .unwrap();
        assert_eq!((cd90.phase.as_str(), cd90.stage), ("CD", cd));
        assert!(list.iter().any(|i| i.phase == "SD"));
        // SD drops the details sheet: it leaves SD's set, the other phases keep it.
        let sd_rows: Vec<IndexRow> = rows_for(&doc, Some(sd))
            .into_iter()
            .filter(|r| r.number != "A-501")
            .collect();
        set_rows(&mut doc, &sd_rows, Some(sd)).unwrap();
        let nums = |rows: Vec<IndexRow>| rows.into_iter().map(|r| r.number).collect::<Vec<_>>();
        assert_eq!(nums(rows_for(&doc, Some(sd))), ["G-001", "A-101"]);
        assert_eq!(nums(rows_for(&doc, Some(dd))), ["G-001", "A-101", "A-501"]);
        // A sheet added to DD's list is in DD's set only.
        let mut dd_rows = rows_for(&doc, Some(dd));
        dd_rows.push(IndexRow {
            sheet: None,
            number: "A-201".into(),
            name: "Exterior Elevations".into(),
            placeholder: false,
        });
        set_rows(&mut doc, &dd_rows, Some(dd)).unwrap();
        assert!(nums(rows_for(&doc, Some(dd))).contains(&"A-201".to_string()));
        assert!(!nums(rows_for(&doc, Some(cd))).contains(&"A-201".to_string()));
        // The CD phase's typical list: its sheets once each, the ones it lacks as new rows.
        let t = typical(&doc, cd, crate::sets::BuildingType::SingleFamily);
        let n = nums(t.clone());
        let unique: std::collections::BTreeSet<&String> = n.iter().collect();
        assert_eq!(unique.len(), n.len(), "{n:?}");
        for have in ["G-001", "A-101", "A-501"] {
            assert!(
                t.iter().any(|r| r.number == have && r.sheet.is_some()),
                "{n:?}"
            );
        }
        assert!(t.iter().any(|r| r.sheet.is_none()), "{n:?}");
    }

    #[test]
    fn numbers_must_be_given_and_unique() {
        let mut doc = doc_with(&["A-101", "A-102"]);
        let mut r = rows(&doc);
        r[1].number = "A-101".into();
        assert!(set_rows(&mut doc, &r, None).is_err());
        r[1].number = " ".into();
        assert!(set_rows(&mut doc, &r, None).is_err());
        // Left out, A-102 keeps its number, which a new row can't take.
        let r = vec![
            rows(&doc)[0].clone(),
            IndexRow {
                sheet: None,
                number: "A-102".into(),
                name: "x".into(),
                placeholder: false,
            },
        ];
        assert!(set_rows(&mut doc, &r, None).is_err());
        assert_eq!(next_number("A-101", &["A-102".into()]), "A-103");
    }
}
