# Microbridge protocol v0

Normative spec for adapter ↔ daemon communication. The Rust types in
[`crates/mb-protocol`](../crates/mb-protocol/src/lib.rs) are the source of
truth; if this document and the types disagree, fix one of them in the same PR.

## Transport

- **Unix domain socket** at `$MICROBRIDGE_SOCKET`, defaulting to
  `~/.microbridge/microbridged.sock`. (Windows named pipes: see ROADMAP M5.)
- **Newline-delimited JSON** (NDJSON): one message per line, UTF-8, `\n`
  terminated. Blank lines are ignored.
- One connection per adapter. The daemon treats a closed connection as the end
  of that adapter's sessions only after `bye` — crash recovery is the
  adapter's job on reconnect.

## Principles

- **Event-driven only.** Messages are sent on state *transitions*. There are no
  heartbeats, no keepalives, no polling. Socket liveness is the liveness
  signal.
- **Status is a full replacement.** Every `status` carries the complete session
  record; the daemon never merges partial updates.
- **Adapters never touch the device.** They publish state and receive routed
  actions. The daemon's focus policy alone decides what the hardware shows.

## Messages: adapter → daemon

### `hello` — required first message

```json
{"type":"hello","adapter":"codex-cli","protocol_version":0}
```

### `status` — on every session transition

```json
{"type":"status","session":{
  "id":"codex:0195fa2e",
  "app":"Codex CLI",
  "title":"fix flaky e2e retries",
  "state":"awaiting_approval",
  "updated_at_ms":1784732400000
}}
```

`state` is one of: `idle` · `thinking` · `working` · `awaiting_approval` ·
`done` · `error`. `title` is optional and defaults to `""`.

### `bye` — session ended

```json
{"type":"bye","session_id":"codex:0195fa2e"}
```

## Messages: daemon → adapter

### `action` — a key press routed to a session the adapter owns

```json
{"type":"action","session_id":"codex:0195fa2e","action":"approve"}
```

`action` is one of: `approve` · `reject` · `interrupt` · `new_session` ·
`cycle_focus`. Adapters should treat unknown actions as a no-op and log them.

## Focus policy (daemon-internal, v0)

1. A session in `awaiting_approval` preempts focus (most recent wins).
2. Otherwise the currently focused session keeps the deck while it exists.
3. Otherwise the most recently updated session takes focus.

Frontmost-app auto-focus and pinning ship with the menu bar app (M3) and do
not change the wire format.

## Versioning

`protocol_version` is a single integer. The daemon accepts mismatched
adapters but logs a warning; breaking wire changes bump the version and are
called out in release notes. Additive fields are not breaking — adapters and
daemon must ignore unknown fields.
