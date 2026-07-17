# Community adapters

Out-of-process adapters live here, one folder per runtime, any language.
Read [docs/adapters.md](../docs/adapters.md) for the contract and the review
checklist, and [docs/protocol.md](../docs/protocol.md) for the wire format.

| Adapter | Status | Language |
|---|---|---|
| [`reference-echo`](reference-echo/) | working example | Node (no deps) |
| `cursor` | wanted — see adapter issues | — |
| `t3code` | wanted — see adapter issues | — |

First-party adapters (Codex CLI, Claude Code) are compiled into the daemon
and live in `crates/`, not here — see
[docs/architecture.md](../docs/architecture.md).
