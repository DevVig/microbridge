//! Wire types for the Microbridge adapter / UI protocol.
//!
//! Transport is newline-delimited JSON over a local Unix domain socket.
//! `docs/protocol.md` is the normative spec; these types are its source of
//! truth — if they disagree, fix one of them in the same PR.

use std::collections::BTreeMap;

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

/// The exact LED state the daemon resolved for one physical Agent Key.
/// This is included in snapshots so the software twin mirrors the hardware
/// frame instead of independently guessing colors or assignments.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgentKeyLed {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub state: Option<AgentState>,
    /// Effective palette color as `#RRGGBB`; `None` means the LED is off.
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub focused: bool,
}

/// The full frame sent to the device layer after palette normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentKeyLedFrame {
    #[serde(default)]
    pub keys: Vec<AgentKeyLed>,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    #[serde(default)]
    pub paused: bool,
}

impl Default for AgentKeyLedFrame {
    fn default() -> Self {
        Self {
            // Empty distinguishes an omitted legacy field from a real six-key
            // frame, allowing newer UIs to derive a compatibility fallback.
            keys: Vec::new(),
            brightness: default_brightness(),
            paused: false,
        }
    }
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
    /// Cross-app: six most recently updated sessions.
    MostRecent,
    /// Owning IDE's newest sessions (default) — Claude, Codex, Cursor, Synara, T3, …
    #[default]
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

impl LightingPreset {
    /// Resolve a named preset to its effective palette. Custom deliberately
    /// returns `None` so callers preserve the user-selected colors.
    pub fn colors(self) -> Option<StateColors> {
        match self {
            Self::Codex => Some(StateColors::codex()),
            Self::Phosphor => Some(StateColors::phosphor()),
            Self::Custom => None,
        }
    }
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

/// Whether an adapter ships with Microbridge or is managed by its host IDE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    #[default]
    Native,
    Community,
}

/// Truthful runtime state shown in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterConnectionState {
    #[default]
    Disabled,
    NeedsSetup,
    Connecting,
    Connected,
    Limited,
    Incompatible,
    Error,
}

/// Commands and observations an adapter can actually perform in its current
/// negotiated version. New fields default false for older clients.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    #[serde(default)]
    pub lifecycle_observation: bool,
    #[serde(default)]
    pub approval_acceptance: bool,
    #[serde(default)]
    pub approval_rejection: bool,
    #[serde(default)]
    pub interrupt: bool,
    #[serde(default)]
    pub new_session: bool,
    #[serde(default)]
    pub focus_open: bool,
    #[serde(default)]
    pub reasoning_effort: bool,
}

impl AdapterCapabilities {
    pub fn lifecycle_only() -> Self {
        Self {
            lifecycle_observation: true,
            ..Self::default()
        }
    }

    pub fn full_control() -> Self {
        Self {
            lifecycle_observation: true,
            approval_acceptance: true,
            approval_rejection: true,
            interrupt: true,
            new_session: true,
            focus_open: true,
            reasoning_effort: true,
        }
    }

