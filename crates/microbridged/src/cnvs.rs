//! Native CNVS integration through the authenticated loopback control API.
//!
//! CNVS owns the canvas and terminal identity, so Microbridge treats
//! `canvas id + node id` as the stable session target. The token is read from
//! CNVS's endpoint descriptor for each scan/action and is never persisted.

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mb_adapters::SessionContext;
use mb_protocol::{
    Action, AdapterCapabilities, AdapterConnectionState, AgentState, ServerMessage, SessionStatus,
};
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, warn};

use crate::state::DaemonState;

pub const CNVS_OWNER: u64 = u64::MAX - 3;
const ACTIVE_REFRESH: Duration = Duration::from_secs(2);
const IDLE_REFRESH: Duration = Duration::from_secs(10);
const OFFLINE_REFRESH: Duration = Duration::from_secs(5);
const SESSION_ID_ENCODE_SET: &AsciiSet = &CONTROLS.add(b'%').add(b':');

pub fn capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        lifecycle_observation: true,
        interrupt: true,
        focus_open: true,
        ..AdapterCapabilities::default()
    }
}

pub fn spawn(
    shared: Arc<Mutex<DaemonState>>,
    mut action_rx: mpsc::UnboundedReceiver<ServerMessage>,
) {
    tokio::spawn(async move {
        let client = match Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(3))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                warn!(%error, "CNVS HTTP client could not start");
                return;
            }
        };
        let mut refresh_after = Duration::ZERO;

        loop {
            tokio::select! {
                action = action_rx.recv() => {
                    let Some(ServerMessage::Action { session_id, action }) = action else {
                        return;
                    };
                    if let Err(error) = perform_action(&client, &session_id, action).await {
                        warn!(%error, ?action, session_id, "CNVS action failed");
                    }
                }
                _ = tokio::time::sleep(refresh_after) => {
                    let enabled = shared.lock().await.adapter_enabled("cnvs");
                    if !enabled {
                        shared.lock().await.replace_hosted_sessions(CNVS_OWNER, Vec::new());
                        refresh_after = OFFLINE_REFRESH;
                        continue;
                    }

                    match discover(&client).await {
                        Ok(discovery) => {
                            let count = discovery.sessions.len();
                            let active = discovery.active;
                            let failures = discovery.failed_canvases;
                            let mut state = shared.lock().await;
                            state.replace_hosted_sessions(CNVS_OWNER, discovery.sessions);
                            state.set_adapter_runtime(
                                "cnvs",
                                if failures == 0 {
                                    AdapterConnectionState::Connected
                                } else {
                                    AdapterConnectionState::Limited
                                },
                                capabilities(),
                                cnvs_diagnostic(count, failures),
                            );
                            refresh_after = if active { ACTIVE_REFRESH } else { IDLE_REFRESH };
                        }
                        Err(error) => {
                            debug!(%error, "CNVS control unavailable");
                            let mut state = shared.lock().await;
                            state.replace_hosted_sessions(CNVS_OWNER, Vec::new());
                            state.set_adapter_runtime(
                                "cnvs",
                                AdapterConnectionState::NeedsSetup,
                                capabilities(),
                                "CNVS is enabled and will connect automatically when CNVS is running.",
                            );
                            refresh_after = OFFLINE_REFRESH;
                        }
                    }
                }
            }
        }
    });
}

struct Discovery {
    sessions: Vec<(SessionStatus, SessionContext)>,
    active: bool,
    failed_canvases: usize,
}

async fn discover(client: &Client) -> Result<Discovery, String> {
    let endpoint = Endpoint::load()?;
    let root = get_state(client, &endpoint, None).await?;
    let mut sessions = Vec::new();
    let mut active = false;
    let mut failed_canvases = 0;

    for canvas in root.state.canvases {
        let snapshot = match get_state(client, &endpoint, Some(&canvas.id)).await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                failed_canvases += 1;
                debug!(canvas_id = canvas.id, %error, "CNVS canvas state unavailable");
                continue;
            }
        };
        for node in snapshot.state.nodes {
            let Some(hosted) = hosted_session(&canvas, node) else {
                continue;
            };
            active |= matches!(
                hosted.0.state,
                AgentState::Thinking | AgentState::Working | AgentState::AwaitingApproval
            );
            sessions.push(hosted);
        }
    }

    Ok(Discovery {
        sessions,
        active,
        failed_canvases,
    })
}

