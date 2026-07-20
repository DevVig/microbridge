# Integration catalog

Integration documentation lives here, one folder per runtime. The local socket
protocol still calls an out-of-process connector an `adapter`; that is an
implementation term, not a separate product category.
Read [docs/adapters.md](../docs/adapters.md) for the contract and the review
checklist, and [docs/protocol.md](../docs/protocol.md) for the wire format.

## Built-in integrations

| Integration | Status | Language |
|---|---|---|
| [`t3code`](t3code/) | daemon-owned paired HTTP | Rust |
| [`factory`](factory/) | bundled official hooks + JSON-RPC controls | Rust helper |
| [`cnvs`](cnvs/) | native authenticated loopback control | Rust |
| [`synara`](synara/) | built-in host attribution | Rust |
| [`conductor`](conductor/) | built-in host attribution | Rust |

These integrations are daemon-owned or reuse watchers compiled into the daemon.
See [docs/architecture.md](../docs/architecture.md).

## Managed and out-of-process integrations

| Integration | Status | Language |
|---|---|---|
| [`cursor`](cursor/) | bundled managed hooks | Node |
| [`opencode`](opencode/) | bundled global plugin + session interrupt | Node (no deps) |
| [`reference-echo`](reference-echo/) | community adapter example | Node (no deps) |

Out-of-process adapters may be written in any language and communicate over the
documented local socket.
