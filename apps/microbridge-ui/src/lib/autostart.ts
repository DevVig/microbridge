/**
 * Launch at login.
 *
 * Microbridge is a menu bar app — if it doesn't come back after a reboot it
 * reads as broken. This used to be the installer's job (`install.sh` hand-wrote
 * a LaunchAgent), which meant Homebrew and DMG installs never got it. It's now
 * a property of the app, so every channel behaves the same.
 *
 * The Rust side registers the signed main app through SMAppService, so macOS
 * shows Microbridge's app identity and icon instead of a Unix executable.
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

export type LaunchAtLoginStatus =
  | "unavailable"
  | "not_registered"
  | "enabled"
  | "requires_approval"
  | "not_found";

export async function launchAtLoginStatus(): Promise<LaunchAtLoginStatus> {
  try {
    return (
      (await invokeTauri<LaunchAtLoginStatus>("launch_at_login_status")) ??
      "unavailable"
    );
  } catch {
    return "unavailable";
  }
}

export async function setLaunchAtLogin(
  enabled: boolean,
): Promise<LaunchAtLoginStatus> {
  const status = await invokeTauri<LaunchAtLoginStatus>("set_launch_at_login", {
    enabled,
  });
  return status ?? "unavailable";
}

export async function openLoginItemsSettings(): Promise<void> {
  await invokeTauri("open_login_items_settings");
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
  const status = await launchAtLoginStatus();
  if (status === "unavailable") return;
  if (status === "enabled" || status === "requires_approval") {
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
