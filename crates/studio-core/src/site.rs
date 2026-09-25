//! The site (ADR-023): a lot located on the earth, its parcel record, and a preliminary
//! topography sampled from USGS 3DEP elevation data (no survey).
//!
//! Geographic positions become project millimetres in a local east/north frame centered
//! on the lot (accurate to a few mm over a few hundred feet), then the site's Offset and
//! Angle to True North place that frame in the project.

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId, ViewKind};
use crate::ops::{len, ro, text, Property};
use crate::units::{format_ft_in, parse_length, MM_PER_FT};

/// What the parcel data says about the lot.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParcelInfo {
    /// Assessor's parcel number.
    pub apn: String,
    pub owner: String,
    pub address: String,
    pub acres: f64,
    /// Where the boundary came from, e.g. "Regrid".
    pub source: String,
}

/// A grid of ground elevations in the site's local frame (mm; absolute elevations in mm
/// above the NAVD88 datum, as 3DEP reports them).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Topo {
    pub x0: f64,
    pub y0: f64,
    pub spacing: f64,
    pub nx: u32,
    pub ny: u32,
    pub z: Vec<f32>,
    /// Finest source resolution in the samples, m (1 = lidar).
    pub resolution: f64,
}

impl Topo {
    pub fn at(&self, i: u32, j: u32) -> f64 {
        f64::from(self.z[(j * self.nx + i) as usize])
    }
    /// Local point of grid node (i, j).
    pub fn node(&self, i: u32, j: u32) -> Pt {
        Pt::new(
            self.x0 + f64::from(i) * self.spacing,
            self.y0 + f64::from(j) * self.spacing,
        )
    }
    /// Bilinear elevation at a local point, if it's on the grid.
    pub fn sample(&self, p: Pt) -> Option<f64> {
        let fx = (p.x - self.x0) / self.spacing;
        let fy = (p.y - self.y0) / self.spacing;
        if fx < 0.0 || fy < 0.0 || fx > f64::from(self.nx - 1) || fy > f64::from(self.ny - 1) {
            return None;
        }
        let (i, j) = (
            (fx.floor() as u32).min(self.nx - 2),
            (fy.floor() as u32).min(self.ny - 2),
        );
        let (tx, ty) = (fx - f64::from(i), fy - f64::from(j));
        let a = self.at(i, j) * (1.0 - tx) + self.at(i + 1, j) * tx;
        let b = self.at(i, j + 1) * (1.0 - tx) + self.at(i + 1, j + 1) * tx;
        Some(a * (1.0 - ty) + b * ty)
    }
}

/// A local east/north frame at (lat0, lon0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoFrame {
    pub lat0: f64,
    pub lon0: f64,
}

impl GeoFrame {
    /// Metres per degree of latitude and longitude here (WGS84).
    fn scale(&self) -> (f64, f64) {
        let p = self.lat0.to_radians();
        let lat = 111_132.92 - 559.82 * (2.0 * p).cos() + 1.175 * (4.0 * p).cos();
        let lon = 111_412.84 * p.cos() - 93.5 * (3.0 * p).cos();
        (lat, lon)
    }
    pub fn to_local(&self, lat: f64, lon: f64) -> Pt {
        let (mlat, mlon) = self.scale();
        Pt::new(
            (lon - self.lon0) * mlon * 1000.0,
            (lat - self.lat0) * mlat * 1000.0,
        )
    }
    pub fn to_geo(&self, p: Pt) -> (f64, f64) {
        let (mlat, mlon) = self.scale();
        (
            self.lat0 + p.y / 1000.0 / mlat,
            self.lon0 + p.x / 1000.0 / mlon,
        )
    }
}

/// The project's site element, if it has one.
pub fn site_of(doc: &Document) -> Option<ElementId> {
    doc.of(Category::Site).next().map(|e| e.id)
}

/// Local site frame → project plan coordinates (rotation, then offset).
pub fn to_project(offset: Pt, rotation: f64, p: Pt) -> Pt {
    let (s, c) = rotation.sin_cos();
    Pt::new(p.x * c - p.y * s, p.x * s + p.y * c).add(offset)
}

