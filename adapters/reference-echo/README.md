# reference-echo

The smallest possible adapter: ~50 lines of dependency-free Node that
registers one fake session and walks it through every state at 1.5s
intervals, then says `bye`. Use it to verify a daemon build end-to-end or as
the starting point for a real adapter.

```sh
cargo run -p microbridged   # shell 1
node index.mjs              # shell 2 — watch the daemon log render frames
```

Note: the state walk is time-driven because it *simulates* transitions. Real
adapters must be event-driven — see the checklist in
[docs/adapters.md](../../docs/adapters.md).
