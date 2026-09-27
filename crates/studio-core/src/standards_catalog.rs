//! The choices offered for each drawing-set standard (ADR-048): what the Standards tab's
//! pop-up lists, as a label (the value it sets) and a line on what it means. The order is
//! meaningful for the standards that drive drawing (spot elevations, north arrows,
//! graphic scales, key plans): each choice's index is a style (see `symbols`).

/// One way a standard can be set.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub label: &'static str,
    pub detail: &'static str,
}

type C = (&'static str, &'static str);

/// The choices for standard `item` in category `category` (empty when none are known).
pub fn choices(category: &str, item: &str) -> Vec<Choice> {
    table(category, item)
        .iter()
        .map(|&(label, detail)| Choice { label, detail })
        .collect()
}

/// The index of the choice whose label is `value` (ignoring case and spacing).
pub fn index_of(category: &str, item: &str, value: &str) -> Option<usize> {
    let norm = |s: &str| {
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let v = norm(value);
    if v.is_empty() {
        return None;
    }
    table(category, item).iter().position(|(l, _)| norm(l) == v)
}

fn table(category: &str, item: &str) -> &'static [C] {
    match (category, item) {
        // ---- Sheet Setup ----
        ("sheet", "Sheet Size") => &[
            ("ARCH D — 24\" × 36\"", "The usual construction set."),
            ("ARCH E1 — 30\" × 42\"", "Larger buildings; plans at 1/8\" without match lines."),
            ("ARCH E — 36\" × 48\"", "Large sites and campus plans."),
            ("ARCH C — 18\" × 24\"", "Houses and small projects."),
            ("ANSI B — 11\" × 17\"", "Half-size review sets."),
        ],
        ("sheet", "Title Block") => &[
            ("Rufplan vertical strip, right edge", "Project data, issues, key plan and sheet number stacked on the right."),
            ("Horizontal strip, bottom edge", "Project data across the bottom; more width for plans."),
            ("Vertical strip with logo band", "The right strip with the office logo and stamp at the top."),
            ("Bordered, stamp box upper right", "A thin border and a separate seal box."),
        ],
        ("sheet", "Sheet Numbering Format") => &[
            ("A-101 (NCS)", "Discipline, dash, sheet type and sequence."),
            ("A1.01 (dot)", "Sheet type before the dot, sequence after."),
            ("A101", "No separator."),
            ("A-1.01", "Dash and dot: discipline, type and sequence."),
        ],
        ("sheet", "Sheet Type Series") => &[
            ("0 General · 1 Plans · 2 Elev · 3 Sect · 4 Enlarged · 5 Details · 6 Schedules · 9 3D", "NCS sheet types, as Rufplan numbers them."),
            ("0 General · 1 Plans · 2 Elev · 3 Sect · 4 Large-scale · 5 Details · 6 Schedules · 7 User · 8 User · 9 3D", "The full NCS list."),
            ("1 Plans · 2 Elevations · 3 Sections · 5 Details · 8 Schedules", "A short office series for small sets."),
        ],
        ("sheet", "Discipline Order") => &[
            ("G, V, C, L, S, A, I, F, P, M, E, T", "NCS order."),
            ("G, C, L, S, A, I, M, P, E", "Residential and small commercial."),
            ("A, S, M, E, P", "Architect-led sets with consultants after."),
        ],
        ("sheet", "Sheet Naming") => &[
            ("LEVEL 2 FLOOR PLAN – AREA B", "Level first, all caps."),
            ("Level 2 Floor Plan – Area B", "Level first, mixed case."),
            ("FLOOR PLAN – LEVEL 2 – AREA B", "View type first."),
            ("L2 PLAN B", "Abbreviated."),
        ],
        ("sheet", "Cover Sheet Contents") => &[
            ("Project directory, code analysis, vicinity map, general notes", "The usual G-001."),
            ("Rendering, project data, sheet index", "A presentation cover."),
            ("Project directory, sheet index, code summary, general notes, vicinity and location maps", "Everything on one sheet (permit sets)."),
        ],
        ("sheet", "Drawing Index") => &[
            ("Auto-generated from Sheet Index schedule", "Always matches the set."),
            ("Index by discipline with issue matrix", "Which issue each sheet was in."),
            ("Manual list on G-001", "Typed by hand."),
        ],
        ("sheet", "Sheet Layout Grid") => &[
            ("NCS module grid — columns 1–n, rows A–F", "Views snap to modules; detail numbers by module."),
            ("Quadrants (4 per sheet)", "Four equal areas for details."),
            ("Free placement, aligned by guide grid", "Views placed freely, lined up on a guide."),
        ],
        ("sheet", "Key Plan & North Arrow") => &[
            ("Title block key plan with north arrow", "The title block shows the building's outline and a north arrow on every sheet."),
            ("Placed key plan, sheet area shaded", "Place Key Plan on sheets (Annotate); the area the sheet's plans show is shaded. The title block leaves it out."),
            ("North arrow on every plan; no key plan", "The title block shows only a north arrow; place north arrows in plans."),
            ("None", "No key plan or north arrow in the title block."),
        ],
        // ---- Symbols ----
        ("sym", "Section Markers") => &[
            ("Split circle head, 1/2\" dia, tail to cut extent", "Detail number over sheet number."),
            ("Circle head with triangle pointer", "The pointer shows the look direction."),
            ("Filled half-circle, number over sheet", "Half the head filled on the look side."),
        ],
        ("sym", "Exterior Elevation Markers") => &[
            ("Circle with solid pointer", "Revit's default elevation mark."),
            ("Square with triangle pointer", "A square body, pointer on the look side."),
            ("Circle with arrow, number in pointer", "The number sits in the arrow."),
        ],
        ("sym", "Interior Elevation Markers") => &[
            ("Single", "One view per mark."),
            ("2-way", "Two opposite views."),
            ("3-way", "Three views."),
            ("4-way square, numbered clockwise", "All four walls from one mark."),
        ],
        ("sym", "Detail Callouts") => &[
            ("Circle bubble, dashed boundary", "A dashed circle around the detail, bubble on a leader."),
            ("Rounded rectangle boundary, circle bubble", "A rounded box around the area."),
            ("Circle bubble on leader, no boundary", "Only the bubble points to the detail."),
        ],
        ("sym", "Enlarged Callouts") => &[
            ("Rectangle boundary, corner tag", "The tag sits on a corner of the box."),
            ("Rectangle boundary, circle bubble on leader", "Like a detail callout."),
            ("Dash-dot boundary, label at corner", "A lighter boundary."),
        ],
        ("sym", "View Titles") => &[
            ("Bubble + title + underline + scale", "Revit's view title."),
            ("Title + underline + scale", "No bubble; the number in the title."),
            ("Bubble + title, scale on second line", "Compact, two lines."),
        ],
        ("sym", "Grid Bubbles") => &[
            ("Numbers E–W, letters N–S, skip I & O", "The usual order."),
            ("Letters E–W, numbers N–S", "Reversed."),
            ("Numbers E–W, letters N–S, both ends", "Bubbles at both ends of every grid."),
        ],
        ("sym", "Level Markers") => &[
            ("Target head, relative 100'-0\"", "Level 1 reads 100'-0\"."),
            ("Target head, actual elevation", "The project's elevation (Level 1 at 0'-0\")."),
            ("Triangle head, actual elevation", "A filled triangle instead of the target."),
        ],
        ("sym", "Spot Elevations") => &[
            ("Triangle, project elevation", "A solid triangle on the point; the height from Level 1 = 0'-0\"."),
            ("Target, relative to 100'-0\"", "A quartered target; Level 1 = 100'-0\", like relative level markers."),
            ("Cross, survey elevation", "A cross on the point with the site's surveyed height in decimal feet, for site plans."),
            ("Text only, EL. prefix", "No symbol: EL. and the height at the leader."),
        ],
        ("sym", "North Arrow") => &[
            ("Project north, true north offset shown", "A solid arrow to project north and a thin TN line to true north (from the Site)."),
            ("Circle with solid pointer", "A circle, a solid pointer and N."),
            ("Half-filled arrow, N above", "An arrow half dark, half light; N above the tip."),
            ("Compass rose", "Four points with N, E, S and W."),
            ("True north only", "Points to true north from the Site."),
        ],
        ("sym", "Matchlines") => &[
            ("Heavy dash-dot, \"MATCHLINE – SEE A-102\"", "The sheet to continue on, on each side."),
            ("Heavy phantom line, far side shaded", "The area on the other sheet in a light tone."),
            ("Heavy solid line, note both sides", "A solid line and a note on each side."),
        ],
        ("sym", "Break Lines") => &[
            ("Single zigzag, thin", "One break in a straight line."),
            ("Double zigzag", "Two breaks, for long runs."),
            ("Curved (round members)", "An S-curve for pipes and round members."),
        ],
        ("sym", "Revision Clouds & Deltas") => &[
            ("Triangle delta with issue number", "The issue's number in a triangle by the cloud."),
            ("Triangle delta with revision letter", "Letters instead of numbers."),
            ("Hexagon with number", "A hexagon tag."),
        ],
        ("sym", "Graphic Scale") => &[
            ("Alternating bar, labeled in feet", "Black and white segments 0 · 4 · 8 · 16'; lengths follow the view's scale."),
            ("Line with ticks", "A single line with tick marks and feet labels."),
            ("Double checkered bar", "Two rows of alternating segments."),
            ("Bar with scale text", "The alternating bar with the scale written below (1/8\" = 1'-0\")."),
        ],
        // ---- Tags ----
        ("tags", "Room") => &[
            ("Name / Number / Area, centered", "Three lines."),
            ("Name / Number", "No area."),
            ("Number in box, name above", "The number boxed."),
            ("Name / Number / Finish code", "Finishes in the tag."),
        ],
        ("tags", "Door") => &[
            ("Circle, room number + suffix", "101A."),
            ("Hexagon, sequential number", "1, 2, 3."),
            ("Rectangle, type mark", "The door type."),
        ],
        ("tags", "Window") => &[
            ("Hexagon, alpha type mark", "A, B, C."),
            ("Diamond, alpha type mark", "A, B, C in a diamond."),
            ("Circle, number", "1, 2, 3."),
        ],
        ("tags", "Wall Type") => &[
            ("Diamond, alphanumeric", "4A."),
            ("Hexagon, number", "1, 2, 3."),
            ("Rectangle, assembly code", "W-1."),
        ],
        ("tags", "Ceiling") => &[
            ("Rectangle — material / height AFF", "ACT-1 / 9'-0\"."),
            ("Ellipse — type / height", "In an ellipse."),
            ("Text only, height AFF", "9'-0\" AFF."),
        ],
        ("tags", "Finish") => &[
            ("Separate finish tag, code per surface", "Floor, base, walls and ceiling codes."),
            ("Room finish code in room tag", "Houses and simple sets."),
            ("Finish code + period (e.g. P-2 1910)", "Historic work."),
            ("Scheduled only (no tag)", "Finishes in the schedule only."),
        ],
        ("tags", "Equipment") => &[
            ("Rounded rectangle, EQ-##", "EQ-01."),
            ("Hexagon, equipment number", "In a hexagon."),
            ("Circle, number", "1, 2, 3."),
        ],
        ("tags", "Casework") => &[
            ("Hexagon, cabinet type", "Base, wall and tall cabinet types."),
            ("Rectangle, CW-##", "CW-01."),
            ("Diamond, type letter", "A, B, C."),
        ],
        ("tags", "Keynote") => &[
            ("Square, CSI code", "04 20 00.A1."),
            ("Circle, sheet-specific number", "1, 2, 3 per sheet."),
            ("Hexagon, keynote number", "In a hexagon."),
        ],
        // ---- Text ----
        ("text", "Font") => &[
            ("Barlow Semi Condensed", "Rufplan's drawing font."),
            ("Arial Narrow", "A common narrow sans."),
            ("Helvetica", "Neutral, wider."),
            ("RomanS (CAD)", "Single-stroke CAD lettering."),
        ],
        ("text", "Heights") => &[
            ("Notes 3/32\" · Room names 1/8\" · View titles 3/16\" · Sheet title 1/4\"", "The usual heights."),
            ("Notes 1/8\" · Room names 5/32\" · View titles 1/4\" · Sheet title 3/8\"", "Larger, for half-size prints."),
            ("Notes 3/32\" everywhere, titles 3/16\"", "Two heights only."),
        ],
        ("text", "Case") => &[
            ("ALL CAPS", "Every note and title in capitals."),
            ("Mixed case", "Sentence case notes."),
            ("Titles caps, notes mixed case", "Capitals for titles only."),
        ],
        ("text", "Title Hierarchy") => &[
            ("Sheet title › View title › Room name › Note", "Four sizes."),
            ("View title › Room name › Note (sheet title in title block only)", "Three sizes on the drawing."),
        ],
        ("text", "Note Formats") => &[
            ("General notes lettered, sheet notes numbered, keynotes by CSI", "The usual set."),
            ("All notes numbered per sheet", "One numbered list per sheet."),
            ("Keynotes only, no general notes on sheets", "Notes in the keynote legend."),
        ],
        ("text", "Abbreviations") => &[
            ("Office list on G-001; no undefined abbreviations", "Only listed abbreviations."),
            ("Industry list (no office list)", "Standard abbreviations."),
            ("No abbreviations", "Everything spelled out."),
        ],
        // ---- Linework ----
        ("line", "Pen Weights") => &[
            ("Pens 1–16, mapped by scale", "Widths change with the view's scale, as in Revit."),
            ("Pens 1–6, fixed widths", "The same width at every scale."),
            ("Revit defaults", "Revit's out-of-the-box line weights."),
        ],
        ("line", "Line Types") => &[
            ("Hidden, overhead, centerline, property, setback, demo, NIC", "The office set."),
            ("Hidden, centerline, property only", "A minimal set."),
            ("Full ANSI set", "Every ANSI pattern."),
        ],
        ("line", "Line Hierarchy") => &[
            ("Cut 5 · Profile 4 · Projection 2 · Beyond 1 · Overhead dashed 1", "Five weights."),
            ("Cut 6 · Profile 4 · Projection 2 · Beyond 1", "Heavier cuts."),
            ("Cut 4 · Projection 1", "Two weights."),
        ],
        ("line", "Halftone & Underlays") => &[
            ("50% halftone for links and underlays", "Linked models and underlays at half tone."),
            ("30% halftone", "Lighter."),
            ("Gray lines, no halftone", "A gray pen instead."),
        ],
        // ---- Material graphics ----
        ("mat", "Cut Hatches & Poché") => &[
            ("Concrete, CMU, brick, stud, insulation, earth, steel, glass", "A pattern per material."),
            ("Solid poché for all cut walls", "Walls cut solid."),
            ("ANSI 31 diagonal for all", "One diagonal hatch."),
        ],
        ("mat", "Surface / Elevation Patterns") => &[
            ("Brick coursing, siding, panels, roofing", "Material patterns in elevations."),
            ("Patterns in enlarged elevations only", "Outlines at small scales."),
            ("Outline only, no surface patterns", "Clean elevations."),
        ],
        ("mat", "Scale-Dependent Fill Rules") => &[
            ("Solid poché ≤ 1/8\", full pattern ≥ 1/4\"", "Small scales poché, large scales pattern."),
            ("Full pattern at every scale", "Always the pattern."),
            ("Solid poché at every scale", "Always solid."),
        ],
        ("mat", "Rated Wall Graphics") => &[
            ("Dash-dot line along rated walls, legend by hour", "On life-safety and floor plans."),
            ("Colored tone by rating (life-safety plans only)", "A tone per hour."),
            ("Rating in wall tag only", "No graphics."),
        ],
        // ---- Phasing ----
        ("phase", "Existing") => &[
            ("Halftone gray, thin", "Existing recedes."),
            ("Solid, full weight — historic fabric is the drawing", "Historic work: existing leads."),
            ("Screened 50%", "Half tone, normal weight."),
        ],
        ("phase", "Demo") => &[
            ("Dashed, keynoted", "Dashed with a keynote."),
            ("Dashed; historic fabric removed only as keynoted", "Historic work."),
            ("Dashed with X hatch", "Crossed out."),
        ],
        ("phase", "New") => &[
            ("Solid black, full weight", "New leads."),
            ("Solid with poché fill", "New walls filled."),
        ],
        ("phase", "Future") => &[
            ("Dash-dot, light", "Light dash-dot."),
            ("Dotted, halftone", "Dotted and gray."),
            ("Not shown", "Future work left off."),
        ],
        ("phase", "NIC") => &[
            ("Hidden line + \"NIC\" note", "Dashed with NIC."),
            ("Halftone + NIC", "Gray with NIC."),
            ("Dash-dot + NIC", "Dash-dot with NIC."),
        ],
        ("phase", "Historic Fabric") => &[
            ("Period-coded hatch, keyed to historic structure report", "A hatch per period."),
            ("Tone per period", "A gray tone per period."),
            ("Not distinguished", "Shown as existing."),
        ],
        ("phase", "Treatment Graphics") => &[
            ("Repair · Replace in kind · Salvage · Reinstall", "Four treatments."),
            ("Preserve · Repair · Replace in kind · Salvage · Reinstall · Reconstruct (per SOI Standards)", "The Secretary of the Interior's treatments."),
            ("Keynotes only", "Treatments in keynotes."),
        ],
        // ---- Dimensioning ----
        ("dim", "Units") => &[
            ("Feet & fractional inches", "12'-6 1/2\"."),
            ("Inches only", "150 1/2\"."),
            ("Millimeters", "3823."),
            ("Feet & decimal feet (civil)", "12.54'."),
        ],
        ("dim", "Precision") => &[
            ("1/16\"", "Details."),
            ("1/8\"", "The usual."),
            ("1/4\"", "Plans at small scales."),
            ("1/2\"", "Site and schematic."),
        ],
        ("dim", "Reference Points") => &[
            ("New: face of stud · Existing: face of finish · Masonry: face", "The usual."),
            ("Existing: as-found face of finish · New: face of stud", "Historic work."),
            ("Centerline of stud (framing)", "For framers."),
            ("Face of finish throughout", "Interiors."),
        ],
        ("dim", "String Order") => &[
            ("Overall → grids → openings", "From the building out."),
            ("Overall → openings → grids", "Openings nearest the overall."),
            ("Grids → overall → openings", "Grids first."),
        ],
        ("dim", "Tick Style") => &[
            ("Diagonal tick", "Architectural ticks."),
            ("Arrow", "Filled arrows."),
            ("Dot", "Filled dots."),
        ],
        // ---- Numbering ----
        ("num", "Rooms") => &[
            ("Level + sequence, clockwise from entry (101)", "101, 102 … clockwise."),
            ("Level + sequence, left to right (101)", "101, 102 … across the plan."),
            ("Sequential per level (1, 2, 3)", "Plain numbers."),
        ],
        ("num", "Doors") => &[
            ("Room number + suffix (101A)", "Doors take their room's number."),
            ("Sequential (1, 2, 3)", "In order placed."),
            ("Level + sequence (D101)", "D and the level."),
        ],
        ("num", "Windows") => &[
            ("Alpha type (A, B, C)", "By type."),
            ("Sequential number", "Every window numbered."),
            ("Type + number (A1, A2)", "Type and count."),
        ],
        ("num", "Wall & Assembly Types") => &[
            ("Walls 4A · Floors F-1 · Roofs R-1 · Ceilings C-1", "Letter by assembly."),
            ("Numbered (1, 2, 3)", "One series."),
            ("CSI-based (W-0921)", "From the spec section."),
        ],
        ("num", "Grids") => &[
            ("Numbers E–W, letters N–S", "The usual."),
            ("Letters E–W, numbers N–S", "Reversed."),
        ],
        ("num", "Levels") => &[
            ("LEVEL 1, LEVEL 2, ROOF", "Revit's naming."),
            ("FIRST FLOOR, SECOND FLOOR, ROOF", "Spelled out."),
            ("L1, L2, RF", "Short."),
            ("T.O. SLAB, T.O. PLATE", "By what the level marks."),
        ],
        ("num", "Equipment") => &[
            ("EQ-## by room", "EQ and the room number."),
            ("Sequential EQ-1, EQ-2", "In order."),
            ("By discipline (P-1, M-1)", "Discipline letter first."),
        ],
        // ---- Schedules & legends ----
        ("sched", "Door Schedule") => &[
            ("Mark, size, type, material, frame, hardware set, rating", "The full schedule."),
            ("Mark, size, type, material, finish, hardware set", "Without ratings."),
            ("Mark, size, type only (hardware by others)", "Short."),
        ],
        ("sched", "Window Schedule") => &[
            ("Type, size, operation, glazing, head/sill", "The full schedule."),
            ("Type, size, manufacturer, model", "By product."),
        ],
        ("sched", "Hardware") => &[
            ("Hardware set per door, in door schedule", "A set number in the door schedule."),
            ("Separate hardware schedule by set", "Its own schedule."),
            ("By hardware consultant (spec)", "In the specifications."),
        ],
        ("sched", "Finish Schedule") => &[
            ("Room, floor, base, walls, ceiling, notes", "By room."),
            ("Room, finish code per surface, legend", "Codes and a legend."),
            ("Finish plan instead of schedule", "Finishes drawn."),
        ],
        ("sched", "Equipment Schedule") => &[
            ("Mark, description, manufacturer, model, supplied/installed by", "The full schedule."),
            ("In specs only", "No schedule."),
        ],
        ("sched", "Wall Type Legend") => &[
            ("Enlarged assembly sections at 1-1/2\"", "A section per type."),
            ("Assembly sections at 3\" = 1'-0\"", "Larger."),
            ("Table only (no sections)", "Layers listed."),
        ],
        ("sched", "Symbol Legend") => &[
            ("On G-002", "With the general sheets."),
            ("On G-001 cover", "On the cover."),
            ("On each sheet", "Where used."),
        ],
        ("sched", "Material Legend") => &[
            ("On G-002", "With the general sheets."),
            ("On G-001 cover", "On the cover."),
            ("On each sheet", "Where used."),
        ],
        // ---- Keynotes & specs ----
        ("key", "Keynote System") => &[
            ("CSI MasterFormat (04 20 00.A1)", "Keyed to the spec."),
            ("Sheet-specific numbers", "1, 2, 3 per sheet."),
            ("Uniformat", "By building system."),
        ],
        ("key", "Spec Section References") => &[
            ("Section number only, no titles", "04 20 00."),
            ("Section number and title", "04 20 00 Unit Masonry."),
            ("No spec references", "Notes only."),
        ],
        // ---- Scales & views ----
        ("scale", "Standard Scales") => &[
            ("Site 1\"=20' · Plans 1/8\" · Enlarged 1/4\" · Int elev 1/4\" · Wall sect 3/4\" · Details 1-1/2\", 3\"", "Commercial."),
            ("Site 1\"=10' · Plans 1/4\" · Int elev 1/2\" · Wall sect 3/4\" · Details 1-1/2\", 3\"", "Residential."),
            ("Site 1:200 · Plans 1:100 · Enlarged 1:50 · Details 1:10, 1:5", "Metric."),
        ],
        ("scale", "Plan Cut Plane") => &[
            ("4'-0\" AFF, overhead dashed", "Revit's cut height, overhead shown dashed."),
            ("4'-0\" AFF, no overhead", "Only what's below."),
            ("5'-0\" AFF (high windows)", "Cuts high windows."),
        ],
        ("scale", "RCP Conventions") => &[
            ("Reflected, fixtures + soffits + heights AFF", "The full RCP."),
            ("Reflected, fixtures only", "Short."),
        ],
        ("scale", "Plan Orientation") => &[
            ("Project north up", "Plans square to the building."),
            ("True north up", "Plans turned to true north."),
        ],
        // ---- BIM ----
        ("bim", "Project Template") => &[
            ("Rufplan Residential 2026", "Houses."),
            ("Rufplan Commercial 2026", "Commercial."),
            ("Rufplan Preservation 2026", "Historic work."),
        ],
        ("bim", "View Templates") => &[
            ("One per view type and scale", "Plans 1/8\", plans 1/4\" …"),
            ("One per view type", "Fewer templates."),
            ("None", "Views set by hand."),
        ],
        ("bim", "View Naming") => &[
            ("LEVEL – VIEW TYPE – AREA", "LEVEL 2 – FLOOR PLAN – B."),
            ("VIEW TYPE – LEVEL", "FLOOR PLAN – LEVEL 2."),
            ("Sheet number – view title", "A-101 – FLOOR PLAN."),
        ],
        ("bim", "Browser Organization") => &[
            ("Discipline › View type › Phase", "Grouped three deep."),
            ("View type › Level", "Revit's default."),
            ("Sheet › View", "By sheet."),
        ],
        ("bim", "Worksets") => &[
            ("Not used (single user)", "One person."),
            ("By discipline (Arch, Interiors, Site, Links)", "The usual split."),
            ("By building area", "One per wing or floor."),
        ],
        ("bim", "Phases") => &[
            ("Existing · Demo · New Construction", "Additions and renovations."),
            ("Historic · Existing · Demo · Rehabilitation", "Historic work."),
            ("New Construction only", "New buildings."),
        ],
        ("bim", "Coordinates") => &[
            ("Shared coordinates from survey point", "From the survey."),
            ("State plane from site", "From the Site's parcel."),
            ("Project internal origin", "No shared coordinates."),
        ],
        // ---- Issuance ----
        ("issue", "Issue Names") => &[
            ("SD, DD, CD 50/90/100%, Permit, Bid, Addendum, ASI, CCD, Record", "The full list."),
            ("HSR, SD, DD, SHPO Review, CD 100%, Permit, Bid, Record", "Historic work."),
            ("SD, DD, CD, Permit, Construction", "Short."),
        ],
        ("issue", "Revision Rules") => &[
            ("Delta per issue; clouds cleared on next issue", "Only the latest changes clouded."),
            ("Clouds kept, deltas cumulative", "Every change stays clouded."),
            ("Revisions by sheet (one per issue)", "A sheet's revisions only."),
        ],
        ("issue", "Date Format") => &[
            ("YYYY-MM-DD", "2026-09-27."),
            ("MM/DD/YYYY", "09/27/2026."),
            ("DD MMM YYYY", "27 SEP 2026."),
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standards::{library, LIBRARIES};

    #[test]
    fn every_standard_has_choices_including_its_default() {
        let s = library(LIBRARIES[0]).unwrap();
        for c in &s.categories {
            for it in &c.items {
                let ch = choices(&c.id, &it.name);
                assert!(ch.len() >= 2, "{} / {} has choices", c.id, it.name);
                if !it.value.is_empty() {
                    assert!(
                        index_of(&c.id, &it.name, &it.value).is_some(),
                        "{} / {}: the default {:?} is a choice",
                        c.id,
                        it.name,
                        it.value
                    );
                }
            }
        }
    }

    #[test]
    fn library_values_are_choices_too() {
        for name in LIBRARIES {
            let s = library(name).unwrap();
            for c in &s.categories {
                for it in c.items.iter().filter(|i| !i.value.is_empty()) {
                    assert!(
                        index_of(&c.id, &it.name, &it.value).is_some(),
                        "{name}: {} / {} = {:?}",
                        c.id,
                        it.name,
                        it.value
                    );
                }
            }
        }
    }

    #[test]
    fn index_ignores_case_and_spacing() {
        assert_eq!(index_of("sym", "North Arrow", "compass  ROSE"), Some(3));
        assert_eq!(index_of("sym", "North Arrow", ""), None);
        assert_eq!(index_of("sym", "North Arrow", "a custom arrow"), None);
    }
}
