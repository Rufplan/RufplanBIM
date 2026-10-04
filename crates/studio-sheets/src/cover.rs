//! The cover sheet (ADR-105), laid out as US permit sets lay theirs out (after the
//! Millbrae, California plan template and its peers): the project title across the top; the
//! rendering, the project description and scope with the vicinity map beside it, and the
//! deferred submittals down the left; the project directory, project data and applicable
//! codes in the middle; the sheet index, the architect's stamp and the agency's approval box
//! on the right, each in a ruled, titled box.

use studio_core::details::FillPattern;
use studio_core::lines::LineStyle;
use studio_core::maps::MapKind;
use studio_core::text::{TextAlign, LINE};
use studio_core::Document;
use studio_geom::Pt;

use crate::california::{areas, feet, noted, thousands};
use crate::general::Jurisdiction;
use crate::sets::BuildingType;

/// What the cover says, from Project Info and the model.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverData {
    pub title: String,
    /// "NEW SINGLE-FAMILY RESIDENCE".
    pub kind: String,
    pub address: String,
    pub description: String,
    pub scope: Vec<String>,
    /// (role, its lines).
    pub directory: Vec<(String, Vec<String>)>,
    pub data: Vec<(String, String)>,
    pub codes: Vec<String>,
    pub deferred: Vec<String>,
    /// Work permitted separately (fire sprinklers, PV, encroachment…).
    pub permits: Vec<String>,
    /// The street the site is on, for the vicinity map.
    pub street: String,
}

fn or(v: &str, dflt: &str) -> String {
    if v.trim().is_empty() {
        dflt.to_string()
    } else {
        v.trim().to_string()
    }
}

/// "2150 Oak Knoll Lane" → "OAK KNOLL LANE".
fn street_name(street: &str) -> String {
    let s = street.trim();
    let rest = match s.split_once(' ') {
        Some((n, r)) if n.chars().all(|c| c.is_ascii_digit() || c == '-') => r,
        _ => s,
    };
    rest.to_uppercase()
}

