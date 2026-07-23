//! Shared daemon state: registry, config, device, subscribers.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mb_adapters::{ObservedSession, SessionContext};
use mb_device::{parse_rgb_hex, Device, LedFrame};
use mb_protocol::{
    Action, AdapterCapabilities, AdapterConnectionState, AdapterKind, AdapterStatus, AgentKeyLed,
    AgentKeyLedFrame, AgentState, BusEvent, DaemonConfig, ServerMessage, SessionStatus, Snapshot,
    AGENT_KEY_COUNT,
};
use tokio::sync::{mpsc, Mutex};
use tracing::warn;

use crate::config::save_config;
use crate::registry::Registry;

pub type SharedState = Arc<Mutex<DaemonState>>;

static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

pub fn next_conn_id() -> u64 {
    NEXT_CONN.fetch_add(1, Ordering::Relaxed)
}

fn should_reopen_device(previous: &DaemonConfig, next: &DaemonConfig, connected: bool) -> bool {
    next.hardware_control_enabled != previous.hardware_control_enabled
        || (next.hardware_control_enabled && !connected)
}

fn unrendered_led_frame() -> LedFrame {
    LedFrame {
        // Normalized user brightness is at most 100, so this sentinel forces
        // one real HID write even when the resolved frame is otherwise empty.
        brightness: u8::MAX,
        ..LedFrame::default()
    }
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
    /// Journal observations remain cached while a native host such as CNVS
    /// claims the same runtime + working directory. This makes the host card
    /// authoritative without losing the raw session when the host closes.
    observed_sessions: HashMap<String, ObservedSession>,
    /// Owner conn_id recorded when each observation was upserted (control planes
    /// must survive hosted reconcile, which must not force owner `0`).
    observed_session_owners: HashMap<String, u64>,
    hosted_claims: HashMap<u64, HashSet<SessionContext>>,
    last_agent_key_press: [Option<Instant>; AGENT_KEY_COUNT],
    last_leds: LedFrame,
}

impl DaemonState {
    pub fn new(device: Box<dyn Device>, mut config: DaemonConfig) -> Self {
        config.normalize();
        let adapters = initial_adapter_statuses(&config);
        let mut state = Self {
            registry: Registry::default(),
            config,
            device,
            adapter_txs: HashMap::new(),
            ui_txs: HashMap::new(),
            adapter_connections: HashMap::new(),
            adapter_capabilities: HashMap::new(),
            adapters,
            leased_sessions: HashMap::new(),
            observed_sessions: HashMap::new(),
            observed_session_owners: HashMap::new(),
            hosted_claims: HashMap::new(),
            last_agent_key_press: [None; AGENT_KEY_COUNT],
            last_leds: unrendered_led_frame(),
        };
        let keys = state.registry.agent_key_ids(&state.config);
        state.render_leds(&keys);
        state
    }

