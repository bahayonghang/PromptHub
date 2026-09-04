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

## Instruction files

`AGENTS.md` is the single source of truth. Each `CLAUDE.md` only imports the
`AGENTS.md` beside it.

- Root `AGENTS.md` holds the rules for the whole repository.
- `src/AGENTS.md` adds the React/TypeScript rules. `src-tauri/AGENTS.md` adds the
  Rust/Tauri rules.
- Codex loads one instruction file per directory, from the repository root down to
  your working directory. It does not load a file below that directory.
- Claude Code loads the root `CLAUDE.md` at launch. It loads a nested `CLAUDE.md`
  when it reads a file in that directory.
- Read the nested file yourself when you work in `src/` or `src-tauri/` and the
  file is not in context.

## What this repo is

PromptHub Desktop is a local-first Tauri 2 app for AI prompts, prompt versions,
and platform rules. The Rust backend in `src-tauri/` reimplements the original
Electron main process. The React frontend in `src/` reaches the backend only
through the Runtime Bridge (`src/runtime`). `ref/PromptHub` keeps the original
Electron app as a read-only reference.

## Navigation

Read `./code_map.md` and use its search anchors before a repository-wide search.
`src/` and `src-tauri/` each have their own `code_map.md`.

## Verification gates

Run every recipe from the repository root. Run `just --list` for the full recipe
list.

| Change                   | Run before you report the work as done            |
| ------------------------ | ------------------------------------------------- |
| Frontend (`src/**`)      | `just build` and `just test`                      |
| Backend (`src-tauri/**`) | `just fmt-check`, `just clippy`, `just test-rust` |
| Broad or cross-boundary  | `just ci`                                         |

While you iterate, use a narrower command: `npx vitest run <path>` for one
frontend test file, or `cargo test <name> --manifest-path src-tauri/Cargo.toml`
for one backend test.

## Cross-boundary contract

- The frontend never imports `@tauri-apps/api` outside `src/runtime`. Every
  backend call and every event subscription goes through the Runtime Bridge.
- Wire names are `domain.action`. Backend commands return the `CommandResult<T>`
  envelope and use the `ErrorCode` taxonomy in `src-tauri/src/error.rs`. The
  string codes are a stable contract.
- Services under `src-tauri/src/services/` must not depend on Tauri.

## Code conventions

- Write code comments and docs in English.
- Keep the existing `Req N.M` and `Requirement N` comments when you edit nearby
  code. Do not invent a new number. When the source spec is absent, verify the
  behavior against the tests and the implementation.

## Safety and permissions

- Never edit `ref/PromptHub/**`. It is a read-only reference implementation.
- Never hand-edit generated output: `dist/`, `src-tauri/target/`, or
  `src-tauri/gen/schemas/`.
- Treat `.trellis/workspace/**` as session and journal state. Use the Trellis
  scripts for task lifecycle changes.
- Outbound requests (AI calls, media download, sync) must pass the backend SSRF
  policy (`SSRF_BLOCKED`). Do not weaken it.
- Updater signing keys (`*.key`, `*.pem`) are secrets and are gitignored. Never
  commit them. The `pubkey` in `src-tauri/tauri.conf.json` is a placeholder.
- Ask before a destructive or production operation: a sync or backup delete, a
  data-path change, a schema change, updater signing, or work with real
  credentials.

## Scope

This rewrite does not migrate old Electron data. CLI and Web variants are out of
scope for this repository.

## Issue tracker

Issues live in GitHub Issues on `bahayonghang/PromptHub`. Use the `gh` CLI.
