//! The review's rules, by category. Each returns findings with the code section or
//! practice it measures against. Sizes are mm internally, reported in inches and feet.

use std::collections::{BTreeMap, HashMap, HashSet};

use studio_core::units::{MM_PER_FT, MM_PER_IN};
use studio_core::{Category as Cat, ElementData, ElementId, ViewKind};
use studio_geom::{point_in_ring, project_to_segment};

use crate::ctx::{has_any, Ctx, Opening};
use crate::{Category, Finding, Severity};

fn inch(mm: f64) -> String {
    let i = mm / MM_PER_IN;
    if (i - i.round()).abs() < 0.05 {
        format!("{:.0}\"", i)
    } else {
        format!("{:.1}\"", i)
    }
}

fn ft_in(mm: f64) -> String {
    let total = (mm / MM_PER_IN).round() as i64;
    format!("{}'-{}\"", total / 12, total % 12)
}

struct F<'a> {
    c: &'a Ctx<'a>,
    out: Vec<Finding>,
}

impl<'a> F<'a> {
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        rule: &str,
        category: Category,
        severity: Severity,
        title: impl Into<String>,
        detail: impl Into<String>,
        fix: impl Into<String>,
        reference: impl Into<String>,
        elements: Vec<ElementId>,
    ) {
        let view = elements
            .first()
            .and_then(|e| self.c.view_for(*e).or_else(|| self.sheet_view(*e)));
        let key = elements.first().map_or(String::new(), |e| e.0.to_string());
        let title = title.into();
        let id = format!("{rule}:{key}:{}", title.len());
        self.out.push(Finding {
            id,
            rule: rule.into(),
            category,
            severity,
            title,
            detail: detail.into(),
            fix: fix.into(),
            reference: reference.into(),
            elements,
            view,
            source: "rules".into(),
        });
    }
    /// A sheet, or the sheet a view is on, to look at.
    fn sheet_view(&self, id: ElementId) -> Option<ElementId> {
        match self.c.doc.data(id).ok()? {
            ElementData::Sheet { .. } => Some(id),
            ElementData::View { .. } => Some(self.c.on_sheet.get(&id).copied().unwrap_or(id)),
            ElementData::TextNote { view, .. }
            | ElementData::KeynoteTag { view, .. }
            | ElementData::ViewReference { view, .. } => Some(*view),
            _ => None,
        }
    }
    /// Completeness items are expected from 50% CDs on; before that they're notes.
    fn later(&self, s: Severity) -> Severity {
        if self.c.milestone.cd() {
            s
        } else {
            Severity::Info
        }
    }
}

/// Every check.
pub fn run_checks(c: &Ctx) -> Vec<Finding> {
    let mut f = F { c, out: vec![] };
    coordination(&mut f);
    completeness(&mut f);
    code(&mut f);
    accessibility(&mut f);
    waterproofing(&mut f);
    drawing_spec(&mut f);
    constructability(&mut f);
    consultants(&mut f);
    f.out
}

fn duplicates(
    items: impl Iterator<Item = (String, ElementId)>,
) -> Vec<(String, Vec<ElementId>)> {
    let mut by: BTreeMap<String, Vec<ElementId>> = BTreeMap::new();
    for (k, id) in items {
        if !k.trim().is_empty() {
            by.entry(k.trim().to_string()).or_default().push(id);
        }
    }
    by.into_iter().filter(|(_, v)| v.len() > 1).collect()
}

// ---- Coordination ----

fn coordination(f: &mut F) {
    let c = f.c;
    let doc = c.doc;
    let marks = |door: bool| {
        duplicates(
            c.openings
                .iter()
                .filter(|o| o.door == door)
                .map(|o| (o.mark.clone(), o.id)),
        )
    };
    for (m, ids) in marks(true) {
        f.add("dup-door-mark", Category::Coordination, Severity::Major, format!("Door mark {m} is used {} times", ids.len()),
            "Two doors with one mark can't both be scheduled; the door schedule and hardware sets will disagree with the plans.",
            "Renumber the doors so each mark is unique.", "Door schedule coordination", ids);
    }
    for (m, ids) in marks(false) {
        f.add(
            "dup-window-mark",
            Category::Coordination,
            Severity::Major,
            format!("Window mark {m} is used {} times", ids.len()),
            "Windows sharing a mark can't be told apart in the window schedule.",
            "Renumber the windows.",
            "Window schedule coordination",
            ids,
        );
    }
    for (n, ids) in duplicates(c.rooms.iter().map(|r| (r.number.clone(), r.id))) {
        f.add(
            "dup-room-number",
            Category::Coordination,
            Severity::Major,
            format!("Room number {n} is used {} times", ids.len()),
            "Duplicate room numbers break the finish schedule and door-to-room references.",
            "Give each room its own number.",
            "Room and finish schedule coordination",
            ids,
        );
    }
    let sheets = doc.of(Cat::Sheet).filter_map(|e| match &e.data {
        ElementData::Sheet { number, .. } => Some((number.clone(), e.id)),
        _ => None,
    });
    for (n, ids) in duplicates(sheets) {
        f.add("dup-sheet", Category::Coordination, Severity::Critical, format!("Sheet number {n} is used {} times", ids.len()),
            "Two sheets with one number make every reference to it ambiguous and the sheet index wrong.", "Renumber one of the sheets.", "Sheet index", ids);
    }
    let grids = doc.of(Cat::Grid).filter_map(|e| match &e.data {
        ElementData::Grid { name, .. } => Some((name.clone(), e.id)),
        _ => None,
    });
    for (n, ids) in duplicates(grids) {
        f.add(
            "dup-grid",
            Category::Coordination,
            Severity::Major,
            format!("Grid {n} appears {} times", ids.len()),
            "Duplicate grid names make structural and architectural dimensions to grids ambiguous.",
            "Rename the grids.",
            "Grid coordination with structural",
            ids,
        );
    }
    let levels = doc.of(Cat::Level).filter_map(|e| match &e.data {
        ElementData::Level { name, .. } => Some((name.clone(), e.id)),
        _ => None,
    });
    for (n, ids) in duplicates(levels) {
        f.add(
            "dup-level",
            Category::Coordination,
            Severity::Major,
            format!("Level name {n} is used {} times", ids.len()),
            "Sections, elevations and consultants refer to levels by name.",
            "Rename one level.",
            "Level coordination",
            ids,
        );
    }
    // Sections and elevations not placed: their marks can't show a sheet reference.
    let unplaced: Vec<(ElementId, String)> = doc
        .of(Cat::View)
        .filter_map(|e| match &e.data {
            ElementData::View { name, kind, .. }
                if matches!(
                    kind,
                    ViewKind::Section { .. } | ViewKind::MarkerElevation { .. }
                ) && !c.on_sheet.contains_key(&e.id) =>
            {
                Some((e.id, name.clone()))
            }
            _ => None,
        })
        .collect();
    if !unplaced.is_empty() {
        let sev = f.later(Severity::Major);
        let names: Vec<&str> = unplaced.iter().take(8).map(|u| u.1.as_str()).collect();
        let title = match unplaced.as_slice() {
            [one] => format!("{} isn't on a sheet", one.1),
            _ => format!(
                "{} sections and elevations aren't on a sheet",
                unplaced.len()
            ),
        };
        f.add("ref-unplaced", Category::Coordination, sev, title,
            format!("Their marks in the plans can't show a detail and sheet number, so the references don't resolve: {}{}.", names.join(", "), if unplaced.len() > 8 { ", …" } else { "" }),
            "Place the views on sheets, or delete the marks that aren't needed.", "Drawing references", unplaced.iter().map(|u| u.0).collect());
    }
    for e in doc.of(Cat::ViewReference) {
        let ElementData::ViewReference { target, .. } = &e.data else {
            continue;
        };
        let ok = doc.get(*target).is_some() && c.on_sheet.contains_key(target);
        if !ok {
            let sev = f.later(Severity::Major);
            f.add(
                "ref-target",
                Category::Coordination,
                sev,
                "A reference callout points to a view that isn't placed",
                "The callout's bubble can't show where the detail is.",
                "Place the referenced view on a sheet, or point the callout at a placed detail.",
                "Drawing references",
                vec![e.id],
            );
        }
    }
}

