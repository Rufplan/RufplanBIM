//! Sheets, title blocks, viewports, schedules and PDF export.

pub mod pdf;
pub mod schedule;
pub mod sheet;

pub use pdf::export_pdf;
pub use schedule::{schedule, Table};
pub use sheet::{sheet_display_list, sheet_display_list_shared};

/// Version of this crate, from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::units::MM_PER_FT;
    use studio_core::{ops, Category, Document, ElementData, ElementId, SheetSize, ViewKind};
    use studio_geom::Pt;
    use studio_views::Prim;

    fn project() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new();
        ops::seed_default_project(&mut doc).unwrap();
        let l1 = doc.levels()[0].0;
        let wt = ops::first_of(&doc, Category::WallType).unwrap();
        let wall = ops::create_wall(
            &mut doc,
            wt,
            l1,
            Pt::new(0.0, 0.0),
            Pt::new(10.0 * MM_PER_FT, 0.0),
        )
        .unwrap();
        let dt = doc
            .of(Category::DoorType)
            .find(|e| e.data.name().starts_with("Single Flush 36"))
            .unwrap()
            .id;
        ops::create_door(&mut doc, dt, wall, 5.0 * MM_PER_FT, false).unwrap();
        let plan = doc
            .of(Category::View)
            .find(|e| matches!(&e.data, ElementData::View { kind: ViewKind::FloorPlan { level }, .. } if *level == l1))
            .unwrap()
            .id;
        (doc, wall, plan)
    }

    fn schedule_view(doc: &Document, name: &str) -> ElementId {
        doc.of(Category::View)
            .find(|e| e.data.name() == name)
            .unwrap()
            .id
    }

    #[test]
    fn door_schedule_lists_doors() {
        let (doc, _, _) = project();
        let t = schedule(&doc, schedule_view(&doc, "Door Schedule")).unwrap();
        assert_eq!(t.columns, ["Mark", "Type", "Width", "Height", "Level"]);
        assert_eq!(
            t.rows,
            [vec![
                "1",
                "Single Flush 36\" x 84\"",
                "3'-0\"",
                "7'-0\"",
                "Level 1"
            ]]
        );
    }

    #[test]
    fn viewport_draws_the_view_at_true_scale() {
        let (mut doc, wall, plan) = project();
        let sheet = ops::create_sheet(&mut doc, "Floor Plan", SheetSize::ArchD).unwrap();
        let vp = ops::place_view(&mut doc, sheet, plan, Pt::new(400.0, 300.0)).unwrap();
        let dl = sheet_display_list(&doc, sheet, "2026-09-24").unwrap();
        assert_eq!(dl.bounds, [0.0, 0.0, 914.4, 609.6]);
        // The 10'-0" wall's poché, printed at 1/4" = 1'-0", is 2 1/2" = 63.5 mm long.
        let model = studio_regen::regenerate(&doc);
        let footprint_len = {
            let w = model.walls.iter().find(|w| w.id == wall).unwrap();
            w.start.dist(w.end)
        };
        assert!((footprint_len - 3048.0).abs() < 1e-9);
        let poche: Vec<_> = dl
            .items
            .iter()
            .filter(|i| {
                i.el == Some(vp)
                    && matches!(
                        i.prim,
                        Prim::Fill {
                            fill: studio_views::FillKind::Poche
                                | studio_views::FillKind::PocheLight,
                            ..
                        }
                    )
            })
            .collect();
        let xs: Vec<f64> = poche
            .iter()
            .flat_map(|i| match &i.prim {
                Prim::Fill { rings, .. } => rings[0].iter().map(|p| p[0]).collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect();
        let span = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - xs.iter().copied().fold(f64::INFINITY, f64::min);
        // Centerline 63.5 mm; mitre-free square ends add nothing, so the span is the length.
        assert!((span - 63.5).abs() < 0.01, "span {span}");
        // Title block carries the sheet number and the current stage.
        let texts: Vec<String> = dl
            .items
            .iter()
            .filter_map(|i| match &i.prim {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"A1.0".to_string()));
        assert!(texts.contains(&"SD".to_string()));
        assert!(texts.contains(&"FLOOR PLAN".to_string()));
    }

    #[test]
    fn pdf_has_one_true_size_page_per_sheet_and_embeds_the_font() {
        let (mut doc, _, plan) = project();
        let a = ops::create_sheet(&mut doc, "Floor Plan", SheetSize::ArchD).unwrap();
        let b = ops::create_sheet(&mut doc, "Details", SheetSize::Tabloid).unwrap();
        ops::place_view(&mut doc, a, plan, Pt::new(400.0, 300.0)).unwrap();
        let doors = schedule_view(&doc, "Door Schedule");
        ops::place_view(&mut doc, b, doors, Pt::new(150.0, 150.0)).unwrap();
        let bytes = export_pdf(&doc, &[a, b], "2026-09-24").unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let text = String::from_utf8_lossy(&bytes);
        // 36" × 24" = 2592 × 1728 pt; 17" × 11" = 1224 × 792 pt.
        assert!(
            text.contains("2592") && text.contains("1728"),
            "ARCH D media box"
        );
        assert!(
            text.contains("1224") && text.contains("792"),
            "tabloid media box"
        );
        assert!(text.contains("BarlowCondensed"), "font embedded");
    }
}
