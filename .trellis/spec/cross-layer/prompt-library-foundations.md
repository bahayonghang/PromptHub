# Prompt Library Foundations

## Scenario: Evolving and transporting the prompt library

### 1. Scope / Trigger

Use this contract when changing prompt persistence, search reachability,
revision history, portable bundles, or private prompt fields. These features
cross SQLite storage, Rust services, Tauri commands, the Runtime Bridge,
Zustand state, and React UI.

### 2. Signatures

Wire commands and their TypeScript mirrors:

```text
prompt.search({ query: SearchQuery }) -> PromptPage
version.list({ promptId }) -> PromptVersion[]
version.create({ promptId, note? }) -> PromptVersion
version.rollback({ promptId, version }) -> Prompt
prompt.bundleExport({ destination? }) -> PortableExportResult
prompt.bundlePreview({ filePath }) -> BundlePreview
prompt.bundleImport({ filePath, policy }) -> PortableImportResult
```

SQLite evolution is owned by `storage::CURRENT_SCHEMA_VERSION`, the ordered
`MIGRATIONS` list, and `PRAGMA user_version`. Version 1 adopts the legacy
desktop schema, version 2 expands immutable prompt revisions, and version 3
adds private flags to prompts and revisions.

### 3. Contracts

- `PromptPage` contains `items`, `total`, `limit`, `offset`, and `hasMore`.
  Ordering always uses prompt id as a stable secondary key.
- `PromptVersion` snapshots title, description, type, system/user prompts,
  variables, tags, folder, media references, favorite/pinned/private flags,
  source, notes, AI response, provenance, parent revision, and timestamp.
- Meaningful saves append one revision in the same transaction. No-op saves
  append none. Rollback appends a revision with `sourceAction=rollback` and
  never rewrites or deletes history.
- A prompt bundle is a ZIP with exactly one `manifest.json`, format version 1,
  and optional `media/<safe-relative-path>` entries. The manifest contains
  prompts, revisions, folders, and the declared media path list.
- Import order is parse, validate, preview, private-key validation, safety
  backup, media staging, transactional database apply, media install, commit.
  Any failure rolls back database writes and removes staged/new media.
- Conflict policy is `skip`, `duplicate`, or `replace`. Replace appends a new
  revision; it does not overwrite existing revision history.
- Private content fields are description, system prompt, user prompt, source,
  notes, and last AI response. They use AES-256-GCM `ENC::` envelopes at rest,
  are redacted while locked, and are excluded from FTS.
- Encrypted private bundles are key-bound. Import requires the library to be
  unlocked with the same derived key that encrypted every private prompt and
  revision. A different key fails before backup or writes.

### 4. Validation & Error Matrix

| Condition | Error / behavior |
|-----------|------------------|
| Search limit outside `1..=100` | Clamp to the supported range |
| Migration sequence skips a version | `INTERNAL`; do not advance `user_version` |
| Existing database needs migration but backup fails | `IO`; do not migrate |
| Revision note exceeds 1000 characters | `VALIDATION`; append nothing |
| Bundle version is not 1 | `VALIDATION`; write nothing |
| Duplicate/empty ids, missing references, folder cycle | `VALIDATION`; write nothing |
| Absolute or traversal media path | `VALIDATION`; never extract it |
| Declared media is absent or existing media bytes conflict | `VALIDATION`; rollback and clean staging |
| Private field is plaintext in a private bundle | `VALIDATION`; write nothing |
| Private bundle is locked or uses another key | `UNAUTHORIZED`; no backup or writes |
| Private prompt is read while locked | Return metadata with `isLocked=true` and redacted content |

### 5. Good / Base / Bad Cases

- Good: page through 250 prompts with a stable sort and observe every id once;
  edit a prompt, diff the new revision, then rollback and see a later revision.
- Base: export/import an empty or public-only library; repeated startup and
  repeated preview remain read-only and idempotent.
- Bad: import a ZIP containing `../escape`, a cyclic folder tree, mismatched
  media bytes, plaintext private fields, or ciphertext from another key. The
  original database and media remain unchanged.

### 6. Tests Required

- Storage: current-schema upgrade, repeated startup, backup precondition, and
  injected migration failure asserting data plus `user_version` are preserved.
- Search: a 250-row fixture asserting total, window boundaries, unique ids, and
  reachability after create/update/delete.
- Revisions: full-field round trip, no-op save, monotonic numbering, structured
  diff, and rollback provenance.
- Portable: Unicode and nested folders, revisions, media round trip, preview,
  all conflict policies, traversal rejection, media collision cleanup, and
  wrong-key private import before writes.
- Privacy: ciphertext-at-rest scans, locked redaction, FTS exclusion, unlock,
  and atomic password re-key for prompts plus revisions.
- Bridge: command names, camelCase argument names, DTO nullability, Rust command
  registration, and frontend API/store tests.
- Finish with `just ci` and an isolated native Tauri export/preview/import smoke.

### 7. Wrong vs Correct

#### Wrong

```rust
// Imports source ciphertext without proving that the target can decrypt it.
portable::import_bundle(&conn, &path, policy, &data, &backup, &media, None)?;
```

#### Correct

```rust
let key = security::unlocked_key(&state.encryption)?;
portable::import_bundle(
    &conn,
    &path,
    policy,
    &data,
    &backup,
    &media,
    key.as_deref(),
)?;
```

The portable service validates every private envelope with that key before it
creates a safety backup, stages media, or starts the database transaction.
