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

export interface DaemonConfig {
  key_source: KeySource;
  pinned_session_ids: string[];
  app_priority: string[];
  custom_key_ids: string[];
  pinned_focus: string | null;
  approvals_interrupt: boolean;
  pause_leds: boolean;
  appearance: Appearance;
  lighting_preset: string;
  state_colors: Record<string, string>;
  brightness: number;
  sleep_minutes: number;
  frontmost_app: string | null;
}

export interface Snapshot {
  sessions: SessionStatus[];
  focused_session_id: string | null;
  agent_key_session_ids: (string | null)[];
  device_connected: boolean;
  device_name: string;
  config: DaemonConfig;
}

export const STATE_COLORS: Record<AgentState, string> = {
  idle: "#E9E9E6",
  thinking: "#3D7EFF",
  working: "#3D7EFF",
  awaiting_approval: "#FFB000",
  done: "#30C463",
  error: "#FF453A",
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
