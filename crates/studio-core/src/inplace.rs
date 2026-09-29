//! Model In-Place (ADR-068), after Revit's in-place families: a one-off element modelled
//! in the project from forms (extrusions, blends and sweeps, with void extrusions cutting
//! them), in a category chosen for it. The category decides what it is: a wall, a door,
//! a piece of casework… for visibility, filters, tags and IFC, and whether plans cut it.
//! Its solids are built by `studio_regen::inplace`.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};
use crate::ops::{choice, len, non_empty, parse_len, text, PropOption, Property};
use crate::sketch::{SketchCurve, SketchError};
use crate::units::{MM_PER_FT, MM_PER_IN};

/// The categories an in-place element can be made in, as Revit's Family Category and
/// Parameters dialog lists them (alphabetically, by their plural names).
pub const CATEGORIES: [(Category, &str); 17] = [
    (Category::Casework, "Casework"),
    (Category::Ceiling, "Ceilings"),
    (Category::Column, "Columns"),
    (Category::Door, "Doors"),
    (Category::Floor, "Floors"),
    (Category::Furniture, "Furniture"),
    (Category::GenericModel, "Generic Models"),
    (Category::LightingFixture, "Lighting Fixtures"),
    (Category::Planting, "Planting"),
    (Category::PlumbingFixture, "Plumbing Fixtures"),
    (Category::Railing, "Railings"),
    (Category::Roof, "Roofs"),
    (Category::SpecialtyEquipment, "Specialty Equipment"),
    (Category::Stair, "Stairs"),
    (Category::Beam, "Structural Framing"),
    (Category::Wall, "Walls"),
    (Category::Window, "Windows"),
];

/// A category's name in the dialog, if in-place elements can be made in it.
pub fn label(c: Category) -> Option<&'static str> {
    CATEGORIES.iter().find(|(k, _)| *k == c).map(|(_, l)| *l)
}

/// Whether plans cut elements of this category (Revit's cuttable categories): furniture,
/// fixtures, equipment and planting always show in projection.
pub fn cuttable(c: Category) -> bool {
    !matches!(
        c,
        Category::Furniture
            | Category::Planting
            | Category::LightingFixture
            | Category::PlumbingFixture
            | Category::SpecialtyEquipment
    )
}

/// A sweep's profile, square to its path: a rectangle centered across the path from its
/// elevation up, or a round bar sitting on its elevation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SweepProfile {
    Rectangle { width: f64, height: f64 },
    Circle { diameter: f64 },
}

impl SweepProfile {
    pub fn height(self) -> f64 {
        match self {
            SweepProfile::Rectangle { height, .. } => height,
            SweepProfile::Circle { diameter } => diameter,
        }
    }
}

/// How a form is made from its sketch. Heights are above the element's level (mm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FormKind {
    /// The sketch's closed loops, from `start` to `end`.
    Extrusion { start: f64, end: f64 },
    /// From the sketch's loop at `base` to the loop `top_sketch` at `top`.
    Blend {
        base: f64,
        top: f64,
        top_sketch: Vec<SketchCurve>,
    },
    /// The profile swept along the sketch (a path, open or closed), its bottom at
    /// `elevation`.
    Sweep {
        elevation: f64,
        profile: SweepProfile,
    },
}

/// One form of an in-place element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Form {
    pub kind: FormKind,
    /// Extrusions: closed loops. Blends: the base loop. Sweeps: the path, in order.
    pub sketch: Vec<Vec<SketchCurve>>,
    /// A void cuts the element's solid forms (extrusions only, for now).
    #[serde(default)]
    pub void: bool,
}

