# Implementation Plan: Extensible Prompt Type Definitions

## Ordered Checklist

1. Add migration/upgrade/idempotency/failure fixtures for the definition table
   and nullable prompt/revision references.
2. Add Rust model, mapping, list/create service, validation/conflict tests,
   commands, registrations, and stable wire names.
3. Extend prompt create/update/get/search/duplicate and private locked DTO paths;
   validate definition/base pairs before mutation.
4. Extend complete revisions, append/no-op detection, structured diff, rollback,
   and version tests.
5. Extend portable manifest/export/preview/import/conflict/id-remap behavior and
   bump or document format compatibility.
6. Extend TypeScript DTOs, API, store loading/actions, and injected test fakes.
7. Replace the selection-only editor field with built-in/custom choices plus an
   inline create name/base interaction; add all locale keys.
8. Verify evaluation uses base kind unchanged and displays custom names only as
   metadata; add regression tests.
9. Run targeted backend/frontend gates, disposable backup/restore migration
   rehearsal, full `just ci`, and native smoke.

## Verification

```powershell
npx vitest run src/features/prompts
npx vitest run src/features/evaluation
cargo test prompt_type --manifest-path src-tauri/Cargo.toml
cargo test portable --manifest-path src-tauri/Cargo.toml
cargo test version --manifest-path src-tauri/Cargo.toml
just build
just test
just fmt-check
just clippy
just test-rust
just ci
```

Required scenarios: current-schema upgrade, repeat startup, migration failure,
legacy built-ins, create/restart, duplicate normalized name, mismatched base,
revision/diff/rollback, private lock/unlock, all three bundle conflict policies,
and evaluation of a custom-named type for each base kind.

## Risk and Rollback

- Snapshot a disposable representative database and prove restore before running
  the migration outside tests.
- Land storage/service invariants before exposing the editor control.
- Review revision and portable round trips before evaluation/UI integration.
- If migration or compatibility checks fail, revert this child; additive dormant
  schema is tolerated, but no destructive cleanup runs automatically.
