//! Spec section styles: how the book is numbered, set and laid out. The PDF and Word
//! writers and the editor all take their look from one of these.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How articles and paragraphs are numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "SpecNumbering")]
pub enum Numbering {
    /// PART 1 - GENERAL, 1.1 SUMMARY, A., 1., a., 1) (CSI SectionFormat).
    Csi,
    /// As Csi with two-digit articles: 1.01, 1.02 (the older CSI habit).
    CsiZero,
    /// 1 GENERAL, 1.1 SUMMARY, 1.1.1, 1.1.1.1 (legal / decimal).
    Decimal,
    /// Headings and bullets, no numbers (narrative and outline specs).
    Bullets,
}

/// The type family: a Times-metric serif, a Calibri-metric sans, or the Rufplan face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "SpecFont")]
pub enum Font {
    Serif,
    Sans,
    Brand,
}

impl Font {
    /// The name Word asks for (the metric-compatible fonts it has on Windows).
    pub fn word_name(self) -> &'static str {
        match self {
            Font::Serif => "Times New Roman",
            Font::Sans => "Calibri",
            Font::Brand => "Barlow",
        }
    }
    /// The CSS stack the editor uses.
    pub fn css(self) -> &'static str {
        match self {
            Font::Serif => "\"Times New Roman\", Tinos, Times, serif",
            Font::Sans => "Calibri, Carlito, \"Segoe UI\", Arial, sans-serif",
            Font::Brand => "Barlow, \"Segoe UI\", sans-serif",
        }
    }
}

/// How a section's title is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "SpecHeading")]
pub enum Heading {
    /// "SECTION 09 29 00 - GYPSUM BOARD", centered.
    Centered,
    /// Number and title on the left, with a rule under.
    Ruled,
    /// A black band with the number and title in white.
    Banner,
}

