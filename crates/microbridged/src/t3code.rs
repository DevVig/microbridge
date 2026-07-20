//! Supported T3 Code pairing + HTTP orchestration adapter.
//!
//! T3's HTTP snapshot and dispatch endpoints share the same typed contracts as
//! its WebSocket client and avoid depending on private desktop state.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mb_protocol::{
    Action, AdapterCapabilities, AdapterConnectionState, AgentState, ServerMessage, SessionStatus,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, warn};
use url::Url;

use crate::state::DaemonState;

pub const T3_OWNER: u64 = u64::MAX - 1;
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "ai.microbridge.t3code";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "paired-environment";
const SUPPORTED_SERVER_VERSION: &str = "0.0.28";
const PINNED_CONTRACT_COMMIT: &str = "ebe8afb1df357423a0e036b388af3e739d640205";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct T3Credential {
    pub http_base_url: String,
    pub bearer_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentDescriptor {
    server_version: String,
    environment_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellSnapshot {
    threads: Vec<ThreadShell>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadShell {
    id: String,
    title: String,
    #[serde(rename = "modelSelection")]
    _model_selection: Value,
    latest_turn: Option<LatestTurn>,
    session: Option<T3Session>,
    has_pending_approvals: bool,
    updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestTurn {
    turn_id: String,
    state: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct T3Session {
    status: String,
}

#[derive(Debug, Clone)]
struct RuntimeThread {
    shell: ThreadShell,
}

pub fn capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        lifecycle_observation: true,
        approval_acceptance: true,
        approval_rejection: true,
        interrupt: true,
        new_session: false,
        // T3 publishes a stable, semantic deep link for a specific environment
        // and thread. This deliberately avoids synthesizing UI keybindings.
        focus_open: true,
        // Enabled dynamically only when provider option descriptors become
        // available over the paired HTTP contract.
        reasoning_effort: false,
    }
}

pub async fn pair(pairing_url: &str) -> Result<T3Credential, String> {
    if !cfg!(target_os = "macos") {
        return Err("T3 credential storage currently requires macOS Keychain.".into());
    }
    let (base_url, one_time_token) = parse_pairing_url(pairing_url)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| error.to_string())?;
    let token_url = base_url
        .join("oauth/token")
        .map_err(|error| format!("invalid T3 token endpoint: {error}"))?;
    let response = client
        .post(token_url)
        .form(&[
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("subject_token", one_time_token.as_str()),
            (
                "subject_token_type",
                "urn:t3:params:oauth:token-type:environment-bootstrap",
            ),
            (
                "requested_token_type",
                "urn:ietf:params:oauth:token-type:access_token",
            ),
            (
                "scope",
                "orchestration:read orchestration:operate review:write",
            ),
            ("client_label", "Microbridge"),
            ("client_device_type", "desktop"),
            ("client_os", "macos"),
        ])
        .send()
        .await
        .map_err(|error| format!("could not reach T3 Code: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "T3 Code rejected the one-time pairing link ({})",
            response.status()
        ));
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|error| format!("invalid T3 Code pairing response: {error}"))?;
    let credential = T3Credential {
        http_base_url: base_url.to_string(),
        bearer_token: token.access_token,
    };
    store_credential(&credential)?;
    Ok(credential)
}

fn parse_pairing_url(input: &str) -> Result<(Url, String), String> {
    let mut url = Url::parse(input.trim())
        .map_err(|_| "Enter a complete T3 Code pairing URL.".to_string())?;
    validate_pairing_base(&url)?;
    let fragment = url.fragment().unwrap_or_default();
    let fragment_token = url::form_urlencoded::parse(fragment.as_bytes())
        .find_map(|(key, value)| (key == "token").then(|| value.into_owned()));
    let query_token = url
        .query_pairs()
        .find_map(|(key, value)| (key == "token").then(|| value.into_owned()));
    let hosted_host = url
        .query_pairs()
        .find_map(|(key, value)| (key == "host").then(|| value.into_owned()));
    let token = fragment_token
        .or(query_token)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "The pairing URL does not contain its one-time token.".to_string())?;
    if let Some(host) = hosted_host {
        url = Url::parse(&host)
            .map_err(|_| "The hosted pairing URL contains an invalid backend.".to_string())?;
        validate_pairing_base(&url)?;
    } else {
        url.set_query(None);
        url.set_fragment(None);
        if url.path().trim_end_matches('/').ends_with("/pair") {
            url.set_path("/");
        }
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok((url, token))
}

fn validate_pairing_base(url: &Url) -> Result<(), String> {
    let loopback = match url.host_str() {
        Some(host) if host.eq_ignore_ascii_case("localhost") => true,
        Some(host) => host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false),
        None => false,
    };
    if !matches!(url.scheme(), "http" | "https")
        || (url.scheme() == "http" && !loopback)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err(
            "Pairing URLs must use HTTPS, or HTTP for a loopback development endpoint, without embedded credentials."
                .into(),
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn store_credential(credential: &T3Credential) -> Result<(), String> {
    let value = serde_json::to_string(credential).map_err(|error| error.to_string())?;
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|error| error.to_string())?
        .set_password(&value)
        .map_err(|error| format!("could not save T3 credential in Keychain: {error}"))
}

#[cfg(not(target_os = "macos"))]
pub fn store_credential(_: &T3Credential) -> Result<(), String> {
    Err("T3 credential storage currently requires macOS Keychain.".into())
}

#[cfg(target_os = "macos")]
pub fn load_credential() -> Result<T3Credential, String> {
    let value = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|error| error.to_string())?
        .get_password()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&value).map_err(|error| format!("invalid Keychain credential: {error}"))
}

