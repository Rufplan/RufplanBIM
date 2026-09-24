//! IFC4 export (hand-written STEP, ADR-007).
//!
//! Spatial structure: IfcProject → IfcSite → IfcBuilding → IfcBuildingStorey per level.
//! Elements: IfcWall (with IfcOpeningElement voids), IfcDoor / IfcWindow filling them,
//! IfcSlab (floors), IfcCovering (ceilings) and IfcSpace (rooms). Geometry is swept solids
//! (extruded polylines) in millimetres. Walls carry their type's material and
//! Pset_WallCommon; spaces carry their net floor area.
//!
//! GlobalIds derive from element ids (DATA_MODEL.md), so re-exports keep stable GUIDs.

use std::fmt::Write as _;

use studio_core::{ops, Document, ElementData, ElementId};
use studio_geom::Pt;
use studio_regen::{regenerate, OpeningKind};
use uuid::Uuid;

/// IFC's 22-character base-64 GlobalId encoding of a 128-bit id.
pub fn ifc_guid(id: Uuid) -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let n = id.as_u128();
    let mut out = String::with_capacity(22);
    out.push(CHARS[(n >> 126) as usize] as char);
    for i in (0..21).rev() {
        out.push(CHARS[((n >> (6 * i)) & 63) as usize] as char);
    }
    out
}

/// A stable id for something derived from `base` (relationships, openings, psets).
fn derived(base: Uuid, what: &str) -> Uuid {
    Uuid::new_v5(&base, what.as_bytes())
}

/// STEP string literal: quotes doubled, non-ASCII as \X2\ escapes.
fn s(text: &str) -> String {
    let mut out = String::from("'");
    for c in text.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            c if (' '..='~').contains(&c) => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                out.push_str("\\X2\\");
                for u in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "{:04X}", u);
                }
                out.push_str("\\X0\\");
            }
        }
    }
    out.push('\'');
    out
}

/// STEP real: always has a decimal point, no exponent for the ranges used here.
fn r(v: f64) -> String {
    if v.abs() < 1e-9 {
        return "0.".into();
    }
    let t = format!("{v:.6}");
    let t = t.trim_end_matches('0');
    t.to_owned()
}

struct Writer {
    lines: Vec<String>,
}

