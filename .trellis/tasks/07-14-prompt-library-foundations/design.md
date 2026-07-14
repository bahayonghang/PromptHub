# Design: Prompt Library Foundations

## Data Evolution

Add a single ordered migration runner owned by storage. Each migration runs in a
transaction, advances an explicit schema version only after success, and has
upgrade tests starting from the current schema. Application startup never drops
legacy Skill data as part of this child.

## Search Contract

Return a structured page containing `items`, `total`, and paging metadata. Use a
stable secondary sort by prompt id so rows cannot move ambiguously between pages.
The UI may use explicit pagination or incremental loading, but it must show the
total and make every row reachable. Filter changes reset the page atomically.

## Revision Contract

Create an immutable revision for the complete user-meaningful prompt state:

- title, description, prompt type/definition;
- variables and validation metadata;
- tags, folder, favorite/pinned state as selected by the final UX decision;
- media references, source, notes;
- revision note, source action, parent revision, created timestamp.

Saving a changed prompt and its revision is one transaction. Rollback reads an
old revision and appends a new one with `source=rollback`; it never rewrites or
deletes history. Diff operates on structured fields, not serialized JSON text.

## Portable Bundle

Use a manifest with an explicit format version plus prompt/folder/revision JSON
records and optional media payloads. Import is parse -> validate -> preview ->
automatic backup -> transactional apply. Conflict policy is explicit per import:
skip, duplicate with new ids, or replace by appending a revision. Paths are
normalized and cannot escape the bundle/root.

## Private Prompts

Define which fields are sensitive and encrypt them with the existing envelope
service. List responses return locked metadata without content. Search never
indexes encrypted plaintext. Export includes encrypted values unless the user
explicitly chooses a decrypted portable export while unlocked; that action must
carry a clear warning and never include provider secrets by default.

## Basic Operations

Expose existing pin/tag capabilities and add duplicate/batch services as atomic
backend operations. The frontend sends intent through typed bridge APIs and
refreshes only affected pages/counts.
