//! Factory Droid lifecycle control through Factory's public hook and JSON-RPC contracts.
//!
//! Hooks feed lifecycle events through `microbridgectl factory-event`. Device
//! actions are intentionally on-demand: no Factory process or network poll is
//! kept alive while Microbridge is idle.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use mb_protocol::{Action, AdapterCapabilities, ServerMessage};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tracing::warn;

use crate::state::DaemonState;

pub const FACTORY_OWNER: u64 = u64::MAX - 2;
const FACTORY_PROTOCOL_VERSION: &str = "1.51.0";

pub fn capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        lifecycle_observation: true,
        interrupt: true,
        reasoning_effort: true,
        ..AdapterCapabilities::default()
    }
}

pub fn spawn(
    _shared: Arc<Mutex<DaemonState>>,
    mut action_rx: mpsc::UnboundedReceiver<ServerMessage>,
) {
    tokio::spawn(async move {
        while let Some(message) = action_rx.recv().await {
            let ServerMessage::Action { session_id, action } = message else {
                continue;
            };
            let Some(factory_id) = session_id.strip_prefix("factory:") else {
                continue;
            };
            if let Err(error) = dispatch_action(factory_id, action).await {
                warn!(%error, ?action, session_id, "Factory action failed");
            }
        }
    });
}

async fn dispatch_action(session_id: &str, action: Action) -> Result<(), String> {
    let method_and_params = match action {
        Action::Interrupt => ("droid.interrupt_session", json!({})),
        Action::ReasoningEffortUp | Action::ReasoningEffortDown => {
            let settings = read_session_settings(session_id)?;
            let model = settings
                .get("model")
                .and_then(Value::as_str)
                .ok_or_else(|| "Factory session settings do not name a model.".to_string())?;
            let help = factory_exec_help().await?;
            let (levels, default) = parse_reasoning_levels(&help, model).ok_or_else(|| {
                format!("Factory does not advertise adjustable reasoning levels for {model}.")
            })?;
            let current = settings
                .get("reasoningEffort")
                .and_then(Value::as_str)
                .unwrap_or(default.as_str());
            let next = adjacent_level(&levels, current, action == Action::ReasoningEffortUp)
                .ok_or_else(|| format!("Factory reasoning effort is already at {current}."))?;
            (
                "droid.update_session_settings",
                json!({ "reasoningEffort": next }),
            )
        }
        _ => return Err(format!("Factory does not advertise {action:?}.")),
    };

    run_jsonrpc(session_id, method_and_params.0, method_and_params.1).await
}

async fn factory_exec_help() -> Result<String, String> {
    let droid = droid_binary();
    let output = tokio::time::timeout(
        Duration::from_secs(8),
        Command::new(&droid).args(["exec", "--help"]).output(),
    )
    .await
    .map_err(|_| "Factory model discovery timed out.".to_string())?
    .map_err(|error| format!("could not run droid: {error}"))?;
    if !output.status.success() {
        return Err("droid exec --help could not discover Factory models.".into());
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

async fn run_jsonrpc(session_id: &str, method: &str, params: Value) -> Result<(), String> {
    let droid = droid_binary();
    let mut child = Command::new(&droid)
        .args([
            "exec",
            "--input-format",
            "stream-jsonrpc",
            "--output-format",
            "stream-jsonrpc",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("could not start droid JSON-RPC: {error}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "droid stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "droid stdout unavailable".to_string())?;
    let mut lines = BufReader::new(stdout).lines();
    let load = rpc_request(
        "microbridge-load",
        "droid.load_session",
        json!({ "sessionId": session_id }),
    );
    stdin
        .write_all(format!("{load}\n").as_bytes())
        .await
        .map_err(|error| format!("write droid JSON-RPC: {error}"))?;
    read_rpc_response(&mut lines, "microbridge-load").await?;

    let action = rpc_request("microbridge-action", method, params);
    stdin
        .write_all(format!("{action}\n").as_bytes())
        .await
        .map_err(|error| format!("write droid JSON-RPC: {error}"))?;
    let result = read_rpc_response(&mut lines, "microbridge-action").await;
    drop(stdin);
    let _ = child.kill().await;
    result
}

fn droid_binary() -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(configured) = std::env::var_os("FACTORY_DROID_PATH") {
        candidates.push(PathBuf::from(configured));
    }
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/bin/droid"));
    }
    candidates.extend([
        PathBuf::from("/Applications/Factory.app/Contents/Resources/bin/droid"),
        PathBuf::from("/opt/homebrew/bin/droid"),
        PathBuf::from("/usr/local/bin/droid"),
    ]);
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("droid"))
}

async fn read_rpc_response<R>(
    lines: &mut tokio::io::Lines<BufReader<R>>,
    wanted_id: &str,
) -> Result<(), String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| format!("read droid JSON-RPC: {error}"))?
        {
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if value.get("id").and_then(Value::as_str) != Some(wanted_id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(format!("Factory rejected the request: {error}"));
            }
            if value.get("result").is_some() {
                return Ok(());
            }
        }
        Err("Factory closed without acknowledging the request.".into())
    })
    .await
    .map_err(|_| "Factory action timed out.".to_string())?
}

