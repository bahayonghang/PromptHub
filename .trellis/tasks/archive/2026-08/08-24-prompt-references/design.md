# Design — backend prompt-reference capability

## What the copy path actually is

The PRD's background says reference expansion "belongs in the same path so both
the list copy control and the detail overlay inherit it". That premise does not
hold. Four facts settle the shape of this task.

1. **`prompt.copy` has no caller.** `copyPrompt` is declared on `PromptApi`
   (`src/features/prompts/api.ts:38`) and wired in `createPromptApi` (`:84`), and
   nothing in `src/` invokes it. Every copy today goes through
   `buildPromptCopyText` (`promptText.ts:138-157`), a pure frontend function
   called from `CopyPromptButton.tsx:70`.

2. **The declared mirror type is wrong.** `api.ts:38` types the result as
   `Promise<string>`. The command returns `PromptCopy { systemPrompt,
userPrompt }` (`src-tauri/src/commands/prompt.rs:98`,
   `services/prompt.rs:1032-1039`). The mismatch has never failed because
   nothing exercises it.

3. **`prompt.copy` ignores chat mode.** `copy_secure`
   (`services/prompt.rs:1108-1127`) substitutes into `system_prompt` and
   `user_prompt` only. It never reads `prompt.messages`. The frontend builder
   does: a chat-mode prompt copies as labeled `[System]` / `[User]` /
   `[Assistant]` blocks (`promptText.ts:140-149`). Routing the existing copy
   controls at today's `prompt.copy` would drop message structure.

4. **`copy_secure` refuses a locked prompt.** It returns `UNAUTHORIZED`
   (`services/prompt.rs:1115-1119`) rather than redacted text, unlike `search`,
   which returns rows with the body redacted.

Consequence: adding expansion only inside today's `prompt.copy` would satisfy
the letter of parent requirement R12 and change nothing a user can observe. This
design therefore makes `prompt.copy` the real copy path, which requires the DTO
to carry what the frontend builder carries.

## What the storage layer allows

5. **Schema changes need two edits plus a constant.** A fresh database runs
   `SCHEMA_SQL` in one batch and stamps `CURRENT_SCHEMA_VERSION`
   (`storage/mod.rs:329-341`); an existing database runs `MIGRATIONS`
   (`:55`). A new table must appear in both, and `CURRENT_SCHEMA_VERSION` must go
   from 5 to 6. Adding only the migration leaves fresh installs without the
   table.

6. **`foreign_keys = ON`** (`storage/mod.rs:283`), so `ON DELETE CASCADE` and
   `ON DELETE SET NULL` are enforced, as `prompt_versions` and `prompts` already
   rely on (`:479`, `:462`).

7. **Titles are not unique.** `prompts.title` has no `UNIQUE` constraint
   (`storage/mod.rs:451-475`). Two prompts may share a title.

8. **The bundle manifest takes additive fields without a version bump.** No
   struct in the codebase uses `deny_unknown_fields`, and
   `PromptBundleManifest.type_definitions` is already
   `#[serde(default)]` (`services/portable.rs:29-30`). An older build reading a
   newer manifest ignores the extra field; a newer build reading an older
   manifest defaults it to empty.

9. **Import remaps ids under the `duplicate` policy.** `prompt_map`
   (`services/portable.rs:858-870`) assigns a fresh uuid per prompt under
   `Duplicate` and keeps the original id under `Skip` and `Replace`.

## Decisions

### D1 — The edge table stores the resolved id, the literal token, and why it did not resolve

Schema, added to `SCHEMA_SQL` and as migration 6:

```sql
CREATE TABLE IF NOT EXISTS prompt_references (
  id TEXT PRIMARY KEY,
  source_prompt_id TEXT NOT NULL REFERENCES prompts(id) ON DELETE CASCADE,
  target_prompt_id TEXT REFERENCES prompts(id) ON DELETE SET NULL,
  token_title TEXT NOT NULL,
  resolution TEXT NOT NULL CHECK(resolution IN ('resolved','missing','ambiguous')),
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_prompt_refs_source ON prompt_references(source_prompt_id);
CREATE INDEX IF NOT EXISTS idx_prompt_refs_target ON prompt_references(target_prompt_id);
```