// ---- Completeness & clarity ----

fn completeness(f: &mut F) {
    let c = f.c;
    let doc = c.doc;
    // Views not on sheets (not templates of the set: 3D, schedules not placed are fine early).
    let unplaced: Vec<(ElementId, String)> = doc
        .of(Cat::View)
        .filter_map(|e| match &e.data {
            ElementData::View { name, kind, .. }
                if matches!(
                    kind,
                    ViewKind::FloorPlan { .. }
                        | ViewKind::CeilingPlan { .. }
                        | ViewKind::Elevation { .. }
                        | ViewKind::Drafting
                ) && !c.on_sheet.contains_key(&e.id) =>
            {
                Some((e.id, name.clone()))
            }
            _ => None,
        })
        .collect();
    if !unplaced.is_empty() && c.milestone.cd() {
        let names: Vec<String> = unplaced.iter().take(8).map(|u| u.1.clone()).collect();
        f.add(
            "views-unplaced",
            Category::Completeness,
            Severity::Minor,
            format!("{} views aren't on any sheet", unplaced.len()),
            format!(
                "{}{}",
                names.join(", "),
                if unplaced.len() > 8 { ", …" } else { "" }
            ),
            "Place the views the set needs; delete or rename working views.",
            "Sheet index completeness",
            unplaced.iter().map(|u| u.0).collect(),
        );
    }
    let sheet_ids: Vec<(ElementId, String, String)> = doc
        .of(Cat::Sheet)
        .filter_map(|e| match &e.data {
            ElementData::Sheet { number, name, .. } => Some((e.id, number.clone(), name.clone())),
            _ => None,
        })
        .collect();
    if sheet_ids.is_empty() {
        let sev = f.later(Severity::Critical);
        f.add(
            "no-sheets",
            Category::Completeness,
            sev,
            "The project has no sheets",
            "There's no drawing set to issue yet.",
            "Create sheets (Sheets > New Sheet) and place the views.",
            "Sheet index",
            vec![],
        );
    }
    for (id, number, name) in &sheet_ids {
        if !c.on_sheet.values().any(|s| s == id) {
            let sev = f.later(Severity::Minor);
            f.add(
                "sheet-empty",
                Category::Completeness,
                sev,
                format!("Sheet {number} {name} is empty"),
                "An empty sheet in the index reads as a missing drawing.",
                "Place its views, or remove the sheet from the set.",
                "Sheet index",
                vec![*id],
            );
        }
        if name.trim().is_empty() || name.eq_ignore_ascii_case("unnamed") {
            f.add(
                "sheet-name",
                Category::Completeness,
                Severity::Minor,
                format!("Sheet {number} has no title"),
                "Every sheet needs a title for the index.",
                "Name the sheet.",
                "Sheet index",
                vec![*id],
            );
        }
    }
    // Untagged doors, windows and rooms.
    for (door, label) in [(true, "doors"), (false, "windows")] {
        let ids: Vec<ElementId> = c
            .openings
            .iter()
            .filter(|o| o.door == door && !c.tagged.contains(&o.id))
            .map(|o| o.id)
            .collect();
        if !ids.is_empty() {
            let sev = f.later(Severity::Minor);
            f.add(
                &format!("untagged-{label}"),
                Category::Completeness,
                sev,
                format!("{} {label} aren't tagged in any plan", ids.len()),
                "Untagged openings can't be found in the schedule.",
                "Tag them (Annotate > Tag All).",
                "Drawing clarity",
                ids,
            );
        }
    }
    let untagged_rooms: Vec<ElementId> = c
        .rooms
        .iter()
        .filter(|r| !c.tagged.contains(&r.id))
        .map(|r| r.id)
        .collect();
    if !untagged_rooms.is_empty() {
        let sev = f.later(Severity::Minor);
        f.add(
            "untagged-rooms",
            Category::Completeness,
            sev,
            format!("{} rooms aren't tagged", untagged_rooms.len()),
            "Room names and numbers should read on the plans.",
            "Tag the rooms.",
            "Drawing clarity",
            untagged_rooms,
        );
    }
    let unnamed: Vec<ElementId> = c
        .rooms
        .iter()
        .filter(|r| r.name.trim().is_empty() || r.name.trim() == "Room")
        .map(|r| r.id)
        .collect();
    if !unnamed.is_empty() {
        f.add("room-unnamed", Category::Completeness, Severity::Minor, format!("{} rooms are unnamed", unnamed.len()), "Rooms named \"Room\" say nothing about use, and use drives occupancy, finishes and code checks.", "Name each room for its use.", "Room data", unnamed);
    }
    // Schedules.
    let has_schedule = |k: studio_core::element::ScheduleKind| {
        doc.of(Cat::View).any(|e| matches!(&e.data, ElementData::View { kind: ViewKind::Schedule { kind }, .. } if *kind == k))
    };
    use studio_core::element::ScheduleKind as SK;
    for (need, kind, label) in [
        (
            c.openings.iter().any(|o| o.door),
            SK::Doors,
            "door schedule",
        ),
        (
            c.openings.iter().any(|o| !o.door),
            SK::Windows,
            "window schedule",
        ),
        (!c.rooms.is_empty(), SK::Rooms, "room finish schedule"),
        (!sheet_ids.is_empty(), SK::Sheets, "sheet index"),
    ] {
        if need && !has_schedule(kind) {
            let sev = f.later(Severity::Minor);
            f.add(
                &format!("no-{}", label.replace(' ', "-")),
                Category::Completeness,
                sev,
                format!("There's no {label}"),
                format!("The set has the elements but no {label} to list them."),
                format!("Add a {label} (View > Schedules) and place it."),
                "Set completeness",
                vec![],
            );
        }
    }
    // TBDs and placeholders in notes.
    for e in doc.of(Cat::TextNote) {
        let ElementData::TextNote { text, .. } = &e.data else {
            continue;
        };
        let u = text.to_uppercase();
        let hit = ["TBD", "TBC", "???", "VIF", "VERIFY"].iter().find(|w| {
            u.split(|ch: char| !ch.is_alphanumeric() && ch != '?')
                .any(|t| t == **w)
                || (w.contains('?') && u.contains(**w))
        });
        let xx = u
            .split(|ch: char| !ch.is_alphanumeric())
            .any(|t| t.len() >= 2 && t.chars().all(|ch| ch == 'X'));
        if let Some(w) = hit.copied().or(xx.then_some("XX")) {
            let sev = if c.milestone.issued() && w != "VERIFY" && w != "VIF" {
                Severity::Critical
            } else if c.milestone.cd() {
                Severity::Major
            } else {
                Severity::Info
            };
            let snippet: String = text.chars().take(60).collect();
            f.add("tbd", Category::Completeness, sev, format!("A note still says {w}"), format!("\"{snippet}\""),
                "Resolve it before the set goes out; notes left as TBD become RFIs and change orders.", "Completeness", vec![e.id]);
        }
    }
    // Keynotes that don't resolve.
    let (table, _) = studio_core::keynotes::table(doc);
    let keys: HashSet<&str> = table.iter().map(|k| k.key.as_str()).collect();
    for e in doc.of(Cat::KeynoteTag) {
        let ElementData::KeynoteTag { source, .. } = &e.data else {
            continue;
        };
        match studio_core::keynotes::tag_key(doc, source) {
            Some(k) if keys.contains(k.as_str()) => {}
            Some(k) => f.add(
                "keynote-missing",
                Category::Completeness,
                Severity::Major,
                format!("Keynote {k} isn't in the keynote table"),
                "The tag will print a key with no text in the legend.",
                "Add the keynote to the table (Keynote Manager) or pick another.",
                "Keynote coordination",
                vec![e.id],
            ),
            None => f.add(
                "keynote-blank",
                Category::Completeness,
                Severity::Major,
                "A keynote tag has no keynote",
                "The element it tags has no keynote assigned, so the tag prints \"?\".",
                "Assign a keynote to the element or its material.",
                "Keynote coordination",
                vec![e.id],
            ),
        }
    }
    // Project information on the title block.
    let d = &c.details;
    let mut missing = vec![];
    if c.project_name.trim().is_empty() || c.project_name == "New Project" {
        missing.push("project name");
    }
    if c.project_number.trim().is_empty() || c.project_number == "0001" {
        missing.push("project number");
    }
    if d.location.one_line().is_empty() {
        missing.push("address");
    }
    if d.client.is_empty() {
        missing.push("client");
    }
    if !d
        .team
        .iter()
        .any(|m| m.discipline == "Architect" && !m.contact.is_empty())
    {
        missing.push("architect of record");
    }
    if !missing.is_empty() {
        let sev = f.later(Severity::Minor);
        f.add(
            "project-info",
            Category::Completeness,
            sev,
            format!("Project information is missing the {}", missing.join(", ")),
            "Title blocks, the project manual and the permit application draw on it.",
            "Fill it in on the Project Info tab.",
            "Title block completeness",
            vec![],
        );
    }
}

