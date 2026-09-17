//! Menu-bar tray icon + menu (plan 0007). On by default.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Runtime,
};

use crate::i18n::{self, Key};

/// Build the tray menu, localized to the current language. Extracted so it can be
/// rebuilt (via `tray.set_menu`) when the language changes.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let open = MenuItem::with_id(app, "open", i18n::t(app, Key::TrayOpen), true, None::<&str>)?;
    let settings = MenuItem::with_id(
        app,
        "settings",
        i18n::t(app, Key::TraySettings),
        true,
        None::<&str>,
    )?;
    let reindex = MenuItem::with_id(
        app,
        "reindex",
        i18n::t(app, Key::TrayReindex),
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", i18n::t(app, Key::TrayQuit), true, None::<&str>)?;
    Menu::with_items(app, &[&open, &settings, &reindex, &sep, &quit])
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    // Dedicated monochrome menu-bar glyph (a template image — macOS tints it for
    // light/dark). NOT the full-colour app icon. Regenerate via `mise run make-icons`.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/menubar.png"))
        .expect("menu-bar icon");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = crate::commands::show_launcher(app);
            }
            "settings" => {
                let _ = crate::commands::open_settings(app.clone());
            }
            "reindex" => crate::reindex(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
