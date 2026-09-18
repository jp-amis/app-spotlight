//! My App Spot — a fast, keyboard-first macOS app launcher (Tauri v2).

mod apps;
mod commands;
mod i18n;
mod iap;
mod shortcut;
mod tray;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod permissions;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

use tauri::{AppHandle, Emitter, Manager, Runtime, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

/// Shared app state: the app index, a lazy icon cache, user-granted folders, the on-disk
/// index cache path (0014), and live indexing status (0015).
pub struct AppState {
    pub apps: RwLock<Vec<apps::AppEntry>>,
    pub icons: Mutex<HashMap<String, Option<String>>>,
    pub granted: RwLock<Vec<String>>,
    pub cache_path: Option<PathBuf>,
    pub indexing: AtomicBool,
    pub last_indexed_ms: Mutex<Option<u64>>,
    /// Free "unlock favorites for this session" flag — reset on process restart.
    pub fav_session: AtomicBool,
    /// Settings window lifecycle (it's lazily pre-warmed when the launcher opens, then
    /// torn down after an idle period to free the webview — see commands.rs):
    ///   `settings_loaded`     — its page has finished loading (safe to show instantly).
    ///   `settings_wants_show` — a show was requested before it finished loading.
    ///   `settings_ttl_gen`    — generation counter that invalidates stale teardown timers.
    pub settings_loaded: AtomicBool,
    pub settings_wants_show: AtomicBool,
    pub settings_ttl_gen: AtomicU64,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn granted_dirs(state: &AppState) -> Vec<PathBuf> {
    state
        .granted
        .read()
        .unwrap()
        .iter()
        .map(PathBuf::from)
        .collect()
}

/// Rebuild the app index (standard locations + granted folders). Emits indexing
/// lifecycle events (0015) and always refreshes the "last indexed" time, but only swaps
/// state / rewrites the cache / emits `apps:reindexed` when the list actually changed —
/// so a no-op refresh (watcher tick, background refresh from a fresh cache) stays cheap.
pub fn reindex<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    state.indexing.store(true, Ordering::SeqCst);
    let _ = app.emit("index:start", ());

    let app2 = app.clone();
    let fresh = apps::enumerate_with(&granted_dirs(&state), |dir, found| {
        let _ = app2.emit(
            "index:progress",
            serde_json::json!({ "dir": dir.to_string_lossy(), "found": found }),
        );
    });

    let changed = {
        let mut current = state.apps.write().unwrap();
        if *current == fresh {
            false
        } else {
            *current = fresh.clone();
            true
        }
    };
    if changed {
        state.icons.lock().unwrap().clear();
        if let Some(path) = &state.cache_path {
            apps::save_cache(path, &fresh);
        }
        let _ = app.emit("apps:reindexed", ());
    }

    *state.last_indexed_ms.lock().unwrap() = Some(now_ms());
    state.indexing.store(false, Ordering::SeqCst);
    let _ = app.emit("index:done", serde_json::json!({ "count": fresh.len() }));
}

/// Watch the app folders with FSEvents and reindex on change, debounced (0014).
/// Event-driven, not polling. The watcher is kept alive on its own thread.
fn start_folder_watcher<R: Runtime>(app: AppHandle<R>, dirs: Vec<PathBuf>) {
    use notify::{RecursiveMode, Watcher};
    use std::sync::mpsc::{channel, RecvTimeoutError};
    use std::time::Duration;

    let (tx, rx) = channel();
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("folder watcher unavailable: {e}");
            return;
        }
    };

    for dir in dirs.iter().filter(|d| d.exists()) {
        // Installs/removes are direct children — non-recursive is enough and cheap.
        let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
    }

    std::thread::spawn(move || {
        let _watcher = watcher; // keep the watch alive for the thread's lifetime
        loop {
            // Block until something changes.
            if rx.recv().is_err() {
                break;
            }
            // Debounce: drain further events until ~800ms of quiet.
            loop {
                match rx.recv_timeout(Duration::from_millis(800)) {
                    Ok(_) => continue,
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            reindex(&app); // no-op if nothing actually changed
        }
    });
}

/// Build the native app menu, localized to the current language. Deliberately has
/// NO Quit item, so ⌘Q doesn't quit this menu-bar app (it closes the focused window
/// instead). Keeps Edit (copy/paste/select-all) and Window→Close (⌘W). Quit is via
/// the tray or the Settings → General button. Rebuilt on a language change.
///
/// Reads the stored language, so it must only be called once the store plugin is
/// managed (i.e. from `setup`/runtime, not the initial `.menu()` builder — that
/// runs too early and uses [`build_app_menu_with`] + OS detection instead).
pub(crate) fn build_app_menu<R: Runtime>(
    handle: &AppHandle<R>,
) -> tauri::Result<tauri::menu::Menu<R>> {
    build_app_menu_with(handle, i18n::resolve(handle))
}

/// Build the native app menu for an explicit language (no store read).
fn build_app_menu_with<R: Runtime>(
    handle: &AppHandle<R>,
    lang: i18n::Lang,
) -> tauri::Result<tauri::menu::Menu<R>> {
    use i18n::{tr, Key};
    use tauri::menu::{Menu, PredefinedMenuItem as P, Submenu};
    // The app-name submenu keeps the brand untranslated.
    let app = Submenu::with_items(
        handle,
        "My App Spot",
        true,
        &[
            &P::about(handle, None, None)?,
            &P::separator(handle)?,
            &P::hide(handle, None)?,
            &P::hide_others(handle, None)?,
            &P::show_all(handle, None)?,
        ],
    )?;
    let edit = Submenu::with_items(
        handle,
        tr(lang, Key::MenuEdit),
        true,
        &[
            &P::undo(handle, None)?,
            &P::redo(handle, None)?,
            &P::separator(handle)?,
            &P::cut(handle, None)?,
            &P::copy(handle, None)?,
            &P::paste(handle, None)?,
            &P::select_all(handle, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        handle,
        tr(lang, Key::MenuWindow),
        true,
        &[
            &P::minimize(handle, None)?,
            &P::close_window(handle, None)?,
        ],
    )?;
    Menu::with_items(handle, &[&app, &edit, &window])
}

pub fn run() {
    tauri::Builder::default()
        // The initial menu is built before the store plugin is managed, so resolve
        // the language from the OS only (no store read). setup() rebuilds it with the
        // stored preference once the store is available.
        .menu(|handle| build_app_menu_with(handle, i18n::detect_os()))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = commands::toggle_launcher(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();

            // Menu-bar app: no dock icon by default (plan 0007).
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Resolve any previously-granted folders (plan 0013).
            let granted = commands::resolve_granted(&handle);
            let dirs: Vec<PathBuf> = granted.iter().map(PathBuf::from).collect();

            // Cache-first startup (0014): paint from the saved index instantly if we
            // have one, then refresh in the background; otherwise build it now.
            let cache_path = app
                .path()
                .app_cache_dir()
                .ok()
                .map(|d| d.join("app-index.json"));
            let (index, from_cache) = match cache_path.as_ref().and_then(|p| apps::load_cache(p)) {
                Some(cached) => (cached, true),
                None => {
                    let fresh = apps::enumerate(&dirs);
                    if let Some(p) = &cache_path {
                        apps::save_cache(p, &fresh);
                    }
                    (fresh, false)
                }
            };
            app.manage(AppState {
                apps: RwLock::new(index),
                icons: Mutex::new(HashMap::new()),
                granted: RwLock::new(granted),
                cache_path,
                indexing: AtomicBool::new(false),
                last_indexed_ms: Mutex::new(Some(now_ms())),
                fav_session: AtomicBool::new(false),
                settings_loaded: AtomicBool::new(false),
                settings_wants_show: AtomicBool::new(false),
                settings_ttl_gen: AtomicU64::new(0),
            });

            // On App Store builds, mirror the StoreKit "favorites unlocked" entitlement
            // into the store so the UI reads a consistent value without a round-trip.
            if iap::is_purchased() {
                commands::mark_favorites_purchased(&handle);
            }

            // Refresh the (possibly stale) cached index off the critical path.
            if from_cache {
                let h = handle.clone();
                std::thread::spawn(move || reindex(&h));
            }

            // Auto-refresh when apps are installed/removed (0014).
            start_folder_watcher(handle.clone(), apps::dirs_to_scan(&dirs));

            // Configure the launcher window: Spotlight-like float + hide-on-blur. The
            // glass/vibrancy surface is applied by commands::apply_theme (called below).
            if let Some(win) = app.get_webview_window("launcher") {
                #[cfg(target_os = "macos")]
                {
                    if let Ok(ptr) = win.ns_window() {
                        macos::tune_launcher_window(ptr);
                    }
                }
                // Pre-size to the last-used content height so the first open doesn't
                // resize-jump (window keeps its size across hide/show afterwards). 0018.
                if let Some(h) = commands::stored_launcher_height(&handle) {
                    let _ = win.set_size(tauri::LogicalSize::new(commands::LAUNCHER_WIDTH, h));
                }
                // Pre-position while hidden so the first show is already placed
                // (last position if still on a connected monitor, else upper third). 0018/0019.
                commands::place_launcher(&handle, &win);
                let hc = handle.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        // Escape hatch for taking App Store screenshots: with
                        // MYAPPSPOT_KEEP_OPEN set, don't hide on blur so the window
                        // survives the screenshot overlay. (Unset in normal use.)
                        if std::env::var_os("MYAPPSPOT_KEEP_OPEN").is_none() {
                            // Blur-hide via the command so it also saves the position.
                            let _ = commands::hide_launcher(hc.clone());
                        }
                    }
                });
            }

            // Apply the saved theme to the launcher window's native appearance
            // (cascades to the webview's prefers-color-scheme).
            commands::apply_theme(&handle);

            tray::build(&handle)?;

            // Re-localize the native app menu now that the store is readable (the
            // initial `.menu()` builder can run before settings are loaded, so a
            // user's non-OS language override wouldn't be reflected there yet).
            if let Ok(menu) = build_app_menu(&handle) {
                let _ = handle.set_menu(menu);
            }

            // Register the global shortcut from settings (default cmd+shift+space).
            let accel = commands::current_shortcut(&handle);
            if let Err(e) = shortcut::register(&handle, &accel) {
                eprintln!("global shortcut '{accel}' failed to register: {e}");
            }

            // Keep the hidden launcher parked on whichever monitor the cursor is on, so
            // opening it never has to reposition (no monitor-switch flash at show time).
            commands::watch_active_monitor(handle.clone());

            // Keep the app (and its hidden launcher webview) out of App Nap, so macOS is far
            // less likely to suspend/kill the WebContent process while it idles in the menu
            // bar — avoiding the "reopen shows empty then reloads" flash. See #5.
            #[cfg(target_os = "macos")]
            macos::prevent_app_nap();

            // Mark a new session in the dev log (no-op unless dev logging is on). Every
            // subsequent "launcher webview mounted" line belongs to this session, so the
            // count/timing of mounts since this line reveals webview reloads (see #5).
            let version = handle.package_info().version.to_string();
            #[cfg(target_os = "macos")]
            let build = macos::bundle_build_number().unwrap_or_else(|| "dev".into());
            #[cfg(not(target_os = "macos"))]
            let build = "n/a".to_string();
            commands::dev_log_line(
                &handle,
                &format!(
                    "==== app started v{version} (build {build}, pid {}) ====",
                    std::process::id()
                ),
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_apps,
            commands::app_icon,
            commands::launch_app,
            commands::set_launcher_height,
            commands::resize_launcher,
            commands::recenter_launcher,
            commands::hide_launcher,
            commands::open_settings,
            commands::get_settings,
            commands::set_setting,
            commands::language,
            commands::restart_app,
            commands::set_shortcut,
            commands::open_keyboard_settings,
            commands::quit_app,
            commands::get_login_item,
            commands::set_login_item,
            commands::suspend_shortcut,
            commands::resume_shortcut,
            commands::access_status,
            commands::grant_folder,
            commands::list_granted_folders,
            commands::remove_granted_folder,
            commands::index_status,
            commands::reindex_now,
            commands::indexed_apps,
            commands::set_app_disabled,
            commands::set_folder_disabled,
            commands::set_apps_disabled,
            commands::get_favorites,
            commands::set_favorites,
            commands::favorites_status,
            commands::unlock_favorites_session,
            commands::purchase_favorites,
            commands::restore_favorites,
            commands::notify_settings_ready,
            commands::dev_log,
            commands::get_dev_logs,
            commands::clear_dev_logs,
            commands::reveal_dev_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