`token_title` is stored, not re-derived on read (PRD D1). Re-deriving means
scanning the source body at read time, and a private prompt's body is ciphertext
in the column — the scan would need the encryption key for a read that otherwise
does not. Resolution runs at save time, when the plaintext is already in hand.

`ON DELETE SET NULL` on the target with `resolution` left at its stored value
would lie. Deleting a target therefore also rewrites `resolution` to `missing`;
see D5.

Privacy note to record: this table is not encrypted. It reveals that prompt A
references a prompt titled T. Titles, tags, and folder membership are already
stored in plaintext (`services/prompt.rs:499-524` leaves them intact when
locked), so this adds a relationship to metadata that is already readable, and
no body text.

### D2 — An ambiguous title stays unresolved and says so

Titles are not unique (finding 7). Three ways to handle `@@Foo` when two prompts
are titled `Foo`:

- **Reject the save.** Hostile. A user cannot save an unrelated edit because two
  other prompts happen to share a title.
- **Pick deterministically**, for example the oldest. The reference silently
  points at a prompt the user did not choose, and copy inlines the wrong body.
- **Leave it unresolved, with `resolution = 'ambiguous'`.** Chosen.

The third state costs one CHECK value and lets the references tab say "two
prompts are titled Foo" instead of "Foo not found", which is the difference
between an actionable message and a confusing one.

### D3 — Bundles carry edges as an additive manifest field

`PromptBundleManifest` gains:

```rust
#[serde(default)]
pub references: Vec<PromptReferenceRecord>,
```

`FORMAT_VERSION` stays at 2, following the `type_definitions` precedent
(finding 8). Old builds ignore the field; new builds default it to empty for old
bundles. Bumping to 3 would make every bundle this build exports unreadable by
older builds (`services/portable.rs:345`) for a field they can safely ignore.

Import remaps `source_prompt_id` and `target_prompt_id` through the existing
`prompt_map` (finding 9), so the `duplicate` policy produces edges among the
duplicated copies rather than pointing at the originals.

Rejected: rebuilding edges on import by re-resolving titles. Under `duplicate`
every imported title now exists twice — once in the library and once as the
import — so every rebuilt edge would resolve to `ambiguous`. Carrying the edges
is exact and cheaper.

An imported edge whose target was skipped is re-resolved once after insert, so
it lands as `resolved` if a same-titled prompt exists or `missing` if not.

### D4 — `PromptCopy` carries messages and an unexpanded list, and `prompt.copy` becomes the real copy path

Findings 1 to 3 mean the DTO must carry what the frontend builder carries, or
the capability is unobservable.

```rust
pub struct PromptCopy {
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub messages: Vec<PromptMessage>,      // new: expanded + substituted
    pub unexpanded: Vec<UnexpandedReference>, // new
}

pub struct UnexpandedReference {
    pub token_title: String,
    pub reason: UnexpandedReason, // missing | ambiguous | locked | depth | cycle
}
```

`api.ts` gains the matching mirror type, replacing the wrong `Promise<string>`
(finding 2). Nothing breaks, because nothing calls it today.

The frontend `buildPromptCopyText` becomes a formatter over the returned parts —
it keeps the `[System]` / `[User]` block format and the chat-mode branch
(`promptText.ts:138-157`), and stops doing the substitution the backend now
does. `defaultVariableValues` moves to the call site: the caller passes the
declared defaults as `values`, which reproduces the archived one-click copy
contract exactly (`08-24-prompt-list-copy` R8) while gaining expansion.

**This task also switches the control.** Earlier planning left the UI switch to
"whichever library child lands later", and `08-24-library-views` design D5 said
the opposite — that the control keeps building text locally. Between them, R12
had no owner and would have shipped as backend code no copy button reaches.

Parent assumption A11 resolves it here. There is exactly one copy control,
`CopyPromptButton`, used by the list today and by the overlay after
`08-24-detail-modal`. This task migrates it, in the same diff that corrects the
mirror type and splits the formatter, because those three edits are one change:

