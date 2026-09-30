//! The report: counts, a score and a summary line; its PDF (with studio-specs' writer);
//! and what Claude is asked for an overall review.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use studio_specs::layout::{Align, Block, Face, Laid, Run};
use studio_specs::style::Font;
use ts_rs::TS;

use crate::ctx::Ctx;
use crate::{Category, Count, Finding, Options, Report, Severity, DISCLAIMER};

pub fn score(findings: &[Finding]) -> u32 {
    let lost: usize = findings
        .iter()
        .map(|f| match f.severity {
            Severity::Critical => 15,
            Severity::Major => 5,
            Severity::Minor => 1,
            Severity::Info => 0,
        })
        .sum();
    100u32.saturating_sub(lost.min(100) as u32)
}

pub fn counts(findings: &[Finding]) -> Vec<Count> {
    Category::ALL
        .iter()
        .map(|c| {
            let of = |s: Severity| {
                findings
                    .iter()
                    .filter(|f| f.category == *c && f.severity == s)
                    .count()
            };
            Count {
                category: *c,
                critical: of(Severity::Critical),
                major: of(Severity::Major),
                minor: of(Severity::Minor),
                info: of(Severity::Info),
            }
        })
        .collect()
}

pub fn summary(findings: &[Finding]) -> String {
    let n = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    let (c, m, mi) = (
        n(Severity::Critical),
        n(Severity::Major),
        n(Severity::Minor),
    );
    if c + m + mi == 0 {
        return "No issues found by the automated checks.".into();
    }
    let mut parts = vec![];
    if c > 0 {
        parts.push(format!("{c} critical"));
    }
    if m > 0 {
        parts.push(format!("{m} major"));
    }
    if mi > 0 {
        parts.push(format!("{mi} minor"));
    }
    let verdict = if c > 0 {
        "Not ready to issue: resolve the critical items first."
    } else if m > 0 {
        "Resolve the major items before issuing."
    } else {
        "Close to ready: clean up the minor items."
    };
    format!("{} issues. {verdict}", parts.join(", "))
}

pub(crate) fn build(c: &Ctx, opts: &Options, findings: Vec<Finding>) -> Report {
    Report {
        milestone: opts.milestone,
        milestone_label: opts.milestone.label().into(),
        project: if c.project_number.is_empty() {
            c.project_name.clone()
        } else {
            format!("{} ({})", c.project_name, c.project_number)
        },
        code_basis: c.code_basis(),
        score: score(&findings),
        summary: summary(&findings),
        counts: counts(&findings),
        findings,
        overview: None,
        checked: opts
            .categories
            .iter()
            .map(|c| c.label().to_string())
            .collect(),
        disclaimer: DISCLAIMER.into(),
    }
}

/// Recomputes the counts, score and summary after findings were added (Claude's).
pub fn refresh(r: &mut Report) {
    r.findings.sort_by(|a, b| {
        (a.severity, a.category, &a.title).cmp(&(b.severity, b.category, &b.title))
    });
    r.score = score(&r.findings);
    r.counts = counts(&r.findings);
    r.summary = summary(&r.findings);
}

/// The report as a PDF: a summary page, then the findings by category.
pub fn pdf(r: &Report, resolved: &[String]) -> Result<Vec<u8>, String> {
    studio_specs::pdf::export(&laid(r, resolved), Font::Sans, 0.85)
}