/// Sets the lot from its boundary in (lat, lon): the site is centered on the lot. Adds a
/// Site plan view the first time.
pub fn set_lot(
    doc: &mut Document,
    ring: &[(f64, f64)],
    parcel: ParcelInfo,
) -> CoreResult<ElementId> {
    if ring.len() < 3 {
        return Err(CoreError::Invalid(
            "the lot needs at least three corners".into(),
        ));
    }
    let n = ring.len() as f64;
    let (lat0, lon0) = ring
        .iter()
        .fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
    let frame = GeoFrame { lat0, lon0 };
    let mut boundary: Vec<Pt> = ring
        .iter()
        .map(|(la, lo)| frame.to_local(*la, *lo))
        .collect();
    if boundary.len() > 3 && boundary[0].dist(boundary[boundary.len() - 1]) < 1.0 {
        boundary.pop();
    }
    if studio_geom::signed_area(&boundary) < 0.0 {
        boundary.reverse();
    }
    let existing = site_of(doc);
    let lowest = doc
        .levels()
        .into_iter()
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|l| l.0)
        .ok_or_else(|| CoreError::Invalid("add a level first".into()))?;
    let has_view = doc
        .iter()
        .any(|e| matches!(&e.data, ElementData::View { site: true, .. }));
    doc.transact("Set lot", |tx| {
        let data = ElementData::Site {
            address: parcel.address.clone(),
            lat: lat0,
            lon: lon0,
            boundary: boundary.clone(),
            parcel: parcel.clone(),
            offset: Pt::default(),
            rotation: 0.0,
            base_elevation: 0.0,
            contour: MM_PER_FT,
            topo: None,
        };
        let id = match existing {
            Some(id) => {
                tx.set(id, data)?;
                id
            }
            None => tx.insert(data),
        };
        if !has_view {
            let mut v = ElementData::view("Site", ViewKind::FloorPlan { level: lowest }, 240);
            if let ElementData::View { site, .. } = &mut v {
                *site = true;
            }
            tx.insert(v);
        }
        Ok(id)
    })
}

/// A topo sampling grid: (nx, ny, lower-left corner in the local frame, (lat, lon) of each
/// node row by row).
pub type TopoRequest = (u32, u32, Pt, Vec<(f64, f64)>);

/// Where to sample elevations: a grid `spacing` apart over the lot plus `margin` around
/// it, as (nx, ny, x0, y0) in the local frame, and the (lat, lon) of each node row by row.
pub fn topo_request(doc: &Document, spacing: f64, margin: f64) -> CoreResult<TopoRequest> {
    let g = topo_grid(doc, spacing, margin, 1.0)?;
    Ok((g.nx, g.ny, g.origin, g.pts))
}

/// Most elevation samples in one topography (about 40 requests to 3DEP).
pub const MAX_TOPO_POINTS: u64 = 40_000;

/// A topography sampling grid (ADR-026).
#[derive(Debug, Clone, PartialEq)]
pub struct TopoGrid {
    pub nx: u32,
    pub ny: u32,
    /// Lower-left node in the local frame.
    pub origin: Pt,
    /// Node spacing actually used (mm): coarser than asked when the area needs it.
    pub spacing: f64,
    /// (lat, lon) of each node, row by row.
    pub pts: Vec<(f64, f64)>,
}

/// The area a topography covers in the local frame: the lot's bounds when `extent` is 1;
/// for 2 to 4, a square `extent` times the lot's longer side, centered on the lot (like
/// zooming a map out). Then `margin` all round.
pub fn topo_area(boundary: &[Pt], margin: f64, extent: f64) -> Option<(Pt, Pt)> {
    let (lo, hi) = studio_geom::bounds_of(boundary)?;
    let extent = extent.clamp(1.0, 4.0);
    let (lo, hi) = if extent > 1.0 {
        let c = Pt::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
        let half = (hi.x - lo.x).max(hi.y - lo.y) * extent / 2.0;
        (c.sub(Pt::new(half, half)), c.add(Pt::new(half, half)))
    } else {
        (lo, hi)
    };
    Some((
        lo.sub(Pt::new(margin, margin)),
        hi.add(Pt::new(margin, margin)),
    ))
}

