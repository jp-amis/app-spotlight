#!/usr/bin/env bash
# Build a signed .pkg for App Store Connect upload, using values from .env.mas.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=scripts/mas-env.sh
. scripts/mas-env.sh
: "${APPLE_INSTALLER_IDENTITY:?set APPLE_INSTALLER_IDENTITY in .env.mas}"

APP="src-tauri/target/universal-apple-darwin/release/bundle/macos/My App Spot.app"
OUT="My App Spot.pkg"
[ -d "$APP" ] || { echo "build+sign first ($APP not found)"; exit 1; }

xcrun productbuild --sign "$APPLE_INSTALLER_IDENTITY" \
  --component "$APP" /Applications/ "$OUT"
echo "wrote: $OUT  (upload via Transporter or 'xcrun altool')"
