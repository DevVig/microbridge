//! Claude Code in-process adapter.
//!
//! Watches `~/.claude/projects` (and the XDG fallback). Skips subagent
//! journals. Titles come from the first real user message; project folder
//! name is the fallback.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use mb_protocol::{AgentState, SessionStatus};
use serde_json::Value;
use tracing::debug;

use crate::title::{clean_title, looks_like_boilerplate, project_label_from_path};
use crate::watch::{path_components_contain, watch_dir};
use crate::{AdapterEvent, AdapterTx};

#[derive(Clone, PartialEq, Eq)]
struct Fingerprint {
    state: AgentState,
    title: String,
}

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
        let seen: Arc<Mutex<HashMap<String, Fingerprint>>> = Arc::new(Mutex::new(HashMap::new()));
        let path_ids: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let seen_cb = Arc::clone(&seen);
        let path_ids_cb = Arc::clone(&path_ids);
        watch_dir(root, move |path| {
            if path_components_contain(&path, "subagents") {
                return;
            }
            let path_key = path.to_string_lossy().into_owned();
            if !path.exists() {
                let id = path_ids_cb.lock().unwrap().remove(&path_key);
                if let Some(id) = id {
                    seen_cb.lock().unwrap().remove(&id);
                    debug!(%id, "claude session removed");
                    let _ = tx.send(AdapterEvent::Remove(id));
                }
                return;
            }
            if let Some(session) = parse_claude_session(&path) {
                path_ids_cb
                    .lock()
                    .unwrap()
                    .insert(path_key, session.id.clone());
                let fp = Fingerprint {
                    state: session.state,
                    title: session.title.clone(),
                };
                let mut map = seen_cb.lock().unwrap();
                if map.get(&session.id) == Some(&fp) {
                    return;
                }
                map.insert(session.id.clone(), fp);
                drop(map);
                debug!(id = %session.id, ?session.state, title = %session.title, "claude session");
                let _ = tx.send(AdapterEvent::Upsert(session));
            }
        });
    }
}

fn parse_claude_session(path: &std::path::Path) -> Option<SessionStatus> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut state = AgentState::Idle;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        if let Some(sid) = value
            .get("sessionId")
            .or_else(|| value.get("session_id"))
            .and_then(|v| v.as_str())
        {
            id.get_or_insert_with(|| sid.to_string());
        }

        let msg = value.get("message").cloned().unwrap_or(Value::Null);
        let role = msg
            .get("role")
            .or_else(|| value.get("type"))
            .and_then(|v| v.as_str());

        if matches!(role, Some("user"))
            || value.get("type").and_then(|v| v.as_str()) == Some("user")
        {
            if let Some(text) = extract_text(&msg).or_else(|| extract_text(&value)) {
                if !looks_like_boilerplate(&text) && title.is_none() {
                    title = Some(clean_title(&text, 72));
                }
            }
        }

        if let Some(status) = value
            .get("status")
            .or_else(|| value.get("state"))
            .and_then(|v| v.as_str())
        {
            state = map_status(status);
        } else if value.get("type").and_then(|v| v.as_str()) == Some("assistant")
            && matches!(state, AgentState::Idle | AgentState::Done)
        {
            state = AgentState::Working;
        }
    }

    let id_raw = id.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    })?;

    let title = title
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| project_label_from_path(path));

    let updated_at_ms = file_mtime_ms(path).unwrap_or_else(now_ms);

    Some(SessionStatus {
        id: format!("claude:{id_raw}"),
        app: "Claude Code".into(),
        title,
        state,
        updated_at_ms,
    })
}

fn extract_text(value: &Value) -> Option<String> {
    if let Some(s) = value.get("content").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    let parts = value.get("content").and_then(|v| v.as_array())?;
    let mut out = String::new();
    for part in parts {
        if part.get("type").and_then(|v| v.as_str()) == Some("text") {
            if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(t);
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn map_status(raw: &str) -> AgentState {
    match raw.to_ascii_lowercase().as_str() {
        "thinking" => AgentState::Thinking,
        "working" | "running" | "tool_use" => AgentState::Working,
        "awaiting_approval" | "permission" | "needs_permission" => AgentState::AwaitingApproval,
        "done" | "completed" => AgentState::Done,
        "idle" => AgentState::Idle,
        "error" | "failed" => AgentState::Error,
        _ => AgentState::Working,
    }
}

fn file_mtime_ms(path: &std::path::Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let dur = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    Some(dur.as_millis() as u64)
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
    use std::io::Write;

    #[test]
    fn skips_boilerplate_and_uses_user_text() {
        let dir = std::env::temp_dir().join(format!("mb-claude-{}", std::process::id()));
        let project = dir.join("-Users-me-dev-microbridge");
        let _ = std::fs::create_dir_all(&project);
        let path = project.join("sess.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","sessionId":"s1","message":{{"role":"user","content":"<system_instruction>\nhi"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","sessionId":"s1","message":{{"role":"user","content":"Wire the menu bar tray icon"}}}}"#
        )
        .unwrap();
        let session = parse_claude_session(&path).unwrap();
        assert_eq!(session.id, "claude:s1");
        assert_eq!(session.title, "Wire the menu bar tray icon");
    }
}