1. `CopyPromptButton` calls `api.copyPrompt(id, defaultVariableValues(prompt))`
   instead of `buildPromptCopyText(prompt)`.
2. It formats the returned parts with the split formatter, keeping the
   `[System]` / `[User]` / `[Assistant]` blocks and the chat-mode branch
   byte for byte.
3. Its in-control success and failure feedback is unchanged, including the
   locked-prompt disabled state (`CopyPromptButton.tsx:33,55,86`).

The control is already async — it awaits the clipboard write — so adding one
awaited command does not change its shape. R12 is verified inside this task
(PRD AC4b), not deferred.

`unexpanded` is returned but not yet surfaced in the UI. The references tab in
`08-24-detail-modal` is where a user sees which references did not expand. The
copy control does not report it: a partial expansion still produces correct,
complete text with the literal token left in place, and a warning on every copy
of a prompt with one missing reference would be noise.

Rejected: a separate `reference.expand` command consumed by the pure frontend
builder. It would make a pure function async, split expansion across two call
paths, and still leave `prompt.copy` unused.

### D5 — Deleting a prompt marks its incoming edges missing

`ON DELETE SET NULL` clears `target_prompt_id` but cannot rewrite `resolution`.
`delete` (`services/prompt.rs:701-711`) therefore runs, in the same transaction:

```sql
UPDATE prompt_references SET resolution = 'missing', target_prompt_id = NULL
WHERE target_prompt_id = ?1;
DELETE FROM prompts WHERE id = ?1;   -- CASCADE removes its outgoing edges
```

Outgoing edges of the deleted prompt go with it through `ON DELETE CASCADE`.
Incoming edges survive as `missing`, so the referencing prompt still reads and
its references tab reports a broken reference instead of a silent gap (PRD R8,
AC7).

`batch_delete` (`services/prompt.rs:839-850`) applies the same statement per id
inside its existing transaction.

### D6 — Expansion runs before substitution, to a depth of 3, and reports everything it could not expand

**Order (PRD R5).** Expand references first, then substitute `{{name}}`
placeholders once over the assembled text. The consequence to state plainly: an
expanded body's placeholders are filled from the _calling_ prompt's values, not
from the referenced prompt's own declared defaults. The alternative — substitute
each body with its own defaults before inlining — would make the same reference
produce different text depending on which prompt inlined it, which is harder to
reason about than one substitution pass over the final text.

**Depth (PRD R6).** Three levels of nesting. A reference inside a reference
inside a reference expands; the fourth does not. The limit is a constant with a
name, not a literal, and it is asserted in a test so the number is the contract.

**Cycle.** A target already on the current expansion path is not expanded again.
Detection is an ancestor set, not a global visited set: the same prompt inlined
twice in different branches is legitimate and both expand.

**Uniform failure reporting (PRD R7).** Every token that does not expand —
missing, ambiguous, locked target, depth exceeded, cycle — is left literally in
the text as `@@Title` and appended to `unexpanded` with its reason. No case
produces empty text, and the caller can report all five the same way.

A locked _target_ does not fail the copy; it yields `reason: locked`. A locked
_source_ still returns `UNAUTHORIZED` from `copy_secure`, unchanged (finding 4).

**The persisted edge is the authority, not a second title lookup.** This is what
makes PRD R1 hold at copy time and not only at list time. The signature is:

```rust
fn expand(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    source_prompt_id: &str,
    body: &str,
    ancestors: &mut Vec<String>,   // prompt ids on the current path
    depth: usize,
) -> Result<(String, Vec<UnexpandedReference>), AppError>
```

Both extra parameters are load-bearing, and an earlier draft signature
`expand(conn, body, ancestors, depth)` could not deliver two stated behaviors:

- **`source_prompt_id`.** For each token found in the body, look up this source's
  row in `prompt_references` by `token_title`, and inline `target_prompt_id`
  when `resolution = 'resolved'`. Re-resolving by title instead would break on
  the exact case R1 names: rename target `Foo` to `Bar`, and the source's body
  still reads `@@Foo`. The edge still points at the right id and `token_title`
  is still `Foo`, so the lookup succeeds — while a fresh title query for `Foo`
  would find nothing and report `missing`. Without the source id there is no way
  to reach the edge, and `reference.list` would show the reference resolved while
  copy reported it missing.
