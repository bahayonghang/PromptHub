# Backend Guidelines

This file governs `src-tauri/**`. Root guidance still applies; this file adds the
Rust/Tauri backend rules and overrides.

For this subtree, start with `src-tauri/code_map.md` before broad grep. If that
file is missing, fall back to `../code_map.md`.

## Runtime and Layering

Rust 2021 (edition), MSRV 1.77.2, Tauri 2. Three layers, kept separate:

- `commands/` — thin Tauri adapter. Maps a pooled connection / app handle into a
  service call and converts `Result<T, AppError>` into `CommandResult<T>`.
- `services/` — business rules. Written against injected I/O (a borrowed
  `rusqlite::Connection`, sinks, gateways) so they are unit/property testable
  without a live window. Services must not depend on Tauri.
- `storage/` — SQLite engine: r2d2 pool, PRAGMAs, schema, FTS, time/mapping helpers.

## Commands

Run from `src-tauri/`:

- `cargo test` — unit tests (in-module `#[cfg(test)]`) + property tests under
  `tests/` and `src/storage/proptest_*`.
- `cargo test <name>` — focused run while iterating.
- `cargo fmt` / `cargo clippy` — format and lint gates.
- `cargo build` — compile only. Full app build/run is `npm run tauri {dev,build}`
  from the repo root.

## Hard Rules

- Every command returns `CommandResult<T>` and is registered in the
  `invoke_handler![...]` list in `src/lib.rs`. Adding a command means: write the
  service, add the `#[tauri::command(rename = "domain.action")]` adapter, AND add
  it to that handler list — all three.
- The `rename = "domain.action"` wire name is the frontend contract. Do not change
  an existing name without updating the frontend caller.
- Errors use the `AppError` constructors and the `ErrorCode` taxonomy in
  `src/error.rs`; the string codes (`as_str`) are a stable contract — keep them.
- Event channel names in `commands/events.rs` are pinned by a test; treat them as
  contract.
- Commands run only after startup readiness — go through `conn(&state)` /
  `ensure_ready` rather than touching the pool directly.

## Data and External Services

- The schema (`SCHEMA_SQL` in `storage/mod.rs`) is created idempotently with
  `IF NOT EXISTS`. There is no migration system and no Electron-data migration;
  treat schema edits as a deliberate, reviewed change.
- Outbound HTTP (AI client, remote skill fetch, media download, sync) uses
  reqwest + rustls and must enforce the SSRF policy (`SSRF_BLOCKED`); redirects
  are re-checked per hop. Do not weaken these checks.
- Crypto (`services/security.rs`): scrypt-derived key + AES-256-GCM. Never log or
  serialize derived keys or master passwords to the frontend.
- Ask before changes touching real sync/backup endpoints, credentials, or the
  updater signing flow.

## Safety and Permissions

- Do not edit generated output: `target/`, `gen/schemas/`.
- `proptest-regressions/` holds saved failing cases; keep them — they are
  regression seeds, not scratch files.
- The updater `pubkey` in `tauri.conf.json` is a placeholder; real signing keys
  are secrets and must stay out of the repo.
