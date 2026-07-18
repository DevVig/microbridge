//! Session registry + focus policy.

use std::collections::HashMap;

use mb_protocol::{AgentState, DaemonConfig, SessionStatus};

use crate::key_source;

#[derive(Debug, Default)]
pub struct Registry {
    pub sessions: HashMap<String, SessionStatus>,
    pub focused: Option<String>,
    /// Which socket connection (adapter name + conn id) owns each session.
    pub owners: HashMap<String, u64>,
}

impl Registry {
    pub fn upsert(&mut self, session: SessionStatus, owner: u64, config: &DaemonConfig) {
        self.owners.insert(session.id.clone(), owner);
        self.sessions.insert(session.id.clone(), session);
        self.resolve_focus(config);
    }

    pub fn remove(&mut self, session_id: &str, config: &DaemonConfig) {
        self.sessions.remove(session_id);
        self.owners.remove(session_id);
        self.resolve_focus(config);
    }

    pub fn remove_owner(&mut self, owner: u64, config: &DaemonConfig) {
        let doomed: Vec<String> = self
            .owners
            .iter()
            .filter(|(_, o)| **o == owner)
            .map(|(id, _)| id.clone())
            .collect();
        for id in doomed {
            self.sessions.remove(&id);
            self.owners.remove(&id);
        }
        self.resolve_focus(config);
    }

    /// Focus policy:
    /// 1. pinned_focus if still alive
    /// 2. awaiting_approval preempts (when approvals_interrupt)
    /// 3. current focus keeps the deck while it exists
    /// 4. frontmost app's most recent session (auto-follow via watcher)
    /// 5. most recently updated session
    pub fn resolve_focus(&mut self, config: &DaemonConfig) {
        if let Some(pin) = &config.pinned_focus {
            if self.sessions.contains_key(pin) {
                self.focused = Some(pin.clone());
                return;
            }
        }

        if config.approvals_interrupt {
            let approval = self
                .sessions
                .values()
                .filter(|s| s.state == AgentState::AwaitingApproval)
                .max_by_key(|s| s.updated_at_ms);
            if let Some(session) = approval {
                self.focused = Some(session.id.clone());
                return;
            }
        }

        if let Some(id) = &self.focused {
            if self.sessions.contains_key(id) {
                return;
            }
        }

        if let Some(app) = &config.frontmost_app {
            let front = self
                .sessions
                .values()
                .filter(|s| &s.app == app)
                .max_by_key(|s| s.updated_at_ms);
            if let Some(session) = front {
                self.focused = Some(session.id.clone());
                return;
            }
        }

        self.focused = self
            .sessions
            .values()
            .max_by_key(|s| s.updated_at_ms)
            .map(|s| s.id.clone());
    }

    pub fn focused_session(&self) -> Option<&SessionStatus> {
        self.focused.as_ref().and_then(|id| self.sessions.get(id))
    }

    pub fn agent_key_ids(&self, config: &DaemonConfig) -> [Option<String>; 6] {
        let list: Vec<_> = self.sessions.values().cloned().collect();
        key_source::resolve_agent_keys(&list, self.focused.as_deref(), config)
    }

    pub fn session_list(&self) -> Vec<SessionStatus> {
        let mut list: Vec<_> = self.sessions.values().cloned().collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at_ms));
        list
    }

    pub fn owner_of(&self, session_id: &str) -> Option<u64> {
        self.owners.get(session_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, state: AgentState, at: u64) -> SessionStatus {
        SessionStatus {
            id: id.into(),
            app: "test".into(),
            title: String::new(),
            state,
            updated_at_ms: at,
        }
    }

    #[test]
    fn most_recent_session_gets_initial_focus() {
        let mut registry = Registry::default();
        let config = DaemonConfig::default();
        registry.upsert(session("a", AgentState::Working, 1), 1, &config);
        registry.upsert(session("b", AgentState::Thinking, 2), 1, &config);
        // "a" already held focus and still exists, so it keeps the deck.
        assert_eq!(registry.focused.as_deref(), Some("a"));
    }

    #[test]
    fn approval_preempts_and_releases() {
        let mut registry = Registry::default();
        let config = DaemonConfig::default();
        registry.upsert(session("a", AgentState::Working, 1), 1, &config);
        registry.upsert(session("b", AgentState::AwaitingApproval, 2), 1, &config);
        assert_eq!(registry.focused.as_deref(), Some("b"));

        registry.upsert(session("b", AgentState::Working, 3), 1, &config);
        assert_eq!(registry.focused.as_deref(), Some("b"));

        registry.remove("b", &config);
        assert_eq!(registry.focused.as_deref(), Some("a"));
    }

    #[test]
    fn empty_registry_clears_the_deck() {
        let mut registry = Registry::default();
        let config = DaemonConfig::default();
        registry.upsert(session("a", AgentState::Done, 1), 1, &config);
        registry.remove("a", &config);
        assert_eq!(registry.focused, None);
    }

    #[test]
    fn pinned_focus_beats_approval() {
        let mut registry = Registry::default();
        let config = DaemonConfig {
            pinned_focus: Some("a".into()),
            ..Default::default()
        };
        registry.upsert(session("a", AgentState::Working, 1), 1, &config);
        registry.upsert(session("b", AgentState::AwaitingApproval, 2), 1, &config);
        assert_eq!(registry.focused.as_deref(), Some("a"));
    }
}
