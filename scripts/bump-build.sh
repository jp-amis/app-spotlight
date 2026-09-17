#!/usr/bin/env bash
# Auto-increment the App Store build number (CFBundleVersion) on the freshly built
# .app, BEFORE signing (editing the plist after codesign breaks the signature).
#
# The marketing version users see (CFBundleShortVersionString) stays whatever
# src-tauri/tauri.conf.json "version" is — that's the only version you edit by hand.
# This just bumps the build number App Store Connect requires to increase on every
# upload. The counter lives in ./build-number (git-tracked); commit it when ready.
set -euo pipefail
cd "$(dirname "$0")/.."

FILE="build-number"
APP="src-tauri/target/universal-apple-darwin/release/bundle/macos/My App Spot.app"
PLIST="$APP/Contents/Info.plist"

[ -f "$PLIST" ] || { echo "build first: mise run build ($PLIST not found)" >&2; exit 1; }

n=$(cat "$FILE" 2>/dev/null || echo 0)
case "$n" in ''|*[!0-9]*) n=0 ;; esac   # guard against a missing/garbled file
n=$((n + 1))

/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $n" "$PLIST"
echo "$n" > "$FILE"

short=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$PLIST" 2>/dev/null || echo "?")
echo "version $short  ·  build $n  (CFBundleVersion set; commit ./build-number when ready)"
