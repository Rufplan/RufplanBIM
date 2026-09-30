//! The project manual (ADR-085): the MasterFormat section library, the sections the model
//! and Project Info call for, section styles, and PDF and Word output.

pub mod coord;
pub mod docx;
pub mod edit;
pub mod features;
pub mod generate;
pub mod layout;
pub mod library;
pub mod pdf;
pub mod style;
pub mod zip;

#[cfg(test)]
mod testkit;

#[cfg(test)]
mod samples {
    /// Writes target/specs-<style>.pdf and .docx for the test house, to look at.
    #[test]
    #[ignore]
    fn write_sample_books() {
        let doc = crate::testkit::house();
        let model = studio_regen::regenerate(&doc);
        let f = crate::features::facts(&doc, &model);
        let front = crate::generate::front(&doc);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
        for s in crate::style::styles() {
            let book = crate::generate::generate(&f, &s.id, "Issued for Permit", "2026-10-01");
            let laid = crate::layout::layout(&book, &s, &front, true);
            let pdf = crate::pdf::export(&laid, s.font, s.margin).unwrap();
            std::fs::write(dir.join(format!("specs-{}.pdf", s.id)), pdf).unwrap();
            std::fs::write(
                dir.join(format!("specs-{}.docx", s.id)),
                crate::docx::export(&laid, &s, "Project Manual"),
            )
            .unwrap();
            let svgs = crate::pdf::svg_pages(&laid, s.font, s.margin, &[0, 1, 3, 4, 30, 31, 120]);
            let html: String = svgs.iter().map(|x| format!("<div style=\"display:inline-block;margin:8px;box-shadow:0 0 4px #0004\">{x}</div>")).collect();
            std::fs::write(
                dir.join(format!("specs-{}.html", s.id)),
                format!("<html><body style=\"background:#999;margin:0\">{html}</body></html>"),
            )
            .unwrap();
            println!(
                "{}: {} sections, {} pages",
                s.id,
                book.sections.len(),
                crate::pdf::page_count(&laid, s.font, s.margin)
            );
        }
    }
}
