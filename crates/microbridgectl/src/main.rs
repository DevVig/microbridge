//! microbridgectl — inspect a running microbridged instance.

use std::path::PathBuf;
use std::process::ExitCode;

use mb_protocol::{
    AdapterCapabilities, AgentState, ClientMessage, ClientRole, ServerMessage, SessionStatus,
    PROTOCOL_VERSION,
};
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
        "hid-capture" => run_hid_capture(args.next()),
        "cursor-event" => run_cursor_event(args.collect()).await,
        "help" | "-h" | "--help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    println!("Usage: microbridgectl [status | hid-capture [seconds] | cursor-event <id> <state> [title]]");
    println!("  status                 print the live bus snapshot as JSON (default)");
    println!("  hid-capture [seconds]  observe raw Codex Micro key/dial/joystick events");
    println!("                         (default 120s; needs the `hid` feature + a device)");
}

async fn run_cursor_event(args: Vec<String>) -> ExitCode {
    if !(2..=3).contains(&args.len()) {
        eprintln!("cursor-event requires <conversation-id> <state> [title]");
        return ExitCode::FAILURE;
    }
    let state = match args[1].as_str() {
        "idle" | "stop" | "session_end" => AgentState::Idle,
        "thinking" | "before_submit_prompt" | "after_agent_thought" => AgentState::Thinking,
        "working" | "pre_tool_use" | "post_tool_use" => AgentState::Working,
        "awaiting_approval" => AgentState::AwaitingApproval,
        "done" | "after_agent_response" => AgentState::Done,
        "error" => AgentState::Error,
        other => {
            eprintln!("unsupported cursor lifecycle state: {other}");
            return ExitCode::FAILURE;
        }
    };
    let session = SessionStatus {
        id: format!("cursor:{}", args[0]),
        app: "Cursor".into(),
        title: args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "Cursor agent".into()),
        state,
        updated_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };
    match send_operation(ClientMessage::IngestLifecycle {
        adapter_id: "cursor".into(),
        session,
        ttl_ms: if args[1] == "session_end" {
            1_000
        } else {
            30 * 60 * 1000
        },
    })
    .await
    {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("microbridgectl cursor-event: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Stream decoded device→host HID events for hardware bring-up.
/// See docs/hardware-bringup.md.
fn run_hid_capture(seconds_arg: Option<String>) -> ExitCode {
    let seconds = seconds_arg.as_deref().map(|s| s.parse::<u64>()).transpose();
    let seconds = match seconds {
        Ok(value) => value.unwrap_or(120),
        Err(_) => {
            eprintln!(
                "microbridgectl hid-capture: seconds must be a whole number (0 = until Ctrl-C)"
            );
            return ExitCode::FAILURE;
        }
    };

    #[cfg(feature = "hid")]
    {
        match mb_device::run_capture(seconds) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("microbridgectl hid-capture: {error}");
                eprintln!("no Codex Micro detected? plug it in, or quit the app that owns it.");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(feature = "hid"))]
    {
        let _ = seconds;
        eprintln!(
            "hid-capture needs the `hid` feature: cargo run -p microbridgectl --features hid -- hid-capture"
        );
        ExitCode::FAILURE
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
        adapter_version: Some(env!("CARGO_PKG_VERSION").into()),
        capabilities: AdapterCapabilities::default(),
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

async fn send_operation(message: ClientMessage) -> Result<String, String> {
    let path = socket_path();
    let stream = UnixStream::connect(&path)
        .await
        .map_err(|e| format!("connect {}: {e}", path.display()))?;
    let (read_half, mut write_half) = stream.into_split();
    write_line(
        &mut write_half,
        &ClientMessage::Hello {
            adapter: "microbridgectl".into(),
            protocol_version: PROTOCOL_VERSION,
            role: ClientRole::Ui,
            adapter_version: Some(env!("CARGO_PKG_VERSION").into()),
            capabilities: AdapterCapabilities::default(),
        },
    )
    .await?;
    write_line(&mut write_half, &message).await?;
    let mut lines = BufReader::new(read_half).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        if let ServerMessage::AdapterOperation { ok, message, .. } =
            serde_json::from_str::<ServerMessage>(&line).map_err(|e| e.to_string())?
        {
            return if ok { Ok(message) } else { Err(message) };
        }
    }
    Err("daemon closed before acknowledging event".into())
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