- **`encryption`.** A private target's body is ciphertext in the column. To
  inline it the function needs the unlocked key; to report `reason: locked` it
  needs to know the key is absent. Reuse the existing `unlocked_key`
  (`services/prompt.rs:569`) and the `decrypt_*` helpers rather than adding a
  second decryption path.

Recursion passes the target's own id as the next `source_prompt_id`, so each
level reads its own edges. A prompt saved before this change has no edges, so
its tokens report `missing` until it is next saved (see Compatibility).

### D7 — Token syntax

`@@` followed by the title, terminated by end of line or by a second `@@`. Two
forms:

- `@@Title of the prompt@@` — explicit, allows a title containing spaces
  unambiguously.
- `@@Title` to end of line — the shorthand the design concept shows.

The explicit form is scanned first so a line holding both is read
deterministically. `@@` at end of input, or with an empty title, is literal text
and produces no edge.

Recorded in `.trellis/spec/` so the frontend picker inserts the same syntax
(PRD R4, and `08-24-detail-modal` R7).

### D8 — One internal write function takes both the storage input and the plaintext scan

The problem this closes: an earlier draft called
`resolve_and_store(tx, prompt_id, body)` from inside `create`/`update` **and**
from `create_secure`/`update_secure` before encryption. Those two call sites
cannot both work.

- `create_secure` (`services/prompt.rs:554-571`) encrypts `input.description`,
  `system_prompt`, `user_prompt`, and `messages`, and only then calls
  `create(conn, input)`. It holds the plaintext but not the transaction, and not
  the prompt id — `create` generates the uuid at `:244` and opens the
  transaction at `:252`.
- `create` holds both, but for a private prompt every body field it can see is
  ciphertext. Scanning it yields no tokens.

So the plaintext and the transaction never meet. Token extraction over ciphertext
would produce empty edges for every private prompt, and resolving outside the
transaction would break PRD R3's atomic-save requirement.

The seam is one internal function that takes both:

```rust
struct ReferenceScan {
    system_prompt: Option<String>,
    user_prompt: String,
    messages: Vec<PromptMessage>,
}
impl ReferenceScan {
    fn from_create(input: &PromptCreate) -> Self { … }
    fn from_update(patch: &PromptUpdate, existing: &Prompt) -> Self { … }
}

fn create_inner(conn: &Connection, input: PromptCreate, scan: ReferenceScan)
    -> Result<Prompt, AppError>;
fn update_inner(conn: &Connection, id: &str, patch: PromptUpdate, scan: ReferenceScan)
    -> Result<Prompt, AppError>;
```

- `create_inner` opens the transaction, inserts, takes the generated id, calls
  `reference::resolve_and_store(&tx, &id, &scan)` **inside** that transaction,
  appends the version snapshot, and commits. One transaction, as PRD R3 requires.
- `create(conn, input)` becomes `create_inner(conn, input, ReferenceScan::from_create(&input))`.
  Its public signature and behavior are unchanged for a non-private prompt.
- `create_secure` builds the scan from the plaintext **before** encrypting, then
  calls `create_inner(conn, encrypted_input, scan)`. It never needs the id or the
  transaction.

`update` and `update_secure` mirror this. `update`'s scan is built from the patch
merged over `existing` (`services/prompt.rs:339-369`), because a patch that omits
`user_prompt` keeps the stored body, and that body's tokens are still the
prompt's references.

Rejected: passing an `Option<&EncryptionState>` down into `create` so it can
decrypt what it just encrypted. It moves a security concern into the storage
layer and decrypts a value the caller already had in plaintext one frame earlier.

### D9 — Every write path that changes a body, listed

The edge table is derived state. A write path that changes a body without
updating the edges leaves the two permanently inconsistent. `create` and
`update` are not the only such paths: `duplicate` inserts into `prompts`
directly (`services/prompt.rs:717-746`) and `rollback` updates it directly
(`services/version.rs:263-290`). Neither goes through `create` or `update`.

