# Implement — backend prompt-reference capability

Execution plan for the decisions in `design.md`. Steps are ordered; each gate
passes before the next step starts.

This is the only child in the tree that changes `src-tauri/`. It also corrects
one frontend contract type and splits one frontend helper.

No dependency on the other children. It can start in parallel with them and must
land before `08-24-detail-modal`.

## Step 0 — Baseline

- [ ] `just test-rust`, `just fmt-check`, `just clippy`, and `just build` pass
      before any edit. Report a pre-existing failure instead of absorbing it.
- [ ] Copy a database file from an existing install, or create one on the current
      build, and keep it. Step 1's gate needs a real v5 database, not a fresh one.

Gate: the v5 database file exists at a recorded path.

## Step 1 — Schema

File: `src-tauri/src/storage/mod.rs`

- [ ] Add the `prompt_references` table and its two indexes to `SCHEMA_SQL`
      (design D1). Place the table near `prompts` and the indexes in the index
      block at `:646-664`.
- [ ] Add `Migration { version: 6, sql: ... }` with the same DDL to `MIGRATIONS`.
- [ ] Change `CURRENT_SCHEMA_VERSION` from 5 to 6 (`:48`).

All three edits are required. A fresh database runs `SCHEMA_SQL` and never runs
the migration list (`:329-341`); an existing database runs only the migrations.
Doing one without the other leaves half the installs without the table.

Gate: the Step 0 database opens, `PRAGMA user_version` reads exactly 6, and the
table exists. A fresh database also has the table and reads 6 (PRD AC1).

## Step 2 — Token extraction

New file: `src-tauri/src/services/reference.rs`

- [ ] `extract_tokens(body: &str) -> Vec<String>` implementing design D7:
      `@@Title@@` scanned first, then `@@Title` to end of line.
- [ ] An empty title, or `@@` at end of input, yields no token and no error.
- [ ] Duplicate tokens in one body collapse to one edge per distinct title.
- [ ] Pure function, no `Connection`. Follow `substitute_placeholders`
      (`services/prompt.rs:1050-1085`) as the precedent for a scanner in this
      codebase.

Gate: unit tests cover both forms, a line holding both, a malformed token, an
empty title, and a title containing `{{` so the two syntaxes do not interfere.

## Step 3 — Resolution and persistence

File: `src-tauri/src/services/reference.rs`

- [ ] `resolve_and_store(tx, prompt_id, scan)`: extract tokens from the scan,
      count matching `prompts.title` rows, write one row per distinct token with
      `resolved` / `missing` / `ambiguous` (design D2).
- [ ] Replace, do not append: delete this prompt's rows before inserting.
- [ ] The scan covers the system prompt, the user prompt, and every message body.

### Step 3a — The write seam (design D8)

Do this before wiring any caller. Calling `resolve_and_store` from `create` and
from `create_secure` both cannot work: `create_secure` encrypts before calling
`create` (`services/prompt.rs:559-568`), so `create` sees ciphertext for a
private prompt; and `create_secure` has neither the generated id (`:244`) nor
the transaction (`:252`).

- [ ] Add `ReferenceScan` with `from_create(&PromptCreate)` and
      `from_update(&PromptUpdate, &Prompt)`. The update form merges the patch over
      the existing row (`:339-369`), so an omitted `user_prompt` still scans the
      stored body.
- [ ] Add `create_inner(conn, input, scan)` and `update_inner(conn, id, patch, scan)`
      holding the current bodies of `create` and `update`, plus the
      `resolve_and_store` call inside the existing transaction.
- [ ] `create` / `update` become one-line wrappers that build the scan from their
      own input. Their public signatures do not change.
- [ ] `create_secure` / `update_secure` build the scan from the plaintext
      **before** encrypting, then call `*_inner` with the encrypted input.

Do not add a command that writes an edge directly. An edge must correspond to a
token in a body (design, command surface).

Gate: saving a prompt containing `@@A` and `@@Missing` writes one `resolved` and
one `missing` row (PRD AC2). Saving it again does not duplicate them. A **private**
prompt with the same body writes the same two rows — the regression this step
exists to prevent is private prompts silently getting zero edges (PRD AC2b).

