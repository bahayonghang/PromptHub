# 补充核心业务测试覆盖

## Goal

Protect PromptHub library integrity by adding tests for uncovered
high-risk business paths: permission, validation, transactions, private
content, portable import, and evaluation labels/cancel/SSRF. Do not add
tests only to raise a coverage percentage.

## User value

A failing core path is caught in CI before it encrypts with the wrong
key, imports a hostile bundle, or records a cancelled evaluation as an
error.

## Background

The existing suite already covers folder CRUD, media formats, settings
patches, window shortcuts, most prompt search/CRUD, private-prompt
redaction on read, portable happy/conflict paths, and evaluation matrix
ordering/cache. The gaps below come from comparing
`.trellis/spec/cross-layer/prompt-library-foundations.md` and
`prompt-evaluation-loop.md` with current `#[test]` names. Full inventory:
`research/coverage-gaps.md`.

## Confirmed facts

- Backend tests live next to services (`src-tauri/src/services/*.rs`)
  and in `src-tauri/tests/*_properties.rs`. Frontend tests are colocated
  `*.test.ts`.
- Services must stay Tauri-free. New tests inject `Connection` /
  `DbPool` / fake adapters, not a live window.
- Error codes are the stable contract in `src-tauri/src/error.rs`.
- `CURRENT_SCHEMA_VERSION = 6`. v4→current upgrade is already tested.
- `security.setMasterPassword` is documented as gated by "no master
  password exists" (`services/security.rs:113-115`) but
  `commands/security.rs:12-20` has no gate.

## Requirements

### R1 — Security and private content

- **R1.1** Unlock and change-password with no stored verifier return
  `UNAUTHORIZED` and leave the lock flag unchanged.
- **R1.2** After a master password exists, a second set-password call
  must not replace the verifier or orphan existing `ENC::` ciphertext
  (Req 15.2). If the command currently allows replacement, add the
  missing Command_Layer gate and test it.
- **R1.3** Creating or updating a private prompt while locked returns
  `UNAUTHORIZED` and writes nothing.
- **R1.4** Locked private copy remains `UNAUTHORIZED` (already covered;
  keep the existing assertion, do not duplicate).

### R2 — Evaluation

- **R2.1** Labels: only `baseline`/`candidate`; only `move`/`rollback`;
  revision must belong to the prompt; target must have a successful
  evaluation cell. A valid move/rollback appends
  `prompt_label_history` and does not rewrite prompt/version rows.
- **R2.2** Manual results stay skipped until reviewed; non-manual
  evaluators and missing cells fail with `VALIDATION` / `NOT_FOUND`.
- **R2.3** Cancelled runs and remaining matrix cells persist as
  `cancelled`, not `error` or `running`.
- **R2.4** Saving or using a credential while locked returns
  `UNAUTHORIZED`. A loopback/`localhost` openai-compatible endpoint
  returns `SSRF_BLOCKED` before any provider body is sent.
- **R2.5** Test-set, evaluator, profile, and matrix input validation
  (empty names, >1000 cases, unsupported kind, invalid regex, empty
  matrix ids, empty test set) returns `VALIDATION` and writes nothing.

### R3 — Prompt validation

- **R3.1** Message role outside `system|user|assistant` or empty
  content returns `VALIDATION` and creates no prompt.
- **R3.2** An update that leaves both `userPrompt` and `messages` empty
  returns `VALIDATION` and leaves the row unchanged.
- **R3.3** Chat-only create (empty `userPrompt`, valid messages)
  succeeds.

### R4 — Portable bundles

- **R4.1** `formatVersion` outside `1..=2` is `VALIDATION` and creates
  no backup.
- **R4.2** A `formatVersion: 1` bundle without type definitions
  imports (legacy compatibility).
- **R4.3** A cyclic folder parent chain is `VALIDATION` before backup.
- **R4.4** Plaintext private fields in a private bundle are
  `VALIDATION` before backup.

### R5 — Frontend error channels

- **R5.1** Evaluation store surfaces `BridgeError` from render, run,
  save-profile, and move-label without mutating other collections.
- **R5.2** Prompt store surfaces `UNAUTHORIZED` from create/update/copy
  of locked private content.

### R6 — Execution discipline

- **R6.1** Add tests in business-risk order: security → evaluation →
  prompt → portable → frontend.
- **R6.2** After each module, run that module's tests. After all
  modules, run `just test-rust` and `just test`. Finish with `just ci`.
- **R6.3** Do not add tautological tests, coverage tooling, or tests
  for already-covered folder/media/settings/window/updater paths.

## Acceptance Criteria

- [ ] AC1: New Rust tests fail for the conditions in R1–R4 with the
      documented `ErrorCode` and assert the database / files / lock
      state is unchanged on failure.
- [ ] AC2: Label move/rollback and cancel persistence match
      `prompt-evaluation-loop.md` (history append, `cancelled` status).
- [ ] AC3: Portable v1 import succeeds; unsupported version, folder
      cycle, and plaintext private fields write nothing.
- [ ] AC4: Frontend store tests in R5 fail closed on injected
      `BridgeError` and keep prior collections.
- [ ] AC5: Each module's targeted `cargo test` / `npx vitest run`
      passed before the next module started.
- [ ] AC6: `just ci` passed after all modules.
- [ ] AC7: No new tests exist solely to cover getters, adapters, or
      already-green paths listed as out of scope.

## Out of scope

- Line-coverage percentage targets and coverage report tooling.
- Live provider smoke, native Tauri window smoke.
- Window, updater install, media format matrix, appearance, i18n key
  parity, Runtime Bridge import-boundary tests.
- `ref/PromptHub/**`, generated `dist/` / `src-tauri/target/` /
  `src-tauri/gen/schemas/`.
- Product behavior changes other than the Req 15.2 set-password gate
  if R1.2 proves the command overwrites a verifier.

## Key decisions

- Tests encode the specs in `.trellis/spec/cross-layer/`, not current
  accidental behavior. If a new test fails against that contract,
  apply the smallest contract-preserving fix in the same module.
- Keep one Trellis task. Modules are sequential checklist items, not
  child tasks.

## Risks

- `security.setMasterPassword` may currently overwrite the verifier.
  The R1.2 test will expose it; the allowed fix is a Command_Layer
  existence check, not a re-key redesign.
- `execute_openai` SSRF tests must use blocked URLs / hostname forms
  so they do not depend on the public network.
