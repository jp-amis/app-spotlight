# 0019 — Remember window positions (launcher + settings)

## Goal

Reopen the launcher and settings windows where the user last left them — but only if that
spot is still on a connected monitor; otherwise fall back to the default placement.

## Approach

Shared helpers (in `commands.rs`), storing physical px in the settings store:
- `save_window_pos(key, x, y)` / `load_window_pos(key)`.
- `pos_on_connected_monitor(win, x, y)` — true if (x,y) is inside any
  `available_monitors()` rect ("same monitor" still present).

**Launcher** (`launcher_pos`):
- `hide_launcher` (incl. blur-hide, now routed through the command) saves `outer_position()`.
- `place_launcher` = restore the saved position if on a connected monitor, else
  `position_launcher` (upper third of active/primary). Used both at startup pre-position and
  on every show.

**Settings** (`settings_pos`):
- On `Moved`, save the position (alongside the existing height-on-`Resized`).
- On open, restore the saved position if on a connected monitor.

## Per-monitor (revised)

Positions are stored **keyed by monitor** so a window opens on the monitor the user is
currently using, not wherever it last sat. Store shape:
`launcher_pos` / `settings_pos` -> `{ "<origin x>x<origin y>": {x,y}, ... }`.

- **Active monitor** = the one under the cursor (`cursor_position` → `monitor_from_point`),
  else primary.
- **Place** = if there's a saved position for the *active* monitor (and it's inside it),
  use it; else the active monitor's default (launcher: upper third; settings: OS-centered).
- **Save** = keyed by the window's *current* monitor (launcher on hide/blur; settings on
  `Moved`).

## Verified (runtime)

- Saved a position only for the primary, but the cursor was on the secondary → launcher
  opened on the **secondary** (default upper third), ignoring the primary's saved spot.
- Saved a position for the active (secondary) monitor → launcher used it.
- Off-monitor / stale coords → fall back to default.
- 6 Rust + 13 TS tests pass; builds.

## Fixes during rollout

- **Key mismatch** — save used `current_monitor()` while load used `monitor_from_point()`;
  those can return different `position()`s (or `current_monitor()` is None mid-hide), so
  keys never matched → nothing restored. Now both resolve the monitor the same way
  (`monitor_containing` scans `available_monitors`): save by the window's center, load by
  the cursor. Verified round-trip restores exactly.
- **Show-at-old-position flash** — on open the window showed before the async `set_position`
  landed, flashing at its previous spot. Fixed by deferring `show()`/`set_focus()`/`emit`
  to a `run_on_main_thread` closure enqueued *after* the reposition, so the move is applied
  first. Verified: position-at-show-time is the new target, not the old spot.
