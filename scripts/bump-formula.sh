#!/usr/bin/env bash
# Bump Formula/microbridge.rb to a new tag and GitHub archive sha256.
# Usage: ./scripts/bump-formula.sh v0.0.2
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAG="${1:?usage: $0 vX.Y.Z}"
TAG="${TAG#v}"
FULL="v${TAG}"
FORMULA="$ROOT/Formula/microbridge.rb"
URL="https://github.com/DevVig/microbridge/archive/refs/tags/${FULL}.tar.gz"

echo "==> Fetching ${URL}"
TMP="$(mktemp)"
if command -v gh >/dev/null; then
  gh api "repos/DevVig/microbridge/tarball/${FULL}" >"$TMP"
else
  curl -fsSL "$URL" -o "$TMP"
fi
SHA="$(shasum -a 256 "$TMP" | awk '{print $1}')"
rm -f "$TMP"

echo "==> Updating formula → ${FULL} sha256=${SHA}"
# Portable in-place edit
perl -0pi -e "s#url \"https://github.com/DevVig/microbridge/archive/refs/tags/v[^\"]+\"#url \"${URL}\"#" "$FORMULA"
perl -0pi -e "s#sha256 \"[a-f0-9]+\"#sha256 \"${SHA}\"#" "$FORMULA"

echo "Updated $FORMULA"
grep -E 'url |sha256 ' "$FORMULA"
