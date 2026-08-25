# Backend Guidelines

This file adds the rules for `src-tauri/**`. The root `AGENTS.md` still applies,
including the `CommandResult` contract and the verification gates.

Start with `src-tauri/code_map.md` before a broad search in `src-tauri/**`.

Rust 2021, MSRV 1.77.2, Tauri 2.

## Layering

- `commands/` — the thin Tauri adapter. It maps a pooled connection or an app
  handle into a service call, then converts `Result<T, AppError>` into
  `CommandResult<T>`.
- `services/` — the business rules. Write them against injected I/O (a borrowed
  `rusqlite::Connection`, sinks, gateways) so a unit or property test runs
  without a live window. A service must not depend on Tauri.
- `storage/` — the SQLite engine: r2d2 pool, PRAGMAs, schema, FTS, and
  time/mapping helpers.

## Hard rules

- A new command needs all three parts: the service, the
  `#[tauri::command(rename = "domain.action")]` adapter, and an entry in the
  `invoke_handler![...]` list in `src-tauri/src/lib.rs`.
- The `rename = "domain.action"` wire name is the frontend contract. Do not
  change an existing name without the frontend caller.
- Errors use the `AppError` constructors and the `ErrorCode` taxonomy in
  `src-tauri/src/error.rs`. The `as_str` string codes are a stable contract.
  Keep them.
- The event channel names in `commands/events.rs` are pinned by a test. Treat
  them as a contract.
- Get a connection through `conn(&state)` or `ensure_ready`. Do not touch the
  pool directly. Commands run only after startup readiness.
- Use `cargo test <name>` for a focused run while you iterate.

## Data and external services

- A fresh database uses `SCHEMA_SQL` in `storage/mod.rs`. An existing database
  uses the ordered transactional `MIGRATIONS` list and the SQLite `user_version`,
  after a required startup safety backup. There is no Electron-data migration.
  Treat a schema edit as a deliberate, reviewed change and add an upgrade test.
- Outbound HTTP (AI client, media download, sync) uses reqwest with rustls and
  must enforce the SSRF policy (`SSRF_BLOCKED`). Each redirect hop is re-checked.
  Do not weaken these checks.
- Crypto (`services/security.rs`) uses a scrypt-derived key with AES-256-GCM.
  Never log a derived key or a master password. Never serialize one to the
  frontend.
- Keep `proptest-regressions/`. Those files are regression seeds.
