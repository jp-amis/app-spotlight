import { createSignal } from "solid-js";
import { acceleratorToSymbols, eventToAccelerator } from "../lib/accelerator";
import { resumeShortcut, suspendShortcut } from "../lib/api";

export default function ShortcutCapture(props: {
  value: string;
  onChange: (accelerator: string) => void;
}) {
  const [capturing, setCapturing] = createSignal(false);

  function start() {
    if (capturing()) return;
    setCapturing(true);
    // Suspend the global shortcut so the combo reaches us instead of firing the launcher.
    void suspendShortcut();
  }

  // `restore` re-registers the previous shortcut (used when the capture is cancelled).
  // On a successful capture we skip it — the parent's setShortcut registers the new one.
  function stop(restore: boolean) {
    if (!capturing()) return;
    setCapturing(false);
    if (restore) void resumeShortcut();
  }

  function onKeyDown(e: KeyboardEvent) {
    // Not capturing: behave like a normal button — let Tab move focus and
    // Enter/Space trigger the click (which starts capture). Don't swallow keys.
    if (!capturing()) return;
    // Tab (or Shift+Tab) leaves the field and cancels capture.
    if (e.key === "Tab") {
      stop(true);
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      stop(true);
      return;
    }
    const accel = eventToAccelerator(e);
    if (accel) {
      props.onChange(accel);
      stop(false);
    }
  }

  return (
    <button
      type="button"
      onClick={start}
      onKeyDown={onKeyDown}
      onBlur={() => stop(true)}
      class="min-w-32 rounded-lg border px-3 py-1.5 text-sm transition-colors"
      classList={{
        "border-[var(--color-accent)] bg-[var(--color-accent)]/10": capturing(),
        "border-black/10 bg-white dark:border-white/10 dark:bg-neutral-800":
          !capturing(),
      }}
    >
      {capturing() ? "Press keys…" : acceleratorToSymbols(props.value)}
    </button>
  );
}
