//! Tauri menu bar companion — talks to microbridged over the local Unix socket.
//! Never opens HID; the daemon owns the device.

mod bus;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bus::{apply_event, spawn_bus_loop, BusHandle, CachedSnapshot};
use mb_protocol::{BusEvent, DaemonConfig, ServerMessage, Snapshot};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, Position, Size, WebviewWindow,
};
use tokio::sync::Mutex;

struct AppState {
    bus: BusHandle,
    snapshot: CachedSnapshot,
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

fn position_below_tray(window: &WebviewWindow, tray_x: f64, tray_y: f64, tray_w: f64, tray_h: f64) {
    let Ok(size) = window.outer_size() else {
        return;
    };
    let width = f64::from(size.width);
    let mut x = tray_x + tray_w / 2.0 - width / 2.0;
    if let Ok(Some(monitor)) = window.current_monitor() {
        let origin = monitor.position();
        let screen = monitor.size();
        let min_x = f64::from(origin.x);
        let max_x = min_x + f64::from(screen.width) - width;
        x = x.clamp(min_x + 8.0, max_x - 8.0);
    }
    let y = (tray_y + tray_h + 6.0).round() as i32;
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x.round() as i32, y)));
}

fn toggle_popover(app: &AppHandle, tray_x: f64, tray_y: f64, tray_w: f64, tray_h: f64) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    position_below_tray(&window, tray_x, tray_y, tray_w, tray_h);
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

/// Install channel of the running app. Homebrew drops a `.microbridge-brew`
/// marker at the bundle root (see the formula's `post_install`); its absence
/// means a DMG/manual install. The in-app self-updater only replaces `direct`
/// installs — brew copies are routed to `brew upgrade` so the formula version
/// and the on-disk bundle never drift apart.
#[tauri::command]
fn update_channel() -> String {
    if brew_marker_present() {
        "brew".into()
    } else {
        "direct".into()
    }
}

/// True when the running bundle carries Homebrew's ownership marker.
/// `…/Microbridge.app/Contents/MacOS/microbridge-ui` → the bundle root is the
/// third ancestor of the executable.
fn brew_marker_present() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    match exe.ancestors().nth(3) {
        Some(bundle) => bundle.join(".microbridge-brew").exists(),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            let (bus, mut event_rx) = spawn_bus_loop();
            let snapshot: CachedSnapshot = Arc::new(Mutex::new(None));
            let hud_generation = Arc::new(AtomicU64::new(0));
            let snap_for_loop = Arc::clone(&snapshot);
            let hud_gen_loop = Arc::clone(&hud_generation);
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let mut last_focus: Option<String> = None;
                let mut saw_snapshot = false;
                while let Some(msg) = event_rx.recv().await {
                    match msg {
                        ServerMessage::Snapshot { snapshot: s } => {
                            last_focus = s.focused_session_id.clone();
                            saw_snapshot = true;
                            *snap_for_loop.lock().await = Some(s.clone());
                            let _ = handle.emit("bus-snapshot", &s);
                        }
                        ServerMessage::Event { event } => {
                            // Ignore reconnect "offline" noise before first snapshot.
                            if !saw_snapshot {
                                if matches!(
                                    &event,
                                    BusEvent::DeviceChanged { connected: false, .. }
                                ) {
                                    continue;
                                }
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
            app.manage(AppState { bus, snapshot });

            // Dedicated monochrome tray glyph (bridge silhouette), rendered as a
            // template image so macOS tints it for light/dark menu bars. The full
            // app icon is an opaque squircle and would appear as a black blob here.
            let tray_icon = tauri::include_image!("icons/tray.png");

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

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Microbridge")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "check-updates" => trigger_update_check(app),
                    "settings" => show_settings_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let scale = app
                            .get_webview_window("popover")
                            .and_then(|w| w.scale_factor().ok())
                            .unwrap_or(2.0);
                        let (x, y, w, h) = physical_tray_rect(&rect, scale);
                        toggle_popover(app, x, y, w, h);
                    }
                })
                .build(app)?;

            if let Some(popover) = app.get_webview_window("popover") {
                let popover_hide = popover.clone();
                popover.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
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
            open_settings,
            close_settings,
            hide_popover,
            quit_ui,
            update_channel,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running microbridge-ui");
}
