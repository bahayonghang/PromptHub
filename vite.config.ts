import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @tauri-apps/cli sets TAURI_DEV_HOST when running on a mobile/remote target.
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],

  // Tauri expects a fixed port and fails if it is not available.
  // Windows Hyper-V / WinNAT often reserves 1390-1489, so the old Tauri
  // default 1420 fails with EACCES. 5173 is Vite's default and sits outside
  // those ranges.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Don't watch the Rust backend or the read-only reference tree.
      ignored: ["**/src-tauri/**", "**/ref/**"],
    },
  },
  build: {
    rollupOptions: {
      output: {
        // Keep the entry under Rollup's 500 kB warning. Locales already load
        // via dynamic import(); vendor libraries stay in their own files.
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return undefined;
          }
          if (id.includes("lucide-react")) {
            return "icons";
          }
          if (id.includes("i18next")) {
            return "i18n";
          }
          // Match the package directory so `react` does not catch `react-dom`.
          if (/[/\\](?:react-dom|scheduler|react)[/\\]/.test(id)) {
            return "react-vendor";
          }
          return undefined;
        },
      },
    },
  },
});
