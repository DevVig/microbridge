import type {
  AdapterConnectionState,
  AdapterStatus,
  SessionStatus,
} from "./types";

/** Traffic-light glance for Integrations cards. */
export type TrafficLight = "green" | "yellow" | "red";

/**
 * Hosts that ride Claude/Codex journals — always listed as Integrations cards,
 * but status is session-derived (no separate pairable adapter).
 */
export const HOST_ATTRIBUTED: ReadonlyArray<{
  id: string;
  app: string;
}> = [
  { id: "synara", app: "Synara" },
  { id: "chatgpt", app: "ChatGPT" },
  { id: "claude_desktop", app: "Claude Desktop" },
  { id: "conductor", app: "Conductor" },
];

/** Opt-in sources that can still receive journal-attributed sessions before enable. */
const JOURNAL_APP_BY_ADAPTER: Record<string, string> = {
  cursor: "Cursor",
  t3code: "T3 Code",
  factory: "Factory",
};

export interface HostPresence {
  count: number;
  lastActivityMs: number | null;
}

export function hostPresence(
  sessions: SessionStatus[],
  appName: string,
): HostPresence {
  let count = 0;
  let lastActivityMs: number | null = null;
  for (const session of sessions) {
    if (session.app !== appName) continue;
    count += 1;
    if (lastActivityMs === null || session.updated_at_ms > lastActivityMs) {
      lastActivityMs = session.updated_at_ms;
    }
  }
  return { count, lastActivityMs };
}

export function isHostAttributed(adapterId: string): boolean {
  return HOST_ATTRIBUTED.some((host) => host.id === adapterId);
}

function journalAppFor(adapterId: string): string | undefined {
  return (
    HOST_ATTRIBUTED.find((host) => host.id === adapterId)?.app ??
    JOURNAL_APP_BY_ADAPTER[adapterId]
  );
}

export interface IntegrationView {
  light: TrafficLight;
  label: string;
  diagnostic: string;
  /** True when the card should sit in the Connected group. */
  connectedGroup: boolean;
}

const STATE_LABELS: Record<AdapterConnectionState, string> = {
  disabled: "Not connected",
  needs_setup: "Setup needed",
  connecting: "Connecting",
  connected: "Connected",
  limited: "Limited",
  incompatible: "Incompatible",
  error: "Error",
};

function lightForState(state: AdapterConnectionState): TrafficLight {
  if (state === "connected") return "green";
  if (state === "error" || state === "incompatible") return "red";
  return "yellow";
}

/** Live links belong in Connected — including partial (limited) capability. */
function connectedGroupForState(state: AdapterConnectionState): boolean {
  return state === "connected" || state === "limited";
}

/**
 * Derive the card's traffic light, label, and diagnostic from daemon adapter
 * state plus live session attribution.
 *
 * Pass `enabled` from config when known so auto-discovered (needs_setup + not
 * enabled) tiles read as “Detected — click to install” instead of “Setup needed”.
 */
export function integrationView(
  adapter: AdapterStatus,
  sessions: SessionStatus[],
  options?: { enabled?: boolean },
): IntegrationView {
  const journalApp = journalAppFor(adapter.id);
  const presence = journalApp
    ? hostPresence(sessions, journalApp)
    : { count: 0, lastActivityMs: null };

  if (isHostAttributed(adapter.id)) {
    if (adapter.state === "disabled") {
      return {
        light: "yellow",
        label: "Not connected",
        diagnostic: "Disabled in Microbridge configuration.",
        connectedGroup: false,
      };
    }
    if (adapter.state === "error" || adapter.state === "incompatible") {
      return {
        light: "red",
        label: STATE_LABELS[adapter.state],
        diagnostic: adapter.diagnostic,
        connectedGroup: false,
      };
    }
    if (presence.count > 0) {
      return {
        light: "green",
        label:
          presence.count === 1
            ? "Active · 1 thread"
            : `Active · ${presence.count} threads`,
        diagnostic:
          "via Claude & Codex journals — no separate adapter needed.",
        connectedGroup: true,
      };
    }
    // Healthy watcher with no sessions — Ready, not "Not connected".
    return {
      light: "green",
      label: "Ready · idle",
      diagnostic:
        "via Claude & Codex journals — waiting for sessions (setup is fine).",
      connectedGroup: true,
    };
  }

  // Opt-in sources: journal sessions already flowing while the card is disabled.
  if (
    adapter.state === "disabled" &&
    presence.count > 0 &&
    JOURNAL_APP_BY_ADAPTER[adapter.id]
  ) {
    return {
      light: "yellow",
      label: "Not connected",
      diagnostic:
        presence.count === 1
          ? "1 thread auto-detected — enable for controls."
          : `${presence.count} threads auto-detected — enable for controls.`,
      connectedGroup: false,
    };
  }

  // Auto-discovered on disk but not yet enabled/installed via first click.
  if (adapter.state === "needs_setup" && options?.enabled === false) {
    return {
      light: "yellow",
      label: "Detected — click to install",
      diagnostic: adapter.diagnostic,
      connectedGroup: false,
    };
  }

  const light = lightForState(adapter.state);
  // Lifecycle-only adapters (Cursor Connected promotion, Limited, etc.) should not
  // read as broken — show Connected · lifecycle when observation is the only lever.
  if (
    adapter.capabilities.lifecycle_observation &&
    !adapter.capabilities.approval_acceptance &&
    (adapter.state === "limited" || adapter.state === "connected")
  ) {
    return {
      light: "green",
      label: "Connected · lifecycle",
      diagnostic: adapter.diagnostic,
      connectedGroup: true,
    };
  }
  return {
    light,
    label: STATE_LABELS[adapter.state],
    diagnostic: adapter.diagnostic,
    connectedGroup: connectedGroupForState(adapter.state),
  };
}

export const TRAFFIC_COLORS: Record<
  TrafficLight,
  { bg: string; fg: string; dot: string }
> = {
  green: { bg: "#30C4631F", fg: "#30A653", dot: "#30C463" },
  yellow: { bg: "#FFB0001F", fg: "#C48400", dot: "#FFB000" },
  red: { bg: "#FF453A1F", fg: "#D93A32", dot: "#FF453A" },
};
