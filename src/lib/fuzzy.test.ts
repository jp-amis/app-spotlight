import { describe, it, expect } from "vitest";
import { score, rank, type AppEntry } from "./fuzzy";

describe("score", () => {
  it("rewards exact and prefix matches highest", () => {
    expect(score("safari", "Safari")).toBeGreaterThan(score("saf", "Safari"));
    expect(score("saf", "Safari")).toBeGreaterThan(score("fri", "Safari"));
  });

  it("returns -Infinity when query is not a subsequence", () => {
    expect(score("xyz", "Safari")).toBe(-Infinity);
  });

  it("matches subsequences across word boundaries", () => {
    expect(score("am", "Activity Monitor")).toBeGreaterThan(0);
    expect(score("actmon", "Activity Monitor")).toBeGreaterThan(0);
  });

  it("empty query scores neutral", () => {
    expect(score("", "Anything")).toBe(0);
  });
});

const apps: AppEntry[] = [
  { name: "Safari", path: "/Applications/Safari.app" },
  { name: "System Settings", path: "/Applications/System Settings.app" },
  { name: "Slack", path: "/Applications/Slack.app" },
  { name: "Activity Monitor", path: "/System/Applications/Utilities/Activity Monitor.app" },
];

describe("rank", () => {
  it("filters out non-matches", () => {
    const r = rank(apps, "saf");
    expect(r.map((a) => a.name)).toEqual(["Safari"]);
  });

  it("ranks prefix match above mid-word match", () => {
    const r = rank(apps, "s");
    // Safari/System Settings/Slack all start with S; Activity Monitor should not appear
    expect(r.map((a) => a.name)).not.toContain("Activity Monitor");
    expect(r[0].name.toLowerCase().startsWith("s")).toBe(true);
  });

  it("empty query returns frecent-first, capped by limit", () => {
    const frecency = { "/Applications/Slack.app": { count: 9 } };
    const r = rank(apps, "", frecency, 2);
    expect(r.length).toBe(2);
    expect(r[0].name).toBe("Slack");
  });

  it("frecency boosts ties on non-empty query", () => {
    const r = rank(apps, "s", { "/Applications/Slack.app": { count: 15 } }, 8);
    expect(r[0].name).toBe("Slack");
  });

  it("empty query shows pinned favorites first, in order", () => {
    const favorites = ["/Applications/Slack.app", "/Applications/Safari.app"];
    const r = rank(apps, "", { "/Applications/System Settings.app": { count: 99 } }, 4, favorites);
    // pinned first (in pin order), regardless of frecency, then the rest by frecency
    expect(r.slice(0, 2).map((a) => a.name)).toEqual(["Slack", "Safari"]);
    expect(r[2].name).toBe("System Settings"); // highest frecency of the non-pinned
    // a pinned app never appears twice
    expect(r.filter((a) => a.name === "Slack").length).toBe(1);
  });
});
