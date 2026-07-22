//! Embedded Model Context Protocol (MCP) server for Microbridge.
//!
//! Exposes an opt-in local HTTP JSON-RPC endpoint (`http://127.0.0.1:9190/mcp`)
//! when `MICROBRIDGE_MCP=1` and `MICROBRIDGE_MCP_TOKEN` are set. Clients must send
//! `Authorization: Bearer <token>` and a loopback `Host` header.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mb_adapters::ObservedSession;
use mb_protocol::{AdapterCapabilities, AdapterConnectionState, AgentState, SessionStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::state::DaemonState;

/// Distinct from Cursor ACP (`u64::MAX - 4`).
pub const MCP_OWNER: u64 = u64::MAX - 5;
pub const MCP_PORT: u16 = 9190;

/// Opt-in only — idle TCP bind is skipped unless `MICROBRIDGE_MCP=1`.
pub fn mcp_enabled_by_env() -> bool {
    matches!(
        std::env::var("MICROBRIDGE_MCP").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

pub fn capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        lifecycle_observation: true,
        approval_acceptance: true,
        approval_rejection: true,
        interrupt: true,
        mcp_native: true,
        ..AdapterCapabilities::default()
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn mcp_auth_token() -> Option<String> {
    std::env::var("MICROBRIDGE_MCP_TOKEN")
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    for line in headers.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(value.trim());
        }
    }
    None
}

fn host_is_loopback(host: &str) -> bool {
    let host = host.split(':').next().unwrap_or(host).trim();
    matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1")
}

const MCP_MAX_HEADERS: usize = 64 * 1024;
const MCP_MAX_BODY: usize = 256 * 1024;
const MCP_READ_TIMEOUT: Duration = Duration::from_secs(5);

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> Option<(String, Vec<u8>)> {
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 4096];
    loop {
        let n = tokio::time::timeout(MCP_READ_TIMEOUT, stream.read(&mut tmp))
            .await
            .ok()?
            .ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let header_end = pos + 4;
            let headers = String::from_utf8_lossy(&buf[..pos]).into_owned();
            let content_length = header_value(&headers, "Content-Length")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            if content_length > MCP_MAX_BODY {
                return None;
            }
            while buf.len() < header_end + content_length {
                let n = tokio::time::timeout(MCP_READ_TIMEOUT, stream.read(&mut tmp))
                    .await
                    .ok()?
                    .ok()?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() > header_end + MCP_MAX_BODY {
                    return None;
                }
            }
            let body = buf
                .get(header_end..header_end + content_length)
                .unwrap_or(&[])
                .to_vec();
            return Some((headers, body));
        }
        if buf.len() > MCP_MAX_HEADERS {
            return None;
        }
    }
    None
}

async fn write_http(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

async fn write_json(stream: &mut tokio::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

pub fn spawn_mcp_server(shared: Arc<Mutex<DaemonState>>) {
    if !mcp_enabled_by_env() {
        info!("Embedded MCP server dormant (set MICROBRIDGE_MCP=1 to enable)");
        return;
    }
    let Some(expected_token) = mcp_auth_token() else {
        warn!("MICROBRIDGE_MCP=1 but MICROBRIDGE_MCP_TOKEN is unset; MCP server not started");
        return;
    };
    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], MCP_PORT));
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                warn!(error = %e, "Failed to bind MCP server TCP listener");
                return;
            }
        };

        info!(%addr, "Embedded MCP server listening for agent connections");
        {
            let mut s = shared.lock().await;
            s.set_adapter_runtime(
                "mcp",
                AdapterConnectionState::Connected,
                capabilities(),
                format!("Listening on http://{}", addr),
            );
        }

        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            let state_clone = Arc::clone(&shared);
            let expected_token = expected_token.clone();
            tokio::spawn(async move {
                let Some((headers, body_bytes)) = read_http_request(&mut stream).await else {
                    return;
                };
                let request_line = headers.lines().next().unwrap_or_default();

                if request_line.starts_with("OPTIONS") {
                    write_http(&mut stream, "204 No Content", "").await;
                    return;
                }

                if !request_line.starts_with("POST") {
                    write_http(&mut stream, "404 Not Found", "Endpoint is POST /mcp").await;
                    return;
                }

                let host = header_value(&headers, "Host").unwrap_or_default();
                if !host_is_loopback(host) {
                    write_http(&mut stream, "403 Forbidden", "Host must be loopback").await;
                    return;
                }

                let auth = header_value(&headers, "Authorization").unwrap_or_default();
                let token = auth
                    .strip_prefix("Bearer ")
                    .or_else(|| auth.strip_prefix("bearer "))
                    .unwrap_or("");
                if token != expected_token {
                    write_http(&mut stream, "401 Unauthorized", "Invalid MCP token").await;
                    return;
                }

                let body = String::from_utf8_lossy(&body_bytes);
                let rpc_req: JsonRpcRequest = match serde_json::from_str(&body) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": { "code": -32700, "message": format!("Parse error: {}", err) }
                        })
                        .to_string();
                        write_json(&mut stream, &resp).await;
                        return;
                    }
                };

                let response_body = process_mcp_rpc(state_clone, rpc_req).await;
                write_json(&mut stream, &response_body).await;
            });
        }
    });
}

