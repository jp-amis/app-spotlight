# 0013 — Test round 1: fixes & permissions UX

Findings from the first hands-on sandboxed run (2026-08-29) and how to address each.
Issues 1–2 are bugs with known causes; 3 is a rename; 4 is a feature (permissions UX).

---

## Issue 1 — App icons not showing ✅ DONE (2026-08-29)

**Symptom:** rows show the grey placeholder, never the real app icon.

**Diagnosis (the plan's initial "deadlock" theory was WRONG — verified by logging):**
the `app_icon` command ran fine and the main-thread hop executed; `NSWorkspace.iconForFile`
returned a valid image. The failure was in **conversion**: we fed the icon's
`TIFFRepresentation` to the Rust `image` crate, but modern macOS icons are **16-bit
floating-point TIFF (sample format 3)** — which `image`'s TIFF decoder rejects
(`Unhandled TIFF sample format 3 for 16 bits`). It was also ~74 MB (all icon sizes in one
TIFF). So every icon returned `None`.

**Fix (implemented):** render the icon to PNG entirely in AppKit — draw the `NSImage` into
a 64×64 `NSBitmapImageRep` and call `representationUsingType:.png`. No `image` crate, no
TIFF decode; produces a clean ~5 KB 8-bit PNG per icon. Dropped the `image` dependency.
(`src-tauri/src/macos.rs::icon_png`.)

- [x] Rewrite `icon_png` to render via `NSBitmapImageRep` (main-thread, as before).
- [x] Verified sandboxed: icons produce ~5 KB PNGs for all sampled apps.
- [x] Removed the `image` crate + its now-obsolete unit test.

## Issue 2 — Last result is cropped ✅ DONE (2026-08-29)

**Symptom:** the bottom result row is clipped by the window edge.

**Cause:** the launcher window is a fixed `420px` tall (`tauri.conf.json`), but the content
(search header ~72px + up to `result_limit` rows × 48px) exceeds it — e.g. 8 rows = 384 +
72 = 456 > 420. The list is `overflow-hidden`, so the overflow is clipped, not scrolled.

**Fix options (pick one):**
- **A (preferred): size the window to content.** Compute height = header + rows×rowHeight
  (+ small padding) and `setSize` when results change, capped at a max; scroll beyond the cap.
  Gives the tight Spotlight look with no clipping.
- **B: make the list scrollable** (`overflow-y-auto`) and add bottom padding so the last row
  isn't flush to the edge. Simpler, but a partial row can still peek.
- **C: enlarge the fixed height** to fit `result_limit` rows. Brittle if the limit changes.

**Implemented (A):** the launcher container is now content-sized (no `h-screen`); the
results list is `max-h-[420px] overflow-y-auto`; a `createEffect` measures the root and
calls `getCurrentWindow().setSize()` whenever results change, so the window grows/shrinks
to fit and the list scrolls past the cap. Added `core:window:allow-set-size` capability.

- [x] Dynamic window height + scrollable list past the cap.
- [x] Fixed-height empty state.
- [ ] (Follow-up) fine-tune the max height / row metrics after visual review.

## Issue 3 — Rename to “My App Spot” ✅ DONE (2026-08-29)

Renamed the product from “Spotlight” to **“My App Spot”**; bundle id set to
**`co.amis.myappspot`**.

- [x] `tauri.conf.json`: `productName`, `mainBinaryName`, window `title`, `identifier`.
- [x] Tray label, settings window title + heading, `index.html` `<title>`.
- [x] Scripts (`sign.sh`, `pkg.sh`, `sandbox-run.sh`): `My App Spot.app` / `My App Spot.pkg`.
- [x] Verified the renamed bundle builds + launches sandboxed.
- [ ] (Follow-up) sweep remaining historical “Spotlight” mentions in older docs/README prose.

Original touch points below (kept for reference):

**Touch points:**
- `src-tauri/tauri.conf.json`: `productName`, `mainBinaryName`, window `title`.
- Bundle paths change (`My App Spot.app`) → update `scripts/*.sh` `APP=…` paths and the
  `codesign`/`open` targets.
- Tray tooltip/menu (`tray.rs`), settings window title + heading (`commands.rs`,
  `Settings.tsx`), `index.html` `<title>`, README, docs.
- Identifier stays `com.example.spotlight` until you set the real one (see 0011).

**Tasks:**
- [ ] Global rename of the display name + binary + bundle paths in scripts.
- [ ] Update docs/README references.

## Issue 4 — Not all apps show + sandbox permissions UX ✅ DONE (2026-08-29)

**Confirmed gap:** the real `~/Applications` (9 JetBrains/Claude apps here) is invisible
because `$HOME` is redirected to the sandbox container. Verified: `access_limited=true`,
`suggested=/Users/<you>/Applications`.

**Implemented:**
- Entitlements added: `files.user-selected.read-only` + `files.bookmarks.app-scope`
  (`entitlements.plist` + template). Verified embedded in the signed bundle.
- `permissions.rs` (objc2): `NSOpenPanel` folder picker, security-scoped **bookmark**
  create/resolve, `startAccessingSecurityScopedResource` (leaked for the session),
  `real_home()` to see past the container.
- Commands: `grant_folder`, `list_granted_folders`, `remove_granted_folder`,
  `access_status`. Bookmarks persisted in the store (`folder_bookmarks`), resolved at
  startup, and folded into `enumerate(extra_dirs)` + `reindex`.
- Launcher: dismissible amber banner when access is limited → “Grant Access” runs the
  picker, then reindexes (window height adjusts to fit the banner).
- Settings: “Folders & Permissions” section — lists granted folders with Remove, plus
  “Add Folder…”.

**Verified:** builds + launches sandboxed with all 4 entitlements; detection returns
`limited=true` so the banner triggers. The interactive grant (clicking through
`NSOpenPanel`) needs a manual click to confirm end-to-end — see checklist below.

_Original design notes:_

### Symptom — reference

**Symptom:** some installed apps are missing from the list.

**Likely causes:**
1. **Result cap.** The list shows only `result_limit` (default 8) rows; empty query shows
   the top 8. This is by design (type to filter) but can read as “missing apps”. Consider a
   scrollable full list and/or a higher default.
2. **`~/Applications` points at the sandbox container.** Under the sandbox, `$HOME` is
   `~/Library/Containers/<id>/Data`, so our `~/Applications` scan reads the *container’s*
   Applications (empty), not the real one. Apps installed in the user’s Applications folder
   are invisible.
3. **Locations outside the sandbox’s readable set** (e.g. a custom folder, Setapp) can’t be
   read without an explicit user grant.

`/Applications` and `/System/Applications` are readable without any grant (0005 spike), so
the common case works — but a launcher should surface + fix the gaps.

**Approach — permissions/onboarding UX (the user-requested part):**
- **Detect** when we likely can’t see everything (e.g. the real `~/Applications` exists but
  is unreadable from the sandbox).
- **In the launcher:** a dismissible banner — “Some apps may be hidden. My App Spot is
  sandboxed and needs your permission to see more folders. Grant access →” opening the grant
  flow (or Settings).
- **Grant flow:** `NSOpenPanel` to pick a folder → store a **security-scoped bookmark** →
  resolve it at launch (`startAccessingSecurityScopedResource`) to enumerate that folder.
  This is the sandbox-approved way to read user-chosen folders on MAS.
- **In Settings — “Folders & Permissions” section:** list the folders being scanned
  (built-in `/Applications`, `/System/Applications`) and any user-granted folders; let the
  user **add** a folder (grant) and **remove** one (drop the bookmark). Show which are
  readable vs. blocked.

**Entitlements:** `com.apple.security.files.user-selected.read-only` (for the open-panel
grant) + the existing `app-sandbox`, `network.client`. Security-scoped bookmarks need
`com.apple.security.files.bookmarks.app-scope`.

**Tasks:**
- [ ] Confirm the real `~/Applications` is unreadable under sandbox; measure the gap.
- [ ] Add the two file entitlements to `entitlements.plist` + template.
- [ ] Rust: `NSOpenPanel` grant command + security-scoped bookmark store/resolve; include
      granted folders in `enumerate()`.
- [ ] Frontend: launcher banner when access is limited; Settings “Folders & Permissions”
      manager (list/add/remove).
- [ ] Persist bookmarks in the store; re-resolve on startup.
- [ ] Decide result cap vs. scrollable full list (ties to Issue 2).

**Risks:** App Review scrutinizes `files.user-selected`; keep the grant user-initiated and
clearly explained. Bookmarks can go stale (app moved) — handle resolve failures gracefully.

---

## Issue 5 — Settings window chrome ✅ DONE (2026-08-29)

From a screenshot test: the settings window had a **light native titlebar** clashing with
the dark content, and the content was **cropped** (fixed 560px, no scroll).

**Fixed:**
- Transparent full-size titlebar + hidden native title (`TitleBarStyle::Transparent`,
  `hidden_title`) so the bar blends into the content; a thin top drag strip preserves
  window dragging. Verified: traffic lights now sit on a dark unified bar.
- Body is `h-screen overflow-y-auto` so all sections scroll instead of clipping.
- Window stays **resizable**. The white flash during resize was root-caused: the shared
  `app.css` forces `body { background: transparent }` for the launcher's vibrancy, so the
  settings body was transparent too → live-resize lag exposed the white window backing.
  Fixed by making the settings window **opaque**: a `.settings-window` body class
  (added in `main.tsx`) sets a solid neutral-50/neutral-900 background, plus an opaque
  dark window `background_color`. Resizing no longer flashes white.

**Later polish (2026-09-01):**
- **Native-style title** — a small, centered, muted title sits in the draggable title-bar
  strip (replacing the big in-content `h1`).
- **Sticky tabs** — layout is `flex-col`: the title bar + tab bar are fixed (`shrink-0`)
  and only the content region (`flex-1 overflow-y-auto`) scrolls.
- **Title on the traffic-light row + bold** — switched the window to
  `TitleBarStyle::Overlay` (full-size content view) so our content starts at y=0; the title
  bar is now `h-[28px]` with a **bold** centered title, aligning with the traffic lights,
  and the tabs sit right beneath it (tight to the top). Window height raised to 760.
- **Width locked, height-only resize** — `min_inner_size` and `max_inner_size` share the
  same width (980), so only height resizes. General tab now spans the full content width.
- **Persisted size** — the window height is saved to the store on resize and restored on
  the next `open_settings` (verified: seeded 600 → reopened at 600). Width is fixed.

## Suggested order

1. Issue 1 (icons) — tiny, high impact.
2. Issue 2 (cropping) — small layout fix.
3. Issue 3 (rename) — mechanical.
4. Issue 4 (permissions UX) — larger; do after 1–3.

## Unresolved questions

- Issue 3: exact casing/identifier — “My App Spot” display + keep `com.example.spotlight`, or set a real bundle id now?
- Issue 4: ship the full permissions UX now, or first just fix the `~/Applications` gap + make the list scrollable, and defer folder-granting?
- Result list: keep top-N (type to filter) or make it a scrollable full list?
