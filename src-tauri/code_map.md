# `src-tauri` Code Map

Use this map for `src-tauri/**` navigation. Behavioral rules and commands live in
`src-tauri/AGENTS.md` (or the root `AGENTS.md`).

## Subtree Responsibility

The Rust/Tauri backend: the command adapter layer, the Tauri-free business
services, the SQLite storage engine, and the shared application state. It owns
everything the original Electron main process did.

## Internal Routing

- `src/commands/` — Tauri command adapters, one module per domain; start here to
  find a command's entry point.
- `src/services/` — business logic, one module per domain; start here for behavior.
- `src/storage/` — SQLite pool, schema, FTS, time/mapping helpers.
- `src/models/` — domain types and DTOs (serde shapes shared with the frontend).
- `tests/` — integration/property tests (`*_properties.rs`).
- `capabilities/` — Tauri ACL capability definitions (`default.json`).

## Key Files

- `src/lib.rs` — `run()`: plugin registration, startup sequence, and the
  authoritative `invoke_handler![...]` command registry.
- `src/main.rs` — binary entry; calls `prompthub_lib::run()`.
- `src/error.rs` — `AppError`, `ErrorCode`, and the `CommandResult<T>` envelope.
- `src/state.rs` — `AppState` (pool, runtime paths, encryption, request registry,
  readiness gate) and `RuntimePaths`.
- `src/commands/mod.rs` — shared command helpers: `conn`, `ensure_ready`,
  `into_command`, `CommandRuntimeState`, `app_status`.
- `src/commands/events.rs` — event channel names + payloads and the AI/updater sinks.
- `src/storage/mod.rs` — `create_pool`, `init_schema`, and `SCHEMA_SQL` (full DDL).
- `Cargo.toml` — dependencies, each annotated with the requirement it serves.
- `tauri.conf.json` — window/bundle/updater config.

## Domain Modules

Each domain typically has a matching `commands/<d>.rs` + `services/<d>.rs` pair:
`prompt`, `folder`, `version` (prompt versions), `skill` (+ `skill_local`,
`skill_md`, `skill_platform`, `skill_safety` services), `rules`, `settings`,
`security`, `data_path`, `sync`, `ai`, `media`, `window`, `updater`.

## Upstream and Downstream Boundaries

- Upstream: the frontend `src/runtime` invokes commands by their `rename` wire
  name and subscribes to the event channels declared in `commands/events.rs`.
- Downstream: services depend on `storage` (pooled `rusqlite::Connection`) and on
  external services (HTTP via reqwest/rustls, WebDAV, S3) behind injected I/O.

## Local Search Anchors

- `#[tauri::command` / `rename = "` — command definitions and their wire names.
- `invoke_handler!` (in `lib.rs`) — the complete registered command set.
- `SCHEMA_SQL` — the SQLite DDL.
- `EVENT_` (in `events.rs`) — event channel name constants.
- `AppError::` / `ErrorCode::` — error construction and the code taxonomy.
- `EventSink` / `UpdaterEventSink` — the streaming/progress injection points.

## Generated or Ignored Local Paths

- `target/` — Cargo build output; do not edit (gitignored).
- `gen/schemas/` — Tauri-generated ACL/capability schemas; regenerated on build.
- `proptest-regressions/` — saved property-test failure seeds; keep, do not edit.
- `Cargo.lock` — managed by Cargo.