// ---- Code compliance ----

fn code(f: &mut F) {
    let c = f.c;
    let doc = c.doc;
    // Emergency escape and rescue openings in sleeping rooms.
    for r in c
        .rooms
        .iter()
        .filter(|r| r.is(&["bed", "bedroom", "sleeping", "bunk"]))
    {
        let windows: Vec<&Opening> = c
            .openings
            .iter()
            .filter(|o| !o.door && c.rooms_at(o).iter().any(|x| x.id == r.id))
            .collect();
        let exterior_door = c.openings.iter().any(|o| {
            o.door
                && c.wall(o.host).is_some_and(|w| w.exterior)
                && c.rooms_at(o).iter().any(|x| x.id == r.id)
        });
        if exterior_door {
            continue;
        }
        let grade = c.lowest_level == Some(r.level)
            || c.level_elev.get(&r.level).copied().unwrap_or(0.0).abs() < 600.0;
        let min_area = if grade { 5.0 } else { 5.7 };
        let qualifies = |o: &Opening| {
            o.clear.is_some_and(|(w, h)| {
                w >= 20.0 * MM_PER_IN
                    && h >= 24.0 * MM_PER_IN
                    && w * h / (MM_PER_FT * MM_PER_FT) >= min_area
            }) && o.sill <= 44.0 * MM_PER_IN
        };
        if windows.is_empty() {
            f.add("egress-none", Category::Code, Severity::Critical, format!("{} has no emergency escape opening", r.name),
                "Every sleeping room needs an operable window or exterior door for emergency escape and rescue.",
                "Add an egress window (sill ≤ 44\", clear opening ≥ 5.7 sf, 20\" wide and 24\" high) or an exterior door.",
                c.code("R310.1", "1031.2"), vec![r.id]);
        } else if !windows.iter().any(|o| qualifies(o)) {
            let best = windows
                .iter()
                .filter_map(|o| o.clear.map(|cl| (o, cl)))
                .max_by(|a, b| (a.1 .0 * a.1 .1).total_cmp(&(b.1 .0 * b.1 .1)));
            let detail = match best {
                Some((o, (w, h))) => format!(
                    "Its best window ({}) opens about {} x {} ({:.1} sf clear) with a {} sill; escape needs ≥ {min_area} sf, 20\" x 24\" clear, sill ≤ 44\".",
                    o.type_name, inch(w), inch(h), w * h / (MM_PER_FT * MM_PER_FT), inch(o.sill)
                ),
                None => "Its windows are fixed (they don't open).".into(),
            };
            f.add("egress-size", Category::Code, Severity::Critical, format!("{} has no window big enough for emergency escape", r.name), detail,
                "Use a larger casement, or a double-hung or slider sized for egress, with the sill at or below 44\".",
                c.code("R310.2.1", "1031.3.1"), windows.iter().map(|o| o.id).chain([r.id]).collect());
        }
    }
    // Stairs.
    for e in doc.of(Cat::Stair) {
        let ElementData::Stair {
            base_level,
            top_level,
            width,
            tread,
            max_riser,
            railings,
            ..
        } = &e.data
        else {
            continue;
        };
        let rise = c.level_elev.get(top_level).copied().unwrap_or(0.0)
            - c.level_elev.get(base_level).copied().unwrap_or(0.0);
        if rise <= 0.0 {
            continue;
        }
        let n = (rise / max_riser - 1e-6).ceil().max(1.0);
        let riser = rise / n;
        let (max_r, min_t, min_w) = if c.residential {
            (7.75, 10.0, 36.0)
        } else {
            (7.0, 11.0, 44.0)
        };
        if riser > max_r * MM_PER_IN + 0.5 {
            f.add(
                "stair-riser",
                Category::Code,
                Severity::Critical,
                format!("Stair risers are {} (max {max_r}\")", inch(riser)),
                format!("{} risers over {}.", n, ft_in(rise)),
                "Add risers (lower the maximum riser height) so each is within the limit.",
                c.code("R311.7.5.1", "1011.5.2"),
                vec![e.id],
            );
        }
        if *tread < min_t * MM_PER_IN - 0.5 {
            f.add(
                "stair-tread",
                Category::Code,
                Severity::Critical,
                format!("Stair treads are {} deep (min {min_t}\")", inch(*tread)),
                "Tread depth is measured nosing to nosing.",
                "Deepen the treads.",
                c.code("R311.7.5.2", "1011.5.2"),
                vec![e.id],
            );
        }
        if *width < min_w * MM_PER_IN - 0.5 {
            f.add(
                "stair-width",
                Category::Code,
                Severity::Major,
                format!("The stair is {} wide (min {min_w}\")", inch(*width)),
                if c.residential {
                    "Clear width above the handrail height."
                } else {
                    "44\" for an occupant load of 50 or more; 36\" below 50."
                },
                "Widen the stair, or confirm the occupant load allows it.",
                c.code("R311.7.1", "1011.2"),
                vec![e.id],
            );
        }
        let near_rail = doc.of(Cat::Railing).next().is_some();
        if !railings && !near_rail {
            f.add("stair-handrail", Category::Code, Severity::Major, "The stair has no handrail",
                "Stairs with four or more risers need a handrail (both sides in most commercial stairs).", "Turn on the stair's railings or add a railing.",
                c.code("R311.7.8", "1011.11"), vec![e.id]);
        }
    }
    // Guards and handrails: railing heights.
    for e in doc.of(Cat::Railing) {
        let ElementData::Railing { type_id, .. } = &e.data else {
            continue;
        };
        if let Ok(ElementData::RailingType { name, height }) = doc.data(*type_id) {
            let min = if c.residential { 36.0 } else { 42.0 };
            if *height < 34.0 * MM_PER_IN - 1.0 {
                f.add(
                    "rail-height",
                    Category::Code,
                    Severity::Major,
                    format!("Railing {name} is {} high", inch(*height)),
                    format!("Guards need {min}\" minimum; handrails 34\"–38\"."),
                    "Raise the railing.",
                    c.code("R312.1.2", "1015.3"),
                    vec![e.id],
                );
            }
        }
    }
    // Ceiling heights in habitable rooms.
    let min_ceiling = if c.residential {
        7.0 * MM_PER_FT
    } else {
        7.5 * MM_PER_FT
    };
    for e in doc.of(Cat::Ceiling) {
        let ElementData::Ceiling {
            level,
            height,
            boundary,
            ..
        } = &e.data
        else {
            continue;
        };
        let Some((lo, hi)) = studio_geom::bounds_of(boundary) else {
            continue;
        };
        let mid = lo.add(hi).scale(0.5);
        let room = c.rooms.iter().find(|r| {
            r.level == *level && r.ring.as_ref().is_some_and(|ring| point_in_ring(mid, ring))
        });
        let Some(room) = room else { continue };
        if room.is(&[
            "bath", "closet", "laundry", "storage", "mech", "garage", "pantry", "wc", "powder",
        ]) {
            continue;
        }
        if *height < min_ceiling - 1.0 {
            f.add(
                "ceiling-height",
                Category::Code,
                Severity::Major,
                format!("{} has a {} ceiling", room.name, ft_in(*height)),
                format!("Habitable rooms need {} minimum.", ft_in(min_ceiling)),
                "Raise the ceiling, or confirm an exception applies (beams, sloped ceilings).",
                c.code("R305.1", "1208.2"),
                vec![e.id, room.id],
            );
        }
    }
    // Garage–dwelling separation.
    let garages: Vec<ElementId> = c
        .rooms
        .iter()
        .filter(|r| r.is(&["garage"]))
        .map(|r| r.id)
        .collect();
    if !garages.is_empty() && c.residential {
        for w in &c.walls {
            let along = c.rooms_along(w);
            let g = along.iter().any(|r| garages.contains(&r.id));
            let house = along.iter().any(|r| !garages.contains(&r.id));
            if !(g && house) {
                continue;
            }
            if !has_any(
                &w.layers,
                &["type x", "5/8", "rated", "fire", "gypsum", "gwb", "drywall"],
            ) {
                f.add("garage-wall", Category::Code, Severity::Major, format!("The garage wall ({}) shows no gypsum separation", w.type_name),
                    "The garage side of walls between the garage and the house needs at least 1/2\" gypsum board (5/8\" Type X on ceilings below habitable rooms).",
                    "Use a wall type with 1/2\" gypsum (or 5/8\" Type X) on the garage side.", c.code("R302.6", "406.3.2"), vec![w.id]);
            }
            for o in c.openings.iter().filter(|o| o.door && o.host == w.id) {
                let n = o.type_name.to_lowercase();
                if !has_any(
                    &n,
                    &[
                        "solid", "20 min", "20-min", "fire", "rated", "s.c.", "steel", "metal",
                    ],
                ) {
                    f.add("garage-door", Category::Code, Severity::Major, format!("Door {} between the garage and the house isn't rated", if o.mark.is_empty() { &o.type_name } else { &o.mark }),
                        "It must be solid wood or solid- or honeycomb-core steel at least 1-3/8\" thick, or 20-minute rated (self-closing where required locally).",
                        "Change the door to a solid-core or 20-minute rated type.", c.code("R302.5.1", "406.3.2"), vec![o.id]);
                }
            }
        }
    }
    // Commercial: occupant load and the number of exits.
    if !c.residential {
        let occ = c.details.codes.occupancy.to_uppercase();
        let (factor, what) = if occ.starts_with('A') {
            (15.0, "assembly, unconcentrated (15 net sf/occupant)")
        } else if occ.starts_with('M') {
            (60.0, "mercantile (60 gross sf/occupant)")
        } else if occ.starts_with('E') {
            (20.0, "educational classrooms (20 net sf/occupant)")
        } else if occ.starts_with('S') {
            (300.0, "storage (300 gross sf/occupant)")
        } else {
            (150.0, "business (150 gross sf/occupant)")
        };
        for r in &c.rooms {
            if r.area_sf <= 0.0 {
                continue;
            }
            let load = (r.area_sf / factor).ceil();
            let doors = c
                .openings
                .iter()
                .filter(|o| o.door && c.rooms_at(o).iter().any(|x| x.id == r.id))
                .count();
            if load > 49.0 && doors < 2 {
                f.add("exits", Category::Code, Severity::Critical, format!("{} needs two exits (occupant load {load:.0})", r.name),
                    format!("{:.0} sf at {what} is {load:.0} occupants; spaces over 49 need two exit access doorways, it has {doors}.", r.area_sf),
                    "Add a second door, separated by at least one-third of the room's diagonal (one-half unsprinklered).", c.code("", "1006.2.1"), vec![r.id]);
            }
        }
        for r in c
            .rooms
            .iter()
            .filter(|r| r.is(&["corridor", "hall", "hallway"]))
        {
            let w = r.min_dim();
            if w > 0.0 && w < 44.0 * MM_PER_IN - 1.0 {
                f.add(
                    "corridor",
                    Category::Code,
                    Severity::Major,
                    format!("{} is {} wide", r.name, inch(w)),
                    "Corridors need 44\" (36\" where the occupant load is under 50).",
                    "Widen the corridor or confirm the load.",
                    c.code("", "1020.3"),
                    vec![r.id],
                );
            }
        }
    } else {
        for r in c
            .rooms
            .iter()
            .filter(|r| r.is(&["hall", "hallway", "corridor"]))
        {
            let w = r.min_dim();
            if w > 0.0 && w < 36.0 * MM_PER_IN - 1.0 {
                f.add(
                    "hallway",
                    Category::Code,
                    Severity::Major,
                    format!("{} is {} wide (min 36\")", r.name, inch(w)),
                    "Hallways need a 3'-0\" minimum clear width.",
                    "Widen the hallway.",
                    c.code("R311.6", ""),
                    vec![r.id],
                );
            }
        }
    }
    // Egress door.
    let ext_doors: Vec<&Opening> = c
        .openings
        .iter()
        .filter(|o| o.door && c.wall(o.host).is_some_and(|w| w.exterior))
        .collect();
    if !c.rooms.is_empty() {
        if ext_doors.is_empty() {
            f.add(
                "egress-door",
                Category::Code,
                Severity::Critical,
                "The building has no exterior door",
                "There's no way out.",
                "Add an exterior egress door.",
                c.code("R311.1", "1006"),
                vec![],
            );
        } else if c.residential
            && !ext_doors
                .iter()
                .any(|o| o.width >= 34.0 * MM_PER_IN - 1.0 && o.height >= 78.0 * MM_PER_IN - 1.0)
        {
            f.add("egress-door-size", Category::Code, Severity::Major, "No exterior door qualifies as the egress door",
                "At least one side-hinged exterior door needs 32\" clear (a 34\"–36\" door) and 78\" clear height.", "Make the main entry a 3'-0\" x 6'-8\" (or taller) door.",
                c.code("R311.2", ""), ext_doors.iter().map(|o| o.id).collect());
        }
    }
    for o in c
        .openings
        .iter()
        .filter(|o| o.door && !c.residential && o.height < 80.0 * MM_PER_IN - 1.0 && o.height > 0.0)
    {
        f.add(
            "door-height",
            Category::Code,
            Severity::Major,
            format!(
                "Door {} is {} high",
                if o.mark.is_empty() {
                    &o.type_name
                } else {
                    &o.mark
                },
                ft_in(o.height)
            ),
            "Means-of-egress doors need 80\" minimum clear height.",
            "Use a 6'-8\" or taller door.",
            c.code("", "1010.1.1"),
            vec![o.id],
        );
    }
    // Energy: insulation in exterior walls and roofs.
    let energy = if c.california {
        "Title 24 Part 6 (California Energy Code)".to_string()
    } else if c.residential {
        "IECC R402.1".to_string()
    } else {
        "IECC C402.1".to_string()
    };
    let mut seen = HashSet::new();
    for w in c.walls.iter().filter(|w| w.exterior && w.has_layers) {
        if seen.insert(w.type_id)
            && !has_any(
                &w.layers,
                &[
                    "insulation",
                    "batt",
                    "rigid",
                    "spray foam",
                    "mineral wool",
                    "polyiso",
                    "xps",
                    "eps",
                ],
            )
        {
            let ids: Vec<ElementId> = c
                .walls
                .iter()
                .filter(|x| x.type_id == w.type_id)
                .map(|x| x.id)
                .collect();
            f.add(
                "wall-insulation",
                Category::Code,
                Severity::Major,
                format!("Exterior wall type {} has no insulation layer", w.type_name),
                "The energy code sets minimum wall R-values by climate zone.",
                "Add an insulation layer (cavity and/or continuous) to the wall type.",
                energy.clone(),
                ids,
            );
        }
    }
    for e in doc.of(Cat::RoofType) {
        let ElementData::RoofType { name, layers, .. } = &e.data else {
            continue;
        };
        let used: Vec<ElementId> = doc
            .of(Cat::Roof)
            .filter(|r| r.data.type_id() == Some(e.id))
            .map(|r| r.id)
            .collect();
        if used.is_empty() || layers.is_empty() {
            continue;
        }
        let t = crate::ctx::text_of(doc, layers);
        if !has_any(
            &t,
            &[
                "insulation",
                "batt",
                "rigid",
                "polyiso",
                "spray",
                "xps",
                "eps",
                "blown",
            ],
        ) {
            f.add(
                "roof-insulation",
                Category::Code,
                Severity::Major,
                format!("Roof type {name} has no insulation layer"),
                "Ceilings and roofs have the energy code's highest R-values.",
                "Add insulation to the roof type (or confirm it's in the attic floor).",
                energy.clone(),
                used,
            );
        }
    }
}

