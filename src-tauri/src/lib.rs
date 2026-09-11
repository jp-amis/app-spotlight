//! Spotlight — a fast, keyboard-first macOS app launcher (Tauri v2).

mod apps;
mod commands;
mod shortcut;
mod tray;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod permissions;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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

pub fn run() {
    tauri::Builder::default()
        // Custom menu WITHOUT a Quit item, so ⌘Q doesn't quit this menu-bar app. Keeps
        // Edit (copy/paste/select-all) and Window→Close (⌘W). Quit only via the tray.
        .menu(|handle| {
            use tauri::menu::{Menu, PredefinedMenuItem as P, Submenu};
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
                "Edit",
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
                "Window",
                true,
                &[
                    &P::minimize(handle, None)?,
                    &P::close_window(handle, None)?,
                ],
            )?;
            Menu::with_items(handle, &[&app, &edit, &window])
        })
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
            });

            // Refresh the (possibly stale) cached index off the critical path.
            if from_cache {
                let h = handle.clone();
                std::thread::spawn(move || reindex(&h));
            }

            // Auto-refresh when apps are installed/removed (0014).
            start_folder_watcher(handle.clone(), apps::dirs_to_scan(&dirs));

            // Configure the launcher window: vibrancy + Spotlight-like float + hide-on-blur.
            if let Some(win) = app.get_webview_window("launcher") {
                #[cfg(target_os = "macos")]
                {
                    use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
                    let _ = apply_vibrancy(
                        &win,
                        NSVisualEffectMaterial::HudWindow,
                        Some(NSVisualEffectState::Active),
                        Some(16.0),
                    );
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
                        // Blur-hide via the command so it also saves the position.
                        let _ = commands::hide_launcher(hc.clone());
                    }
                });
            }

            // Apply the saved theme to the launcher window's native appearance
            // (cascades to the webview's prefers-color-scheme).
            commands::apply_theme(&handle);

            tray::build(&handle)?;

            // Register the global shortcut from settings (default cmd+shift+space).
            let accel = commands::current_shortcut(&handle);
            if let Err(e) = shortcut::register(&handle, &accel) {
                eprintln!("global shortcut '{accel}' failed to register: {e}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_apps,
            commands::app_icon,
            commands::launch_app,
            commands::set_launcher_height,
            commands::recenter_launcher,
            commands::hide_launcher,
            commands::open_settings,
            commands::get_settings,
            commands::set_setting,
            commands::set_shortcut,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
