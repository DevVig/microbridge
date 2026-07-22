# Installing Microbridge

Microbridge installs a **menu bar app** (primary UI for the keyboard) plus a
local daemon that drives the Micro. There is **no cloud account** — everything
runs on your machine.

## Recommended on macOS: Homebrew (with updates)

This is the easy path. You do **not** need to clone the repo. Homebrew installs
the **menu bar app** (primary UI), its bundled daemon, and the CLI. The explicit
app helper preserves the signed bundle and avoids a separate background item.

```sh
brew tap DevVig/microbridge https://github.com/DevVig/microbridge
brew install microbridge
microbridge-app install
microbridgectl status
```

### Updates

```sh
brew update && brew upgrade microbridge
microbridge-app install
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
microbridge-app uninstall
brew uninstall microbridge
# optional: brew untap DevVig/microbridge
```

Advanced headless mode: `brew services start microbridge` runs the standalone
daemon without the menu-bar app and intentionally creates a separate background
item. Stop it with `brew services stop microbridge` before returning to the
standard app-owned lifecycle.

Governance / why this path: [docs/governance.md](docs/governance.md).

---

## Requirements

| Piece | Need |
|---|---|
| macOS (Homebrew) | Homebrew + **Xcode Command Line Tools** (`xcode-select --install`); Rust + Node pulled in as **build** deps (builds `.app` + daemon) |
| From source | Rust stable, Node ≥ 20; macOS also needs Xcode CLT for the `.app` |
| Hardware LEDs/keys | Codex Micro over USB; claim it from the popover, the menu-bar icon’s right-click menu, or **Settings → Device** (`MICROBRIDGE_HID_CLAIM=1` remains a developer override) |

## From source (developers)

```sh
git clone https://github.com/DevVig/microbridge.git
cd microbridge
./scripts/install.sh                 # macOS: menu bar app + app-owned daemon
# ./scripts/install.sh --no-ui       # daemon/CLI only (headless)
# ./scripts/install-linux-systemd.sh
```

Uninstall: `./scripts/uninstall.sh` (add `--purge` to remove `~/.microbridge`).

During UI development: `cd apps/microbridge-ui && npm install && npm run tauri dev`.

## Linux

```sh
./scripts/install-linux-systemd.sh
# or:
./scripts/install.sh --no-launchd && microbridged
```

Sample unit: [`scripts/microbridge.service`](scripts/microbridge.service).

## Install from a GitHub Release (binaries)

When a `v*` tag is published, CI attaches platform archives (daemon +
arch-specific menu bar app). On macOS, releases also include a
**Developer ID–signed and notarized** DMG
(`microbridge-ui-<tag>-<arch>.dmg`).

```sh
./scripts/install-from-release.sh          # latest (prefers DMG on macOS)
./scripts/install-from-release.sh v0.1.0
```

Or open the DMG from the GitHub Release page and drag Microbridge into
Applications. The direct-download app includes and starts its own local daemon;
there is no second install step. If a Homebrew/launchd daemon is already
running, the app uses that service instead of starting another copy.

**In-app updates (direct installs).** A DMG/manual install updates itself:
right-click the menu bar icon → **Check for Updates…**, or turn on *Settings →
Updates → check automatically at launch* (off by default). The app downloads
the signed update, verifies it, and relaunches. Update checks are the only
app-originated network call. The daemon also contacts a T3 Code environment
only after you explicitly enable that integration and exchange a one-time pairing
link; Microbridge has no telemetry or cloud relay.

Homebrew installs are managed by brew instead: the app detects the brew marker
and points you at `brew update && brew upgrade microbridge && microbridge-app
install` rather than self-replacing, so the formula version and the stable app
copy never drift apart.

### Cursor integration

Cursor support ships inside Microbridge. Open **Settings → Integrations** and click
**Enable Cursor**; Microbridge installs its bundled lifecycle integration into
Cursor's supported local-plugin directory after that explicit consent. Reload
Cursor once if it is already open. **Remove** disables the adapter and removes
only Microbridge's local integration. No Marketplace download is required.

### Factory integration

