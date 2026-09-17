//! Tauri command surface exposed to the frontend, plus window + settings helpers.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime, State};
use tauri_plugin_store::StoreExt;

use crate::apps::AppEntry;
use crate::AppState;

pub const DEFAULT_SHORTCUT: &str = "CmdOrCtrl+Shift+Space";
pub(crate) const SETTINGS_FILE: &str = "settings.json";
const ICON_SIZE: u32 = 64;

// ---------- app list + icons ----------

/// Read a persisted list-of-strings setting (e.g. disabled folders/apps).
fn read_string_set<R: Runtime>(app: &AppHandle<R>, key: &str) -> std::collections::HashSet<String> {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get(key))
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        .unwrap_or_default()
        .into_iter()
        .collect()
}

/// An app is enabled unless it's explicitly disabled or sits under a disabled folder (0017).
fn app_enabled(
    path: &str,
    disabled_apps: &std::collections::HashSet<String>,
    disabled_folders: &std::collections::HashSet<String>,
) -> bool {
    if disabled_apps.contains(path) {
        return false;
    }
    !disabled_folders
        .iter()
        .any(|f| path.starts_with(&format!("{}/", f.trim_end_matches('/'))))
}

/// The launcher's app list — enabled apps only.
#[tauri::command]
pub fn list_apps<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Vec<AppEntry> {
    let da = read_string_set(&app, "disabled_apps");
    let df = read_string_set(&app, "disabled_folders");
    state
        .apps
        .read()
        .unwrap()
        .iter()
        .filter(|a| app_enabled(&a.path, &da, &df))
        .cloned()
        .collect()
}

#[derive(serde::Serialize)]
pub struct IndexedApp {
    pub name: String,
    pub path: String,
    pub bundle_id: Option<String>,
    pub enabled: bool,
}

/// Every indexed app plus its enabled flag — for settings, which shows disabled apps too.
#[tauri::command]
pub fn indexed_apps<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Vec<IndexedApp> {
    let da = read_string_set(&app, "disabled_apps");
    let df = read_string_set(&app, "disabled_folders");
    state
        .apps
        .read()
        .unwrap()
        .iter()
        .map(|a| IndexedApp {
            name: a.name.clone(),
            path: a.path.clone(),
            bundle_id: a.bundle_id.clone(),
            enabled: app_enabled(&a.path, &da, &df),
        })
        .collect()
}

fn toggle_string_set<R: Runtime>(
    app: &AppHandle<R>,
    key: &str,
    value: &str,
    present: bool,
) -> Result<(), String> {
    let mut set = read_string_set(app, key);
    if present {
        set.insert(value.to_string());
    } else {
        set.remove(value);
    }
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    store.set(key, json!(set.into_iter().collect::<Vec<_>>()));
    store.save().map_err(|e| e.to_string())?;
    // The effective app list changed → bump "last updated" and re-filter the launcher.
    *app.state::<AppState>().last_indexed_ms.lock().unwrap() = Some(crate::now_ms());
    let _ = app.emit("apps:reindexed", ());
    Ok(())
}

#[tauri::command]
pub fn set_app_disabled<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    disabled: bool,
) -> Result<(), String> {
    toggle_string_set(&app, "disabled_apps", &path, disabled)
}

#[tauri::command]
pub fn set_folder_disabled<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    disabled: bool,
) -> Result<(), String> {
    toggle_string_set(&app, "disabled_folders", &path, disabled)
}

/// Bulk enable/disable many apps at once (e.g. "check/uncheck all" in a folder).
#[tauri::command]
pub fn set_apps_disabled<R: Runtime>(
    app: AppHandle<R>,
    paths: Vec<String>,
    disabled: bool,
) -> Result<(), String> {
    let mut set = read_string_set(&app, "disabled_apps");
    for p in &paths {
        if disabled {
            set.insert(p.clone());
        } else {
            set.remove(p);
        }
    }
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    store.set("disabled_apps", json!(set.into_iter().collect::<Vec<_>>()));
    store.save().map_err(|e| e.to_string())?;
    *app.state::<AppState>().last_indexed_ms.lock().unwrap() = Some(crate::now_ms());
    let _ = app.emit("apps:reindexed", ());
    Ok(())
}

/// Lazily extract + cache one app's icon as a `data:` URI. Frontend calls this
/// only for visible rows, so we never do bulk icon work on the hot path.
#[tauri::command]
pub fn app_icon<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    state: State<'_, AppState>,
) -> Option<String> {
    if let Some(hit) = state.icons.lock().unwrap().get(&path) {
        return hit.clone();
    }
    let uri = icon_data_uri(&app, &path);
    state.icons.lock().unwrap().insert(path, uri.clone());
    uri
}

#[cfg(target_os = "macos")]
fn icon_data_uri<R: Runtime>(app: &AppHandle<R>, path: &str) -> Option<String> {
    use base64::Engine;
    // AppKit icon/drawing APIs are main-thread-only; hop there and wait for the bytes.
    let (tx, rx) = std::sync::mpsc::channel();
    let p = path.to_string();
    app.run_on_main_thread(move || {
        let _ = tx.send(crate::macos::icon_png(&p, ICON_SIZE));
    })
    .ok()?;
    let png = rx.recv().ok().flatten()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    Some(format!("data:image/png;base64,{b64}"))
}

#[cfg(not(target_os = "macos"))]
fn icon_data_uri<R: Runtime>(_app: &AppHandle<R>, _path: &str) -> Option<String> {
    None
}