#[cfg(not(target_os = "macos"))]
pub fn load_credential() -> Result<T3Credential, String> {
    Err("T3 credential storage currently requires macOS Keychain.".into())
}

#[cfg(target_os = "macos")]
pub fn forget_credential() -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|error| error.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!(
            "could not remove T3 credential from Keychain: {error}"
        )),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn forget_credential() -> Result<(), String> {
    Ok(())
}

pub fn spawn(
    shared: Arc<Mutex<DaemonState>>,
    mut action_rx: mpsc::UnboundedReceiver<ServerMessage>,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                warn!(%error, "could not create T3 HTTP client");
                return;
            }
        };
        let mut runtime = HashMap::<String, RuntimeThread>::new();
        let mut credential: Option<T3Credential> = None;
        let mut verified_version: Option<String> = None;
        let mut environment_id: Option<String> = None;
        let mut was_enabled = false;
        let mut failures = 0u32;
        let mut retry_after = tokio::time::Instant::now();
        let mut interval = tokio::time::interval(Duration::from_millis(900));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let enabled = shared.lock().await.adapter_enabled("t3code");
                    if !enabled {
                        if was_enabled {
                            shared.lock().await.remove_owner_sessions(T3_OWNER);
                        }
                        was_enabled = false;
                        credential = None;
                        verified_version = None;
                        environment_id = None;
                        runtime.clear();
                        continue;
                    }
                    was_enabled = true;
                    if tokio::time::Instant::now() < retry_after {
                        continue;
                    }
                    if credential.is_none() {
                        credential = load_credential().ok();
                    }
                    let Some(active) = credential.as_ref() else { continue };
                    if verified_version.is_none() {
                        match fetch_descriptor(&client, active).await {
                            Ok(descriptor) if contract_is_supported(&descriptor.server_version) => {
                                verified_version = Some(descriptor.server_version.clone());
                                environment_id = Some(descriptor.environment_id.clone());
                                shared.lock().await.set_adapter_version(
                                    "t3code",
                                    Some(descriptor.server_version),
                                );
                            }
                            Ok(descriptor) => {
                                retry_after = tokio::time::Instant::now() + Duration::from_secs(30);
                                runtime.clear();
                                let mut state = shared.lock().await;
                                state.remove_owner_sessions(T3_OWNER);
                                state.set_adapter_version("t3code", Some(descriptor.server_version.clone()));
                                state.set_adapter_runtime(
                                    "t3code",
                                    AdapterConnectionState::Incompatible,
                                    AdapterCapabilities::default(),
                                    format!(
                                        "T3 Code server {} is incompatible with Microbridge's pinned {} contract ({}). Update Microbridge before reconnecting.",
                                        descriptor.server_version,
                                        SUPPORTED_SERVER_VERSION,
                                        &PINNED_CONTRACT_COMMIT[..12],
                                    ),
                                );
                                continue;
                            }
                            Err(error) => {
                                failures = failures.saturating_add(1);
                                let delay = 1u64 << failures.min(5);
                                retry_after = tokio::time::Instant::now() + Duration::from_secs(delay.min(30));
                                debug!(%error, "T3 Code descriptor unavailable");
                                shared.lock().await.set_adapter_runtime(
                                    "t3code",
                                    AdapterConnectionState::Connecting,
                                    AdapterCapabilities::default(),
                                    "Checking the paired T3 Code contract version…",
                                );
                                continue;
                            }
                        }
                    }
                    match fetch_shell(&client, active).await {
                        Ok(snapshot) => {
                            failures = 0;
                            retry_after = tokio::time::Instant::now();
                            apply_snapshot(&shared, snapshot, &mut runtime).await
                        },
                        Err(T3HttpError::Unauthorized) => {
                            let _ = forget_credential();
                            credential = None;
                            verified_version = None;
                            environment_id = None;
                            runtime.clear();
                            let mut state = shared.lock().await;
                            state.remove_owner_sessions(T3_OWNER);
                            state.set_adapter_runtime(
                                "t3code",
                                AdapterConnectionState::NeedsSetup,
                                AdapterCapabilities::default(),
                                "The paired T3 Code credential was revoked. Pair again.",
                            );
                        }
                        Err(T3HttpError::Other(error)) => {
                            failures = failures.saturating_add(1);
                            let delay = 1u64 << failures.min(5);
                            retry_after = tokio::time::Instant::now() + Duration::from_secs(delay.min(30));
                            debug!(%error, "T3 Code snapshot unavailable");
                            shared.lock().await.set_adapter_runtime(
                                "t3code",
                                AdapterConnectionState::Connecting,
                                capabilities(),
                                "Reconnecting to the paired T3 Code environment…",
                            );
                        }
                    }
                }
                Some(message) = action_rx.recv() => {
                    let ServerMessage::Action { session_id, action } = message else { continue };
                    let Some(active) = credential.as_ref() else { continue };
                    let Some(thread) = runtime.get(&session_id).cloned() else { continue };
                    if let Err(error) = dispatch_action(
                        &client,
                        active,
                        environment_id.as_deref(),
                        &thread.shell,
                        action,
                    ).await {
                        warn!(%error, ?action, session_id, "T3 action failed");
                    }
                }
            }
        }
    });
}

