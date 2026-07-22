//! Tauri menu bar companion — talks to microbridged over the local Unix socket.
//! Never opens HID; the daemon owns the device.

mod bus;

use std::fs::{self, OpenOptions};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use bus::{apply_event, spawn_bus_loop, BusHandle, CachedSnapshot};
use mb_protocol::{BusEvent, ClientMessage, DaemonConfig, ServerMessage, Snapshot};
use tauri::{
    menu::{ContextMenu, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewWindow,
};
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::Mutex;

struct AppState {
    bus: BusHandle,
    snapshot: CachedSnapshot,
    bundled_daemon: StdMutex<Option<Child>>,
}

fn daemon_socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("MICROBRIDGE_SOCKET") {
        return PathBuf::from(path);
    }
    let user_home = std::env::var_os("HOME").unwrap_or_else(|| ".".into());
    PathBuf::from(user_home)
        .join(".microbridge")
        .join("microbridged.sock")
}

fn daemon_is_reachable() -> bool {
    StdUnixStream::connect(daemon_socket_path()).is_ok()
}

/// Direct-download builds carry `microbridged` beside the UI executable. If a
/// Homebrew/launchd daemon is already reachable we leave it alone; otherwise
/// the app owns this child for its lifetime.
fn start_bundled_daemon() -> Option<Child> {
    if daemon_is_reachable() {
        return None;
    }
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("microbridged"));
        }
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/microbridged"),
        PathBuf::from("/usr/local/bin/microbridged"),
    ]);
    let binary = candidates
        .into_iter()
        .find(|candidate| candidate.is_file())?;

    let log_path = daemon_socket_path().with_file_name("microbridged-app.log");
    if let Some(directory) = log_path.parent() {
        let _ = fs::create_dir_all(directory);
    }
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok()?;
    let stderr = stdout.try_clone().ok()?;
    Command::new(binary)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .ok()
}

const CURSOR_PLUGIN_NAME: &str = "microbridge";
const CURSOR_PLUGIN_MARKER: &str = ".microbridge-owned";
static CURSOR_PLUGIN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn validate_cursor_plugin(path: &Path) -> Result<(), String> {
    let manifest_path = path.join(".cursor-plugin/plugin.json");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest)
        .map_err(|e| format!("parse {}: {e}", manifest_path.display()))?;
    if manifest.get("name").and_then(|value| value.as_str()) != Some(CURSOR_PLUGIN_NAME) {
        return Err(format!(
            "{} is not the Microbridge Cursor integration",
            path.display()
        ));
    }
    for relative in [
        "hooks/hooks.json",
        "hooks/microbridge-event.mjs",
        "hooks/event.mjs",
    ] {
        if !path.join(relative).is_file() {
            return Err(format!("Cursor integration is missing {relative}"));
        }
    }
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|e| format!("create {}: {e}", destination.display()))?;
    for entry in fs::read_dir(source).map_err(|e| format!("read {}: {e}", source.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)
                .map_err(|e| format!("copy {}: {e}", target.display()))?;
        }
    }
    Ok(())
}

fn cursor_plugin_source(app: &AppHandle) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("cursor-plugin");
    if validate_cursor_plugin(&bundled).is_ok() {
        return Ok(bundled);
    }

    // `tauri dev` reads the repository copy; release bundles use Resources.
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../adapters/cursor");
    validate_cursor_plugin(&repository)?;
    Ok(repository)
}

fn cursor_plugin_destination() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_string())?;
    Ok(PathBuf::from(home)
        .join(".cursor/plugins/local")
        .join(CURSOR_PLUGIN_NAME))
}

fn install_cursor_integration(app: &AppHandle) -> Result<PathBuf, String> {
    let source = cursor_plugin_source(app)?;
    let destination = cursor_plugin_destination()?;
    install_cursor_integration_at(
        &source,
        &destination,
        &app.package_info().version.to_string(),
    )?;
    Ok(destination)
}

fn install_cursor_integration_at(
    source: &Path,
    destination: &Path,
    version: &str,
) -> Result<(), String> {
    let _operation = CURSOR_PLUGIN_LOCK
        .lock()
        .map_err(|_| "Cursor integration installer lock is unavailable".to_string())?;
    validate_cursor_plugin(source)?;
    let parent = destination
        .parent()
        .ok_or_else(|| "Cursor plugin destination has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;

    if destination.exists() {
        if !destination.join(CURSOR_PLUGIN_MARKER).is_file() {
            return Err(format!(
                "Preserving unowned Cursor plugin at {}. Move it aside before enabling Microbridge.",
                destination.display()
            ));
        }
        validate_cursor_plugin(destination)?;
    }

    let pid = std::process::id();
    let staging = parent.join(format!(".{CURSOR_PLUGIN_NAME}-installing-{pid}"));
    let backup = parent.join(format!(".{CURSOR_PLUGIN_NAME}-backup-{pid}"));
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|e| format!("remove stale {}: {e}", staging.display()))?;
    }
    if backup.exists() {
        fs::remove_dir_all(&backup)
            .map_err(|e| format!("remove stale {}: {e}", backup.display()))?;
    }

    copy_dir(source, &staging)?;
    fs::write(
        staging.join(CURSOR_PLUGIN_MARKER),
        format!("Microbridge {version}\n"),
    )
    .map_err(|e| format!("write ownership marker: {e}"))?;
    validate_cursor_plugin(&staging)?;

    if destination.exists() {
        fs::rename(destination, &backup)
            .map_err(|e| format!("prepare Cursor integration update: {e}"))?;
    }
    if let Err(error) = fs::rename(&staging, destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(format!("install Cursor integration: {error}"));
    }
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|e| format!("remove old Cursor integration: {e}"))?;
    }
    Ok(())
}

fn remove_cursor_integration() -> Result<bool, String> {
    let destination = cursor_plugin_destination()?;
    remove_cursor_integration_at(&destination)
}

