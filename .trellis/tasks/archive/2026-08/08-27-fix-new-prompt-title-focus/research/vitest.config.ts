import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    include: [
      ".trellis/tasks/08-27-fix-new-prompt-title-focus/research/*.test.tsx",
    ],
  },
});
