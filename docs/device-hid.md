# Codex Micro HID notes

Best-effort reverse-engineering of the Work Louder / OpenAI Codex Micro
(kbd-1.0). All HID code lives in [`crates/mb-device`](../crates/mb-device).
Firmware changes may invalidate this document — treat it as a living map.

## Status

| Capability | Status |
|---|---|
| USB open / claim | presence probe on macOS (`system_profiler`); claim deferred until VID/PID + report map confirmed |
| LED frames (6 Agent Keys) | mock logs frames; HID packing TBD |
| Key / dial / joystick input | trait defined (`DeviceInput`); no live reports yet |
| Bluetooth | out of scope for M2 (USB-first) |

Without a claimed device the daemon uses [`MockDevice`](../crates/mb-device/src/lib.rs)
so CI and headless installs stay green.

## Hardware (product facts)

- 13 mechanical switches, rotary encoder, planar joystick, capacitive touch
- 6 frosted Agent Keys with per-key RGB
- USB-C and BLE; Microbridge M2 targets USB only
- Also configurable via Work Louder Input / VIA for non-agent layers

## Probe checklist (when hardware is available)

1. `system_profiler SPUSBDataType` / `lsusb` — record VID/PID/iProduct
2. Capture HID report descriptor (`hidutil` / Wireshark USBPcap / `usbhid-dump`)
3. Observe ChatGPT desktop LED traffic while forcing each `AgentState`
4. Map report IDs → Agent Key RGB slots and command key bitfields
5. Document double-press window (ChatGPT uses ≤350ms) for Agent Keys

Until those captures land, `HidDevice::open()` never claims the interface —
we refuse to guess report layouts that could fight ChatGPT desktop.

## Exclusive ownership

Only one process should drive Agent Key LEDs. If ChatGPT desktop is open and
owning the Micro, pause Microbridge LEDs (Settings → Pause LEDs) or quit the
desktop bridge. The companion empty state should mention this.

## Descriptor-driven layout

The daemon never hardcodes a key grid. `DeviceDescriptor` reports
`agent_key_count`, dial, and joystick capabilities. UI device twins and LED
frames size themselves from that descriptor.