// ---------- launching ----------

#[tauri::command]
pub fn launch_app<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), String> {
    record_frecency(&app, &path);
    let _ = hide_launcher(app);
    #[cfg(target_os = "macos")]
    {
        if !crate::macos::launch(&path) {
            return Err(format!("failed to launch {path}"));
        }
    }
    Ok(())
}

// ---------- window position persistence (shared, per-monitor) ----------
//
// Positions are stored keyed by monitor so a window reopens where the user left it *on the
// monitor they're currently using* — not on some other display it last happened to be on.
// Store shape: `<key>` -> { "<monitor-origin>": { "x": .., "y": .. }, ... } (physical px).

/// Stable-ish id for a monitor within the current arrangement (its top-left origin).
fn monitor_key(m: &tauri::Monitor) -> String {
    let p = m.position();
    format!("{}x{}", p.x, p.y)
}

fn within_monitor(m: &tauri::Monitor, x: i32, y: i32) -> bool {
    let p = m.position();
    let s = m.size();
    x >= p.x && y >= p.y && x < p.x + s.width as i32 && y < p.y + s.height as i32
}

/// The connected monitor whose bounds contain (x, y). Both save and load resolve the
/// monitor this same way (scanning `available_monitors`) so their keys always agree —
/// unlike mixing `current_monitor()` with `monitor_from_point()`, which can disagree.
fn monitor_containing<R: Runtime>(win: &tauri::WebviewWindow<R>, x: i32, y: i32) -> Option<tauri::Monitor> {
    win.available_monitors()
        .ok()?
        .into_iter()
        .find(|m| within_monitor(m, x, y))
}

/// The monitor the user is currently on: the one under the cursor, else primary.
fn active_monitor<R: Runtime>(win: &tauri::WebviewWindow<R>) -> Option<tauri::Monitor> {
    if let Ok(c) = win.cursor_position() {
        if let Some(m) = monitor_containing(win, c.x as i32, c.y as i32) {
            return Some(m);
        }
    }
    win.primary_monitor().ok().flatten()
}

/// The monitor a window is currently on, resolved via its center point.
fn window_monitor<R: Runtime>(win: &tauri::WebviewWindow<R>) -> Option<tauri::Monitor> {
    let p = win.outer_position().ok()?;
    let s = win.outer_size().ok()?;
    let cx = p.x + s.width as i32 / 2;
    let cy = p.y + s.height as i32 / 2;
    monitor_containing(win, cx, cy).or_else(|| win.current_monitor().ok().flatten())
}

/// Place a window in the upper third (horizontally centered) of the given monitor.
fn place_on_monitor<R: Runtime>(win: &tauri::WebviewWindow<R>, m: &tauri::Monitor) {
    if let Ok(ws) = win.outer_size() {
        let ms = m.size();
        let mp = m.position();
        let x = mp.x + ((ms.width as i32 - ws.width as i32) / 2);
        let y = mp.y + (ms.height as f64 * 0.20) as i32;
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }
}

fn save_pos_for_monitor<R: Runtime>(app: &AppHandle<R>, key: &str, mon_key: &str, x: i32, y: i32) {
    if let Ok(store) = app.store(SETTINGS_FILE) {
        let mut map = store
            .get(key)
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        map.insert(mon_key.to_string(), json!({ "x": x, "y": y }));
        store.set(key, Value::Object(map));
        let _ = store.save();
    }
}

fn load_pos_for_monitor<R: Runtime>(
    app: &AppHandle<R>,
    key: &str,
    mon_key: &str,
) -> Option<(i32, i32)> {
    let o = app.store(SETTINGS_FILE).ok()?.get(key)?;
    let p = o.get(mon_key)?;
    Some((p.get("x")?.as_i64()? as i32, p.get("y")?.as_i64()? as i32))
}

// ---------- launcher window ----------

pub const LAUNCHER_WIDTH: f64 = 680.0;
const LAUNCHER_HEIGHT_KEY: &str = "launcher_height";
const LAUNCHER_POS_KEY: &str = "launcher_pos";

#[tauri::command]
pub fn hide_launcher<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("launcher") {
        // Remember where the user left it, keyed by the monitor it's on.
        if let (Ok(p), Some(m)) = (win.outer_position(), window_monitor(&win)) {
            save_pos_for_monitor(&app, LAUNCHER_POS_KEY, &monitor_key(&m), p.x, p.y);
        }
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Hidden ⌘⇧C command: re-center the (open) launcher to the default upper third of the
/// monitor it's currently on. The centered spot persists via the usual save-on-hide.
#[tauri::command]
pub fn recenter_launcher<R: Runtime>(app: AppHandle<R>) {
    if let Some(win) = app.get_webview_window("launcher") {
        if let Some(m) = window_monitor(&win).or_else(|| active_monitor(&win)) {
            place_on_monitor(&win, &m);
        }
    }
}

/// Place the launcher on the *active* monitor (where the cursor is): reuse the saved
/// position for that monitor if it's still valid, else its upper third.
pub fn place_launcher<R: Runtime>(app: &AppHandle<R>, win: &tauri::WebviewWindow<R>) {
    let Some(mon) = active_monitor(win).or_else(|| win.current_monitor().ok().flatten()) else {
        let _ = win.center();
        return;
    };
    if let Some((x, y)) = load_pos_for_monitor(app, LAUNCHER_POS_KEY, &monitor_key(&mon)) {
        if within_monitor(&mon, x, y) {
            let _ = win.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }
    place_on_monitor(win, &mon);
}

/// How often the background watcher checks whether the cursor moved to a different
/// monitor (kept slow — this is a proactive nicety, not the source of truth).
const ACTIVE_MONITOR_POLL_SECS: u64 = 2;

/// While the launcher is HIDDEN, pre-move it to the monitor the cursor is now on, so the
/// next open has no repositioning to do (avoids any monitor-switch flash at show time).
/// No-op while it's visible (never yank it out from under the user). Main-thread only.
fn reposition_hidden_launcher<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window("launcher") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        return;
    }
    let active = active_monitor(&win);
    let current = window_monitor(&win);
    let changed = match (&active, &current) {
        (Some(a), Some(c)) => monitor_key(a) != monitor_key(c),
        (Some(_), None) => true, // window isn't on any known monitor — reseat it
        _ => false,
    };
    if changed {
        place_launcher(app, &win);
    }
}

