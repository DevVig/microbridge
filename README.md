# Microbridge

**An open-source control plane for the Codex Micro — one macropad, every coding agent.**

Microbridge is a tiny local daemon that bridges AI coding agents — Codex CLI, Claude Code, Cursor, T3 Code, and anything else with an adapter — to the [Work Louder Codex Micro](https://worklouder.cc/). Per-key RGB mirrors live agent state; the keys drive agent actions (approve, reject, interrupt, switch focus). No vendor desktop app required.

> **Status: early public alpha (`v0.1.x`).** Menu bar UI, local daemon, in-process Codex/Claude watchers, and signed macOS packages are shipping. **HID protocol (VID/PID, framing, `v.oai.thstatus`) is implemented from ChatGPT’s Work Louder kit**; live LED writes stay opt-in (`MICROBRIDGE_HID_CLAIM=1`) until hardware validation. See [docs/device-hid.md](docs/device-hid.md).

## Screenshots

Optional companion UI — status and setup only. Agent actions stay on the Micro.

<p align="center">
  <img src="docs/screenshots/menu-bar-popover.png" alt="Microbridge menu bar popover" width="720" />
</p>

<p align="center">
  <img src="docs/screenshots/settings.png" alt="Microbridge Settings with device twin" width="720" />
</p>

<p align="center">
  <img src="docs/screenshots/focus-hud.png" alt="Microbridge focus HUD" width="720" />
</p>

| Surface | File |
|---|---|
| Menu bar popover | [`docs/screenshots/menu-bar-popover.png`](docs/screenshots/menu-bar-popover.png) |
| Settings (device twin) | [`docs/screenshots/settings.png`](docs/screenshots/settings.png) |
| Focus HUD | [`docs/screenshots/focus-hud.png`](docs/screenshots/focus-hud.png) |

Design spec: [docs/design/README.md](docs/design/README.md).

## Why

The Micro's best feature — bidirectional Agent Keys — currently works through exactly one vendor's desktop app. Most of us run agents in more than one place. Microbridge turns the deck into a shared, neutral surface:

- **Adapters publish state.** Each agent session reports `thinking`, `working`, `awaiting_approval`, … as transitions happen.
- **The focus policy decides.** Exactly one session owns the deck at a time; approval requests can preempt. Adapters never touch the device, so two apps can never fight over your keys.
- **The device layer renders.** State becomes LEDs; key presses become routed actions.

## Design principles

1. **Invisible footprint.** Event-driven end to end: no polling loops, no heartbeat timers. Idle CPU is 0.0% and idle RSS targets single-digit megabytes. If Microbridge is noticeable in Activity Monitor, that is a bug — the [footprint budget](docs/architecture.md#footprint-budget) is a spec, not an aspiration.
2. **Zero network.** No telemetry, no update pings, no cloud. The daemon's only I/O is a local Unix socket and the USB device. It links no HTTP client — auditable in `Cargo.lock`.
3. **Rust core, any-language adapters.** The always-resident part is a single static Rust binary. First-party adapters compile into it (in-process, ~zero overhead). Community adapters are separate processes speaking [newline-delimited JSON](docs/protocol.md) — write one in whatever you like.
4. **The menu bar app is the product UI.** Configure keys, lighting, and adapters there. The daemon keeps the hardware alive underneath; `microbridgectl` is a support/debug escape hatch.

## Architecture

```
┌───────────┐ ┌─────────────┐   ┌──────────────────┐
│ Codex CLI │ │ Claude Code │   │ community adapters │
│ (in-proc) │ │ (in-proc)   │   │ (any language)     │
└─────┬─────┘ └─────┬───────┘   └────────┬─────────┘
      │  in-process │            NDJSON over unix socket
      ▼             ▼                     ▼
┌──────────────────────────────────────────────────┐
│ microbridged — status bus + focus policy          │  Rust daemon
├──────────────────────────────────────────────────┤
│ device layer (HID) → Codex Micro LEDs / keys      │
└──────────────────────┬───────────────────────────┘
                       │ same socket (status + commands)
             ┌─────────┴─────────┐
             │ menu bar app       │  primary UI (Tauri)
             └───────────────────┘
```

Details in [docs/architecture.md](docs/architecture.md). The wire format is specified in [docs/protocol.md](docs/protocol.md). UI design direction lives in [docs/design](docs/design/README.md).

## Repository layout

```
crates/mb-protocol     wire types (serde) — the protocol's source of truth
crates/mb-device       device abstraction; HID framing + opt-in claim
crates/mb-adapters     first-party Codex CLI + Claude Code watchers
crates/microbridged    the daemon: socket server, registry, focus, key source
crates/microbridgectl  support/debug CLI (`status`)
apps/microbridge-ui    menu bar app — primary UI (MagicPath-faithful)
adapters/              out-of-process community adapters + reference impl
docs/                  protocol, architecture, adapter guide, design, HID notes
```

## Install

Full guide: **[INSTALL.md](INSTALL.md)**. Governance / branch rules: **[docs/governance.md](docs/governance.md)**.

**macOS (recommended — Homebrew installs the menu bar app + daemon):**

```sh
brew tap DevVig/microbridge https://github.com/DevVig/microbridge
brew install microbridge
brew services start microbridge
open ~/Applications/Microbridge.app
# updates: brew update && brew upgrade microbridge
# optional background updates: brew autoupdate start --upgrade --cleanup
```

From source / Linux:

```sh
./scripts/install.sh                 # macOS launchd
./scripts/install-linux-systemd.sh   # Linux systemd --user
```

## Building (dev)

```sh
cargo test          # protocol, focus, key-source, adapters
cargo run -p microbridged
# in another shell:
cargo run -p microbridgectl -- status
node adapters/reference-echo/index.mjs   # walks a fake session through the states
```

Companion UI:

```sh
cd apps/microbridge-ui && npm install && npm run dev
# or: make ui
```

Requires stable Rust (see `rust-toolchain.toml`) and, for Node adapters / UI, Node ≥ 20. macOS and Linux today; Windows (named pipes) is on the roadmap.

## Contributing

Adapter PRs are explicitly welcome — that is the point of the project. Start with [docs/adapters.md](docs/adapters.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Relationship to Work Louder / OpenAI

Microbridge is an independent community project. It is not affiliated with or endorsed by Work Louder or OpenAI. Driving the Micro's LEDs outside official software relies on best-effort reverse engineering of the device's HID protocol and may lag firmware updates.

## Acknowledgments

Microbridge only exists because the Codex Micro is *open* — and that was a choice, not an accident.

OpenAI is a for-profit company, and it would have been easy to lock the Micro to a single first-party app: a closed protocol, an exclusive USB claim, no way for anyone else to light a key. They did the opposite — they ship the device kit in the open, keep the HID interface **non-exclusive** so third-party software can coexist with the official experience instead of fighting it, and keep giving users a choice (Codex CLI is open source; the models are reachable over documented APIs). None of that was required of them. **Thank you.**

Thanks too to **Work Louder** for designing a genuinely hackable macropad, and to **everyone who writes an adapter, files an issue, or plugs in a device and tells us what really happens** — adapters are the point of this project.

Full notes: [ACKNOWLEDGMENTS.md](ACKNOWLEDGMENTS.md).

## License

[MIT](LICENSE) licensed — a permissive, OSI-approved open source license. Contributions are accepted under the same terms (inbound = outbound).
