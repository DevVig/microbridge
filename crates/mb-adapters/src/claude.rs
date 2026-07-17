//! Claude Code in-process adapter.
//!
//! Prefers project-local Claude session journals when present
//! (`~/.claude/projects` or `~/.config/claude`). Maps coarse status fields
//! onto [`mb_protocol::AgentState`]. Official hooks can later replace file
//! watching without changing the bus contract.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mb_protocol::{AgentState, SessionStatus};
use serde_json::Value;
use tracing::debug;

use crate::watch::watch_dir;
use crate::{AdapterEvent, AdapterTx};

pub fn spawn_claude_adapter(tx: AdapterTx) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let candidates = [
        PathBuf::from(&home).join(".claude").join("projects"),
        PathBuf::from(&home)
            .join(".config")
            .join("claude")
            .join("projects"),
    ];

    for root in candidates {
        let tx = tx.clone();
        let seen: Arc<Mutex<HashMap<String, AgentState>>> = Arc::new(Mutex::new(HashMap::new()));
        let seen_cb = Arc::clone(&seen);
        watch_dir(root, move |path| {
            if let Some(session) = parse_claude_session(&path) {
                let mut map = seen_cb.lock().unwrap();
                if map.get(&session.id) == Some(&session.state) {
                    return;
                }
                map.insert(session.id.clone(), session.state);
                drop(map);
                debug!(id = %session.id, ?session.state, "claude session");
                let _ = tx.send(AdapterEvent::Upsert(session));
            }
        });
    }
}

fn parse_claude_session(path: &std::path::Path) -> Option<SessionStatus> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: Value = if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
        let last = text.lines().rev().find(|l| !l.trim().is_empty())?;
        serde_json::from_str(last).ok()?
    } else {
        serde_json::from_str(&text).ok()?
    };

    let id_raw = value
        .get("sessionId")
        .or_else(|| value.get("session_id"))
        .or_else(|| value.get("id"))
        .and_then(|v| v.as_str())
        .or_else(|| path.file_stem().and_then(|s| s.to_str()))?;

    let title = value
        .get("summary")
        .or_else(|| value.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let state = map_state(&value);
    let updated_at_ms = value
        .get("updated_at_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(now_ms);

    Some(SessionStatus {
        id: format!("claude:{id_raw}"),
        app: "Claude Code".into(),
        title,
        state,
        updated_at_ms,
    })
}

fn map_state(value: &Value) -> AgentState {
    let raw = value
        .get("status")
        .or_else(|| value.get("state"))
        .and_then(|v| v.as_str())
        .unwrap_or("idle")
        .to_ascii_lowercase();
    match raw.as_str() {
        "thinking" => AgentState::Thinking,
        "working" | "running" | "tool_use" => AgentState::Working,
        "awaiting_approval" | "permission" | "needs_permission" => AgentState::AwaitingApproval,
        "done" | "completed" | "idle" => {
            if raw == "idle" {
                AgentState::Idle
            } else {
                AgentState::Done
            }
        }
        "error" | "failed" => AgentState::Error,
        _ => AgentState::Working,
    }
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
    fn maps_permission() {
        let v = json!({"status": "needs_permission"});
        assert_eq!(map_state(&v), AgentState::AwaitingApproval);
    }
}
