//! Drawings made ready for the sets (ADR-105), as an architect sets up views before laying
//! out sheets: landscape off in plans, ceiling plans, elevations and sections (the site plan
//! keeps it); each view cropped to the building with room for its dimensions and datums; a
//! site plan when the lot is known; and the drawings a construction set needs that the model
//! doesn't have yet: wall sections through each exterior wall condition, cut from footing
//! to roof at 3/4" = 1'-0" and noted layer by layer, enlarged plans of the kitchen and baths,
//! and typical details from the library.

use studio_core::details::FillPattern;
use studio_core::lines::LineStyle;
use studio_core::sketch::SketchCurve;
use studio_core::text::{Leader, TextAlign, LINE};
use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::{
    Category, CoreResult, CropBox, DetailLevel, Document, ElementData, ElementId, ViewKind,
};
use studio_geom::Pt;
use studio_views::Prim;

/// Hidden in drawings other than the site plan.
pub const LANDSCAPE: [Category; 3] = [
    Category::Planting,
    Category::GroundRegion,
    Category::GrassPatch,
];

/// What a crop is fitted to: the building itself.
const BUILDING: &[Category] = &[
    Category::Wall,
    Category::Floor,
    Category::Roof,
    Category::Ceiling,
    Category::Door,
    Category::Window,
    Category::Stair,
    Category::Column,
    Category::Beam,
    Category::Railing,
    Category::WallOpening,
    Category::Casework,
    Category::PlumbingFixture,
];

/// Annotation the crop also keeps, within reach of the building.
const ANNOTATION: &[Category] = &[
    Category::Level,
    Category::Grid,
    Category::Dimension,
    Category::Tag,
    Category::TextNote,
    Category::SpotElevation,
    Category::ElevationMarker,
    Category::KeynoteTag,
    Category::Room,
    Category::View,
];

/// Wall section views are named with this, which keeps them off the building section sheets.
pub const WALL_SECTION: &str = "Wall Section";

/// Paper mm of wall section notes: text height, and the width of the notes outside and
/// inside the wall.
// Notes, leaders and keynote text: 3/32" (ADR-109).
const NOTE: f64 = studio_core::text::sizes::NOTE;
const OUT_W: f64 = 56.0;
const IN_W: f64 = 40.0;
/// Wall sections: 3/4" = 1'-0".
pub const WALL_SECTION_SCALE: u32 = 16;

/// What [`prepare`] did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Prepared {
    pub site_plan: bool,
    pub cropped: usize,
    pub wall_sections: usize,
    pub enlarged: usize,
    pub details: usize,
}

/// True for a wall section view.
pub fn is_wall_section(data: &ElementData) -> bool {
    matches!(data, ElementData::View { kind: ViewKind::Section { .. }, name, .. } if name.starts_with(WALL_SECTION))
}

/// Readies the drawings for the sets. `construction`: the sets include design development
/// or later, which get wall sections, enlarged plans and details.
pub fn prepare(doc: &mut Document, construction: bool) -> CoreResult<Prepared> {
    let mut out = Prepared {
        site_plan: ensure_site_view(doc)?,
        ..Prepared::default()
    };
    let drawings: Vec<ElementId> = doc
        .of(Category::View)
        .filter(|e| {
            matches!(
                &e.data,
                ElementData::View {
                    kind: ViewKind::FloorPlan { .. }
                        | ViewKind::CeilingPlan { .. }
                        | ViewKind::Elevation { .. }
                        | ViewKind::Section { .. },
                    site: false,
                    callout_of: None,
                    ..
                }
            )
        })
        .map(|e| e.id)
        .collect();
    // Plans also leave out room separation lines (ADR-106).
    let off = |kind: &ViewKind| -> Vec<Category> {
        let mut cats = LANDSCAPE.to_vec();
        if matches!(
            kind,
            ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. }
        ) {
            cats.push(Category::RoomSeparator);
        }
        cats
    };
    let bare: Vec<ElementId> = drawings
        .iter()
        .copied()
        .filter(|v| {
            matches!(doc.data(*v), Ok(ElementData::View { hidden_categories, kind, .. })
                if off(kind).iter().any(|c| !hidden_categories.contains(c)))
        })
        .collect();
    if !bare.is_empty() {
        doc.transact("Hide landscape in drawings", |tx| {
            for v in &bare {
                tx.modify(*v, |d| {
                    if let ElementData::View {
                        hidden_categories,
                        kind,
                        ..
                    } = d
                    {
                        for c in off(kind) {
                            if !hidden_categories.contains(&c) {
                                hidden_categories.push(c);
                            }
                        }
                    }
                })?;
            }
            Ok(())
        })?;
    }
    // Outdoor furniture (terrace seating, umbrellas) stays in the site plan and the
    // renderings; floor plans show what's inside.
    let model = studio_regen::regenerate(doc);
    let outdoor: Vec<ElementId> = doc
        .of(Category::Furniture)
        .filter_map(|e| match &e.data {
            ElementData::Ffe { level, at, .. } => {
                let inside = studio_regen::wall_regions(&model, *level)
                    .iter()
                    .any(|r| studio_geom::point_in_ring(*at, &r.outer));
                (!inside).then_some(e.id)
            }
            _ => None,
        })
        .collect();
    let plans: Vec<ElementId> = drawings
        .iter()
        .copied()
        .filter(|v| {
            matches!(doc.data(*v), Ok(ElementData::View { kind: ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. }, hidden, .. })
                if outdoor.iter().any(|o| !hidden.contains(o)))
        })
        .collect();
    if !plans.is_empty() {
        doc.transact("Hide outdoor furniture in plans", |tx| {
            for v in &plans {
                tx.modify(*v, |d| {
                    if let ElementData::View { hidden, .. } = d {
                        for o in &outdoor {
                            if !hidden.contains(o) {
                                hidden.push(*o);
                            }
                        }
                    }
                })?;
            }
            Ok(())
        })?;
    }
    for v in drawings {
        let uncropped = matches!(doc.data(v), Ok(ElementData::View { crop: None, .. }));
        if !uncropped || doc.data(v).is_ok_and(is_wall_section) {
            continue;
        }
        if let Some(c) = fit_crop(doc, v) {
            crop(doc, v, c)?;
            out.cropped += 1;
        }
    }
    if construction {
        out.wall_sections = ensure_wall_sections(doc)?;
        out.enlarged = ensure_enlarged_plans(doc)?;
        ensure_interior_markers(doc)?;
        out.details = ensure_details(doc)?;
        ensure_roof_plan_marks(doc)?;
    }
    // The site plan leaves out room separation lines and interior elevation marks.
    let interior_marks: Vec<ElementId> = doc
        .of(Category::ElevationMarker)
        .filter(|e| matches!(&e.data, ElementData::ElevationMarker { interior: true, .. }))
        .flat_map(|e| {
            std::iter::once(e.id).chain(
                studio_core::detail::marker_views(doc, e.id)
                    .into_iter()
                    .map(|v| v.1),
            )
        })
        .collect();
    let sites: Vec<ElementId> = doc
        .of(Category::View)
        .filter(|e| matches!(&e.data, ElementData::View { site: true, .. }))
        .map(|e| e.id)
        .collect();
    if !sites.is_empty() {
        doc.transact("Site plan graphics", |tx| {
            for v in &sites {
                tx.modify(*v, |d| {
                    if let ElementData::View {
                        hidden,
                        hidden_categories,
                        ..
                    } = d
                    {
                        if !hidden_categories.contains(&Category::RoomSeparator) {
                            hidden_categories.push(Category::RoomSeparator);
                        }
                        for m in &interior_marks {
                            if !hidden.contains(m) {
                                hidden.push(*m);
                            }
                        }
                    }
                })?;
            }
            Ok(())
        })?;
    }
    if let Some((site, c)) = site_crop(doc) {
        crop(doc, site, c)?;
        out.cropped += 1;
    }
    Ok(out)
}

