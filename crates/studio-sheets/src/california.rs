//! California's general and energy sheets (ADR-105), as a permit set there is organised:
//! G-002 Project Data & Code Analysis (the 2025 Title 24 codes, zoning required against
//! provided, occupancy and construction, sprinklers, the area tabulation, parking and the
//! site's hazards), G-003 General Notes with abbreviations and symbols, G-004 the CALGreen
//! mandatory measures, G-005 Construction Best Management Practices, the life safety,
//! accessibility and special inspection sheets of buildings under the CBC, and T-001
//! Title 24 energy compliance. Values the model knows are filled in; the rest are bracketed
//! for the architect, and every code reference says to verify the local amendments.

use studio_core::project::ProjectDetails;
use studio_core::units::MM_PER_FT;
use studio_core::{Category, Document, ElementData};
use studio_geom::{Poly, Pt};

use crate::general::{block, numbered, s, GeneralSheet};
use crate::sets::BuildingType;

const SD_ON: &[&str] = &["SD", "DD", "CD", "BN", "CA"];
const DD_ON: &[&str] = &["DD", "CD", "BN", "CA"];
const CD_ON: &[&str] = &["CD", "BN", "CA"];

/// What the general sheets say about the project, worked out once.
pub struct Facts {
    pub name: String,
    pub details: ProjectDetails,
    pub occupancy: String,
    pub construction: String,
    pub sprinklers: String,
    pub stories: usize,
    pub gross_sf: f64,
}

/// The building's areas, from the model: each walled level's gross area (outside face of
/// walls) and its garage, the footprint, the height, and the lot.
pub struct Areas {
    /// (level, gross sf, garage sf).
    pub levels: Vec<(String, f64, f64)>,
    pub footprint_sf: f64,
    pub height_ft: f64,
    pub lot_sf: Option<f64>,
    /// Least distance from the building to each side of the lot: (side, feet).
    pub setbacks: Vec<(String, f64)>,
    /// Each level's outline rings and its garage rooms (parallel to `levels`), the
    /// footprint and the lot, for the diagrams.
    pub outlines: Vec<Vec<Vec<Pt>>>,
    pub garages: Vec<Vec<Vec<Pt>>>,
    pub footprint: Vec<Vec<Pt>>,
    pub lot: Option<Vec<Pt>>,
}

const SF: f64 = MM_PER_FT * MM_PER_FT;

pub fn areas(doc: &Document) -> Areas {
    let model = studio_regen::regenerate(doc);
    let mut levels = vec![];
    let mut outlines: Vec<Poly> = vec![];
    let mut level_rings = vec![];
    let mut garages = vec![];
    let garage_on = |level| {
        model
            .rooms
            .iter()
            .filter(|r| r.level == level && r.name.to_lowercase().contains("garage"))
            .map(|r| r.area())
            .sum::<f64>()
    };
    for l in &model.levels {
        let polys = studio_regen::wall_regions(&model, l.id);
        if polys.is_empty() {
            continue;
        }
        let gross: f64 = polys
            .iter()
            .map(|p| studio_geom::signed_area(&p.outer).abs())
            .sum();
        outlines.extend(polys.iter().map(|p| Poly::simple(p.outer.clone())));
        level_rings.push(polys.iter().map(|p| p.outer.clone()).collect::<Vec<_>>());
        garages.push(
            model
                .rooms
                .iter()
                .filter(|r| r.level == l.id && r.name.to_lowercase().contains("garage"))
                .filter_map(|r| r.boundary.clone())
                .collect::<Vec<_>>(),
        );
        levels.push((l.name.clone(), gross / SF, garage_on(l.id) / SF));
    }
    let footprint = studio_geom::union_all(&outlines);
    let footprint_sf = footprint
        .iter()
        .map(|p| studio_geom::signed_area(&p.outer).abs())
        .sum::<f64>()
        / SF;
    let base = model
        .levels
        .iter()
        .map(|l| l.elevation)
        .fold(f64::INFINITY, f64::min);
    let height_ft = (model.z_range().1 - base.min(0.0)).max(0.0) / MM_PER_FT;
    let lot: Option<Vec<Pt>> = doc.of(Category::Site).find_map(|e| match &e.data {
        ElementData::Site { boundary, .. } if boundary.len() >= 3 => Some(boundary.clone()),
        _ => None,
    });
    let mut setbacks = vec![];
    if let Some(lot) = &lot {
        let pts: Vec<Pt> = footprint.iter().flat_map(|p| p.outer.clone()).collect();
        let ccw = studio_geom::signed_area(lot) > 0.0;
        for i in 0..lot.len() {
            let (a, b) = (lot[i], lot[(i + 1) % lot.len()]);
            let d = b.sub(a).norm();
            // Outward: right of a counter-clockwise ring.
            let out = if ccw {
                Pt::new(d.y, -d.x)
            } else {
                Pt::new(-d.y, d.x)
            };
            let side = if out.x.abs() > out.y.abs() {
                if out.x > 0.0 {
                    "East"
                } else {
                    "West"
                }
            } else if out.y > 0.0 {
                "North"
            } else {
                "South"
            };
            let least = pts
                .iter()
                .map(|p| a.sub(*p).dot(out).abs())
                .fold(f64::INFINITY, f64::min);
            if least.is_finite() {
                setbacks.push((side.to_string(), least / MM_PER_FT));
            }
        }
    }
    Areas {
        levels,
        footprint_sf,
        height_ft,
        lot_sf: lot.as_ref().map(|l| studio_geom::signed_area(l).abs() / SF),
        setbacks,
        outlines: level_rings,
        garages,
        footprint: footprint.iter().map(|p| p.outer.clone()).collect(),
        lot,
    }
}