## Step 4 — Every other write path (design D9)

Files: `src-tauri/src/services/prompt.rs`, `src-tauri/src/services/version.rs`

`create` and `update` are not the only paths that change a body. Work the D9
table row by row; each row is a separate checkbox because each is a separate
place the edge table can go stale.

- [ ] `delete` (`:701-711`): wrap in a transaction — it runs a bare
      `conn.execute` today — and run
      `UPDATE prompt_references SET resolution = 'missing', target_prompt_id = NULL
WHERE target_prompt_id = ?1` before the `DELETE FROM prompts` (design D5).
- [ ] `batch_delete` (`:839-850`): the same statement per id, inside its existing
      transaction.
- [ ] `duplicate` (`:712-758`): resolve the copy's body under the new id inside
      the existing transaction. Resolve, do not copy the source's rows — the
      duplicate may itself change an `ambiguous` count.
- [ ] `rollback` (`version.rs:243-290`): delete that prompt's rows and re-resolve
      from the restored body, inside the existing rollback transaction.
- [ ] Confirm `batch_move` and `batch_tag` (`:771-837`) need no edge work; they
      change no body.
- [ ] Confirm the outgoing edges of a deleted prompt disappear through
      `ON DELETE CASCADE` rather than needing a second statement.

Gate: a test per row. Deleting a referenced prompt leaves the referencing prompt
readable with the edge `missing` (PRD AC7); duplicating a prompt with `@@A`
gives the copy its own resolved edge; rolling back to a version whose body had a
different token replaces the edges (PRD AC7b).

## Step 5 — Expansion

File: `src-tauri/src/services/reference.rs`

- [ ] `expand(conn, encryption, source_prompt_id, body, ancestors, depth)
-> Result<(String, Vec<UnexpandedReference>), AppError>` per design D6.
      Both `encryption` and `source_prompt_id` are required; a signature without
      them cannot deliver the rename case or the locked-target case.
- [ ] Resolve each token through **this source's** `prompt_references` row,
      matched on `token_title`. Do not re-query `prompts.title`. After a target is
      renamed the body still holds the old title, and only the stored edge knows
      where it points (PRD R1, AC3b).
- [ ] Recurse with the target's id as the next `source_prompt_id`, so each level
      reads its own edges.
- [ ] Decrypt a private target through the existing `unlocked_key`
      (`services/prompt.rs:569`) and `decrypt_*` helpers. No second decryption
      path. With no key, report `reason: locked` and do not fail.
- [ ] Name the depth limit as a constant. Three levels of nesting.
- [ ] Cycle detection uses the ancestor path, not a global visited set. The same
      target inlined in two branches expands in both.
- [ ] Every failure leaves `@@Title` literally in the text and appends an entry
      with its reason: `missing`, `ambiguous`, `locked`, `depth`, `cycle`.
- [ ] A locked target yields `locked` and does not fail the call. A locked source
      still returns `UNAUTHORIZED` from `copy_secure`
      (`services/prompt.rs:1115-1119`), unchanged.

Gate: tests cover a two-level chain, a four-level chain hitting the limit, a
cycle `A → B → A` with no hang and no stack overflow, a diamond where one target
appears twice and expands twice, a locked target, an unlocked private target that
does expand, and a **renamed** target whose body token is stale but whose edge
still resolves (PRD AC5, AC6, AC3b).

## Step 6 — Rework `prompt.copy`

Files: `src-tauri/src/services/prompt.rs`, `src-tauri/src/commands/prompt.rs`

- [ ] Extend `PromptCopy` with `messages: Vec<PromptMessage>` and
      `unexpanded: Vec<UnexpandedReference>` (design D4).
- [ ] `copy_secure` expands first, then substitutes, in that order, once over the
      assembled text. Document the order in the doc comment, including that an
      expanded body's placeholders take the calling prompt's values.
- [ ] Apply the same expansion and substitution to `messages`, which the function
      ignores today.
- [ ] Keep the existing locked-source behavior.

