// Lightweight, reactive i18n for the two windows (launcher + settings).
//
// The effective language is owned by the Rust backend (it also localizes the native
// menu/tray); this module mirrors it into a Solid signal and exposes a reactive
// `t()`. Any JSX that reads `t(...)` re-renders when the language changes, because
// `t()` reads the `locale()` signal. pt-BR wording follows Apple's macOS terminology.
import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { getLanguage } from "./api";

export type Lang = "en-US" | "pt-BR";

// English is the source of truth for the key set; pt-BR must cover the same keys.
const en = {
  // ---- launcher ----
  "launcher.search": "Search apps...",
  "launcher.openSettings": "Open settings",
  "launcher.accessBanner":
    "Some apps may be hidden. My App Spot is sandboxed, so grant a folder (e.g. your Applications) to find more.",
  "launcher.grantAccess": "Grant Access",
  "launcher.dismiss": "Dismiss",
  "launcher.pinned": "Pinned",
  "launcher.noMatch": "No matching apps",
  "launcher.startTyping": "Start typing to search",
  "launcher.indexing": "Finding your apps...",

  // ---- settings chrome ----
  "settings.title": "My App Spot Settings",
  "settings.loading": "Loading...",
  "tab.general": "General",
  "tab.favorites": "Favorites",
  "tab.index": "Index & Permissions",
  "tab.shortcuts": "Shortcuts",

  // ---- general tab ----
  "section.shortcut": "Shortcut",
  "shortcut.want": "Want",
  "key.space": "Space",
  "shortcut.reserved": "? macOS reserves it for Spotlight.",
  "shortcut.openKeyboard": "Open Keyboard settings",
  "shortcut.thenChoose":
    ", then choose Keyboard Shortcuts > Spotlight and uncheck \"Show Spotlight search\" to free it up.",
  "field.globalHotkey": "Global hotkey",
  "section.general": "General",
  "toggle.openAtLogin": "Open at login",
  "toggle.menubar": "Show menu-bar icon",
  "field.resultsShown": "Results shown",
  "section.appearance": "Appearance",
  "field.theme": "Theme",
  "theme.system": "System",
  "theme.light": "Light",
  "theme.dark": "Dark",
  "field.launcherStyle": "Launcher style",
  "style.glass": "Liquid Glass",
  "style.clear": "Clear Glass",
  "style.frosted": "Frosted",
  "field.textSize": "Text size",
  "size.smaller": "Smaller",
  "size.normal": "Normal",
  "size.bigger": "Bigger",
  "size.xbig": "Extra big",
  "field.language": "Language",
  "lang.system": "System",
  "lang.restartNote": "Restart to finish applying the language change.",
  "lang.restartNow": "Restart now",
  "button.quit": "Quit My App Spot",

  // ---- favorites: paywall ----
  "fav.unlockTitle": "Unlock Favorites",
  "fav.unlockDescA": "Pin up to {max} apps to the top of the launcher, each with a",
  "fav.unlockDescB":
    "shortcut. A one-time unlock that helps support development. Thank you!",
  "fav.unlockForeverPriced": "Unlock forever ({price})",
  "fav.unlockForever": "Unlock forever",
  "fav.restore": "Restore purchase",
  "fav.freePre": "Don't want to pay? No worries, you can unlock favorites",
  "fav.freeLink": "free",
  "fav.freePost": "for this session (it stays unlocked until you quit the app).",
  "fav.appStoreOnly": "Buying is only available in the App Store version.",
  "fav.noPreviousPurchase": "No previous purchase to restore.",

  // ---- favorites: unlocked ----
  "fav.introA": "Pinned apps appear at the top of the launcher, with",
  "fav.introB":
    "shortcuts, the moment you open it, before you type anything. Up to {max}.",
  "fav.pinnedCount": "Pinned ({n}/{max})",
  "fav.noPinned": "No pinned apps yet. Pin some from the list below.",
  "fav.dragReorder": "Drag to reorder",
  "fav.moveUp": "Move up",
  "fav.moveDown": "Move down",
  "fav.unpin": "Unpin",
  "fav.addApps": "Add apps",
  "fav.limitReached": "(limit reached)",
  "fav.searchToPin": "Search apps to pin...",
  "fav.noApps": "No apps",
  "fav.opens": "{n} opens",
  "fav.pin": "Pin",

  // ---- index & permissions tab ----
  "index.ofApps": "of {total} apps",
  "index.off": "{n} off",
  "index.indexing": "Indexing...",
  "index.reindex": "Reindex",
  "index.searchApps": "Search apps...",
  "index.addFolder": "Add Folder...",
  "index.explainA":
    "Sandboxed, so it always searches the system app folders; grant extra folders (like",
  "index.explainB":
    "). Turn a folder or app off to hide it from results (without removing it).",
  "index.folderDisabled": "Folder disabled",
  "index.folderEnabled": "Folder enabled",
  "index.kind.system": "system",
  "index.kind.granted": "granted",
  "index.appsCount": "{n} apps",
  "index.notReadable": "Not readable",
  "index.remove": "Remove",
  "index.checkAll": "Check all",
  "index.uncheckAll": "Uncheck all",
  "index.enabled": "Enabled",
  "index.disabled": "Disabled",
  "index.updatedNow": "· updated just now",
  "index.updatedSecs": "· updated {n}s ago",
  "index.updatedMins": "· updated {n}m ago",
  "index.updatedHours": "· updated {n}h ago",

  // ---- shortcuts (help) tab ----
  "section.launcherShortcuts": "Launcher shortcuts",
  "sc.openToggle": "Open / toggle launcher",
  "sc.launchSelected": "Launch selected",
  "sc.launchNth": "Launch 1st to 10th result",
  "sc.moveSelection": "Move selection",
  "sc.clearClose": "Clear query / close",
  "sc.recenter": "Re-center the launcher window",
  "sc.openSettings": "Open settings",
  "sc.pinUnpin": "Pin / unpin selected app",
  "sc.movePinned": "Move pinned app up / down",
  "section.tips": "Tips",
  "tips.bodyA":
    "The launcher lives in the menu bar and opens on the screen your cursor is on, remembering a position per monitor. Drag it to reposition; press",
  "tips.bodyB": "while it's open to snap it back to the default spot.",

  // ---- shortcut capture ----
  "shortcut.pressKeys": "Press keys...",

  // ---- developer tab (hidden; ⌘⇧D) ----
  "tab.developer": "Developer",
  "fav.purchasing": "Processing...",
  "dev.title": "Diagnostic log",
  "dev.desc":
    "Records low-level events (such as launcher window reloads) to a local file to help diagnose issues. Off by default.",
  "dev.enable": "Enable logging",
  "dev.hint":
    "Turn on logging, reproduce the issue, then Refresh to see entries. Press ⌘⇧D to hide this tab.",
  "dev.refresh": "Refresh",
  "dev.clear": "Clear",
  "dev.copy": "Copy",
  "dev.copied": "Copied",
  "dev.reveal": "Reveal in Finder",
  "dev.empty": "No log entries yet.",

  // ---- shared ----
  "common.to": "to",
  "common.or": "or",
} as const;

