//! General Notes (ADR-103): the numbered notes a US drawing set carries, preset by the
//! building type and by the drawing they go on (the cover, site plan, floor plans, ceiling
//! plans, roof plan, elevations, sections, details, enlarged plans, schedules). Residential
//! types cite the IRC, the rest the IBC with accessibility and fire separations; the code
//! edition comes from Project Info when it's set. Placed as one text note, edited freely
//! afterwards (they're the architect's notes, not the model's).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::ElementId;
use studio_geom::Pt;

/// The building types notes are tailored to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NotesBuilding {
    SingleFamily,
    DuplexTownhouse,
    Multifamily,
    MixedUse,
    Hotel,
    Commercial,
}

impl NotesBuilding {
    pub const ALL: [NotesBuilding; 6] = [
        NotesBuilding::SingleFamily,
        NotesBuilding::DuplexTownhouse,
        NotesBuilding::Multifamily,
        NotesBuilding::MixedUse,
        NotesBuilding::Hotel,
        NotesBuilding::Commercial,
    ];
    pub fn label(self) -> &'static str {
        match self {
            NotesBuilding::SingleFamily => "Single-Family House",
            NotesBuilding::DuplexTownhouse => "Duplex / Townhouses",
            NotesBuilding::Multifamily => "Multifamily Apartments",
            NotesBuilding::MixedUse => "Mixed-Use (Retail + Residential)",
            NotesBuilding::Hotel => "Hotel",
            NotesBuilding::Commercial => "Office / Commercial",
        }
    }
    /// Built under the residential code (IRC) rather than the IBC.
    pub fn irc(self) -> bool {
        matches!(
            self,
            NotesBuilding::SingleFamily | NotesBuilding::DuplexTownhouse
        )
    }
    /// The type Project Info's project type names, if any.
    pub fn from_project_type(t: &str) -> Option<NotesBuilding> {
        let t = t.to_lowercase();
        Some(if t.contains("single") || t.contains("house") {
            NotesBuilding::SingleFamily
        } else if t.contains("duplex") || t.contains("townho") {
            NotesBuilding::DuplexTownhouse
        } else if t.contains("mixed") {
            NotesBuilding::MixedUse
        } else if t.contains("hotel") || t.contains("hospitality") {
            NotesBuilding::Hotel
        } else if t.contains("multi") || t.contains("apartment") {
            NotesBuilding::Multifamily
        } else if t.contains("office") || t.contains("commercial") || t.contains("retail") {
            NotesBuilding::Commercial
        } else {
            return None;
        })
    }
}

/// The drawings notes are written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NotesDrawing {
    General,
    SitePlan,
    FloorPlan,
    CeilingPlan,
    RoofPlan,
    Elevations,
    Sections,
    Details,
    EnlargedPlans,
    Schedules,
}

impl NotesDrawing {
    pub const ALL: [NotesDrawing; 10] = [
        NotesDrawing::General,
        NotesDrawing::SitePlan,
        NotesDrawing::FloorPlan,
        NotesDrawing::CeilingPlan,
        NotesDrawing::RoofPlan,
        NotesDrawing::Elevations,
        NotesDrawing::Sections,
        NotesDrawing::Details,
        NotesDrawing::EnlargedPlans,
        NotesDrawing::Schedules,
    ];
    pub fn label(self) -> &'static str {
        match self {
            NotesDrawing::General => "General (Cover Sheet)",
            NotesDrawing::SitePlan => "Site Plan",
            NotesDrawing::FloorPlan => "Floor Plans",
            NotesDrawing::CeilingPlan => "Reflected Ceiling Plans",
            NotesDrawing::RoofPlan => "Roof Plan",
            NotesDrawing::Elevations => "Exterior Elevations",
            NotesDrawing::Sections => "Building Sections",
            NotesDrawing::Details => "Wall Sections & Details",
            NotesDrawing::EnlargedPlans => "Enlarged Plans & Interior Elevations",
            NotesDrawing::Schedules => "Door & Window Schedules",
        }
    }
    /// The heading the notes are placed under.
    pub fn heading(self) -> String {
        match self {
            NotesDrawing::General => "GENERAL NOTES".into(),
            d => format!("{} GENERAL NOTES", d.label().to_uppercase()),
        }
    }
}

