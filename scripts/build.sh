#!/usr/bin/env bash
# Universal release build. If .env.mas exists, applies the MAS identifier +
# embedded provisioning profile via a Tauri config override; otherwise builds
# a plain unsigned universal bundle.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -f .env.mas ]; then
  # shellcheck source=scripts/mas-env.sh
  . scripts/mas-env.sh

  if [ -f src-tauri/embedded.provisionprofile ]; then
    FILES='"files": { "embedded.provisionprofile": "embedded.provisionprofile" }'
  else
    echo "note: src-tauri/embedded.provisionprofile not found — building without an embedded profile." >&2
    FILES=''
  fi

  OVERRIDE="src-tauri/tauri.mas.conf.json"
  cat > "$OVERRIDE" <<EOF
{ "identifier": "$BUNDLE_ID", "bundle": { "macOS": { $FILES } } }
EOF
  echo "building with identifier=$BUNDLE_ID"
  pnpm tauri build --target universal-apple-darwin --config "$OVERRIDE"
else
  echo "no .env.mas — plain unsigned universal build."
  echo "  (cp .env.mas.example .env.mas to configure Mac App Store signing)"
  pnpm tauri build --target universal-apple-darwin
fi
