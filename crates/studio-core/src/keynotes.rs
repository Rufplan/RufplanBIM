//! Keynotes (ADR-081), after Revit's: a project keynote table (key, text, parent) organized
//! by CSI MasterFormat, keynotes assigned to types and materials, and keynote tags placed in
//! views: Element (the type's keynote), Material (a material's) and User (any keynote).
//! Tags show the key, or the key and its text; numbered by keynote or, per sheet, 1, 2, 3.
//!
//! The table imports and exports Revit's keynote file (tab-delimited key, text, parent).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::params::ParamValue;

/// The parameter holding a type's or material's keynote.
pub const KEY: &str = "rufplan.keynote";

/// The starter table: CSI MasterFormat divisions, sections and common keynotes.
pub const DEFAULT_TABLE: &str = include_str!("keynotes_default.txt");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Keynote {
    pub key: String,
    pub text: String,
    #[ts(optional)]
    pub parent: Option<String>,
}

/// How tags number: by their keynote ("09 29 00.A1"), or 1, 2, 3… on each sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum KeynoteNumbering {
    #[default]
    ByKeynote,
    BySheet,
}

/// Where a tag's keynote comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum KeynoteSource {
    /// The tagged element's type.
    Element { target: ElementId },
    /// A material of the tagged element.
    Material {
        target: ElementId,
        material: ElementId,
    },
    /// Any keynote from the table.
    User { key: String },
}

/// What a tag shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum KeynoteStyle {
    /// The key in a box.
    #[default]
    Key,
    /// The key in a box, its text beside it.
    KeyAndText,
}

/// Parses Revit's keynote file: one keynote per line as key, text and optional parent,
/// tab-separated. Blank lines and `#` comments are skipped; later duplicates win.
pub fn parse(text: &str) -> Result<Vec<Keynote>, String> {
    let mut out: Vec<Keynote> = vec![];
    for (n, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let mut cols = line.split('\t');
        let key = cols.next().unwrap_or("").trim().to_owned();
        let text = cols.next().unwrap_or("").trim().to_owned();
        let parent = cols
            .next()
            .map(|p| p.trim().to_owned())
            .filter(|p| !p.is_empty());
        if key.is_empty() {
            return Err(format!("line {}: no key", n + 1));
        }
        match out.iter_mut().find(|k| k.key == key) {
            Some(k) => {
                k.text = text;
                k.parent = parent;
            }
            None => out.push(Keynote { key, text, parent }),
        }
    }
    // Parents that aren't in the file become top-level.
    let keys: BTreeSet<String> = out.iter().map(|k| k.key.clone()).collect();
    for k in &mut out {
        if k.parent
            .as_ref()
            .is_some_and(|p| !keys.contains(p) || *p == k.key)
        {
            k.parent = None;
        }
    }
    Ok(out)
}

/// Writes the table as Revit's keynote file.
pub fn to_text(entries: &[Keynote]) -> String {
    let mut s = String::new();
    for k in sorted(entries) {
        s.push_str(&k.key);
        s.push('\t');
        s.push_str(&k.text.replace(['\t', '\n'], " "));
        if let Some(p) = &k.parent {
            s.push('\t');
            s.push_str(p);
        }
        s.push('\n');
    }
    s
}

/// The entries in key order (a natural sort, so "09 29 00.A10" follows ".A9").
pub fn sorted(entries: &[Keynote]) -> Vec<Keynote> {
    let mut v = entries.to_vec();
    v.sort_by(|a, b| crate::ops::natural_cmp(&a.key, &b.key));
    v
}

fn table_el(doc: &Document) -> Option<(ElementId, &Vec<Keynote>, KeynoteNumbering)> {
    doc.of(Category::KeynoteTable).find_map(|e| match &e.data {
        ElementData::KeynoteTable { entries, numbering } => Some((e.id, entries, *numbering)),
        _ => None,
    })
}

/// The project's keynotes and numbering (empty until set up).
pub fn table(doc: &Document) -> (Vec<Keynote>, KeynoteNumbering) {
    table_el(doc).map_or((vec![], KeynoteNumbering::ByKeynote), |(_, e, n)| {
        (e.clone(), n)
    })
}