// ---- Accessibility (commercial) ----

fn accessibility(f: &mut F) {
    let c = f.c;
    if c.residential {
        return;
    }
    for o in c.openings.iter().filter(|o| o.door) {
        let rooms = c.rooms_at(o);
        if rooms.iter().any(|r| {
            r.is(&[
                "closet", "storage", "shaft", "chase", "janitor", "jan", "elec", "mech",
            ])
        }) {
            continue;
        }
        // A door's clear width is about 2" less than its nominal width (stop and leaf).
        if o.width > 0.0 && o.width < 34.0 * MM_PER_IN - 1.0 {
            f.add(
                "door-clear",
                Category::Accessibility,
                Severity::Major,
                format!(
                    "Door {} is {} wide: about {} clear",
                    if o.mark.is_empty() {
                        &o.type_name
                    } else {
                        &o.mark
                    },
                    inch(o.width),
                    inch(o.width - 2.0 * MM_PER_IN)
                ),
                "Doors on an accessible route need 32\" clear width with the door open 90°.",
                "Use a 3'-0\" door.",
                c.access("404.2.3"),
                vec![o.id],
            );
        }
    }
    for r in c.rooms.iter().filter(|r| {
        r.is(&[
            "restroom", "toilet", "bath", "wc", "lavatory", "men", "women", "unisex",
        ])
    }) {
        let d = r.min_dim();
        if d > 0.0 && d < 60.0 * MM_PER_IN - 1.0 {
            f.add("turning", Category::Accessibility, Severity::Major, format!("{} is {} across: no 60\" turning space", r.name, inch(d)),
                "Accessible toilet rooms need a 60\" diameter (or T-shaped) turning space clear of fixtures.", "Enlarge the room or rework the layout.", c.access("304.3"), vec![r.id]);
        }
    }
    let levels_above = c.doc.levels().iter().filter(|l| l.2 > 1000.0).count();
    let elevator = c.rooms.iter().any(|r| r.is(&["elevator", "elev", "lift"]));
    if levels_above > 0 && !elevator && !c.rooms.is_empty() {
        f.add("vertical-access", Category::Accessibility, Severity::Minor, "There's no elevator or lift to the upper floors",
            "Multistory buildings generally need an accessible route to each floor (exceptions depend on size and use).", "Add an elevator or confirm an exception applies.", c.access("206.2.3"), vec![]);
    }
}

