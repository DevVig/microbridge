//! Shared recursive directory watcher (event-driven via `notify`).

use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{info, warn};

/// How recent a file must be (mtime) to be published on the initial scan.
const INITIAL_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24); // 24h

/// Spawn a background thread that watches `root` and invokes `on_change`
/// whenever a matching file is created/modified. Initial scan only includes
/// files touched within the last 24 hours so historical journals don't flood
/// the bus.
pub fn watch_dir(root: PathBuf, mut on_change: impl FnMut(PathBuf) + Send + 'static) {
    if !root.exists() {
        info!(path = %root.display(), "session dir absent; adapter idle");
        return;
    }

    thread::Builder::new()
        .name("mb-watch".into())
        .spawn(move || {
            let (tx, rx) = std_mpsc::channel();
            let mut watcher = match RecommendedWatcher::new(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        let _ = tx.send(event);
                    }
                },
                notify::Config::default(),
            ) {
                Ok(w) => w,
                Err(error) => {
                    warn!(%error, "failed to create watcher");
                    return;
                }
            };

            if let Err(error) = watcher.watch(&root, RecursiveMode::Recursive) {
                warn!(%error, path = %root.display(), "failed to watch");
                return;
            }
            info!(path = %root.display(), "watching session directory");

            scan_recent(&root, &mut on_change);

            while let Ok(event) = rx.recv() {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        while rx.try_recv().is_ok() {}
                        thread::sleep(Duration::from_millis(80));
                        while rx.try_recv().is_ok() {}
                        for path in event.paths {
                            if is_session_file(&path) {
                                on_change(path);
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            if is_session_file(&path) {
                                on_change(path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
        .ok();
}

fn is_session_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e == "json" || e == "jsonl")
}

fn scan_recent(root: &Path, on_change: &mut impl FnMut(PathBuf)) {
    let cutoff = SystemTime::now()
        .checked_sub(INITIAL_MAX_AGE)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    walk_recent(root, cutoff, on_change);
}

fn walk_recent(root: &Path, cutoff: SystemTime, on_change: &mut impl FnMut(PathBuf)) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_recent(&path, cutoff, on_change);
            continue;
        }
        if !is_session_file(&path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        if mtime >= cutoff {
            on_change(path);
        }
    }
}
