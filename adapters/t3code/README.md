# t3code adapter (community)

Out-of-process Microbridge adapter for [T3 Code](https://github.com/pingdotgg/t3code).

## Status

**Scaffold.** Wire this to T3 Code's local session / agent status surface when
one is available. Upstream contributions to T3 Code itself are currently
closed; this adapter can still ship in Microbridge independently.

## Rules

- Event-driven only (no polling loops)
- No scraping of private Electron internals
- Prefer official hooks / documented session files

## Run (once implemented)

```sh
cargo run -p microbridged           # shell 1
node adapters/t3code/index.mjs      # shell 2
```

## Supported versions

TBD — document the T3 Code build you tested against before merging a real
implementation.
