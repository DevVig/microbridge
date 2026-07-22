//! Cursor ACP / CLI agent control plane (Microbridge-owned sessions).
//!
//! The IDE composer adapter (`cursor`) is lifecycle-only. This module adds a
//! separate `cursor_acp` integration that talks to Cursor's public ACP surface
//! (`agent acp`) when the CLI is installed — enabling interrupt and a path to
//! approve/reject/new-session for agents Microbridge owns.
//!
//! This does **not** remote-control an already-open IDE composer chat.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use mb_protocol::{Action, AdapterCapabilities, AdapterConnectionState, ServerMessage};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::state::DaemonState;

pub const CURSOR_ACP_OWNER: u64 = u64::MAX - 4;
const ADAPTER_ID: &str = "cursor_acp";

pub fn capabilities() -> AdapterCapabilities {
    // Only advertise what dispatch_action implements (lean honesty).
    AdapterCapabilities {
        lifecycle_observation: true,
        approval_acceptance: true,
        approval_rejection: true,
        interrupt: true,
        new_session: true,
        ..AdapterCapabilities::default()
    }
}

pub fn spawn(
    shared: Arc<Mutex<DaemonState>>,
    mut action_rx: mpsc::UnboundedReceiver<ServerMessage>,
) {
    tokio::spawn(async move {
        let mut acp: Option<AcpSession> = None;
        // Cheap PATH probe only when the card is enabled — no busy loop.
        refresh_status(&shared, false).await;

        loop {
            tokio::select! {
                message = action_rx.recv() => {
                    let Some(message) = message else { break };
                    let ServerMessage::Action { session_id, action } = message else {
                        continue;
                    };
                    if !session_id.starts_with("cursor_acp:") {
                        continue;
                    }
                    match dispatch_action(&mut acp, &session_id, action).await {
                        Ok(Some(new_id)) => {
                            {
                                let mut state = shared.lock().await;
                                let session = mb_protocol::SessionStatus {
                                    id: format!("cursor_acp:{new_id}"),
                                    app: "Cursor Agent (ACP)".into(),
                                    title: "Cursor ACP session".into(),
                                    state: mb_protocol::AgentState::Idle,
                                    updated_at_ms: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64,
                                    focus_uri: None,
                                };
                                state.upsert_session(session, CURSOR_ACP_OWNER);
                            }
                            refresh_status(&shared, true).await;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            warn!(%error, ?action, session_id, "Cursor ACP action failed");
                        }
                    }
                }
            }
        }
    });
}

async fn refresh_status(shared: &Arc<Mutex<DaemonState>>, session_live: bool) {
    let agent = agent_binary();
    let mut state = shared.lock().await;
    if !state.adapter_enabled(ADAPTER_ID) {
        return;
    }
    match agent {
        Some(path) if session_live => {
            state.set_adapter_runtime(
                ADAPTER_ID,
                AdapterConnectionState::Connected,
                capabilities(),
                format!(
                    "Cursor ACP session live via {} — interrupt/approve/new session.",
                    path.display()
                ),
            );
        }
        Some(path) => {
            state.set_adapter_runtime(
                ADAPTER_ID,
                AdapterConnectionState::Connected,
                capabilities(),
                format!(
                    "Cursor CLI found at {}. New Session spawns ACP on demand (not the IDE composer).",
                    path.display()
                ),
            );
        }
        None => {
            state.set_adapter_runtime(
                ADAPTER_ID,
                AdapterConnectionState::NeedsSetup,
                AdapterCapabilities::default(),
                "Install the Cursor CLI (`agent` / `cursor-agent`) to enable ACP control. IDE Composer stays on the Cursor tile.",
            );
        }
    }
}

async fn dispatch_action(
    acp: &mut Option<AcpSession>,
    session_id: &str,
    action: Action,
) -> Result<Option<String>, String> {
    match action {
        Action::NewSession => {
            let mut session = AcpSession::start().await?;
            let id = session
                .request(
                    "session/new",
                    json!({
                        "cwd": std::env::current_dir()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|_| ".".into()),
                    }),
                )
                .await?;
            let acp_id = id
                .pointer("/result/sessionId")
                .or_else(|| id.pointer("/result/session_id"))
                .and_then(Value::as_str)
                .unwrap_or("default")
                .to_string();
            session.session_id = acp_id.clone();
            info!(session_id = %session.session_id, "Cursor ACP session started");
            *acp = Some(session);
            Ok(Some(acp_id))
        }
        Action::Interrupt => {
            let session = acp.as_mut().ok_or_else(|| {
                "No active Cursor ACP session — press New Session first.".to_string()
            })?;
            session
                .request("session/cancel", json!({ "sessionId": session.session_id }))
                .await?;
            Ok(None)
        }
        Action::Approve => {
            let session = acp
                .as_mut()
                .ok_or_else(|| "No active Cursor ACP session.".to_string())?;
            session
                .request(
                    "session/request_permission",
                    json!({
                        "sessionId": session.session_id,
                        "outcome": "allow-once",
                    }),
                )
                .await?;
            Ok(None)
        }
        Action::Reject => {
            let session = acp
                .as_mut()
                .ok_or_else(|| "No active Cursor ACP session.".to_string())?;
            session
                .request(
                    "session/request_permission",
                    json!({
                        "sessionId": session.session_id,
                        "outcome": "reject-once",
                    }),
                )
                .await?;
            Ok(None)
        }
        other => Err(format!(
            "Cursor ACP does not handle {other:?} for {session_id} yet."
        )),
    }
}

struct AcpSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
    session_id: String,
}

impl AcpSession {
    async fn start() -> Result<Self, String> {
        let agent = agent_binary()
            .ok_or_else(|| "Cursor CLI (`agent` / `cursor-agent`) is not on PATH.".to_string())?;
        let mut child = Command::new(&agent)
            .args(["acp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| format!("failed to spawn {}: {error}", agent.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "ACP stdin missing".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "ACP stdout missing".to_string())?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
            session_id: String::new(),
        })
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = format!("{payload}\n");
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| format!("ACP write failed: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("ACP flush failed: {error}"))?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err("ACP request timed out.".into());
            }
            let mut buf = String::new();
            let read = tokio::time::timeout(remaining, self.stdout.read_line(&mut buf))
                .await
                .map_err(|_| "ACP request timed out.".to_string())?
                .map_err(|error| format!("ACP read failed: {error}"))?;
            if read == 0 {
                return Err("ACP process closed stdout.".into());
            }
            let Ok(value) = serde_json::from_str::<Value>(buf.trim()) else {
                continue;
            };
            if value.get("id").and_then(Value::as_u64) == Some(id)
                || value.get("id").and_then(Value::as_i64) == Some(id as i64)
            {
                if let Some(error) = value.get("error") {
                    return Err(format!("ACP error: {error}"));
                }
                return Ok(value);
            }
        }
    }
}

impl Drop for AcpSession {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn agent_binary() -> Option<PathBuf> {
    for name in ["agent", "cursor-agent", "cursor"] {
        if let Ok(output) = std::process::Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    // `cursor` alone is the GUI shim; prefer agent-named binaries.
                    if name == "cursor" {
                        continue;
                    }
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_advertise_control_surface() {
        let caps = capabilities();
        assert!(caps.interrupt);
        assert!(caps.approval_acceptance);
        assert!(caps.new_session);
    }
}
