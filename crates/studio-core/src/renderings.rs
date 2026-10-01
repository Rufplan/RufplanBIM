//! Renderings saved to the project (ADR-095), as Revit's Save to Project: the image is an
//! element, shown by a Rendering view that can be placed on a sheet like any other.

use crate::document::{CoreError, CoreResult, Document};
use crate::element::{ElementData, ElementId, ViewKind};

/// The default printed width of a rendering on a sheet (mm): most of an ARCH D sheet's
/// drawing area.
pub const PAPER_WIDTH: f64 = 560.0;

/// Saves an image (base64 `data` of `mime`, `width` x `height` px) as a rendering and its
/// view named `name`; returns the view.
pub fn save(
    doc: &mut Document,
    name: &str,
    mime: &str,
    data: String,
    width: u32,
    height: u32,
) -> CoreResult<ElementId> {
    if width == 0 || height == 0 || data.is_empty() {
        return Err(CoreError::Invalid("the rendering is empty".into()));
    }
    if !matches!(mime, "image/jpeg" | "image/png") {
        return Err(CoreError::Invalid(format!("{mime} isn't a JPEG or PNG")));
    }
    let name = if name.trim().is_empty() {
        "Rendering".to_string()
    } else {
        name.trim().to_string()
    };
    doc.transact("Save rendering to project", |tx| {
        let image = tx.insert(ElementData::RenderImage {
            name: name.clone(),
            mime: mime.into(),
            data,
            width,
            height,
            paper_width: PAPER_WIDTH,
        });
        Ok(tx.insert(ElementData::view(
            name.clone(),
            ViewKind::Rendering { image },
            1,
        )))
    })
}

/// A Rendering view's image: (mime, base64 data, pixel width, height, paper width mm).
pub fn image_of(doc: &Document, view: ElementId) -> Option<(&str, &str, u32, u32, f64)> {
    let ElementData::View {
        kind: ViewKind::Rendering { image },
        ..
    } = doc.data(view).ok()?
    else {
        return None;
    };
    match doc.data(*image).ok()? {
        ElementData::RenderImage {
            mime,
            data,
            width,
            height,
            paper_width,
            ..
        } => Some((mime, data, *width, *height, *paper_width)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rendering_is_an_image_and_a_view() {
        let mut doc = Document::new();
        crate::ops::seed_default_project(&mut doc).unwrap();
        let v = save(&mut doc, "Hero", "image/jpeg", "AAAA".into(), 1600, 900).unwrap();
        let (mime, data, w, h, paper) = image_of(&doc, v).unwrap();
        assert_eq!((mime, data, w, h), ("image/jpeg", "AAAA", 1600, 900));
        assert!((paper - PAPER_WIDTH).abs() < 1e-9);
        assert_eq!(doc.data(v).unwrap().name(), "Hero");
        assert!(save(&mut doc, "x", "image/gif", "AAAA".into(), 1, 1).is_err());
    }
}