pub fn cover_data(doc: &Document, t: BuildingType, j: Jurisdiction) -> CoverData {
    let (ident, d) =
        studio_core::project::get(doc).unwrap_or_else(|_| (Default::default(), Default::default()));
    let irc = t.irc();
    let ca = j == Jurisdiction::California;
    let a = areas(doc);
    let gross: f64 = a.levels.iter().map(|l| l.1).sum();
    let garage: f64 = a.levels.iter().map(|l| l.2).sum();
    let stories = a.levels.len().max(1);
    let l = &d.location;
    let city_line = [l.city.trim(), l.state.trim()]
        .iter()
        .filter(|x| !x.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let city_line = [city_line.as_str(), l.zip.trim()]
        .iter()
        .filter(|x| !x.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let address = [l.street.trim(), city_line.as_str()]
        .iter()
        .filter(|x| !x.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let address = or(
        &address,
        &or(&ident.address, "[Street address, city, state, zip]"),
    );
    let kind = match t {
        BuildingType::SingleFamily => "NEW SINGLE-FAMILY RESIDENCE",
        BuildingType::Duplex => "NEW TWO-FAMILY DWELLING",
        BuildingType::Townhouses => "NEW TOWNHOUSES",
        BuildingType::GardenApartments => "NEW MULTIFAMILY RESIDENTIAL",
        BuildingType::MidRiseApartments => "NEW MID-RISE MULTIFAMILY RESIDENTIAL",
        BuildingType::MixedUse => "NEW MIXED-USE BUILDING",
        BuildingType::Hotel => "NEW HOTEL",
    };
    let stories_word = match stories {
        1 => "one-story",
        2 => "two-story",
        3 => "three-story",
        4 => "four-story",
        _ => "multistory",
    };
    let description = or(
        &d.overview.description,
        &format!(
            "A new {stories_word} {} of {} sf{}{}.",
            t.label().to_lowercase(),
            thousands(gross - garage),
            if garage > 0.0 {
                format!(" with an attached {} sf garage", thousands(garage))
            } else {
                String::new()
            },
            a.lot_sf
                .map(|s| format!(", on a {} sf lot", thousands(s)))
                .unwrap_or_default()
        ),
    );
    let mut scope = vec![
        format!(
            "New construction: {} sf gross over {stories} {}.",
            thousands(gross),
            if stories == 1 { "story" } else { "stories" }
        ),
        "Architectural, structural, mechanical, plumbing and electrical work as shown.".to_string(),
        "Site work: grading and drainage, utilities, hardscape, landscaping and irrigation.".into(),
    ];
    if ca {
        scope.push("Solar photovoltaic system and battery-ready, heat-pump-ready and EV provisions per Title 24.".into());
    }
    scope.push("Demolition: none (vacant lot), unless noted.".into());

    // The directory: owner, the team, then the roles a permit set lists.
    let mut directory: Vec<(String, Vec<String>)> = vec![(
        "OWNER".into(),
        vec![or(&d.client.company, &or(&ident.client, "[Owner]"))]
            .into_iter()
            .chain(
                [
                    d.client.name.as_str(),
                    d.client.phone.as_str(),
                    d.client.email.as_str(),
                ]
                .iter()
                .filter(|x| !x.trim().is_empty())
                .map(|x| x.to_string()),
            )
            .collect(),
    )];
    for m in &d.team {
        let c = &m.contact;
        let mut lines: Vec<String> = [c.company.as_str(), c.name.as_str()]
            .iter()
            .filter(|x| !x.trim().is_empty())
            .map(|x| x.to_string())
            .collect();
        let reach = [c.phone.as_str(), c.email.as_str()]
            .iter()
            .filter(|x| !x.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        if !reach.is_empty() {
            lines.push(reach);
        }
        if lines.is_empty() {
            lines.push("[firm, contact, phone]".into());
        }
        directory.push((m.discipline.to_uppercase(), lines));
    }
    let roles: &[&str] = if ca {
        &[
            "ARCHITECT",
            "STRUCTURAL ENGINEER",
            "CIVIL ENGINEER",
            "MEP ENGINEER",
            "LANDSCAPE ARCHITECT",
            "TITLE 24 CONSULTANT",
            "SURVEYOR",
            "GEOTECHNICAL ENGINEER",
        ]
    } else {
        &[
            "ARCHITECT",
            "STRUCTURAL ENGINEER",
            "CIVIL ENGINEER",
            "MEP ENGINEER",
        ]
    };
    for role in roles {
        let key = role.split(' ').next().unwrap_or(role);
        let mep = *role == "MEP ENGINEER"
            && directory
                .iter()
                .any(|(r, _)| r.starts_with("MECHANICAL") || r.starts_with("ELECTRICAL"));
        if !mep && !directory.iter().any(|(r, _)| r.starts_with(key)) {
            directory.push((role.to_string(), vec!["[firm, contact, phone]".into()]));
        }
    }

    let (occ, ct, _) = crate::general::occupancy(t);
    let occ = if irc && ca {
        "R-3 / U (CRC)".to_string()
    } else {
        or(&d.codes.occupancy, occ)
    };
    let ct = or(&d.codes.construction_type, ct);
    let pct = |v: f64| {
        a.lot_sf
            .map(|s| format!(" ({:.1}%)", v / s * 100.0))
            .unwrap_or_default()
    };
    let mut data: Vec<(String, String)> = vec![
        ("APN".into(), or(&l.apn, "[Assessor's parcel number]")),
        (
            "ZONING".into(),
            or(&d.codes.zoning_district, "[Zoning district]"),
        ),
        ("OCCUPANCY".into(), occ),
        ("CONSTRUCTION TYPE".into(), ct),
        (
            "FIRE SPRINKLERS".into(),
            if irc {
                if ca {
                    "Yes, NFPA 13D (CRC R313), deferred submittal".into()
                } else {
                    or(
                        &d.codes.sprinklered,
                        "Per local amendment (NFPA 13D where provided)",
                    )
                }
            } else {
                or(&d.codes.sprinklered, "NFPA 13 / 13R throughout")
            },
        ),
        (
            "STORIES / HEIGHT".into(),
            format!(
                "{stories} / {} (allowed {})",
                feet(a.height_ft),
                or(&d.codes.max_height, "[ ]")
            ),
        ),
        (
            "LOT AREA".into(),
            a.lot_sf
                .map(|s| format!("{} sf", thousands(s)))
                .unwrap_or_else(|| or(&d.codes.lot_area, "[sf]")),
        ),
    ];
    for (level, g, gar) in &a.levels {
        let c = g - gar;
        data.push((
            format!("{} AREA", level.to_uppercase()),
            if *gar > 0.0 {
                format!("{} sf + {} sf garage", thousands(c), thousands(*gar))
            } else {
                format!("{} sf", thousands(c))
            },
        ));
    }
    data.push((
        "GROSS FLOOR AREA".into(),
        format!(
            "{} sf{}",
            thousands(gross),
            pct(gross).replace(')', " FAR)")
        ),
    ));
    if !d.codes.far.trim().is_empty() {
        data.push(("FAR ALLOWED".into(), d.codes.far.trim().into()));
    }
    data.push((
        "LOT COVERAGE".into(),
        format!(
            "{} sf{}{}",
            thousands(a.footprint_sf),
            pct(a.footprint_sf),
            if d.codes.lot_coverage.trim().is_empty() {
                String::new()
            } else {
                format!(", allowed {}", d.codes.lot_coverage.trim())
            }
        ),
    ));
    if !d.codes.setbacks.trim().is_empty() {
        data.push(("SETBACKS".into(), d.codes.setbacks.trim().into()));
    }
    data.push((
        "PARKING".into(),
        or(
            &d.codes.parking,
            if irc {
                "2 covered spaces in the garage"
            } else {
                "[required / provided]"
            },
        ),
    ));
    for (key, label) in [
        ("climate zone", "CLIMATE ZONE"),
        ("fire hazard", "FIRE HAZARD ZONE"),
        ("flood zone", "FLOOD ZONE"),
    ] {
        if let Some(v) = noted(&d, key) {
            data.push((label.into(), v));
        }
    }

    let codes: Vec<String> = if ca {
        let city = or(&l.city, "City");
        let mut v: Vec<String> = [
            if irc {
                "2025 California Residential Code (CRC)"
            } else {
                "2025 California Building Code (CBC)"
            },
            if irc {
                "2025 California Building Code (CBC), as referenced"
            } else {
                "2025 California Existing Building Code, where applicable"
            },
            "2025 California Electrical Code (CEC)",
            "2025 California Mechanical Code (CMC)",
            "2025 California Plumbing Code (CPC)",
            "2025 California Energy Code",
            "2025 California Fire Code (CFC)",
            "2025 California Green Building Standards Code (CALGreen)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        v.push(format!("{city} Municipal Code, with its local amendments"));
        v.push("Title 24, CCR, effective January 1, 2026".into());
        v
    } else {
        crate::general::codes(t, j)
            .into_iter()
            .filter(|c| !c.ends_with(':') && !c.starts_with("Verify"))
            .map(|c| c.trim().to_string())
            .collect()
    };

    let mut deferred = vec![if irc {
        "Fire sprinkler system: drawings and calculations by a licensed C-16 contractor".to_string()
    } else {
        "Fire sprinkler and fire alarm systems".to_string()
    }];
    deferred.push("Pre-engineered trusses, where used: layout and calculations".into());
    if ca {
        deferred.push(
            "Photovoltaic system and energy storage, where the city allows them deferred".into(),
        );
    }
    if !irc {
        deferred.push("Exterior cladding attachments, guards and storefront engineering".into());
    }
    if j == Jurisdiction::Florida {
        deferred.push("Product approvals not listed on the drawings".into());
    }
    deferred.push("Deferred submittals go to the architect for review, then to the building official for approval before installation.".into());

    let mut permits =
        vec!["Fire sprinkler system: fire department plan review and permit.".to_string()];
    if ca {
        permits.push("Solar photovoltaic system and energy storage: electrical permit.".into());
    }
    permits.extend([
        "Encroachment permit for work in the public right-of-way: driveway approach, sidewalk, utility connections.".to_string(),
        "Utility services: water, sewer, storm drain and electric connections with the utility providers.".into(),
        "Grading and drainage, where the city's thresholds require a grading permit.".into(),
        "Demolition, tree removal and work near protected trees, where applicable.".into(),
        "Fences, retaining walls, pools and spas: separate submittals.".into(),
    ]);
    CoverData {
        title: or(&ident.name, "PROJECT").to_uppercase(),
        kind: kind.into(),
        address: address.to_uppercase(),
        description,
        scope,
        directory,
        data,
        codes,
        deferred,
        permits,
        street: street_name(&l.street),
    }
}

/// Where everything on the cover goes, in paper mm.
#[derive(Debug, Clone, Default)]
pub struct CoverPlan {
    /// (top-left, text, height, width, alignment).
    pub notes: Vec<(Pt, String, f64, Option<f64>, TextAlign)>,
    pub lines: Vec<(Pt, Pt, LineStyle)>,
    pub regions: Vec<(Vec<Pt>, FillPattern)>,
    pub north: Option<Pt>,
    /// The rendering's centre and printed width.
    pub rendering: Option<(Pt, f64)>,
    /// The sheet index's centre.
    pub index: Option<Pt>,
    /// The location and vicinity maps' boxes (ADR-107).
    pub maps: Vec<(MapKind, Pt, Pt)>,
}

/// A titled box's body.
enum Body<'a> {
    Text(&'a [String], bool),
    Pairs(&'a [(String, String)]),
    Directory(&'a [(String, Vec<String>)]),
    Empty(f64),
}

pub(crate) struct Pen {
    k: f64,
    plan: CoverPlan,
    /// Heading and body text heights at full size (paper mm).
    head_size: f64,
    body_size: f64,
}

impl Pen {
    fn head(&self) -> f64 {
        self.head_size * self.k
    }
    fn body(&self) -> f64 {
        self.body_size * self.k
    }
    fn pad(&self) -> f64 {
        3.0 * self.k
    }
    fn bar(&self) -> f64 {
        self.head() * 2.4
    }
    fn lines_h(&self, text: &str, size: f64, width: f64) -> f64 {
        studio_core::text::wrap(text, size, Some(width))
            .len()
            .max(1) as f64
            * size
            * LINE
    }
    fn note(&mut self, at: Pt, text: impl Into<String>, size: f64, width: Option<f64>) {
        self.plan
            .notes
            .push((at, text.into(), size, width, TextAlign::Left));
    }
    fn rect(&mut self, x: f64, top: f64, w: f64, h: f64, style: LineStyle) {
        let (a, b, c, d) = (
            Pt::new(x, top),
            Pt::new(x + w, top),
            Pt::new(x + w, top - h),
            Pt::new(x, top - h),
        );
        for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
            self.plan.lines.push((p, q, style));
        }
    }
    /// The body's height inside a box `w` wide.
    fn measure(&self, body: &Body<'_>, w: f64) -> f64 {
        let inner = w - 2.0 * self.pad();
        let s = self.body();
        let gap = s * 0.6;
        match body {
            Body::Text(lines, numbered) => lines
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let t = if *numbered {
                        format!("{}. {l}", i + 1)
                    } else {
                        l.clone()
                    };
                    self.lines_h(&t, s, inner) + gap
                })
                .sum(),
            Body::Pairs(rows) => {
                let lw = inner * 0.4;
                rows.iter()
                    .map(|(_, v)| self.lines_h(v, s, inner - lw).max(s * LINE) + gap * 0.5)
                    .sum()
            }
            Body::Directory(rows) => rows
                .iter()
                .map(|(_, ls)| {
                    s * LINE + ls.iter().map(|l| self.lines_h(l, s, inner)).sum::<f64>() + gap
                })
                .sum(),
            Body::Empty(h) => *h,
        }
    }
    /// A ruled box with its title bar and body, top-left at (x, top); returns its height.
    /// `height` stretches it (never shrinks it below its content).
    fn boxed(
        &mut self,
        x: f64,
        top: f64,
        w: f64,
        title: &str,
        body: Body<'_>,
        height: Option<f64>,
    ) -> f64 {
        let pad = self.pad();
        let content = self.measure(&body, w);
        let h = (self.bar() + pad + content + pad).max(height.unwrap_or(0.0));
        self.rect(x, top, w, h, LineStyle::Medium);
        let bar = self.bar();
        self.plan.lines.push((
            Pt::new(x, top - bar),
            Pt::new(x + w, top - bar),
            LineStyle::Thin,
        ));
        let hs = self.head();
        self.note(
            Pt::new(x + pad, top - bar / 2.0 - hs * 0.4),
            title,
            hs,
            None,
        );
        let s = self.body();
        let inner = w - 2.0 * pad;
        let gap = s * 0.6;
        let mut y = top - bar - pad;
        match body {
            Body::Text(lines, numbered) => {
                for (i, l) in lines.iter().enumerate() {
                    let t = if numbered {
                        format!("{}. {l}", i + 1)
                    } else {
                        l.clone()
                    };
                    let lh = self.lines_h(&t, s, inner);
                    self.note(Pt::new(x + pad, y - s * 0.8), t, s, Some(inner));
                    y -= lh + gap;
                }
            }
            Body::Pairs(rows) => {
                let lw = inner * 0.4;
                for (k, v) in rows {
                    let lh = self.lines_h(v, s, inner - lw).max(s * LINE);
                    self.note(Pt::new(x + pad, y - s * 0.8), k.clone(), s, Some(lw - 2.0));
                    self.note(
                        Pt::new(x + pad + lw, y - s * 0.8),
                        v.clone(),
                        s,
                        Some(inner - lw),
                    );
                    y -= lh + gap * 0.5;
                }
            }
            Body::Directory(rows) => {
                for (role, ls) in rows {
                    self.note(Pt::new(x + pad, y - s * 0.8), role.clone(), s, None);
                    y -= s * LINE;
                    for l in ls {
                        let lh = self.lines_h(l, s, inner - 4.0);
                        self.note(
                            Pt::new(x + pad + 4.0 * self.k, y - s * 0.8),
                            l.clone(),
                            s,
                            Some(inner - 4.0),
                        );
                        y -= lh;
                    }
                    y -= gap;
                }
            }
            Body::Empty(_) => {}
        }
        h
    }
}

/// Lays the cover out in the drawing area (x0, y0)–(x1, y1). `index` is the sheet index's
/// printed size, `rendering` the aspect (width / height) of the rendering to show; `located`:
/// the site is located, so the maps can show it.
pub fn layout(
    c: &CoverData,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    index: Option<(f64, f64)>,
    rendering: Option<f64>,
    located: bool,
) -> CoverPlan {
    let w = x1 - x0;
    let k = (w / 775.0).clamp(0.55, 1.0);
    let mut p = Pen {
        k,
        plan: CoverPlan::default(),
        head_size: 4.0,
        body_size: 3.2,
    };
    let g = 8.0 * k;

    // The title band.
    let head = 50.0 * k;
    p.note(
        Pt::new(x0 + 2.0, y1 - 15.0 * k),
        c.title.clone(),
        12.0 * k,
        None,
    );
    p.note(
        Pt::new(x0 + 2.5, y1 - 29.0 * k),
        c.kind.clone(),
        5.0 * k,
        None,
    );
    p.note(
        Pt::new(x0 + 2.5, y1 - 39.0 * k),
        c.address.clone(),
        3.6 * k,
        None,
    );
    p.plan.lines.push((
        Pt::new(x0, y1 - head),
        Pt::new(x1, y1 - head),
        LineStyle::Wide,
    ));
    let top = y1 - head - g;

    // Columns: the right fits the sheet index.
    let iw = index.map_or(170.0 * k, |s| s.0);
    let c3 = (iw + 6.0).clamp(150.0 * k, w * 0.34);
    let rest = w - c3 - 2.0 * g;
    let c1 = rest * 0.6;
    let c2 = rest - c1 - g;
    let (xa, xb, xc) = (x0, x0 + c1 + g, x0 + c1 + g + c2 + g);

    // Left: rendering, description and vicinity map, deferred submittals.
    let aspect = rendering.unwrap_or(16.0 / 9.0);
    let body_h = top - y0;
    let rh = (c1 / aspect).min(body_h * 0.38);
    let rw = rh * aspect;
    let title_room = 12.0 * k;
    match rendering {
        Some(_) => {
            p.plan.rendering = Some((Pt::new(xa + c1 / 2.0, top - rh / 2.0), rw));
        }
        None => {
            p.rect(xa, top, c1, rh, LineStyle::Thin);
            let s = 4.0 * k;
            p.plan.notes.push((
                Pt::new(xa + c1 / 2.0, top - rh / 2.0),
                "PROJECT RENDERING".into(),
                s,
                None,
                TextAlign::Center,
            ));
        }
    }
    let mut y = top - rh - title_room - g * 0.5;
    // The location and vicinity maps side by side (ADR-107): Google's imagery with the
    // address over it, or, before the site is located, a schematic vicinity map.
    let mw = (c1 - g) / 2.0;
    let mh = mw * 0.66;
    let bar = p.bar();
    for (i, (kind, title)) in [
        (MapKind::Location, "LOCATION MAP"),
        (MapKind::Vicinity, "VICINITY MAP"),
    ]
    .into_iter()
    .enumerate()
    {
        let x = xa + i as f64 * (mw + g);
        p.boxed(x, y, mw, title, Body::Empty(0.0), Some(mh));
        let (lo, hi) = (
            Pt::new(x + 2.0 * k, y - mh + 2.0 * k),
            Pt::new(x + mw - 2.0 * k, y - bar - 2.0 * k),
        );
        if located {
            p.plan.maps.push((kind, lo, hi));
            if kind == MapKind::Vicinity {
                p.plan.north = Some(Pt::new(hi.x - 8.0 * k, hi.y - 9.0 * k));
            }
        } else if kind == MapKind::Vicinity {
            vicinity(&mut p, x, y - bar, mw, mh - bar, &c.street);
        } else {
            p.plan.notes.push((
                Pt::new(x + mw / 2.0, y - mh / 2.0),
                "Locate the project on the Site tab to show its map".into(),
                2.2 * k,
                None,
                TextAlign::Center,
            ));
        }
    }
    y -= mh + g;
    let mut desc = vec![c.description.clone()];
    desc.extend(c.scope.iter().map(|s| format!("• {s}")));
    let dh = p.bar() + 2.0 * p.pad() + p.measure(&Body::Text(&desc, false), c1);
    p.boxed(
        xa,
        y,
        c1,
        "PROJECT DESCRIPTION & SCOPE OF WORK",
        Body::Text(&desc, false),
        Some(dh),
    );
    y -= dh + g;
    let left = (y - y0).max(0.0);
    let half1 = (c1 - g) / 2.0;
    p.boxed(
        xa,
        y,
        half1,
        "DEFERRED SUBMITTALS",
        Body::Text(&c.deferred, true),
        Some(left),
    );
    p.boxed(
        xa + half1 + g,
        y,
        half1,
        "SEPARATE PERMITS AND APPROVALS",
        Body::Text(&c.permits, true),
        Some(left),
    );

    // Middle: directory, project data, codes.
    let mut y = top;
    y -= p.boxed(
        xb,
        y,
        c2,
        "PROJECT DIRECTORY",
        Body::Directory(&c.directory),
        None,
    ) + g;
    y -= p.boxed(xb, y, c2, "PROJECT DATA", Body::Pairs(&c.data), None) + g;
    let left = (y - y0).max(0.0);
    p.boxed(
        xb,
        y,
        c2,
        "APPLICABLE CODES",
        Body::Text(&c.codes, false),
        Some(left),
    );

    // Right: the sheet index, then the stamp and the agency's approval.
    if let Some((iw, ih)) = index {
        p.plan.index = Some(Pt::new(xc + iw / 2.0 + 1.0, top - ih / 2.0));
    }
    let bh = 62.0 * k;
    let half = (c3 - g) / 2.0;
    p.boxed(
        xc,
        y0 + bh,
        half,
        "ARCHITECT'S STAMP",
        Body::Empty(0.0),
        Some(bh),
    );
    p.boxed(
        xc + half + g,
        y0 + bh,
        half,
        "AGENCY APPROVAL",
        Body::Empty(0.0),
        Some(bh),
    );
    p.plan
}

/// "ZONING DISTRICT: R-1" → ("ZONING DISTRICT", "R-1"), for lines that read as data.
fn pair(line: &str) -> Option<(String, String)> {
    let (k, v) = line.split_once(": ")?;
    let key = k.trim();
    let upper = key.chars().any(|c| c.is_ascii_uppercase())
        && !key.chars().any(|c| c.is_ascii_lowercase())
        && key.len() <= 48;
    upper.then(|| (key.to_string(), v.trim().to_string()))
}

/// A general sheet's blocks laid out as the cover's boxes (ADR-105): each a ruled box with
/// its title bar, in columns balanced across the drawing area (x0, y0)–(x1, y1); data
/// blocks ("KEY: value" lines) as two-column tables; a block too tall for a column goes
/// on in the next as "(CONT.)".
pub fn text_sheet(
    blocks: &[crate::general::Block],
    (x0, y0, x1, y1): (f64, f64, f64, f64),
) -> CoverPlan {
    let (w, h) = (x1 - x0, y1 - y0);
    let mut p = Pen {
        k: (w / 775.0).clamp(0.6, 1.0),
        plan: CoverPlan::default(),
        head_size: 4.0,
        body_size: 3.2,
    };
    let g = 8.0 * p.k;
    let n = ((w + g) / (180.0 * p.k + g)).floor().max(1.0);
    let cw = (w - g * (n - 1.0)) / n;
    enum Kind {
        Text(Vec<String>),
        Pairs(Vec<(String, String)>),
    }
    // Numbered lines keep their numbers when a block splits.
    let pieces: Vec<(String, Kind, Option<crate::general::Figure>)> = blocks
        .iter()
        .map(|b| {
            let pairs: Vec<(String, String)> = b.lines.iter().filter_map(|l| pair(l)).collect();
            let kind = if !b.numbered && pairs.len() == b.lines.len() && !pairs.is_empty() {
                Kind::Pairs(pairs)
            } else {
                Kind::Text(
                    b.lines
                        .iter()
                        .enumerate()
                        .map(|(i, l)| {
                            if b.numbered {
                                format!("{}. {l}", i + 1)
                            } else {
                                l.clone()
                            }
                        })
                        .collect(),
                )
            };
            (b.title.clone(), kind, b.figure.clone())
        })
        .collect();
    let chrome = p.bar() + 2.0 * p.pad();
    let height = |p: &Pen, k: &Kind| -> f64 {
        chrome
            + match k {
                Kind::Text(ls) => p.measure(&Body::Text(ls, false), cw),
                Kind::Pairs(ps) => p.measure(&Body::Pairs(ps), cw),
            }
    };
    // A figure fills the box width at its frame's proportions, 80 mm tall at most.
    let (fpad, fk) = (p.pad(), p.k);
    let fig_h = |f: &Option<crate::general::Figure>| -> f64 {
        f.as_ref().map_or(0.0, |f| {
            let (lo, hi) = f.frame;
            let (fw, fh) = ((hi.x - lo.x).max(1.0), (hi.y - lo.y).max(1.0));
            ((cw - 2.0 * fpad) * fh / fw).min(80.0 * fk) + fpad
        })
    };
    let total: f64 = pieces
        .iter()
        .map(|b| height(&p, &b.1) + fig_h(&b.2) + g)
        .sum();
    let tallest = pieces
        .iter()
        .map(|b| height(&p, &b.1) + fig_h(&b.2))
        .fold(0.0, f64::max);
    // Columns about equally full, none past the sheet.
    let soft = (total / n).max(tallest.min(h)).min(h) * 1.04;
    let (mut col, mut y) = (0.0, y1);
    for (title, kind, mut figure) in pieces {
        let mut rest = kind;
        let mut first = true;
        let extra = fig_h(&figure);
        loop {
            let x = x0 + col * (cw + g);
            let full = height(&p, &rest) + if first { extra } else { 0.0 };
            let used = y1 - y;
            if used > 0.0 && used + full > soft && col < n - 1.0 {
                col += 1.0;
                y = y1;
                continue;
            }
            let room = y - y0;
            let label = if first {
                title.clone()
            } else {
                format!("{title} (CONT.)")
            };
            if full <= room || col >= n - 1.0 || figure.is_some() {
                let body = match &rest {
                    Kind::Text(ls) => Body::Text(ls, false),
                    Kind::Pairs(ps) => Body::Pairs(ps),
                };
                let boxed = p.boxed(x, y, cw, &label, body, Some(full));
                if let Some(f) = figure.take() {
                    let pad = p.pad();
                    figure_in(
                        &mut p,
                        &f,
                        x + pad,
                        y - boxed + pad,
                        cw - 2.0 * pad,
                        extra - pad,
                    );
                }
                y -= boxed + g;
                break;
            }
            // Split: as many lines as fit here, the rest in the next column.
            let fits = |m: usize, r: &Kind| -> f64 {
                match r {
                    Kind::Text(ls) => height(&p, &Kind::Text(ls[..m].to_vec())),
                    Kind::Pairs(ps) => height(&p, &Kind::Pairs(ps[..m].to_vec())),
                }
            };
            let len = match &rest {
                Kind::Text(ls) => ls.len(),
                Kind::Pairs(ps) => ps.len(),
            };
            let mut m = 0;
            while m < len && fits(m + 1, &rest) <= room {
                m += 1;
            }
            if m == 0 {
                col += 1.0;
                y = y1;
                continue;
            }
            let (now, later) = match rest {
                Kind::Text(ls) => (Kind::Text(ls[..m].to_vec()), Kind::Text(ls[m..].to_vec())),
                Kind::Pairs(ps) => (Kind::Pairs(ps[..m].to_vec()), Kind::Pairs(ps[m..].to_vec())),
            };
            let body = match &now {
                Kind::Text(ls) => Body::Text(ls, false),
                Kind::Pairs(ps) => Body::Pairs(ps),
            };
            p.boxed(x, y, cw, &label, body, None);
            rest = later;
            first = false;
            col += 1.0;
            y = y1;
        }
    }
    p.plan
}

/// Draws a figure's rings fitted and centred in the box (x, y)–(x + w, y + h).
fn figure_in(p: &mut Pen, f: &crate::general::Figure, x: f64, y: f64, w: f64, h: f64) {
    let (lo, hi) = f.frame;
    let (fw, fh) = ((hi.x - lo.x).max(1.0), (hi.y - lo.y).max(1.0));
    let s = (w / fw).min(h / fh);
    let off = Pt::new(x + (w - fw * s) / 2.0, y + (h - fh * s) / 2.0);
    let map = |q: Pt| off.add(q.sub(lo).scale(s));
    for (ring, dashed) in &f.rings {
        let style = if *dashed {
            LineStyle::Hidden
        } else {
            LineStyle::Medium
        };
        for i in 0..ring.len() {
            let (a, b) = (map(ring[i]), map(ring[(i + 1) % ring.len()]));
            p.plan.lines.push((a, b, style));
        }
    }
}
/// A schematic vicinity map in the box (x, top, w, h): the site's street, cross streets and
/// a block grid, the site marked, a north arrow, not to scale.
fn vicinity(p: &mut Pen, x: f64, top: f64, w: f64, h: f64, street: &str) {
    let k = p.k;
    let (bx0, by0) = (x + 4.0 * k, top - h + 4.0 * k);
    let (bw, bh) = (w - 8.0 * k, h - 8.0 * k);
    let at = |u: f64, v: f64| Pt::new(bx0 + u * bw, by0 + v * bh);
    // The site's street, two cross streets, a parallel street and an arterial.
    p.plan
        .lines
        .push((at(0.36, 0.0), at(0.36, 1.0), LineStyle::Wide));
    for v in [0.22, 0.74] {
        p.plan
            .lines
            .push((at(0.0, v), at(1.0, v), LineStyle::Medium));
    }
    p.plan
        .lines
        .push((at(0.78, 0.0), at(0.78, 1.0), LineStyle::Thin));
    p.plan
        .lines
        .push((at(0.0, 0.92), at(0.24, 1.0), LineStyle::Medium));
    // The site: the lot beside its street, between the cross streets.
    let s = [
        at(0.39, 0.42),
        at(0.52, 0.42),
        at(0.52, 0.56),
        at(0.39, 0.56),
    ];
    p.plan.regions.push((s.to_vec(), FillPattern::Solid));
    let ts = 1.9 * k;
    p.note(at(0.55, 0.52), "PROJECT SITE", ts, None);
    if !street.is_empty() {
        p.note(at(0.38, 0.97), street.to_string(), ts, Some(bw * 0.4));
    }
    p.note(Pt::new(bx0, by0 + ts * 0.4), "NOT TO SCALE", ts, None);
    p.plan.north = Some(Pt::new(bx0 + bw - 8.0 * k, by0 + bh - 9.0 * k));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_street_name_drops_the_house_number() {
        assert_eq!(street_name("2150 Oak Knoll Lane"), "OAK KNOLL LANE");
        assert_eq!(street_name("Oak Knoll Lane"), "OAK KNOLL LANE");
    }

    #[test]
    fn the_cover_fills_its_columns_and_stays_on_the_sheet() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let c = cover_data(&doc, BuildingType::SingleFamily, Jurisdiction::California);
        assert!(c.codes[0].contains("2025 California Residential Code"));
        assert!(c.directory.iter().any(|d| d.0 == "TITLE 24 CONSULTANT"));
        assert!(c.deferred[0].contains("sprinkler"));
        let area = (31.4, 18.7, 806.8, 590.9);
        let plan = layout(&c, area, Some((180.0, 200.0)), Some(16.0 / 9.0), true);
        assert_eq!(plan.maps.len(), 2);
        assert!(plan.rendering.is_some() && plan.index.is_some() && plan.north.is_some());
        for (a, b, _) in &plan.lines {
            for q in [a, b] {
                assert!(q.x >= area.0 - 0.01 && q.x <= area.2 + 0.01, "{q:?}");
                assert!(q.y >= area.1 - 0.01 && q.y <= area.3 + 0.01, "{q:?}");
            }
        }
        let titles: Vec<&str> = plan.notes.iter().map(|n| n.1.as_str()).collect();
        for t in [
            "PROJECT DIRECTORY",
            "PROJECT DATA",
            "APPLICABLE CODES",
            "DEFERRED SUBMITTALS",
            "VICINITY MAP",
            "ARCHITECT'S STAMP",
            "AGENCY APPROVAL",
        ] {
            assert!(titles.contains(&t), "{t}");
        }
    }
}