enum T3HttpError {
    Unauthorized,
    Other(String),
}

async fn fetch_descriptor(
    client: &reqwest::Client,
    credential: &T3Credential,
) -> Result<EnvironmentDescriptor, String> {
    let url = Url::parse(&credential.http_base_url)
        .and_then(|base| base.join(".well-known/t3/environment"))
        .map_err(|error| error.to_string())?;
    client
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())
}

fn contract_is_supported(server_version: &str) -> bool {
    server_version == SUPPORTED_SERVER_VERSION
}

async fn fetch_shell(
    client: &reqwest::Client,
    credential: &T3Credential,
) -> Result<ShellSnapshot, T3HttpError> {
    let url = Url::parse(&credential.http_base_url)
        .and_then(|base| base.join("api/orchestration/shell"))
        .map_err(|error| T3HttpError::Other(error.to_string()))?;
    let response = client
        .get(url)
        .bearer_auth(&credential.bearer_token)
        .send()
        .await
        .map_err(|error| T3HttpError::Other(error.to_string()))?;
    if matches!(
        response.status(),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
    ) {
        return Err(T3HttpError::Unauthorized);
    }
    response
        .error_for_status()
        .map_err(|error| T3HttpError::Other(error.to_string()))?
        .json()
        .await
        .map_err(|error| T3HttpError::Other(error.to_string()))
}