// ---- Waterproofing & envelope ----

fn waterproofing(f: &mut F) {
    let c = f.c;
    let doc = c.doc;
    let mut seen = HashSet::new();
    for w in c.walls.iter().filter(|w| w.exterior) {
        if !seen.insert(w.type_id) {
            continue;
        }
        let ids: Vec<ElementId> = c
            .walls
            .iter()
            .filter(|x| x.type_id == w.type_id)
            .map(|x| x.id)
            .collect();
        if !w.has_layers {
            f.add(
                "wrb-unknown",
                Category::Waterproofing,
                Severity::Info,
                format!("Exterior wall type {} has no layers", w.type_name),
                "Its water-resistive barrier and flashing can't be checked.",
                "Give the wall type its layers (Edit Type > Structure).",
                c.code("R703.2", "1403.2"),
                ids,
            );
            continue;
        }
        if !has_any(
            &w.layers,
            &[
                "weather",
                "wrb",
                "water-resistive",
                "water resistive",
                "house wrap",
                "housewrap",
                "building paper",
                "felt",
                "membrane",
                "air barrier",
                "tyvek",
            ],
        ) {
            f.add("wrb", Category::Waterproofing, Severity::Major, format!("Exterior wall type {} shows no water-resistive barrier", w.type_name),
                "Exterior walls need a water-resistive barrier behind the cladding, lapped and integrated with flashing at openings.",
                "Add a weather barrier (membrane) layer to the wall type.", c.code("R703.2", "1403.2"), ids.clone());
        }
        let below = c.level_elev.get(&w.level).copied().unwrap_or(0.0) < -900.0;
        if below
            && !has_any(
                &w.layers,
                &[
                    "dampproof",
                    "damp-proof",
                    "waterproof",
                    "bituminous",
                    "drainage",
                    "membrane",
                ],
            )
        {
            f.add("below-grade", Category::Waterproofing, Severity::Major, format!("Below-grade wall type {} has no dampproofing or waterproofing", w.type_name),
                "Foundation walls enclosing habitable space below grade need dampproofing (waterproofing where there's a high water table) and drainage.",
                "Add a dampproofing or waterproofing layer, and a drainage board.", c.code("R406.1", "1805.2"), ids);
        }
    }
    // Wet rooms: tile backers and floor finishes.
    let wet: Vec<&crate::ctx::RoomRec> = c
        .rooms
        .iter()
        .filter(|r| {
            r.is(&[
                "bath", "shower", "restroom", "toilet", "laundry", "wc", "powder", "mud",
            ])
        })
        .collect();
    let mut flagged = HashSet::new();
    for r in &wet {
        for w in c
            .walls
            .iter()
            .filter(|w| w.has_layers && c.rooms_along(w).iter().any(|x| x.id == r.id))
        {
            if r.is(&["bath", "shower"])
                && flagged.insert(w.type_id)
                && !has_any(
                    &w.layers,
                    &[
                        "cement board",
                        "cementitious",
                        "backer",
                        "moisture",
                        "mold",
                        "water-resistant",
                        "waterproof",
                        "tile",
                        "glass mat",
                        "densshield",
                        "mr ",
                    ],
                )
            {
                f.add("wet-backer", Category::Waterproofing, Severity::Minor, format!("Wall type {} in {} has no moisture-resistant backer", w.type_name, r.name),
                    "Tile in showers and tub surrounds needs a cement, fiber-cement or glass-mat backer (with a waterproof membrane at the shower).",
                    "Use a bath wall type with a tile backer, or note it in the finish schedule.", c.code("R702.4.2", "1210.2.2"), vec![w.id]);
            }
        }
        for e in doc.of(Cat::Floor) {
            let ElementData::Floor {
                type_id,
                boundary,
                level,
                ..
            } = &e.data
            else {
                continue;
            };
            if *level != r.level {
                continue;
            }
            let Some((lo, hi)) = studio_geom::bounds_of(boundary) else {
                continue;
            };
            let inside = r
                .ring
                .as_ref()
                .is_some_and(|ring| point_in_ring(lo.add(hi).scale(0.5), ring));
            if !inside {
                continue;
            }
            if let Ok(ElementData::FloorType { name, layers, .. }) = doc.data(*type_id) {
                let t = format!(
                    "{} {}",
                    name.to_lowercase(),
                    crate::ctx::text_of(doc, layers)
                );
                if has_any(&t, &["carpet", "hardwood", "wood floor", "oak", "laminate"]) {
                    f.add("wet-floor", Category::Waterproofing, Severity::Minor, format!("{} has a {} floor", r.name, name),
                        "Wet rooms need water-resistant flooring (tile, sheet vinyl, sealed concrete).", "Change the floor finish in the wet room.", if c.residential { "Good practice (water-resistant wet-area floors)".to_string() } else { c.code("", "1210.2.1") }, vec![e.id]);
                }
            }
        }
    }
    // Roofs: slope against the roof covering, and underlayment.
    for e in doc.of(Cat::Roof) {
        let ElementData::Roof {
            type_id,
            slope,
            sloped,
            ..
        } = &e.data
        else {
            continue;
        };
        let Ok(ElementData::RoofType { name, layers, .. }) = doc.data(*type_id) else {
            continue;
        };
        let t = format!(
            "{} {}",
            name.to_lowercase(),
            crate::ctx::text_of(doc, layers)
        );
        let flat = !sloped.iter().any(|s| *s);
        let s12 = if flat { 0.0 } else { slope * 12.0 };
        if has_any(&t, &["shingle", "asphalt"]) {
            if flat || s12 < 2.0 - 1e-6 {
                f.add("roof-shingle-slope", Category::Waterproofing, Severity::Critical, format!("{name} has shingles at {s12:.1}:12"),
                    "Asphalt shingles need 2:12 minimum; 2:12 to 4:12 needs a double layer of underlayment.", "Steepen the roof or change to a low-slope membrane.",
                    c.code("R905.2.2", "1507.2.2"), vec![e.id]);
            } else if s12 < 4.0 - 1e-6 {
                f.add(
                    "roof-shingle-double",
                    Category::Waterproofing,
                    Severity::Minor,
                    format!("{name} at {s12:.1}:12 needs double underlayment"),
                    "Between 2:12 and 4:12 shingles need two layers of underlayment.",
                    "Note double underlayment on the roof plan and in 07 31 13.",
                    c.code("R905.1.1", "1507.1.1"),
                    vec![e.id],
                );
            }
        }
        if has_any(
            &t,
            &["tpo", "epdm", "membrane", "pvc", "built-up", "modified"],
        ) && flat
        {
            f.add("roof-drain-slope", Category::Waterproofing, Severity::Major, format!("{name} is modeled dead flat"),
                "Membrane roofs need 1/4\":12 minimum slope to drain (tapered insulation or sloped framing), with drains or scuppers and overflows.",
                "Slope the roof or add tapered insulation; show drains and overflows.", c.code("R905.13.1", "1507.12.1 / 1507.13.1"), vec![e.id]);
        }
        if !layers.is_empty()
            && !flat
            && !has_any(
                &t,
                &[
                    "underlayment",
                    "felt",
                    "ice",
                    "synthetic",
                    "membrane",
                    "self-adhered",
                ],
            )
        {
            f.add("roof-underlayment", Category::Waterproofing, Severity::Minor, format!("{name} shows no underlayment"),
                "Steep-slope roof coverings go over underlayment (ice barrier at eaves in cold climates).", "Add an underlayment layer to the roof type.", c.code("R905.1.1", "1507.1.1"), vec![e.id]);
        }
    }
    // Slab-on-grade vapor retarder.
    if let Some(low) = c.lowest_level {
        let mut seen = HashSet::new();
        for e in doc.of(Cat::Floor) {
            let ElementData::Floor { type_id, level, .. } = &e.data else {
                continue;
            };
            if *level != low || !seen.insert(*type_id) {
                continue;
            }
            if let Ok(ElementData::FloorType { name, layers, .. }) = doc.data(*type_id) {
                let t = format!(
                    "{} {}",
                    name.to_lowercase(),
                    crate::ctx::text_of(doc, layers)
                );
                let slab = has_any(&t, &["slab", "concrete"]);
                if slab
                    && !layers.is_empty()
                    && !has_any(&t, &["vapor", "poly", "retarder", "membrane", "barrier"])
                {
                    f.add("slab-vapor", Category::Waterproofing, Severity::Minor, format!("Slab type {name} shows no vapor retarder"),
                        "Slabs on grade under conditioned space need a 6-mil (10-mil recommended) vapor retarder over a capillary break.",
                        "Add a vapor retarder layer below the slab.", c.code("R506.2.3", "1907.1"), vec![e.id]);
                }
            }
        }
    }
}

