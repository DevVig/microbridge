//! Claude Code PermissionRequest bridge (push hooks, zero idle cost).
//!
//! Hooks write a pending approval under `~/.microbridge/claude-pending/` and
//! wait for a decision file. Micro Approve/Reject writes that file. No daemon
//! poll loop — cost is only while Claude is blocked on a permission prompt.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use mb_protocol::{Action, AdapterCapabilities, AdapterConnectionState, ServerMessage};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::state::DaemonState;

pub const CLAUDE_OWNER: u64 = u64::MAX - 7;
const ADAPTER_ID: &str = "claude";

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

pub fn pending_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".microbridge")
        .join("claude-pending")
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
            if !session_id.starts_with("claude:") {
                continue;
            }
            if let Err(error) = dispatch_action(&session_id, action) {
                warn!(%error, ?action, session_id, "Claude control action failed");
            } else {
                info!(session_id, ?action, "Claude control action delivered");
            }
        }
    });
}

async fn refresh_status(shared: &Arc<Mutex<DaemonState>>) {
    let mut state = shared.lock().await;
    if !state.adapter_enabled(ADAPTER_ID) {
        return;
    }
    let hooks = claude_hooks_installed();
    if hooks {
        state.set_internal_capabilities(CLAUDE_OWNER, control_capabilities());
        state.set_adapter_runtime(
            ADAPTER_ID,
            AdapterConnectionState::Connected,
            control_capabilities(),
            "Claude journals live; PermissionRequest hooks bridge Approve/Reject/Interrupt.",
        );
        state.set_adapter_runtime(
            "claude_desktop",
            AdapterConnectionState::Connected,
            control_capabilities(),
            "Claude Desktop sessions inherit the PermissionRequest hook bridge.",
        );
    } else {
        state.set_internal_capabilities(CLAUDE_OWNER, lifecycle_capabilities());
        state.set_adapter_runtime(
            ADAPTER_ID,
            AdapterConnectionState::Connected,
            lifecycle_capabilities(),
            "Built-in lifecycle watcher is active. Enable Claude hooks in Settings for Approve/Reject.",
        );
    }
}

fn claude_hooks_installed() -> bool {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let settings = PathBuf::from(home).join(".claude").join("settings.json");
    let Ok(text) = fs::read_to_string(settings) else {
        return false;
    };
    text.contains("microbridge") && text.contains("PermissionRequest")
}

fn dispatch_action(session_id: &str, action: Action) -> Result<(), String> {
    let claude_id = session_id
        .strip_prefix("claude:")
        .ok_or_else(|| "Not a Claude session.".to_string())?;
    let dir = pending_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    match action {
        Action::Approve => write_decision(claude_id, "allow"),
        Action::Reject => write_decision(claude_id, "deny"),
        Action::Interrupt => {
            // Prefer denying a pending permission with interrupt; also drop a
            // sticky interrupt flag for PreToolUse hooks.
            let _ = write_decision(claude_id, "deny_interrupt");
            let flag = dir.join("interrupt").join(format!("{claude_id}.flag"));
            if let Some(parent) = flag.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&flag, b"1").map_err(|e| e.to_string())?;
            Ok(())
        }
        other => Err(format!("Claude control does not handle {other:?}.")),
    }
}

fn write_decision(claude_id: &str, decision: &str) -> Result<(), String> {
    let dir = pending_dir();
    // Hook may use conversation id or "latest" while waiting.
    for name in [claude_id.to_string(), "latest".into()] {
        let path = dir.join(format!("{name}.decision"));
        fs::write(&path, decision).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Used by tests / installers — wait briefly for a decision (hook side).
pub fn wait_for_decision(claude_id: &str, timeout: Duration) -> Option<String> {
    let dir = pending_dir();
    let path = dir.join(format!("{claude_id}.decision"));
    let latest = dir.join("latest.decision");
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        for candidate in [&path, &latest] {
            if let Ok(text) = fs::read_to_string(candidate) {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = fs::remove_file(candidate);
                    return Some(trimmed);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_caps_are_approvals_and_interrupt() {
        let caps = control_capabilities();
        assert!(caps.approval_acceptance);
        assert!(caps.interrupt);
        assert!(!caps.new_session);
    }
}
