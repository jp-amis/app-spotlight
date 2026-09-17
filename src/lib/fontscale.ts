// UI text-size scaling. We set the root `html` font-size so every rem-based Tailwind
// utility (text, spacing, icon sizes, row heights) scales proportionally — a clean
// whole-UI zoom. Applied in both windows and kept in sync via the `font:changed`
// event (mirrors the i18n `language:changed` pattern).
import { createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { getSettings } from "./api";

export type FontScale = "smaller" | "normal" | "bigger" | "extra_big";

const FACTORS: Record<FontScale, number> = {
  smaller: 0.9,
  normal: 1,
  bigger: 1.15,
  extra_big: 1.3,
};

const BASE_PX = 16;
const CACHE_KEY = "fontScale";

function normalize(s: unknown): FontScale {
  return s === "smaller" || s === "bigger" || s === "extra_big" ? s : "normal";
}

const [fontScale, setFontScale] = createSignal<FontScale>("normal");
export { fontScale };

/** Set the root font-size (so the whole rem-based UI scales) and remember it. */
export function applyFontScale(s: FontScale) {
  document.documentElement.style.fontSize = `${BASE_PX * FACTORS[s]}px`;
  setFontScale(s);
  try {
    localStorage.setItem(CACHE_KEY, s);
  } catch {
    /* ignore */
  }
}

// Apply the last-known size synchronously at module load, before first paint, so the
// window never flashes at the default size while getSettings() resolves.
try {
  const cached = localStorage.getItem(CACHE_KEY);
  if (cached) applyFontScale(normalize(cached));
} catch {
  /* ignore */
}

let initialized = false;

/** Load the persisted size from the backend and keep it in sync across windows. */
export async function initFontScale(): Promise<void> {
  try {
    const s = await getSettings();
    applyFontScale(normalize(s.font_scale));
  } catch {
    /* no backend (dev/browser) — keep the cached/default size */
  }
  if (initialized) return;
  initialized = true;
  void listen<{ scale?: string }>("font:changed", (e) => {
    applyFontScale(normalize(e.payload?.scale));
  });
}