/// Spawn a slow background timer that keeps the hidden launcher parked on the active
/// monitor (see `reposition_hidden_launcher`). The check itself runs on the main thread.
pub fn watch_active_monitor<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(ACTIVE_MONITOR_POLL_SECS));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || reposition_hidden_launcher(&handle));
    });
}

/// Persist the launcher's measured content height so the next launch can pre-size the
/// window to it — avoiding the first-open resize "blink" (the window otherwise opens at
/// its config height and jumps to fit content on first show). See plan 0018.
#[tauri::command]
pub fn set_launcher_height<R: Runtime>(app: AppHandle<R>, height: f64) -> Result<(), String> {
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    store.set(LAUNCHER_HEIGHT_KEY, json!(height));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// The persisted launcher height (if sane), for pre-sizing at startup.
pub fn stored_launcher_height<R: Runtime>(app: &AppHandle<R>) -> Option<f64> {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get(LAUNCHER_HEIGHT_KEY))
        .and_then(|v| v.as_f64())
        .filter(|h| (120.0..=1200.0).contains(h))
}

/// Show the launcher: position on the current monitor's upper third, focus it,
/// and tell the frontend to reset its query.
pub fn show_launcher<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(win) = app.get_webview_window("launcher") else {
        return Ok(());
    };
    let app2 = app.clone();
    let w = win.clone();
    let _ = app.run_on_main_thread(move || {
        // Reposition to the active monitor and show — both on the main thread and in
        // this order, so the move is applied synchronously BEFORE the window becomes
        // visible. Previously set_position ran on the caller (shortcut) thread while
        // show ran on the main thread; in release the window would flash at its old
        // monitor/spot for a frame before the async move landed.
        place_launcher(&app2, &w);
        let _ = w.show();
        // Menu-bar (accessory) apps don't activate when a window is shown, so the
        // WKWebView can't take key focus and the search input stays unfocused for up
        // to a second. Activate the app (like Spotlight) so typing works instantly.
        #[cfg(target_os = "macos")]
        crate::macos::activate_app();
        let _ = w.set_focus();
        let _ = app2.emit("launcher:opened", ());
    });
    // Warm the settings window in the background so ⌘; from here opens instantly.
    prewarm_settings(app);
    Ok(())
}

/// Toggle visibility — bound to the global shortcut.
pub fn toggle_launcher<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("launcher") {
        if win.is_visible().unwrap_or(false) {
            return hide_launcher(app.clone());
        }
    }
    show_launcher(app)
}

const SETTINGS_WIDTH: f64 = 720.0;
const SETTINGS_HEIGHT_KEY: &str = "settings_height";
const SETTINGS_POS_KEY: &str = "settings_pos";
const SETTINGS_MIN_H: f64 = 520.0;
const SETTINGS_MAX_H: f64 = 2000.0;

/// How long the settings window lingers (hidden, warm) after it's closed — or after
/// it's pre-warmed but never opened — before we tear it down to free the webview.
const SETTINGS_TTL_SECS: u64 = 180;

#[tauri::command]
pub fn open_settings<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let win = match app.get_webview_window("settings") {
        Some(win) => win,
        // Not pre-warmed (e.g. opened straight from the tray without opening the
        // launcher first) — build it now; it'll reveal itself once loaded.
        None => build_settings_window(&app)?,
    };
    let state = app.state::<AppState>();
    // We're showing it, so cancel any pending teardown timer.
    cancel_settings_ttl(&app);
    if state.settings_loaded.load(std::sync::atomic::Ordering::SeqCst) {
        // Warm and loaded — show instantly.
        front_settings(&win);
    } else {
        // Still loading (freshly built, or mid pre-warm): show it the moment its
        // page finishes loading, so it never appears as an empty pane.
        state
            .settings_wants_show
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
    Ok(())
}

/// Lazily pre-warm the settings window (hidden) so a subsequent open is instant. Called
/// when the launcher opens: if the user then hits ⌘; the webview is already booted.
/// Built a beat after the launcher shows so its cold-boot doesn't hitch the launcher.
pub fn prewarm_settings<R: Runtime>(app: &AppHandle<R>) {
    // Already exists — just extend its lifetime (the user is active).
    if app.get_webview_window("settings").is_some() {
        touch_settings_ttl(app);
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let a2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            if a2.get_webview_window("settings").is_none() {
                let _ = build_settings_window(&a2);
            }
            touch_settings_ttl(&a2);
        });
    });
}