async fn process_mcp_rpc(state: Arc<Mutex<DaemonState>>, req: JsonRpcRequest) -> String {
    let req_id = req.id.clone().unwrap_or(Value::Null);

    let result = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "microbridge-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => Ok(json!({
            "tools": [
                {
                    "name": "microbridge_report_state",
                    "description": "Report current AI agent session state to Microbridge hardware deck",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "session_id": { "type": "string" },
                            "app_name": { "type": "string" },
                            "title": { "type": "string" },
                            "state": { "type": "string", "enum": ["idle", "thinking", "working", "awaiting_approval", "done", "error"] }
                        },
                        "required": ["session_id", "state"]
                    }
                },
                {
                    "name": "microbridge_request_approval",
                    "description": "Fire-and-forget: show an approval prompt on the Microbridge deck (does not block for the keypress; poll session state or listen on the bus for Approve/Reject)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "session_id": { "type": "string" },
                            "prompt": { "type": "string" }
                        },
                        "required": ["session_id", "prompt"]
                    }
                }
            ]
        })),
        "tools/call" => handle_tool_call(state, &req.params).await,
        _ => Err((-32601, format!("Method not found: {}", req.method))),
    };

    match result {
        Ok(val) => serde_json::to_string(&JsonRpcResponse {
            jsonrpc: "2.0",
            id: req_id,
            result: Some(val),
            error: None,
        })
        .unwrap_or_default(),
        Err((code, msg)) => serde_json::to_string(&JsonRpcResponse {
            jsonrpc: "2.0",
            id: req_id,
            result: None,
            error: Some(json!({ "code": code, "message": msg })),
        })
        .unwrap_or_default(),
    }
}

async fn handle_tool_call(
    state: Arc<Mutex<DaemonState>>,
    params: &Value,
) -> Result<Value, (i32, String)> {
    let tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| (-32602, "Missing tool name".into()))?;

    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "microbridge_report_state" => {
            let session_id = args
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or_else(|| (-32602, "Missing session_id".into()))?;

            let raw_state = args
                .get("state")
                .and_then(Value::as_str)
                .ok_or_else(|| (-32602, "Missing state".into()))?;

            let app_name = args
                .get("app_name")
                .and_then(Value::as_str)
                .unwrap_or("MCP Agent");
            let title = args.get("title").and_then(Value::as_str).unwrap_or("");

            let agent_state = match raw_state {
                "idle" => AgentState::Idle,
                "thinking" => AgentState::Thinking,
                "working" => AgentState::Working,
                "awaiting_approval" => AgentState::AwaitingApproval,
                "done" => AgentState::Done,
                "error" => AgentState::Error,
                _ => AgentState::Idle,
            };

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let full_id = format!("mcp:{}", session_id);
            let status = SessionStatus {
                id: full_id,
                app: app_name.to_string(),
                title: title.to_string(),
                state: agent_state,
                updated_at_ms: now,
                focus_uri: None,
            };

            let observed = ObservedSession {
                session: status,
                context: None,
            };

            let mut dstate = state.lock().await;
            dstate.upsert_observed_session(observed, MCP_OWNER);

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!("Reported state {} for session {}", raw_state, session_id)
                }]
            }))
        }
        "microbridge_request_approval" => {
            let session_id = args
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or_else(|| (-32602, "Missing session_id".into()))?;
            let prompt = args
                .get("prompt")
                .and_then(Value::as_str)
                .unwrap_or("Approval requested");

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let full_id = format!("mcp:{}", session_id);
            let status = SessionStatus {
                id: full_id,
                app: "MCP Agent".to_string(),
                title: prompt.to_string(),
                state: AgentState::AwaitingApproval,
                updated_at_ms: now,
                focus_uri: None,
            };

            let observed = ObservedSession {
                session: status,
                context: None,
            };

            let mut dstate = state.lock().await;
            dstate.upsert_observed_session(observed, MCP_OWNER);

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Approval prompt displayed on Microbridge deck for session {} (fire-and-forget; not waiting for keypress)",
                        session_id
                    )
                }]
            }))
        }
        _ => Err((-32601, format!("Unknown MCP tool: {}", tool_name))),
    }
}