/// Feet as feet and inches to the nearest inch: 20.5 → 20'-6".
pub fn feet(ft: f64) -> String {
    let inches = (ft * 12.0).round() as i64;
    format!("{}'-{}\"", inches / 12, inches % 12)
}

/// A whole number with thousands separators: 16500.0 → "16,500".
pub fn thousands(v: f64) -> String {
    let n = v.round() as i64;
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// A value Project Info's code notes give as "Key: value" lines, e.g. "Flood zone: X".
pub fn noted(d: &ProjectDetails, key: &str) -> Option<String> {
    let key = key.to_lowercase();
    d.codes.notes.lines().chain(d.notes.lines()).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim().to_lowercase() == key && !v.trim().is_empty()).then(|| v.trim().to_string())
    })
}

fn or(v: &str, dflt: &str) -> String {
    if v.trim().is_empty() {
        dflt.to_string()
    } else {
        v.trim().to_string()
    }
}

/// The 2025 California Building Standards Code, as a project's code list gives it.
pub fn codes(residential: bool, city: &str) -> Vec<String> {
    let mut v = s(&[
        "2025 California Building Standards Code, Title 24, California Code of Regulations, effective January 1, 2026 (based on the 2024 International Codes):",
    ]);
    if residential {
        v.push("  Part 2.5 — 2025 California Residential Code (CRC)".into());
        v.push("  Part 2 — 2025 California Building Code (CBC), where the CRC refers to it".into());
    } else {
        v.push("  Part 2 — 2025 California Building Code (CBC), Volumes 1 and 2".into());
    }
    v.extend(s(&[
        "  Part 3 — 2025 California Electrical Code (CEC)",
        "  Part 4 — 2025 California Mechanical Code (CMC)",
        "  Part 5 — 2025 California Plumbing Code (CPC)",
        "  Part 6 — 2025 California Energy Code",
        "  Part 9 — 2025 California Fire Code (CFC)",
        "  Part 10 — 2025 California Existing Building Code (CEBC), where applicable",
        "  Part 11 — 2025 California Green Building Standards Code (CALGreen)",
        "  Part 12 — 2025 California Referenced Standards Code",
    ]));
    v.push(format!(
        "Local amendments: {} Municipal Code and fire code amendments.",
        or(city, "the city's")
    ));
    v.push(
        "Verify the code editions in force with the building department at permit application."
            .into(),
    );
    v
}

