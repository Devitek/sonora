#!/usr/bin/env bash
# Generate on-brand screenshot placeholders for the GitHub Pages site.
# Requires ImageMagick 7 (`magick`). Re-run after tweaking to regenerate.
#
#   bash scripts/gen-placeholders.sh
#
# To use a real screenshot instead, simply overwrite the produced PNG with the
# same filename (e.g. docs/assets/img/hero.png) — no HTML change needed.
set -euo pipefail

OUT_DIR="docs/assets/img"
mkdir -p "$OUT_DIR"

W=1280
H=720

# Brand palette
BG_GLOW="#171231"
BG="#0A0B12"
PANEL="#11131D"
BORDER="#262A3B"
TITLE_COL="#E7E9F3"
FILE_COL="#22D3EE"
EYEBROW_COL="#767C95"

# Waveform bars (echoes the logo): centers, colors, half-heights
CENTERS=(544 592 640 688 736)
COLORS=("#7C5CFF" "#6A78FF" "#479BF4" "#2FC2EC" "#22D3EE")
HALF=(40 70 100 70 40)
CY=300

gen() {
  local out="$1" title="$2" label="$3"

  # Build the per-bar draw arguments
  local bars=()
  for i in 0 1 2 3 4; do
    local c=${CENTERS[$i]} hh=${HALF[$i]}
    local x0=$((c - 9)) x1=$((c + 9)) y0=$((CY - hh)) y1=$((CY + hh))
    bars+=(-fill "${COLORS[$i]}" -draw "roundrectangle ${x0},${y0} ${x1},${y1} 9,9")
  done

  magick -size ${W}x${H} "radial-gradient:${BG_GLOW}-${BG}" \
    -stroke none \
    -fill "$PANEL" -draw "roundrectangle 40,40 1240,680 24,24" \
    -fill none -stroke "$BORDER" -strokewidth 2 -draw "roundrectangle 40,40 1240,680 24,24" \
    -stroke none \
    "${bars[@]}" \
    -font DejaVu-Sans -pointsize 22 -fill "$EYEBROW_COL" \
    -gravity North -annotate +0+120 "S O N O R A" \
    -font DejaVu-Sans-Bold -pointsize 44 -fill "$TITLE_COL" \
    -gravity North -annotate +0+430 "$title" \
    -font DejaVu-Sans -pointsize 26 -fill "$FILE_COL" \
    -gravity North -annotate +0+500 "$label" \
    -depth 8 -strip "$out"
  echo "wrote $out"
}

gen "$OUT_DIR/hero.png"     "Capture d'écran à venir"        "assets/img/hero.png"
gen "$OUT_DIR/settings.png" "Panneau Réglages — à venir"     "assets/img/settings.png"

echo "Done."
