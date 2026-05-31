# Repository Guidelines

This `AGENTS.md` governs the repository root and all descendants unless a deeper
`AGENTS.md` overrides or narrows it. Two nested guides apply:

- `src/AGENTS.md` — React/TypeScript frontend (`src/**`).
- `src-tauri/AGENTS.md` — Rust/Tauri backend (`src-tauri/**`).

Before broad search or repo-wide grep, read `./code_map.md` and use its search
anchors to jump to targeted files.

## What This Repo Is

PromptHub Desktop: a local-first Tauri 2 app for managing AI prompts, prompt
versions, reusable skills (SKILL.md), and platform rules. The Rust backend
(`src-tauri/`) reimplements the original Electron main process; the React
frontend (`src/`) reaches the backend only through the Runtime Bridge
(`src/runtime`). The original Electron app is kept read-only under `ref/PromptHub`.

## Build, Test, and Development Commands

Run from the repo root (frontend tooling shares the root `package.json`):

- `npm install` — install frontend dependencies.
- `npm run tauri dev` — full desktop dev environment (Vite + native window).
- `npm run dev` — frontend only, on `http://localhost:1420`.
- `npm run build` — frontend gate: `tsc` typecheck + Vite production build.
- `npm run test` — frontend Vitest suite (`vitest run`).
- `npm run tauri build` — package installers into `src-tauri/target/release/bundle/`.

Rust backend, run from `src-tauri/`:

- `cargo test` — backend unit + property tests.
- `cargo fmt` and `cargo clippy` — standard Rust formatting/lint gates.

There is no ESLint/Prettier setup; `tsc` (via `npm run build`) is the frontend
static gate.

## Coding Standards

- Frontend never imports `@tauri-apps/api` directly — every backend call and
  event subscription goes through `src/runtime` (the Runtime Bridge).
- Backend commands return the `CommandResult<T>` envelope and use the stable
  `ErrorCode` taxonomy in `src-tauri/src/error.rs`; services stay Tauri-free.
- Code comments and docs in this repo are written in English; match that.
- Many symbols cite spec requirement numbers (e.g. `Req 16.7`) that map to
  `.kiro/specs/tauri-rewrite/`. Keep those references accurate when you touch them.

## Testing and Verification

- Prefer targeted checks while iterating: a single Vitest file with
  `npx vitest run <path>`, or a focused `cargo test <name>` in `src-tauri/`.
- Before claiming completion for broad changes, run `npm run build`,
  `npm run test`, and (for backend changes) `cargo test`.

## Safety and Permissions

- Never edit `ref/PromptHub/**` — it is a read-only reference implementation.
- Do not hand-edit generated/build output: `dist/`, `src-tauri/target/`,
  `src-tauri/gen/schemas/`, or `.omx/state/**`.
- Updater signing keys (`*.key`, `*.pem`) are secrets and are gitignored; never
  commit them. The `pubkey` in `src-tauri/tauri.conf.json` is a placeholder.
- Outbound requests (AI calls, remote skill fetch, media download) must pass the
  backend SSRF policy (`SSRF_BLOCKED`); do not bypass it.
- Ask before destructive or external-production operations: sync/backup deletes,
  data-path changes, schema changes, or anything touching real credentials.

## Notes on Scope

This rewrite intentionally does not migrate old Electron data. CLI and Web
variants are out of scope for this repository.
