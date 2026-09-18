import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  clearDevLogs,
  favoritesStatus,
  getDevLogs,
  revealDevLogs,
  getFavorites,
  getLoginItem,
  getSettings,
  grantFolder,
  notifySettingsReady,
  indexedApps,
  indexStatus,
  openKeyboardSettings,
  purchaseFavorites,
  quitApp,
  reindexNow,
  removeGrantedFolder,
  restoreFavorites,
  setAppDisabled,
  setAppsDisabled,
  setFavorites as saveFavoritesApi,
  setLoginItem as setLoginItemApi,
  setFolderDisabled,
  setSetting,
  setShortcut,
  unlockFavoritesSession,
  restartApp,
  type DirInfo,
  type FavoritesStatus,
  type IndexedApp,
  type IndexStatus,
  type Settings as SettingsData,
} from "../lib/api";
import AppIcon from "../launcher/AppIcon";
import { acceleratorToSymbols } from "../lib/accelerator";
import ShortcutCapture from "./ShortcutCapture";
import { initI18n, t } from "../lib/i18n";
import { initFontScale } from "../lib/fontscale";

type Tab = "general" | "favorites" | "index" | "help" | "developer";

const BASE_TABS: Tab[] = ["general", "favorites", "index", "help"];

export default function Settings() {
  const [tab, setTab] = createSignal<Tab>("general");
  const tabRefs: Partial<Record<Tab, HTMLButtonElement>> = {};
  // The Developer tab is hidden until ⌘⇧D reveals it (stays for the session).
  const [devVisible, setDevVisible] = createSignal(false);
  const tabOrder = (): Tab[] =>
    devVisible() ? [...BASE_TABS, "developer"] : BASE_TABS;

  // Arrow keys move between tabs (ARIA tablist), keeping focus on the tab.
  function onTabsKeyDown(e: KeyboardEvent) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const order = tabOrder();
    const i = order.indexOf(tab());
    const delta = e.key === "ArrowRight" ? 1 : order.length - 1;
    const next = order[(i + delta) % order.length];
    setTab(next);
    tabRefs[next]?.focus();
  }
  const [data, setData] = createSignal<SettingsData>();
  const [status, setStatus] = createSignal<IndexStatus>();
  const [allApps, setAllApps] = createSignal<IndexedApp[]>([]);
  const [query, setQuery] = createSignal("");
  const [expandedDir, setExpandedDir] = createSignal<string | null>(null);
  const [favorites, setFavorites] = createSignal<string[]>([]);
  const [favFilter, setFavFilter] = createSignal("");
  const [favStatus, setFavStatus] = createSignal<FavoritesStatus>();
  // True while a StoreKit purchase/restore is in flight (drives the button spinner).
  const [purchasing, setPurchasing] = createSignal(false);
  // Developer tab: the diagnostic log text and its loading state.
  const [devLogs, setDevLogs] = createSignal("");
  const [loginItem, setLoginItem] = createSignal(false);
  const [error, setError] = createSignal("");
  // Set if a language change couldn't be applied live to a native surface.
  const [langNeedsRestart, setLangNeedsRestart] = createSignal(false);

  const favUnlocked = () => favStatus()?.unlocked ?? false;
  async function refreshFavStatus() {
    try {
      setFavStatus(await favoritesStatus());
    } catch {
      /* no backend */
    }
  }
  // Free: unlock favorites for this session (until the app quits).
  async function unlockSession() {
    setError("");
    try {
      await unlockFavoritesSession();
      await refreshFavStatus();
    } catch (e) {
      setError(String(e));
    }
  }
  // Buy: unlock forever (StoreKit; unavailable outside the App Store build).
  async function buyUnlock() {
    if (purchasing()) return; // guard against double-invoke while the sheet is up
    setError("");
    setPurchasing(true);
    try {
      await purchaseFavorites();
      await refreshFavStatus();
    } catch (e) {
      if (String(e) !== "cancelled") setError(String(e));
    } finally {
      setPurchasing(false);
    }
  }
  async function restoreUnlock() {
    if (purchasing()) return;
    setError("");
    setPurchasing(true);
    try {
      const ok = await restoreFavorites();
      await refreshFavStatus();
      if (!ok) setError(t("fav.noPreviousPurchase"));
    } catch (e) {
      setError(String(e));
    } finally {
      setPurchasing(false);
    }
  }

  // Developer tab helpers.
  async function loadDevLogs() {
    try {
      setDevLogs(await getDevLogs());
    } catch {
      /* no backend */
    }
  }
  async function clearLogs() {
    try {
      await clearDevLogs();
      setDevLogs("");
    } catch {
      /* ignore */
    }
  }
  const [copied, setCopied] = createSignal(false);
  async function copyLogs() {
    try {
      await navigator.clipboard.writeText(formatDevLogs(devLogs()));
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable */
    }
  }
  async function toggleDevLogging(v: boolean) {
    await update("dev_logging", v);
  }

  // Toggle "open at login" (macOS SMAppService); optimistic, reverts on failure.
  async function toggleLoginItem(v: boolean) {
    setError("");
    setLoginItem(v);
    try {
      await setLoginItemApi(v);
    } catch (e) {
      setLoginItem(!v);
      setError(String(e));
    }
  }

  const MAX_FAVORITES = 10;
  const appByPath = (path: string) => allApps().find((a) => a.path === path);
  const opensOf = (path: string) =>
    (data()?.frecency as Record<string, { count?: number }> | undefined)?.[path]?.count ?? 0;

  // Apps that can be pinned: not already favorited, matching the filter, most-opened first.
  const favSuggestions = () => {
    const favSet = new Set(favorites());
    const q = favFilter().trim().toLowerCase();
    return allApps()
      .filter((a) => !favSet.has(a.path))
      .filter(
        (a) =>
          !q ||
          a.name.toLowerCase().includes(q) ||
          (a.bundle_id ?? "").toLowerCase().includes(q),
      )
      .sort((a, b) => opensOf(b.path) - opensOf(a.path) || a.name.localeCompare(b.name));
  };

  async function saveFavorites(list: string[]) {
    const capped = list.slice(0, MAX_FAVORITES);
    setFavorites(capped);
    try {
      await saveFavoritesApi(capped);
    } catch (e) {
      setError(String(e));
    }
  }
  const pinApp = (path: string) => void saveFavorites([...favorites(), path]);
  const unpinApp = (path: string) =>
    void saveFavorites(favorites().filter((p) => p !== path));
  function moveFavorite(i: number, dir: -1 | 1) {
    const list = [...favorites()];
    const j = i + dir;
    if (j < 0 || j >= list.length) return;
    [list[i], list[j]] = [list[j], list[i]];
    void saveFavorites(list);
  }
  // Drag-to-reorder the pinned list. Native HTML5 DnD is flaky in the webview,
  // so we track it with pointer events. `dragIndex` is the row being dragged;
  // `dropIndex` is the slot the pointer is currently over.
  const [dragIndex, setDragIndex] = createSignal<number | null>(null);
  const [dropIndex, setDropIndex] = createSignal<number | null>(null);
  let favRefs: (HTMLLIElement | undefined)[] = [];

  function reorderFavorite(from: number, to: number) {
    if (from === to) return;
    const list = [...favorites()];
    const [moved] = list.splice(from, 1);
    list.splice(to > from ? to - 1 : to, 0, moved);
    void saveFavorites(list);
  }

  // Which slot (0..len) does clientY fall into — before the row whose top half
  // it's above, else at the end.
  function slotAt(clientY: number): number {
    const n = favorites().length;
    for (let i = 0; i < n; i++) {
      const r = favRefs[i]?.getBoundingClientRect();
      if (r && clientY < r.top + r.height / 2) return i;
    }
    return n;
  }

  function startDrag(i: number, e: PointerEvent) {
    e.preventDefault();
    setDragIndex(i);
    setDropIndex(i);
    const onMove = (ev: PointerEvent) => setDropIndex(slotAt(ev.clientY));
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      const from = dragIndex();
      const to = dropIndex();
      if (from !== null && to !== null) reorderFavorite(from, to);
      setDragIndex(null);
      setDropIndex(null);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }

  const searching = () => query().trim().length > 0;

  // Counts derived from the live app list so they update on every enable/disable.
  const total = () => allApps().length;
  const enabledCount = () => allApps().filter((a) => a.enabled).length;
  const disabledCount = () => total() - enabledCount();

  // During search, only show folders that actually have a match.
  const visibleDirs = () => {
    const dirs = status()?.dirs ?? [];
    return searching() ? dirs.filter((d) => appsInDir(d.path).length > 0) : dirs;
  };

  // Apps under a folder, filtered by the search query (name / bundle id / path).
  const appsInDir = (path: string) => {
    const prefix = path.replace(/\/+$/, "") + "/";
    const q = query().trim().toLowerCase();
    return allApps()
      .filter((a) => a.path.startsWith(prefix))
      .filter(
        (a) =>
          !q ||
          a.name.toLowerCase().includes(q) ||
          (a.bundle_id ?? "").toLowerCase().includes(q) ||
          a.path.toLowerCase().includes(q),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  };

  async function refreshStatus() {
    try {
      setStatus(await indexStatus());
    } catch (e) {
      setError(String(e));
    }
  }

  async function refreshApps() {
    try {
      setAllApps(await indexedApps());
    } catch (e) {
      setError(String(e));
    }
  }

  // Toggle one app. The new `disabled` flag equals its current `enabled` state
  // (enabled → disable; disabled → enable). If we're ENABLING an app that lives in a
  // currently-disabled folder, auto re-enable the folder too (per-app prefs elsewhere
  // are untouched, so the folder's other apps keep their previous state).
  async function toggleApp(app: IndexedApp, dir: DirInfo) {
    const enabling = !app.enabled;
    try {
      await setAppDisabled(app.path, app.enabled);
      if (enabling && dir.disabled) {
        await setFolderDisabled(dir.path, false);
      }
      await refreshApps();
      await refreshStatus();
    } catch (e) {
      setError(String(e));
    }
  }

  // Bulk enable/disable every (currently shown) app in a folder. Enabling all also
  // re-enables the folder if it was off.
  async function toggleAllInDir(dir: DirInfo, disabled: boolean) {
    const paths = appsInDir(dir.path).map((a) => a.path);
    if (paths.length === 0) return;
    try {
      await setAppsDisabled(paths, disabled);
      if (!disabled && dir.disabled) await setFolderDisabled(dir.path, false);
      await refreshApps();
      await refreshStatus();
    } catch (e) {
      setError(String(e));
    }
  }

  // `disabled` is the folder's CURRENT disabled state; flip it.
  async function toggleFolder(path: string, disabled: boolean) {
    try {
      await setFolderDisabled(path, !disabled);
      await refreshStatus();
      await refreshApps();
    } catch (e) {
      setError(String(e));
    }
  }

  async function addFolder() {
    try {
      if (await grantFolder()) {
        await refreshStatus();
        await refreshApps();
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function removeFolder(path: string) {
    try {
      await removeGrantedFolder(path);
      await refreshStatus();
      await refreshApps();
    } catch (e) {
      setError(String(e));
    }
  }

  onMount(async () => {
    void initI18n();
    void initFontScale();
    // Start keyboard focus on the active tab so arrows/Tab work immediately.
    queueMicrotask(() => tabRefs[tab()]?.focus());
    try {
      setData(await getSettings());
      setFavorites(await getFavorites());
      setLoginItem(await getLoginItem());
      // The shell + General tab are populated and about to paint — tell the backend it's
      // safe to reveal the window now (no empty-pane flash). Do the slower StoreKit /
      // index queries afterwards so they can't delay the reveal.
      requestAnimationFrame(() => void notifySettingsReady());
      await refreshFavStatus();
      await refreshStatus();
      await refreshApps();
    } catch (e) {
      setError(String(e));
      // Even on error, reveal so the window can't get stuck hidden.
      void notifySettingsReady();
    }
    // Live index status while the window is open.
    const uns = await Promise.all([
      listen("index:start", () => void refreshStatus()),
      listen("index:done", () => void refreshStatus()),
      listen("apps:reindexed", () => void refreshApps()),
      // Keep the Favorites tab in sync when the launcher pins/reorders (⌘P/⌘J/⌘K).
      listen("favorites:changed", async () => {
        try {
          setFavorites(await getFavorites());
        } catch {
          /* ignore */
        }
      }),
      listen("favorites:unlocked", () => void refreshFavStatus()),
      // Reflect the raw language setting when it changes, and surface a restart
      // hint if a native surface couldn't be re-localized live.
      listen<{ needs_restart?: boolean }>("language:changed", async (e) => {
        if (e.payload?.needs_restart) setLangNeedsRestart(true);
        try {
          setData(await getSettings());
        } catch {
          /* ignore */
        }
      }),
    ]);
    // ⌘Q closes this window (the app never quits from ⌘Q — same as ⌘W).
    const onKey = (e: KeyboardEvent) => {
      // ⌘Q closes this window; ⌘⇧Q (handled by the app menu) quits the app.
      if (e.metaKey && !e.shiftKey && e.code === "KeyQ") {
        e.preventDefault();
        void getCurrentWindow().close();
      }
      // ⌘⇧D reveals (or hides) the hidden Developer tab.
      if (e.metaKey && e.shiftKey && e.code === "KeyD") {
        e.preventDefault();
        const show = !devVisible();
        setDevVisible(show);
        if (show) {
          setTab("developer");
          void loadDevLogs();
        } else if (tab() === "developer") {
          setTab("general");
        }
      }
    };
    document.addEventListener("keydown", onKey);
    onCleanup(() => {
      uns.forEach((f) => f());
      document.removeEventListener("keydown", onKey);
    });
  });

  async function update<K extends keyof SettingsData>(key: K, value: SettingsData[K]) {
    setData((d) => (d ? { ...d, [key]: value } : d));
    try {
      await setSetting(key, value);
    } catch (e) {
      setError(String(e));
    }
  }

  async function rebind(accelerator: string) {
    setError("");
    try {
      await setShortcut(accelerator);
      setData((d) => (d ? { ...d, shortcut: accelerator } : d));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div class="flex h-screen flex-col bg-neutral-50 text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
      {/* native-style title bar: centered + bold, on the traffic-light row (draggable) */}
      <div
        data-tauri-drag-region
        class="relative flex h-[28px] shrink-0 items-center justify-center"
      >
        <span class="pointer-events-none text-[0.8125rem] font-semibold text-neutral-800 dark:text-neutral-100">
          {t("settings.title")}
        </span>
      </div>

      {/* tabs — fixed while the content scrolls, with a little breathing room up top */}
      <div class="shrink-0 border-b border-black/10 px-8 pt-4 dark:border-white/10">
        <div role="tablist" onKeyDown={onTabsKeyDown} class="mx-auto flex max-w-[672px] gap-6">
          <TabButton
            active={tab() === "general"}
            onClick={() => setTab("general")}
            ref={(el) => (tabRefs.general = el)}
          >
            {t("tab.general")}
          </TabButton>
          <TabButton
            active={tab() === "favorites"}
            onClick={() => setTab("favorites")}
            ref={(el) => (tabRefs.favorites = el)}
          >
            {t("tab.favorites")}
          </TabButton>
          <TabButton
            active={tab() === "index"}
            onClick={() => setTab("index")}
            ref={(el) => (tabRefs.index = el)}
          >
            {t("tab.index")}
          </TabButton>
          <TabButton
            active={tab() === "help"}
            onClick={() => setTab("help")}
            ref={(el) => (tabRefs.help = el)}
          >
            {t("tab.shortcuts")}
          </TabButton>
          <Show when={devVisible()}>
            <TabButton
              active={tab() === "developer"}
              onClick={() => setTab("developer")}
              ref={(el) => (tabRefs.developer = el)}
            >
              {t("tab.developer")}
            </TabButton>
          </Show>
        </div>
      </div>

      {/* scrollable content */}
      <div class="flex-1 overflow-y-auto px-8 py-6">
        <div class="mx-auto max-w-[672px]">
          <Show when={data()} fallback={<p class="text-sm text-neutral-500">{t("settings.loading")}</p>}>
            {(d) => (
              <>
              {/* ---------- General tab ---------- */}
              <Show when={tab() === "general"}>
                <div class="space-y-6">
                  <Section
                    title={t("section.shortcut")}
                    footnote={
                      <>
                        {t("shortcut.want")} <Kbd>⌘</Kbd> <Kbd>{t("key.space")}</Kbd>
                        {t("shortcut.reserved")}{" "}
                        <button
                          onClick={() => void openKeyboardSettings()}
                          class="font-medium text-[var(--color-accent)] hover:underline"
                        >
                          {t("shortcut.openKeyboard")}
                        </button>
                        {t("shortcut.thenChoose")}
                      </>
                    }
                  >
                    <Field label={t("field.globalHotkey")}>
                      <ShortcutCapture value={d().shortcut} onChange={rebind} />
                    </Field>
                  </Section>

                  <Section title={t("section.general")}>
                    <Toggle
                      label={t("toggle.openAtLogin")}
                      checked={loginItem()}
                      onChange={(v) => void toggleLoginItem(v)}
                    />
                    <Toggle
                      label={t("toggle.menubar")}
                      checked={d().menubar_visible}
                      onChange={(v) => update("menubar_visible", v)}
                    />
                    <Field label={t("field.resultsShown")}>
                      <input
                        type="number"
                        min="3"
                        max="15"
                        value={d().result_limit}
                        onChange={(e) =>
                          update("result_limit", Number(e.currentTarget.value))
                        }
                        class="w-20 rounded-lg border border-black/10 bg-white px-2 py-1 text-sm dark:border-white/10 dark:bg-neutral-800"
                      />
                    </Field>
                  </Section>

                  <Section title={t("section.appearance")}>
                    <Field label={t("field.theme")}>
                      <Segmented
                        value={d().theme}
                        onChange={(v) => update("theme", v)}
                        options={[
                          { value: "system", label: t("theme.system"), icon: <IconAuto /> },
                          { value: "light", label: t("theme.light"), icon: <IconSun /> },
                          { value: "dark", label: t("theme.dark"), icon: <IconMoon /> },
                        ]}
                      />
                    </Field>
                    <Field label={t("field.textSize")}>
                      <Segmented
                        value={d().font_scale ?? "normal"}
                        onChange={(v) => update("font_scale", v)}
                        options={[
                          { value: "smaller", label: t("size.smaller") },
                          { value: "normal", label: t("size.normal") },
                          { value: "bigger", label: t("size.bigger") },
                          { value: "extra_big", label: t("size.xbig") },
                        ]}
                      />
                    </Field>
                    <Field label={t("field.launcherStyle")} align="start">
                      <div class="flex gap-3">
                        <For
                          each={
                            [
                              { value: "glass", label: t("style.glass"), overlay: "bg-white/25 backdrop-blur-sm border border-white/50" },
                              { value: "glass_clear", label: t("style.clear"), overlay: "bg-white/10 border border-white/30" },
                              { value: "vibrancy", label: t("style.frosted"), overlay: "bg-white/60 backdrop-blur-md border border-white/40" },
                            ] as const
                          }
                        >
                          {(o) => (
                            <button
                              role="radio"
                              aria-checked={(d().launcher_style ?? "glass") === o.value}
                              aria-label={o.label}
                              onClick={() => update("launcher_style", o.value)}
                              class="flex flex-col items-center gap-1.5"
                            >
                              <span
                                class="relative h-12 w-[76px] overflow-hidden rounded-lg ring-2 transition"
                                classList={{
                                  "ring-[var(--color-accent)]":
                                    (d().launcher_style ?? "glass") === o.value,
                                  "ring-transparent": (d().launcher_style ?? "glass") !== o.value,
                                }}
                              >
                                {/* mini wallpaper so translucency reads */}
                                <span class="absolute inset-0 bg-gradient-to-br from-sky-400 via-violet-500 to-fuchsia-500" />
                                {/* material overlay approximating each style */}
                                <span class={`absolute inset-2 rounded-md ${o.overlay}`} />
                              </span>
                              <span
                                class="text-xs"
                                classList={{
                                  "font-medium text-neutral-900 dark:text-neutral-100":
                                    (d().launcher_style ?? "glass") === o.value,
                                  "text-neutral-500":
                                    (d().launcher_style ?? "glass") !== o.value,
                                }}
                              >
                                {o.label}
                              </span>
                            </button>
                          )}
                        </For>
                      </div>
                    </Field>
                    <Field label={t("field.language")}>
                      <Segmented
                        value={d().language ?? "system"}
                        onChange={(v) => update("language", v)}
                        options={[
                          { value: "system", label: t("lang.system") },
                          { value: "en-US", label: "English" },
                          { value: "pt-BR", label: "Português" },
                        ]}
                      />
                    </Field>
                  </Section>

                  <Show when={langNeedsRestart()}>
                    <p class="flex items-center justify-center gap-2 text-xs text-neutral-500">
                      {t("lang.restartNote")}{" "}
                      <button
                        onClick={() => void restartApp()}
                        class="font-medium text-[var(--color-accent)] hover:underline"
                      >
                        {t("lang.restartNow")}
                      </button>
                    </p>
                  </Show>

                  <div class="flex justify-center pt-2">
                    <button
                      onClick={() => void quitApp()}
                      class="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-2 text-sm font-medium text-red-600 transition-colors hover:bg-red-500/20 dark:text-red-400"
                    >
                      {t("button.quit")}
                    </button>
                  </div>
                </div>
              </Show>

              {/* ---------- Favorites tab ---------- */}
              <Show when={tab() === "favorites"}>
                <Show
                  when={favUnlocked()}
                  fallback={
                    <div class="mx-auto max-w-md space-y-5 py-8 text-center">
                      <div class="text-4xl">★</div>
                      <h2 class="text-lg font-semibold">{t("fav.unlockTitle")}</h2>
                      <p class="text-sm leading-relaxed text-neutral-500">
                        {t("fav.unlockDescA", { max: MAX_FAVORITES })}{" "}
                        <Kbd>⌘</Kbd>
                        <Kbd>1</Kbd> {t("common.to")} <Kbd>⌘</Kbd>
                        <Kbd>0</Kbd> {t("fav.unlockDescB")}
                      </p>
                      <div class="flex flex-col items-center gap-3">
                        <button
                          onClick={() => void buyUnlock()}
                          disabled={!favStatus()?.purchasable || purchasing()}
                          class="flex w-full max-w-xs items-center justify-center gap-2 rounded-lg bg-[var(--color-accent)] px-4 py-2.5 text-sm font-semibold text-white transition-colors hover:brightness-110 disabled:opacity-40"
                        >
                          <Show when={purchasing()}>
                            <Spinner />
                          </Show>
                          {purchasing()
                            ? t("fav.purchasing")
                            : favStatus()?.purchasable
                              ? t("fav.unlockForeverPriced", { price: favStatus()?.price ?? "" })
                              : t("fav.unlockForever")}
                        </button>
                        <button
                          onClick={() => void restoreUnlock()}
                          disabled={purchasing()}
                          class="text-xs text-neutral-500 hover:underline disabled:opacity-40"
                        >
                          {t("fav.restore")}
                        </button>
                      </div>
                      <p class="text-xs leading-relaxed text-neutral-400">
                        {t("fav.freePre")}{" "}
                        <button
                          onClick={() => void unlockSession()}
                          class="font-semibold text-[var(--color-accent)] hover:underline"
                        >
                          {t("fav.freeLink")}
                        </button>{" "}
                        {t("fav.freePost")}
                        <Show when={!favStatus()?.purchasable}>
                          {" "}
                          {t("fav.appStoreOnly")}
                        </Show>
                      </p>
                    </div>
                  }
                >
                <div class="space-y-6">
                  <p class="text-xs leading-relaxed text-neutral-500">
                    {t("fav.introA")} <Kbd>⌘</Kbd>
                    <Kbd>1</Kbd> {t("common.to")} <Kbd>⌘</Kbd>
                    <Kbd>0</Kbd> {t("fav.introB", { max: MAX_FAVORITES })}
                  </p>

                  {/* pinned, ordered */}
                  <div class="space-y-2">
                    <h2 class="text-xs font-semibold uppercase tracking-wide text-neutral-400">
                      {t("fav.pinnedCount", { n: favorites().length, max: MAX_FAVORITES })}
                    </h2>
                    <Show
                      when={favorites().length > 0}
                      fallback={
                        <p class="text-xs text-neutral-400">
                          {t("fav.noPinned")}
                        </p>
                      }
                    >
                      <ul class="space-y-1">
                        <For each={favorites()}>
                          {(path, i) => (
                            <li
                              ref={(el) => (favRefs[i()] = el)}
                              class="relative flex items-center gap-3 rounded-lg border border-black/10 px-3 py-1.5 dark:border-white/10"
                              classList={{
                                "opacity-40": dragIndex() === i(),
                              }}
                            >
                              {/* drop-line indicator (before this row) */}
                              <Show
                                when={
                                  dragIndex() !== null && dropIndex() === i()
                                }
                              >
                                <span class="pointer-events-none absolute -top-[3px] left-0 right-0 h-0.5 rounded bg-[var(--color-accent)]" />
                              </Show>
                              {/* drop-line at the very end */}
                              <Show
                                when={
                                  dragIndex() !== null &&
                                  dropIndex() === favorites().length &&
                                  i() === favorites().length - 1
                                }
                              >
                                <span class="pointer-events-none absolute -bottom-[3px] left-0 right-0 h-0.5 rounded bg-[var(--color-accent)]" />
                              </Show>
                              <span
                                onPointerDown={(e) => startDrag(i(), e)}
                                aria-label={t("fav.dragReorder")}
                                class="cursor-grab touch-none select-none px-0.5 text-neutral-400 hover:text-neutral-700 active:cursor-grabbing dark:text-neutral-500 dark:hover:text-neutral-200"
                              >
                                ⠿
                              </span>
                              <span class="w-6 shrink-0 text-xs tabular-nums text-neutral-400">
                                ⌘{(i() + 1) % 10}
                              </span>
                              <AppIcon path={path} />
                              <span class="flex-1 truncate text-sm">
                                {appByPath(path)?.name ?? path}
                              </span>
                              <button
                                onClick={() => moveFavorite(i(), -1)}
                                disabled={i() === 0}
                                aria-label={t("fav.moveUp")}
                                class="px-1 text-neutral-400 hover:text-neutral-700 disabled:opacity-30 dark:hover:text-neutral-200"
                              >
                                ↑
                              </button>
                              <button
                                onClick={() => moveFavorite(i(), 1)}
                                disabled={i() === favorites().length - 1}
                                aria-label={t("fav.moveDown")}
                                class="px-1 text-neutral-400 hover:text-neutral-700 disabled:opacity-30 dark:hover:text-neutral-200"
                              >
                                ↓
                              </button>
                              <button
                                onClick={() => unpinApp(path)}
                                class="text-xs text-red-600 hover:underline"
                              >
                                {t("fav.unpin")}
                              </button>
                            </li>
                          )}
                        </For>
                      </ul>
                    </Show>
                  </div>

                  {/* suggestions to pin (most-opened first) */}
                  <div class="space-y-2">
                    <h2 class="text-xs font-semibold uppercase tracking-wide text-neutral-400">
                      {t("fav.addApps")}{" "}
                      <Show when={favorites().length >= MAX_FAVORITES}>
                        <span class="text-amber-600">{t("fav.limitReached")}</span>
                      </Show>
                    </h2>
                    <input
                      value={favFilter()}
                      onInput={(e) => setFavFilter(e.currentTarget.value)}
                      placeholder={t("fav.searchToPin")}
                      class="w-full rounded-lg border border-black/10 bg-white px-3 py-2 text-sm dark:border-white/10 dark:bg-neutral-800"
                    />
                    <ul class="max-h-96 space-y-0.5 overflow-y-auto rounded-lg border border-black/10 p-1 dark:border-white/10">
                      <For
                        each={favSuggestions()}
                        fallback={
                          <li class="px-2 py-2 text-xs text-neutral-400">{t("fav.noApps")}</li>
                        }
                      >
                        {(app) => (
                          <li class="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-black/5 dark:hover:bg-white/5">
                            <AppIcon path={app.path} />
                            <div class="min-w-0 flex-1">
                              <div class="truncate text-sm">{app.name}</div>
                              <Show when={opensOf(app.path) > 0}>
                                <div class="text-[0.6875rem] text-neutral-400">
                                  {t("fav.opens", { n: opensOf(app.path) })}
                                </div>
                              </Show>
                            </div>
                            <button
                              onClick={() => pinApp(app.path)}
                              disabled={favorites().length >= MAX_FAVORITES}
                              class="shrink-0 rounded-md border border-[var(--color-accent)] bg-[var(--color-accent)]/10 px-2 py-0.5 text-xs font-medium text-[var(--color-accent)] disabled:opacity-40"
                            >
                              {t("fav.pin")}
                            </button>
                          </li>
                        )}
                      </For>
                    </ul>
                  </div>
                </div>
                </Show>
              </Show>

              {/* ---------- Index & Permissions tab ---------- */}
              <Show when={tab() === "index"}>
                <div class="space-y-4">
                  {/* status row */}
                  <div class="flex items-center justify-between gap-3">
                    <span class="flex items-center gap-2 text-sm">
                      <Show
                        when={status()?.indexing}
                        fallback={
                          <>
                            <span class="font-medium">{enabledCount()}</span>
                            <span class="text-neutral-400">{t("index.ofApps", { total: total() })}</span>
                            <Show when={disabledCount() > 0}>
                              <span class="text-xs text-amber-600">
                                · {t("index.off", { n: disabledCount() })}
                              </span>
                            </Show>
                            <span class="text-xs text-neutral-400">
                              {lastUpdatedLabel(status()?.last_indexed_ms ?? null)}
                            </span>
                          </>
                        }
                      >
                        <Spinner /> <span>{t("index.indexing")}</span>
                      </Show>
                    </span>
                    <button
                      onClick={() => void reindexNow()}
                      class="rounded-lg border border-black/10 px-3 py-1 text-xs font-medium hover:bg-black/5 dark:border-white/10 dark:hover:bg-white/5"
                    >
                      {t("index.reindex")}
                    </button>
                  </div>

                  {/* search + add folder */}
                  <div class="flex items-center gap-2">
                    <input
                      value={query()}
                      onInput={(e) => setQuery(e.currentTarget.value)}
                      placeholder={t("index.searchApps")}
                      class="min-w-0 flex-1 rounded-lg border border-black/10 bg-white px-3 py-2 text-sm dark:border-white/10 dark:bg-neutral-800"
                    />
                    <button
                      onClick={() => void addFolder()}
                      class="shrink-0 rounded-lg border border-[var(--color-accent)] bg-[var(--color-accent)]/10 px-3 py-2 text-sm font-medium"
                    >
                      {t("index.addFolder")}
                    </button>
                  </div>

                  <p class="text-xs leading-relaxed text-neutral-500">
                    {t("index.explainA")} <code>~/Applications</code>
                    {t("index.explainB")}
                  </p>

                  {/* scanned locations — each expandable to its apps */}
                  <ul class="space-y-1.5">
                    <For each={visibleDirs()}>
                      {(dir) => (
                        <li
                          class="rounded-lg border border-black/10 dark:border-white/10"
                          classList={{ "opacity-50": dir.disabled }}
                        >
                          <div class="flex items-center gap-2 px-3 py-2 text-sm">
                            {/* folder on/off — hidden while searching (partial view) */}
                            <Show when={!searching()}>
                              <input
                                type="checkbox"
                                checked={!dir.disabled}
                                title={dir.disabled ? t("index.folderDisabled") : t("index.folderEnabled")}
                                onChange={() => void toggleFolder(dir.path, dir.disabled)}
                                class="h-4 w-4 shrink-0 accent-[var(--color-accent)]"
                              />
                            </Show>
                            <button
                              onClick={() =>
                                setExpandedDir((c) => (c === dir.path ? null : dir.path))
                              }
                              class="flex min-w-0 flex-1 items-center gap-2 text-left"
                            >
                              <span class="shrink-0 text-neutral-400">
                                {expandedDir() === dir.path || searching() ? "▾" : "▸"}
                              </span>
                              <span
                                class="truncate"
                                classList={{ "line-through": dir.disabled }}
                                title={dir.path}
                              >
                                {dir.path}
                              </span>
                            </button>
                            <span
                              class="shrink-0 rounded px-1.5 py-0.5 text-[0.625rem] uppercase tracking-wide"
                              classList={{
                                "bg-black/10 text-neutral-500 dark:bg-white/10":
                                  dir.kind === "system",
                                "bg-[var(--color-accent)]/20 text-[var(--color-accent)]":
                                  dir.kind === "granted",
                              }}
                            >
                              {dir.kind === "system" ? t("index.kind.system") : t("index.kind.granted")}
                            </span>
                            <span class="shrink-0 text-xs text-neutral-400">
                              {t("index.appsCount", { n: searching() ? appsInDir(dir.path).length : dir.count })}
                            </span>
                            <Show when={!dir.readable}>
                              <span
                                class="shrink-0 text-xs text-amber-600"
                                title={t("index.notReadable")}
                              >
                                ⚠
                              </span>
                            </Show>
                            <Show when={dir.kind === "granted"}>
                              <button
                                onClick={() => void removeFolder(dir.path)}
                                class="shrink-0 text-xs text-red-600 hover:underline"
                              >
                                {t("index.remove")}
                              </button>
                            </Show>
                          </div>
                          <Show when={expandedDir() === dir.path || searching()}>
                            <div class="border-t border-black/10 dark:border-white/10">
                              {/* bulk check/uncheck all shown apps */}
                              <div class="flex items-center gap-3 px-3 py-1.5 text-xs text-neutral-400">
                                <span>{t("index.appsCount", { n: appsInDir(dir.path).length })}</span>
                                <button
                                  onClick={() => void toggleAllInDir(dir, false)}
                                  class="text-[var(--color-accent)] hover:underline"
                                >
                                  {t("index.checkAll")}
                                </button>
                                <button
                                  onClick={() => void toggleAllInDir(dir, true)}
                                  class="text-red-600 hover:underline"
                                >
                                  {t("index.uncheckAll")}
                                </button>
                              </div>
                              <ul class="max-h-[32rem] overflow-y-auto px-2 pb-2">
                                <For each={appsInDir(dir.path)}>
                                  {(app) => (
                                    <AppRow
                                      app={app}
                                      onToggle={() => void toggleApp(app, dir)}
                                    />
                                  )}
                                </For>
                              </ul>
                            </div>
                          </Show>
                        </li>
                      )}
                    </For>
                  </ul>
                </div>
              </Show>

              {/* ---------- Help tab ---------- */}
              <Show when={tab() === "help"}>
                <div class="space-y-6">
                  <Section title={t("section.launcherShortcuts")}>
                    <ShortcutRow label={t("sc.openToggle")}>
                      <Kbd>{acceleratorToSymbols(d().shortcut)}</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.launchSelected")}>
                      <Kbd>↵</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.launchNth")}>
                      <Kbd>⌘1</Kbd>
                      <span class="text-neutral-400">{t("common.to")}</span>
                      <Kbd>⌘0</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.moveSelection")}>
                      <Kbd>↑</Kbd>
                      <Kbd>↓</Kbd>
                      <span class="text-neutral-400">{t("common.or")}</span>
                      <Kbd>⌃P</Kbd>
                      <Kbd>⌃N</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.clearClose")}>
                      <Kbd>esc</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.recenter")}>
                      <Kbd>⌘</Kbd>
                      <Kbd>⇧</Kbd>
                      <Kbd>C</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.openSettings")}>
                      <Kbd>⌘</Kbd>
                      <Kbd>;</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.pinUnpin")}>
                      <Kbd>⌘</Kbd>
                      <Kbd>P</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label={t("sc.movePinned")}>
                      <Kbd>⌘</Kbd>
                      <Kbd>K</Kbd>
                      <span class="text-neutral-400">/</span>
                      <Kbd>⌘</Kbd>
                      <Kbd>J</Kbd>
                      <span class="text-neutral-400">{t("common.or")}</span>
                      <Kbd>⌘</Kbd>
                      <Kbd>[</Kbd>
                      <span class="text-neutral-400">/</span>
                      <Kbd>⌘</Kbd>
                      <Kbd>]</Kbd>
                    </ShortcutRow>
                  </Section>

                  <Section title={t("section.tips")}>
                    <p class="px-4 py-3 text-xs leading-relaxed text-neutral-500">
                      {t("tips.bodyA")}{" "}
                      <Kbd>⌘</Kbd> <Kbd>⇧</Kbd> <Kbd>C</Kbd> {t("tips.bodyB")}
                    </p>
                  </Section>
                </div>
              </Show>

              {/* ---------- Developer tab (hidden; ⌘⇧D) ---------- */}
              <Show when={tab() === "developer"}>
                <div class="space-y-6">
                  <Section
                    title={t("dev.title")}
                    footnote={
                      <>
                        <span class="block">{t("dev.desc")}</span>
                        <span class="mt-1 block">{t("dev.hint")}</span>
                      </>
                    }
                  >
                    <Toggle
                      label={t("dev.enable")}
                      checked={d().dev_logging}
                      onChange={(v) => void toggleDevLogging(v)}
                    />
                  </Section>

                  <div class="space-y-2">
                    <div class="flex flex-wrap items-center justify-end gap-2">
                      <button
                        onClick={() => void copyLogs()}
                        disabled={devLogs().trim().length === 0}
                        class="rounded-lg border border-black/10 px-2.5 py-1 text-xs font-medium hover:bg-black/5 disabled:opacity-40 dark:border-white/10 dark:hover:bg-white/10"
                      >
                        {copied() ? t("dev.copied") : t("dev.copy")}
                      </button>
                      <button
                        onClick={() => void revealDevLogs()}
                        class="rounded-lg border border-black/10 px-2.5 py-1 text-xs font-medium hover:bg-black/5 dark:border-white/10 dark:hover:bg-white/10"
                      >
                        {t("dev.reveal")}
                      </button>
                      <button
                        onClick={() => void loadDevLogs()}
                        class="rounded-lg border border-black/10 px-2.5 py-1 text-xs font-medium hover:bg-black/5 dark:border-white/10 dark:hover:bg-white/10"
                      >
                        {t("dev.refresh")}
                      </button>
                      <button
                        onClick={() => void clearLogs()}
                        class="rounded-lg border border-black/10 px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-500/10 dark:border-white/10"
                      >
                        {t("dev.clear")}
                      </button>
                    </div>
                    <Show
                      when={devLogs().trim().length > 0}
                      fallback={
                        <p class="text-xs text-neutral-400">{t("dev.empty")}</p>
                      }
                    >
                      <pre class="max-h-80 w-full overflow-auto whitespace-pre-wrap break-all rounded-lg border border-black/10 bg-black/5 p-3 text-[11px] leading-relaxed text-neutral-700 dark:border-white/10 dark:bg-white/5 dark:text-neutral-300">
                        {formatDevLogs(devLogs())}
                      </pre>
                    </Show>
                  </div>
                </div>
              </Show>
              </>
            )}
          </Show>

          <Show when={error()}>
            <p class="mt-6 rounded-lg bg-red-500/10 px-3 py-2 text-sm text-red-600">
              {error()}
            </p>
          </Show>
        </div>
      </div>
    </div>
  );
}

function ShortcutRow(props: { label: string; children: any }) {
  return (
    <div class="flex items-center justify-between gap-4 px-4 py-2.5">
      <span class="text-sm">{props.label}</span>
      <span class="flex items-center gap-1">{props.children}</span>
    </div>
  );
}

function TabButton(props: {
  active: boolean;
  onClick: () => void;
  children: any;
  ref?: (el: HTMLButtonElement) => void;
}) {
  return (
    <button
      ref={props.ref}
      role="tab"
      aria-selected={props.active}
      tabindex={props.active ? 0 : -1}
      onClick={props.onClick}
      class="-mb-px grid border-b-2 px-1 pb-2 text-sm transition-colors"
      classList={{
        "border-[var(--color-accent)] text-neutral-900 dark:text-neutral-100":
          props.active,
        "border-transparent text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300":
          !props.active,
      }}
    >
      {/* visible label — weight changes when active */}
      <span
        class="col-start-1 row-start-1"
        classList={{ "font-medium": props.active }}
      >
        {props.children}
      </span>
      {/* invisible bold copy reserves the active width so nothing shifts */}
      <span aria-hidden="true" class="invisible col-start-1 row-start-1 font-medium">
        {props.children}
      </span>
    </button>
  );
}

function AppRow(props: { app: IndexedApp; onToggle: () => void }) {
  return (
    <li
      class="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-black/5 dark:hover:bg-white/5"
      classList={{ "opacity-40": !props.app.enabled }}
    >
      <input
        type="checkbox"
        checked={props.app.enabled}
        title={props.app.enabled ? t("index.enabled") : t("index.disabled")}
        onChange={props.onToggle}
        class="h-4 w-4 shrink-0 accent-[var(--color-accent)]"
      />
      <AppIcon path={props.app.path} />
      <div class="min-w-0">
        <div class="truncate text-sm" classList={{ "line-through": !props.app.enabled }}>
          {props.app.name}
        </div>
        <div class="truncate text-[0.6875rem] text-neutral-400" title={props.app.path}>
          {props.app.bundle_id ? `${props.app.bundle_id} · ` : ""}
          {props.app.path}
        </div>
      </div>
    </li>
  );
}

function Section(props: { title?: string; footnote?: any; children: any }) {
  return (
    <section>
      <Show when={props.title}>
        <h2 class="mb-2 px-1 text-[0.6875rem] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
          {props.title}
        </h2>
      </Show>
      {/* Grouped card, like macOS System Settings: rounded, hairline row dividers. */}
      <div class="divide-y divide-black/[0.06] overflow-hidden rounded-xl bg-white shadow-sm ring-1 ring-black/[0.06] dark:divide-white/[0.07] dark:bg-white/[0.04] dark:ring-white/10">
        {props.children}
      </div>
      <Show when={props.footnote}>
        <p class="mt-2 px-1 text-xs leading-relaxed text-neutral-500">{props.footnote}</p>
      </Show>
    </section>
  );
}

function Field(props: { label: string; children: any; align?: "center" | "start" }) {
  return (
    <div
      class="flex justify-between gap-4 px-4 py-3"
      classList={{
        "items-center": (props.align ?? "center") === "center",
        "items-start": props.align === "start",
      }}
    >
      <span class="text-sm" classList={{ "pt-1": props.align === "start" }}>
        {props.label}
      </span>
      {props.children}
    </div>
  );
}

/** A visual segmented control (radiogroup) — replaces a plain <select>. */
function Segmented<T extends string>(props: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: string; icon?: any }[];
}) {
  return (
    <div
      role="radiogroup"
      class="inline-flex gap-0.5 rounded-xl bg-black/5 p-1 dark:bg-white/10"
    >
      <For each={props.options}>
        {(o) => (
          <button
            role="radio"
            aria-checked={props.value === o.value}
            aria-label={o.label}
            onClick={() => props.onChange(o.value)}
            class="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm transition"
            classList={{
              "bg-white text-neutral-900 shadow-sm dark:bg-neutral-700 dark:text-white":
                props.value === o.value,
              "text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200":
                props.value !== o.value,
            }}
          >
            {o.icon}
            <span>{o.label}</span>
          </button>
        )}
      </For>
    </div>
  );
}

function IconSun() {
  return (
    <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M19.1 4.9l-1.4 1.4M6.3 17.7l-1.4 1.4" />
    </svg>
  );
}
function IconMoon() {
  return (
    <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
      <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
    </svg>
  );
}
function IconAuto() {
  return (
    <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor" stroke="none" />
    </svg>
  );
}

function Toggle(props: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label class="flex cursor-pointer items-center justify-between gap-4 px-4 py-3">
      <span class="text-sm">{props.label}</span>
      <input
        type="checkbox"
        checked={props.checked}
        onChange={(e) => props.onChange(e.currentTarget.checked)}
        class="h-4 w-4 accent-[var(--color-accent)]"
      />
    </label>
  );
}

function Spinner() {
  return (
    <span class="inline-block h-3 w-3 animate-spin rounded-full border-2 border-current border-t-transparent opacity-70" />
  );
}

// Turn raw "<epoch-ms>\t<message>" log lines into "<local date-time>  <message>".
function formatDevLogs(raw: string): string {
  return raw
    .split("\n")
    .filter((l) => l.length > 0)
    .map((line) => {
      const i = line.indexOf("\t");
      if (i < 0) return line;
      const ms = Number(line.slice(0, i));
      const stamp = Number.isFinite(ms)
        ? new Date(ms).toLocaleString()
        : line.slice(0, i);
      return `${stamp}  ${line.slice(i + 1)}`;
    })
    .join("\n");
}

function lastUpdatedLabel(ms: number | null): string {
  if (!ms) return "";
  const secs = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (secs < 5) return t("index.updatedNow");
  if (secs < 60) return t("index.updatedSecs", { n: secs });
  const mins = Math.round(secs / 60);
  if (mins < 60) return t("index.updatedMins", { n: mins });
  return t("index.updatedHours", { n: Math.round(mins / 60) });
}

function Kbd(props: { children: any }) {
  return (
    <kbd class="rounded border border-black/15 bg-white px-1.5 py-0.5 text-[0.6875rem] dark:border-white/15 dark:bg-neutral-800">
      {props.children}
    </kbd>
  );
}
