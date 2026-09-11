# 0002 — Architecture overview

## Goal

Define the process/module structure and data flow so later plans slot in cleanly.

## Components

1. **Launcher window** (frontend, SolidJS) — the Spotlight-like palette. Borderless, centered, appears/hides instantly. Renders the app list + search input.
2. **Settings window** (frontend, same bundle, separate window/route) — shortcut config, menu-bar toggle, appearance.
3. **Rust backend** (`src-tauri`):
   - `launcher/window.rs` — create/show/hide the NSPanel-style launcher window.
   - `launcher/shortcut.rs` — global shortcut register/unregister + rebind.
   - `launcher/apps.rs` — enumerate installed apps, resolve icons, launch.
   - `launcher/tray.rs` — menu-bar icon + menu.
   - `settings.rs` — persisted config via `tauri-plugin-store`.
   - `commands.rs` — `#[tauri::command]` surface exposed to frontend.

## Data flow (open → launch)

```
global shortcut fires (Rust)
  → show launcher window, focus input, emit "launcher:opened"
frontend requests app list (once, cached) via invoke("list_apps")
  → Rust returns [{name, path, bundle_id, icon (data-uri/asset)}]
user types → frontend filters in-memory (fuzzy) → renders top N
Enter → invoke("launch_app", path) → Rust NSWorkspace open → hide window
Esc / blur → hide window
```

## State & persistence

- Config store (`tauri-plugin-store`, JSON in app-support): shortcut, menubar-enabled, dock-hidden, theme, result-limit.
- App index cached in memory (Rust) + optional on-disk cache to make first paint instant; invalidate on app install/remove (filesystem watch or TTL).

## Performance principles

- App list built once at startup (background), not per-open.
- Filtering is client-side over a preloaded array — no IPC per keystroke.
- Icons pre-decoded to a served asset or cached data-uri; avoid per-keystroke icon work.
- Window is created once and hidden/shown (not recreated) to keep open latency < a few ms.

## Key macOS integration decisions

- Launcher must be a **non-activating panel** so invoking it doesn't steal focus from / reorder the frontmost app, and so it floats above fullscreen apps. Plain Tauri windows activate the app. → likely need `tauri-plugin-nspanel` or a small native crate (see 0003).

## Tasks

- [ ] Lock module boundaries above.
- [ ] Define the `commands.rs` IPC contract (list_apps, launch_app, get/set_settings, set_shortcut).
- [ ] Decide icon delivery (asset protocol vs data-uri) — benchmark in 0005.

## Unresolved questions

- ~~On-disk app-index cache in v1, or memory-only?~~ **RESOLVED (0014):** disk cache
  (`app-index.json`) for instant startup + background refresh.
- ~~Filesystem watch for app install/remove, or TTL refresh + manual "reindex"?~~
  **RESOLVED (0014):** event-driven `notify`/FSEvents watch, debounced. Manual reindex
  still available in the tray.