/// Schedule teardown of the (hidden) settings window after `SETTINGS_TTL_SECS`. Bumps a
/// generation counter so any earlier timer is invalidated — call it to (re)start the
/// idle countdown. The window is only destroyed if it's still hidden when the timer fires.
fn touch_settings_ttl<R: Runtime>(app: &AppHandle<R>) {
    let gen = app
        .state::<AppState>()
        .settings_ttl_gen
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(SETTINGS_TTL_SECS));
        // Superseded by a newer open/close/prewarm — this timer is stale.
        if app
            .state::<AppState>()
            .settings_ttl_gen
            .load(std::sync::atomic::Ordering::SeqCst)
            != gen
        {
            return;
        }
        let a2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(win) = a2.get_webview_window("settings") {
                if !win.is_visible().unwrap_or(false) {
                    let _ = win.destroy();
                    let s = a2.state::<AppState>();
                    s.settings_loaded
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    s.settings_wants_show
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                }
            }
        });
    });
}

/// Cancel a pending teardown (the window is about to be, or is, shown).
fn cancel_settings_ttl<R: Runtime>(app: &AppHandle<R>) {
    app.state::<AppState>()
        .settings_ttl_gen
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

/// Build the settings window (hidden). Lazily pre-warmed when the launcher opens (see
/// `prewarm_settings`) so a subsequent open is instant instead of paying a full
/// WKWebView cold-boot + bundle load. Closing it HIDES (kept warm), and it's torn down
/// after an idle period. Starts hidden and reveals itself once its page has loaded.
pub fn build_settings_window<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<tauri::WebviewWindow<R>, String> {
    // Restore the last-used height (width is locked).
    let height = app
        .store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get(SETTINGS_HEIGHT_KEY))
        .and_then(|v| v.as_f64())
        .filter(|h| (SETTINGS_MIN_H..=SETTINGS_MAX_H).contains(h))
        .unwrap_or(760.0);

    let win = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title(crate::i18n::t(app, crate::i18n::Key::SettingsTitle))
    .inner_size(SETTINGS_WIDTH, height)
    // Lock the width (min == max), allow only height resizing.
    .min_inner_size(SETTINGS_WIDTH, SETTINGS_MIN_H)
    .max_inner_size(SETTINGS_WIDTH, SETTINGS_MAX_H)
    .resizable(true)
    // No web inspector / "Inspect Element", even in debug builds.
    .devtools(false)
    // Opaque dark backing so the window never flashes white during live resize
    // (the shared app.css keeps the body transparent for the launcher's vibrancy;
    // the settings body is made opaque in the frontend — see .settings-window).
    .background_color(tauri::window::Color(23, 23, 23, 255))
    // Overlay = full-size content view: our content extends under the titlebar so the
    // custom (centered, bold) title can sit on the same row as the traffic lights.
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .hidden_title(true)
    // Created hidden and warmed in the background; revealed once its page has loaded
    // (either right away if a show is already pending, or by `open_settings` later).
    .visible(false)
    .on_page_load(|window, payload| {
        if payload.event() == tauri::webview::PageLoadEvent::Finished {
            let app = window.app_handle();
            let state = app.state::<AppState>();
            state
                .settings_loaded
                .store(true, std::sync::atomic::Ordering::SeqCst);
            // If an open was requested before we finished loading, honour it now.
            if state
                .settings_wants_show
                .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                front_settings(&window);
            }
        }
    })
    .build()
    .map_err(|e| e.to_string())?;

    // Open on the active monitor (the one under the cursor), restoring the saved position
    // for that monitor if it's still valid; otherwise the OS centers it there.
    if let Some(mon) = active_monitor(&win) {
        if let Some((x, y)) = load_pos_for_monitor(app, SETTINGS_POS_KEY, &monitor_key(&mon)) {
            if within_monitor(&mon, x, y) {
                let _ = win.set_position(PhysicalPosition::new(x, y));
            }
        }
    }

    // Persist height (on resize) and position (on move) so they're restored next open;
    // and intercept close so the window HIDES (stays warm) instead of being destroyed.
    let scale = win.scale_factor().unwrap_or(2.0);
    let app_for_event = app.clone();
    let last_h = std::sync::Mutex::new(0.0_f64);
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            // Hide (keep warm) instead of destroying, and start the idle countdown that
            // tears it down if it isn't reopened within SETTINGS_TTL_SECS.
            api.prevent_close();
            app_for_event
                .state::<AppState>()
                .settings_wants_show
                .store(false, std::sync::atomic::Ordering::SeqCst);
            if let Some(win) = app_for_event.get_webview_window("settings") {
                let _ = win.hide();
            }
            touch_settings_ttl(&app_for_event);
        }
        tauri::WindowEvent::Resized(size) => {
            let h = (size.height as f64 / scale).round();
            if !(SETTINGS_MIN_H..=SETTINGS_MAX_H).contains(&h) {
                return;
            }
            let mut last = last_h.lock().unwrap();
            if (h - *last).abs() < 1.0 {
                return;
            }
            *last = h;
            if let Ok(store) = app_for_event.store(SETTINGS_FILE) {
                store.set(SETTINGS_HEIGHT_KEY, json!(h));
                let _ = store.save();
            }
        }
        tauri::WindowEvent::Moved(pos) => {
            if let Some(win) = app_for_event.get_webview_window("settings") {
                if let Some(m) = window_monitor(&win) {
                    save_pos_for_monitor(
                        &app_for_event,
                        SETTINGS_POS_KEY,
                        &monitor_key(&m),
                        pos.x,
                        pos.y,
                    );
                }
            }
        }
        _ => {}
    });

    // Match the saved theme (light/dark/system) so the window opens in the right
    // appearance. (Applied while still hidden.)
    apply_theme(app);
    Ok(win)
}

