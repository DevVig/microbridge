//! Device abstraction: turns resolved agent state into hardware output.
//!
//! Real Codex Micro HID support lands in M2 (see ROADMAP.md). Until then the
//! daemon drives [`MockDevice`], which logs the frames a real device would
//! render.

use mb_protocol::AgentState;

pub trait Device: Send {
    fn name(&self) -> &str;

    /// Render the focused session's state, or clear the deck when nothing is
    /// focused. Called only on transitions — implementations may assume calls
    /// are rare and need not debounce.
    fn set_state(&mut self, state: Option<AgentState>);
}

/// Logs what a real device would display.
#[derive(Debug, Default)]
pub struct MockDevice {
    last: Option<AgentState>,
}

impl Device for MockDevice {
    fn name(&self) -> &str {
        "mock"
    }

    fn set_state(&mut self, state: Option<AgentState>) {
        if self.last != state {
            tracing::info!(device = self.name(), ?state, "render frame");
            self.last = state;
        }
    }
}
