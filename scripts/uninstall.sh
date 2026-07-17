#!/usr/bin/env bash
# Remove Microbridge binaries, launchd agent, and optionally config.
set -euo pipefail

BIN_DIR="${MICROBRIDGE_BIN:-$HOME/.local/bin}"
LABEL="ai.microbridge.daemon"
UI_LABEL="ai.microbridge.ui"
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
  rm -f "$HOME/Library/LaunchAgents/${LABEL}.plist"
  rm -f "$HOME/Library/LaunchAgents/${UI_LABEL}.plist"
  if [[ -d "$HOME/Applications/Microbridge.app" ]]; then
    echo "==> Removing menu bar app"
    rm -rf "$HOME/Applications/Microbridge.app"
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