// ---- Drawings to specifications ----

fn drawing_spec(f: &mut F) {
    let c = f.c;
    let doc = c.doc;
    let Some(book) = studio_core::specs::book(doc) else {
        if c.milestone.cd() {
            f.add(
                "no-specs",
                Category::DrawingSpec,
                if c.milestone.issued() {
                    Severity::Critical
                } else {
                    Severity::Major
                },
                "There's no project manual",
                "The drawings reference materials and systems the specifications should define.",
                "Generate the project manual on the Specifications tab.",
                "Drawing-to-spec consistency",
                vec![],
            );
        }
        return;
    };
    let facts = studio_specs::features::facts(doc, c.model);
    let (_, upd) = studio_specs::generate::update(&book, &facts);
    for n in &upd.added {
        let title = studio_specs::library::entry(n)
            .map(|l| l.section.title.clone())
            .unwrap_or_default();
        let why = studio_specs::library::entry(n)
            .and_then(|l| facts.reason(&l.when))
            .unwrap_or_default();
        f.add(
            "spec-missing",
            Category::DrawingSpec,
            Severity::Major,
            format!("The drawings show work with no spec section: {n} {title}"),
            format!("The model has {why}, but the project manual has no {n}."),
            "Add the section (Specifications > Update from Model).",
            "Drawing-to-spec consistency",
            vec![],
        );
    }
    for s in book.sections.iter().filter(|s| !s.included) {
        let called = studio_specs::library::entry(&s.number).is_some_and(|l| facts.picks(&l.when));
        if called {
            f.add(
                "spec-excluded",
                Category::DrawingSpec,
                Severity::Major,
                format!(
                    "{} {} is excluded, but the model calls for it",
                    s.number, s.title
                ),
                "The drawings show this work; an excluded section won't be issued.",
                "Include the section, or change the drawings.",
                "Drawing-to-spec consistency",
                vec![],
            );
        }
    }
    for r in studio_specs::coord::missing_references(&book) {
        f.add(
            "spec-ref",
            Category::DrawingSpec,
            Severity::Minor,
            format!(
                "Section {} refers to {}{}, which isn't issued",
                r.from.join(", "),
                r.to,
                r.title
                    .as_ref()
                    .map(|t| format!(" {t}"))
                    .unwrap_or_default()
            ),
            if r.excluded {
                "The referenced section is in the manual but excluded."
            } else {
                "The referenced section isn't in the manual."
            },
            "Add or include the section, or edit the reference.",
            "Specification coordination",
            vec![],
        );
    }
    // Keynotes keyed to spec sections ("09 29 00.A1"), and notes naming sections.
    let issued: HashSet<&str> = book
        .sections
        .iter()
        .filter(|s| s.included)
        .map(|s| s.number.as_str())
        .collect();
    let (table, _) = studio_core::keynotes::table(doc);
    let mut used = HashMap::new();
    for e in doc.of(Cat::KeynoteTag) {
        if let ElementData::KeynoteTag { source, .. } = &e.data {
            if let Some(k) = studio_core::keynotes::tag_key(doc, source) {
                used.entry(k).or_insert_with(Vec::new).push(e.id);
            }
        }
    }
    for (k, ids) in used {
        let sec: String = k.chars().take(8).collect();
        if studio_core::specs::valid_number(&sec) && !issued.contains(sec.as_str()) {
            let text = table
                .iter()
                .find(|x| x.key == k)
                .map(|x| x.text.clone())
                .unwrap_or_default();
            f.add(
                "keynote-spec",
                Category::DrawingSpec,
                Severity::Major,
                format!("Keynote {k} points to Section {sec}, which isn't issued"),
                format!("\"{text}\""),
                "Add the section to the manual, or re-key the keynote.",
                "Drawing-to-spec consistency",
                ids,
            );
        }
    }
    for e in doc.of(Cat::TextNote) {
        let ElementData::TextNote { text, .. } = &e.data else {
            continue;
        };
        for n in studio_specs::coord::numbers_in(text) {
            if !issued.contains(n.as_str()) {
                f.add(
                    "note-spec",
                    Category::DrawingSpec,
                    Severity::Major,
                    format!("A note refers to Section {n}, which isn't issued"),
                    format!("\"{}\"", text.chars().take(60).collect::<String>()),
                    "Add the section, or correct the note.",
                    "Drawing-to-spec consistency",
                    vec![e.id],
                );
            }
        }
    }
}

