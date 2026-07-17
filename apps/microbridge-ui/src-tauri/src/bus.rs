//! Persistent UI socket: Hello → Subscribe → push Snapshot / Event to the app.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use mb_protocol::{
    BusEvent, ClientMessage, ClientRole, DaemonConfig, ServerMessage, Snapshot, PROTOCOL_VERSION,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, Mutex};

pub type CachedSnapshot = Arc<Mutex<Option<Snapshot>>>;

pub struct BusHandle {
    // marker — writes use short-lived connections
}

impl BusHandle {
    pub async fn set_config(&self, config: DaemonConfig) -> Result<DaemonConfig, String> {
        let (mut write, mut reader) = open_ui_client().await?;
        write_msg(
            &mut write,
            &ClientMessage::SetConfig {
                config: config.clone(),
            },
        )
        .await?;
        match read_matching(&mut reader, false, true).await? {
            ServerMessage::Config { config } => Ok(config),
            _ => Err("unexpected set_config reply".into()),
        }
    }
}

fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("MICROBRIDGE_SOCKET") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".microbridge")
        .join("microbridged.sock")
}

async fn open_ui_client() -> Result<
    (
        tokio::net::unix::OwnedWriteHalf,
        BufReader<tokio::net::unix::OwnedReadHalf>,
    ),
    String,
> {
    let path = socket_path();
    let stream = UnixStream::connect(&path)
        .await
        .map_err(|e| format!("connect {}: {e}", path.display()))?;
    let (read_half, mut write_half) = stream.into_split();
    write_msg(
        &mut write_half,
        &ClientMessage::Hello {
            adapter: "microbridge-ui".into(),
            protocol_version: PROTOCOL_VERSION,
            role: ClientRole::Ui,
        },
    )
    .await?;
    Ok((write_half, BufReader::new(read_half)))
}

async fn write_msg(
    write_half: &mut tokio::net::unix::OwnedWriteHalf,
    msg: &ClientMessage,
) -> Result<(), String> {
    let line = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    write_half
        .write_all(line.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    write_half
        .write_all(b"\n")
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn read_matching(
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    want_snapshot: bool,
    want_config: bool,
) -> Result<ServerMessage, String> {
    let mut lines = reader.lines();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| format!("read: {e}"))?
    {
        if line.trim().is_empty() {
            continue;
        }
        let msg: ServerMessage =
            serde_json::from_str(&line).map_err(|e| format!("parse: {e}"))?;
        match &msg {
            ServerMessage::Snapshot { .. } if want_snapshot => return Ok(msg),
            ServerMessage::Config { .. } if want_config => return Ok(msg),
            _ => continue,
        }
    }
    Err("daemon closed".into())
}

pub fn apply_event(snap: &mut Snapshot, event: BusEvent) {
    match event {
        BusEvent::SessionUpserted { session } => {
            if let Some(existing) = snap.sessions.iter_mut().find(|s| s.id == session.id) {
                *existing = session;
            } else {
                snap.sessions.push(session);
            }
        }
        BusEvent::SessionRemoved { session_id } => {
            snap.sessions.retain(|s| s.id != session_id);
            if snap.focused_session_id.as_deref() == Some(session_id.as_str()) {
                snap.focused_session_id = None;
            }
            for slot in &mut snap.agent_key_session_ids {
                if slot.as_deref() == Some(session_id.as_str()) {
                    *slot = None;
                }
            }
        }
        BusEvent::FocusChanged { session_id } => {
            snap.focused_session_id = session_id;
        }
        BusEvent::AgentKeysChanged { session_ids } => {
            snap.agent_key_session_ids = session_ids;
        }
        BusEvent::DeviceChanged { connected, name } => {
            snap.device_connected = connected;
            snap.device_name = name;
        }
        BusEvent::ConfigChanged { config } => {
            snap.config = config;
        }
    }
}

/// Spawn reconnecting subscribe loop. Returns a handle for short-lived writes
/// and a channel of inbound Snapshot / Event / Config messages.
pub fn spawn_bus_loop() -> (BusHandle, mpsc::UnboundedReceiver<ServerMessage>) {
    let (tx, rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = run_subscribe_once(&tx).await;
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    });
    (BusHandle {}, rx)
}

async fn run_subscribe_once(
    tx: &mpsc::UnboundedSender<ServerMessage>,
) -> Result<(), String> {
    let (mut write, reader) = open_ui_client().await?;
    write_msg(&mut write, &ClientMessage::Subscribe).await?;

    let mut lines = reader.lines();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| format!("read: {e}"))?
    {
        if line.trim().is_empty() {
            continue;
        }
        let msg: ServerMessage =
            serde_json::from_str(&line).map_err(|e| format!("parse: {e}"))?;
        if tx.send(msg).is_err() {
            break;
        }
    }
    Err("daemon closed".into())
}
