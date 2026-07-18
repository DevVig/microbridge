#!/usr/bin/env bash
# Install a candidate formula in an isolated tap and verify the complete macOS
# package lifecycle. The caller supplies a local formula, expected version, and
# expected machine architecture.
set -euo pipefail

FORMULA="${1:?usage: $0 path/to/microbridge.rb VERSION ARCH}"
VERSION="${2:?usage: $0 path/to/microbridge.rb VERSION ARCH}"
EXPECTED_ARCH="${3:?usage: $0 path/to/microbridge.rb VERSION ARCH}"
TAP="devvig/microbridge-ci"
APP="$HOME/Applications/Microbridge.app"

cleanup() {
  brew services stop "$TAP/microbridge" >/dev/null 2>&1 || true
  HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall "$TAP/microbridge" >/dev/null 2>&1 || true
  if [[ -f "$APP/.microbridge-brew" ]]; then
    rm -rf "$APP"
  fi
  brew untap "$TAP" >/dev/null 2>&1 || true
}
trap cleanup EXIT

test "$(uname -m)" = "$EXPECTED_ARCH"
brew tap-new "$TAP"
TAP_PATH="$(brew --repo "$TAP")"
mkdir -p "$TAP_PATH/Formula"
cp "$FORMULA" "$TAP_PATH/Formula/microbridge.rb"

HOMEBREW_NO_INSTALL_CLEANUP=1 brew install "$TAP/microbridge"
microbridgectl help | grep -q Usage
PREFIX="$(brew --prefix "$TAP/microbridge")"
test -x "$PREFIX/bin/microbridged"
test -x "$PREFIX/bin/microbridgectl"

brew services start "$TAP/microbridge"
for _ in {1..10}; do
  [[ -f "$APP/.microbridge-brew" ]] && break
  sleep 1
done
test -d "$APP"
test -f "$APP/.microbridge-brew"
test "$(defaults read "$APP/Contents/Info" CFBundleShortVersionString)" = "$VERSION"
brew services list | grep -E '^microbridge[[:space:]]+(started|scheduled)'

open "$APP"
APP_PID=""
for _ in {1..10}; do
  APP_PID="$(pgrep -f "$APP/Contents/MacOS/" | head -1 || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 1
done
test -n "$APP_PID"
kill "$APP_PID"

brew services stop "$TAP/microbridge"
HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall "$TAP/microbridge"
test ! -e "$(brew --cellar)/microbridge/$VERSION"
if brew services list | grep -q '^microbridge[[:space:]]'; then
  echo "microbridge service is still registered after uninstall" >&2
  exit 1
fi

# The app is deliberately outside the Cellar so the menu-bar UI survives
# formula upgrades. Remove it only after verifying this install's marker.
test -f "$APP/.microbridge-brew"
rm -rf "$APP"
test ! -e "$APP"
brew untap "$TAP"
trap - EXIT
