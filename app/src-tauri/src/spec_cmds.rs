//! IPC for the Specifications tab (ADR-085): thin wrappers over studio-core `specs` (the
//! book, saved in the project) and studio-specs (library, picks, styles, PDF and Word, and
//! Edit Specs with Claude).

use serde::Serialize;
use studio_core::specs::{self, SpecBook, SpecSection};
use studio_specs::coord::{missing_references, SpecReference};
use studio_specs::edit::{SpecChange, SpecEdit};
use studio_specs::features::{facts, Facts};
use studio_specs::generate::{front, generate, update, Front, SpecUpdate};
use studio_specs::library::{library, DIVISIONS};
use studio_specs::style::{style, styles, SpecStyle};
use tauri::{State, WebviewWindow};
use ts_rs::TS;

use crate::commands::{finish, lock, today, CommandError, SessionState};
use crate::generate_cmds::CLAUDE_KEY;
use crate::session::AppState;
use crate::site_cmds::get;

type CommandResult<T> = Result<T, CommandError>;
type StateResult = Result<Option<AppState>, CommandError>;

/// One library section as the Add Sections dialog lists it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecLibraryRow {
    pub number: String,
    pub title: String,
    pub paragraphs: usize,
    /// The model and Project Info call for it, and why.
    pub picked: bool,
    pub reason: Option<String>,
    pub in_book: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecDivision {
    pub code: String,
    pub title: String,
}

/// Everything the Specifications tab shows.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecState {
    pub book: Option<SpecBook>,
    pub styles: Vec<SpecStyle>,
    pub library: Vec<SpecLibraryRow>,
    pub divisions: Vec<SpecDivision>,
    /// Cross-references to sections missing from the book (or excluded).
    pub references: Vec<SpecReference>,
    /// Sections in the book the model no longer calls for.
    pub unindicated: Vec<String>,
    /// Sections the model calls for that aren't in the book.
    pub indicated: Vec<String>,
    /// What the generated front matter is written from.
    pub front: Front,
    pub today: String,
}

fn project_facts(doc: &studio_core::Document) -> Facts {
    facts(doc, &studio_regen::regenerate(doc))
}

#[tauri::command]
pub fn spec_state(state: State<'_, SessionState>) -> CommandResult<SpecState> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let f = project_facts(doc);
    let book = specs::book(doc);
    let (references, unindicated, indicated) = match &book {
        Some(b) => {
            let (_, u) = update(b, &f);
            (missing_references(b), u.unindicated, u.added)
        }
        None => (vec![], vec![], vec![]),
    };
    Ok(SpecState {
        library: library()
            .iter()
            .map(|l| SpecLibraryRow {
                number: l.section.number.clone(),
                title: l.section.title.clone(),
                paragraphs: l.section.paragraph_count(),
                picked: f.picks(&l.when),
                reason: f.reason(&l.when),
                in_book: book
                    .as_ref()
                    .is_some_and(|b| b.section(&l.section.number).is_some()),
            })
            .collect(),
        book,
        styles: styles(),
        divisions: DIVISIONS
            .iter()
            .map(|(c, t)| SpecDivision {
                code: (*c).into(),
                title: (*t).into(),
            })
            .collect(),
        references,
        unindicated,
        indicated,
        front: front(doc),
        today: today(),
    })
}

fn edit_book(
    window: &WebviewWindow,
    state: &State<'_, SessionState>,
    f: impl FnOnce(&mut studio_core::Document) -> studio_core::CoreResult<()>,
) -> StateResult {
    let mut session = lock(state)?;
    session.edit(f)?;
    finish(window, &session)
}

/// The library's text of section `number`, filled from the project (to revert an edit).
#[tauri::command]
pub fn spec_library_section(
    number: String,
    state: State<'_, SessionState>,
) -> CommandResult<Option<SpecSection>> {
    let session = lock(&state)?;
    let f = project_facts(session.doc()?);
    Ok(studio_specs::library::entry(&number).map(|l| studio_specs::generate::from_library(l, &f)))
}

/// Makes the project manual from the model and Project Info (replacing any book).
#[tauri::command]
pub fn spec_generate(
    style_id: String,
    issue: String,
    date: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| {
        let f = project_facts(d);
        specs::save(
            d,
            "Generate Project Manual",
            generate(&f, &style_id, &issue, &date),
        )
    })
}

