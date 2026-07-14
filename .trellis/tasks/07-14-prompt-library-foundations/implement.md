# Implementation Plan: Prompt Library Foundations

## Ordered Checklist

1. Add migration runner, current-schema fixture, backup gate, and failure tests.
2. Implement counted/stable search contract; update Rust/TS DTOs, API/store/UI,
   and >100-row tests.
3. Introduce complete revision schema and atomic save/no-op/rollback semantics.
4. Add structured diff UI and revision provenance.
5. Implement portable manifest export, parser/validator/preview, conflict policy,
   automatic backup, transactional import, and path-safety tests.
6. Wire private prompt encryption, locked DTOs, FTS exclusion, export policy, and
   password re-key tests.
7. Expose pin/tag management, duplicate, and batch operations; reconcile locale
   claims with reachable behavior.
8. Update docs/code maps and run full/native gates.

## Verification

```powershell
just build
just test
just fmt-check
just clippy
just test-rust
just ci
```

Required fixtures/scenarios:

- current-schema database with folders/prompts/versions/private data;
- 250-prompt deterministic paging corpus;
- Unicode and media portable bundle;
- duplicate ids, malformed versions, traversal paths, interrupted import;
- lock/unlock/re-key/search/export with plaintext leak scans.

## Rollback Points

- Migration runner before any schema migration.
- Paging contract before revision conversion.
- Complete revisions before portable import.
- Portable import before private-content rollout.

Every schema-changing phase requires a disposable backup/restore rehearsal before
continuing.
