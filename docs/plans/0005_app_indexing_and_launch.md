# 0005 — App indexing & launch

## Goal

Enumerate installed apps (name, path, icon, bundle id), keep it fresh, and launch the chosen app — all under App Sandbox.

## ✅ Spike results (2026-08-28) — RISK CLEARED

Ran a Swift probe ad-hoc-signed with **only** `com.apple.security.app-sandbox`, executed under real sandbox enforcement (confirmed: `home=~/Library/Containers/<id>/Data`). Baseline (unsandboxed) vs sandboxed were **identical — all passed**:

| Test | API | Sandboxed result |
|------|-----|------------------|
| Enumerate via directory read | `FileManager.contentsOfDirectory(/Applications, /System/Applications)` | ✅ 85 + 46 `.app`, clean list |
| Enumerate via Spotlight | `NSMetadataQuery` (`kMDItemContentType == com.apple.application-bundle`) | ✅ works but returns **10506** (includes every nested helper `.app`) — too noisy |
| Targeted lookup | `NSWorkspace.urlForApplication(withBundleIdentifier:)` | ✅ |
| Icon + PNG encode | `NSWorkspace.icon(forFile:)` → tiff → PNG | ✅ (but 1 icon = 1.49 MB raw; must downscale/cache) |
| Launch | `NSWorkspace.openApplication(at:configuration:)` | ✅ Calculator launched from sandbox |

**Conclusions:**
- **No private APIs needed.** Drop `LSApplicationWorkspace` entirely — all-public `NSWorkspace` + `FileManager` suffice. Lowest App Review risk.
- **No extra entitlements** beyond `app-sandbox` *for the app logic*. No temporary-exception, no file entitlements. (`/Applications` + `/System/Applications` are readable under sandbox.)
  - ⚠️ **Caveat found later (2026-08-29):** the spike had no WKWebView. The actual app also needs `com.apple.security.network.client` or the sandboxed webview renders blank. See 0008.
- **Enumerate by directory scan, not Spotlight** — Spotlight's 10506 is unusable noise; the folder scan yields the clean ~131-app list users expect.
- Icon raw size is large → extract, **downscale to ~64px, cache as PNG**; never in hot path (see below).

Spike code: throwaway, in session scratchpad (`scratchpad/spike/`), not committed.

Caveats still to confirm on a real MAS build: `~/Applications` read (not covered by spike; may need the folder to exist / a grant), and final App Review acceptance (low risk — public APIs + base sandbox only).

## Enumeration

Search standard locations:
- `/Applications`, `/System/Applications` (+ subfolders like `/System/Applications/Utilities`)
- `~/Applications`
- Setapp/other? (defer)

**Resolved approach (per spike): directory scan.** Enumerate `.app` bundles in the standard locations via `FileManager`. For each: read `Info.plist` for `CFBundleDisplayName`/`CFBundleName`, `CFBundleIdentifier`; get icon via `NSWorkspace.icon(forFile:)` (handles AssetCatalog/`.icns` uniformly — no manual icon parsing).

Do NOT use `NSMetadataQuery`/Spotlight for the primary list (10506 noisy nested results) and do NOT use `LSApplicationWorkspace` (private, review risk) — both rejected by the spike.

Rust side: call `NSWorkspace`/`NSFileManager` via `objc2` (`objc2-app-kit`, `objc2-foundation`). FFI to these public APIs is well-trodden; the sandbox-permission risk (the real unknown) is already cleared.

## Sandbox implications — CLEARED ✅ (see Spike results above)

- Directory read of `/Applications` + `/System/Applications`, icon extraction, and launch **all work under sandbox with only `com.apple.security.app-sandbox`**. No file/temporary-exception entitlements needed. This was the #1 project risk; it is retired.
- Remaining minor unknown: `~/Applications` read on a real MAS build (spike didn't cover). Handle gracefully if empty/denied.

## Launch

- `NSWorkspace.openApplication(at:configuration:)` — **confirmed working from sandbox** in the spike (async completion handler). No entitlement needed.

## Freshness

- Build index at startup (background thread) → cache in memory.
- Refresh on: manual reindex, TTL, or `NSWorkspace` app-install notifications if available under sandbox.

## Icon delivery to frontend

- Extract `NSImage` → **downscale to ~64px** → PNG → serve via Tauri asset protocol (cached) or base64 data-uri. Benchmark both; prefer asset protocol for large lists. (Raw icons are ~1.5 MB each — never ship full-res.)
- Pre-generate at index time; never in the keystroke path.

## Tasks

- [x] **SPIKE**: sandboxed build proving app-list + icons + launch. ✅ Done 2026-08-28 — all pass with base sandbox entitlement, public APIs only. Risk retired.
- [ ] Implement enumeration (`objc2` `NSFileManager` directory scan + `Info.plist` parse for name/bundle-id).
- [ ] Icon extraction (`NSWorkspace.icon(forFile:)`) + downscale to 64px + caching + delivery.
- [ ] `list_apps` + `launch_app` commands.
- [ ] Background index build + memory cache.
- [ ] Reindex trigger.
- [ ] Confirm `~/Applications` read on a real signed build.

## Risks

- ~~Sandbox blocking enumeration/launch~~ — **cleared by spike.**
- Icon extraction cost for 100+ apps — do once, cache, downscale.
- App Review acceptance: low risk (base sandbox + public APIs only), but only a real submission fully confirms.

## Unresolved questions

- ~~Which enumeration API~~ — **RESOLVED**: directory scan + public `NSWorkspace`. Spotlight/`LSApplicationWorkspace` rejected.
- Include `/System/Applications/Utilities` and other subfolders? (recommend yes, recurse one level into known subdirs)
- Include System apps, or curate out obscure ones? (recommend include all top-level `.app`, exclude nested helpers)
