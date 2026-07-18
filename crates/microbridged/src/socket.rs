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
        } => {
            *role = hello_role;
            *named = true;
            let mut state = shared.lock().await;
            match hello_role {
                ClientRole::Adapter => {
                    state.adapter_txs.insert(conn_id, tx.clone());
                }
                ClientRole::Ui => {
                    state.ui_txs.insert(conn_id, tx.clone());
                }
            }
            if protocol_version == PROTOCOL_VERSION {
                info!(adapter, ?hello_role, conn_id, "client connected");
            } else {
                warn!(
                    adapter,
                    protocol_version, "client speaks a different protocol revision"
                );
            }
        }
        ClientMessage::Status { session } => {
            if !*named {
                warn!("status before hello; ignoring");
                return;
            }
            let mut state = shared.lock().await;
            state.upsert_session(session, conn_id);
        }
        ClientMessage::Bye { session_id } => {
            let mut state = shared.lock().await;
            state.remove_session(&session_id);
        }
        ClientMessage::Subscribe => {
            let state = shared.lock().await;
            let snap = state.snapshot();
            let _ = tx.send(ServerMessage::Snapshot { snapshot: snap });
        }
        ClientMessage::GetConfig => {
            let state = shared.lock().await;
            let _ = tx.send(ServerMessage::Config {
                config: state.config.clone(),
            });
        }
        ClientMessage::SetConfig { config } => {
            let mut state = shared.lock().await;
            state.set_config(config.clone());
            let _ = tx.send(ServerMessage::Config { config });
        }
    }
}
