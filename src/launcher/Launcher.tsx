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
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { rank, type AppEntry, type Frecency } from "../lib/fuzzy";
import {
  accessStatus,
  getFavorites,
  getSettings,
  grantFolder,
  hideLauncher,
  launchApp,
  listApps,
  openSettings,
  recenterLauncher,
  setFavorites as saveFavoritesApi,
  setLauncherHeight,
} from "../lib/api";
import AppIcon from "./AppIcon";

const BANNER_DISMISS_KEY = "accessBannerDismissed";

const WINDOW_WIDTH = 680;

export default function Launcher() {
  const [apps, setApps] = createSignal<AppEntry[]>([]);
  const [query, setQuery] = createSignal("");
  const [selected, setSelected] = createSignal(0);
  const [frecency, setFrecency] = createSignal<Frecency>({});
  const [favorites, setFavorites] = createSignal<string[]>([]);
  const [limit, setLimit] = createSignal(10);
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
      setFavorites(await getFavorites());
    } catch {
      /* no backend */
    }
  }

  // Resize the window to fit its content so the last row is never clipped
  // (the results list scrolls past its max-height).
  let lastPersistedHeight = 0;
  function syncWindowHeight() {
    if (!rootEl) return;
    const h = Math.ceil(rootEl.getBoundingClientRect().height);
    if (h < 60) return;
    void getCurrentWindow().setSize(new LogicalSize(WINDOW_WIDTH, h)).catch(() => {});
    // Persist so the next launch pre-sizes the window (no first-open resize blink).
    if (Math.abs(h - lastPersistedHeight) >= 1) {
      lastPersistedHeight = h;
      void setLauncherHeight(h);
    }
  }

  createEffect(() => {
    results(); // re-measure whenever the visible results change
    showBanner(); // ...or when the access banner appears/disappears
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

  onMount(() => {
    void refreshApps();
    void refreshSettings();
    void refreshAccess();
    void refreshFavorites();
    inputEl?.focus();

    // Backend fires this each time the window is shown via the global shortcut.
    const un = listen("launcher:opened", () => {
      setQuery("");
      setSelected(0);
      void refreshSettings();
      void refreshAccess();
      void refreshFavorites();
      queueMicrotask(() => inputEl?.focus());
    });
    // A grant from the Settings window reindexes; refresh our list when it does.
    const un2 = listen("apps:reindexed", () => {
      void refreshApps();
      void refreshAccess();
    });
    const un3 = listen("favorites:changed", () => void refreshFavorites());
    onCleanup(() => {
      void un.then((f) => f());
      void un2.then((f) => f());
      void un3.then((f) => f());
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
    // ⌘Q closes the launcher (it never quits the app).
    if (e.metaKey && e.code === "KeyQ") {
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
             text-neutral-900 ring-1 ring-black/10
             dark:text-neutral-100 dark:ring-white/10"
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
          placeholder="Search apps…"
          spellcheck={false}
          autocapitalize="off"
          autocomplete="off"
          class="w-full bg-transparent text-2xl font-light outline-none
                 placeholder:text-neutral-500 dark:placeholder:text-neutral-500"
        />
      </div>

      <Show when={showBanner()}>
        <div class="mx-3 mb-2 flex items-center gap-2 rounded-xl bg-amber-500/15 px-3 py-2 text-xs text-amber-700 dark:text-amber-300">
          <span class="flex-1">
            Some apps may be hidden. My App Spot is sandboxed — grant a folder (e.g. your
            Applications) to find more.
          </span>
          <button
            onClick={() => void grant()}
            class="rounded-lg bg-amber-500/25 px-2 py-1 font-medium hover:bg-amber-500/40"
          >
            Grant Access
          </button>
          <button
            onClick={dismissBanner}
            aria-label="Dismiss"
            class="px-1 text-amber-700/70 hover:text-amber-700 dark:text-amber-300/70"
          >
            ✕
          </button>
        </div>
      </Show>

      <Show when={results().length > 0} fallback={<Empty query={query()} />}>
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
      <span class="flex-1 truncate text-[15px]">{props.app.name}</span>
      {/* pinned indicator */}
      <Show when={props.pinned}>
        <span
          aria-label="Pinned"
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
      {props.query ? "No matching apps" : "Start typing to search"}
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
