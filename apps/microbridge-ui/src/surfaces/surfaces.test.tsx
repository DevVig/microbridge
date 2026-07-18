import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import type { SessionStatus, Snapshot } from "../lib/types";
import { Popover } from "./Popover";
import { Settings } from "./Settings";

function snapshot(sessions: SessionStatus[] = []): Snapshot {
  return {
    sessions,
    focused_session_id: sessions[0]?.id ?? null,
    agent_key_session_ids: sessions.slice(0, 6).map((session) => session.id),
    device_connected: false,
    device_name: "demo-browser",
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
        cursor: { enabled: true },
        t3code: { enabled: false },
      },
      hardware_control_enabled: false,
      brightness: 80,
      sleep_minutes: 3,
      frontmost_app: null,
    },
    adapters: [
      {
        id: "cursor",
        display_name: "Cursor",
        kind: "community",
        state: "limited",
        capabilities: {
          lifecycle_observation: true,
          approval_acceptance: false,
          approval_rejection: false,
          interrupt: false,
          new_session: false,
          focus_open: false,
          reasoning_effort: false,
        },
        diagnostic: "Lifecycle is connected; unsupported IDE commands remain disabled.",
      },
    ],
  };
}

const noop = vi.fn();

describe("Settings", () => {
  it("explains and selects all lighting presets", () => {
    const html = renderToStaticMarkup(
      <Settings
        snapshot={snapshot()}
        dark
        tab="device"
        onTab={noop}
        onConfig={noop}
        onClose={noop}
      />,
    );
    expect(html).toContain("Lighting maps agent lifecycle states to the Agent Key LEDs");
    expect(html).toContain("Codex Defaults");
    expect(html).toContain("Phosphor");
    expect(html).toContain("Custom");
    expect(html).toContain("Reset to Codex Defaults");
  });

  it("renders daemon adapter state and capability limits", () => {
    const html = renderToStaticMarkup(
      <Settings
        snapshot={snapshot()}
        dark
        tab="adapters"
        onTab={noop}
        onConfig={noop}
        onClose={noop}
      />,
    );
    expect(html).toContain("limited");
    expect(html).toContain("Lifecycle is connected");
    expect(html).toContain("Live state");
    expect(html).toContain("Cursor ships inside Microbridge");
    expect(html).toContain("Repair bundled integration");
    expect(html).not.toContain("Install managed plugin");
    expect(html).not.toContain("scaffold only");
    expect(html).not.toContain("not production");
  });
});

describe("Popover", () => {
  it("renders every thread and makes only the thread list scrollable", () => {
    const sessions = Array.from({ length: 12 }, (_, index): SessionStatus => ({
      id: `thread-${index}`,
      app: "Codex",
      title: `Thread title ${index}`,
      state: index === 0 ? "working" : "idle",
      updated_at_ms: Date.now() - index * 1_000,
    }));
    const html = renderToStaticMarkup(
      <Popover
        snapshot={snapshot(sessions)}
        dark
        onOpenSettings={noop}
        onTogglePause={noop}
        onQuit={noop}
      />,
    );
    for (const session of sessions) expect(html).toContain(session.title);
    expect(html.match(/overflow-y-auto/g)).toHaveLength(1);
  });
});
