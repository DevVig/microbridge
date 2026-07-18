//! microbridged library — status bus, focus policy, key source, socket server.

pub mod config;
pub mod frontmost;
pub mod key_source;
pub mod registry;
pub mod socket;
pub mod state;
pub mod t3code;

pub use config::{config_path, load_config, save_config};
pub use registry::Registry;
pub use state::DaemonState;
