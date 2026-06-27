#!/usr/bin/env bash
# Start the app in dev, after freeing the Vite port if a previous `tauri dev`
# left a stray Vite behind (Ctrl+C doesn't always kill the grandchild process,
# and strictPort:true then fails -> blank white window).
#
#   nix develop --command bash scripts/dev.sh
set -euo pipefail

cd "$(dirname "$0")/.."
PORT="${TRANSCRIPT_DEV_PORT:-1420}"
HERE="$(pwd)"

# Kill only OUR leftover process on the dev port (scoped by cwd).
for p in $(ss -ltnp 2>/dev/null | grep ":$PORT " | grep -oP 'pid=\K[0-9]+' | sort -u); do
  cwd="$(readlink "/proc/$p/cwd" 2>/dev/null || true)"
  if [ "$cwd" = "$HERE" ]; then
    echo "› freeing port $PORT (stale dev server pid $p)"
    kill "$p" 2>/dev/null || true
  fi
done

exec bun run tauri dev "$@"