fn rpc_request(id: &str, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "factoryApiVersion": "1.0.0",
        "factoryProtocolVersion": FACTORY_PROTOCOL_VERSION,
        "type": "request",
        "id": id,
        "method": method,
        "params": params,
    })
}

fn read_session_settings(session_id: &str) -> Result<Value, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_string())?;
    let root = PathBuf::from(home).join(".factory/sessions");
    let direct = root.join(format!("{session_id}.settings.json"));
    let path = if direct.is_file() {
        direct
    } else {
        find_settings(&root, session_id, 3)
            .ok_or_else(|| format!("Factory settings for session {session_id} were not found."))?
    };
    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn find_settings(root: &Path, session_id: &str, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let wanted = format!("{session_id}.settings.json");
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some(wanted.as_str()) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_settings(&path, session_id, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

fn parse_reasoning_levels(help: &str, model_id: &str) -> Option<(Vec<String>, String)> {
    let mut in_models = false;
    let mut display = None;
    for line in help.lines() {
        let trimmed = line.trim();
        if trimmed == "Available Models:" {
            in_models = true;
            continue;
        }
        if trimmed == "Custom Models:" || trimmed == "Model details:" {
            in_models = false;
        }
        if in_models {
            let mut fields = trimmed.split_whitespace();
            if fields.next() == Some(model_id) {
                display = Some(
                    fields
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim_end_matches(" (default)")
                        .trim_end_matches(" [Deprecated]")
                        .to_string(),
                );
                break;
            }
        }
    }
    let display = display?;
    let prefix = format!("- {display}: supports reasoning: Yes; supported: [");
    let line = help
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(&prefix))?;
    let levels_raw = line.strip_prefix(&prefix)?.split(']').next()?;
    let levels: Vec<String> = levels_raw
        .split(',')
        .map(|item| item.trim().to_string())
        .collect();
    let default = line
        .split("default: ")
        .nth(1)
        .map(str::trim)
        .unwrap_or_else(|| levels.first().map(String::as_str).unwrap_or("none"))
        .to_string();
    Some((levels, default))
}

fn adjacent_level(levels: &[String], current: &str, up: bool) -> Option<String> {
    let index = levels.iter().position(|level| level == current)?;
    let next = if up {
        index.checked_add(1)?
    } else {
        index.checked_sub(1)?
    };
    levels.get(next).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_factory_model_specific_reasoning_levels() {
        let help = r#"
Available Models:
  gpt-5.4  GPT-5.4
Model details:
  - GPT-5.4: supports reasoning: Yes; supported: [low, medium, high, xhigh]; default: medium
"#;
        let (levels, default) = parse_reasoning_levels(help, "gpt-5.4").unwrap();
        assert_eq!(levels, ["low", "medium", "high", "xhigh"]);
        assert_eq!(default, "medium");
        assert_eq!(
            adjacent_level(&levels, "medium", true).as_deref(),
            Some("high")
        );
        assert_eq!(adjacent_level(&levels, "low", false), None);
    }

    #[test]
    fn emits_factory_extended_jsonrpc_envelopes() {
        let value = rpc_request("one", "droid.interrupt_session", json!({}));
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["factoryApiVersion"], "1.0.0");
        assert_eq!(value["factoryProtocolVersion"], FACTORY_PROTOCOL_VERSION);
        assert_eq!(value["type"], "request");
    }

    #[test]
    fn discovers_a_factory_droid_binary_outside_gui_path() {
        let binary = droid_binary();
        assert!(binary.is_file() || binary == PathBuf::from("droid"));
    }
}
