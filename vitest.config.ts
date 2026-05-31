import { defineConfig } from "vitest/config";

// Unit/property tests run in a Node environment; the Runtime_Bridge under test
// receives its Tauri primitives via dependency injection, so no webview is needed.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