fn remove_cursor_integration_at(destination: &Path) -> Result<bool, String> {
    let _operation = CURSOR_PLUGIN_LOCK
        .lock()
        .map_err(|_| "Cursor integration installer lock is unavailable".to_string())?;
    if !destination.exists() {
        return Ok(false);
    }
    if !destination.join(CURSOR_PLUGIN_MARKER).is_file() {
        return Err(format!(
            "Preserving unowned Cursor plugin at {}. Remove it manually if that is intended.",
            destination.display()
        ));
    }
    validate_cursor_plugin(destination)?;
    fs::remove_dir_all(destination)
        .map_err(|e| format!("remove {}: {e}", destination.display()))?;
    Ok(true)
}

#[cfg(test)]
mod cursor_integration_tests {
    use super::*;

    #[test]
    fn installs_updates_and_removes_only_the_microbridge_plugin() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "microbridge-cursor-installer-{}-{nonce}",
            std::process::id()
        ));
        let destination = root.join("local/microbridge");
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../adapters/cursor");

        install_cursor_integration_at(&source, &destination, "0.2.1").unwrap();
        validate_cursor_plugin(&destination).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join(CURSOR_PLUGIN_MARKER)).unwrap(),
            "Microbridge 0.2.1\n"
        );

        install_cursor_integration_at(&source, &destination, "0.2.2").unwrap();
        assert_eq!(
            fs::read_to_string(destination.join(CURSOR_PLUGIN_MARKER)).unwrap(),
            "Microbridge 0.2.2\n"
        );
        assert!(remove_cursor_integration_at(&destination).unwrap());
        assert!(!destination.exists());
        assert!(!remove_cursor_integration_at(&destination).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preserves_an_unowned_cursor_plugin() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "microbridge-unowned-cursor-plugin-{}-{nonce}",
            std::process::id()
        ));
        let destination = root.join("local/microbridge");
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../adapters/cursor");
        copy_dir(&source, &destination).unwrap();

        assert!(install_cursor_integration_at(&source, &destination, "0.2.1").is_err());
        assert!(!destination.join(CURSOR_PLUGIN_MARKER).exists());
        assert!(remove_cursor_integration_at(&destination).is_err());
        assert!(destination.join(".cursor-plugin/plugin.json").is_file());
        let _ = fs::remove_dir_all(root);
    }
}

fn sync_cursor_integration(app: &AppHandle, synced: &AtomicBool, enabled: bool) {
    if !enabled {
        synced.store(false, Ordering::Relaxed);
        return;
    }
    if !synced.swap(true, Ordering::Relaxed) && install_cursor_integration(app).is_err() {
        // Retry on the next daemon snapshot or config event. Settings reports
        // explicit installation errors to the user.
        synced.store(false, Ordering::Relaxed);
    }
}

const FACTORY_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "Notification",
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "SessionEnd",
];
static FACTORY_HOOK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn factory_hooks_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_string())?;
    Ok(PathBuf::from(home).join(".factory/hooks.json"))
}

fn factory_bridge_destination() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_string())?;
    Ok(PathBuf::from(home).join(".microbridge/integrations/factory/microbridgectl"))
}

fn microbridgectl_source() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("microbridgectl"));
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("../../../target/debug/microbridgectl"));
    candidates.push(manifest.join("../../../target/release/microbridgectl"));
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/bin/microbridgectl"));
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/microbridgectl"),
        PathBuf::from("/usr/local/bin/microbridgectl"),
    ]);
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "The bundled microbridgectl helper is missing. Reinstall Microbridge.".into()
        })
}

fn factory_hook_command(binary: &Path) -> String {
    let escaped = binary.to_string_lossy().replace('\'', "'\\''");
    format!("'{escaped}' factory-event")
}

fn merge_factory_hooks(
    root: &mut serde_json::Value,
    command: &str,
    install: bool,
) -> Result<(), String> {
    if !root.is_object() {
        *root = serde_json::json!({});
    }
    let hooks = root
        .as_object_mut()
        .expect("object initialized")
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    if !hooks.is_object() {
        return Err(
            "Factory hooks.json has a non-object `hooks` value; preserving it unchanged.".into(),
        );
    }
    let hooks = hooks.as_object_mut().expect("validated object");
    for event in FACTORY_HOOK_EVENTS {
        let groups = hooks
            .entry((*event).to_string())
            .or_insert_with(|| serde_json::json!([]));
        let groups = groups.as_array_mut().ok_or_else(|| {
            format!("Factory {event} hooks are not an array; preserving them unchanged.")
        })?;
        for group in groups.iter_mut() {
            let Some(commands) = group
                .get_mut("hooks")
                .and_then(|value| value.as_array_mut())
            else {
                continue;
            };
            commands.retain(|hook| {
                hook.get("command")
                    .and_then(|value| value.as_str())
                    .is_none_or(|existing| existing != command)
            });
        }
        groups.retain(|group| {
            group
                .get("hooks")
                .and_then(|value| value.as_array())
                .is_none_or(|commands| !commands.is_empty())
        });
        if install {
            groups.push(serde_json::json!({
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": command,
                    "timeout": 10
                }]
            }));
        }
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "hooks path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let staging = path.with_extension("json.microbridge.tmp");
    let body = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(&staging, body).map_err(|error| format!("write {}: {error}", staging.display()))?;
    fs::rename(&staging, path).map_err(|error| format!("replace {}: {error}", path.display()))
}

fn install_factory_integration() -> Result<PathBuf, String> {
    let _operation = FACTORY_HOOK_LOCK
        .lock()
        .map_err(|_| "Factory integration installer lock is unavailable".to_string())?;
    let source = microbridgectl_source()?;
    let destination = factory_bridge_destination()?;
    let hooks_path = factory_hooks_path()?;
    let mut hooks = if hooks_path.is_file() {
        let body = fs::read_to_string(&hooks_path)
            .map_err(|error| format!("read {}: {error}", hooks_path.display()))?;
        serde_json::from_str(&body)
            .map_err(|error| format!("parse {}: {error}", hooks_path.display()))?
    } else {
        serde_json::json!({})
    };
    merge_factory_hooks(&mut hooks, &factory_hook_command(&destination), true)?;
    let parent = destination
        .parent()
        .ok_or_else(|| "Factory helper has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let staging = destination.with_extension("installing");
    fs::copy(&source, &staging).map_err(|error| format!("copy Factory helper: {error}"))?;
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("make Factory helper executable: {error}"))?;
    let backup = destination.with_extension("backup");
    if backup.exists() {
        fs::remove_file(&backup)
            .map_err(|error| format!("remove stale {}: {error}", backup.display()))?;
    }
    if destination.exists() {
        fs::rename(&destination, &backup)
            .map_err(|error| format!("prepare Factory helper update: {error}"))?;
    }
    if let Err(error) = fs::rename(&staging, &destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, &destination);
        }
        return Err(format!("install Factory helper: {error}"));
    }
    if let Err(error) = write_json_atomic(&hooks_path, &hooks) {
        let _ = fs::remove_file(&destination);
        if backup.exists() {
            let _ = fs::rename(&backup, &destination);
        }
        return Err(error);
    }
    if backup.exists() {
        fs::remove_file(&backup).map_err(|error| format!("remove old Factory helper: {error}"))?;
    }
    Ok(hooks_path)
}

