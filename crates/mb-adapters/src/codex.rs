//! Codex CLI in-process adapter.
//!
//! Watches `~/.codex/sessions` for JSON/JSONL session journals and maps
//! coarse lifecycle fields onto [`mb_protocol::AgentState`]. Action routing
//! (approve/reject) is logged until the local Codex surface exposes a stable
//! hook — see the adapter README notes in `docs/adapters.md`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mb_protocol::{AgentState, SessionStatus};
use serde_json::Value;
use tracing::{debug, warn};

use crate::watch::watch_dir;
use crate::{AdapterEvent, AdapterTx};

pub fn spawn_codex_adapter(tx: AdapterTx) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let root = PathBuf::from(home).join(".codex").join("sessions");
    let seen: Arc<Mutex<HashMap<String, AgentState>>> = Arc::new(Mutex::new(HashMap::new()));

    let seen_cb = Arc::clone(&seen);
    watch_dir(root, move |path| {
        if let Some(session) = parse_codex_session(&path) {
            let mut map = seen_cb.lock().unwrap();
            let prev = map.get(&session.id).copied();
            if prev == Some(session.state) {
                return;
            }
            map.insert(session.id.clone(), session.state);
            drop(map);
            debug!(id = %session.id, ?session.state, "codex session");
            let _ = tx.send(AdapterEvent::Upsert(session));
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "json" || e == "jsonl")
        {
            // File removed or unreadable — best-effort bye from filename.
            if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                let sid = format!("codex:{id}");
                let mut map = seen_cb.lock().unwrap();
                if map.remove(&sid).is_some() {
                    let _ = tx.send(AdapterEvent::Remove(sid));
                }
            }
        }
    });
}

fn parse_codex_session(path: &std::path::Path) -> Option<SessionStatus> {
    let text = std::fs::read_to_string(path).ok()?;
    // Prefer last JSON object from jsonl; otherwise whole file.
    let value = if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
        let last = text.lines().rev().find(|l| !l.trim().is_empty())?;
        serde_json::from_str::<Value>(last).ok()?
    } else {
        serde_json::from_str::<Value>(&text).ok()?
    };

    let id_raw = value
        .get("id")
        .or_else(|| value.get("session_id"))
        .or_else(|| value.get("thread_id"))
        .and_then(|v| v.as_str())
        .or_else(|| path.file_stem().and_then(|s| s.to_str()))?;

    let title = value
        .get("title")
        .or_else(|| value.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let state = map_state(&value);
    let updated_at_ms = value
        .get("updated_at_ms")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            value
                .get("updated_at")
                .and_then(|v| v.as_str())
                .and_then(parse_iso_ms)
        })
        .unwrap_or_else(now_ms);

    Some(SessionStatus {
        id: format!("codex:{id_raw}"),
        app: "Codex CLI".into(),
        title,
        state,
        updated_at_ms,
    })
}

fn map_state(value: &Value) -> AgentState {
    let raw = value
        .get("state")
        .or_else(|| value.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("idle")
        .to_ascii_lowercase();

    match raw.as_str() {
        "thinking" | "reasoning" => AgentState::Thinking,
        "working" | "running" | "in_progress" | "active" => AgentState::Working,
        "awaiting_approval" | "awaiting_input" | "needs_approval" | "approval" => {
            AgentState::AwaitingApproval
        }
        "done" | "completed" | "complete" | "finished" => AgentState::Done,
        "error" | "failed" => AgentState::Error,
        "idle" | "ready" => AgentState::Idle,
        other => {
            warn!(state = other, "unknown codex state; treating as idle");
            AgentState::Idle
        }
    }
}

fn parse_iso_ms(_s: &str) -> Option<u64> {
    None
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_awaiting_approval() {
        let v = json!({"status": "awaiting_input"});
        assert_eq!(map_state(&v), AgentState::AwaitingApproval);
    }
}