fn crop(doc: &mut Document, view: ElementId, c: CropBox) -> CoreResult<()> {
    doc.transact("Crop view to the building", |tx| {
        tx.modify(view, |d| {
            if let ElementData::View {
                crop, show_crop, ..
            } = d
            {
                *crop = Some(c);
                *show_crop = false;
            }
        })
    })
}

/// A site plan, when the lot is drawn and the project has none.
fn ensure_site_view(doc: &mut Document) -> CoreResult<bool> {
    let has_lot = doc
        .of(Category::Site)
        .any(|e| matches!(&e.data, ElementData::Site { boundary, .. } if boundary.len() >= 3));
    let has_view = doc
        .of(Category::View)
        .any(|e| matches!(&e.data, ElementData::View { site: true, .. }));
    let lowest = doc
        .levels()
        .into_iter()
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|l| l.0);
    let (true, false, Some(level)) = (has_lot, has_view, lowest) else {
        return Ok(false);
    };
    doc.transact("Site plan", |tx| {
        let mut v = ElementData::view("Site", ViewKind::FloorPlan { level }, 240);
        if let ElementData::View { site, .. } = &mut v {
            *site = true;
        }
        tx.insert(v);
        Ok(())
    })?;
    Ok(true)
}

/// The site plan, uncropped: cropped to the lot with room round it for its notes.
fn site_crop(doc: &Document) -> Option<(ElementId, CropBox)> {
    let view = doc.of(Category::View).find(|e| {
        matches!(
            &e.data,
            ElementData::View {
                site: true,
                crop: None,
                ..
            }
        )
    })?;
    let lot: Vec<Pt> = doc.of(Category::Site).find_map(|e| match &e.data {
        ElementData::Site { boundary, .. } if boundary.len() >= 3 => Some(boundary.clone()),
        _ => None,
    })?;
    let (lo, hi) = studio_geom::bounds_of(&lot)?;
    let pad = 12.0 * MM_PER_FT;
    Some((
        view.id,
        CropBox {
            min: Pt::new(lo.x - pad, lo.y - pad),
            max: Pt::new(hi.x + pad, hi.y + pad),
        },
    ))
}

/// Grows (lo, hi) to take `p`.
fn grow(b: &mut Option<(Pt, Pt)>, p: Pt) {
    *b = Some(match *b {
        None => (p, p),
        Some((lo, hi)) => (
            Pt::new(lo.x.min(p.x), lo.y.min(p.y)),
            Pt::new(hi.x.max(p.x), hi.y.max(p.y)),
        ),
    });
}

/// The corners of what a primitive covers.
fn prim_points(p: &Prim) -> Vec<Pt> {
    let pt = |q: &[f64; 2]| Pt::new(q[0], q[1]);
    match p {
        Prim::Line { pts, .. } => pts.iter().map(pt).collect(),
        Prim::Fill { rings, .. } => rings.iter().flatten().map(pt).collect(),
        Prim::Text {
            at,
            text,
            size,
            anchor,
            ..
        } => {
            let w = studio_core::text::text_width(text, *size);
            let x0 = match anchor {
                studio_views::Anchor::Left => at[0],
                studio_views::Anchor::Center => at[0] - w / 2.0,
                studio_views::Anchor::Right => at[0] - w,
            };
            vec![Pt::new(x0, at[1] - size), Pt::new(x0 + w, at[1] + size)]
        }
        Prim::Circle { c, r, .. } => vec![Pt::new(c[0] - r, c[1] - r), Pt::new(c[0] + r, c[1] + r)],
        Prim::Image { min, max, .. } => vec![pt(min), pt(max)],
    }
}

/// The building's extent in a view, and its annotation's: (building, annotation).
type Extents = (Option<(Pt, Pt)>, Option<(Pt, Pt)>);

fn extents(doc: &Document, view: ElementId) -> Extents {
    let Some(dl) = studio_views::display_list(doc, view) else {
        return (None, None);
    };
    let (mut core, mut notes) = (None, None);
    for it in &dl.items {
        let Some(c) = it.el.and_then(|e| doc.data(e).ok()).map(|d| d.category()) else {
            continue;
        };
        let target = if BUILDING.contains(&c) {
            &mut core
        } else if ANNOTATION.contains(&c) {
            &mut notes
        } else {
            continue;
        };
        for p in prim_points(&it.prim) {
            grow(target, p);
        }
    }
    (core, notes)
}

/// A crop round the building: plans 8' clear of it, elevations and sections 6' each side
/// and 4' above and below, grown to take dimensions, datums and marks within 24'.
fn fit_crop(doc: &Document, view: ElementId) -> Option<CropBox> {
    let plan = matches!(
        doc.data(view).ok()?,
        ElementData::View {
            kind: ViewKind::FloorPlan { .. } | ViewKind::CeilingPlan { .. },
            ..
        }
    );
    let (core, notes) = extents(doc, view);
    let (lo, hi) = core?;
    let ft = MM_PER_FT;
    let (side, below, above) = if plan {
        (8.0 * ft, 8.0 * ft, 8.0 * ft)
    } else {
        (6.0 * ft, 4.0 * ft, 4.0 * ft)
    };
    let mut min = Pt::new(lo.x - side, lo.y - below);
    let mut max = Pt::new(hi.x + side, hi.y + above);
    if let Some((nlo, nhi)) = notes {
        let reach = 24.0 * ft;
        let pad = 2.0 * ft;
        min.x = min.x.min((nlo.x - pad).max(lo.x - reach));
        min.y = min.y.min((nlo.y - pad).max(lo.y - reach));
        max.x = max.x.max((nhi.x + pad).min(hi.x + reach));
        max.y = max.y.max((nhi.y + pad).min(hi.y + reach));
    }
    Some(CropBox { min, max })
}

// ---------------------------------------------------------------------------------------
// Wall sections
// ---------------------------------------------------------------------------------------

/// Where a wall section is cut: the wall, the point on its location line, the exterior
/// normal, and the walls above it.
struct Cut {
    wall: usize,
    at: Pt,
    out: Pt,
    above: Vec<usize>,
    score: f64,
}

/// Intersection of segments ab and cd: (t along ab, s along cd), both 0..1.
fn crossing(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<(f64, f64)> {
    let r = b.sub(a);
    let s = d.sub(c);
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-9 {
        return None;
    }
    let q = c.sub(a);
    let t = (q.x * s.y - q.y * s.x) / den;
    let u = (q.x * r.y - q.y * r.x) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some((t, u))
}

/// Distance from p to segment ab.
fn to_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let d = b.sub(a);
    let l2 = d.dot(d);
    if l2 < 1e-9 {
        return p.dist(a);
    }
    let t = (p.sub(a).dot(d) / l2).clamp(0.0, 1.0);
    p.dist(a.add(d.scale(t)))
}

/// The section line through `at` square to a wall: (start outside, end inside).
fn section_line(at: Pt, out: Pt, half: f64, inside: f64) -> (Pt, Pt) {
    (
        at.add(out.scale(half + 5.0 * MM_PER_FT)),
        at.sub(out.scale(half + inside)),
    )
}

/// How far inside the wall a section reaches: 3'-6", or 6' at a wall with nothing under it
/// (a cantilever), to take the wall below.
fn inside_reach(cantilever: bool) -> f64 {
    if cantilever {
        6.0 * MM_PER_FT
    } else {
        3.5 * MM_PER_FT
    }
}