fn hosted_session(canvas: &CanvasSummary, node: Node) -> Option<(SessionStatus, SessionContext)> {
    if node.kind != "terminal" {
        return None;
    }
    let runtime = node.agent_id?.trim().to_ascii_lowercase();
    if runtime.is_empty() {
        return None;
    }
    let cwd = node
        .cwd
        .filter(|cwd| !cwd.trim().is_empty())
        .unwrap_or_else(|| canvas.project_directory.clone());
    if cwd.trim().is_empty() {
        return None;
    }
    let state = map_status(&node.status);
    let agent = display_agent(&runtime);
    let terminal = nonempty(&node.title).unwrap_or(agent);
    let title = truncate_title(&format!("{} · {terminal} · {agent}", canvas.name), 72);

    Some((
        SessionStatus {
            id: format!(
                "cnvs:{}:{}",
                encode_id_component(&canvas.id),
                encode_id_component(&node.id)
            ),
            app: "CNVS".into(),
            title,
            state,
            updated_at_ms: now_ms(),
            focus_uri: None,
        },
        SessionContext { runtime, cwd },
    ))
}

async fn perform_action(client: &Client, session_id: &str, action: Action) -> Result<(), String> {
    let (canvas_id, node_id) = parse_session_id(session_id)?;
    let action_name = match action {
        Action::OpenFocusedThread => "focus",
        Action::Interrupt => "stop_agent",
        _ => return Err(format!("CNVS does not advertise {action:?}.")),
    };
    let endpoint = Endpoint::load()?;
    let snapshot = get_state(client, &endpoint, Some(&canvas_id)).await?;
    let target = exact_node_target(&snapshot.state, &node_id)?;
    let response = client
        .post(endpoint.url("/action"))
        .header("X-CNVS-Token", &endpoint.token)
        .header("X-CNVS-Canvas-ID", &canvas_id)
        .json(&ActionRequest {
            action: action_name,
            target: &target,
        })
        .send()
        .await
        .map_err(|error| format!("could not reach CNVS control: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("CNVS rejected the action ({status})."));
    }
    let response = response
        .json::<ControlResponse>()
        .await
        .map_err(|error| format!("CNVS returned an invalid action response: {error}"))?;
    if response.ok {
        if response
            .outcomes
            .iter()
            .filter_map(|outcome| outcome.node_id.as_deref())
            .any(|returned| returned != node_id)
        {
            return Err("CNVS focused a different terminal than Microbridge requested.".into());
        }
        if action == Action::OpenFocusedThread {
            if let Err(error) = activate_cnvs() {
                warn!(%error, "CNVS focus succeeded but the app could not be activated");
            }
        }
        Ok(())
    } else {
        Err(response
            .status
            .unwrap_or_else(|| "CNVS did not accept the action.".into()))
    }
}

#[cfg(target_os = "macos")]
fn activate_cnvs() -> Result<(), String> {
    let status = std::process::Command::new("open")
        .args(["-a", "CNVS"])
        .status()
        .map_err(|error| format!("could not activate CNVS: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "macOS could not activate CNVS.".into())
}

#[cfg(not(target_os = "macos"))]
fn activate_cnvs() -> Result<(), String> {
    Err("CNVS focus is currently available on macOS only.".into())
}

async fn get_state(
    client: &Client,
    endpoint: &Endpoint,
    canvas_id: Option<&str>,
) -> Result<StateEnvelope, String> {
    let mut request = client
        .get(endpoint.url("/state"))
        .header("X-CNVS-Token", &endpoint.token);
    if let Some(canvas_id) = canvas_id {
        request = request.header("X-CNVS-Canvas-ID", canvas_id);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("could not reach CNVS control: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("CNVS state request failed ({status})."));
    }
    response
        .json::<StateEnvelope>()
        .await
        .map_err(|error| format!("CNVS returned an invalid state response: {error}"))
}

#[derive(Debug, Deserialize)]
struct Endpoint {
    host: String,
    port: u16,
    token: String,
}

impl Endpoint {
    fn load() -> Result<Self, String> {
        let path = endpoint_path();
        let bytes = std::fs::read(&path)
            .map_err(|_| format!("CNVS endpoint is not available at {}.", path.display()))?;
        let endpoint: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("CNVS endpoint descriptor is invalid: {error}"))?;
        endpoint.validate()?;
        Ok(endpoint)
    }

    fn validate(&self) -> Result<(), String> {
        let loopback = self.host == "localhost"
            || self
                .host
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback());
        if !loopback {
            return Err("CNVS control refused a non-loopback endpoint.".into());
        }
        if self.port == 0 || self.token.trim().is_empty() {
            return Err("CNVS endpoint descriptor is incomplete.".into());
        }
        Ok(())
    }

    fn url(&self, path: &str) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("http://{host}:{}{path}", self.port)
    }
}

fn endpoint_path() -> PathBuf {
    if let Ok(path) = std::env::var("CNVS_CONTROL_ENDPOINT_PATH") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("CNVS")
        .join("control-endpoint.json")
}

#[derive(Debug, Deserialize)]
struct StateEnvelope {
    #[serde(default)]
    state: CnvsState,
}

#[derive(Debug, Default, Deserialize)]
struct CnvsState {
    #[serde(default)]
    canvases: Vec<CanvasSummary>,
    #[serde(default)]
    nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CanvasSummary {
    id: String,
    name: String,
    #[serde(default)]
    project_directory: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    id: String,
    kind: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    status: String,
    #[serde(default, rename = "agentID")]
    agent_id: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Serialize)]
struct ActionRequest<'a> {
    action: &'a str,
    target: &'a str,
}

