# App Store screenshots

`mise run screenshots` (→ `scripts/gen-store-screenshots.py`) builds 2560×1600
marketing images: a benefit headline + branding over the brand gradient, with the
launcher in the middle.

By default the launcher is a **vector mock**. To use a **real capture** (so the
Liquid Glass is authentic), drop a PNG named after the shot's slug into
`captures/` and re-run — the generator composites it in place of the mock.

Slugs: `hero-launch`, `fuzzy-search`, `favorites`, `keyboard`, `private`
→ e.g. `assets/store/captures/hero-launch.png`.

## Optimal state for each screenshot

Set the launcher to the state below before capturing, so the real window matches
the message of its frame. (The headline and any badge are drawn by the generator;
you only need the launcher itself in the right state.)

| Slug (`captures/<slug>.png`) | Headline it sits under | Put the launcher in this state |
|---|---|---|
| `hero-launch` | Open any app in a keystroke | **Empty query.** Have 3+ favorites pinned so the top rows show the ★ and Cmd 1-0 badges. Top row highlighted. This is the hero, so keep it clean and recognizable. |
| `fuzzy-search` | Fuzzy search that keeps up with you | **Type a short, non-prefix query** so the match is clearly *fuzzy* (e.g. `term` finds Terminal, `sys` finds System Settings, `actmon` finds Activity Monitor). Best match selected at the top, 2-3 results total. |
| `favorites` | Pin favorites. Launch with Cmd 1-0 | **Empty query with 4-6 favorites pinned** — every top row shows ★ + its Cmd-number badge. Highlight a favorite row (not the first) so the eye lands on the shortcut. |
| `keyboard` | Keyboard-first, mouse-optional | **Empty (or short) query** with the results list full and the Cmd-number badges visible. Select a row a few down from the top, to imply arrow-key navigation. |
| `private` | Private by design. No network. Ever. | **Empty query, a calm/tidy list.** This shot is about trust, so no clutter and no sensitive or personal app names. The "Offline" badge is added by the frame. |

**Consistency tips (make them feel like one set):**
- Pick **one theme** (light or dark) and use it for every shot.
- Use the **same window height / number of results** across shots.
- Prefer clean, widely-recognized app names; hide anything private first
  (Settings → Index & Permissions, or disable apps).
- Capture at **native Retina** resolution for the crispest result.

## Capturing the real launcher (with its glass)

The glass samples whatever is *behind* the window, so we make that match the frame.

1. **Generate the matching wallpaper:** `mise run screenshots` writes
   `assets/store/wallpaper.png`. Set it as your desktop (System Settings →
   Wallpaper → add file). Now the glass tint matches the marketing gradient.

2. **Keep the launcher from hiding** while you capture (it normally hides on blur):
   ```
   launchctl setenv MYAPPSPOT_KEEP_OPEN 1
   ```
   Then launch the app (`mise run sandbox`, or open the built .app). Apps launched
   after this inherit the variable.

3. **Open the launcher** (⌘⇧Space) and set the state you want for that shot
   (type a query, show favorites, select a row…).

4. **Capture just the window, without the OS shadow** (the frame adds its own):
   ```
   screencapture -iWo assets/store/captures/hero-launch.png
   ```
   `-W` starts in window mode, so just click the launcher window (no crosshair);
   `-o` drops the OS shadow. The PNG keeps transparent rounded corners + the real
   glass over the gradient.

5. **Re-run** `mise run screenshots` — the capture is composited into the frame.

6. **Done capturing?** Restore normal hide-on-blur:
   ```
   launchctl unsetenv MYAPPSPOT_KEEP_OPEN
   ```

## Mascot overlay (per-shot, drop-in like captures)

Every shot shows a mascot, placed per shot. Each shot loads its own image
`assets/store/mascot-<slug>.png` if present, else the generic `mascot.png`, else a
dashed placeholder. Themed placeholders are included — replace any PNG with your
final art (they're what the buddy is "holding" to match each message):

| File | Shot | Theme |
|---|---|---|
| `mascot-hero-launch.png` | hero | magnifier |
| `mascot-fuzzy-search.png` | fuzzy search | magnifier |
| `mascot-favorites.png` | favorites | star |
| `mascot-keyboard.png` | keyboard | Command keycap |
| `mascot-private.png` | private | shield |

- **Position/size** per shot via the `M_*` presets (or `mascot=dict(x=,y=,w=)`
  inline) near the top of `scripts/gen-store-screenshots.py`. Current spots vary
  by shot (bottom-left, bottom-right, or centered/bigger for sparse shots).
- The placeholder `.svg` sources sit next to the PNGs; re-render one with
  `rsvg-convert -w 760 assets/store/mascot-<slug>.svg -o assets/store/mascot-<slug>.png`.

## Tuning the frames

Edit the `SHOTS` list in `scripts/gen-store-screenshots.py` — headline (`title`),
subtitle (`sub`), and per-shot `top` (vertical position of the window) / `cap_w`
(width the capture is scaled to). Mock-only fields (`rows`, `query`, `sel`,
`fav_count`, `badge`) are ignored once a real capture exists for that slug.
