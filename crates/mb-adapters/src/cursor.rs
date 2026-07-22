//! Cursor IDE agent-transcript watcher (Claude-parity observation).
//!
//! Watches `~/.cursor/projects/*/agent-transcripts/*.jsonl` and publishes
//! `cursor:<id>` sessions when hooks miss events. Transcripts are treated as
//! lossy — no prompt/response content is sent on the bus.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use mb_protocol::{AgentState, SessionStatus};
use serde_json::Value;
use tracing::debug;

use crate::title::project_label_from_path;
use crate::watch::{path_components_contain, watch_dir};
use crate::{AdapterEvent, AdapterTx, ObservedSession, SessionContext};

#[derive(Clone, PartialEq, Eq)]
struct Fingerprint {
    state: AgentState,
    title: String,
}

pub fn spawn_cursor_adapter(tx: AdapterTx) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let root = PathBuf::from(&home).join(".cursor").join("projects");
    let seen: Arc<Mutex<HashMap<String, Fingerprint>>> = Arc::new(Mutex::new(HashMap::new()));
    let path_ids: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let seen_cb = Arc::clone(&seen);
    let path_ids_cb = Arc::clone(&path_ids);
    watch_dir(root, move |path| {
        if !path_components_contain(&path, "agent-transcripts") {
            return;
        }
        let path_key = path.to_string_lossy().into_owned();
        if !path.exists() {
            let id = path_ids_cb.lock().unwrap().remove(&path_key);
            if let Some(id) = id {
                seen_cb.lock().unwrap().remove(&id);
                debug!(%id, "cursor transcript removed");
                let _ = tx.send(AdapterEvent::Remove(id));
            }
            return;
        }
        if let Some(observed) = parse_cursor_transcript(&path) {
            let session = &observed.session;
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
            debug!(id = %session.id, ?session.state, title = %session.title, "cursor transcript");
            let _ = tx.send(AdapterEvent::Upsert(observed));
        }
    });
}

fn parse_cursor_transcript(path: &Path) -> Option<ObservedSession> {
    let text = std::fs::read_to_string(path).ok()?;
    let file_stem = path.file_stem()?.to_str()?.to_string();
    let mut conversation_id: Option<String> = None;
    let mut state = AgentState::Idle;
    let mut cwd: Option<String> = None;
    let mut updated_at_ms = mtime_ms(path);
    let mut state_locked = false;

    for line in text.lines().rev().take(80) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if conversation_id.is_none() {
            conversation_id = value
                .get("conversation_id")
                .or_else(|| value.get("conversationId"))
                .or_else(|| value.get("session_id"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if cwd.is_none() {
            cwd = value
                .get("cwd")
                .or_else(|| value.get("workspace_root"))
                .or_else(|| {
                    value
                        .get("workspace_roots")
                        .and_then(Value::as_array)
                        .and_then(|arr| arr.first())
                })
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        if !state_locked {
            if let Some(inferred) = infer_state(&value) {
                state = inferred;
                state_locked = true;
            }
        }
        if conversation_id.is_some() && cwd.is_some() && state_locked {
            break;
        }
    }

    let id_raw = conversation_id.unwrap_or(file_stem);
    if id_raw.is_empty() {
        return None;
    }
    let title = cwd
        .as_deref()
        .map(|c| format!("Cursor · {}", project_label_from_path(Path::new(c))))
        .unwrap_or_else(|| "Cursor agent".into());
    let focus_uri = cwd
        .as_ref()
        .map(|c| format!("cursor://file{}", c.trim_end_matches('/')));

    if updated_at_ms == 0 {
        updated_at_ms = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as u64;
    }

    Some(ObservedSession {
        session: SessionStatus {
            id: format!("cursor:{id_raw}"),
            app: "Cursor".into(),
            title,
            state,
            updated_at_ms,
            focus_uri,
        },
        context: cwd.map(|cwd| SessionContext {
            runtime: "cursor".into(),
            cwd,
        }),
    })
}

fn infer_state(value: &Value) -> Option<AgentState> {
    let type_hint = value
        .get("type")
        .or_else(|| value.get("role"))
        .or_else(|| value.get("event"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if type_hint.contains("error") {
        return Some(AgentState::Error);
    }
    if type_hint.contains("tool") || type_hint.contains("assistant") {
        return Some(AgentState::Working);
    }
    if type_hint.contains("thinking") || type_hint.contains("thought") {
        return Some(AgentState::Thinking);
    }
    if type_hint.contains("user") || type_hint.contains("human") {
        return Some(AgentState::Thinking);
    }
    if type_hint.contains("done") || type_hint.contains("stop") || type_hint.contains("end") {
        return Some(AgentState::Done);
    }
    let status = value
        .get("status")
        .and_then(Value::as_str)?
        .to_ascii_lowercase();
    match status.as_str() {
        "working" | "running" => Some(AgentState::Working),
        "thinking" => Some(AgentState::Thinking),
        "done" | "completed" | "idle" => Some(AgentState::Done),
        "error" | "failed" => Some(AgentState::Error),
        "awaiting_approval" | "needs_approval" => Some(AgentState::AwaitingApproval),
        _ => None,
    }
}

fn mtime_ms(path: &Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_lossy_transcript_without_prompt_body() {
        let dir = std::env::temp_dir().join(format!(
            "mb-cursor-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("abc-123.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"conversation_id":"abc-123","type":"user","cwd":"/Users/me/dev/app"}}"#
        )
        .unwrap();
        writeln!(file, r#"{{"type":"assistant","role":"assistant"}}"#).unwrap();
        let observed = parse_cursor_transcript(&path).expect("parsed");
        assert_eq!(observed.session.id, "cursor:abc-123");
        assert_eq!(observed.session.app, "Cursor");
        assert_eq!(observed.session.state, AgentState::Working);
        assert!(observed
            .session
            .focus_uri
            .as_deref()
            .unwrap()
            .starts_with("cursor://file"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
