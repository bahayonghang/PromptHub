# Design: Extensible Prompt Type Definitions

## Domain Model

```text
PromptType (existing base kind)
  text | image | video

PromptTypeDefinition (new organizational metadata)
  id
  name
  normalized_name
  base_kind -> PromptType
  created_at

Prompt / PromptVersion
  prompt_type -> existing base kind
  type_definition_id -> nullable definition reference
```

Built-in Text/Image/Video choices remain virtual and localized; they do not need
rows. A custom definition references one base kind. Services guarantee that a
prompt's base kind matches the referenced definition.

## Storage and Migration

Add an ordered additive migration that creates `prompt_type_definitions` and
nullable references on `prompts` and `prompt_versions`. Use a normalized unique
name column/index for deterministic conflict detection. Do not rewrite legacy
prompt rows and do not weaken the existing `prompt_type` CHECK constraints.

Because SQLite cannot add every desired foreign-key behavior safely with a
single `ALTER TABLE`, choose the smallest migration that preserves rollback and
enforce cross-field/reference invariants in the service, backed by migration and
service tests. Definitions are immutable and not deleted in this scope.

## Service and Wire Contract

Add typed list/create DTOs and `promptType.list` / `promptType.create` commands
(final wire names must follow the repository's domain.action convention). The
create service trims, length-checks, normalizes, validates base kind, inserts,
and returns the stored definition transactionally.

Prompt create/update resolves the optional definition before mutation and
derives or verifies `promptType`. The backend remains authoritative even if the
frontend sends a stale or inconsistent pair.

## Revision and Portable Compatibility

Complete revisions include the nullable definition id and a stable definition
snapshot sufficient for diff/history display. Rollback restores both definition
and base kind by appending a new revision.

Portable manifests include referenced definitions. Preview classifies each as
add/reuse/conflict. Import reuses only a normalized-name + matching-base
definition; same normalized name with a different base is an explicit conflict.
Duplicate prompt import may remap ids but preserves semantic definitions.

## Frontend Interaction

The editor picker shows built-ins first and custom definitions after a divider.
An adjacent plus action reveals inline name and base-format fields. After a
successful create, select the authoritative returned definition and update the
draft base kind. Failure keeps the draft and input unchanged.

The list badge/icon and evaluation branch continue to use the base kind. Where a
human-facing type name is shown, prefer the custom name when present.

## Compatibility and Rollback

Old frontend/backend combinations continue to understand the base kind. Existing
prompts need no migration. Rollback of the binary leaves additive table/columns
dormant; no data is dropped. A portable format version bump or optional-field
compatibility rule must be explicit and tested before release.