/// The best place to cut a wall section through `w`: clear of openings, wall ends,
/// crossing walls and columns, on every wall the cut crosses.
fn best_cut(model: &studio_regen::Model, w: usize, levels: &[(ElementId, f64)]) -> Option<Cut> {
    let wall = &model.walls[w];
    let len = wall.start.dist(wall.end);
    if len < 6.0 * MM_PER_FT {
        return None;
    }
    let dir = wall.end.sub(wall.start).norm();
    let half = wall.thickness / 2.0;
    let regions = studio_regen::wall_regions(model, wall.level);
    let inside = |p: Pt| {
        regions
            .iter()
            .any(|r| studio_geom::point_in_ring(p, &r.outer))
    };
    let mid = wall.start.add(dir.scale(len / 2.0));
    let left = dir.perp();
    let probe = half + 600.0;
    let out = match (
        inside(mid.add(left.scale(probe))),
        inside(mid.sub(left.scale(probe))),
    ) {
        (false, true) => left,
        (true, false) => left.scale(-1.0),
        _ => return None,
    };
    let elevation = |level: ElementId| levels.iter().find(|l| l.0 == level).map_or(0.0, |l| l.1);
    let z = elevation(wall.level);
    let lowest = levels.first().map_or(0.0, |l| l.1);
    let mut best: Option<Cut> = None;
    let step = 6.0 * MM_PER_IN;
    let mut t = 2.0 * MM_PER_FT;
    while t <= len - 2.0 * MM_PER_FT {
        let at = wall.start.add(dir.scale(t));
        // Walls in line with this one at `at`, above and below.
        let stacked = |below: bool| -> Vec<usize> {
            model
                .walls
                .iter()
                .enumerate()
                .filter(|(i, x)| {
                    *i != w && {
                        let xz = elevation(x.level);
                        (if below { xz < z - 1.0 } else { xz > z + 1.0 })
                            && to_segment(at, x.start, x.end) < half + x.thickness / 2.0
                    }
                })
                .map(|(i, _)| i)
                .collect()
        };
        if !stacked(true).is_empty() {
            // Not the bottom of its stack: the wall below is cut instead.
            return None;
        }
        let cantilever = z > lowest + 1.0;
        let (a, b) = section_line(at, out, half, inside_reach(cantilever));
        let mut score = f64::INFINITY;
        for (i, x) in model.walls.iter().enumerate() {
            let xl = x.start.dist(x.end);
            if let Some((_, s)) = crossing(a, b, x.start, x.end) {
                let s = s * xl;
                let mut clear = s.min(xl - s);
                if i != w
                    && clear < 1.5 * MM_PER_FT
                    && to_segment(at, x.start, x.end) > half + x.thickness
                {
                    // A wall meeting this one near the cut, or a corner.
                    clear = clear.min(0.0);
                }
                for o in model.openings.iter().filter(|o| o.host == x.id) {
                    let d = if s < o.t0 {
                        o.t0 - s
                    } else if s > o.t1 {
                        s - o.t1
                    } else {
                        -1.0
                    };
                    clear = clear.min(d);
                }
                score = score.min(clear);
            } else if x.end.sub(x.start).norm().dot(out).abs() > 0.9 {
                // A wall running along the cut, seen end-on just beyond it.
                let d = to_segment(at, x.start, x.end).min(to_segment(b, x.start, x.end));
                if d < 2.0 * MM_PER_FT {
                    score = score.min(d - 1.0 * MM_PER_FT);
                }
            }
        }
        for c in &model.columns {
            let cpts = &c.base.outer;
            if let Some((lo, hi)) = studio_geom::bounds_of(cpts) {
                let cc = Pt::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
                score = score.min(to_segment(cc, a, b) - 2.0 * MM_PER_FT);
            }
        }
        // Prefer the middle of the clear stretch, then the middle of the wall, and above
        // all a place with the walls of the stories above in line (the full stack).
        let above = stacked(false);
        let score = score - (t - len / 2.0).abs() * 1e-3;
        let rank = if score > 0.0 {
            score + above.len() as f64 * 1e5
        } else {
            score
        };
        if score > 0.0 && best.as_ref().is_none_or(|c| rank > c.score) {
            best = Some(Cut {
                wall: w,
                at,
                out,
                above,
                score: rank,
            });
        }
        t += step;
    }
    best
}

fn wall_type(
    doc: &Document,
    wall: ElementId,
) -> Option<(ElementId, String, Vec<studio_core::WallLayer>)> {
    let ElementData::Wall { type_id, .. } = doc.data(wall).ok()? else {
        return None;
    };
    match doc.data(*type_id).ok()? {
        ElementData::WallType { name, layers, .. } => {
            Some((*type_id, name.clone(), layers.clone()))
        }
        _ => None,
    }
}

/// "Exterior - Stucco on 2x6 Stud" → "Stucco".
fn short_type(name: &str) -> String {
    let n = name.strip_prefix("Exterior - ").unwrap_or(name);
    n.split([' ', '-']).next().unwrap_or(n).to_string()
}

/// Wall sections through each exterior wall condition (wall type and what's above it),
/// most stories first, up to four. Returns how many were made.
pub fn ensure_wall_sections(doc: &mut Document) -> CoreResult<usize> {
    if doc.of(Category::View).any(|e| is_wall_section(&e.data)) {
        return Ok(0);
    }
    let model = studio_regen::regenerate(doc);
    let mut levels: Vec<(ElementId, f64)> =
        model.levels.iter().map(|l| (l.id, l.elevation)).collect();
    levels.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut cuts: Vec<Cut> = (0..model.walls.len())
        .filter(|i| model.walls[*i].exterior)
        .filter_map(|i| best_cut(&model, i, &levels))
        .collect();
    // Most stories, then the longest wall.
    cuts.sort_by(|a, b| {
        b.above.len().cmp(&a.above.len()).then_with(|| {
            let l = |c: &Cut| model.walls[c.wall].start.dist(model.walls[c.wall].end);
            l(b).total_cmp(&l(a))
        })
    });
    let mut seen: Vec<(ElementId, Vec<ElementId>, bool)> = vec![];
    let lowest = levels.first().map_or(0.0, |l| l.1);
    let mut chosen: Vec<(Cut, String)> = vec![];
    for c in cuts {
        let w = &model.walls[c.wall];
        // A one-story wall under an overhanging floor is drawn by the cantilever's section.
        let outside = c.at.add(c.out.scale(w.thickness / 2.0 + 300.0));
        let under_floor = c.above.is_empty()
            && model
                .floors
                .iter()
                .any(|f| (f.z1 - w.z1).abs() < 450.0 && f.base.contains(outside));
        if under_floor {
            continue;
        }
        let Some((ty, name, _)) = wall_type(doc, w.id) else {
            continue;
        };
        let above: Vec<ElementId> = c
            .above
            .iter()
            .filter_map(|i| wall_type(doc, model.walls[*i].id).map(|t| t.0))
            .collect();
        let z = levels.iter().find(|l| l.0 == w.level).map_or(0.0, |l| l.1);
        let cantilever = z > lowest + 1.0;
        let key = (ty, above.clone(), cantilever);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let mut label = short_type(&name);
        for a in &c.above {
            if let Some((_, n, _)) = wall_type(doc, model.walls[*a].id) {
                let s = short_type(&n);
                if !label.contains(&s) {
                    label = format!("{label} & {s}");
                }
            }
        }
        let stories = 1 + c.above.len();
        let what = if cantilever {
            format!("{label} Cantilever")
        } else if stories > 1 {
            format!("{label}, {stories} Stories")
        } else {
            format!("{label}, One Story")
        };
        chosen.push((c, what));
        if chosen.len() == 4 {
            break;
        }
    }
    let n = chosen.len();
    for (i, (cut, what)) in chosen.into_iter().enumerate() {
        make_wall_section(
            doc,
            &model,
            &levels,
            &cut,
            &format!("{WALL_SECTION} {} - {what}", i + 1),
        )?;
    }
    Ok(n)
}

/// Text height in model mm per line of a wall section note.
fn line_h() -> f64 {
    NOTE * LINE * f64::from(WALL_SECTION_SCALE)
}

/// A note to place, pointing at `target`.
struct Callout {
    text: String,
    target: Pt,
    /// Where the note's first line goes; None: level with its target.
    top: Option<f64>,
}

