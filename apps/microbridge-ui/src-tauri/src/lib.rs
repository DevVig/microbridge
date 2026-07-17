//! Tauri menu bar companion — talks to microbridged over the local Unix socket.
//! Never opens HID; the daemon owns the device.

mod bus;

use std::sync::Arc;
use std::time::Duration;

use bus::{apply_event, spawn_bus_loop, BusHandle, CachedSnapshot};
use mb_protocol::{BusEvent, DaemonConfig, ServerMessage, Snapshot};
use tauri::{
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
    let x = (tray_x + tray_w / 2.0 - width / 2.0).round() as i32;
    let y = (tray_y + tray_h + 6.0).round() as i32;
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
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

fn show_hud(app: &AppHandle) {
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
    let _ = window.show();
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(2500)).await;
        if let Some(hud) = app2.get_webview_window("hud") {
            let _ = hud.hide();
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
async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(popover) = app.get_webview_window("popover") {
        let _ = popover.hide();
    }
    let window = app
        .get_webview_window("settings")
        .ok_or("settings window missing")?;
    let _ = window.show();
    let _ = window.set_focus();
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            let (bus, mut event_rx) = spawn_bus_loop();
            let snapshot: CachedSnapshot = Arc::new(Mutex::new(None));
            let snap_for_loop = Arc::clone(&snapshot);
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
                                    show_hud(&handle);
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

            app.manage(AppState { bus, snapshot });

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("default window icon");

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(true)
                .tooltip("Microbridge")
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
            quit_ui
        ])
        .run(tauri::generate_context!())
        .expect("error while running microbridge-ui");
}
