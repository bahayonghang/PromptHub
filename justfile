# PromptHub Desktop task runner.
# Run `just` (or `just --list`) to see all recipes.
# Frontend tooling uses the root package.json; the Rust backend lives in src-tauri/.

set windows-shell := ["cmd.exe", "/c"]

# Path to the Rust backend manifest (avoids `cd` so recipes stay shell-agnostic).
manifest := "src-tauri/Cargo.toml"

# List available recipes.
default:
    @just --list

# --- Setup ---

# Install frontend dependencies.
install:
    npm install

# --- Dev ---

# Frontend-only dev server on http://localhost:1420.
dev:
    npm run dev

# Full desktop dev environment (Vite + native window + Rust backend).
tauri-dev:
    npm run tauri dev

# Short alias for `tauri-dev` (frontend-only `dev` cannot reach the backend).
alias tdev := tauri-dev

# --- Build ---

# Frontend gate: tsc typecheck + Vite production build.
build:
    npm run build

# Package desktop installers into src-tauri/target/release/bundle/.
tauri-build:
    npm run tauri build

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
