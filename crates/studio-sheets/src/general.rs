//! The general (G-series) sheets of a drawing set (ADR-104), as US practice issues them: by
//! building type, by phase, and by the jurisdiction the project is permitted in. They're
//! filled with preset text the architect edits (project data, codes, notes, abbreviations,
//! code analysis, accessibility, energy, special inspections, product approvals), not
//! placeholders. Code editions are those commonly adopted; the sheets say to verify them
//! with the building department at permit.

use serde::{Deserialize, Serialize};
use studio_core::project::ProjectDetails;
use studio_core::Document;
use ts_rs::TS;

use crate::sets::BuildingType;

/// Where the project is permitted: the code family and local requirements that shape the
/// general sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Jurisdiction {
    /// The I-Codes as published (most US jurisdictions adopt them with local amendments).
    ModelCodes,
    California,
    NewYorkCity,
    Florida,
    Texas,
    Washington,
    Massachusetts,
    Chicago,
}

impl Jurisdiction {
    pub const ALL: [Jurisdiction; 8] = [
        Jurisdiction::ModelCodes,
        Jurisdiction::California,
        Jurisdiction::NewYorkCity,
        Jurisdiction::Florida,
        Jurisdiction::Texas,
        Jurisdiction::Washington,
        Jurisdiction::Massachusetts,
        Jurisdiction::Chicago,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Jurisdiction::ModelCodes => "International Codes (most US jurisdictions)",
            Jurisdiction::California => "California (Title 24)",
            Jurisdiction::NewYorkCity => "New York City (NYC DOB)",
            Jurisdiction::Florida => "Florida (FBC)",
            Jurisdiction::Texas => "Texas (TAS / TDLR)",
            Jurisdiction::Washington => "Washington (WSEC)",
            Jurisdiction::Massachusetts => "Massachusetts (780 CMR)",
            Jurisdiction::Chicago => "Chicago (Chicago Construction Codes)",
        }
    }
    /// The jurisdiction Project Info's location implies.
    pub fn from_project(d: &ProjectDetails) -> Jurisdiction {
        let l = &d.location;
        let state = l.state.trim().to_lowercase();
        let city = format!("{} {}", l.city, l.jurisdiction).to_lowercase();
        let is = |abbr: &str, name: &str| state == abbr || state == name;
        if is("ca", "california") {
            Jurisdiction::California
        } else if is("ny", "new york")
            && [
                "new york",
                "brooklyn",
                "queens",
                "bronx",
                "staten island",
                "manhattan",
                "nyc",
            ]
            .iter()
            .any(|c| city.contains(c))
        {
            Jurisdiction::NewYorkCity
        } else if is("fl", "florida") {
            Jurisdiction::Florida
        } else if is("tx", "texas") {
            Jurisdiction::Texas
        } else if is("wa", "washington") {
            Jurisdiction::Washington
        } else if is("ma", "massachusetts") {
            Jurisdiction::Massachusetts
        } else if is("il", "illinois") && city.contains("chicago") {
            Jurisdiction::Chicago
        } else {
            Jurisdiction::ModelCodes
        }
    }
    /// High-velocity hurricane zone (Miami-Dade and Broward counties).
    fn hvhz(self, d: &ProjectDetails) -> bool {
        let c = d.location.county.to_lowercase();
        self == Jurisdiction::Florida && (c.contains("miami") || c.contains("broward"))
    }
}

/// A titled block of text on a general sheet: a heading and its lines (numbered when
/// `numbered`).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub title: String,
    pub lines: Vec<String>,
    pub numbered: bool,
}

fn block(title: &str, lines: Vec<String>) -> Block {
    Block {
        title: title.into(),
        lines,
        numbered: false,
    }
}

fn numbered(title: &str, lines: Vec<String>) -> Block {
    Block {
        title: title.into(),
        lines,
        numbered: true,
    }
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

/// A general sheet: number within G, name, blocks, and the phases whose sets include it.
pub struct GeneralSheet {
    pub number: usize,
    pub name: String,
    pub blocks: Vec<Block>,
    pub phases: &'static [&'static str],
}

const SD_ON: &[&str] = &["SD", "DD", "CD", "BN", "CA"];
const DD_ON: &[&str] = &["DD", "CD", "BN", "CA"];
const CD_ON: &[&str] = &["CD", "BN", "CA"];

