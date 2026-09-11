# 0010 — Build, release & CI

## Goal

Reproducible builds and a smooth release path, driven by `mise` tasks.

## `mise` tasks

- `dev` — `pnpm tauri dev`.
- `build` — `cargo tauri build --target universal-apple-darwin`.
- `sign` — codesign `.app` with Apple Distribution + entitlements.
- `pkg` — `productbuild` → signed `.pkg` for MAS.
- `release` — build → sign → pkg → (optional) upload via `notarytool`/Transporter.
- `icon` — port reference `build-icon.sh` (sips + iconutil → `.icns` + png set).
- `fmt` / `lint` / `test`.

Keep signing identities/Team ID out of the repo (env or a git-ignored `.env` / mise env).

## Versioning

- Single source in `tauri.conf.json` `version`; bump per release. Git Flow release branches.

## CI (optional, later)

- macOS runner: `mise install` → build (unsigned) → basic checks. Signing/upload stays local (needs certs) unless secrets configured.

## Release checklist

- [ ] Version bump + changelog.
- [ ] `mise run release` produces signed `.pkg`.
- [ ] Upload to App Store Connect; submit for review.
- [ ] Tag release (Git Flow).

## Tasks

- [ ] Author `mise.toml` tasks above.
- [ ] Port icon script.
- [ ] Document required env (Team ID, identities) in `docs/`.
- [ ] Dry-run full release once (0008).

## Risks

- Universal build + signing quirks (see 0008).
- Secrets management for CI signing — defer until needed.

## Unresolved questions

- Want CI now (unsigned verify builds), or local-only until v1 ships?
- Auto-notarize/upload in `release`, or keep upload manual via Transporter?
