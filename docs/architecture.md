# Architecture

## Components

| Component | Runs as | Language | Required? |
|---|---|---|---|
| `microbridged` | resident daemon (launchd agent) | Rust | yes |
| First-party adapters (Codex CLI, Claude Code) | in-process modules of the daemon | Rust | bundled |
| Community adapters | separate processes on the socket | any | optional |
| Menu bar app | tray app talking to the same socket | TBD (M3) | optional, quit-able |

The daemon owns three things: the **status bus** (session registry fed by
adapters), the **focus policy** (which session owns the deck), and the
**device layer** (LED frames out, key events in). Adapters never touch the
device — see [protocol.md](protocol.md).

## Footprint budget

These are commitments, not aspirations. CI and release checklists hold the
line; regressions are release blockers.

| Metric | Budget | How |
|---|---|---|
| Idle CPU | 0.0% (no wakeups between events) | no polling loops, no timers; blocking reads on socket + HID; FSEvents/inotify for session-file watching |
| Idle RSS (daemon) | < 15 MB, target single-digit | single static Rust binary, no runtime |
| Network | **zero, by design** | no HTTP client linked; no telemetry, no update pings; auditable via `Cargo.lock` |
| Device traffic | bytes per state *transition* | LED frames written only when resolved state changes; a 32–64 byte HID report each |
| Disk | config file + log (rotated) | logs at `info` are transition-only |

First-party adapters are compiled into the daemon precisely to protect this
budget: watching `~/.codex/sessions` or Claude Code hooks is file watching,
which Rust does natively for free. Community adapters run out-of-process, so
their cost is theirs — the settings UI surfaces per-adapter footprint so users
can see who is spending what.

## Focus model — "one owner, no fighting"

The deck shows exactly one session at a time:

1. **Approvals preempt.** A session entering `awaiting_approval` takes the
   deck (and the approve/reject keys) until resolved.
2. **Pinned beats auto.** The user can pin a session from the menu bar or a
   device key; pinning disables auto-follow until unpinned.
3. **Auto-follow (M3).** Otherwise the frontmost app's active session owns the
   deck — driven by `NSWorkspace` frontmost-app notifications (event-driven,
   not polled).
4. **Fallback.** With no other signal, the most recently updated session wins.

Because adapters only publish, a misbehaving adapter can spam the bus but can
never seize the hardware.

## Platform support

- **macOS** is the reference platform (Micro owners skew Mac).
- **Linux** should work for daemon + socket; HID via `hidapi` in M2.
- **Windows** needs a named-pipe transport and is scheduled for M5.

## Security posture

- The socket lives in the user's home directory with default `0600`-style
  ownership; there is no privileged component.
- The daemon executes nothing: actions are JSON commands to adapters, which
  decide what they mean in their own app's context.
- Hardware access is best-effort reverse engineering of the Micro's HID
  protocol; the device layer is isolated in `mb-device` so a firmware change
  cannot ripple past one crate.
