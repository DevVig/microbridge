#!/usr/bin/env bash
# Download a GitHub Release archive and install the menu bar app + daemon.
# The GUI owns its bundled daemon; launchd is reserved for explicit headless installs.
set -euo pipefail

REPO="${MICROBRIDGE_REPO:-DevVig/microbridge}"
BIN_DIR="${MICROBRIDGE_BIN:-$HOME/.local/bin}"
TAG="${1:-}"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: '$1' is required" >&2
    exit 1
  }
}

need curl
need tar

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS-$ARCH" in
  Darwin-arm64) TARGET="aarch64-apple-darwin" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  Linux-x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64)
    echo "error: Linux aarch64 release binaries are not published yet" >&2
    echo "hint: build from source with ./scripts/install.sh --no-ui" >&2
    exit 1
    ;;
  *)
    echo "unsupported platform: $OS $ARCH" >&2
    exit 1
    ;;
esac

if [[ -z "$TAG" ]]; then
  need jq
  TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | jq -r .tag_name)"
  if [[ -z "$TAG" || "$TAG" == "null" ]]; then
    echo "error: no GitHub releases found for ${REPO}" >&2
    echo "hint: build from source with ./scripts/install.sh" >&2
    exit 1
  fi
fi

ASSET="microbridge-${TAG}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
TMP="$(mktemp -d)"
APP_STAGING=""
trap 'rm -rf "$TMP"; if [[ -n "${APP_STAGING:-}" ]]; then rm -rf "$APP_STAGING"; fi' EXIT

echo "==> Downloading $URL"
if ! curl -fsSL -o "$TMP/$ASSET" "$URL"; then
  echo "error: download failed — is ${TAG} published with ${ASSET}?" >&2
  echo "hint: ./scripts/install.sh builds from source instead" >&2
  exit 1
fi

tar -xzf "$TMP/$ASSET" -C "$TMP"
BIN_SRC="$(find "$TMP" -type f -name microbridged | head -n1)"
CTL_SRC="$(find "$TMP" -type f -name microbridgectl | head -n1)"
if [[ -z "$BIN_SRC" || -z "$CTL_SRC" ]]; then
  echo "error: archive missing microbridged/microbridgectl" >&2
  exit 1
fi
mkdir -p "$BIN_DIR" "$HOME/.microbridge"
install -m 755 "$BIN_SRC" "$BIN_DIR/microbridged"
install -m 755 "$CTL_SRC" "$BIN_DIR/microbridgectl"

if [[ "$OS" == "Darwin" ]]; then
  DEST="$HOME/Applications/Microbridge.app"
  MARKER="$HOME/Applications/.Microbridge.app.microbridge-release"
  LEGACY_MARKER="$DEST/.microbridge-release"
  install_app_bundle() {
    local APP_SRC="$1"
    if [[ -d "$DEST" && ! -f "$MARKER" && ! -f "$LEGACY_MARKER" && "${MICROBRIDGE_FORCE_APP:-}" != "1" ]]; then
      echo "    warning: $DEST exists and is not release-managed — leave it"
      echo "    set MICROBRIDGE_FORCE_APP=1 to replace"
      return 0
    fi
    local STAGING="$HOME/Applications/.Microbridge.app.installing.$$"
    APP_STAGING="$STAGING"
    mkdir -p "$HOME/Applications"
    rm -rf "$STAGING"
    /usr/bin/ditto "$APP_SRC" "$STAGING"
    /usr/bin/codesign --verify --deep --strict "$STAGING"
    if [[ -d "$DEST" ]]; then
      while read -r pid; do
        kill "$pid" 2>/dev/null || true
      done < <(/usr/bin/pgrep -f "^${DEST}/Contents/MacOS/microbridge-ui$" 2>/dev/null || true)
      for _ in 1 2 3 4 5 6 7 8 9 10; do
        /usr/bin/pgrep -f "^${DEST}/Contents/MacOS/microbridge-ui$" >/dev/null 2>&1 || break
        /bin/sleep 0.1
      done
    fi
    rm -rf "$DEST"
    mv "$STAGING" "$DEST"
    APP_STAGING=""
    xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true
    touch "$MARKER"
    rm -f "$LEGACY_MARKER"
    # Launch at login and the bundled daemon are both owned by the app.
    open "$HOME/Applications/Microbridge.app" 2>/dev/null || true
    echo "    installed ~/Applications/Microbridge.app"
  }

  DMG_ASSET="microbridge-ui-${TAG}-${TARGET}.dmg"
  DMG_URL="https://github.com/${REPO}/releases/download/${TAG}/${DMG_ASSET}"
  UI_ASSET="microbridge-ui-${TAG}-${TARGET}.tar.gz"
  UI_URL="https://github.com/${REPO}/releases/download/${TAG}/${UI_ASSET}"
  # Backward-compatible fallback for older releases that shipped a single asset.
  UI_FALLBACK="microbridge-ui-${TAG}-macos.tar.gz"

  INSTALLED_UI=0
  echo "==> Trying signed DMG $DMG_URL"
  if curl -fsSL -o "$TMP/$DMG_ASSET" "$DMG_URL"; then
    MOUNT="$(mktemp -d "$TMP/dmg.XXXXXX")"
    if hdiutil attach "$TMP/$DMG_ASSET" -mountpoint "$MOUNT" -nobrowse -quiet; then
      APP_SRC="$(find "$MOUNT" -name 'Microbridge.app' -type d | head -n1 || true)"
      if [[ -n "$APP_SRC" ]]; then
        install_app_bundle "$APP_SRC"
        INSTALLED_UI=1
      fi
      hdiutil detach "$MOUNT" -quiet || true
    fi
  fi

  if [[ "$INSTALLED_UI" -eq 0 ]]; then
    echo "==> Downloading menu bar app archive $UI_URL"
    if curl -fsSL -o "$TMP/$UI_ASSET" "$UI_URL" \
      || curl -fsSL -o "$TMP/$UI_ASSET" "https://github.com/${REPO}/releases/download/${TAG}/${UI_FALLBACK}"; then
      tar -xzf "$TMP/$UI_ASSET" -C "$TMP"
      APP_SRC="$(find "$TMP" -name 'Microbridge.app' -type d | head -n1 || true)"
      if [[ -n "$APP_SRC" ]]; then
        install_app_bundle "$APP_SRC"
        INSTALLED_UI=1
      else
        echo "    warning: archive had no Microbridge.app"
      fi
    else
      echo "    warning: no UI asset for ${TAG} — install UI with ./scripts/install.sh or brew"
    fi
  fi
fi

echo "Installed ${TAG}"
echo "  UI:     ~/Applications/Microbridge.app (macOS)"
echo "  bins:   $BIN_DIR"
echo "  status: microbridgectl status"
