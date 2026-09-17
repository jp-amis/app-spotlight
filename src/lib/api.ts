// Typed wrappers over the Tauri command surface (see src-tauri/src/commands.rs).
import { invoke } from "@tauri-apps/api/core";
import type { AppEntry, Frecency } from "./fuzzy";

export interface Settings {
  shortcut: string;
  menubar_visible: boolean;
  theme: "system" | "light" | "dark";
  launcher_style: "glass" | "glass_clear" | "vibrancy";
  language: "system" | "en-US" | "pt-BR";
  font_scale: "smaller" | "normal" | "bigger" | "extra_big";
  result_limit: number;
  frecency: Frecency;
}

export interface LanguageInfo {
  setting: "system" | "en-US" | "pt-BR";
  effective: "en-US" | "pt-BR";
}
export const getLanguage = () => invoke<LanguageInfo>("language");
export const restartApp = () => invoke<void>("restart_app");

export const listApps = () => invoke<AppEntry[]>("list_apps");

export const getFavorites = () => invoke<string[]>("get_favorites");
export const setFavorites = (paths: string[]) =>
  invoke<void>("set_favorites", { paths });

export interface FavoritesStatus {
  unlocked: boolean; // usable now (purchased OR session-unlocked)
  purchased: boolean; // owned forever
  purchasable: boolean; // StoreKit product available (App Store build)
  price: string | null; // localized price, when purchasable
}
export const favoritesStatus = () =>
  invoke<FavoritesStatus>("favorites_status");
export const unlockFavoritesSession = () =>
  invoke<void>("unlock_favorites_session");
export const purchaseFavorites = () => invoke<void>("purchase_favorites");
export const restoreFavorites = () => invoke<boolean>("restore_favorites");

export const getLoginItem = () => invoke<boolean>("get_login_item");
export const setLoginItem = (enabled: boolean) =>
  invoke<void>("set_login_item", { enabled });

export const quitApp = () => invoke<void>("quit_app");
export const openKeyboardSettings = () =>
  invoke<void>("open_keyboard_settings");

export interface AccessStatus {
  limited: boolean;
  suggested_path: string;
}
export const accessStatus = () => invoke<AccessStatus>("access_status");
export const grantFolder = () => invoke<string | null>("grant_folder");
export const listGrantedFolders = () => invoke<string[]>("list_granted_folders");
export const removeGrantedFolder = (path: string) =>
  invoke<void>("remove_granted_folder", { path });

export interface DirInfo {
  path: string;
  kind: "system" | "granted";
  count: number;
  readable: boolean;
  disabled: boolean;
}
export interface IndexStatus {
  indexing: boolean;
  count: number;
  last_indexed_ms: number | null;
  dirs: DirInfo[];
}
export const indexStatus = () => invoke<IndexStatus>("index_status");
export const reindexNow = () => invoke<void>("reindex_now");

export interface IndexedApp extends AppEntry {
  enabled: boolean;
}
export const indexedApps = () => invoke<IndexedApp[]>("indexed_apps");
export const setAppDisabled = (path: string, disabled: boolean) =>
  invoke<void>("set_app_disabled", { path, disabled });
export const setFolderDisabled = (path: string, disabled: boolean) =>
  invoke<void>("set_folder_disabled", { path, disabled });
export const setAppsDisabled = (paths: string[], disabled: boolean) =>
  invoke<void>("set_apps_disabled", { paths, disabled });

export const appIcon = (path: string) => invoke<string | null>("app_icon", { path });
export const launchApp = (path: string) => invoke<void>("launch_app", { path });
export const hideLauncher = () => invoke<void>("hide_launcher");
export const recenterLauncher = () => invoke<void>("recenter_launcher");
export const setLauncherHeight = (height: number) =>
  invoke<void>("set_launcher_height", { height }).catch(() => {});
export const openSettings = () => invoke<void>("open_settings");
export const getSettings = () => invoke<Settings>("get_settings");
export const setSetting = (key: keyof Settings, value: unknown) =>
  invoke<void>("set_setting", { key, value });
export const setShortcut = (accelerator: string) =>
  invoke<void>("set_shortcut", { accelerator });
export const suspendShortcut = () => invoke<void>("suspend_shortcut");
export const resumeShortcut = () => invoke<void>("resume_shortcut");
