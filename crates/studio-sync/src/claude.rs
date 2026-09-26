//! Claude (ADR-030): a Messages API call that streams a forced tool call, so the page can
//! show Claude planning as it goes. The API key comes from the OS credential store and is
//! only ever sent to api.anthropic.com.

use std::io::{BufRead, BufReader};
use std::time::Duration;

use serde_json::{json, Value};

use crate::{SyncError, SyncResult};

const URL: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";

/// An image attached as a reference.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInput {
    /// image/jpeg, image/png, image/webp or image/gif.
    pub media_type: String,
    /// Base64, without a data: prefix.
    pub data: String,
}

/// One request: the system prompt, the user's text and images, and the tool Claude must
/// answer with.
#[derive(Debug, Clone)]
pub struct Request {
    pub model: String,
    pub system: String,
    pub text: String,
    pub images: Vec<ImageInput>,
    pub tool_name: String,
    pub tool_description: String,
    pub tool_schema: Value,
    pub max_tokens: u32,
}

/// The request body: streaming, with the tool offered. Current models refuse a forced
/// tool choice ("tool"/"any"), so the choice is "auto" and the system prompt tells Claude
/// to answer only by calling it.
pub fn body(r: &Request) -> Value {
    let mut content: Vec<Value> = r
        .images
        .iter()
        .map(|i| {
            json!({
                "type": "image",
                "source": { "type": "base64", "media_type": i.media_type, "data": i.data }
            })
        })
        .collect();
    content.push(json!({ "type": "text", "text": r.text }));
    json!({
        "model": r.model,
        "max_tokens": r.max_tokens,
        "stream": true,
        "system": r.system,
        "messages": [{ "role": "user", "content": content }],
        "tools": [{
            "name": r.tool_name,
            "description": r.tool_description,
            "input_schema": r.tool_schema,
        }],
        "tool_choice": { "type": "auto" },
    })
}

/// Reads the event stream: calls `progress` with the tool input received so far, and
/// returns the tool input once the message ends.
pub fn read_stream(reader: impl BufRead, progress: &mut dyn FnMut(&str)) -> SyncResult<Value> {
    let mut json_text = String::new();
    // Anything Claude wrote instead of (or before) calling the tool.
    let mut text = String::new();
    let mut stop: Option<String> = None;
    for line in reader.lines() {
        let line = line.map_err(|e| SyncError::Network(format!("Claude: {e}")))?;
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let Ok(ev) = serde_json::from_str::<Value>(data.trim()) else {
            continue;
        };
        match ev.get("type").and_then(Value::as_str) {
            Some("content_block_delta") => {
                if let Some(part) = ev.pointer("/delta/partial_json").and_then(Value::as_str) {
                    json_text.push_str(part);
                    progress(&json_text);
                } else if let Some(part) = ev.pointer("/delta/text").and_then(Value::as_str) {
                    text.push_str(part);
                }
            }
            Some("message_delta") => {
                stop = ev
                    .pointer("/delta/stop_reason")
                    .and_then(Value::as_str)
                    .map(Into::into);
            }
            Some("error") => {
                let msg = ev
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error");
                return Err(SyncError::Api(format!("Claude: {msg}")));
            }
            Some("message_stop") => break,
            _ => {}
        }
    }
    if stop.as_deref() == Some("max_tokens") {
        return Err(SyncError::Api(
            "Claude's plan ran past its length limit; ask for fewer stories or rooms, or let it model units as single rooms".into(),
        ));
    }
    if json_text.trim().is_empty() {
        let said: String = text.trim().chars().take(400).collect();
        return Err(SyncError::Api(if said.is_empty() {
            "Claude returned no plan; try again".into()
        } else {
            format!("Claude answered without a plan: \"{said}\"")
        }));
    }
    serde_json::from_str(&json_text)
        .map_err(|e| SyncError::Decode(format!("Claude's plan isn't valid JSON: {e}")))
}

/// What an HTTP error from the API means for the owner.
pub fn explain(status: u16, body: &str) -> SyncError {
    let msg = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| body.chars().take(200).collect());
    match status {
        401 | 403 => SyncError::Api(format!(
            "Claude refused the API key; check it in Generate > Claude API key ({msg})"
        )),
        429 | 529 => SyncError::Api(format!(
            "Claude is busy or you've hit a rate limit; try again in a minute ({msg})"
        )),
        _ => SyncError::Api(format!("Claude returned {status}: {msg}")),
    }
}