Gate: `prompt.copy` on a prompt referencing A returns A's body inlined with
variables substituted in the documented order (PRD AC4), and a chat-mode prompt
returns its messages.

## Step 7 — `reference.list`

Files: `src-tauri/src/commands/prompt.rs` (or a new `commands/reference.rs`),
`src-tauri/src/lib.rs`

- [ ] `reference.list` taking `promptId`, returning `{ outgoing, incoming }`.
- [ ] Two shapes, not one (design, command surface). Outgoing:
      `targetPromptId` (nullable), `targetTitle` (nullable), `tokenTitle`,
      `resolution`. Incoming: `sourcePromptId`, `sourceTitle`, `tokenTitle`,
      `resolution`. An incoming entry carrying `targetPromptId` would return the
      caller's own id and leave the tab unable to name what references it.
- [ ] `incoming` filters `resolution = 'resolved'`. An unresolved edge points at
      no target, so it belongs only to its own source's `outgoing`.
- [ ] Both titles are read live by join, so a rename is reflected without
      touching the edge (PRD R1, AC3).
- [ ] Register in `invoke_handler!` next to the other prompt commands.
- [ ] `CommandResult<T>` envelope, `domain.action` name.

Gate: renaming prompt A keeps the edge and `reference.list` returns A's new
title (PRD AC3).

## Step 8 — Bundles

File: `src-tauri/src/services/portable.rs`

- [ ] Add `#[serde(default)] pub references: Vec<PromptReferenceRecord>` to
      `PromptBundleManifest` (design D3). Do not change `FORMAT_VERSION`.
- [ ] Export the edges of the exported prompts.
- [ ] On import, remap `source_prompt_id` and `target_prompt_id` through the
      existing `prompt_map` (`:858-870`).
- [ ] Generate the edge `id` locally on every insert. `PromptReferenceRecord`
      carries no `id` field. Carrying one would collide on the primary key when
      the same bundle is imported twice under `Skip` or `Replace` (design D9).
- [ ] Per policy, per the D9 table: `Skip` drops the bundle's edges for that
      source and leaves the resident prompt's rows untouched; `Replace` deletes
      that source's rows and inserts the remapped ones; `Duplicate` inserts the
      remapped edges among the duplicated copies.
- [ ] Re-resolve any imported edge whose target id is absent after insert, so it
      lands as `resolved` or `missing` rather than dangling.

Do not bump `FORMAT_VERSION`. The compatibility check at `:345` would reject
every bundle this build exports on older builds, for a field they ignore
(design D3, finding 8).

Gate: edges survive export and import under `skip`, `duplicate`, and `replace`,
and a bundle written before this change imports cleanly.

## Step 9 — Frontend contract

Files: `src/features/prompts/api.ts`, `src/features/prompts/types.ts`,
`src/features/prompts/promptText.ts`

- [ ] Add `PromptCopyResult` and `UnexpandedReference` mirror types matching the
      Rust DTOs.
- [ ] Fix `copyPrompt`'s declared return type. It says `Promise<string>`
      (`api.ts:38`) and the command returns `PromptCopy`.
- [ ] Add `listReferences(promptId)` calling `reference.list`.
- [ ] Split `buildPromptCopyText`: keep the `[System]` / `[User]` /
      `[Assistant]` block format and the chat-mode branch
      (`promptText.ts:138-157`); take the already-substituted parts as input
      instead of substituting. `defaultVariableValues` moves to the call site and
      is passed as the command's `values`.

Gate: `just build` passes and `api.test.ts` asserts the corrected mirror type
(PRD AC9).

## Step 9b — Switch the copy control to `prompt.copy`

File: `src/features/prompts/components/CopyPromptButton.tsx`

This task owns the switch (design D4, parent assumption A11). Without it, R12 is
backend code that no copy button reaches: nothing in `src/` calls `prompt.copy`
today, and `08-24-library-views` design D5 does not migrate the control.

- [ ] Replace `buildPromptCopyText(source)` (`:70`) with
      `api.copyPrompt(id, defaultVariableValues(prompt))`, then format the
      returned parts with the Step 9 formatter.
