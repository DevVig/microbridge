/**
 * Launch at login.
 *
 * Microbridge is a menu bar app — if it doesn't come back after a reboot it
 * reads as broken. This used to be the installer's job (`install.sh` hand-wrote
 * a LaunchAgent), which meant Homebrew and DMG installs never got it. It's now
 * a property of the app, so every channel behaves the same.
 *
 * The Rust side owns the launchd plist via tauri-plugin-autostart, pinned to
 * the same `ai.microbridge.ui` label the installer used so there is exactly one
 * login entry. See `set_launch_at_login` in src-tauri/src/lib.rs.
 */

import { invokeTauri } from "./tauri";

const ASKED_KEY = "microbridge.launchAtLoginAsked";

function alreadyAsked(): boolean {
  try {
    return localStorage.getItem(ASKED_KEY) === "1";
  } catch {
    // No persistent storage — treat as asked so we never nag on every launch.
    return true;
  }
}

function markAsked(): void {
  try {
    localStorage.setItem(ASKED_KEY, "1");
  } catch {
    /* no persistent storage — nothing to record */
  }
}

/**
 * Whether a login item can meaningfully be registered for this build.
 *
 * False under `tauri dev`: the plist records `current_exe()`, so accepting the
 * prompt from a dev build would register `target/debug/microbridge-ui` to launch
 * at every login — a throwaway binary that breaks the moment `target/` is
 * cleaned. Also false outside Tauri.
 */
export async function canLaunchAtLogin(): Promise<boolean> {
  try {
    return (await invokeTauri<boolean>("can_launch_at_login")) ?? false;
  } catch {
    return false;
  }
}

/** `null` outside Tauri, where there is no login item to report on. */
export async function launchAtLoginEnabled(): Promise<boolean | null> {
  try {
    return await invokeTauri<boolean>("launch_at_login_enabled");
  } catch {
    return null;
  }
}

export async function setLaunchAtLogin(enabled: boolean): Promise<void> {
  await invokeTauri("set_launch_at_login", { enabled });
}

/**
 * Ask once, on first launch, whether to start at login.
 *
 * Runs from the popover webview only — the same reasoning as the update check
 * in App.tsx: it's the always-loaded window, so this fires once rather than
 * once per window. Silently does nothing outside Tauri, in a dev build, when
 * already answered, or when a login item already exists (an existing
 * `install.sh` user has already expressed the preference; re-asking would be
 * noise).
 *
 * The dev-build check deliberately runs *before* `markAsked()`, so developers
 * still get the prompt the first time they run a real installed build.
 */
export async function promptLaunchAtLoginOnce(): Promise<void> {
  if (alreadyAsked()) return;
  if (!(await canLaunchAtLogin())) return;

  const enabled = await launchAtLoginEnabled();
  if (enabled === null) return; // not under Tauri
  if (enabled) {
    markAsked();
    return;
  }

  try {
    const { ask } = await import("@tauri-apps/plugin-dialog");
    const proceed = await ask(
      "Start Microbridge automatically when you log in?\n\nIt lives in the menu bar, so it needs to be running to light up your deck. You can change this any time in Settings.",
      {
        title: "Launch at Login",
        kind: "info",
        okLabel: "Start at Login",
        cancelLabel: "Not Now",
      },
    );
    // Mark asked only once the user has actually answered, so a dialog that
    // fails to open is retried next launch rather than silently swallowed.
    markAsked();
    if (proceed) await setLaunchAtLogin(true);
  } catch {
    /* dialog unavailable — try again next launch */
  }
}