/// The code family, by building type and jurisdiction: (building code, residential? , the
/// applicable codes list).
fn codes(t: BuildingType, j: Jurisdiction) -> Vec<String> {
    let irc = t.irc();
    let mut v: Vec<String> = match j {
        Jurisdiction::ModelCodes => vec![
            if irc { "2021 International Residential Code (IRC)" } else { "2021 International Building Code (IBC)" }.into(),
            "2021 International Existing Building Code (IEBC), where applicable".into(),
            "2021 International Energy Conservation Code (IECC)".into(),
            "2021 International Mechanical Code (IMC)".into(),
            "2021 International Plumbing Code (IPC)".into(),
            "2021 International Fuel Gas Code (IFGC)".into(),
            "2020 National Electrical Code (NFPA 70)".into(),
            "2021 International Fire Code (IFC)".into(),
        ],
        Jurisdiction::California => vec![
            "2022 California Building Standards Code, Title 24, CCR:".into(),
            if irc { "  Part 2.5 — California Residential Code (CRC)" } else { "  Part 2 — California Building Code (CBC), Vols. 1 and 2" }.into(),
            "  Part 3 — California Electrical Code (CEC)".into(),
            "  Part 4 — California Mechanical Code (CMC)".into(),
            "  Part 5 — California Plumbing Code (CPC)".into(),
            "  Part 6 — California Energy Code".into(),
            "  Part 9 — California Fire Code (CFC)".into(),
            "  Part 11 — California Green Building Standards Code (CALGreen)".into(),
            "Local amendments and ordinances of the city or county".into(),
        ],
        Jurisdiction::NewYorkCity => vec![
            "2022 New York City Building Code (NYC BC)".into(),
            "2020 New York City Energy Conservation Code (NYCECC)".into(),
            "2022 NYC Mechanical, Plumbing and Fuel Gas Codes".into(),
            "2011 NYC Electrical Code (NEC 2008 with amendments) and current amendments".into(),
            "2022 New York City Fire Code".into(),
            "New York City Zoning Resolution".into(),
            "NYC Local Law 97 (building emissions limits), where applicable".into(),
        ],
        Jurisdiction::Florida => vec![
            "Florida Building Code, 8th Edition (2023):".into(),
            if irc { "  FBC — Residential" } else { "  FBC — Building" }.into(),
            "  FBC — Energy Conservation".into(),
            "  FBC — Accessibility".into(),
            "  FBC — Mechanical, Plumbing and Fuel Gas".into(),
            "  FBC — Existing Building, where applicable".into(),
            "2020 National Electrical Code (NFPA 70)".into(),
            "Florida Fire Prevention Code, 8th Edition (NFPA 1 and NFPA 101)".into(),
        ],
        Jurisdiction::Texas => vec![
            if irc { "International Residential Code, as adopted and amended by the city" } else { "International Building Code, as adopted and amended by the city" }.into(),
            "International Energy Conservation Code, as adopted by the city (state minimum applies)".into(),
            "International Mechanical, Plumbing and Fuel Gas Codes, as adopted".into(),
            "National Electrical Code (NFPA 70), as adopted".into(),
            "International Fire Code, as adopted by the fire marshal".into(),
            "2012 Texas Accessibility Standards (TAS), Architectural Barriers Act, Gov. Code Ch. 469".into(),
        ],
        Jurisdiction::Washington => vec![
            if irc { "2021 International Residential Code with Washington amendments (WAC 51-51)" } else { "2021 International Building Code with Washington amendments (WAC 51-50)" }.into(),
            "2021 Washington State Energy Code (WSEC), Residential or Commercial".into(),
            "2021 International Mechanical Code with Washington amendments (WAC 51-52)".into(),
            "2021 Uniform Plumbing Code with Washington amendments (WAC 51-56)".into(),
            "2023 National Electrical Code (NFPA 70) as adopted by L&I".into(),
            "2021 International Fire Code with Washington amendments (WAC 51-54A)".into(),
            "Local amendments (e.g., Seattle Building and Energy Codes), where applicable".into(),
        ],
        Jurisdiction::Massachusetts => vec![
            "780 CMR, Massachusetts State Building Code, 10th Edition (2021 I-Codes with amendments)".into(),
            "225 CMR 22/23 Stretch Energy Code or Specialized Code, as adopted by the municipality".into(),
            "521 CMR, Massachusetts Architectural Access Board (MAAB) regulations".into(),
            "248 CMR, Massachusetts Plumbing and Fuel Gas Code".into(),
            "527 CMR 12.00, Massachusetts Electrical Code (NFPA 70 with amendments)".into(),
            "527 CMR 1.00, Massachusetts Comprehensive Fire Safety Code (NFPA 1)".into(),
        ],
        Jurisdiction::Chicago => vec![
            "Chicago Construction Codes (Municipal Code Titles 14A–14X):".into(),
            "  Title 14B — Chicago Building Code (based on the 2018 IBC)".into(),
            "  Title 14R — Chicago Rehabilitation Code, where applicable".into(),
            "  Title 14N — Chicago Energy Transformation Code".into(),
            "  Title 14E — Chicago Electrical Code".into(),
            "  Title 14M — Chicago Mechanical Code".into(),
            "  Title 14P — Chicago Plumbing Code".into(),
            "Chicago Zoning Ordinance (Title 17)".into(),
        ],
    };
    if !irc {
        v.push(match j {
            Jurisdiction::California => "Accessibility: CBC Chapter 11A (housing) and 11B (public accommodations)".into(),
            Jurisdiction::NewYorkCity => "Accessibility: NYC BC Chapter 11 and ICC A117.1-2009 as modified".into(),
            Jurisdiction::Texas => "Accessibility: 2012 TAS (TDLR registration and RAS inspection), and the Fair Housing Act".into(),
            Jurisdiction::Massachusetts => "Accessibility: 521 CMR (MAAB) and the ADA 2010 Standards".into(),
            Jurisdiction::Florida => "Accessibility: FBC — Accessibility and the ADA 2010 Standards".into(),
            _ => "Accessibility: IBC Chapter 11, ICC A117.1-2017 and the 2010 ADA Standards".into(),
        });
        if matches!(
            t,
            BuildingType::GardenApartments
                | BuildingType::MidRiseApartments
                | BuildingType::MixedUse
                | BuildingType::Townhouses
        ) {
            v.push(
                "Federal Fair Housing Act Design Manual and Accessibility Guidelines (FHAG)".into(),
            );
        }
    }
    v.push("Verify the code editions in force with the building department at the time of permit application.".into());
    v
}

/// Occupancy and construction type defaults by building type (Project Info overrides).
fn occupancy(t: BuildingType) -> (&'static str, &'static str, &'static str) {
    match t {
        BuildingType::SingleFamily => (
            "R-3 (IRC one-family dwelling)",
            "V-B",
            "Not required by the IRC unless locally amended (NFPA 13D where provided)",
        ),
        BuildingType::Duplex => (
            "R-3 (IRC two-family dwelling)",
            "V-B",
            "NFPA 13D where required locally",
        ),
        BuildingType::Townhouses => (
            "R-3 (IRC townhouses)",
            "V-B",
            "NFPA 13D or IRC P2904 where required",
        ),
        BuildingType::GardenApartments => ("R-2", "V-A", "NFPA 13R throughout"),
        BuildingType::MidRiseApartments => (
            "R-2 over S-2 parking",
            "III-A over I-A podium",
            "NFPA 13 throughout",
        ),
        BuildingType::MixedUse => (
            "R-2 over M / B, separated occupancies",
            "V-A or III-A over I-A podium",
            "NFPA 13 throughout",
        ),
        BuildingType::Hotel => (
            "R-1 with A-2 / A-3 / B accessory",
            "III-A or V-A",
            "NFPA 13 throughout",
        ),
    }
}

