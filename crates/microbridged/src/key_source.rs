//! Resolve which sessions occupy the six Agent Keys.

use mb_protocol::{AgentState, DaemonConfig, KeySource, SessionStatus, AGENT_KEY_COUNT};

use crate::app_match::same_app;

/// Fill six Agent Key slots from the session bus + config.
pub fn resolve_agent_keys(
    sessions: &[SessionStatus],
    focused_session_id: Option<&str>,
    config: &DaemonConfig,
) -> [Option<String>; AGENT_KEY_COUNT] {
    let mut slots = [None, None, None, None, None, None];
    let ids = match config.key_source {
        KeySource::MostRecent => most_recent(sessions),
        KeySource::FocusedApp => focused_app(sessions, focused_session_id, config),
        KeySource::Pinned => pinned(sessions, config),
        KeySource::Priority => priority(sessions, config),
        KeySource::Custom => custom(config),
    };
    for (i, id) in ids.into_iter().take(AGENT_KEY_COUNT).enumerate() {
        slots[i] = id;
    }
    slots
}

fn most_recent(sessions: &[SessionStatus]) -> Vec<Option<String>> {
    let mut sorted: Vec<_> = sessions.iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.updated_at_ms));
    pad(sorted.into_iter().map(|s| Some(s.id.clone())).collect())
}

fn focused_app(
    sessions: &[SessionStatus],
    focused_session_id: Option<&str>,
    config: &DaemonConfig,
) -> Vec<Option<String>> {
    let app = focused_session_id
        .and_then(|id| sessions.iter().find(|s| s.id == id))
        .map(|s| s.app.as_str())
        .or(config.frontmost_app.as_deref());

    let Some(app) = app else {
        return most_recent(sessions);
    };

    let mut sorted: Vec<_> = sessions.iter().filter(|s| same_app(&s.app, app)).collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.updated_at_ms));
    pad(sorted.into_iter().map(|s| Some(s.id.clone())).collect())
}

fn pinned(sessions: &[SessionStatus], config: &DaemonConfig) -> Vec<Option<String>> {
    let known: std::collections::HashSet<_> = sessions.iter().map(|s| s.id.as_str()).collect();
    pad(config
        .pinned_session_ids
        .iter()
        .map(|id| {
            if known.contains(id.as_str()) {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect())
}

fn priority(sessions: &[SessionStatus], config: &DaemonConfig) -> Vec<Option<String>> {
    let mut sorted: Vec<_> = sessions.iter().collect();
    sorted.sort_by(|a, b| {
        priority_rank(a, config)
            .cmp(&priority_rank(b, config))
            .then_with(|| b.updated_at_ms.cmp(&a.updated_at_ms))
    });
    pad(sorted.into_iter().map(|s| Some(s.id.clone())).collect())
}

fn priority_rank(session: &SessionStatus, config: &DaemonConfig) -> u8 {
    let state_rank: u8 = match session.state {
        AgentState::AwaitingApproval => 0,
        AgentState::Working | AgentState::Thinking => 1,
        AgentState::Error => 2,
        AgentState::Done => 3,
        AgentState::Idle => 4,
    };
    let app_rank = config
        .app_priority
        .iter()
        .position(|a| a == &session.app)
        .unwrap_or(99) as u8;
    state_rank.saturating_add(app_rank / 10)
}

fn custom(config: &DaemonConfig) -> Vec<Option<String>> {
    let mut out: Vec<Option<String>> = config
        .custom_key_ids
        .iter()
        .map(|id| {
            if id.is_empty() {
                None
            } else {
                Some(id.clone())
            }
        })
        .collect();
    out.resize(AGENT_KEY_COUNT, None);
    out
}

fn pad(mut ids: Vec<Option<String>>) -> Vec<Option<String>> {
    ids.resize(AGENT_KEY_COUNT, None);
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, app: &str, state: AgentState, at: u64) -> SessionStatus {
        SessionStatus {
            id: id.into(),
            app: app.into(),
            title: String::new(),
            state,
            updated_at_ms: at,
            focus_uri: None,
        }
    }

    #[test]
    fn most_recent_orders_by_updated_at() {
        let sessions = vec![
            session("a", "Codex", AgentState::Idle, 1),
            session("b", "Cursor", AgentState::Working, 3),
            session("c", "Claude Code", AgentState::Thinking, 2),
        ];
        let config = DaemonConfig {
            key_source: KeySource::MostRecent,
            ..Default::default()
        };
        let keys = resolve_agent_keys(&sessions, Some("a"), &config);
        assert_eq!(keys[0].as_deref(), Some("b"));
        assert_eq!(keys[1].as_deref(), Some("c"));
        assert_eq!(keys[2].as_deref(), Some("a"));
        assert!(keys[3].is_none());
    }

    #[test]
    fn focused_app_filters_to_owning_app() {
        let sessions = vec![
            session("c1", "Codex CLI", AgentState::Working, 5),
            session("c2", "Codex CLI", AgentState::Idle, 4),
            session("x1", "Cursor", AgentState::Working, 9),
        ];
        // Default key source is focused_app: IDE-scoped, newest first.
        let config = DaemonConfig::default();
        let keys = resolve_agent_keys(&sessions, Some("c1"), &config);
        assert_eq!(keys[0].as_deref(), Some("c1"));
        assert_eq!(keys[1].as_deref(), Some("c2"));
        assert!(keys[2].is_none());
    }

    #[test]
    fn focused_app_matches_t3_nightly_and_cursor() {
        let sessions = vec![
            session("t1", "T3 Code", AgentState::Working, 5),
            session("t2", "T3 Code", AgentState::Idle, 4),
            session("c1", "Cursor", AgentState::Working, 9),
            session("s1", "Synara", AgentState::Thinking, 8),
        ];
        let config = DaemonConfig {
            frontmost_app: Some("T3 Code (Nightly)".into()),
            ..Default::default()
        };
        // No focused session → frontmost Nightly still scopes to T3 Code threads.
        let keys = resolve_agent_keys(&sessions, None, &config);
        assert_eq!(keys[0].as_deref(), Some("t1"));
        assert_eq!(keys[1].as_deref(), Some("t2"));
        assert!(keys[2].is_none());

        let cursor_focus = resolve_agent_keys(&sessions, Some("c1"), &config);
        assert_eq!(cursor_focus[0].as_deref(), Some("c1"));
        assert!(cursor_focus[1].is_none());
    }

    #[test]
    fn focused_app_matches_claude_frontmost() {
        let sessions = vec![
            session("cl1", "Claude Code", AgentState::Working, 6),
            session("cl2", "Claude Code", AgentState::Idle, 3),
            session("x1", "Cursor", AgentState::Working, 9),
        ];
        let config = DaemonConfig {
            frontmost_app: Some("Claude".into()),
            ..Default::default()
        };
        let keys = resolve_agent_keys(&sessions, None, &config);
        assert_eq!(keys[0].as_deref(), Some("cl1"));
        assert_eq!(keys[1].as_deref(), Some("cl2"));
        assert!(keys[2].is_none());
    }

    #[test]
    fn custom_assignments_honored() {
        let sessions = vec![session("a", "Codex", AgentState::Working, 1)];
        let config = DaemonConfig {
            key_source: KeySource::Custom,
            custom_key_ids: vec![
                String::new(),
                "a".into(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            ..Default::default()
        };
        let keys = resolve_agent_keys(&sessions, None, &config);
        assert!(keys[0].is_none());
        assert_eq!(keys[1].as_deref(), Some("a"));
    }
}
