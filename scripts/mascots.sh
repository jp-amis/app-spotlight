#!/usr/bin/env bash
# Re-export the mascot SVGs (extras/) as retina PNGs. Single source of truth is
# extras/; this rasterizes into both consumers:
#   site/assets/  — hero slot (mascot) + closing CTA card (mascot-cta)
#   assets/store/ — store screenshot overlays (gen-store-screenshots.py)
set -euo pipefail

cd "$(dirname "$0")/.."

command -v rsvg-convert >/dev/null || {
  echo "error: rsvg-convert not found (brew install librsvg)" >&2; exit 1
}

WIDTH=1200  # retina; scaled down by consumers (site slots, 2560px store frames)

# extras/<src>.svg  ->  each <dest>.png  (source SVG stays in extras/)
export_one() {
  local src="extras/$1.svg"; shift
  [ -f "$src" ] || { echo "error: $src missing" >&2; exit 1; }
  local dest
  for dest in "$@"; do
    rsvg-convert -w "$WIDTH" "$src" -o "$dest"
    echo "  $src -> $dest"
  done
}

echo "exporting mascots:"
export_one main-mascot site/assets/mascot.png     assets/store/mascot.png
export_one mascot-cta  site/assets/mascot-cta.png assets/store/mascot-cta.png
echo "done."
