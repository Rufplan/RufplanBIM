//! The element store with transactions and undo/redo.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::element::{Category, Element, ElementData, ElementId};
use crate::params::ParamValue;

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

/// Source of content stamps: every document state gets a number no other state has.
static NEXT_STAMP: AtomicU64 = AtomicU64::new(1);

fn fresh_stamp() -> u64 {
    NEXT_STAMP.fetch_add(1, Ordering::Relaxed)
}

/// Elements by category, kept in step with the element map.
#[derive(Debug, Default, Clone)]
struct CategoryIndex(HashMap<Category, BTreeSet<ElementId>>);

impl CategoryIndex {
    fn add(&mut self, e: &Element) {
        self.0.entry(e.category()).or_default().insert(e.id);
    }
    fn remove(&mut self, e: &Element) {
        if let Some(s) = self.0.get_mut(&e.category()) {
            s.remove(&e.id);
        }
    }
    fn ids(&self, cat: Category) -> impl Iterator<Item = &ElementId> {
        self.0.get(&cat).into_iter().flatten()
    }
}

/// Inserts, replaces (`Some`) or removes (`None`) an element, keeping the index current.
fn put_indexed(
    elements: &mut BTreeMap<ElementId, Element>,
    index: &mut CategoryIndex,
    id: ElementId,
    el: Option<Element>,
) {
    if let Some(old) = elements.remove(&id) {
        index.remove(&old);
    }
    if let Some(e) = el {
        index.add(&e);
        elements.insert(id, e);
    }
}

/// Derived data (regenerated geometry, display lists) cached on a document by the crates
/// that compute it. Never persisted; a clone starts with a copy of the entries.
#[derive(Default)]
pub struct DerivedCache(Mutex<HashMap<&'static str, Arc<dyn Any + Send + Sync>>>);

impl DerivedCache {
    /// The entry stored under `key`, if it has type `T`.
    pub fn get<T: Any + Send + Sync>(&self, key: &'static str) -> Option<Arc<T>> {
        let map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.get(key).cloned().and_then(|v| v.downcast::<T>().ok())
    }
    pub fn put<T: Any + Send + Sync>(&self, key: &'static str, value: Arc<T>) {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(key, value);
    }
}

impl Clone for DerivedCache {
    fn clone(&self) -> Self {
        let map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        Self(Mutex::new(map.clone()))
    }
}

impl std::fmt::Debug for DerivedCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DerivedCache")
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    elements: BTreeMap<ElementId, Element>,
    index: CategoryIndex,
    undo: Vec<ChangeSet>,
    redo: Vec<ChangeSet>,
    /// Set whenever a transaction commits or is undone/redone; cleared on save.
    dirty: bool,
    /// Changes whenever the content changes. Two documents with the same stamp have the
    /// same content, so derived data (regenerated geometry, display lists) is cached by it.
    stamp: u64,
    derived: DerivedCache,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            elements: BTreeMap::new(),
            index: CategoryIndex::default(),
            undo: vec![],
            redo: vec![],
            dirty: false,
            stamp: fresh_stamp(),
            derived: DerivedCache::default(),
        }
    }
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a document from stored elements (no undo history).
    pub fn from_elements(elements: impl IntoIterator<Item = Element>) -> Self {
        let mut doc = Self::default();
        for e in elements {
            doc.index.add(&e);
            doc.elements.insert(e.id, e);
        }
        doc
    }

    /// Identifies the current content (see the field docs).
    pub fn stamp(&self) -> u64 {
        self.stamp
    }

    /// Cache for data derived from this document (check entries against [`Self::stamp`]).
    pub fn derived(&self) -> &DerivedCache {
        &self.derived
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

    /// Elements of one category, in id order (from the category index).
    pub fn of(&self, cat: Category) -> impl Iterator<Item = &Element> {
        self.index.ids(cat).filter_map(|id| self.elements.get(id))
    }

    /// Number of elements in a category.
    pub fn count(&self, cat: Category) -> usize {
        self.index.0.get(&cat).map_or(0, BTreeSet::len)
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

    /// A project parameter value of an element, if set.
    pub fn param(&self, id: ElementId, key: &str) -> Option<&ParamValue> {
        self.get(id)?.params.get(key)
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
            index: &mut self.index,
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
                    self.stamp = fresh_stamp();
                }
                Ok(v)
            }
            Err(e) => {
                for (id, prev) in before {
                    put_indexed(&mut self.elements, &mut self.index, id, prev);
                }
                Err(e)
            }
        }
    }

    /// Reverts the last transaction. Returns its name.
    pub fn undo(&mut self) -> CoreResult<ChangeSet> {
        let cs = self.undo.pop().ok_or(CoreError::NothingTo("undo"))?;
        for (id, before, _) in cs.entries.iter().rev() {
            put_indexed(&mut self.elements, &mut self.index, *id, before.clone());
        }
        self.redo.push(cs.clone());
        self.dirty = true;
        self.stamp = fresh_stamp();
        Ok(cs)
    }

    /// Re-applies the last undone transaction.
    pub fn redo(&mut self) -> CoreResult<ChangeSet> {
        let cs = self.redo.pop().ok_or(CoreError::NothingTo("redo"))?;
        for (id, _, after) in &cs.entries {
            put_indexed(&mut self.elements, &mut self.index, *id, after.clone());
        }
        self.undo.push(cs.clone());
        self.dirty = true;
        self.stamp = fresh_stamp();
        Ok(cs)
    }
}

