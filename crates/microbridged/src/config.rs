//! Persistent daemon configuration at `~/.microbridge/config.toml`.

use std::path::{Path, PathBuf};

use mb_protocol::DaemonConfig;
use tracing::{info, warn};

pub fn microbridge_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".microbridge")
}

pub fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("MICROBRIDGE_CONFIG") {
        return PathBuf::from(path);
    }
    microbridge_dir().join("config.toml")
}

pub fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("MICROBRIDGE_SOCKET") {
        return PathBuf::from(path);
    }
    microbridge_dir().join("microbridged.sock")
}

pub fn load_config() -> DaemonConfig {
    load_config_from(&config_path())
}

pub fn load_config_from(path: &Path) -> DaemonConfig {
    match std::fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<DaemonConfig>(&text) {
            Ok(config) => {
                info!(path = %path.display(), "loaded config");
                config
            }
            Err(error) => {
                warn!(%error, path = %path.display(), "invalid config; using defaults");
                DaemonConfig::default()
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DaemonConfig::default(),
        Err(error) => {
            warn!(%error, path = %path.display(), "could not read config; using defaults");
            DaemonConfig::default()
        }
    }
}

/// Write config on change only (caller compares). Event-driven — no timers.
/// `frontmost_app` is runtime-only and is cleared before persist.
pub fn save_config(config: &DaemonConfig) -> std::io::Result<()> {
    let mut to_save = config.clone();
    to_save.frontmost_app = None;
    save_config_to(&config_path(), &to_save)
}

pub fn save_config_to(path: &Path, config: &DaemonConfig) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut to_save = config.clone();
    to_save.frontmost_app = None;
    let text = toml::to_string_pretty(&to_save)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, text)?;
    info!(path = %path.display(), "saved config");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mb_protocol::KeySource;

    #[test]
    fn round_trip_toml() {
        let dir = std::env::temp_dir().join(format!("mb-cfg-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        let config = DaemonConfig {
            key_source: KeySource::FocusedApp,
            pause_leds: true,
            ..Default::default()
        };
        save_config_to(&path, &config).unwrap();
        let loaded = load_config_from(&path);
        assert_eq!(loaded.key_source, KeySource::FocusedApp);
        assert!(loaded.pause_leds);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