impl Form {
    pub fn label(&self) -> &'static str {
        match (&self.kind, self.void) {
            (FormKind::Extrusion { .. }, false) => "Extrusion",
            (FormKind::Extrusion { .. }, true) => "Void Extrusion",
            (FormKind::Blend { .. }, _) => "Blend",
            (FormKind::Sweep { .. }, _) => "Sweep",
        }
    }

    /// Bottom and top above the level.
    pub fn z_range(&self) -> (f64, f64) {
        match &self.kind {
            FormKind::Extrusion { start, end } => (start.min(*end), start.max(*end)),
            FormKind::Blend { base, top, .. } => (base.min(*top), base.max(*top)),
            FormKind::Sweep { elevation, profile } => (*elevation, elevation + profile.height()),
        }
    }

    /// Every curve, mapped (for Move, Rotate, Mirror and Copy).
    pub fn map(&mut self, f: &dyn Fn(Pt) -> Pt, mirror: bool) {
        for c in self.sketch.iter_mut().flatten() {
            *c = c.mapped(f, mirror);
        }
        if let FormKind::Blend { top_sketch, .. } = &mut self.kind {
            for c in top_sketch.iter_mut() {
                *c = c.mapped(f, mirror);
            }
        }
    }

    fn check(&self) -> CoreResult<()> {
        let bad = |m: &str| Err(CoreError::Invalid(m.into()));
        let (lo, hi) = self.z_range();
        if hi - lo < 1.0 {
            return bad("a form needs some height");
        }
        if self.void && !matches!(self.kind, FormKind::Extrusion { .. }) {
            return bad("voids are extrusions");
        }
        if self.sketch.iter().all(Vec::is_empty) {
            return bad("a form needs a sketch");
        }
        match &self.kind {
            FormKind::Sweep { profile, .. } => match profile {
                SweepProfile::Rectangle { width, height } if *width < 1.0 || *height < 1.0 => {
                    bad("the profile needs a size")
                }
                SweepProfile::Circle { diameter } if *diameter < 1.0 => {
                    bad("the profile needs a size")
                }
                _ => Ok(()),
            },
            FormKind::Blend { top_sketch, .. } if top_sketch.is_empty() => {
                bad("a blend needs its top sketched")
            }
            _ => Ok(()),
        }
    }
}

/// A new form's settings, as the options bar has them (Revit's defaults: 1'-0" deep).
pub fn default_kind(kind: &str) -> Option<FormKind> {
    Some(match kind {
        "Extrusion" | "VoidExtrusion" => FormKind::Extrusion {
            start: 0.0,
            end: MM_PER_FT,
        },
        "Blend" => FormKind::Blend {
            base: 0.0,
            top: MM_PER_FT,
            top_sketch: vec![],
        },
        "Sweep" => FormKind::Sweep {
            elevation: 0.0,
            profile: SweepProfile::Rectangle {
                width: 6.0 * MM_PER_IN,
                height: 6.0 * MM_PER_IN,
            },
        },
        _ => return None,
    })
}

/// A sketch as a sweep's path: its curves in order, end to end (reversing any drawn the
/// other way). One chain, open or closed, without branches.
pub fn path(curves: &[SketchCurve]) -> Result<Vec<SketchCurve>, SketchError> {
    const TOL: f64 = 1.0;
    if curves.is_empty() {
        return Err(SketchError::new("Sketch the path to sweep along.", vec![]));
    }
    let ends: Vec<(Pt, Pt)> = curves.iter().map(SketchCurve::ends).collect();
    let degree = |p: Pt| {
        ends.iter()
            .map(|(a, b)| usize::from(a.dist(p) < TOL) + usize::from(b.dist(p) < TOL))
            .sum::<usize>()
    };
    for (i, (a, b)) in ends.iter().enumerate() {
        if degree(*a) > 2 || degree(*b) > 2 {
            return Err(SketchError::new(
                "The path branches. Sweep along one chain of lines.",
                vec![i],
            ));
        }
    }
    // Start from a free end (open path), else anywhere (closed).
    let first = ends
        .iter()
        .enumerate()
        .find_map(|(i, (a, b))| {
            if degree(*a) == 1 {
                Some((i, false))
            } else if degree(*b) == 1 {
                Some((i, true))
            } else {
                None
            }
        })
        .unwrap_or((0, false));
    let mut used = vec![false; curves.len()];
    let mut out = vec![];
    let (mut i, mut flip) = first;
    loop {
        used[i] = true;
        let c = if flip {
            curves[i].reversed()
        } else {
            curves[i].clone()
        };
        let at = c.ends().1;
        out.push(c);
        let next = (0..curves.len()).filter(|j| !used[*j]).find_map(|j| {
            if ends[j].0.dist(at) < TOL {
                Some((j, false))
            } else if ends[j].1.dist(at) < TOL {
                Some((j, true))
            } else {
                None
            }
        });
        match next {
            Some(n) => (i, flip) = n,
            None => break,
        }
    }
    let rest: Vec<usize> = (0..curves.len()).filter(|j| !used[*j]).collect();
    if !rest.is_empty() {
        return Err(SketchError::new(
            "The path must be one connected chain of lines.",
            rest,
        ));
    }
    Ok(out)
}