/// IBC Table 601: fire-resistance ratings (hours) of building elements by construction type.
fn table_601(ct: &str) -> Option<[&'static str; 6]> {
    let k = ct.trim().to_uppercase().replace(' ', "");
    let k = k.split(['(', 'O']).next().unwrap_or("").to_string();
    // [primary frame, ext. bearing walls, int. bearing walls, int. nonbearing, floors, roof]
    Some(match k.as_str() {
        "I-A" | "IA" => ["3", "3", "3", "0", "2", "1-1/2"],
        "I-B" | "IB" => ["2", "2", "2", "0", "2", "1"],
        "II-A" | "IIA" => ["1", "1", "1", "0", "1", "1"],
        "II-B" | "IIB" => ["0", "0", "0", "0", "0", "0"],
        "III-A" | "IIIA" => ["1", "2", "1", "0", "1", "1"],
        "III-B" | "IIIB" => ["0", "2", "0", "0", "0", "0"],
        "IV-HT" | "IVHT" => ["HT", "2", "1/HT", "0/HT", "HT", "HT"],
        "V-A" | "VA" => ["1", "1", "1", "0", "1", "1"],
        "V-B" | "VB" => ["0", "0", "0", "0", "0", "0"],
        _ => return None,
    })
}

const ABBREVIATIONS: &[&str] = &[
    "A.F.F. — above finished floor",
    "ACT — acoustical ceiling tile",
    "ADJ — adjustable",
    "ALUM — aluminum",
    "APPROX — approximate",
    "ARCH — architectural",
    "BD — board",
    "BLKG — blocking",
    "BOT — bottom",
    "BRG — bearing",
    "CJ — control joint",
    "CL — centerline",
    "CLG — ceiling",
    "CLR — clear",
    "CMU — concrete masonry unit",
    "COL — column",
    "CONC — concrete",
    "CONT — continuous",
    "DIA — diameter",
    "DIM — dimension",
    "DN — down",
    "DS — downspout",
    "DWG — drawing",
    "EA — each",
    "EJ — expansion joint",
    "EL — elevation",
    "ELEC — electrical",
    "EQ — equal",
    "EXIST — existing",
    "EXT — exterior",
    "FD — floor drain",
    "FDN — foundation",
    "FIN — finish",
    "FLR — floor",
    "F.O.S. — face of stud",
    "FRP — fiberglass reinforced panel",
    "FTG — footing",
    "GA — gauge",
    "GALV — galvanized",
    "GWB — gypsum wallboard",
    "HB — hose bibb",
    "HDR — header",
    "HM — hollow metal",
    "HT — height",
    "INSUL — insulation",
    "INT — interior",
    "JST — joist",
    "MAX — maximum",
    "MECH — mechanical",
    "MFR — manufacturer",
    "MIN — minimum",
    "MTL — metal",
    "N.I.C. — not in contract",
    "NOM — nominal",
    "N.T.S. — not to scale",
    "O.C. — on center",
    "OPNG — opening",
    "OPP — opposite",
    "PLYWD — plywood",
    "PT — pressure treated / paint",
    "R — radius / riser",
    "RCP — reflected ceiling plan",
    "REF — reference",
    "REQ'D — required",
    "RM — room",
    "R.O. — rough opening",
    "SCHED — schedule",
    "SHT — sheet",
    "SIM — similar",
    "SPEC — specification",
    "SS — stainless steel",
    "STD — standard",
    "STL — steel",
    "STRUCT — structural",
    "T&G — tongue and groove",
    "T.O. — top of",
    "TYP — typical",
    "U.N.O. — unless noted otherwise",
    "V.I.F. — verify in field",
    "W/ — with",
    "WD — wood",
    "WP — waterproof",
    "WRB — weather-resistive barrier",
    "WWF — welded wire fabric",
];

const SYMBOLS: &[&str] = &[
    "Building section mark: section number over sheet number, arrow toward the view",
    "Wall section and detail callout: detail number over sheet number",
    "Exterior and interior elevation marks: elevation number over sheet number",
    "Enlarged plan callout: dashed boundary with callout tag",
    "Column grid bubble: letters one way, numbers the other",
    "Level datum: level name and elevation",
    "Spot elevation and slope arrow",
    "Room tag: name, number and area",
    "Door tag (number) and window tag (type letter)",
    "Wall type tag (hexagon) and keynote tag (square)",
    "Revision cloud and delta: revision number",
    "North arrow and graphic scale",
    "Match line, break line and centerline",
];

