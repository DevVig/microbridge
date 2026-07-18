//! Shared daemon state: registry, config, device, subscribers.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mb_device::{parse_rgb_hex, Device, LedFrame};
use mb_protocol::{
    Action, AdapterCapabilities, AdapterConnectionState, AdapterKind, AdapterStatus, AgentState,
    BusEvent, DaemonConfig, ServerMessage, SessionStatus, Snapshot, AGENT_KEY_COUNT,
};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::config::save_config;
use crate::registry::Registry;

pub type SharedState = Arc<Mutex<DaemonState>>;

static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

pub fn next_conn_id() -> u64 {
    NEXT_CONN.fetch_add(1, Ordering::Relaxed)
}

pub struct DaemonState {
    pub registry: Registry,
    pub config: DaemonConfig,
    pub device: Box<dyn Device>,
    /// Write channels for adapter connections (conn_id → tx).
    pub adapter_txs: HashMap<u64, mpsc::UnboundedSender<ServerMessage>>,
    /// Write channels for subscribed UI clients.
    pub ui_txs: HashMap<u64, mpsc::UnboundedSender<ServerMessage>>,
    /// Adapter metadata negotiated by connection id.
    pub adapter_connections: HashMap<u64, String>,
    pub adapter_capabilities: HashMap<u64, AdapterCapabilities>,
    /// Runtime adapter cards keyed by stable adapter id.
    pub adapters: BTreeMap<String, AdapterStatus>,
    /// One-shot hook sessions survive their short socket connection until this deadline.
    leased_sessions: HashMap<String, Instant>,
    last_agent_key_press: [Option<Instant>; AGENT_KEY_COUNT],
    last_leds: LedFrame,
}

