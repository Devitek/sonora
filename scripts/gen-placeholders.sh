#!/usr/bin/env bash
# Generate on-brand screenshot / video placeholders for the GitHub Pages site.
# Requires ImageMagick 7 (`magick`). Re-run after tweaking to regenerate.
#
#   bash scripts/gen-placeholders.sh
#
# To use a real screenshot instead, simply overwrite the produced PNG with the
# same filename (e.g. docs/assets/img/hero.png) — no HTML change needed.
# For the demo: drop docs/assets/img/demo.mp4 (the poster is shown until played).
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

# gen <out> <title> <label> [mark: wave|play]
gen() {
  local out="$1" title="$2" label="$3" mark="${4:-wave}"

  local marks=()
  if [[ "$mark" == "play" ]]; then
    # A play button: soft circle + triangle, centered on (640, CY)
    marks+=(-fill "rgba(124,92,255,0.16)" -stroke "#7C5CFF" -strokewidth 3 \
            -draw "circle 640,${CY} 640,$((CY + 66))" \
            -stroke none -fill "#E7E9F3" \
            -draw "polygon 614,$((CY - 34)) 614,$((CY + 34)) 676,${CY}")
  else
    # Waveform bars
    local i
    for i in 0 1 2 3 4; do
      local c=${CENTERS[$i]} hh=${HALF[$i]}
      local x0=$((c - 9)) x1=$((c + 9)) y0=$((CY - hh)) y1=$((CY + hh))
      marks+=(-fill "${COLORS[$i]}" -draw "roundrectangle ${x0},${y0} ${x1},${y1} 9,9")
    done
  fi

  magick -size ${W}x${H} "radial-gradient:${BG_GLOW}-${BG}" \
    -stroke none \
    -fill "$PANEL" -draw "roundrectangle 40,40 1240,680 24,24" \
    -fill none -stroke "$BORDER" -strokewidth 2 -draw "roundrectangle 40,40 1240,680 24,24" \
    -stroke none \
    "${marks[@]}" \
    -stroke none \
    -font DejaVu-Sans -pointsize 22 -fill "$EYEBROW_COL" \
    -gravity North -annotate +0+120 "S O N O R A" \
    -font DejaVu-Sans-Bold -pointsize 44 -fill "$TITLE_COL" \
    -gravity North -annotate +0+430 "$title" \
    -font DejaVu-Sans -pointsize 26 -fill "$FILE_COL" \
    -gravity North -annotate +0+500 "$label" \
    -depth 8 -strip "$out"
  echo "wrote $out"
}

gen "$OUT_DIR/hero.png"        "Capture d'écran à venir"     "assets/img/hero.png"      wave
gen "$OUT_DIR/settings.png"    "Panneau Réglages — à venir"  "assets/img/settings.png"  wave
gen "$OUT_DIR/docs-reglages.png" "Réglages de Sonora — à venir" "assets/img/docs-reglages.png" wave
gen "$OUT_DIR/demo-poster.png" "Démo vidéo — à venir"        "assets/img/demo.mp4"      play

echo "Done."
