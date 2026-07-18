# Privacy

Microbridge is a **local-only** control plane. It does not phone home.

## What stays on your machine

| Data | Where | Why |
|---|---|---|
| Agent session journals | Read from paths like `~/.codex/sessions` and Claude project folders | Derive session titles/state for the menu bar and LED mapping |
| Config | `~/.microbridge/config.toml` | Key source, lighting, appearance |
| Daemon log | `~/.microbridge/daemon.log` (or Homebrew service logs) | Debug |
| Unix socket | `~/.microbridge/microbridged.sock` (mode `0600`) | Local IPC for UI + adapters |

## What we do **not** do

- No telemetry, analytics, or crash upload
- No update pings or cloud accounts
- No network client in the daemon (auditable in `Cargo.lock`)
- No uploading of session text or source code

## Adapters

First-party adapters watch **local** session stores. Community adapters must
follow the same rule (see [docs/adapters.md](docs/adapters.md)): talk only to
local runtimes; no network I/O.

## Hardware

USB presence probing (macOS `system_profiler`) and future HID traffic stay on
the host. Device captures used for reverse-engineering are documented in
[docs/device-hid.md](docs/device-hid.md) and are not sent anywhere by Microbridge.

## Questions

Security-sensitive reports: [SECURITY.md](SECURITY.md).
