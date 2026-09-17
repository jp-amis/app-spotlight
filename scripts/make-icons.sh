#!/usr/bin/env bash
# Rasterize the SVG icon sources in assets/ into the app's icon set and the
# menu-bar template icon. Edit the SVGs (e.g. in Inkscape), then run:
#   mise run make-icons     (or ./scripts/make-icons.sh)
set -euo pipefail
cd "$(dirname "$0")/.."

APP_SVG="assets/app-icon.svg"
BAR_SVG="assets/menubar-icon.svg"
[ -f "$APP_SVG" ] || { echo "missing $APP_SVG" >&2; exit 1; }
[ -f "$BAR_SVG" ] || { echo "missing $BAR_SVG" >&2; exit 1; }

# Rasterize <svg> <w> <h> <out> using whatever's installed.
render() {
  local svg=$1 w=$2 h=$3 out=$4
  if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w "$w" -h "$h" "$svg" -o "$out"
  elif command -v inkscape >/dev/null 2>&1; then
    inkscape "$svg" --export-type=png --export-filename="$out" -w "$w" -h "$h"
  elif [ -x /Applications/Inkscape.app/Contents/MacOS/inkscape ]; then
    /Applications/Inkscape.app/Contents/MacOS/inkscape "$svg" \
      --export-type=png --export-filename="$out" -w "$w" -h "$h"
  elif command -v magick >/dev/null 2>&1; then
    magick -background none "$svg" -resize "${w}x${h}" "$out"
  else
    echo "no SVG rasterizer found — brew install librsvg (rsvg-convert)" >&2
    exit 1
  fi
}

echo "==> app icon: $APP_SVG -> 1024 png -> icon set"
TMP="assets/app-icon-1024.png"
render "$APP_SVG" 1024 1024 "$TMP"
pnpm tauri icon "$TMP"
# Drop the non-macOS assets the generator emits.
rm -rf src-tauri/icons/android src-tauri/icons/ios \
       src-tauri/icons/icon.ico src-tauri/icons/Square*Logo.png \
       src-tauri/icons/StoreLogo.png 2>/dev/null || true

echo "==> menu-bar template: $BAR_SVG -> src-tauri/icons/menubar.png (@2x)"
render "$BAR_SVG" 36 36 src-tauri/icons/menubar.png

echo "done — rebuild the app (mise run sandbox) to see the new icons."