/// Adds the sections the model now calls for; returns what changed.
#[tauri::command]
pub fn spec_update(
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> Result<(SpecUpdate, Option<AppState>), CommandError> {
    let mut session = lock(&state)?;
    let doc = session.doc()?;
    let book =
        specs::book(doc).ok_or_else(|| anyhow::anyhow!("generate the project manual first"))?;
    let (b, report) = update(&book, &project_facts(doc));
    if !report.added.is_empty() {
        session.edit(|d| specs::save(d, "Update Project Manual from Model", b))?;
    }
    Ok((report, finish(&window, &session)?))
}

#[tauri::command]
pub fn spec_set_section(
    number: String,
    section: SpecSection,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| specs::set_section(d, &number, section))
}

/// Adds library sections (placeholders filled from the project).
#[tauri::command]
pub fn spec_add_library(
    numbers: Vec<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| {
        let f = project_facts(d);
        let add: Vec<SpecSection> = numbers
            .iter()
            .filter_map(|n| studio_specs::library::entry(n))
            .map(|l| studio_specs::generate::from_library(l, &f))
            .collect();
        specs::add_sections(d, add)
    })
}

/// Adds a new section with the usual articles to fill in.
#[tauri::command]
pub fn spec_add_custom(
    number: String,
    title: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| {
        specs::add_sections(d, vec![SpecSection::blank(&number, &title)])
    })
}

#[tauri::command]
pub fn spec_remove(
    numbers: Vec<String>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| specs::remove_sections(d, &numbers))
}

#[tauri::command]
pub fn spec_set_included(
    numbers: Vec<String>,
    included: bool,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| {
        specs::set_included(d, &numbers, included)
    })
}

/// The book's style, issue and date.
#[tauri::command]
pub fn spec_set_settings(
    style_id: String,
    issue: String,
    date: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| {
        specs::change(d, "Project Manual Settings", |b| {
            b.style = style_id;
            b.issue = issue;
            b.date = date;
            Ok(())
        })
    })
}

/// Writes the book (included sections, as issued) to `path` as "pdf" or "docx"; returns
/// the path written and the page count.
#[tauri::command]
pub fn spec_export(
    path: String,
    format: String,
    state: State<'_, SessionState>,
) -> CommandResult<(String, usize)> {
    let session = lock(&state)?;
    let doc = session.doc()?;
    let book =
        specs::book(doc).ok_or_else(|| anyhow::anyhow!("generate the project manual first"))?;
    let st = style(&book.style);
    let laid = studio_specs::layout::layout(&book, &st, &front(doc), true);
    let pages = studio_specs::pdf::page_count(&laid, st.font, st.margin);
    let mut p = std::path::PathBuf::from(&path);
    let ext = if format == "docx" { "docx" } else { "pdf" };
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case(ext)) {
        p.set_extension(ext);
    }
    let bytes = if ext == "docx" {
        studio_specs::docx::export(&laid, &st, "Project Manual")
    } else {
        studio_specs::pdf::export(&laid, st.font, st.margin).map_err(|e| anyhow::anyhow!(e))?
    };
    std::fs::write(&p, bytes)
        .map_err(|e| anyhow::anyhow!("could not write {}: {e}", p.display()))?;
    Ok((p.display().to_string(), pages))
}

/// Claude's proposal and what it changes (or why it can't be done).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecEditPlan {
    pub edit: SpecEdit,
    pub changes: Vec<SpecChange>,
    pub error: Option<String>,
}

/// Asks Claude for an edit of the book and previews it. Nothing changes until Apply.
#[tauri::command]
pub async fn spec_edit_preview(
    prompt: String,
    focus: Option<String>,
    state: State<'_, SessionState>,
) -> CommandResult<SpecEditPlan> {
    if prompt.trim().is_empty() {
        return Err(anyhow::anyhow!("describe a change to the specifications").into());
    }
    let key = get(CLAUDE_KEY).ok_or_else(|| {
        anyhow::anyhow!("add your Claude API key first (Architecture > Generate > Claude API key)")
    })?;
    let (summary, book) = {
        let s = lock(&state)?;
        let doc = s.doc()?;
        let book =
            specs::book(doc).ok_or_else(|| anyhow::anyhow!("generate the project manual first"))?;
        let f = project_facts(doc);
        (
            studio_specs::edit::describe(&book, focus.as_deref(), &prompt, &f),
            book,
        )
    };
    let p = prompt.clone();
    let edit = tauri::async_runtime::spawn_blocking(move || ask(&key, &summary, &p))
        .await
        .map_err(anyhow::Error::from)??;
    let s = lock(&state)?;
    let f = project_facts(s.doc()?);
    Ok(match studio_specs::edit::apply(&book, &edit, &f) {
        Ok(after) => SpecEditPlan {
            changes: studio_specs::edit::preview(&book, &after),
            edit,
            error: None,
        },
        Err(e) => SpecEditPlan {
            edit,
            changes: vec![],
            error: Some(e),
        },
    })
}

