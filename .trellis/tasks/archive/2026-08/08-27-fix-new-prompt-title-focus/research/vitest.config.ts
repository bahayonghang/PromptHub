import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    include: [
      ".trellis/tasks/archive/2026-08/08-27-fix-new-prompt-title-focus/research/*.test.tsx",
    ],
  },
});
