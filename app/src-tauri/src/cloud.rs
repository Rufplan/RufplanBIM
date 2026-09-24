//! Rufplan.io account, project link and publishing (ADR-016). Network calls run on a
//! blocking thread so the UI stays responsive; the refresh token lives in the OS
//! credential store and the access token only in memory.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::Serialize;
use studio_core::{ops, ElementData, RufplanLink};
use studio_sync::{pkce, Client, PublishFile, PublishPackage, SyncError, UreqHttp};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use crate::commands::{finish, today, CommandError, SessionState};
use crate::session::AppState;

type CommandResult<T> = Result<T, CommandError>;

const KEYRING_SERVICE: &str = "Rufplan Studio";
const KEYRING_USER: &str = "rufplan-refresh-token";

/// Baked in by build.rs from RUFPLAN_SUPABASE_ANON_KEY; absent in unconfigured builds.
fn anon_key() -> Option<&'static str> {
    option_env!("RUFPLAN_SUPABASE_ANON_KEY").filter(|k| !k.is_empty())
}

#[derive(Default)]
pub struct Cloud {
    client: Option<Client>,
    tried_restore: bool,
}

pub type CloudState = Arc<Mutex<Cloud>>;

impl Cloud {
    fn client(&mut self) -> anyhow::Result<&mut Client> {
        let key = anon_key().ok_or_else(|| {
            anyhow::anyhow!("Rufplan sign-in isn't configured in this build (no anon key)")
        })?;
        Ok(self.client.get_or_insert_with(|| {
            Client::new(
                Box::new(UreqHttp::default()),
                studio_sync::SUPABASE_URL,
                key,
            )
        }))
    }

    /// Signs in from the stored refresh token, once per run.
    fn restore_once(&mut self) {
        if self.tried_restore || anon_key().is_none() {
            return;
        }
        self.tried_restore = true;
        let Some(token) = keyring_get() else { return };
        let Ok(client) = self.client() else { return };
        match client.restore(&token) {
            Ok(s) => keyring_set(&s.refresh_token.clone()),
            // A revoked or expired token: forget it. Network errors keep it for next time.
            Err(SyncError::Api(_)) => keyring_delete(),
            Err(_) => {}
        }
    }

    fn status(&self) -> CloudStatus {
        let s = self.client.as_ref().and_then(|c| c.session());
        CloudStatus {
            configured: anon_key().is_some(),
            signed_in: s.is_some(),
            email: s.map(|s| s.email.clone()),
            name: s.map(|s| s.display_name.clone()),
        }
    }

    /// Stores the (possibly rotated) refresh token after any authenticated call.
    fn remember(&mut self) {
        if let Some(s) = self.client.as_ref().and_then(|c| c.session()) {
            keyring_set(&s.refresh_token.clone());
        }
    }
}

