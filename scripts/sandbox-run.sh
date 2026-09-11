#!/usr/bin/env bash
# Build a local .app, codesign it WITH the app-sandbox entitlement, and launch it —
# so you can test real sandboxed behavior locally. No Apple Developer Program needed.
#
# Signing identity, in order of preference:
#   - $APPLE_DEV_IDENTITY  (e.g. "Apple Development: you@example.com (TEAMID)" from a
#                           free Apple ID added in Xcode → Settings → Accounts)
#   - "-"                  (ad-hoc; no account required — still enforces the sandbox)
set -euo pipefail
cd "$(dirname "$0")/.."

# Persist APPLE_DEV_IDENTITY in a git-ignored .env.local instead of export-ing each time.
if [ -f .env.local ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env.local
  set +a
fi

IDENTITY="${APPLE_DEV_IDENTITY:--}"
echo "building local .app (debug, native arch)…"
pnpm tauri build --debug --bundles app

APP="src-tauri/target/debug/bundle/macos/My App Spot.app"
[ -d "$APP" ] || { echo "expected $APP — build failed?"; exit 1; }

echo "codesigning with identity: ${IDENTITY} + app-sandbox…"
codesign --force --sign "$IDENTITY" \
  --entitlements src-tauri/entitlements.plist \
  "$APP"

echo "sandbox container: ~/Library/Containers/<bundle-id>/"
echo "launching…  (press your shortcut — default ⌘⇧Space)"
open "$APP"