/// Inches as a fraction to the nearest 1/8": 0.875 → 7/8", 5.5 → 5 1/2".
fn inches(mm: f64) -> String {
    let eighths = (mm / MM_PER_IN * 8.0).round() as i64;
    let (whole, frac) = (eighths / 8, eighths % 8);
    let f = match frac {
        0 => String::new(),
        n => {
            let (mut a, mut b) = (n, 8);
            while a % 2 == 0 {
                a /= 2;
                b /= 2;
            }
            format!("{a}/{b}")
        }
    };
    match (whole, f.is_empty()) {
        (0, false) => format!("{f}\""),
        (w, true) => format!("{w}\""),
        (w, false) => format!("{w} {f}\""),
    }
}

/// A layer's note, in the words a wall section uses (what it is, how it's installed, the
/// code section that asks for it).
pub fn layer_note(name: &str, thickness: f64) -> String {
    let n = name.to_lowercase();
    let t = inches(thickness);
    let stud = ["2x4", "2x6", "2x8"]
        .iter()
        .find(|s| n.contains(*s))
        .copied()
        .unwrap_or("wood");
    if n.contains("stucco") || n.contains("plaster") {
        format!("{t} three-coat cement plaster (stucco) on self-furring galv. metal lath over two layers Grade D building paper (WRB), CRC R703.7; weep screed at the foundation plate line, 4\" min. above earth / 2\" above paving")
    } else if n.contains("cedar") || n.contains("siding") {
        format!("{t} {} on 1x3 P.T. furring @ 16\" o.c. (rainscreen cavity), insect screen top and bottom, over WRB, CRC R703", name.split(" on ").next().unwrap_or(name).to_lowercase())
    } else if n.contains("sheathing") {
        format!("{t} structural plywood sheathing; nailing per the shear wall schedule (S-series)")
    } else if n.contains("metal stud") {
        format!("{t} metal studs @ 16\" o.c.")
    } else if n.contains("stud") {
        let r = if stud == "2x6" { "R-21" } else { "R-15" };
        format!("{stud} wood studs @ 16\" o.c. with {r} batt insulation (or as the CF1R requires, T-001)")
    } else if n.contains("gypsum") {
        format!("{t} gypsum board, Level 4 finish; Type X at the garage separation, CRC R302.6")
    } else if n.contains("cmu") {
        format!("{t} CMU, grouted and reinforced per structural")
    } else if n.contains("rigid") {
        format!("{t} rigid insulation, R-value per the CF1R (T-001)")
    } else if n.contains("air") {
        format!("{t} air space")
    } else if n.contains("membrane") {
        "Single-ply TPO roof membrane, Class A, CRRC-rated cool roof per Title 24 Part 6; 1/4\" per foot min. slope to drains and scuppers, CRC R905.15".into()
    } else if n.contains("cover board") {
        format!("{t} cover board")
    } else if n.contains("concrete deck") {
        format!("{t} concrete deck per structural")
    } else if n.contains("concrete") {
        format!("{t} concrete slab, reinforcing per structural, on 15-mil vapor retarder (CALGreen 4.505.2) over 4\" crushed rock capillary break")
    } else if n.contains("hardwood") || n.contains("flooring") {
        format!("{t} {}", name.to_lowercase())
    } else if n.contains("subfloor") {
        format!("{t} T&G plywood subfloor, glued and nailed")
    } else if n.contains("joist") || n.contains("rafter") {
        format!("{t} {} @ 16\" o.c. per structural", name.to_lowercase())
    } else {
        format!("{t} {}", name.to_lowercase())
    }
}

/// Lays out notes on one side of a wall section: stacked from the top, each as near its
/// target's height as the ones above leave room for, with a leader to the target.
/// `x` is the notes' left edge, `w` their width (paper mm).
fn place_notes(
    doc: &mut Document,
    view: ElementId,
    notes: &mut [Callout],
    x: f64,
    w: f64,
    (top, floor): (f64, f64),
) -> CoreResult<()> {
    let k = f64::from(WALL_SECTION_SCALE);
    let lh = line_h();
    let wanted = |n: &Callout| n.top.unwrap_or(n.target.y + lh * 0.3);
    notes.sort_by(|a, b| wanted(b).total_cmp(&wanted(a)));
    let gap = NOTE * k * 0.9;
    let tall =
        |n: &Callout| studio_core::text::wrap(&n.text, NOTE, Some(w)).len().max(1) as f64 * lh;
    // Down from the top: each first line level with its target, unless the note above is
    // in the way.
    let mut cursor = top;
    let mut ys: Vec<f64> = notes
        .iter()
        .map(|n| {
            let y = wanted(n).min(cursor);
            cursor = y - tall(n) - gap;
            y
        })
        .collect();
    // Then up from the crop's bottom (ADR-108): notes that ran past it rise, in order.
    let mut limit = floor;
    for (i, n) in notes.iter().enumerate().rev() {
        let h = tall(n);
        if ys[i] - h < limit {
            ys[i] = limit + h;
        }
        limit = ys[i] + gap;
    }
    let mut placed = vec![];
    for (n, y) in notes.iter().zip(ys) {
        let right = n.target.x > x;
        let shoulder = if right {
            x + w * k + 2.0 * k
        } else {
            x - 2.0 * k
        };
        placed.push((
            Pt::new(x, y),
            n.text.clone(),
            Leader {
                end: n.target,
                elbow: Some(Pt::new(shoulder, y)),
                arc: false,
            },
        ));
    }
    doc.transact("Wall section notes", |tx| {
        for (at, text, leader) in placed {
            tx.insert(ElementData::TextNote {
                view,
                at,
                text,
                size: NOTE,
                leaders: vec![leader],
                align: TextAlign::Left,
                width: Some(w),
                angle: 0.0,
            });
        }
        Ok(())
    })
}

/// A floor's, ceiling's or roof's type: (name, layers top down, thickness).
fn type_layers(
    doc: &Document,
    id: ElementId,
) -> Option<(String, Vec<studio_core::WallLayer>, f64)> {
    let ty = match doc.data(id).ok()? {
        ElementData::Floor { type_id, .. }
        | ElementData::Ceiling { type_id, .. }
        | ElementData::Roof { type_id, .. } => *type_id,
        _ => return None,
    };
    match doc.data(ty).ok()? {
        ElementData::FloorType {
            name,
            layers,
            thickness,
        }
        | ElementData::CeilingType {
            name,
            layers,
            thickness,
        }
        | ElementData::RoofType {
            name,
            layers,
            thickness,
        } => Some((name.clone(), layers.clone(), *thickness)),
        _ => None,
    }
}

/// A layer of a floor, ceiling or roof assembly, in a few words.
fn short_layer(name: &str, thickness: f64) -> String {
    let n = name.to_lowercase();
    let t = inches(thickness);
    if n.contains("membrane") {
        "TPO roof membrane, Class A, CRRC-rated cool roof".into()
    } else if n.contains("concrete") && !n.contains("deck") {
        format!(
            "{t} concrete slab on 15-mil vapor retarder over 4\" crushed rock (CALGreen 4.505.2)"
        )
    } else if n.contains("joist") || n.contains("rafter") {
        format!("{t} {n} @ 16\" o.c. per structural")
    } else if n.contains("gypsum") {
        format!("{t} gypsum board")
    } else if n.contains("insulation") {
        format!("{t} {n}, R-value per the CF1R")
    } else if n.contains("subfloor") {
        format!("{t} T&G plywood subfloor, glued and nailed")
    } else {
        format!("{t} {n}")
    }
}

