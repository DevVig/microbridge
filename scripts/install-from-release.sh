#!/usr/bin/env bash
# Download a GitHub Release archive and install the menu bar app + daemon
# (+ launchd on macOS). Not a CLI-only install.
set -euo pipefail

REPO="${MICROBRIDGE_REPO:-DevVig/microbridge}"
BIN_DIR="${MICROBRIDGE_BIN:-$HOME/.local/bin}"
TAG="${1:-}"
LABEL="ai.microbridge.daemon"
UI_LABEL="ai.microbridge.ui"

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
  Linux-aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
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
trap 'rm -rf "$TMP"' EXIT

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
  </dict>
</dict>
</plist>
EOF
  launchctl bootout "gui/$(id -u)/${LABEL}" 2>/dev/null || true
  launchctl bootstrap "gui/$(id -u)" "$PLIST"
  launchctl enable "gui/$(id -u)/${LABEL}"
  launchctl kickstart -k "gui/$(id -u)/${LABEL}"

  UI_ASSET="microbridge-ui-${TAG}-macos.tar.gz"
  UI_URL="https://github.com/${REPO}/releases/download/${TAG}/${UI_ASSET}"
  echo "==> Downloading menu bar app $UI_URL"
  if curl -fsSL -o "$TMP/$UI_ASSET" "$UI_URL"; then
    tar -xzf "$TMP/$UI_ASSET" -C "$TMP"
    APP_SRC="$(find "$TMP" -name 'Microbridge.app' -type d | head -n1 || true)"
    if [[ -n "$APP_SRC" ]]; then
      rm -rf "$HOME/Applications/Microbridge.app"
      mkdir -p "$HOME/Applications"
      cp -R "$APP_SRC" "$HOME/Applications/Microbridge.app"
      UI_PLIST="$HOME/Library/LaunchAgents/${UI_LABEL}.plist"
      cat >"$UI_PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${UI_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${HOME}/Applications/Microbridge.app/Contents/MacOS/microbridge-ui</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <false/>
</dict>
</plist>
EOF
      launchctl bootout "gui/$(id -u)/${UI_LABEL}" 2>/dev/null || true
      launchctl bootstrap "gui/$(id -u)" "$UI_PLIST"
      launchctl enable "gui/$(id -u)/${UI_LABEL}"
      launchctl kickstart -k "gui/$(id -u)/${UI_LABEL}" 2>/dev/null || open "$HOME/Applications/Microbridge.app"
      echo "    installed ~/Applications/Microbridge.app"
    else
      echo "    warning: archive had no Microbridge.app"
    fi
  else
    echo "    warning: no UI asset for ${TAG} — install UI with ./scripts/install.sh or brew"
  fi
fi

echo "Installed ${TAG}"
echo "  UI:     ~/Applications/Microbridge.app (macOS)"
echo "  bins:   $BIN_DIR"
echo "  status: microbridgectl status"
