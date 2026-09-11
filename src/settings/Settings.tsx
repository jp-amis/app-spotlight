import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  getFavorites,
  getSettings,
  grantFolder,
  indexedApps,
  indexStatus,
  reindexNow,
  removeGrantedFolder,
  setAppDisabled,
  setAppsDisabled,
  setFavorites as saveFavoritesApi,
  setFolderDisabled,
  setSetting,
  setShortcut,
  type DirInfo,
  type IndexedApp,
  type IndexStatus,
  type Settings as SettingsData,
} from "../lib/api";
import AppIcon from "../launcher/AppIcon";
import { acceleratorToSymbols } from "../lib/accelerator";
import ShortcutCapture from "./ShortcutCapture";

type Tab = "general" | "favorites" | "index" | "help";

const TAB_ORDER: Tab[] = ["general", "favorites", "index", "help"];

export default function Settings() {
  const [tab, setTab] = createSignal<Tab>("general");
  const tabRefs: Partial<Record<Tab, HTMLButtonElement>> = {};

  // Arrow keys move between tabs (ARIA tablist), keeping focus on the tab.
  function onTabsKeyDown(e: KeyboardEvent) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const i = TAB_ORDER.indexOf(tab());
    const delta = e.key === "ArrowRight" ? 1 : TAB_ORDER.length - 1;
    const next = TAB_ORDER[(i + delta) % TAB_ORDER.length];
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
  const [error, setError] = createSignal("");

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
    // Start keyboard focus on the active tab so arrows/Tab work immediately.
    queueMicrotask(() => tabRefs[tab()]?.focus());
    try {
      setData(await getSettings());
      setFavorites(await getFavorites());
      await refreshStatus();
      await refreshApps();
    } catch (e) {
      setError(String(e));
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
    ]);
    // ⌘Q closes this window (the app never quits from ⌘Q — same as ⌘W).
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.code === "KeyQ") {
        e.preventDefault();
        void getCurrentWindow().close();
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
        <span class="pointer-events-none text-[13px] font-semibold text-neutral-800 dark:text-neutral-100">
          My App Spot Settings
        </span>
      </div>

      {/* tabs — fixed while the content scrolls, with a little breathing room up top */}
      <div class="shrink-0 border-b border-black/10 px-10 pt-4 dark:border-white/10">
        <div role="tablist" onKeyDown={onTabsKeyDown} class="mx-auto flex max-w-4xl gap-6">
          <TabButton
            active={tab() === "general"}
            onClick={() => setTab("general")}
            ref={(el) => (tabRefs.general = el)}
          >
            General
          </TabButton>
          <TabButton
            active={tab() === "favorites"}
            onClick={() => setTab("favorites")}
            ref={(el) => (tabRefs.favorites = el)}
          >
            Favorites
          </TabButton>
          <TabButton
            active={tab() === "index"}
            onClick={() => setTab("index")}
            ref={(el) => (tabRefs.index = el)}
          >
            Index & Permissions
          </TabButton>
          <TabButton
            active={tab() === "help"}
            onClick={() => setTab("help")}
            ref={(el) => (tabRefs.help = el)}
          >
            Shortcuts
          </TabButton>
        </div>
      </div>

      {/* scrollable content */}
      <div class="flex-1 overflow-y-auto px-10 py-6">
        <div class="mx-auto max-w-4xl">
          <Show when={data()} fallback={<p class="text-sm text-neutral-500">Loading…</p>}>
            {(d) => (
              <>
              {/* ---------- General tab ---------- */}
              <Show when={tab() === "general"}>
                <div class="space-y-8">
                  <Section title="Shortcut">
                    <Field label="Global hotkey">
                      <ShortcutCapture value={d().shortcut} onChange={rebind} />
                    </Field>
                    <p class="text-xs leading-relaxed text-neutral-500">
                      Want <Kbd>⌘</Kbd> <Kbd>Space</Kbd>? macOS reserves it for Spotlight.
                      Open System Settings → Keyboard → Keyboard Shortcuts → Spotlight and
                      uncheck “Show Spotlight search”, then rebind here.
                    </p>
                  </Section>

                  <Section title="General">
                    <Toggle
                      label="Show menu-bar icon"
                      checked={d().menubar_visible}
                      onChange={(v) => update("menubar_visible", v)}
                    />
                    <Field label="Results shown">
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

                  <Section title="Appearance">
                    <Field label="Theme">
                      <select
                        value={d().theme}
                        onChange={(e) =>
                          update("theme", e.currentTarget.value as SettingsData["theme"])
                        }
                        class="rounded-lg border border-black/10 bg-white px-2 py-1 text-sm dark:border-white/10 dark:bg-neutral-800"
                      >
                        <option value="system">System</option>
                        <option value="light">Light</option>
                        <option value="dark">Dark</option>
                      </select>
                    </Field>
                  </Section>
                </div>
              </Show>

              {/* ---------- Favorites tab ---------- */}
              <Show when={tab() === "favorites"}>
                <div class="space-y-6">
                  <p class="text-xs leading-relaxed text-neutral-500">
                    Pinned apps appear at the top of the launcher — with <Kbd>⌘</Kbd>
                    <Kbd>1</Kbd>…<Kbd>⌘</Kbd>
                    <Kbd>0</Kbd> shortcuts — the moment you open it, before you type
                    anything. Up to {MAX_FAVORITES}.
                  </p>

                  {/* pinned, ordered */}
                  <div class="space-y-2">
                    <h2 class="text-xs font-semibold uppercase tracking-wide text-neutral-400">
                      Pinned ({favorites().length}/{MAX_FAVORITES})
                    </h2>
                    <Show
                      when={favorites().length > 0}
                      fallback={
                        <p class="text-xs text-neutral-400">
                          No pinned apps yet — pin some from the list below.
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
                                aria-label="Drag to reorder"
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
                                aria-label="Move up"
                                class="px-1 text-neutral-400 hover:text-neutral-700 disabled:opacity-30 dark:hover:text-neutral-200"
                              >
                                ↑
                              </button>
                              <button
                                onClick={() => moveFavorite(i(), 1)}
                                disabled={i() === favorites().length - 1}
                                aria-label="Move down"
                                class="px-1 text-neutral-400 hover:text-neutral-700 disabled:opacity-30 dark:hover:text-neutral-200"
                              >
                                ↓
                              </button>
                              <button
                                onClick={() => unpinApp(path)}
                                class="text-xs text-red-600 hover:underline"
                              >
                                Unpin
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
                      Add apps{" "}
                      <Show when={favorites().length >= MAX_FAVORITES}>
                        <span class="text-amber-600">(limit reached)</span>
                      </Show>
                    </h2>
                    <input
                      value={favFilter()}
                      onInput={(e) => setFavFilter(e.currentTarget.value)}
                      placeholder="Search apps to pin…"
                      class="w-full rounded-lg border border-black/10 bg-white px-3 py-2 text-sm dark:border-white/10 dark:bg-neutral-800"
                    />
                    <ul class="max-h-96 space-y-0.5 overflow-y-auto rounded-lg border border-black/10 p-1 dark:border-white/10">
                      <For
                        each={favSuggestions()}
                        fallback={
                          <li class="px-2 py-2 text-xs text-neutral-400">No apps</li>
                        }
                      >
                        {(app) => (
                          <li class="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-black/5 dark:hover:bg-white/5">
                            <AppIcon path={app.path} />
                            <div class="min-w-0 flex-1">
                              <div class="truncate text-sm">{app.name}</div>
                              <Show when={opensOf(app.path) > 0}>
                                <div class="text-[11px] text-neutral-400">
                                  {opensOf(app.path)} opens
                                </div>
                              </Show>
                            </div>
                            <button
                              onClick={() => pinApp(app.path)}
                              disabled={favorites().length >= MAX_FAVORITES}
                              class="shrink-0 rounded-md border border-[var(--color-accent)] bg-[var(--color-accent)]/10 px-2 py-0.5 text-xs font-medium text-[var(--color-accent)] disabled:opacity-40"
                            >
                              Pin
                            </button>
                          </li>
                        )}
                      </For>
                    </ul>
                  </div>
                </div>
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
                            <span class="text-neutral-400">of {total()} apps</span>
                            <Show when={disabledCount() > 0}>
                              <span class="text-xs text-amber-600">
                                · {disabledCount()} off
                              </span>
                            </Show>
                            <span class="text-xs text-neutral-400">
                              {lastUpdatedLabel(status()?.last_indexed_ms ?? null)}
                            </span>
                          </>
                        }
                      >
                        <Spinner /> <span>Indexing…</span>
                      </Show>
                    </span>
                    <button
                      onClick={() => void reindexNow()}
                      class="rounded-lg border border-black/10 px-3 py-1 text-xs font-medium hover:bg-black/5 dark:border-white/10 dark:hover:bg-white/5"
                    >
                      Reindex
                    </button>
                  </div>

                  {/* search + add folder */}
                  <div class="flex items-center gap-2">
                    <input
                      value={query()}
                      onInput={(e) => setQuery(e.currentTarget.value)}
                      placeholder="Search apps…"
                      class="min-w-0 flex-1 rounded-lg border border-black/10 bg-white px-3 py-2 text-sm dark:border-white/10 dark:bg-neutral-800"
                    />
                    <button
                      onClick={() => void addFolder()}
                      class="shrink-0 rounded-lg border border-[var(--color-accent)] bg-[var(--color-accent)]/10 px-3 py-2 text-sm font-medium"
                    >
                      Add Folder…
                    </button>
                  </div>

                  <p class="text-xs leading-relaxed text-neutral-500">
                    Sandboxed, so it always searches the system app folders; grant extra
                    folders (like <code>~/Applications</code>). Turn a folder or app off to
                    hide it from results (without removing it).
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
                                title={dir.disabled ? "Folder disabled" : "Folder enabled"}
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
                              class="shrink-0 rounded px-1.5 py-0.5 text-[10px] uppercase tracking-wide"
                              classList={{
                                "bg-black/10 text-neutral-500 dark:bg-white/10":
                                  dir.kind === "system",
                                "bg-[var(--color-accent)]/20 text-[var(--color-accent)]":
                                  dir.kind === "granted",
                              }}
                            >
                              {dir.kind}
                            </span>
                            <span class="shrink-0 text-xs text-neutral-400">
                              {searching() ? appsInDir(dir.path).length : dir.count} apps
                            </span>
                            <Show when={!dir.readable}>
                              <span
                                class="shrink-0 text-xs text-amber-600"
                                title="Not readable"
                              >
                                ⚠
                              </span>
                            </Show>
                            <Show when={dir.kind === "granted"}>
                              <button
                                onClick={() => void removeFolder(dir.path)}
                                class="shrink-0 text-xs text-red-600 hover:underline"
                              >
                                Remove
                              </button>
                            </Show>
                          </div>
                          <Show when={expandedDir() === dir.path || searching()}>
                            <div class="border-t border-black/10 dark:border-white/10">
                              {/* bulk check/uncheck all shown apps */}
                              <div class="flex items-center gap-3 px-3 py-1.5 text-xs text-neutral-400">
                                <span>{appsInDir(dir.path).length} apps</span>
                                <button
                                  onClick={() => void toggleAllInDir(dir, false)}
                                  class="text-[var(--color-accent)] hover:underline"
                                >
                                  Check all
                                </button>
                                <button
                                  onClick={() => void toggleAllInDir(dir, true)}
                                  class="text-red-600 hover:underline"
                                >
                                  Uncheck all
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
                <div class="max-w-xl space-y-8">
                  <Section title="Launcher shortcuts">
                    <ShortcutRow label="Open / toggle launcher">
                      <Kbd>{acceleratorToSymbols(d().shortcut)}</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Launch selected">
                      <Kbd>↵</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Launch 1st–10th result">
                      <Kbd>⌘1</Kbd>
                      <span class="text-neutral-400">…</span>
                      <Kbd>⌘0</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Move selection">
                      <Kbd>↑</Kbd>
                      <Kbd>↓</Kbd>
                      <span class="text-neutral-400">or</span>
                      <Kbd>⌃P</Kbd>
                      <Kbd>⌃N</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Clear query / close">
                      <Kbd>esc</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Re-center the launcher window">
                      <Kbd>⌘</Kbd>
                      <Kbd>⇧</Kbd>
                      <Kbd>C</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Open settings">
                      <Kbd>⌘</Kbd>
                      <Kbd>;</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Pin / unpin selected app">
                      <Kbd>⌘</Kbd>
                      <Kbd>P</Kbd>
                    </ShortcutRow>
                    <ShortcutRow label="Move pinned app up / down">
                      <Kbd>⌘</Kbd>
                      <Kbd>K</Kbd>
                      <span class="text-neutral-400">/</span>
                      <Kbd>⌘</Kbd>
                      <Kbd>J</Kbd>
                    </ShortcutRow>
                  </Section>

                  <Section title="Tips">
                    <p class="text-xs leading-relaxed text-neutral-500">
                      The launcher lives in the menu bar and opens on the screen your cursor
                      is on, remembering a position per monitor. Drag it to reposition; press{" "}
                      <Kbd>⌘</Kbd> <Kbd>⇧</Kbd> <Kbd>C</Kbd> while it's open to snap it back to
                      the default spot.
                    </p>
                  </Section>
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
    <div class="flex items-center justify-between gap-4">
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
        title={props.app.enabled ? "Enabled" : "Disabled"}
        onChange={props.onToggle}
        class="h-4 w-4 shrink-0 accent-[var(--color-accent)]"
      />
      <AppIcon path={props.app.path} />
      <div class="min-w-0">
        <div class="truncate text-sm" classList={{ "line-through": !props.app.enabled }}>
          {props.app.name}
        </div>
        <div class="truncate text-[11px] text-neutral-400" title={props.app.path}>
          {props.app.bundle_id ? `${props.app.bundle_id} · ` : ""}
          {props.app.path}
        </div>
      </div>
    </li>
  );
}

function Section(props: { title: string; children: any }) {
  return (
    <section>
      <h2 class="mb-3 text-xs font-semibold uppercase tracking-wide text-neutral-400">
        {props.title}
      </h2>
      <div class="space-y-3">{props.children}</div>
    </section>
  );
}

function Field(props: { label: string; children: any }) {
  return (
    <div class="flex items-center justify-between gap-4">
      <span class="text-sm">{props.label}</span>
      {props.children}
    </div>
  );
}

function Toggle(props: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label class="flex cursor-pointer items-center justify-between gap-4">
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

function lastUpdatedLabel(ms: number | null): string {
  if (!ms) return "";
  const secs = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (secs < 5) return "· updated just now";
  if (secs < 60) return `· updated ${secs}s ago`;
  const mins = Math.round(secs / 60);
  if (mins < 60) return `· updated ${mins}m ago`;
  return `· updated ${Math.round(mins / 60)}h ago`;
}

function Kbd(props: { children: any }) {
  return (
    <kbd class="rounded border border-black/15 bg-white px-1.5 py-0.5 text-[11px] dark:border-white/15 dark:bg-neutral-800">
      {props.children}
    </kbd>
  );
}
