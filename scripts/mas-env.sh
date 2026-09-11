#!/usr/bin/env bash
# Sourced by the build/sign/pkg scripts. Loads .env.mas and validates the basics.
# Run from the repo root (the scripts cd there first).

if [ ! -f .env.mas ]; then
  echo "error: .env.mas not found." >&2
  echo "  cp .env.mas.example .env.mas   then fill in TEAM_ID / BUNDLE_ID / identities" >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
. ./.env.mas
set +a

: "${TEAM_ID:?set TEAM_ID in .env.mas}"
: "${BUNDLE_ID:?set BUNDLE_ID in .env.mas}"
