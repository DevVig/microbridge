//! Auto-discovery engine for local AI agent runtimes.
//!
//! Scans the system for installed IDEs and CLI runtimes on daemon startup,
//! updating the runtime adapter registry with detected/available tools.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::info;

use crate::state::DaemonState;
use mb_protocol::{AdapterCapabilities, AdapterConnectionState};

#[derive(Debug, Clone)]
pub struct DiscoveredRuntime {
    pub id: &'static str,
    pub name: &'static str,
    pub installed: bool,
    pub capabilities: AdapterCapabilities,
}

pub fn scan_runtimes() -> Vec<DiscoveredRuntime> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let home_path = PathBuf::from(&home);

    let cursor_installed = home_path.join(".cursor").exists()
        || home_path
            .join("Library/Application Support/Cursor")
            .exists();

    let opencode_installed = home_path.join(".config/opencode").exists();

    let zed_installed = home_path.join(".config/zed").exists()
        || home_path.join("Library/Application Support/Zed").exists();

    let windsurf_installed = home_path.join(".windsurf").exists()
        || home_path
            .join("Library/Application Support/Windsurf")
            .exists();

    let vscode_installed = home_path.join(".vscode").exists()
        || home_path.join("Library/Application Support/Code").exists();

    let goose_installed = home_path.join(".config/goose").exists();

    let factory_installed = home_path.join(".factory").exists()
        || std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .any(|p| PathBuf::from(p).join("droid").exists());

    vec![
        DiscoveredRuntime {
            id: "cursor",
            name: "Cursor",
            installed: cursor_installed,
            capabilities: AdapterCapabilities {
                lifecycle_observation: true,
                focus_open: true,
                uri_focus: true,
                ..AdapterCapabilities::default()
            },
        },
        DiscoveredRuntime {
            id: "opencode",
            name: "OpenCode",
            installed: opencode_installed,
            capabilities: AdapterCapabilities {
                lifecycle_observation: true,
                interrupt: true,
                approval_acceptance: true,
                approval_rejection: true,
                ..AdapterCapabilities::default()
            },
        },
        DiscoveredRuntime {
            id: "zed",
            name: "Zed",
            installed: zed_installed,
            capabilities: AdapterCapabilities {
                lifecycle_observation: true,
                focus_open: true,
                uri_focus: true,
                ..AdapterCapabilities::default()
            },
        },
        DiscoveredRuntime {
            id: "windsurf",
            name: "Windsurf",
            installed: windsurf_installed,
            capabilities: AdapterCapabilities {
                lifecycle_observation: true,
                focus_open: true,
                uri_focus: true,
                ..AdapterCapabilities::default()
            },
        },
        DiscoveredRuntime {
            id: "vscode",
            name: "VS Code",
            installed: vscode_installed,
            capabilities: AdapterCapabilities {
                lifecycle_observation: true,
                focus_open: true,
                uri_focus: true,
                ..AdapterCapabilities::default()
            },
        },
        DiscoveredRuntime {
            id: "goose",
            name: "Goose AI",
            installed: goose_installed,
            capabilities: AdapterCapabilities {
                lifecycle_observation: true,
                mcp_native: true,
                ..AdapterCapabilities::default()
            },
        },
        DiscoveredRuntime {
            id: "factory",
            name: "Factory (Droid)",
            installed: factory_installed,
            capabilities: AdapterCapabilities {
                lifecycle_observation: true,
                interrupt: true,
                reasoning_effort: true,
                ..AdapterCapabilities::default()
            },
        },
    ]
}

pub fn spawn_auto_discovery(shared: Arc<Mutex<DaemonState>>) {
    tokio::spawn(async move {
        // Initial scan after daemon startup delay
        tokio::time::sleep(Duration::from_millis(500)).await;
        let runtimes = scan_runtimes();

        let mut state = shared.lock().await;
        for rt in runtimes {
            if rt.installed {
                info!(
                    id = rt.id,
                    name = rt.name,
                    "Auto-discovered installed agent runtime"
                );
                let current = state.adapter_enabled(rt.id);
                if !current {
                    state.set_adapter_runtime(
                        rt.id,
                        AdapterConnectionState::NeedsSetup,
                        rt.capabilities,
                        format!("{} detected on local machine.", rt.name),
                    );
                }
            }
        }
    });
}
