<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->

# Repository Guidelines

This `AGENTS.md` governs the repository root and all descendants unless a deeper
`AGENTS.md` overrides or narrows it. Two nested guides apply:

- `src/AGENTS.md` - React/TypeScript frontend (`src/**`).
- `src-tauri/AGENTS.md` - Rust/Tauri backend (`src-tauri/**`).

Before broad search or repo-wide grep, read `./code_map.md` and use its search
anchors to jump to targeted files.

## What This Repo Is

PromptHub Desktop: a local-first Tauri 2 app for managing AI prompts, prompt
versions, and platform rules. The Rust backend
(`src-tauri/`) reimplements the original Electron main process; the React
frontend (`src/`) reaches the backend only through the Runtime Bridge
(`src/runtime`). The original Electron app is kept read-only under `ref/PromptHub`.

## Build, Test, and Development Commands

Prefer the checked-in `justfile` from the repo root:

- `just deps` - install frontend dependencies.
- `just install` - build and install PromptHub on this machine.
- `just dev` - full desktop dev environment (Vite + native window + Rust backend).
- `just frontend` - frontend-only Vite dev server on `http://localhost:1420`.
- `just build` - frontend gate: `tsc` typecheck + Vite production build.
- `just test` - frontend Vitest suite.
- `just test-rust` - backend unit + property tests.
- `just fmt-check` / `just clippy` - backend format and lint gates.
- `just ci` - full local gate: frontend build/tests, Rust fmt/clippy/tests.

Raw equivalents live in `package.json` and `src-tauri/Cargo.toml`; use them only
when a narrower command is needed while iterating.

## Coding Standards

- Frontend never imports `@tauri-apps/api` directly. Every backend call and event
  subscription goes through `src/runtime` (the Runtime Bridge).
- Backend commands return the `CommandResult<T>` envelope and use the stable
  `ErrorCode` taxonomy in `src-tauri/src/error.rs`; services stay Tauri-free.
- Code comments and docs in this repo are written in English; match that.
- Existing `Req N.M` / `Requirement N` comments are historical contract markers.
  Preserve them when editing nearby code; when the source spec is absent, verify
  behavior against tests and implementation instead of inventing new numbers.

## Testing and Verification

- Prefer targeted checks while iterating: `npx vitest run <path>` for one
  frontend test file, or `cargo test <name> --manifest-path src-tauri/Cargo.toml`
  for a focused backend run.
- Run `just build` and `just test` before claiming a frontend change is done.
- Run `just fmt-check`, `just clippy`, and `just test-rust` before claiming a
  backend change is done.
- Run `just ci` for broad or cross-boundary changes.

## Safety and Permissions

- Never edit `ref/PromptHub/**`; it is a read-only reference implementation.
- Do not hand-edit generated/build output: `dist/`, `src-tauri/target/`,
  `src-tauri/gen/schemas/`, or `.omx/state/**`.
- Treat `.trellis/workspace/**` as session/journal state. Prefer Trellis scripts
  for task lifecycle changes instead of manual edits.
- Updater signing keys (`*.key`, `*.pem`) are secrets and are gitignored; never
  commit them. The `pubkey` in `src-tauri/tauri.conf.json` is a placeholder.
- Outbound requests (AI calls, media download, sync) must
  pass the backend SSRF policy (`SSRF_BLOCKED`); do not bypass it.
- Ask before destructive or external-production operations: sync/backup deletes,
  data-path changes, schema changes, updater signing, or anything touching real
  credentials.

## Notes on Scope

This rewrite intentionally does not migrate old Electron data. CLI and Web
variants are out of scope for this repository.