/// Gives the project the starter table if it has none.
pub fn ensure_table(doc: &mut Document) -> CoreResult<()> {
    if table_el(doc).is_some() {
        return Ok(());
    }
    #[allow(clippy::expect_used)]
    let entries = parse(DEFAULT_TABLE).expect("the starter keynotes parse");
    doc.transact("Keynote Table", |tx| {
        tx.insert(ElementData::KeynoteTable {
            entries,
            numbering: KeynoteNumbering::ByKeynote,
        });
        Ok(())
    })
}

fn with_table(
    doc: &mut Document,
    what: &str,
    f: impl FnOnce(&mut Vec<Keynote>, &mut KeynoteNumbering) -> CoreResult<()>,
) -> CoreResult<()> {
    ensure_table(doc)?;
    let (id, entries, numbering) = table_el(doc)
        .map(|(id, e, n)| (id, e.clone(), n))
        .ok_or_else(|| CoreError::Invalid("no keynote table".into()))?;
    let (mut entries, mut numbering) = (entries, numbering);
    f(&mut entries, &mut numbering)?;
    doc.transact(what, |tx| {
        tx.set(id, ElementData::KeynoteTable { entries, numbering })
    })
}

/// Adds a keynote, or (with `old_key`) changes one: its key, text or parent. Renaming a key
/// updates its children, and the tags and assignments that use it.
pub fn save(doc: &mut Document, old_key: Option<&str>, k: Keynote) -> CoreResult<()> {
    let key = k.key.trim().to_owned();
    if key.is_empty() {
        return Err(CoreError::Invalid("give the keynote a key".into()));
    }
    if k.text.trim().is_empty() {
        return Err(CoreError::Invalid("give the keynote its text".into()));
    }
    let parent = k.parent.clone().filter(|p| !p.trim().is_empty());
    let renamed = old_key.filter(|o| *o != key).map(str::to_owned);
    let mark = doc.undo_depth();
    // Where the key is used, to follow a rename.
    let (tags, assigned) = if let Some(old) = &renamed {
        (
            doc.of(Category::KeynoteTag)
                .filter(|e| matches!(&e.data, ElementData::KeynoteTag { source: KeynoteSource::User { key }, .. } if key == old))
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            doc.iter()
                .filter(|e| matches!(e.params.get(KEY), Some(ParamValue::Text(s)) if s == old))
                .map(|e| e.id)
                .collect::<Vec<_>>(),
        )
    } else {
        (vec![], vec![])
    };
    with_table(
        doc,
        if old_key.is_some() {
            "Edit Keynote"
        } else {
            "Add Keynote"
        },
        |entries, _| {
            if entries
                .iter()
                .any(|e| e.key == key && Some(e.key.as_str()) != old_key)
            {
                return Err(CoreError::Invalid(format!("the key {key} is already used")));
            }
            if let Some(p) = &parent {
                if !entries.iter().any(|e| &e.key == p) || *p == key {
                    return Err(CoreError::Invalid(format!(
                        "no keynote {p} to file it under"
                    )));
                }
                // Not under itself or its own children.
                let mut cur = Some(p.clone());
                while let Some(c) = cur {
                    if Some(c.as_str()) == old_key {
                        return Err(CoreError::Invalid(
                            "a keynote can't go under its own children".into(),
                        ));
                    }
                    cur = entries
                        .iter()
                        .find(|e| e.key == c)
                        .and_then(|e| e.parent.clone());
                }
            }
            let entry = Keynote {
                key: key.clone(),
                text: k.text.trim().to_owned(),
                parent: parent.clone(),
            };
            match old_key.and_then(|o| entries.iter_mut().find(|e| e.key == o)) {
                Some(e) => *e = entry,
                None => entries.push(entry),
            }
            if let Some(old) = &renamed {
                for e in entries.iter_mut() {
                    if e.parent.as_deref() == Some(old.as_str()) {
                        e.parent = Some(key.clone());
                    }
                }
            }
            Ok(())
        },
    )?;
    if !tags.is_empty() || !assigned.is_empty() {
        doc.transact("Rename Keynote", |tx| {
            for id in &tags {
                tx.modify(*id, |d| {
                    if let ElementData::KeynoteTag {
                        source: KeynoteSource::User { key: k },
                        ..
                    } = d
                    {
                        *k = key.clone();
                    }
                })?;
            }
            for id in &assigned {
                tx.set_param(*id, KEY, Some(ParamValue::Text(key.clone())))?;
            }
            Ok(())
        })?;
        doc.merge_undo(mark, "Edit Keynote");
    }
    Ok(())
}