/// The sampling grid over [`topo_area`]. A grid over `MAX_TOPO_POINTS` doubles its
/// spacing until it fits.
pub fn topo_grid(doc: &Document, spacing: f64, margin: f64, extent: f64) -> CoreResult<TopoGrid> {
    let Some(ElementData::Site {
        lat, lon, boundary, ..
    }) = site_of(doc).and_then(|id| doc.data(id).ok())
    else {
        return Err(CoreError::Invalid("find the lot first".into()));
    };
    if spacing < 300.0 {
        return Err(CoreError::Invalid("use a grid of at least 1'".into()));
    }
    let (lo, hi) = topo_area(boundary, margin, extent)
        .ok_or_else(|| CoreError::Invalid("the lot has no boundary".into()))?;
    let mut spacing = spacing;
    let (nx, ny) = loop {
        let nx = ((hi.x - lo.x) / spacing).ceil() as u32 + 1;
        let ny = ((hi.y - lo.y) / spacing).ceil() as u32 + 1;
        if u64::from(nx) * u64::from(ny) <= MAX_TOPO_POINTS {
            break (nx, ny);
        }
        spacing *= 2.0;
    };
    let frame = GeoFrame {
        lat0: *lat,
        lon0: *lon,
    };
    let mut pts = vec![];
    for j in 0..ny {
        for i in 0..nx {
            let p = lo.add(Pt::new(f64::from(i) * spacing, f64::from(j) * spacing));
            pts.push(frame.to_geo(p));
        }
    }
    Ok(TopoGrid {
        nx,
        ny,
        origin: lo,
        spacing,
        pts,
    })
}

/// A satellite image covering the site (ADR-026): what to ask Google's Static Maps for, and
/// where the image's corners fall in the project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImageryFrame {
    /// Image center.
    pub lat: f64,
    pub lon: f64,
    /// Web Mercator zoom level.
    pub zoom: u32,
    /// Size in map pixels (the image is fetched at twice this).
    pub width: u32,
    pub height: u32,
    /// The image's corners in project plan coordinates (mm): lower left, lower right, upper
    /// right, upper left.
    pub corners: [Pt; 4],
}

/// Largest Static Maps image side, in map pixels.
pub const MAX_IMAGE_PX: u32 = 640;

/// Ground metres per map pixel at `zoom` and latitude `lat` (Web Mercator).
pub fn meters_per_pixel(lat: f64, zoom: u32) -> f64 {
    156_543.033_92 * lat.to_radians().cos() / f64::from(1u32 << zoom)
}

/// The image covering the topography (or the lot and 25' around it, before there is one):
/// the closest zoom that fits it in one image.
pub fn imagery_frame(doc: &Document) -> CoreResult<ImageryFrame> {
    let Some(ElementData::Site {
        lat,
        lon,
        boundary,
        offset,
        rotation,
        topo,
        ..
    }) = site_of(doc).and_then(|id| doc.data(id).ok())
    else {
        return Err(CoreError::Invalid("find the lot first".into()));
    };
    let (lo, hi) = match topo {
        Some(t) => (
            Pt::new(t.x0, t.y0),
            Pt::new(
                t.x0 + f64::from(t.nx - 1) * t.spacing,
                t.y0 + f64::from(t.ny - 1) * t.spacing,
            ),
        ),
        None => topo_area(boundary, 25.0 * MM_PER_FT, 1.0)
            .ok_or_else(|| CoreError::Invalid("the lot has no boundary".into()))?,
    };
    let c = Pt::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let (w, h) = ((hi.x - lo.x) / 1000.0, (hi.y - lo.y) / 1000.0);
    let fits = |z: u32| {
        let m = meters_per_pixel(*lat, z);
        w / m <= f64::from(MAX_IMAGE_PX) && h / m <= f64::from(MAX_IMAGE_PX)
    };
    let zoom = (1..=21).rev().find(|z| fits(*z)).unwrap_or(1);
    let m = meters_per_pixel(*lat, zoom);
    let px = |d: f64| ((d / m).ceil() as u32).clamp(16, MAX_IMAGE_PX);
    let (width, height) = (px(w), px(h));
    let (hw, hh) = (f64::from(width) * m * 500.0, f64::from(height) * m * 500.0);
    let frame = GeoFrame {
        lat0: *lat,
        lon0: *lon,
    };
    let (clat, clon) = frame.to_geo(c);
    let corner = |dx: f64, dy: f64| to_project(*offset, *rotation, c.add(Pt::new(dx, dy)));
    Ok(ImageryFrame {
        lat: clat,
        lon: clon,
        zoom,
        width,
        height,
        corners: [
            corner(-hw, -hh),
            corner(hw, -hh),
            corner(hw, hh),
            corner(-hw, hh),
        ],
    })
}

