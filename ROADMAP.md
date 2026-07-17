# Roadmap

Milestones are deliberately small; each one is independently useful and
reviewable. Issues are labeled `M0`…`M5`.

## M0 — Protocol + skeleton ✅
Protocol v0 spec and types, daemon with registry + focus policy v0, mock
device, reference adapter, CI.

## M1 — Real state in ✅ (foundation)
In-process **Codex CLI** and **Claude Code** adapters (session-file watching).
`microbridgectl status` for inspecting the bus. launchd install script +
Homebrew formula skeleton. UI/control protocol (`subscribe` / config) and
five key-source modes.

## M2 — Real light out 🚧
Codex Micro HID driver in `mb-device` (LED frames, key events, encoder),
behind a capability-probed device descriptor. Mock remains the default until
VID/PID + report map are captured — see [docs/device-hid.md](docs/device-hid.md).

## M3 — Focus + menu bar 🚧
Tauri companion (`apps/microbridge-ui`) ports the approved MagicPath surfaces
(popover / settings / HUD). Frontmost-app auto-focus via `frontmost_app`
config (NSWorkspace wiring next). Key remapping UI continues to track the
MagicPath device twin.

## M4 — Community adapters 🚧
Cursor and T3 Code adapter scaffolds under `adapters/`. Harden as session
sources appear. Per-adapter footprint reporting in Settings.

## M5 — Portability
Windows transport (named pipes) + Windows/Linux tray. Signed release
binaries for all platforms.
