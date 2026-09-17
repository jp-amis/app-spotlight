#!/usr/bin/env bash
# Reset all local app data to a clean slate — for testing a fresh install
# (e.g. a new TestFlight build) as if the app had never run.
#
# Everything the sandboxed app persists lives in ONE container:
#   ~/Library/Containers/co.amis.myappspot/
#     Data/Library/Application Support/co.amis.myappspot/settings.json  (tauri store:
#       shortcut, theme, language, font_scale, favorites, favorites_purchased,
#       frecency, granted-folder security bookmarks)
#     Data/Library/Caches/co.amis.myappspot/app-index.json             (index cache)
#     Data/Library/WebKit/.../LocalStorage                             (accessBannerDismissed, fontScale)
#
# Deleting/reinstalling the app on macOS does NOT wipe the container, so we do it here.
# The TestFlight/MAS build and the local `mise run sandbox` build share this container.
#
# NOT cleared (they live outside the container):
#   - "Open at login" (SMAppService, system-level) — toggle it off in the app first.
#   - In-app purchase (tied to your Apple ID) — use "Restore purchase" in the app.
set -euo pipefail

BUNDLE_ID="co.amis.myappspot"
CONTAINER="$HOME/Library/Containers/$BUNDLE_ID"
DATA="$CONTAINER/Data/Library"

# We wipe the app-owned data INSIDE the container, not the container root: macOS
# protects the container's top-level `.com.apple.containermanagerd.metadata.plist`
# (owned by containermanagerd), so `rm -rf` on the root fails with "Operation not
# permitted". The app regenerates everything under Data/ on next launch anyway.
TARGETS=(
  "$DATA/Application Support/$BUNDLE_ID"   # tauri store: settings.json
  "$DATA/Caches/$BUNDLE_ID"                # index cache: app-index.json
  "$DATA/Preferences/$BUNDLE_ID.plist"     # NSUserDefaults, if any
  "$DATA/WebKit"                           # WKWebView storage (localStorage)
  "$DATA/Caches/WebKit"                    # WKWebView caches
  "$DATA/HTTPStorages/$BUNDLE_ID"          # webview HTTP storage, if present
)

echo "quitting the app if it's running…"
pkill -f "My App Spot" 2>/dev/null || true
sleep 1

if [ ! -d "$CONTAINER" ]; then
  echo "nothing to clean — no container at ${CONTAINER/#$HOME/~}"
  exit 0
fi

removed=0
for t in "${TARGETS[@]}"; do
  if [ -e "$t" ]; then
    rm -rf "$t"
    echo "removed ${t/#$HOME/~}"
    removed=1
  fi
done

if [ "$removed" -eq 0 ]; then
  echo "already clean — no app data found in the container."
else
  echo "done — next launch starts fresh (default settings, index rebuilt)."
fi
