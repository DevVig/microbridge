//! Frontmost application tracking for the `focused_app` key source.
//!
//! macOS: reads `NSWorkspace.frontmostApplication` on a short cadence and
//! pushes changes onto the bus. A CFRunLoop notification observer can replace
//! the poll later; the important property is that the daemon (not the UI)
//! owns this signal so `--no-ui` still works.

use tokio::sync::mpsc;

/// Spawn a background task that emits localized app names when the frontmost
/// app changes. Non-macOS builds emit nothing.
pub fn spawn_frontmost_watcher(tx: mpsc::UnboundedSender<Option<String>>) {
    #[cfg(target_os = "macos")]
    {
        spawn_macos_watcher(tx);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = tx;
    }
}

#[cfg(target_os = "macos")]
fn spawn_macos_watcher(tx: mpsc::UnboundedSender<Option<String>>) {
    use std::time::Duration;

    use tracing::debug;

    std::thread::Builder::new()
        .name("mb-frontmost".into())
        .spawn(move || {
            let mut last: Option<String> = None;
            loop {
                let next = frontmost_app_name();
                if next != last {
                    debug!(?next, "frontmost app changed");
                    last = next.clone();
                    if tx.send(next).is_err() {
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(400));
            }
        })
        .expect("spawn frontmost watcher");
}

#[cfg(target_os = "macos")]
fn frontmost_app_name() -> Option<String> {
    use objc2_app_kit::NSWorkspace;

    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let name = app.localizedName()?;
    Some(name.to_string())
}
