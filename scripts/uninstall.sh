#!/usr/bin/env bash
# Remove Microbridge binaries, launchd agent, and optionally config.
set -euo pipefail

BIN_DIR="${MICROBRIDGE_BIN:-$HOME/.local/bin}"
LABEL="ai.microbridge.daemon"
UI_LABEL="ai.microbridge.ui"
BREW_LABEL="homebrew.mxcl.microbridge"
PURGE=0

usage() {
  cat <<EOF
Usage: ./scripts/uninstall.sh [--purge]

  --purge   Also delete ~/.microbridge/ (config, socket, logs)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --purge) PURGE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ "$(uname -s)" == "Darwin" ]]; then
  echo "==> Stopping launchd agents"
  launchctl bootout "gui/$(id -u)/${LABEL}" 2>/dev/null || true
  launchctl bootout "gui/$(id -u)/${UI_LABEL}" 2>/dev/null || true
  launchctl bootout "gui/$(id -u)/${BREW_LABEL}" 2>/dev/null || true
  rm -f "$HOME/Library/LaunchAgents/${LABEL}.plist"
  rm -f "$HOME/Library/LaunchAgents/${UI_LABEL}.plist"
  rm -f "$HOME/Library/LaunchAgents/${BREW_LABEL}.plist"
  APP="$HOME/Applications/Microbridge.app"
  SOURCE_MARKER="$HOME/Applications/.Microbridge.app.microbridge-source"
  BREW_MARKER="$HOME/Applications/.Microbridge.app.microbridge-brew"
  RELEASE_MARKER="$HOME/Applications/.Microbridge.app.microbridge-release"
  if [[ -d "$APP" && ( -f "$SOURCE_MARKER" || -f "$BREW_MARKER" || -f "$RELEASE_MARKER" || -f "$APP/.microbridge-brew" || -f "$APP/.microbridge-release" ) ]]; then
    echo "==> Removing menu bar app"
    "$APP/Contents/MacOS/microbridge-ui" \
      --unregister-login-item 2>/dev/null || true
    while read -r pid; do
      kill "$pid" 2>/dev/null || true
    done < <(/usr/bin/pgrep -f "^${APP}/Contents/MacOS/microbridge-ui$" 2>/dev/null || true)
    for _ in 1 2 3 4 5 6 7 8 9 10; do
      /usr/bin/pgrep -f "^${APP}/Contents/MacOS/microbridge-ui$" >/dev/null 2>&1 || break
      /bin/sleep 0.1
    done
    rm -rf "$APP"
    rm -f "$SOURCE_MARKER" "$BREW_MARKER" "$RELEASE_MARKER"
  elif [[ -d "$APP" ]]; then
    echo "==> Preserving unowned $APP"
  fi
fi

if [[ -f "$HOME/.config/systemd/user/microbridge.service" ]]; then
  echo "==> Stopping systemd --user unit"
  systemctl --user disable --now microbridge.service 2>/dev/null || true
  rm -f "$HOME/.config/systemd/user/microbridge.service"
  systemctl --user daemon-reload 2>/dev/null || true
fi

echo "==> Removing binaries from $BIN_DIR"
rm -f "$BIN_DIR/microbridged" "$BIN_DIR/microbridgectl"

if [[ "$PURGE" -eq 1 ]]; then
  echo "==> Purging ~/.microbridge"
  rm -rf "$HOME/.microbridge"
else
  echo "    kept ~/.microbridge (pass --purge to delete config/logs)"
fi

echo "Microbridge uninstalled."
