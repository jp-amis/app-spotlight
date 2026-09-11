# 0006 — Search UX & keyboard navigation

## Goal

Instant, fuzzy, keyboard-first result list. Zero-lag typing. Mouse supported but never required.

## Matching

- In-memory fuzzy match over the preloaded app array (no IPC per keystroke).
- Algorithm: subsequence/fuzzy scoring (e.g. lightweight fzf-style) with ranking by: exact prefix > word-boundary > fuzzy; tiebreak by recent/frequent usage (frecency).
- Frecency: persist launch counts + last-used per app (in store); boost ranking. Empty query → top frecency apps.

## Keyboard model (hard requirement)

- Focus is always in the search input; list navigation via keys without leaving input.
- `↑/↓` or `Ctrl+P/Ctrl+N`: move selection.
- `Enter`: launch selected. `Cmd+Enter` (future: open enclosing folder / alt action).
- `Esc`: clear query if non-empty, else hide window.
- `Tab`: (reserved for future actions/sub-results).
- `1..9` (Cmd+n): quick-launch nth result (optional).
- Selection wraps or clamps (decide); always one item selected.

## Rendering performance

- Solid fine-grained updates; render only the visible top-N (cap ~8–10) — no full-list DOM.
- Virtualize only if we later show long lists; v1 caps results so no virtualization needed.
- Debounce NOT needed (filtering is cheap + sync); render on every keystroke for immediacy.
- Avoid layout thrash: fixed row height, `content-visibility` where useful.

## Mouse support

- Hover to highlight, click to launch. Scroll for overflow. Never steals the "one selected item" invariant on keydown.

## Tasks

- [ ] Fuzzy matcher + ranking (with frecency hook).
- [ ] Result list component (fixed rows, top-N, selection state).
- [ ] Full keybinding map above.
- [ ] Frecency persistence + boosting.
- [ ] Empty-state (top apps) + no-results state.
- [ ] Latency check: keystroke→paint under a few ms with 200+ apps.

## Risks

- Fuzzy lib size/perf — pick tiny or hand-roll.
- Frecency store writes on every launch — batch/async, off hot path.

## Unresolved questions

- Selection wrap-around vs clamp at ends?
- Ship frecency in v1 or alphabetical + prefix only first?
