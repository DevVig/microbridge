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

use crate::hosts::host_from_cwd;
use crate::title::{clean_title, looks_like_boilerplate, project_label_from_path};
use crate::watch::{path_components_contain, watch_dir};
use crate::{AdapterEvent, AdapterTx};

#[derive(Clone, PartialEq, Eq)]
struct Fingerprint {
    state: AgentState,
    title: String,
    app: String,
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
                    app: session.app.clone(),
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
    // Host discriminators, both already present in the journal we're reading —
    // no extra I/O. `entrypoint` separates the CLI/desktop from SDK-embedded
    // hosts; `cwd` names the host when it runs under a known host home.
    let mut entrypoint: Option<String> = None;
    let mut cwd: Option<String> = None;

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

        if let Some(ep) = value.get("entrypoint").and_then(|v| v.as_str()) {
            entrypoint.get_or_insert_with(|| ep.to_string());
        }
        if let Some(c) = value.get("cwd").and_then(|v| v.as_str()) {
            cwd.get_or_insert_with(|| c.to_string());
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
        app: claude_app_label(entrypoint.as_deref(), cwd.as_deref()),
        title,
        state,
        updated_at_ms,
    })
}

/// Truthful host label for a Claude-SDK session journal.
///
/// The `~/.claude/projects` store is shared by the interactive CLI, Claude
/// Desktop, and every app that embeds the Claude Agent SDK (Synara, T3 Code,
/// …), so a hardcoded "Claude Code" mislabels the SDK hosts. `entrypoint`
/// separates the categories; for SDK-embedded sessions we name the host from
/// `cwd` when it runs under a known host home, else fall back to the honest
/// generic label. Everything is derived from fields already in the journal —
/// no process inspection, no extra I/O.
fn claude_app_label(entrypoint: Option<&str>, cwd: Option<&str>) -> String {
    match entrypoint {
        Some("cli") => "Claude Code".into(),
        Some("claude-desktop") => "Claude Desktop".into(),
        Some(ep) if ep.starts_with("sdk") => {
            host_from_cwd(cwd).unwrap_or("Claude Agent SDK").into()
        }
        // Missing/unknown entrypoint: keep the historical default.
        _ => "Claude Code".into(),
    }
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
        // No entrypoint in the journal → historical default.
        assert_eq!(session.app, "Claude Code");
    }

    #[test]
    fn labels_host_from_entrypoint_and_cwd() {
        // Interactive CLI and the desktop app are named directly.
        assert_eq!(claude_app_label(Some("cli"), None), "Claude Code");
        assert_eq!(
            claude_app_label(Some("claude-desktop"), None),
            "Claude Desktop"
        );

        // SDK-embedded host with no locating cwd → honest generic label,
        // never a false "Claude Code".
        assert_eq!(
            claude_app_label(Some("sdk-ts"), Some("/Users/me/dev/repo")),
            "Claude Agent SDK"
        );
        assert_eq!(claude_app_label(Some("sdk-cli"), None), "Claude Agent SDK");

        // SDK-embedded host running under a known host home → named host.
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            claude_app_label(
                Some("sdk-ts"),
                Some(&format!("{home}/.synara/worktrees/Foo"))
            ),
            "Synara"
        );
        assert_eq!(
            claude_app_label(Some("sdk-cli"), Some(&format!("{home}/.t3/worktrees/Bar"))),
            "T3 Code"
        );
        assert_eq!(
            claude_app_label(
                Some("sdk-ts"),
                Some(&format!("{home}/.cursor/worktrees/Baz"))
            ),
            "Cursor"
        );
        assert_eq!(
            claude_app_label(
                Some("sdk-ts"),
                Some(&format!("{home}/conductor/workspaces/Quux"))
            ),
            "Conductor"
        );

        // Missing entrypoint stays back-compatible.
        assert_eq!(claude_app_label(None, None), "Claude Code");
    }
}
