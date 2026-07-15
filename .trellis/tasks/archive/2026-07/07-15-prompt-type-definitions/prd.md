# Add extensible prompt type definitions

## Goal

Support user-defined prompt type names while preserving `text`, `image`, and
`video` as stable base formats across storage, revisions, bundles, and evaluation.

## Confirmed Constraint

The current field is not free-form classification:

- frontend: `PromptType = "text" | "image" | "video"` (`types.ts:10-13`);
- backend: serde enum (`models/enums.rs:10-20`);
- service: validation and exhaustive wire mapping (`prompt.rs:151-170`);
- storage: CHECK constraints on prompts and revisions
  (`storage/mod.rs:427`, `storage/mod.rs:458`);
- versions, portable bundles, mapping, and evaluation all consume this invariant.

Allowing arbitrary strings in the current select would be a cross-layer defect.
This task adds custom organizational definitions without replacing the base
format or inventing new execution behavior.

## Requirements

- R1: Add a persisted `PromptTypeDefinition` with stable id, trimmed user-facing
  name, and required `baseKind` in `text|image|video`.
- R2: Keep the existing `PromptType`/`prompt_type` base-format field and CHECK
  constraint authoritative for rendering, execution, provider routing, and
  evaluation behavior.
- R3: Add an optional type-definition reference to prompts and immutable
  revisions. Existing rows with no reference retain current behavior and labels.
- R4: Provide typed list/create commands through service -> command -> Runtime
  Bridge -> feature API/store. Components never invoke commands directly.
- R5: In the editor, list built-in base formats plus custom definitions and allow
  inline custom creation with a name and chosen base format.
- R6: Custom names are unique after trim and case normalization. Creation is
  transactional and returns structured validation/conflict errors.
- R7: Saving a custom type stores a matching base kind and definition id in one
  transaction; invalid/mismatched pairs never mutate the prompt.
- R8: Complete revision snapshots/diffs, duplicate, batch behavior, portable
  export/import/preview/conflict handling, and evaluation links for the new
  reference.
- R9: Preserve private-prompt locking/encryption boundaries and never put prompt
  content or provider secrets into type definitions.
- R10: Localize built-in labels and custom-type create/validation states in all
  seven bundles.

## Acceptance Criteria

- [ ] AC1: Legacy schema upgrade is additive, idempotent, and preserves every
  existing prompt/revision byte-for-byte except required schema metadata.
- [ ] AC2: Existing prompts display Text/Image/Video exactly as before when no
  custom definition is assigned.
- [ ] AC3: A user can create a unique named type with one base format, select it,
  save a prompt, restart, and retrieve the same definition and base behavior.
- [ ] AC4: Empty, overlong, duplicate-normalized, unknown-base, missing-id, and
  id/base mismatch inputs fail before mutation with stable error codes.
- [ ] AC5: Save, automatic revision, diff, rollback, duplicate, portable
  export/import (skip/duplicate/replace), and evaluation round-trip the custom
  definition without changing base-format execution.
- [ ] AC6: Bundle preview reports definition additions/conflicts; a same-name
  different-base conflict requires explicit handling and never silently remaps.
- [ ] AC7: Locked private prompts expose safe type metadata but no protected
  prompt content; lock/unlock behavior is unchanged.
- [ ] AC8: Frontend and Rust targeted tests plus `just ci` and native migration/
  bundle/evaluation smoke pass.

## Out of Scope

- New executable modalities, plugins/handlers, provider adapters, per-type icons
  or colors, type rename/delete/reorder, aliases, hierarchy, or an administration
  page.
- Converting tags or folders into types.
- Dropping or widening the existing base-format CHECK constraint.

## Planning Decision

Confirmed by the product owner on 2026-07-15: custom types use a user-defined
name plus one required `text`, `image`, or `video` base format. Definitions are
immutable and create-only in this child. This is the smallest contract that
satisfies inline creation without introducing rename/delete lifecycle and
historical-label problems that the request did not ask for.
