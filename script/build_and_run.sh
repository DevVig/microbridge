#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UI_DIR="$ROOT_DIR/apps/microbridge-ui"
TAURI_DIR="$UI_DIR/src-tauri"
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
APP_BUNDLE="$TAURI_DIR/target/debug/bundle/macos/Microbridge.app"
APP_BINARY="$APP_BUNDLE/Contents/MacOS/microbridge-ui"

pkill -x microbridge-ui >/dev/null 2>&1 || true

if [[ ! -d "$UI_DIR/node_modules" ]]; then
  (cd "$UI_DIR" && npm ci)
fi
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" \
  -p microbridged -p microbridgectl --target "$TARGET"
mkdir -p "$TAURI_DIR/binaries"
cp "$ROOT_DIR/target/$TARGET/debug/microbridged" \
  "$TAURI_DIR/binaries/microbridged-$TARGET"
cp "$ROOT_DIR/target/$TARGET/debug/microbridgectl" \
  "$TAURI_DIR/binaries/microbridgectl-$TARGET"
(cd "$UI_DIR" && npm run tauri -- build --debug --bundles app \
  --config '{"bundle":{"createUpdaterArtifacts":false}}')

open_app() {
  /usr/bin/open -n "$APP_BUNDLE"
}

case "$MODE" in
  run)
    open_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    open_app
    /usr/bin/log stream --info --style compact --predicate 'process == "microbridge-ui"'
    ;;
  --telemetry|telemetry)
    open_app
    /usr/bin/log stream --info --style compact --predicate 'process == "microbridge-ui" OR process == "microbridged"'
    ;;
  --verify|verify)
    open_app
    for _ in {1..20}; do
      if pgrep -x microbridge-ui >/dev/null; then
        exit 0
      fi
      sleep 0.5
    done
    echo "Microbridge did not remain running after launch" >&2
    exit 1
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
