# Microbridge protocol v0

Normative spec for adapter ↔ daemon ↔ UI communication. The Rust types in
[`crates/mb-protocol`](../crates/mb-protocol/src/lib.rs) are the source of
truth; if this document and the types disagree, fix one of them in the same PR.

## Transport

- **Unix domain socket** at `$MICROBRIDGE_SOCKET`, defaulting to
  `~/.microbridge/microbridged.sock`. (Windows named pipes: see ROADMAP M5.)
- **Newline-delimited JSON** (NDJSON): one message per line, UTF-8, `\n`
  terminated. Blank lines are ignored.
- One connection per client. The daemon treats a closed adapter connection as
  the end of that adapter's sessions. UI disconnects do not affect the bus.

## Principles

- **Event-driven only.** Messages are sent on state *transitions*. There are no
  heartbeats, no keepalives, no polling. Socket liveness is the liveness
  signal.
- **Status is a full replacement.** Every `status` carries the complete session
  record; the daemon never merges partial updates.
- **Adapters never touch the device.** They publish state and receive routed
  actions. The daemon's focus policy alone decides what the hardware shows.
- **UI never talks to HID.** The companion mirrors daemon-resolved focus and
  writes config only.

## Client roles

`hello` accepts an optional `role` (`adapter` default, or `ui`):

```json
{"type":"hello","adapter":"reference-echo","protocol_version":0}
{"type":"hello","adapter":"microbridge-ui","protocol_version":0,"role":"ui"}
```

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

## Messages: UI → daemon

### `subscribe` — request a snapshot and subsequent events

```json
{"type":"subscribe"}
```

### `get_config` / `set_config`

```json
{"type":"get_config"}
{"type":"set_config","config":{ /* DaemonConfig */ }}
```

Config fields include `key_source` (`most_recent` · `focused_app` · `pinned` ·
`priority` · `custom`), `pinned_focus`, `approvals_interrupt`, `pause_leds`,
`appearance`, `lighting_preset`, `state_colors`, `brightness`,
`sleep_minutes`, `frontmost_app`, and key-assignment lists. Persisted at
`~/.microbridge/config.toml`.

## Messages: daemon → UI

### `snapshot`

Full bus view: sessions, focused session, six Agent Key assignments, device
connection, and config.

### `event`

Incremental `BusEvent`: `session_upserted`, `session_removed`,
`focus_changed`, `agent_keys_changed`, `device_changed`, `config_changed`.

### `config`

Response to `get_config` / ack of `set_config`.

## Focus policy (daemon-internal)

1. `pinned_focus` if that session still exists.
2. A session in `awaiting_approval` preempts (when `approvals_interrupt`).
3. Otherwise the currently focused session keeps the deck while it exists.
4. Otherwise the frontmost app's most recent session (via `frontmost_app`).
5. Otherwise the most recently updated session.

## Key source (six Agent Keys)

| Mode | Behavior |
|---|---|
| `most_recent` | Cross-app; six newest sessions (default) |
| `focused_app` | Repopulate from the app that owns the deck |
| `pinned` | First six `pinned_session_ids` |
| `priority` | Approvals / active / app-priority ordering |
| `custom` | Explicit `custom_key_ids` (empty string = unassigned) |

Command keys always route to the single focused session.

## Versioning

`protocol_version` is a single integer. The daemon accepts mismatched
clients but logs a warning; breaking wire changes bump the version.
Additive fields are not breaking — clients and daemon must ignore unknown
fields. UI role + subscribe/config messages are additive in v0.
