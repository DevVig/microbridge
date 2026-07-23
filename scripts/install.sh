#!/usr/bin/env bash
# Unified Microbridge installer — daemon + menu bar app (default).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${MICROBRIDGE_BIN:-$HOME/.local/bin}"
WITH_UI=1
WITH_LAUNCHD=1
LABEL="ai.microbridge.daemon"

usage() {
  cat <<EOF
Usage: ./scripts/install.sh [options]

  --no-ui         Skip the menu bar companion (daemon/CLI only)
  --no-launchd    Skip macOS launchd agents
  --bin-dir DIR   Install binaries here (default: ~/.local/bin)
  -h, --help      Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-ui) WITH_UI=0; shift ;;
    --with-ui) WITH_UI=1; shift ;; # back-compat; UI is already default
    --no-launchd) WITH_LAUNCHD=0; shift ;;
    --bin-dir) BIN_DIR="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  WITH_LAUNCHD=0
fi

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: '$1' is required but not on PATH" >&2
    exit 1
  }
}

need cargo
need rustc

echo "==> Building release binaries"
(
  cd "$ROOT"
  cargo build --release -p microbridged -p microbridgectl
)

mkdir -p "$BIN_DIR" "$HOME/.microbridge"
install -m 755 "$ROOT/target/release/microbridged" "$BIN_DIR/microbridged"
install -m 755 "$ROOT/target/release/microbridgectl" "$BIN_DIR/microbridgectl"
echo "    installed $BIN_DIR/microbridged"
echo "    installed $BIN_DIR/microbridgectl"

if ! command -v microbridgectl >/dev/null 2>&1; then
  if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo ""
    echo "Note: add this to your shell rc so the tools are on PATH:"
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
  fi
fi

if [[ ! -f "$HOME/.microbridge/config.toml" ]]; then
  cat >"$HOME/.microbridge/config.toml" <<'TOML'
# Microbridge daemon config — see docs/protocol.md
key_source = "most_recent"
approvals_interrupt = true
pause_leds = false
appearance = "system"
lighting_preset = "codex"
brightness = 80
sleep_minutes = 3
TOML
  echo "    wrote ~/.microbridge/config.toml"
fi

if [[ "$WITH_LAUNCHD" -eq 1 && "$WITH_UI" -eq 0 ]]; then
  echo "==> Installing launchd agent ($LABEL)"
  PLIST="$HOME/Library/LaunchAgents/${LABEL}.plist"
  mkdir -p "$HOME/Library/LaunchAgents"
  cat >"$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${BIN_DIR}/microbridged</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>${HOME}/.microbridge/daemon.log</string>
  <key>StandardErrorPath</key>
  <string>${HOME}/.microbridge/daemon.log</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>HOME</key>
    <string>${HOME}</string>
    <key>PATH</key>
    <string>${BIN_DIR}:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin</string>
    <key>RUST_LOG</key>
    <string>info</string>
  </dict>
</dict>
</plist>
EOF
  launchctl bootout "gui/$(id -u)/${LABEL}" 2>/dev/null || true
  launchctl bootstrap "gui/$(id -u)" "$PLIST"
  launchctl enable "gui/$(id -u)/${LABEL}"
  launchctl kickstart -k "gui/$(id -u)/${LABEL}"
  echo "    launchd agent running"
  sleep 0.5
  if "$BIN_DIR/microbridgectl" status >/dev/null 2>&1; then
    echo "    microbridgectl status: ok"
  else
    echo "    warning: daemon not responding yet — check ~/.microbridge/daemon.log"
  fi
else
  if [[ "$WITH_UI" -eq 1 ]]; then
    echo "==> App-owned daemon (no separate launchd background item)"
  else
    echo "==> Skipping launchd (run manually: $BIN_DIR/microbridged)"
  fi
fi

if [[ "$WITH_UI" -eq 1 ]]; then
  echo "==> Menu bar app (primary UI)"
  need npm
  (
    cd "$ROOT/apps/microbridge-ui"
    npm ci
    npm run build
    if npm run tauri build; then
      APP_SRC="$(find "$ROOT/apps/microbridge-ui/src-tauri/target/release/bundle" -name 'Microbridge.app' -type d 2>/dev/null | head -n1 || true)"
      if [[ -n "$APP_SRC" && "$(uname -s)" == "Darwin" ]]; then
        APP_DEST="$HOME/Applications/Microbridge.app"
        APP_MARKER="$HOME/Applications/.Microbridge.app.microbridge-source"
        if [[ -e "$APP_DEST" && ! -f "$APP_MARKER" && "${MICROBRIDGE_FORCE_APP:-}" != "1" ]]; then
          echo "    warning: preserving unowned $APP_DEST"
          echo "    set MICROBRIDGE_FORCE_APP=1 to replace"
        else
          while read -r pid; do
            kill "$pid" 2>/dev/null || true
          done < <(/usr/bin/pgrep -f "^${APP_DEST}/Contents/MacOS/microbridge-ui$" 2>/dev/null || true)
          rm -rf "$APP_DEST"
          mkdir -p "$HOME/Applications"
          /usr/bin/ditto "$APP_SRC" "$APP_DEST"
          touch "$APP_MARKER"
          echo "    installed ~/Applications/Microbridge.app"
          # Launch at login and the bundled daemon are both owned by the app.
          open "$APP_DEST" 2>/dev/null || true
        fi
      else
        echo "    note: .app bundle not found — web build is in apps/microbridge-ui/dist"
        echo "    run: cd apps/microbridge-ui && npm run tauri dev"
      fi
    else
      echo "    note: Tauri bundle failed — web build is in apps/microbridge-ui/dist"
      echo "    run: cd apps/microbridge-ui && npm run tauri dev"
    fi
  )
fi

echo ""
echo "Microbridge installed."
echo "  UI:      ~/Applications/Microbridge.app (menu bar)"
echo "  status:  $BIN_DIR/microbridgectl status"
echo "  logs:    ~/.microbridge/daemon.log"
echo "  config:  ~/.microbridge/config.toml"
echo "  docs:    INSTALL.md"
echo "  remove:  ./scripts/uninstall.sh"