/// Stores sampled elevations (m; None where 3DEP has no data) as the site's topography.
/// Gaps take the nearest sample. The first topo sets the project base elevation to the
/// ground at the lot's center.
pub fn set_topo(
    doc: &mut Document,
    (nx, ny, origin): (u32, u32, Pt),
    spacing: f64,
    meters: &[Option<f64>],
    resolution: f64,
) -> CoreResult<()> {
    let id = site_of(doc).ok_or_else(|| CoreError::Invalid("find the lot first".into()))?;
    if meters.len() != (nx * ny) as usize {
        return Err(CoreError::Invalid(
            "elevation samples don't fit the grid".into(),
        ));
    }
    let known: Vec<(u32, u32, f64)> = (0..ny)
        .flat_map(|j| (0..nx).map(move |i| (i, j)))
        .zip(meters)
        .filter_map(|((i, j), m)| m.map(|v| (i, j, v)))
        .collect();
    if known.is_empty() {
        return Err(CoreError::Invalid(
            "no elevation data for this location".into(),
        ));
    }
    let mut z = Vec::with_capacity(meters.len());
    for j in 0..ny {
        for i in 0..nx {
            let v = meters[(j * nx + i) as usize].unwrap_or_else(|| {
                known
                    .iter()
                    .min_by_key(|(a, b, _)| {
                        let (dx, dy) = (i64::from(*a) - i64::from(i), i64::from(*b) - i64::from(j));
                        dx * dx + dy * dy
                    })
                    .map_or(0.0, |k| k.2)
            });
            z.push((v * 1000.0) as f32);
        }
    }
    let topo = Topo {
        x0: origin.x,
        y0: origin.y,
        spacing,
        nx,
        ny,
        z,
        resolution,
    };
    let center = topo.sample(Pt::default());
    doc.transact("Get topography", |tx| {
        tx.modify(id, |d| {
            if let ElementData::Site {
                topo: t,
                base_elevation,
                ..
            } = d
            {
                if t.is_none() {
                    if let Some(c) = center {
                        *base_elevation = (c / 25.4).round() * 25.4;
                    }
                }
                *t = Some(topo.clone());
            }
        })
    })
}

fn degrees(r: f64) -> String {
    format!("{:.2}°", r.to_degrees())
}

pub(crate) fn properties(doc: &Document, id: ElementId, props: &mut Vec<Property>) {
    let Ok(ElementData::Site {
        address,
        lat,
        lon,
        boundary,
        parcel,
        offset,
        rotation,
        base_elevation,
        contour,
        topo,
    }) = doc.data(id)
    else {
        return;
    };
    const L: &str = "Location";
    props.push(ro("address", "Address", L, address.clone()));
    props.push(ro(
        "latlon",
        "Latitude, Longitude",
        L,
        format!("{lat:.6}, {lon:.6}"),
    ));
    props.push(ro(
        "apn",
        "Parcel Number (APN)",
        "Parcel",
        parcel.apn.clone(),
    ));
    props.push(ro(
        "owner",
        "Owner of Record",
        "Parcel",
        parcel.owner.clone(),
    ));
    let area_sf = studio_geom::signed_area(boundary).abs() / (MM_PER_FT * MM_PER_FT);
    props.push(ro(
        "area",
        "Lot Area",
        "Parcel",
        format!("{:.0} SF ({:.3} ac)", area_sf, area_sf / 43_560.0),
    ));
    props.push(ro(
        "source",
        "Boundary Source",
        "Parcel",
        parcel.source.clone(),
    ));
    props.push(len("offset_e", "Offset East", "Placement", offset.x));
    props.push(len("offset_n", "Offset North", "Placement", offset.y));
    props.push(text(
        "rotation",
        "Angle to True North",
        "Placement",
        &degrees(*rotation),
    ));
    props.push(len(
        "base_elevation",
        "Level 1 Elevation (NAVD88)",
        "Topography",
        *base_elevation,
    ));
    props.push(len("contour", "Contour Interval", "Topography", *contour));
    match topo {
        Some(t) => {
            props.push(ro(
                "grid",
                "Topo Grid",
                "Topography",
                format!(
                    "{} × {} points, {} apart",
                    t.nx,
                    t.ny,
                    format_ft_in(t.spacing)
                ),
            ));
            props.push(ro(
                "resolution",
                "Source",
                "Topography",
                format!(
                    "USGS 3DEP, {} m data (preliminary; not a survey)",
                    t.resolution
                ),
            ));
        }
        None => props.push(ro("grid", "Topo Grid", "Topography", "None yet".into())),
    }
}