fn remove_factory_integration() -> Result<bool, String> {
    let _operation = FACTORY_HOOK_LOCK
        .lock()
        .map_err(|_| "Factory integration installer lock is unavailable".to_string())?;
    let destination = factory_bridge_destination()?;
    let hooks_path = factory_hooks_path()?;
    let mut changed = false;
    if hooks_path.is_file() {
        let body = fs::read_to_string(&hooks_path)
            .map_err(|error| format!("read {}: {error}", hooks_path.display()))?;
        let mut hooks: serde_json::Value = serde_json::from_str(&body)
            .map_err(|error| format!("parse {}: {error}", hooks_path.display()))?;
        let before = hooks.clone();
        merge_factory_hooks(&mut hooks, &factory_hook_command(&destination), false)?;
        if hooks != before {
            write_json_atomic(&hooks_path, &hooks)?;
            changed = true;
        }
    }
    if destination.is_file() {
        fs::remove_file(&destination)
            .map_err(|error| format!("remove {}: {error}", destination.display()))?;
        changed = true;
    }
    Ok(changed)
}

fn sync_factory_integration(synced: &AtomicBool, enabled: bool) {
    if !enabled {
        synced.store(false, Ordering::Relaxed);
        return;
    }
    if !synced.swap(true, Ordering::Relaxed) && install_factory_integration().is_err() {
        synced.store(false, Ordering::Relaxed);
    }
}

/// Install Claude PermissionRequest hooks under ~/.microbridge/claude-hooks
/// and merge into ~/.claude/settings.json (idempotent, lean — only writes when needed).
fn claude_hook_source(app: &AppHandle) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("claude-hooks")
        .join("microbridge-permission.mjs");
    if bundled.is_file() {
        return Ok(bundled);
    }
    // `tauri dev` reads the repository copy; release bundles use Resources.
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../adapters/claude/hooks/microbridge-permission.mjs");
    if repository.is_file() {
        return Ok(repository);
    }
    Err("Claude hook script not found in the Microbridge bundle.".into())
}

fn install_claude_hooks(app: &AppHandle) -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is unset".to_string())?;
    let hook_dir = PathBuf::from(&home).join(".microbridge").join("claude-hooks");
    fs::create_dir_all(&hook_dir).map_err(|e| e.to_string())?;
    let dest = hook_dir.join("microbridge-permission.mjs");
    let source = claude_hook_source(app)?;
    let body = fs::read_to_string(&source).map_err(|e| e.to_string())?;
    if fs::read_to_string(&dest).ok().as_deref() != Some(body.as_str()) {
        fs::write(&dest, &body).map_err(|e| e.to_string())?;
    }
    let command = format!(
        "node \"{}\" permission",
        dest.to_string_lossy().replace('"', "\\\"")
    );
    let pretool = format!(
        "node \"{}\" pretool",
        dest.to_string_lossy().replace('"', "\\\"")
    );
    let settings_path = PathBuf::from(&home).join(".claude").join("settings.json");
    let mut settings = if settings_path.is_file() {
        let text = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text)
            .map_err(|e| format!("parse {}: {e}", settings_path.display()))?
    } else {
        serde_json::json!({})
    };
    let hooks = settings
        .as_object_mut()
        .ok_or_else(|| "Claude settings.json root must be an object".to_string())?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or_else(|| "Claude settings hooks must be an object".to_string())?;
    let permission_entry = serde_json::json!([{
        "hooks": [{ "type": "command", "command": command }]
    }]);
    let pretool_entry = serde_json::json!([{
        "hooks": [{ "type": "command", "command": pretool }]
    }]);
    let mut changed = false;
    let mb_owned_or_empty = |existing: &serde_json::Value| {
        let text = existing.to_string();
        text.contains("microbridge-permission")
            || existing.as_array().is_some_and(|a| a.is_empty())
    };
    if hooks_obj.get("PermissionRequest") != Some(&permission_entry) {
        let existing = hooks_obj
            .get("PermissionRequest")
            .cloned()
            .unwrap_or(serde_json::json!([]));
        if mb_owned_or_empty(&existing) {
            hooks_obj.insert("PermissionRequest".into(), permission_entry);
            changed = true;
        }
    }
    if hooks_obj.get("PreToolUse") != Some(&pretool_entry) {
        // Merge carefully: only replace if empty or already Microbridge-owned.
        let existing = hooks_obj
            .get("PreToolUse")
            .cloned()
            .unwrap_or(serde_json::json!([]));
        if mb_owned_or_empty(&existing) {
            hooks_obj.insert("PreToolUse".into(), pretool_entry);
            changed = true;
        }
    }
    if changed {
        write_json_atomic(&settings_path, &settings)?;
    }
    Ok(dest)
}

