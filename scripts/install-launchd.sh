#!/usr/bin/env bash
# Back-compat wrapper — prefer ./scripts/install.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/scripts/install.sh" "$@"
