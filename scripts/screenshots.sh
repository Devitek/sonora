#!/usr/bin/env bash
# Capture real Sonora UI screenshots for the docs site and compose them onto an
# on-brand backdrop. Renders the actual Svelte UI in headless WebKitGTK (the
# same engine Tauri uses on Linux) via scripts/screenshot.js.
#
# Run inside the dev shell (needs gjs + webkitgtk + xvfb + ImageMagick):
#   nix develop --command bash scripts/screenshots.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PORT="${PORT:-8766}"
OUT="docs/assets/img"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"

echo "› building frontend…" >&2
bun run build >/dev/null

echo "› serving dist/ on :$PORT" >&2
python3 -m http.server "$PORT" --directory dist >/dev/null 2>&1 &
SRV=$!
trap 'kill "$SRV" 2>/dev/null || true; rm -rf "$TMP"' EXIT
sleep 0.7

export GDK_SCALE=1 GDK_DPI_SCALE=1 GDK_BACKEND=x11

shoot() { # <shot> <w> <h> <settleMs>
  echo "› rendering shot=$1 (${2}x${3})" >&2
  xvfb-run -a -s "-screen 0 1400x1200x24 -dpi 96" \
    gjs scripts/screenshot.js "http://localhost:$PORT/?shot=$1" "$TMP/$1.png" "$2" "$3" "$4"
}

shoot bar 760 460 1100
shoot result 760 320 800
shoot settings 760 1180 900

# --- compose onto an on-brand backdrop --------------------------------------
backdrop() { magick -size "$1x$2" "radial-gradient:#171231-#0A0B12" "$3"; }

# Wide 16:9 product shot: small/wide capture centered on a 1280x720 backdrop.
compose_wide() { # <capture> <resize-geom> <out>
  magick "$TMP/$1.png" -trim +repage "$TMP/$1-t.png"
  backdrop 1280 720 "$TMP/bg-$1.png"
  magick "$TMP/bg-$1.png" \
    \( "$TMP/$1-t.png" -resize "$2" \) -gravity center -geometry +0+0 -composite \
    -depth 8 -strip "$3"
  echo "wrote $3" >&2
}

# Tight shot: keep the (tall) panel near native size with a thin brand margin.
compose_tight() { # <capture> <margin> <out>
  magick "$TMP/$1.png" -trim +repage "$TMP/$1-t.png"
  local dim w h
  dim="$(magick identify -format '%w %h' "$TMP/$1-t.png")"
  w="${dim% *}"
  h="${dim#* }"
  local bw=$((w + 2 * $2)) bh=$((h + 2 * $2))
  backdrop "$bw" "$bh" "$TMP/bg-$1.png"
  magick "$TMP/bg-$1.png" "$TMP/$1-t.png" -gravity center -composite \
    -depth 8 -strip "$3"
  echo "wrote $3 (${bw}x${bh})" >&2
}

compose_wide  bar    "880x520>" "$OUT/hero.png"
compose_wide  result "880x420>" "$OUT/settings.png"
compose_tight settings 56       "$OUT/docs-reglages.png"

echo "Done. Captured: hero.png (bar), settings.png (result), docs-reglages.png (settings)." >&2
