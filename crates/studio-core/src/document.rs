//! The element store with transactions and undo/redo.

use std::collections::{BTreeMap, HashMap};

use crate::element::{Category, Element, ElementData, ElementId};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CoreError {
    #[error("element {0} not found")]
    NotFound(ElementId),
    #[error("{kind} references missing element {missing}")]
    DanglingRef {
        kind: &'static str,
        missing: ElementId,
    },
    #[error("{0}")]
    Invalid(String),
    #[error("nothing to {0}")]
    NothingTo(&'static str),
}

pub type CoreResult<T> = Result<T, CoreError>;

/// One committed transaction: every touched element with its before and after image.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeSet {
    pub name: String,
    pub entries: Vec<(ElementId, Option<Element>, Option<Element>)>,
}

impl ChangeSet {
    pub fn touched(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.entries.iter().map(|e| e.0)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Document {
    elements: BTreeMap<ElementId, Element>,
    undo: Vec<ChangeSet>,
    redo: Vec<ChangeSet>,
    /// Set whenever a transaction commits or is undone/redone; cleared on save.
    dirty: bool,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a document from stored elements (no undo history).
    pub fn from_elements(elements: impl IntoIterator<Item = Element>) -> Self {
        Self {
            elements: elements.into_iter().map(|e| (e.id, e)).collect(),
            ..Self::default()
        }
    }

    pub fn get(&self, id: ElementId) -> Option<&Element> {
        self.elements.get(&id)
    }

    pub fn data(&self, id: ElementId) -> CoreResult<&ElementData> {
        self.get(id).map(|e| &e.data).ok_or(CoreError::NotFound(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.elements.values()
    }

    pub fn of(&self, cat: Category) -> impl Iterator<Item = &Element> {
        self.elements.values().filter(move |e| e.category() == cat)
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Forgets undo/redo history (e.g. after seeding a new project).
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn can_undo(&self) -> Option<&str> {
        self.undo.last().map(|c| c.name.as_str())
    }

    pub fn can_redo(&self) -> Option<&str> {
        self.redo.last().map(|c| c.name.as_str())
    }

    /// Levels sorted by elevation, as (id, name, elevation mm).
    pub fn levels(&self) -> Vec<(ElementId, String, f64)> {
        let mut v: Vec<_> = self
            .of(Category::Level)
            .filter_map(|e| match &e.data {
                ElementData::Level { name, elevation } => Some((e.id, name.clone(), *elevation)),
                _ => None,
            })
            .collect();
        v.sort_by(|a, b| a.2.total_cmp(&b.2));
        v
    }

    /// Elevation of a level in mm.
    pub fn level_elevation(&self, id: ElementId) -> CoreResult<f64> {
        match self.data(id)? {
            ElementData::Level { elevation, .. } => Ok(*elevation),
            _ => Err(CoreError::Invalid(format!("{id} is not a level"))),
        }
    }

    /// Runs `f` as one undoable transaction. If `f` or validation fails, every change is
    /// rolled back and the document is unchanged.
    pub fn transact<T>(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Tx<'_>) -> CoreResult<T>,
    ) -> CoreResult<T> {
        let mut tx = Tx {
            elements: &mut self.elements,
            before: HashMap::new(),
            order: vec![],
        };
        let result = f(&mut tx).and_then(|v| tx.validate().map(|_| v));
        let Tx { before, order, .. } = tx;
        match result {
            Ok(v) => {
                let mut entries = vec![];
                for id in order {
                    let prev = before.get(&id).cloned().flatten();
                    let after = self.elements.get_mut(&id).map(|e| {
                        if prev.is_some() {
                            e.rev = prev.as_ref().map_or(1, |p| p.rev + 1);
                        }
                        e.clone()
                    });
                    if prev != after {
                        entries.push((id, prev, after));
                    }
                }
                if !entries.is_empty() {
                    self.undo.push(ChangeSet {
                        name: name.to_owned(),
                        entries,
                    });
                    self.redo.clear();
                    self.dirty = true;
                }
                Ok(v)
            }
            Err(e) => {
                for (id, prev) in before {
                    match prev {
                        Some(el) => {
                            self.elements.insert(id, el);
                        }
                        None => {
                            self.elements.remove(&id);
                        }
                    }
                }
                Err(e)
            }
        }
    }

    /// Reverts the last transaction. Returns its name.
    pub fn undo(&mut self) -> CoreResult<ChangeSet> {
        let cs = self.undo.pop().ok_or(CoreError::NothingTo("undo"))?;
        for (id, before, _) in cs.entries.iter().rev() {
            self.put(*id, before.clone());
        }
        self.redo.push(cs.clone());
        self.dirty = true;
        Ok(cs)
    }

    /// Re-applies the last undone transaction.
    pub fn redo(&mut self) -> CoreResult<ChangeSet> {
        let cs = self.redo.pop().ok_or(CoreError::NothingTo("redo"))?;
        for (id, _, after) in &cs.entries {
            self.put(*id, after.clone());
        }
        self.undo.push(cs.clone());
        self.dirty = true;
        Ok(cs)
    }

    fn put(&mut self, id: ElementId, el: Option<Element>) {
        match el {
            Some(e) => {
                self.elements.insert(id, e);
            }
            None => {
                self.elements.remove(&id);
            }
        }
    }
}

/// A transaction in progress. Records the before-image of every element it touches.
pub struct Tx<'a> {
    elements: &'a mut BTreeMap<ElementId, Element>,
    before: HashMap<ElementId, Option<Element>>,
    order: Vec<ElementId>,
}

impl Tx<'_> {
    fn touch(&mut self, id: ElementId) {
        if !self.before.contains_key(&id) {
            self.before.insert(id, self.elements.get(&id).cloned());
            self.order.push(id);
        }
    }

    pub fn get(&self, id: ElementId) -> Option<&Element> {
        self.elements.get(&id)
    }

    pub fn data(&self, id: ElementId) -> CoreResult<&ElementData> {
        self.get(id).map(|e| &e.data).ok_or(CoreError::NotFound(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.elements.values()
    }

    /// Adds a new element and returns its id.
    pub fn insert(&mut self, data: ElementData) -> ElementId {
        let el = Element::new(data);
        let id = el.id;
        self.touch(id);
        self.elements.insert(id, el);
        id
    }

    /// Replaces an element's data.
    pub fn set(&mut self, id: ElementId, data: ElementData) -> CoreResult<()> {
        if !self.elements.contains_key(&id) {
            return Err(CoreError::NotFound(id));
        }
        self.touch(id);
        if let Some(e) = self.elements.get_mut(&id) {
            e.data = data;
        }
        Ok(())
    }

    /// Mutates an element's data in place.
    pub fn modify(&mut self, id: ElementId, f: impl FnOnce(&mut ElementData)) -> CoreResult<()> {
        let mut data = self.data(id)?.clone();
        f(&mut data);
        self.set(id, data)
    }

    /// Deletes an element and, recursively, every element that depends on it.
    /// Returns all deleted ids.
    pub fn delete(&mut self, id: ElementId) -> CoreResult<Vec<ElementId>> {
        if !self.elements.contains_key(&id) {
            return Err(CoreError::NotFound(id));
        }
        let mut deleted = vec![];
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if !self.elements.contains_key(&cur) {
                continue;
            }
            self.touch(cur);
            self.elements.remove(&cur);
            deleted.push(cur);
            stack.extend(
                self.elements
                    .values()
                    .filter(|e| e.data.refs().contains(&cur))
                    .map(|e| e.id),
            );
        }
        Ok(deleted)
    }

    /// Every touched element that still exists must reference only existing elements.
    fn validate(&self) -> CoreResult<()> {
        for id in &self.order {
            let Some(el) = self.elements.get(id) else {
                continue;
            };
            for r in el.data.refs() {
                if !self.elements.contains_key(&r) {
                    return Err(CoreError::DanglingRef {
                        kind: el.category().as_str(),
                        missing: r,
                    });
                }
            }
            if let ElementData::Wall { start, end, .. } = &el.data {
                if start.dist(*end) < 1.0 {
                    return Err(CoreError::Invalid("wall is too short".into()));
                }
            }
            if let ElementData::Floor { boundary, .. } | ElementData::Ceiling { boundary, .. } =
                &el.data
            {
                if boundary.len() < 3 || studio_geom::signed_area(boundary).abs() < 1.0 {
                    return Err(CoreError::Invalid("boundary must enclose an area".into()));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{WallFunction, WallTop};
    use studio_geom::Pt;

    fn level(doc: &mut Document, elevation: f64) -> ElementId {
        doc.transact("level", |tx| {
            Ok(tx.insert(ElementData::Level {
                name: "L".into(),
                elevation,
            }))
        })
        .unwrap()
    }

    fn wall_setup(doc: &mut Document) -> (ElementId, ElementId, ElementId) {
        let l1 = level(doc, 0.0);
        let wt = doc
            .transact("type", |tx| {
                Ok(tx.insert(ElementData::WallType {
                    name: "W".into(),
                    thickness: 200.0,
                    function: WallFunction::Interior,
                }))
            })
            .unwrap();
        let w = doc
            .transact("wall", |tx| {
                Ok(tx.insert(ElementData::Wall {
                    type_id: wt,
                    start: Pt::new(0.0, 0.0),
                    end: Pt::new(5000.0, 0.0),
                    base_level: l1,
                    base_offset: 0.0,
                    top: WallTop::Unconnected { height: 3000.0 },
                }))
            })
            .unwrap();
        (l1, wt, w)
    }

    #[test]
    fn undo_and_redo_restore_exact_state() {
        let mut doc = Document::new();
        let l = level(&mut doc, 0.0);
        doc.transact("move", |tx| {
            tx.modify(l, |d| {
                if let ElementData::Level { elevation, .. } = d {
                    *elevation = 3048.0
                }
            })
        })
        .unwrap();
        assert_eq!(doc.level_elevation(l).unwrap(), 3048.0);
        assert_eq!(doc.get(l).unwrap().rev, 2);
        assert_eq!(doc.undo().unwrap().name, "move");
        assert_eq!(doc.level_elevation(l).unwrap(), 0.0);
        assert_eq!(doc.get(l).unwrap().rev, 1);
        doc.redo().unwrap();
        assert_eq!(doc.level_elevation(l).unwrap(), 3048.0);
        doc.undo().unwrap();
        doc.undo().unwrap();
        assert!(doc.get(l).is_none());
        assert_eq!(doc.undo(), Err(CoreError::NothingTo("undo")));
    }

    #[test]
    fn failed_transaction_rolls_back() {
        let mut doc = Document::new();
        let (_, _, w) = wall_setup(&mut doc);
        let before = doc.get(w).cloned();
        let r = doc.transact("bad", |tx| {
            tx.modify(w, |d| {
                if let ElementData::Wall { end, .. } = d {
                    *end = Pt::new(0.0, 0.0)
                }
            })?;
            Ok(())
        });
        assert!(matches!(r, Err(CoreError::Invalid(_))));
        assert_eq!(doc.get(w).cloned(), before);
        assert_eq!(doc.can_undo(), Some("wall"));
    }

    #[test]
    fn dangling_reference_is_rejected() {
        let mut doc = Document::new();
        let r = doc.transact("floor", |tx| {
            Ok(tx.insert(ElementData::Floor {
                type_id: ElementId::new(),
                level: ElementId::new(),
                offset: 0.0,
                boundary: vec![Pt::new(0.0, 0.0), Pt::new(1.0, 0.0), Pt::new(0.0, 1.0)],
            }))
        });
        assert!(matches!(r, Err(CoreError::DanglingRef { .. })));
        assert!(doc.is_empty());
    }

    #[test]
    fn deleting_a_level_deletes_dependents_and_undo_restores_them() {
        let mut doc = Document::new();
        let (l1, wt, w) = wall_setup(&mut doc);
        let deleted = doc.transact("delete", |tx| tx.delete(l1)).unwrap();
        assert_eq!(deleted.len(), 2);
        assert!(doc.get(w).is_none());
        assert!(doc.get(wt).is_some(), "types are not owned by levels");
        doc.undo().unwrap();
        assert!(doc.get(w).is_some() && doc.get(l1).is_some());
    }

    #[test]
    fn empty_transaction_is_not_recorded() {
        let mut doc = Document::new();
        doc.transact("noop", |_| Ok(())).unwrap();
        assert_eq!(doc.can_undo(), None);
        assert!(!doc.is_dirty());
    }
}
