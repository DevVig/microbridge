#!/usr/bin/env bash
# Bump Formula/microbridge.rb to a new release tag and fill sha256 for the
# prebuilt daemon + UI archives (same URLs Homebrew fetches).
# Usage: ./scripts/bump-formula.sh v0.1.0
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAG="${1:?usage: $0 vX.Y.Z}"
TAG="${TAG#v}"
FULL="v${TAG}"
FORMULA="$ROOT/Formula/microbridge.rb"
REPO="${MICROBRIDGE_REPO:-DevVig/microbridge}"
# Tests may point at the exact workflow artifacts through a local file:// base.
# Publication keeps the default public GitHub release base.
BASE="${MICROBRIDGE_ASSET_BASE:-https://github.com/${REPO}/releases/download/${FULL}}"

sha_of() {
  local url="$1"
  local tmp
  tmp="$(mktemp)"
  echo "==> Fetching ${url}" >&2
  # Prefer the public release asset URL (identical bytes to brew's download).
  # Fall back to gh for private repos during the bump job.
  if curl -fsSL -o "$tmp" "$url" 2>/dev/null; then
    :
  elif command -v gh >/dev/null; then
    local name
    name="$(basename "$url")"
    gh release download "$FULL" --repo "$REPO" --pattern "$name" --dir "$(dirname "$tmp")" --clobber
    mv "$(dirname "$tmp")/$name" "$tmp"
  else
    echo "error: cannot download $url" >&2
    exit 1
  fi
  shasum -a 256 "$tmp" | awk '{print $1}'
  rm -f "$tmp"
}

DAEMON_ARM_URL="${BASE}/microbridge-${FULL}-aarch64-apple-darwin.tar.gz"
DAEMON_INTEL_URL="${BASE}/microbridge-${FULL}-x86_64-apple-darwin.tar.gz"
UI_ARM_URL="${BASE}/microbridge-ui-${FULL}-aarch64-apple-darwin.tar.gz"
UI_INTEL_URL="${BASE}/microbridge-ui-${FULL}-x86_64-apple-darwin.tar.gz"

DAEMON_ARM_SHA="$(sha_of "$DAEMON_ARM_URL")"
DAEMON_INTEL_SHA="$(sha_of "$DAEMON_INTEL_URL")"
UI_ARM_SHA="$(sha_of "$UI_ARM_URL")"
UI_INTEL_SHA="$(sha_of "$UI_INTEL_URL")"

echo "==> Rewriting ${FORMULA} → ${FULL}"

python3 - "$FORMULA" "$TAG" \
  "$DAEMON_ARM_URL" "$DAEMON_ARM_SHA" \
  "$DAEMON_INTEL_URL" "$DAEMON_INTEL_SHA" \
  "$UI_ARM_URL" "$UI_ARM_SHA" \
  "$UI_INTEL_URL" "$UI_INTEL_SHA" <<'PY'
import pathlib, re, sys

path = pathlib.Path(sys.argv[1])
version = sys.argv[2]
daemon_arm_url, daemon_arm_sha = sys.argv[3], sys.argv[4]
daemon_intel_url, daemon_intel_sha = sys.argv[5], sys.argv[6]
ui_arm_url, ui_arm_sha = sys.argv[7], sys.argv[8]
ui_intel_url, ui_intel_sha = sys.argv[9], sys.argv[10]

text = path.read_text()
text = re.sub(r'version "[^"]+"', f'version "{version}"', text, count=1)

def replace_block(src: str, arch_marker: str, daemon_url: str, daemon_sha: str, ui_url: str, ui_sha: str) -> str:
    # Replace the on_arm / on_intel block URLs + sha256 pairs in order.
    pattern = rf'(on_{arch_marker} do\n)(.*?)(\n    end\n)'
    m = re.search(pattern, src, flags=re.S)
    if not m:
        raise SystemExit(f"missing on_{arch_marker} block")
    body = m.group(2)
    # First url/sha256 = daemon; next url/sha256 inside resource = ui
    urls = list(re.finditer(r'url "[^"]+"', body))
    shas = list(re.finditer(r'sha256 "[^"]+"', body))
    if len(urls) < 2 or len(shas) < 2:
        raise SystemExit(f"expected 2 url/sha256 pairs in on_{arch_marker}, found {len(urls)}/{len(shas)}")
    # Rebuild body with substitutions from the end so offsets stay valid.
    replacements = [
        (urls[0], f'url "{daemon_url}"'),
        (shas[0], f'sha256 "{daemon_sha}"'),
        (urls[1], f'url "{ui_url}"'),
        (shas[1], f'sha256 "{ui_sha}"'),
    ]
    replacements.sort(key=lambda x: x[0].start(), reverse=True)
    for match, new in replacements:
        body = body[: match.start()] + new + body[match.end() :]
    return src[: m.start(2)] + body + src[m.end(2) :]

text = replace_block(text, "arm", daemon_arm_url, daemon_arm_sha, ui_arm_url, ui_arm_sha)
text = replace_block(text, "intel", daemon_intel_url, daemon_intel_sha, ui_intel_url, ui_intel_sha)
path.write_text(text)
print(path)
PY

echo "Updated formula:"
grep -E 'version |url |sha256 ' "$FORMULA" | head -40
