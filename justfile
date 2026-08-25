# PromptHub Desktop task runner.
# Run `just` (or `just --list`) to see all recipes.
# Frontend tooling uses the root package.json; the Rust backend lives in src-tauri/.

set windows-shell := ["cmd.exe", "/c"]

# Path to the Rust backend manifest (avoids `cd` so recipes stay shell-agnostic).
manifest := "src-tauri/Cargo.toml"

# List available recipes.
default:
    @just --list

# Show available recipes.
help:
    @just --list

# --- Setup ---

# Install frontend dependencies.
deps:
    npm install

# --- Dev ---

# Full desktop dev environment (Vite + native window + Rust backend).
dev:
    npm run tauri dev

# Frontend-only Vite server on http://localhost:5173.
frontend:
    npm run dev

# Backward-compatible aliases for `dev`.
alias tauri-dev := dev
alias tdev := dev

# --- Build ---

# Frontend gate: tsc typecheck + Vite production build.
build:
    npm run build

# Package desktop installers into src-tauri/target/release/bundle/.
tauri-build:
    npm run tauri build

# Build and install PromptHub on this machine.
[script("node")]
install: tauri-build
    const { existsSync, readdirSync, statSync } = require("node:fs");
    const { basename, join, resolve } = require("node:path");
    const { spawnSync } = require("node:child_process");

    const bundleRoot = resolve("src-tauri", "target", "release", "bundle");

    function newestEntry(directory, predicate) {
      if (!existsSync(directory)) return null;
      return readdirSync(directory)
        .map((name) => join(directory, name))
        .filter(predicate)
        .sort((left, right) => statSync(right).mtimeMs - statSync(left).mtimeMs)[0] ?? null;
    }

    function run(command, args) {
      const result = spawnSync(command, args, { stdio: "inherit" });
      if (result.error) throw result.error;
      if (result.status !== 0) process.exit(result.status ?? 1);
    }

    if (process.platform === "win32") {
      const nsis = newestEntry(join(bundleRoot, "nsis"), (path) => path.endsWith(".exe"));
      if (nsis) {
        run(nsis, []);
        process.exit(0);
      }

      const msi = newestEntry(join(bundleRoot, "msi"), (path) => path.endsWith(".msi"));
      if (msi) {
        run("msiexec.exe", ["/i", msi]);
        process.exit(0);
      }

      throw new Error("No Windows installer was produced by tauri-build");
    }

    if (process.platform === "darwin") {
      const app = newestEntry(
        join(bundleRoot, "macos"),
        (path) => path.endsWith(".app") && statSync(path).isDirectory(),
      );
      if (!app) throw new Error("No macOS app bundle was produced by tauri-build");
      run("sudo", ["ditto", app, join("/Applications", basename(app))]);
      process.exit(0);
    }

    if (process.platform === "linux") {
      const deb = newestEntry(join(bundleRoot, "deb"), (path) => path.endsWith(".deb"));
      if (!deb) throw new Error("No Debian package was produced by tauri-build");
      run("sudo", ["apt-get", "install", "-y", deb]);
      process.exit(0);
    }

    throw new Error("Unsupported platform: " + process.platform);

# Backward-compatible alias for `install`.
alias tinstall := install

# --- Test ---

# Frontend Vitest suite.
test:
    npm run test

# Backend unit + property tests.
test-rust:
    cargo test --manifest-path {{manifest}}

# --- Lint / Format (backend) ---

# Format Rust code in place.
fmt:
    cargo fmt --manifest-path {{manifest}}

# Verify Rust formatting without writing (CI-friendly).
fmt-check:
    cargo fmt --manifest-path {{manifest}} --check

# Lint Rust code; warnings fail the build.
clippy:
    cargo clippy --manifest-path {{manifest}} --all-targets -- -D warnings

# --- CI ---

# Full gate: frontend build + tests, then backend format, lint, and tests.
ci: build test fmt-check clippy test-rust
