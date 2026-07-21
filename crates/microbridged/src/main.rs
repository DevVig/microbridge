//! microbridged — the Microbridge daemon.
//!
//! Listens on a local Unix socket for adapter and UI messages, resolves which
//! session owns the device, and renders Agent Key LEDs. Fully event-driven:
//! the daemon does no work between messages.

use std::sync::Arc;

use mb_adapters::{spawn_claude_adapter, spawn_codex_adapter, AdapterEvent};
use mb_device::open_default_device_with_claim;
use microbridged::cnvs::{self, CNVS_OWNER};
use microbridged::config::load_config;
use microbridged::factory::{self, FACTORY_OWNER};
use microbridged::frontmost::spawn_frontmost_watcher;
use microbridged::socket::serve;
use microbridged::state::DaemonState;
use microbridged::t3code::{self, T3_OWNER};
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
    let env_claim = matches!(
        std::env::var("MICROBRIDGE_HID_CLAIM").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    );
    let device = open_default_device_with_claim(config.hardware_control_enabled || env_claim);
    info!(device = %device.descriptor().name, "device layer ready");

    let (t3_action_tx, t3_action_rx) = mpsc::unbounded_channel();
    let (factory_action_tx, factory_action_rx) = mpsc::unbounded_channel();
    let (cnvs_action_tx, cnvs_action_rx) = mpsc::unbounded_channel();
    let mut daemon_state = DaemonState::new(device, config);
    daemon_state.install_internal_adapter(T3_OWNER, "t3code", t3code::capabilities(), t3_action_tx);
    daemon_state.install_internal_adapter(
        FACTORY_OWNER,
        "factory",
        factory::capabilities(),
        factory_action_tx,
    );
    daemon_state.install_internal_adapter(CNVS_OWNER, "cnvs", cnvs::capabilities(), cnvs_action_tx);
    let shared = Arc::new(Mutex::new(daemon_state));
    t3code::spawn(Arc::clone(&shared), t3_action_rx);
    factory::spawn(Arc::clone(&shared), factory_action_rx);
    cnvs::spawn(Arc::clone(&shared), cnvs_action_rx);
    microbridged::mcp::spawn_mcp_server(Arc::clone(&shared));
    microbridged::auto_discover::spawn_auto_discovery(Arc::clone(&shared));

    // Hardware notifications are non-blocking. This small bounded drain also
    // expires lease-backed IDE hook sessions without introducing network polling.
    let input_bus = Arc::clone(&shared);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(16));
        loop {
            interval.tick().await;
            let mut state = input_bus.lock().await;
            state.poll_device_inputs();
            state.expire_leased_sessions();
        }
    });

    let (adapter_tx, mut adapter_rx) = mpsc::unbounded_channel::<AdapterEvent>();
    spawn_codex_adapter(adapter_tx.clone());
    spawn_claude_adapter(adapter_tx);

    let bus = Arc::clone(&shared);
    tokio::spawn(async move {
        while let Some(event) = adapter_rx.recv().await {
            let mut state = bus.lock().await;
            match event {
                // conn_id 0 = in-process owner
                AdapterEvent::Upsert(observed) => {
                    let adapter_id = observed.session.id.split(':').next().unwrap_or_default();
                    if state.adapter_enabled(adapter_id) {
                        state.upsert_observed_session(observed, 0);
                    }
                }
                AdapterEvent::Remove(id) => state.remove_observed_session(&id),
            }
        }
    });

    let (frontmost_tx, mut frontmost_rx) = mpsc::unbounded_channel::<Option<String>>();
    spawn_frontmost_watcher(frontmost_tx);
    let bus_front = Arc::clone(&shared);
    tokio::spawn(async move {
        while let Some(app) = frontmost_rx.recv().await {
            let mut state = bus_front.lock().await;
            state.set_frontmost_app(app);
        }
    });

    serve(shared).await
}