impl DaemonState {
    pub fn new(device: Box<dyn Device>, mut config: DaemonConfig) -> Self {
        config.normalize();
        let adapters = initial_adapter_statuses(&config);
        Self {
            registry: Registry::default(),
            config,
            device,
            adapter_txs: HashMap::new(),
            ui_txs: HashMap::new(),
            adapter_connections: HashMap::new(),
            adapter_capabilities: HashMap::new(),
            adapters,
            leased_sessions: HashMap::new(),
            last_agent_key_press: [None; AGENT_KEY_COUNT],
            last_leds: LedFrame::default(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let desc = self.device.descriptor();
        let keys = self.registry.agent_key_ids(&self.config);
        Snapshot {
            sessions: self.registry.session_list(),
            focused_session_id: self.registry.focused.clone(),
            agent_key_session_ids: keys.into_iter().collect(),
            device_connected: desc.connected,
            device_name: desc.name,
            config: self.config.clone(),
            adapters: self.adapters.values().cloned().collect(),
        }
    }

    pub fn upsert_session(&mut self, session: SessionStatus, owner: u64) {
        let prev_focus = self.registry.focused.clone();
        self.registry.upsert(session.clone(), owner, &self.config);
        self.broadcast_ui(BusEvent::SessionUpserted {
            session: session.clone(),
        });
        self.after_bus_change(prev_focus);
    }

    pub fn remove_session(&mut self, session_id: &str) {
        let prev_focus = self.registry.focused.clone();
        self.registry.remove(session_id, &self.config);
        self.broadcast_ui(BusEvent::SessionRemoved {
            session_id: session_id.to_string(),
        });
        self.after_bus_change(prev_focus);
    }

    pub fn drop_connection(&mut self, conn_id: u64) {
        let prev_focus = self.registry.focused.clone();
        self.adapter_txs.remove(&conn_id);
        self.ui_txs.remove(&conn_id);
        self.adapter_capabilities.remove(&conn_id);
        if let Some(adapter_id) = self.adapter_connections.remove(&conn_id) {
            let still_connected = self
                .adapter_connections
                .values()
                .any(|connected| connected == &adapter_id);
            if !still_connected {
                if let Some(status) = self.adapters.get_mut(&adapter_id) {
                    if status.state != AdapterConnectionState::Disabled {
                        status.state = AdapterConnectionState::NeedsSetup;
                        status.diagnostic = reconnect_diagnostic(&adapter_id);
                    }
                }
                self.broadcast_adapters();
            }
        }
        self.registry.remove_owner(conn_id, &self.config);
        self.after_bus_change(prev_focus);
    }

    pub fn set_config(&mut self, mut config: DaemonConfig) -> Result<(), String> {
        let hardware_control_changed =
            config.hardware_control_enabled != self.config.hardware_control_enabled;
        // `frontmost_app` is watcher-owned runtime state — clients cannot set it.
        let frontmost = self.config.frontmost_app.clone();
        config.normalize();
        config.frontmost_app = frontmost;
        save_config(&config).map_err(|error| format!("failed to persist config: {error}"))?;

        let prev_focus = self.registry.focused.clone();
        self.config = config;
        if hardware_control_changed {
            self.device = mb_device::open_default_device_with_claim(
                self.config.hardware_control_enabled || hid_claim_env_enabled(),
            );
            self.last_leds = LedFrame::default();
            let descriptor = self.device.descriptor();
            self.broadcast_ui(BusEvent::DeviceChanged {
                connected: descriptor.connected,
                name: descriptor.name,
            });
        }
        self.registry.resolve_focus(&self.config);
        self.broadcast_ui(BusEvent::ConfigChanged {
            config: Box::new(self.config.clone()),
        });
        self.after_bus_change(prev_focus);
        Ok(())
    }

    pub fn adapter_enabled(&self, adapter_id: &str) -> bool {
        self.config
            .adapters
            .get(adapter_id)
            .map(|preference| preference.enabled)
            .unwrap_or(false)
    }

    pub fn set_adapter_enabled(&mut self, adapter_id: &str, enabled: bool) -> Result<(), String> {
        if !self.adapters.contains_key(adapter_id) {
            return Err(format!("unknown adapter: {adapter_id}"));
        }
        let mut next_config = self.config.clone();
        next_config
            .adapters
            .entry(adapter_id.to_string())
            .or_default()
            .enabled = enabled;
        next_config.normalize();
        save_config(&next_config)
            .map_err(|error| format!("failed to persist adapter consent: {error}"))?;
        self.config = next_config;

        let status = self
            .adapters
            .get_mut(adapter_id)
            .expect("adapter existence checked above");
        if enabled {
            if status.kind == AdapterKind::Native && matches!(adapter_id, "codex" | "claude") {
                status.state = AdapterConnectionState::Connected;
                status.capabilities = AdapterCapabilities::lifecycle_only();
                status.diagnostic = "Built-in lifecycle watcher is active.".into();
            } else {
                status.state = AdapterConnectionState::NeedsSetup;
                status.diagnostic = setup_diagnostic(adapter_id);
            }
        } else {
            status.state = AdapterConnectionState::Disabled;
            status.last_activity_ms = None;
            status.diagnostic = "Disabled until you explicitly enable this integration.".into();
            let session_prefix = format!("{adapter_id}:");
            let session_ids: Vec<String> = self
                .registry
                .sessions
                .keys()
                .filter(|id| id.starts_with(&session_prefix))
                .cloned()
                .collect();
            for session_id in session_ids {
                self.leased_sessions.remove(&session_id);
                self.remove_session(&session_id);
            }
            let owners: Vec<u64> = self
                .adapter_connections
                .iter()
                // High owner ids are daemon-owned adapters whose command
                // channel must survive disable/re-enable cycles.
                .filter_map(|(owner, id)| {
                    (id == adapter_id && *owner < u64::MAX - 1024).then_some(*owner)
                })
                .collect();
            for owner in owners {
                self.adapter_txs.remove(&owner);
                self.adapter_capabilities.remove(&owner);
                self.adapter_connections.remove(&owner);
                self.registry.remove_owner(owner, &self.config);
            }
        }
        self.broadcast_ui(BusEvent::ConfigChanged {
            config: Box::new(self.config.clone()),
        });
        self.broadcast_adapters();
        Ok(())
    }

    pub fn register_adapter(
        &mut self,
        conn_id: u64,
        adapter_id: String,
        version: Option<String>,
        capabilities: AdapterCapabilities,
        tx: mpsc::UnboundedSender<ServerMessage>,
    ) -> Result<(), String> {
        if !self.adapters.contains_key(&adapter_id) {
            self.config
                .adapters
                .insert(adapter_id.clone(), Default::default());
            self.adapters.insert(
                adapter_id.clone(),
                AdapterStatus {
                    id: adapter_id.clone(),
                    display_name: adapter_id.clone(),
                    kind: AdapterKind::Community,
                    state: AdapterConnectionState::NeedsSetup,
                    capabilities,
                    version,
                    last_activity_ms: Some(now_ms()),
                    diagnostic: "Integration discovered. Review its capabilities and enable it before Microbridge accepts events.".into(),
                },
            );
            self.broadcast_adapters();
            return Err(format!(
                "{adapter_id} is pending approval in Microbridge Settings"
            ));
        }
        if !self.adapter_enabled(&adapter_id) {
            return Err(format!(
                "{adapter_id} is disabled; enable it in Settings first"
            ));
        }
        let status = self
            .adapters
            .get_mut(&adapter_id)
            .ok_or_else(|| format!("unknown adapter: {adapter_id}"))?;
        status.state =
            if adapter_id == "cursor" && capabilities != AdapterCapabilities::full_control() {
                AdapterConnectionState::Limited
            } else {
                AdapterConnectionState::Connected
            };
        status.version = version;
        status.last_activity_ms = Some(now_ms());
        status.capabilities = capabilities.clone();
        status.diagnostic = if status.state == AdapterConnectionState::Limited {
            "Lifecycle is connected. Cursor does not yet expose every hardware command.".into()
        } else {
            "Connected and ready.".into()
        };
        self.adapter_txs.insert(conn_id, tx);
        self.adapter_connections.insert(conn_id, adapter_id);
        self.adapter_capabilities.insert(conn_id, capabilities);
        self.broadcast_adapters();
        Ok(())
    }

    pub fn ingest_lifecycle(
        &mut self,
        adapter_id: &str,
        session: SessionStatus,
        ttl_ms: u64,
    ) -> Result<(), String> {
        if !self.adapter_enabled(adapter_id) {
            return Err(format!(
                "{adapter_id} is disabled; approve it in Settings first"
            ));
        }
        let expected_prefix = format!("{adapter_id}:");
        let scoped_id = session.id.strip_prefix(&expected_prefix);
        if scoped_id.is_none() || scoped_id.is_some_and(str::is_empty) {
            return Err(format!(
                "session id must use the {expected_prefix}<session> namespace"
            ));
        }
        let status = self
            .adapters
            .get_mut(adapter_id)
            .ok_or_else(|| format!("unknown adapter: {adapter_id}"))?;
        status.state = AdapterConnectionState::Limited;
        status.capabilities = AdapterCapabilities::lifecycle_only();
        status.last_activity_ms = Some(now_ms());
        status.diagnostic =
            "Lifecycle is connected; unsupported IDE commands remain disabled.".into();
        let id = session.id.clone();
        self.upsert_session(session, 0);
        self.leased_sessions.insert(
            id,
            Instant::now() + Duration::from_millis(ttl_ms.clamp(1_000, 24 * 60 * 60 * 1000)),
        );
        self.broadcast_adapters();
        Ok(())
    }

    pub fn set_adapter_runtime(
        &mut self,
        adapter_id: &str,
        state: AdapterConnectionState,
        capabilities: AdapterCapabilities,
        diagnostic: impl Into<String>,
    ) {
        let diagnostic = diagnostic.into();
        if let Some(status) = self.adapters.get_mut(adapter_id) {
            let changed = status.state != state
                || status.capabilities != capabilities
                || status.diagnostic != diagnostic;
            status.state = state;
            status.capabilities = capabilities;
            status.diagnostic = diagnostic;
            status.last_activity_ms = Some(now_ms());
            if changed {
                self.broadcast_adapters();
            }
        }
    }

    pub fn set_adapter_version(&mut self, adapter_id: &str, version: Option<String>) {
        if let Some(status) = self.adapters.get_mut(adapter_id) {
            if status.version != version {
                status.version = version;
                self.broadcast_adapters();
            }
        }
    }

    pub fn install_internal_adapter(
        &mut self,
        owner: u64,
        adapter_id: &str,
        capabilities: AdapterCapabilities,
        tx: mpsc::UnboundedSender<ServerMessage>,
    ) {
        self.adapter_txs.insert(owner, tx);
        self.adapter_connections.insert(owner, adapter_id.into());
        self.adapter_capabilities.insert(owner, capabilities);
    }

    pub fn remove_owner_sessions(&mut self, owner: u64) {
        let previous = self.registry.focused.clone();
        let removed: Vec<String> = self
            .registry
            .owners
            .iter()
            .filter_map(|(id, session_owner)| (*session_owner == owner).then_some(id.clone()))
            .collect();
        for id in removed {
            self.registry.remove(&id, &self.config);
            self.broadcast_ui(BusEvent::SessionRemoved { session_id: id });
        }
        self.after_bus_change(previous);
    }

    pub fn expire_leased_sessions(&mut self) {
        let now = Instant::now();
        let expired: Vec<String> = self
            .leased_sessions
            .iter()
            .filter_map(|(id, deadline)| (*deadline <= now).then_some(id.clone()))
            .collect();
        for id in expired {
            self.leased_sessions.remove(&id);
            self.remove_session(&id);
        }
    }

    /// Update the ephemeral frontmost app (not written to disk).
    pub fn set_frontmost_app(&mut self, app: Option<String>) {
        if self.config.frontmost_app == app {
            return;
        }
        let prev_focus = self.registry.focused.clone();
        self.config.frontmost_app = app;
        self.registry.resolve_focus(&self.config);
        self.broadcast_ui(BusEvent::ConfigChanged {
            config: Box::new(self.config.clone()),
        });
        self.after_bus_change(prev_focus);
    }

    fn after_bus_change(&mut self, prev_focus: Option<String>) {
        if self.registry.focused != prev_focus {
            self.broadcast_ui(BusEvent::FocusChanged {
                session_id: self.registry.focused.clone(),
            });
        }
        let keys = self.registry.agent_key_ids(&self.config);
        self.broadcast_ui(BusEvent::AgentKeysChanged {
            session_ids: keys.clone().into_iter().collect(),
        });
        self.render_leds(&keys);
    }

    pub fn render_leds(&mut self, keys: &[Option<String>; AGENT_KEY_COUNT]) {
        let mut frame = LedFrame {
            keys: [None; AGENT_KEY_COUNT],
            key_colors: [None; AGENT_KEY_COUNT],
            focus_index: None,
            brightness: self.config.brightness,
            paused: self.config.pause_leds,
        };
        for (i, id) in keys.iter().enumerate() {
            let state = id
                .as_ref()
                .and_then(|sid| self.registry.sessions.get(sid))
                .map(|s| s.state);
            frame.keys[i] = state;
            frame.key_colors[i] = state.and_then(|s| color_for_state(&self.config, s));
            if id.as_ref() == self.registry.focused.as_ref() {
                frame.focus_index = Some(i);
            }
        }
        if frame != self.last_leds {
            self.device.set_leds(&frame);
            self.last_leds = frame;
        }
    }

    pub fn route_action(&self, session_id: &str, action: Action) -> Result<(), String> {
        let Some(session) = self.registry.sessions.get(session_id) else {
            warn!(session_id, ?action, "session no longer exists");
            return Err("The focused thread has expired or ended.".into());
        };
        if matches!(action, Action::Approve | Action::Reject)
            && session.state != AgentState::AwaitingApproval
        {
            return Err("That approval has already expired or been resolved.".into());
        }
        let Some(owner) = self.registry.owner_of(session_id) else {
            warn!(session_id, ?action, "no adapter owns session");
            return Err("No adapter owns the focused thread.".into());
        };
        if let Some(capabilities) = self.adapter_capabilities.get(&owner) {
            if !capabilities.supports(action) {
                return Err(format!("The active adapter does not support {action:?}."));
            }
        }
        let Some(tx) = self.adapter_txs.get(&owner) else {
            // In-process adapters: owner id 0 is reserved for local handlers.
            // Codex/Claude control-plane mapping lands in a follow-up (#24);
            // until then actions are acknowledged but not forwarded to a CLI.
            if owner == 0 {
                info!(
                    session_id,
                    ?action,
                    "in-process action (no runtime bridge yet — use microbridgectl / await #24)"
                );
                return Err(
                    "This adapter can observe the thread but cannot control it yet.".into(),
                );
            }
            warn!(session_id, ?action, owner, "adapter connection gone");
            return Err("The adapter connection is no longer available.".into());
        };
        tx.send(ServerMessage::Action {
            session_id: session_id.to_string(),
            action,
        })
        .map_err(|_| "The adapter stopped before the action was delivered.".to_string())
    }

    pub fn handle_device_action(&mut self, action: Action) {
        if let Some(id) = self.registry.focused.clone() {
            if let Err(error) = self.route_action(&id, action) {
                warn!(%error, ?action, "device action was not delivered");
            }
        }
    }

    pub fn handle_device_input(&mut self, input: mb_device::DeviceInput) {
        use mb_device::{DeviceInput, JoystickDir};
        match input {
            DeviceInput::AgentKeyPress { index } => {
                let now = Instant::now();
                let double = self
                    .last_agent_key_press
                    .get(index)
                    .and_then(|value| *value)
                    .map(|previous| now.duration_since(previous) <= Duration::from_millis(350))
                    .unwrap_or(false);
                if let Some(slot) = self.last_agent_key_press.get_mut(index) {
                    *slot = if double { None } else { Some(now) };
                }
                self.focus_agent_key(index);
                if double {
                    self.handle_device_action(Action::OpenFocusedThread);
                }
            }
            DeviceInput::AgentKeyDoublePress { index } => {
                self.focus_agent_key(index);
                self.handle_device_action(Action::OpenFocusedThread);
            }
            DeviceInput::Approve => self.handle_device_action(Action::Approve),
            DeviceInput::Reject => self.handle_device_action(Action::Reject),
            DeviceInput::Interrupt => self.handle_device_action(Action::Interrupt),
            DeviceInput::NewSession => self.handle_device_action(Action::NewSession),
            DeviceInput::CycleFocus | DeviceInput::TouchTap => self.move_focus(1),
            DeviceInput::DialRotate { delta } if delta < 0 => {
                self.handle_device_action(Action::ReasoningEffortDown)
            }
            DeviceInput::DialRotate { delta } if delta > 0 => {
                self.handle_device_action(Action::ReasoningEffortUp)
            }
            DeviceInput::DialRotate { .. } => {}
            DeviceInput::DialPress => self.handle_device_action(Action::OpenFocusedThread),
            DeviceInput::JoystickFlick { direction } => match direction {
                JoystickDir::Up | JoystickDir::Left => self.move_focus(-1),
                JoystickDir::Down | JoystickDir::Right => self.move_focus(1),
            },
        }
    }

    pub fn poll_device_inputs(&mut self) {
        if !self.config.hardware_control_enabled && !hid_claim_env_enabled() {
            return;
        }
        // Drain a bounded batch so noisy firmware cannot starve socket work.
        for _ in 0..64 {
            let Some(input) = self.device.poll_input() else {
                break;
            };
            self.handle_device_input(input);
        }
    }

    fn move_focus(&mut self, offset: isize) {
        let sessions = self.registry.session_list();
        if sessions.is_empty() {
            return;
        }
        let current = self
            .registry
            .focused
            .as_ref()
            .and_then(|id| sessions.iter().position(|session| &session.id == id))
            .unwrap_or(0) as isize;
        let next = (current + offset).rem_euclid(sessions.len() as isize) as usize;
        let previous = self.registry.focused.clone();
        self.registry.focused = Some(sessions[next].id.clone());
        if previous != self.registry.focused {
            self.broadcast_ui(BusEvent::FocusChanged {
                session_id: self.registry.focused.clone(),
            });
            let keys = self.registry.agent_key_ids(&self.config);
            self.render_leds(&keys);
        }
    }

    pub fn focus_agent_key(&mut self, index: usize) {
        let keys = self.registry.agent_key_ids(&self.config);
        if let Some(Some(id)) = keys.get(index) {
            let prev = self.registry.focused.clone();
            self.registry.focused = Some(id.clone());
            if prev != self.registry.focused {
                self.broadcast_ui(BusEvent::FocusChanged {
                    session_id: self.registry.focused.clone(),
                });
                self.render_leds(&keys);
            }
        }
    }

    fn broadcast_ui(&mut self, event: BusEvent) {
        let msg = ServerMessage::Event { event };
        self.ui_txs.retain(|_, tx| tx.send(msg.clone()).is_ok());
    }

    fn broadcast_adapters(&mut self) {
        self.broadcast_ui(BusEvent::AdaptersChanged {
            adapters: self.adapters.values().cloned().collect(),
        });
    }

    pub fn send_to_ui(&self, conn_id: u64, msg: ServerMessage) {
        if let Some(tx) = self.ui_txs.get(&conn_id) {
            let _ = tx.send(msg);
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn setup_diagnostic(adapter_id: &str) -> String {
    match adapter_id {
        "cursor" => {
            "The bundled Cursor integration is installed. Reload Cursor once if it is already open."
                .into()
        }
        "t3code" => "Paste a one-time pairing link from T3 Code Settings → Connections.".into(),
        _ => "Waiting for the adapter to connect.".into(),
    }
}

fn reconnect_diagnostic(adapter_id: &str) -> String {
    match adapter_id {
        "cursor" => "Cursor is enabled, but no lifecycle event has arrived yet. Reload Cursor if it was already open.".into(),
        "t3code" => "T3 Code is enabled, but its paired connection is offline.".into(),
        _ => "Adapter is offline.".into(),
    }
}

fn initial_adapter_statuses(config: &DaemonConfig) -> BTreeMap<String, AdapterStatus> {
    let native = |id: &str, display_name: &str| {
        let enabled = config.adapters.get(id).map(|v| v.enabled).unwrap_or(true);
        AdapterStatus {
            id: id.into(),
            display_name: display_name.into(),
            kind: AdapterKind::Native,
            state: if enabled {
                AdapterConnectionState::Connected
            } else {
                AdapterConnectionState::Disabled
            },
            capabilities: if enabled {
                AdapterCapabilities::lifecycle_only()
            } else {
                AdapterCapabilities::default()
            },
            version: Some(env!("CARGO_PKG_VERSION").into()),
            last_activity_ms: None,
            diagnostic: if enabled {
                "Built-in lifecycle watcher is active.".into()
            } else {
                "Disabled in Microbridge configuration.".into()
            },
        }
    };
    let opt_in = |id: &str, display_name: &str| {
        let enabled = config.adapters.get(id).map(|v| v.enabled).unwrap_or(false);
        AdapterStatus {
            id: id.into(),
            display_name: display_name.into(),
            kind: AdapterKind::Community,
            state: if enabled {
                AdapterConnectionState::NeedsSetup
            } else {
                AdapterConnectionState::Disabled
            },
            capabilities: AdapterCapabilities::default(),
            version: None,
            last_activity_ms: None,
            diagnostic: if enabled {
                setup_diagnostic(id)
            } else {
                "Disabled until you explicitly enable this integration.".into()
            },
        }
    };
    BTreeMap::from([
        ("claude".into(), native("claude", "Claude Code")),
        ("codex".into(), native("codex", "Codex CLI")),
        ("cursor".into(), opt_in("cursor", "Cursor")),
        ("t3code".into(), opt_in("t3code", "T3 Code")),
    ])
}

fn hid_claim_env_enabled() -> bool {
    matches!(
        std::env::var("MICROBRIDGE_HID_CLAIM").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn color_for_state(config: &DaemonConfig, state: AgentState) -> Option<u32> {
    let hex = match state {
        AgentState::Idle => &config.state_colors.idle,
        AgentState::Thinking => &config.state_colors.thinking,
        AgentState::Working => &config.state_colors.working,
        AgentState::AwaitingApproval => &config.state_colors.awaiting_approval,
        AgentState::Done => &config.state_colors.done,
        AgentState::Error => &config.state_colors.error,
    };
    parse_rgb_hex(hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mb_device::MockDevice;

    fn session(id: &str, state: AgentState) -> SessionStatus {
        SessionStatus {
            id: id.into(),
            app: "Cursor".into(),
            title: "Test thread".into(),
            state,
            updated_at_ms: 1,
        }
    }

    fn state() -> DaemonState {
        DaemonState::new(Box::<MockDevice>::default(), DaemonConfig::default())
    }

    #[test]
    fn native_adapters_honor_disabled_config_on_restart() {
        let mut config = DaemonConfig::default();
        config.adapters.get_mut("codex").unwrap().enabled = false;
        config.adapters.get_mut("claude").unwrap().enabled = false;
        let state = DaemonState::new(Box::<MockDevice>::default(), config);
        assert_eq!(
            state.adapters["codex"].state,
            AdapterConnectionState::Disabled
        );
        assert_eq!(
            state.adapters["claude"].state,
            AdapterConnectionState::Disabled
        );
        assert!(!state.adapters["codex"].capabilities.lifecycle_observation);
        assert!(!state.adapters["claude"].capabilities.lifecycle_observation);
    }

    #[test]
    fn disabled_adapter_cannot_ingest_lifecycle() {
        let mut state = state();
        let result =
            state.ingest_lifecycle("cursor", session("cursor:one", AgentState::Working), 5_000);
        assert!(result.is_err());
        assert!(!state.registry.sessions.contains_key("cursor:one"));
    }

    #[test]
    fn lifecycle_session_must_match_declared_adapter_namespace() {
        let mut state = state();
        state.config.adapters.get_mut("cursor").unwrap().enabled = true;
        let result =
            state.ingest_lifecycle("cursor", session("t3code:one", AgentState::Working), 5_000);
        assert!(result.unwrap_err().contains("cursor:<session>"));
        assert!(!state.registry.sessions.contains_key("t3code:one"));
    }

    #[test]
    fn newly_discovered_adapter_stays_pending_until_approved() {
        let mut state = state();
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = state.register_adapter(
            24,
            "example-community".into(),
            Some("1.0.0".into()),
            AdapterCapabilities::lifecycle_only(),
            tx,
        );
        assert!(result.unwrap_err().contains("pending approval"));
        let status = &state.adapters["example-community"];
        assert_eq!(status.state, AdapterConnectionState::NeedsSetup);
        assert!(!state.config.adapters["example-community"].enabled);
        assert!(!state.adapter_txs.contains_key(&24));
    }

    #[test]
    fn route_action_rejects_unadvertised_capability() {
        let mut state = state();
        state.config.adapters.get_mut("cursor").unwrap().enabled = true;
        let (tx, _rx) = mpsc::unbounded_channel();
        state
            .register_adapter(
                42,
                "cursor".into(),
                Some("test".into()),
                AdapterCapabilities::lifecycle_only(),
                tx,
            )
            .unwrap();
        state.upsert_session(session("cursor:one", AgentState::AwaitingApproval), 42);
        let error = state
            .route_action("cursor:one", Action::Approve)
            .unwrap_err();
        assert!(error.contains("does not support"));
    }

    #[test]
    fn resolved_approval_cannot_be_routed() {
        let mut state = state();
        state.upsert_session(
            session("t3code:one", AgentState::Working),
            T3_OWNER_FOR_TEST,
        );
        let error = state
            .route_action("t3code:one", Action::Approve)
            .unwrap_err();
        assert!(error.contains("expired or been resolved"));
    }

    #[test]
    fn lifecycle_lease_expiration_removes_session() {
        let mut state = state();
        state.config.adapters.get_mut("cursor").unwrap().enabled = true;
        state
            .ingest_lifecycle("cursor", session("cursor:one", AgentState::Done), 5_000)
            .unwrap();
        state.leased_sessions.insert(
            "cursor:one".into(),
            Instant::now() - Duration::from_millis(1),
        );
        state.expire_leased_sessions();
        assert!(!state.registry.sessions.contains_key("cursor:one"));
    }

    const T3_OWNER_FOR_TEST: u64 = 77;
}