/// An assembly's note: "FLOOR: 3/4\" hardwood flooring; …".
fn assembly(name: &str, layers: &[studio_core::WallLayer], thickness: f64, what: &str) -> String {
    let what = what.to_uppercase();
    if layers.is_empty() {
        let n = name.to_lowercase();
        return if n.contains("gwb") || n.contains("gyp") {
            format!("{what}: {} gypsum board, Level 4 finish", inches(thickness))
        } else {
            format!("{what}: {name}")
        };
    }
    let parts: Vec<String> = layers
        .iter()
        .map(|l| short_layer(&l.name, l.thickness))
        .collect();
    format!("{what}: {}", parts.join("; "))
}
/// Builds one wall section: the view, its crop from footing to roof, the footing and
/// slab base drawn in, and its notes.
fn make_wall_section(
    doc: &mut Document,
    model: &studio_regen::Model,
    levels: &[(ElementId, f64)],
    cut: &Cut,
    name: &str,
) -> CoreResult<ElementId> {
    let wall = &model.walls[cut.wall];
    let half = wall.thickness / 2.0;
    let lowest = levels.first().map_or(0.0, |l| l.1);
    let z_wall = levels
        .iter()
        .find(|l| l.0 == wall.level)
        .map_or(0.0, |l| l.1);
    let cantilever = z_wall > lowest + 1.0;
    let (start, end) = section_line(cut.at, cut.out, half, inside_reach(cantilever));
    let length = start.dist(end);
    let view = doc.transact("Wall section", |tx| {
        let mut v = ElementData::view(
            name,
            ViewKind::Section {
                start,
                end,
                depth: 2.0 * MM_PER_FT,
            },
            WALL_SECTION_SCALE,
        );
        if let ElementData::View {
            detail_level,
            hidden_categories,
            show_crop,
            ..
        } = &mut v
        {
            *detail_level = Some(DetailLevel::Fine);
            // The site's earth too: the section draws its own grade and footing.
            *hidden_categories = [&LANDSCAPE[..], &[Category::Site]].concat();
            *show_crop = false;
        }
        Ok(tx.insert(v))
    })?;
    // Each wall the cut crosses, as (wall, u of its outside face, u inside face, z0, z1).
    let drawn = studio_views::display_list(doc, view);
    let drawn_bounds = |id: ElementId| -> Option<(Pt, Pt)> {
        let mut b = None;
        for it in drawn.as_ref()?.items.iter().filter(|i| i.el == Some(id)) {
            if matches!(it.prim, Prim::Fill { .. }) {
                for p in prim_points(&it.prim) {
                    grow(&mut b, p);
                }
            }
        }
        b
    };
    let mut crossed: Vec<(usize, f64, f64, f64, f64)> = vec![];
    for (i, x) in model.walls.iter().enumerate() {
        if let Some((t, _)) = crossing(start, end, x.start, x.end) {
            let (u0, u1) = match drawn_bounds(x.id) {
                Some((lo, hi)) if hi.x - lo.x < x.thickness * 1.5 => (lo.x, hi.x),
                _ => {
                    let h = x.thickness / 2.0;
                    (t * length - h, t * length + h)
                }
            };
            crossed.push((i, u0, u1, x.z0, x.z1));
        }
    }
    crossed.sort_by(|a, b| a.3.total_cmp(&b.3));
    let (core, notes) = extents(doc, view);
    let Some((lo, hi)) = core else {
        return Ok(view);
    };
    let ft = MM_PER_FT;
    let inch = MM_PER_IN;
    let bottom = lowest - 3.5 * ft;
    // Up to the roof over the wall's own stack, not a higher one seen beyond.
    let inward = end.sub(start).norm();
    let host_face = crossed
        .iter()
        .find(|c| c.0 == cut.wall)
        .map_or(length / 2.0, |c| c.2);
    let over = start.add(inward.scale((host_face + 1.5 * ft).min(length - 1.0)));
    let roof_top = model
        .roofs
        .iter()
        .filter(|r| studio_geom::point_in_ring(over, &r.boundary))
        .map(|r| r.base + r.thickness)
        .fold(f64::NEG_INFINITY, f64::max);
    let top = if roof_top.is_finite() {
        roof_top.min(hi.y) + 3.0 * ft
    } else {
        hi.y + 3.0 * ft
    };
    let right = notes.map_or(length, |(_, nhi)| nhi.x.max(length) + 1.0 * ft);
    doc.transact("Crop wall section", |tx| {
        tx.modify(view, |d| {
            if let ElementData::View { crop, .. } = d {
                *crop = Some(CropBox {
                    min: Pt::new(lo.x.min(0.0), bottom),
                    max: Pt::new(right, top),
                });
            }
        })
    })?;

    // The foundation under each ground-floor wall the cut crosses: a thickened slab edge
    // on a continuous footing, the slab on its rock base, and finish grade outside.
    let mut out_notes: Vec<Callout> = vec![];
    let mut in_notes: Vec<Callout> = vec![];
    let ground: Vec<&(usize, f64, f64, f64, f64)> = crossed
        .iter()
        .filter(|c| (model.walls[c.0].z0 - lowest).abs() < 1.0 && model.walls[c.0].exterior)
        .collect();
    let line = |a: Pt, b: Pt| SketchCurve::Line { a, b, wall: None };
    let mut regions: Vec<(Vec<Pt>, FillPattern, Option<LineStyle>)> = vec![];
    let mut lines: Vec<(Pt, Pt, LineStyle)> = vec![];
    let slab_bottom = lowest - 6.0 * inch;
    for g in &ground {
        let ue = g.1;
        let footing = vec![
            Pt::new(ue, slab_bottom),
            Pt::new(ue, lowest - 18.0 * inch),
            Pt::new(ue - 4.0 * inch, lowest - 18.0 * inch),
            Pt::new(ue - 4.0 * inch, lowest - 30.0 * inch),
            Pt::new(ue + 14.0 * inch, lowest - 30.0 * inch),
            Pt::new(ue + 14.0 * inch, lowest - 18.0 * inch),
            Pt::new(ue + 8.0 * inch, lowest - 18.0 * inch),
            Pt::new(ue + 8.0 * inch, slab_bottom),
        ];
        regions.push((footing, FillPattern::Concrete, Some(LineStyle::Medium)));
        let rock_end = crossed
            .iter()
            .find(|c| c.1 > g.2 + 1.0 && (model.walls[c.0].z0 - lowest).abs() < 1.0)
            .map_or(right.min(length), |c| c.1);
        regions.push((
            vec![
                Pt::new(ue + 8.0 * inch, slab_bottom - 4.0 * inch),
                Pt::new(rock_end, slab_bottom - 4.0 * inch),
                Pt::new(rock_end, slab_bottom),
                Pt::new(ue + 8.0 * inch, slab_bottom),
            ],
            FillPattern::Gravel,
            Some(LineStyle::Thin),
        ));
        // Finish grade 8" below the floor, sloping away.
        let grade = lowest - 8.0 * inch;
        lines.push((
            Pt::new(ue - 4.0 * ft, grade - 4.0 * inch),
            Pt::new(ue, grade),
            LineStyle::Wide,
        ));
        regions.push((
            vec![
                Pt::new(ue - 2.5 * ft, bottom + 6.0 * inch),
                Pt::new(ue - 4.0 * inch, bottom + 6.0 * inch),
                Pt::new(ue - 4.0 * inch, lowest - 18.0 * inch),
                Pt::new(ue, lowest - 18.0 * inch),
                Pt::new(ue, grade),
                Pt::new(ue - 2.5 * ft, grade - 3.0 * inch),
            ],
            FillPattern::Earth,
            None,
        ));
        out_notes.push(Callout {
            text: "Finish grade: 6\" min. clearance to wood framing, slope 6\" min. within the first 10'-0\" away from the building, CRC R401.3 / R317.1".into(),
            target: Pt::new(ue - 1.5 * ft, grade - 1.5 * inch),
            top: None,
        });
        out_notes.push(Callout {
            text: "Continuous concrete footing and thickened slab edge per structural, 12\" min. below undisturbed grade, CRC R403.1".into(),
            target: Pt::new(ue - 2.0 * inch, lowest - 24.0 * inch),
            top: None,
        });
        out_notes.push(Callout {
            text: "P.T. sill plate with 1/2\" anchor bolts @ 6'-0\" o.c. max., 7\" min. embedment, sill sealer, CRC R403.1.6".into(),
            target: Pt::new(ue + 3.0 * inch, lowest + 0.75 * inch),
            top: None,
        });
    }
    if !regions.is_empty() || !lines.is_empty() {
        doc.transact("Wall section foundation", |tx| {
            for (ring, pattern, outline) in &regions {
                tx.insert(ElementData::FilledRegion {
                    view,
                    boundary: vec![ring.clone()],
                    pattern: *pattern,
                    outline: *outline,
                });
            }
            for (a, b, style) in &lines {
                tx.insert(ElementData::DetailLine {
                    view,
                    curve: line(*a, *b),
                    style: *style,
                });
            }
            Ok(())
        })?;
    }

    // The walls' layers, outside to inside, at mid-height of each story.
    let mut noted: Vec<ElementId> = vec![];
    for c in crossed.iter().filter(|c| model.walls[c.0].exterior) {
        let Some((ty, _, layers)) = wall_type(doc, model.walls[c.0].id) else {
            continue;
        };
        if noted.contains(&ty) || layers.is_empty() {
            continue;
        }
        noted.push(ty);
        let mid = (c.3 + c.4) / 2.0;
        let mut u = c.1;
        for (i, l) in layers.iter().enumerate() {
            let target = Pt::new(u + l.thickness / 2.0, mid + 2.0 * ft - i as f64 * 1.2 * ft);
            u += l.thickness;
            out_notes.push(Callout {
                text: layer_note(&l.name, l.thickness),
                target,
                top: None,
            });
        }
        if c.1 < length / 2.0 {
            in_notes.push(Callout {
                text: "Fireblocking at the floor and ceiling lines and at 10'-0\" max. vertically, CRC R302.11".into(),
                target: Pt::new(c.2 - 2.0 * inch, c.4 - 6.0 * inch),
                top: None,
            });
        }
    }
    // Floors, ceilings and the roof over the cut, noted inside, in the rooms' clear space:
    // a floor above its line, a ceiling and the roof under the ceiling.
    let k = f64::from(WALL_SECTION_SCALE);
    let lh = line_h();
    let host_in = crossed
        .iter()
        .find(|c| c.0 == cut.wall)
        .map_or(length / 2.0, |c| c.2);
    let probe = start.add(
        end.sub(start)
            .norm()
            .scale((host_in + 1.5 * ft).min(length - 1.0)),
    );
    let mut slabs: Vec<(ElementId, Category, f64, f64)> = model
        .floors
        .iter()
        .chain(&model.ceilings)
        .filter(|s| s.base.contains(probe))
        .map(|s| (s.id, s.category, s.z0, s.z1))
        .collect();
    slabs.sort_by(|a, b| a.2.total_cmp(&b.2));
    let x_in = (host_in + 0.5 * ft).min(length - IN_W * k - 1.0 * ft);
    let reach = x_in + IN_W * k;
    let tall =
        |text: &str| studio_core::text::wrap(text, NOTE, Some(IN_W)).len().max(1) as f64 * lh;
    for (id, cat, z0, z1) in &slabs {
        let Some((name, layers, thickness)) = type_layers(doc, *id) else {
            continue;
        };
        let (what, top, target) = match cat {
            Category::Ceiling => (
                "Ceiling",
                *z0 - 0.4 * ft,
                Pt::new(reach + 0.6 * ft, (z0 + z1) / 2.0),
            ),
            _ => {
                let what = if (*z1 - lowest).abs() < 1.0 {
                    "Slab on grade"
                } else {
                    "Floor"
                };
                let text = assembly(&name, &layers, thickness, what);
                (
                    what,
                    *z1 + 0.5 * ft + tall(&text),
                    Pt::new(reach + 0.9 * ft, (z0 + z1) / 2.0),
                )
            }
        };
        in_notes.push(Callout {
            text: assembly(&name, &layers, thickness, what),
            target,
            top: Some(top),
        });
    }
    if cantilever {
        out_notes.push(Callout {
            text: "Floor over exterior space: R-30 min. insulation in the joist bays (or per the CF1R), soffit finish to match the wall below over WRB, continuous soffit vent screen".into(),
            target: Pt::new(start.dist(cut.at) - half + 1.0 * ft, z_wall - 6.0 * inch),
            top: None,
        });
    }
    // The highest roof over the cut.
    let roof = model
        .roofs
        .iter()
        .filter(|r| studio_geom::point_in_ring(probe, &r.boundary))
        .max_by(|a, b| a.base.total_cmp(&b.base));
    if let Some(rs) = roof {
        if let Some((name, layers, thickness)) = type_layers(doc, rs.id) {
            let under = slabs
                .iter()
                .filter(|s| s.1 == Category::Ceiling && s.3 <= rs.base + 1.0)
                .map(|s| s.2)
                .fold(f64::NEG_INFINITY, f64::max);
            let under = if under.is_finite() { under } else { rs.base };
            in_notes.push(Callout {
                text: format!(
                    "{}; slope 1/4\" per foot min. to drains and scuppers, CRC R905",
                    assembly(&name, &layers, thickness, "Roof")
                ),
                target: Pt::new(reach + 1.0 * ft, rs.base + rs.thickness / 2.0),
                top: Some(under - 0.4 * ft + 0.01),
            });
        }
        if let Ok(ElementData::Roof {
            fascia: Some(f), ..
        }) = doc.data(rs.id)
        {
            let edge = start.dist(cut.at) - half;
            out_notes.push(Callout {
                text: format!(
                    "{} fascia, prefinished metal over P.T. blocking; coping sloped to the roof, cleats @ 12\" o.c., CRC R905",
                    f.name
                ),
                target: Pt::new(edge - 1.0 * ft, rs.base + rs.thickness / 2.0),
                top: None,
            });
        }
    }
    place_notes(
        doc,
        view,
        &mut out_notes,
        lo.x.min(0.0) + 3.0 * k,
        OUT_W,
        (top - 2.0 * k, bottom + 4.0 * k),
    )?;
    place_notes(
        doc,
        view,
        &mut in_notes,
        x_in,
        IN_W,
        (top - 2.0 * k, bottom + 4.0 * k),
    )?;
    Ok(view)
}

