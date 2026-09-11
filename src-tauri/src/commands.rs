//! Tauri command surface exposed to the frontend, plus window + settings helpers.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime, State};
use tauri_plugin_store::StoreExt;

use crate::apps::AppEntry;
use crate::AppState;

pub const DEFAULT_SHORTCUT: &str = "CmdOrCtrl+Shift+Space";
const SETTINGS_FILE: &str = "settings.json";
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
    // Enqueue the reposition first...
    place_launcher(app, &win);
    // ...then show on the next main-thread tick, after set_position has been applied —
    // otherwise the window flashes at its previous spot before jumping. (set_position is
    // async; deferring show preserves ordering.)
    let app2 = app.clone();
    let w = win.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app2.emit("launcher:opened", ());
    });
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

const SETTINGS_WIDTH: f64 = 980.0;
const SETTINGS_HEIGHT_KEY: &str = "settings_height";
const SETTINGS_POS_KEY: &str = "settings_pos";
const SETTINGS_MIN_H: f64 = 520.0;
const SETTINGS_MAX_H: f64 = 2000.0;

#[tauri::command]
pub fn open_settings<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        win.show().ok();
        win.set_focus().ok();
        return Ok(());
    }

    // Restore the last-used height (width is locked).
    let height = app
        .store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get(SETTINGS_HEIGHT_KEY))
        .and_then(|v| v.as_f64())
        .filter(|h| (SETTINGS_MIN_H..=SETTINGS_MAX_H).contains(h))
        .unwrap_or(760.0);

    let win = tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("My App Spot Settings")
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
    .build()
    .map_err(|e| e.to_string())?;

    // Open on the active monitor (the one under the cursor), restoring the saved position
    // for that monitor if it's still valid; otherwise the OS centers it there.
    if let Some(mon) = active_monitor(&win) {
        if let Some((x, y)) = load_pos_for_monitor(&app, SETTINGS_POS_KEY, &monitor_key(&mon)) {
            if within_monitor(&mon, x, y) {
                let _ = win.set_position(PhysicalPosition::new(x, y));
            }
        }
    }

    // Persist height (on resize) and position (on move) so they're restored next open.
    let scale = win.scale_factor().unwrap_or(2.0);
    let app_for_event = app.clone();
    let last_h = std::sync::Mutex::new(0.0_f64);
    win.on_window_event(move |event| match event {
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

    // Match the saved theme (light/dark/system) so the window doesn't open in the
    // wrong appearance.
    apply_theme(&app);
    Ok(())
}

// ---------- settings store ----------

fn defaults() -> Value {
    json!({
        "shortcut": DEFAULT_SHORTCUT,
        "menubar_visible": true,
        "theme": "system",
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

fn apply_side_effect<R: Runtime>(app: &AppHandle<R>, key: &str, value: &Value) {
    match key {
        "menubar_visible" => {
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_visible(value.as_bool().unwrap_or(true));
            }
        }
        "theme" => apply_theme(app),
        _ => {}
    }
}

/// The saved theme preference: "system" | "light" | "dark" (defaults to system).
fn current_theme<R: Runtime>(app: &AppHandle<R>) -> String {
    app.store(SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get("theme"))
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "system".into())
}

/// Apply the saved theme to every window's native appearance. On macOS the
/// NSWindow appearance cascades to the WKWebView, so the web content's
/// `prefers-color-scheme` (and our `dark:` styles) follow along. No-op elsewhere.
pub fn apply_theme<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let dark = match current_theme(app).as_str() {
            "light" => Some(false),
            "dark" => Some(true),
            _ => None, // system
        };
        // The launcher is translucent: its vibrancy material must switch too, or a
        // light theme leaves dark HUD glass behind readable light-mode text.
        if let Some(win) = app.get_webview_window("launcher") {
            let w = win.clone();
            let _ = win.run_on_main_thread(move || {
                if let Ok(ptr) = w.ns_window() {
                    crate::macos::set_appearance(ptr, dark);
                    let is_dark = dark.unwrap_or_else(|| crate::macos::is_dark(ptr));
                    let _ = window_vibrancy::clear_vibrancy(&w);
                    let _ = window_vibrancy::apply_vibrancy(
                        &w,
                        crate::macos::launcher_material(is_dark),
                        Some(window_vibrancy::NSVisualEffectState::Active),
                        Some(16.0),
                    );
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
#[tauri::command]
pub fn set_favorites<R: Runtime>(app: AppHandle<R>, paths: Vec<String>) -> Result<(), String> {
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

/// Temporarily unregister the global shortcut so the settings capture box can receive the
/// key combo itself (instead of the global handler firing / opening the launcher).
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
        // NSOpenPanel must run on the main thread.
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(crate::permissions::pick_folder());
        })
        .map_err(|e| e.to_string())?;

        let Some((path, bytes)) = rx.recv().map_err(|e| e.to_string())? else {
            return Ok(None);
        };

        // Reject folders already covered by the default system scan or an existing grant.
        if let Some(base) = folder_already_covered(&app, &path) {
            return Err(format!("“{path}” is already searched (via {base})."));
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