/// Calls Claude, streaming. Blocks until the plan is complete.
pub fn call(api_key: &str, r: &Request, progress: &mut dyn FnMut(&str)) -> SyncResult<Value> {
    if api_key.trim().is_empty() {
        return Err(SyncError::Api("add your Claude API key first".into()));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        // A large building can take several minutes to plan.
        .timeout_global(Some(Duration::from_secs(900)))
        .build()
        .into();
    let req = ureq::http::Request::builder()
        .method("POST")
        .uri(URL)
        .header("x-api-key", api_key.trim())
        .header("anthropic-version", VERSION)
        .header("content-type", "application/json")
        .body(body(r).to_string())
        .map_err(|e| SyncError::Network(e.to_string()))?;
    let mut resp = agent
        .run(req)
        .map_err(|e| SyncError::Network(format!("couldn't reach Claude: {e}")))?;
    let status = resp.status().as_u16();
    if status != 200 {
        let text = resp.body_mut().read_to_string().unwrap_or_default();
        return Err(explain(status, &text));
    }
    let reader = BufReader::new(resp.into_body().into_reader());
    read_stream(reader, progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> Request {
        Request {
            model: "claude-opus-5-5".into(),
            system: "You are an architect.".into(),
            text: "A small house.".into(),
            images: vec![ImageInput {
                media_type: "image/jpeg".into(),
                data: "QUJD".into(),
            }],
            tool_name: "build_model".into(),
            tool_description: "Build it.".into(),
            tool_schema: json!({"type": "object"}),
            max_tokens: 1000,
        }
    }

    #[test]
    fn the_request_streams_and_offers_the_tool() {
        let b = body(&req());
        assert_eq!(b["stream"], true);
        assert_eq!(b["tool_choice"], json!({"type": "auto"}));
        assert_eq!(b["tools"][0]["input_schema"], json!({"type": "object"}));
        let content = b["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["media_type"], "image/jpeg");
        assert_eq!(
            content[1],
            json!({"type": "text", "text": "A small house."})
        );
    }

    #[test]
    fn the_stream_assembles_the_tool_input() {
        let sse = r#"event: message_start
data: {"type":"message_start","message":{"id":"m"}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t","name":"build_model","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"name\": \"Ho"}}

event: ping
data: {"type": "ping"}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"use\", \"stories\": []}"}}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"}}

event: message_stop
data: {"type":"message_stop"}
"#;
        let mut seen = vec![];
        let v = read_stream(sse.as_bytes(), &mut |s| seen.push(s.len())).unwrap();
        assert_eq!(v, json!({"name": "House", "stories": []}));
        assert_eq!(seen.len(), 2);
        assert!(seen[1] > seen[0]);
    }

    #[test]
    fn a_reply_without_the_tool_is_reported() {
        let sse = "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"How many bedrooms?\"}}\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\ndata: {\"type\":\"message_stop\"}\n";
        let e = read_stream(sse.as_bytes(), &mut |_| {})
            .unwrap_err()
            .to_string();
        assert!(
            e.contains("without a plan") && e.contains("How many bedrooms?"),
            "{e}"
        );
    }

    #[test]
    fn errors_are_explained() {
        let cut = "data: {\"type\":\"content_block_delta\",\"delta\":{\"partial_json\":\"{\\\"a\\\":\"}}\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"}}\n";
        let e = read_stream(cut.as_bytes(), &mut |_| {})
            .unwrap_err()
            .to_string();
        assert!(e.contains("length limit"), "{e}");
        let err = "data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n";
        assert!(read_stream(err.as_bytes(), &mut |_| {})
            .unwrap_err()
            .to_string()
            .contains("Overloaded"));
        assert!(explain(401, r#"{"error":{"message":"invalid x-api-key"}}"#)
            .to_string()
            .contains("refused the API key"));
        assert!(explain(529, "{}").to_string().contains("busy"));
        assert!(call(" ", &req(), &mut |_| {}).is_err());
    }
}
