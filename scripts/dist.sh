#!/usr/bin/env bash
# Developer ID build for direct distribution — the .dmg you send to friends/testers.
# Produces a UNIVERSAL (Intel + Apple Silicon), hardened-runtime, signed, notarized
# and stapled app inside a .dmg. Requires a paid Apple Developer account and a
# "Developer ID Application" certificate in your login keychain.
#
# Setup once:  cp .env.dist.example .env.dist   then fill it in.
# Run:         mise run dist   (or ./scripts/dist.sh)
set -euo pipefail
cd "$(dirname "$0")/.."

[ -f .env.dist ] || {
  echo "error: .env.dist not found." >&2
  echo "  cp .env.dist.example .env.dist   then fill in your signing identity + notarization creds" >&2
  exit 1
}

# Export everything from .env.dist so `tauri build` picks up APPLE_* automatically.
set -a
# shellcheck disable=SC1091
. ./.env.dist
set +a

: "${APPLE_SIGNING_IDENTITY:?set APPLE_SIGNING_IDENTITY in .env.dist (Developer ID Application: NAME (TEAMID))}"

# Tauri notarizes automatically when signing + notarization creds are both present.
if [ -z "${APPLE_ID:-}" ] && [ -z "${APPLE_API_KEY:-}" ]; then
  echo "warning: no notarization credentials in .env.dist — the app will be signed but NOT" >&2
  echo "         notarized, so friends must right-click → Open the first time. Fill in" >&2
  echo "         APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID to notarize." >&2
fi

echo "==> building universal .dmg (sign + notarize) — this takes several minutes…"
pnpm tauri build --target universal-apple-darwin

DMG=$(ls -t src-tauri/target/universal-apple-darwin/release/bundle/dmg/*.dmg 2>/dev/null | head -1 || true)
echo
if [ -n "$DMG" ]; then
  echo "✅ done: $DMG"
  echo "   Send that .dmg to your testers: they open it, drag the app to Applications, launch it."
else
  echo "build finished but no .dmg was found — check the output above for signing/notarization errors." >&2
  exit 1
fi
