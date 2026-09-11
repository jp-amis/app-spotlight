# 0007 — Settings & menu bar

## Goal

A settings window to configure the app, and a menu-bar (tray) presence that's on by default.

## Menu bar (tray)

- `tray-icon` feature (as in reference `Cargo.toml`). Template icon (auto light/dark).
- Menu: Open launcher · Settings… · Reindex apps · Launch at login (toggle) · Quit.
- Setting to **hide the menu-bar icon** (then app is reachable only via shortcut) — but default visible.
- Dock: option to hide dock icon (`LSUIElement`/`ActivationPolicy::Accessory`) so it lives purely in menu bar. Default: accessory (menu-bar app, no dock) — matches Spotlight feel. Confirm.

## Settings window

- Separate Tauri window (normal, activating), lazy-created. SolidJS route/view.
- Sections:
  - **Shortcut**: current hotkey + rebind capture (0004) + Spotlight-reclaim guide.
  - **General**: launch at login, menu-bar icon toggle, dock icon toggle, result limit.
  - **Appearance**: theme (system/light/dark), accent, transparency/vibrancy (0009).
  - **Apps**: reindex button, included locations (future).
  - **About**: version, links.

## Persistence

- `tauri-plugin-store` JSON in app-support. Single `settings.json`. Typed getter/setter commands; frontend reads on open, writes on change; Rust reacts (re-register shortcut, toggle tray/dock).

## Launch at login

- `tauri-plugin-autostart` or `SMAppService` (macOS 13+). Verify MAS/sandbox compatibility (`SMAppService.mainApp` is the sandbox-friendly path).

## Tasks

- [ ] Tray icon + menu + actions.
- [ ] Settings window + views.
- [ ] Store schema + get/set commands + reactive apply.
- [ ] Menu-bar / dock visibility toggles (ActivationPolicy).
- [ ] Launch-at-login via SMAppService.
- [ ] Wire appearance settings to launcher styling.

## Risks

- ActivationPolicy switching at runtime can be glitchy — test show/hide of dock icon.
- Autostart under sandbox: use `SMAppService`; older login-item APIs may be rejected.

## Unresolved questions

- Default to no-dock (accessory) app? (recommended yes)
- Settings window: single scroll page or tabbed sections?
