# Privacy

Microbridge is a local-first control plane. It has no telemetry or Microbridge
cloud account. Network access occurs only for a user-requested or user-enabled
update check or an explicitly enabled and paired T3 Code environment. There is
no unconfigured network traffic.

## What stays on your machine

| Data | Where | Why |
|---|---|---|
| Agent session journals | Read from paths like `~/.codex/sessions` and Claude project folders | Derive session titles/state for the menu bar and LED mapping |
| Config | `~/.microbridge/config.toml` | Key source, lighting, appearance |
| Daemon log | `~/.microbridge/daemon.log` (or Homebrew service logs) | Debug |
| Unix socket | `~/.microbridge/microbridged.sock` (mode `0600`) | Local IPC for UI + adapters |
| T3 Code credential | macOS Keychain (`ai.microbridge.t3code`) | Access the environment the user explicitly paired |

## What we do **not** do

- No telemetry, analytics, or crash upload
- No telemetry or Microbridge cloud account
- No unconfigured network traffic; update checks and T3 access require opt-in
- No uploading of session text or source code

## Adapters

First-party adapters watch local session stores. The bundled Cursor integration
sends metadata-only lifecycle events over the local socket and never sends
prompt, response, transcript, or tool argument content. The T3 Code adapter
talks only to the exact environment the user pairs, using scoped orchestration
access.

The one-time T3 pairing token is exchanged immediately, never logged, and not
stored. Removing the adapter deletes its Keychain credential.

## Hardware

USB presence probing (macOS `system_profiler`) and future HID traffic stay on
the host. Device captures used for reverse-engineering are documented in
[docs/device-hid.md](docs/device-hid.md) and are not sent anywhere by Microbridge.

## Questions

Security-sensitive reports: [SECURITY.md](SECURITY.md).