/// The G sheets for a building type and jurisdiction, with the project's information.
pub fn general_sheets(doc: &Document, t: BuildingType, j: Jurisdiction) -> Vec<GeneralSheet> {
    let irc = t.irc();
    let (ident, d) =
        studio_core::project::get(doc).unwrap_or_else(|_| (Default::default(), Default::default()));
    let or = |v: &str, dflt: &str| {
        if v.trim().is_empty() {
            dflt.to_string()
        } else {
            v.trim().to_string()
        }
    };
    let (occ, ct, spr) = occupancy(t);
    let occ = or(&d.codes.occupancy, occ);
    let ct = or(&d.codes.construction_type, ct);
    let spr = or(&d.codes.sprinklered, spr);
    let model = studio_regen::regenerate(doc);
    let gross_sf = model.gross_area() / (304.8 * 304.8);
    let stories = doc
        .levels()
        .iter()
        .filter(|l| {
            doc.of(studio_core::Category::Wall)
                .any(|w| w.data.level() == Some(l.0))
        })
        .count()
        .max(1);
    let mut out: Vec<GeneralSheet> = vec![];

    // ------------------------------------------------ G-002 Project Information
    let addr = [
        d.location.street.as_str(),
        d.location.city.as_str(),
        d.location.state.as_str(),
        d.location.zip.as_str(),
    ]
    .iter()
    .filter(|x| !x.trim().is_empty())
    .cloned()
    .collect::<Vec<_>>()
    .join(", ");
    let mut directory: Vec<String> = vec![format!(
        "OWNER: {}",
        or(
            &d.client.company,
            &or(&ident.client, "[Owner name, address, phone]")
        )
    )];
    for m in &d.team {
        let who = [
            m.contact.company.as_str(),
            m.contact.name.as_str(),
            m.contact.phone.as_str(),
        ]
        .iter()
        .filter(|x| !x.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" · ");
        directory.push(format!(
            "{}: {}",
            m.discipline.to_uppercase(),
            or(&who, "[name, phone, email]")
        ));
    }
    for role in [
        "ARCHITECT",
        "STRUCTURAL ENGINEER",
        "MEP ENGINEER",
        "CIVIL ENGINEER",
    ] {
        // One MEP line only where the team doesn't list them separately.
        if role == "MEP ENGINEER" && directory.iter().any(|l| l.starts_with("MECHANICAL")) {
            continue;
        }
        if !directory.iter().any(|l| l.starts_with(role)) {
            directory.push(format!("{role}: [firm, contact, phone, email]"));
        }
    }
    let mut data = vec![
        format!("PROJECT: {}", or(&ident.name, "[Project name]")),
        format!(
            "ADDRESS: {}",
            or(&addr, &or(&ident.address, "[Street, city, state, zip]"))
        ),
        format!(
            "APN / PARCEL: {}",
            or(&d.location.apn, "[Assessor's parcel number]")
        ),
        format!(
            "ZONING: {}",
            or(&d.codes.zoning_district, "[Zoning district]")
        ),
        format!("LOT AREA: {}", or(&d.codes.lot_area, "[sf]")),
        format!("OCCUPANCY: {occ}"),
        format!("CONSTRUCTION TYPE: {ct}"),
        format!("FIRE SPRINKLERS: {spr}"),
        format!("STORIES: {stories}"),
        format!(
            "GROSS BUILDING AREA: {:.0} sf (outside face of walls)",
            gross_sf
        ),
        format!("BUILDING TYPE: {}", t.label()),
    ];
    if !d.codes.max_height.trim().is_empty() {
        data.push(format!("MAXIMUM HEIGHT ALLOWED: {}", d.codes.max_height));
    }
    if !d.codes.setbacks.trim().is_empty() {
        data.push(format!("SETBACKS: {}", d.codes.setbacks));
    }
    let scope = vec![or(
        &d.overview.description,
        &format!(
            "New {} of {:.0} sf on {} {}: architectural, structural, mechanical, plumbing and electrical work as shown.",
            t.label().to_lowercase(),
            gross_sf,
            stories,
            if stories == 1 { "story" } else { "stories" },
        ),
    )];
    let mut deferred =
        s(&["Fire sprinkler system (shop drawings and calculations by a licensed contractor)"]);
    if !irc {
        deferred.extend(s(&[
            "Fire alarm system",
            "Pre-engineered roof and floor trusses",
            "Exterior cladding attachments and curtain wall / storefront engineering",
            "Guardrail and handrail systems",
        ]));
    } else {
        deferred.extend(s(&[
            "Pre-engineered roof trusses (truss calculations and layout)",
        ]));
    }
    if j == Jurisdiction::Florida {
        deferred.push(
            "Product approvals for windows, doors, roofing and shutters not listed on G-011".into(),
        );
    }
    if j == Jurisdiction::California {
        deferred.push("Photovoltaic system per the California Energy Code".into());
    }
    deferred.push("Deferred submittals shall be reviewed by the architect and approved by the building official before installation.".into());
    out.push(GeneralSheet {
        number: 2,
        name: "Project Information".into(),
        blocks: vec![
            block("PROJECT DIRECTORY", directory),
            block("PROJECT DATA", data),
            block("SCOPE OF WORK", scope),
            block("APPLICABLE CODES", codes(t, j)),
            block(
                "JURISDICTION",
                vec![or(&d.location.jurisdiction, j.label())],
            ),
            numbered("DEFERRED SUBMITTALS", deferred),
            block(
                "VICINITY MAP",
                s(&["[Insert vicinity map: site location, north arrow, nearby streets.]"]),
            ),
        ],
        phases: SD_ON,
    });

    // ------------------------------------------------ G-003 General Notes
    let nb = match t {
        BuildingType::SingleFamily => studio_core::general_notes::NotesBuilding::SingleFamily,
        BuildingType::Duplex | BuildingType::Townhouses => {
            studio_core::general_notes::NotesBuilding::DuplexTownhouse
        }
        BuildingType::GardenApartments | BuildingType::MidRiseApartments => {
            studio_core::general_notes::NotesBuilding::Multifamily
        }
        BuildingType::MixedUse => studio_core::general_notes::NotesBuilding::MixedUse,
        BuildingType::Hotel => studio_core::general_notes::NotesBuilding::Hotel,
    };
    let code_name = codes(t, j).first().cloned().unwrap_or_default();
    let code = or(&d.codes.building_code, code_name.trim_end_matches(':'));
    let mut general = studio_core::general_notes::notes(
        nb,
        studio_core::general_notes::NotesDrawing::General,
        &code,
    );
    general.extend(match j {
        Jurisdiction::California => s(&[
            "Comply with the California Energy Code and CALGreen; the signed compliance documents (CF1R / NRCC) are part of these drawings.",
            "Construction waste: divert at least 65% of nonhazardous construction and demolition waste per CALGreen 4.408 / 5.408.",
        ]),
        Jurisdiction::NewYorkCity => s(&[
            "All work shall be filed with and approved by the NYC Department of Buildings; post permits on site.",
            "Special and progress inspections per the TR1 / TR8 statements of responsibility.",
            "Site safety per NYC BC Chapter 33; protect adjoining properties per BC 3309.",
        ]),
        Jurisdiction::Florida => s(&[
            "Design wind speed and exposure per FBC Chapter 16 and ASCE 7; see the structural drawings for design pressures.",
            "All exterior windows, doors, skylights, roofing and shutters shall have Florida Product Approval or Miami-Dade NOA (see G-011).",
        ]),
        Jurisdiction::Texas => s(&[
            "Projects of $50,000 or more subject to the Architectural Barriers Act shall be registered with TDLR and inspected by a Registered Accessibility Specialist.",
        ]),
        Jurisdiction::Washington => s(&[
            "Comply with the Washington State Energy Code, including its energy credits and air leakage testing.",
        ]),
        Jurisdiction::Massachusetts => s(&[
            "Construction control per 780 CMR 107 where required: the registered design professional shall file the construction control document.",
        ]),
        Jurisdiction::Chicago => s(&[
            "Permit through the Chicago Department of Buildings; comply with the Chicago Energy Transformation Code.",
        ]),
        Jurisdiction::ModelCodes => vec![],
    });
    out.push(GeneralSheet {
        number: 3,
        name: "General Notes".into(),
        blocks: vec![numbered("GENERAL NOTES", general)],
        phases: CD_ON,
    });

    // ------------------------------------------------ G-004 Abbreviations & Symbols
    out.push(GeneralSheet {
        number: 4,
        name: "Abbreviations, Symbols & Legends".into(),
        blocks: vec![
            block("ABBREVIATIONS", s(ABBREVIATIONS)),
            block("SYMBOLS LEGEND", s(SYMBOLS)),
            block(
                "MATERIAL INDICATIONS",
                s(&["Concrete, CMU, brick, stone, earth, gravel, wood (finish and framing), plywood, steel, insulation (batt and rigid), gypsum board — as shown in section."]),
            ),
        ],
        phases: DD_ON,
    });

    // ------------------------------------------------ G-005 Code Analysis / Summary
    if irc {
        out.push(GeneralSheet {
            number: 5,
            name: "Code Summary".into(),
            blocks: vec![
                block("CODE SUMMARY", vec![
                    format!("CODE: {code}"),
                    format!("OCCUPANCY: {occ}"),
                    format!("CONSTRUCTION TYPE: {ct}"),
                    format!("STORIES ABOVE GRADE: {stories}"),
                    format!("CONDITIONED AREA: {gross_sf:.0} sf"),
                    format!("AUTOMATIC SPRINKLERS: {spr}"),
                ]),
                numbered("LIFE SAFETY", s(&[
                    "Emergency escape and rescue openings in every sleeping room and habitable basement (IRC R310).",
                    "Smoke alarms (R314) and carbon monoxide alarms (R315), interconnected and hard-wired with battery backup.",
                    "Dwelling / garage separation per R302.5 and R302.6.",
                    "Stairs, handrails and guards per R311.7 and R312; window fall protection per R312.2.",
                ])),
                numbered("FIRE SEPARATION", vec![
                    "Exterior walls less than 5'-0\" from the lot line: 1-hour rated, openings limited per R302.1.".into(),
                    if t == BuildingType::Townhouses || t == BuildingType::Duplex {
                        "Dwelling unit separation: 1-hour (or 2-hour townhouse common wall) per R302.2 / R302.3, continuous to the roof sheathing.".into()
                    } else {
                        "Projections within 5'-0\" of the lot line protected per R302.1.".into()
                    },
                ]),
                block("ENERGY", vec![match j {
                    Jurisdiction::California => "California Energy Code, Part 6: performance compliance, CF1R signed by the documentation author (see G-008).".into(),
                    Jurisdiction::Washington => "WSEC Residential: prescriptive envelope plus energy credits per Table R406.2 (see G-008).".into(),
                    _ => "IECC Residential Chapter 4: prescriptive or REScheck compliance for the climate zone (see G-008).".into(),
                }]),
            ],
            phases: DD_ON,
        });
    } else {
        let r = table_601(&ct);
        let mut ratings =
            vec!["BUILDING ELEMENT — REQUIRED RATING (HOURS), IBC TABLE 601".to_string()];
        let names = [
            "Primary structural frame",
            "Exterior bearing walls",
            "Interior bearing walls",
            "Interior nonbearing walls",
            "Floor construction",
            "Roof construction",
        ];
        match r {
            Some(r) => {
                for (n, h) in names.iter().zip(r) {
                    ratings.push(format!("{n}: {h}"));
                }
            }
            None => ratings.push(format!("[Fill in for construction type {ct}]")),
        }
        // Residential (R) occupant load factor, gross (Table 1004.5).
        let load = 200.0;
        out.push(GeneralSheet {
            number: 5,
            name: "Code Analysis".into(),
            blocks: vec![
                block("BUILDING DATA", vec![
                    format!("CODE: {code}"),
                    format!("OCCUPANCY CLASSIFICATION: {occ}"),
                    format!("CONSTRUCTION TYPE: {ct}"),
                    format!("AUTOMATIC SPRINKLER SYSTEM: {spr}"),
                    format!("STORIES ABOVE GRADE: {stories}"),
                    format!("GROSS BUILDING AREA: {gross_sf:.0} sf"),
                ]),
                block("ALLOWABLE HEIGHT AND AREA (IBC CHAPTER 5)", s(&[
                    "Allowable height (Table 504.3) and stories (Table 504.4): [ft / stories] for the occupancy, type and sprinklers",
                    "Allowable area per story (Table 506.2): [sf], with frontage increase per 506.3: [sf]",
                    "Actual building height and largest floor area: see elevations and plans",
                    "Mixed occupancies: separated per 508.4 (Table 508.4) or nonseparated per 508.3, as indicated",
                ])),
                block("FIRE-RESISTANCE RATINGS", ratings),
                block("OCCUPANT LOAD AND EGRESS (IBC CHAPTER 10)", vec![
                    format!("Occupant load: {:.0} sf gross / {load:.0} sf per occupant = {:.0} occupants (Table 1004.5); see life safety plans by floor", gross_sf, (gross_sf / load).ceil()),
                    "Exits required per story: 2 (Table 1006.3.3), 1 permitted where Table 1006.3.4 allows".into(),
                    "Exit access travel distance (Table 1017.2): 250' sprinklered (R)".into(),
                    "Common path of egress travel (Table 1006.2.1): 125' sprinklered (R-2)".into(),
                    "Corridor fire rating (Table 1020.2): 0.5 hr sprinklered (R)".into(),
                    "Egress width: 0.2\"/occupant stairs, 0.15\"/occupant other components (sprinklered)".into(),
                ]),
                block("PLUMBING FIXTURES", s(&["Required fixtures per IPC Table 2902.1 (or the local plumbing code) by occupancy: [table]"])),
                block("SEPARATIONS", vec![
                    match t {
                        BuildingType::Hotel => "Sleeping units: 1-hour fire partitions and horizontal assemblies (420.2, 420.3; 0.5 hr in Type II-B, III-B, V-B sprinklered).".into(),
                        BuildingType::MixedUse => "Dwelling units: 1-hour; occupancy separations per Table 508.4 between R-2 and M / B / S-2.".into(),
                        _ => "Dwelling units: 1-hour fire partitions and horizontal assemblies (420.2, 420.3).".into(),
                    },
                    "Shafts: 2-hour connecting four or more stories, else 1-hour (713.4).".into(),
                    "Rated doors: 90 min in 2-hour, 60 min in 1-hour shafts and stairs, 20 min in corridors (Table 716.1(2)).".into(),
                ]),
            ],
            phases: DD_ON,
        });
        // ------------------------------------------------ G-006 Life Safety Plans
        out.push(GeneralSheet {
            number: 6,
            name: "Life Safety Plans".into(),
            blocks: vec![block(
                "LIFE SAFETY LEGEND",
                s(&[
                    "Exit and exit discharge, with occupant load and capacity at each exit",
                    "Exit access travel distance path (dimensioned) and common path of travel",
                    "1-hour and 2-hour fire-rated walls and fire partitions (line types)",
                    "Rated doors with their ratings; smoke barriers",
                    "Fire extinguisher cabinets, fire department connection, alarm panels",
                    "Accessible means of egress and areas of refuge",
                    "[Place a life safety plan of each floor on this sheet.]",
                ]),
            )],
            phases: CD_ON,
        });
        // ------------------------------------------------ G-007 Accessibility
        let std = match j {
            Jurisdiction::California => {
                "CBC Chapter 11A (covered multifamily dwellings) and 11B (public accommodations)"
            }
            Jurisdiction::Texas => "2012 Texas Accessibility Standards (TAS)",
            Jurisdiction::NewYorkCity => "NYC BC Chapter 11 and ICC A117.1-2009 as modified",
            Jurisdiction::Massachusetts => "521 CMR (MAAB)",
            Jurisdiction::Florida => "FBC — Accessibility",
            _ => "IBC Chapter 11 and ICC A117.1-2017",
        };
        let mut acc = vec![
            format!("Comply with {std} and the 2010 ADA Standards for Accessible Design."),
            "Accessible route connecting public way, parking, entrances and all accessible spaces; 36\" min. clear width, 1:20 max. running slope, 1:48 max. cross slope.".into(),
            "Ramps 1:12 max. with handrails both sides and level landings; curb ramps with detectable warnings.".into(),
            "Doors: 32\" min. clear opening, maneuvering clearances, 5 lbf max. opening force (interior), hardware 34\"–48\" AFF.".into(),
            "Toilet rooms: 60\" turning space, grab bars, water closet centerline 16\"–18\" from the side wall, lavatory 34\" max. with knee clearance.".into(),
            "Signage: tactile room identification signs at the latch side of doors, 48\"–60\" AFF to the baseline of the text.".into(),
        ];
        match t {
            BuildingType::Hotel => acc.push("Accessible guest rooms per IBC 1108 / ADA 224: with mobility features (with and without roll-in showers) and communication features, dispersed among room types.".into()),
            BuildingType::GardenApartments | BuildingType::MidRiseApartments | BuildingType::MixedUse => {
                acc.push("Accessible (Type A) units: 2% of units (IBC 1108.6.2.1); Type B units: all units in buildings with elevators, ground-floor units otherwise.".into());
                acc.push("Fair Housing Act: all covered ground-floor and elevator-served units meet the FHAG requirements.".into());
            }
            _ => {}
        }
        if j == Jurisdiction::Texas {
            acc.push("Register the project with TDLR (TABS) before construction; RAS plan review and inspection within one year of completion.".into());
        }
        if j == Jurisdiction::California {
            acc.push("Path-of-travel and 11B-202.4 disproportionality for alterations; CASp review recommended.".into());
        }
        out.push(GeneralSheet {
            number: 7,
            name: "Accessibility Notes & Details".into(),
            blocks: vec![
                numbered("ACCESSIBILITY NOTES", acc),
                block("DETAILS", s(&["[Place accessible clearance diagrams, toilet room plans and unit plans on this sheet.]"])),
            ],
            phases: CD_ON,
        });
    }

    // ------------------------------------------------ G-008 Energy Compliance
    let energy: Vec<String> = match (j, irc) {
        (Jurisdiction::California, true) => s(&[
            "California Energy Code (Title 24, Part 6), low-rise residential performance compliance.",
            "The registered CF1R-PRF-01 is printed on this sheet and is part of the drawings.",
            "HERS verification required for duct leakage, refrigerant charge, airflow, fan watt draw and QII as listed on the CF1R.",
            "Solar photovoltaic system sized per §150.1(c)14; battery-ready per §150.0(s); heat-pump-ready and electric-ready provisions per §150.0(n), (t), (u), (v).",
            "Installation certificates (CF2R) and HERS certificates (CF3R) shall be completed before final inspection.",
        ]),
        (Jurisdiction::California, false) => s(&[
            "California Energy Code (Title 24, Part 6), nonresidential and high-rise residential compliance.",
            "NRCC envelope, mechanical, lighting and solar forms printed on these drawings; NRCI and NRCA by the contractor and acceptance test technician.",
            "Solar PV and battery storage per §140.10 where required.",
        ]),
        (Jurisdiction::Washington, _) => s(&[
            "Washington State Energy Code: envelope (U-factor alternative or component performance), mechanical and lighting compliance forms on these drawings.",
            "Residential: energy credits per Table R406.2 / R406.3 as selected below; blower-door air leakage test required.",
            "Commercial: additional efficiency credits per C406; commissioning per C408.",
        ]),
        (Jurisdiction::NewYorkCity, _) => s(&[
            "NYC Energy Conservation Code: Energy Analysis (EN1) and supporting documentation filed with DOB; progress inspections per the TR8.",
            "Local Law 97 emissions limits apply to buildings over 25,000 sf.",
        ]),
        (Jurisdiction::Massachusetts, _) => s(&[
            "Massachusetts Stretch Energy Code (or Specialized Code where adopted): HERS rating / performance path, or Passive House where required by size.",
            "Blower-door test and HERS rater verification required.",
        ]),
        (Jurisdiction::Florida, _) => s(&[
            "FBC — Energy Conservation: EnergyGauge or prescriptive compliance forms (R405 / C407) attached to these drawings.",
            "Duct leakage test and envelope air leakage test per R402.4 / R403.3.",
        ]),
        (_, true) => s(&[
            "IECC Residential Chapter 4 for the climate zone: insulation and fenestration per Table R402.1.3 (or REScheck report attached).",
            "Air sealing per R402.4: blower-door test at 5.0 ACH50 max. (3.0 in climate zones 3–8), duct leakage test per R403.3.",
            "Mechanical ventilation per R403.6; programmable thermostat; high-efficacy lighting per R404.",
        ]),
        (_, false) => s(&[
            "IECC Commercial (or ASHRAE 90.1): COMcheck envelope, mechanical and lighting compliance reports on these drawings.",
            "Additional efficiency package per C406; commissioning per C408 for systems over the thresholds.",
            "Continuous air barrier per C402.5; lighting controls per C405.",
        ]),
    };
    out.push(GeneralSheet {
        number: 8,
        name: "Energy Code Compliance".into(),
        blocks: vec![
            numbered("ENERGY COMPLIANCE", energy),
            block(
                "ENVELOPE SUMMARY",
                s(&[
                    "Roof / ceiling: R-[ ] (U-[ ])",
                    "Above-grade walls: R-[ ] + R-[ ] ci",
                    "Floor over unconditioned: R-[ ]",
                    "Slab edge: R-[ ], [ ] ft deep",
                    "Fenestration: U-[ ], SHGC [ ]",
                    "Air leakage: [ ] ACH50",
                ]),
            ),
        ],
        phases: CD_ON,
    });

    // ------------------------------------------------ G-009 CALGreen (California)
    if j == Jurisdiction::California {
        out.push(GeneralSheet {
            number: 9,
            name: "CALGreen Checklist".into(),
            blocks: vec![numbered("CALIFORNIA GREEN BUILDING STANDARDS (CALGREEN)", if irc { s(&[
                "4.106.2 Storm water drainage and retention during construction.",
                "4.106.4 Electric vehicle (EV) charging: EV-capable / EV-ready spaces as required.",
                "4.303.1 Water-conserving plumbing fixtures and fittings (maximum flow rates).",
                "4.304.1 Outdoor potable water use: MWELO-compliant irrigation where applicable.",
                "4.408.1 Construction waste management: 65% diversion; documentation to the enforcing agency.",
                "4.410.1 Operation and maintenance manual given to the building owner.",
                "4.503.1 Fireplaces: direct-vent sealed-combustion or EPA-certified wood stoves.",
                "4.504 Pollutant control: low-VOC adhesives, sealants, paints, carpet and composite wood.",
                "4.505.2 Concrete slab vapor retarder; 4.505.3 framing moisture content 19% max. before enclosure.",
                "4.506.1 Bathroom exhaust fans ENERGY STAR, humidistat-controlled.",
            ]) } else { s(&[
                "5.106.4 Bicycle parking; 5.106.5.3 EV charging spaces.",
                "5.303 Indoor water use reduction: metering and fixture flow rates.",
                "5.304 Outdoor water use: irrigation controllers, MWELO.",
                "5.408 Construction waste: 65% diversion with waste management plan.",
                "5.410 Building commissioning for new buildings over 10,000 sf.",
                "5.504 Pollutant control: low-emitting finish materials, filters MERV 13.",
                "5.506 Indoor air quality: outdoor air delivery monitoring.",
            ]) })],
            phases: CD_ON,
        });
    }

    // ------------------------------------------------ G-010 Special Inspections (IBC)
    if !irc || j == Jurisdiction::NewYorkCity {
        let mut si = match j {
            Jurisdiction::NewYorkCity => s(&[
                "Special and progress inspections per NYC BC Chapter 17, identified on the TR1 (special inspections), TR8 (energy) and TR2/TR3 (concrete) forms filed with DOB.",
            ]),
            Jurisdiction::Florida => s(&[
                "Special inspections per FBC Chapter 17; threshold buildings (over 3 stories or 50 ft, or assembly over 5,000 sf / 500 occupants) require a threshold inspection plan per F.S. 553.79.",
            ]),
            _ => s(&["Special inspections per IBC Chapter 17 by an approved agency engaged by the owner, per the statement of special inspections (IBC 1704.3)."]),
        };
        si.extend(s(&[
            "Concrete: reinforcement placement, cylinders, placement and curing (Table 1705.3).",
            "Structural steel: high-strength bolting and welding (1705.2).",
            "Masonry: Level 1 or 2 per TMS 402 (1705.4).",
            "Wood: high-load diaphragms and metal-plate-connected trusses over 60' (1705.5).",
            "Soils: excavation, fill and bearing verification (1705.6).",
            "Wind and seismic: main wind- and seismic-force-resisting systems where required (1705.11, 1705.12).",
            "Fire-resistant penetrations and joints in high-rise and Risk Category III/IV buildings (1705.17); sprayed fire-resistive materials (1705.14).",
            "The special inspector's reports go to the building official, the architect and the engineer of record; a final report is required before the certificate of occupancy.",
        ]));
        out.push(GeneralSheet {
            number: 10,
            name: "Statement of Special Inspections".into(),
            blocks: vec![numbered("SPECIAL INSPECTIONS", si)],
            phases: CD_ON,
        });
    }

    // ------------------------------------------------ G-011 Product Approvals (Florida)
    if j == Jurisdiction::Florida {
        let hvhz = j.hvhz(&d);
        out.push(GeneralSheet {
            number: 11,
            name: "Product Approval Schedule".into(),
            blocks: vec![
                block("PRODUCT APPROVALS", vec![
                    if hvhz {
                        "High-Velocity Hurricane Zone (Miami-Dade / Broward): every exterior component listed below requires a Miami-Dade County Notice of Acceptance (NOA).".into()
                    } else {
                        "Every exterior component listed below requires Florida Product Approval (FL#) or local approval per FBC 1709 / Rule 61G20-3.".into()
                    },
                    "CATEGORY — MANUFACTURER — PRODUCT — FL# / NOA — DESIGN PRESSURE".into(),
                    "Exterior doors — [ ] — [ ] — [ ] — +[ ]/-[ ] psf".into(),
                    "Windows — [ ] — [ ] — [ ] — +[ ]/-[ ] psf".into(),
                    "Garage doors — [ ] — [ ] — [ ] — +[ ]/-[ ] psf".into(),
                    "Skylights — [ ] — [ ] — [ ] — +[ ]/-[ ] psf".into(),
                    "Roofing system and underlayment — [ ] — [ ] — [ ] — [ ]".into(),
                    "Storm shutters / impact protection — [ ] — [ ] — [ ] — [ ]".into(),
                    "Soffits and siding — [ ] — [ ] — [ ] — [ ]".into(),
                    "Structural connectors — [ ] — [ ] — [ ] — [ ]".into(),
                ]),
            ],
            phases: CD_ON,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Document {
        let mut d = Document::new();
        studio_core::ops::seed_default_project(&mut d).unwrap();
        d
    }

    #[test]
    fn the_general_sheets_follow_the_building_type_and_jurisdiction() {
        let d = doc();
        let nums = |t, j| {
            general_sheets(&d, t, j)
                .iter()
                .map(|g| g.number)
                .collect::<Vec<_>>()
        };
        // A house under the model codes: information, notes, abbreviations, code summary,
        // energy.
        assert_eq!(
            nums(BuildingType::SingleFamily, Jurisdiction::ModelCodes),
            [2, 3, 4, 5, 8]
        );
        // Apartments add life safety, accessibility and special inspections.
        assert_eq!(
            nums(BuildingType::MidRiseApartments, Jurisdiction::ModelCodes),
            [2, 3, 4, 5, 6, 7, 8, 10]
        );
        // California adds CALGreen; Florida product approvals.
        assert!(nums(BuildingType::SingleFamily, Jurisdiction::California).contains(&9));
        assert!(nums(BuildingType::Hotel, Jurisdiction::Florida).contains(&11));
        let text = |t, j| {
            general_sheets(&d, t, j)
                .iter()
                .flat_map(|g| g.blocks.iter().flat_map(|b| b.lines.clone()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let ca = text(BuildingType::SingleFamily, Jurisdiction::California);
        assert!(ca.contains("California Residential Code") && ca.contains("CF1R"));
        let tx = text(BuildingType::GardenApartments, Jurisdiction::Texas);
        assert!(
            tx.contains("Texas Accessibility Standards")
                && tx.contains("TDLR")
                && tx.contains("Type A")
        );
        let nyc = text(BuildingType::Hotel, Jurisdiction::NewYorkCity);
        assert!(nyc.contains("TR1") && nyc.contains("Accessible guest rooms"));
        // Code analysis fills the Table 601 ratings for the construction type.
        let mf = text(BuildingType::GardenApartments, Jurisdiction::ModelCodes);
        assert!(mf.contains("Primary structural frame: 1"), "V-A");
        assert_eq!(table_601("I-A").unwrap()[0], "3");
        assert_eq!(table_601("V-B").unwrap()[5], "0");
        // Every sheet has content.
        for t in BuildingType::ALL {
            for j in Jurisdiction::ALL {
                for g in general_sheets(&d, t, j) {
                    assert!(
                        g.blocks.iter().all(|b| !b.lines.is_empty()),
                        "{} {:?}",
                        g.name,
                        j
                    );
                }
            }
        }
    }

    #[test]
    fn the_location_picks_the_jurisdiction() {
        let mut d = ProjectDetails::default();
        assert_eq!(Jurisdiction::from_project(&d), Jurisdiction::ModelCodes);
        d.location.state = "CA".into();
        assert_eq!(Jurisdiction::from_project(&d), Jurisdiction::California);
        d.location.state = "NY".into();
        d.location.city = "Brooklyn".into();
        assert_eq!(Jurisdiction::from_project(&d), Jurisdiction::NewYorkCity);
        d.location.city = "Albany".into();
        assert_eq!(Jurisdiction::from_project(&d), Jurisdiction::ModelCodes);
        d.location.state = "Illinois".into();
        d.location.city = "Chicago".into();
        assert_eq!(Jurisdiction::from_project(&d), Jurisdiction::Chicago);
    }
}
