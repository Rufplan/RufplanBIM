//! IPC for the QA/QC tab (ADR-088): thin wrappers over studio-qa (the review, its PDF) and
//! Claude's overall review.

use studio_qa::fix::{self, Action, Plan};
use studio_qa::report::{self, ClaudeReview};
use studio_qa::{Options, Report};
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::generate_cmds::CLAUDE_KEY;
use crate::session::AppState;
use crate::site_cmds::get;

type CommandResult<T> = Result<T, CommandError>;

/// Reviews the set for the milestone and categories asked.
#[tauri::command]
pub fn qa_review(options: Options, state: State<'_, SessionState>) -> CommandResult<Report> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let model = studio_regen::regenerate(doc);
    Ok(studio_qa::review(doc, &model, &options))
}

/// Claude's overall review, merged into the report.
#[tauri::command]
pub async fn qa_claude(report: Report, state: State<'_, SessionState>) -> CommandResult<Report> {
    let key = get(CLAUDE_KEY).ok_or_else(|| {
        anyhow::anyhow!("add your Claude API key first (Architecture > Generate > Claude API key)")
    })?;
    let digest = {
        let s = lock(&state)?;
        let doc = s.doc()?;
        let model = studio_regen::regenerate(doc);
        report::digest(doc, &model, &report)
    };
    let review = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<ClaudeReview> {
        let request = studio_sync::claude::Request {
            model: "claude-opus-5-5".into(),
            system: report::claude_prompt(),
            text: digest,
            images: vec![],
            tool_name: "qa_review".into(),
            tool_description:
                "The overall QA/QC review and any issues the automated checks missed.".into(),
            tool_schema: report::claude_schema(),
            max_tokens: 8_000,
        };
        let answer = studio_sync::claude::call(&key, &request, &mut |_| {})?;
        serde_json::from_value(answer)
            .map_err(|e| anyhow::anyhow!("Claude's review doesn't fit: {e}"))
    })
    .await
    .map_err(anyhow::Error::from)??;
    let mut r = report;
    report::merge_claude(&mut r, review);
    Ok(r)
}

/// Writes the report as a PDF; returns the path written.
#[tauri::command]
pub fn qa_export_pdf(path: String, report: Report, resolved: Vec<String>) -> CommandResult<String> {
    let mut p = std::path::PathBuf::from(&path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("pdf")) {
        p.set_extension("pdf");
    }
    let bytes = report::pdf(&report, &resolved).map_err(|e| anyhow::anyhow!(e))?;
    std::fs::write(&p, bytes)
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", p.display()))?;
    Ok(p.display().to_string())
}

/// Fix Issues (ADR-089): the fixes for the report's findings.
#[tauri::command]
pub fn qa_fix_plan(report: Report, state: State<'_, SessionState>) -> CommandResult<Plan> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let model = studio_regen::regenerate(doc);
    Ok(fix::plan(doc, &model, &report))
}

/// Applies fixes as one undo step named `label`; returns how many applied, the errors of
/// those that couldn't, and the new state.
#[tauri::command]
pub fn qa_fix_apply(
    actions: Vec<Action>,
    label: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<(usize, Vec<String>, Option<AppState>), CommandError> {
    let mut session = lock(&state)?;
    let model = studio_regen::regenerate(session.doc()?);
    let (n, errors) = session.edit(|d| Ok(fix::apply(d, &model, &actions, &label)))?;
    Ok((n, errors, finish(&window, &session)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_qa::{Category, Milestone};

    #[test]
    fn the_sample_house_reviews_without_a_flood() {
        let mut s = crate::session::Session::default();
        s.new_sample("0").unwrap();
        let doc = s.doc().unwrap();
        let model = studio_regen::regenerate(doc);
        let r = studio_qa::review(
            doc,
            &model,
            &Options {
                milestone: Milestone::Cd90,
                categories: Category::ALL.to_vec(),
            },
        );
        for f in &r.findings {
            println!(
                "[{:?} {:?}] {} — {} ({})",
                f.severity, f.category, f.title, f.detail, f.reference
            );
        }
        println!("score {} — {}", r.score, r.summary);
        assert!(r.findings.len() < 80, "{} findings", r.findings.len());
        assert!(!r.findings.is_empty());
    }

    #[test]
    fn fixing_the_sample_house_raises_its_score_and_still_draws() {
        let mut s = crate::session::Session::default();
        s.new_sample("0").unwrap();
        let opts = Options {
            milestone: Milestone::Cd90,
            categories: Category::ALL.to_vec(),
        };
        let (before, plan) = {
            let doc = s.doc().unwrap();
            let model = studio_regen::regenerate(doc);
            let r = studio_qa::review(doc, &model, &opts);
            let p = fix::plan(doc, &model, &r);
            (r, p)
        };
        for f in &plan.fixes {
            println!(
                "FIX {}{} — {}",
                if f.design_change { "[design] " } else { "" },
                f.title,
                f.change
            );
        }
        let actions: Vec<Action> = plan.fixes.iter().map(|f| f.action.clone()).collect();
        let model = studio_regen::regenerate(s.doc().unwrap());
        let (n, errors) = s
            .edit(|d| Ok(fix::apply(d, &model, &actions, "QA/QC fixes")))
            .unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(n, actions.len());
        let doc = s.doc().unwrap();
        let model = studio_regen::regenerate(doc);
        let after = studio_qa::review(doc, &model, &opts);
        for f in &after.findings {
            println!("AFTER [{:?}] {} — {}", f.severity, f.title, f.detail);
        }
        println!("score {} → {}", before.score, after.score);
        assert!(after.score > before.score);
        // Membrane layers of no thickness still draw in plan and 3D.
        let plan_view = doc
            .of(studio_core::Category::View)
            .find(|e| e.data.name() == "Level 1")
            .map(|e| e.id)
            .unwrap();
        assert!(studio_views::display_list(doc, plan_view).is_some());
        assert!(!studio_views::meshes(doc).is_empty());
    }
}
