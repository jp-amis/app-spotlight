# 0018 — Launcher first-open blink

## Problem

The **first** ⌘⇧Space open blinked; every open after that was smooth. Root cause: the
launcher window is created at its config height and the frontend resizes it to fit content
on first show — a one-time resize jump. The window keeps its size across `hide()`/`show()`,
so subsequent opens (already at content height) don't blink.

## Rejected approaches

- **Alpha-0 warm-up + hide** — warmed the JS but still blinked (the blink is the resize,
  not a cold webview).
- **Off-screen parking** (keep window ordered-in, move on/off screen) — user rejected: an
  always-present off-screen window can confuse window managers / Mission Control. `hide()`
  is the correct primitive.

## Fix (shipped)

Keep normal `hide()`/`show()`, and **pre-size the window to the last content height** so the
first open doesn't resize:

- Frontend `syncWindowHeight` persists the measured content height via a new
  `set_launcher_height(h)` command (deduped) whenever it resizes the window.
- At startup, `setup` reads the stored height (`stored_launcher_height`) and `set_size`s the
  (still-hidden) launcher to it. Verified: the async `set_size` applies within ~600 ms —
  long before any open — so the first show is already the right size.
- Default config height bumped 420 → 460 (closer to typical content) so even the very first
  run, before any height is stored, barely moves.

## Actual root cause (position, not resize)

Testing showed it wasn't a resize — it was the **window position** jumping. The launcher
window had `center: true` in config, so on first show it appeared centered, then
`position_launcher` moved it to the upper third (and `set_position` applies async, after the
window is already visible) → a visible reposition that reads as a blink.

Fix:
- Removed `center: true` from the launcher config.
- **Pre-position the hidden window at startup** (`position_launcher` in setup) so it's in
  the upper third before the first show. Verified: at startup the hidden window sits at
  x=3160, y=864 (h-centered, 20% down on the 7680×4320 display) — correct, so the first show
  is already placed.

The height pre-size (above) stays — both pre-size and pre-position apply asynchronously but
well before any open.

- Verified: 6 Rust + 13 TS tests pass; builds.
