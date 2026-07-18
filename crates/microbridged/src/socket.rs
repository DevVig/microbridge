//! Unix domain socket server for adapters and UI clients.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use mb_protocol::{ClientMessage, ClientRole, ServerMessage, PROTOCOL_VERSION};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::config::socket_path;
use crate::state::{next_conn_id, SharedState};
use crate::t3code;

pub async fn serve(shared: SharedState) -> std::io::Result<()> {
    let path = socket_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        // Keep the config dir private; the socket itself is locked to 0600 below.
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    info!(
        socket = %path.display(),
        protocol = PROTOCOL_VERSION,
        "microbridged listening"
    );

    loop {
        let (stream, _) = listener.accept().await?;
        let shared = Arc::clone(&shared);
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, shared).await {
                warn!(%error, "connection closed with error");
            }
        });
    }
}

async fn handle_connection(stream: UnixStream, shared: SharedState) -> std::io::Result<()> {
    let conn_id = next_conn_id();
    let (read_half, mut write_half) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let Ok(line) = serde_json::to_string(&msg) else {
                continue;
            };
            if write_half.write_all(line.as_bytes()).await.is_err() {
                break;
            }
            if write_half.write_all(b"\n").await.is_err() {
                break;
            }
        }
    });

    let mut lines = BufReader::new(read_half).lines();
    let mut role = ClientRole::Adapter;
    let mut named = false;

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let message = match serde_json::from_str::<ClientMessage>(&line) {
            Ok(m) => m,
            Err(error) => {
                warn!(%error, "dropping unparseable message");
                continue;
            }
        };
        apply(message, conn_id, &tx, &mut role, &mut named, &shared).await;
    }

    {
        let mut state = shared.lock().await;
        state.drop_connection(conn_id);
    }
    drop(tx);
    let _ = writer.await;
    Ok(())
}

