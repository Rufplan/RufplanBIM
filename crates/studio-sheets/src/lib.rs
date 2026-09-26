//! Sheets, title blocks, viewports, schedules and PDF export.

pub mod pdf;
pub mod schedule;
pub mod sets;
pub mod sheet;

pub use pdf::export_pdf;
pub use schedule::{schedule, Table};
pub use sheet::{
    drag_title, sheet_display_list, sheet_display_list_shared, sheet_handles, title_line,
};

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

    #[test]
    fn a_view_title_rule_stretches_and_is_kept() {
        let (mut doc, _, plan) = project();
        let sheet = ops::create_sheet(&mut doc, "Floor Plan", SheetSize::ArchD).unwrap();
        let vp = ops::place_view(&mut doc, sheet, plan, Pt::new(400.0, 300.0)).unwrap();
        let (a, b) = title_line(&doc, vp).unwrap();
        // Fitted to the title at first; its grip is at the rule's end.
        let fitted = b.x - a.x;
        assert!(
            fitted > sheet::MIN_TITLE_LENGTH && fitted < 120.0,
            "{fitted}"
        );
        let grips = sheet_handles(&doc, sheet, &[vp]).grips;
        assert_eq!(grips.len(), 1);
        assert_eq!(grips[0].key, "title_end");
        assert!(grips[0].at.dist(b) < 1e-9);
        // Dragged 150 mm out: the rule is that long on the sheet (and in the PDF).
        drag_title(&mut doc, vp, Pt::new(a.x + 150.0, a.y + 20.0)).unwrap();
        let (a2, b2) = title_line(&doc, vp).unwrap();
        assert!(a2.dist(a) < 1e-9);
        assert!((b2.x - a2.x - 150.0).abs() < 1e-9);
        let dl = sheet_display_list(&doc, sheet, "2026-09-26").unwrap();
        let rule = dl.items.iter().any(|it| {
            it.el == Some(vp)
                && matches!(&it.prim, Prim::Line { pts, w: 5, .. } if pts.len() == 2 && (pts[1][0] - pts[0][0] - 150.0).abs() < 1e-6)
        });
        assert!(rule, "the heavy rule is 150 mm long");
        // Never shorter than the least length; undo puts it back.
        drag_title(&mut doc, vp, Pt::new(a.x - 500.0, a.y)).unwrap();
        let (a3, b3) = title_line(&doc, vp).unwrap();
        assert!((b3.x - a3.x - sheet::MIN_TITLE_LENGTH).abs() < 1e-9);
        doc.undo().unwrap();
        doc.undo().unwrap();
        let (a4, b4) = title_line(&doc, vp).unwrap();
        assert!((b4.x - a4.x - fitted).abs() < 1e-9);
    }

    fn schedule_view(doc: &Document, name: &str) -> ElementId {
        doc.of(Category::View)
            .find(|e| e.data.name() == name)
            .unwrap()
            .id
    }

    #[test]
    fn structure_and_material_schedules() {
        let (mut doc, _, _) = project();
        let l1 = doc.levels()[0].0;
        let l2 = doc.levels()[1].0;
        let ft = MM_PER_FT;
        let g = ops::create_grid(
            &mut doc,
            Pt::new(4.0 * ft, -5.0 * ft),
            Pt::new(4.0 * ft, 20.0 * ft),
        )
        .unwrap();
        ops::set_property(&mut doc, g, "name", "2", 0).unwrap();
        let a = ops::create_grid(
            &mut doc,
            Pt::new(-5.0 * ft, 8.0 * ft),
            Pt::new(20.0 * ft, 8.0 * ft),
        )
        .unwrap();
        ops::set_property(&mut doc, a, "name", "C", 0).unwrap();
        let named = |doc: &Document, cat: Category, n: &str| {
            doc.of(cat)
                .find(|e| e.data.name().starts_with(n))
                .unwrap()
                .id
        };
        let ct = named(&doc, Category::ColumnType, "Steel W10");
        studio_core::structure::create_column(&mut doc, ct, l1, Pt::new(4.0 * ft, 8.0 * ft), 0.0)
            .unwrap();
        let bt = named(&doc, Category::BeamType, "Glulam");
        studio_core::structure::create_beam(
            &mut doc,
            bt,
            l2,
            Pt::new(0.0, 8.0 * ft),
            Pt::new(12.0 * ft, 8.0 * ft),
        )
        .unwrap();
        let c = schedule(&doc, schedule_view(&doc, "Structural Column Schedule")).unwrap();
        assert_eq!(
            c.rows,
            [vec!["C-2", "Steel W10x33", "Level 1", "Level 2", "10'-0\""]]
        );
        let b = schedule(&doc, schedule_view(&doc, "Structural Framing Schedule")).unwrap();
        assert_eq!(
            b.rows,
            [vec!["Glulam - 5 1/8\" x 12\"", "Level 2", "12'-0\""]]
        );
        let m = schedule(&doc, schedule_view(&doc, "Material Takeoff")).unwrap();
        assert_eq!(m.columns, ["Material", "Area", "Volume"]);
        let row = |n: &str| m.rows.iter().find(|r| r[0] == n).cloned().unwrap();
        // The glulam beam: 5 1/8" × 12" × 12' = 5.125 CF, volume only.
        assert_eq!(row("Glulam"), ["Glulam", "", "5.13 CF"]);
        assert!(row("Structural Steel")[2].ends_with(" CF"));
        assert!(
            !row("Gypsum Board")[1].is_empty(),
            "wall finishes have area"
        );
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
