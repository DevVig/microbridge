import { describe, expect, it } from "vitest";

import type { AdapterStatus, SessionStatus } from "./types";
import { hostPresence, integrationView } from "./hosts";

function session(app: string, id = "1"): SessionStatus {
  return {
    id,
    app,
    title: "Thread",
    state: "working",
    updated_at_ms: 100,
  };
}

function adapter(
  partial: Partial<AdapterStatus> & Pick<AdapterStatus, "id" | "display_name" | "state">,
): AdapterStatus {
  return {
    kind: "native",
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
    ...partial,
  };
}

describe("hostPresence", () => {
  it("counts sessions for one app", () => {
    const presence = hostPresence(
      [session("Synara", "a"), session("Codex CLI", "b"), session("Synara", "c")],
      "Synara",
    );
    expect(presence.count).toBe(2);
  });
});

describe("integrationView", () => {
  it("marks Synara green when sessions are live", () => {
    const view = integrationView(
      adapter({ id: "synara", display_name: "Synara", state: "connected" }),
      [session("Synara")],
    );
    expect(view.light).toBe("green");
    expect(view.label).toContain("Active");
    expect(view.connectedGroup).toBe(true);
    expect(view.diagnostic).toContain("no separate adapter");
  });

  it("marks Synara yellow while waiting for sessions", () => {
    const view = integrationView(
      adapter({ id: "synara", display_name: "Synara", state: "connected" }),
      [],
    );
    expect(view.light).toBe("yellow");
    expect(view.label).toBe("Waiting");
    expect(view.connectedGroup).toBe(false);
  });

  it("flags disabled Cursor when journal sessions already exist", () => {
    const view = integrationView(
      adapter({
        id: "cursor",
        display_name: "Cursor",
        kind: "community",
        state: "disabled",
        diagnostic: "Disabled until you explicitly enable this integration.",
      }),
      [session("Cursor"), session("Cursor", "2")],
    );
    expect(view.light).toBe("yellow");
    expect(view.diagnostic).toContain("2 threads auto-detected");
  });

  it("keeps Claude Code green when the watcher is connected", () => {
    const view = integrationView(
      adapter({ id: "claude", display_name: "Claude Code", state: "connected" }),
      [],
    );
    expect(view.light).toBe("green");
    expect(view.label).toBe("Connected");
  });

  it("maps adapter errors to red", () => {
    const view = integrationView(
      adapter({
        id: "t3code",
        display_name: "T3 Code",
        kind: "community",
        state: "error",
        diagnostic: "Pairing failed.",
      }),
      [],
    );
    expect(view.light).toBe("red");
    expect(view.label).toBe("Error");
  });
});