fn sync_claude_hooks(app: &AppHandle, synced: &AtomicBool) {
    if !synced.swap(true, Ordering::Relaxed) && install_claude_hooks(app).is_err() {
        synced.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod factory_integration_tests {
    use super::*;

    #[test]
    fn merges_and_removes_only_microbridge_factory_hooks() {
        let command = "'/tmp/microbridgectl' factory-event";
        let mut value = serde_json::json!({
            "hooks": {
                "Stop": [{"matcher":"*","hooks":[{"type":"command","command":"keep-me"}]}]
            }
        });
        merge_factory_hooks(&mut value, command, true).unwrap();
        for event in FACTORY_HOOK_EVENTS {
            assert!(value["hooks"][event]
                .as_array()
                .unwrap()
                .iter()
                .any(|group| group["hooks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|hook| hook["command"] == command)));
        }
        merge_factory_hooks(&mut value, command, false).unwrap();
        assert_eq!(value["hooks"]["Stop"][0]["hooks"][0]["command"], "keep-me");
        assert!(!value.to_string().contains(command));
    }
}

const OPENCODE_PLUGIN_MARKER: &str = "MICROBRIDGE_OPENCODE_PLUGIN";
const OPENCODE_PLUGIN_NAME: &str = "microbridge.mjs";
static OPENCODE_PLUGIN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn validate_opencode_plugin(path: &Path) -> Result<String, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    if !source.contains(OPENCODE_PLUGIN_MARKER) || !source.contains("export const Microbridge") {
        return Err(format!(
            "{} is not the Microbridge OpenCode integration",
            path.display()
        ));
    }
    Ok(source)
}

fn opencode_plugin_source(app: &AppHandle) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join("opencode-plugin")
        .join(OPENCODE_PLUGIN_NAME);
    if validate_opencode_plugin(&bundled).is_ok() {
        return Ok(bundled);
    }
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../adapters/opencode")
        .join(OPENCODE_PLUGIN_NAME);
    validate_opencode_plugin(&repository)?;
    Ok(repository)
}

fn opencode_plugin_destination() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is unavailable".to_string())?;
    Ok(PathBuf::from(home)
        .join(".config/opencode/plugins")
        .join(OPENCODE_PLUGIN_NAME))
}

fn install_opencode_integration(app: &AppHandle) -> Result<PathBuf, String> {
    let source = opencode_plugin_source(app)?;
    let destination = opencode_plugin_destination()?;
    install_opencode_integration_at(
        &source,
        &destination,
        &app.package_info().version.to_string(),
    )?;
    Ok(destination)
}

fn install_opencode_integration_at(
    source: &Path,
    destination: &Path,
    version: &str,
) -> Result<(), String> {
    let _operation = OPENCODE_PLUGIN_LOCK
        .lock()
        .map_err(|_| "OpenCode integration installer lock is unavailable".to_string())?;
    let template = validate_opencode_plugin(source)?;
    if destination.is_file() {
        validate_opencode_plugin(destination).map_err(|_| {
            format!(
                "Preserving unowned OpenCode plugin at {}. Move it aside before enabling Microbridge.",
                destination.display()
            )
        })?;
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "OpenCode plugin destination has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let rendered = template.replace("__MICROBRIDGE_VERSION__", version);
    let staging = destination.with_extension("mjs.microbridge.tmp");
    fs::write(&staging, rendered)
        .map_err(|error| format!("write {}: {error}", staging.display()))?;
    validate_opencode_plugin(&staging)?;
    fs::rename(&staging, destination)
        .map_err(|error| format!("install OpenCode integration: {error}"))
}

fn remove_opencode_integration() -> Result<bool, String> {
    let destination = opencode_plugin_destination()?;
    remove_opencode_integration_at(&destination)
}

fn remove_opencode_integration_at(destination: &Path) -> Result<bool, String> {
    let _operation = OPENCODE_PLUGIN_LOCK
        .lock()
        .map_err(|_| "OpenCode integration installer lock is unavailable".to_string())?;
    if !destination.exists() {
        return Ok(false);
    }
    validate_opencode_plugin(destination).map_err(|_| {
        format!(
            "Preserving unowned OpenCode plugin at {}.",
            destination.display()
        )
    })?;
    fs::remove_file(destination)
        .map_err(|error| format!("remove {}: {error}", destination.display()))?;
    Ok(true)
}

fn sync_opencode_integration(app: &AppHandle, synced: &AtomicBool, enabled: bool) {
    if !enabled {
        synced.store(false, Ordering::Relaxed);
        return;
    }
    if !synced.swap(true, Ordering::Relaxed) && install_opencode_integration(app).is_err() {
        synced.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod opencode_integration_tests {
    use super::*;

    #[test]
    fn installs_updates_and_removes_only_owned_opencode_plugin() {
        let root =
            std::env::temp_dir().join(format!("microbridge-opencode-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.mjs");
        let destination = root.join("plugins/microbridge.mjs");
        fs::write(
            &source,
            "// MICROBRIDGE_OPENCODE_PLUGIN\nconst v='__MICROBRIDGE_VERSION__';\nexport const Microbridge = async () => ({})\n",
        )
        .unwrap();

        install_opencode_integration_at(&source, &destination, "0.4.0").unwrap();
        let installed = fs::read_to_string(&destination).unwrap();
        assert!(installed.contains("0.4.0"));
        assert!(!installed.contains("__MICROBRIDGE_VERSION__"));
        assert!(remove_opencode_integration_at(&destination).unwrap());
        assert!(!destination.exists());

        fs::write(&destination, "export const SomeoneElse = {}\n").unwrap();
        assert!(install_opencode_integration_at(&source, &destination, "0.4.1").is_err());
        assert!(remove_opencode_integration_at(&destination).is_err());
        assert_eq!(
            fs::read_to_string(&destination).unwrap(),
            "export const SomeoneElse = {}\n"
        );
        let _ = fs::remove_dir_all(&root);
    }
}

fn physical_tray_rect(rect: &tauri::Rect, scale: f64) -> (f64, f64, f64, f64) {
    let (x, y) = match rect.position {
        Position::Physical(p) => (p.x as f64, p.y as f64),
        Position::Logical(p) => (p.x * scale, p.y * scale),
    };
    let (w, h) = match rect.size {
        Size::Physical(s) => (s.width as f64, s.height as f64),
        Size::Logical(s) => (s.width * scale, s.height * scale),
    };
    (x, y, w, h)
}

/// Popover geometry, in logical pixels.
const POPOVER_WIDTH: f64 = 380.0;
/// Below this the card has nothing useful to show, so never shrink past it.
const POPOVER_MIN_HEIGHT: f64 = 160.0;
/// Ceiling, not a target — the webview drives the real height via
/// `resize_popover`, so the window is usually well under this. Sized to clear
/// the tallest the card can legitimately be: ~438px of chrome (header, focused
/// thread, the device echo at ~150px, the simulator note, footer) plus the
/// 10-row thread viewport, plus the shadow slack the webview adds.
const POPOVER_MAX_HEIGHT: f64 = 780.0;
/// Gap between the menu bar icon and the top of the popover window.
const POPOVER_TRAY_GAP: f64 = 6.0;
/// Breathing room left between the popover and the bottom of the work area.
const POPOVER_BOTTOM_MARGIN: f64 = 12.0;

/// Scale factor for `window`, falling back to its monitor and finally to 1.0.
///
/// Assuming Retina would place the popover at half the intended offset on a 1x
/// external display.
fn window_scale(window: &WebviewWindow) -> f64 {
    window
        .scale_factor()
        .ok()
        .or_else(|| {
            window
                .current_monitor()
                .ok()
                .flatten()
                .map(|m| m.scale_factor())
        })
        .unwrap_or(1.0)
}

/// Logical height available between `top_y` (physical, in screen coordinates)
/// and the bottom of the monitor's work area — which on macOS already excludes
/// the menu bar and the Dock, so the popover lands above the Dock rather than
/// behind it. Falls back to the maximum when the monitor can't be read.
fn available_popover_height(window: &WebviewWindow, top_y: i32) -> f64 {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return POPOVER_MAX_HEIGHT;
    };
    let work = monitor.work_area();
    let bottom = work.position.y + work.size.height as i32;
    let room = f64::from(bottom - top_y) / window_scale(window) - POPOVER_BOTTOM_MARGIN;
    room.clamp(POPOVER_MIN_HEIGHT, POPOVER_MAX_HEIGHT)
}

/// Room below the popover's current top edge, recomputed from where the window
/// actually sits — so moving between monitors needs no shared state.
fn popover_available_height(window: &WebviewWindow) -> f64 {
    window
        .outer_position()
        .map(|pos| available_popover_height(window, pos.y))
        .unwrap_or(POPOVER_MAX_HEIGHT)
}

fn position_below_tray(window: &WebviewWindow, tray_x: f64, tray_y: f64, tray_w: f64, tray_h: f64) {
    let scale = window_scale(window);
    let width = POPOVER_WIDTH * scale;
    let mut x = tray_x + tray_w / 2.0 - width / 2.0;
    if let Ok(Some(monitor)) = window.current_monitor() {
        let origin = monitor.position();
        let screen = monitor.size();
        let min_x = f64::from(origin.x);
        let max_x = min_x + f64::from(screen.width) - width;
        x = x.clamp(min_x + 8.0, max_x - 8.0);
    }
    let y = (tray_y + tray_h + POPOVER_TRAY_GAP * scale).round() as i32;
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
        x.round() as i32,
        y,
    )));
    // Size to the room actually left below the menu bar *before* showing, so
    // the popover can't hang off the bottom of the screen even if the webview
    // hasn't reported its content height yet.
    let _ = window.set_size(LogicalSize::new(
        POPOVER_WIDTH,
        available_popover_height(window, y),
    ));
}

/// When the popover last hid itself because it lost focus. Shared by the blur
/// handler and the tray click handler; see `toggle_popover`.
type BlurHideClock = Arc<std::sync::Mutex<Option<std::time::Instant>>>;

/// A click on the tray icon steals focus from the popover on mouse *down*,
/// which fires `Focused(false)` and hides it — before the tray's mouse *up*
/// click event arrives. By then the window reads as hidden, so a naive toggle
/// would reopen it and the popover could never be dismissed by clicking the
/// icon. Treat a click landing right after a blur-hide as the dismiss it was.
const BLUR_HIDE_DISMISS_WINDOW: Duration = Duration::from_millis(250);

fn toggle_popover(
    app: &AppHandle,
    tray_x: f64,
    tray_y: f64,
    tray_w: f64,
    tray_h: f64,
    blur_hide: &BlurHideClock,
) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    let just_blurred = blur_hide
        .lock()
        .ok()
        .and_then(|t| *t)
        .is_some_and(|t| t.elapsed() < BLUR_HIDE_DISMISS_WINDOW);
    if just_blurred {
        return;
    }
    position_below_tray(&window, tray_x, tray_y, tray_w, tray_h);
    // The cap depends on the monitor the popover just landed on, so hand the
    // webview the new one before it paints.
    let _ = window.emit("popover-fit", popover_available_height(&window));
    let _ = window.show();
    let _ = window.set_focus();
}

fn show_hud(app: &AppHandle, generation: Arc<AtomicU64>) {
    let Some(window) = app.get_webview_window("hud") else {
        return;
    };
    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen = monitor.size();
        let origin = monitor.position();
        let Ok(size) = window.outer_size() else {
            return;
        };
        let x = origin.x + ((screen.width.saturating_sub(size.width)) / 2) as i32;
        let y = origin.y + (screen.height as f64 * 0.12).round() as i32;
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    }
    let token = generation.fetch_add(1, Ordering::Relaxed) + 1;
    let _ = window.show();
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(2500)).await;
        if generation.load(Ordering::Relaxed) == token {
            if let Some(hud) = app2.get_webview_window("hud") {
                let _ = hud.hide();
            }
        }
    });
}

