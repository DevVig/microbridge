# Codex Micro HID notes

Best-effort reverse-engineering of the Work Louder / OpenAI Codex Micro
(kbd-1.0). All HID code lives in [`crates/mb-device`](../crates/mb-device).
Firmware changes may invalidate this document — treat it as a living map.

## Status

| Capability | Status |
|---|---|
| USB identity (VID/PID) | **known** — from ChatGPT Desktop `@worklouder/wl-device-kit` |
| USB/Bluetooth presence | macOS HID registry matches VID `0x303A`, known PIDs, and vendor usage page `0xFF00` (Detected) |
| HID claim / LED write | opt-in: build with `hid` feature (default) + Device → Hardware control; `MICROBRIDGE_HID_CLAIM=1` is a diagnostic override |
| LED frames (6 Agent Keys) | packed as JSON-RPC `v.oai.thstatus` over framed HID reports |
| Key / dial / joystick input | notify parsers and conservative routing map implemented (`v.oai.hid` / `v.oai.rad`); exact codes remain gated on physical capture |
| Bluetooth | supported when the paired Micro exposes its vendor HID interface; transport is reported separately from USB |

Without a claimed device the daemon uses [`MockDevice`](../crates/mb-device/src/lib.rs)
so CI and headless installs stay green. A detected-but-unclaimed device still shows
in the UI as **Detected** (not Connected).
The daemon refreshes presence every two seconds while keeping explicit Retry
semantics for a stable device whose claim failed.

## Source of truth (no hardware required)

ChatGPT macOS app ships the protocol stack:

```text
/Applications/ChatGPT.app/Contents/Resources/app.asar
  → node_modules/@worklouder/device-kit-oai
  → node_modules/@worklouder/wl-device-kit
```

Useful symbols mined from that kit:

- `WL_VID`, `DEVICE_REGISTRY`, `WL_MANUFACTURER`
- HID framing in `WLDeviceCommImpl.sendDataHID` / `parseHIDReport`
- OAI RPC in `RPCApiOAI` (`v.oai.thstatus`, `v.oai.rgbcfg`, notify keys)

## USB identity

| Field | Value |
|---|---|
| Vendor ID | `0x303A` (Espressif / Work Louder) |
| Manufacturer | `Work Louder` / `Work_Louder` |
| Codex Micro PID | `0x8360` (`project_2077`) |
| Creator Micro V2 PIDs | `0x8297`, `0x8298` |
| Vendor HID usage page | `0xFF00` |

Microbridge treats Codex Micro + Creator Micro V2 PIDs as supported.

## HID report framing

64-byte interrupt reports on the vendor usage page:

| Offset | Field |
|---|---|
| 0 | Report ID `0x06` |
| 1 | Channel: `1` = debug log, `2` = JSON-RPC |
| 2 | Payload length `0..=61` |
| 3… | UTF-8 payload |

Messages longer than 61 bytes are split across multiple reports. ChatGPT opens
the interface with **non-exclusive** access on macOS; Microbridge does the same
when claiming (`hidapi` `set_open_exclusive(false)`).

## JSON-RPC (Work Louder compact)

Requests are **not** JSON-RPC 2.0 envelopes — just:

```json
{"method":"v.oai.thstatus","params":[...],"id":42}
```

- `id` must be in `0..999` (firmware constraint)
- Responses carry `id`; notifications omit `id` and set `method` / `m`

### OAI methods Microbridge uses

| Method | Direction | Purpose |
|---|---|---|
| `v.oai.thstatus` | host → device | Per-thread / Agent Key lighting |
| `v.oai.rgbcfg` | host → device | Keys + ambient ring config (reserved) |
| `v.oai.hid` | device → host | Key events (`params.k`, `act`, `ag`) |
| `v.oai.rad` | device → host | Joystick (`params.a` angle, `d` distance) |

### Thread lighting params (minimized)

| Field | Meaning |
|---|---|
| `id` | Thread / Agent Key index |
| `c` | Packed RGB integer |
| `b` | Brightness `0.0`–`1.0` |
| `e` | Effect enum (0 off … 6 shallowBreath) |
| `s` | Effect speed `0.0`–`1.0` |
| `sk` / `sa` | Sync keys / ambient to this thread (`1` / `0`) |

Effects: `off=0`, `solid=1`, `snake=2`, `rainbow=3`, `breath=4`,
`gradient=5`, `shallowBreath=6`.

## Claiming the device

Default daemon behavior: **probe only** (Detected). Choose **Claim Codex Micro**
in the home popover or the menu-bar icon’s right-click menu. The same advanced
control remains under **Settings → Device**. A requested claim that remains
Detected can be retried after closing the other HID owner.
For command-line diagnostics:

```bash
export MICROBRIDGE_HID_CLAIM=1
# quit or pause ChatGPT Desktop Agent Key ownership if LEDs fight
microbridged
```

Build flag: `mb-device` feature `hid` (on by default). Disable with
`--no-default-features` if you need a hidapi-free binary.

## Hardware (product facts)

- 13 mechanical switches, rotary encoder, planar joystick, capacitive touch
- 6 frosted Agent Keys with per-key RGB
- USB-C and BLE; Microbridge discovers BLE when the paired device exposes its
  vendor HID interface through the macOS HID registry
- Also configurable via Work Louder Input / VIA for non-agent layers

## Remaining validation (needs hardware)

Run the [hardware bring-up runbook](hardware-bringup.md) — it drives each of
these with exact commands. `microbridgectl hid-capture` harvests item 2 in one
pass.

1. Confirm live PID/iProduct string on the shipping Codex Micro unit
2. Map `v.oai.hid` key strings → Agent Key indices / Approve / Reject / etc.
3. Confirm double-press window (ChatGPT uses ≤350ms) for Agent Keys
4. Tune effect/speed mapping vs ChatGPT Desktop visuals
5. Ownership UX when ChatGPT Desktop and Microbridge both want LEDs

## Exclusive ownership

Only one process should drive Agent Key LEDs. If ChatGPT desktop is open and
owning the Micro, pause Microbridge LEDs (Settings → Pause LEDs), leave claim
off, or quit the desktop bridge. The companion empty state should mention this.

## Descriptor-driven layout

The daemon never hardcodes a key grid. `DeviceDescriptor` reports
`agent_key_count`, dial, and joystick capabilities. UI device twins and LED
frames size themselves from that descriptor.
