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
MARKER="$HOME/Applications/.Microbridge.app.microbridge-brew"
LEGACY_MARKER="$APP/.microbridge-brew"
BREW_LOG="$(brew --prefix)/var/log/microbridge.log"
APP_LOG="${RUNNER_TEMP:-/tmp}/microbridge-app.log"

cleanup() {
  brew services stop "$TAP/microbridge" >/dev/null 2>&1 || true
  microbridge-app uninstall >/dev/null 2>&1 || true
  HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall "$TAP/microbridge" >/dev/null 2>&1 || true
  if [[ -f "$MARKER" || -f "$LEGACY_MARKER" ]]; then
    rm -rf "$APP"
    rm -f "$MARKER"
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

microbridge-app install
for _ in {1..30}; do
  [[ -f "$MARKER" ]] && break
  sleep 1
done
test -d "$APP"
test -f "$MARKER"
test ! -e "$LEGACY_MARKER"
test "$(/usr/libexec/PlistBuddy -c 'Print:CFBundleShortVersionString' "$APP/Contents/Info.plist")" = "$VERSION"
codesign --verify --deep --strict --verbose=4 "$APP"
spctl --assess --type execute --verbose=4 "$APP"
xcrun stapler validate "$APP"
syspolicy_check distribution "$APP"

APP_EXECUTABLE="$APP/Contents/MacOS/microbridge-ui"
APP_PID=""
for _ in {1..30}; do
  APP_PID="$(pgrep -f "^${APP_EXECUTABLE}$" | head -n1 || true)"
  [[ -n "$APP_PID" ]] && break
  sleep 1
done
[[ -n "$APP_PID" ]]
kill -0 "$APP_PID"
microbridgectl status >"$APP_LOG"
kill "$APP_PID" || true
wait "$APP_PID" || true
APP_DAEMON_STOPPED=0
for _ in {1..30}; do
  if ! microbridgectl status >/dev/null 2>&1; then
    APP_DAEMON_STOPPED=1
    break
  fi
  sleep 1
done
test "$APP_DAEMON_STOPPED" -eq 1

# The daemon service is an explicit headless path, verified separately from
# the standard app-owned GUI lifecycle above.
brew services start "$TAP/microbridge"
SERVICE_STATE=""
for _ in {1..30}; do
  SERVICE_STATE="$(brew services list --json | jq -r 'map(select(.name=="microbridge")) | .[0].status // empty')"
  [[ "$SERVICE_STATE" == "started" || "$SERVICE_STATE" == "scheduled" ]] && break
  sleep 1
done
test "$SERVICE_STATE" = "started" || test "$SERVICE_STATE" = "scheduled"

brew services stop "$TAP/microbridge"
microbridge-app uninstall
HOMEBREW_NO_INSTALL_CLEANUP=1 brew uninstall "$TAP/microbridge"
test ! -e "$(brew --cellar)/microbridge/$VERSION"
if brew services list --json | jq -e '.[] | select(.name=="microbridge")' >/dev/null; then
  echo "microbridge service is still registered after uninstall" >&2
  exit 1
fi

test ! -e "$APP"
test ! -e "$MARKER"
brew untap "$TAP"
trap - EXIT
