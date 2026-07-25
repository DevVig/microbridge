//! Session registry + focus policy.

use std::collections::HashMap;

use mb_protocol::{AgentState, DaemonConfig, SessionStatus};

use crate::app_match::{app_family, same_app};
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

    /// The pinned controller, but only while it actually has live sessions.
    ///
    /// Returning `None` when the pinned IDE is empty is what makes the deck fall
    /// back to the most recent thread instead of going dark — a lock on an IDE
    /// you closed hours ago would leave the hardware inert with no explanation.
    /// The menu bar app surfaces the fallback in the tray so it stays legible.
    pub fn active_controller<'a>(&self, config: &'a DaemonConfig) -> Option<&'a str> {
        let family = config.controlling_ide.as_deref()?;
        self.sessions
            .values()
            .any(|session| app_family(&session.app) == family)
            .then_some(family)
    }

    /// Focus policy, applied within the controlling IDE when one is pinned:
    /// 1. pinned_focus if still alive
    /// 2. awaiting_approval preempts (when approvals_interrupt)
    /// 3. current focus keeps the deck while it exists
    /// 4. frontmost app's most recent session (auto-follow via watcher)
    /// 5. most recently updated session
    ///
    /// A pinned controller filters every step, so an approval in another IDE no
    /// longer preempts and the frontmost watcher no longer moves the deck. That
    /// is the lock doing its job, not a bug.
    pub fn resolve_focus(&mut self, config: &DaemonConfig) {
        let controller = self.active_controller(config);
        let eligible = |session: &SessionStatus| match controller {
            Some(family) => app_family(&session.app) == family,
            None => true,
        };

        if let Some(pin) = &config.pinned_focus {
            if self.sessions.get(pin).is_some_and(eligible) {
                self.focused = Some(pin.clone());
                return;
            }
        }

        if config.approvals_interrupt {
            let approval = self
                .sessions
                .values()
                .filter(|s| s.state == AgentState::AwaitingApproval)
                .filter(|s| eligible(s))
                .max_by_key(|s| s.updated_at_ms);
            if let Some(session) = approval {
                self.focused = Some(session.id.clone());
                return;
            }
        }

        if let Some(id) = &self.focused {
            if self.sessions.get(id).is_some_and(eligible) {
                return;
            }
        }

        // Skipped while pinned: the whole point is that alt-tabbing no longer
        // moves the deck.
        if controller.is_none() {
            if let Some(app) = &config.frontmost_app {
                let front = self
                    .sessions
                    .values()
                    .filter(|s| same_app(&s.app, app))
                    .max_by_key(|s| s.updated_at_ms);
                if let Some(session) = front {
                    self.focused = Some(session.id.clone());
                    return;
                }
            }
        }

        self.focused = self
            .sessions
            .values()
            .filter(|s| eligible(s))
            .max_by_key(|s| s.updated_at_ms)
            .map(|s| s.id.clone());
    }

    pub fn focused_session(&self) -> Option<&SessionStatus> {
        self.focused.as_ref().and_then(|id| self.sessions.get(id))
    }

    pub fn agent_key_ids(&self, config: &DaemonConfig) -> [Option<String>; 6] {
        let list: Vec<_> = self.sessions.values().cloned().collect();
        key_source::resolve_agent_keys(
            &list,
            self.focused.as_deref(),
            self.active_controller(config),
            config,
        )
    }

    /// Sessions the deck may select, newest first. Honors the controller lock so
    /// joystick cycling cannot walk off the pinned IDE.
    pub fn selectable_sessions(&self, config: &DaemonConfig) -> Vec<SessionStatus> {
        let controller = self.active_controller(config);
        let mut list: Vec<_> = self
            .sessions
            .values()
            .filter(|session| match controller {
                Some(family) => app_family(&session.app) == family,
                None => true,
            })
            .cloned()
            .collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at_ms));
        list
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
            focus_uri: None,
        }
    }

    fn app_session(id: &str, app: &str, state: AgentState, at: u64) -> SessionStatus {
        SessionStatus {
            app: app.into(),
            ..session(id, state, at)
        }
    }

    fn pinned_to(family: &str) -> DaemonConfig {
        DaemonConfig {
            controlling_ide: Some(family.into()),
            ..Default::default()
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
    fn pinned_controller_keeps_the_deck_through_another_ides_approval() {
        let mut registry = Registry::default();
        let config = pinned_to("t3");
        registry.upsert(
            app_session("t1", "T3 Code", AgentState::Working, 1),
            1,
            &config,
        );
        registry.upsert(
            app_session("c1", "Cursor", AgentState::AwaitingApproval, 2),
            1,
            &config,
        );
        // The headline behavior change: approvals elsewhere no longer preempt.
        assert_eq!(registry.focused.as_deref(), Some("t1"));
    }

    #[test]
    fn pinned_controller_still_honors_approvals_inside_its_own_family() {
        let mut registry = Registry::default();
        let config = pinned_to("t3");
        registry.upsert(
            app_session("t1", "T3 Code", AgentState::Working, 1),
            1,
            &config,
        );
        registry.upsert(
            app_session("t2", "T3 Code", AgentState::AwaitingApproval, 2),
            1,
            &config,
        );
        assert_eq!(registry.focused.as_deref(), Some("t2"));
    }

    #[test]
    fn pinned_controller_ignores_the_frontmost_watcher() {
        let mut registry = Registry::default();
        let config = DaemonConfig {
            frontmost_app: Some("Cursor".into()),
            ..pinned_to("t3")
        };
        registry.upsert(
            app_session("c1", "Cursor", AgentState::Working, 2),
            1,
            &config,
        );
        registry.upsert(
            app_session("t1", "T3 Code", AgentState::Working, 1),
            1,
            &config,
        );
        assert_eq!(registry.focused.as_deref(), Some("t1"));
    }

    /// Sessions can reach one family through several adapters — a T3 thread may
    /// arrive from the `t3code` control plane or the `codex` journal watcher.
    /// The lock is on the family, so it must collect both.
    #[test]
    fn pinned_controller_collects_every_provider_for_its_family() {
        let mut registry = Registry::default();
        let config = pinned_to("t3");
        registry.upsert(
            app_session("codex:abc", "T3 Code", AgentState::Working, 1),
            1,
            &config,
        );
        registry.upsert(
            app_session("t3code:xyz", "T3 Code", AgentState::Working, 2),
            1,
            &config,
        );
        registry.upsert(
            app_session("c1", "Cursor", AgentState::Working, 9),
            1,
            &config,
        );

        let selectable = registry.selectable_sessions(&config);
        assert_eq!(selectable.len(), 2);
        assert!(selectable.iter().all(|s| s.app == "T3 Code"));
    }

    #[test]
    fn empty_controller_falls_back_to_most_recent() {
        let mut registry = Registry::default();
        let config = pinned_to("t3");
        registry.upsert(
            app_session("c1", "Cursor", AgentState::Working, 1),
            1,
            &config,
        );
        registry.upsert(
            app_session("c2", "Cursor", AgentState::Working, 2),
            1,
            &config,
        );
        // No T3 thread exists, so the lock yields rather than going dark.
        assert!(registry.active_controller(&config).is_none());
        assert_eq!(registry.focused.as_deref(), Some("c1"));

        // …and reclaims the deck the moment T3 comes back.
        registry.upsert(
            app_session("t1", "T3 Code", AgentState::Working, 3),
            1,
            &config,
        );
        assert_eq!(registry.focused.as_deref(), Some("t1"));
    }

    #[test]
    fn pinned_focus_is_ignored_when_it_points_outside_the_controller() {
        let mut registry = Registry::default();
        let config = DaemonConfig {
            pinned_focus: Some("c1".into()),
            ..pinned_to("t3")
        };
        registry.upsert(
            app_session("c1", "Cursor", AgentState::Working, 2),
            1,
            &config,
        );
        registry.upsert(
            app_session("t1", "T3 Code", AgentState::Working, 1),
            1,
            &config,
        );
        assert_eq!(registry.focused.as_deref(), Some("t1"));
    }

    #[test]
    fn unset_controller_is_never_active() {
        let mut registry = Registry::default();
        let config = DaemonConfig::default();
        registry.upsert(
            app_session("t1", "T3 Code", AgentState::Working, 1),
            1,
            &config,
        );
        assert!(registry.active_controller(&config).is_none());
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