/// A transaction in progress. Records the before-image of every element it touches.
pub struct Tx<'a> {
    elements: &'a mut BTreeMap<ElementId, Element>,
    index: &'a mut CategoryIndex,
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

    /// Elements of one category (from the category index).
    pub fn of(&self, cat: Category) -> impl Iterator<Item = &Element> {
        self.index.ids(cat).filter_map(|id| self.elements.get(id))
    }

    /// Adds a new element and returns its id.
    pub fn insert(&mut self, data: ElementData) -> ElementId {
        self.insert_element(Element::new(data))
    }

    /// Adds a new element (with any project parameter values) and returns its id.
    pub fn insert_element(&mut self, el: Element) -> ElementId {
        let id = el.id;
        self.touch(id);
        put_indexed(self.elements, self.index, id, Some(el));
        id
    }

    /// Replaces an element's data.
    pub fn set(&mut self, id: ElementId, data: ElementData) -> CoreResult<()> {
        let Some(old) = self.elements.get(&id) else {
            return Err(CoreError::NotFound(id));
        };
        let mut el = old.clone();
        el.data = data;
        self.touch(id);
        put_indexed(self.elements, self.index, id, Some(el));
        Ok(())
    }

    /// Mutates an element's data in place.
    pub fn modify(&mut self, id: ElementId, f: impl FnOnce(&mut ElementData)) -> CoreResult<()> {
        let mut data = self.data(id)?.clone();
        f(&mut data);
        self.set(id, data)
    }

    /// Sets (`Some`) or clears (`None`) a project parameter value.
    pub fn set_param(
        &mut self,
        id: ElementId,
        key: &str,
        value: Option<ParamValue>,
    ) -> CoreResult<()> {
        if !self.elements.contains_key(&id) {
            return Err(CoreError::NotFound(id));
        }
        self.touch(id);
        if let Some(e) = self.elements.get_mut(&id) {
            match value {
                Some(v) => {
                    e.params.insert(key.to_owned(), v);
                }
                None => {
                    e.params.remove(key);
                }
            }
        }
        Ok(())
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
            put_indexed(self.elements, self.index, cur, None);
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

    /// Every touched element that still exists must reference only existing elements and
    /// be geometrically valid.
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
            el.data.validate()?;
        }
        // Any edit (wall length, level height, type size) can make an opening stop fitting,
        // so every opening is checked on every commit.
        let get = |id: ElementId| self.elements.get(&id).map(|e| &e.data);
        crate::hosting::validate_openings(self.elements.values().map(|e| (e.id, &e.data)), &get)
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
                    layers: vec![],
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
                    location: Default::default(),
                    attach_top: false,
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

    #[test]
    fn stamps_change_with_content_only() {
        let mut doc = Document::new();
        let s0 = doc.stamp();
        let (_, _, w) = wall_setup(&mut doc);
        let s1 = doc.stamp();
        assert_ne!(s0, s1);
        // A failed transaction and an empty one leave the stamp alone.
        let _ = doc.transact("bad", |tx| {
            tx.modify(w, |d| {
                if let ElementData::Wall { end, .. } = d {
                    *end = Pt::new(0.0, 0.0)
                }
            })
        });
        doc.transact("noop", |_| Ok(())).unwrap();
        doc.mark_saved();
        assert_eq!(doc.stamp(), s1);
        doc.undo().unwrap();
        let s2 = doc.stamp();
        doc.redo().unwrap();
        assert!(s2 != s1 && doc.stamp() != s2 && doc.stamp() != s1);
        // A clone has the same content, so the same stamp, until one of them changes.
        let copy = doc.clone();
        assert_eq!(copy.stamp(), doc.stamp());
    }

    #[test]
    fn category_index_follows_every_change() {
        let mut doc = Document::new();
        let (l1, _, w) = wall_setup(&mut doc);
        assert_eq!(doc.count(Category::Wall), 1);
        assert_eq!(doc.count(Category::Level), 1);
        doc.transact("delete", |tx| tx.delete(l1)).unwrap();
        assert_eq!(doc.count(Category::Wall), 0);
        assert_eq!(doc.of(Category::Level).count(), 0);
        doc.undo().unwrap();
        assert_eq!(doc.of(Category::Wall).next().map(|e| e.id), Some(w));
        // Rolled-back inserts leave no trace in the index.
        let _ = doc.transact("bad", |tx| {
            tx.insert(ElementData::Level {
                name: "X".into(),
                elevation: 1.0,
            });
            Err::<(), _>(CoreError::Invalid("no".into()))
        });
        assert_eq!(doc.count(Category::Level), 1);
        let rebuilt = Document::from_elements(doc.iter().cloned());
        assert_eq!(rebuilt.count(Category::Wall), 1);
    }

    #[test]
    fn parameter_values_are_undoable() {
        let mut doc = Document::new();
        let (_, _, w) = wall_setup(&mut doc);
        doc.transact("param", |tx| {
            tx.set_param(w, "fire_rating", Some(ParamValue::Text("1 HR".into())))
        })
        .unwrap();
        assert_eq!(
            doc.param(w, "fire_rating"),
            Some(&ParamValue::Text("1 HR".into()))
        );
        doc.undo().unwrap();
        assert_eq!(doc.param(w, "fire_rating"), None);
    }
}
