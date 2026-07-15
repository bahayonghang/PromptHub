# Harden prompt library foundations

## Goal

Make the local prompt library complete, reachable, version-safe, portable, and
actually private before adding an evaluation workbench.

## Confirmed Defects

- Search defaults to 50 rows while the frontend has no pagination or total.
- Prompt versions snapshot only system prompt, user prompt, and variables and are
  created manually; the UI cannot diff them.
- Master-password encryption is not wired to prompt fields.
- Export is a ZIP of data directories; there is no prompt-level portable import,
  preview, conflict handling, or round trip.
- `isPinned` and tag rename/delete exist in storage/backend but are unreachable;
  several other locale claims have no live implementation.
- The repository has no ordered schema migration mechanism.

## Requirements

- R1: Add an ordered, transactional schema migration mechanism with explicit
  version tracking, backup precondition, and upgrade tests.
- R2: Replace one-page search with a counted page/cursor contract and UI loading
  that reaches every prompt deterministically.
- R3: Store complete immutable prompt revisions and create them automatically on
  meaningful saves; no-op saves create no revision.
- R4: Provide field-aware version diff and rollback-as-new-revision so history is
  append-only and attributable.
- R5: Define a documented, versioned, human-readable prompt bundle format with
  preview, validation, duplicate/conflict policy, media references, and safety
  backup before import.
- R6: Make private prompts real: selected content fields are encrypted at rest,
  unavailable while locked, excluded from plaintext FTS/export/logging, and
  re-keyed atomically on password change.
- R7: Complete basic library affordances already represented by storage or
  product copy: pin/unpin, tag rename/delete, duplicate, and safe batch
  move/tag/delete. Remove unsupported stale locale claims not owned by this or
  the evaluation child.
- R8: Preserve all existing prompt/folder/rule/settings/media data through upgrade
  and keep Runtime Bridge/error-envelope conventions.

## Acceptance Criteria

- [ ] AC1: A 250-prompt fixture exposes all rows without duplicates or omissions
  across initial load, paging, filter, sort, create, update, and delete.
- [ ] AC2: Migration from the current schema and repeated startup are idempotent;
  injected migration failure leaves the old database usable.
- [ ] AC3: Every mutable prompt field covered by the revision contract round-trips
  and appears in diff; rollback creates a new revision with provenance.
- [ ] AC4: Portable export/import round-trips prompts, revisions, folders, media
  references, and Unicode; preview reports adds/conflicts/skips before writes.
- [ ] AC5: Import failure restores or leaves the original state atomically and an
  automatic safety backup is available.
- [ ] AC6: Locked private prompts expose neither plaintext nor searchable content;
  unlock and password change preserve access.
- [ ] AC7: Pin, tag management, duplicate, and batch operations are reachable,
  keyboard operable, localized, transactional where needed, and tested.
- [ ] AC8: `just ci` and native upgrade/export/import/lock smoke pass.

## Out of Scope

- Prompt execution, providers, run history, datasets, evaluators, or model compare.
- Hosted sync protocol redesign, team permissions, or public sharing service.
- Prompt references, experiment branches, or deployment environments.