| Path                                    | Body changes                      | Edge action                                                                      | Transaction                                              |
| --------------------------------------- | --------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `create` / `create_secure`              | yes                               | resolve and insert (D8)                                                          | existing create tx                                       |
| `update` / `update_secure`              | yes                               | delete this source's rows, resolve, insert                                       | existing update tx                                       |
| `duplicate` (`:712`)                    | copies a body to a new id         | resolve the copy's body under the new id                                         | existing duplicate tx                                    |
| `rollback` (`version.rs:243`)           | replaces the body from a snapshot | delete and re-resolve for that prompt                                            | existing rollback tx                                     |
| `delete` (`:701`)                       | removes a body                    | mark incoming `missing`; outgoing go by CASCADE (D5)                             | **add** a tx — `delete` runs a bare `conn.execute` today |
| `batch_delete` (`:839-850`)             | removes bodies                    | same statement per id                                                            | existing batch tx                                        |
| `batch_move` / `batch_tag` (`:771-837`) | no body change                    | none                                                                             | —                                                        |
| import `Skip`                           | existing prompt untouched         | drop the bundle's edges for that source; do not touch the existing prompt's rows | existing import tx                                       |
| import `Replace`                        | overwrites a body                 | delete that source's rows, insert the bundle's remapped edges                    | existing import tx                                       |
| import `Duplicate`                      | new ids                           | insert the bundle's edges remapped through `prompt_map`                          | existing import tx                                       |

Two details the table depends on:

- **`duplicate` resolves rather than copying the source's edges.** The copy's
  body is identical, so the same tokens are present, but a token that was
  `ambiguous` for the original is now ambiguous by a different count — the
  duplicate itself may share a title. Resolving is one query per token and is
  always correct; copying is a guess.
- **Edge `id` is always generated locally, never carried from a bundle.** The
  bundle's `PromptReferenceRecord` does not include an `id` field. Importing a
  bundle twice would otherwise collide on the primary key under `Skip` and
  `Replace`, and produce a second row for the same edge under `Duplicate`.

`Skip` explicitly does not modify an existing prompt's edges. Skip means the
incoming prompt was not applied; writing its edges over the resident prompt's
would change a prompt the policy says to leave alone.

## Data flow

```
create_secure / update_secure
        │  builds ReferenceScan from the PLAINTEXT body        (D8)
        │  then encrypts the input
        ↓
create_inner / update_inner  (also the entry for plain create / update)
        │  opens the transaction, generates or reads the id
        ↓
  reference::resolve_and_store(&tx, id, &scan)
        │
        ├─ extract_tokens(scan)  ──→ resolve each against prompts.title
        │                              0 matches → missing
        │                              1 match   → resolved
        │                              2+        → ambiguous
        ↓
  replace this prompt's rows in prompt_references   (same transaction)

duplicate / rollback / delete / batch_delete / import      (D9)
        └─ each updates the edge table inside its own existing transaction

prompt.copy
        │
        ↓
  expand(conn, encryption, source_id, body, ancestors, depth ≤ 3)      (D6)
        │  per token: look up THIS source's edge row by token_title
        │             resolved → inline the target body (decrypt if private
        │                        and unlocked, else reason: locked)
        │  recurse with the target id as the next source_id
        ↓
  substitute_placeholders(assembled, values)
        ↓
  PromptCopy { systemPrompt, userPrompt, messages, unexpanded }

reference.list(promptId)
        ↓
  outgoing: rows WHERE source_prompt_id = ?1
            (LEFT JOIN prompts for the live target title)
  incoming: rows WHERE target_prompt_id = ?1 AND resolution = 'resolved'
            (JOIN prompts on source_prompt_id for the source id and title)
```

## Command surface

| Command          | Args       | Returns                                |
| ---------------- | ---------- | -------------------------------------- |
| `reference.list` | `promptId` | `{ outgoing: [...], incoming: [...] }` |

One new command. Edge writes happen inside the prompt write paths (D9); they are
not separately invocable, so a reference cannot be created that does not
correspond to a token in a body.

