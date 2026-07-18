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

- **Transition-oriented.** Socket adapters send on state transitions. Managed
  one-shot IDE hooks use expiring lifecycle leases; the paired T3 adapter
  refreshes its supported orchestration snapshot and emits only changed rows.
- **Status is a full replacement.** Every `status` carries the complete session
  record; the daemon never merges partial updates.
- **Adapters never touch the device.** They publish state and receive routed
  actions. The daemon's focus policy alone decides what the hardware shows.
- **UI never talks to HID.** The companion mirrors daemon-resolved focus and
  writes config only.

## Client roles

`hello` accepts an optional `role` (`adapter` default, or `ui`):

```json
{"type":"hello","adapter":"reference-echo","protocol_version":0,"adapter_version":"1.2.0","capabilities":{"lifecycle_observation":true}}
{"type":"hello","adapter":"microbridge-ui","protocol_version":0,"role":"ui"}
```

Management operations require a completed, protocol-compatible `ui` hello.
The mode-`0600` socket trusts processes running as the same logged-in user;
roles provide protocol separation, not isolation between hostile same-user
processes.

`capabilities` is an object whose canonical boolean keys all default to
`false`: `lifecycle_observation`, `approval_acceptance`,
`approval_rejection`, `interrupt`, `new_session`, `focus_open`, and
`reasoning_effort`. The action mapping is:

| Action | Required capability |
|---|---|
| `approve` | `approval_acceptance` |
| `reject` | `approval_rejection` |
| `interrupt` | `interrupt` |
| `new_session` | `new_session` |
| `open_focused_thread` | `focus_open` |
| `reasoning_effort_up`, `reasoning_effort_down` | `reasoning_effort` |

Focus navigation is daemon-local and does not require a host capability.

## Messages: adapter → daemon

### `hello` — required first message

```json
{"type":"hello","adapter":"example-host","protocol_version":0,"capabilities":{"lifecycle_observation":true,"interrupt":true}}
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

`action` also includes `reasoning_effort_up`, `reasoning_effort_down`,
`navigate_up`, `navigate_down`, `navigate_left`, `navigate_right`, and
`open_focused_thread`. The daemon sends a host command only when that adapter
advertised the corresponding capability. Unknown actions remain a logged no-op.

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
`sleep_minutes`, `frontmost_app`, `hardware_control_enabled`, adapter consent,
and key-assignment lists. Persisted at
`~/.microbridge/config.toml`.

### Adapter consent and pairing

```json
{"type":"set_adapter_enabled","adapter_id":"cursor","enabled":true}
{"type":"pair_adapter","adapter_id":"t3code","pairing_url":"https://…/#token=…"}
{"type":"forget_adapter","adapter_id":"t3code"}
```

Pairing tokens are exchanged immediately and are never written to config or
logs. `adapter_operation` returns a success/failure acknowledgment.

### Simulated Agent Key activation

```json
{"type":"activate_agent_key","index":2,"open":false}
```

`index` is zero-based. This follows the same focus route as a physical Agent
Key. With `open:true`, the daemon additionally requests `open_focused_thread`
only when the owning adapter advertises `focus_open`.

## Messages: daemon → UI

### `snapshot`

Full bus view: sessions, focused session, six Agent Key assignments, device
connection, config, live adapter states/capabilities, and
`agent_key_led_frame`. The frame is the exact device-layer input after palette
normalization and includes each key's session, state, `#RRGGBB` color, focus,
plus frame brightness and pause state. An omitted legacy frame defaults empty
so newer UIs can derive a compatibility view from the assignments.

### `event`

Incremental `BusEvent`: `session_upserted`, `session_removed`,
`focus_changed`, `agent_keys_changed`, `device_changed`, `config_changed`,
`adapters_changed`.

`agent_keys_changed` carries both `session_ids` and the resolved `led_frame` so
the software twin and physical device cannot drift between full snapshots.

`adapters_changed` carries the complete live adapter-card list, not an
invalidation token:

```json
{"kind":"adapters_changed","adapters":[{"id":"cursor","display_name":"Cursor","kind":"community","state":"limited","capabilities":{"lifecycle_observation":true},"diagnostic":"Lifecycle connected; IDE controls remain unavailable."}]}
```

Clients replace their current adapter list with this payload. Missing
capability keys decode as `false`.

### `config`

Response to `get_config` / ack of `set_config`.
If persistence fails, the daemon leaves runtime state unchanged and replies
with `{"type":"config_error","message":"…"}`.

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
