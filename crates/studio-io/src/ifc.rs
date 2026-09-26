//! IFC4 export (hand-written STEP, ADR-007).
//!
//! Spatial structure: IfcProject → IfcSite → IfcBuilding → IfcBuildingStorey per level.
//! Elements: IfcWall (with IfcOpeningElement voids), IfcDoor / IfcWindow filling them,
//! IfcSlab (floors), IfcCovering (ceilings) and IfcSpace (rooms). Geometry is swept solids
//! (extruded polylines) in millimetres. Walls carry their type's material and
//! Pset_WallCommon; spaces carry their net floor area.
//!
//! GlobalIds derive from element ids (DATA_MODEL.md), so re-exports keep stable GUIDs.

use std::collections::HashMap;
use std::fmt::Write as _;

use studio_core::{ops, Document, ElementData, ElementId};
use studio_geom::{Poly, Pt};
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

/// IfcWindow's PartitioningType and UserDefinedPartitioningType for a window family
/// (ADR-031): how its lites divide, or the family's name when IFC has no word for it.
fn partitioning(style: studio_core::windows::WindowStyle) -> String {
    use studio_core::WindowFamily as F;
    let units = if studio_core::windows::info(style.family).mullable {
        style.units
    } else {
        1
    };
    let known = match (style.family, units) {
        (F::Fixed | F::Casement | F::Awning | F::Hopper, 1) => Some("SINGLE_PANEL"),
        (F::DoubleHung | F::SingleHung | F::PictureAwning, 1) => Some("DOUBLE_PANEL_HORIZONTAL"),
        (F::Slider, _) | (F::Fixed | F::Casement | F::Awning, 2) => Some("DOUBLE_PANEL_VERTICAL"),
        (F::Slider3 | F::PictureCasement, _) | (F::Fixed | F::Casement | F::Awning, 3) => {
            Some("TRIPLE_PANEL_VERTICAL")
        }
        _ => None,
    };
    match known {
        Some(k) => format!(".{k}.,$"),
        None => format!(
            ".USERDEFINED.,{}",
            s(&studio_core::windows::base_name(
                style.family,
                units,
                0.0,
                1.0
            ))
        ),
    }
}

/// IfcDoor's OperationType and UserDefinedOperationType for a door family (ADR-033).
fn door_operation(style: studio_core::doors::DoorStyle, flip_hand: bool) -> String {
    use studio_core::DoorFamily as F;
    let side = if flip_hand { "RIGHT" } else { "LEFT" };
    let known = match (style.family, style.panels) {
        (F::SingleFlush, _) | (F::Storefront, 1) => format!("SINGLE_SWING_{side}"),
        (F::DoubleFlush, _) | (F::Storefront, _) => "DOUBLE_DOOR_SINGLE_SWING".into(),
        (F::Sidelites, 1) => format!("SWING_FIXED_{side}"),
        (F::SlidingGlass, 2) | (F::Pocket, 1) | (F::Barn, 1) => format!("SLIDING_TO_{side}"),
        (F::Pocket, _) | (F::Barn, _) => "DOUBLE_DOOR_SLIDING".into(),
        (F::Bifold, 2) => format!("FOLDING_TO_{side}"),
        (F::Bifold, _) => "DOUBLE_DOOR_FOLDING".into(),
        _ => String::new(),
    };
    if known.is_empty() {
        let name = studio_core::doors::info(style.family).label;
        format!(".USERDEFINED.,{}", s(name))
    } else {
        format!(".{known}.,$")
    }
}