#[tauri::command]
async fn get_snapshot(state: tauri::State<'_, AppState>) -> Result<Snapshot, String> {
    let guard = state.snapshot.lock().await;
    guard
        .clone()
        .ok_or_else(|| "waiting for microbridged".into())
}

#[tauri::command]
async fn set_config(
    config: DaemonConfig,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<DaemonConfig, String> {
    let next = state.bus.set_config(config).await?;
    {
        let mut snap = state.snapshot.lock().await;
        if let Some(s) = snap.as_mut() {
            s.config = next.clone();
            let payload = s.clone();
            let _ = app.emit("bus-snapshot", payload);
        }
    }
    Ok(next)
}

#[tauri::command]
async fn set_adapter_enabled(
    adapter_id: String,
    enabled: bool,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let was_enabled = state
        .snapshot
        .lock()
        .await
        .as_ref()
        .and_then(|snapshot| snapshot.config.adapters.get(&adapter_id))
        .map(|preference| preference.enabled)
        .unwrap_or(false);
    let message = state
        .bus
        .adapter_operation(ClientMessage::SetAdapterEnabled {
            adapter_id: adapter_id.clone(),
            enabled,
        })
        .await?;
    if adapter_id == "cursor" && enabled {
        match install_cursor_integration(&app) {
            Ok(path) => {
                return Ok(format!(
                    "{message} Cursor integration installed from Microbridge at {}. Reload Cursor once if it is already open.",
                    path.display()
                ));
            }
            Err(error) => {
                if !was_enabled {
                    let _ = state
                        .bus
                        .adapter_operation(ClientMessage::SetAdapterEnabled {
                            adapter_id: adapter_id.clone(),
                            enabled: false,
                        })
                        .await;
                }
                return Err(format!("Cursor was not enabled because its bundled integration could not be installed: {error}"));
            }
        }
    }
    if adapter_id == "factory" && enabled {
        match install_factory_integration() {
            Ok(path) => {
                return Ok(format!(
                    "{message} Factory lifecycle hooks installed in {}. New and active Droid sessions will appear automatically.",
                    path.display()
                ));
            }
            Err(error) => {
                if !was_enabled {
                    let _ = state
                        .bus
                        .adapter_operation(ClientMessage::SetAdapterEnabled {
                            adapter_id: adapter_id.clone(),
                            enabled: false,
                        })
                        .await;
                }
                return Err(format!(
                    "Factory was not enabled because its hooks could not be installed: {error}"
                ));
            }
        }
    }
    if adapter_id == "opencode" && enabled {
        match install_opencode_integration(&app) {
            Ok(path) => {
                return Ok(format!(
                    "{message} OpenCode integration installed at {}. Restart OpenCode once if it is already running.",
                    path.display()
                ));
            }
            Err(error) => {
                if !was_enabled {
                    let _ = state
                        .bus
                        .adapter_operation(ClientMessage::SetAdapterEnabled {
                            adapter_id: adapter_id.clone(),
                            enabled: false,
                        })
                        .await;
                }
                return Err(format!(
                    "OpenCode was not enabled because its bundled integration could not be installed: {error}"
                ));
            }
        }
    }
    Ok(message)
}

#[tauri::command]
async fn pair_adapter(
    adapter_id: String,
    pairing_url: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state
        .bus
        .adapter_operation(ClientMessage::PairAdapter {
            adapter_id,
            pairing_url,
        })
        .await
}

#[tauri::command]
async fn forget_adapter(
    adapter_id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let was_enabled = state
        .snapshot
        .lock()
        .await
        .as_ref()
        .and_then(|snapshot| snapshot.config.adapters.get(&adapter_id))
        .map(|preference| preference.enabled)
        .unwrap_or(false);
    let removed = match adapter_id.as_str() {
        "cursor" => remove_cursor_integration()?,
        "factory" => remove_factory_integration()?,
        "opencode" => remove_opencode_integration()?,
        _ => false,
    };
    let operation = state
        .bus
        .adapter_operation(ClientMessage::ForgetAdapter {
            adapter_id: adapter_id.clone(),
        })
        .await;
    let message = match operation {
        Ok(message) => message,
        Err(error) => {
            if removed {
                if adapter_id == "cursor" {
                    let _ = install_cursor_integration(&app);
                } else if adapter_id == "factory" {
                    let _ = install_factory_integration();
                } else if adapter_id == "opencode" {
                    let _ = install_opencode_integration(&app);
                }
            }
            if was_enabled {
                let _ = state
                    .bus
                    .adapter_operation(ClientMessage::SetAdapterEnabled {
                        adapter_id: adapter_id.clone(),
                        enabled: true,
                    })
                    .await;
            }
            return Err(format!(
                "Adapter removal failed and the prior state was restored: {error}"
            ));
        }
    };
    if adapter_id == "cursor" && removed {
        return Ok(format!(
            "{message} The bundled Cursor integration was removed. Reload Cursor once if it is already open."
        ));
    }
    if adapter_id == "factory" && removed {
        return Ok(format!(
            "{message} The Microbridge-owned Factory hooks and helper were removed."
        ));
    }
    if adapter_id == "opencode" && removed {
        return Ok(format!(
            "{message} The Microbridge-owned OpenCode integration was removed. Restart OpenCode if it is running."
        ));
    }
    Ok(message)
}

#[tauri::command]
async fn activate_agent_key(
    index: usize,
    open: bool,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state
        .bus
        .adapter_operation(ClientMessage::ActivateAgentKey { index, open })
        .await
}

/// Show the settings window (and hide the popover). Shared by the `open_settings`
/// command and the tray right-click menu.
fn show_settings_window(app: &AppHandle) {
    if let Some(popover) = app.get_webview_window("popover") {
        let _ = popover.hide();
    }
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
async fn open_settings(app: AppHandle) -> Result<(), String> {
    show_settings_window(&app);
    Ok(())
}

#[tauri::command]
async fn close_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
async fn hide_popover(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("popover") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
fn quit_ui(app: AppHandle) {
    app.exit(0);
}

/// Height cap, in logical pixels, that the popover webview applies to its card.
#[tauri::command]
fn popover_max_height(app: AppHandle) -> f64 {
    app.get_webview_window("popover")
        .map(|window| popover_available_height(&window))
        .unwrap_or(POPOVER_MAX_HEIGHT)
}

/// Resize the popover to hug its content.
///
/// The webview measures its card and calls this, so the window is never taller
/// than what it draws — a transparent window is still hit-testable, and the
/// unused remainder of a fixed frame swallows clicks meant for the app
/// underneath. The clamp keeps the window inside the room below the menu bar,
/// so content can't push the footer off the bottom of the screen either.
#[tauri::command]
fn resize_popover(app: AppHandle, height: f64) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    let clamped = height.clamp(POPOVER_MIN_HEIGHT, popover_available_height(&window));
    let _ = window.set_size(LogicalSize::new(POPOVER_WIDTH, clamped));
}

/// Install channel of the running app. Homebrew drops an ownership marker next
/// to the bundle (installed by the formula's service wrapper); its absence means
/// a DMG/manual install. The marker must stay outside `Microbridge.app`, because
/// adding a file to the bundle after signing invalidates its sealed signature.
/// The in-app self-updater only replaces `direct` installs — brew copies are
/// routed to `brew upgrade` so the formula version and bundle never drift apart.
#[tauri::command]
fn update_channel() -> String {
    if brew_marker_present() {
        "brew".into()
    } else {
        "direct".into()
    }
}

/// True when the running bundle has Homebrew's ownership marker beside it.
/// `…/Microbridge.app/Contents/MacOS/microbridge-ui` → the bundle root is the
/// third ancestor of the executable. The in-bundle check is migration support
/// for installs made before the sidecar marker was introduced.
fn brew_marker_present() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    match exe.ancestors().nth(3) {
        Some(bundle) => {
            let sidecar = bundle
                .parent()
                .map(|parent| parent.join(".Microbridge.app.microbridge-brew"));
            sidecar.is_some_and(|marker| marker.exists())
                || bundle.join(".microbridge-brew").exists()
        }
        None => false,
    }
}

/// App version, read from the bundle metadata (single source of truth in
/// `tauri.conf.json`). Shown on the Updates settings tab.
#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Ask the frontend to run a user-initiated update check. The check / prompt /
/// download / relaunch flow lives in the always-loaded popover webview and
/// shows native dialogs via the dialog plugin.
fn trigger_update_check(app: &AppHandle) {
    let _ = app.emit("menu://check-updates", ());
}

/// launchd label for the login item. Deliberately the same label `install.sh`
/// used to write by hand, so the autostart plugin owns that exact file
/// (`~/Library/LaunchAgents/ai.microbridge.ui.plist`) instead of creating a
/// second one. Without this the plugin would default to `package_info().name`
/// ("Microbridge") and a source install would end up with two login entries.
const LOGIN_ITEM_LABEL: &str = "ai.microbridge.ui";

/// True when the executable sits inside a `.app` bundle.
///
/// `tauri dev` runs the bare binary out of `target/debug`, and the login item
/// records whatever `current_exe()` returns — so accepting the prompt during
/// development would register a throwaway build to launch at every login, and
/// leave a dangling login item behind the moment `target/` is cleaned.
fn running_from_app_bundle() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.ancestors()
                .nth(3)
                .map(|bundle| bundle.extension().is_some_and(|ext| ext == "app"))
        })
        .unwrap_or(false)
}