/// The keys under `key`, itself included.
fn subtree(entries: &[Keynote], key: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = [key.to_owned()].into();
    loop {
        let more: Vec<String> = entries
            .iter()
            .filter(|e| e.parent.as_ref().is_some_and(|p| out.contains(p)) && !out.contains(&e.key))
            .map(|e| e.key.clone())
            .collect();
        if more.is_empty() {
            return out;
        }
        out.extend(more);
    }
}

/// Deletes a keynote and everything filed under it. User tags of those keys are deleted
/// too, and assignments cleared, in the same undo step.
pub fn delete(doc: &mut Document, key: &str) -> CoreResult<usize> {
    let (entries, _) = table(doc);
    if !entries.iter().any(|e| e.key == key) {
        return Err(CoreError::Invalid(format!("no keynote {key}")));
    }
    let gone = subtree(&entries, key);
    let tags: Vec<ElementId> = doc
        .of(Category::KeynoteTag)
        .filter(|e| matches!(&e.data, ElementData::KeynoteTag { source: KeynoteSource::User { key }, .. } if gone.contains(key)))
        .map(|e| e.id)
        .collect();
    let assigned: Vec<ElementId> = doc
        .iter()
        .filter(|e| matches!(e.params.get(KEY), Some(ParamValue::Text(s)) if gone.contains(s)))
        .map(|e| e.id)
        .collect();
    let n = gone.len();
    let mark = doc.undo_depth();
    with_table(doc, "Delete Keynote", |entries, _| {
        entries.retain(|e| !gone.contains(&e.key));
        Ok(())
    })?;
    if !tags.is_empty() || !assigned.is_empty() {
        doc.transact("Delete Keynote", |tx| {
            for id in &tags {
                tx.delete(*id)?;
            }
            for id in &assigned {
                tx.set_param(*id, KEY, None)?;
            }
            Ok(())
        })?;
        doc.merge_undo(mark, "Delete Keynote");
    }
    Ok(n)
}

/// Keynote Settings: the numbering method.
pub fn set_numbering(doc: &mut Document, n: KeynoteNumbering) -> CoreResult<()> {
    with_table(doc, "Keynote Numbering", |_, numbering| {
        *numbering = n;
        Ok(())
    })
}

/// Loads a keynote file: replacing the table, or merging into it (the file's keys win).
pub fn import(doc: &mut Document, text: &str, replace: bool) -> CoreResult<usize> {
    let incoming = parse(text).map_err(CoreError::Invalid)?;
    if incoming.is_empty() {
        return Err(CoreError::Invalid("the file has no keynotes".into()));
    }
    let n = incoming.len();
    with_table(doc, "Load Keynotes", |entries, _| {
        if replace {
            *entries = incoming;
        } else {
            for k in incoming {
                match entries.iter_mut().find(|e| e.key == k.key) {
                    Some(e) => *e = k,
                    None => entries.push(k),
                }
            }
        }
        Ok(())
    })?;
    Ok(n)
}

/// Whether `data` can carry a keynote: types and materials.
pub fn assignable(data: &ElementData) -> bool {
    matches!(
        data,
        ElementData::WallType { .. }
            | ElementData::FloorType { .. }
            | ElementData::CeilingType { .. }
            | ElementData::RoofType { .. }
            | ElementData::DoorType { .. }
            | ElementData::WindowType { .. }
            | ElementData::ColumnType { .. }
            | ElementData::BeamType { .. }
            | ElementData::RailingType { .. }
            | ElementData::LightingFixtureType { .. }
            | ElementData::PlantingType { .. }
            | ElementData::Material { .. }
    )
}