**The two directions carry different identities.** One shared entry shape does
not work here. An incoming row is selected by `target_prompt_id = ?`, so its
`target_prompt_id` is the prompt being asked about — returning it as
`targetPromptId` tells the caller only what it already passed in, and the
references tab could neither name nor navigate to the prompt doing the
referencing. That is half the tab's function.

```ts
type OutgoingReference = {
  targetPromptId: string | null; // null when unresolved
  targetTitle: string | null; // the target's live title; null when unresolved
  tokenTitle: string; // the literal text in the body
  resolution: "resolved" | "missing" | "ambiguous";
};

type IncomingReference = {
  sourcePromptId: string; // never null; the edge cascades with its source
  sourceTitle: string; // read live from the source row
  tokenTitle: string; // the token that produced this edge
  resolution: "resolved" | "missing" | "ambiguous";
};
```

`incoming` selects `WHERE target_prompt_id = ?1 AND resolution = 'resolved'` and
joins `prompts` on `source_prompt_id` for the title. An unresolved edge has no
resolved target, so it cannot appear as anyone's incoming reference; it appears
only in its own source's `outgoing`.

R1's "renaming a target must not break an edge" holds because the edge keys on
the id and both titles are read live at list time.

Rejected: one `counterpartPromptId` / `direction` shape. It types the two
directions as interchangeable when they are not — `targetPromptId` is nullable
and `sourcePromptId` is not — and the tab renders two visually distinct lists
anyway.

Registration follows the existing convention: `domain.action`, `CommandResult<T>`
envelope, registered in `invoke_handler!` (`src-tauri/src/lib.rs`), reachable
only through `src/features/prompts/api.ts` (PRD R9).

## Error mapping (PRD R10)

| Case                                                | Code                                 |
| --------------------------------------------------- | ------------------------------------ |
| source prompt not found                             | `NOT_FOUND`                          |
| locked source on copy                               | `UNAUTHORIZED` (unchanged)           |
| malformed token                                     | none — literal text, no error        |
| missing / ambiguous / locked target / depth / cycle | none — reported in `unexpanded`      |
| database failure                                    | `INTERNAL` via the existing `db_err` |

No new `ErrorCode` variant. Reference resolution failures are data, not errors:
a user saving a prompt that mentions a prompt they have not written yet is doing
something normal.

## Compatibility

- Migration 6 is additive: one `CREATE TABLE`, two `CREATE INDEX`. It runs on an
  existing database without touching a row (PRD AC1).
- `CURRENT_SCHEMA_VERSION` goes 5 → 6, and the same DDL is added to `SCHEMA_SQL`
  so fresh installs get the table (finding 5).
- `PromptCopy` gains two fields. Serde adds them to the JSON; no existing caller
  reads them, because there is no existing caller.
- Bundle format version is unchanged (D3). Bundles round-trip both directions
  between this build and older builds; older builds simply lose the edges.
- A prompt saved before this change has no rows in `prompt_references` until it
  is next saved. Its references tab shows empty. A one-time backfill is not run:
  a private prompt's body is unreadable without the key, so a backfill would be
  correct for public prompts and silently skip private ones. Saving the prompt
  once resolves it. Record this in the task's user-visible notes.

## Test impact

- `src-tauri/src/storage/mod.rs` tests — migration 6 applies to a v5 database and
  `user_version` becomes exactly 6.
- New Rust tests: token extraction (both forms, malformed, empty), resolution
  (0 / 1 / 2+ matches), rename keeps the edge, delete marks incoming missing,
  expansion order, depth limit, cycle `A → B → A`, locked target.
- `services/portable.rs` tests — edges survive export and import under all three
  conflict policies, and a v2 bundle without the field imports cleanly.
- `src/features/prompts/api.test.ts` — the corrected `copyPrompt` mirror type.
- `src/features/prompts/promptText.test.ts` — the formatter split; the chat-mode
  block format is unchanged.

## Rollback

The table is additive and nothing reads it unless the reference code runs.
Reverting means dropping the `reference.list` command, the resolve call in
create/update, and the expansion in `copy`; the table can stay with stale rows
and the schema version stays at 6. A schema version does not go backwards, so a
revert must not remove migration 6.
