//! Claude copilot — streams an advise-only response over SSE.
//!
//! The handler assembles real policy context (via `studio-core`, which never touches keys), calls
//! the Anthropic Messages API with streaming, and relays text deltas to the browser. It performs no
//! signing or mutation: Claude proposes; the human applies and signs through the ordinary routes.
//! Absent `ANTHROPIC_API_KEY`, the copilot degrades to a single "disabled" event.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::Json;
use serde::Deserialize;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

use studio_core::copilot;
use studio_core::workspace::PolicyId;
use studio_core::Workspace;

use crate::app::AppState;

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-opus-4-8";

#[derive(Deserialize)]
pub struct CopilotReq {
    pub policy_id: String,
    #[serde(default)]
    pub surface: String,
    pub prompt: String,
}

/// `POST /api/copilot/stream` — stream a copilot answer for a policy + prompt.
pub async fn stream(
    State(st): State<AppState>,
    Json(req): Json<CopilotReq>,
) -> Sse<impl tokio_stream::Stream<Item = Result<SseEvent, Infallible>>> {
    let (tx, rx) = unbounded_channel::<SseEvent>();
    let ws = st.ws.clone();
    tokio::spawn(async move { run(ws, req, tx).await });
    Sse::new(UnboundedReceiverStream::new(rx).map(Ok)).keep_alive(KeepAlive::default())
}

async fn run(ws: Arc<Workspace>, req: CopilotReq, tx: UnboundedSender<SseEvent>) {
    let Ok(id) = PolicyId::new(&req.policy_id) else {
        let _ = tx.send(event(&serde_json::json!({ "error": "invalid policy id" })));
        return;
    };
    let key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            let _ = tx.send(event(&serde_json::json!({
                "disabled": true,
                "message": "Copilot disabled — set ANTHROPIC_API_KEY to enable it."
            })));
            return;
        }
    };
    let surface = if req.surface.is_empty() {
        "editor"
    } else {
        &req.surface
    };
    let ctx = match copilot::build_context(&ws, &id, surface) {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(event(&serde_json::json!({ "error": e.to_string() })));
            return;
        }
    };
    let body = serde_json::json!({
        "model": std::env::var("COPILOT_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
        "max_tokens": 1500,
        "system": ctx.system,
        "stream": true,
        "messages": [ { "role": "user", "content": format!("{}\n\nRequest: {}", ctx.context, req.prompt) } ],
    });

    let relay = tx.clone();
    let outcome = tokio::task::spawn_blocking(move || call_anthropic(&key, &body, &relay)).await;
    if let Ok(Err(msg)) = outcome {
        let _ = tx.send(event(&serde_json::json!({ "error": msg })));
    }
    let _ = tx.send(event(&serde_json::json!({ "done": true })));
}

/// Blocking call to Anthropic; forwards each text delta as a `token` event. Returns a human error.
fn call_anthropic(
    key: &str,
    body: &serde_json::Value,
    tx: &UnboundedSender<SseEvent>,
) -> Result<(), String> {
    let resp = ureq::post(ANTHROPIC_URL)
        .set("x-api-key", key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| match e {
            ureq::Error::Status(code, r) => {
                format!("anthropic {code}: {}", r.into_string().unwrap_or_default())
            }
            other => other.to_string(),
        })?;

    use std::io::BufRead;
    let reader = std::io::BufReader::new(resp.into_reader());
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let Some(data) = line.strip_prefix("data: ") else {
            continue;
        };
        if data == "[DONE]" {
            break;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else {
            continue;
        };
        match v["type"].as_str() {
            Some("content_block_delta") => {
                if let Some(t) = v["delta"]["text"].as_str() {
                    let _ = tx.send(event(&serde_json::json!({ "token": t })));
                }
            }
            Some("message_stop") => break,
            _ => {}
        }
    }
    Ok(())
}

fn event(value: &serde_json::Value) -> SseEvent {
    SseEvent::default().json_data(value).unwrap_or_default()
}
