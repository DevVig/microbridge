//! microbridged — the Microbridge daemon.
//!
//! Listens on a local Unix socket for adapter and UI messages, resolves which
//! session owns the device, and renders Agent Key LEDs. Fully event-driven:
//! the daemon does no work between messages.

use std::sync::Arc;

use mb_adapters::{spawn_claude_adapter, spawn_codex_adapter, AdapterEvent};
use mb_device::open_default_device;
use microbridged::config::load_config;
use microbridged::socket::serve;
use microbridged::state::DaemonState;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = load_config();
    let device = open_default_device();
    info!(device = %device.descriptor().name, "device layer ready");

    let shared = Arc::new(Mutex::new(DaemonState::new(device, config)));

    let (adapter_tx, mut adapter_rx) = mpsc::unbounded_channel::<AdapterEvent>();
    spawn_codex_adapter(adapter_tx.clone());
    spawn_claude_adapter(adapter_tx);

    let bus = Arc::clone(&shared);
    tokio::spawn(async move {
        while let Some(event) = adapter_rx.recv().await {
            let mut state = bus.lock().await;
            match event {
                // conn_id 0 = in-process owner
                AdapterEvent::Upsert(session) => state.upsert_session(session, 0),
                AdapterEvent::Remove(id) => state.remove_session(&id),
            }
        }
    });

    serve(shared).await
}