/// Who a note is for.
#[derive(Clone, Copy)]
enum For {
    All,
    /// IRC buildings (houses, duplexes, townhouses).
    Irc,
    /// IBC buildings.
    Ibc,
    /// Buildings with dwelling units over one another or side by side (fire separations).
    Attached,
    /// Public and employee spaces (accessibility under the IBC and ADA).
    Public,
    Hotel,
}

impl For {
    fn applies(self, b: NotesBuilding) -> bool {
        use NotesBuilding as B;
        match self {
            For::All => true,
            For::Irc => b.irc(),
            For::Ibc => !b.irc(),
            For::Attached => !matches!(b, B::SingleFamily),
            For::Public => matches!(b, B::Multifamily | B::MixedUse | B::Hotel | B::Commercial),
            For::Hotel => b == B::Hotel,
        }
    }
}

/// `{code}` is the building code edition ("2021 IRC").
const NOTES: &[(NotesDrawing, For, &str)] = &[
    // ---------------------------------------------------------------- general
    (NotesDrawing::General, For::All, "All work shall comply with the {code} as adopted and amended by the authority having jurisdiction, and with all applicable local, state and federal codes and ordinances."),
    (NotesDrawing::General, For::All, "The contractor shall visit the site and verify all existing conditions and dimensions before starting work, and notify the architect of any discrepancy before proceeding."),
    (NotesDrawing::General, For::All, "Do not scale the drawings. Written dimensions govern; large-scale details govern over small-scale drawings."),
    (NotesDrawing::General, For::All, "The drawings, specifications and general conditions are complementary: what is called for by one is as binding as if called for by all. Bring conflicts to the architect's attention for clarification."),
    (NotesDrawing::General, For::All, "The contractor shall obtain and pay for all permits, inspections and approvals required for the work, and keep the approved permit set on site."),
    (NotesDrawing::General, For::All, "Coordinate the work of all trades. Structural, mechanical, plumbing and electrical drawings are part of the contract documents; verify locations of openings, sleeves and equipment before construction."),
    (NotesDrawing::General, For::All, "Provide blocking in walls for all wall-mounted items: cabinets, shelving, grab bars, accessories, equipment and fixtures."),
    (NotesDrawing::General, For::All, "Install all products per the manufacturer's written instructions and recommendations."),
    (NotesDrawing::General, For::Irc, "Provide smoke alarms in each sleeping room, outside each sleeping area and on each story, and carbon monoxide alarms outside sleeping areas, hard-wired, interconnected and with battery backup."),
    (NotesDrawing::General, For::Ibc, "Provide fire extinguishers, fire alarm and sprinkler systems as required by the code and the fire marshal; sprinkler and alarm systems are deferred submittals by licensed designers."),
    (NotesDrawing::General, For::Public, "All public and common-use spaces, routes and elements shall be accessible per ICC A117.1 and the ADA Standards for Accessible Design."),
    (NotesDrawing::General, For::Attached, "Maintain the fire-resistance ratings of all rated assemblies; seal penetrations with listed firestop systems."),
    // ---------------------------------------------------------------- site plan
    (NotesDrawing::SitePlan, For::All, "Survey information is from the survey by others. The contractor shall verify property lines, setbacks, easements and utilities before construction; locate the building by a licensed surveyor."),
    (NotesDrawing::SitePlan, For::All, "Call 811 to locate all underground utilities at least two working days before digging."),
    (NotesDrawing::SitePlan, For::All, "Grade the site to slope away from the building at least 6\" in the first 10'-0\" (5%); impervious surfaces at least 2% away."),
    (NotesDrawing::SitePlan, For::All, "Install and maintain erosion and sediment controls before and throughout construction, per the local stormwater requirements."),
    (NotesDrawing::SitePlan, For::All, "Protect existing trees to remain with fencing at the drip line; do not store materials or park within it."),
    (NotesDrawing::SitePlan, For::All, "Coordinate utility connections (water, sewer, gas, electric, telecom) with the utility providers and the civil drawings."),
    (NotesDrawing::SitePlan, For::Public, "Provide an accessible route from public sidewalks, accessible parking and passenger loading zones to the accessible entrances; maximum running slope 1:20, cross slope 1:48."),
    // ---------------------------------------------------------------- floor plans
    (NotesDrawing::FloorPlan, For::All, "Dimensions are to the face of stud, face of concrete or masonry, or centerline of column, unless noted otherwise."),
    (NotesDrawing::FloorPlan, For::All, "Interior partitions are type as noted; where not noted, 2x4 studs at 16\" o.c. with 1/2\" gypsum board each side."),
    (NotesDrawing::FloorPlan, For::All, "Locate door openings 4\" from the adjacent wall at the hinge side unless dimensioned otherwise."),
    (NotesDrawing::FloorPlan, For::All, "Provide moisture-resistant backer board at all tub, shower and wet locations."),
    (NotesDrawing::FloorPlan, For::All, "Refer to the door and window schedules for sizes, types, hardware and glazing; provide safety glazing in hazardous locations."),
    (NotesDrawing::FloorPlan, For::Irc, "Each sleeping room shall have an emergency escape and rescue opening: 5.7 sf net clear (5.0 sf at grade floor), 24\" min. clear height, 20\" min. clear width, sill 44\" max. above the floor."),
    (NotesDrawing::FloorPlan, For::Irc, "Separate the garage from the dwelling with 1/2\" gypsum board on the garage side (5/8\" Type X at habitable rooms above), and a solid-core or 20-minute door, self-closing, between them."),
    (NotesDrawing::FloorPlan, For::Attached, "Separate dwelling units with the fire-resistance-rated walls and floor/ceiling assemblies indicated, continuous to the roof deck or to the rated floor above."),
    (NotesDrawing::FloorPlan, For::Ibc, "Exit signs, emergency lighting, travel distances and exit widths per the life safety plans; doors in means of egress shall open without keys or special knowledge."),
    (NotesDrawing::FloorPlan, For::Public, "Provide accessible units and accessible routes as indicated; 32\" min. clear door openings, 18\" min. pull-side and 12\" push-side maneuvering clearances."),
    (NotesDrawing::FloorPlan, For::Hotel, "Provide the required number of accessible guest rooms, with and without roll-in showers, and communication-feature rooms, dispersed among room types."),
    // ---------------------------------------------------------------- ceiling plans
    (NotesDrawing::CeilingPlan, For::All, "Ceiling heights are measured from the finish floor to the finish ceiling."),
    (NotesDrawing::CeilingPlan, For::All, "Center light fixtures, diffusers and devices in ceiling tiles or align as shown; coordinate with the mechanical and electrical drawings before rough-in."),
    (NotesDrawing::CeilingPlan, For::All, "Provide access panels where required for valves, dampers, junction boxes and equipment above inaccessible ceilings."),
    (NotesDrawing::CeilingPlan, For::All, "Gypsum board ceilings: 5/8\" Type X where fire-rated, moisture-resistant at wet areas; paint finish as scheduled."),
    (NotesDrawing::CeilingPlan, For::Ibc, "Maintain the rating of rated ceiling assemblies; fixtures and diffusers in rated ceilings shall be listed or protected."),
    // ---------------------------------------------------------------- roof plan
    (NotesDrawing::RoofPlan, For::All, "Slope roofs to drains, gutters and scuppers as shown; minimum 1/4\" per foot on low-slope roofs."),
    (NotesDrawing::RoofPlan, For::All, "Install roofing, flashing and underlayment per the manufacturer's requirements for the specified warranty; provide cricket at the high side of curbs and penetrations over 30\" wide."),
    (NotesDrawing::RoofPlan, For::All, "Flash and counterflash all penetrations, curbs, walls and edges; coordinate penetration locations with mechanical and plumbing."),
    (NotesDrawing::RoofPlan, For::All, "Provide attic and roof ventilation as required, or an unvented assembly with insulation per the energy code."),
    (NotesDrawing::RoofPlan, For::Ibc, "Provide roof access, guards at roof equipment within 10'-0\" of the roof edge, and overflow drainage per the plumbing code."),
    // ---------------------------------------------------------------- elevations
    (NotesDrawing::Elevations, For::All, "Elevation heights are to the top of the subfloor or slab unless noted otherwise."),
    (NotesDrawing::Elevations, For::All, "Exterior finishes as indicated; see the specifications for products, colors and warranties. Submit samples for approval."),
    (NotesDrawing::Elevations, For::All, "Provide a continuous weather-resistive barrier, and flash all window and door openings, roof-wall intersections and penetrations per the manufacturer and the code."),
    (NotesDrawing::Elevations, For::All, "Provide control and expansion joints in stucco and masonry as indicated or per the manufacturer; align with openings where possible."),
    (NotesDrawing::Elevations, For::All, "Maintain a minimum 6\" clearance between wood siding and finish grade, 2\" to roofing and paved surfaces."),
    (NotesDrawing::Elevations, For::Attached, "Exterior walls within the fire separation distance shall be rated and openings limited as required by the code."),
    // ---------------------------------------------------------------- sections
    (NotesDrawing::Sections, For::All, "Refer to the structural drawings for framing, foundations, connections and member sizes."),
    (NotesDrawing::Sections, For::All, "Insulation shall meet the energy code: values as indicated in the wall, floor and roof assemblies."),
    (NotesDrawing::Sections, For::All, "Provide fireblocking at concealed spaces, floor and ceiling levels, soffits and penetrations, and draftstopping as required."),
    (NotesDrawing::Sections, For::Irc, "Stairs: 7 3/4\" max. riser, 10\" min. tread, 6'-8\" min. headroom; handrails 34\" to 38\" above the nosings; guards 36\" min. where the drop exceeds 30\"."),
    (NotesDrawing::Sections, For::Ibc, "Stairs: 7\" max. riser, 11\" min. tread, 80\" min. headroom; handrails 34\" to 38\" on both sides; guards 42\" min. where the drop exceeds 30\"."),
    // ---------------------------------------------------------------- details
    (NotesDrawing::Details, For::All, "Details are typical; apply them at similar conditions unless noted otherwise."),
    (NotesDrawing::Details, For::All, "Lap the weather-resistive barrier shingle-fashion, flashing over and under it as shown; seal all laps and penetrations."),
    (NotesDrawing::Details, For::All, "Use corrosion-resistant fasteners compatible with the materials fastened; stainless or hot-dip galvanized at exterior and treated wood."),
    (NotesDrawing::Details, For::All, "Provide sealant with backer rod at all joints between dissimilar materials and at all exterior openings."),
    // ---------------------------------------------------------------- enlarged plans
    (NotesDrawing::EnlargedPlans, For::All, "Coordinate cabinet, fixture and appliance sizes with the manufacturers before rough-in; verify appliance clearances and utility locations."),
    (NotesDrawing::EnlargedPlans, For::All, "Countertops at 36\" above the finish floor unless noted; vanity tops at 34\"."),
    (NotesDrawing::EnlargedPlans, For::All, "Provide waterproofing membrane at all showers and wet floors, turned up the walls as required."),
    (NotesDrawing::EnlargedPlans, For::Public, "Accessible toilet rooms: 60\" turning space, grab bars, 17\" to 19\" water closet seat height, 34\" max. lavatory rim height with knee and toe clearance."),
    // ---------------------------------------------------------------- schedules
    (NotesDrawing::Schedules, For::All, "Door and window sizes are nominal; the contractor shall verify rough openings with the manufacturer before framing."),
    (NotesDrawing::Schedules, For::All, "Provide safety glazing in doors, sidelites, glazing within 24\" of doors, large panes near floors, and at tubs and showers, as required by the code."),
    (NotesDrawing::Schedules, For::All, "Windows and doors shall meet the energy code U-factor and SHGC; provide NFRC labels."),
    (NotesDrawing::Schedules, For::Ibc, "Fire-rated doors and frames shall be labeled, self-closing and latching, with hardware listed for the rating."),
    (NotesDrawing::Schedules, For::Public, "Door hardware on accessible routes shall be operable with one hand without tight grasping or twisting, 34\" to 48\" above the floor."),
];

