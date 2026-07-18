#!/usr/bin/env bash
# Install a candidate formula in an isolated tap and verify the complete macOS
# package lifecycle. The caller supplies a local formula, expected version, and
# expected machine architecture.
set -euo pipefail
export HOMEBREW_NO_COLOR=1

FORMULA="${1:?usage: $0 path/to/microbridge.rb VERSION ARCH}"
VERSION="${2:?usage: $0 path/to/microbridge.rb VERSION ARCH}"
EXPECTED_ARCH="${3:?usage: $0 path/to/microbridge.rb VERSION ARCH}"
TAP="devvig/microbridge-ci"
APP="$HOME/Applications/Microbridge.app"
BREW_LOG="$(brew --prefix)/var/log/microbridge.log"
APP_LOG="${RUNNER_TEMP:-/tmp}/microbridge-app.log"

cleanup() {
  brew services stop "$TAP/microbridge" >/dev/null 2>&1 || true
  HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall "$TAP/microbridge" >/dev/null 2>&1 || true
  if [[ -f "$APP/.microbridge-brew" ]]; then
    rm -rf "$APP"
  fi
  brew untap "$TAP" >/dev/null 2>&1 || true
}

diagnostics() {
  echo "==> Microbridge formula smoke diagnostics" >&2
  brew services list >&2 || true
  if [[ -f "$BREW_LOG" ]]; then
    echo "==> $BREW_LOG" >&2
    tail -200 "$BREW_LOG" >&2 || true
  fi
  if [[ -f "$APP_LOG" ]]; then
    echo "==> $APP_LOG" >&2
    tail -200 "$APP_LOG" >&2 || true
  fi
  if [[ -d "$APP" ]]; then
    echo "==> Installed app bundle" >&2
    find "$APP/Contents" -maxdepth 2 -type f -print >&2 || true
    /usr/libexec/PlistBuddy -c 'Print:CFBundleShortVersionString' \
      "$APP/Contents/Info.plist" >&2 || true
  fi
}

on_exit() {
  local status=$?
  trap - EXIT
  if [[ "$status" -ne 0 ]]; then
    diagnostics
  fi
  cleanup
  exit "$status"
}
trap on_exit EXIT

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
for _ in {1..30}; do
  [[ -f "$APP/.microbridge-brew" ]] && break
  sleep 1
done
test -d "$APP"
test -f "$APP/.microbridge-brew"
test "$(/usr/libexec/PlistBuddy -c 'Print:CFBundleShortVersionString' "$APP/Contents/Info.plist")" = "$VERSION"

SERVICE_STATE=""
for _ in {1..30}; do
  SERVICE_STATE="$(brew services list | awk '$1 == "microbridge" { print $2; exit }')"
  [[ "$SERVICE_STATE" == "started" || "$SERVICE_STATE" == "scheduled" ]] && break
  sleep 1
done
test "$SERVICE_STATE" = "started" || test "$SERVICE_STATE" = "scheduled"

"$APP/Contents/MacOS/microbridge-ui" >"$APP_LOG" 2>&1 &
APP_PID=$!
sleep 3
kill -0 "$APP_PID"
kill "$APP_PID" || true
wait "$APP_PID" || true

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
