# Prompt Management Product Research

Accessed: 2026-07-14

## Method

- Audited reachable React, Runtime Bridge, Tauri command/service, SQLite,
  settings, localization, and test surfaces from the repository code maps.
- Searched the GitHub `prompt-management` topic to discover active projects.
- Used primary project documentation for detailed capability claims.
- Treated locale-only strings and reference-app behavior as unimplemented unless
  the live React/Rust code exposes the workflow.

## Current Product Baseline

### Confirmed strengths

| Capability | Evidence |
| --- | --- |
| Prompt CRUD and typed DTOs | `src/features/prompts/api.ts:20`, `src/features/prompts/types.ts:37`, `src-tauri/src/commands/prompt.rs:9` |
| Hierarchical folders | `src/features/prompts/folderTree.ts:21`, `src-tauri/src/services/folder.rs:87` |
| FTS search, tag/folder/favorite filters, sorting | `src-tauri/src/services/prompt.rs:373`, `src/features/prompts/components/SearchBar.tsx:24` |
| Typed variables and copy-time substitution | `src/features/prompts/types.ts:17`, `src/features/prompts/promptText.ts`, `src-tauri/src/services/prompt.rs:542` |
| Text/image/video prompt records and media references | `src/features/prompts/types.ts:10`, `src/features/prompts/components/MediaRefList.tsx:14` |
| Manual snapshots with note, rollback, delete | `src/features/prompts/components/VersionHistory.tsx:17`, `src-tauri/src/services/version.rs:102` |
| Whole-data backup/restore and selective ZIP export | `src/features/settings/api.ts:59`, `src-tauri/src/services/sync.rs:568` |

### Verified gaps and defects

| Priority | Finding | Evidence and impact |
| --- | --- | --- |
| P0 | Libraries silently stop at 50 prompts | Backend defaults to `limit=50` (`src-tauri/src/services/prompt.rs:397`); the frontend never sends `limit`/`offset` or exposes paging (`src/features/prompts/promptStore.ts:58`). Prompts beyond the first page are unreachable. |
| P0 | The master-password UI does not protect prompt content | `src-tauri/src/services/security.rs:143` explicitly says encrypted prompt fields are not wired. Local-first privacy is therefore weaker than the product surface implies. |
| P0 | No prompt-level portable import/export | Settings exposes ZIP export and backup restore only (`src/features/settings/api.ts:59`). Import/share/clipboard locale strings have no command or React implementation. A SQLite snapshot is recovery, not a durable human-readable prompt format. |
| P1 | Versions are partial and manual | `src-tauri/src/services/version.rs:26` snapshots only system prompt, user prompt, and variables. Title, type, description, tags, folder, media, source, notes, and model settings are not versioned. The UI has no diff. |
| P1 | No actual prompt test/evaluation loop | The backend registers generic `ai.request`/`ai.stream`, but no frontend production call exists. There are no datasets, evaluators, run records, cost/latency metrics, or version-linked outputs. `last_ai_response` is only a single mutable field. |
| P1 | Prompt structure is fixed to system + user text | `src/features/prompts/types.ts:37` cannot represent ordered chat messages, assistant examples, tool messages, or message placeholders. |
| P1 | Several stored capabilities are unreachable | `isPinned` exists only in types/storage; tag rename/delete are backend-only; duplicate, batch actions, version compare, quick add, and AI rewrite exist only as stale locale copy. |
| P1 | Backup compatibility lacks a schema migration framework | The schema is idempotent `CREATE TABLE IF NOT EXISTS`; no `user_version`, migration table, or ordered migration runner exists. Cross-version data evolution is unsafe. |
| P2 | No prompt composition or reusable references | Repeated instruction fragments must be duplicated, which increases drift. This is prompt-domain reuse, not Skill management. |
| P2 | No candidate/baseline labels or experiment variants | History is linear and numeric only; users cannot mark a tested baseline or compare an experimental branch without overwriting the current draft. |

## Skill Removal Blast Radius

- Frontend domain: `src/features/skills/**`, wrapper view, `AppView`, navigation,
  shell registry, capability gates, settings export scope, runtime-path UI, tests.
- Backend domain: 28 `skill.*` commands in `src-tauri/src/commands/skill.rs`,
  registrations at `src-tauri/src/lib.rs:121`, five Skill services, two models,
  row mappings, property tests, and schema tables at
  `src-tauri/src/storage/mod.rs:212`.