/// The preset notes for a building type and drawing, the code edition filled in.
pub fn notes(building: NotesBuilding, drawing: NotesDrawing, code: &str) -> Vec<String> {
    let code = if code.trim().is_empty() {
        if building.irc() {
            "2021 International Residential Code (IRC)".to_owned()
        } else {
            "2021 International Building Code (IBC)".to_owned()
        }
    } else {
        code.trim().to_owned()
    };
    NOTES
        .iter()
        .filter(|(d, f, _)| *d == drawing && f.applies(building))
        .map(|(_, _, t)| t.replace("{code}", &code))
        .collect()
}

/// The drawing a view's notes default to.
pub fn drawing_for(doc: &Document, view: ElementId) -> NotesDrawing {
    use crate::element::{ElementData, ViewKind};
    match doc.data(view) {
        Ok(ElementData::View { kind, name, .. }) => {
            let n = name.to_lowercase();
            match kind {
                ViewKind::FloorPlan { .. } if n.contains("site") => NotesDrawing::SitePlan,
                ViewKind::FloorPlan { .. } if n.contains("roof") => NotesDrawing::RoofPlan,
                ViewKind::FloorPlan { .. } => NotesDrawing::FloorPlan,
                ViewKind::CeilingPlan { .. } => NotesDrawing::CeilingPlan,
                ViewKind::Elevation { .. } => NotesDrawing::Elevations,
                ViewKind::MarkerElevation { .. } => NotesDrawing::EnlargedPlans,
                ViewKind::Section { .. } => NotesDrawing::Sections,
                ViewKind::Drafting => NotesDrawing::Details,
                ViewKind::Schedule { .. } => NotesDrawing::Schedules,
                _ => NotesDrawing::General,
            }
        }
        Ok(ElementData::Sheet { name, .. }) => {
            let n = name.to_lowercase();
            if n.contains("site") {
                NotesDrawing::SitePlan
            } else if n.contains("ceiling") {
                NotesDrawing::CeilingPlan
            } else if n.contains("roof") {
                NotesDrawing::RoofPlan
            } else if n.contains("elevation") && n.contains("interior") {
                NotesDrawing::EnlargedPlans
            } else if n.contains("elevation") {
                NotesDrawing::Elevations
            } else if n.contains("section") || n.contains("detail") {
                if n.contains("wall") || n.contains("detail") {
                    NotesDrawing::Details
                } else {
                    NotesDrawing::Sections
                }
            } else if n.contains("enlarged") {
                NotesDrawing::EnlargedPlans
            } else if n.contains("schedule") {
                NotesDrawing::Schedules
            } else if n.contains("plan") {
                NotesDrawing::FloorPlan
            } else {
                NotesDrawing::General
            }
        }
        _ => NotesDrawing::General,
    }
}