/// Bring the settings window to the front, focused. Menu-bar (accessory) apps must
/// activate the app or the window opens behind whatever's frontmost.
fn front_settings<R: Runtime>(win: &tauri::WebviewWindow<R>) {
    let w = win.clone();
    let _ = win.run_on_main_thread(move || {
        let _ = w.show();
        #[cfg(target_os = "macos")]
        crate::macos::activate_app();
        let _ = w.set_focus();
    });
}

// ---------- settings store ----------

fn defaults() -> Value {
    json!({
        "shortcut": DEFAULT_SHORTCUT,
        "menubar_visible": true,
        "theme": "system",
        "launcher_style": "glass",
        "language": "system",
        "font_scale": "normal",
        "result_limit": 10,
        "frecency": {},
    })
}

#[tauri::command]
pub fn get_settings<R: Runtime>(app: AppHandle<R>) -> Result<Value, String> {
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    let mut out = defaults();
    if let Some(obj) = out.as_object_mut() {
        for key in obj.keys().cloned().collect::<Vec<_>>() {
            if let Some(v) = store.get(&key) {
                obj.insert(key, v);
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn set_setting<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    value: Value,
) -> Result<(), String> {
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    store.set(&key, value.clone());
    store.save().map_err(|e| e.to_string())?;
    apply_side_effect(&app, &key, &value);
    Ok(())
}

/// The language preference for the UI: the raw `setting` ("system" | "en-US" | "pt-BR")
/// plus the resolved `effective` tag the frontend should load its catalog for.
#[tauri::command]
pub fn language<R: Runtime>(app: AppHandle<R>) -> Value {
    json!({
        "setting": crate::i18n::setting(&app),
        "effective": crate::i18n::resolve(&app).as_tag(),
    })
}

/// Relaunch the app (used as a fallback to fully apply a language change if a native
/// surface couldn't be swapped live).
#[tauri::command]
pub fn restart_app<R: Runtime>(app: AppHandle<R>) {
    app.restart();
}

fn apply_side_effect<R: Runtime>(app: &AppHandle<R>, key: &str, value: &Value) {
    match key {
        "menubar_visible" => {
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_visible(value.as_bool().unwrap_or(true));
            }
        }
        "theme" | "launcher_style" => apply_theme(app),
        "language" => apply_language(app),
        // Pure CSS concern (root font-size): just notify both windows to re-apply.
        "font_scale" => {
            let _ = app.emit("font:changed", json!({ "scale": value.as_str() }));
        }
        _ => {}
    }
}

/// Re-localize the native surfaces after a language change and tell the frontend to
/// re-render. Rebuilds the tray + app menus and retitles the Settings window; any
/// surface that can't be swapped live sets `needs_restart` so the UI can offer a
/// restart to finish applying.
fn apply_language<R: Runtime>(app: &AppHandle<R>) {
    let lang = crate::i18n::resolve(app);
    let mut needs_restart = false;

    // Tray menu.
    if let Some(tray) = app.tray_by_id("main") {
        match crate::tray::build_menu(app) {
            Ok(menu) => {
                if tray.set_menu(Some(menu)).is_err() {
                    needs_restart = true;
                }
            }
            Err(_) => needs_restart = true,
        }
    }

    // Native app menu (Edit/Window titles).
    match crate::build_app_menu(app) {
        Ok(menu) => {
            if app.set_menu(menu).is_err() {
                needs_restart = true;
            }
        }
        Err(_) => needs_restart = true,
    }

    // Settings window title (if open).
    if let Some(win) = app.get_webview_window("settings") {
        let title = crate::i18n::tr(lang, crate::i18n::Key::SettingsTitle).to_string();
        let w = win.clone();
        let _ = win.run_on_main_thread(move || {
            let _ = w.set_title(&title);
        });
    }

    let _ = app.emit(
        "language:changed",
        json!({ "lang": lang.as_tag(), "needs_restart": needs_restart }),
    );
}

/// The saved theme preference: "system" | "light" | "dark" (defaults to system).
fn current_theme<R: Runtime>(app: &AppHandle<R>) -> String {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get("theme"))
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "system".into())
}

/// The launcher's saved surface style: "glass" | "glass_clear" | "vibrancy".
/// Falls back to "glass" on macOS 26+ (Liquid Glass) and "vibrancy" elsewhere.
fn current_launcher_style<R: Runtime>(app: &AppHandle<R>) -> String {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get("launcher_style"))
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "glass".into())
}

