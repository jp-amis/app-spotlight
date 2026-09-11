import { defineConfig } from "vitest/config";

// Pure-logic unit tests (matcher, accelerator) — no DOM, no Solid transform.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