async fn apply(
    message: ClientMessage,
    conn_id: u64,
    tx: &mpsc::UnboundedSender<ServerMessage>,
    role: &mut ClientRole,
    named: &mut bool,
    shared: &SharedState,
) {
    match message {
        ClientMessage::Hello {
            adapter,
            protocol_version,
            role: hello_role,
            adapter_version,
            capabilities,
        } => {
            if *named {
                warn!(conn_id, "duplicate hello ignored");
                return;
            }
            if protocol_version != PROTOCOL_VERSION {
                warn!(
                    adapter,
                    protocol_version, "client speaks a different protocol revision"
                );
                let _ = tx.send(ServerMessage::AdapterOperation {
                    adapter_id: adapter,
                    ok: false,
                    message: format!(
                        "Protocol {protocol_version} is incompatible with Microbridge protocol {PROTOCOL_VERSION}."
                    ),
                });
                return;
            }
            *role = hello_role;
            *named = true;
            let mut state = shared.lock().await;
            match hello_role {
                ClientRole::Adapter => {
                    if let Err(error) = state.register_adapter(
                        conn_id,
                        adapter.clone(),
                        adapter_version,
                        capabilities,
                        tx.clone(),
                    ) {
                        *named = false;
                        warn!(adapter, %error, "adapter connection requires consent or setup");
                        let _ = tx.send(ServerMessage::AdapterOperation {
                            adapter_id: adapter.clone(),
                            ok: false,
                            message: error,
                        });
                    }
                }
                ClientRole::Ui => {
                    state.ui_txs.insert(conn_id, tx.clone());
                }
            }
            info!(adapter, ?hello_role, conn_id, "client connected");
        }
        ClientMessage::Status { session } => {
            if !*named || *role != ClientRole::Adapter {
                warn!(
                    conn_id,
                    "adapter status without a completed adapter handshake; ignoring"
                );
                return;
            }
            let mut state = shared.lock().await;
            state.upsert_session(session, conn_id);
        }
        ClientMessage::Bye { session_id } => {
            if !*named || *role != ClientRole::Adapter {
                warn!(
                    conn_id,
                    "adapter bye without a completed adapter handshake; ignoring"
                );
                return;
            }
            let mut state = shared.lock().await;
            state.remove_session(&session_id);
        }
        ClientMessage::Subscribe => {
            if !*named || *role != ClientRole::Ui {
                warn!(
                    conn_id,
                    "subscribe without a completed UI handshake; ignoring"
                );
                return;
            }
            let state = shared.lock().await;
            let snap = state.snapshot();
            let _ = tx.send(ServerMessage::Snapshot { snapshot: snap });
        }
        ClientMessage::GetConfig => {
            if !*named || *role != ClientRole::Ui {
                warn!(
                    conn_id,
                    "get_config without a completed UI handshake; ignoring"
                );
                return;
            }
            let state = shared.lock().await;
            let _ = tx.send(ServerMessage::Config {
                config: state.config.clone(),
            });
        }
        ClientMessage::SetConfig { config } => {
            if !*named || *role != ClientRole::Ui {
                let _ = tx.send(ServerMessage::ConfigError {
                    message: "set_config requires a completed UI handshake".into(),
                });
                return;
            }
            let mut state = shared.lock().await;
            match state.set_config(config) {
                Ok(()) => {
                    let _ = tx.send(ServerMessage::Config {
                        config: state.config.clone(),
                    });
                }
                Err(message) => {
                    let _ = tx.send(ServerMessage::ConfigError { message });
                }
            }
        }
        ClientMessage::SetAdapterEnabled {
            adapter_id,
            enabled,
        } => {
            if !*named || *role != ClientRole::Ui {
                let _ = tx.send(ServerMessage::AdapterOperation {
                    adapter_id,
                    ok: false,
                    message: "Adapter consent changes require a completed UI handshake.".into(),
                });
                return;
            }
            let mut state = shared.lock().await;
            let result = state.set_adapter_enabled(&adapter_id, enabled);
            let _ = tx.send(ServerMessage::AdapterOperation {
                adapter_id,
                ok: result.is_ok(),
                message: result.err().unwrap_or_else(|| {
                    if enabled {
                        "Integration enabled."
                    } else {
                        "Integration disabled."
                    }
                    .into()
                }),
            });
        }
        ClientMessage::PairAdapter {
            adapter_id,
            pairing_url,
        } => {
            if !*named || *role != ClientRole::Ui {
                let _ = tx.send(ServerMessage::AdapterOperation {
                    adapter_id,
                    ok: false,
                    message: "Adapter pairing requires a completed UI handshake.".into(),
                });
                return;
            }
            let allowed =
                adapter_id == "t3code" && shared.lock().await.adapter_enabled(&adapter_id);
            let result = if adapter_id != "t3code" {
                Err("This adapter does not use pairing links.".to_string())
            } else if !allowed {
                Err("Enable T3 Code before pairing it.".to_string())
            } else {
                t3code::pair(&pairing_url)
                    .await
                    .map(|_| "Pairing accepted. Microbridge is connecting to T3 Code.".to_string())
            };
            if result.is_ok() {
                shared.lock().await.set_adapter_runtime(
                    "t3code",
                    mb_protocol::AdapterConnectionState::Connecting,
                    t3code::capabilities(),
                    "Pairing accepted. Loading T3 Code threads…",
                );
            }
            let _ = tx.send(ServerMessage::AdapterOperation {
                adapter_id,
                ok: result.is_ok(),
                message: result.unwrap_or_else(|error| error),
            });
        }
        ClientMessage::ForgetAdapter { adapter_id } => {
            if !*named || *role != ClientRole::Ui {
                let _ = tx.send(ServerMessage::AdapterOperation {
                    adapter_id,
                    ok: false,
                    message: "Removing an adapter requires a completed UI handshake.".into(),
                });
                return;
            }
            let disabled = {
                let mut state = shared.lock().await;
                state.set_adapter_enabled(&adapter_id, false)
            };
            let result = disabled.and_then(|_| {
                if adapter_id == "t3code" {
                    t3code::forget_credential()
                } else {
                    Ok(())
                }
            });
            let _ = tx.send(ServerMessage::AdapterOperation {
                adapter_id,
                ok: result.is_ok(),
                message: result
                    .err()
                    .unwrap_or_else(|| "Integration removed.".into()),
            });
        }
        ClientMessage::IngestLifecycle {
            adapter_id,
            session,
            ttl_ms,
        } => {
            if !*named || *role != ClientRole::Ui {
                let _ = tx.send(ServerMessage::AdapterOperation {
                    adapter_id,
                    ok: false,
                    message: "Lifecycle hook ingestion requires a completed UI handshake.".into(),
                });
                return;
            }
            let mut state = shared.lock().await;
            let result = state.ingest_lifecycle(&adapter_id, session, ttl_ms);
            let _ = tx.send(ServerMessage::AdapterOperation {
                adapter_id,
                ok: result.is_ok(),
                message: result
                    .err()
                    .unwrap_or_else(|| "Lifecycle event accepted.".into()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mb_device::MockDevice;
    use mb_protocol::{AdapterCapabilities, DaemonConfig};

    fn state() -> SharedState {
        Arc::new(tokio::sync::Mutex::new(crate::state::DaemonState::new(
            Box::<MockDevice>::default(),
            DaemonConfig::default(),
        )))
    }

    #[tokio::test]
    async fn adapter_role_cannot_change_consent() {
        let shared = state();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut role = ClientRole::Adapter;
        let mut named = true;
        apply(
            ClientMessage::SetAdapterEnabled {
                adapter_id: "cursor".into(),
                enabled: true,
            },
            9,
            &tx,
            &mut role,
            &mut named,
            &shared,
        )
        .await;
        let ServerMessage::AdapterOperation { ok, message, .. } = rx.recv().await.unwrap() else {
            panic!("expected adapter-operation rejection");
        };
        assert!(!ok);
        assert!(message.contains("UI handshake"));
        assert!(!shared.lock().await.adapter_enabled("cursor"));
    }

    #[tokio::test]
    async fn incompatible_ui_handshake_gets_no_control_channel() {
        let shared = state();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut role = ClientRole::Adapter;
        let mut named = false;
        apply(
            ClientMessage::Hello {
                adapter: "test-ui".into(),
                protocol_version: PROTOCOL_VERSION + 1,
                role: ClientRole::Ui,
                adapter_version: Some("test".into()),
                capabilities: AdapterCapabilities::default(),
            },
            10,
            &tx,
            &mut role,
            &mut named,
            &shared,
        )
        .await;
        let ServerMessage::AdapterOperation { ok, message, .. } = rx.recv().await.unwrap() else {
            panic!("expected protocol rejection");
        };
        assert!(!ok);
        assert!(message.contains("incompatible"));
        assert!(!named);
        assert!(!shared.lock().await.ui_txs.contains_key(&10));
    }
}
