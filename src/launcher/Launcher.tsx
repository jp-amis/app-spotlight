import {
  createEffect,
  createMemo,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
} from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { rank, type AppEntry, type Frecency } from "../lib/fuzzy";
import {
  accessStatus,
  devLog,
  favoritesStatus,
  getFavorites,
  getSettings,
  grantFolder,
  hideLauncher,
  indexStatus,
  launchApp,
  listApps,
  openSettings,
  recenterLauncher,
  resizeLauncher,
  setFavorites as saveFavoritesApi,
} from "../lib/api";
import AppIcon from "./AppIcon";
import { initI18n, t } from "../lib/i18n";
import { fontScale, initFontScale } from "../lib/fontscale";

const BANNER_DISMISS_KEY = "accessBannerDismissed";

export default function Launcher() {
  const [apps, setApps] = createSignal<AppEntry[]>([]);
  const [query, setQuery] = createSignal("");
  const [selected, setSelected] = createSignal(0);
  const [frecency, setFrecency] = createSignal<Frecency>({});
  const [favorites, setFavorites] = createSignal<string[]>([]);
  const [favUnlocked, setFavUnlocked] = createSignal(false);
  const [limit, setLimit] = createSignal(10);
  const [indexing, setIndexing] = createSignal(false);
  const [accessLimited, setAccessLimited] = createSignal(false);
  const [bannerDismissed, setBannerDismissed] = createSignal(
    localStorage.getItem(BANNER_DISMISS_KEY) === "1",
  );
  let inputEl: HTMLInputElement | undefined;
  let rootEl: HTMLDivElement | undefined;

  const showBanner = () => accessLimited() && !bannerDismissed();

  async function refreshAccess() {
    try {
      setAccessLimited((await accessStatus()).limited);
    } catch {
      /* no backend */
    }
  }

  function dismissBanner() {
    localStorage.setItem(BANNER_DISMISS_KEY, "1");
    setBannerDismissed(true);
  }

  async function grant() {
    try {
      const path = await grantFolder();
      if (path) {
        await refreshApps();
        await refreshAccess();
      }
    } catch (e) {
      console.error(e);
    }
  }

  const results = createMemo(() =>
    rank(apps(), query(), frecency(), limit(), favorites()),
  );

  // On an empty query, how many leading results are pinned favorites — used to
  // draw a divider where the favorites end (only if there's something below them).
  const favCount = createMemo(() => {
    if (query().trim().length > 0) return 0;
    const favSet = new Set(favorites());
    return results().filter((a) => favSet.has(a.path)).length;
  });

  async function refreshFavorites() {
    try {
      // Favorites are a paid/unlockable feature — only apply them when unlocked,
      // otherwise the launcher ranks by frecency alone (no pins, no ⌘-badges).
      const [status, favs] = await Promise.all([favoritesStatus(), getFavorites()]);
      setFavUnlocked(status.unlocked);
      setFavorites(status.unlocked ? favs : []);
    } catch {
      /* no backend */
    }
  }

  // Resize the window to fit its content so the last row is never clipped
  // (the results list scrolls past its max-height).
  let lastAppliedHeight = 0;
  function syncWindowHeight() {
    if (!rootEl) return;
    const h = Math.ceil(rootEl.getBoundingClientRect().height);
    if (h < 60 || h === lastAppliedHeight) return;
    lastAppliedHeight = h;
    // Resize natively with the top-left pinned (grows downward, no anchor shift). Persist
    // only the EMPTY-query height — that's the size the launcher reopens at, so it's what
    // the backend should pre-size to on the next show. Persisting a shrunken search height
    // instead would make it open small and resize after show (the reopen blink).
    void resizeLauncher(h, query().trim().length === 0);
  }

  createEffect(() => {
    results(); // re-measure whenever the visible results change
    showBanner(); // ...or when the access banner appears/disappears
    fontScale(); // ...or when the UI text size changes (rows grow/shrink)
    requestAnimationFrame(syncWindowHeight);
  });

  async function refreshApps() {
    try {
      setApps(await listApps());
    } catch {
      /* dev/browser without backend */
    }
  }

  async function refreshSettings() {
    try {
      const s = await getSettings();
      setFrecency(s.frecency ?? {});
      setLimit(s.result_limit ?? 10);
    } catch {
      /* dev/browser without backend */
    }
  }

  // Whether the backend is (re)building the app index right now — used to show a
  // "building" hint on the very first launch, before any apps are indexed yet.
  async function refreshIndexing() {
    try {
      setIndexing((await indexStatus()).indexing);
    } catch {
      /* no backend */
    }
  }

  // Focus the search input. WKWebView can drop focus right as the window shows, so
  // try now and again next frame.
  function focusInput() {
    inputEl?.focus();
    requestAnimationFrame(() => inputEl?.focus());
  }

  onMount(() => {
    void initI18n();
    void initFontScale();
    void refreshApps();
    void refreshSettings();
    void refreshAccess();
    void refreshFavorites();
    void refreshIndexing();
    focusInput();

    // Diagnostic (#5): each webview (re)mount bumps a persisted counter. If this climbs
    // while the app just sits idle, the launcher's WKWebView was reloaded from scratch
    // (content-process purge) — the cause of the "empty then repositioned" flash. No-op
    // unless dev logging is enabled in Settings → Developer.
    const mountCount = Number(localStorage.getItem("launcherMountCount") ?? "0") + 1;
    localStorage.setItem("launcherMountCount", String(mountCount));
    void devLog(`launcher mounted (lifetime #${mountCount})`);

    // Backend fires this right after the window hides — clear the query/selection now, so
    // the next open is already blank instead of flashing the previous search.
    const unHidden = listen("launcher:hidden", () => {
      setQuery("");
      setSelected(0);
    });
    // Backend fires this each time the window is shown via the global shortcut.
    const un = listen("launcher:opened", () => {
      setQuery("");
      setSelected(0);
      void refreshSettings();
      void refreshAccess();
      void refreshFavorites();
      void refreshIndexing();
      focusInput();
    });
    // A grant from the Settings window reindexes; refresh our list when it does.
    const un2 = listen("apps:reindexed", () => {
      void refreshApps();
      void refreshAccess();
    });
    const un3 = listen("favorites:changed", () => void refreshFavorites());
    const un4 = listen("favorites:unlocked", () => void refreshFavorites());
    // First-launch indexing lifecycle: show the hint while building, refresh when done.
    const un5 = listen("index:start", () => setIndexing(true));
    const un6 = listen("index:done", () => {
      setIndexing(false);
      void refreshApps();
    });
    onCleanup(() => {
      void unHidden.then((f) => f());
      void un.then((f) => f());
      void un2.then((f) => f());
      void un3.then((f) => f());
      void un4.then((f) => f());
      void un5.then((f) => f());
      void un6.then((f) => f());
    });
  });

  function move(delta: number) {
    const n = results().length;
    if (n === 0) return;
    setSelected((i) => (i + delta + n) % n);
  }

  const MAX_FAVORITES = 10;

  // ⌘P: pin/unpin the selected app (no-op when pinning past the limit).
  async function togglePin(entry?: AppEntry) {
    if (!favUnlocked()) return; // favorites is a locked feature
    const target = entry ?? results()[selected()];
    if (!target) return;
    const favs = favorites();
    const pinned = favs.includes(target.path);
    if (!pinned && favs.length >= MAX_FAVORITES) return; // no room
    const next = pinned
      ? favs.filter((p) => p !== target.path)
      : [...favs, target.path];
    setFavorites(next); // optimistic; backend also emits favorites:changed
    try {
      await saveFavoritesApi(next);
    } catch (e) {
      console.error(e);
    }
  }

  // ⌘K/⌘[ (up) and ⌘J/⌘] (down): reorder the selected app within the pinned
  // list. Only applies when the selected app is itself pinned.
  async function movePinned(dir: -1 | 1) {
    if (!favUnlocked()) return; // favorites is a locked feature
    const target = results()[selected()];
    if (!target) return;
    const favs = [...favorites()];
    const idx = favs.indexOf(target.path);
    if (idx < 0) return; // not pinned
    const j = idx + dir;
    if (j < 0 || j >= favs.length) return;
    [favs[idx], favs[j]] = [favs[j], favs[idx]];
    setFavorites(favs);
    // On an empty query the pinned order is the leading order — follow the move.
    if (query().trim().length === 0) setSelected(j);
    try {
      await saveFavoritesApi(favs);
    } catch (e) {
      console.error(e);
    }
  }

  async function activate(entry?: AppEntry) {
    const target = entry ?? results()[selected()];
    if (!target) return;
    try {
      await launchApp(target.path);
    } catch (e) {
      console.error(e);
    }
  }

  function onKeyDown(e: KeyboardEvent) {
    // Hidden: ⌘⇧C re-centers the launcher on its monitor.
    if (e.metaKey && e.shiftKey && e.code === "KeyC") {
      e.preventDefault();
      void recenterLauncher();
      return;
    }
    // ⌘; opens the settings window.
    if (e.metaKey && e.code === "Semicolon") {
      e.preventDefault();
      void hideLauncher();
      void openSettings();
      return;
    }
    // ⌘Q closes the launcher (it never quits the app; ⌘⇧Q quits, via the menu).
    if (e.metaKey && !e.shiftKey && e.code === "KeyQ") {
      e.preventDefault();
      void hideLauncher();
      return;
    }
    // ⌘P toggles pin/unpin for the selected app.
    if (e.metaKey && e.code === "KeyP") {
      e.preventDefault();
      void togglePin();
      return;
    }
    // ⌘K/⌘[ move the selected pinned app up, ⌘J/⌘] move it down.
    if (e.metaKey && (e.code === "KeyK" || e.code === "BracketLeft")) {
      e.preventDefault();
      void movePinned(-1);
      return;
    }
    if (e.metaKey && (e.code === "KeyJ" || e.code === "BracketRight")) {
      e.preventDefault();
      void movePinned(1);
      return;
    }
    // ⌘1..⌘9 launch the 1st..9th result, ⌘0 the 10th.
    if (e.metaKey && /^Digit[0-9]$/.test(e.code)) {
      e.preventDefault();
      const d = Number(e.code.slice(5));
      const idx = d === 0 ? 9 : d - 1;
      const target = results()[idx];
      if (target) void activate(target);
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        move(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        move(-1);
        break;
      case "n":
        if (e.ctrlKey) {
          e.preventDefault();
          move(1);
        }
        break;
      case "p":
        if (e.ctrlKey) {
          e.preventDefault();
          move(-1);
        }
        break;
      case "Enter":
        e.preventDefault();
        void activate();
        break;
      case "Escape":
        e.preventDefault();
        if (query().length > 0) {
          setQuery("");
          setSelected(0);
        } else {
          void hideLauncher();
        }
        break;
    }
  }

  // keep selection in range as results change
  createMemo(() => {
    if (selected() >= results().length) setSelected(0);
  });

  return (
    <div
      ref={rootEl}
      class="flex w-screen flex-col overflow-hidden rounded-2xl
             text-neutral-900 dark:text-neutral-100"
      onKeyDown={onKeyDown}
    >
      {/* draggable search header */}
      <div class="flex items-center gap-3 px-5 py-4" data-tauri-drag-region>
        <SearchGlyph />
        <input
          ref={inputEl}
          value={query()}
          onInput={(e) => {
            setQuery(e.currentTarget.value);
            setSelected(0);
          }}
          placeholder={t("launcher.search")}
          spellcheck={false}
          autocapitalize="off"
          autocomplete="off"
          class="min-w-0 flex-1 bg-transparent text-2xl font-light outline-none
                 placeholder:text-neutral-500 dark:placeholder:text-neutral-500"
        />
        {/* Settings shortcut — only on an empty query, where the header has room. */}
        <Show when={query().length === 0}>
          <button
            type="button"
            title={t("launcher.openSettings")}
            aria-label={t("launcher.openSettings")}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={() => {
              void hideLauncher();
              void openSettings();
            }}
            class="flex shrink-0 items-center gap-1.5 rounded-lg px-2 py-1 text-xs
                   text-neutral-500 transition-colors hover:bg-black/5 hover:text-neutral-800
                   dark:text-neutral-400 dark:hover:bg-white/10 dark:hover:text-neutral-200"
          >
            <GearGlyph />
            <span class="tabular-nums">⌘;</span>
          </button>
        </Show>
      </div>

      <Show when={showBanner()}>
        <div class="mx-3 mb-2 flex items-center gap-2 rounded-xl bg-amber-500/15 px-3 py-2 text-xs text-amber-700 dark:text-amber-300">
          <span class="flex-1">{t("launcher.accessBanner")}</span>
          <button
            onClick={() => void grant()}
            class="rounded-lg bg-amber-500/25 px-2 py-1 font-medium hover:bg-amber-500/40"
          >
            {t("launcher.grantAccess")}
          </button>
          <button
            onClick={dismissBanner}
            aria-label={t("launcher.dismiss")}
            class="px-1 text-amber-700/70 hover:text-amber-700 dark:text-amber-300/70"
          >
            ✕
          </button>
        </div>
      </Show>

      <Show
        when={results().length > 0}
        fallback={
          <Show
            when={indexing() && apps().length === 0}
            fallback={<Empty query={query()} />}
          >
            <Indexing />
          </Show>
        }
      >
        <ul class="max-h-[520px] overflow-y-auto px-2 pb-2">
          <For each={results()}>
            {(app, i) => (
              <>
                <Show when={favCount() > 0 && i() === favCount() && favCount() < results().length}>
                  <li aria-hidden="true" class="mx-3 my-1 border-t border-black/10 dark:border-white/10" />
                </Show>
                <Row
                  app={app}
                  index={i()}
                  active={i() === selected()}
                  pinned={favorites().includes(app.path)}
                  onHover={() => setSelected(i())}
                  onClick={() => void activate(app)}
                />
              </>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

function Row(props: {
  app: AppEntry;
  index: number;
  active: boolean;
  pinned: boolean;
  onHover: () => void;
  onClick: () => void;
}) {
  let el: HTMLLIElement | undefined;
  // Keep the keyboard-selected row visible as arrows move past the fold.
  createEffect(() => {
    if (props.active) el?.scrollIntoView({ block: "nearest" });
  });
  return (
    <li
      ref={el}
      onMouseMove={props.onHover}
      onClick={props.onClick}
      class="flex h-12 cursor-pointer items-center gap-3 rounded-xl px-3 transition-colors"
      classList={{
        "bg-accent text-white": props.active,
        "hover:bg-black/5 dark:hover:bg-white/5": !props.active,
      }}
    >
      <AppIcon path={props.app.path} />
      <span class="flex-1 truncate text-[0.9375rem]">{props.app.name}</span>
      {/* pinned indicator */}
      <Show when={props.pinned}>
        <span
          aria-label={t("launcher.pinned")}
          class="shrink-0 text-xs"
          classList={{
            "text-white/80": props.active,
            "text-neutral-500 dark:text-neutral-400": !props.active,
          }}
        >
          ★
        </span>
      </Show>
      {/* ⌘1..⌘9 / ⌘0 for the first ten results */}
      <Show when={props.index < 10}>
        <span
          class="shrink-0 rounded-md px-1.5 py-0.5 text-xs tabular-nums"
          classList={{
            "bg-white/20 text-white": props.active,
            "bg-black/5 text-neutral-500 dark:bg-white/10 dark:text-neutral-400":
              !props.active,
          }}
        >
          ⌘{(props.index + 1) % 10}
        </span>
      </Show>
    </li>
  );
}

function Empty(props: { query: string }) {
  return (
    <div class="flex h-20 items-center justify-center text-sm text-neutral-400">
      {props.query ? t("launcher.noMatch") : t("launcher.startTyping")}
    </div>
  );
}

// First-launch state: the index is still being built, so there's nothing to show yet.
function Indexing() {
  return (
    <div class="flex h-20 items-center justify-center gap-2 text-sm text-neutral-400">
      <span class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-current border-t-transparent opacity-70" />
      <span>{t("launcher.indexing")}</span>
    </div>
  );
}

function SearchGlyph() {
  return (
    <svg
      class="h-6 w-6 shrink-0 text-neutral-400"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
    >
      <circle cx="11" cy="11" r="7" />
      <path d="m21 21-4.3-4.3" stroke-linecap="round" />
    </svg>
  );
}

function GearGlyph() {
  return (
    <svg
      class="h-4 w-4 shrink-0"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
    >
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}