/// Apply the saved theme + launcher surface to the windows. On macOS the NSWindow
/// appearance cascades to the WKWebView, so the web content's `prefers-color-scheme`
/// (and our `dark:` styles) follow along. No-op elsewhere.
pub fn apply_theme<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let dark = match current_theme(app).as_str() {
            "light" => Some(false),
            "dark" => Some(true),
            _ => None, // system
        };
        let style = current_launcher_style(app);
        // Launcher: set appearance, then (re)apply the chosen translucent surface.
        if let Some(win) = app.get_webview_window("launcher") {
            let w = win.clone();
            let _ = win.run_on_main_thread(move || {
                if let Ok(ptr) = w.ns_window() {
                    crate::macos::set_appearance(ptr, dark);
                    // Start clean so switching styles never stacks surfaces.
                    crate::macos::clear_glass(ptr);
                    let _ = window_vibrancy::clear_vibrancy(&w);
                    let want_glass = style == "glass" || style == "glass_clear";
                    let applied = want_glass
                        && crate::macos::apply_liquid_glass(ptr, 16.0, style == "glass_clear");
                    if !applied {
                        // "vibrancy", or glass unavailable (pre-macOS 26): frosted material.
                        let is_dark = dark.unwrap_or_else(|| crate::macos::is_dark(ptr));
                        let _ = window_vibrancy::apply_vibrancy(
                            &w,
                            crate::macos::launcher_material(is_dark),
                            Some(window_vibrancy::NSVisualEffectState::Active),
                            Some(16.0),
                        );
                    }
                    // Clip to the rounded card so the glass rim doesn't show as a border.
                    crate::macos::round_launcher_content(ptr, 16.0);
                }
            });
        }
        // The settings window is opaque; just set its appearance.
        if let Some(win) = app.get_webview_window("settings") {
            let w = win.clone();
            let _ = win.run_on_main_thread(move || {
                if let Ok(ptr) = w.ns_window() {
                    crate::macos::set_appearance(ptr, dark);
                }
            });
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

pub fn current_shortcut<R: Runtime>(app: &AppHandle<R>) -> String {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get("shortcut"))
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string())
}

/// Rebind the global shortcut: unregister the old, register the new, persist.
/// Rolls back on failure so we never end up with no working hotkey.
#[tauri::command]
pub fn set_shortcut<R: Runtime>(app: AppHandle<R>, accelerator: String) -> Result<(), String> {
    let previous = current_shortcut(&app);
    crate::shortcut::unregister_all(&app);
    if let Err(e) = crate::shortcut::register(&app, &accelerator) {
        // roll back
        let _ = crate::shortcut::register(&app, &previous);
        return Err(format!("could not register '{accelerator}': {e}"));
    }
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    store.set("shortcut", json!(accelerator));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- favorites (pinned apps) ----------

const FAVORITES_KEY: &str = "favorites";
const MAX_FAVORITES: usize = 10;
/// Persisted mirror of the StoreKit "unlock favorites" entitlement.
const FAV_PURCHASED_KEY: &str = "favorites_purchased";

/// Has the user bought the favorites unlock (owned forever)? True if StoreKit
/// reports the entitlement, or the persisted mirror flag is set.
pub fn favorites_purchased<R: Runtime>(app: &AppHandle<R>) -> bool {
    crate::iap::is_purchased()
        || app
            .store(SETTINGS_FILE)
            .ok()
            .and_then(|s| s.get(FAV_PURCHASED_KEY))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
}

/// Are favorites usable right now? Purchased forever, or unlocked for this session.
pub fn favorites_unlocked<R: Runtime>(app: &AppHandle<R>, state: &AppState) -> bool {
    favorites_purchased(app) || state.fav_session.load(std::sync::atomic::Ordering::SeqCst)
}

/// The ordered list of pinned app paths.
#[tauri::command]
pub fn get_favorites<R: Runtime>(app: AppHandle<R>) -> Vec<String> {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get(FAVORITES_KEY))
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        .unwrap_or_default()
}

/// Replace the favorites list (add / remove / reorder in one call). De-duped, capped at 10.
/// Refuses when favorites aren't unlocked (UI won't call it either — defense in depth).
#[tauri::command]
pub fn set_favorites<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<(), String> {
    if !favorites_unlocked(&app, &state) {
        return Err(crate::i18n::t(&app, crate::i18n::Key::ErrUnlockFavorites));
    }
    let mut seen = std::collections::HashSet::new();
    let list: Vec<String> = paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .take(MAX_FAVORITES)
        .collect();
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    store.set(FAVORITES_KEY, json!(list));
    store.save().map_err(|e| e.to_string())?;
    let _ = app.emit("favorites:changed", ());
    Ok(())
}

/// Entitlement + product info for the favorites unlock, for the paywall UI.
#[tauri::command]
pub fn favorites_status<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Value {
    json!({
        "unlocked": favorites_unlocked(&app, &state),
        "purchased": favorites_purchased(&app),
        "purchasable": crate::iap::purchasable(),
        "price": crate::iap::price(),
    })
}

/// Free "unlock for this session" — lasts until the app quits.
#[tauri::command]
pub fn unlock_favorites_session<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) {
    state
        .fav_session
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = app.emit("favorites:unlocked", ());
}

/// Buy the non-consumable unlock (StoreKit). Persists the mirror flag on success.
#[tauri::command]
pub fn purchase_favorites<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    crate::iap::purchase().map_err(|e| crate::i18n::t(&app, e.key()))?;
    mark_favorites_purchased(&app);
    let _ = app.emit("favorites:unlocked", ());
    Ok(())
}

/// Restore a previous purchase (StoreKit). Returns true if something was restored.
#[tauri::command]
pub fn restore_favorites<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    let restored = crate::iap::restore().map_err(|e| crate::i18n::t(&app, e.key()))?;
    if restored {
        mark_favorites_purchased(&app);
        let _ = app.emit("favorites:unlocked", ());
    }
    Ok(restored)
}

