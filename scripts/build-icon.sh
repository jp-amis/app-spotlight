#!/usr/bin/env bash
# Generate the macOS icon set (.icns + png sizes) from a 1024x1024 source.
# Usage: mise run icon -- path/to/icon-1024.png
set -euo pipefail
SRC="${1:?usage: mise run icon -- path/to/icon-1024.png}"
pnpm tauri icon "$SRC"
# Drop the non-macOS assets the generator emits.
rm -rf src-tauri/icons/android src-tauri/icons/ios \
       src-tauri/icons/icon.ico src-tauri/icons/Square*Logo.png \
       src-tauri/icons/StoreLogo.png 2>/dev/null || true
echo "icon set regenerated in src-tauri/icons/"
