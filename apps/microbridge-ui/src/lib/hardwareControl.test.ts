import { describe, expect, it } from "vitest";
import {
  deviceTransportLabel,
  hardwareControlState,
  isPhysicalMicro,
} from "./hardwareControl";
import type { Snapshot } from "./types";

function snapshot(
  deviceName: string,
  connected = false,
  requested = false,
): Snapshot {
  return {
    sessions: [],
    focused_session_id: null,
    agent_key_session_ids: [null, null, null, null, null, null],
    agent_key_led_frame: { keys: [], brightness: 80, paused: false },
    device_connected: connected,
    device_name: deviceName,
    config: {
      key_source: "most_recent",
      pinned_session_ids: [],
      app_priority: [],
      custom_key_ids: [],
      pinned_focus: null,
      approvals_interrupt: true,
      pause_leds: false,
      appearance: "system",
      lighting_preset: "codex",
      state_colors: {
        idle: "#000000",
        thinking: "#000000",
        working: "#000000",
        awaiting_approval: "#000000",
        done: "#000000",
        error: "#000000",
      },
      brightness: 80,
      sleep_minutes: 3,
      hardware_control_enabled: requested,
      adapters: {},
      frontmost_app: null,
    },
    adapters: [],
  };
}

describe("hardwareControlState", () => {
  it("distinguishes actual ownership from requested ownership", () => {
    expect(hardwareControlState(snapshot("codex-micro-usb"))).toBe(
      "available",
    );
    expect(hardwareControlState(snapshot("codex-micro-usb", false, true))).toBe(
      "claim_failed",
    );
    expect(hardwareControlState(snapshot("codex-micro-usb", true, false))).toBe(
      "connected",
    );
    expect(hardwareControlState(snapshot("codex-micro-bluetooth"))).toBe(
      "available",
    );
  });

  it("recognizes supported HID transports", () => {
    expect(isPhysicalMicro("codex-micro-usb")).toBe(true);
    expect(isPhysicalMicro("codex-micro-bluetooth")).toBe(true);
    expect(deviceTransportLabel("codex-micro-bluetooth")).toBe("Bluetooth");
    expect(deviceTransportLabel("unrelated-usb")).toBeNull();
    expect(isPhysicalMicro("daemon-offline")).toBe(false);
  });

  it("does not offer hardware actions for non-device surfaces", () => {
    expect(hardwareControlState(snapshot("mock"))).toBe("unavailable");
    expect(hardwareControlState(snapshot("demo-browser"))).toBe("unavailable");
    expect(hardwareControlState(snapshot("daemon-offline"))).toBe(
      "unavailable",
    );
    expect(hardwareControlState(snapshot("not connected"))).toBe(
      "unavailable",
    );
  });
});
