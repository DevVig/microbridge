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

End-user install paths are documented in [INSTALL.md](INSTALL.md)
(`./scripts/install.sh`, uninstall, releases).

## Before you push

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
# menu bar app:
cd apps/microbridge-ui && npm ci && npm run build
```

Or `make ci`. CI enforces Rust checks on macOS/Linux and the UI build on Ubuntu.

## Releases

Push a version tag to publish binaries via GitHub Actions:

```sh
git tag v0.0.1
git push origin v0.0.1
```

Assets are attached to the GitHub Release; users can run
`./scripts/install-from-release.sh v0.0.1`.

## Commits and PRs

`main` is protected — **no direct pushes**. Open a PR; squash-merge only.

- PR titles must be Conventional Commits (`feat:`, `fix:`, `docs:`,
  `adapter:`, …) — enforced by CI (`PR title` workflow).
- Required checks: `rust (ubuntu-latest)`, `rust (macos-latest)`, `ui`.
- Resolve review threads before merge.
- One logical change per PR; small PRs merge fast here.
- PRs must say how they were tested — "ran the reference adapter against the
  daemon and watched the frames" is a fine answer at this stage.

See [docs/governance.md](docs/governance.md).

## Reporting security issues

Privately, please — see [SECURITY.md](SECURITY.md).
