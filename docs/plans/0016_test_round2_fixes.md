# 0016 — Test round 2 fixes

From the second hands-on pass.

## Issue 1 — App name should fall back to the file name

`make_entry` already falls back to the `.app` file stem when there's no
`CFBundleDisplayName`/`CFBundleName`, but only for a *missing* key — an empty/whitespace
name slips through. Harden: treat blank names as missing and use the file stem.

## Issue 2 — Bigger settings window, two columns, richer per-folder info

The single narrow column is cramped now that Index & Permissions carries more. Make the
window larger and lay it out in two columns:
- **Left:** Shortcut, General, Appearance.
- **Right (wider):** Index & Permissions.

Enrich each folder row: keep path + `system`/`granted` badge + app count + readability +
Remove/Add, and make a row **expandable** to list the apps found *in that folder*
(client-side filter of the index by path prefix).

## Issue 3 — Don't list the container's `~/Applications` as "no access"

Under the sandbox, `$HOME` is the app container, so `dirs_to_scan` was adding
`<container>/Data/Applications` — a useless path that shows up "no access". Fix at the
source: **skip `~/Applications` when `$HOME` is a container** (contains
`/Library/Containers/`); the real `~/Applications` only comes via a grant. Also, in the
status breakdown, hide **system** locations that aren't readable (e.g. a missing
`/Applications/Utilities`). Granted folders stay visible even when unreadable, so a stale
grant can be removed.

## Tasks ✅ DONE (2026-08-29)

- [x] `make_entry`: blank/whitespace name → file stem.
- [x] `dirs_to_scan`: skip `~/Applications` when `$HOME` is a sandbox container.
- [x] `index_status`: drop unreadable **system** dirs (granted stay, for removal).
- [x] Settings: two-column layout (left: Shortcut/General/Appearance; right: Index &
      Permissions), larger window (900×700, resizable), expandable per-folder app lists +
      "Show all indexed apps".
- [x] **Verified (backend, via runtime diag):** breakdown now lists only the 4 readable
      system dirs — no container `.../Data/Applications`; `blank-named apps = 0`.
      Frontend builds + typechecks; 3 Rust + 13 TS tests pass.

## Follow-up (round 2b)

The two-column layout felt empty/weird, so it was replaced with **tabs**:
- **General** tab: Shortcut, General, Appearance.
- **Index & Permissions** tab: status + folders (expandable) + a searchable **All indexed
  apps** list that now shows **icons** (reuses the launcher's `AppIcon`).
- Default window widened to **980×640**.

Builds + typechecks; 3 Rust + 13 TS tests pass; smoke-launches. Visual confirmation of the
exact layout is on the user's side (dual-4K desktop defeats reliable auto-screenshots here).
