# Repository Code Map

Use this map for navigation and search routing. Behavioral rules, required
commands, and safety constraints live in `AGENTS.md`.

## Top-Level Routing

- `src/` — React/TypeScript frontend; start here for UI, state, i18n, and the
  Runtime Bridge. See `src/code_map.md`.
- `src-tauri/` — Rust/Tauri backend; start here for commands, services, storage,
  and the SQLite schema. See `src-tauri/code_map.md`.
- `.trellis/workflow.md` — task lifecycle, planning, implementation, and finish
  rules for Trellis-managed sessions.
- `.trellis/spec/` — project coding guidelines loaded for Trellis tasks; start
  here when updating durable AI-facing conventions.
- `.trellis/tasks/` — active and archived task artifacts, PRDs, and context.
- `ref/PromptHub/` — read-only Electron reference implementation; consult for
  intended behavior, never edit.
- `index.html` — Vite HTML entry; loads `src/main.tsx`.

## Key Entrypoints

- `src/main.tsx` — frontend bootstrap (theme + i18n + React root).
- `src/runtime/index.ts` — Runtime Bridge: the only path from frontend to backend.
- `src-tauri/src/lib.rs` — `run()` builds the Tauri app and registers every
  command in `invoke_handler!` (the full command list lives here).
- `src-tauri/src/main.rs` — thin binary entry calling `prompthub_lib::run()`.
- `src-tauri/tauri.conf.json` — window, bundle, and updater configuration.

## Cross-Boundary Contract

- Commands are named `domain.action` (e.g. `prompt.create`, `settings.get`) on
  the wire via `#[tauri::command(rename = "...")]`; the frontend calls them by
  that string through `runtime.invoke(...)`.
- Every command returns `CommandResult<T>` = `{ ok: true, data } | { ok: false, error }`.
  Frontend mirror: the `CommandResult<T>` type in `src/runtime/index.ts`.
- Events are named `domain:action` (e.g. `ai:stream-chunk`, `updater:status`);
  defined in `src-tauri/src/commands/events.rs`, subscribed via `runtime.on(...)`.

## Search Anchors

- `CommandResult` — the success/error envelope (both Rust and TS sides).
- `ErrorCode` / `as_str` — the stable error-code taxonomy in `src-tauri/src/error.rs`.
- `createRuntimeBridge` — the bridge factory; `CAPABILITY_GATES` lists gated commands.
- `invoke_handler!` (in `lib.rs`) — the authoritative registry of every command.
- `SCHEMA_SQL` — the full SQLite DDL in `src-tauri/src/storage/mod.rs`.
- `rename = "` — locate the wire name of any backend command.

## Generated, Vendored, and Ignored Paths

- `dist/` — Vite build output; do not edit by hand.
- `src-tauri/target/` — Cargo build output; do not edit.
- `src-tauri/gen/schemas/` — Tauri-generated ACL/capability schemas; regenerated on build.
- `node_modules/` — installed dependencies; skip during guidance/navigation.
- `ref/` — read-only reference tree; gitignored, excluded from the build.
- `.agents/`, `.codex/`, `.claude/` — local agent/platform tooling; inspect only
  for agent workflow changes and do not treat as application source.
- `.trellis/workspace/` — per-developer journals/session traces; prefer Trellis
  scripts for lifecycle edits.
- `.omx/state/`, `.kiro/`, `.vscode/` — local tooling/editor state.

## Verification Command Index

- `just build` — frontend typecheck + production build (run from root).
- `just test` — frontend Vitest suite (run from root).
- `just test-rust` — backend tests (run from root via `src-tauri/Cargo.toml`).
- `just fmt-check` / `just clippy` — backend format/lint gates.
- `just ci` — full local gate for broad or cross-boundary changes.
