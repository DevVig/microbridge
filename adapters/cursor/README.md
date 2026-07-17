# cursor adapter (community)

Out-of-process Microbridge adapter for [Cursor](https://cursor.com/).

## Status

**Scaffold.** Cursor does not publish a stable local session journal API.
This adapter connects and stays idle until a supported state source is
documented. PRs welcome — see the checklist in
[docs/adapters.md](../../docs/adapters.md).

## Rules

- Event-driven only (no polling loops)
- No scraping of Cursor's private Electron internals
- Prefer official hooks / documented session files when available

## Run (once implemented)

```sh
cargo run -p microbridged          # shell 1
node adapters/cursor/index.mjs     # shell 2
```

## Supported versions

TBD — document the Cursor build you tested against before merging a real
implementation.
