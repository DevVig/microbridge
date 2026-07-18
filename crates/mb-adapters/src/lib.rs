//! First-party in-process adapters.
//!
//! These watch local session stores with FSEvents/inotify (via `notify`) and
//! publish transitions into the daemon bus. They never talk to the device.

mod claude;
mod codex;
mod title;
mod watch;

use mb_protocol::SessionStatus;
use tokio::sync::mpsc;

pub use claude::spawn_claude_adapter;
pub use codex::spawn_codex_adapter;

/// Events emitted by in-process adapters toward the daemon bus.
#[derive(Debug, Clone)]
pub enum AdapterEvent {
    Upsert(SessionStatus),
    Remove(String),
}

pub type AdapterTx = mpsc::UnboundedSender<AdapterEvent>;
