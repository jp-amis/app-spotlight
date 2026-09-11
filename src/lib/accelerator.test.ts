import { describe, it, expect } from "vitest";
import { eventToAccelerator, acceleratorToSymbols } from "./accelerator";

const ev = (over: Partial<Parameters<typeof eventToAccelerator>[0]>) => ({
  code: "Space",
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...over,
});

describe("eventToAccelerator", () => {
  it("builds the default combo", () => {
    expect(
      eventToAccelerator(ev({ code: "Space", metaKey: true, shiftKey: true })),
    ).toBe("Cmd+Shift+Space");
  });

  it("maps letter codes", () => {
    expect(eventToAccelerator(ev({ code: "KeyK", metaKey: true }))).toBe("Cmd+K");
  });

  it("rejects modifier-only presses", () => {
    expect(eventToAccelerator(ev({ code: "MetaLeft", metaKey: true }))).toBeNull();
  });

  it("rejects combos without a modifier", () => {
    expect(eventToAccelerator(ev({ code: "KeyK" }))).toBeNull();
  });
});

describe("acceleratorToSymbols", () => {
  it("renders mac glyphs", () => {
    expect(acceleratorToSymbols("Cmd+Shift+Space")).toBe("⌘ ⇧ Space");
    expect(acceleratorToSymbols("CommandOrControl+K")).toBe("⌘ K");
  });
});