    pub fn supports(&self, action: Action) -> bool {
        match action {
            Action::Approve => self.approval_acceptance,
            Action::Reject => self.approval_rejection,
            Action::Interrupt => self.interrupt,
            Action::NewSession => self.new_session,
            Action::OpenFocusedThread => self.focus_open,
            Action::ReasoningEffortUp | Action::ReasoningEffortDown => self.reasoning_effort,
            Action::CycleFocus
            | Action::NavigateUp
            | Action::NavigateDown
            | Action::NavigateLeft
            | Action::NavigateRight => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterStatus {
    pub id: String,
    pub display_name: String,
    pub kind: AdapterKind,
    pub state: AdapterConnectionState,
    #[serde(default)]
    pub capabilities: AdapterCapabilities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity_ms: Option<u64>,
    #[serde(default)]
    pub diagnostic: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterPreference {
    #[serde(default)]
    pub enabled: bool,
}

pub fn default_adapter_preferences() -> BTreeMap<String, AdapterPreference> {
    BTreeMap::from([
        ("codex".into(), AdapterPreference { enabled: true }),
        ("claude".into(), AdapterPreference { enabled: true }),
        ("cnvs".into(), AdapterPreference { enabled: true }),
        ("cursor".into(), AdapterPreference { enabled: false }),
        ("t3code".into(), AdapterPreference { enabled: false }),
        ("factory".into(), AdapterPreference { enabled: false }),
        ("opencode".into(), AdapterPreference { enabled: false }),
    ])
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
    /// Per-host consent. Cursor, T3 Code, Factory, and OpenCode are disabled
    /// until explicitly enabled.
    #[serde(default = "default_adapter_preferences")]
    pub adapters: BTreeMap<String, AdapterPreference>,
    /// Claim and poll the physical HID interface. Off unless explicitly enabled.
    #[serde(default)]
    pub hardware_control_enabled: bool,
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
            key_source: KeySource::FocusedApp,
            pinned_session_ids: Vec::new(),
            app_priority: Vec::new(),
            custom_key_ids: vec![String::new(); AGENT_KEY_COUNT],
            pinned_focus: None,
            approvals_interrupt: true,
            pause_leds: false,
            appearance: Appearance::System,
            lighting_preset: LightingPreset::Codex,
            state_colors: StateColors::codex(),
            adapters: default_adapter_preferences(),
            hardware_control_enabled: false,
            brightness: 80,
            sleep_minutes: 3,
            frontmost_app: None,
        }
    }
}

impl DaemonConfig {
    /// Normalize legacy and partial config at every trust boundary.
    pub fn normalize(&mut self) {
        for (id, preference) in default_adapter_preferences() {
            self.adapters.entry(id).or_insert(preference);
        }
        self.brightness = self.brightness.min(100);
        if let Some(colors) = self.lighting_preset.colors() {
            self.state_colors = colors;
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
    /// Exact frame passed to the device implementation. Older daemons omit it;
    /// clients must fall back to `agent_key_session_ids` in that case.
    #[serde(default)]
    pub agent_key_led_frame: AgentKeyLedFrame,
    pub device_connected: bool,
    pub device_name: String,
    pub config: DaemonConfig,
    #[serde(default)]
    pub adapters: Vec<AdapterStatus>,
}

/// Incremental bus change for subscribed UI clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BusEvent {
    SessionUpserted {
        session: SessionStatus,
    },
    SessionRemoved {
        session_id: String,
    },
    FocusChanged {
        session_id: Option<String>,
    },
    AgentKeysChanged {
        session_ids: Vec<Option<String>>,
        #[serde(default)]
        led_frame: AgentKeyLedFrame,
    },
    DeviceChanged {
        connected: bool,
        name: String,
    },
    ConfigChanged {
        config: Box<DaemonConfig>,
    },
    AdaptersChanged {
        adapters: Vec<AdapterStatus>,
    },
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adapter_version: Option<String>,
        #[serde(default)]
        capabilities: AdapterCapabilities,
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
    /// UI: enable or disable one opt-in integration.
    SetAdapterEnabled { adapter_id: String, enabled: bool },
    /// UI: pair a T3 Code environment with a one-time pairing URL.
    PairAdapter {
        adapter_id: String,
        pairing_url: String,
    },
    /// UI: remove all local credentials and runtime state for an integration.
    ForgetAdapter { adapter_id: String },
    /// Managed IDE hook: upsert a lease-backed session without owning a long-lived socket.
    IngestLifecycle {
        adapter_id: String,
        session: SessionStatus,
        #[serde(default = "default_lifecycle_ttl_ms")]
        ttl_ms: u64,
    },
    /// UI/software-twin equivalent of pressing one physical Agent Key.
    ActivateAgentKey {
        index: usize,
        #[serde(default)]
        open: bool,
    },
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
    /// A requested config mutation was rejected; existing runtime state is unchanged.
    ConfigError { message: String },
    AdapterOperation {
        adapter_id: String,
        ok: bool,
        message: String,
    },
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
    ReasoningEffortUp,
    ReasoningEffortDown,
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    OpenFocusedThread,
}

fn default_lifecycle_ttl_ms() -> u64 {
    30 * 60 * 1000
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
            adapter_version: None,
            capabilities: AdapterCapabilities::default(),
        };
        let json = serde_json::to_string(&hello).unwrap();
        assert!(json.contains(r#""role":"ui""#));

        let snap = ServerMessage::Snapshot {
            snapshot: Snapshot {
                sessions: vec![],
                focused_session_id: None,
                agent_key_session_ids: vec![None; AGENT_KEY_COUNT],
                agent_key_led_frame: AgentKeyLedFrame::default(),
                device_connected: false,
                device_name: "mock".into(),
                config: DaemonConfig::default(),
                adapters: vec![],
            },
        };
        let round =
            serde_json::from_str::<ServerMessage>(&serde_json::to_string(&snap).unwrap()).unwrap();
        assert_eq!(round, snap);
    }

    #[test]
    fn legacy_snapshot_defaults_the_led_frame() {
        let json = r#"{"type":"snapshot","snapshot":{"sessions":[],"focused_session_id":null,"agent_key_session_ids":[null,null,null,null,null,null],"device_connected":false,"device_name":"mock","config":{},"adapters":[]}}"#;
        let ServerMessage::Snapshot { snapshot } = serde_json::from_str(json).unwrap() else {
            panic!("expected snapshot");
        };
        assert_eq!(snapshot.agent_key_led_frame, AgentKeyLedFrame::default());
    }

    #[test]
    fn legacy_config_gets_adapter_and_hardware_defaults() {
        let config: DaemonConfig = serde_json::from_str("{}").unwrap();
        assert!(config.adapters["codex"].enabled);
        assert!(!config.adapters["cursor"].enabled);
        assert!(!config.hardware_control_enabled);
    }

    #[test]
    fn named_lighting_presets_resolve_their_palette() {
        assert_eq!(LightingPreset::Codex.colors(), Some(StateColors::codex()));
        assert_eq!(
            LightingPreset::Phosphor.colors(),
            Some(StateColors::phosphor())
        );
        assert_eq!(LightingPreset::Custom.colors(), None);
    }

    #[test]
    fn normalize_applies_named_palette_and_preserves_custom_colors() {
        let mut named = DaemonConfig {
            lighting_preset: LightingPreset::Phosphor,
            state_colors: StateColors::codex(),
            ..DaemonConfig::default()
        };
        named.normalize();
        assert_eq!(named.state_colors, StateColors::phosphor());

        let custom_colors = StateColors {
            idle: "#010101".into(),
            ..StateColors::codex()
        };
        let mut custom = DaemonConfig {
            lighting_preset: LightingPreset::Custom,
            state_colors: custom_colors.clone(),
            ..DaemonConfig::default()
        };
        custom.normalize();
        assert_eq!(custom.state_colors, custom_colors);
    }
}