#[derive(Deserialize)]
struct ControlResponse {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    outcomes: Vec<ControlOutcome>,
}

#[derive(Deserialize)]
struct ControlOutcome {
    #[serde(default, rename = "nodeID")]
    node_id: Option<String>,
}

fn parse_session_id(session_id: &str) -> Result<(String, String), String> {
    let (canvas, node) = session_id
        .strip_prefix("cnvs:")
        .and_then(|target| target.split_once(':'))
        .filter(|(canvas, node)| !canvas.is_empty() && !node.is_empty())
        .ok_or_else(|| "CNVS session target is invalid.".to_string())?;
    if node.contains(':') {
        return Err("CNVS session target is ambiguous; encode component delimiters.".into());
    }
    Ok((decode_id_component(canvas)?, decode_id_component(node)?))
}

fn encode_id_component(value: &str) -> String {
    utf8_percent_encode(value, SESSION_ID_ENCODE_SET).to_string()
}

fn decode_id_component(value: &str) -> Result<String, String> {
    percent_decode_str(value)
        .decode_utf8()
        .map(String::from)
        .map_err(|_| "CNVS session target contains invalid UTF-8.".into())
}

fn cnvs_diagnostic(terminals: usize, failed_canvases: usize) -> String {
    let mut diagnostic = format!(
        "CNVS control is connected across {} canvas terminal{}.",
        terminals,
        if terminals == 1 { "" } else { "s" }
    );
    if failed_canvases > 0 {
        diagnostic.push_str(&format!(
            " {} canvas{} could not be refreshed.",
            failed_canvases,
            if failed_canvases == 1 { "" } else { "es" }
        ));
    }
    diagnostic
}

fn exact_node_target(state: &CnvsState, node_id: &str) -> Result<String, String> {
    let node = state
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| "The CNVS terminal no longer exists.".to_string())?;
    let title = nonempty(&node.title)
        .ok_or_else(|| "The CNVS terminal has no supported focus target.".to_string())?;
    let matches = state
        .nodes
        .iter()
        .filter(|candidate| candidate.title.trim() == title)
        .count();
    if matches != 1 {
        return Err("The CNVS terminal name is ambiguous within its canvas.".into());
    }
    Ok(title.to_string())
}

fn map_status(status: &str) -> AgentState {
    match status.trim().to_ascii_lowercase().as_str() {
        "thinking" => AgentState::Thinking,
        "working" | "running" => AgentState::Working,
        "waiting" | "awaiting_approval" | "needs_input" => AgentState::AwaitingApproval,
        "done" | "completed" => AgentState::Done,
        "error" | "failed" => AgentState::Error,
        _ => AgentState::Idle,
    }
}

fn display_agent(runtime: &str) -> &str {
    match runtime {
        "codex" => "Codex",
        "claude" => "Claude",
        "cursor" => "Cursor",
        "droid" | "factory" => "Factory",
        "opencode" => "OpenCode",
        _ => runtime,
    }
}