// ---------------------------------------------------------------------------------------
// Enlarged plans and details
// ---------------------------------------------------------------------------------------

/// Enlarged plans (callouts at 1/4" = 1'-0") of the kitchen and baths, when the project
/// has no plan callouts. Returns how many were made.
pub fn ensure_enlarged_plans(doc: &mut Document) -> CoreResult<usize> {
    let has = doc.of(Category::View).any(|e| {
        matches!(
            &e.data,
            ElementData::View {
                callout_of: Some(_),
                kind: ViewKind::FloorPlan { .. },
                ..
            }
        )
    });
    if has {
        return Ok(0);
    }
    let model = studio_regen::regenerate(doc);
    let mut rooms: Vec<(usize, &studio_regen::RoomInfo)> = model
        .rooms
        .iter()
        .filter(|r| r.boundary.is_some())
        .filter_map(|r| {
            let n = r.name.to_lowercase();
            let rank = if n.contains("kitchen") {
                0
            } else if n.contains("primary bath") || n.contains("master bath") {
                1
            } else if n.contains("bath") {
                2
            } else {
                return None;
            };
            Some((rank, r))
        })
        .collect();
    rooms.sort_by_key(|r| r.0);
    let mut made = 0;
    for (_, room) in rooms.into_iter().take(4) {
        let Some((lo, hi)) = room.boundary.as_deref().and_then(studio_geom::bounds_of) else {
            continue;
        };
        let parent = doc.of(Category::View).find(|e| {
            matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, site: false, callout_of: None, .. } if *level == room.level)
        });
        let Some(parent) = parent.map(|e| e.id) else {
            continue;
        };
        let pad = 1.5 * MM_PER_FT;
        let v = studio_core::detail::create_callout(
            doc,
            parent,
            Pt::new(lo.x - pad, lo.y - pad),
            Pt::new(hi.x + pad, hi.y + pad),
        )?;
        let name = format!("Enlarged Plan - {}", room.name);
        doc.transact("Name enlarged plan", |tx| {
            tx.modify(v, |d| {
                if let ElementData::View {
                    name: n,
                    scale,
                    hidden_categories,
                    show_crop,
                    ..
                } = d
                {
                    *n = name.clone();
                    *scale = 24;
                    *hidden_categories = LANDSCAPE.to_vec();
                    *show_crop = false;
                }
            })
        })?;
        made += 1;
    }
    Ok(made)
}