/// Which content goes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "SpecContent")]
pub enum Content {
    /// Everything: the full three-part sections.
    Full,
    /// Short form: the summary, products (two levels) and installation.
    Short,
    /// Narrative (SD/DD): each section's products as bullets, by division.
    Narrative,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecStyle {
    pub id: String,
    pub name: String,
    pub description: String,
    pub numbering: Numbering,
    pub font: Font,
    /// Body size, pt.
    pub size: f32,
    pub heading: Heading,
    pub content: Content,
    /// Article titles in bold.
    pub bold_articles: bool,
    /// Each section on new pages, numbered "09 29 00 - 1" (else one run of pages).
    pub page_per_section: bool,
    /// Left and right margins, inches.
    pub margin: f32,
    /// Space between paragraphs, pt.
    pub gap: f32,
}

pub fn styles() -> Vec<SpecStyle> {
    let s = |id: &str, name: &str, description: &str| SpecStyle {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        numbering: Numbering::Csi,
        font: Font::Serif,
        size: 10.0,
        heading: Heading::Centered,
        content: Content::Full,
        bold_articles: false,
        page_per_section: true,
        margin: 1.0,
        gap: 6.0,
    };
    vec![
        SpecStyle {
            numbering: Numbering::CsiZero,
            ..s(
                "csi-classic",
                "CSI Classic",
                "CSI SectionFormat and PageFormat in a Times-style serif: centered section titles, 1.01 articles, pages numbered by section.",
            )
        },
        SpecStyle {
            font: Font::Sans,
            size: 10.5,
            heading: Heading::Ruled,
            bold_articles: true,
            margin: 0.9,
            ..s(
                "csi-modern",
                "CSI Modern",
                "MasterSpec-style 1.1 numbering in a clean sans, bold article titles and a ruled section heading.",
            )
        },
        SpecStyle {
            numbering: Numbering::Decimal,
            font: Font::Sans,
            size: 10.0,
            heading: Heading::Ruled,
            bold_articles: true,
            ..s(
                "decimal",
                "Decimal Outline",
                "Legal numbering throughout (1.1, 1.1.1, 1.1.1.1), as many public agencies and engineers issue specs.",
            )
        },
        SpecStyle {
            font: Font::Brand,
            size: 10.0,
            heading: Heading::Banner,
            bold_articles: true,
            margin: 0.85,
            ..s(
                "rufplan",
                "Rufplan",
                "The Rufplan look: Barlow type, black section banners, 1.1 numbering.",
            )
        },
        SpecStyle {
            font: Font::Sans,
            size: 9.5,
            heading: Heading::Ruled,
            content: Content::Short,
            bold_articles: true,
            page_per_section: false,
            gap: 4.0,
            ..s(
                "short-form",
                "Short Form",
                "Outline specifications for small projects and early packages: summary, products and installation, sections run on.",
            )
        },
        SpecStyle {
            numbering: Numbering::Bullets,
            font: Font::Sans,
            size: 10.5,
            heading: Heading::Ruled,
            content: Content::Narrative,
            bold_articles: true,
            page_per_section: false,
            gap: 4.0,
            ..s(
                "narrative",
                "Narrative (SD/DD)",
                "A design-phase narrative: each section's products as short bullets, grouped by division.",
            )
        },
    ]
}

/// The style `id`, or the first one.
pub fn style(id: &str) -> SpecStyle {
    let all = styles();
    all.iter()
        .find(|s| s.id == id)
        .cloned()
        .unwrap_or_else(|| all[0].clone())
}

/// A paragraph's label: level 0 A., 1 1., 2 a., 3 1). `path` is the article's numbers
/// (part, article) and the counts down to this paragraph (decimal numbering).
pub fn label(numbering: Numbering, level: u8, n: u32, path: &[u32]) -> String {
    match numbering {
        Numbering::Csi | Numbering::CsiZero => match level {
            0 => format!("{}.", letters(n, true)),
            1 => format!("{n}."),
            2 => format!("{}.", letters(n, false)),
            _ => format!("{n})"),
        },
        Numbering::Decimal => path
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("."),
        Numbering::Bullets => match level {
            0 => "\u{2022}".into(),
            _ => "\u{2013}".into(),
        },
    }
}

/// An article's number: "1.1" or "1.01" (none for bullets).
pub fn article_label(numbering: Numbering, part: u32, n: u32) -> String {
    match numbering {
        Numbering::CsiZero => format!("{part}.{n:02}"),
        Numbering::Bullets => String::new(),
        _ => format!("{part}.{n}"),
    }
}

pub fn part_label(numbering: Numbering, part: u32, title: &str) -> String {
    match numbering {
        Numbering::Decimal => format!("{part}  {title}"),
        _ => format!("PART {part} - {title}"),
    }
}

/// A, B, … Z, AA, BB (as specs letter past Z).
fn letters(n: u32, upper: bool) -> String {
    let n = n.max(1) - 1;
    let c = (b'A' + (n % 26) as u8) as char;
    let s = c.to_string().repeat((n / 26 + 1) as usize);
    if upper {
        s
    } else {
        s.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_the_numbering() {
        assert_eq!(label(Numbering::Csi, 0, 1, &[]), "A.");
        assert_eq!(label(Numbering::Csi, 0, 27, &[]), "AA.");
        assert_eq!(label(Numbering::Csi, 1, 3, &[]), "3.");
        assert_eq!(label(Numbering::Csi, 2, 2, &[]), "b.");
        assert_eq!(label(Numbering::Csi, 3, 4, &[]), "4)");
        assert_eq!(label(Numbering::Decimal, 1, 2, &[1, 3, 1, 2]), "1.3.1.2");
        assert_eq!(article_label(Numbering::CsiZero, 2, 3), "2.03");
        assert_eq!(article_label(Numbering::Csi, 2, 3), "2.3");
        assert_eq!(
            part_label(Numbering::Csi, 3, "EXECUTION"),
            "PART 3 - EXECUTION"
        );
    }

    #[test]
    fn there_are_six_styles_and_unknown_ids_fall_back() {
        let all = styles();
        assert_eq!(all.len(), 6);
        let ids: std::collections::HashSet<_> = all.iter().map(|s| s.id.clone()).collect();
        assert_eq!(ids.len(), 6);
        assert_eq!(style("nope").id, "csi-classic");
        assert_eq!(style("narrative").content, Content::Narrative);
    }
}
