# Architecture

## Components

| Component | Runs as | Language | Required? |
|---|---|---|---|
| `microbridged` | app-owned child (standard GUI) or launchd service (explicit headless mode) | Rust | yes |
| First-party integrations (ChatGPT, Claude Desktop, Codex CLI, Claude Code, CNVS, Synara/Conductor attribution) | in-process modules of the daemon | Rust | bundled |
| Managed integrations (OpenCode, Cursor, Factory, T3 Code) | host plugins/hooks or socket clients | Rust/Node | opt-in |
| Community integrations | separate processes on the socket | any | optional |
| Menu bar app | primary UI (tray + settings + focus HUD) | Tauri 2 + React (`apps/microbridge-ui`) | yes (default install) |

The daemon owns three things: the **status bus** (session registry fed by
adapters), the **focus policy** (which session owns the deck), and the
**device layer** (LED frames out, key events in). Integrations never touch the
device — see [protocol.md](protocol.md).

On macOS, the signed menu-bar app is the only standard Login Item and starts
the bundled daemon for its own lifetime. Legacy UI/daemon LaunchAgents migrate
only after the native login item and bundled socket are proven working. Users
who deliberately enable the Homebrew headless service may still have a separate
daemon background item.

## Footprint budget

These are commitments, not aspirations. CI and release checklists hold the
line; regressions are release blockers.

| Metric | Budget | How |
|---|---|---|
| Idle CPU | near-idle with bounded wakeups | local session and frontmost-app watchers are event-driven; enabled hardware input is drained on a 16 ms tick, CNVS refreshes its local snapshot every 2 seconds while agents are active and every 10 seconds while idle, and an enabled paired T3 adapter refreshes at 900 ms with exponential backoff. |
| Idle RSS (daemon) | < 15 MB, target single-digit | single static Rust binary, no runtime |
| Network | explicit only | no telemetry; CNVS uses only its token-authenticated loopback endpoint, update checks are opt-in, T3 traffic requires an enabled paired environment, and Factory invokes the signed-in Droid CLI only for a requested control |
| Device traffic | bytes per state *transition* | LED frames written only when resolved state changes; a 32–64 byte HID report each |
| Disk | config file + log (rotated) | logs at `info` are transition-only |

First-party integrations are compiled into the daemon precisely to protect this
budget: watching `~/.codex/sessions` or Claude Code hooks is file watching.
The paired T3 adapter uses a bounded refresh with exponential backoff because
the supported HTTP contract is snapshot-based. Cursor and Factory use one-shot
managed hooks and leave no resident helper process. Factory starts Droid
JSON-RPC only for an interrupt or reasoning-effort action and exits afterward.
OpenCode uses its official event plugin and creates no polling loop; reconnects
to the local daemon use bounded backoff. CNVS exposes snapshot state, so its in-process integration uses an adaptive
local-only refresh and publishes only changed sessions. It rereads CNVS's
endpoint descriptor for each scan or action, never persists its token, and
rejects endpoints that are not loopback addresses.

## Focus model — "one owner, no fighting"

The deck shows exactly one session at a time:

1. **Approvals preempt.** A session entering `awaiting_approval` takes the
   deck (and the approve/reject keys) until resolved.
2. **Pinned beats auto.** The user can pin a session from Settings or a
   device key; pinning disables auto-follow until unpinned.
3. **Auto-follow.** Otherwise the frontmost app's active session owns the
   deck — driven by `NSWorkspace` frontmost-app notifications (event-driven,
   not polled).
4. **Fallback.** With no other signal, the most recently updated session wins.

Integrations never seize the hardware. Commands route only to the focused session's
owner and only when that owner advertised the required capability.

## Platform support

- **macOS** is the reference platform (Micro owners skew Mac).
- **Linux** should work for daemon + socket; HID via `hidapi` in M2.
- **Windows** needs a named-pipe transport and is scheduled for M5.

## Security posture

- The socket lives in the user's home directory with mode `0600`; there is no
  privileged component. The logged-in macOS user is the trust boundary.
- Management messages require a completed, protocol-compatible `ui` handshake.
  Client roles prevent accidental adapter privilege; they do not claim to
  isolate mutually hostile processes already running as the same user.
- Actions route only through advertised host contracts. Most are JSON commands
  to adapters; Factory actions start the user-installed `droid` CLI in its
  documented JSON-RPC mode for that single request. CNVS actions use its
  authenticated loopback API and include the exact canvas and terminal node.
- Hardware access is best-effort reverse engineering of the Micro's HID
  protocol; the device layer is isolated in `mb-device` so a firmware change
  cannot ripple past one crate.
