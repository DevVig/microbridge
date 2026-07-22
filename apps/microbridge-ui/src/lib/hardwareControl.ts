import type { Snapshot } from "./types";

export type HardwareControlState =
  | "available"
  | "connected"
  | "claim_failed"
  | "unavailable";

/**
 * Keep requested ownership separate from an actual HID claim.
 * `hardware_control_enabled` is consent/intent; only `device_connected` proves
 * that Microbridge currently owns the interface.
 */
export function hardwareControlState(
  snapshot: Snapshot,
): HardwareControlState {
  if (snapshot.device_connected) return "connected";

  const unavailable =
    snapshot.device_name === "mock" ||
    snapshot.device_name === "demo-browser" ||
    snapshot.device_name === "daemon-offline" ||
    !snapshot.device_name.includes("usb");
  if (unavailable) return "unavailable";

  return snapshot.config.hardware_control_enabled
    ? "claim_failed"
    : "available";
}
