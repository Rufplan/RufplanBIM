//! OAuth sign-in in the system browser (RFC 8252): a PKCE pair, and a one-shot loopback
//! listener that catches the redirect carrying the authorization code.
//!
//! The redirect URL, [`redirect_url`], must be in the Supabase project's allowed
//! redirect URLs (Authentication → URL Configuration).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::{SyncError, SyncResult};

/// Fixed so the allow-list entry is exact.
pub const LOOPBACK_PORT: u16 = 53682;

pub fn redirect_url() -> String {
    format!("http://127.0.0.1:{LOOPBACK_PORT}/auth/callback")
}

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> Self {
        // 64 hex characters from two random v4 UUIDs: 244 bits of entropy.
        let verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let challenge = challenge_for(&verifier);
        Self {
            verifier,
            challenge,
        }
    }
}

impl Default for Pkce {
    fn default() -> Self {
        Self::new()
    }
}

/// S256 code challenge: base64url(sha256(verifier)), unpadded.
pub fn challenge_for(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Binds the loopback port; call before opening the browser.
pub fn listen() -> SyncResult<TcpListener> {
    TcpListener::bind(("127.0.0.1", LOOPBACK_PORT)).map_err(|e| {
        SyncError::Network(format!(
            "could not listen on port {LOOPBACK_PORT} for the sign-in redirect: {e}"
        ))
    })
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A query parameter from a request line such as `GET /auth/callback?code=x HTTP/1.1`.
fn param(target: &str, key: &str) -> Option<String> {
    let query = target.split_once('?')?.1;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        (k == key).then(|| decode(v))
    })
}

const PAGE: &str = r#"<!doctype html><meta charset="utf-8"><title>Rufplan Studio</title>
<body style="margin:0;display:grid;place-items:center;height:100vh;background:#0a0a0a;color:#f8f8f6;font:16px system-ui,sans-serif">
<div style="text-align:center"><div style="letter-spacing:.2em;text-transform:uppercase;color:#3ECFF7;font-size:12px">Rufplan Studio</div>
<h1 style="font-weight:600">{title}</h1><p style="opacity:.7">{body}</p></div>"#;

fn page(title: &str, body: &str) -> String {
    PAGE.replace("{title}", title).replace("{body}", body)
}

/// Waits for the browser to come back with `?code=…`; returns the code.
pub fn wait_for_code(listener: &TcpListener, timeout: Duration) -> SyncResult<String> {
    let deadline = Instant::now() + timeout;
    listener
        .set_nonblocking(true)
        .map_err(|e| SyncError::Network(e.to_string()))?;
    loop {
        let (mut stream, _) = match listener.accept() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err(SyncError::Api("sign-in timed out".into()));
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => return Err(SyncError::Network(e.to_string())),
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let head = String::from_utf8_lossy(&buf[..n]);
        let target = head.split_whitespace().nth(1).unwrap_or("").to_owned();
        if !target.starts_with("/auth/callback") {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            continue;
        }
        let (result, html) = if let Some(code) = param(&target, "code") {
            (
                Ok(code),
                page(
                    "You're signed in",
                    "You can close this tab and return to Rufplan Studio.",
                ),
            )
        } else {
            let why = param(&target, "error_description")
                .or_else(|| param(&target, "error"))
                .unwrap_or_else(|| "no authorization code was returned".into());
            (
                Err(SyncError::Api(why.clone())),
                page("Sign-in failed", &why.replace('<', "&lt;")),
            )
        };
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
            html.len()
        );
        let _ = stream.write_all(reply.as_bytes());
        return result;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_rfc_7636_example() {
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let p = Pkce::new();
        assert_eq!(p.verifier.len(), 64);
        assert_eq!(p.challenge, challenge_for(&p.verifier));
    }

    #[test]
    fn query_params_are_decoded() {
        let t = "/auth/callback?code=ab%2Dc&error_description=Email+not%20confirmed";
        assert_eq!(param(t, "code").as_deref(), Some("ab-c"));
        assert_eq!(
            param(t, "error_description").as_deref(),
            Some("Email not confirmed")
        );
        assert_eq!(param(t, "missing"), None);
    }

    #[test]
    fn loopback_catches_the_redirect() {
        // Any free port works for the test; the real one is fixed.
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::thread::spawn(move || {
            let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
            let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.write_all(b"GET /auth/callback?code=xyz HTTP/1.1\r\nHost: x\r\n\r\n")
                .unwrap();
            let mut reply = String::new();
            s.read_to_string(&mut reply).unwrap();
            reply
        });
        let code = wait_for_code(&listener, Duration::from_secs(10)).unwrap();
        assert_eq!(code, "xyz");
        assert!(client.join().unwrap().contains("signed in"));
    }
}
