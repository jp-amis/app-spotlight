# Plans Index

Docs-first workflow. Every change gets a numbered plan here before implementation. Update the Status column as work progresses.

## Locked decisions

- **Platform**: Tauri v2, macOS-only.
- **Distribution**: Mac App Store only (sandboxed, provisioning profile, Apple Distribution signing).
- **Frontend**: SolidJS + Vite + Tailwind (chosen for lowest keystroke latency).
- **Scope (v1)**: App launcher only — enumerate + launch installed apps. No file/content indexing.
- **Default shortcut**: `cmd+shift+space`, fully rebindable. Ships with a guide to reclaim `cmd+space` from Spotlight.
- **Menu bar**: On by default (tray icon + optional dock hiding).
- **Tooling**: `mise` for tool versions + task scripts.
- **Keyboard-first**: Full keyboard operability is a hard requirement; mouse is secondary.

## Plans

| #    | Plan | Status |
|------|------|--------|
| 0000 | [Index](0000_index.md) | living |
| 0001 | [Project setup & tooling](0001_project_setup_and_tooling.md) | done ✅ |
| 0002 | [Architecture overview](0002_architecture_overview.md) | done ✅ |
| 0003 | [Launcher window (NSPanel)](0003_launcher_window_nspanel.md) | done ✅ (see limitations) |
| 0004 | [Global shortcut](0004_global_shortcut.md) | done ✅ |
| 0005 | [App indexing & launch](0005_app_indexing_and_launch.md) | done ✅ |
| 0006 | [Search UX & keyboard nav](0006_search_ux_keyboard.md) | done ✅ |
| 0007 | [Settings & menu bar](0007_settings_and_menubar.md) | done ✅ |
| 0008 | [Sandbox & MAS distribution](0008_sandbox_and_mas_distribution.md) | done ✅ (unsigned; needs Apple certs) |
| 0009 | [Styling & theming](0009_styling_and_theming.md) | done ✅ |
| 0010 | [Build, release & CI](0010_build_release_ci.md) | done ✅ (CI deferred) |
| 0011 | [App Store build guide (runbook)](0011_app_store_build_guide.md) | ready to run ▶️ |
| 0012 | [Local testing & signing certs](0012_local_testing_and_certs.md) | ready to run ▶️ |
| 0013 | [Test round 1: fixes & permissions UX](0013_test_round1_fixes.md) | done ✅ (grant flow needs manual click test) |
| 0014 | [Index disk cache + folder watch](0014_index_cache_and_watch.md) | done ✅ |
| 0015 | [Settings: Index & Permissions area](0015_settings_index_permissions.md) | done ✅ |
| 0016 | [Test round 2 fixes](0016_test_round2_fixes.md) | done ✅ |
| 0017 | [Index tab: search + enable/disable](0017_index_disable_and_search.md) | done ✅ |
| 0018 | [Launcher warm-up (no first-open blink)](0018_launcher_warmup.md) | done ✅ |
| 0019 | [Remember window positions](0019_window_position_persistence.md) | done ✅ |

Status legend: `draft` → `approved` → `in-progress` → `done` (+ `living` for docs that stay open).

## Implementation & testing status (2026-08-28)

**Built.** Full Tauri v2 + SolidJS app compiles and runs. `mise` manages node/pnpm + tasks.
Backend modules: `apps` (dir-scan enumeration), `macos` (objc2 icon/launch/window), `commands`,
`shortcut`, `tray`, `lib` (setup, vibrancy, accessory policy). Frontend: launcher palette
(fuzzy search, keyboard nav, lazy icons) + settings window (shortcut capture, toggles, theme).

**Automated tests — all green:**
- Rust: `apps` enumeration (finds Calculator, sorted/unique) + `macos::tiff_to_png` pipeline. `cargo test` → 3 passed.
- TS (vitest): `fuzzy` matcher/ranking/frecency + `accelerator` builder/formatter. → 13 passed.
- Build: `cargo check`/`cargo test` clean (no warnings); `pnpm build` (tsc + vite) clean; `tauri build` produces the app binary.
- Smoke: built binary launches, registers the global shortcut, and stays alive with no errors.

**Not automatable here (documented manual checklist):**
- Interactive GUI (press hotkey → panel appears → type → Enter launches) needs Accessibility grant + a display; AppleScript keystroke injection hangs on the permission prompt headlessly. See `README.md` → Manual verification.
- MAS signing/upload needs a real Apple Team ID + Distribution/Installer identities + provisioning profile (not present in this environment). Scripts are ready (`scripts/sign.sh`, `scripts/pkg.sh`).

## Known limitations / follow-ups

- **Launcher window** floats via high window level + all-Spaces collection behavior + accessory activation policy (no dock). True `NSPanel` *non-activating* conversion (à la `tauri-nspanel`) is a future refinement; current approach gets Spotlight-like behavior without the private class-swizzle.
- **Multi-monitor**: positions on the current monitor's upper third; "screen under the cursor" is a refinement.
- **Icons**: extracted on the main thread (AppKit requirement) and cached; downscaled to 64px.
- **CI**: not set up (local build/test only) — see 0010.

## Key findings

- **2026-08-28 — sandbox risk retired (0005 spike):** app enumeration (directory scan), icon extraction, and app launch all work under real App Sandbox with **only** the base `com.apple.security.app-sandbox` entitlement, using **public `NSWorkspace` + `FileManager`** — no private `LSApplicationWorkspace`, no temporary-exception entitlements. This was the #1 project risk; the MAS approach is validated. Enumerate by directory scan (clean ~131-app list), not Spotlight (10506 noisy results).
- **2026-08-29 — webview needs `network.client`:** the sandboxed app first rendered an empty window because WKWebView requires `com.apple.security.network.client` to load even the *local* frontend. Added to entitlements; app makes no real network calls. (The spike only covered enumerate/icon/launch, not the webview.)

## Conventions

- Filenames: `NNNN_snake_case_title.md`.
- Each plan: Goal, Context, Approach, Tasks (checkboxes), Risks, Unresolved questions.
- Reference app (patterns only, not gospel): `/Volumes/Storage/Projects/multi-time/MultiTimeApp`.