/// Plan callouts (enlarged plans): (view, level, crop).
fn enlarged_plans(doc: &Document) -> Vec<(ElementId, ElementId, CropBox)> {
    doc.of(Category::View)
        .filter_map(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::FloorPlan { level },
                callout_of: Some(_),
                crop: Some(c),
                ..
            } => Some((e.id, *level, *c)),
            _ => None,
        })
        .collect()
}

/// Interior elevation markers (ADR-106): every room with an enlarged plan gets one, looking
/// at all four walls, when it has none; the enlarged plan shows it, and the floor plans
/// hide it (they show the markers of rooms without enlarged plans). Returns how many were
/// placed.
pub fn ensure_interior_markers(doc: &mut Document) -> CoreResult<usize> {
    let model = studio_regen::regenerate(doc);
    let callouts = enlarged_plans(doc);
    let markers = |doc: &Document| -> Vec<(ElementId, ElementId, Pt)> {
        doc.of(Category::ElevationMarker)
            .filter_map(|e| match &e.data {
                ElementData::ElevationMarker {
                    level,
                    at,
                    interior: true,
                    ..
                } => Some((e.id, *level, *at)),
                _ => None,
            })
            .collect()
    };
    let existing = markers(doc);
    let mut made = 0;
    for (_, level, c) in &callouts {
        for r in model.rooms.iter().filter(|r| r.level == *level) {
            let Some(ring) = &r.boundary else { continue };
            if !c.contains(r.point) {
                continue;
            }
            let has = existing
                .iter()
                .any(|m| m.1 == *level && studio_geom::point_in_ring(m.2, ring));
            if has {
                continue;
            }
            let Some((lo, hi)) = studio_geom::bounds_of(ring) else {
                continue;
            };
            let mid = Pt::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
            let at = if studio_geom::point_in_ring(mid, ring) {
                mid
            } else {
                r.point
            };
            let m = studio_regen::derived::create_elevation_marker(doc, *level, at, true)?;
            for dir in ["North", "East", "South", "West"] {
                studio_regen::derived::set_property(doc, m, &format!("view_{dir}"), "yes")?;
            }
            made += 1;
        }
    }
    // Floor plans leave the markers to the enlarged plans that show them.
    let shown: Vec<ElementId> = markers(doc)
        .into_iter()
        .filter(|m| {
            callouts
                .iter()
                .any(|(_, l, c)| *l == m.1 && c.contains(m.2))
        })
        // The mark and its pointers, which are its views.
        .flat_map(|m| {
            std::iter::once(m.0).chain(
                studio_core::detail::marker_views(doc, m.0)
                    .into_iter()
                    .map(|v| v.1),
            )
        })
        .collect();
    let plans: Vec<ElementId> = doc
        .of(Category::View)
        .filter(|e| {
            matches!(
                &e.data,
                ElementData::View {
                    kind: ViewKind::FloorPlan { .. },
                    callout_of: None,
                    site: false,
                    ..
                }
            )
        })
        .map(|e| e.id)
        .collect();
    if !shown.is_empty() {
        doc.transact("Interior elevation marks on enlarged plans", |tx| {
            for v in &plans {
                tx.modify(*v, |d| {
                    if let ElementData::View { hidden, .. } = d {
                        for m in &shown {
                            if !hidden.contains(m) {
                                hidden.push(*m);
                            }
                        }
                    }
                })?;
            }
            Ok(())
        })?;
    }
    Ok(made)
}
/// Typical details from the library for what the model has (foundation, openings, roof
/// edge, floor line, stair, casework), when the project has no drafting views. Returns how
/// many were inserted.
pub fn ensure_details(doc: &mut Document) -> CoreResult<usize> {
    if doc.of(Category::View).any(|e| {
        matches!(
            &e.data,
            ElementData::View {
                kind: ViewKind::Drafting,
                ..
            }
        )
    }) {
        return Ok(0);
    }
    let model = studio_regen::regenerate(doc);
    if model.walls.is_empty() {
        return Ok(0);
    }
    let has = |c: Category| doc.of(c).next().is_some();
    let mut ids: Vec<&str> = vec![];
    let lowest = doc
        .levels()
        .into_iter()
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|l| l.0);
    let slab = model.floors.iter().any(|f| {
        Some(f.level) == lowest
            && doc
                .data(f.id)
                .ok()
                .and_then(|d| match d {
                    ElementData::Floor { type_id, .. } => doc.data(*type_id).ok(),
                    _ => None,
                })
                .is_some_and(|t| t.name().to_lowercase().contains("concrete"))
    });
    ids.push(if slab { "slab-edge" } else { "stem-wall" });
    if has(Category::Window) {
        ids.extend(["window-head", "window-sill", "window-jamb"]);
    }
    if has(Category::Door) {
        ids.push("door-threshold");
    }
    // Roof details (ADR-117): what the roof does at its edges, where a low roof meets a
    // taller wall, and at a chimney.
    let flat: Vec<&studio_regen::roof::RoofSolid> =
        model.roofs.iter().filter(|r| r.faces.is_empty()).collect();
    if !flat.is_empty() {
        // A flat roof trimmed with a fascia ends at metal edge; one without, at a parapet.
        if flat.iter().any(|r| r.fascia.is_some()) {
            ids.push("roof-edge");
        }
        if flat.iter().any(|r| r.fascia.is_none()) {
            ids.push("parapet");
        }
    }
    if model.roofs.iter().any(|r| !r.faces.is_empty()) {
        ids.extend(["eave", "rake"]);
        if has_chimney(doc) {
            ids.push("chimney-flashing");
        }
    }
    // A flat roof below another roof meets the taller walls rising past it.
    let top = model
        .roofs
        .iter()
        .map(|r| r.base)
        .fold(f64::NEG_INFINITY, f64::max);
    if flat.iter().any(|r| r.base < top - 1000.0) {
        ids.push("roof-wall");
    }
    if model.levels.len() > 2 || doc.of(Category::Floor).count() > 1 {
        ids.push("rim-joist");
    }
    if has(Category::Stair) {
        ids.extend(["stair-tread", "handrail"]);
    }
    if has(Category::Casework) {
        ids.extend(["base-cabinet", "countertop-backsplash"]);
    }
    for id in &ids {
        studio_core::details::insert(doc, id)?;
    }
    Ok(ids.len())
}

/// The project has a chimney or a fireplace: an element or type named for one.
fn has_chimney(doc: &Document) -> bool {
    doc.iter().any(|e| {
        let n = e.data.name().to_lowercase();
        (n.contains("chimney") && !n.contains("hood")) || n.contains("fireplace")
    })
}

/// The drafting view a library detail was inserted as (named for it, ADR-069).
pub fn detail_view(doc: &Document, id: &str) -> Option<ElementId> {
    let name = studio_core::details::catalog()
        .into_iter()
        .find(|d| d.id == id)?
        .name;
    doc.of(Category::View)
        .find(|e| {
            matches!(&e.data, ElementData::View { kind: ViewKind::Drafting, name: n, .. } if n.starts_with(&name))
        })
        .map(|e| e.id)
}