    pub fn snapshot(&self) -> Snapshot {
        let desc = self.device.descriptor();
        let keys = self.registry.agent_key_ids(&self.config);
        let led_frame = self.resolved_led_frame(&keys);
        Snapshot {
            sessions: self.registry.session_list(),
            focused_session_id: self.registry.focused.clone(),
            agent_key_session_ids: keys.to_vec(),
            agent_key_led_frame: AgentKeyLedFrame {
                keys: keys
                    .iter()
                    .enumerate()
                    .map(|(index, session_id)| AgentKeyLed {
                        session_id: session_id.clone(),
                        state: led_frame.keys[index],
                        color: led_frame.key_colors[index].map(|color| format!("#{color:06X}")),
                        focused: led_frame.focus_index == Some(index),
                    })
                    .collect(),
                brightness: led_frame.brightness,
                paused: led_frame.paused,
            },
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

    pub fn upsert_observed_session(&mut self, mut observed: ObservedSession, owner: u64) {
        if let Some(context) = observed.context.as_mut() {
            context.cwd = normalize_cwd(&context.cwd);
        }
        let id = observed.session.id.clone();
        self.observed_sessions.insert(id.clone(), observed.clone());
        self.observed_session_owners.insert(id.clone(), owner);
        if self.observation_is_hosted(&observed) {
            if self.registry.sessions.contains_key(&id) {
                self.remove_session(&id);
            }
        } else if self.registry.sessions.get(&id) != Some(&observed.session)
            || self.registry.owner_of(&id) != Some(owner)
        {
            self.upsert_session(observed.session, owner);
        }
        // Transcript watch is Claude-parity for Cursor IDE — promote the tile when
        // hooks miss events but JSONL activity is flowing.
        if id.starts_with("cursor:") && self.adapter_enabled("cursor") {
            self.set_adapter_runtime(
                "cursor",
                AdapterConnectionState::Connected,
                AdapterCapabilities::lifecycle_only(),
                "Lifecycle connected. Cursor IDE does not expose approve/interrupt APIs yet.",
            );
        }
    }

    pub fn remove_observed_session(&mut self, id: &str) {
        self.observed_sessions.remove(id);
        self.observed_session_owners.remove(id);
        self.remove_session(id);
    }

    /// Replace one native host's full terminal snapshot atomically. CNVS node
    /// ids are the user-facing sessions; matching raw Codex/Claude journals
    /// stay cached and return if the host terminal disappears.
    pub fn replace_hosted_sessions(
        &mut self,
        owner: u64,
        sessions: Vec<(SessionStatus, SessionContext)>,
    ) {
        let claims = sessions
            .iter()
            .map(|(_, context)| SessionContext {
                runtime: context.runtime.clone(),
                cwd: normalize_cwd(&context.cwd),
            })
            .collect::<HashSet<_>>();
        self.hosted_claims.insert(owner, claims);

        let next_ids = sessions
            .iter()
            .map(|(session, _)| session.id.clone())
            .collect::<HashSet<_>>();
        let removed = self
            .registry
            .owners
            .iter()
            .filter_map(|(id, session_owner)| {
                (*session_owner == owner && !next_ids.contains(id)).then_some(id.clone())
            })
            .collect::<Vec<_>>();
        for id in removed {
            self.remove_session(&id);
        }

        for (mut session, _) in sessions {
            if let Some(existing) = self.registry.sessions.get(&session.id) {
                if existing.app == session.app
                    && existing.title == session.title
                    && existing.state == session.state
                {
                    session.updated_at_ms = existing.updated_at_ms;
                }
            }
            if self.registry.sessions.get(&session.id) != Some(&session)
                || self.registry.owner_of(&session.id) != Some(owner)
            {
                self.upsert_session(session, owner);
            }
        }
        self.reconcile_observed_sessions();
    }

    fn observation_is_hosted(&self, observed: &ObservedSession) -> bool {
        let Some(context) = observed.context.as_ref() else {
            return false;
        };
        if !matches!(context.runtime.as_str(), "codex" | "claude") {
            return false;
        }
        // A stable host attribution from another app must never be hidden just
        // because that app happens to use the same runtime and working tree.
        if !matches!(
            observed.session.app.as_str(),
            "Codex CLI" | "Claude Code" | "Claude Agent SDK" | "CNVS"
        ) {
            return false;
        }
        let normalized = SessionContext {
            runtime: context.runtime.clone(),
            cwd: normalize_cwd(&context.cwd),
        };
        self.hosted_claims
            .values()
            .any(|claims| claims.contains(&normalized))
    }

    fn reconcile_observed_sessions(&mut self) {
        let observations = self
            .observed_sessions
            .iter()
            .map(|(id, observed)| {
                (
                    id.clone(),
                    observed.clone(),
                    self.observed_session_owners.get(id).copied().unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        for (id, observed, owner) in observations {
            if self.observation_is_hosted(&observed) {
                if self.registry.sessions.contains_key(&id) {
                    self.remove_session(&id);
                }
            } else if self.registry.sessions.get(&id) != Some(&observed.session)
                || self.registry.owner_of(&id) != Some(owner)
            {
                self.upsert_session(observed.session, owner);
            }
        }
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
        // Re-sending enabled while disconnected is an explicit retry. The
        // config bit records consent/intent; the descriptor is the only proof
        // that the HID interface was actually claimed.
        let reopen_device =
            should_reopen_device(&self.config, &config, self.device.descriptor().connected);
        // `frontmost_app` is watcher-owned runtime state — clients cannot set it.
        let frontmost = self.config.frontmost_app.clone();
        config.normalize();
        config.frontmost_app = frontmost;
        save_config(&config).map_err(|error| format!("failed to persist config: {error}"))?;

        let prev_focus = self.registry.focused.clone();
        self.config = config;
        if reopen_device {
            self.device = mb_device::open_default_device_with_claim(
                self.config.hardware_control_enabled || hid_claim_env_enabled(),
            );
            self.last_leds = unrendered_led_frame();
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
                // Lifecycle-only Cursor IDE hooks are Connected — same observation
                // ceiling as Claude Code. Approve/interrupt stay off the capability chips.
                if capabilities.lifecycle_observation {
                    AdapterConnectionState::Connected
                } else {
                    AdapterConnectionState::Limited
                }
            } else {
                AdapterConnectionState::Connected
            };
        status.version = version;
        status.last_activity_ms = Some(now_ms());
        status.capabilities = capabilities.clone();
        status.diagnostic = if adapter_id == "cursor"
            && !capabilities.approval_acceptance
            && capabilities.lifecycle_observation
        {
            "Lifecycle connected. Cursor IDE does not expose approve/interrupt APIs yet.".into()
        } else if status.state == AdapterConnectionState::Limited {
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
        let internal_owner = self.adapter_connections.iter().find_map(|(owner, id)| {
            (id == adapter_id && *owner >= u64::MAX - 1024).then_some(*owner)
        });
        let capabilities = internal_owner
            .and_then(|owner| self.adapter_capabilities.get(&owner).cloned())
            .unwrap_or_else(AdapterCapabilities::lifecycle_only);
        let status = self
            .adapters
            .get_mut(adapter_id)
            .ok_or_else(|| format!("unknown adapter: {adapter_id}"))?;
        status.state = if internal_owner.is_some() || adapter_id == "cursor" {
            AdapterConnectionState::Connected
        } else {
            AdapterConnectionState::Limited
        };
        status.capabilities = capabilities;
        status.last_activity_ms = Some(now_ms());
        status.diagnostic = if adapter_id == "factory" {
            "Factory lifecycle, interrupt, and model-aware reasoning effort are connected through official hooks and JSON-RPC.".into()
        } else if adapter_id == "cursor" {
            "Lifecycle connected. Cursor IDE does not expose approve/interrupt APIs yet.".into()
        } else {
            "Lifecycle is connected; unsupported IDE commands remain disabled.".into()
        };
        let id = session.id.clone();
        self.upsert_session(session, internal_owner.unwrap_or(0));
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

    pub fn set_internal_capabilities(&mut self, owner: u64, capabilities: AdapterCapabilities) {
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
        self.hosted_claims.remove(&owner);
        self.reconcile_observed_sessions();
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
        let led_frame = self.resolved_led_frame(&keys);
        self.broadcast_ui(BusEvent::AgentKeysChanged {
            session_ids: keys.clone().into_iter().collect(),
            led_frame: AgentKeyLedFrame {
                keys: keys
                    .iter()
                    .enumerate()
                    .map(|(index, session_id)| AgentKeyLed {
                        session_id: session_id.clone(),
                        state: led_frame.keys[index],
                        color: led_frame.key_colors[index].map(|color| format!("#{color:06X}")),
                        focused: led_frame.focus_index == Some(index),
                    })
                    .collect(),
                brightness: led_frame.brightness,
                paused: led_frame.paused,
            },
        });
        self.render_leds(&keys);
    }

    pub fn render_leds(&mut self, keys: &[Option<String>; AGENT_KEY_COUNT]) {
        let frame = self.resolved_led_frame(keys);
        if frame != self.last_leds {
            let before = self.device.descriptor();
            self.device.set_leds(&frame);
            let after = self.device.descriptor();
            if before.connected != after.connected || before.name != after.name {
                self.broadcast_ui(BusEvent::DeviceChanged {
                    connected: after.connected,
                    name: after.name,
                });
            }
            self.last_leds = frame;
        }
    }

    fn resolved_led_frame(&self, keys: &[Option<String>; AGENT_KEY_COUNT]) -> LedFrame {
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
        frame
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
        if action == Action::OpenFocusedThread {
            let uri: Option<String> = session.focus_uri.clone().or_else(|| {
                let cwd = session.id.split(':').nth(1)?;
                // Only synthesize file://-style deep links from absolute paths.
                // Conversation IDs and other opaque segments must not become `cursor://file…`.
                if !cwd.starts_with('/') {
                    return None;
                }
                match session.app.as_str() {
                    "Cursor" => Some(format!("cursor://file{}", cwd)),
                    "VS Code" => Some(format!("vscode://file{}", cwd)),
                    "Zed" => Some(format!("zed://file{}", cwd)),
                    "Windsurf" => Some(format!("windsurf://file{}", cwd)),
                    _ => None,
                }
            });
            if let Some(url) = uri {
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open").arg(&url).spawn();
                    tracing::info!(url = %url, session_id, "Launched deep-link focus URI");
                    return Ok(());
                }
                #[cfg(not(target_os = "macos"))]
                {
                    return Err(format!(
                        "Opening the focused thread via deep link is only supported on macOS ({url})."
                    ));
                }
            }
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
            // Owner 0 = journal FS watchers. Never report silent success — either a
            // dedicated control owner owns the session, or the lever is unsupported.
            if owner == 0 {
                warn!(
                    session_id,
                    ?action,
                    "journal-observed session has no control plane for this action"
                );
                return Err(format!(
                    "This thread is lifecycle-only — {action:?} is not available until a control plane is attached."
                ));
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
                let _ = self.focus_agent_key(index);
                if double {
                    self.handle_device_action(Action::OpenFocusedThread);
                }
            }
            DeviceInput::AgentKeyDoublePress { index } => {
                let _ = self.focus_agent_key(index);
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

    /// Refresh physical presence without repeatedly retrying a failed claim.
    /// The 2-second caller cadence is only for hot-plug discovery; stable
    /// detected/connected devices keep their existing handle.
    pub fn refresh_device_presence(&mut self) {
        self.refresh_device_presence_with(mb_device::open_default_device_with_claim);
    }

    fn refresh_device_presence_with<F>(&mut self, mut open: F)
    where
        F: FnMut(bool) -> Box<dyn Device>,
    {
        let observed_device = open(false);
        let observed = observed_device.descriptor();
        let Some((expected_name, should_claim)) = self.plan_device_presence_refresh(&observed)
        else {
            return;
        };
        let claimed_device = should_claim.then(|| open(true));
        self.apply_device_presence_refresh(&expected_name, observed_device, claimed_device);
    }

    /// Decide whether a separately probed descriptor represents a hot-plug or
    /// transport change. The caller may perform the potentially blocking HID
    /// probe and claim without holding the daemon state mutex.
    pub fn plan_device_presence_refresh(
        &self,
        observed: &mb_device::DeviceDescriptor,
    ) -> Option<(String, bool)> {
        let current = self.device.descriptor();
        (current.name != observed.name).then(|| {
            (
                current.name,
                observed.name != "mock" && self.config.hardware_control_enabled,
            )
        })
    }

    /// Apply devices opened outside the daemon state mutex. If state or the
    /// physical transport changed while I/O was in flight, defer to the next
    /// bounded refresh instead of overwriting newer state.
    pub fn apply_device_presence_refresh(
        &mut self,
        expected_current_name: &str,
        observed_device: Box<dyn Device>,
        claimed_device: Option<Box<dyn Device>>,
    ) {
        if self.device.descriptor().name != expected_current_name {
            return;
        }
        let observed = observed_device.descriptor();
        if observed.name == expected_current_name {
            return;
        }

        let replacement = if observed.name != "mock" && self.config.hardware_control_enabled {
            let Some(claimed) = claimed_device else {
                return;
            };
            if claimed.descriptor().name != observed.name {
                return;
            }
            claimed
        } else {
            observed_device
        };
        let before_render = replacement.descriptor();
        self.device = replacement;
        self.last_leds = unrendered_led_frame();
        self.broadcast_ui(BusEvent::DeviceChanged {
            connected: before_render.connected,
            name: before_render.name,
        });
        let keys = self.registry.agent_key_ids(&self.config);
        self.render_leds(&keys);
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
        self.after_bus_change(previous);
    }

    pub fn focus_agent_key(&mut self, index: usize) -> Result<String, String> {
        let keys = self.registry.agent_key_ids(&self.config);
        let id = keys
            .get(index)
            .and_then(Clone::clone)
            .ok_or_else(|| format!("Agent Key {} is not assigned to a live thread.", index + 1))?;
        let prev = self.registry.focused.clone();
        self.registry.focused = Some(id.clone());
        self.after_bus_change(prev);
        Ok(id)
    }

    pub fn activate_agent_key(&mut self, index: usize, open: bool) -> Result<String, String> {
        let session_id = self.focus_agent_key(index)?;
        if open {
            self.route_action(&session_id, Action::OpenFocusedThread)?;
            Ok(format!("Opened Agent Key {} thread.", index + 1))
        } else {
            Ok(format!("Focused Agent Key {} thread.", index + 1))
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

fn normalize_cwd(cwd: &str) -> String {
    let trimmed = cwd.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.into()
    }
}

fn setup_diagnostic(adapter_id: &str) -> String {
    match adapter_id {
        "cursor" => {
            "The bundled Cursor integration is installed. Reload Cursor once if it is already open."
                .into()
        }
        "cursor_acp" => {
            "Install the Cursor CLI (`agent` / `cursor-agent`) so Microbridge can drive ACP sessions."
                .into()
        }
        "t3code" => "Paste a one-time pairing link from T3 Code Settings → Connections.".into(),
        "factory" => "Enable the bundled Factory lifecycle hooks to connect Droid sessions.".into(),
        "opencode" => {
            "The bundled OpenCode integration is installed. Restart OpenCode once if it is already running."
                .into()
        }
        "cnvs" => {
            "Open CNVS; Microbridge connects to its authenticated local control API automatically."
                .into()
        }
        _ => "Waiting for the adapter to connect.".into(),
    }
}

fn reconnect_diagnostic(adapter_id: &str) -> String {
    match adapter_id {
        "cursor" => "Cursor is enabled, but no lifecycle event has arrived yet. Reload Cursor if it was already open.".into(),
        "cursor_acp" => "Cursor ACP is enabled, but the Cursor CLI is not on PATH yet.".into(),
        "t3code" => "T3 Code is enabled, but its paired connection is offline.".into(),
        "factory" => "Factory is enabled, but no Droid lifecycle event has arrived yet.".into(),
        "opencode" => "OpenCode is enabled, but its integration has not connected yet. Restart OpenCode if it was already running.".into(),
        "cnvs" => "CNVS is enabled and will reconnect automatically when the app is running.".into(),
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
        (
            "claude_desktop".into(),
            native("claude_desktop", "Claude Desktop"),
        ),
        ("codex".into(), native("codex", "Codex CLI")),
        ("chatgpt".into(), native("chatgpt", "ChatGPT")),
        ("synara".into(), native("synara", "Synara")),
        ("conductor".into(), native("conductor", "Conductor")),
        ("cnvs".into(), native("cnvs", "CNVS")),
        ("cursor".into(), opt_in("cursor", "Cursor")),
        (
            "cursor_acp".into(),
            opt_in("cursor_acp", "Cursor Agent (ACP)"),
        ),
        ("t3code".into(), opt_in("t3code", "T3 Code")),
        ("factory".into(), opt_in("factory", "Factory")),
        ("opencode".into(), opt_in("opencode", "OpenCode")),
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
    use mb_adapters::{ObservedSession, SessionContext};

    const CNVS_OWNER_FOR_TEST: u64 = u64::MAX - 3;
    use mb_device::{DeviceDescriptor, MockDevice};

    struct DescriptorDevice(DeviceDescriptor);

    impl Device for DescriptorDevice {
        fn descriptor(&self) -> DeviceDescriptor {
            self.0.clone()
        }

        fn set_leds(&mut self, _frame: &LedFrame) {}
    }

    fn physical(name: &str, connected: bool) -> Box<dyn Device> {
        Box::new(DescriptorDevice(DeviceDescriptor {
            name: name.into(),
            agent_key_count: AGENT_KEY_COUNT,
            has_dial: true,
            has_joystick: true,
            connected,
        }))
    }

    fn session(id: &str, state: AgentState) -> SessionStatus {
        SessionStatus {
            id: id.into(),
            app: "Cursor".into(),
            title: "Test thread".into(),
            state,
            updated_at_ms: 1,
            focus_uri: None,
        }
    }

    fn state() -> DaemonState {
        DaemonState::new(Box::<MockDevice>::default(), DaemonConfig::default())
    }

    #[test]
    fn hardware_control_retries_when_requested_but_disconnected() {
        let disabled = DaemonConfig::default();
        let mut enabled = disabled.clone();
        enabled.hardware_control_enabled = true;

        assert!(should_reopen_device(&disabled, &enabled, false));
        assert!(should_reopen_device(&enabled, &enabled, false));
        assert!(!should_reopen_device(&enabled, &enabled, true));
        assert!(should_reopen_device(&enabled, &disabled, true));
    }

    #[test]
    fn hotplug_discovers_and_claims_a_new_transport_when_requested() {
        let mut state = state();
        state.config.hardware_control_enabled = true;
        let mut calls = Vec::new();

        state.refresh_device_presence_with(|should_claim| {
            calls.push(should_claim);
            physical("codex-micro-bluetooth", should_claim)
        });

        assert_eq!(calls, vec![false, true]);
        assert_eq!(state.device.descriptor().name, "codex-micro-bluetooth");
        assert!(state.device.descriptor().connected);
    }

    #[test]
    fn stable_failed_claim_does_not_retry_until_requested() {
        let config = DaemonConfig {
            hardware_control_enabled: true,
            ..DaemonConfig::default()
        };
        let mut state = DaemonState::new(physical("codex-micro-usb", false), config);
        let mut calls = Vec::new();

        state.refresh_device_presence_with(|should_claim| {
            calls.push(should_claim);
            physical("codex-micro-usb", false)
        });

        assert_eq!(calls, vec![false]);
        assert!(!state.device.descriptor().connected);
    }

    #[test]
    fn stale_presence_result_does_not_replace_newer_device_state() {
        let mut state =
            DaemonState::new(physical("codex-micro-usb", false), DaemonConfig::default());
        let observed = physical("codex-micro-bluetooth", false);
        let (expected_name, _) = state
            .plan_device_presence_refresh(&observed.descriptor())
            .expect("transport changed");

        state.device = physical("codex-micro-hid", false);
        state.apply_device_presence_refresh(&expected_name, observed, None);

        assert_eq!(state.device.descriptor().name, "codex-micro-hid");
    }

    #[test]
    fn hosted_terminal_replaces_and_then_restores_raw_journal() {
        let mut state = state();
        let context = SessionContext {
            runtime: "codex".into(),
            cwd: "/Users/me/dev/project/".into(),
        };
        let raw = SessionStatus {
            id: "codex:thread-1".into(),
            app: "Codex CLI".into(),
            title: "Repair checkout".into(),
            state: AgentState::Working,
            updated_at_ms: 1,
            focus_uri: None,
        };
        state.upsert_observed_session(
            ObservedSession {
                session: raw.clone(),
                context: Some(context.clone()),
            },
            0,
        );
        assert!(state.registry.sessions.contains_key(&raw.id));

        let hosted = SessionStatus {
            id: "cnvs:canvas-1:node-1".into(),
            app: "CNVS".into(),
            title: "Project · Repair checkout · Codex".into(),
            state: AgentState::Working,
            updated_at_ms: 2,
            focus_uri: None,
        };
        state.replace_hosted_sessions(CNVS_OWNER_FOR_TEST, vec![(hosted.clone(), context)]);
        assert!(!state.registry.sessions.contains_key(&raw.id));
        assert!(state.registry.sessions.contains_key(&hosted.id));

        state.replace_hosted_sessions(CNVS_OWNER_FOR_TEST, Vec::new());
        assert!(state.registry.sessions.contains_key(&raw.id));
        assert!(!state.registry.sessions.contains_key(&hosted.id));
    }

    #[test]
    fn hosted_terminal_does_not_hide_a_different_attributed_host() {
        let mut state = state();
        let context = SessionContext {
            runtime: "codex".into(),
            cwd: "/Users/me/dev/project".into(),
        };
        let synara = SessionStatus {
            id: "codex:synara-thread".into(),
            app: "Synara".into(),
            title: "Independent Synara thread".into(),
            state: AgentState::Working,
            updated_at_ms: 1,
            focus_uri: None,
        };
        state.upsert_observed_session(
            ObservedSession {
                session: synara.clone(),
                context: Some(context.clone()),
            },
            0,
        );
        state.replace_hosted_sessions(
            CNVS_OWNER_FOR_TEST,
            vec![(
                SessionStatus {
                    id: "cnvs:canvas-1:node-1".into(),
                    app: "CNVS".into(),
                    title: "Project · Odin · Codex".into(),
                    state: AgentState::Idle,
                    updated_at_ms: 2,
                    focus_uri: None,
                },
                context,
            )],
        );
        assert!(state.registry.sessions.contains_key(&synara.id));
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
    fn route_action_rejects_owner_zero_without_control_plane() {
        let mut state = state();
        state.upsert_session(session("cursor:one", AgentState::Working), 0);
        let error = state
            .route_action("cursor:one", Action::Interrupt)
            .unwrap_err();
        assert!(
            error.contains("lifecycle-only") || error.contains("not available"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn route_action_open_focused_thread_uses_focus_uri() {
        let mut state = state();
        let mut sess = session("cursor:one", AgentState::Working);
        sess.focus_uri = Some("cursor://file/test".into());
        state.upsert_session(sess, 0);

        #[cfg(target_os = "macos")]
        {
            assert!(state
                .route_action("cursor:one", Action::OpenFocusedThread)
                .is_ok());
        }

        #[cfg(not(target_os = "macos"))]
        {
            let error = state
                .route_action("cursor:one", Action::OpenFocusedThread)
                .unwrap_err();
            assert!(error.contains("only supported on macOS"));
            assert!(error.contains("cursor://file/test"));
        }
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
        assert_eq!(
            state.adapters["cursor"].state,
            AdapterConnectionState::Connected
        );
        assert!(state.adapters["cursor"]
            .diagnostic
            .contains("Lifecycle connected"));
        state.leased_sessions.insert(
            "cursor:one".into(),
            Instant::now() - Duration::from_millis(1),
        );
        state.expire_leased_sessions();
        assert!(!state.registry.sessions.contains_key("cursor:one"));
    }

    #[test]
    fn snapshot_exposes_the_exact_effective_led_frame_and_activation() {
        let mut state = state();
        state.config.key_source = mb_protocol::KeySource::Custom;
        state.config.custom_key_ids = vec![
            "cursor:one".into(),
            "cursor:two".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        state.config.lighting_preset = mb_protocol::LightingPreset::Phosphor;
        state.config.normalize();
        state.upsert_session(session("cursor:one", AgentState::Working), 0);
        state.upsert_session(session("cursor:two", AgentState::AwaitingApproval), 0);

        let activation = state.activate_agent_key(1, false).unwrap();
        let snapshot = state.snapshot();
        assert_eq!(activation, "Focused Agent Key 2 thread.");
        assert_eq!(snapshot.focused_session_id.as_deref(), Some("cursor:two"));
        assert_eq!(
            snapshot.agent_key_led_frame.keys[0].color.as_deref(),
            Some("#FF6A00")
        );
        assert_eq!(
            snapshot.agent_key_led_frame.keys[1].color.as_deref(),
            Some("#FF3D00")
        );
        assert!(snapshot.agent_key_led_frame.keys[1].focused);
        assert!(state.activate_agent_key(5, false).is_err());
    }

    const T3_OWNER_FOR_TEST: u64 = 77;
}
