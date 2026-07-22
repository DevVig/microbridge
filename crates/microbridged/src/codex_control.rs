//! Lean Codex app-server control plane (attach-on-demand).
//!
//! Does **not** spawn a second always-on `codex app-server`. When a Micro lever
//! is pressed, briefly attaches via `codex app-server proxy` (existing control
//! socket) and issues interrupt / approval RPCs. If no socket or attach fails,
//! returns a clear error — capabilities stay lifecycle-only until a socket exists.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use mb_protocol::{Action, AdapterCapabilities, AdapterConnectionState, ServerMessage};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::state::DaemonState;

pub const CODEX_OWNER: u64 = u64::MAX - 6;
const ADAPTER_ID: &str = "codex";

pub fn lifecycle_capabilities() -> AdapterCapabilities {
    AdapterCapabilities::lifecycle_only()
}

pub fn control_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        lifecycle_observation: true,
        approval_acceptance: true,
        approval_rejection: true,
        interrupt: true,
        ..AdapterCapabilities::default()
    }
}

pub fn spawn(
    shared: Arc<Mutex<DaemonState>>,
    mut action_rx: mpsc::UnboundedReceiver<ServerMessage>,
) {
    tokio::spawn(async move {
        refresh_status(&shared).await;
        while let Some(message) = action_rx.recv().await {
            let ServerMessage::Action { session_id, action } = message else {
                continue;
            };
            if !session_id.starts_with("codex:") {
                continue;
            }
            if let Err(error) = dispatch_action(&session_id, action).await {
                warn!(%error, ?action, session_id, "Codex control action failed");
            } else {
                info!(session_id, ?action, "Codex control action delivered");
            }
            refresh_status(&shared).await;
        }
    });
}

/// Cheap existence check — no process spawn.
pub fn control_socket_path() -> Option<PathBuf> {
    let home = std::env::var("CODEX_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".codex"))
        })?;
    let candidates = [
        home.join("app-server-control")
            .join("app-server-control.sock"),
        home.join("app-server-control.sock"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

async fn refresh_status(shared: &Arc<Mutex<DaemonState>>) {
    let mut state = shared.lock().await;
    if !state.adapter_enabled(ADAPTER_ID) {
        return;
    }
    if control_socket_path().is_some() {
        state.set_internal_capabilities(CODEX_OWNER, control_capabilities());
        state.set_adapter_runtime(
            ADAPTER_ID,
            AdapterConnectionState::Connected,
            control_capabilities(),
            "Codex journals live; app-server control socket detected — interrupt/approve attach on demand.",
        );
        state.set_adapter_runtime(
            "chatgpt",
            AdapterConnectionState::Connected,
            control_capabilities(),
            "ChatGPT Codex sessions inherit Codex app-server control when the socket is present.",
        );
        state.set_adapter_runtime(
            "synara",
            AdapterConnectionState::Connected,
            control_capabilities(),
            "Synara Codex-backed sessions inherit app-server control when the socket is present.",
        );
    } else {
        state.set_internal_capabilities(CODEX_OWNER, lifecycle_capabilities());
        state.set_adapter_runtime(
            ADAPTER_ID,
            AdapterConnectionState::Connected,
            lifecycle_capabilities(),
            "Built-in lifecycle watcher is active. Start Codex with app-server for interrupt/approve.",
        );
    }
}

async fn dispatch_action(session_id: &str, action: Action) -> Result<(), String> {
    let thread_id = session_id
        .strip_prefix("codex:")
        .ok_or_else(|| "Not a Codex session.".to_string())?;
    let sock = control_socket_path().ok_or_else(|| {
        "No Codex app-server control socket. Run Codex with app-server enabled, then retry."
            .to_string()
    })?;

    match action {
        Action::Interrupt => {
            // turnId optional in some builds; send threadId and let server resolve.
            proxy_rpc(&sock, "turn/interrupt", json!({ "threadId": thread_id })).await
        }
        Action::Approve => match proxy_rpc(
            &sock,
            "item/commandExecution/requestApproval",
            json!({
                "threadId": thread_id,
                "decision": "accept",
            }),
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(_) => poll_and_respond_approval(&sock, thread_id, "accept").await,
        },
        Action::Reject => match proxy_rpc(
            &sock,
            "item/commandExecution/requestApproval",
            json!({
                "threadId": thread_id,
                "decision": "decline",
            }),
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(_) => poll_and_respond_approval(&sock, thread_id, "decline").await,
        },
        other => Err(format!("Codex control does not handle {other:?}.")),
    }
}

async fn poll_and_respond_approval(
    _sock: &Path,
    _thread_id: &str,
    _decision: &str,
) -> Result<(), String> {
    Err(
        "Approval response needs a live app-server request id. Keep the approval prompt open and ensure Codex app-server owns this thread."
            .into(),
    )
}

async fn proxy_rpc(sock: &Path, method: &str, params: Value) -> Result<(), String> {
    let mut child = Command::new("codex")
        .args(["app-server", "proxy", "--sock", &sock.to_string_lossy()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("could not attach via `codex app-server proxy`: {error}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Codex proxy stdin unavailable.".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Codex proxy stdout unavailable.".to_string())?;
    let mut reader = BufReader::new(stdout);

    let mut next_id = 1u64;
    write_rpc(
        &mut stdin,
        next_id,
        "initialize",
        json!({
            "clientInfo": { "name": "microbridge", "version": env!("CARGO_PKG_VERSION") }
        }),
    )
    .await?;
    let _ = read_rpc_response(&mut reader, next_id).await?;
    next_id += 1;

    // initialized notification (no id)
    let note = json!({ "method": "initialized" });
    stdin
        .write_all(format!("{note}\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())?;

    write_rpc(&mut stdin, next_id, method, params).await?;
    let value = read_rpc_response(&mut reader, next_id).await?;
    if let Some(error) = value.get("error") {
        return Err(format!("Codex app-server error: {error}"));
    }

    let _ = child.kill().await;
    Ok(())
}

async fn write_rpc(
    stdin: &mut tokio::process::ChildStdin,
    id: u64,
    method: &str,
    params: Value,
) -> Result<(), String> {
    let msg = json!({ "id": id, "method": method, "params": params });
    stdin
        .write_all(format!("{msg}\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())
}

async fn read_rpc_response(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    id: u64,
) -> Result<Value, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("Codex app-server proxy timed out.".into());
        }
        let mut line = String::new();
        let n = tokio::time::timeout(remaining, reader.read_line(&mut line))
            .await
            .map_err(|_| "Codex app-server proxy timed out.".to_string())?
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Codex app-server proxy closed.".into());
        }
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if value.get("id").and_then(Value::as_u64) == Some(id)
            || value.get("id").and_then(Value::as_i64) == Some(id as i64)
        {
            return Ok(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_caps_include_interrupt_and_approvals() {
        let caps = control_capabilities();
        assert!(caps.interrupt);
        assert!(caps.approval_acceptance);
        assert!(!caps.new_session);
    }
}
