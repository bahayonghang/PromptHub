# Backend prompt-reference capability

Child of `08-24-ui-refactor`. Owns parent requirement R12. This is the only
backend change in the tree.

## Goal

Let a prompt body reference another prompt with `@@<title>`, persist the
resulting edges, expose them over the Runtime Bridge, and expand every reference
when the prompt is copied.

## Ordering

Lands before `08-24-detail-modal`, whose references tab consumes this contract.
It has no dependency on the other children and can start in parallel with them.

## Background

- No table, service, or command represents a link between two prompts today.
  `src-tauri/src/storage/mod.rs:451` defines `prompts`; the indexes at
  `:646-650` cover folder, updated, favorite, pinned, and created.
- Schema changes go through the `MIGRATIONS` list
  (`src-tauri/src/storage/mod.rs:55`) keyed on SQLite `user_version` (`:336`).
- `prompt.copy` (`src-tauri/src/commands/prompt.rs:93`) already substitutes
  declared variables. Reference expansion belongs in the same path so both the
  list copy control and the detail overlay inherit it.
- Every command is registered in `invoke_handler!`
  (`src-tauri/src/lib.rs`), named `domain.action`, and returns
  `CommandResult<T>` (`src/code_map.md:32-40`).
- Prompt content may be encrypted at rest (`is_private`) and unavailable when no
  key is cached (`is_locked`) (`src-tauri/src/models/prompt.rs:65-68`).
- Portable bundles export and import prompts
  (`src-tauri/src/services/portable.rs`); edges must survive that round trip or
  be explicitly dropped.

## Requirements

- R1: A prompt body may contain `@@<title>`. Saving a prompt resolves each
  occurrence to a target prompt and persists an edge keyed on prompt ids, not on
  titles. Renaming a target must not break an existing edge.
- R2: An unresolved `@@<title>` is not an error. It is persisted as unresolved
  and reported to the frontend as such, so the references tab can show it.
- R3: Edges are stored in their own table with a new migration. The migration is
  additive and runs cleanly on an existing database.
- R4: The bridge exposes, for one prompt, the outgoing references and the
  incoming references, each with target id, title, and the token that produced
  it.
- R5: `prompt.copy` expands every resolved reference to the referenced prompt's
  body, then substitutes variables. The order of the two steps is fixed and
  documented, because an expanded body may itself contain placeholders.
- R5b (added by `design.md` D4): `PromptCopy` gains `messages` and `unexpanded`,
  and `prompt.copy` becomes the real copy path. Today the command has no caller:
  every copy runs through the frontend `buildPromptCopyText`
  (`promptText.ts:138-157`, called from `CopyPromptButton.tsx:70`), the declared
  mirror type is wrong (`api.ts:38` says `Promise<string>`, the command returns
  `PromptCopy`), and `copy_secure` ignores `prompt.messages` entirely
  (`services/prompt.rs:1108-1127`). Without these three corrections, expansion
  added to `prompt.copy` changes nothing a user can observe, and routing the
  existing controls at the command as it stands would drop chat-mode message
  structure.
- R6: Expansion is depth-limited and cycle-safe. A cycle does not hang, does not
  recurse without bound, and produces a stated, testable result.
- R7: A reference whose target is locked or missing does not silently produce
  empty text. Copy states what could not be expanded.
- R8: Deleting a prompt that others reference does not delete them and does not
  leave a dangling row that breaks a later read.
- R9: New commands follow the existing naming, envelope, and registration
  conventions, and are reachable only through a feature `api.ts` on the
  frontend.
- R10: Error cases map to the existing `ErrorCode` taxonomy
  (`src-tauri/src/error.rs`). No new ad-hoc error string.

## Open design decisions — settled in `design.md`

- D1: Stores the literal token. Re-deriving needs the plaintext body, and a
  private prompt's body is ciphertext in the column.
- D2: An ambiguous title stays unresolved with `resolution = 'ambiguous'`, so
  the references tab can say why instead of reporting "not found" for a title
  that exists. `prompts.title` has no `UNIQUE` constraint.
- D3: Bundles carry edges as an additive `#[serde(default)]` manifest field.
  `FORMAT_VERSION` stays at 2, following the `type_definitions` precedent.
  Rebuilding by re-resolving titles fails under the `duplicate` policy, where
  every title then exists twice.
- D4: Depth limit is three levels of nesting. A cycle, an exceeded depth, a
  missing target, an ambiguous title, and a locked target all leave the token
  literal and report a reason in `unexpanded`. No case refuses the copy.

## Acceptance criteria

Each criterion names the requirement it closes.

- [ ] AC1 (R2): The migration applies to a database created before this change,
      and `user_version` advances by exactly one. A fresh database also has the
      table and reads 6, because `SCHEMA_SQL` and `MIGRATIONS` are separate paths
      (design finding 5).
- [ ] AC2 (R3): Saving a prompt containing `@@A` and `@@Missing` persists one
      resolved edge and one unresolved reference.
- [ ] AC2b (R3, design D8): A **private** prompt with the same body persists the
      same two edges. Resolution runs on the plaintext scan taken before
      encryption, so private prompts do not silently get zero edges.
- [ ] AC3 (R1): Renaming prompt A keeps the edge intact; `reference.list` returns
      A's new title.
- [ ] AC3b (R1, design D6): Copy also follows the rename. After A is renamed, a
      prompt whose body still reads `@@OldTitle` copies with A's body inlined,
      because expansion resolves through the stored edge and not through a fresh
      title lookup.
- [ ] AC3c (R8, design command surface): `reference.list` incoming entries carry
      `sourcePromptId` and `sourceTitle`, naming the prompt that references this
      one. They do not echo the caller's own id.
- [ ] AC4 (R5, R6): `prompt.copy` on a prompt referencing A returns A's body
      inlined, with variables substituted per the documented order, in text mode
      and in chat mode.
- [ ] AC4b (R5, parent A11, CC8): `CopyPromptButton` produces that expanded text.
      For a prompt with no references it produces exactly the text it produced
      before this change.
- [ ] AC5 (R6): A cycle `A -> B -> A` produces the documented result within the
      depth limit, in a test, with no stack overflow and no hang.
- [ ] AC6 (R7): Copying a prompt whose reference target is locked reports the
      unexpanded reference rather than emitting empty text. An unlocked private
      target expands normally.
- [ ] AC7 (R8): Deleting a referenced prompt leaves the referencing prompt
      readable and its reference list coherent.
- [ ] AC7b (R3, design D9): Every write path in the D9 table keeps the edge table
      consistent with the body — create, update, duplicate, rollback, delete,
      batch delete, and import under each of `skip`, `duplicate`, and `replace`.
      One test per row.
- [ ] AC7c (R4, design D9): Importing the same bundle twice does not collide on
      the edge primary key and does not produce duplicate edges.
- [ ] AC8: `just test-rust`, `just fmt-check`, and `just clippy` pass.
- [ ] AC9 (R9): The frontend `api.ts` mirror types match the Rust DTOs, and
      `just build` passes.

## Out of scope

- Any reference UI. The references tab is `08-24-detail-modal`.
- Autocomplete for `@@` inside the body editor.
- Reference-aware search ranking.
