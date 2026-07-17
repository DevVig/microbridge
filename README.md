# Microbridge

**An open-source control plane for the Codex Micro — one macropad, every coding agent.**

Microbridge is a tiny local daemon that bridges AI coding agents — Codex CLI, Claude Code, Cursor, T3 Code, and anything else with an adapter — to the [Work Louder Codex Micro](https://worklouder.cc/). Per-key RGB mirrors live agent state; the keys drive agent actions (approve, reject, interrupt, switch focus). No vendor desktop app required.

> **Status: pre-alpha.** Protocol v0 and the daemon skeleton. Nothing drives real hardware yet — see [ROADMAP.md](ROADMAP.md).

## Why

The Micro's best feature — bidirectional Agent Keys — currently works through exactly one vendor's desktop app. Most of us run agents in more than one place. Microbridge turns the deck into a shared, neutral surface:

- **Adapters publish state.** Each agent session reports `thinking`, `working`, `awaiting_approval`, … as transitions happen.
- **The focus policy decides.** Exactly one session owns the deck at a time; approval requests can preempt. Adapters never touch the device, so two apps can never fight over your keys.
- **The device layer renders.** State becomes LEDs; key presses become routed actions.

## Design principles

1. **Invisible footprint.** Event-driven end to end: no polling loops, no heartbeat timers. Idle CPU is 0.0% and idle RSS targets single-digit megabytes. If Microbridge is noticeable in Activity Monitor, that is a bug — the [footprint budget](docs/architecture.md#footprint-budget) is a spec, not an aspiration.
2. **Zero network.** No telemetry, no update pings, no cloud. The daemon's only I/O is a local Unix socket and the USB device. It links no HTTP client — auditable in `Cargo.lock`.
3. **Rust core, any-language adapters.** The always-resident part is a single static Rust binary. First-party adapters compile into it (in-process, ~zero overhead). Community adapters are separate processes speaking [newline-delimited JSON](docs/protocol.md) — write one in whatever you like.
4. **The UI is optional.** A menu bar companion app provides status and key remapping, and you can quit it; the daemon keeps working without it.

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
             │ menu bar app       │  optional, quit-able
             └───────────────────┘
```

Details in [docs/architecture.md](docs/architecture.md). The wire format is specified in [docs/protocol.md](docs/protocol.md). UI design direction lives in [docs/design](docs/design/README.md).

## Repository layout

```
crates/mb-protocol    wire types (serde) — the protocol's source of truth
crates/mb-device      device abstraction; mock today, HID in M2
crates/microbridged   the daemon: socket server, registry, focus policy
adapters/             out-of-process community adapters + reference impl
docs/                 protocol spec, architecture, adapter guide, design
```

## Building

```sh
cargo test          # protocol round-trips + focus policy
cargo run -p microbridged
# in another shell:
node adapters/reference-echo/index.mjs   # walks a fake session through the states
```

Requires stable Rust (see `rust-toolchain.toml`) and, for the reference adapter only, Node ≥ 20. macOS and Linux today; Windows (named pipes) is on the roadmap.

## Contributing

Adapter PRs are explicitly welcome — that is the point of the project. Start with [docs/adapters.md](docs/adapters.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Relationship to Work Louder / OpenAI

Microbridge is an independent community project. It is not affiliated with or endorsed by Work Louder or OpenAI. Driving the Micro's LEDs outside official software relies on best-effort reverse engineering of the device's HID protocol and may lag firmware updates.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. Contributions are accepted under the same terms.
