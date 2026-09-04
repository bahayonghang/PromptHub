# Coverage gap inventory (2026-09-03)

Inventory of existing tests versus core-business contracts. Counts are
unit tests plus crate integration tests. They are not line-coverage
percentages. Command adapters that only wrap a service call are listed
as out of scope unless they own a permission or validation gate.

## Current suite shape

| Area | Existing tests | Risk if a gap fails |
|------|----------------|---------------------|
| `services/security.rs` | 14 unit + 4 property | Confidentiality / data loss |
| `services/prompt.rs` | 45 unit + search/version properties | Library integrity |
| `services/portable.rs` | 10 unit | Import corrupts or escapes |
| `services/evaluation.rs` | 10 unit (happy matrix, cache, credentials) | Cost, labels, cancel, lock |
| `services/reference.rs` | 8 unit | Copy/expand correctness |
| `services/version.rs` | 13 unit + properties | History rewrite |
| `services/folder.rs` | 20 unit + 6 property | Tree integrity |
| `storage/` | schema, FTS, round-trip, atomicity, v4→current | Migration data loss |
| `network_safety.rs` | 6 unit (IP/hostname, local URL precheck) | SSRF |
| Frontend stores/API | prompts/settings/system/runtime covered; evaluation store has load+cancel only | Error surfacing |

Folder, media, settings, window, updater, data-path, rules, and AI
stream decoding already cover their validation and rollback contracts.
Do not add tests there unless a new gap appears while writing the
priority modules.

## High-risk gaps (in scope)

### 1. Security permission gate

- `services/security.rs:113-120` documents that `set_master_password` is
  a primitive and that the Command_Layer gates "no master password
  exists" (Req 15.2).
- `commands/security.rs:12-20` calls the primitive with no gate.
- UI hides the form after a password exists
  (`SecurityPanel.tsx` `!hasMasterPassword`), but the wire command
  remains callable.
- A second `security.setMasterPassword` would replace the verifier
  without `change_master_password` re-keying `ENC::` rows.

Needed tests:

- Unlock / change password with no verifier stored → `UNAUTHORIZED`,
  lock state unchanged.
- Second `set_master_password` after a verifier exists must not replace
  the verifier or leave existing ciphertext undecryptable. If the
  command currently allows that, apply the smallest Command_Layer gate
  (`CONFLICT` or equivalent) so the test matches Req 15.2.

### 2. Private content while locked

Covered: locked get redacts, locked copy returns `UNAUTHORIZED`, re-key
preserves prompt ciphertext (`prompt.rs`
`private_prompt_is_encrypted_redacted_unsearchable_and_rekeyed`).

Missing:

- `create_secure` with `is_private=true` while locked →
  `UNAUTHORIZED`, no row.
- `update_secure` public→private or private content edit while locked →
  `UNAUTHORIZED`, row unchanged.
- Locked private prompt used as an evaluation revision / credential
  profile → `UNAUTHORIZED`, no run row.

### 3. Evaluation contracts from `prompt-evaluation-loop.md`

Covered: render variable requirement, evaluator evidence/skips, SSE
line decode, credential encrypt/redact/re-key, matrix cache, preflight
cell error, 20-case ordering, retry, native mock restart, opt-in live
smoke.

Missing:

- `move_label` / `label_history`: invalid label/action, revision that
  does not belong to the prompt (`NOT_FOUND`), unevaluated revision
  (`VALIDATION`), successful move/rollback appends history and does not
  rewrite prompt history (`evaluation.rs:1384-1450`).
- `set_manual_result`: unknown cell, non-manual evaluator, skipped
  until reviewed.
- `execute_run` / `run_matrix` cancel: remaining cells and the parent
  run persist as `cancelled`, not `error` (`evaluation.rs:914-924`,
  `1149-1163`).
- `create_profile` with a credential while locked → `UNAUTHORIZED`.
- `validate_profile` / `save_test_set` / `create_evaluator` /
  `import_test_set` / `run_matrix` empty-input validation: empty name,
  >1000 cases, empty case name, non-object annotations, unsupported
  kind, invalid regex, empty matrix ids, empty test set. Failure must
  write nothing.
- `execute_openai` / `prepare_public_url` with a loopback or
  `localhost` endpoint → `SSRF_BLOCKED` before a request. Redirect
  re-check already lives in `network_safety`; wire it through the
  provider adapter with a blocked URL, not a live public host.

### 4. Prompt parameter validation

Covered: empty title, empty user prompt with no messages, invalid
`promptType`, missing/mismatched type definition, batch missing-id
atomicity.

Missing:

- `validate_messages`: role outside `system|user|assistant`, empty
  content → `VALIDATION`, no write (`prompt.rs:195-208`).
- Update that clears both `user_prompt` and `messages` → `VALIDATION`,
  row unchanged (`prompt.rs:381-385`).
- Chat-only create (empty `user_prompt`, non-empty valid messages)
  remains allowed (already used by evaluation setup; add an explicit
  assertion).

### 5. Portable bundle validation

Covered: Unicode/media/preview, traversal, missing folder/media, private
key mismatch, media-byte conflict rollback, nested folders, custom-type
remap/conflict, skip/duplicate/replace for references.

Missing:

- Format version `0` or `3` → `VALIDATION`, no backup
  (`portable.rs:350-354`). Spec requires v1 compatibility; there is no
  import of a `formatVersion: 1` ZIP.
- Folder cyclic parent chain → `VALIDATION` before backup
  (`portable.rs:402-419`).
- Private field stored as plaintext in a private bundle → `VALIDATION`.

### 6. Frontend error channels for the same contracts

Covered: settings store master-password validation/unlock errors,
prompt store not-found, evaluation store load + cancel happy path.

Missing:

- `evaluationStore`: `render`/`run`/`saveProfile`/`moveLabel` catch
  `BridgeError` and set `error` without mutating collections.
- `promptStore`: create/update/copy `UNAUTHORIZED` for locked private
  content.

## Explicitly out of scope

- Line-coverage tooling (`llvm-cov`, Istanbul) and coverage-percentage
  targets.
- Tests whose deletion would still pass (tautological wrappers,
  getters, already-covered serde round-trips).
- Window chrome, updater install, media format matrix, appearance,
  i18n key parity, Runtime Bridge import boundary (already tested).
- Live OpenAI, native Tauri smoke, `ref/PromptHub/**`.
- Command adapters that only call `into_command(service(...))`.
- Product behavior changes other than the Req 15.2 set-password gate
  if the new test proves the command overwrites a verifier.

## Verification commands

```text
cargo test --manifest-path src-tauri/Cargo.toml <filter>
npx vitest run <path>
just test-rust
just test
just ci
```
