// Translate a keyboard event into a Tauri accelerator string, and format
// accelerators for display. Kept pure + testable (plan 0004).

const MODIFIER_CODES = new Set([
  "MetaLeft",
  "MetaRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
]);

/**
 * Build a Tauri accelerator (e.g. "Cmd+Shift+Space") from a keydown event.
 * Returns null for modifier-only presses or combos with no modifier
 * (a global hotkey needs at least one modifier).
 */
export function eventToAccelerator(e: {
  code: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;

  const mods: string[] = [];
  if (e.metaKey) mods.push("Cmd");
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");

  const key = keyName(e.code);
  if (!key || mods.length === 0) return null;

  return [...mods, key].join("+");
}

function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (/^Arrow(Up|Down|Left|Right)$/.test(code)) return code.slice(5);

  const map: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    Minus: "-",
    Equal: "=",
    Comma: ",",
    Period: ".",
    Slash: "/",
    Backslash: "\\",
    Semicolon: ";",
    Quote: "'",
    BracketLeft: "[",
    BracketRight: "]",
    Backquote: "`",
  };
  return map[code] ?? null;
}

const SYMBOLS: Record<string, string> = {
  Cmd: "⌘",
  Command: "⌘",
  CommandOrControl: "⌘",
  CmdOrCtrl: "⌘",
  Super: "⌘",
  Meta: "⌘",
  Ctrl: "⌃",
  Control: "⌃",
  Alt: "⌥",
  Option: "⌥",
  Shift: "⇧",
};

/** Pretty-print an accelerator using mac glyphs, e.g. "Cmd+Shift+Space" → "⌘⇧Space". */
export function acceleratorToSymbols(accelerator: string): string {
  return accelerator
    .split("+")
    .map((t) => SYMBOLS[t] ?? t)
    .join(" ");
}
