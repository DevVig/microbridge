# Installing Microbridge

Microbridge is a local daemon plus an optional companion UI. There is **no
network** and **no cloud account** — install puts binaries on your machine and
(on macOS) a per-user launchd agent.

## Requirements

| Piece | Need |
|---|---|
| Daemon | Rust stable (`rustup`), macOS 13+ or Linux |
| Companion UI (optional) | Node ≥ 20; full `.app` also needs Xcode CLT |
| Hardware LEDs | Codex Micro over USB (HID packing still landing — mock works without hardware) |

## Quick install (macOS, from source)

```sh
git clone https://github.com/DevVig/microbridge.git
cd microbridge
./scripts/install.sh
```

This will:

1. `cargo build --release` for `microbridged` and `microbridgectl`
2. Install them to `~/.local/bin` (override with `MICROBRIDGE_BIN=…`)
3. Ensure `~/.local/bin` is on your PATH (prints a hint if not)
4. Install and start the launchd agent `ai.microbridge.daemon`
5. Write config defaults under `~/.microbridge/`

Verify:

```sh
microbridgectl status
# or:
tail -f ~/.microbridge/daemon.log
```

### Optional companion UI

```sh
./scripts/install.sh --with-ui
# web preview during development:
cd apps/microbridge-ui && npm install && npm run dev
```

`--with-ui` installs frontend deps and, when Tauri/Xcode tooling is available,
attempts `npm run tauri build`. You can always run the Vite UI against a live
daemon without bundling an `.app`.

## Linux (from source)

```sh
./scripts/install.sh --no-launchd
# run in the foreground, or add your own systemd --user unit:
microbridged
```

A sample user unit is in [`scripts/microbridge.service`](scripts/microbridge.service).

## Homebrew (skeleton)

```sh
brew install --build-from-source ./Formula/microbridge.rb
brew services start microbridge   # when using the formula's service block
```

A published tap/bottle is not available yet — use `./scripts/install.sh` for
day-to-day installs.

## Install from a GitHub Release

When a `v*` tag is pushed, CI attaches platform archives. Then:

```sh
./scripts/install-from-release.sh v0.0.1
# or latest:
./scripts/install-from-release.sh
```

## Uninstall

```sh
./scripts/uninstall.sh
```

Removes the launchd agent, binaries from `MICROBRIDGE_BIN` / `~/.local/bin`,
and optionally (`--purge`) `~/.microbridge/` (config, socket, logs).

## Layout after install

| Path | Purpose |
|---|---|
| `~/.local/bin/microbridged` | Daemon |
| `~/.local/bin/microbridgectl` | CLI |
| `~/Library/LaunchAgents/ai.microbridge.daemon.plist` | macOS autostart |
| `~/.microbridge/microbridged.sock` | Local NDJSON socket |
| `~/.microbridge/config.toml` | Key source, lighting, appearance |
| `~/.microbridge/daemon.log` | launchd stdout/stderr |

## Troubleshooting

**`microbridgectl: connect …`** — daemon not running. On macOS:
`launchctl kickstart -k gui/$(id -u)/ai.microbridge.daemon`.

**LEDs stay dark** — HID packing is still best-effort; ChatGPT desktop may
also own the device. Pause that app or use Settings → Pause LEDs while testing
the mock path (`microbridgectl status` still works).

**PATH** — add `export PATH="$HOME/.local/bin:$PATH"` to your shell rc if
`microbridgectl` is not found.