async fn apply_snapshot(
    shared: &Arc<Mutex<DaemonState>>,
    snapshot: ShellSnapshot,
    runtime: &mut HashMap<String, RuntimeThread>,
) {
    let mut state = shared.lock().await;
    let incoming: HashSet<String> = snapshot
        .threads
        .iter()
        .map(|thread| format!("t3code:{}", thread.id))
        .collect();
    let stale: Vec<String> = runtime
        .keys()
        .filter(|id| !incoming.contains(*id))
        .cloned()
        .collect();
    for id in stale {
        runtime.remove(&id);
        state.remove_session(&id);
    }
    for thread in snapshot.threads {
        let id = format!("t3code:{}", thread.id);
        let changed = runtime
            .get(&id)
            .map(|current| current.shell != thread)
            .unwrap_or(true);
        let session = SessionStatus {
            id: id.clone(),
            app: "T3 Code".into(),
            title: thread.title.clone(),
            state: map_state(&thread),
            updated_at_ms: parse_iso_ms(&thread.updated_at).unwrap_or_else(now_ms),
        };
        if changed {
            state.upsert_session(session, T3_OWNER);
        }
        runtime.insert(id, RuntimeThread { shell: thread });
    }
    state.set_adapter_runtime(
        "t3code",
        AdapterConnectionState::Limited,
        capabilities(),
        "Paired lifecycle, approvals, interrupt, and native thread focus are ready. Effort remains disabled until T3 advertises provider option descriptors.",
    );
}

fn map_state(thread: &ThreadShell) -> AgentState {
    if thread.has_pending_approvals {
        return AgentState::AwaitingApproval;
    }
    if let Some(turn) = &thread.latest_turn {
        match turn.state.as_str() {
            "running" => return AgentState::Working,
            "error" => return AgentState::Error,
            "completed" => return AgentState::Done,
            "interrupted" => return AgentState::Idle,
            _ => {}
        }
    }
    match thread
        .session
        .as_ref()
        .map(|session| session.status.as_str())
    {
        Some("starting") => AgentState::Thinking,
        Some("running") => AgentState::Working,
        Some("error") => AgentState::Error,
        _ => AgentState::Idle,
    }
}

async fn dispatch_action(
    client: &reqwest::Client,
    credential: &T3Credential,
    environment_id: Option<&str>,
    thread: &ThreadShell,
    action: Action,
) -> Result<(), String> {
    if action == Action::OpenFocusedThread {
        return open_thread(environment_id, &thread.id);
    }
    let command_id = format!("microbridge-{}", now_ms());
    let created_at = now_iso();
    let command = match action {
        Action::Interrupt => {
            let turn_id = thread
                .latest_turn
                .as_ref()
                .map(|turn| turn.turn_id.clone())
                .ok_or_else(|| "T3 Code has no active turn to interrupt.".to_string())?;
            json!({
                "type": "thread.turn.interrupt",
                "commandId": command_id,
                "threadId": thread.id,
                "turnId": turn_id,
                "createdAt": created_at,
            })
        }
        Action::Approve | Action::Reject => {
            let request_id = fetch_pending_approval_id(client, credential, &thread.id).await?;
            json!({
                "type": "thread.approval.respond",
                "commandId": command_id,
                "threadId": thread.id,
                "requestId": request_id,
                "decision": if action == Action::Approve { "accept" } else { "decline" },
                "createdAt": created_at,
            })
        }
        _ => {
            return Err(format!(
                "T3 Code does not advertise {action:?} over the paired HTTP contract."
            ))
        }
    };
    let url = Url::parse(&credential.http_base_url)
        .and_then(|base| base.join("api/orchestration/dispatch"))
        .map_err(|error| error.to_string())?;
    client
        .post(url)
        .bearer_auth(&credential.bearer_token)
        .json(&command)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn open_thread(environment_id: Option<&str>, thread_id: &str) -> Result<(), String> {
    let environment_id = environment_id
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "T3 Code did not expose its environment id.".to_string())?;
    let mut url = Url::parse("t3code://threads/").map_err(|error| error.to_string())?;
    url.path_segments_mut()
        .map_err(|_| "could not construct the T3 Code thread link".to_string())?
        .push(environment_id)
        .push(thread_id);
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg(url.as_str())
            .status()
            .map_err(|error| format!("could not open T3 Code: {error}"))?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| "macOS could not open the T3 Code thread link.".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
        Err("T3 Code thread focus is currently available on macOS only.".into())
    }
}

