# Writing an adapter

An adapter watches one agent runtime (an IDE, a CLI, a desktop app) and
publishes its sessions to the daemon. This is the contribution the project
most wants — a good adapter PR needs no prior discussion.

## The contract

1. Connect to the Unix socket (`$MICROBRIDGE_SOCKET`, default
   `~/.microbridge/microbridged.sock`).
2. Send `hello` with your adapter name and protocol version.
3. Send `status` on every session state *transition* — never on a timer unless
   the host's supported contract only provides snapshots.
4. Send `bye` when a session ends; reconnect and republish after crashes.
5. Advertise exact capabilities in `hello`. Handle only supported incoming
   actions; unknown actions are a logged no-op.

Wire format: [protocol.md](protocol.md). Working example:
[`adapters/reference-echo`](../adapters/reference-echo/index.mjs) (~50 lines
of dependency-free Node).

## Where state comes from

Prefer, in order:

1. **Official hooks/APIs** — e.g. CNVS's authenticated loopback control API,
   Cursor/Factory hooks, Factory JSON-RPC, and T3 Code's authenticated paired
   HTTP GET/POST orchestration contract. Stable and supported.
2. **Session files** — many runtimes journal to disk (e.g.
   `~/.codex/sessions`). Watch with FSEvents/inotify, not polling.
3. **Logs** — fragile; document exactly which version you tested.

Never scrape another app's private Electron internals — adapters that do
will not be merged.

## The adapter checklist (reviewed against every adapter PR)

- [ ] Event-driven where the host supports it; bounded snapshot refreshes are
      documented and emit only changed sessions
- [ ] Idle cost ≈ 0 CPU; states from watching, not asking
- [ ] `status` sent only on transitions, with complete session records
- [ ] Sessions cleaned up (`bye`) on end; correct republish on reconnect
- [ ] No undeclared network I/O. Paired remote hosts require explicit consent,
      scoped credentials, timeouts, and revocation behavior
- [ ] `README.md` in your adapter folder: supported runtime versions, how
      state is sourced, known limitations
- [ ] Tested against the daemon with `cargo run -p microbridged`

## In-process vs out-of-process

First-party adapters (Codex CLI, Claude Code) are Rust modules compiled into
the daemon to keep the resident footprint at one process. Community adapters
live in `adapters/<name>/` in any language and run as their own process. A
community adapter that proves stable and broadly used can graduate to
in-process. T3 Code is daemon-owned because its paired credential and action
routing must share Microbridge's consent boundary. Cursor and Factory remain
host-managed so each host owns hook execution and Microbridge owns only the
entries it installs. CNVS is daemon-owned because its canvas/node identity and
short-lived local token must remain inside the same routing boundary. Synara
and Conductor reuse the built-in journal watchers.

## Hosts vs adapters (Settings → Integrations)

**Adapters / session sources** publish or watch state (Claude Code, Codex CLI,
CNVS, Cursor, T3 Code, Factory, OpenCode). They may have Enable / Pair /
Disconnect actions.

**Host-attributed apps** (Synara, ChatGPT, Claude Desktop, Conductor) are not
separate pairable adapters. They share `~/.claude/projects` and
`~/.codex/sessions`; the built-in watchers label sessions by `entrypoint` /
`originator` / cwd. Settings still shows each as its own Integrations card with
a green / yellow / red status derived from live threads — do **not** open a PR
that adds a Synara (or ChatGPT) pairing adapter unless the host publishes a
distinct control API.

## IDE families and the controller lock

`mb_protocol::ide::IDES` is the single source of truth for IDE identity: it maps
each family key (`t3`) to its canonical session label (`"T3 Code"`), its aliases,
and the adapter ids that can feed it (`t3code`, `codex`, `claude`). That last
field is a list because hosts and harnesses are many-to-many — one IDE, several
possible session sources.

Two tests keep the table honest and will fail your PR if you drift: every
`family` must round-trip through `app_match::app_family(label)`, and every
`providers` entry must exist in the daemon's adapter registry. If you add an
adapter for a new IDE, add its family here too — the tray's `Controlled by`
submenu is generated from this table, so an IDE missing from it cannot be pinned.

## `navigation` capability and per-IDE profiles

`Action::NavigateUp` / `NavigateDown` / `NavigateLeft` / `NavigateRight` are
gated on the `navigation` capability. It defaults to `false`, and an adapter that
does not advertise it gets an honest "does not support" error rather than having
the action accepted and dropped. Advertise it only when your host exposes a
navigation surface you actually drive.