impl Writer {
    /// Adds an entity and returns its #id.
    fn add(&mut self, entity: String) -> usize {
        self.lines.push(entity);
        self.lines.len()
    }
    fn point2(&mut self, p: Pt) -> usize {
        self.add(format!("IFCCARTESIANPOINT(({},{}))", r(p.x), r(p.y)))
    }
    fn point3(&mut self, x: f64, y: f64, z: f64) -> usize {
        self.add(format!("IFCCARTESIANPOINT(({},{},{}))", r(x), r(y), r(z)))
    }
    /// Placement at (0, 0, z) relative to `parent`.
    fn placement(&mut self, parent: Option<usize>, z: f64) -> usize {
        let o = self.point3(0.0, 0.0, z);
        let ax = self.add(format!("IFCAXIS2PLACEMENT3D(#{o},$,$)"));
        let rel = parent.map_or("$".into(), |p| format!("#{p}"));
        self.add(format!("IFCLOCALPLACEMENT({rel},#{ax})"))
    }
    /// A body representation: `ring` (plan mm, relative to the placement's origin)
    /// extruded up by `depth` mm from `z0`.
    fn extrusion(&mut self, ctx: usize, ring: &[Pt], z0: f64, depth: f64) -> usize {
        let mut ids: Vec<usize> = ring.iter().map(|p| self.point2(*p)).collect();
        if let Some(first) = ids.first().copied() {
            ids.push(first);
        }
        let list = ids
            .iter()
            .map(|i| format!("#{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let poly = self.add(format!("IFCPOLYLINE(({list}))"));
        let profile = self.add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{poly})"));
        let o = self.point3(0.0, 0.0, z0);
        let pos = self.add(format!("IFCAXIS2PLACEMENT3D(#{o},$,$)"));
        let up = self.add("IFCDIRECTION((0.,0.,1.))".into());
        let solid = self.add(format!(
            "IFCEXTRUDEDAREASOLID(#{profile},#{pos},#{up},{})",
            r(depth)
        ));
        let rep = self.add(format!(
            "IFCSHAPEREPRESENTATION(#{ctx},'Body','SweptSolid',(#{solid}))"
        ));
        self.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"))
    }
    fn refs(ids: &[usize]) -> String {
        ids.iter()
            .map(|i| format!("#{i}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// What the export contains, for reporting and tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IfcSummary {
    pub storeys: usize,
    pub walls: usize,
    pub doors: usize,
    pub windows: usize,
    pub slabs: usize,
    pub coverings: usize,
    pub spaces: usize,
}

/// Writes the model as an IFC4 STEP file. `timestamp` is ISO 8601 (for the header).
pub fn export_ifc(doc: &Document, app_version: &str, timestamp: &str) -> (String, IfcSummary) {
    let model = regenerate(doc);
    let mut w = Writer { lines: vec![] };
    let mut summary = IfcSummary::default();

    let info = ops::project_info(doc);
    let (project_name, project_number) = match info.and_then(|i| doc.data(i).ok()) {
        Some(ElementData::ProjectInfo { name, number, .. }) => (name.clone(), number.clone()),
        _ => ("Project".into(), String::new()),
    };
    let project_uuid = info.map_or_else(Uuid::nil, |i| i.0);

    // Units: millimetres, square and cubic metres, radians.
    let mm = w.add("IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)".into());
    let m2 = w.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)".into());
    let m3 = w.add("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)".into());
    let rad = w.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)".into());
    let units = w.add(format!(
        "IFCUNITASSIGNMENT(({}))",
        Writer::refs(&[mm, m2, m3, rad])
    ));
    let origin = w.point3(0.0, 0.0, 0.0);
    let world = w.add(format!("IFCAXIS2PLACEMENT3D(#{origin},$,$)"));
    let north = w.add("IFCDIRECTION((0.,1.))".into());
    let ctx = w.add(format!(
        "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{world},#{north})"
    ));
    let body = w.add(format!(
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{ctx},$,.MODEL_VIEW.,$)"
    ));

    let project = w.add(format!(
        "IFCPROJECT({},$,{},$,$,{},$,(#{ctx}),#{units})",
        s(&ifc_guid(project_uuid)),
        s(&project_name),
        s(&project_number)
    ));
    let site_place = w.placement(None, 0.0);
    let site = w.add(format!(
        "IFCSITE({},$,'Site',$,$,#{site_place},$,$,.ELEMENT.,$,$,$,$,$)",
        s(&ifc_guid(derived(project_uuid, "site")))
    ));
    let bldg_place = w.placement(Some(site_place), 0.0);
    let building = w.add(format!(
        "IFCBUILDING({},$,{},$,$,#{bldg_place},$,$,.ELEMENT.,$,$,$)",
        s(&ifc_guid(derived(project_uuid, "building"))),
        s(&project_name)
    ));
    w.add(format!(
        "IFCRELAGGREGATES({},$,$,$,#{project},(#{site}))",
        s(&ifc_guid(derived(project_uuid, "agg-site")))
    ));
    w.add(format!(
        "IFCRELAGGREGATES({},$,$,$,#{site},(#{building}))",
        s(&ifc_guid(derived(project_uuid, "agg-building")))
    ));

    // Storeys.
    let mut storeys: Vec<(ElementId, usize, usize, f64)> = vec![]; // (level, entity, placement, elevation)
    for l in &model.levels {
        let place = w.placement(Some(bldg_place), l.elevation);
        let e = w.add(format!(
            "IFCBUILDINGSTOREY({},$,{},$,$,#{place},$,$,.ELEMENT.,{})",
            s(&ifc_guid(l.id.0)),
            s(&l.name),
            r(l.elevation)
        ));
        storeys.push((l.id, e, place, l.elevation));
        summary.storeys += 1;
    }
    if !storeys.is_empty() {
        let refs: Vec<usize> = storeys.iter().map(|s| s.1).collect();
        w.add(format!(
            "IFCRELAGGREGATES({},$,$,$,#{building},({}))",
            s(&ifc_guid(derived(project_uuid, "agg-storeys"))),
            Writer::refs(&refs)
        ));
    }
    let storey_of = |level: ElementId| storeys.iter().find(|s| s.0 == level).copied();
    let mut contained: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    let mut spaces_in: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    let mut materials: std::collections::BTreeMap<String, Vec<usize>> = Default::default();
    let type_name = |id: ElementId| {
        doc.data(id)
            .ok()
            .and_then(|d| d.type_id())
            .and_then(|t| doc.data(t).ok())
            .map(|t| t.name())
            .unwrap_or_default()
    };

    // Walls, their openings and fillings.
    for wall in &model.walls {
        let Some((_, storey, splace, elev)) = storey_of(wall.level) else {
            continue;
        };
        let place = w.placement(Some(splace), 0.0);
        let shape = w.extrusion(
            body,
            &wall.footprint.outer,
            wall.z0 - elev,
            wall.z1 - wall.z0,
        );
        let ty = type_name(wall.id);
        let e = w.add(format!(
            "IFCWALL({},$,{},$,{},#{place},#{shape},$,.STANDARD.)",
            s(&ifc_guid(wall.id.0)),
            s(&ty),
            s(&ty)
        ));
        contained.entry(storey).or_default().push(e);
        materials.entry(ty.clone()).or_default().push(e);
        summary.walls += 1;
        let ext = w.add(format!(
            "IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN({}),$)",
            if wall.exterior { ".T." } else { ".F." }
        ));
        let reference = w.add(format!(
            "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
            s(&ty)
        ));
        let pset = w.add(format!(
            "IFCPROPERTYSET({},$,'Pset_WallCommon',$,(#{ext},#{reference}))",
            s(&ifc_guid(derived(wall.id.0, "pset")))
        ));
        w.add(format!(
            "IFCRELDEFINESBYPROPERTIES({},$,$,$,(#{e}),#{pset})",
            s(&ifc_guid(derived(wall.id.0, "pset-rel")))
        ));

        for o in model.openings.iter().filter(|o| o.host == wall.id) {
            // Opening box: the opening's width, a little deeper than the wall.
            let n = o.dir.perp().scale(o.half_thickness + 10.0);
            let (a, b) = (o.at(o.t0), o.at(o.t1));
            let ring = [a.sub(n), b.sub(n), b.add(n), a.add(n)];
            let oplace = w.placement(Some(place), 0.0);
            let oshape = w.extrusion(body, &ring, o.z0 - elev, o.z1 - o.z0);
            let opening = w.add(format!(
                "IFCOPENINGELEMENT({},$,'Opening',$,$,#{oplace},#{oshape},$,.OPENING.)",
                s(&ifc_guid(derived(o.id.0, "opening")))
            ));
            w.add(format!(
                "IFCRELVOIDSELEMENT({},$,$,$,#{e},#{opening})",
                s(&ifc_guid(derived(o.id.0, "voids")))
            ));
            let (leaf_depth, is_door) = match o.kind {
                OpeningKind::Door(_) => (1.75 * 25.4, true),
                OpeningKind::Window(_) => (0.75 * 25.4, false),
            };
            let panel = o.panel(leaf_depth, o.z0, o.z1);
            let fplace = w.placement(Some(oplace), 0.0);
            let fshape = w.extrusion(body, &panel.base.outer, o.z0 - elev, o.z1 - o.z0);
            let mark = match doc.data(o.id) {
                Ok(ElementData::Door { mark, .. } | ElementData::Window { mark, .. }) => {
                    mark.clone()
                }
                _ => String::new(),
            };
            let (width, height) = (o.width(), o.z1 - o.z0);
            let ty = type_name(o.id);
            let fill = if is_door {
                summary.doors += 1;
                let op = match o.kind {
                    OpeningKind::Door(studio_core::DoorFamily::DoubleFlush) => {
                        ".DOUBLE_DOOR_SINGLE_SWING."
                    }
                    _ => ".SINGLE_SWING_LEFT.",
                };
                w.add(format!(
                    "IFCDOOR({},$,{},$,{},#{fplace},#{fshape},{},{},{},.DOOR.,{op},$)",
                    s(&ifc_guid(o.id.0)),
                    s(&format!("Door {mark}")),
                    s(&ty),
                    s(&mark),
                    r(height),
                    r(width)
                ))
            } else {
                summary.windows += 1;
                w.add(format!(
                    "IFCWINDOW({},$,{},$,{},#{fplace},#{fshape},{},{},{},.WINDOW.,.SINGLE_PANEL.,$)",
                    s(&ifc_guid(o.id.0)),
                    s(&format!("Window {mark}")),
                    s(&ty),
                    s(&mark),
                    r(height),
                    r(width)
                ))
            };
            w.add(format!(
                "IFCRELFILLSELEMENT({},$,$,$,#{opening},#{fill})",
                s(&ifc_guid(derived(o.id.0, "fills")))
            ));
            contained.entry(storey).or_default().push(fill);
            let reference = w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
                s(&ty)
            ));
            let ext = w.add(format!(
                "IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN({}),$)",
                if wall.exterior { ".T." } else { ".F." }
            ));
            let pname = if is_door {
                "Pset_DoorCommon"
            } else {
                "Pset_WindowCommon"
            };
            let pset = w.add(format!(
                "IFCPROPERTYSET({},$,{},$,(#{reference},#{ext}))",
                s(&ifc_guid(derived(o.id.0, "pset"))),
                s(pname)
            ));
            w.add(format!(
                "IFCRELDEFINESBYPROPERTIES({},$,$,$,(#{fill}),#{pset})",
                s(&ifc_guid(derived(o.id.0, "pset-rel")))
            ));
        }
    }

    // Floors and ceilings.
    for (slabs, is_floor) in [(&model.floors, true), (&model.ceilings, false)] {
        for slab in slabs {
            let Some((_, storey, splace, elev)) = storey_of(slab.level) else {
                continue;
            };
            let place = w.placement(Some(splace), 0.0);
            let shape = w.extrusion(body, &slab.base.outer, slab.z0 - elev, slab.z1 - slab.z0);
            let ty = type_name(slab.id);
            let e = if is_floor {
                summary.slabs += 1;
                w.add(format!(
                    "IFCSLAB({},$,{},$,$,#{place},#{shape},$,.FLOOR.)",
                    s(&ifc_guid(slab.id.0)),
                    s(&ty)
                ))
            } else {
                summary.coverings += 1;
                w.add(format!(
                    "IFCCOVERING({},$,{},$,$,#{place},#{shape},$,.CEILING.)",
                    s(&ifc_guid(slab.id.0)),
                    s(&ty)
                ))
            };
            contained.entry(storey).or_default().push(e);
            materials.entry(ty).or_default().push(e);
        }
    }

    // Rooms as spaces: their enclosed area, from the level up to the level above.
    let levels = doc.levels();
    for room in &model.rooms {
        let Some(boundary) = &room.boundary else {
            continue;
        };
        let Some((_, storey, splace, elev)) = storey_of(room.level) else {
            continue;
        };
        let height = levels
            .iter()
            .position(|l| l.0 == room.level)
            .and_then(|i| levels.get(i + 1))
            .map_or(ops::DEFAULT_FLOOR_TO_FLOOR, |above| above.2 - elev);
        let place = w.placement(Some(splace), 0.0);
        let shape = w.extrusion(body, boundary, 0.0, height);
        let e = w.add(format!(
            "IFCSPACE({},$,{},$,$,#{place},#{shape},{},.ELEMENT.,.INTERNAL.,$)",
            s(&ifc_guid(room.id.0)),
            s(&room.number),
            s(&room.name)
        ));
        spaces_in.entry(storey).or_default().push(e);
        summary.spaces += 1;
        let area = w.add(format!(
            "IFCQUANTITYAREA('NetFloorArea',$,$,{},$)",
            r(room.area() / 1.0e6)
        ));
        let qto = w.add(format!(
            "IFCELEMENTQUANTITY({},$,'Qto_SpaceBaseQuantities',$,$,(#{area}))",
            s(&ifc_guid(derived(room.id.0, "qto")))
        ));
        w.add(format!(
            "IFCRELDEFINESBYPROPERTIES({},$,$,$,(#{e}),#{qto})",
            s(&ifc_guid(derived(room.id.0, "qto-rel")))
        ));
    }

    for (storey, items) in &contained {
        w.add(format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,({}),#{storey})",
            s(&ifc_guid(derived(
                project_uuid,
                &format!("contains-{storey}")
            ))),
            Writer::refs(items)
        ));
    }
    for (storey, items) in &spaces_in {
        w.add(format!(
            "IFCRELAGGREGATES({},$,$,$,#{storey},({}))",
            s(&ifc_guid(derived(
                project_uuid,
                &format!("spaces-{storey}")
            ))),
            Writer::refs(items)
        ));
    }
    for (name, items) in &materials {
        if name.is_empty() {
            continue;
        }
        let m = w.add(format!("IFCMATERIAL({},$,$)", s(name)));
        w.add(format!(
            "IFCRELASSOCIATESMATERIAL({},$,$,$,({}),#{m})",
            s(&ifc_guid(derived(
                project_uuid,
                &format!("material-{name}")
            ))),
            Writer::refs(items)
        ));
    }

    let mut out = String::new();
    out.push_str("ISO-10303-21;\nHEADER;\n");
    out.push_str("FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\n");
    let _ = writeln!(
        out,
        "FILE_NAME({},{},(''),(''),{},{},'');",
        s(&format!("{project_name}.ifc")),
        s(timestamp),
        s(&format!("Rufplan Studio {app_version}")),
        s("Rufplan Studio")
    );
    out.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
    for (i, line) in w.lines.iter().enumerate() {
        let _ = writeln!(out, "#{}={};", i + 1, line);
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    (out, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::units::MM_PER_FT;
    use studio_core::Category;

    #[test]
    fn guid_encoding_is_22_chars_from_the_ifc_alphabet() {
        let g = ifc_guid(Uuid::from_u128(0));
        assert_eq!(g, "0000000000000000000000");
        let g = ifc_guid(Uuid::from_u128(u128::MAX));
        assert_eq!(g, "3$$$$$$$$$$$$$$$$$$$$$");
        let g = ifc_guid(Uuid::parse_str("01a0d1ba-ace4-721d-8553-6b383bdc2cf9").unwrap());
        assert_eq!(g.len(), 22);
        assert!(g
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$'));
    }

    #[test]
    fn step_strings_and_reals() {
        assert_eq!(s("O'Brien"), "'O''Brien'");
        assert_eq!(s("Café"), "'Caf\\X2\\00E9\\X0\\'");
        assert_eq!(r(3048.0), "3048.");
        assert_eq!(r(0.0), "0.");
        assert_eq!(r(-12.5), "-12.5");
    }

    #[test]
    fn exports_a_house_with_openings_slabs_and_spaces() {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = doc
            .of(Category::WallType)
            .find(|e| e.data.name().starts_with("Exterior - 8"))
            .unwrap()
            .id;
        let (w, h) = (30.0 * MM_PER_FT, 20.0 * MM_PER_FT);
        let c = [
            Pt::new(0.0, 0.0),
            Pt::new(w, 0.0),
            Pt::new(w, h),
            Pt::new(0.0, h),
        ];
        let walls: Vec<_> = (0..4)
            .map(|i| ops::create_wall(&mut doc, wt, l1, c[i], c[(i + 1) % 4]).unwrap())
            .collect();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        let wn = doc
            .of(Category::WindowType)
            .find(|e| e.data.name().starts_with("Fixed 48"))
            .unwrap()
            .id;
        ops::create_door(&mut doc, dt, walls[0], 5.0 * MM_PER_FT, false).unwrap();
        ops::create_window(&mut doc, wn, walls[1], 10.0 * MM_PER_FT, true).unwrap();
        let m = regenerate(&doc);
        let ft = ops::first_of(&doc, Category::FloorType).unwrap();
        ops::create_floor(
            &mut doc,
            ft,
            l1,
            studio_regen::outer_boundary(&m, l1).unwrap(),
        )
        .unwrap();
        ops::create_room(&mut doc, l1, Pt::new(3000.0, 3000.0)).unwrap();

        let (ifc, sum) = export_ifc(&doc, "0.0.1", "2026-09-24T00:00:00");
        assert_eq!(
            sum,
            IfcSummary {
                storeys: 2,
                walls: 4,
                doors: 1,
                windows: 1,
                slabs: 1,
                coverings: 0,
                spaces: 1
            }
        );
        assert!(ifc.starts_with("ISO-10303-21;"));
        assert!(ifc.contains("FILE_SCHEMA(('IFC4'));"));
        assert!(ifc.trim_end().ends_with("END-ISO-10303-21;"));
        for entity in [
            "IFCPROJECT(",
            "IFCSITE(",
            "IFCBUILDING(",
            "IFCBUILDINGSTOREY(",
            "IFCWALL(",
            "IFCOPENINGELEMENT(",
            "IFCRELVOIDSELEMENT(",
            "IFCDOOR(",
            "IFCWINDOW(",
            "IFCRELFILLSELEMENT(",
            "IFCSLAB(",
            "IFCSPACE(",
            "IFCMATERIAL(",
            "Pset_WallCommon",
            "IFCQUANTITYAREA('NetFloorArea'",
        ] {
            assert!(ifc.contains(entity), "missing {entity}");
        }
        // Every #reference points at an entity that exists.
        let n = ifc.lines().filter(|l| l.starts_with('#')).count();
        for cap in ifc.split('#').skip(1) {
            let digits: String = cap.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(i) = digits.parse::<usize>() {
                assert!(i >= 1 && i <= n, "dangling #{i}");
            }
        }
        // GUIDs are unique.
        let mut guids: Vec<&str> = ifc
            .lines()
            .filter_map(|l| l.split('(').nth(1))
            .filter_map(|a| a.split(',').next())
            .filter(|g| g.len() == 24 && g.starts_with('\''))
            .collect();
        let before = guids.len();
        guids.sort_unstable();
        guids.dedup();
        assert_eq!(guids.len(), before, "duplicate GlobalIds");
        // Stable: exporting again gives the same file.
        assert_eq!(export_ifc(&doc, "0.0.1", "2026-09-24T00:00:00").0, ifc);
    }
}
