//! Embedded Model Context Protocol (MCP) server for Microbridge.
//!
//! Exposes a local HTTP JSON-RPC endpoint (`http://127.0.0.1:9190/mcp`)
//! enabling any MCP-compatible agent (Claude Desktop, Goose, Roo Code, Continue)
//! to report session lifecycle states and handle interactive hardware approvals.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn spawn_mcp_server(shared: Arc<Mutex<DaemonState>>) {
    if !mcp_enabled_by_env() {
        info!("Embedded MCP server dormant (set MICROBRIDGE_MCP=1 to enable)");
        return;
    }
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
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };
                let request_str = String::from_utf8_lossy(&buf[..n]);

                if request_str.starts_with("OPTIONS") {
                    let response = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes()).await;
                    return;
                }

                if !request_str.starts_with("POST") {
                    let response =
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 26\r\n\r\nEndpoint is POST /mcp";
                    let _ = stream.write_all(response.as_bytes()).await;
                    return;
                }

                let body = if let Some(pos) = request_str.find("\r\n\r\n") {
                    &request_str[pos + 4..]
                } else {
                    ""
                };

                let rpc_req: JsonRpcRequest = match serde_json::from_str(body) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": { "code": -32700, "message": format!("Parse error: {}", err) }
                        })
                        .to_string();
                        let http_resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                            resp.len(),
                            resp
                        );
                        let _ = stream.write_all(http_resp.as_bytes()).await;
                        return;
                    }
                };

                let response_body = process_mcp_rpc(state_clone, rpc_req).await;
                let http_resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(http_resp.as_bytes()).await;
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
                    "description": "Trigger hardware approval prompt on Microbridge deck and wait for user keypress",
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
                    "text": format!("Approval prompt displayed on Microbridge deck for session {}", session_id)
                }]
            }))
        }
        _ => Err((-32601, format!("Unknown MCP tool: {}", tool_name))),
    }
}