`IdeProfile` (also in `mb_protocol::ide`) binds the dial and joystick per IDE:

| Field | Values | Default |
|---|---|---|
| `dial` | `Effort` · `Navigate` | `Effort` |
| `joystick` | `DeckCycle` · `Navigate` | `DeckCycle` |
| `dial_press` | any `Action` | `OpenFocusedThread` |

The profile is chosen from the **focused session's** family, so it applies in
Automatic mode too; pinning a controller is what makes *which* profile you get
predictable. There is deliberately no "off" role — whether a host can act on a
lever is already answered dynamically by `AdapterCapabilities`, and duplicating
that as a static per-IDE fact would only go stale.

Every IDE currently uses the default profile. Two notes on why:

- **Cursor** has no navigation surface Microbridge may drive: the `cursor`
  adapter is lifecycle-only hooks and `cursor_acp` explicitly does not
  remote-control an open composer. Synthesizing UI keystrokes is not on the
  table. Revisit when Cursor publishes one.
- **T3 Code** was checked against a live `0.0.29-nightly.20260725.899`
  environment. **It exposes no navigation or selection command.** The tempting
  `thread.jump.1` … `thread.jump.9` are `THREAD_JUMP_KEYBINDING_COMMANDS` —
  local UI keybindings, not part of the dispatch contract — so binding them
  would be exactly the keystroke synthesis this adapter refuses. See below for
  what T3 *does* expose.

### Re-checking the T3 contract

The environment descriptor needs no auth, so the version and feature flags are
one command away:

```bash
BASE=http://127.0.0.1:3774     # note: 3774, not the 3773 in the unit-test fixture
curl -s "$BASE/.well-known/t3/environment" | jq .
```

The full dispatch command union can be recovered from the app bundle without
pairing, which is how the list below was produced:

```bash
ASAR="/Applications/T3 Code.app/Contents/Resources/app.asar"   # or T3 Code (Nightly).app
LC_ALL=C strings -a "$ASAR" | grep -oE 'Schema\.Literal\("(thread|project)\.[a-z.-]+"\)' | sort -u
```

Compare the thread shape against `ThreadShell` in
`crates/microbridged/src/t3code.rs`. If `serverVersion` moves past the pinned
`SUPPORTED_SERVER_VERSIONS`, bump those and `PINNED_CONTRACT_COMMIT`; the version
gate fails closed to `Incompatible`, so an unbumped daemon is safe, just inert.

**Dispatchable and already wired**: `thread.turn.interrupt`,
`thread.approval.respond` (decision is one of `accept` · `acceptForSession` ·
`decline` · `cancel` — Microbridge uses `accept` / `decline`).

**Dispatchable, advertised, not yet bound to a key** — the thread-lifecycle
surface, gated by the descriptor's `threadSettlement` / `threadSnooze` flags,
which the adapter now reads and reports in its diagnostic:

| Command | Payload beyond `commandId` + `threadId` |
|---|---|
| `thread.settle` | — |
| `thread.unsettle` | `reason: "user"` |
| `thread.snooze` | `snoozedUntil: IsoDateTime` |
| `thread.unsnooze` | `reason: "user"` |
| `thread.archive` / `thread.unarchive` | — |

**Deliberately still unadvertised**, with the reasons:

- `reasoning_effort` stays `false`. T3 has **no reasoning-effort concept** —
  `reasoningEffort` appears nowhere in the bundle, and `ModelSelection.options`
  is still `Schema.Unknown`, which is the "provider option descriptors" the
  capability comment waits on. The two near-misses are different things:
  `RuntimeMode` is an autonomy ladder (`approval-required` ·
  `auto-accept-edits` · `auto` · `full-access`, set via
  `thread.runtime-mode.set`) and `ProviderInteractionMode` is `default` · `plan`
  (via `thread.interaction-mode.set`). Mapping effort onto either would be a
  semantic lie; both would want their own `Action` if we bind them.
- `new_session` stays `false`. `thread.turn.start` needs an existing `threadId`,
  or a `bootstrap.createThread` requiring `projectId`, `title` and a
  `modelSelection` whose `model` is `Unknown`. Microbridge has no project or
  model registry, so this needs a projects endpoint and a live paired
  environment to verify against before it can be advertised honestly.
