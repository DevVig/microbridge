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

use crate::hosts::host_from_cwd;
use crate::title::{clean_title, cwd_basename, looks_like_boilerplate};
use crate::watch::{path_components_contain, watch_dir};
use crate::{AdapterEvent, AdapterTx, ObservedSession, SessionContext};

#[derive(Clone, PartialEq, Eq)]
struct Fingerprint {
    state: AgentState,
    app: String,
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
        if let Some(observed) = parse_codex_session(&path) {
            let session = &observed.session;
            path_ids_cb
                .lock()
                .unwrap()
                .insert(path_key, session.id.clone());
            let fp = Fingerprint {
                state: session.state,
                app: session.app.clone(),
                title: session.title.clone(),
            };
            let mut map = seen_cb.lock().unwrap();
            if map.get(&session.id) == Some(&fp) {
                return;
            }
            map.insert(session.id.clone(), fp);
            drop(map);
            debug!(id = %session.id, ?session.state, title = %session.title, "codex session");
            let _ = tx.send(AdapterEvent::Upsert(observed));
        }
    });
}

fn parse_codex_session(path: &std::path::Path) -> Option<ObservedSession> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut id: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut originator: Option<String> = None;
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
                if is_subagent_source(payload.get("source")) {
                    return None;
                }
                if let Some(raw) = payload.get("id").and_then(|v| v.as_str()) {
                    id = Some(raw.to_string());
                }
                if let Some(c) = payload.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(c.to_string());
                }
                if let Some(value) = payload.get("originator").and_then(|v| v.as_str()) {
                    originator = Some(value.to_string());
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

    Some(ObservedSession {
        session: SessionStatus {
            id: format!("codex:{id_raw}"),
            app: codex_app_label(originator.as_deref(), cwd.as_deref()),
            title,
            state,
            updated_at_ms,
        },
        context: cwd.map(|cwd| SessionContext {
            runtime: "codex".into(),
            cwd,
        }),
    })
}

/// Name the application hosting a Codex session rather than assuming every
/// Codex rollout was launched from the standalone CLI.
///
/// `~/.codex/sessions` is a shared store: the interactive CLI, `codex exec`,
/// the Codex desktop app, and every app that wraps `codex app-server` (T3 Code,
/// Synara, …) all journal here. Recent Codex builds record an `originator`,
/// which is authoritative; `cwd` covers older journals from before that field
/// existed, since these hosts keep their worktrees under their own home.
fn codex_app_label(originator: Option<&str>, cwd: Option<&str>) -> String {
    // (lowercased originator prefix, display name). Prefix-matched because
    // hosts suffix the field — `t3code_desktop`, `codex_work_desktop`.
    //
    // Deliberately no catch-all `codex` entry: the CLI's own originator is
    // `codex_cli_rs`, so a greedy prefix would relabel real CLI sessions as the
    // desktop app. An unrecognized originator falls through to `cwd` and then
    // to the historical default, which is wrong for at most the name of a host
    // we have never seen — never wrong about one we have.
    const ORIGINATORS: &[(&str, &str)] = &[
        ("t3code", "T3 Code"),
        ("synara", "Synara"),
        ("conductor", "Conductor"),
        ("factory", "Factory"),
        ("codex desktop", "ChatGPT"),
        ("codex_work_desktop", "ChatGPT"),
    ];

    if let Some(originator) = originator {
        let originator = originator.to_ascii_lowercase();
        if let Some((_, name)) = ORIGINATORS
            .iter()
            .find(|(prefix, _)| originator.starts_with(prefix))
        {
            return (*name).into();
        }
    }
    host_from_cwd(cwd).unwrap_or("Codex CLI").into()
}

