import { hasTauri, invokeQuiet, invokeTauri } from "./tauri";
import type { DaemonConfig, Snapshot } from "./types";

/**
 * Talks to microbridged via Tauri when available; demo snapshot in browser only.
 *
 * "Browser only" is load-bearing: inside the app a daemon that isn't up yet must
 * read as *not connected*, never as a populated deck. See `hasTauri` in ./tauri.
 */

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
  // focused_app default: only Codex threads while s1 owns the deck
  agent_key_session_ids: ["s1", null, null, null, null, null],
  device_connected: false,
  device_name: "demo-browser",
  config: {
    key_source: "focused_app",
    pinned_session_ids: [],
    app_priority: [],
    custom_key_ids: ["", "", "", "", "", ""],
    pinned_focus: null,
    approvals_interrupt: true,
    pause_leds: false,
    appearance: "system",
    lighting_preset: "codex",
    state_colors: {
      idle: "#E9E9E6",
      thinking: "#3D7EFF",
      working: "#3D7EFF",
      awaiting_approval: "#FFB000",
      done: "#30C463",
      error: "#FF453A",
    },
    adapters: {
      codex: { enabled: true },
      claude: { enabled: true },
      cursor: { enabled: false },
      t3code: { enabled: false },
    },
    hardware_control_enabled: false,
    brightness: 80,
    sleep_minutes: 3,
    frontmost_app: null,
  },
  adapters: [
    {
      id: "codex",
      display_name: "Codex CLI",
      kind: "native",
      state: "connected",
      capabilities: {
        lifecycle_observation: true,
        approval_acceptance: false,
        approval_rejection: false,
        interrupt: false,
        new_session: false,
        focus_open: false,
        reasoning_effort: false,
      },
      diagnostic: "Built-in lifecycle watcher is active.",
    },
    {
      id: "claude",
      display_name: "Claude Code",
      kind: "native",
      state: "connected",
      capabilities: {
        lifecycle_observation: true,
        approval_acceptance: false,
        approval_rejection: false,
        interrupt: false,
        new_session: false,
        focus_open: false,
        reasoning_effort: false,
      },
      diagnostic: "Built-in lifecycle watcher is active.",
    },
    {
      id: "cursor",
      display_name: "Cursor",
      kind: "community",
      state: "disabled",
      capabilities: {
        lifecycle_observation: false,
        approval_acceptance: false,
        approval_rejection: false,
        interrupt: false,
        new_session: false,
        focus_open: false,
        reasoning_effort: false,
      },
      diagnostic: "Disabled until you explicitly enable this integration.",
    },
    {
      id: "t3code",
      display_name: "T3 Code",
      kind: "community",
      state: "disabled",
      capabilities: {
        lifecycle_observation: false,
        approval_acceptance: false,
        approval_rejection: false,
        interrupt: false,
        new_session: false,
        focus_open: false,
        reasoning_effort: false,
      },
      diagnostic: "Disabled until you explicitly enable this integration.",
    },
  ],
};

const DAEMON_OFFLINE: Snapshot = {
  ...DEMO,
  sessions: [],
  focused_session_id: null,
  agent_key_session_ids: [null, null, null, null, null, null],
  device_name: "daemon-offline",
  adapters: DEMO.adapters.map((adapter) => ({
    ...adapter,
    state: adapter.state === "disabled" ? "disabled" : "error",
    diagnostic:
      adapter.state === "disabled"
        ? adapter.diagnostic
        : "Microbridge daemon is not running; no live lifecycle data is available.",
  })),
};

/**
 * Demo snapshot, optionally padded out to `?threads=N` sessions.
 *
 * Three sessions aren't enough to exercise the popover's scrolling thread list
 * in a browser preview, and this is the only place that can produce sessions
 * without a daemon. Reachable only outside Tauri, so it can't leak into the app.
 */
function demoSnapshot(): Snapshot {
  const requested = Number(
    new URLSearchParams(window.location.search).get("threads"),
  );
  if (!Number.isFinite(requested) || requested <= DEMO.sessions.length) {
    return DEMO;
  }
  const sessions = Array.from({ length: Math.min(requested, 200) }, (_, i) => {
    const base = DEMO.sessions[i % DEMO.sessions.length];
    return i < DEMO.sessions.length
      ? base
      : { ...base, id: `${base.id}-${i}`, title: `${base.title} (${i + 1})` };
  });
  return { ...DEMO, sessions };
}

/**
 * Current snapshot, or `null` when microbridged hasn't sent one yet.
 *
 * Outside Tauri this is the demo snapshot so the surfaces are previewable in a
 * browser. Inside Tauri, `get_snapshot` rejects with "waiting for microbridged"
 * until the first snapshot lands — that rejection must stay visible as `null`.
 */
export async function fetchSnapshot(): Promise<Snapshot | null> {
  if (!hasTauri()) return demoSnapshot();
  try {
    return await invokeTauri<Snapshot>("get_snapshot");
  } catch {
    return null;
  }
}
export function isDemoSnapshot(snapshot: Snapshot): boolean {
  return snapshot.device_name === "demo-browser";
}

/** Rejects if the daemon refuses the write, so the UI can revert rather than lie. */
export async function setConfig(config: DaemonConfig): Promise<DaemonConfig> {
  const next = await invokeTauri<DaemonConfig>("set_config", { config });
  return next ?? config; // no Tauri: browser preview, echo the optimistic value
}

export async function setAdapterEnabled(adapterId: string, enabled: boolean): Promise<string> {
  const message = await invokeTauri<string>("set_adapter_enabled", { adapterId, enabled });
  if (message === null) throw new Error("Adapter controls require the Microbridge app.");
  return message;
}

export async function pairAdapter(adapterId: string, pairingUrl: string): Promise<string> {
  const message = await invokeTauri<string>("pair_adapter", { adapterId, pairingUrl });
  if (message === null) throw new Error("Pairing requires the Microbridge app.");
  return message;
}

export async function forgetAdapter(adapterId: string): Promise<string> {
  const message = await invokeTauri<string>("forget_adapter", { adapterId });
  if (message === null) throw new Error("Adapter controls require the Microbridge app.");
  return message;
}

export async function activateAgentKey(index: number, open = false): Promise<string> {
  const message = await invokeTauri<string>("activate_agent_key", { index, open });
  if (message === null) throw new Error("Agent Key simulation requires the Microbridge app.");
  return message;
}

export async function openSettings(): Promise<void> {
  await invokeQuiet("open_settings");
}

export async function closeSettings(): Promise<void> {
  await invokeQuiet("close_settings");
}

export async function quitUi(): Promise<void> {
  await invokeQuiet("quit_ui");
}

/** Subscribe to live bus snapshots. Returns an unsubscribe fn. */
export async function subscribeSnapshot(
  onSnapshot: (snapshot: Snapshot) => void,
): Promise<() => void> {
  if (!hasTauri()) {
    onSnapshot(demoSnapshot());
    return () => {};
  }
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<Snapshot>("bus-snapshot", (event) => {
      onSnapshot(event.payload);
    });
    // Keep the listener when the daemon is offline; the native reconnect loop
    // publishes a real snapshot as soon as it becomes reachable.
    onSnapshot((await fetchSnapshot()) ?? DAEMON_OFFLINE);
    return unlisten;
  } catch {
    // Never substitute canned threads in the production app.
    onSnapshot(DAEMON_OFFLINE);
    return () => {};
  }
}