/// The roof plans: plan views of a level above every level with walls.
pub fn roof_plans(doc: &Document, model: &studio_regen::Model) -> Vec<ElementId> {
    let levels = doc.levels();
    let at = |l: ElementId| levels.iter().find(|x| x.0 == l).map(|x| x.2);
    let walled = model
        .walls
        .iter()
        .filter_map(|w| at(w.level))
        .fold(f64::NEG_INFINITY, f64::max);
    doc.of(Category::View)
        .filter(|e| match &e.data {
            ElementData::View {
                kind: ViewKind::FloorPlan { level },
                site: false,
                ..
            } => at(*level).is_some_and(|z| z > walled + 1.0),
            _ => false,
        })
        .map(|e| e.id)
        .collect()
}

/// The roof plan's references (ADR-117): wall sections aren't marked on it (they cut the
/// walls, not the roof); its roof's edge details are, by section marks cut across the roof's
/// edges as a wall section's mark is drawn in plan (they're sections through the edge, not
/// enlarged plans): the eave and the rake of a sloped roof, the metal fascia or parapet of a
/// flat one. Once.
pub fn ensure_roof_plan_marks(doc: &mut Document) -> CoreResult<usize> {
    let model = studio_regen::regenerate(doc);
    let plans = roof_plans(doc, &model);
    if plans.is_empty() || model.roofs.is_empty() {
        return Ok(0);
    }
    let wall_sections: Vec<ElementId> = doc
        .of(Category::View)
        .filter(|e| is_wall_section(&e.data))
        .map(|e| e.id)
        .collect();
    doc.transact("Roof plan marks", |tx| {
        for v in &plans {
            tx.modify(*v, |d| {
                if let ElementData::View { hidden, .. } = d {
                    for w in &wall_sections {
                        if !hidden.contains(w) {
                            hidden.push(*w);
                        }
                    }
                }
            })?;
        }
        Ok(())
    })?;
    // The highest roofs are the ones the roof plan shows.
    let top = model
        .roofs
        .iter()
        .map(|r| r.base)
        .fold(f64::NEG_INFINITY, f64::max);
    // Each mark: the plan it goes in, the detail, where on the roof's edge, and the edge's
    // outward normal.
    let mut marks: Vec<(ElementId, &str, Pt, Pt)> = vec![];
    let edges_of = |r: &studio_regen::RoofSolid| -> Vec<(Pt, Pt)> {
        let n = r.boundary.len();
        (0..n)
            .map(|i| (r.boundary[i], r.boundary[(i + 1) % n]))
            .collect()
    };
    // A third of the way along, clear of the section line usually through the middle, with
    // the normal pointing out of the roof.
    let along = |(p, q): &(Pt, Pt), ring: &[Pt]| {
        let m = p.lerp(*q, 0.33);
        let n = q.sub(*p).norm().perp();
        let out = if studio_geom::point_in_ring(m.add(n.scale(100.0)), ring) {
            n.scale(-1.0)
        } else {
            n
        };
        (m, out)
    };
    let longest = |edges: &[(Pt, Pt)], ring: &[Pt], pick: &dyn Fn(&(Pt, Pt)) -> bool| {
        edges
            .iter()
            .filter(|e| pick(e))
            .max_by(|a, b| a.0.dist(a.1).total_cmp(&b.0.dist(b.1)))
            .map(|e| along(e, ring))
    };
    for r in model.roofs.iter().filter(|r| r.base > top - 1000.0) {
        let edges = edges_of(r);
        let eave = |(p, q): &(Pt, Pt)| {
            let dir = q.sub(*p).norm();
            r.faces.iter().any(|f| {
                let t = f.a.sub(*p).dot(dir).clamp(0.0, p.dist(*q));
                p.add(dir.scale(t)).dist(f.a) < 50.0 && f.n.dot(dir).abs() < 0.2
            })
        };
        let ring = &r.boundary;
        let mut at = |id: &'static str, m: Option<(Pt, Pt)>| {
            if let Some((m, out)) = m {
                marks.extend(plans.iter().map(|v| (*v, id, m, out)));
            }
        };
        if r.faces.is_empty() {
            let id = if r.fascia.is_some() {
                "roof-edge"
            } else {
                "parapet"
            };
            at(id, longest(&edges, ring, &|_| true));
        } else {
            at("eave", longest(&edges, ring, &eave));
            at("rake", longest(&edges, ring, &|e| !eave(e)));
        }
    }
    // A low flat roof shows in the plan of its level: mark where it meets a taller wall.
    for r in model
        .roofs
        .iter()
        .filter(|r| r.faces.is_empty() && r.base < top - 1000.0)
    {
        let roof_top = r.base + r.thickness;
        let meets = |(p, q): &(Pt, Pt)| {
            let m = p.lerp(*q, 0.5);
            model.walls.iter().any(|w| {
                w.z1 > roof_top + 600.0
                    && studio_geom::project_to_segment(m, w.start, w.end).1 < w.thickness + 300.0
            })
        };
        let Some((m, out)) = longest(&edges_of(r), &r.boundary, &meets) else {
            continue;
        };
        for e in doc.of(Category::View) {
            if let ElementData::View {
                kind: ViewKind::FloorPlan { level },
                site: false,
                ..
            } = &e.data
            {
                if *level == r.level {
                    marks.push((e.id, "roof-wall", m, out));
                }
            }
        }
    }
    // One section mark per detail in each plan, cut across the edge as a wall section is cut
    // across its wall (5'-0" outside, 3'-6" in); none where the plan already refers to that
    // detail. A callout box an earlier version drew there becomes the section mark.
    use studio_core::references::RefShape;
    let mut made = 0;
    let mut done: Vec<(ElementId, ElementId)> = vec![];
    for (v, id, at, out) in &marks {
        let Some(target) = detail_view(doc, id) else {
            continue;
        };
        if done.contains(&(*v, target)) {
            continue;
        }
        done.push((*v, target));
        let (start, end) = section_line(*at, *out, 0.0, inside_reach(false));
        let has: Vec<(ElementId, bool)> = doc
            .iter()
            .filter_map(|e| match &e.data {
                ElementData::ViewReference {
                    view,
                    target: t,
                    shape,
                } if view == v && *t == target => {
                    Some((e.id, matches!(shape, RefShape::Callout { .. })))
                }
                _ => None,
            })
            .collect();
        if has.is_empty() {
            studio_core::references::create(
                doc,
                *v,
                RefShape::Section { start, end },
                Some(target),
            )?;
            made += 1;
            continue;
        }
        let boxes: Vec<ElementId> = has.iter().filter(|h| h.1).map(|h| h.0).collect();
        if !boxes.is_empty() {
            doc.transact("Roof plan marks", |tx| {
                for b in &boxes {
                    tx.modify(*b, |d| {
                        if let ElementData::ViewReference { shape, .. } = d {
                            *shape = RefShape::Section { start, end };
                        }
                    })?;
                }
                Ok(())
            })?;
            made += boxes.len();
        }
    }
    Ok(made)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inches_read_as_fractions() {
        assert_eq!(inches(0.875 * MM_PER_IN), "7/8\"");
        assert_eq!(inches(5.5 * MM_PER_IN), "5 1/2\"");
        assert_eq!(inches(6.0 * MM_PER_IN), "6\"");
        assert_eq!(inches(0.625 * MM_PER_IN), "5/8\"");
    }

    #[test]
    fn layers_are_noted_as_a_wall_section_notes_them() {
        let s = layer_note("Stucco", 0.875 * MM_PER_IN);
        assert!(s.starts_with("7/8\" three-coat") && s.contains("R703.7"));
        let s = layer_note("Wood Stud 2x6 with Batt Insulation", 5.5 * MM_PER_IN);
        assert!(s.starts_with("2x6 wood studs") && s.contains("R-21"));
        let s = layer_note("Gypsum Board", 0.625 * MM_PER_IN);
        assert!(s.starts_with("5/8\" gypsum board"));
        assert_eq!(short_type("Exterior - Stucco on 2x6 Stud"), "Stucco");
    }
}
