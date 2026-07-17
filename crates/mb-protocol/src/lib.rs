//! Wire types for the Microbridge adapter protocol.
//!
//! Transport is newline-delimited JSON over a local Unix domain socket.
//! `docs/protocol.md` is the normative spec; these types are its source of
//! truth — if they disagree, fix one of them in the same PR.

use serde::{Deserialize, Serialize};

/// Protocol revision. Bumped on breaking changes; adapters announce theirs in
/// [`Message::Hello`].
pub const PROTOCOL_VERSION: u32 = 0;

/// Lifecycle state of one agent session, as reported by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Thinking,
    Working,
    AwaitingApproval,
    Done,
    Error,
}

/// One agent session — a single thread/conversation in a single app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStatus {
    /// Adapter-scoped unique id, e.g. `codex:0195fa…`.
    pub id: String,
    /// Human-readable app name, e.g. `Claude Code`.
    pub app: String,
    /// Short label for UIs, e.g. the task title. May be empty.
    #[serde(default)]
    pub title: String,
    pub state: AgentState,
    /// Milliseconds since the Unix epoch, supplied by the adapter.
    pub updated_at_ms: u64,
}

/// Adapter → daemon messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Must be the first message on every connection.
    Hello {
        adapter: String,
        protocol_version: u32,
    },
    /// Full state for one session. Sent on every transition — never on a
    /// timer. The daemon treats each `status` as a complete replacement.
    Status { session: SessionStatus },
    /// The session ended and should be dropped from the registry.
    Bye { session_id: String },
}

/// Daemon → adapter messages: key presses routed to the focused session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Action { session_id: String, action: Action },
}

/// Actions a physical key can trigger on the focused agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Approve,
    Reject,
    Interrupt,
    NewSession,
    CycleFocus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips() {
        let msg = Message::Status {
            session: SessionStatus {
                id: "codex:abc".into(),
                app: "Codex CLI".into(),
                title: "fix flaky e2e retries".into(),
                state: AgentState::AwaitingApproval,
                updated_at_ms: 1,
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"status""#));
        assert!(json.contains(r#""state":"awaiting_approval""#));
        assert_eq!(serde_json::from_str::<Message>(&json).unwrap(), msg);
    }

    #[test]
    fn title_defaults_to_empty() {
        let json = r#"{"type":"status","session":{"id":"x:1","app":"X","state":"idle","updated_at_ms":0}}"#;
        let Message::Status { session } = serde_json::from_str(json).unwrap() else {
            panic!("expected status");
        };
        assert_eq!(session.title, "");
    }
}
