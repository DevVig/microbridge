#!/usr/bin/env bash
# Install daemon binaries and a systemd --user unit on Linux.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${MICROBRIDGE_BIN:-$HOME/.local/bin}"

"$ROOT/scripts/install.sh" --no-launchd --bin-dir "$BIN_DIR"

UNIT_DIR="$HOME/.config/systemd/user"
mkdir -p "$UNIT_DIR"
sed "s|%h/.local/bin|${BIN_DIR}|g" "$ROOT/scripts/microbridge.service" >"$UNIT_DIR/microbridge.service"
systemctl --user daemon-reload
systemctl --user enable --now microbridge.service
echo "systemd --user unit microbridge.service started"
echo "  logs: journalctl --user -u microbridge -f"