/// Persist the "owned forever" mirror flag (source of truth is StoreKit, but this
/// keeps the frontend consistent across launches without a StoreKit round-trip).
pub fn mark_favorites_purchased<R: Runtime>(app: &AppHandle<R>) {
    if let Ok(store) = app.store(SETTINGS_FILE) {
        store.set(FAV_PURCHASED_KEY, json!(true));
        let _ = store.save();
    }
}

/// Temporarily unregister the global shortcut so the settings capture box can receive the
/// key combo itself (instead of the global handler firing / opening the launcher).
/// Fully quit the app (there is no ⌘Q quit; this is the explicit Quit action).
#[tauri::command]
pub fn quit_app<R: Runtime>(app: AppHandle<R>) {
    app.exit(0);
}

/// Whether the app is set to open at login (macOS SMAppService).
#[tauri::command]
pub fn get_login_item() -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::macos::login_item_enabled()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Enable/disable "open at login".
#[tauri::command]
pub fn set_login_item<R: Runtime>(app: AppHandle<R>, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::set_login_item(enabled).map_err(|e| match e {
            crate::macos::LoginError::Os(s) => s,
            other => crate::i18n::t(&app, other.key()),
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, enabled);
        Ok(())
    }
}

/// Open System Settings at the Keyboard pane, where the macOS Spotlight ⌘Space
/// shortcut lives (Keyboard Shortcuts → Spotlight), so the user can free it up.
#[tauri::command]
pub fn open_keyboard_settings() {
    #[cfg(target_os = "macos")]
    {
        // Modern System Settings (macOS 13+). Opens Keyboard settings; the user then
        // taps "Keyboard Shortcuts…" → Spotlight.
        let _ = crate::macos::open_url("x-apple.systempreferences:com.apple.Keyboard-Settings.extension");
    }
}

#[tauri::command]
pub fn suspend_shortcut<R: Runtime>(app: AppHandle<R>) {
    crate::shortcut::unregister_all(&app);
}

/// Re-register the stored shortcut after a cancelled capture.
#[tauri::command]
pub fn resume_shortcut<R: Runtime>(app: AppHandle<R>) {
    let accel = current_shortcut(&app);
    let _ = crate::shortcut::register(&app, &accel);
}