pub(crate) fn set_property(
    doc: &mut Document,
    id: ElementId,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    let bad_len = || CoreError::Invalid(format!("\"{value}\" is not a length"));
    let v_len = || parse_length(value).ok_or_else(bad_len);
    let mut d = doc.data(id)?.clone();
    let ElementData::Site {
        offset,
        rotation,
        base_elevation,
        contour,
        ..
    } = &mut d
    else {
        return Err(CoreError::Invalid("not a site".into()));
    };
    match key {
        "offset_e" => offset.x = v_len()?,
        "offset_n" => offset.y = v_len()?,
        "rotation" => {
            let deg: f64 = value
                .trim()
                .trim_end_matches('°')
                .trim()
                .parse()
                .map_err(|_| {
                    CoreError::Invalid(format!("\"{value}\" is not an angle in degrees"))
                })?;
            *rotation = deg.to_radians();
        }
        "base_elevation" => *base_elevation = v_len()?,
        "contour" => {
            let c = v_len()?;
            if c < 25.0 {
                return Err(CoreError::Invalid(
                    "use a contour interval of at least 1\"".into(),
                ));
            }
            *contour = c;
        }
        _ => return Err(CoreError::Invalid(format!("unknown property {key}"))),
    }
    doc.transact(&format!("Change site {key}"), |tx| tx.set(id, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_frame_round_trips_and_scales_like_wgs84() {
        let f = GeoFrame {
            lat0: 37.7749,
            lon0: -122.4194,
        };
        let p = f.to_local(37.7758, -122.4180);
        let (la, lo) = f.to_geo(p);
        assert!((la - 37.7758).abs() < 1e-9 && (lo + 122.4180).abs() < 1e-9);
        // 0.0009° of latitude ≈ 99.9 m; 0.0014° of longitude at 37.77° ≈ 123.4 m.
        assert!((p.y / 1000.0 - 99.93).abs() < 0.1, "{}", p.y);
        assert!((p.x / 1000.0 - 123.30).abs() < 0.2, "{}", p.x);
    }

    #[test]
    fn lot_topo_and_properties() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        // A lot about 30 m × 40 m.
        let ring = [
            (37.7749, -122.4194),
            (37.7749, -122.41906),
            (37.77526, -122.41906),
            (37.77526, -122.4194),
        ];
        let parcel = ParcelInfo {
            apn: "3702-001".into(),
            owner: "Owner".into(),
            address: "1 Main St".into(),
            acres: 0.3,
            source: "Regrid".into(),
        };
        let id = set_lot(&mut doc, &ring, parcel).unwrap();
        assert!(doc.iter().any(|e| matches!(
            &e.data,
            ElementData::View {
                site: true,
                scale: 240,
                ..
            }
        )));
        let ElementData::Site { boundary, .. } = doc.data(id).unwrap() else {
            panic!()
        };
        let area = studio_geom::signed_area(boundary);
        assert!(
            area > 0.0 && (area / 1e6 - 30.0 * 40.0).abs() < 25.0,
            "{}",
            area / 1e6
        );
        // A 10' grid with 20' around the lot.
        let (nx, ny, origin, pts) = topo_request(&doc, 10.0 * MM_PER_FT, 20.0 * MM_PER_FT).unwrap();
        assert_eq!(pts.len() as u32, nx * ny);
        // A plane rising 1 m per 10 m east, with one gap.
        let frame = GeoFrame {
            lat0: ring.iter().map(|p| p.0).sum::<f64>() / 4.0,
            lon0: ring.iter().map(|p| p.1).sum::<f64>() / 4.0,
        };
        let mut m: Vec<Option<f64>> = pts
            .iter()
            .map(|(la, lo)| Some(20.0 + frame.to_local(*la, *lo).x / 10_000.0))
            .collect();
        m[0] = None;
        set_topo(&mut doc, (nx, ny, origin), 10.0 * MM_PER_FT, &m, 1.0).unwrap();
        let ElementData::Site {
            topo: Some(t),
            base_elevation,
            ..
        } = doc.data(id).unwrap()
        else {
            panic!()
        };
        // Level 1 sits on the ground at the lot's center: 20 m, rounded to the inch.
        assert!((base_elevation - 20_000.0).abs() < 13.0, "{base_elevation}");
        assert!((t.sample(Pt::new(5000.0, 0.0)).unwrap() - 20_500.0).abs() < 1.0);
        set_property(&mut doc, id, "rotation", "12.5").unwrap();
        set_property(&mut doc, id, "contour", "2'").unwrap();
        let ElementData::Site {
            rotation, contour, ..
        } = doc.data(id).unwrap()
        else {
            panic!()
        };
        assert!(
            (rotation.to_degrees() - 12.5).abs() < 1e-9 && (contour - 2.0 * MM_PER_FT).abs() < 1e-9
        );
        assert!(topo_request(&doc, 100.0, 0.0).is_err(), "too fine");
    }

    #[test]
    fn wider_areas_and_the_satellite_image_frame() {
        // A 30' x 60' lot: 2x is a 120' square around it, 4x a 240' square.
        let lot = [
            Pt::new(0.0, 0.0),
            Pt::new(30.0 * MM_PER_FT, 0.0),
            Pt::new(30.0 * MM_PER_FT, 60.0 * MM_PER_FT),
            Pt::new(0.0, 60.0 * MM_PER_FT),
        ];
        let (lo, hi) = topo_area(&lot, 0.0, 1.0).unwrap();
        assert_eq!((lo, hi), (lot[0], lot[2]));
        let (lo, hi) = topo_area(&lot, 10.0 * MM_PER_FT, 2.0).unwrap();
        assert!((hi.x - lo.x - 140.0 * MM_PER_FT).abs() < 1e-6);
        assert!((hi.y - lo.y - 140.0 * MM_PER_FT).abs() < 1e-6);
        assert!(((lo.x + hi.x) / 2.0 - 15.0 * MM_PER_FT).abs() < 1e-6);
        let (lo, hi) = topo_area(&lot, 0.0, 4.0).unwrap();
        assert!((hi.y - lo.y - 240.0 * MM_PER_FT).abs() < 1e-6);
        // Past 4x stays 4x.
        assert_eq!(topo_area(&lot, 0.0, 9.0), topo_area(&lot, 0.0, 4.0));

        // Web Mercator: 0.2986 m per pixel at zoom 19 on the equator.
        assert!((meters_per_pixel(0.0, 19) - 0.298_582).abs() < 1e-5);

        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        // About 100' x 100' in San Francisco.
        let (la, lo) = (37.7773, -122.4620);
        let (dla, dlo) = (30.48 / 111_000.0, 30.48 / 88_000.0);
        let ring = [
            (la, lo),
            (la, lo + dlo),
            (la + dla, lo + dlo),
            (la + dla, lo),
        ];
        let id = set_lot(&mut doc, &ring, ParcelInfo::default()).unwrap();
        let f = imagery_frame(&doc).unwrap();
        // 150' (lot and 25' around) fits in 640 px at zoom 20 (0.118 m/px), not 21.
        assert_eq!(f.zoom, 20);
        let m = meters_per_pixel(37.7773, 20);
        assert!(f.width <= MAX_IMAGE_PX && f64::from(f.width) * m >= 45.0);
        // The corners enclose the lot, centered on it, with north up.
        let [ll, lr, ur, ul] = f.corners;
        assert!((ll.y - lr.y).abs() < 1e-6 && (ll.x - ul.x).abs() < 1e-6);
        assert!(((ll.x + ur.x) / 2.0).abs() < 50.0 && ((ll.y + ur.y) / 2.0).abs() < 50.0);
        assert!(ur.x - ll.x >= 45_720.0);
        // Rotating the site (Angle to True North) turns the image with it.
        set_property(&mut doc, id, "rotation", "90").unwrap();
        let g = imagery_frame(&doc).unwrap();
        assert!((g.corners[0].x - (-ll.y)).abs() < 1.0 && (g.corners[0].y - ll.x).abs() < 1.0);
        // A 4x topography: the image covers it at a wider zoom.
        let g = topo_grid(&doc, 5.0 * MM_PER_FT, 0.0, 4.0).unwrap();
        assert_eq!(g.pts.len() as u32, g.nx * g.ny);
        assert!(u64::from(g.nx * g.ny) <= MAX_TOPO_POINTS);
        let meters = vec![Some(20.0); g.pts.len()];
        set_topo(&mut doc, (g.nx, g.ny, g.origin), g.spacing, &meters, 1.0).unwrap();
        assert!(imagery_frame(&doc).unwrap().zoom < f.zoom);
    }
}