/// A path's points in order, and whether it closes on itself.
pub fn path_points(path: &[SketchCurve]) -> (Vec<Pt>, bool) {
    let mut pts: Vec<Pt> = vec![];
    for c in path {
        for p in c.points() {
            if pts.last().is_none_or(|q: &Pt| q.dist(p) > 0.5) {
                pts.push(p);
            }
        }
    }
    let closed = pts.len() > 2 && pts[0].dist(pts[pts.len() - 1]) < 1.0;
    if closed {
        pts.pop();
    }
    (pts, closed)
}

/// Turns a finished sketch into a form's sketch: closed loops for extrusions and blends
/// (a blend's base and top are one loop each), an ordered path for sweeps. Lines locked to
/// walls are freed where they are.
pub fn sketch_for(
    doc: &Document,
    kind: &FormKind,
    top: bool,
    curves: &[SketchCurve],
) -> Result<Vec<Vec<SketchCurve>>, SketchError> {
    if let FormKind::Sweep { .. } = kind {
        return Ok(vec![path(curves)?]);
    }
    let loops = crate::sketch::loops(curves)?;
    let loops = crate::sketch::unlock(doc, &loops);
    if crate::sketch::polygons(doc, &loops).is_empty() {
        return Err(SketchError::new("A loop has no area.", vec![]));
    }
    if matches!(kind, FormKind::Blend { .. }) && loops.len() != 1 {
        let which = if top { "top" } else { "base" };
        return Err(SketchError::new(
            &format!("A blend's {which} is one closed loop."),
            vec![],
        ));
    }
    Ok(loops)
}

/// The next free default name for an element of this category ("Casework 1").
pub fn default_name(doc: &Document, category: Category) -> String {
    let base = label(category).unwrap_or("Model In-Place");
    (1..)
        .map(|n| format!("{base} {n}"))
        .find(|n| {
            !doc.iter()
                .any(|e| matches!(&e.data, ElementData::InPlace { name, .. } if name == n))
        })
        .unwrap_or_default()
}

/// Model In-Place: a new in-place element of `category` on `level`, with no forms yet
/// (the editor adds them).
pub fn create(
    doc: &mut Document,
    category: Category,
    name: Option<&str>,
    level: ElementId,
) -> CoreResult<ElementId> {
    if label(category).is_none() {
        return Err(CoreError::Invalid(
            "in-place elements can't be made in that category".into(),
        ));
    }
    doc.level_elevation(level)?;
    let name = match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => n.to_owned(),
        None => default_name(doc, category),
    };
    doc.transact("Model In-Place", |tx| {
        Ok(tx.insert(ElementData::InPlace {
            name,
            category,
            level,
            material: None,
            forms: vec![],
        }))
    })
}

fn edit_forms(
    doc: &mut Document,
    id: ElementId,
    label: &str,
    f: impl FnOnce(&mut Vec<Form>) -> CoreResult<()>,
) -> CoreResult<()> {
    let mut d = doc.data(id)?.clone();
    let ElementData::InPlace { forms, .. } = &mut d else {
        return Err(CoreError::Invalid("select an in-place element".into()));
    };
    f(forms)?;
    for form in forms.iter() {
        form.check()?;
    }
    doc.transact(label, |tx| tx.set(id, d))
}