- Runtime/data: Skill directory creation, runtime-path reporting, selective
  export category, and old backup contents.
- Shared dependency: `src-tauri/src/services/media.rs:56` imports public-network
  guards from `skill_safety`; those guards must move to a neutral SSRF module
  before `skill_safety` is removed.
- Localization: Skill-specific navigation, settings, system paths, CLI/product
  claims, prototype copy, and the full `skillsView` namespace across 7 locales.

## External Project Matrix

| Project/source | Observed practices | Relevance to PromptHub |
| --- | --- | --- |
| [GitHub prompt-management topic](https://github.com/topics/prompt-management) | 253 public repositories; prominent active projects include Langfuse, Helicone, CozeLoop, Agenta, and Pezzo. | Confirms that prompt management is usually coupled to evaluation and observability, but the full hosted LLMOps scope is larger than this desktop product needs. |
| [Langfuse Prompt Management](https://langfuse.com/docs/prompt-management/overview) and [core concepts](https://langfuse.com/docs/prompt-management/data-model) | Prompt objects include instructions plus config; text/chat types, variables, prompt references, message placeholders, immutable versions, movable labels, cache, deployment workflow. | Adopt structured prompts, complete immutable versions, reusable prompt references, and stable labels. Do not copy hosted deployment infrastructure. |
| [Langfuse trace linkage](https://langfuse.com/docs/prompt-management/features/link-to-traces) | Links each generation to an exact prompt version so metrics and evaluations are attributable. | Store local run records keyed to prompt revision, model/profile, input, output, timing, token/cost metadata, and evaluation result. |
| [LangSmith manage prompts](https://docs.langchain.com/langsmith/manage-prompts) | Two-pane commit history, diff, staging/production pointers, tag rollback history, owners, webhooks, public hub. | Adopt diff and movable baseline/candidate labels. Defer owners, webhooks, public hub, and hosted environments. |
| [Agenta prompt concepts](https://agenta.ai/docs/prompt-engineering/concepts) | Git-like variants, immutable versions, environment history, commit notes, compare/evaluate prompt variants. | Use lightweight experiment variants and mandatory change notes only after the linear revision model is made complete. |
| [Agenta evaluation concepts](https://agenta.ai/docs/evaluation/concepts) | Test sets contain inputs, optional ground truth, and annotations; evaluators return boolean, numeric, or structured scores; automated and human evaluation workflows. | Direct model for local test cases, reusable evaluators, manual review, and regression detection. |
| [Promptfoo](https://www.promptfoo.dev/docs/intro/) | Local/private, declarative test cases, assertions/metrics, provider matrix, caching, side-by-side outputs, CI support. | Adopt the test-driven loop, local execution, provider matrix, assertions, and result cache. CI integration is out of scope for the desktop MVP. |

## Best-Practice Synthesis

1. Store complete immutable revisions, not partial manual snapshots.
2. Separate draft editing from named tested states such as `baseline` and
   `candidate`; make rollback a pointer move or explicit new revision.
3. Evaluate prompt revisions against saved test cases before calling one better.
4. Link every run to exact prompt revision, model/profile, rendered variables,
   parameters, output, timing, and evaluation result.
5. Represent chat prompts as ordered role/content messages; keep simple text as a
   first-class ergonomic mode.
6. Keep model/provider configuration separate from prompt content but allow a
   revision to pin the test profile used for reproducibility.
7. Offer a documented, versioned, human-readable import/export format in
   addition to binary backup/restore.
8. Encrypt private prompt content and secrets; locking must actually prevent
   plaintext retrieval and FTS indexing of protected fields.
9. Prove large-library behavior with pagination/count contracts and end-to-end
   tests above the default page size.
10. Prefer local execution and explicit outbound calls; do not add hosted
    observability, team governance, or marketplace complexity without a new
    product decision.

## Recommended Priority

1. Retire Skill management without deleting legacy data.
2. Fix prompt reachability, migrations, complete revisions/diff, portability,
   and real private-prompt protection.
3. Add structured prompt execution, run history, test sets, evaluators, and
   side-by-side comparison.
4. Later consider prompt references, experiment variants, and baseline labels.
