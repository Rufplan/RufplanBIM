//! The book as a Word document (.docx, WordprocessingML): the same blocks as the PDF, with
//! Word's own multilevel numbering (so paragraphs renumber as they're edited in Word), a
//! Word section per spec section with its footer ("09 29 00 - 3"), and the style's font.

use crate::layout::{indents, Align, Block, Face, Laid};
use crate::style::{Numbering, SpecStyle};
use crate::zip;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c if (c as u32) < 0x20 && c != '\t' => {}
            c => o.push(c),
        }
    }
    o
}

const W: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

fn tw(pt: f32) -> i32 {
    (pt * 20.0).round() as i32
}

fn run(text: &str, face: Face, size: f32, white: bool, font: &str) -> String {
    let mut pr = format!(r#"<w:rFonts w:ascii="{font}" w:hAnsi="{font}" w:cs="{font}"/>"#);
    if matches!(face, Face::Bold | Face::Display) {
        pr.push_str("<w:b/>");
    }
    if face == Face::Italic {
        pr.push_str("<w:i/>");
    }
    if white {
        pr.push_str(r#"<w:color w:val="FFFFFF"/>"#);
    }
    let hp = (size * 2.0).round() as i32;
    pr.push_str(&format!(r#"<w:sz w:val="{hp}"/><w:szCs w:val="{hp}"/>"#));
    format!(
        r#"<w:r><w:rPr>{pr}</w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#,
        esc(text)
    )
}

/// Whether Word's numbering sets this block's label.
fn numbered(style: &SpecStyle, b: &Block) -> bool {
    b.list.is_some()
        && !b.label.is_empty()
        && (style.numbering != Numbering::Bullets || b.list.is_some_and(|l| l.level >= 2))
}

fn paragraph(
    style: &SpecStyle,
    b: &Block,
    num_id: Option<u32>,
    page_break: bool,
    width_pt: f32,
) -> String {
    let font = style.font.word_name();
    let display = if style.font == crate::style::Font::Brand {
        "Barlow Condensed SemiBold"
    } else {
        font
    };
    let f = if b.face == Face::Display {
        display
    } else {
        font
    };
    let mut ppr = String::new();
    let use_num = numbered(style, b);
    // Word wants these in schema order.
    if b.keep_next {
        ppr.push_str("<w:keepNext/>");
    }
    if page_break {
        ppr.push_str("<w:pageBreakBefore/>");
    }
    if let (true, Some(l), Some(id)) = (use_num, b.list, num_id) {
        ppr.push_str(&format!(
            r#"<w:numPr><w:ilvl w:val="{}"/><w:numId w:val="{id}"/></w:numPr>"#,
            l.level
        ));
    }
    if b.rule || b.boxed > 0.0 {
        if b.boxed > 0.0 {
            ppr.push_str(r#"<w:pBdr><w:top w:val="single" w:sz="4" w:space="0" w:color="555555"/><w:left w:val="single" w:sz="4" w:space="0" w:color="555555"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="555555"/><w:right w:val="single" w:sz="4" w:space="0" w:color="555555"/></w:pBdr>"#);
        } else {
            ppr.push_str(r#"<w:pBdr><w:bottom w:val="single" w:sz="6" w:space="3" w:color="0A0A0A"/></w:pBdr>"#);
        }
    }
    if b.banner {
        ppr.push_str(r#"<w:shd w:val="clear" w:color="auto" w:fill="0A0A0A"/>"#);
    }
    if !b.label.is_empty() && !use_num {
        ppr.push_str(&format!(
            r#"<w:tabs><w:tab w:val="left" w:pos="{}"/></w:tabs>"#,
            tw(b.x)
        ));
    }
    let line = if b.boxed > 0.0 {
        format!(r#" w:line="{}" w:lineRule="exact""#, tw(b.boxed))
    } else {
        String::new()
    };
    ppr.push_str(&format!(
        r#"<w:spacing w:before="{}" w:after="{}"{line}/>"#,
        tw(b.before),
        tw(b.after)
    ));
    if b.boxed > 0.0 {
        ppr.push_str(&format!(
            r#"<w:ind w:left="{}" w:right="{}"/>"#,
            tw(b.x),
            tw((width_pt - b.x - 190.0).max(0.0))
        ));
    } else if !use_num && b.align == Align::Left {
        let hanging = if b.label.is_empty() {
            0.0
        } else {
            b.x - b.label_x
        };
        ppr.push_str(&format!(
            r#"<w:ind w:left="{}" w:hanging="{}"/>"#,
            tw(b.x),
            tw(hanging)
        ));
    }
    if b.align == Align::Center {
        ppr.push_str(r#"<w:jc w:val="center"/>"#);
    }
    let mut runs = String::new();
    if !b.label.is_empty() && !use_num {
        runs.push_str(&run(&b.label, b.face, b.size, b.banner, f));
        runs.push_str("<w:r><w:tab/></w:r>");
    }
    runs.push_str(&run(&b.text, b.face, b.size, b.banner, f));
    format!("<w:p><w:pPr>{ppr}</w:pPr>{runs}</w:p>")
}

fn level_xml(style: &SpecStyle, level: u8) -> String {
    let n = style.numbering;
    let (fmt, text) = match (n, level) {
        (Numbering::Bullets, 2) => ("bullet", "\u{2022}".to_string()),
        (Numbering::Bullets, _) => ("bullet", "\u{2013}".to_string()),
        (Numbering::Decimal, l) => (
            "decimal",
            (1..=l + 1)
                .map(|i| format!("%{i}"))
                .collect::<Vec<_>>()
                .join("."),
        ),
        (_, 0) => ("decimal", "PART %1 -".into()),
        (Numbering::CsiZero, 1) => ("decimalZero", "%1.%2".into()),
        (_, 1) => ("decimal", "%1.%2".into()),
        (_, 2) => ("upperLetter", "%3.".into()),
        (_, 3) => ("decimal", "%4.".into()),
        (_, 4) => ("lowerLetter", "%5.".into()),
        (_, _) => ("decimal", "%6)".into()),
    };
    let (lx, x) = match level {
        0 => (0.0, 0.0),
        1 => indents(n, None),
        l => indents(n, Some(l - 2)),
    };
    let suff = if level == 0 && n != Numbering::Decimal {
        "space"
    } else {
        "tab"
    };
    let bold = if level <= 1 && (style.bold_articles || level == 0) {
        "<w:b/>"
    } else {
        ""
    };
    format!(
        r#"<w:lvl w:ilvl="{level}"><w:start w:val="1"/><w:numFmt w:val="{fmt}"/><w:suff w:val="{suff}"/><w:lvlText w:val="{}"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="{}" w:hanging="{}"/></w:pPr><w:rPr>{bold}</w:rPr></w:lvl>"#,
        esc(&text),
        tw(x),
        tw(x - lx)
    )
}

fn numbering_xml(style: &SpecStyle, lists: u32) -> String {
    let levels: String = (0..6).map(|l| level_xml(style, l)).collect();
    let nums: String = (1..=lists)
        .map(|id| {
            let over: String = (0..6)
                .map(|l| format!(r#"<w:lvlOverride w:ilvl="{l}"><w:startOverride w:val="1"/></w:lvlOverride>"#))
                .collect();
            format!(r#"<w:num w:numId="{id}"><w:abstractNumId w:val="0"/>{over}</w:num>"#)
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering {W}><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="multilevel"/>{levels}</w:abstractNum>{nums}</w:numbering>"#
    )
}

fn header_footer(
    tag: &str,
    left: &str,
    right: &str,
    page_field: bool,
    font: &str,
    width_pt: f32,
) -> String {
    let r = |t: &str| run(t, Face::Regular, 8.0, false, font);
    let field = if page_field {
        format!(
            r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r>{}<w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
            r("1")
        )
    } else {
        String::new()
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{tag} {W}><w:p><w:pPr><w:tabs><w:tab w:val="right" w:pos="{}"/></w:tabs></w:pPr>{}<w:r><w:tab/></w:r>{}{field}</w:p></w:{tag}>"#,
        tw(width_pt),
        r(left),
        r(right)
    )
}

/// Writes the book's .docx.
pub fn export(laid: &Laid, style: &SpecStyle, title: &str) -> Vec<u8> {
    let margin = style.margin * 72.0;
    let width_pt = 612.0 - 2.0 * margin;
    let font = style.font.word_name();
    // Word sections: a new one where a spec section starts on a new page.
    let mut body = String::new();
    let mut footers: Vec<(String, String, bool)> = vec![];
    let mut current: Option<usize> = None;
    let mut numbers: std::collections::BTreeMap<u32, u32> = Default::default();
    let sect = |footer: usize, restart: bool| {
        format!(
            r#"<w:sectPr><w:headerReference w:type="default" r:id="rIdHeader"/><w:footerReference w:type="default" r:id="rIdFooter{footer}"/><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1600" w:right="{m}" w:bottom="1440" w:left="{m}" w:header="720" w:footer="720" w:gutter="0"/>{}</w:sectPr>"#,
            if restart {
                r#"<w:pgNumType w:start="1"/>"#
            } else {
                ""
            },
            m = tw(margin)
        )
    };
    for (i, b) in laid.blocks.iter().enumerate() {
        let starts = i == 0 || (laid.per_section && b.new_page && current != Some(b.section));
        if starts {
            if let Some(prev) = current {
                let _ = prev;
                body.push_str(&format!(
                    "<w:p><w:pPr>{}</w:pPr></w:p>",
                    sect(footers.len() - 1, laid.per_section)
                ));
            }
            let run = laid.sections.get(b.section);
            let (left, right, field) = match run {
                Some(r) if laid.per_section && !r.number.is_empty() => {
                    (r.title.clone(), format!("{} - ", r.number), true)
                }
                _ => (laid.header_left.clone(), String::new(), true),
            };
            footers.push((left, right, field));
            current = Some(b.section);
        }
        let num_id = b.list.map(|l| {
            let next = numbers.len() as u32 + 1;
            *numbers.entry(l.list).or_insert(next)
        });
        let page_break = b.new_page && !starts;
        body.push_str(&paragraph(style, b, num_id, page_break, width_pt));
    }
    if footers.is_empty() {
        footers.push((laid.header_left.clone(), String::new(), true));
    }
    let last = sect(footers.len() - 1, laid.per_section);
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {W}><w:body>{body}{last}</w:body></w:document>"#
    );
    let hp = (style.size * 2.0).round() as i32;
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles {W}><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="{font}" w:hAnsi="{font}" w:cs="{font}" w:eastAsia="{font}"/><w:sz w:val="{hp}"/><w:szCs w:val="{hp}"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="264" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style></w:styles>"#
    );
    let settings = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings {W}><w:defaultTabStop w:val="720"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    );
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdNumbering" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/><Relationship Id="rIdSettings" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/><Relationship Id="rIdHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>"#,
    );
    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>"#,
    );
    let mut files: Vec<(String, Vec<u8>)> = vec![];
    for (i, (left, right, field)) in footers.iter().enumerate() {
        rels.push_str(&format!(
            r#"<Relationship Id="rIdFooter{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer{i}.xml"/>"#
        ));
        types.push_str(&format!(
            r#"<Override PartName="/word/footer{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>"#
        ));
        files.push((
            format!("word/footer{i}.xml"),
            header_footer("ftr", left, right, *field, font, width_pt).into_bytes(),
        ));
    }
    rels.push_str("</Relationships>");
    types.push_str("</Types>");
    let core = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{}</dc:title><dc:creator>Rufplan Studio</dc:creator></cp:coreProperties>"#,
        esc(title)
    );
    let mut all = vec![
        ("[Content_Types].xml".to_string(), types.into_bytes()),
        (
            "_rels/.rels".to_string(),
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/></Relationships>"#.to_vec(),
        ),
        ("docProps/core.xml".to_string(), core.into_bytes()),
        ("word/document.xml".to_string(), document.into_bytes()),
        ("word/styles.xml".to_string(), styles.into_bytes()),
        ("word/numbering.xml".to_string(), numbering_xml(style, numbers.len() as u32).into_bytes()),
        ("word/settings.xml".to_string(), settings.into_bytes()),
        (
            "word/header1.xml".to_string(),
            header_footer("hdr", &laid.header_left, &laid.header_right, false, font, width_pt).into_bytes(),
        ),
        ("word/_rels/document.xml.rels".to_string(), rels.into_bytes()),
    ];
    all.extend(files);
    zip::store(&all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_is_escaped() {
        assert_eq!(
            esc("A & B <C> \"D\"\u{1}"),
            "A &amp; B &lt;C&gt; &quot;D&quot;"
        );
    }

    #[test]
    fn csi_levels_use_word_numbering() {
        let s = crate::style::style("csi-classic");
        assert!(level_xml(&s, 0).contains(r#"w:lvlText w:val="PART %1 -""#));
        assert!(level_xml(&s, 1).contains("decimalZero"));
        assert!(
            level_xml(&s, 2).contains("upperLetter")
                && level_xml(&s, 2).contains(r#"w:left="1440" w:hanging="720""#)
        );
        let d = crate::style::style("decimal");
        assert!(level_xml(&d, 4).contains("%1.%2.%3.%4.%5"));
    }
}