/// The California sheets, from the general sheets made for California (`generic`): their
/// code analysis, notes, abbreviations, life safety, accessibility and special inspection
/// blocks are kept, organised as a California set is.
pub fn sheets(
    doc: &Document,
    t: BuildingType,
    f: &Facts,
    generic: Vec<GeneralSheet>,
) -> Vec<GeneralSheet> {
    let irc = t.irc();
    let d = &f.details;
    let a = areas(doc);
    let mut out: Vec<GeneralSheet> = vec![];
    let take = |n: usize| generic.iter().find(|g| g.number == n);
    let unknown = |key: &str, what: &str| noted(d, key).unwrap_or_else(|| format!("[{what}]"));

    // ------------------------------------------------ G-002 Project Data & Code Analysis
    let city = d.location.city.trim().to_string();
    let lot_sf = a.lot_sf.or_else(|| {
        d.codes
            .lot_area
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>()
            .parse::<f64>()
            .ok()
    });
    let pct = |v: f64| lot_sf.map_or("[%]".to_string(), |l| format!("{:.1}%", v / l * 100.0));
    let mut zoning = vec![
        format!(
            "ZONING DISTRICT: {}",
            or(&d.codes.zoning_district, "[zoning district]")
        ),
        format!("GENERAL PLAN DESIGNATION: {}", unknown("general plan", "general plan land use")),
        format!("OVERLAY DISTRICTS: {}", unknown("overlay", "none, or the overlays")),
        format!(
            "LOT AREA: {}",
            match lot_sf {
                Some(l) => format!("{} sf ({:.2} acres)", thousands(l), l / 43_560.0),
                None => or(&d.codes.lot_area, "[sf]"),
            }
        ),
        format!(
            "FLOOR AREA RATIO: allowed {} — proposed {} sf, {}",
            or(&d.codes.far, "[per zoning]"),
            thousands(f.gross_sf),
            pct(f.gross_sf)
        ),
        format!(
            "LOT COVERAGE: allowed {} — proposed {} sf, {}",
            or(&d.codes.lot_coverage, "[per zoning]"),
            thousands(a.footprint_sf),
            pct(a.footprint_sf)
        ),
        format!(
            "BUILDING HEIGHT: allowed {} — proposed {} to the highest point of the roof above finish floor",
            or(&d.codes.max_height, "[per zoning]"),
            feet(a.height_ft)
        ),
        format!("STORIES: {} above grade, no basement", f.stories),
        format!(
            "SETBACKS REQUIRED: {}",
            or(&d.codes.setbacks, "[front / sides / rear per zoning]")
        ),
    ];
    if !a.setbacks.is_empty() {
        zoning.push(format!(
            "SETBACKS PROVIDED (to the nearest wall): {}",
            a.setbacks
                .iter()
                .map(|(side, ft)| format!("{side} {}", feet(*ft)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut building = vec![
        format!(
            "OCCUPANCY: {}",
            if irc {
                "R-3 dwelling and U private garage (CRC)".to_string()
            } else {
                f.occupancy.clone()
            }
        ),
        format!("CONSTRUCTION TYPE: {}", f.construction),
        format!(
            "FIRE SPRINKLERS: {}",
            if irc {
                "Required in new one- and two-family dwellings, CRC R313.2: NFPA 13D or CRC R313.3 system throughout (deferred submittal).".to_string()
            } else {
                format!(
                    "{} (CBC 903.2.8 requires sprinklers throughout Group R).",
                    f.sprinklers
                )
            }
        ),
        format!("STORIES ABOVE GRADE: {}", f.stories),
    ];
    if irc {
        building.push("FIRE SEPARATION: dwelling / garage per CRC R302.6, 5/8\" Type X gypsum board at the garage ceiling below habitable rooms; self-closing, 20-minute or solid-core door per R302.5.1.".into());
    }
    let mut tab: Vec<String> = vec![];
    let (mut conditioned, mut garage) = (0.0, 0.0);
    for (name, gross, gar) in &a.levels {
        let c = (gross - gar).max(0.0);
        conditioned += c;
        garage += gar;
        tab.push(format!(
            "{}: {} sf conditioned{}",
            name.to_uppercase(),
            thousands(c),
            if *gar > 0.0 {
                format!(", {} sf garage", thousands(*gar))
            } else {
                String::new()
            }
        ));
    }
    tab.push(format!("TOTAL CONDITIONED: {} sf", thousands(conditioned)));
    if garage > 0.0 {
        tab.push(format!("GARAGE (U): {} sf", thousands(garage)));
    }
    tab.push(format!(
        "DECKS, BALCONIES AND COVERED PORCHES: {}",
        unknown("decks", "none, or sf")
    ));
    tab.push(format!(
        "ACCESSORY DWELLING UNIT: {}",
        unknown("adu", "none proposed")
    ));
    tab.push(format!(
        "GROSS FLOOR AREA (outside face of walls): {} sf",
        thousands(f.gross_sf)
    ));
    let spaces = (garage / 200.0).floor() as i64;
    let parking = vec![
        format!(
            "REQUIRED: {}",
            or(
                &d.codes.parking,
                if irc {
                    "2 spaces, 1 covered (verify per zoning)"
                } else {
                    "[per zoning]"
                }
            )
        ),
        format!(
            "PROVIDED: {}",
            if spaces > 0 {
                format!("{spaces} covered in the garage")
            } else {
                "[spaces]".into()
            }
        ),
        if irc {
            "EV: charging provisions in the garage per CALGreen 4.106.4 and the local reach code (see G-004).".into()
        } else {
            "EV: EV capable, EV ready and EVSE spaces per CALGreen 4.106.4 / 5.106.5.3 (see G-004)."
                .into()
        },
    ];
    let hazards = vec![
        format!(
            "FIRE HAZARD SEVERITY ZONE: {}",
            unknown("fire hazard", "verify on the CAL FIRE FHSZ map; in a FHSZ or WUI area, materials per CRC R337 / CBC Chapter 7A and defensible space per PRC 4291")
        ),
        format!(
            "FLOOD ZONE: {}",
            unknown("flood zone", "FEMA FIRM panel and zone")
        ),
        format!(
            "SOILS: {}",
            unknown("soils", "geotechnical report by [engineer], dated [date]")
        ),
        format!(
            "SEISMIC DESIGN CATEGORY: {}",
            unknown("seismic", "D, per the structural drawings")
        ),
        format!(
            "CLIMATE ZONE: {}",
            unknown("climate zone", "per the Title 24 Part 6 climate zone map")
        ),
    ];
    let mut blocks = vec![
        block("APPLICABLE CODES", codes(irc, &city)),
        block("ZONING AND PLANNING", zoning),
        block("BUILDING DATA", building),
        block("AREA TABULATION", tab),
        block("PARKING", parking),
        block("SITE HAZARDS AND CONDITIONS", hazards),
    ];
    // Area diagrams (ADR-105): each level's outline at one scale, the garage dashed, and
    // the footprint on the lot for coverage.
    let all: Vec<Pt> = a.outlines.iter().flatten().flatten().copied().collect();
    if let Some(frame) = studio_geom::bounds_of(&all) {
        for (i, (name, gross, gar)) in a.levels.iter().enumerate() {
            let mut rings: Vec<(Vec<Pt>, bool)> =
                a.outlines[i].iter().map(|r| (r.clone(), false)).collect();
            rings.extend(a.garages[i].iter().map(|r| (r.clone(), true)));
            let mut lines = vec![format!("CONDITIONED: {} sf", thousands(gross - gar))];
            if *gar > 0.0 {
                lines.push(format!("GARAGE (DASHED): {} sf", thousands(*gar)));
            }
            lines.push(format!("GROSS: {} sf", thousands(*gross)));
            blocks.push(crate::general::Block {
                figure: Some(crate::general::Figure { rings, frame }),
                ..block(
                    &format!("FLOOR AREA DIAGRAM — {}", name.to_uppercase()),
                    lines,
                )
            });
        }
    }
    if let Some(lot) = &a.lot {
        let mut rings: Vec<(Vec<Pt>, bool)> = vec![(lot.clone(), true)];
        rings.extend(a.footprint.iter().map(|r| (r.clone(), false)));
        let frame = studio_geom::bounds_of(lot).unwrap_or_default();
        blocks.push(crate::general::Block {
            figure: Some(crate::general::Figure { rings, frame }),
            ..block(
                "LOT COVERAGE DIAGRAM",
                vec![
                    format!(
                        "LOT (DASHED): {} sf",
                        lot_sf.map_or("[ ]".into(), thousands)
                    ),
                    format!(
                        "FOOTPRINT: {} sf, {}",
                        thousands(a.footprint_sf),
                        pct(a.footprint_sf)
                    ),
                ],
            )
        });
    }
    if !irc {
        if let Some(g) = take(5) {
            blocks.extend(g.blocks.iter().skip(1).cloned());
        }
    }
    out.push(GeneralSheet {
        number: 2,
        discipline: "G",
        name: "Project Data & Code Analysis".into(),
        blocks,
        phases: SD_ON,
    });

    // ------------------------------------------------ G-003 General Notes, Abbreviations
    let mut blocks = vec![];
    if let Some(g) = take(3) {
        blocks.extend(g.blocks.iter().cloned());
    }
    blocks.push(numbered(
        "LIFE SAFETY AND FIRE DEPARTMENT NOTES",
        if irc {
            s(&[
                "Smoke alarms in each sleeping room, outside each sleeping area and on every story, hard-wired, interconnected, with battery backup, CRC R314.",
                "Carbon monoxide alarms outside each sleeping area and on every story, CRC R315; combination smoke / CO alarms permitted.",
                "Emergency escape and rescue openings in every sleeping room: 5.7 sf net clear (5.0 sf at grade floor), 24\" min. clear height, 20\" min. clear width, sill 44\" max. above the floor, CRC R310.",
                "Address numbers 4\" high min., 1/2\" stroke, contrasting with their background and visible from the street, illuminated at night, CRC R319 and the fire code.",
                "Fire department access road and hydrant within the distances of CFC 503 and 507 (Appendices B and C); Knox box where the fire department requires it.",
                "Automatic fire sprinklers throughout per CRC R313 (deferred submittal); the water service and meter sized for the sprinkler demand.",
            ])
        } else {
            s(&[
                "Smoke alarms and detection per CBC 907.2.11 and the fire alarm system per CBC 907 (deferred submittal).",
                "Carbon monoxide detection per CBC 915.",
                "Fire department access, hydrants and fire flow per CFC 503, 507 and Appendices B and C; Knox box per CFC 506.",
                "Address identification per CFC 505.",
                "Portable fire extinguishers per CFC 906.",
            ])
        },
    ));
    blocks.push(numbered(
        "CONSTRUCTION AND INSPECTION NOTES",
        s(&[
            "Special inspections, where the structural drawings require them, per CBC Chapter 17 and the statement of special inspections on the S-series sheets.",
            "Call USA North 811 at least two working days before any excavation, Government Code 4216.",
            "Construction hours and noise per the municipal code; protect public improvements, and repair any damage to them before final.",
            "Encroachment permit from the public works department for work in the public right-of-way.",
            "The contractor shall keep a set of approved drawings, the permit card and the CALGreen and energy documentation on site.",
        ]),
    ));
    if let Some(g) = take(4) {
        blocks.extend(g.blocks.iter().cloned());
    }
    out.push(GeneralSheet {
        number: 3,
        discipline: "G",
        name: "General Notes, Abbreviations & Symbols".into(),
        blocks,
        phases: DD_ON,
    });

    // ------------------------------------------------ G-004 CALGreen
    let calgreen = if irc
        || matches!(
            t,
            BuildingType::GardenApartments
                | BuildingType::MidRiseApartments
                | BuildingType::MixedUse
        ) {
        vec![
            numbered("PLANNING AND DESIGN (DIVISION 4.1)", s(&[
                "4.106.2 Storm water drainage and retention during construction: retain storm water on site, or use the BMPs on G-005, for projects disturbing less than one acre.",
                "4.106.3 Grading and paving: divert surface water from the building with swales, drains or water retention gardens.",
                "4.106.4 Electric vehicle charging: one- and two-family dwellings and townhouses with an attached garage get a listed raceway, panel capacity and a breaker space for a dedicated 208/240 V, 40 A branch circuit to the garage (more where the city's reach code asks, e.g. EV Ready); multifamily parking EV capable, EV ready and EVSE per 4.106.4.2.",
            ])),
            numbered("WATER EFFICIENCY (DIVISION 4.3)", s(&[
                "4.303.1 Plumbing fixtures: water closets 1.28 gpf max.; showerheads 1.8 gpm at 80 psi; lavatory faucets 1.2 gpm at 60 psi; kitchen faucets 1.8 gpm at 60 psi (2.2 gpm temporary); all WaterSense where listed.",
                "4.303.2 Standards for plumbing fixtures and fittings per the California Plumbing Code.",
                "4.304.1 Outdoor potable water use: weather- or soil-moisture-based irrigation controllers; landscapes of 500 sf or more comply with the MWELO or the local ordinance.",
            ])),
            numbered("MATERIAL CONSERVATION (DIVISION 4.4)", s(&[
                "4.406.1 Rodent proofing: annular spaces round pipes, cables and conduits through exterior walls closed with cement mortar or a similar method.",
                "4.408.1 Construction waste management: recycle or salvage at least 65% of nonhazardous construction and demolition waste.",
                "4.408.2 Construction waste management plan, or 4.408.4 a waste management company; documentation to the enforcing agency per 4.408.5.",
                "4.410.1 Operation and maintenance manual given to the building occupant or owner at final.",
            ])),
            numbered("ENVIRONMENTAL QUALITY (DIVISION 4.5)", s(&[
                "4.503.1 Fireplaces: direct-vent sealed-combustion gas, or EPA Phase II wood stoves; no open wood-burning fireplaces where local rules forbid them.",
                "4.504.1 Duct openings and mechanical equipment covered during construction.",
                "4.504.2 Adhesives, sealants, caulks, paints and coatings within the VOC limits of Tables 4.504.1–4.504.3; aerosol paints per 4.504.2.3; verification per 4.504.2.4.",
                "4.504.3 Carpet systems: CRI Green Label Plus or equal; 4.504.4 resilient flooring: 80% FloorScore or equal; 4.504.5 composite wood: CARB / TSCA Title VI formaldehyde limits.",
                "4.505.2 Concrete slab foundations: vapor retarder and 4\" min. capillary break of 1/2\" or larger clean aggregate.",
                "4.505.3 Moisture content of building materials: framing 19% max. before enclosure, verified and recorded.",
                "4.506.1 Bathroom exhaust fans: ENERGY STAR, ducted to the outside, humidistat-controlled (unless part of whole-house ventilation).",
                "4.507.2 Heating and air conditioning systems sized and designed per ANSI/ACCA Manual J, D and S.",
            ])),
            numbered("INSTALLER AND INSPECTOR QUALIFICATIONS (CHAPTER 7)", s(&[
                "702.1 HVAC installers trained and certified in their work; 702.2 special inspectors qualified for green building inspection.",
                "703.1 Verification: documentation of compliance (CALGreen checklist, waste diversion, VOC data sheets, moisture readings) to the enforcing agency before final.",
                "Tier 1 and Tier 2 voluntary measures (Appendix A4) apply only where the city adopts them; list any adopted here.",
            ])),
        ]
    } else {
        vec![numbered("CALGREEN NONRESIDENTIAL MANDATORY MEASURES (CHAPTER 5)", s(&[
            "5.106.1 Storm water pollution prevention for projects disturbing less than one acre (see G-005).",
            "5.106.4 Bicycle parking: short- and long-term per the tables.",
            "5.106.5.2 Designated parking for clean air vehicles; 5.106.5.3 EV capable spaces and EVSE per Table 5.106.5.3.1.",
            "5.106.8 Light pollution reduction: backlight, uplight and glare ratings per Table 5.106.8.",
            "5.303.1 Meters for each tenant over 50,000 sf or water use over 1,000 gal/day; 5.303.3 water-conserving fixtures.",
            "5.304.1 Outdoor water use: MWELO or the local ordinance.",
            "5.408.1 Construction waste: 65% diversion with a waste management plan; 5.408.3 excavated soil and land-clearing debris 100% reused or recycled.",
            "5.410.2 Commissioning of new buildings of 10,000 sf or more; 5.410.4 testing and adjusting below that.",
            "5.504.1 Temporary ventilation during construction with MERV 8 filters; 5.504.5.3 MERV 13 filters in occupied spaces.",
            "5.504.4 Finish materials within the VOC and formaldehyde limits of the tables.",
            "5.505.1 Indoor moisture control; 5.506.1 outdoor air delivery per the California Mechanical Code.",
        ]))]
    };
    out.push(GeneralSheet {
        number: 4,
        discipline: "G",
        name: "CALGreen Mandatory Measures".into(),
        blocks: calgreen,
        phases: CD_ON,
    });

    // ------------------------------------------------ G-005 Construction BMPs
    out.push(GeneralSheet {
        number: 5,
        discipline: "G",
        name: "Construction Best Management Practices".into(),
        blocks: vec![
            numbered("CONSTRUCTION BEST MANAGEMENT PRACTICES (BMPs)", s(&[
                "Include the city's standard Construction BMPs sheet where the city requires it, in its current version; it governs where it differs from these notes.",
                "No discharge of sediment, concrete, paint, fuel or wash water to the street, gutter or storm drain. Storm drains are for rain only.",
                "Stabilized construction entrance of crushed rock; sweep the street and sidewalk daily, never wash them into the gutter.",
                "Perimeter sediment control (fiber rolls or silt fence) on the downslope sides of disturbed areas; storm drain inlet protection at inlets near the work.",
                "Cover stockpiles of soil, sand and gravel with plastic sheeting when not in use and before rain; protect bare soil with mulch or erosion control blankets October 1 – April 30.",
                "Concrete, stucco and paint washout in a lined, contained washout area at least 50' from storm drains; dispose of the residue as solid waste.",
                "Store materials, fuels and chemicals covered and in secondary containment; keep spill kits on site and clean spills immediately.",
                "Trash and construction waste in covered bins emptied before overflowing; no hosing of dumpsters.",
                "Vehicle and equipment maintenance and fueling off site, or on a designated, contained area with drip pans.",
                "Dust control by water spray or other means; no visible dust leaving the site.",
                "Dewatering discharges only with the city's approval and treatment as it requires.",
            ])),
            numbered("POST-CONSTRUCTION STORMWATER", s(&[
                "Projects that create or replace 2,500 sf or more of impervious surface include site design measures per the MS4 permit (in the Bay Area, MRP Provision C.3.i): downspouts to landscaping or rain barrels, runoff from paving to landscaping, permeable paving where feasible.",
                "Larger projects (10,000 sf or more of impervious surface, 5,000 sf for certain uses) are regulated projects needing a stormwater control plan and treatment measures; verify the thresholds with the city.",
                "Show the impervious surface tabulation (existing, created, replaced) on the site plan.",
            ])),
        ],
        phases: CD_ON,
    });

    // ------------------------------------------------ CBC buildings: life safety,
    // accessibility, special inspections.
    if !irc {
        for (from, to) in [(6, 6), (7, 7), (10, 8)] {
            if let Some(g) = take(from) {
                out.push(GeneralSheet {
                    number: to,
                    discipline: "G",
                    name: g.name.clone(),
                    blocks: g.blocks.clone(),
                    phases: g.phases,
                });
            }
        }
    }

    // ------------------------------------------------ T-001 Title 24 Energy
    let energy = if irc || matches!(t, BuildingType::GardenApartments | BuildingType::Townhouses) {
        vec![
            numbered("TITLE 24, PART 6 COMPLIANCE", s(&[
                "2025 California Energy Code, residential performance compliance: the registered CF1R-PRF-01, signed by the documentation author and the responsible designer, is printed on the T-sheets and is part of these drawings.",
                "Build to the CF1R: insulation, fenestration U-factor and SHGC, HVAC and water heating equipment, and every HERS feature it lists. A change to any of them needs a revised CF1R.",
                "Installation certificates (CF2R) by the installers and HERS verification certificates (CF3R) by the HERS rater, registered with a data registry before final inspection.",
            ])),
            numbered("MANDATORY MEASURES", s(&[
                "§150.0(a)–(d) Mandatory minimum insulation in ceilings and roofs, walls and raised floors, or better where the CF1R lists it; Quality Insulation Installation where the CF1R takes credit for QII.",
                "§150.0(i)–(j) Thermostats setback; pipe insulation on hot water and refrigerant lines.",
                "§150.0(k) Lighting: all luminaires high efficacy; vacancy sensors in baths, garages, laundry and utility rooms; dimmers or vacancy sensors elsewhere; exterior lighting on photocontrol and motion sensor or timer.",
                "§150.0(m) Ducts sealed and tested; §150.0(o) whole-dwelling mechanical ventilation and a rated kitchen range hood.",
                "§110.10 Solar ready provisions where no PV is required.",
            ])),
            numbered("ELECTRIFICATION, PV AND STORAGE", s(&[
                "§150.1(c)14 Solar photovoltaic system sized per the CF1R (kWdc), with the PV array location and inverter on the roof plan; PV plans as a deferred submittal where the city allows.",
                "§150.0(s) Energy storage ready: a battery-ready main panel or subpanel with space for a two-pole breaker, and a raceway to the battery location; battery storage where the CF1R lists it.",
                "§150.0(n), (t), (u), (v) Heat pump and electric ready: dedicated 240 V circuits and space at the water heater, furnace, range and dryer locations wherever a gas appliance is installed.",
                "List the space heating, cooling and water heating equipment and their efficiencies (HSPF2 / SEER2 / UEF) from the CF1R.",
            ])),
            numbered("HERS VERIFICATION", s(&[
                "Duct leakage test, airflow and fan efficacy (W/cfm), refrigerant charge verification, whole-dwelling ventilation airflow and kitchen hood rating, as the CF1R lists.",
                "Quality Insulation Installation (QII) and building air leakage, where the CF1R takes credit for them.",
            ])),
            block("ENVELOPE SUMMARY (FROM THE CF1R)", s(&[
                "Climate zone: [ ]",
                "Roof / ceiling: R-[ ] below deck / R-[ ] ceiling",
                "Above-grade walls: R-[ ] cavity + R-[ ] continuous",
                "Slab edge: [ ]; raised floor: R-[ ]",
                "Fenestration: U-[ ], SHGC [ ]",
                "Space heating / cooling: [heat pump, HSPF2 / SEER2]",
                "Water heating: [heat pump water heater, UEF]",
                "PV: [ ] kWdc; battery: [ ] kWh",
            ])),
        ]
    } else {
        vec![
            numbered("TITLE 24, PART 6 COMPLIANCE", s(&[
                "2025 California Energy Code, nonresidential and multifamily compliance: the NRCC envelope (ENV), mechanical (MCH), lighting (LTI / LTO) and solar (PVB) forms are printed on the T-sheets and are part of these drawings.",
                "NRCI installation certificates by the installers; NRCA acceptance tests by a certified acceptance test technician before final.",
                "§140.10 Solar PV and battery storage where Tables 140.10-A and -B require them.",
                "§110.10 Solar ready areas; §110.12 demand responsive controls; §120.1 ventilation; §130 lighting controls.",
            ])),
            block("ENVELOPE SUMMARY (FROM THE NRCC)", s(&[
                "Climate zone: [ ]",
                "Roof: U-[ ]; walls: U-[ ]; floors: U-[ ]",
                "Fenestration: U-[ ], RSHGC [ ], VT [ ]",
                "Lighting power density: [ ] W/sf",
            ])),
        ]
    };
    out.push(GeneralSheet {
        number: 1,
        discipline: "T",
        name: "Title 24 Energy Compliance".into(),
        blocks: energy,
        phases: CD_ON,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_as_drawings_write_them() {
        assert_eq!(feet(20.5), "20'-6\"");
        assert_eq!(feet(0.0), "0'-0\"");
        assert_eq!(thousands(16500.0), "16,500");
        assert_eq!(thousands(950.4), "950");
        assert_eq!(thousands(1234567.0), "1,234,567");
    }

    #[test]
    fn code_notes_fill_the_bracketed_values() {
        let mut d = ProjectDetails::default();
        d.codes.notes = "Flood zone: X (FIRM 06085C0020H)\nClimate zone: 4".into();
        assert_eq!(
            noted(&d, "Flood zone").as_deref(),
            Some("X (FIRM 06085C0020H)")
        );
        assert_eq!(noted(&d, "climate zone").as_deref(), Some("4"));
        assert_eq!(noted(&d, "seismic"), None);
        let c = codes(true, "Palo Alto");
        assert!(c[0].contains("2025") && c[0].contains("January 1, 2026"));
        assert!(c.iter().any(|l| l.contains("California Residential Code")));
        assert!(c.iter().any(|l| l.contains("Palo Alto Municipal Code")));
    }
}
