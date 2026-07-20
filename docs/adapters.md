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

1. **Official hooks/APIs** — e.g. Cursor/Factory hooks, Factory JSON-RPC, and T3 Code's
   authenticated paired HTTP GET/POST orchestration contract. Stable and
   supported.
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
entries it installs. Synara and Conductor reuse the built-in journal watchers.