/// Adds a form (returns its index).
pub fn add_form(doc: &mut Document, id: ElementId, form: Form) -> CoreResult<usize> {
    let mut at = 0;
    let label = format!("Create {}", form.label());
    edit_forms(doc, id, &label, |forms| {
        forms.push(form);
        at = forms.len() - 1;
        Ok(())
    })?;
    Ok(at)
}

/// Replaces form `index` (Edit Sketch).
pub fn set_form(doc: &mut Document, id: ElementId, index: usize, form: Form) -> CoreResult<()> {
    edit_forms(doc, id, "Edit Form", |forms| {
        let f = forms
            .get_mut(index)
            .ok_or_else(|| CoreError::Invalid("no such form".into()))?;
        *f = form;
        Ok(())
    })
}

pub fn delete_form(doc: &mut Document, id: ElementId, index: usize) -> CoreResult<()> {
    edit_forms(doc, id, "Delete Form", |forms| {
        if index >= forms.len() {
            return Err(CoreError::Invalid("no such form".into()));
        }
        forms.remove(index);
        Ok(())
    })
}

/// The element's forms, for the editor.
pub fn forms(doc: &Document, id: ElementId) -> CoreResult<&[Form]> {
    match doc.data(id)? {
        ElementData::InPlace { forms, .. } => Ok(forms),
        _ => Err(CoreError::Invalid("select an in-place element".into())),
    }
}

