import type { DaemonConfig, Snapshot } from "./types";

/** Talks to microbridged via Tauri when available; demo snapshot in browser. */

const DEMO: Snapshot = {
  sessions: [
    {
      id: "s1",
      app: "Codex",
      title: "microbridge — HID reconnect on wake",
      state: "working",
      updated_at_ms: Date.now() - 12 * 60000,
    },
    {
      id: "s2",
      app: "Claude Code",
      title: "adapters — cursor beta cleanup",
      state: "awaiting_approval",
      updated_at_ms: Date.now() - 4 * 60000,
    },
    {
      id: "s3",
      app: "Cursor",
      title: "synara — onboarding empty states",
      state: "thinking",
      updated_at_ms: Date.now() - 60000,
    },
  ],
  focused_session_id: "s1",
  agent_key_session_ids: ["s1", "s2", "s3", null, null, null],
  device_connected: true,
  device_name: "mock",
  config: {
    key_source: "most_recent",
    pinned_session_ids: [],
    app_priority: [],
    custom_key_ids: ["", "", "", "", "", ""],
    pinned_focus: null,
    approvals_interrupt: true,
    pause_leds: false,
    appearance: "system",
    lighting_preset: "codex",
    state_colors: {},
    brightness: 80,
    sleep_minutes: 3,
    frontmost_app: null,
  },
};

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args);
  } catch {
    return null;
  }
}

export async function fetchSnapshot(): Promise<Snapshot> {
  const snap = await invoke<Snapshot>("get_snapshot");
  return snap ?? DEMO;
}

export async function setConfig(config: DaemonConfig): Promise<DaemonConfig> {
  const next = await invoke<DaemonConfig>("set_config", { config });
  return next ?? config;
}

export async function openSettings(): Promise<void> {
  await invoke("open_settings");
}

export async function closeSettings(): Promise<void> {
  await invoke("close_settings");
}

export async function quitUi(): Promise<void> {
  await invoke("quit_ui");
}

/** Subscribe to live bus snapshots. Returns an unsubscribe fn. */
export async function subscribeSnapshot(
  onSnapshot: (snapshot: Snapshot) => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<Snapshot>("bus-snapshot", (event) => {
      onSnapshot(event.payload);
    });
    const initial = await fetchSnapshot();
    onSnapshot(initial);
    return unlisten;
  } catch {
    onSnapshot(DEMO);
    const id = window.setInterval(() => onSnapshot(DEMO), 2000);
    return () => window.clearInterval(id);
  }
}

export function isDemoSnapshot(snapshot: Snapshot): boolean {
  return snapshot.device_name === "mock" && snapshot.sessions.some((s) => s.id === "s1");
}
