// Lightweight fuzzy matcher + ranking for the app palette (plan 0006).
// Runs synchronously over the preloaded app array on every keystroke — no IPC.

export interface AppEntry {
  name: string;
  path: string;
  bundle_id?: string | null;
}

export interface Ranked extends AppEntry {
  score: number;
}

/**
 * Score `query` against `text`. Returns a number where higher is better,
 * or -Infinity if `query` is not a subsequence of `text`.
 *
 * Scoring favors, in order: exact match, prefix match, word-boundary hits,
 * and contiguous runs. Case-insensitive.
 */
export function score(query: string, text: string): number {
  if (query.length === 0) return 0;
  const q = query.toLowerCase();
  const t = text.toLowerCase();

  if (t === q) return 10_000;
  if (t.startsWith(q)) return 5_000 - t.length;

  let qi = 0;
  let s = 0;
  let run = 0;
  let prevMatchIdx = -2;
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      let bonus = 1;
      // word-boundary bonus (start, or preceded by a separator)
      const prev = ti > 0 ? t[ti - 1] : " ";
      if (ti === 0 || prev === " " || prev === "-" || prev === "_" || prev === ".") {
        bonus += 8;
      }
      // contiguous-run bonus
      if (prevMatchIdx === ti - 1) {
        run += 1;
        bonus += run * 2;
      } else {
        run = 0;
      }
      s += bonus;
      prevMatchIdx = ti;
      qi++;
    }
  }
  if (qi < q.length) return -Infinity; // not all query chars matched
  // prefer shorter targets on ties
  return s - t.length * 0.1;
}

export type Frecency = Record<string, { count?: number } | undefined>;

/**
 * Filter + rank apps for a query. With an empty query, pinned `favorites` come first
 * (in their pinned order), then the most-frecent apps fill the rest. Results are capped
 * to `limit`.
 */
export function rank(
  apps: AppEntry[],
  query: string,
  frecency: Frecency = {},
  limit = 8,
  favorites: string[] = [],
): Ranked[] {
  const q = query.trim();

  if (q.length === 0) {
    const byPath = new Map(apps.map((a) => [a.path, a]));
    const favSet = new Set(favorites);
    const pinned: Ranked[] = favorites
      .map((p) => byPath.get(p))
      .filter((a): a is AppEntry => !!a)
      .map((a) => ({ ...a, score: 0 }));
    const rest = apps
      .filter((a) => !favSet.has(a.path))
      .map((a) => ({ ...a, score: frecency[a.path]?.count ?? 0 }))
      .sort((a, b) => b.score - a.score || a.name.localeCompare(b.name));
    return [...pinned, ...rest].slice(0, limit);
  }

  const out: Ranked[] = [];
  for (const a of apps) {
    const base = score(q, a.name);
    if (base === -Infinity) continue;
    // small frecency boost so frequently-launched apps float up on ties
    const boost = Math.min((frecency[a.path]?.count ?? 0), 20) * 3;
    out.push({ ...a, score: base + boost });
  }
  out.sort((a, b) => b.score - a.score || a.name.localeCompare(b.name));
  return out.slice(0, limit);
}
