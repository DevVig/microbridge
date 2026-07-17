//! Tauri companion — talks to microbridged over the local Unix socket.
//! Never opens HID; the daemon owns the device.

use std::path::PathBuf;

use mb_protocol::{
    ClientMessage, ClientRole, DaemonConfig, ServerMessage, Snapshot, PROTOCOL_VERSION,
};
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

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

#[tauri::command]
async fn get_snapshot() -> Result<Snapshot, String> {
    let (mut write, mut reader) = open_ui_client().await?;
    write_msg(&mut write, &ClientMessage::Subscribe).await?;
    match read_matching(&mut reader, true, false).await? {
        ServerMessage::Snapshot { snapshot } => Ok(snapshot),
        _ => Err("unexpected response".into()),
    }
}

#[tauri::command]
async fn set_config(config: DaemonConfig) -> Result<DaemonConfig, String> {
    let (mut write, mut reader) = open_ui_client().await?;
    write_msg(&mut write, &ClientMessage::SetConfig { config }).await?;
    match read_matching(&mut reader, false, true).await? {
        ServerMessage::Config { config } => Ok(config),
        _ => Err("unexpected response".into()),
    }
}

#[tauri::command]
async fn set_frontmost_app(app: Option<String>) -> Result<(), String> {
    let snap = get_snapshot().await?;
    let mut config = snap.config;
    config.frontmost_app = app;
    let _ = set_config(config).await?;
    Ok(())
}

#[tauri::command]
fn quit_ui(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            set_config,
            set_frontmost_app,
            quit_ui
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("Microbridge");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running microbridge-ui");
}