- [ ] Keep the output byte for byte for a prompt with no references: the
      `[System]` / `[User]` / `[Assistant]` blocks, the chat-mode branch, and the
      declared-default substitution the archived `08-24-prompt-list-copy` R8
      contract specifies.
- [ ] Keep the in-control success and failure feedback and the locked disabled
      state (`:33,55,86`) unchanged. Do not route copy through a toast
      (parent R14).
- [ ] Do not surface `unexpanded` here. The references tab reports it
      (`08-24-detail-modal`); a warning on every copy would be noise (design D4).
- [ ] The control already awaits the clipboard write, so one more awaited call
      does not change its shape.

Gate: `CopyPromptButton.test.tsx` asserts, against a fake api, that a prompt with
no references copies exactly the text it copied before this change, and that a
prompt whose body holds `@@Title` copies with the target's body inlined — in text
mode and in chat mode (PRD AC4b, parent CC8).

## Step 10 — Document the token syntax

- [ ] Write the `@@` syntax from design D7 into `.trellis/spec/`, so the
      references-tab picker in `08-24-detail-modal` inserts the same form.
- [ ] Note in the same place that a prompt saved before this change has no edges
      until it is next saved, and that no backfill runs — a private prompt's body
      is ciphertext, so a backfill would silently cover public prompts only.

Gate: the syntax document exists and the two forms match the implementation.

## Step 11 — Full check

- [ ] `just test-rust`
- [ ] `just fmt-check`
- [ ] `just clippy`
- [ ] `just build`
- [ ] `just ci`
- [ ] Re-open the Step 0 database and confirm it still reads and searches.

## Review gates

| After step | Gate                                                             |
| ---------- | ---------------------------------------------------------------- |
| 1          | Both fresh and migrated databases have the table and read 6      |
| 3          | Edges are replaced per save, never appended                      |
| 3a         | A **private** prompt gets the same edges as a public one         |
| 4          | Every D9 row has a test; no write path leaves the table stale    |
| 5          | Depth, cycle, locked target, and a renamed target all hold       |
| 6          | Expansion order documented; chat-mode messages no longer ignored |
| 7          | Incoming entries name the source, not the caller's own id        |
| 8          | Bundle format version unchanged; edges round-trip; no id carried |
| 9          | The mirror type matches the DTO                                  |
| 9b         | Copy output unchanged with no references, expanded with one      |
| 11         | `just ci` green; the pre-existing database still works           |

## Rollback points

- Step 1 is the one edit that cannot be reverted cleanly. A schema version does
  not go backwards, so a revert keeps migration 6 and the table, and removes only
  the code that reads it.
- Step 2 is a pure function with no caller.
- Step 3a refactors `create` and `update` without changing their behavior. It is
  the one step to revert first if a regression appears in prompt saving, and it
  is verified by the existing prompt tests passing unchanged.
- Steps 3 to 5 are additive service code with no caller until Step 6.
- Step 6 changes an existing DTO. It is safe to revert **before** Step 9b,
  because no shipped code calls `prompt.copy` (design finding 1). After Step 9b
  the two revert together.
- Step 8 is independent; reverting it drops edges from bundles without affecting
  the live database.
- Step 9b is the only user-visible change in this task. Reverting it restores
  `buildPromptCopyText` and leaves the backend capability unobservable.

## Open items carried out of this task

- `unexpanded` is returned by `prompt.copy` and nothing renders it yet. The
  references tab in `08-24-detail-modal` is where a user learns which references
  did not expand. Copy stays silent about it by design (D4).
- Prompts saved before this change have no edges until their next save. No
  backfill runs (design, compatibility).
- The `prompt_references` table is not encrypted. It records that a prompt
  references a title. Titles are already plaintext, so this adds a relationship
  to metadata that was already readable, and no body text.
- The editor already has a section keyed
  `promptsView.editor.sections.references` (`PromptEditor.tsx:1079-1087`) holding
  images, videos, source, and notes. That is media provenance, not
  prompt-to-prompt references. Two concepts now share the word "references" in
  this feature. `08-24-detail-modal` decides which one gets renamed.
