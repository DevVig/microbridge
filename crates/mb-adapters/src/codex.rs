//! Codex CLI in-process adapter.
//!
//! Watches `~/.codex/sessions` for JSONL rollout journals. Titles come from
//! `user_message` events (not the last line of the file); state is inferred
//! from recent `task_*` / approval events.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use mb_protocol::{AgentState, SessionStatus};
use serde_json::Value;
use tracing::debug;

use crate::title::{clean_title, cwd_basename, looks_like_boilerplate};
use crate::watch::{path_components_contain, watch_dir};
use crate::{AdapterEvent, AdapterTx};

#[derive(Clone, PartialEq, Eq)]
struct Fingerprint {
    state: AgentState,
    title: String,
}

pub fn spawn_codex_adapter(tx: AdapterTx) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let root = PathBuf::from(home).join(".codex").join("sessions");
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
                debug!(%id, "codex session removed");
                let _ = tx.send(AdapterEvent::Remove(id));
            }
            return;
        }
        if let Some(session) = parse_codex_session(&path) {
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
            debug!(id = %session.id, ?session.state, title = %session.title, "codex session");
            let _ = tx.send(AdapterEvent::Upsert(session));
        }
    });
}

fn parse_codex_session(path: &std::path::Path) -> Option<SessionStatus> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut id: Option<String> = None;
    let mut cwd: Option<String> = None;
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
        match value.get("type").and_then(|v| v.as_str()) {
            Some("session_meta") => {
                let payload = value.get("payload").unwrap_or(&value);
                if let Some(raw) = payload.get("id").and_then(|v| v.as_str()) {
                    id = Some(raw.to_string());
                }
                if let Some(c) = payload.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(c.to_string());
                }
            }
            Some("event_msg") => {
                let payload = value.get("payload").unwrap_or(&Value::Null);
                match payload.get("type").and_then(|v| v.as_str()) {
                    Some("user_message") => {
                        if let Some(msg) = payload.get("message").and_then(|v| v.as_str()) {
                            if !looks_like_boilerplate(msg) && title.is_none() {
                                title = Some(clean_title(msg, 72));
                            }
                        }
                    }
                    Some("task_started") => state = AgentState::Working,
                    Some("task_complete") => state = AgentState::Done,
                    Some("turn_aborted") => state = AgentState::Error,
                    Some(other) if other.contains("approval") || other.contains("permission") => {
                        state = AgentState::AwaitingApproval;
                    }
                    Some("agent_reasoning") => {
                        if matches!(state, AgentState::Idle | AgentState::Done) {
                            state = AgentState::Thinking;
                        }
                    }
                    _ => {}
                }
            }
            Some("response_item") => {
                let payload = value.get("payload").unwrap_or(&Value::Null);
                if payload.get("type").and_then(|v| v.as_str()) == Some("message")
                    && payload.get("role").and_then(|v| v.as_str()) == Some("user")
                {
                    if let Some(content) = payload.get("content").and_then(|v| v.as_array()) {
                        for part in content {
                            if part.get("type").and_then(|v| v.as_str()) == Some("input_text") {
                                if let Some(msg) = part.get("text").and_then(|v| v.as_str()) {
                                    if !looks_like_boilerplate(msg) && title.is_none() {
                                        title = Some(clean_title(msg, 72));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let id_raw = id.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    })?;

    let title = title
        .filter(|t| !t.is_empty())
        .or_else(|| cwd.as_deref().map(cwd_basename))
        .unwrap_or_else(|| "Codex session".into());

    let updated_at_ms = file_mtime_ms(path).unwrap_or_else(now_ms);

    Some(SessionStatus {
        id: format!("codex:{id_raw}"),
        app: "Codex CLI".into(),
        title,
        state,
        updated_at_ms,
    })
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
    fn parses_user_message_and_task_state() {
        let dir = tempfile_dir();
        let path = dir.join("rollout.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"abc-123","cwd":"/Users/me/dev/AIhero"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"Build the AIhero menu bar pet"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"task_started","turn_id":"t1"}}}}"#
        )
        .unwrap();
        let session = parse_codex_session(&path).unwrap();
        assert_eq!(session.id, "codex:abc-123");
        assert_eq!(session.title, "Build the AIhero menu bar pet");
        assert_eq!(session.state, AgentState::Working);
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mb-codex-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}