// ---- Constructability ----

fn constructability(f: &mut F) {
    let c = f.c;
    for w in &c.walls {
        if w.len < 6.0 * MM_PER_IN {
            f.add("wall-short", Category::Constructability, Severity::Minor, format!("A {} wall is only {} long", w.type_name, inch(w.len)),
                "Very short walls are usually drafting leftovers that confuse framing and quantities.", "Delete it, or join it properly.", "Model integrity", vec![w.id]);
        }
    }
    // Overlapping walls.
    for (i, a) in c.walls.iter().enumerate() {
        for b in c.walls.iter().skip(i + 1) {
            if a.level != b.level || a.len < 1.0 || b.len < 1.0 {
                continue;
            }
            let (ta, da) = project_to_segment(b.start, a.start, a.end);
            let (tb, db) = project_to_segment(b.end, a.start, a.end);
            let _ = (ta, tb);
            if da > 25.0 || db > 25.0 {
                continue;
            }
            // Collinear: how much of b lies along a.
            let dir = a.end.sub(a.start).scale(1.0 / a.len);
            let s = |p: studio_geom::Pt| p.sub(a.start).dot(dir);
            let (b0, b1) = (s(b.start).min(s(b.end)), s(b.start).max(s(b.end)));
            let overlap = b1.min(a.len) - b0.max(0.0);
            if overlap > 100.0 {
                f.add(
                    "wall-overlap",
                    Category::Constructability,
                    Severity::Major,
                    format!("Two walls overlap for {}", inch(overlap)),
                    format!(
                        "{} and {} run on top of each other.",
                        a.type_name, b.type_name
                    ),
                    "Delete or trim one of them.",
                    "Model integrity",
                    vec![a.id, b.id],
                );
            }
        }
    }
    // Openings: in the wall, not colliding, below the wall's top.
    let mut by_host: HashMap<ElementId, Vec<&Opening>> = HashMap::new();
    for o in &c.openings {
        by_host.entry(o.host).or_default().push(o);
    }
    for (host, list) in &by_host {
        let Some(w) = c.wall(*host) else { continue };
        for o in list {
            let (a, b) = (o.offset - o.width / 2.0, o.offset + o.width / 2.0);
            let name = if o.mark.is_empty() {
                o.type_name.clone()
            } else {
                format!("{} {}", if o.door { "Door" } else { "Window" }, o.mark)
            };
            if a < -12.0 || b > w.len + 12.0 {
                f.add(
                    "opening-past-end",
                    Category::Constructability,
                    Severity::Major,
                    format!("{name} runs past the end of its wall"),
                    format!("It's {} wide in a {} wall.", inch(o.width), inch(w.len)),
                    "Move it into the wall or lengthen the wall.",
                    "Model integrity",
                    vec![o.id],
                );
            }
            let head = o.sill + o.height;
            if w.height > 0.0 && head > w.height + 12.0 {
                f.add(
                    "opening-head",
                    Category::Constructability,
                    Severity::Major,
                    format!("{name} is taller than its wall"),
                    format!(
                        "Its head is at {}, the wall is {} high.",
                        ft_in(head),
                        ft_in(w.height)
                    ),
                    "Lower the opening or raise the wall; leave room for a header.",
                    "Model integrity",
                    vec![o.id],
                );
            } else if w.height > 0.0 && w.exterior && head > w.height - 6.0 * MM_PER_IN {
                f.add(
                    "opening-header",
                    Category::Constructability,
                    Severity::Minor,
                    format!("{name} leaves no room for a header"),
                    format!(
                        "Its head is at {} in a {} wall.",
                        ft_in(head),
                        ft_in(w.height)
                    ),
                    "Allow for the header and top plate above the opening.",
                    "Framing",
                    vec![o.id],
                );
            }
        }
        for (i, p) in list.iter().enumerate() {
            for q in list.iter().skip(i + 1) {
                let gap = (p.offset - q.offset).abs() - (p.width + q.width) / 2.0;
                if gap < 0.0 {
                    f.add(
                        "opening-clash",
                        Category::Constructability,
                        Severity::Major,
                        "Two openings overlap in one wall",
                        format!(
                            "{} and {} overlap by {}.",
                            p.type_name,
                            q.type_name,
                            inch(-gap)
                        ),
                        "Move one of them.",
                        "Model integrity",
                        vec![p.id, q.id],
                    );
                } else if gap < 3.0 * MM_PER_IN {
                    f.add(
                        "opening-tight",
                        Category::Constructability,
                        Severity::Minor,
                        "Two openings are almost touching",
                        format!(
                            "{} and {} are {} apart: no room for framing between them.",
                            p.type_name,
                            q.type_name,
                            inch(gap)
                        ),
                        "Space them at least a stud's width apart, or mull them.",
                        "Framing",
                        vec![p.id, q.id],
                    );
                }
            }
        }
    }
    // Rooms.
    for r in &c.rooms {
        if r.ring.is_none() {
            f.add(
                "room-open",
                Category::Constructability,
                Severity::Major,
                format!(
                    "{} isn't enclosed",
                    if r.name.is_empty() { "A room" } else { &r.name }
                ),
                "Its area and finishes can't be computed.",
                "Close the walls or add room separation lines.",
                "Model integrity",
                vec![r.id],
            );
            continue;
        }
        if r.is(&["shaft", "chase", "open", "void", "attic", "crawl", "plenum"]) {
            continue;
        }
        let doors = c
            .openings
            .iter()
            .filter(|o| o.door && c.rooms_at(o).iter().any(|x| x.id == r.id))
            .count();
        let open_to = c.doc.of(Cat::RoomSeparator).next().is_some();
        // A stair arriving in the room is a way in.
        let ring = r.ring.as_deref().unwrap_or(&[]);
        let stair = c.doc.of(Cat::Stair).any(|e| match &e.data {
            ElementData::Stair {
                start,
                end,
                base_level,
                top_level,
                ..
            } => {
                (*top_level == r.level && point_in_ring(*end, ring))
                    || (*base_level == r.level && point_in_ring(*start, ring))
            }
            _ => false,
        });
        if doors == 0 && !open_to && !stair {
            f.add(
                "room-no-door",
                Category::Constructability,
                Severity::Major,
                format!("{} has no door", r.name),
                "An enclosed room with no door or opening can't be entered.",
                "Add a door or a cased opening.",
                "Planning",
                vec![r.id],
            );
        }
    }
    let mut seen = HashSet::new();
    for r in &c.rooms {
        let Some(ring) = &r.ring else { continue };
        for q in c
            .rooms
            .iter()
            .filter(|q| q.id != r.id && q.level == r.level)
        {
            let pt = c.doc.data(q.id).ok().and_then(|d| match d {
                ElementData::Room { point, .. } => Some(*point),
                _ => None,
            });
            if pt.is_some_and(|p| point_in_ring(p, ring))
                && seen.insert((r.id.min(q.id), r.id.max(q.id)))
            {
                f.add(
                    "room-dup",
                    Category::Constructability,
                    Severity::Major,
                    format!("{} and {} are in the same space", r.name, q.name),
                    "Two rooms in one enclosed area double-count its area.",
                    "Delete one, or separate them with a wall or room separation line.",
                    "Model integrity",
                    vec![r.id, q.id],
                );
            }
        }
    }
}

