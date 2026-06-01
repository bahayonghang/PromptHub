import { defineConfig } from "vitest/config";

// Unit/property tests run in a Node environment; the Runtime_Bridge under test
// receives its Tauri primitives via dependency injection, so no webview is needed.
//
// Component render/interaction tests (`*.test.tsx`) opt into a DOM via a per-file
// `// @vitest-environment jsdom` docblock, keeping the Node default for the
// dependency-injected logic/property tests.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
