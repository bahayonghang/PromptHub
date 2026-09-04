# Design: core-business test coverage

## Architecture and boundaries

Add tests at the service layer. Services already accept a rusqlite
`Connection` or `DbPool`, an `EncryptionState` mutex, and injected
`ProviderAdapter` / `EvaluationEventSink` / `CancellationToken`. New
tests reuse those seams.

Command adapters stay untested except `security.setMasterPassword`,
which is the only adapter that currently omits a documented permission
gate. Frontend tests stay in colocated store files and inject fake
`api` objects, matching `settingsStore.test.ts`.

```text
test  →  service(conn, encryption, adapter)
           ├─ SQLite memory pool + init_schema
           ├─ EncryptionState mutex
           └─ fake ProviderAdapter / pre-cancelled token

R1.2 only:
test  →  commands/security.rs gate  →  security::set_master_password
```

## Data flow and contracts

Assertions use `AppError.code` / `code_str()`, not message text, except
when a message is the published store error channel.

On every negative case:

1. Capture a before snapshot (row count, ciphertext, lock flag, backup
   directory existence).
2. Call the operation.
3. Assert the `ErrorCode`.
4. Assert the snapshot is unchanged.

Positive cancel/label cases assert the persisted status/history, not
only the returned DTO.

## Module design

### Security (R1)

- Extend `services/security.rs` tests for unlock/change with no
  verifier.
- Add a Command_Layer test or a focused service-plus-command unit that
  proves a second set is rejected. Preferred implementation if the
  current command overwrites: in `commands/security.rs`, after
  `conn()`, read `security::status`; if `has_master_password`, return
  `AppError::conflict(...)`. Do not change `set_master_password` itself;
  `change_master_password` still uses it as a primitive after re-key.
- Extend `services/prompt.rs` tests for `create_secure` /
  `update_secure` while locked.

### Evaluation (R2)

- Add tests in `services/evaluation.rs` next to the existing mock
  adapter helpers (`setup`, `Sink`, `DefaultProviderAdapter`).
- Cancel: start `execute_run` / `run_matrix` with a token that is
  already cancelled or cancelled after the first cell. Assert SQLite
  `status='cancelled'`.
- Labels: seed one successful evaluation cell, then move/rollback;
  seed an unevaluated revision and assert `VALIDATION`.
- SSRF: call `execute_run` with an openai-compatible profile whose
  endpoint is `http://127.0.0.1/` or `http://localhost/`. No network
  listener required; `prepare_public_url` fails first.
- Validation: call `save_test_set` / `create_evaluator` /
  `create_profile` / `run_matrix` with invalid input and count tables.

### Prompt (R3)

- Add cases beside `create_rejects_empty_user_prompt`.
- Invalid role and empty content must not increment `prompts`.
- Update-clear uses an existing row and compares `get` before/after.

### Portable (R4)

- Build tiny ZIP fixtures in a `tempfile` tree, same pattern as
  `bundle_rejects_traversal_and_duplicate_ids_before_writes`.
- v1 fixture: `formatVersion: 1`, no `typeDefinitions` / `references`.
- Cycle fixture: two folders that parent each other.
- Assert `backups` directory is absent after `VALIDATION`.

### Frontend (R5)

- `evaluationStore.test.ts`: inject rejecting `render` / `run` /
  `saveProfile` / `moveLabel`; assert `error` and unchanged arrays.
- `promptStore.test.ts`: inject `UNAUTHORIZED` on create/update/copy.

## Compatibility

- No schema, wire name, or DTO changes.
- The only allowed product edit is the Req 15.2 existence gate on
  `security.setMasterPassword`.
- Keep `proptest-regressions/` untouched.

## Trade-offs

- Service tests over command tests: commands are thin; the risk lives
  in services. Exception: the missing set-password gate.
- No llvm-cov: the request is risk-ordered tests, not a percentage.
- One task, sequential modules: matches "run that module's tests after
  each module" without a parent/child tree.

## Rollback

Delete the new `#[test]` / `it(...)` blocks. If the Command_Layer gate
was added, revert `commands/security.rs` only together with its test.
