# 0015 — Settings: Index & Permissions area

A dedicated settings area that surfaces the index: live indexing status/progress, the
folders being scanned (built-in + granted, with per-folder counts + stale state), and a
detailed, filterable view of every app that got indexed.

## Goal

- **Status:** show when the index is (re)building — a spinner + "Indexing…", and when idle
  "N apps · updated Xs ago", plus a manual "Reindex now".
- **Permissions:** a clear list of every scanned location — system (read-only) vs. granted
  (removable) — each with how many apps it contributed and whether it's currently readable.
- **Detailed view:** an expandable, filterable list of all indexed apps (name, bundle id,
  path).

## Backend

- `AppState`: `indexing: AtomicBool`, `last_indexed_ms: Mutex<Option<u64>>`.
- `apps::enumerate_with(extra, on_dir)` — a progress-reporting variant; `enumerate` wraps it.
- `reindex()` emits lifecycle events (from any thread):
  - `index:start`
  - `index:progress` `{ dir, found }` per scanned directory
  - `index:done` `{ count }`
  and always updates `indexing` + `last_indexed_ms` (even on a no-op diff, so "last
  updated" reflects the last *check*). Still only swaps state / emits `apps:reindexed`
  when the list actually changed.
- Commands:
  - `index_status()` → `{ indexing, count, last_indexed_ms, dirs: [{ path, kind, count, readable }] }`
    where `kind` ∈ system|granted and `count` = apps whose path is under that dir.
  - `reindex_now()` → triggers `reindex` (button + parity with the tray).

## Frontend (Settings)

Replace the current "Folders & Permissions" section with an **"Index & Permissions"** block:
- Status row: spinner/"Indexing…" (live via events) or "157 apps · updated 3s ago" +
  **Reindex** button.
- Scanned locations: one row per dir — path, a `system`/`granted` badge, app count, and a
  "not readable" warning for stale grants; granted rows have **Remove**; a footer **Add
  Folder…**.
- Detailed apps: a collapsible "Show indexed apps (157)" with a filter box and a scrollable
  list (name · bundle id · path). Icons deferred (lazy, heavy for long lists).

Live updates: the settings window listens to `index:start/progress/done` + `apps:reindexed`
and refreshes `index_status` + the app list.

## Tasks ✅ DONE (2026-08-29)

- [x] `AppState` status fields (`indexing`, `last_indexed_ms`) + `enumerate_with` +
      event-emitting `reindex` (`index:start/progress/done`).
- [x] `index_status` + `reindex_now` commands.
- [x] Settings "Index & Permissions" UI: live status (spinner / "N apps · updated Xs ago"
      + Reindex), per-folder locations with `system`/`granted` badges + counts + "no access"
      warning + Remove/Add, and a collapsible filterable detailed app list.
- [x] Verified via screenshot: section renders with "157 apps · updated just now" + Reindex;
      builds clean; 3 Rust + 13 TS tests pass.

## Risks / notes

- Enumeration is fast, so progress may flash — still useful for large `~/Applications` or
  many grants; the spinner + last-updated give steady feedback regardless.
- Detailed list can be long; virtualize only if it becomes a problem (cap/scroll first).

## Unresolved questions

- Show icons in the detailed list (lazy) or keep it text-only for speed?
- Tabs vs. one long scroll for settings as it grows? (one scroll for now.)