/// Detects rollouts spawned as subagents of another session.
///
/// Codex writes these into the same dated directory as ordinary sessions and
/// marks them only inside `session_meta.source`, so the `subagents` path filter
/// in the watcher never matches one. They are steps *within* a session the user
/// can already see, and there are routinely more of them than there are real
/// sessions — left in, they bury the list the popover exists to show.
fn is_subagent_source(source: Option<&Value>) -> bool {
    match source {
        Some(Value::Object(map)) => map.contains_key("subagent"),
        Some(Value::String(name)) => name == "subagent",
        _ => false,
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
        let session = parse_codex_session(&path).unwrap().session;
        assert_eq!(session.id, "codex:abc-123");
        assert_eq!(session.title, "Build the AIhero menu bar pet");
        assert_eq!(session.state, AgentState::Working);
        assert_eq!(session.app, "Codex CLI");
    }

    #[test]
    fn labels_t3_hosted_codex_session_from_originator() {
        let dir = tempfile_dir();
        let path = dir.join("t3-originator.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"t3-1","cwd":"/Users/me/dev/repo","originator":"t3code_desktop"}}}}"#
        )
        .unwrap();
        let session = parse_codex_session(&path).unwrap().session;
        assert_eq!(session.id, "codex:t3-1");
        assert_eq!(session.app, "T3 Code");
    }

    #[test]
    fn labels_sdk_hosts_and_the_codex_app_from_originator() {
        // The CLI's own originator must survive the desktop-app entries.
        assert_eq!(codex_app_label(Some("codex_cli_rs"), None), "Codex CLI");
        assert_eq!(codex_app_label(Some("codex-tui"), None), "Codex CLI");
        assert_eq!(codex_app_label(Some("codex_exec"), None), "Codex CLI");

        assert_eq!(codex_app_label(Some("synara_desktop"), None), "Synara");
        assert_eq!(codex_app_label(Some("t3code_desktop"), None), "T3 Code");
        assert_eq!(codex_app_label(Some("factory_desktop"), None), "Factory");
        // The desktop app writes a display-cased originator.
        assert_eq!(codex_app_label(Some("Codex Desktop"), None), "ChatGPT");
        assert_eq!(codex_app_label(Some("codex_work_desktop"), None), "ChatGPT");

        // Unknown host, no locating cwd → historical default rather than a
        // guess.
        assert_eq!(
            codex_app_label(Some("some_future_ide"), Some("/Users/me/dev/repo")),
            "Codex CLI"
        );
    }

    #[test]
    fn labels_pre_originator_journal_from_host_worktree() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            codex_app_label(None, Some(&format!("{home}/.synara/worktrees/app/branch"))),
            "Synara"
        );
        assert_eq!(
            codex_app_label(
                Some("codex_sdk_ts"),
                Some(&format!("{home}/conductor/workspaces/app/branch"))
            ),
            "Conductor"
        );
    }

    #[test]
    fn skips_subagent_rollouts() {
        let dir = tempfile_dir();
        let path = dir.join("subagent.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        // Shape as written by Codex: a normal rollout in a normal dated
        // directory, distinguishable only by `source`.
        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"sub-1","cwd":"/Users/me/dev/repo","originator":"Codex Desktop","source":{{"subagent":{{"thread_spawn":{{"parent_thread_id":"parent-1","depth":1}}}}}}}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"Review the billing bridge"}}}}"#
        )
        .unwrap();
        assert!(parse_codex_session(&path).is_none());
    }

    #[test]
    fn keeps_ordinary_rollouts_with_a_plain_source() {
        let dir = tempfile_dir();
        let path = dir.join("vscode-source.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"top-1","cwd":"/Users/me/dev/repo","originator":"synara_desktop","source":"vscode"}}}}"#
        )
        .unwrap();
        let session = parse_codex_session(&path).unwrap().session;
        assert_eq!(session.id, "codex:top-1");
        assert_eq!(session.app, "Synara");
    }

    #[test]
    fn labels_older_t3_hosted_session_from_worktree() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            codex_app_label(None, Some(&format!("{home}/.t3/worktrees/project/branch"))),
            "T3 Code"
        );
        assert_eq!(
            codex_app_label(Some("codex_cli_rs"), Some("/Users/me/dev/project")),
            "Codex CLI"
        );
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mb-codex-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}
