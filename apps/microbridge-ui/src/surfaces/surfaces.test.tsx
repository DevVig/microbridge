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
        cnvs: { enabled: true },
        cursor: { enabled: true },
        t3code: { enabled: false },
        factory: { enabled: false },
        opencode: { enabled: false },
      },
      hardware_control_enabled: false,
      brightness: 80,
      sleep_minutes: 3,
      frontmost_app: null,
    },
    adapters: [
      {
        id: "cnvs",
        display_name: "CNVS",
        kind: "native",
        state: "connected",
        capabilities: {
          lifecycle_observation: true,
          approval_acceptance: false,
          approval_rejection: false,
          interrupt: true,
          new_session: false,
          focus_open: true,
          reasoning_effort: false,
        },
        diagnostic: "Connected across 3 exact canvas terminal targets.",
      },
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

  it("renders unified integration state and capability limits", () => {
    const html = renderToStaticMarkup(
      <Settings
        snapshot={snapshot()}
        dark
        tab="integrations"
        onTab={noop}
        onConfig={noop}
        onClose={noop}
      />,
    );
    expect(html).toContain("Limited");
    expect(html).toContain("Lifecycle is connected");
    expect(html).toContain("Live state");
    expect(html).toContain("Integrations");
    expect(html).toContain("Connected · 1");
    expect(html).toContain("Not connected · 1");
    expect(html).toContain("Connected across 3 exact canvas terminal targets");
    expect(html).toContain("✓ Open");
    expect(html).toContain("Interrupt");
    expect(html).toContain("OpenCode uses its official global plugin");
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

  it("caps a runaway thread list while reporting the full total", () => {
    const sessions = Array.from({ length: 60 }, (_, index): SessionStatus => ({
      id: `thread-${index}`,
      app: "Codex",
      title: `Safety row ${index}`,
      state: "idle",
      updated_at_ms: 60_000 - index,
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
    expect(html).toContain("Safety row 49");
    expect(html).not.toContain("Safety row 50");
    expect(html).toContain("50/60");
  });

  it("renders the daemon LED frame and clickable Agent Key assignment", () => {
    const live = snapshot([
      {
        id: "codex:live",
        app: "Codex",
        title: "Live release thread",
        state: "working",
        updated_at_ms: Date.now(),
      },
    ]);
    live.device_name = "mock";
    live.agent_key_led_frame = {
      keys: [
        {
          session_id: "codex:live",
          state: "working",
          color: "#12AB34",
          focused: true,
        },
      ],
      brightness: 60,
      paused: false,
    };
    const html = renderToStaticMarkup(
      <Popover
        snapshot={live}
        dark
        onOpenSettings={noop}
        onTogglePause={noop}
        onQuit={noop}
        onAgentKey={noop}
      />,
    );
    expect(html).toContain("#12AB34");
    expect(html).toContain("Agent Key 1: Live release thread");
  });

  it("shows an honest offline state instead of demo sessions", () => {
    const offline = snapshot();
    offline.device_name = "daemon-offline";
    const html = renderToStaticMarkup(
      <Popover
        snapshot={offline}
        dark
        onOpenSettings={noop}
        onTogglePause={noop}
        onQuit={noop}
      />,
    );
    expect(html).toContain("No live daemon connection is available");
    expect(html).not.toContain("microbridge — HID reconnect on wake");
  });
});
