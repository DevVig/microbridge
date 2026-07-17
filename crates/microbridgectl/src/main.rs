//! microbridgectl — inspect a running microbridged instance.

use std::path::PathBuf;
use std::process::ExitCode;

use mb_protocol::{ClientMessage, ClientRole, ServerMessage, PROTOCOL_VERSION};
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

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "status".into());

    match cmd.as_str() {
        "status" => match fetch_snapshot().await {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("microbridgectl: {error}");
                eprintln!("is the daemon running? (cargo run -p microbridged)");
                ExitCode::FAILURE
            }
        },
        "help" | "-h" | "--help" => {
            println!("Usage: microbridgectl [status]");
            println!("  status   print the live bus snapshot as JSON (default)");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("Usage: microbridgectl [status]");
            ExitCode::FAILURE
        }
    }
}

async fn fetch_snapshot() -> Result<String, String> {
    let path = socket_path();
    let stream = UnixStream::connect(&path)
        .await
        .map_err(|e| format!("connect {}: {e}", path.display()))?;
    let (read_half, mut write_half) = stream.into_split();

    let hello = ClientMessage::Hello {
        adapter: "microbridgectl".into(),
        protocol_version: PROTOCOL_VERSION,
        role: ClientRole::Ui,
    };
    write_line(&mut write_half, &hello).await?;
    write_line(&mut write_half, &ClientMessage::Subscribe).await?;

    let mut lines = BufReader::new(read_half).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| format!("read: {e}"))? {
        if line.trim().is_empty() {
            continue;
        }
        let msg: ServerMessage = serde_json::from_str(&line).map_err(|e| format!("parse: {e}"))?;
        if let ServerMessage::Snapshot { snapshot } = msg {
            return serde_json::to_string_pretty(&snapshot).map_err(|e| format!("serialize: {e}"));
        }
    }
    Err("daemon closed before sending snapshot".into())
}

async fn write_line(
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
