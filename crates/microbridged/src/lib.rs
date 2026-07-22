//! microbridged library — status bus, focus policy, key source, socket server.

pub mod app_match;
pub mod auto_discover;
pub mod claude_control;
pub mod cnvs;
pub mod codex_control;
pub mod config;
pub mod cursor_acp;
pub mod factory;
pub mod frontmost;
pub mod key_source;
pub mod mcp;
pub mod registry;
pub mod socket;
pub mod state;
pub mod t3code;

pub use config::{config_path, load_config, save_config};
pub use registry::Registry;
pub use state::DaemonState;
