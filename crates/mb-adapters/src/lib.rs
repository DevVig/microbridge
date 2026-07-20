//! First-party in-process adapters.
//!
//! These watch local session stores with FSEvents/inotify (via `notify`) and
//! publish transitions into the daemon bus. They never talk to the device.

mod claude;
mod codex;
mod hosts;
mod title;
mod watch;

use mb_protocol::SessionStatus;
use tokio::sync::mpsc;

pub use claude::spawn_claude_adapter;
pub use codex::spawn_codex_adapter;

/// Events emitted by in-process adapters toward the daemon bus.
#[derive(Debug, Clone)]
pub enum AdapterEvent {
    Upsert(ObservedSession),
    Remove(String),
}

/// Runtime identity used to reconcile a hosted terminal with the underlying
/// journal watcher. The host owns display/focus while it is present; the raw
/// session becomes visible again when the host claim disappears.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionContext {
    pub runtime: String,
    pub cwd: String,
}

#[derive(Debug, Clone)]
pub struct ObservedSession {
    pub session: SessionStatus,
    pub context: Option<SessionContext>,
}

pub type AdapterTx = mpsc::UnboundedSender<AdapterEvent>;