async fn fetch_pending_approval_id(
    client: &reqwest::Client,
    credential: &T3Credential,
    thread_id: &str,
) -> Result<String, String> {
    let url = Url::parse(&credential.http_base_url)
        .and_then(|base| base.join(&format!("api/orchestration/threads/{thread_id}")))
        .map_err(|error| error.to_string())?;
    let detail: Value = client
        .get(url)
        .bearer_auth(&credential.bearer_token)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    detail
        .get("activities")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .rev()
        .find(|activity| activity.get("tone").and_then(Value::as_str) == Some("approval"))
        .and_then(|activity| find_string_key(activity, "requestId"))
        .ok_or_else(|| "T3 reports a pending approval but did not expose its request id.".into())
}

fn find_string_key(value: &Value, wanted: &str) -> Option<String> {
    match value {
        Value::Object(map) => map.iter().find_map(|(key, value)| {
            if key == wanted {
                value.as_str().map(str::to_string)
            } else {
                find_string_key(value, wanted)
            }
        }),
        Value::Array(values) => values
            .iter()
            .find_map(|value| find_string_key(value, wanted)),
        _ => None,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn parse_iso_ms(value: &str) -> Option<u64> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .ok()
        .and_then(|timestamp| u64::try_from(timestamp.unix_timestamp_nanos() / 1_000_000).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_direct_and_hosted_pairing_urls_without_retaining_token() {
        let (direct, token) = parse_pairing_url("http://127.0.0.1:3773/#token=secret").unwrap();
        assert_eq!(direct.as_str(), "http://127.0.0.1:3773/");
        assert_eq!(token, "secret");
        assert!(!direct.as_str().contains("secret"));

        let (direct_with_slash, _) =
            parse_pairing_url("http://localhost:3773/pair/#token=secret").unwrap();
        assert_eq!(direct_with_slash.as_str(), "http://localhost:3773/");

        let (hosted, token) = parse_pairing_url(
            "https://app.t3.codes/pair?host=https%3A%2F%2Ft3.example.test%2F#token=once",
        )
        .unwrap();
        assert_eq!(hosted.as_str(), "https://t3.example.test/");
        assert_eq!(token, "once");

        assert!(parse_pairing_url(
            "https://app.t3.codes/pair?host=file%3A%2F%2F%2Ftmp%2Fsocket#token=once"
        )
        .is_err());
        assert!(parse_pairing_url("http://t3.example.test/pair#token=once").is_err());
    }

    #[test]
    fn maps_pending_approval_before_running_state() {
        let thread = ThreadShell {
            id: "thread-1".into(),
            title: "Test".into(),
            _model_selection: json!({}),
            latest_turn: Some(LatestTurn {
                turn_id: "turn-1".into(),
                state: "running".into(),
            }),
            session: None,
            has_pending_approvals: true,
            updated_at: "2026-07-18T12:00:00Z".into(),
        };
        assert_eq!(map_state(&thread), AgentState::AwaitingApproval);
    }

    #[test]
    fn pins_the_supported_t3_contract_version() {
        assert!(contract_is_supported("0.0.28"));
        assert!(!contract_is_supported("0.0.28-nightly.1"));
        assert!(!contract_is_supported("0.0.27"));
        assert!(!contract_is_supported("0.0.29"));
        assert_eq!(PINNED_CONTRACT_COMMIT.len(), 40);
    }

    #[tokio::test]
    async fn interrupt_requires_an_active_turn_id() {
        let thread = ThreadShell {
            id: "thread-1".into(),
            title: "Test".into(),
            _model_selection: json!({}),
            latest_turn: None,
            session: None,
            has_pending_approvals: false,
            updated_at: "2026-07-18T12:00:00Z".into(),
        };
        let credential = T3Credential {
            http_base_url: "https://t3.example.test/".into(),
            bearer_token: "unused".into(),
        };

        let error = dispatch_action(
            &reqwest::Client::new(),
            &credential,
            Some("environment-1"),
            &thread,
            Action::Interrupt,
        )
        .await
        .unwrap_err();

        assert_eq!(error, "T3 Code has no active turn to interrupt.");
    }

    #[test]
    fn constructs_the_official_t3_thread_deep_link() {
        let mut url = Url::parse("t3code://threads/").unwrap();
        url.path_segments_mut()
            .unwrap()
            .push("environment 1")
            .push("thread/1");
        assert_eq!(url.as_str(), "t3code://threads/environment%201/thread%2F1");
    }
}
