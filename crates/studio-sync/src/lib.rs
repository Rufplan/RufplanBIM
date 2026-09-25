//! Supabase auth, publishing to Rufplan and (later) worksharing.
//!
//! Talks to the Rufplan.io Supabase project with the **anon** key only; every read and
//! write is governed by Rufplan's row-level security (ADR-016). Publishing writes into
//! the tables and bucket Rufplan's own "Upload Design Set" flow uses, so no schema
//! change is needed: files go to `project-media` and each gets a
//! `project_deliverables` row.
//!
//! HTTP goes through the [`Http`] trait so the client is tested offline.

pub mod gis;
pub mod pkce;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The Rufplan.io Supabase project (not a secret; the anon key comes from the build).
pub const SUPABASE_URL: &str = "https://gdqvrhoufwklgcanfjpj.supabase.co";
/// Where Rufplan keeps deliverable files.
pub const BUCKET: &str = "project-media";
/// Public site, for links to a project page.
pub const RUFPLAN_SITE: &str = "https://rufplan.io";

/// Version of this crate, from Cargo metadata.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("could not reach Rufplan: {0}")]
    Network(String),
    #[error("{0}")]
    Api(String),
    #[error("sign in to Rufplan first")]
    SignedOut,
    #[error("unexpected response from Rufplan: {0}")]
    Decode(String),
}

pub type SyncResult<T> = Result<T, SyncError>;

/// A minimal HTTP request; bodies are bytes, headers are sent as given.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

pub trait Http: Send + Sync {
    /// Sends `req`. Non-2xx statuses are responses, not errors.
    fn send(&self, req: Request) -> Result<Response, String>;
}

/// Blocking HTTPS via ureq + rustls, trusting the OS certificate store (so machines behind
/// TLS-inspecting antivirus or proxies work like the browser does).
pub struct UreqHttp(ureq::Agent);

impl Default for UreqHttp {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .timeout_global(Some(Duration::from_secs(300)))
            .build()
            .into();
        Self(agent)
    }
}

impl Http for UreqHttp {
    fn send(&self, req: Request) -> Result<Response, String> {
        let mut b = ureq::http::Request::builder()
            .method(req.method)
            .uri(&req.url);
        for (k, v) in &req.headers {
            b = b.header(k, v);
        }
        let result = if req.body.is_empty() {
            self.0.run(b.body(()).map_err(|e| e.to_string())?)
        } else {
            self.0.run(b.body(req.body).map_err(|e| e.to_string())?)
        };
        let mut resp = result.map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp
            .body_mut()
            .with_config()
            .limit(64 << 20)
            .read_to_vec()
            .map_err(|e| e.to_string())?;
        Ok(Response { status, body })
    }
}

/// A signed-in Rufplan user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds.
    pub expires_at: u64,
    pub user_id: String,
    pub email: String,
    pub display_name: String,
}

/// A Rufplan project the user owns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProject {
    pub id: String,
    pub name: String,
    pub slug: String,
}

/// One file to publish into a deliverable.
#[derive(Debug, Clone)]
pub struct PublishFile {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub content_type: String,
    /// Rufplan deliverable slot: "drawings" (the compiled set) or a discipline code ("A").
    pub slot: String,
    pub sheet_count: Option<u32>,
    /// If Rufplan refuses an optional file (e.g. a type the bucket does not allow), the
    /// publish carries on and reports it in [`PublishReceipt::skipped`].
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct PublishPackage {
    pub project_id: String,
    pub project_slug: String,
    /// "sd", "dd" or "cd" (see [`phase_kind_for_stage`]).
    pub phase_kind: String,
    /// Catalog id such as "sd60" or "permit" (see [`deliverables`]).
    pub deliverable_id: String,
    pub notes: String,
    pub files: Vec<PublishFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishReceipt {
    pub project_url: String,
    /// `project_deliverables` row ids, one per file.
    pub rows: Vec<String>,
    pub file_urls: Vec<String>,
    /// Optional files Rufplan refused, as (filename, reason).
    pub skipped: Vec<(String, String)>,
}

/// Rufplan's deliverables tab for a design stage (by abbreviation): PD and SD file under
/// Schematic Design, DD under Design Development, CD / BN / CA under Construction Docs.
pub fn phase_kind_for_stage(abbreviation: &str) -> &'static str {
    match abbreviation {
        "DD" => "dd",
        "CD" | "BN" | "CA" => "cd",
        _ => "sd",
    }
}

/// Rufplan's deliverable catalog per phase kind, as (id, label); mirrors the Rufplan
/// project page. Rufplan also accepts custom ids, but these get proper cards.
pub fn deliverables(phase_kind: &str) -> &'static [(&'static str, &'static str)] {
    match phase_kind {
        "sd" => &[
            ("sd30", "30% Schematic Design"),
            ("sd60", "60% Schematic Design"),
            ("sd90", "90% Schematic Design"),
            ("sd100", "100% Schematic Design"),
            ("sd-alt", "Alternates Study"),
            ("sd-owner", "Owner Review Set"),
        ],
        "dd" => &[
            ("dd30", "30% Design Development"),
            ("dd60", "60% Design Development"),
            ("dd90", "90% Design Development"),
            ("dd100", "100% Design Development"),
            ("dd-owner", "Owner Review Set"),
            ("dd-ve", "Value Engineering Set"),
        ],
        "cd" => &[
            ("cd30", "30% Construction Documents"),
            ("cd60", "60% Construction Documents"),
            ("cd90", "90% Construction Documents"),
            ("cd100", "100% Construction Documents"),
            ("bid", "Bid Set"),
            ("permit", "Permit Set"),
            ("ifc", "IFC — Issued for Construction"),
        ],
        _ => &[],
    }
}