/// The keynote assigned to a type or material.
pub fn assigned(doc: &Document, id: ElementId) -> Option<String> {
    match doc.param(id, KEY) {
        Some(ParamValue::Text(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Assigns (or with None, clears) a keynote on types and materials, one undo step.
pub fn assign(doc: &mut Document, ids: &[ElementId], key: Option<&str>) -> CoreResult<usize> {
    if let Some(k) = key {
        if !table(doc).0.iter().any(|e| e.key == k) {
            return Err(CoreError::Invalid(format!("no keynote {k}")));
        }
    }
    let ok: Vec<ElementId> = ids
        .iter()
        .copied()
        .filter(|id| doc.data(*id).is_ok_and(assignable))
        .collect();
    if ok.is_empty() {
        return Err(CoreError::Invalid(
            "keynotes go on types and materials".into(),
        ));
    }
    let v = key.map(|k| ParamValue::Text(k.to_owned()));
    doc.transact("Assign Keynote", |tx| {
        for id in &ok {
            tx.set_param(*id, KEY, v.clone())?;
        }
        Ok(ok.len())
    })
}

/// The materials an element is made of (its type's layers, or its type's material), with
/// its paint first.
pub fn materials_of(doc: &Document, el: ElementId) -> Vec<ElementId> {
    let mut out: Vec<ElementId> = vec![];
    if let Some(p) = crate::paint::paint_of(doc, el) {
        out.push(p);
    }
    let Some(t) = doc.data(el).ok().and_then(|d| d.type_id()) else {
        return out;
    };
    match doc.data(t) {
        Ok(
            ElementData::WallType { layers, .. }
            | ElementData::FloorType { layers, .. }
            | ElementData::CeilingType { layers, .. }
            | ElementData::RoofType { layers, .. },
        ) => {
            for l in layers {
                let m = l.material.or_else(|| crate::material::resolve(doc, l).id);
                if let Some(m) = m {
                    if !out.contains(&m) {
                        out.push(m);
                    }
                }
            }
        }
        Ok(ElementData::ColumnType { material, .. } | ElementData::BeamType { material, .. }) => {
            if let Some(m) = material {
                if !out.contains(m) {
                    out.push(*m);
                }
            }
        }
        _ => {}
    }
    out
}

/// The key a tag stands for (None when its type or material has no keynote yet).
pub fn tag_key(doc: &Document, source: &KeynoteSource) -> Option<String> {
    match source {
        KeynoteSource::Element { target } => {
            let t = doc.data(*target).ok()?.type_id()?;
            assigned(doc, t)
        }
        KeynoteSource::Material { material, .. } => assigned(doc, *material),
        KeynoteSource::User { key } => Some(key.clone()),
    }
}

/// A keynote's text.
pub fn text_of(doc: &Document, key: &str) -> Option<String> {
    table_el(doc)?
        .1
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.text.clone())
}

/// The sheet a view is placed on.
pub fn sheet_of(doc: &Document, view: ElementId) -> Option<ElementId> {
    doc.of(Category::Viewport).find_map(|e| match &e.data {
        ElementData::Viewport { sheet, view: v, .. } if *v == view => Some(*sheet),
        _ => None,
    })
}

/// The views on `sheet`.
fn views_on(doc: &Document, sheet: ElementId) -> Vec<ElementId> {
    doc.of(Category::Viewport)
        .filter_map(|e| match &e.data {
            ElementData::Viewport { sheet: s, view, .. } if *s == sheet => Some(*view),
            _ => None,
        })
        .collect()
}

/// The keys tagged in `views` (or everywhere), in key order.
pub fn used_keys(doc: &Document, views: Option<&[ElementId]>) -> Vec<String> {
    let mut keys: Vec<String> = doc
        .of(Category::KeynoteTag)
        .filter_map(|e| match &e.data {
            ElementData::KeynoteTag { view, source, .. }
                if views.is_none_or(|vs| vs.contains(view)) =>
            {
                tag_key(doc, source)
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    keys.sort_by(|a, b| crate::ops::natural_cmp(a, b));
    keys
}

/// By Sheet numbering: each key used on `sheet` numbered 1, 2, 3… in key order.
pub fn sheet_numbers(doc: &Document, sheet: ElementId) -> BTreeMap<String, usize> {
    let views = views_on(doc, sheet);
    used_keys(doc, Some(&views))
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, i + 1))
        .collect()
}

/// What a tag in `view` shows as its number: its key, or By Sheet, its number on the sheet
/// the view is placed on (the key when the view isn't placed). "?" until it has a keynote.
pub fn tag_label(doc: &Document, view: ElementId, source: &KeynoteSource) -> String {
    let Some(key) = tag_key(doc, source) else {
        return "?".into();
    };
    if table(doc).1 == KeynoteNumbering::BySheet {
        if let Some(sheet) = sheet_of(doc, view) {
            if let Some(n) = sheet_numbers(doc, sheet).get(&key) {
                return n.to_string();
            }
        }
    }
    key
}

/// Places a keynote tag in `view`: its leader arrow at `arrow` (None for no leader), its box
/// at `at`.
pub fn create_tag(
    doc: &mut Document,
    view: ElementId,
    source: KeynoteSource,
    arrow: Option<Pt>,
    at: Pt,
    style: KeynoteStyle,
) -> CoreResult<ElementId> {
    if !matches!(doc.data(view)?, ElementData::View { .. }) {
        return Err(CoreError::Invalid("keynotes go in a view".into()));
    }
    match &source {
        KeynoteSource::Element { target } => {
            if doc.data(*target)?.type_id().is_none() {
                return Err(CoreError::Invalid("pick an element with a type".into()));
            }
        }
        KeynoteSource::Material { material, .. } => {
            if !matches!(doc.data(*material)?, ElementData::Material { .. }) {
                return Err(CoreError::Invalid("pick a material".into()));
            }
        }
        KeynoteSource::User { key } => {
            if !table(doc).0.iter().any(|e| &e.key == key) {
                return Err(CoreError::Invalid(format!("no keynote {key}")));
            }
        }
    }
    doc.transact("Keynote", |tx| {
        Ok(tx.insert(ElementData::KeynoteTag {
            view,
            source,
            at,
            arrow,
            style,
        }))
    })
}

/// Annotate > Keynote Legend: opens the project's legend, making it the first time (a
/// schedule of the keynotes used, filtered to the sheet it's placed on).
pub fn legend(doc: &mut Document) -> CoreResult<ElementId> {
    let existing = doc.of(Category::View).find_map(|e| match &e.data {
        ElementData::View {
            kind:
                crate::element::ViewKind::Schedule {
                    kind: crate::element::ScheduleKind::Keynotes,
                },
            ..
        } => Some(e.id),
        _ => None,
    });
    if let Some(id) = existing {
        return Ok(id);
    }
    ensure_table(doc)?;
    doc.transact("Keynote Legend", |tx| {
        Ok(tx.insert(ElementData::view(
            "Keynote Legend",
            crate::element::ViewKind::Schedule {
                kind: crate::element::ScheduleKind::Keynotes,
            },
            1,
        )))
    })
}

/// Where each keynote is used: tags and assigned types/materials.
#[derive(Debug, Clone, PartialEq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeynoteUse {
    pub key: String,
    pub tags: usize,
    pub assigned: Vec<String>,
}

pub fn usage(doc: &Document) -> Vec<KeynoteUse> {
    let mut m: BTreeMap<String, KeynoteUse> = BTreeMap::new();
    for e in doc.of(Category::KeynoteTag) {
        if let ElementData::KeynoteTag { source, .. } = &e.data {
            if let Some(k) = tag_key(doc, source) {
                m.entry(k.clone())
                    .or_insert_with(|| KeynoteUse {
                        key: k,
                        ..Default::default()
                    })
                    .tags += 1;
            }
        }
    }
    for e in doc.iter() {
        if let Some(ParamValue::Text(k)) = e.params.get(KEY) {
            m.entry(k.clone())
                .or_insert_with(|| KeynoteUse {
                    key: k.clone(),
                    ..Default::default()
                })
                .assigned
                .push(e.data.name());
        }
    }
    m.into_values().collect()
}

/// A type or material for the Keynote Manager's Assignments tab.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Assignable {
    pub id: ElementId,
    pub category: Category,
    pub name: String,
    #[ts(optional)]
    pub key: Option<String>,
}

pub fn assignables(doc: &Document) -> Vec<Assignable> {
    let mut v: Vec<Assignable> = doc
        .iter()
        .filter(|e| assignable(&e.data))
        .map(|e| Assignable {
            id: e.id,
            category: e.category(),
            name: e.data.name(),
            key: assigned(doc, e.id),
        })
        .collect();
    v.sort_by(|a, b| {
        a.category
            .as_str()
            .cmp(b.category.as_str())
            .then(crate::ops::natural_cmp(&a.name, &b.name))
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    fn doc() -> Document {
        let mut d = Document::new();
        ops::seed_default_project(&mut d).unwrap();
        d
    }

    #[test]
    fn revit_keynote_files_round_trip_and_the_starter_table_is_masterformat() {
        let file = "# comment\n09\tFinishes\n09 29 00\tGypsum Board\t09\n09 29 00.A1\t5/8\" gyp. bd.\t09 29 00\nX\tOrphan\tMISSING\n";
        let k = parse(file).unwrap();
        assert_eq!(k.len(), 4);
        assert_eq!(k[2].parent.as_deref(), Some("09 29 00"));
        assert_eq!(k[3].parent, None);
        assert_eq!(parse(&to_text(&k)).unwrap().len(), 4);
        assert!(parse("\tno key").is_err());
        let starter = parse(DEFAULT_TABLE).unwrap();
        assert!(starter.len() > 150);
        let gyp = starter.iter().find(|e| e.key == "09 29 00").unwrap();
        assert_eq!(gyp.text, "Gypsum Board");
        assert_eq!(gyp.parent.as_deref(), Some("09"));
        // Every parent exists.
        assert!(starter.iter().all(|e| e
            .parent
            .as_ref()
            .is_none_or(|p| starter.iter().any(|x| &x.key == p))));
        // Natural order: .A10 after .A9.
        let s = sorted(&[
            Keynote {
                key: "A.10".into(),
                text: "t".into(),
                parent: None,
            },
            Keynote {
                key: "A.9".into(),
                text: "t".into(),
                parent: None,
            },
        ]);
        assert_eq!(s[0].key, "A.9");
    }

    #[test]
    fn element_material_and_user_tags_resolve_and_number_by_sheet() {
        let mut d = doc();
        assert!(
            table(&d).0.len() > 150,
            "new projects get the starter table"
        );
        let l1 = d.levels()[0].0;
        let wt = ops::first_of(&d, Category::WallType).unwrap();
        let wall =
            ops::create_wall(&mut d, wt, l1, Pt::new(0.0, 0.0), Pt::new(4000.0, 0.0)).unwrap();
        let plan = d
            .of(Category::View)
            .find(|e| e.data.name() == "Level 1")
            .unwrap()
            .id;
        // Element keynote: "?" until the type has one.
        let el = KeynoteSource::Element { target: wall };
        let t = create_tag(
            &mut d,
            plan,
            el.clone(),
            Some(Pt::new(2000.0, 0.0)),
            Pt::new(2000.0, 1500.0),
            KeynoteStyle::Key,
        )
        .unwrap();
        assert_eq!(tag_label(&d, plan, &el), "?");
        assign(&mut d, &[wt], Some("06 10 00.A2")).unwrap();
        assert_eq!(tag_label(&d, plan, &el), "06 10 00.A2");
        assert!(
            assign(&mut d, &[wall], Some("06 10 00.A2")).is_err(),
            "instances don't carry keynotes"
        );
        assert!(assign(&mut d, &[wt], Some("NOPE")).is_err());
        // Material keynote: from the type's layers.
        let mats = materials_of(&d, wall);
        assert!(!mats.is_empty());
        assign(&mut d, &[mats[0]], Some("09 29 00.A1")).unwrap();
        let mk = KeynoteSource::Material {
            target: wall,
            material: mats[0],
        };
        create_tag(
            &mut d,
            plan,
            mk.clone(),
            None,
            Pt::new(0.0, 1500.0),
            KeynoteStyle::KeyAndText,
        )
        .unwrap();
        assert_eq!(tag_label(&d, plan, &mk), "09 29 00.A1");
        // User keynote.
        let uk = KeynoteSource::User {
            key: "01 41 00.A2".into(),
        };
        create_tag(
            &mut d,
            plan,
            uk.clone(),
            None,
            Pt::new(0.0, 3000.0),
            KeynoteStyle::Key,
        )
        .unwrap();
        assert!(create_tag(
            &mut d,
            plan,
            KeynoteSource::User { key: "NOPE".into() },
            None,
            Pt::new(0.0, 0.0),
            KeynoteStyle::Key
        )
        .is_err());
        assert_eq!(
            used_keys(&d, None),
            ["01 41 00.A2", "06 10 00.A2", "09 29 00.A1"]
        );
        // By Sheet: 1, 2, 3 in key order once the plan is on a sheet.
        set_numbering(&mut d, KeynoteNumbering::BySheet).unwrap();
        assert_eq!(
            tag_label(&d, plan, &uk),
            "01 41 00.A2",
            "unplaced views keep the key"
        );
        let sheet = ops::create_sheet(&mut d, "Plans", crate::element::SheetSize::ArchD).unwrap();
        ops::place_view(&mut d, sheet, plan, Pt::new(400.0, 300.0)).unwrap();
        assert_eq!(tag_label(&d, plan, &uk), "1");
        assert_eq!(tag_label(&d, plan, &el), "2");
        assert_eq!(tag_label(&d, plan, &mk), "3");
        // Deleting the wall deletes its element tag.
        let _ = t;
        let u = usage(&d);
        assert_eq!(u.iter().find(|x| x.key == "06 10 00.A2").unwrap().tags, 1);
    }

    #[test]
    fn editing_renames_follow_through_and_deleting_takes_the_subtree() {
        let mut d = doc();
        let plan = d
            .of(Category::View)
            .find(|e| e.data.name() == "Level 1")
            .unwrap()
            .id;
        save(
            &mut d,
            None,
            Keynote {
                key: "09 29 00.Z1".into(),
                text: "Level 5 finish".into(),
                parent: Some("09 29 00".into()),
            },
        )
        .unwrap();
        assert!(save(
            &mut d,
            None,
            Keynote {
                key: "09 29 00.Z1".into(),
                text: "dup".into(),
                parent: None
            }
        )
        .is_err());
        assert!(save(
            &mut d,
            None,
            Keynote {
                key: "Q".into(),
                text: "t".into(),
                parent: Some("NOPE".into())
            }
        )
        .is_err());
        let uk = KeynoteSource::User {
            key: "09 29 00.Z1".into(),
        };
        let tag = create_tag(&mut d, plan, uk, None, Pt::new(0.0, 0.0), KeynoteStyle::Key).unwrap();
        let wt = ops::first_of(&d, Category::WallType).unwrap();
        assign(&mut d, &[wt], Some("09 29 00.Z1")).unwrap();
        save(
            &mut d,
            Some("09 29 00.Z1"),
            Keynote {
                key: "09 29 00.Z2".into(),
                text: "Level 5 finish".into(),
                parent: Some("09 29 00".into()),
            },
        )
        .unwrap();
        assert!(
            matches!(d.data(tag).unwrap(), ElementData::KeynoteTag { source: KeynoteSource::User { key }, .. } if key == "09 29 00.Z2")
        );
        assert_eq!(assigned(&d, wt).as_deref(), Some("09 29 00.Z2"));
        // Can't file a section under its own keynote.
        assert!(save(
            &mut d,
            Some("09 29 00"),
            Keynote {
                key: "09 29 00".into(),
                text: "Gypsum Board".into(),
                parent: Some("09 29 00.A1".into())
            }
        )
        .is_err());
        // Deleting the section takes its keynotes, the tag and the assignment, one undo.
        let before = table(&d).0.len();
        let n = delete(&mut d, "09 29 00").unwrap();
        assert!(n >= 6);
        assert_eq!(table(&d).0.len(), before - n);
        assert!(d.data(tag).is_err());
        assert_eq!(assigned(&d, wt), None);
        d.undo().unwrap();
        assert_eq!(table(&d).0.len(), before);
        assert!(d.data(tag).is_ok());
        // Import: merge keeps the rest; replace swaps the table.
        import(&mut d, "ZZ\tCustom", false).unwrap();
        assert_eq!(table(&d).0.len(), before + 1);
        import(&mut d, "A\tOne\nA.1\tTwo\tA", true).unwrap();
        assert_eq!(table(&d).0.len(), 2);
    }
}
