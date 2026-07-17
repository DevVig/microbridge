//! Wire types for the Microbridge adapter / UI protocol.
//!
//! Transport is newline-delimited JSON over a local Unix domain socket.
//! `docs/protocol.md` is the normative spec; these types are its source of
//! truth — if they disagree, fix one of them in the same PR.

use serde::{Deserialize, Serialize};

/// Protocol revision. Bumped on breaking changes; clients announce theirs in
/// [`ClientMessage::Hello`].
pub const PROTOCOL_VERSION: u32 = 0;

/// Number of Agent Keys on the Codex Micro (kbd-1.0).
pub const AGENT_KEY_COUNT: usize = 6;

/// Lifecycle state of one agent session, as reported by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Thinking,
    Working,
    AwaitingApproval,
    Done,
    Error,
}

/// One agent session — a single thread/conversation in a single app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStatus {
    /// Adapter-scoped unique id, e.g. `codex:0195fa…`.
    pub id: String,
    /// Human-readable app name, e.g. `Claude Code`.
    pub app: String,
    /// Short label for UIs, e.g. the task title. May be empty.
    #[serde(default)]
    pub title: String,
    pub state: AgentState,
    /// Milliseconds since the Unix epoch, supplied by the adapter.
    pub updated_at_ms: u64,
}

/// Who is speaking on the socket. Additive in v0; omitted ⇒ adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientRole {
    #[default]
    Adapter,
    Ui,
}

/// How the six Agent Keys are filled from the session bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeySource {
    /// Cross-app: six most recently updated sessions (default).
    #[default]
    MostRecent,
    /// All six keys re-populate from whichever app owns the deck.
    FocusedApp,
    /// Follow the first six pinned session ids.
    Pinned,
    /// Approvals / active / recent priority ordering.
    Priority,
    /// Explicit per-key session ids (null = unassigned).
    Custom,
}

/// Appearance preference for the optional companion UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

/// Lighting palette name (colors live in config, not on the wire as states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightingPreset {
    #[default]
    Codex,
    Phosphor,
    Custom,
}

/// Per-state RGB as `#RRGGBB`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateColors {
    pub idle: String,
    pub thinking: String,
    pub working: String,
    pub awaiting_approval: String,
    pub done: String,
    pub error: String,
}

impl Default for StateColors {
    fn default() -> Self {
        Self::codex()
    }
}

impl StateColors {
    pub fn codex() -> Self {
        Self {
            idle: "#E9E9E6".into(),
            thinking: "#3D7EFF".into(),
            working: "#3D7EFF".into(),
            awaiting_approval: "#FFB000".into(),
            done: "#30C463".into(),
            error: "#FF453A".into(),
        }
    }

    pub fn phosphor() -> Self {
        Self {
            idle: "#4A4A52".into(),
            thinking: "#FFB454".into(),
            working: "#FF6A00".into(),
            awaiting_approval: "#FF3D00".into(),
            done: "#3DDC84".into(),
            error: "#FF4757".into(),
        }
    }
}

/// Persistent daemon configuration (`~/.microbridge/config.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default)]
    pub key_source: KeySource,
    /// Session ids for `KeySource::Pinned`.
    #[serde(default)]
    pub pinned_session_ids: Vec<String>,
    /// App names in priority order (higher first) for `KeySource::Priority`.
    #[serde(default)]
    pub app_priority: Vec<String>,
    /// Explicit assignments for `KeySource::Custom` (len ≤ 6; empty string = unassigned).
    #[serde(default)]
    pub custom_key_ids: Vec<String>,
    /// When set, this session owns the deck until cleared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_focus: Option<String>,
    /// Approvals preempt focus (default true).
    #[serde(default = "default_true")]
    pub approvals_interrupt: bool,
    #[serde(default)]
    pub pause_leds: bool,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub lighting_preset: LightingPreset,
    #[serde(default)]
    pub state_colors: StateColors,
    /// 0–100
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    /// Minutes of idle before LEDs sleep; 0 = never. Default 3.
    #[serde(default = "default_sleep_minutes")]
    pub sleep_minutes: u32,
    /// Frontmost app name (updated by companion / NSWorkspace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmost_app: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_brightness() -> u8 {
    80
}
fn default_sleep_minutes() -> u32 {
    3
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            key_source: KeySource::MostRecent,
            pinned_session_ids: Vec::new(),
            app_priority: Vec::new(),
            custom_key_ids: vec![String::new(); AGENT_KEY_COUNT],
            pinned_focus: None,
            approvals_interrupt: true,
            pause_leds: false,
            appearance: Appearance::System,
            lighting_preset: LightingPreset::Codex,
            state_colors: StateColors::codex(),
            brightness: 80,
            sleep_minutes: 3,
            frontmost_app: None,
        }
    }
}