/// Same sanitizing as Rufplan's `uploadDeliverable`, so paths look alike.
fn safe(s: &str, fallback: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if t.is_empty() {
        fallback.into()
    } else {
        t
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// Percent-encodes a query parameter value.
pub(crate) fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The best human message in a Supabase error body.
fn api_error(status: u16, body: &[u8]) -> SyncError {
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let msg = ["error_description", "msg", "message", "error"]
        .iter()
        .find_map(|k| v.get(k).and_then(Value::as_str))
        .map(str::to_owned)
        .unwrap_or_else(|| String::from_utf8_lossy(body).chars().take(200).collect());
    let msg = if msg.is_empty() {
        format!("request failed ({status})")
    } else {
        msg
    };
    SyncError::Api(msg)
}

pub struct Client {
    http: Box<dyn Http>,
    url: String,
    anon_key: String,
    session: Option<AuthSession>,
}

impl Client {
    pub fn new(http: Box<dyn Http>, url: &str, anon_key: &str) -> Self {
        Self {
            http,
            url: url.trim_end_matches('/').to_owned(),
            anon_key: anon_key.to_owned(),
            session: None,
        }
    }

    pub fn session(&self) -> Option<&AuthSession> {
        self.session.as_ref()
    }

    fn send(
        &self,
        method: &'static str,
        path: &str,
        bearer: Option<&str>,
        extra: &[(&str, &str)],
        body: Vec<u8>,
    ) -> SyncResult<Response> {
        let mut headers = vec![
            ("apikey".to_owned(), self.anon_key.clone()),
            (
                "Authorization".to_owned(),
                format!("Bearer {}", bearer.unwrap_or(&self.anon_key)),
            ),
        ];
        headers.extend(
            extra
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
        let resp = self
            .http
            .send(Request {
                method,
                url: format!("{}{path}", self.url),
                headers,
                body,
            })
            .map_err(SyncError::Network)?;
        if (200..300).contains(&resp.status) {
            Ok(resp)
        } else {
            Err(api_error(resp.status, &resp.body))
        }
    }

    fn token(&mut self, grant: &str, body: Value) -> SyncResult<&AuthSession> {
        let resp = self.send(
            "POST",
            &format!("/auth/v1/token?grant_type={grant}"),
            None,
            &[("Content-Type", "application/json")],
            body.to_string().into_bytes(),
        )?;
        let v: Value =
            serde_json::from_slice(&resp.body).map_err(|e| SyncError::Decode(e.to_string()))?;
        let text = |p: &str| {
            v.pointer(p)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        };
        let expires_at = v
            .get("expires_at")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                now_secs() + v.get("expires_in").and_then(Value::as_u64).unwrap_or(3600)
            });
        let email = text("/user/email");
        let display_name = [
            text("/user/user_metadata/full_name"),
            text("/user/user_metadata/name"),
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_else(|| email.clone());
        let session = AuthSession {
            access_token: text("/access_token"),
            refresh_token: text("/refresh_token"),
            expires_at,
            user_id: text("/user/id"),
            email,
            display_name,
        };
        if session.access_token.is_empty() || session.user_id.is_empty() {
            return Err(SyncError::Decode(
                "no session in the sign-in response".into(),
            ));
        }
        Ok(self.session.insert(session))
    }

    /// Email and password sign-in (the same accounts as rufplan.io).
    pub fn sign_in_password(&mut self, email: &str, password: &str) -> SyncResult<&AuthSession> {
        self.token(
            "password",
            json!({ "email": email.trim(), "password": password }),
        )
    }

    /// Where to send the browser for Google sign-in (PKCE; see [`pkce`]).
    pub fn oauth_url(&self, provider: &str, redirect_to: &str, challenge: &str) -> String {
        format!(
            "{}/auth/v1/authorize?provider={}&redirect_to={}&code_challenge={}&code_challenge_method=s256",
            self.url,
            encode(provider),
            encode(redirect_to),
            encode(challenge)
        )
    }

    /// Finishes an OAuth sign-in with the code the browser brought back.
    pub fn exchange_code(&mut self, code: &str, verifier: &str) -> SyncResult<&AuthSession> {
        self.token(
            "pkce",
            json!({ "auth_code": code, "code_verifier": verifier }),
        )
    }

    /// Signs in again from a stored refresh token.
    pub fn restore(&mut self, refresh_token: &str) -> SyncResult<&AuthSession> {
        self.token("refresh_token", json!({ "refresh_token": refresh_token }))
    }

    /// Forgets the session (and revokes it server-side, best effort).
    pub fn sign_out(&mut self) {
        if let Some(s) = self.session.take() {
            let _ = self.send(
                "POST",
                "/auth/v1/logout",
                Some(&s.access_token),
                &[],
                vec![],
            );
        }
    }

    /// The current session, refreshed if it expires within a minute.
    fn fresh(&mut self) -> SyncResult<AuthSession> {
        let s = self.session.clone().ok_or(SyncError::SignedOut)?;
        if s.expires_at > now_secs() + 60 {
            return Ok(s);
        }
        self.restore(&s.refresh_token).cloned()
    }

    /// Rufplan projects the signed-in user owns, by name.
    pub fn my_projects(&mut self) -> SyncResult<Vec<RemoteProject>> {
        let s = self.fresh()?;
        let resp = self.send(
            "GET",
            &format!(
                "/rest/v1/open_projects?select=id,name,slug&owner_business_id=eq.{}&order=name.asc",
                encode(&s.user_id)
            ),
            Some(&s.access_token),
            &[("Accept", "application/json")],
            vec![],
        )?;
        serde_json::from_slice(&resp.body).map_err(|e| SyncError::Decode(e.to_string()))
    }

    /// Uploads to the deliverables bucket; returns the file's public URL.
    fn upload(&mut self, path: &str, bytes: Vec<u8>, content_type: &str) -> SyncResult<String> {
        let s = self.fresh()?;
        self.send(
            "POST",
            &format!("/storage/v1/object/{BUCKET}/{path}"),
            Some(&s.access_token),
            &[("Content-Type", content_type), ("x-upsert", "false")],
            bytes,
        )?;
        Ok(format!(
            "{}/storage/v1/object/public/{BUCKET}/{path}",
            self.url
        ))
    }

    /// Uploads each file and records it as a deliverable on the Rufplan project.
    pub fn publish(&mut self, pkg: PublishPackage) -> SyncResult<PublishReceipt> {
        let s = self.fresh()?;
        let mut receipt = PublishReceipt {
            project_url: format!("{RUFPLAN_SITE}/projects/{}", pkg.project_slug),
            rows: vec![],
            file_urls: vec![],
            skipped: vec![],
        };
        for f in pkg.files {
            let size_mb = (f.bytes.len() as f64 / 1_048_576.0 * 100.0).round() / 100.0;
            let path = format!(
                "{}/deliverables/{}/{}/{}/{}-{}",
                s.user_id,
                safe(&pkg.project_id, "project"),
                safe(&pkg.phase_kind, "cd"),
                safe(&pkg.deliverable_id, "unknown"),
                now_millis(),
                safe(&f.filename, "file")
            );
            let url = match self.upload(&path, f.bytes, &f.content_type) {
                Ok(url) => url,
                Err(SyncError::Api(why)) if f.optional => {
                    receipt.skipped.push((f.filename, why));
                    continue;
                }
                Err(e) => return Err(e),
            };
            let row = json!({
                "project_id": pkg.project_id,
                "phase_kind": pkg.phase_kind,
                "deliverable_id": pkg.deliverable_id,
                "slot": f.slot,
                "filename": f.filename,
                "size_mb": size_mb,
                "sheet_count": f.sheet_count,
                "file_url": url,
                "uploaded_by": s.display_name,
                "uploader_id": s.user_id,
                "notes": if pkg.notes.is_empty() { Value::Null } else { json!(pkg.notes) },
            });
            let resp = self.send(
                "POST",
                "/rest/v1/project_deliverables",
                Some(&s.access_token),
                &[
                    ("Content-Type", "application/json"),
                    ("Prefer", "return=representation"),
                ],
                row.to_string().into_bytes(),
            )?;
            let v: Value =
                serde_json::from_slice(&resp.body).map_err(|e| SyncError::Decode(e.to_string()))?;
            let id = v
                .pointer("/0/id")
                .and_then(Value::as_str)
                .ok_or_else(|| SyncError::Decode("no deliverable id returned".into()))?;
            receipt.rows.push(id.to_owned());
            receipt.file_urls.push(url);
        }
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Answers requests from a script and records what was sent.
    #[derive(Clone, Default)]
    struct Fake {
        sent: Arc<Mutex<Vec<Request>>>,
    }

    impl Http for Fake {
        fn send(&self, req: Request) -> Result<Response, String> {
            let url = req.url.clone();
            self.sent.lock().unwrap().push(req);
            let (status, body) = if url.contains("/auth/v1/token") {
                if url.contains("grant_type=password") && !url.is_empty() {
                    let last = self.sent.lock().unwrap().last().unwrap().body.clone();
                    if String::from_utf8_lossy(&last).contains("wrong") {
                        (400, r#"{"error":"invalid_grant","error_description":"Invalid login credentials"}"#.to_owned())
                    } else {
                        (200, token_body(4_000_000_000))
                    }
                } else {
                    (200, token_body(4_000_000_000))
                }
            } else if url.contains("/rest/v1/open_projects") {
                (
                    200,
                    r#"[{"id":"p1","name":"Lake House","slug":"lake-house"}]"#.to_owned(),
                )
            } else if url.contains("/storage/v1/object/") && url.ends_with(".ifc") {
                (400, r#"{"statusCode":"415","error":"invalid_mime_type","message":"mime type application/x-step is not supported"}"#.to_owned())
            } else if url.contains("/storage/v1/object/") {
                (200, r#"{"Key":"x"}"#.to_owned())
            } else if url.contains("/rest/v1/project_deliverables") {
                (201, r#"[{"id":"row-1"}]"#.to_owned())
            } else {
                (404, "{}".to_owned())
            };
            Ok(Response {
                status,
                body: body.into_bytes(),
            })
        }
    }

    fn token_body(expires_at: u64) -> String {
        json!({
            "access_token": "at", "refresh_token": "rt", "expires_at": expires_at,
            "user": { "id": "u1", "email": "a@b.co", "user_metadata": { "full_name": "Ada Arch" } }
        })
        .to_string()
    }

    fn client() -> (Client, Fake) {
        let fake = Fake::default();
        (
            Client::new(Box::new(fake.clone()), "https://x.supabase.co/", "anon"),
            fake,
        )
    }

    fn header<'a>(r: &'a Request, k: &str) -> Option<&'a str> {
        r.headers
            .iter()
            .find(|(h, _)| h == k)
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn password_sign_in_keeps_the_session() {
        let (mut c, fake) = client();
        let s = c.sign_in_password(" a@b.co ", "pw").unwrap();
        assert_eq!(s.user_id, "u1");
        assert_eq!(s.display_name, "Ada Arch");
        let sent = fake.sent.lock().unwrap();
        assert_eq!(
            sent[0].url,
            "https://x.supabase.co/auth/v1/token?grant_type=password"
        );
        assert_eq!(header(&sent[0], "apikey"), Some("anon"));
        let body: Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["email"], "a@b.co");
    }

    #[test]
    fn bad_credentials_surface_supabase_message() {
        let (mut c, _) = client();
        let err = c.sign_in_password("a@b.co", "wrong").unwrap_err();
        assert_eq!(err.to_string(), "Invalid login credentials");
        assert!(c.session().is_none());
    }

    #[test]
    fn signed_out_calls_are_refused() {
        let (mut c, fake) = client();
        assert!(matches!(c.my_projects(), Err(SyncError::SignedOut)));
        assert!(fake.sent.lock().unwrap().is_empty());
    }

    #[test]
    fn lists_owned_projects_with_the_user_token() {
        let (mut c, fake) = client();
        c.sign_in_password("a@b.co", "pw").unwrap();
        let p = c.my_projects().unwrap();
        assert_eq!(p[0].slug, "lake-house");
        let sent = fake.sent.lock().unwrap();
        assert!(sent[1].url.contains("owner_business_id=eq.u1"));
        assert_eq!(header(&sent[1], "Authorization"), Some("Bearer at"));
    }

    #[test]
    fn expiring_session_refreshes_first() {
        let (mut c, fake) = client();
        c.sign_in_password("a@b.co", "pw").unwrap();
        c.session.as_mut().unwrap().expires_at = 1;
        c.my_projects().unwrap();
        let sent = fake.sent.lock().unwrap();
        assert!(sent[1].url.ends_with("grant_type=refresh_token"));
    }

    #[test]
    fn publish_uploads_then_records_each_file() {
        let (mut c, fake) = client();
        c.sign_in_password("a@b.co", "pw").unwrap();
        let r = c
            .publish(PublishPackage {
                project_id: "p1".into(),
                project_slug: "lake-house".into(),
                phase_kind: "sd".into(),
                deliverable_id: "sd60".into(),
                notes: "SD Review Set".into(),
                files: vec![PublishFile {
                    filename: "Lake House - SD Set.pdf".into(),
                    bytes: vec![0; 2_000_000],
                    content_type: "application/pdf".into(),
                    slot: "drawings".into(),
                    sheet_count: Some(4),
                    optional: false,
                }],
            })
            .unwrap();
        assert_eq!(r.project_url, "https://rufplan.io/projects/lake-house");
        assert_eq!(r.rows, vec!["row-1"]);
        let sent = fake.sent.lock().unwrap();
        let upload = &sent[1];
        assert!(upload.url.starts_with(
            "https://x.supabase.co/storage/v1/object/project-media/u1/deliverables/p1/sd/sd60/"
        ));
        assert!(upload.url.ends_with("-Lake_House_-_SD_Set.pdf"));
        assert_eq!(header(upload, "Content-Type"), Some("application/pdf"));
        let row: Value = serde_json::from_slice(&sent[2].body).unwrap();
        assert_eq!(row["slot"], "drawings");
        assert_eq!(row["uploader_id"], "u1");
        assert_eq!(row["sheet_count"], 4);
        assert_eq!(row["size_mb"], 1.91);
        assert!(row["file_url"]
            .as_str()
            .unwrap()
            .contains("/storage/v1/object/public/project-media/u1/"));
    }

    #[test]
    fn refused_optional_file_is_skipped_not_fatal() {
        let (mut c, _) = client();
        c.sign_in_password("a@b.co", "pw").unwrap();
        let file = |name: &str, optional| PublishFile {
            filename: name.into(),
            bytes: vec![1],
            content_type: "application/octet-stream".into(),
            slot: "A".into(),
            sheet_count: None,
            optional,
        };
        let pkg = |files| PublishPackage {
            project_id: "p1".into(),
            project_slug: "s".into(),
            phase_kind: "cd".into(),
            deliverable_id: "permit".into(),
            notes: String::new(),
            files,
        };
        let r = c
            .publish(pkg(vec![file("set.pdf", false), file("model.ifc", true)]))
            .unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.skipped[0].0, "model.ifc");
        assert!(r.skipped[0].1.contains("not supported"));
        assert!(c.publish(pkg(vec![file("model.ifc", false)])).is_err());
    }

    #[test]
    fn stages_map_to_rufplan_phases() {
        assert_eq!(phase_kind_for_stage("PD"), "sd");
        assert_eq!(phase_kind_for_stage("SD"), "sd");
        assert_eq!(phase_kind_for_stage("DD"), "dd");
        for s in ["CD", "BN", "CA"] {
            assert_eq!(phase_kind_for_stage(s), "cd");
        }
        assert!(deliverables("cd").iter().any(|d| d.0 == "permit"));
    }

    #[test]
    fn oauth_url_carries_pkce_challenge() {
        let (c, _) = client();
        let u = c.oauth_url("google", "http://127.0.0.1:53682/auth/callback", "abc");
        assert_eq!(
            u,
            "https://x.supabase.co/auth/v1/authorize?provider=google&redirect_to=http%3A%2F%2F127.0.0.1%3A53682%2Fauth%2Fcallback&code_challenge=abc&code_challenge_method=s256"
        );
    }
}