fn nonempty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn truncate_title(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{}…", prefix.trim_end())
    } else {
        prefix
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_stable_hosted_session_identity() {
        let canvas = CanvasSummary {
            id: "canvas-1".into(),
            name: "Payments".into(),
            project_directory: "/Users/me/dev/payments".into(),
        };
        let node = Node {
            id: "node-9".into(),
            kind: "terminal".into(),
            title: "Checkout repair".into(),
            status: "working".into(),
            agent_id: Some("codex".into()),
            cwd: Some("/Users/me/dev/payments/api".into()),
        };
        let (session, context) = hosted_session(&canvas, node).unwrap();
        assert_eq!(session.id, "cnvs:canvas-1:node-9");
        assert_eq!(session.app, "CNVS");
        assert_eq!(session.state, AgentState::Working);
        assert!(session.title.contains("Payments"));
        assert_eq!(context.runtime, "codex");
        assert_eq!(context.cwd, "/Users/me/dev/payments/api");
    }

    #[test]
    fn decodes_cnvs_agent_id_spelling() {
        let node: Node = serde_json::from_str(
            r#"{"id":"node-1","kind":"terminal","title":"Odin","status":"working","agentID":"codex","cwd":"/tmp/project"}"#,
        )
        .unwrap();
        assert_eq!(node.agent_id.as_deref(), Some("codex"));
    }

    #[test]
    fn ignores_non_agent_terminals_and_maps_waiting() {
        let canvas = CanvasSummary {
            id: "c".into(),
            name: "Canvas".into(),
            project_directory: "/tmp/project".into(),
        };
        let shell = Node {
            id: "shell".into(),
            kind: "terminal".into(),
            title: "Shell".into(),
            status: "idle".into(),
            agent_id: None,
            cwd: None,
        };
        assert!(hosted_session(&canvas, shell).is_none());
        assert_eq!(map_status("waiting"), AgentState::AwaitingApproval);
    }

    #[test]
    fn parses_exact_canvas_and_node_target() {
        assert_eq!(
            parse_session_id("cnvs:canvas-id:node-id").unwrap(),
            ("canvas-id".to_string(), "node-id".to_string())
        );
        assert!(parse_session_id("codex:thread").is_err());
    }

    #[test]
    fn session_identity_round_trips_colons_without_changing_uuid_ids() {
        assert_eq!(encode_id_component("plain-uuid"), "plain-uuid");
        let id = format!(
            "cnvs:{}:{}",
            encode_id_component("remote:canvas"),
            encode_id_component("node:one")
        );
        assert_eq!(
            parse_session_id(&id).unwrap(),
            ("remote:canvas".to_string(), "node:one".to_string())
        );
        assert!(parse_session_id("cnvs:raw:canvas:node").is_err());
    }

    #[test]
    fn resolves_node_id_to_an_unambiguous_supported_target() {
        let state = CnvsState {
            nodes: vec![
                Node {
                    id: "node-1".into(),
                    kind: "terminal".into(),
                    title: "Odin".into(),
                    status: "idle".into(),
                    agent_id: Some("codex".into()),
                    cwd: Some("/tmp/project".into()),
                },
                Node {
                    id: "node-2".into(),
                    kind: "terminal".into(),
                    title: "Thor".into(),
                    status: "idle".into(),
                    agent_id: Some("claude".into()),
                    cwd: Some("/tmp/project".into()),
                },
            ],
            ..CnvsState::default()
        };
        assert_eq!(exact_node_target(&state, "node-1").unwrap(), "Odin");
    }

    #[test]
    fn refuses_an_ambiguous_cnvs_target() {
        let node = |id: &str| Node {
            id: id.into(),
            kind: "terminal".into(),
            title: "Odin".into(),
            status: "idle".into(),
            agent_id: Some("codex".into()),
            cwd: Some("/tmp/project".into()),
        };
        let state = CnvsState {
            nodes: vec![node("node-1"), node("node-2")],
            ..CnvsState::default()
        };
        assert!(exact_node_target(&state, "node-1").is_err());
    }

    #[test]
    fn refuses_remote_control_endpoints() {
        let endpoint = Endpoint {
            host: "example.com".into(),
            port: 1234,
            token: "secret".into(),
        };
        assert!(endpoint.validate().is_err());
    }
}