/// The report laid out as blocks (the PDF's pages).
pub fn laid(r: &Report, resolved: &[String]) -> Laid {
    let size = 10.0;
    let b = |text: &str, size: f32, face: Face, before: f32, after: f32| Block {
        label: String::new(),
        text: text.into(),
        label_x: 0.0,
        x: 0.0,
        size,
        face,
        align: Align::Left,
        before,
        after,
        keep_next: false,
        new_page: false,
        banner: false,
        rule: false,
        boxed: 0.0,
        list: None,
        section: 0,
    };
    let mut blocks = vec![];
    let mut t = b("QA/QC REVIEW", size + 12.0, Face::Display, 0.0, 6.0);
    t.new_page = true;
    blocks.push(t);
    blocks.push(b(&r.project, size + 4.0, Face::Bold, 0.0, 2.0));
    blocks.push(b(
        &format!(
            "Milestone: {}   ·   Code basis: {}",
            r.milestone_label, r.code_basis
        ),
        size,
        Face::Regular,
        0.0,
        10.0,
    ));
    let mut s = b(
        &format!("Score {} / 100   ·   {}", r.score, r.summary),
        size + 2.0,
        Face::Bold,
        4.0,
        10.0,
    );
    s.rule = true;
    blocks.push(s);
    if let Some(o) = &r.overview {
        blocks.push(b("OVERALL REVIEW (Claude)", size, Face::Bold, 6.0, 3.0));
        for para in o.split("\n").filter(|p| !p.trim().is_empty()) {
            blocks.push(b(para.trim(), size, Face::Regular, 0.0, 4.0));
        }
    }
    for cat in Category::ALL {
        let fs: Vec<&Finding> = r.findings.iter().filter(|f| f.category == cat).collect();
        if fs.is_empty() {
            continue;
        }
        let mut h = b(cat.label(), size + 3.0, Face::Display, 16.0, 6.0);
        h.keep_next = true;
        h.banner = true;
        h.label = format!("{}", fs.len());
        h.label_x = 8.0;
        h.x = 36.0;
        blocks.push(h);
        for f in fs {
            let done = resolved.contains(&f.id);
            let mut head = b(
                &format!("{}{}", f.title, if done { "  (resolved)" } else { "" }),
                size,
                Face::Bold,
                8.0,
                1.0,
            );
            head.label = f.severity.label().to_uppercase();
            head.x = 64.0;
            head.keep_next = true;
            blocks.push(head);
            let mut d = b(&f.detail, size, Face::Regular, 0.0, 1.0);
            d.x = 64.0;
            d.keep_next = true;
            blocks.push(d);
            let mut fix = b(&format!("Fix: {}", f.fix), size, Face::Italic, 0.0, 1.0);
            fix.x = 64.0;
            fix.keep_next = !f.reference.is_empty();
            blocks.push(fix);
            if !f.reference.is_empty() {
                let mut rf = b(
                    &format!("Reference: {}", f.reference),
                    size - 1.0,
                    Face::Regular,
                    0.0,
                    2.0,
                );
                rf.x = 64.0;
                blocks.push(rf);
            }
        }
    }
    let mut d = b(DISCLAIMER, size - 1.0, Face::Italic, 18.0, 0.0);
    d.rule = false;
    blocks.push(d);
    Laid {
        blocks,
        sections: vec![Run {
            number: String::new(),
            title: String::new(),
        }],
        header_left: format!("{} — QA/QC Review", r.project),
        header_right: r.milestone_label.clone(),
        per_section: false,
    }
}

/// One finding Claude adds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "QaClaudeFinding")]
pub struct ClaudeFinding {
    pub category: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    #[serde(default)]
    pub fix: String,
    #[serde(default)]
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "QaClaudeReview")]
pub struct ClaudeReview {
    pub overview: String,
    #[serde(default)]
    pub findings: Vec<ClaudeFinding>,
}

pub fn claude_prompt() -> String {
    "You are a senior architect doing the QA/QC review of a US project's drawing set at a milestone, the way a firm's quality reviewer does before a set goes to the client, the building department or bidders. You get the automated checks' findings and a digest of the model (rooms, openings, assemblies, sheets, specifications). Answer only by calling the qa_review tool.\n\noverview: 4-8 sentences: overall readiness for the milestone, the main risks (coordination, code, envelope/waterproofing, constructability, drawing-to-spec), and what to fix first. Plain, direct, like a review memo. Never state the design complies with code: say what to verify.\nfindings: up to 12 additional issues the rules can't see but the digest shows (judgment calls: layout, egress paths, detailing gaps, missing sheets or views for the building type, spec gaps, consultant coordination). Don't repeat the automated findings. category is one of Coordination, Completeness, Code, Accessibility, Waterproofing, DrawingSpec, Constructability, Consultants; severity Critical, Major, Minor or Info; cite a code section only when you're sure of it, else leave reference empty.".into()
}

pub fn claude_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "overview": { "type": "string" },
            "findings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "category": { "type": "string", "enum": ["Coordination", "Completeness", "Code", "Accessibility", "Waterproofing", "DrawingSpec", "Constructability", "Consultants"] },
                        "severity": { "type": "string", "enum": ["Critical", "Major", "Minor", "Info"] },
                        "title": { "type": "string" },
                        "detail": { "type": "string" },
                        "fix": { "type": "string" },
                        "reference": { "type": "string" }
                    },
                    "required": ["category", "severity", "title", "detail"]
                }
            }
        },
        "required": ["overview", "findings"]
    })
}

