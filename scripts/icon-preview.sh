#!/usr/bin/env bash
# Preview the shipped app icon at every size macOS embeds, at TRUE pixel size
# (so small-size legibility is honest). Expands icon.icns and opens a montage.
#   mise run icon-preview   (or ./scripts/icon-preview.sh)
set -euo pipefail
cd "$(dirname "$0")/.."

ICNS="src-tauri/icons/icon.icns"
[ -f "$ICNS" ] || { echo "no $ICNS — run 'mise run make-icons' first" >&2; exit 1; }

TMP="$(mktemp -d)"
iconutil -c iconset "$ICNS" -o "$TMP/AppIcon.iconset"

OUT="$TMP/icon-sizes.png"
if command -v magick >/dev/null 2>&1; then
  magick montage \
    "$TMP/AppIcon.iconset/icon_16x16.png" \
    "$TMP/AppIcon.iconset/icon_32x32.png" \
    "$TMP/AppIcon.iconset/icon_128x128.png" \
    "$TMP/AppIcon.iconset/icon_256x256.png" \
    "$TMP/AppIcon.iconset/icon_512x512.png" \
    -tile 5x1 -geometry +14+14 -background "#dedede" \
    -label '%wx%h' -pointsize 16 -fill "#333" "$OUT"
  echo "wrote $OUT"
  open "$OUT"
else
  echo "imagemagick not found (brew install imagemagick) — opening icon.icns in Preview instead." >&2
  open "$ICNS"
fi