// ---- Consultant coordination ----

fn consultants(f: &mut F) {
    let c = f.c;
    let doc = c.doc;
    for e in doc.of(Cat::StructuralScheme) {
        let ElementData::StructuralScheme { layout, .. } = &e.data else {
            continue;
        };
        for fl in &layout.flags {
            f.add(
                "structural-flag",
                Category::Consultants,
                Severity::Major,
                format!(
                    "Structure: {}",
                    fl.message.split(':').next().unwrap_or(&fl.message)
                ),
                fl.message.clone(),
                "Coordinate with the structural engineer; the suggested structure is preliminary.",
                "Structural coordination (preliminary)",
                vec![e.id],
            );
        }
    }
    for e in doc.of(Cat::MepScheme) {
        let ElementData::MepScheme { settings, layout } = &e.data else {
            continue;
        };
        for fl in &layout.flags {
            f.add(
                "mep-flag",
                Category::Consultants,
                Severity::Minor,
                format!("{:?}: {}", settings.discipline, fl.title),
                fl.message.clone(),
                "Coordinate with the MEP engineer; the suggested systems are preliminary.",
                "MEP coordination (preliminary)",
                vec![e.id],
            );
        }
    }
    if c.milestone.cd() && !c.residential {
        let hired = |d: &str| {
            c.details
                .team
                .iter()
                .any(|m| m.discipline.to_lowercase().contains(d) && !m.contact.is_empty())
        };
        for (d, label) in [
            ("structural", "structural engineer"),
            ("mechanical", "mechanical engineer"),
            ("electrical", "electrical engineer"),
            ("plumbing", "plumbing engineer"),
        ] {
            if !hired(d) && !hired("mep") {
                f.add("consultant", Category::Consultants, Severity::Minor, format!("No {label} is listed"),
                    "Construction documents for this building type normally carry an engineer's drawings and seal.", "Add the consultant on Project Info > Consultants.", "Project team", vec![]);
            }
        }
    }
}
