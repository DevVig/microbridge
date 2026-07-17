//! Shared daemon state: registry, config, device, subscribers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use mb_device::{Device, LedFrame};
use mb_protocol::{
    Action, BusEvent, DaemonConfig, ServerMessage, SessionStatus, Snapshot, AGENT_KEY_COUNT,
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
    last_leds: LedFrame,
}

impl DaemonState {
    pub fn new(device: Box<dyn Device>, config: DaemonConfig) -> Self {
        Self {
            registry: Registry::default(),
            config,
            device,
            adapter_txs: HashMap::new(),
            ui_txs: HashMap::new(),
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
        self.registry.remove_owner(conn_id, &self.config);
        self.after_bus_change(prev_focus);
    }

    pub fn set_config(&mut self, config: DaemonConfig) {
        let prev_focus = self.registry.focused.clone();
        // `frontmost_app` is watcher-owned runtime state — clients cannot set it.
        let frontmost = self.config.frontmost_app.clone();
        self.config = config;
        self.config.frontmost_app = frontmost;
        if let Err(error) = save_config(&self.config) {
            warn!(%error, "failed to persist config");
        }
        self.registry.resolve_focus(&self.config);
        self.broadcast_ui(BusEvent::ConfigChanged {
            config: self.config.clone(),
        });
        self.after_bus_change(prev_focus);
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
            config: self.config.clone(),
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
            focus_index: None,
            brightness: self.config.brightness,
            paused: self.config.pause_leds,
        };
        for (i, id) in keys.iter().enumerate() {
            frame.keys[i] = id
                .as_ref()
                .and_then(|sid| self.registry.sessions.get(sid))
                .map(|s| s.state);
            if id.as_ref() == self.registry.focused.as_ref() {
                frame.focus_index = Some(i);
            }
        }
        if frame != self.last_leds {
            self.device.set_leds(&frame);
            self.last_leds = frame;
        }
    }

    pub fn route_action(&self, session_id: &str, action: Action) {
        let Some(owner) = self.registry.owner_of(session_id) else {
            warn!(session_id, ?action, "no adapter owns session");
            return;
        };
        let Some(tx) = self.adapter_txs.get(&owner) else {
            // In-process adapters: owner id 0 is reserved for local handlers.
            if owner == 0 {
                info!(session_id, ?action, "in-process action");
                return;
            }
            warn!(session_id, ?action, owner, "adapter connection gone");
            return;
        };
        let _ = tx.send(ServerMessage::Action {
            session_id: session_id.to_string(),
            action,
        });
    }

    pub fn handle_device_action(&mut self, action: Action) {
        if let Some(id) = self.registry.focused.clone() {
            self.route_action(&id, action);
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

    pub fn send_to_ui(&self, conn_id: u64, msg: ServerMessage) {
        if let Some(tx) = self.ui_txs.get(&conn_id) {
            let _ = tx.send(msg);
        }
    }
}