export type MsgKey = keyof typeof en;

const ptBR: Record<MsgKey, string> = {
  // ---- launcher ----
  "launcher.search": "Buscar apps...",
  "launcher.openSettings": "Abrir os ajustes",
  "launcher.accessBanner":
    "Alguns apps podem estar ocultos. O My App Spot roda em sandbox, então conceda uma pasta (por exemplo, sua pasta de Aplicativos) para encontrar mais.",
  "launcher.grantAccess": "Conceder acesso",
  "launcher.dismiss": "Dispensar",
  "launcher.pinned": "Fixado",
  "launcher.noMatch": "Nenhum app correspondente",
  "launcher.startTyping": "Comece a digitar para buscar",
  "launcher.indexing": "Localizando seus apps...",

  // ---- settings chrome ----
  "settings.title": "Ajustes do My App Spot",
  "settings.loading": "Carregando...",
  "tab.general": "Geral",
  "tab.favorites": "Favoritos",
  "tab.index": "Índice e Permissões",
  "tab.shortcuts": "Atalhos",

  // ---- general tab ----
  "section.shortcut": "Atalho",
  "shortcut.want": "Quer usar",
  "key.space": "Espaço",
  "shortcut.reserved": "? O macOS reserva esse atalho para o Spotlight.",
  "shortcut.openKeyboard": "Abra os Ajustes de Teclado",
  "shortcut.thenChoose":
    ", depois escolha Atalhos de Teclado > Spotlight e desmarque \"Mostrar busca do Spotlight\" para liberá-lo.",
  "field.globalHotkey": "Atalho global",
  "section.general": "Geral",
  "toggle.openAtLogin": "Abrir ao iniciar a sessão",
  "toggle.menubar": "Mostrar ícone na barra de menus",
  "field.resultsShown": "Resultados exibidos",
  "section.appearance": "Aparência",
  "field.theme": "Tema",
  "theme.system": "Automática",
  "theme.light": "Clara",
  "theme.dark": "Escura",
  "field.launcherStyle": "Estilo do launcher",
  "style.glass": "Vidro Líquido",
  "style.clear": "Vidro Transparente",
  "style.frosted": "Vidro Fosco",
  "field.textSize": "Tamanho do texto",
  "size.smaller": "Menor",
  "size.normal": "Normal",
  "size.bigger": "Maior",
  "size.xbig": "Extragrande",
  "field.language": "Idioma",
  "lang.system": "Automático",
  "lang.restartNote": "Reinicie para concluir a mudança de idioma.",
  "lang.restartNow": "Reiniciar agora",
  "button.quit": "Encerrar o My App Spot",

  // ---- favorites: paywall ----
  "fav.unlockTitle": "Desbloquear Favoritos",
  "fav.unlockDescA": "Fixe até {max} apps no topo do launcher, cada um com um atalho",
  "fav.unlockDescB":
    ". Um desbloqueio único que ajuda a apoiar o desenvolvimento. Obrigado!",
  "fav.unlockForeverPriced": "Desbloquear para sempre ({price})",
  "fav.unlockForever": "Desbloquear para sempre",
  "fav.restore": "Restaurar compra",
  "fav.freePre":
    "Não quer pagar? Sem problemas, você pode desbloquear os favoritos",
  "fav.freeLink": "de graça",
  "fav.freePost": "nesta sessão (fica desbloqueado até você encerrar o app).",
  "fav.appStoreOnly": "A compra só está disponível na versão da App Store.",
  "fav.noPreviousPurchase": "Nenhuma compra anterior para restaurar.",

  // ---- favorites: unlocked ----
  "fav.introA": "Os apps fixados aparecem no topo do launcher, com os atalhos",
  "fav.introB":
    ", assim que você o abre, antes de digitar qualquer coisa. Até {max}.",
  "fav.pinnedCount": "Fixados ({n}/{max})",
  "fav.noPinned": "Nenhum app fixado ainda. Fixe alguns na lista abaixo.",
  "fav.dragReorder": "Arraste para reordenar",
  "fav.moveUp": "Mover para cima",
  "fav.moveDown": "Mover para baixo",
  "fav.unpin": "Desafixar",
  "fav.addApps": "Adicionar apps",
  "fav.limitReached": "(limite atingido)",
  "fav.searchToPin": "Buscar apps para fixar...",
  "fav.noApps": "Nenhum app",
  "fav.opens": "{n} aberturas",
  "fav.pin": "Fixar",

  // ---- index & permissions tab ----
  "index.ofApps": "de {total} apps",
  "index.off": "{n} desativados",
  "index.indexing": "Indexando...",
  "index.reindex": "Reindexar",
  "index.searchApps": "Buscar apps...",
  "index.addFolder": "Adicionar pasta...",
  "index.explainA":
    "Em sandbox, então sempre busca nas pastas de apps do sistema; conceda pastas extras (como",
  "index.explainB":
    "). Desative uma pasta ou app para ocultá-lo dos resultados (sem removê-lo).",
  "index.folderDisabled": "Pasta desativada",
  "index.folderEnabled": "Pasta ativada",
  "index.kind.system": "sistema",
  "index.kind.granted": "concedida",
  "index.appsCount": "{n} apps",
  "index.notReadable": "Não legível",
  "index.remove": "Remover",
  "index.checkAll": "Marcar todos",
  "index.uncheckAll": "Desmarcar todos",
  "index.enabled": "Ativado",
  "index.disabled": "Desativado",
  "index.updatedNow": "· atualizado agora",
  "index.updatedSecs": "· atualizado há {n}s",
  "index.updatedMins": "· atualizado há {n}min",
  "index.updatedHours": "· atualizado há {n}h",

  // ---- shortcuts (help) tab ----
  "section.launcherShortcuts": "Atalhos do launcher",
  "sc.openToggle": "Abrir / alternar o launcher",
  "sc.launchSelected": "Abrir o selecionado",
  "sc.launchNth": "Abrir do 1º ao 10º resultado",
  "sc.moveSelection": "Mover a seleção",
  "sc.clearClose": "Limpar busca / fechar",
  "sc.recenter": "Recentralizar a janela do launcher",
  "sc.openSettings": "Abrir os ajustes",
  "sc.pinUnpin": "Fixar / desafixar o app selecionado",
  "sc.movePinned": "Mover o app fixado para cima / baixo",
  "section.tips": "Dicas",
  "tips.bodyA":
    "O launcher fica na barra de menus e abre na tela onde está o cursor, lembrando uma posição por monitor. Arraste para reposicionar; pressione",
  "tips.bodyB": "enquanto ele estiver aberto para voltar à posição padrão.",

  // ---- shortcut capture ----
  "shortcut.pressKeys": "Pressione as teclas...",

  // ---- developer tab (hidden; ⌘⇧D) ----
  "tab.developer": "Desenvolvedor",
  "fav.purchasing": "Processando...",
  "dev.title": "Registro de diagnóstico",
  "dev.desc":
    "Registra eventos de baixo nível (como recarregamentos da janela do launcher) em um arquivo local para ajudar a diagnosticar problemas. Desativado por padrão.",
  "dev.enable": "Ativar registro",
  "dev.hint":
    "Ative o registro, reproduza o problema e toque em Atualizar para ver as entradas. Pressione ⌘⇧D para ocultar esta aba.",
  "dev.refresh": "Atualizar",
  "dev.clear": "Limpar",
  "dev.copy": "Copiar",
  "dev.copied": "Copiado",
  "dev.reveal": "Mostrar no Finder",
  "dev.empty": "Nenhuma entrada de registro ainda.",

  // ---- shared ----
  "common.to": "a",
  "common.or": "ou",
};

const catalogs: Record<Lang, Record<MsgKey, string>> = {
  "en-US": en,
  "pt-BR": ptBR,
};

const [locale, setLocale] = createSignal<Lang>("en-US");
export { locale };

type Params = Record<string, string | number>;

function interpolate(s: string, params?: Params): string {
  if (!params) return s;
  return s.replace(/\{(\w+)\}/g, (_, k: string) =>
    k in params ? String(params[k]) : `{${k}}`,
  );
}

/** Reactive translation: reads `locale()`, so JSX using it updates on a lang change. */
export function t(key: MsgKey, params?: Params): string {
  const table = catalogs[locale()] ?? en;
  return interpolate(table[key] ?? en[key], params);
}

let initialized = false;

/** Load the effective language from the backend and keep it in sync across windows. */
export async function initI18n(): Promise<void> {
  try {
    const { effective } = await getLanguage();
    if (effective === "pt-BR" || effective === "en-US") setLocale(effective);
  } catch {
    /* no backend (dev/browser) — stay on en-US */
  }
  if (initialized) return;
  initialized = true;
  void listen<{ lang?: string }>("language:changed", (e) => {
    const lang = e.payload?.lang;
    if (lang === "pt-BR" || lang === "en-US") setLocale(lang);
  });
}
