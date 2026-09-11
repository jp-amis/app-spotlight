//! Global shortcut registration (plan 0004). Default is CmdOrCtrl+Shift+Space.

use std::str::FromStr;

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> Result<(), String> {
    let shortcut = Shortcut::from_str(accelerator).map_err(|e| e.to_string())?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| e.to_string())
}

pub fn unregister_all<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.global_shortcut().unregister_all();
}
