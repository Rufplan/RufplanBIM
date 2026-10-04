//! Location and vicinity maps on sheets (ADR-107), as architects put Google Maps images of
//! the site on the cover with the address over them. The project keeps only each map's
//! frame (where it is, what it centres on, how far out); the image is fetched from
//! Google's Maps Static API whenever the sheet is shown or printed, never stored, as
//! Google's terms require (ADR-026).

use serde::{Deserialize, Serialize};
use studio_geom::Pt;
use ts_rs::TS;

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{Category, ElementData, ElementId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum MapKind {
    /// The lot and its neighbours, from above (satellite with labels).
    Location,
    /// The neighbourhood's streets round the site (road map).
    Vicinity,
}

impl MapKind {
    pub fn label(self) -> &'static str {
        match self {
            MapKind::Location => "Location Map",
            MapKind::Vicinity => "Vicinity Map",
        }
    }
    /// Google's zoom: the block for a location map, about a mile across for a vicinity map.
    pub fn zoom(self) -> u32 {
        match self {
            MapKind::Location => 18,
            MapKind::Vicinity => 14,
        }
    }
    /// The Static Maps `maptype`.
    pub fn maptype(self) -> &'static str {
        match self {
            MapKind::Location => "hybrid",
            MapKind::Vicinity => "roadmap",
        }
    }
}

/// Where the project is: the site's latitude and longitude, when it has been located.
pub fn site_location(doc: &Document) -> Option<(f64, f64)> {
    doc.of(Category::Site).find_map(|e| match &e.data {
        ElementData::Site { lat, lon, .. } if lat.abs() > 1e-9 || lon.abs() > 1e-9 => {
            Some((*lat, *lon))
        }
        _ => None,
    })
}

/// The address printed over a map: Project Info's, else the site's.
pub fn address(doc: &Document) -> String {
    let from_info = crate::project::get(doc)
        .map(|(_, d)| {
            let l = d.location;
            let city = [l.city.trim(), l.state.trim()]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            [l.street.trim(), format!("{city} {}", l.zip.trim()).trim()]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    if !from_info.trim().is_empty() {
        return from_info;
    }
    doc.of(Category::Site)
        .find_map(|e| match &e.data {
            ElementData::Site { address, .. } => Some(address.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Places a map of `kind` on `sheet` in the box `min`–`max` (paper mm), centred on the site.
pub fn place(
    doc: &mut Document,
    sheet: ElementId,
    kind: MapKind,
    min: Pt,
    max: Pt,
) -> CoreResult<ElementId> {
    if !matches!(doc.data(sheet), Ok(ElementData::Sheet { .. })) {
        return Err(CoreError::Invalid("maps go on sheets".into()));
    }
    let (lat, lon) = site_location(doc)
        .ok_or_else(|| CoreError::Invalid("locate the project on the Site tab first".into()))?;
    let label = address(doc);
    doc.transact("Place map", |tx| {
        Ok(tx.insert(ElementData::MapFrame {
            sheet,
            kind,
            min,
            max,
            lat,
            lon,
            zoom: kind.zoom(),
            label: label.clone(),
        }))
    })
}

/// The Static Maps image size for a frame: its paper aspect at up to 640 px (the API's
/// largest; fetched at scale 2).
pub fn pixels(min: Pt, max: Pt) -> (u32, u32) {
    let (w, h) = ((max.x - min.x).max(1.0), (max.y - min.y).max(1.0));
    let k = 640.0 / w.max(h);
    (
        ((w * k).round() as u32).max(1),
        ((h * k).round() as u32).max(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_needs_a_located_site_and_keeps_only_its_frame() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let sheet = crate::ops::create_sheet(&mut doc, "Cover", crate::SheetSize::ArchD).unwrap();
        let (a, b) = (Pt::new(10.0, 10.0), Pt::new(170.0, 110.0));
        assert!(place(&mut doc, sheet, MapKind::Vicinity, a, b).is_err());
        doc.transact("site", |tx| {
            Ok(tx.insert(ElementData::Site {
                address: "1 Main St".into(),
                lat: 37.42,
                lon: -122.13,
                boundary: vec![],
                parcel: Default::default(),
                offset: Pt::default(),
                rotation: 0.0,
                base_elevation: 0.0,
                contour: 304.8,
                topo: None,
            }))
        })
        .unwrap();
        let m = place(&mut doc, sheet, MapKind::Vicinity, a, b).unwrap();
        match doc.data(m).unwrap() {
            ElementData::MapFrame {
                zoom, label, lat, ..
            } => {
                assert_eq!(*zoom, 14);
                assert_eq!(label, "1 Main St");
                assert!((lat - 37.42).abs() < 1e-9);
            }
            _ => panic!(),
        }
        assert_eq!(pixels(a, b), (640, 400));
    }
}
