#!/usr/bin/env bash
# Codesign the universal .app for the Mac App Store, using values from .env.mas.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=scripts/mas-env.sh
. scripts/mas-env.sh
: "${APPLE_DIST_IDENTITY:?set APPLE_DIST_IDENTITY in .env.mas}"

APP="src-tauri/target/universal-apple-darwin/release/bundle/macos/My App Spot.app"
[ -d "$APP" ] || { echo "build first: mise run build ($APP not found)"; exit 1; }

# Strip extended attributes before signing. The provisioning profile (downloaded
# from developer.apple.com) carries com.apple.quarantine, and App Store Connect
# rejects any bundle containing it (error 91109). Must run BEFORE codesign so the
# signature seals the cleaned bundle.
xattr -cr "$APP"

# Generate concrete entitlements from the template (git-ignored output).
GEN="src-tauri/entitlements.generated.plist"
sed -e "s/__TEAM_ID__/${TEAM_ID}/g" \
    -e "s#__BUNDLE_ID__#${BUNDLE_ID}#g" \
    src-tauri/entitlements.template.plist > "$GEN"

codesign --force --timestamp \
  --sign "$APPLE_DIST_IDENTITY" \
  --entitlements "$GEN" \
  "$APP"

codesign --verify --strict --verbose=2 "$APP"
echo "signed: $APP"
echo "entitlements used:"; codesign -d --entitlements - "$APP" 2>/dev/null || true
