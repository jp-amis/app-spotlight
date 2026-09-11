# 0001 — Project setup & tooling

## Goal

Scaffold the Tauri v2 + SolidJS repo with `mise`-managed tools and task scripts, ready for MAS.

## Context

- Empty repo (only README). Fresh scaffold, no legacy to migrate.
- Reference app uses Tauri v2 + Vite; we swap Vue → SolidJS.
- `mise` already installed (`/opt/homebrew/bin/mise`, v2025.12.9). Rust 1.94, cargo present. No node yet — mise provides it.

## Repo layout

```
app-spotlight/
  mise.toml                 # tool versions + tasks
  docs/plans/               # this
  src/                      # SolidJS frontend
  src-tauri/                # Rust/Tauri backend
    tauri.conf.json
    Cargo.toml
    entitlements.plist
    Info.plist
    icons/
    src/
      main.rs
      lib.rs
      launcher/             # window, shortcut, app-index modules
  index.html
  vite.config.ts
  package.json
  tailwind.config.js
  tsconfig.json
```

Single-crate backend to start (reference splits into workspace crates; we defer that until a macOS-native crate is actually needed — see 0003).

## Tooling (`mise.toml`)

Pin: `node` (LTS, e.g. 22), `rust` (stable, matches installed 1.94), `pnpm`. Tasks wrap common flows:

- `mise run dev` → `pnpm tauri dev`
- `mise run build` → universal build (see 0010)
- `mise run icon` → generate `.icns` from 1024 png (sips/iconutil, port reference `build-icon.sh`)
- `mise run sign` / `mise run pkg` → MAS signing (see 0008)
- `mise run fmt` / `mise run lint`

## Frontend

- `create-tauri-app` is not used directly (Solid template differs); scaffold manually or via `pnpm create vite --template solid-ts` then add Tauri.
- Deps: `@tauri-apps/api`, `solid-js`, `vite-plugin-solid`, `tailwindcss`, `@tauri-apps/plugin-store`, `@tauri-apps/plugin-global-shortcut`.

## Tauri baseline (`tauri.conf.json`)

Mirror reference where sensible:
- `macOSPrivateApi: true` (needed for vibrancy + NSPanel tricks).
- `withGlobalTauri` off (Solid imports the API directly — smaller/cleaner).
- `bundle.category: "Productivity"`, `minimumSystemVersion: "13.0"` (Ventura+; revisit).
- `identifier`: TBD (see questions).

## Tasks

- [ ] Write `mise.toml` (tools + tasks).
- [ ] Scaffold Vite + Solid frontend.
- [ ] `pnpm tauri init` → `src-tauri`, then port config to SolidJS dist path.
- [ ] Add `.gitignore` (target, dist, node_modules, .DS_Store).
- [ ] Verify `mise run dev` opens a window.
- [ ] Commit as Git Flow feature branch.

## Risks

- Tauri Solid template drift — may need manual `vite.config.ts` glue.
- Node in sandbox not relevant (dev-only), but pnpm store path must not leak into bundle.

## Unresolved questions

- Bundle identifier? (need Apple Team ID + reverse-domain, e.g. `com.<you>.spotlight`)
- Node major: 22 LTS ok?
- Min macOS: 13.0 ok, or need 12/11?