fn record_frecency<R: Runtime>(app: &AppHandle<R>, path: &str) {
    let Ok(store) = app.store(SETTINGS_FILE) else {
        return;
    };
    let mut frecency = store
        .get("frecency")
        .unwrap_or_else(|| json!({}));
    let obj = frecency.as_object_mut();
    if let Some(obj) = obj {
        let count = obj
            .get(path)
            .and_then(|v| v.get("count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            + 1;
        obj.insert(path.to_string(), json!({ "count": count }));
    }
    store.set("frecency", frecency);
    let _ = store.save();
}

// ---------- folder grants / sandbox permissions (plan 0013 issue 4) ----------

const BOOKMARKS_KEY: &str = "folder_bookmarks";

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Bookmark {
    path: String,
    bookmark: String, // base64-encoded security-scoped bookmark
}

fn read_bookmarks<R: Runtime>(app: &AppHandle<R>) -> Vec<Bookmark> {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get(BOOKMARKS_KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn write_bookmarks<R: Runtime>(app: &AppHandle<R>, list: &[Bookmark]) -> Result<(), String> {
    let store = app.store(SETTINGS_FILE).map_err(|e| e.to_string())?;
    store.set(
        BOOKMARKS_KEY,
        serde_json::to_value(list).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())
}

/// Resolve every stored bookmark, begin accessing it, and return the live paths.
/// Called at startup (on the main thread) before the first enumeration.
pub fn resolve_granted<R: Runtime>(app: &AppHandle<R>) -> Vec<String> {
    let mut out = Vec::new();
    for bm in read_bookmarks(app) {
        #[cfg(target_os = "macos")]
        {
            use base64::Engine;
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&bm.bookmark) {
                if let Some(path) = crate::permissions::resolve_and_access(&bytes) {
                    out.push(path);
                    continue;
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        out.push(bm.path.clone());
    }
    out
}

#[derive(serde::Serialize)]
pub struct AccessStatus {
    pub limited: bool,
    pub suggested_path: String,
}

/// Whether the real `~/Applications` is currently unreadable (so we should offer a grant).
#[tauri::command]
pub fn access_status<R: Runtime>(_app: AppHandle<R>) -> AccessStatus {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = crate::permissions::real_home() {
            let apps = home.join("Applications");
            let readable = std::fs::read_dir(&apps).is_ok();
            return AccessStatus {
                limited: !readable,
                suggested_path: apps.to_string_lossy().to_string(),
            };
        }
    }
    AccessStatus {
        limited: false,
        suggested_path: String::new(),
    }
}

/// Show the folder picker; on grant, persist the bookmark, start accessing it, and
/// If `path` is already reachable — equal to, or nested under, a default scanned location
/// or an existing granted folder — return the covering base path (else None).
fn folder_already_covered<R: Runtime>(app: &AppHandle<R>, path: &str) -> Option<String> {
    let norm = |p: &str| p.trim_end_matches('/').to_string();
    let target = norm(path);

    let mut bases: Vec<String> = crate::apps::dirs_to_scan(&[])
        .iter()
        .map(|d| norm(&d.to_string_lossy()))
        .collect();
    bases.extend(read_bookmarks(app).into_iter().map(|b| norm(&b.path)));

    bases
        .into_iter()
        .find(|base| !base.is_empty() && (target == *base || target.starts_with(&format!("{base}/"))))
}

/// reindex. Returns the granted path (or None if the user cancelled).
#[tauri::command]
pub fn grant_folder<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        use base64::Engine;
        let lang = crate::i18n::resolve(&app);
        let prompt = crate::i18n::tr(lang, crate::i18n::Key::PickerPrompt).to_string();
        let message = crate::i18n::tr(lang, crate::i18n::Key::PickerMessage).to_string();
        // NSOpenPanel must run on the main thread.
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(crate::permissions::pick_folder(&prompt, &message));
        })
        .map_err(|e| e.to_string())?;

        let Some((path, bytes)) = rx.recv().map_err(|e| e.to_string())? else {
            return Ok(None);
        };

        // Reject folders already covered by the default system scan or an existing grant.
        if let Some(base) = folder_already_covered(&app, &path) {
            return Err(crate::i18n::already_searched(lang, &path, &base));
        }

        let mut list = read_bookmarks(&app);
        list.retain(|b| b.path != path);
        list.push(Bookmark {
            path: path.clone(),
            bookmark: base64::engine::general_purpose::STANDARD.encode(&bytes),
        });
        write_bookmarks(&app, &list)?;

        if let Some(resolved) = crate::permissions::resolve_and_access(&bytes) {
            let state = app.state::<AppState>();
            let mut granted = state.granted.write().unwrap();
            if !granted.contains(&resolved) {
                granted.push(resolved);
            }
        }
        crate::reindex(&app);
        return Ok(Some(path));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(None)
    }
}

#[tauri::command]
pub fn list_granted_folders<R: Runtime>(app: AppHandle<R>) -> Vec<String> {
    read_bookmarks(&app).into_iter().map(|b| b.path).collect()
}

// ---------- index status / detailed view (plan 0015) ----------

#[derive(serde::Serialize)]
pub struct DirInfo {
    pub path: String,
    pub kind: String, // "system" | "granted"
    pub count: usize,
    pub readable: bool,
    pub disabled: bool,
}

#[derive(serde::Serialize)]
pub struct IndexStatus {
    pub indexing: bool,
    pub count: usize,
    pub last_indexed_ms: Option<u64>,
    pub dirs: Vec<DirInfo>,
}

/// A snapshot of the index for the settings "Index & Permissions" view: are we indexing,
/// how many apps, when last built, and the per-folder breakdown.
#[tauri::command]
pub fn index_status<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> IndexStatus {
    use std::sync::atomic::Ordering;

    let apps = state.apps.read().unwrap();
    let granted = state.granted.read().unwrap();
    let granted_paths: Vec<std::path::PathBuf> =
        granted.iter().map(std::path::PathBuf::from).collect();
    let disabled_folders = read_string_set(&app, "disabled_folders");

    let dirs = crate::apps::dirs_to_scan(&granted_paths)
        .into_iter()
        .map(|dir| {
            let p = dir.to_string_lossy().to_string();
            let prefix = format!("{}/", p.trim_end_matches('/'));
            let count = apps.iter().filter(|a| a.path.starts_with(&prefix)).count();
            let kind = if granted.iter().any(|g| g == &p) {
                "granted"
            } else {
                "system"
            };
            DirInfo {
                readable: std::fs::read_dir(&dir).is_ok(),
                disabled: disabled_folders.contains(&p),
                path: p,
                kind: kind.to_string(),
                count,
            }
        })
        // Hide system locations we can't read (e.g. a missing /Applications/Utilities);
        // keep granted folders even when unreadable so a stale grant can be removed (0016).
        .filter(|d| d.kind == "granted" || d.readable)
        .collect();

    IndexStatus {
        indexing: state.indexing.load(Ordering::SeqCst),
        count: apps.len(),
        last_indexed_ms: *state.last_indexed_ms.lock().unwrap(),
        dirs,
    }
}

/// Trigger a reindex from the UI (parity with the tray's "Reindex Apps").
#[tauri::command]
pub fn reindex_now<R: Runtime>(app: AppHandle<R>) {
    let handle = app.clone();
    std::thread::spawn(move || crate::reindex(&handle));
}

#[tauri::command]
pub fn remove_granted_folder<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), String> {
    let mut list = read_bookmarks(&app);
    list.retain(|b| b.path != path);
    write_bookmarks(&app, &list)?;
    {
        let state = app.state::<AppState>();
        state.granted.write().unwrap().retain(|p| p != &path);
    }
    crate::reindex(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::app_enabled;
    use std::collections::HashSet;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn enabled_by_default() {
        assert!(app_enabled("/Applications/Safari.app", &set(&[]), &set(&[])));
    }

    #[test]
    fn disabled_individual_app() {
        let da = set(&["/Applications/Safari.app"]);
        assert!(!app_enabled("/Applications/Safari.app", &da, &set(&[])));
        assert!(app_enabled("/Applications/Mail.app", &da, &set(&[])));
    }

    #[test]
    fn disabled_folder_hides_its_apps() {
        let df = set(&["/Users/jp/Applications"]);
        assert!(!app_enabled("/Users/jp/Applications/CLion.app", &set(&[]), &df));
        // a sibling path that merely shares a prefix string must NOT match
        assert!(app_enabled("/Users/jp/ApplicationsX/Foo.app", &set(&[]), &df));
        assert!(app_enabled("/Applications/Safari.app", &set(&[]), &df));
    }
}