/// The building type Project Info implies, else a single-family house.
pub fn building_for(doc: &Document) -> NotesBuilding {
    crate::project::get(doc)
        .ok()
        .and_then(|(_, d)| NotesBuilding::from_project_type(&d.overview.project_type))
        .unwrap_or(NotesBuilding::SingleFamily)
}

/// The code edition Project Info names ("" when not set).
pub fn code_for(doc: &Document) -> String {
    crate::project::get(doc)
        .map(|(_, d)| d.codes.building_code)
        .unwrap_or_default()
}

/// The notes as placed: the heading, then each numbered.
pub fn text(heading: &str, notes: &[String]) -> String {
    let mut s = heading.trim().to_uppercase();
    for (i, n) in notes.iter().enumerate() {
        s.push_str(&format!("\n{}. {}", i + 1, n.trim()));
    }
    s
}

/// Places the notes in `view` at `at` (the view's own coordinates), 2.4 mm text wrapping at
/// `width` paper mm, as one undo step.
pub fn place(
    doc: &mut Document,
    view: ElementId,
    at: Pt,
    heading: &str,
    notes: &[String],
    width: f64,
) -> CoreResult<ElementId> {
    if notes.is_empty() {
        return Err(CoreError::Invalid("choose at least one note".into()));
    }
    crate::ops::create_text_note(
        doc,
        view,
        at,
        &text(heading, notes),
        3.2,
        vec![],
        crate::text::TextAlign::Left,
        Some(width.clamp(60.0, 400.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_building_and_drawing_has_notes_and_the_code_fits_the_building() {
        for b in NotesBuilding::ALL {
            for d in NotesDrawing::ALL {
                let n = notes(b, d, "");
                assert!(n.len() >= 3, "{b:?} {d:?}: {}", n.len());
                assert!(n.iter().all(|t| !t.contains("{code}")));
            }
        }
        let house = notes(NotesBuilding::SingleFamily, NotesDrawing::General, "");
        assert!(house[0].contains("IRC"));
        assert!(!house.iter().any(|t| t.contains("ADA")));
        let hotel = notes(NotesBuilding::Hotel, NotesDrawing::FloorPlan, "2024 IBC");
        assert!(hotel.iter().any(|t| t.contains("accessible guest rooms")));
        assert!(
            notes(NotesBuilding::Hotel, NotesDrawing::General, "2024 IBC")[0].contains("2024 IBC")
        );
        // Stairs differ by code.
        let s = |b| notes(b, NotesDrawing::Sections, "").join(" ");
        assert!(s(NotesBuilding::SingleFamily).contains("7 3/4"));
        assert!(s(NotesBuilding::Commercial).contains("7\" max. riser"));
        assert_eq!(
            NotesBuilding::from_project_type("Multifamily Residential"),
            Some(NotesBuilding::Multifamily)
        );
    }

    #[test]
    fn notes_are_placed_numbered_under_their_heading() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let plan = doc
            .of(crate::element::Category::View)
            .find(|e| {
                matches!(
                    &e.data,
                    crate::element::ElementData::View {
                        kind: crate::element::ViewKind::FloorPlan { .. },
                        ..
                    }
                )
            })
            .unwrap()
            .id;
        assert_eq!(drawing_for(&doc, plan), NotesDrawing::FloorPlan);
        let n = notes(building_for(&doc), drawing_for(&doc, plan), &code_for(&doc));
        let id = place(
            &mut doc,
            plan,
            Pt::new(0.0, 0.0),
            &NotesDrawing::FloorPlan.heading(),
            &n,
            150.0,
        )
        .unwrap();
        let crate::element::ElementData::TextNote { text, width, .. } = doc.data(id).unwrap()
        else {
            panic!()
        };
        assert!(text.starts_with("FLOOR PLANS GENERAL NOTES\n1. Dimensions"));
        assert_eq!(*width, Some(150.0));
        assert!(place(&mut doc, plan, Pt::new(0.0, 0.0), "X", &[], 150.0).is_err());
    }
}
