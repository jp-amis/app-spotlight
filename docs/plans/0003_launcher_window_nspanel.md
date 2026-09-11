# 0003 — Launcher window (NSPanel)

## Goal

A Spotlight-like window: borderless, centered, appears instantly on shortcut, floats above everything, does NOT activate the app or steal focus ordering, hides on Esc/selection/blur. Fully keyboard-driven.

## Why plain Tauri window isn't enough

A normal `NSWindow`/Tauri window activates the app when shown (dock bounce, menu-bar takeover, focus reorder). Spotlight uses a non-activating `NSPanel` (`NSWindowStyleMaskNonactivatingPanel`) with high window level so it can become key-for-typing without activating the owning app, and appear over fullscreen spaces.

## Approach

- Use **`tauri-plugin-nspanel`** (community) to convert the launcher window into an `NSPanel`. If it proves insufficient, fall back to a tiny local crate (`crates/spotlight-macos`) using `objc2`/`cocoa` to set the style mask, window level (`NSStatusWindowLevel`+), `collectionBehavior` (`.canJoinAllSpaces | .fullScreenAuxiliary`), and `becomesKeyOnlyIfNeeded`.
- Window config: `decorations: false`, `transparent: true`, `resizable: false`, `alwaysOnTop: true`, `visibleOnAllWorkspaces`, `skipTaskbar`, `focus: true` on show.
- Created once at startup, hidden. Show = position-center-on-active-screen + orderFront + make key. Hide = orderOut (keep instance).
- Multi-monitor: center on the screen containing the mouse / active window.
- Vibrancy background via `macOSPrivateApi` (see 0009).

## Show/hide triggers

- Show: global shortcut (0004), or tray menu "Open".
- Hide: `Esc`, successful launch, click-outside (panel `resignKey`/blur), shortcut pressed again (toggle).

## Sizing

- Fixed width (~640px), height grows with results up to a cap then scrolls. Rounded corners, shadow.

## Tasks

- [ ] Add launcher window (hidden) to `tauri.conf.json` or create programmatically at setup.
- [ ] Integrate `tauri-plugin-nspanel`; convert window to non-activating panel.
- [ ] Set window level + collection behavior for all-spaces/fullscreen float.
- [ ] Implement `show_launcher` / `hide_launcher` commands + toggle.
- [ ] Center-on-active-screen logic (multi-monitor).
- [ ] Blur-to-hide + Esc-to-hide.
- [ ] Verify no dock activation / no focus theft when opening over another app.

## Risks

- `tauri-plugin-nspanel` version compat with Tauri 2.x — verify before committing; native fallback ready.
- Sandbox: NSPanel/window-level APIs are fine under sandbox (no extra entitlement).
- Fullscreen-space float can be finicky; needs real-device testing.

## Unresolved questions

- Accept community `tauri-plugin-nspanel` dependency, or write the small native crate from the start for control?
- Toggle-on-repeat-shortcut, or shortcut only opens (Esc closes)?
