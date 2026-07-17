# Installing Microbridge

Microbridge is a local daemon plus an optional companion UI. There is **no
cloud account** — install puts binaries on your machine and runs a user-level
service.

## Recommended on macOS: Homebrew (with updates)

This is the easy path. You do **not** need to clone the repo. Homebrew owns
install, upgrades, and the launchd service.

```sh
brew tap DevVig/microbridge https://github.com/DevVig/microbridge
brew install microbridge
brew services start microbridge
microbridgectl status
```

### Updates

```sh
brew update && brew upgrade microbridge
brew services restart microbridge
```

Optional **background** upgrades (Homebrew’s autoupdate):

```sh
brew autoupdate start --upgrade --cleanup --immediate
# later: brew autoupdate status / brew autoupdate stop
```

Private tap note: if the GitHub repo is private, authenticate once
(`gh auth login` or a `HOMEBREW_GITHUB_API_TOKEN`) so `brew` can fetch the
tarball.

Uninstall:

```sh
brew services stop microbridge
brew uninstall microbridge
# optional: brew untap DevVig/microbridge
```

Governance / why this path: [docs/governance.md](docs/governance.md).

---

## Requirements

| Piece | Need |
|---|---|
| Daemon (Homebrew) | Homebrew; Rust pulled in as a build dependency |
| Daemon (from source) | Rust stable (`rustup`), macOS 13+ or Linux |
| Companion UI (optional) | Node ≥ 20; full `.app` also needs Xcode CLT |
| Hardware LEDs | Codex Micro over USB (HID packing still landing — mock works without hardware) |

## From source (developers)

```sh
git clone https://github.com/DevVig/microbridge.git
cd microbridge
./scripts/install.sh              # macOS: binaries + launchd
# ./scripts/install.sh --with-ui
# ./scripts/install-linux-systemd.sh
microbridgectl status
```

Uninstall: `./scripts/uninstall.sh` (add `--purge` to remove `~/.microbridge`).

### Optional companion UI

```sh
./scripts/install.sh --with-ui
# or during development:
cd apps/microbridge-ui && npm install && npm run dev
```

## Linux

```sh
./scripts/install-linux-systemd.sh
# or:
./scripts/install.sh --no-launchd && microbridged
```

Sample unit: [`scripts/microbridge.service`](scripts/microbridge.service).

## Install from a GitHub Release (binaries)

When a `v*` tag is published, CI attaches platform archives:

```sh
./scripts/install-from-release.sh          # latest
./scripts/install-from-release.sh v0.0.1
```

## Layout after install

| Path | Purpose |
|---|---|
| `$(brew --prefix)/bin/microbridged` | Daemon (Homebrew) |
| `~/.local/bin/microbridged` | Daemon (source install script) |
| `~/.microbridge/microbridged.sock` | Local NDJSON socket |
| `~/.microbridge/config.toml` | Key source, lighting, appearance |
| `~/.microbridge/daemon.log` | launchd / service logs |

## Troubleshooting

**`microbridgectl: connect …`** — daemon not running.

```sh
brew services restart microbridge
# or:
launchctl kickstart -k "gui/$(id -u)/ai.microbridge.daemon"
```

**LEDs stay dark** — HID packing is still best-effort; ChatGPT desktop may
also own the device.

**Homebrew can’t fetch (private repo)** — `gh auth login`, or set
`HOMEBREW_GITHUB_API_TOKEN` to a PAT with `repo` scope.
