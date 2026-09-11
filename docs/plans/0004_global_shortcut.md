# 0004 — Global shortcut

## Goal

Register a customizable global hotkey that opens the launcher from anywhere. Default `cmd+shift+space`. Ship guidance to reclaim `cmd+space` from Spotlight.

## Approach

- Use **`tauri-plugin-global-shortcut`** (v2). Register on startup from persisted config; re-register on change.
- Default accelerator: `CommandOrControl+Shift+Space` → on macOS `Cmd+Shift+Space`.
- Rebind flow: settings UI captures a key combo → validates → `invoke("set_shortcut", accel)` → Rust unregisters old, registers new, persists. On failure (already taken), keep old + surface error.

## Reclaiming cmd+space (Spotlight)

- Sandbox **cannot** programmatically disable Spotlight's shortcut. Provide a guide (settings + docs):
  1. System Settings → Keyboard → Keyboard Shortcuts → Spotlight → uncheck "Show Spotlight search".
  2. Then rebind this app to `cmd+space`.
- Detect-and-warn: if user picks `cmd+space` while Spotlight still owns it, registration fails → show the guide inline.

## Edge cases

- Conflicting/failed registration → validation error, no silent drop.
- Re-register after wake/login: verify shortcut survives; re-register on `resume` if needed.
- Multiple rapid presses → toggle/debounce (ties to 0003 toggle decision).

## Tasks

- [ ] Add `tauri-plugin-global-shortcut`, capabilities entry.
- [ ] Register default on setup from store.
- [ ] `set_shortcut` command (unregister→register→persist, rollback on fail).
- [ ] Frontend key-capture component in settings.
- [ ] Inline Spotlight-reclaim guide + failure detection.
- [ ] Test registration persists across sleep/login.

## Risks

- Global shortcut under App Sandbox: allowed (no Accessibility permission needed for hotkeys via this API). Verify on-device — if it requires Input Monitoring, document the prompt.

## Unresolved questions

- Allow multiple shortcuts (e.g. one to open, one for settings), or single hotkey in v1?
- Should picking an already-taken combo hard-fail or offer to "use anyway"?
