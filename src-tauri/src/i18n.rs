//! App localization for the native (Rust) surfaces: the menu-bar tray menu, the
//! native app menu titles, the Settings window title, the folder-picker prompt,
//! and user-facing error strings.
//!
//! The frontend has its own catalog (`src/lib/i18n.ts`); this module covers only
//! the strings produced on the Rust side. Supported languages: en-US, pt-BR.
//! pt-BR wording follows Apple's own macOS terminology so it feels native.

use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

/// A supported UI language. The stored `language` setting can also be `"system"`,
/// which resolves to one of these via the OS locale (see [`resolve`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    EnUs,
    PtBr,
}

impl Lang {
    /// BCP-47-ish tag used in the store and over the wire to the frontend.
    pub fn as_tag(self) -> &'static str {
        match self {
            Lang::EnUs => "en-US",
            Lang::PtBr => "pt-BR",
        }
    }
}

/// Detect the OS UI language and map it to a supported [`Lang`]: Portuguese in any
/// region maps to pt-BR; everything else falls back to en-US (requirement #3).
pub fn detect_os() -> Lang {
    match sys_locale::get_locale() {
        Some(l) if l.to_ascii_lowercase().starts_with("pt") => Lang::PtBr,
        _ => Lang::EnUs,
    }
}

/// The raw stored `language` preference: "system" | "en-US" | "pt-BR" ("system" if unset).
pub fn setting<R: Runtime>(app: &AppHandle<R>) -> String {
    app.store(crate::commands::SETTINGS_FILE)
        .ok()
        .and_then(|s| s.get("language"))
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "system".into())
}

/// Resolve the effective language from the stored setting (or the OS when "system").
pub fn resolve<R: Runtime>(app: &AppHandle<R>) -> Lang {
    match setting(app).as_str() {
        "en-US" => Lang::EnUs,
        "pt-BR" => Lang::PtBr,
        _ => detect_os(),
    }
}

/// Keys for the static backend strings. Dynamic (interpolated) strings have their
/// own helper functions below.
#[derive(Clone, Copy)]
pub enum Key {
    // Tray menu
    TrayOpen,
    TraySettings,
    TrayReindex,
    TrayQuit,
    // Native app menu submenu titles
    MenuEdit,
    MenuWindow,
    // Windows
    SettingsTitle,
    // Folder picker (NSOpenPanel)
    PickerPrompt,
    PickerMessage,
    // Errors
    ErrUnlockFavorites,
    IapCancelled,
    IapUnavailable,
    IapPurchaseFailed,
    IapRestoreFailed,
    IapStoreOnly,
    LoginNeedsMacos13,
    LoginServiceUnavailable,
    LoginUnknownError,
}

/// Translate a static key for an already-resolved language.
pub fn tr(lang: Lang, key: Key) -> &'static str {
    use Key::*;
    use Lang::*;
    match (lang, key) {
        (EnUs, TrayOpen) => "Open My App Spot",
        (PtBr, TrayOpen) => "Abrir o My App Spot",
        (EnUs, TraySettings) => "Settings...",
        (PtBr, TraySettings) => "Ajustes...",
        (EnUs, TrayReindex) => "Reindex Apps",
        (PtBr, TrayReindex) => "Reindexar apps",
        (EnUs, TrayQuit) => "Quit",
        (PtBr, TrayQuit) => "Encerrar",

        (EnUs, MenuEdit) => "Edit",
        (PtBr, MenuEdit) => "Editar",
        (EnUs, MenuWindow) => "Window",
        (PtBr, MenuWindow) => "Janela",

        (EnUs, SettingsTitle) => "My App Spot Settings",
        (PtBr, SettingsTitle) => "Ajustes do My App Spot",

        (EnUs, PickerPrompt) => "Grant Access",
        (PtBr, PickerPrompt) => "Conceder acesso",
        (EnUs, PickerMessage) => {
            "Choose a folder for My App Spot to search (e.g. your Applications folder)."
        }
        (PtBr, PickerMessage) => {
            "Escolha uma pasta para o My App Spot buscar (por exemplo, sua pasta de Aplicativos)."
        }

        (EnUs, ErrUnlockFavorites) => "Unlock favorites first.",
        (PtBr, ErrUnlockFavorites) => "Desbloqueie os favoritos primeiro.",
        // Stable sentinel (never shown to the user; the frontend swallows it),
        // so it stays language-independent.
        (EnUs, IapCancelled) => "cancelled",
        (PtBr, IapCancelled) => "cancelled",
        (EnUs, IapUnavailable) => "Purchasing isn't available right now.",
        (PtBr, IapUnavailable) => "As compras não estão disponíveis no momento.",
        (EnUs, IapPurchaseFailed) => "The purchase couldn't be completed.",
        (PtBr, IapPurchaseFailed) => "Não foi possível concluir a compra.",
        (EnUs, IapRestoreFailed) => "Restore failed.",
        (PtBr, IapRestoreFailed) => "A restauração falhou.",
        (EnUs, IapStoreOnly) => "In-app purchase is only available in the App Store build.",
        (PtBr, IapStoreOnly) => "As compras no app só estão disponíveis na versão da App Store.",
        (EnUs, LoginNeedsMacos13) => "Launch at login requires macOS 13 or later",
        (PtBr, LoginNeedsMacos13) => "Abrir ao iniciar a sessão requer o macOS 13 ou posterior",
        (EnUs, LoginServiceUnavailable) => "SMAppService.mainApp unavailable",
        (PtBr, LoginServiceUnavailable) => "SMAppService.mainApp indisponível",
        (EnUs, LoginUnknownError) => "unknown error",
        (PtBr, LoginUnknownError) => "erro desconhecido",
    }
}

/// Translate a static key using the app's currently resolved language.
pub fn t<R: Runtime>(app: &AppHandle<R>, key: Key) -> String {
    tr(resolve(app), key).to_string()
}

/// "<path> is already searched (via <base>)." — the grant-folder rejection message.
pub fn already_searched(lang: Lang, path: &str, base: &str) -> String {
    match lang {
        Lang::EnUs => format!("\"{path}\" is already searched (via {base})."),
        Lang::PtBr => format!("\"{path}\" já está na busca (via {base})."),
    }
}
