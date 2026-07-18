# Security policy

Microbridge runs unprivileged on your Mac, listens only on a **local Unix
socket** (mode `0600` under `~/.microbridge/`), and performs **no network I/O**.
It sits between your coding agents (session journals / adapter IPC) and —
eventually — a USB input device (Codex Micro).

## Scope we care about

- Unauthorized clients attaching to the daemon socket
- Privilege escalation via the menu bar app or daemon
- Unexpected network egress from daemon / first-party adapters
- Leaking session contents beyond the local machine
- Malicious adapter PRs that scrape private Electron internals or add idle
  footprint / network I/O (declined on sight — see CONTRIBUTING)

## Reporting

**Report vulnerabilities privately** via GitHub's
[private vulnerability reporting](../../security/advisories/new) on this
repository. Please do not open public issues for security problems.

You can expect an acknowledgment within a week. Supported version: the
latest release (pre-1.0, there are no backports).

## Out of scope (for now)

- Full HID claim / exclusive USB ownership (landing with device captures)
- Guarantees about third-party community adapters you choose to run
