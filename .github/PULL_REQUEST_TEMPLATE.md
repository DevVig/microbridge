## What

<!-- One or two sentences. Small PRs merge fast here. -->

## Why

## How was this tested?

<!-- e.g. "cargo test + ran the reference adapter against the daemon" -->

## Checklist

- [ ] `cargo fmt` / `clippy -D warnings` / `cargo test` pass locally
- [ ] Protocol changes update both `mb-protocol` and `docs/protocol.md`
- [ ] Adapter PRs: the [adapter checklist](../docs/adapters.md#the-adapter-checklist-reviewed-against-every-adapter-pr) is satisfied
- [ ] Nothing added to the daemon's idle footprint (CPU, RSS, network)