Factory support ships inside Microbridge. Open **Settings → Integrations** and
click **Enable Factory**. Microbridge copies its signed `microbridgectl` helper
to `~/.microbridge/integrations/factory/` and merges only its own entries into
Factory's supported `~/.factory/hooks.json`; existing hooks are preserved.
**Remove** deletes the Microbridge-owned hook entries and helper. Droid must be
installed and signed in for interrupt and reasoning-effort controls.

### CNVS integration

CNVS support ships inside the daemon and is enabled by default. Start CNVS and
Microbridge connects automatically to CNVS's authenticated loopback control
API. Every running agent terminal is identified by its exact canvas and node,
so an Agent Key can focus the correct workspace and terminal or interrupt that
specific agent. There is no pairing code, plugin installation, or CNVS file
modification.

CNVS currently exposes lifecycle, focus, and interrupt controls through this
contract. Microbridge leaves approval, new-session, and reasoning-effort
controls disabled until CNVS exposes stable targets for them.

Synara and Conductor do not need an installer: their Codex/Claude sessions are
named by the built-in journal watchers. ChatGPT, Claude Desktop, Codex CLI, and
Claude Code are distinguished automatically by those same watchers. T3 Code
controls require the one-time pairing flow shown in **Settings → Integrations**.

### OpenCode integration

OpenCode support ships inside Microbridge. Open **Settings → Integrations** and
click **Enable OpenCode**. Microbridge installs its dependency-free global plugin
at `~/.config/opencode/plugins/microbridge.mjs`. Restart OpenCode if it is already
running. The plugin publishes local lifecycle state and routes Interrupt to the
exact OpenCode session; it does not read or send prompts, transcripts, source
code, or tool arguments. **Remove** deletes only the Microbridge-owned file.

**Note:** Homebrew installs **prebuilt** release binaries (not a from-source
Tauri build). The formula checksums are refreshed by CI after each `v*` tag.

## Layout after install

| Path | Purpose |
|---|---|
| `~/Applications/Microbridge.app` | Menu bar app (primary UI) |
| `Microbridge.app/Contents/MacOS/microbridged` | Bundled daemon for direct installs |
| `$(brew --prefix)/bin/microbridged` | Daemon (Homebrew) |
| `~/.local/bin/microbridged` | Daemon (source / release install) |
| `~/.microbridge/microbridged.sock` | Local NDJSON socket |
| `~/.microbridge/config.toml` | Key source, lighting, appearance |
| `~/.microbridge/microbridged-app.log` | Standard app-owned daemon log |
| `~/.microbridge/daemon.log` | Headless launchd / service log |
| `~/.cursor/plugins/local/microbridge` | Bundled Cursor lifecycle integration (only after consent) |
| `~/.factory/hooks.json` | Existing Factory hooks plus Microbridge-owned lifecycle entries (only after consent) |
| `~/.microbridge/integrations/factory/microbridgectl` | Signed Factory hook helper (only after consent) |
| `~/.config/opencode/plugins/microbridge.mjs` | Bundled OpenCode lifecycle and interrupt integration (only after consent) |
| macOS Login Items | Branded Microbridge main-app registration (only if enabled in Settings → General) |

## Launch at login

The menu bar app asks once, on first launch, whether to start automatically at
login, and registers the signed main app with macOS ServiceManagement if you say
yes. Toggle it any time in **Settings → General**; if macOS requires approval,
the same surface opens Login Items directly. The standard GUI path shows the
Microbridge name and icon rather than a Unix executable.

## Troubleshooting

**`microbridgectl: connect …`** — daemon not running. Standard GUI installs
start the bundled daemon with the app; relaunch Microbridge first. For explicit
headless operation:

```sh
brew services restart microbridge
```

**LEDs stay dark** — by default Microbridge only probes USB (Detected). Choose
**Claim Codex Micro** in the popover or right-click menu. If the interface is
busy, pause the other device owner and choose **Retry**. The advanced control
also remains in **Settings → Device**. Developers can still set
`MICROBRIDGE_HID_CLAIM=1` before starting the daemon. See
[docs/device-hid.md](docs/device-hid.md).

**Homebrew can’t fetch (private repo)** — `gh auth login`, or set
`HOMEBREW_GITHUB_API_TOKEN` to a PAT with `repo` scope.
