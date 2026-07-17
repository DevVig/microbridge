//! microbridged — the Microbridge daemon.
//!
//! Listens on a local Unix socket for adapter status messages, resolves which
//! session owns the device, and renders that session's state. Fully
//! event-driven: the daemon does no work between messages.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{info, warn};

use mb_device::{Device, MockDevice};
use mb_protocol::{AgentState, Message, SessionStatus, PROTOCOL_VERSION};

#[derive(Default)]
struct Registry {
    sessions: HashMap<String, SessionStatus>,
    focused: Option<String>,
}

impl Registry {
    fn upsert(&mut self, session: SessionStatus) {
        self.sessions.insert(session.id.clone(), session);
        self.resolve_focus();
    }

    fn remove(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
        self.resolve_focus();
    }

    /// Focus policy v0: approval requests preempt; otherwise the current focus
    /// keeps the deck while its session lives; otherwise the most recently
    /// updated session wins. Frontmost-app tracking arrives in M3.
    fn resolve_focus(&mut self) {
        let approval = self
            .sessions
            .values()
            .filter(|s| s.state == AgentState::AwaitingApproval)
            .max_by_key(|s| s.updated_at_ms);
        if let Some(session) = approval {
            self.focused = Some(session.id.clone());
            return;
        }
        if let Some(id) = &self.focused {
            if self.sessions.contains_key(id) {
                return;
            }
        }
        self.focused = self
            .sessions
            .values()
            .max_by_key(|s| s.updated_at_ms)
            .map(|s| s.id.clone());
    }

    fn focused_state(&self) -> Option<AgentState> {
        self.focused
            .as_ref()
            .and_then(|id| self.sessions.get(id))
            .map(|s| s.state)
    }
}

struct State {
    registry: Registry,
    device: Box<dyn Device>,
}

impl State {
    fn render(&mut self) {
        self.device.set_state(self.registry.focused_state());
    }
}

type Shared = Arc<Mutex<State>>;

fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("MICROBRIDGE_SOCKET") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".microbridge")
        .join("microbridged.sock")
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let path = socket_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // A previous run may have left its socket file behind; clear it so bind
    // succeeds. Running two daemons is unsupported (last one wins the path).
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    info!(socket = %path.display(), protocol = PROTOCOL_VERSION, "microbridged listening");

    let shared: Shared = Arc::new(Mutex::new(State {
        registry: Registry::default(),
        device: Box::new(MockDevice::default()),
    }));

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(handle_connection(stream, Arc::clone(&shared)));
    }
}

async fn handle_connection(stream: UnixStream, shared: Shared) {
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Message>(&line) {
            Ok(message) => apply(message, &shared).await,
            Err(error) => warn!(%error, "dropping unparseable message"),
        }
    }
}

async fn apply(message: Message, shared: &Shared) {
    let mut state = shared.lock().await;
    match message {
        Message::Hello {
            adapter,
            protocol_version,
        } => {
            if protocol_version == PROTOCOL_VERSION {
                info!(adapter, "adapter connected");
            } else {
                warn!(
                    adapter,
                    protocol_version, "adapter speaks a different protocol revision"
                );
            }
        }
        Message::Status { session } => {
            state.registry.upsert(session);
            state.render();
        }
        Message::Bye { session_id } => {
            state.registry.remove(&session_id);
            state.render();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, state: AgentState, at: u64) -> SessionStatus {
        SessionStatus {
            id: id.into(),
            app: "test".into(),
            title: String::new(),
            state,
            updated_at_ms: at,
        }
    }

    #[test]
    fn most_recent_session_gets_initial_focus() {
        let mut registry = Registry::default();
        registry.upsert(session("a", AgentState::Working, 1));
        registry.upsert(session("b", AgentState::Thinking, 2));
        // "a" already held focus and still exists, so it keeps the deck.
        assert_eq!(registry.focused.as_deref(), Some("a"));
    }

    #[test]
    fn approval_preempts_and_releases() {
        let mut registry = Registry::default();
        registry.upsert(session("a", AgentState::Working, 1));
        registry.upsert(session("b", AgentState::AwaitingApproval, 2));
        assert_eq!(registry.focused.as_deref(), Some("b"));

        // Approval resolved: focus stays where the user just acted.
        registry.upsert(session("b", AgentState::Working, 3));
        assert_eq!(registry.focused.as_deref(), Some("b"));

        registry.remove("b");
        assert_eq!(registry.focused.as_deref(), Some("a"));
    }

    #[test]
    fn empty_registry_clears_the_deck() {
        let mut registry = Registry::default();
        registry.upsert(session("a", AgentState::Done, 1));
        registry.remove("a");
        assert_eq!(registry.focused, None);
        assert_eq!(registry.focused_state(), None);
    }
}
