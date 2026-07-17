# Contributing

Contributions are open, and **adapter PRs are the ones we want most** — no
prior discussion needed if you follow [docs/adapters.md](docs/adapters.md).
For daemon/protocol changes, open an issue first so we can agree on the
shape before you spend time.

## Ground rules

- **The footprint budget is law.** Anything that adds idle CPU, resident
  memory, or network I/O to the daemon will be declined regardless of the
  feature. See [docs/architecture.md](docs/architecture.md#footprint-budget).
- **Protocol changes** must update `crates/mb-protocol` and
  `docs/protocol.md` in the same PR, with a version bump if breaking.
- **No CLA.** Contributions are accepted under the project's dual
  MIT/Apache-2.0 license (inbound = outbound).

## Dev setup

```sh
rustup show          # toolchain comes from rust-toolchain.toml
cargo test           # all crates
cargo run -p microbridged
node adapters/reference-echo/index.mjs   # exercise the daemon end-to-end
```

## Before you push

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI enforces all three on macOS and Linux.

## Commits and PRs

- Conventional commits (`feat:`, `fix:`, `docs:`, `adapter:` for adapter
  work).
- One logical change per PR; small PRs merge fast here.
- PRs must say how they were tested — "ran the reference adapter against the
  daemon and watched the frames" is a fine answer at this stage.

## Reporting security issues

Privately, please — see [SECURITY.md](SECURITY.md).
