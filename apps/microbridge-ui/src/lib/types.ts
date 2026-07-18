export type AgentState =
  | "idle"
  | "thinking"
  | "working"
  | "awaiting_approval"
  | "done"
  | "error";

export interface SessionStatus {
  id: string;
  app: string;
  title: string;
  state: AgentState;
  updated_at_ms: number;
}

export type KeySource =
  | "most_recent"
  | "focused_app"
  | "pinned"
  | "priority"
  | "custom";

export type Appearance = "system" | "light" | "dark";
export type LightingPreset = "codex" | "phosphor" | "custom";

export interface StateColors {
  idle: string;
  thinking: string;
  working: string;
  awaiting_approval: string;
  done: string;
  error: string;
}

export type AdapterConnectionState =
  | "disabled"
  | "needs_setup"
  | "connecting"
  | "connected"
  | "limited"
  | "incompatible"
  | "error";

export interface AdapterCapabilities {
  lifecycle_observation: boolean;
  approval_acceptance: boolean;
  approval_rejection: boolean;
  interrupt: boolean;
  new_session: boolean;
  focus_open: boolean;
  reasoning_effort: boolean;
}

export interface AdapterStatus {
  id: string;
  display_name: string;
  kind: "native" | "community";
  state: AdapterConnectionState;
  capabilities: AdapterCapabilities;
  version?: string;
  last_activity_ms?: number;
  diagnostic: string;
}

export interface DaemonConfig {
  key_source: KeySource;
  pinned_session_ids: string[];
  app_priority: string[];
  custom_key_ids: string[];
  pinned_focus: string | null;
  approvals_interrupt: boolean;
  pause_leds: boolean;
  appearance: Appearance;
  lighting_preset: LightingPreset;
  state_colors: StateColors;
  adapters: Record<string, { enabled: boolean }>;
  hardware_control_enabled: boolean;
  brightness: number;
  sleep_minutes: number;
  frontmost_app: string | null;
}

export interface Snapshot {
  sessions: SessionStatus[];
  focused_session_id: string | null;
  agent_key_session_ids: (string | null)[];
  agent_key_led_frame?: AgentKeyLedFrame;
  device_connected: boolean;
  device_name: string;
  config: DaemonConfig;
  adapters: AdapterStatus[];
}

export interface AgentKeyLed {
  session_id: string | null;
  state: AgentState | null;
  color: string | null;
  focused: boolean;
}

export interface AgentKeyLedFrame {
  keys: AgentKeyLed[];
  brightness: number;
  paused: boolean;
}

/** Backward-compatible effective frame for snapshots from older daemons. */
export function agentKeyLedFrame(snapshot: Snapshot): AgentKeyLedFrame {
  if (snapshot.agent_key_led_frame?.keys?.length) {
    return snapshot.agent_key_led_frame;
  }
  return {
    keys: snapshot.agent_key_session_ids.map((sessionId) => {
      const session = sessionId
        ? snapshot.sessions.find((candidate) => candidate.id === sessionId)
        : null;
      return {
        session_id: sessionId,
        state: session?.state ?? null,
        color: session ? snapshot.config.state_colors[session.state] : null,
        focused: sessionId != null && sessionId === snapshot.focused_session_id,
      };
    }),
    brightness: snapshot.config.brightness,
    paused: snapshot.config.pause_leds,
  };
}

export const STATE_COLORS: Record<AgentState, string> = {
  idle: "#E9E9E6",
  thinking: "#3D7EFF",
  working: "#3D7EFF",
  awaiting_approval: "#FFB000",
  done: "#30C463",
  error: "#FF453A",
};

export const CODEX_PALETTE: StateColors = { ...STATE_COLORS };
export const PHOSPHOR_PALETTE: StateColors = {
  idle: "#4A4A52",
  thinking: "#FFB454",
  working: "#FF6A00",
  awaiting_approval: "#FF3D00",
  done: "#3DDC84",
  error: "#FF4757",
};

export const STATE_LABELS: Record<AgentState, string> = {
  idle: "Idle",
  thinking: "Thinking",
  working: "Working",
  awaiting_approval: "Needs approval",
  done: "Done",
  error: "Error",
};

export function elapsed(updatedAtMs: number): string {
  const mins = Math.max(0, Math.floor((Date.now() - updatedAtMs) / 60000));
  if (mins < 1) return "<1m";
  if (mins < 60) return `${mins}m`;
  return `${Math.floor(mins / 60)}h`;
}
