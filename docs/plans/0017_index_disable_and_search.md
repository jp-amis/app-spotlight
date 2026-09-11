# 0017 — Index tab: per-folder search + enable/disable

Refinements to the Index & Permissions tab.

1. **Drop the "All indexed apps" section** — redundant now that folders expand to their apps.
2. **Search** — one box that filters apps; matching folders auto-expand and show only
   matches (results organized by folder).
3. **Enable/disable (not remove)** — toggle a whole folder or an individual app out of the
   launcher's results without revoking the grant/index. Disabled items stay visible in
   settings (dimmed) so they can be re-enabled.
4. **Taller folder detail** — the expanded per-folder app list gets more height.

## Backend

- Persist two sets in the store: `disabled_folders` (paths) and `disabled_apps` (paths).
- `app_enabled(path)` = path not in `disabled_apps` **and** not under any `disabled_folder`.
- `list_apps` (launcher) now returns **enabled apps only**.
- New `indexed_apps()` → every app + an `enabled` flag (for settings, which must show
  disabled ones too).
- `index_status` `DirInfo` gains `disabled: bool`.
- New commands `set_app_disabled(path, disabled)` / `set_folder_disabled(path, disabled)`;
  both persist and emit `apps:reindexed` so the launcher re-filters immediately.

## Frontend (settings)

- Remove the All-indexed-apps block.
- A search box at the top of the folders area; when non-empty, auto-expand folders and show
  only matching apps (hide folders with no matches).
- Folder row: an enable/disable checkbox (dim + strike when disabled) alongside the existing
  badge/count/readable/Remove.
- App row: a small enable/disable toggle.
- Bigger max-height on the expanded per-folder lists.

## Tasks ✅ DONE (2026-09-01)

- [x] Backend: `disabled_apps`/`disabled_folders` sets, `app_enabled`, filtered
      `list_apps`, `indexed_apps` (+ `enabled`), `DirInfo.disabled`, `set_app_disabled` /
      `set_folder_disabled` (persist + emit `apps:reindexed`).
- [x] Frontend: removed the global "All indexed apps" list; one search box that filters apps
      and auto-expands folders to their matches; folder + per-app enable/disable checkboxes
      (dim + strike-through when off); expanded lists taller (`max-h-[26rem]`).
- [x] Tests: 3 new `app_enabled` unit tests (default on, individual disable, folder-prefix
      disable with a prefix-collision guard). 6 Rust + 13 TS pass; builds + smoke-launches.

Note: the launcher re-filters immediately because toggles emit `apps:reindexed`, which the
launcher already listens to.

## Follow-up refinements (2026-09-01)

Backend disable path verified end-to-end (runtime diag: disabling Calculator removed it from
`list_apps` and flipped `indexed_apps` `enabled=false`). Frontend feedback/UX fixes:

1. **Single-app disable "not working"** was a feedback problem — the count never changed, so
   it looked inert. Fixed by (3) below; the row also dims + strikes through now.
2. **Live counts** — the status count now derives from the live app list, so it updates the
   instant you toggle.
3. **Show total incl. disabled** — status reads "N of TOTAL apps · X off".
4. **Taller detail** — expanded per-folder list `max-h-[26rem]` → `max-h-[32rem]`.
5. **Add Folder moved to top** — now inline with the search box, above the folder list.
6. **No empty rows on search** — folders with zero matches are hidden entirely (was a
   "No matches" row).
7. **No folder-level disable while searching** — the folder on/off checkbox is hidden during
   search (you're only seeing a partial folder); per-app toggles remain. The count shown on
   a folder also switches to the match count while searching.

### Bug fix (2026-09-01) — single-app disable did nothing

Real logic bug (backend was fine): `toggleApp` called `setAppDisabled(path, !enabled)`, but
`enabled` was already the *current* state — for an enabled app that set `disabled=false`
(a no-op). Fixed to `setAppDisabled(path, enabled)` (currently-enabled → disable). Added
`console.log`s to `toggleApp`/`toggleFolder` for inspection.

Also: the "updated Xs ago" now bumps on a toggle — `set_app_disabled`/`set_folder_disabled`
update `last_indexed_ms` (the effective list changed), and settings re-reads status after a
toggle → shows "updated just now".

### Folder/app interaction (2026-09-01)

- **Enabling an app inside a disabled folder auto re-enables the folder** — `toggleApp`
  now takes the folder; when it's enabling an app whose folder is off, it also clears the
  folder's disabled flag.
- **Disabling a folder never disables its apps individually** — folder off only writes
  `disabled_folders`; per-app `disabled_apps` is untouched, so re-enabling the folder
  restores each app's previous preference. Verified at runtime: disable app → disable its
  folder → re-enable folder → the app stays disabled.

### Bulk check/uncheck (2026-09-01)

Per-folder **"Check all" / "Uncheck all"** buttons (in the expanded folder header) toggle
every currently-shown app at once via a new bulk `set_apps_disabled(paths, disabled)`
command (one store write + one event). "Check all" also re-enables the folder if it was
off. When searching, the buttons act on the visible matches.