/// An IFC compound plane angle: (degrees, minutes, seconds, millionths of a second).
fn compound_angle(deg: f64) -> String {
    let sign = if deg < 0.0 { -1 } else { 1 };
    let micro = (deg.abs() * 3600.0 * 1e6).round() as i64;
    let (d, rest) = (micro / 3_600_000_000, micro % 3_600_000_000);
    let (m, rest) = (rest / 60_000_000, rest % 60_000_000);
    let (sec, mic) = (rest / 1_000_000, rest % 1_000_000);
    format!("({},{},{},{})", sign * d, sign * m, sign * sec, sign * mic)
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
    fn polyline(&mut self, ring: &[Pt]) -> usize {
        let mut ids: Vec<usize> = ring.iter().map(|p| self.point2(*p)).collect();
        if let Some(first) = ids.first().copied() {
            ids.push(first);
        }
        self.add(format!("IFCPOLYLINE(({}))", Self::refs(&ids)))
    }
    /// A plan profile, with voids when the polygon has holes (a floor's stair opening).
    fn profile(&mut self, poly: &Poly) -> usize {
        let outer = self.polyline(&poly.outer);
        if poly.holes.is_empty() {
            return self.add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{outer})"));
        }
        let inner: Vec<usize> = poly.holes.iter().map(|h| self.polyline(h)).collect();
        self.add(format!(
            "IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#{outer},({}))",
            Self::refs(&inner)
        ))
    }
    /// A body of several extrusions (e.g. a stair's steps), each (base, z0, depth).
    fn extrusions(&mut self, ctx: usize, parts: &[(Poly, f64, f64)]) -> usize {
        let mut solids = vec![];
        for (base, z0, depth) in parts {
            let profile = self.profile(base);
            let o = self.point3(0.0, 0.0, *z0);
            let pos = self.add(format!("IFCAXIS2PLACEMENT3D(#{o},$,$)"));
            let up = self.add("IFCDIRECTION((0.,0.,1.))".into());
            solids.push(self.add(format!(
                "IFCEXTRUDEDAREASOLID(#{profile},#{pos},#{up},{})",
                r(*depth)
            )));
        }
        let rep = self.add(format!(
            "IFCSHAPEREPRESENTATION(#{ctx},'Body','SweptSolid',({}))",
            Self::refs(&solids)
        ));
        self.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"))
    }

    /// A triangulated body (IFC4 tessellation) from a triangle soup (9 floats per
    /// triangle), shifted down by `dz`.
    fn tessellation(&mut self, ctx: usize, tris: &[f32], dz: f64) -> usize {
        let mut verts: Vec<[i64; 3]> = vec![];
        let mut index: HashMap<[i64; 3], usize> = HashMap::new();
        let mut faces: Vec<[usize; 3]> = vec![];
        for tri in tris.as_chunks::<9>().0 {
            let mut f = [0usize; 3];
            for k in 0..3 {
                // Weld vertices on a 0.01 mm grid.
                let key = [
                    (f64::from(tri[3 * k]) * 100.0).round() as i64,
                    (f64::from(tri[3 * k + 1]) * 100.0).round() as i64,
                    ((f64::from(tri[3 * k + 2]) - dz) * 100.0).round() as i64,
                ];
                f[k] = *index.entry(key).or_insert_with(|| {
                    verts.push(key);
                    verts.len()
                });
            }
            if f[0] != f[1] && f[1] != f[2] && f[0] != f[2] {
                faces.push(f);
            }
        }
        let coords = verts
            .iter()
            .map(|v| {
                format!(
                    "({},{},{})",
                    r(v[0] as f64 / 100.0),
                    r(v[1] as f64 / 100.0),
                    r(v[2] as f64 / 100.0)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let list = self.add(format!("IFCCARTESIANPOINTLIST3D(({coords}))"));
        let idx = faces
            .iter()
            .map(|f| format!("({},{},{})", f[0], f[1], f[2]))
            .collect::<Vec<_>>()
            .join(",");
        let set = self.add(format!("IFCTRIANGULATEDFACESET(#{list},$,.T.,({idx}),$)"));
        let rep = self.add(format!(
            "IFCSHAPEREPRESENTATION(#{ctx},'Body','Tessellation',(#{set}))"
        ));
        self.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"))
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
    pub roofs: usize,
    pub stairs: usize,
    pub columns: usize,
    pub beams: usize,
    pub railings: usize,
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
    // The site's geographic location, when found (ADR-023).
    let located = doc
        .of(studio_core::Category::Site)
        .next()
        .and_then(|e| match &e.data {
            ElementData::Site {
                lat,
                lon,
                base_elevation,
                ..
            } => Some((*lat, *lon, *base_elevation)),
            _ => None,
        });
    let (ref_lat, ref_lon, ref_elev) = match located {
        Some((la, lo, z)) => (compound_angle(la), compound_angle(lo), r(z)),
        None => ("$".into(), "$".into(), "$".into()),
    };
    let site = w.add(format!(
        "IFCSITE({},$,'Site',$,$,#{site_place},$,$,.ELEMENT.,{ref_lat},{ref_lon},{ref_elev},$,$)",
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
    // Layered wall types → the walls using them.
    let mut layer_sets: std::collections::BTreeMap<ElementId, Vec<usize>> = Default::default();
    // The element's type, when it has a layer build-up.
    let layered_type = |id: ElementId| {
        let t = doc.data(id).ok()?.type_id()?;
        match doc.data(t).ok()? {
            ElementData::WallType { layers, .. }
            | ElementData::FloorType { layers, .. }
            | ElementData::CeilingType { layers, .. }
            | ElementData::RoofType { layers, .. }
                if !layers.is_empty() =>
            {
                Some(t)
            }
            _ => None,
        }
    };
    // The material of a column's or beam's type.
    let type_material = |id: ElementId| {
        let t = doc.data(id).ok().and_then(|d| d.type_id());
        match t.and_then(|t| doc.data(t).ok()) {
            Some(
                ElementData::ColumnType { name, material, .. }
                | ElementData::BeamType { name, material, .. },
            ) => studio_core::material::resolve_type(doc, *material, name).name,
            _ => String::new(),
        }
    };
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
        let shape = if wall.top_profile.is_some() {
            // Under a sloped roof: the top follows the roof.
            let tris =
                studio_geom::prism_triangles_to(&wall.footprint, wall.z0, |p| wall.top_at(p));
            w.tessellation(body, &tris, elev)
        } else {
            w.extrusion(
                body,
                &wall.footprint.outer,
                wall.z0 - elev,
                wall.z1 - wall.z0,
            )
        };
        let ty = type_name(wall.id);
        let e = w.add(format!(
            "IFCWALL({},$,{},$,{},#{place},#{shape},$,.STANDARD.)",
            s(&ifc_guid(wall.id.0)),
            s(&ty),
            s(&ty)
        ));
        contained.entry(storey).or_default().push(e);
        let wall_type = doc.data(wall.id).ok().and_then(|d| d.type_id());
        match wall_type.and_then(|t| doc.data(t).ok()) {
            Some(ElementData::WallType { layers, .. }) if !layers.is_empty() => layer_sets
                .entry(wall_type.unwrap_or(wall.id))
                .or_default()
                .push(e),
            _ => materials.entry(ty.clone()).or_default().push(e),
        }
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
                    OpeningKind::Door(style) => door_operation(style, o.flip_hand),
                    OpeningKind::Window(_) => ".NOTDEFINED.,$".into(),
                };
                w.add(format!(
                    "IFCDOOR({},$,{},$,{},#{fplace},#{fshape},{},{},{},.DOOR.,{op})",
                    s(&ifc_guid(o.id.0)),
                    s(&format!("Door {mark}")),
                    s(&ty),
                    s(&mark),
                    r(height),
                    r(width)
                ))
            } else {
                summary.windows += 1;
                let part = match o.kind {
                    OpeningKind::Window(style) => partitioning(style),
                    OpeningKind::Door(_) => ".SINGLE_PANEL.,$".into(),
                };
                w.add(format!(
                    "IFCWINDOW({},$,{},$,{},#{fplace},#{fshape},{},{},{},.WINDOW.,{part})",
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
        let mut seen_ids = std::collections::HashSet::new();
        for slab in slabs {
            if !seen_ids.insert(slab.id) {
                continue; // Further parts of a slab split by an opening.
            }
            let Some((_, storey, splace, elev)) = storey_of(slab.level) else {
                continue;
            };
            let place = w.placement(Some(splace), 0.0);
            let parts: Vec<(Poly, f64, f64)> = slabs
                .iter()
                .filter(|p| p.id == slab.id)
                .map(|p| (p.base.clone(), p.z0 - elev, p.z1 - p.z0))
                .collect();
            let shape = w.extrusions(body, &parts);
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
            match layered_type(slab.id) {
                Some(t) => layer_sets.entry(t).or_default().push(e),
                None => materials.entry(ty).or_default().push(e),
            }
        }
    }

    for roof in &model.roofs {
        let Some((_, storey, splace, elev)) = storey_of(roof.level) else {
            continue;
        };
        let place = w.placement(Some(splace), 0.0);
        let shape = w.tessellation(body, &roof.triangles(), elev);
        let ty = type_name(roof.id);
        let kind = if roof.is_flat() {
            ".FLAT_ROOF."
        } else if roof.faces.len() >= roof.boundary.len() {
            ".HIP_ROOF."
        } else {
            ".GABLE_ROOF."
        };
        let e = w.add(format!(
            "IFCROOF({},$,'Roof',$,{},#{place},#{shape},$,{kind})",
            s(&ifc_guid(roof.id.0)),
            s(&ty)
        ));
        contained.entry(storey).or_default().push(e);
        match layered_type(roof.id) {
            Some(t) => layer_sets.entry(t).or_default().push(e),
            None => materials.entry(ty).or_default().push(e),
        }
        summary.roofs += 1;
    }
    for stair in &model.stairs {
        let Some((_, storey, splace, elev)) = storey_of(stair.base_level) else {
            continue;
        };
        let place = w.placement(Some(splace), 0.0);
        let parts: Vec<(Poly, f64, f64)> = stair
            .steps
            .iter()
            .map(|p| (p.base.clone(), p.z0 - elev, p.z1 - p.z0))
            .collect();
        let shape = w.extrusions(body, &parts);
        let kind = match stair.runs.as_slice() {
            [a, b] if a.dir.dot(b.dir) < -0.5 => ".HALF_TURN_STAIR.",
            [_, _] => ".QUARTER_TURN_STAIR.",
            _ => ".STRAIGHT_RUN_STAIR.",
        };
        let e = w.add(format!(
            "IFCSTAIR({},$,'Stair',$,$,#{place},#{shape},$,{kind})",
            s(&ifc_guid(stair.id.0))
        ));
        contained.entry(storey).or_default().push(e);
        let risers = w.add(format!(
            "IFCPROPERTYSINGLEVALUE('NumberOfRiser',$,IFCCOUNTMEASURE({}),$)",
            stair.risers
        ));
        let riser = w.add(format!(
            "IFCPROPERTYSINGLEVALUE('RiserHeight',$,IFCPOSITIVELENGTHMEASURE({}),$)",
            r(stair.riser)
        ));
        let tread = w.add(format!(
            "IFCPROPERTYSINGLEVALUE('TreadLength',$,IFCPOSITIVELENGTHMEASURE({}),$)",
            r(stair.tread)
        ));
        let pset = w.add(format!(
            "IFCPROPERTYSET({},$,'Pset_StairCommon',$,(#{risers},#{riser},#{tread}))",
            s(&ifc_guid(derived(stair.id.0, "pset")))
        ));
        w.add(format!(
            "IFCRELDEFINESBYPROPERTIES({},$,$,$,(#{e}),#{pset})",
            s(&ifc_guid(derived(stair.id.0, "pset-rel")))
        ));
        summary.stairs += 1;
    }

    for col in &model.columns {
        let Some((_, storey, splace, elev)) = storey_of(col.level) else {
            continue;
        };
        let place = w.placement(Some(splace), 0.0);
        let shape = w.extrusions(body, &[(col.base.clone(), col.z0 - elev, col.z1 - col.z0)]);
        let ty = type_name(col.id);
        let e = w.add(format!(
            "IFCCOLUMN({},$,{},$,{},#{place},#{shape},$,.COLUMN.)",
            s(&ifc_guid(col.id.0)),
            s(&ty),
            s(&ty)
        ));
        contained.entry(storey).or_default().push(e);
        materials.entry(type_material(col.id)).or_default().push(e);
        let lb = w.add(format!(
            "IFCPROPERTYSINGLEVALUE('LoadBearing',$,IFCBOOLEAN({}),$)",
            if col.structural { ".T." } else { ".F." }
        ));
        let pset = w.add(format!(
            "IFCPROPERTYSET({},$,'Pset_ColumnCommon',$,(#{lb}))",
            s(&ifc_guid(derived(col.id.0, "pset")))
        ));
        w.add(format!(
            "IFCRELDEFINESBYPROPERTIES({},$,$,$,(#{e}),#{pset})",
            s(&ifc_guid(derived(col.id.0, "pset-rel")))
        ));
        summary.columns += 1;
    }
    for beam in &model.beams {
        let Some((_, storey, splace, elev)) = storey_of(beam.level) else {
            continue;
        };
        let place = w.placement(Some(splace), 0.0);
        let parts: Vec<(Poly, f64, f64)> = beam
            .prisms
            .iter()
            .map(|p| (p.base.clone(), p.z0 - elev, p.z1 - p.z0))
            .collect();
        let shape = w.extrusions(body, &parts);
        let ty = type_name(beam.id);
        let e = w.add(format!(
            "IFCBEAM({},$,{},$,{},#{place},#{shape},$,.BEAM.)",
            s(&ifc_guid(beam.id.0)),
            s(&ty),
            s(&ty)
        ));
        contained.entry(storey).or_default().push(e);
        materials.entry(type_material(beam.id)).or_default().push(e);
        summary.beams += 1;
    }
    for rail in &model.railings {
        let Some((_, storey, splace, elev)) = storey_of(rail.level) else {
            continue;
        };
        let on_stair = model.stairs.iter().any(|s| s.id == rail.id);
        let mut tris: Vec<f32> = rail
            .boxes
            .iter()
            .flat_map(studio_geom::box_triangles)
            .collect();
        tris.extend(rail.posts.iter().flat_map(|p| p.triangles()));
        if tris.is_empty() {
            continue;
        }
        let place = w.placement(Some(splace), 0.0);
        let shape = w.tessellation(body, &tris, elev);
        // A stair's handrails are their own railing, with a GUID derived from the stair.
        let (guid, name, kind) = if on_stair {
            (
                derived(rail.id.0, "railing"),
                "Handrail".to_owned(),
                ".HANDRAIL.",
            )
        } else {
            (rail.id.0, type_name(rail.id), ".GUARDRAIL.")
        };
        let e = w.add(format!(
            "IFCRAILING({},$,{},$,$,#{place},#{shape},$,{kind})",
            s(&ifc_guid(guid)),
            s(&name)
        ));
        contained.entry(storey).or_default().push(e);
        summary.railings += 1;
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

    // Compound types: one layer set each, exterior (or top) layer first.
    let mut layer_materials: HashMap<String, usize> = HashMap::new();
    for (type_id, items) in &layer_sets {
        let (name, layers) = match doc.data(*type_id) {
            Ok(
                ElementData::WallType { name, layers, .. }
                | ElementData::FloorType { name, layers, .. }
                | ElementData::CeilingType { name, layers, .. }
                | ElementData::RoofType { name, layers, .. },
            ) => (name, layers),
            _ => continue,
        };
        let mut ls = vec![];
        for l in layers {
            // The layer's material (ADR-020), so every layer of one material shares it.
            let mname = studio_core::material::resolve(doc, l).name;
            let m = match layer_materials.get(&mname) {
                Some(m) => *m,
                None => {
                    let m = w.add(format!("IFCMATERIAL({},$,$)", s(&mname)));
                    layer_materials.insert(mname.clone(), m);
                    m
                }
            };
            ls.push(w.add(format!(
                "IFCMATERIALLAYER(#{m},{},{},{},$,$,$)",
                r(l.thickness),
                if l.function == studio_core::LayerFunction::AirGap {
                    ".T."
                } else {
                    ".F."
                },
                s(l.function.label())
            )));
        }
        let set = w.add(format!(
            "IFCMATERIALLAYERSET(({}),{},$)",
            Writer::refs(&ls),
            s(name)
        ));
        w.add(format!(
            "IFCRELASSOCIATESMATERIAL({},$,$,$,({}),#{set})",
            s(&ifc_guid(derived(type_id.0, "layers"))),
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
        let l2 = doc.levels()[1].0;
        let rt = studio_core::build::default_roof_type(&doc).unwrap();
        let eave = studio_geom::offset_ring(&c, 300.0);
        studio_core::build::create_roof(&mut doc, rt, l2, 0.0, eave, 0.4).unwrap();
        studio_core::build::create_stair(
            &mut doc,
            l1,
            Pt::new(1000.0, 1000.0),
            Pt::new(1000.0, 2000.0),
            1000.0,
        )
        .unwrap();
        // The upper floor, opened by the stair; structure and a guardrail.
        let joist = doc
            .of(Category::FloorType)
            .find(|e| e.data.name().starts_with("Wood Joist"))
            .unwrap()
            .id;
        ops::create_floor(
            &mut doc,
            joist,
            l2,
            studio_regen::outer_boundary(&m, l1).unwrap(),
        )
        .unwrap();
        studio_core::structure::ensure_structure_types(&mut doc).unwrap();
        let named = |doc: &Document, cat: Category, name: &str| {
            doc.of(cat)
                .find(|e| e.data.name().starts_with(name))
                .unwrap()
                .id
        };
        let ct = named(&doc, Category::ColumnType, "Steel W10");
        studio_core::structure::create_column(&mut doc, ct, l1, Pt::new(4000.0, 3000.0), 0.0)
            .unwrap();
        let bt = named(&doc, Category::BeamType, "Glulam");
        studio_core::structure::create_beam(
            &mut doc,
            bt,
            l2,
            Pt::new(0.0, 3000.0),
            Pt::new(w, 3000.0),
        )
        .unwrap();
        let rt = named(&doc, Category::RailingType, "Guardrail");
        studio_core::structure::create_railing(
            &mut doc,
            rt,
            l2,
            vec![Pt::new(1600.0, 1000.0), Pt::new(1600.0, 5000.0)],
        )
        .unwrap();

        let (ifc, sum) = export_ifc(&doc, "0.0.1", "2026-09-24T00:00:00");
        assert_eq!(
            sum,
            IfcSummary {
                storeys: 2,
                walls: 4,
                doors: 1,
                windows: 1,
                slabs: 2,
                coverings: 0,
                spaces: 1,
                roofs: 1,
                stairs: 1,
                columns: 1,
                beams: 1,
                railings: 2,
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
            "IFCROOF(",
            ".HIP_ROOF.",
            "IFCTRIANGULATEDFACESET(",
            "IFCSTAIR(",
            "Pset_StairCommon",
            "IFCMATERIALLAYERSET(",
            "IFCCOLUMN(",
            "Pset_ColumnCommon",
            "IFCBEAM(",
            "IFCRAILING(",
            ".HANDRAIL.",
            "IFCARBITRARYPROFILEDEFWITHVOIDS(",
            "'Wood Joist Floor",
            "IFCMATERIAL('Structural Steel'",
            "IFCMATERIAL('Batt Insulation'",
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
