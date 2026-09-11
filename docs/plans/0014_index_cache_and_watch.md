# 0014 — Index disk cache + lightweight folder watch

Make the app index (a) survive restarts so we don't rebuild from scratch on every
launch, and (b) refresh automatically when apps are installed/removed — event-driven,
not polling.

## Goal

- **Instant first paint:** load a cached index from disk at startup, then refresh in the
  background.
- **Auto-fresh:** watch the app folders with FSEvents (via `notify`) and reindex on
  change, debounced. No manual "Reindex Apps" needed for normal install/remove.

## Approach

### Disk cache
- Serialize the `Vec<AppEntry>` to `app-index.json` in the app cache dir
  (`app.path().app_cache_dir()`, inside the sandbox container).
- Startup: if the cache exists, load it → set the index immediately (no enumerate on the
  critical path). Then reindex in a background thread. If no cache (first run), enumerate
  synchronously once and write the cache.
- `reindex()` writes the cache after a successful rebuild, and **only** updates state +
  emits `apps:reindexed` when the list actually changed (so cached-vs-fresh with no diff
  is a no-op — no frontend churn).

### Watcher
- `notify` recommended watcher on the scanned dirs (standard locations + granted folders),
  non-recursive (installs/removes are direct children).
- Manual debounce: on the first event, drain further events until ~800 ms of quiet, then
  call `reindex()` once. The watcher is kept alive on its own thread.
- Under the sandbox, FSEvents on readable paths needs no extra entitlement; granted folders
  are already access-started via their bookmarks.

## Changes

- `AppEntry`: add `Deserialize` + `PartialEq`.
- `apps.rs`: `dirs_to_scan(extra)` helper (shared by enumerate + watcher); `load_cache` /
  `save_cache`.
- `AppState`: add `cache_path: Option<PathBuf>`.
- `lib.rs`: cache-first startup, background refresh, `start_watcher`, change-aware `reindex`.
- `Cargo.toml`: add `notify`.

## Tasks ✅ DONE (2026-08-29)

- [x] `AppEntry` derives (`PartialEq`, `Deserialize`) + `dirs_to_scan` + `load_cache`/`save_cache` (+ round-trip test).
- [x] Cache-first startup + background refresh thread.
- [x] Change-aware `reindex` (diff → update/save/emit only on change).
- [x] `notify` watcher with 800 ms debounce on its own thread.
- [x] **Verified:** run 1 builds+writes (157), run 2 loads from cache instantly (157);
      adding `DummyWatch.app` → auto `reindex → 158`, removing → `→ 157`. 3 Rust + 13 TS tests pass.

## Risks / notes

- Cache can go stale vs. reality between launches — the background refresh + watcher fix it
  within a second of open. Acceptable.
- FSEvents under sandbox for granted folders relies on the active security scope; if a
  bookmark fails to resolve, that folder just isn't watched.

## Unresolved questions

- Cache invalidation on app version bump? (Format is simple; a stale cache self-heals on
  the next background reindex, so probably unnecessary.)
