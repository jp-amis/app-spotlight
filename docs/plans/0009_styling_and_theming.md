# 0009 — Styling & theming

## Goal

Extra-stylized, native-feeling, fast launcher UI. Vibrancy, crisp typography, smooth (but cheap) motion.

## Visuals

- Background: macOS **vibrancy/blur** via `macOSPrivateApi` (`NSVisualEffectView`, `.hudWindow`/`.popover` material) behind a translucent Solid UI. Transparent Tauri window.
- Rounded corners + soft shadow (panel-level, see 0003).
- Large search input, comfortable row height, app icon + name (+ subtle path/category), selection highlight with accent.
- Theme: follow system (light/dark), with manual override (0007). Accent color option.

## Motion (keep cheap)

- Appear: fast fade/scale (~80–120ms), GPU-only transforms/opacity. No layout animation.
- Selection move: instant or ≤60ms highlight slide.
- Respect `prefers-reduced-motion`.

## Performance rules

- Transforms/opacity only; avoid animating blur/box-shadow.
- Fixed row heights; avoid reflow on keystroke.
- Preload icon assets; no runtime image decoding in hot path.
- Tailwind with purge; ship minimal CSS.

## Tasks

- [ ] Vibrancy window background (private API / native).
- [ ] Design tokens (spacing, radius, accent, typography) in Tailwind config.
- [ ] Launcher layout: input + result rows + selection styling.
- [ ] Light/dark + accent theming wired to settings.
- [ ] Appear/hide + selection transitions (reduced-motion aware).
- [ ] Visual QA on Retina + external displays.

## Risks

- Private-API vibrancy could break on OS updates — isolate behind a small module; graceful fallback to solid translucent bg.

## Unresolved questions

- Vibrancy material: HUD (dark, punchy) vs popover (adaptive)?
- Show secondary metadata (path/category) or name-only minimalist?