/// Whether a login item can meaningfully be registered for this build.
#[tauri::command]
fn can_launch_at_login() -> bool {
    running_from_app_bundle()
}

#[tauri::command]
fn launch_at_login_enabled(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        // Writes the plist with RunAtLoad; launchd picks it up at next login.
        // Deliberately not bootstrapped here — that would fire RunAtLoad
        // immediately and start a second copy of the app.
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
        bootout_login_item();
    }
    Ok(())
}

/// `disable()` only deletes the plist. Installs that came from `install.sh` also
/// had the agent *bootstrapped* into the running launchd session, so without
/// this it would linger in `launchctl print` until the next logout. Best-effort:
/// a missing agent is the normal case and its error is not interesting.
#[cfg(target_os = "macos")]
fn bootout_login_item() {
    let _ = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("launchctl bootout gui/$(id -u)/{LOGIN_ITEM_LABEL}"))
        .status();
}

#[cfg(not(target_os = "macos"))]
fn bootout_login_item() {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(LOGIN_ITEM_LABEL)
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            let bundled_daemon = start_bundled_daemon();
            let (bus, mut event_rx) = spawn_bus_loop();
            let snapshot: CachedSnapshot = Arc::new(Mutex::new(None));
            let hud_generation = Arc::new(AtomicU64::new(0));
            let snap_for_loop = Arc::clone(&snapshot);
            let hud_gen_loop = Arc::clone(&hud_generation);
            let handle = app.handle().clone();
            let cursor_integration_synced = Arc::new(AtomicBool::new(false));
            let cursor_sync_loop = Arc::clone(&cursor_integration_synced);
            let factory_integration_synced = Arc::new(AtomicBool::new(false));
            let factory_sync_loop = Arc::clone(&factory_integration_synced);
            let claude_hooks_synced = Arc::new(AtomicBool::new(false));
            let claude_sync_loop = Arc::clone(&claude_hooks_synced);
            let opencode_integration_synced = Arc::new(AtomicBool::new(false));
            let opencode_sync_loop = Arc::clone(&opencode_integration_synced);

            tauri::async_runtime::spawn(async move {
                let mut last_focus: Option<String> = None;
                let mut saw_snapshot = false;
                while let Some(msg) = event_rx.recv().await {
                    match msg {
                        ServerMessage::Snapshot { snapshot: s } => {
                            let cursor_enabled = s
                                .config
                                .adapters
                                .get("cursor")
                                .map(|preference| preference.enabled)
                                .unwrap_or(false);
                            sync_cursor_integration(&handle, &cursor_sync_loop, cursor_enabled);
                            let factory_enabled = s
                                .config
                                .adapters
                                .get("factory")
                                .map(|preference| preference.enabled)
                                .unwrap_or(false);
                            sync_factory_integration(&factory_sync_loop, factory_enabled);
                            let claude_enabled = s
                                .config
                                .adapters
                                .get("claude")
                                .map(|preference| preference.enabled)
                                .unwrap_or(true);
                            if claude_enabled {
                                sync_claude_hooks(&handle, &claude_sync_loop);
                            }
                            let opencode_enabled = s
                                .config
                                .adapters
                                .get("opencode")
                                .map(|preference| preference.enabled)
                                .unwrap_or(false);
                            sync_opencode_integration(
                                &handle,
                                &opencode_sync_loop,
                                opencode_enabled,
                            );
                            last_focus = s.focused_session_id.clone();
                            saw_snapshot = true;
                            *snap_for_loop.lock().await = Some(s.clone());
                            let _ = handle.emit("bus-snapshot", &s);
                        }
                        ServerMessage::Event { event } => {
                            // Ignore reconnect "offline" noise before first snapshot.
                            if !saw_snapshot
                                && matches!(
                                    &event,
                                    BusEvent::DeviceChanged {
                                        connected: false,
                                        ..
                                    }
                                )
                            {
                                continue;
                            }
                            if let BusEvent::ConfigChanged { config } = &event {
                                let cursor_enabled = config
                                    .adapters
                                    .get("cursor")
                                    .map(|preference| preference.enabled)
                                    .unwrap_or(false);
                                sync_cursor_integration(&handle, &cursor_sync_loop, cursor_enabled);
                                let factory_enabled = config
                                    .adapters
                                    .get("factory")
                                    .map(|preference| preference.enabled)
                                    .unwrap_or(false);
                                sync_factory_integration(&factory_sync_loop, factory_enabled);
                                let opencode_enabled = config
                                    .adapters
                                    .get("opencode")
                                    .map(|preference| preference.enabled)
                                    .unwrap_or(false);
                                sync_opencode_integration(
                                    &handle,
                                    &opencode_sync_loop,
                                    opencode_enabled,
                                );
                            }
                            let focus_changed = matches!(&event, BusEvent::FocusChanged { .. });
                            let mut guard = snap_for_loop.lock().await;
                            if let Some(s) = guard.as_mut() {
                                let prev = s.focused_session_id.clone();
                                apply_event(s, event);
                                let changed = focus_changed && s.focused_session_id != prev;
                                if changed {
                                    last_focus = s.focused_session_id.clone();
                                }
                                let payload = s.clone();
                                let _ = handle.emit("bus-snapshot", payload);
                                drop(guard);
                                if changed && last_focus.is_some() {
                                    show_hud(&handle, Arc::clone(&hud_gen_loop));
                                }
                            }
                        }
                        ServerMessage::Config { config } => {
                            let cursor_enabled = config
                                .adapters
                                .get("cursor")
                                .map(|preference| preference.enabled)
                                .unwrap_or(false);
                            sync_cursor_integration(&handle, &cursor_sync_loop, cursor_enabled);
                            let factory_enabled = config
                                .adapters
                                .get("factory")
                                .map(|preference| preference.enabled)
                                .unwrap_or(false);
                            sync_factory_integration(&factory_sync_loop, factory_enabled);
                            let opencode_enabled = config
                                .adapters
                                .get("opencode")
                                .map(|preference| preference.enabled)
                                .unwrap_or(false);
                            sync_opencode_integration(
                                &handle,
                                &opencode_sync_loop,
                                opencode_enabled,
                            );
                            let mut guard = snap_for_loop.lock().await;
                            if let Some(s) = guard.as_mut() {
                                s.config = config;
                                let payload = s.clone();
                                let _ = handle.emit("bus-snapshot", payload);
                            }
                        }
                        _ => {}
                    }
                }
            });

            let _ = hud_generation; // owned by the bus event loop
            app.manage(AppState {
                bus,
                snapshot,
                bundled_daemon: StdMutex::new(bundled_daemon),
            });

            // Dedicated monochrome tray glyph (bridge silhouette), rendered as a
            // template image so macOS tints it for light/dark menu bars. The full
            // app icon is an opaque squircle and would appear as a black blob here.
            let tray_icon = tauri::include_image!("icons/tray.png");
            let last_left_click_ms = Arc::new(AtomicU64::new(0));

            // Right-click context menu. Left-click still toggles the popover
            // (`show_menu_on_left_click(false)` keeps the menu on right-click only).
            let check_updates_item = MenuItem::with_id(
                app,
                "check-updates",
                "Check for Updates…",
                true,
                None::<&str>,
            )?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let quit_item =
                MenuItem::with_id(app, "quit", "Quit Microbridge", true, Some("CmdOrCtrl+Q"))?;
            let tray_menu = Menu::with_items(
                app,
                &[
                    &check_updates_item,
                    &settings_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit_item,
                ],
            )?;
            let context_menu = tray_menu.clone();

            let blur_hide: BlurHideClock = Arc::new(std::sync::Mutex::new(None));
            let blur_hide_tray = Arc::clone(&blur_hide);

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Microbridge")
                // Keep the utility menu detached. On macOS an attached NSMenu
                // can consume the status-item click before the tray callback,
                // making left and right clicks indistinguishable. We pop it up
                // explicitly only for a right-button release below.
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "check-updates" => trigger_update_check(app),
                    "settings" => show_settings_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let previous = last_left_click_ms.swap(now, Ordering::Relaxed);
                        if now.saturating_sub(previous) < 180 {
                            return;
                        }
                        let app = tray.app_handle();
                        let scale = app
                            .get_webview_window("popover")
                            .map(|w| window_scale(&w))
                            .unwrap_or(1.0);
                        let (x, y, w, h) = physical_tray_rect(&rect, scale);
                        toggle_popover(app, x, y, w, h, &blur_hide_tray);
                    }
                    TrayIconEvent::Click {
                        button: MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        if let Some(window) = tray.app_handle().get_webview_window("popover") {
                            let _ = context_menu.popup(window.as_ref().window());
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            if let Some(popover) = app.get_webview_window("popover") {
                let popover_hide = popover.clone();
                popover.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        if let Ok(mut last) = blur_hide.lock() {
                            *last = Some(std::time::Instant::now());
                        }
                        let _ = popover_hide.hide();
                    }
                });
            }

            if let Some(settings) = app.get_webview_window("settings") {
                let settings_hide = settings.clone();
                settings.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_hide.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            set_config,
            set_adapter_enabled,
            pair_adapter,
            forget_adapter,
            activate_agent_key,
            open_settings,
            close_settings,
            hide_popover,
            quit_ui,
            update_channel,
            app_version,
            launch_at_login_enabled,
            set_launch_at_login,
            can_launch_at_login,
            popover_max_height,
            resize_popover
        ])
        .build(tauri::generate_context!())
        .expect("error while building microbridge-ui")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut guard) = state.bundled_daemon.lock() {
                        if let Some(mut child) = guard.take() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                }
            }
        });
}