/// Adds Claude's review to the report.
pub fn merge_claude(r: &mut Report, review: ClaudeReview) {
    r.overview = Some(review.overview);
    for (i, f) in review.findings.into_iter().enumerate() {
        let category = match f.category.as_str() {
            "Coordination" => Category::Coordination,
            "Completeness" => Category::Completeness,
            "Code" => Category::Code,
            "Accessibility" => Category::Accessibility,
            "Waterproofing" => Category::Waterproofing,
            "DrawingSpec" => Category::DrawingSpec,
            "Consultants" => Category::Consultants,
            _ => Category::Constructability,
        };
        let severity = match f.severity.as_str() {
            "Critical" => Severity::Critical,
            "Major" => Severity::Major,
            "Minor" => Severity::Minor,
            _ => Severity::Info,
        };
        r.findings.push(Finding {
            id: format!("claude:{i}:{}", f.title),
            rule: "claude".into(),
            category,
            severity,
            title: f.title,
            detail: f.detail,
            fix: f.fix,
            reference: f.reference,
            elements: vec![],
            view: None,
            source: "claude".into(),
        });
    }
    refresh(r);
}

/// What Claude reads: the report and a digest of the set.
pub fn digest(doc: &studio_core::Document, model: &studio_regen::Model, r: &Report) -> String {
    use studio_core::{Category as Cat, ElementData};
    let mut s = format!(
        "PROJECT: {}\nMILESTONE: {}\nCODE BASIS: {}\n",
        r.project, r.milestone_label, r.code_basis
    );
    let (_, d) = studio_core::project::get(doc).unwrap_or_default();
    s.push_str(&format!(
        "TYPE: {} / {} / occupancy {} / construction {} / sprinklers {}\n",
        d.overview.project_type,
        d.overview.work_type,
        d.codes.occupancy,
        d.codes.construction_type,
        d.codes.sprinklered
    ));
    s.push_str(&format!(
        "LEVELS: {}\n",
        doc.levels()
            .iter()
            .map(|l| format!("{} at {:.0} mm", l.1, l.2))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    s.push_str("ROOMS:\n");
    for r in &model.rooms {
        s.push_str(&format!(
            "- {} {} ({:.0} sf)\n",
            r.number,
            r.name,
            r.area() / 92903.04
        ));
    }
    let mut types = std::collections::BTreeMap::new();
    for e in doc.of(Cat::Wall) {
        if let Some(t) = e.data.type_id() {
            *types
                .entry(doc.data(t).map(|x| x.name()).unwrap_or_default())
                .or_insert(0) += 1;
        }
    }
    s.push_str(&format!(
        "WALL TYPES: {}\n",
        types
            .iter()
            .map(|(k, v)| format!("{k} ×{v}"))
            .collect::<Vec<_>>()
            .join("; ")
    ));
    s.push_str(&format!(
        "DOORS: {}  WINDOWS: {}\n",
        doc.of(Cat::Door).count(),
        doc.of(Cat::Window).count()
    ));
    s.push_str("SHEETS:\n");
    for e in doc.of(Cat::Sheet) {
        if let ElementData::Sheet { number, name, .. } = &e.data {
            s.push_str(&format!("- {number} {name}\n"));
        }
    }
    if let Some(b) = studio_core::specs::book(doc) {
        s.push_str(&format!(
            "SPEC SECTIONS ({} issued): {}\n",
            b.sections.iter().filter(|x| x.included).count(),
            b.sections
                .iter()
                .filter(|x| x.included)
                .map(|x| x.number.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        s.push_str("SPECIFICATIONS: none\n");
    }
    s.push_str(&format!(
        "\nAUTOMATED FINDINGS ({}), score {}:\n",
        r.findings.len(),
        r.score
    ));
    for f in r.findings.iter().take(80) {
        s.push_str(&format!(
            "- [{} / {}] {} — {}\n",
            f.severity.label(),
            f.category.label(),
            f.title,
            f.detail
        ));
    }
    s
}
