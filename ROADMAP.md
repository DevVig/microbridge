# Roadmap

Milestones are deliberately small; each one is independently useful and
reviewable. Issues are labeled `M0`…`M5`.

## M0 — Protocol + skeleton *(current)*
Protocol v0 spec and types, daemon with registry + focus policy v0, mock
device, reference adapter, CI.

## M1 — Real state in
In-process **Codex CLI** and **Claude Code** adapters (session-file watching
/ hooks). `microbridgectl status` for inspecting the bus. Daemon ships as a
launchd agent with `brew install`able bottle.

## M2 — Real light out
Codex Micro HID driver in `mb-device` (LED frames, key events, encoder),
behind a capability-probed device descriptor so layouts aren't hardcoded.
Documented findings from the reverse-engineering work, kept isolated in one
crate.

## M3 — Focus + menu bar
Frontmost-app auto-focus (NSWorkspace notifications), pinning, approvals
preemption end-to-end. Menu bar companion app and focus HUD implementing the
[design spec](docs/design/README.md). Key remapping UI (profiles per app).

## M4 — Community adapters
Cursor and T3 Code adapters (community-led, out-of-process), adapter
developer guide hardening, per-adapter footprint reporting in the UI.

## M5 — Portability
Windows transport (named pipes) + Windows/Linux tray. Signed release
binaries for all platforms.