/// Full bus view pushed to UI clients after `subscribe`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub sessions: Vec<SessionStatus>,
    pub focused_session_id: Option<String>,
    /// Six Agent Key slots — session ids or null.
    pub agent_key_session_ids: Vec<Option<String>>,
    pub device_connected: bool,
    pub device_name: String,
    pub config: DaemonConfig,
}

/// Incremental bus change for subscribed UI clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BusEvent {
    SessionUpserted { session: SessionStatus },
    SessionRemoved { session_id: String },
    FocusChanged { session_id: Option<String> },
    AgentKeysChanged { session_ids: Vec<Option<String>> },
    DeviceChanged { connected: bool, name: String },
    ConfigChanged { config: DaemonConfig },
}

/// Client → daemon messages (adapters and UI share the socket).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Must be the first message on every connection.
    Hello {
        adapter: String,
        protocol_version: u32,
        #[serde(default)]
        role: ClientRole,
    },
    /// Full state for one session. Sent on every transition — never on a
    /// timer. The daemon treats each `status` as a complete replacement.
    Status { session: SessionStatus },
    /// The session ended and should be dropped from the registry.
    Bye { session_id: String },
    /// UI: request a full [`Snapshot`] and subsequent [`BusEvent`]s.
    Subscribe,
    /// UI: fetch current config (also included in snapshot).
    GetConfig,
    /// UI: replace config and persist.
    SetConfig { config: DaemonConfig },
}

/// Daemon → client messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Key press routed to a session the adapter owns.
    Action { session_id: String, action: Action },
    /// Full bus view (response to Subscribe / reconnect).
    Snapshot { snapshot: Snapshot },
    /// Incremental update for subscribed UI clients.
    Event { event: BusEvent },
    /// Response to GetConfig / acknowledgment of SetConfig.
    Config { config: DaemonConfig },
}

/// Actions a physical key can trigger on the focused agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Approve,
    Reject,
    Interrupt,
    NewSession,
    CycleFocus,
}

/// Backward-compatible aliases used by older docs / reference adapter.
pub type Message = ClientMessage;
pub type Command = ServerMessage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips() {
        let msg = ClientMessage::Status {
            session: SessionStatus {
                id: "codex:abc".into(),
                app: "Codex CLI".into(),
                title: "fix flaky e2e retries".into(),
                state: AgentState::AwaitingApproval,
                updated_at_ms: 1,
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"status""#));
        assert!(json.contains(r#""state":"awaiting_approval""#));
        assert_eq!(serde_json::from_str::<ClientMessage>(&json).unwrap(), msg);
    }

    #[test]
    fn title_defaults_to_empty() {
        let json = r#"{"type":"status","session":{"id":"x:1","app":"X","state":"idle","updated_at_ms":0}}"#;
        let ClientMessage::Status { session } = serde_json::from_str(json).unwrap() else {
            panic!("expected status");
        };
        assert_eq!(session.title, "");
    }

    #[test]
    fn hello_role_defaults_to_adapter() {
        let json = r#"{"type":"hello","adapter":"reference-echo","protocol_version":0}"#;
        let ClientMessage::Hello { role, .. } = serde_json::from_str(json).unwrap() else {
            panic!("expected hello");
        };
        assert_eq!(role, ClientRole::Adapter);
    }

    #[test]
    fn ui_hello_and_snapshot_round_trip() {
        let hello = ClientMessage::Hello {
            adapter: "microbridge-ui".into(),
            protocol_version: 0,
            role: ClientRole::Ui,
        };
        let json = serde_json::to_string(&hello).unwrap();
        assert!(json.contains(r#""role":"ui""#));

        let snap = ServerMessage::Snapshot {
            snapshot: Snapshot {
                sessions: vec![],
                focused_session_id: None,
                agent_key_session_ids: vec![None; AGENT_KEY_COUNT],
                device_connected: false,
                device_name: "mock".into(),
                config: DaemonConfig::default(),
            },
        };
        let round =
            serde_json::from_str::<ServerMessage>(&serde_json::to_string(&snap).unwrap()).unwrap();
        assert_eq!(round, snap);
    }
}
