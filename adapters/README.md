# Community adapters

Out-of-process adapters live here, one folder per runtime, any language.
Read [docs/adapters.md](../docs/adapters.md) for the contract and the review
checklist, and [docs/protocol.md](../docs/protocol.md) for the wire format.

| Adapter | Status | Language |
|---|---|---|
| [`reference-echo`](reference-echo/) | working example | Node (no deps) |
| [`cursor`](cursor/) | bundled managed hooks | Node |
| [`t3code`](t3code/) | daemon-owned paired HTTP | Rust |
| [`factory`](factory/) | bundled official hooks + JSON-RPC controls | Rust helper |
| [`synara`](synara/) | built-in host attribution | Rust |
| [`conductor`](conductor/) | built-in host attribution | Rust |

First-party watchers (Codex CLI, Claude Code, including their embedding hosts) are compiled into the daemon
(`crates/mb-adapters`) — see [docs/architecture.md](../docs/architecture.md).