fn ask(key: &str, summary: &str, prompt: &str) -> anyhow::Result<SpecEdit> {
    let request = studio_sync::claude::Request {
        model: "claude-opus-5-5".into(),
        system: studio_specs::edit::system_prompt(),
        text: format!("{summary}\n\nRequest: {}", prompt.trim()),
        images: vec![],
        tool_name: "edit_specs".into(),
        tool_description: "Change the project manual: a list of operations run in order.".into(),
        tool_schema: studio_specs::edit::schema(),
        max_tokens: 32_000,
    };
    let answer = studio_sync::claude::call(key, &request, &mut |_| {})?;
    serde_json::from_value(answer)
        .map_err(|e| anyhow::anyhow!("Claude's answer doesn't fit the specifications: {e}"))
}

/// Applies a previewed edit as one undo step.
#[tauri::command]
pub fn spec_edit_apply(
    edit: SpecEdit,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit_book(&window, &state, |d| {
        let book = specs::book(d).ok_or_else(|| {
            studio_core::CoreError::Invalid("generate the project manual first".into())
        })?;
        let f = project_facts(d);
        let after =
            studio_specs::edit::apply(&book, &edit, &f).map_err(studio_core::CoreError::Invalid)?;
        let label = if edit.summary.trim().is_empty() {
            "Edit Specs".to_string()
        } else {
            format!("Edit Specs: {}", edit.summary.trim())
        };
        specs::save(d, &label, after)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_house_gets_a_book_that_exports() {
        let mut s = crate::session::Session::default();
        s.new_sample("0").unwrap();
        s.edit(|d| {
            let f = project_facts(d);
            specs::save(
                d,
                "Generate",
                generate(&f, "csi-modern", "Bid Set", "2026-10-01"),
            )
        })
        .unwrap();
        let doc = s.doc().unwrap();
        let book = specs::book(doc).unwrap();
        assert!(book.sections.len() > 50, "{}", book.sections.len());
        let st = style(&book.style);
        let laid = studio_specs::layout::layout(&book, &st, &front(doc), true);
        // The drawing list is written from the sample's sheets.
        assert!(!front(doc).sheets.is_empty());
        let pdf = studio_specs::pdf::export(&laid, st.font, st.margin).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        let docx = studio_specs::docx::export(&laid, &st, "Project Manual");
        assert!(docx.starts_with(b"PK"));
        // Nothing the model calls for is missing, right after generating.
        assert!(update(&book, &project_facts(doc)).1.added.is_empty());
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// Asks Claude for a real edit of the sample manual (uses API credit):
    /// cargo test -p rufplan-studio live_spec_edit -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_spec_edit() {
        let key = get(CLAUDE_KEY).expect("a Claude key in the credential store");
        let mut s = crate::session::Session::default();
        s.new_sample("0").unwrap();
        s.edit(|d| {
            let f = project_facts(d);
            specs::save(
                d,
                "Generate",
                generate(&f, "csi-classic", "Bid Set", "2026-10-01"),
            )
        })
        .unwrap();
        let doc = s.doc().unwrap();
        let book = specs::book(doc).unwrap();
        let f = project_facts(doc);
        let prompt =
            "Make all gypsum board 5/8 inch Type X and add a mold-resistant board for wet areas";
        let summary = studio_specs::edit::describe(&book, Some("09 29 00"), prompt, &f);
        let edit = ask(&key, &summary, prompt).unwrap();
        let after = studio_specs::edit::apply(&book, &edit, &f).unwrap();
        for c in studio_specs::edit::preview(&book, &after) {
            println!("{} {} +{} -{}", c.kind, c.number, c.added, c.removed);
        }
        assert!(after.section("09 29 00").unwrap().edited);
    }
}
