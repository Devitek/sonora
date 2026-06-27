#!/usr/bin/env bash
# Build the frontend, serve dist/ statically, and dump real WebKitGTK layout
# for the given selectors (default: the mic buttons + their containers).
#
#   nix develop --command bash scripts/inspect-layout.sh ".big-mic,.mic"
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SELECTORS="${1:-.big-mic,.mic,.empty,.body,.hud}"
PORT="${PORT:-8765}"

echo "› building frontend…" >&2
bun run build >/dev/null

echo "› serving dist/ on :$PORT" >&2
python3 -m http.server "$PORT" --directory dist >/dev/null 2>&1 &
SRV=$!
trap 'kill "$SRV" 2>/dev/null || true' EXIT
sleep 0.6

echo "› rendering in headless WebKitGTK…" >&2
export GDK_SCALE=1 GDK_DPI_SCALE=1 GDK_BACKEND=x11
xvfb-run -a -s "-screen 0 1280x800x24 -dpi 96" \
  gjs scripts/inspect-layout.js "http://localhost:$PORT" "$SELECTORS"