fn keyring_entry() -> Option<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()
}
fn keyring_get() -> Option<String> {
    keyring_entry()?.get_password().ok()
}
fn keyring_set(token: &str) {
    if let Some(e) = keyring_entry() {
        let _ = e.set_password(token);
    }
}
fn keyring_delete() {
    if let Some(e) = keyring_entry() {
        let _ = e.delete_credential();
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloudStatus {
    /// False when this build has no Rufplan anon key.
    pub configured: bool,
    pub signed_in: bool,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Deliverable {
    pub id: String,
    pub label: String,
}

/// What Publish offers for the current design stage.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PublishOptions {
    /// Rufplan phase tab: "sd", "dd" or "cd".
    pub phase_kind: String,
    pub stage: String,
    pub deliverables: Vec<Deliverable>,
    /// Sheets in the current stage's set.
    pub sheet_count: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PublishResult {
    pub state: Option<AppState>,
    /// The Rufplan project page.
    pub url: String,
    pub published: Vec<String>,
    /// Files Rufplan refused, with the reason.
    pub skipped: Vec<String>,
}

fn lock(cloud: &CloudState) -> anyhow::Result<MutexGuard<'_, Cloud>> {
    cloud
        .lock()
        .map_err(|_| anyhow::anyhow!("account state is unavailable after an earlier crash"))
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(anyhow::Error::from)?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn cloud_status(cloud: State<'_, CloudState>) -> CommandResult<CloudStatus> {
    let cloud = cloud.inner().clone();
    blocking(move || {
        let mut c = lock(&cloud)?;
        c.restore_once();
        Ok(c.status())
    })
    .await
}

#[tauri::command]
pub async fn cloud_sign_in(
    email: String,
    password: String,
    cloud: State<'_, CloudState>,
) -> CommandResult<CloudStatus> {
    let cloud = cloud.inner().clone();
    blocking(move || {
        let mut c = lock(&cloud)?;
        c.tried_restore = true;
        c.client()?.sign_in_password(&email, &password)?;
        c.remember();
        Ok(c.status())
    })
    .await
}

/// Google sign-in in the system browser; resolves when the browser comes back.
#[tauri::command]
pub async fn cloud_sign_in_google(
    app: AppHandle,
    cloud: State<'_, CloudState>,
) -> CommandResult<CloudStatus> {
    let cloud = cloud.inner().clone();
    blocking(move || {
        let pair = pkce::Pkce::new();
        let listener = pkce::listen()?;
        let url =
            lock(&cloud)?
                .client()?
                .oauth_url("google", &pkce::redirect_url(), &pair.challenge);
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|e| anyhow::anyhow!("could not open the browser: {e}"))?;
        // The lock is not held while waiting, so status calls keep answering.
        let code = pkce::wait_for_code(&listener, Duration::from_secs(300))?;
        let mut c = lock(&cloud)?;
        c.tried_restore = true;
        c.client()?.exchange_code(&code, &pair.verifier)?;
        c.remember();
        Ok(c.status())
    })
    .await
}

#[tauri::command]
pub async fn cloud_sign_out(cloud: State<'_, CloudState>) -> CommandResult<CloudStatus> {
    let cloud = cloud.inner().clone();
    blocking(move || {
        let mut c = lock(&cloud)?;
        if let Some(client) = c.client.as_mut() {
            client.sign_out();
        }
        keyring_delete();
        Ok(c.status())
    })
    .await
}

/// Rufplan projects the user owns, to link this model to.
#[tauri::command]
pub async fn cloud_projects(cloud: State<'_, CloudState>) -> CommandResult<Vec<RufplanLink>> {
    let cloud = cloud.inner().clone();
    blocking(move || {
        let mut c = lock(&cloud)?;
        let projects = c.client()?.my_projects()?;
        c.remember();
        Ok(projects
            .into_iter()
            .map(|p| RufplanLink {
                id: p.id,
                name: p.name,
                slug: p.slug,
            })
            .collect())
    })
    .await
}

/// Links the open model to a Rufplan project (`None` unlinks). Undoable.
#[tauri::command]
pub fn link_rufplan(
    link: Option<RufplanLink>,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> CommandResult<Option<AppState>> {
    let mut session = crate::commands::lock(&state)?;
    session.edit(|d| ops::link_rufplan(d, link))?;
    finish(&window, &session)
}

fn current_stage(doc: &studio_core::Document) -> (Option<studio_core::ElementId>, String) {
    let stage = ops::project_info(doc).and_then(|i| match doc.data(i) {
        Ok(ElementData::ProjectInfo { current_stage, .. }) => *current_stage,
        _ => None,
    });
    let abbr = ops::stages(doc)
        .into_iter()
        .find(|s| Some(s.0) == stage)
        .map(|s| s.2)
        .unwrap_or_default();
    (stage, abbr)
}

#[tauri::command]
pub fn publish_options(state: State<'_, SessionState>) -> CommandResult<PublishOptions> {
    let session = crate::commands::lock(&state)?;
    let doc = session.doc()?;
    let (stage, abbr) = current_stage(doc);
    let phase = studio_sync::phase_kind_for_stage(&abbr);
    Ok(PublishOptions {
        phase_kind: phase.to_owned(),
        stage: abbr,
        deliverables: studio_sync::deliverables(phase)
            .iter()
            .map(|(id, label)| Deliverable {
                id: (*id).to_owned(),
                label: (*label).to_owned(),
            })
            .collect(),
        sheet_count: ops::stage_sheets(doc, stage).len() as u32,
    })
}

/// Issues the current stage's sheet set to the linked Rufplan project: uploads the PDF
/// (and the IFC model) into deliverable `deliverable`, then records the issuance.
#[tauri::command]
pub async fn publish_to_rufplan(
    name: String,
    deliverable: String,
    window: WebviewWindow,
    state: State<'_, SessionState>,
    cloud: State<'_, CloudState>,
) -> CommandResult<PublishResult> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(anyhow::anyhow!("name the issue first").into());
    }
    // Build the files from a snapshot, then let go of the model while uploading.
    let (pkg, sheets) = {
        let session = crate::commands::lock(&state)?;
        let doc = session.doc()?;
        let link = ops::rufplan_link(doc)
            .ok_or_else(|| anyhow::anyhow!("link this model to a Rufplan project first"))?;
        let (stage, abbr) = current_stage(doc);
        let sheets = ops::stage_sheets(doc, stage);
        if sheets.is_empty() {
            return Err(anyhow::anyhow!(
                "the {abbr} set has no sheets; add sheets to it in their Stage Sets properties"
            )
            .into());
        }
        let pdf = studio_sheets::export_pdf(doc, &sheets, &today()).map_err(anyhow::Error::from)?;
        let (ifc, _) =
            studio_io::ifc::export_ifc(doc, env!("CARGO_PKG_VERSION"), &crate::commands::now_iso());
        let project = session.state().map(|s| s.project_name).unwrap_or_default();
        let base = format!("{project} - {name}");
        let pkg = PublishPackage {
            project_id: link.id,
            project_slug: link.slug,
            phase_kind: studio_sync::phase_kind_for_stage(&abbr).to_owned(),
            deliverable_id: deliverable,
            notes: format!("{name} — published from Rufplan Studio"),
            files: vec![
                PublishFile {
                    filename: format!("{base}.pdf"),
                    bytes: pdf,
                    content_type: "application/pdf".into(),
                    slot: "drawings".into(),
                    sheet_count: Some(sheets.len() as u32),
                    optional: false,
                },
                PublishFile {
                    filename: format!("{base}.ifc"),
                    bytes: ifc.into_bytes(),
                    content_type: "application/x-step".into(),
                    slot: "A".into(),
                    sheet_count: None,
                    optional: true,
                },
            ],
        };
        (pkg, sheets)
    };
    let names: Vec<String> = pkg.files.iter().map(|f| f.filename.clone()).collect();
    let cloud = cloud.inner().clone();
    let receipt = blocking(move || {
        let mut c = lock(&cloud)?;
        let r = c.client()?.publish(pkg)?;
        c.remember();
        Ok(r)
    })
    .await?;
    // Record the issue so title blocks list it, as Issue Set does.
    let mut session = crate::commands::lock(&state)?;
    let date = today();
    session.edit(|d| ops::create_issuance(d, &name, &date, sheets))?;
    let skipped: Vec<String> = receipt
        .skipped
        .iter()
        .map(|(f, why)| format!("{f}: {why}"))
        .collect();
    Ok(PublishResult {
        state: finish(&window, &session)?,
        url: receipt.project_url,
        published: names
            .into_iter()
            .filter(|n| !receipt.skipped.iter().any(|(f, _)| f == n))
            .collect(),
        skipped,
    })
}