/// Numbered labels of the forms ("Extrusion 1", "Void Extrusion 1", "Blend 1").
pub fn form_labels(forms: &[Form]) -> Vec<String> {
    let mut seen: Vec<&str> = vec![];
    forms
        .iter()
        .map(|f| {
            seen.push(f.label());
            let n = seen.iter().filter(|l| **l == f.label()).count();
            format!("{} {n}", f.label())
        })
        .collect()
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(ElementData::InPlace {
        name,
        category,
        level,
        material,
        forms,
    }) = doc.data(id)
    else {
        return;
    };
    props.push(text("name", "Name", "Identity Data", name));
    props.push(choice(
        "category",
        "Family Category",
        "Identity Data",
        category.as_str().into(),
        CATEGORIES
            .iter()
            .map(|(c, l)| PropOption {
                id: c.as_str().into(),
                label: (*l).into(),
            })
            .collect(),
    ));
    props.push(crate::ops::ro(
        "level",
        "Level",
        "Constraints",
        doc.data(*level).map(|d| d.name()).unwrap_or_default(),
    ));
    props.push(choice(
        "material",
        "Material",
        "Materials and Finishes",
        material.map(|m| m.to_string()).unwrap_or_default(),
        crate::material::options(doc),
    ));
    for (i, (f, group)) in forms.iter().zip(form_labels(forms)).enumerate() {
        let k = |s: &str| format!("form:{i}:{s}");
        match &f.kind {
            FormKind::Extrusion { start, end } => {
                props.push(len(&k("start"), "Extrusion Start", &group, *start));
                props.push(len(&k("end"), "Extrusion End", &group, *end));
                props.push(choice(
                    &k("void"),
                    "Solid/Void",
                    &group,
                    if f.void { "yes" } else { "no" }.into(),
                    vec![
                        PropOption {
                            id: "no".into(),
                            label: "Solid".into(),
                        },
                        PropOption {
                            id: "yes".into(),
                            label: "Void".into(),
                        },
                    ],
                ));
            }
            FormKind::Blend { base, top, .. } => {
                props.push(len(&k("base"), "First End (Base)", &group, *base));
                props.push(len(&k("top"), "Second End (Top)", &group, *top));
            }
            FormKind::Sweep { elevation, profile } => {
                props.push(len(
                    &k("elevation"),
                    "Profile Elevation",
                    &group,
                    *elevation,
                ));
                let (shape, dims) = match profile {
                    SweepProfile::Rectangle { width, height } => (
                        "Rectangle",
                        vec![("width", "Width", *width), ("height", "Height", *height)],
                    ),
                    SweepProfile::Circle { diameter } => {
                        ("Circle", vec![("diameter", "Diameter", *diameter)])
                    }
                };
                props.push(choice(
                    &k("profile"),
                    "Profile",
                    &group,
                    shape.into(),
                    ["Rectangle", "Circle"]
                        .iter()
                        .map(|s| PropOption {
                            id: (*s).into(),
                            label: (*s).into(),
                        })
                        .collect(),
                ));
                for (key, label, v) in dims {
                    props.push(len(&k(key), label, &group, v));
                }
            }
        }
    }
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let mut d = doc.data(id)?.clone();
    let unknown = || CoreError::Invalid(format!("unknown property {key}"));
    let ElementData::InPlace {
        name,
        category,
        material,
        forms,
        ..
    } = &mut d
    else {
        return Err(unknown());
    };
    match key {
        "name" => *name = non_empty(value)?,
        "category" => {
            *category = CATEGORIES
                .iter()
                .find(|(c, _)| c.as_str() == value.trim())
                .map(|(c, _)| *c)
                .ok_or_else(|| CoreError::Invalid("pick a category".into()))?
        }
        "material" => {
            *material = if value.is_empty() {
                None
            } else {
                let m = crate::ops::parse_id(value)?;
                crate::material::name_of(doc, m)
                    .ok_or_else(|| CoreError::Invalid("pick a material".into()))?;
                Some(m)
            }
        }
        k if k.starts_with("form:") => {
            let mut parts = k["form:".len()..].splitn(2, ':');
            let i: usize = parts
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(unknown)?;
            let field = parts.next().ok_or_else(unknown)?;
            let f = forms.get_mut(i).ok_or_else(unknown)?;
            match (&mut f.kind, field) {
                (_, "void") => f.void = value == "yes",
                (FormKind::Extrusion { start, .. }, "start") => *start = parse_len(value)?,
                (FormKind::Extrusion { end, .. }, "end") => *end = parse_len(value)?,
                (FormKind::Blend { base, .. }, "base") => *base = parse_len(value)?,
                (FormKind::Blend { top, .. }, "top") => *top = parse_len(value)?,
                (FormKind::Sweep { elevation, .. }, "elevation") => *elevation = parse_len(value)?,
                (FormKind::Sweep { profile, .. }, "profile") => {
                    let size = profile.height();
                    *profile = match value {
                        "Rectangle" => SweepProfile::Rectangle {
                            width: size,
                            height: size,
                        },
                        "Circle" => SweepProfile::Circle { diameter: size },
                        _ => return Err(unknown()),
                    }
                }
                (
                    FormKind::Sweep {
                        profile: SweepProfile::Rectangle { width, .. },
                        ..
                    },
                    "width",
                ) => *width = parse_len(value)?,
                (
                    FormKind::Sweep {
                        profile: SweepProfile::Rectangle { height, .. },
                        ..
                    },
                    "height",
                ) => *height = parse_len(value)?,
                (
                    FormKind::Sweep {
                        profile: SweepProfile::Circle { diameter },
                        ..
                    },
                    "diameter",
                ) => *diameter = parse_len(value)?,
                _ => return Err(unknown()),
            }
            f.check()?;
        }
        _ => return Err(unknown()),
    }
    let label = format!("Change {}", key.rsplit(':').next().unwrap_or(key));
    doc.transact(&label, |tx| tx.set(id, d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<SketchCurve> {
        let p = [
            Pt::new(x0, y0),
            Pt::new(x1, y0),
            Pt::new(x1, y1),
            Pt::new(x0, y1),
        ];
        (0..4)
            .map(|i| SketchCurve::line(p[i], p[(i + 1) % 4]))
            .collect()
    }

    #[test]
    fn an_in_place_element_is_in_its_chosen_category_with_forms() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let id = create(&mut doc, Category::Casework, None, l1).unwrap();
        assert_eq!(doc.data(id).unwrap().category(), Category::Casework);
        assert_eq!(doc.data(id).unwrap().name(), "Casework 1");
        assert_eq!(default_name(&doc, Category::Casework), "Casework 2");
        assert!(doc.of(Category::Casework).any(|e| e.id == id));
        assert!(create(&mut doc, Category::View, None, l1).is_err());

        let kind = default_kind("Extrusion").unwrap();
        let sketch = sketch_for(&doc, &kind, false, &rect(0.0, 0.0, 2000.0, 600.0)).unwrap();
        add_form(
            &mut doc,
            id,
            Form {
                kind,
                sketch,
                void: false,
            },
        )
        .unwrap();
        let void = Form {
            kind: default_kind("VoidExtrusion").unwrap(),
            sketch: vec![rect(500.0, 0.0, 900.0, 300.0)],
            void: true,
        };
        add_form(&mut doc, id, void).unwrap();
        assert_eq!(
            form_labels(forms(&doc, id).unwrap()),
            vec!["Extrusion 1", "Void Extrusion 1"]
        );
        // Properties: the category can change; each form's heights are its own.
        ops::set_property(&mut doc, id, "form:0:end", "3'", 0).unwrap();
        assert_eq!(
            forms(&doc, id).unwrap()[0].z_range(),
            (0.0, 3.0 * MM_PER_FT)
        );
        ops::set_property(&mut doc, id, "category", "Furniture", 0).unwrap();
        assert!(doc.of(Category::Furniture).any(|e| e.id == id));
        assert!(!doc.of(Category::Casework).any(|e| e.id == id));
        assert!(ops::set_property(&mut doc, id, "form:0:end", "0", 0).is_err());
        let sheet = ops::properties(&doc, id).unwrap();
        assert!(sheet
            .properties
            .iter()
            .any(|p| p.key == "form:1:void" && p.value == "yes"));
        delete_form(&mut doc, id, 1).unwrap();
        assert_eq!(forms(&doc, id).unwrap().len(), 1);
    }

    #[test]
    fn a_sweep_path_is_ordered_end_to_end_and_must_not_branch() {
        let (a, b, c) = (
            Pt::new(0.0, 0.0),
            Pt::new(1000.0, 0.0),
            Pt::new(1000.0, 1000.0),
        );
        // Drawn out of order and one backwards.
        let p = path(&[SketchCurve::line(c, b), SketchCurve::line(a, b)]).unwrap();
        let (pts, closed) = path_points(&p);
        assert!(!closed);
        assert_eq!(pts.len(), 3);
        assert!(pts[1].dist(b) < 1e-9);
        assert!(pts[0].dist(a) < 1e-9 || pts[0].dist(c) < 1e-9);
        // A closed loop.
        let (pts, closed) = path_points(&path(&rect(0.0, 0.0, 1000.0, 1000.0)).unwrap());
        assert!(closed);
        assert_eq!(pts.len(), 4);
        // A branch and a separate piece are refused.
        let d = Pt::new(1000.0, -1000.0);
        assert!(path(&[
            SketchCurve::line(a, b),
            SketchCurve::line(b, c),
            SketchCurve::line(b, d)
        ])
        .is_err());
        let far = Pt::new(5000.0, 0.0);
        assert!(path(&[SketchCurve::line(a, b), SketchCurve::line(far, c)]).is_err());
    }

    #[test]
    fn a_blend_is_one_loop_at_each_end() {
        let doc = Document::new();
        let kind = default_kind("Blend").unwrap();
        let mut two = rect(0.0, 0.0, 100.0, 100.0);
        two.extend(rect(500.0, 0.0, 600.0, 100.0));
        assert!(sketch_for(&doc, &kind, false, &two).is_err());
        assert_eq!(
            sketch_for(&doc, &kind, true, &rect(0.0, 0.0, 100.0, 100.0))
                .unwrap()
                .len(),
            1
        );
    }
}
