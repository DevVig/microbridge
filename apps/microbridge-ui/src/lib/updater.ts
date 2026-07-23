/**
 * In-app updates for direct (DMG) installs.
 *
 * User-initiated by default (tray "Check for Updates…" / Settings button); an
 * opt-in toggle also runs one silent check at launch. Channel-aware: Homebrew
 * installs are routed to `brew upgrade` and never self-replaced, so the two
 * update paths can't drift. Every Tauri import is dynamic + guarded so the
 * browser/demo build (no Tauri runtime) degrades to a no-op.
 */

const AUTO_CHECK_KEY = "microbridge.autoCheckUpdates";
const LAST_AUTO_CHECK_KEY = "microbridge.lastAutoCheckAt";
const AUTO_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

export function automaticUpdateCheckDue(
  lastAttempt: string | null,
  now = Date.now(),
): boolean {
  const last = Number(lastAttempt);
  return (
    !Number.isFinite(last) ||
    last <= 0 ||
    last > now ||
    now - last >= AUTO_CHECK_INTERVAL_MS
  );
}

/** Opt-in launch check — defaults off to honor "no update pings" by default. */
export function autoCheckEnabled(): boolean {
  try {
    return localStorage.getItem(AUTO_CHECK_KEY) === "1";
  } catch {
    return false;
  }
}

export function setAutoCheckEnabled(value: boolean): void {
  try {
    localStorage.setItem(AUTO_CHECK_KEY, value ? "1" : "0");
  } catch {
    /* no persistent storage — nothing to do */
  }
}

/**
 * Run the opt-in background check at most once per 24 hours.
 *
 * The timestamp is recorded when the attempt starts, not only on success, so a
 * temporary network failure cannot turn every app restart into another ping.
 * Manual checks bypass this throttle.
 */
export async function runAutomaticUpdateCheck(): Promise<void> {
  if (!autoCheckEnabled()) return;
  try {
    const now = Date.now();
    if (!automaticUpdateCheckDue(localStorage.getItem(LAST_AUTO_CHECK_KEY), now)) {
      return;
    }
    localStorage.setItem(LAST_AUTO_CHECK_KEY, String(now));
  } catch {
    // Without durable storage there is no honest way to enforce once a day.
    return;
  }
  await runUpdateCheck({ silent: true });
}

async function invokeCmd<T>(cmd: string): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd);
  } catch {
    return null;
  }
}

export type UpdateChannel = "brew" | "direct";

/** `"brew"` when Homebrew manages this bundle, else `"direct"`. */
export async function updateChannel(): Promise<UpdateChannel | null> {
  return await invokeCmd<UpdateChannel>("update_channel");
}

export async function appVersion(): Promise<string | null> {
  return await invokeCmd<string>("app_version");
}

/**
 * Run a channel-aware update check.
 *
 * `silent` suppresses the "up to date" and Homebrew dialogs — used by the
 * opt-in launch check so a current install shows nothing. Never throws; a
 * network error, a missing manifest, or an absent Tauri runtime resolves to a
 * quiet no-op (or a soft warning when not silent).
 */
export async function runUpdateCheck({
  silent,
}: {
  silent: boolean;
}): Promise<void> {
  try {
    // Homebrew owns this bundle — defer to brew, never self-replace.
    if ((await updateChannel()) === "brew") {
      if (!silent) {
        const { message } = await import("@tauri-apps/plugin-dialog");
        await message(
          "Microbridge was installed with Homebrew.\n\nUpdate and refresh the app from Terminal:\n\n    brew update && brew upgrade microbridge && microbridge-app install",
          { title: "Update Microbridge", kind: "info" },
        );
      }
      return;
    }

    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();

    if (!update) {
      if (!silent) {
        const { message } = await import("@tauri-apps/plugin-dialog");
        await message("You're on the latest version.", {
          title: "Microbridge",
          kind: "info",
        });
      }
      return;
    }

    const { ask } = await import("@tauri-apps/plugin-dialog");
    const notes = update.body ? `\n\n${update.body}` : "";
    const proceed = await ask(`Version ${update.version} is available.${notes}`, {
      title: "Update Microbridge",
      kind: "info",
      okLabel: "Install & Restart",
      cancelLabel: "Later",
    });
    if (!proceed) return;

    await update.downloadAndInstall();
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch {
    if (!silent) {
      try {
        const { message } = await import("@tauri-apps/plugin-dialog");
        await message(
          "Couldn't check for updates right now. Please try again later.",
          { title: "Microbridge", kind: "warning" },
        );
      } catch {
        /* dialog plugin unavailable — nothing to show */
      }
    }
  }
}
