import type { Snapshot } from "./types";
import { invokeTauri } from "./tauri";

export type HardwareControlState =
  | "available"
  | "connected"
  | "claim_failed"
  | "unavailable";

export function isPhysicalMicro(deviceName: string): boolean {
  return (
    deviceName.startsWith("codex-micro-") ||
    deviceName.startsWith("creator-micro-v2-")
  );
}

export function deviceTransportLabel(deviceName: string): string | null {
  if (!isPhysicalMicro(deviceName)) return null;
  if (deviceName.endsWith("-bluetooth")) return "Bluetooth";
  if (deviceName.endsWith("-usb")) return "USB";
  if (deviceName.endsWith("-hid")) return "HID";
  return null;
}

export async function requestInputMonitoringAccess(): Promise<boolean> {
  return (await invokeTauri<boolean>("request_input_monitoring_access")) ?? true;
}

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
    !isPhysicalMicro(snapshot.device_name);
  if (unavailable) return "unavailable";

  return snapshot.config.hardware_control_enabled
    ? "claim_failed"
    : "available";
}
