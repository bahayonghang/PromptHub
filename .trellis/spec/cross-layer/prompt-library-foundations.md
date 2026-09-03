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
prompt.copy({ id, values }) -> PromptCopy
prompt.incrementUsage({ id }) -> PromptUsage
promptType.list() -> PromptTypeDefinition[]
promptType.create({ input: { name, baseKind } }) -> PromptTypeDefinition
version.list({ promptId }) -> PromptVersion[]
version.create({ promptId, note? }) -> PromptVersion
version.rollback({ promptId, version }) -> Prompt
prompt.bundleExport({ destination? }) -> PortableExportResult
prompt.bundlePreview({ filePath }) -> BundlePreview
prompt.bundleImport({ filePath, policy }) -> PortableImportResult
```

`PromptCopy` is substituted system/user/messages plus `unexpanded` reference tokens.
`PromptUsage` is `{ id, usageCount }`. `prompt.copy` is read-only. `prompt.incrementUsage`
adds one to `usage_count` and does not write `updated_at`.

`PromptTypeDefinition` is `{ id, name, baseKind, createdAt }`, where `baseKind`
is `text`, `image`, or `video`. Prompt create/update carries the existing
`promptType` plus nullable `typeDefinitionId`.

SQLite evolution is owned by `storage::CURRENT_SCHEMA_VERSION`, the ordered
`MIGRATIONS` list, and `PRAGMA user_version`. Version 1 adopts the legacy
desktop schema, version 2 expands immutable prompt revisions, and version 3
adds private flags to prompts and revisions. Version 4 adds chat/evaluation
storage. Version 5 adds immutable prompt type definitions, nullable Prompt
references, and definition snapshots on revisions.

### 3. Contracts

- `PromptPage` contains `items`, `total`, `limit`, `offset`, and `hasMore`.
  Ordering always uses prompt id as a stable secondary key.
- `PromptVersion` snapshots title, description, type, system/user prompts,
  variables, tags, folder, media references, favorite/pinned/private flags,
  source, notes, AI response, provenance, parent revision, and timestamp.
- Custom prompt types are organizational metadata only. `prompt_type` remains
  the authoritative execution format; a referenced definition must have the
  same `base_kind`. Built-in Text/Image/Video choices use a null definition id.
- Type names are trimmed, limited to 100 Unicode scalar values, and unique by
  trimmed lowercase normalization. Definitions are immutable and create-only.
  Missing ids return `NOT_FOUND`; id/base mismatches return `VALIDATION` before
  any Prompt write; normalized duplicates return `CONFLICT`.
- Revisions store the definition id plus immutable name/base metadata. Rollback,
  duplicate, batch snapshots, locked private DTOs, and evaluation rendering
  preserve this metadata while evaluation continues to consume the base kind.
- Meaningful saves append one revision in the same transaction. No-op saves
  append none. Rollback appends a revision with `sourceAction=rollback` and
  never rewrites or deletes history.
- A prompt bundle is a ZIP with exactly one `manifest.json`, format version 2,
  and optional `media/<safe-relative-path>` entries. The manifest contains
  prompts, revisions, folders, referenced type definitions, and the declared
  media path list. Import still accepts version 1 bundles with no definitions.
- Bundle preview reports custom-type additions and normalized-name/base
  conflicts. Import reuses a same-name/same-base target definition and remaps
  references to its id. A same-name/different-base definition returns
  `CONFLICT` before backup or writes; prompt conflict policy never silently
  changes type semantics.
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
| Type name empty/over 100 chars or base kind unknown | `VALIDATION`; create nothing |
| Type name duplicates after trim/lowercase | `CONFLICT`; create nothing |
| Definition id is missing | `NOT_FOUND`; Prompt unchanged |
| Definition base differs from `promptType` | `VALIDATION`; Prompt unchanged |
| Bundle version is outside `1..=2` | `VALIDATION`; write nothing |
| Bundle type name exists with another base kind | Preview conflict; import `CONFLICT` before backup |
| Duplicate/empty ids, missing references, folder cycle | `VALIDATION`; write nothing |
| Absolute or traversal media path | `VALIDATION`; never extract it |
| Declared media is absent or existing media bytes conflict | `VALIDATION`; rollback and clean staging |
| Private field is plaintext in a private bundle | `VALIDATION`; write nothing |
| Private bundle is locked or uses another key | `UNAUTHORIZED`; no backup or writes |
| Private prompt is read while locked | Return metadata with `isLocked=true` and redacted content |
| `security.setMasterPassword` when a verifier already exists | `CONFLICT`; verifier and `ENC::` rows unchanged. Re-key only through `security.changeMasterPassword`. The service primitive still overwrites; the Command_Layer owns the gate. |
| Create or update private content while locked | `UNAUTHORIZED`; no Prompt write |

### 5. Good / Base / Bad Cases

- Good: page through 250 prompts with a stable sort and observe every id once;
  create a Storyboard/image type, edit a prompt, diff the new revision, then
  rollback and retain both the custom name and image execution behavior.
- Base: export/import an empty or public-only library; repeated startup and
  repeated preview remain read-only and idempotent. A version 1 bundle and a
  built-in prompt with a null definition id retain legacy behavior.
- Bad: import a ZIP containing `../escape`, a cyclic folder tree, mismatched
  media bytes, a missing/mismatched type definition, plaintext private fields,
  or ciphertext from another key. The original database and media remain
  unchanged.

### 6. Tests Required

- Storage: current-schema upgrade, repeated startup, backup precondition, and
  injected migration failure asserting data plus `user_version` are preserved;
  v4 to v5 must leave legacy Prompt/Revision values unchanged and refs null.
- Search: a 250-row fixture asserting total, window boundaries, unique ids, and
  reachability after create/update/delete.
- Revisions: full-field round trip, no-op save, monotonic numbering, structured
- Type definitions: trim/case uniqueness, all three base kinds, empty/overlong/
  unknown-base validation, missing-id and mismatch atomicity, duplicate/batch/
  private-lock metadata preservation, and evaluation rendering regression.
- Portable: Unicode and nested folders, revisions, media round trip, preview,
  all conflict policies, v1 compatibility, definition id remap/reuse/conflict,
  traversal rejection, media collision cleanup, and wrong-key private import
  before writes.
- Privacy: ciphertext-at-rest scans, locked redaction, FTS exclusion, unlock,
  atomic password re-key for prompts plus revisions, second
  `setMasterPassword` leaving the verifier unchanged, and locked
  create/update writing nothing.
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

#### Wrong

```rust
// Treating a custom name as a new executable modality breaks every base-kind consumer.
prompt.prompt_type = parse_arbitrary_string(input.name)?;
```

#### Correct

```rust
let definition = prompt_type::get(conn, definition_id)?;
ensure_requested_base_matches(definition.base_kind, input.prompt_type)?;
prompt.type_definition_id = Some(definition.id);
prompt.prompt_type = definition.base_kind;
```

The service owns the pair invariant in the same transaction as the Prompt
mutation. Frontend selection and validation improve feedback but are never the
authority for execution behavior.

## Scenario: Recording a clipboard copy as usage

### 1. Scope / Trigger

Use this contract when changing Prompt copy, clipboard write, the library
usage column, or `usage_count`. Copy spans `prompt.copy` (read-only expansion),
the frontend clipboard write, `prompt.incrementUsage`, and the prompt store
patch. Do not treat copy as a content edit.

### 2. Signatures

```text
prompt.copy({ id: string, values: Record<string, string> }) -> PromptCopy
prompt.incrementUsage({ id: string }) -> { id: string, usageCount: number }
```

SQL for increment:

```sql
UPDATE prompts SET usage_count = usage_count + 1 WHERE id = ?1
RETURNING usage_count
```

### 3. Contracts

- Clipboard text is assembled on the frontend from `PromptCopy` (`formatCopiedPrompt`).
- Increment runs only after `writeText` succeeds, and only when a persisted
  `promptId` is present.
- Increment does not write `updated_at`, does not append a revision, and does
  not reload `prompt.search`. The store patches `prompts` and `selectedPrompt`.
- Create-draft copy (no `promptId`) writes the clipboard and does not increment.
- Keyboard copy (`Ctrl+Enter` / `Cmd+Enter`) and fill-and-copy of a persisted
  Prompt increment once per successful write.
- Success feedback is in-control `Check` plus a `ToastHost` success toast with
  `replaceGroup: "prompt-copy"`. Failure stays on the control and may push a
  danger toast in the same group. Increment failure after a successful write
  must not flip the control to failed.

### 4. Validation & Error Matrix

| Condition | Error / behavior |
|-----------|------------------|
| Increment id is missing | `NOT_FOUND`; no row created |
| Copy of a locked private Prompt | `UNAUTHORIZED`; no clipboard write; no increment |
| Clipboard write fails after `prompt.copy` | In-control failed + danger toast; no increment |
| Increment fails after a successful write | Copy toast stays; `usageCount` may stay stale until the next load |
| `prompt.update({ usageCount })` used to count a copy | Forbidden: that path stamps `updated_at` |

### 5. Good / Base / Bad Cases

- Good: copy an unlocked Prompt with `usageCount` 0, then again; stored counts
  are 1 then 2 and `updated_at` is unchanged.
- Base: create-draft copy writes the clipboard and does not call increment.
- Bad: increment inside `prompt.copy` before the clipboard write; a failed
  write would still count a use.

### 6. Tests Required

- Service: 0→1→2, `updated_at` unchanged, missing id → `NOT_FOUND`, `copy`
  leaves `usage_count` unchanged.
- Bridge: `prompt.incrementUsage` wire name and `{ id }` args.
- Store: patches list + selected usage; failure returns `null` without a view error.
- Control: increment after `writeText`; locked / no `promptId` / write failure
  do not increment; `replaceGroup` replaces copy toasts and keeps save toasts.

### 7. Wrong vs Correct

#### Wrong

```rust
// Counting a use inside read-only copy, and stamping updated_at.
pub fn copy_secure(...) -> Result<PromptCopy, AppError> {
    let prompt = get_secure(...)?;
    conn.execute(
        "UPDATE prompts SET usage_count = usage_count + 1, updated_at = ?1 WHERE id = ?2",
        params![now_millis(), id],
    )?;
    Ok(build_copy(prompt))
}
```

#### Correct

```rust
pub fn increment_usage(conn: &Connection, id: &str) -> Result<PromptUsage, AppError> {
    let usage_count: i64 = conn
        .query_row(
            "UPDATE prompts SET usage_count = usage_count + 1 WHERE id = ?1 RETURNING usage_count",
            [id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found(format!("prompt `{id}` not found")))?;
    Ok(PromptUsage { id: id.to_string(), usage_count })
}
```

Frontend order: `prompt.copy` → `writeText` → `prompt.incrementUsage`.
