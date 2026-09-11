# Spotlight

A fast, keyboard-first app launcher for macOS — a Spotlight replacement (⌘⇧Space by
default) that shows and launches your installed apps. Tauri v2 + SolidJS, sandboxed for
the Mac App Store.

Design docs and decisions live in [`docs/plans/`](docs/plans/0000_index.md).

## Prerequisites

- [mise](https://mise.jdx.dev) (manages node + pnpm)
- Rust toolchain (`cargo`)
- Xcode Command Line Tools

```sh
mise install          # node + pnpm
pnpm install          # or: mise exec -- pnpm install
```

## Develop

```sh
mise run dev          # tauri dev (hot-reloading frontend + backend), unsigned
```

To test the app **sandboxed** locally (no Apple Developer Program needed):

```sh
cp .env.local.example .env.local   # optional: set APPLE_DEV_IDENTITY (else ad-hoc)
mise run sandbox                   # build → codesign w/ app-sandbox → launch
```

## Test

```sh
mise run test         # cargo test + vitest
```

- Rust: app enumeration + icon pipeline (`src-tauri/src/{apps,macos}.rs`).
- TS: fuzzy matcher + accelerator parsing (`src/lib/*.test.ts`).

## Build

```sh
mise run build        # universal (aarch64 + x86_64) release .app
```

## Sign & package for the Mac App Store

Requires an Apple Developer account. Put all your values in one git-ignored file:

```sh
cp .env.mas.example .env.mas      # then fill TEAM_ID, BUNDLE_ID, signing identities
# drop your MAS provisioning profile at src-tauri/embedded.provisionprofile
mise run release                  # build → sign → pkg  →  Spotlight.pkg
```

The scripts read `.env.mas` and generate the identifier override + entitlements
automatically — no hand-editing of config files. Full walkthrough (certs, profiles,
App Store Connect, upload): [`docs/plans/0011_app_store_build_guide.md`](docs/plans/0011_app_store_build_guide.md).

## Manual GUI verification

Automated tests cover the logic; the interactive flow needs a real session
(and, for the global hotkey, an Accessibility grant on first run):

1. `mise run dev`. The app has no dock icon — look for the menu-bar icon.
2. Press **⌘⇧Space** → the launcher panel appears centered, focus in the search field.
3. Type a few letters → results filter instantly; **↑/↓** (or **⌃P/⌃N**) move the selection.
4. **Enter** → the selected app launches and the panel hides.
5. **Esc** clears the query, then (pressed again) hides the panel. Clicking away also hides it.
6. Menu-bar icon → **Settings…** → rebind the shortcut, toggle menu-bar/dock, change result count.
7. Want ⌘Space? Disable Spotlight's shortcut in System Settings → Keyboard → Keyboard
   Shortcuts → Spotlight, then rebind here.

## Project layout

```
mise.toml              tool versions + tasks
index.html             frontend entry
src/                   SolidJS frontend
  launcher/            the ⌘⇧Space palette
  settings/            settings window
  lib/                 fuzzy matcher, accelerator, typed API
src-tauri/             Rust / Tauri backend
  src/apps.rs          app enumeration (dir scan + Info.plist)
  src/macos.rs         objc2: icon extraction, launch, window tuning
  src/commands.rs      IPC surface + settings store
  src/{shortcut,tray}.rs
  entitlements.plist   app-sandbox only (proven sufficient — see plan 0005)
scripts/               icon generation, sign, pkg
```
