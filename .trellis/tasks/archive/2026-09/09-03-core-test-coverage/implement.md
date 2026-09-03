# Implement: core-business test coverage

## Ordered checklist

### 0. Before coding

- Read `prd.md`, `design.md`, `research/coverage-gaps.md`.
- Read `.trellis/spec/cross-layer/prompt-library-foundations.md` and
  `prompt-evaluation-loop.md`.
- Do not add coverage tooling.

### 1. Security and private content (R1)

Files:

- `src-tauri/src/services/security.rs` (tests; possibly no product edit)
- `src-tauri/src/commands/security.rs` (existence gate if R1.2 fails)
- `src-tauri/src/services/prompt.rs` (locked create/update tests)

Checks:

- [ ] Unlock / change-password with no verifier → `UNAUTHORIZED`
- [ ] Second set-password does not replace verifier; add Command_Layer
      `CONFLICT` gate if the command currently overwrites
- [ ] `create_secure` / `update_secure` while locked → `UNAUTHORIZED`,
      no write

Validate:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib security::
cargo test --manifest-path src-tauri/Cargo.toml --lib prompt::private
cargo test --manifest-path src-tauri/Cargo.toml --lib prompt::create_secure
```

Use the actual new test names if they differ. Do not start module 2
until these pass.

### 2. Evaluation (R2)

File: `src-tauri/src/services/evaluation.rs`

Checks:

- [ ] Label validation, unevaluated target, history append, rollback
- [ ] Manual result skipped / non-manual / missing cell
- [ ] Cancelled run and remaining matrix cells persist `cancelled`
- [ ] Credential while locked → `UNAUTHORIZED`
- [ ] Loopback/localhost endpoint → `SSRF_BLOCKED`
- [ ] Test-set / evaluator / matrix input validation writes nothing

Validate:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib evaluation::
```

### 3. Prompt validation (R3)

File: `src-tauri/src/services/prompt.rs`

Checks:

- [ ] Invalid message role / empty content → `VALIDATION`, no row
- [ ] Update clearing body and messages → `VALIDATION`, unchanged row
- [ ] Chat-only create succeeds

Validate:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib prompt::
```

### 4. Portable bundles (R4)

File: `src-tauri/src/services/portable.rs`

Checks:

- [ ] Unsupported `formatVersion` → `VALIDATION`, no backup
- [ ] `formatVersion: 1` import succeeds
- [ ] Folder cycle → `VALIDATION`, no backup
- [ ] Plaintext private field → `VALIDATION`, no backup

Validate:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib portable::
```

### 5. Frontend error channels (R5)

Files:

- `src/features/evaluation/evaluationStore.test.ts`
- `src/features/prompts/promptStore.test.ts`

Checks:

- [ ] Evaluation store surfaces render/run/saveProfile/moveLabel errors
- [ ] Prompt store surfaces `UNAUTHORIZED` on locked private mutations

Validate:

```text
npx vitest run src/features/evaluation/evaluationStore.test.ts
npx vitest run src/features/prompts/promptStore.test.ts
```

### 6. Full suite (R6)

```text
just test-rust
just test
just ci
```

## Validation commands

| After | Command |
|-------|---------|
| Module 1 | `cargo test --manifest-path src-tauri/Cargo.toml --lib` filters above |
| Module 2 | `cargo test --manifest-path src-tauri/Cargo.toml --lib evaluation::` |
| Module 3 | `cargo test --manifest-path src-tauri/Cargo.toml --lib prompt::` |
| Module 4 | `cargo test --manifest-path src-tauri/Cargo.toml --lib portable::` |
| Module 5 | `npx vitest run src/features/evaluation/evaluationStore.test.ts src/features/prompts/promptStore.test.ts` |
| Finish | `just ci` |

Backend gate for this task: `just fmt-check`, `just clippy`, `just test-rust`.
Frontend gate: `just build`, `just test`. Combined: `just ci`.

## Risky files

| File | Risk |
|------|------|
| `src-tauri/src/commands/security.rs` | Only allowed product edit (Req 15.2 gate) |
| `src-tauri/src/services/security.rs` | Do not change key derivation or re-key |
| `src-tauri/src/services/evaluation.rs` | Large module; append tests, do not retune cache/runtime version |
| `src-tauri/src/services/portable.rs` | Do not change import order or conflict policy |

## Rollback points

- After each module: if tests fail because of a spec defect, apply the
  smallest contract-preserving fix in that module only, then re-run
  the module command.
- If the premise is wrong (behavior is already covered), delete the
  duplicate test rather than keep a tautology.
- Do not revert unrelated dirty files.

## Follow-up before `task.py start`

- `implement.jsonl` and `check.jsonl` have real spec/research entries.
- User has approved this planning summary.
