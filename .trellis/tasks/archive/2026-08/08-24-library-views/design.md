# Design — grid and list library views

## What the model supplies, and what the design asks for

The design's card and row show: 置顶 badge, 草稿 badge, title, description, up to
three tags, type, usage count, last update, version, favorite toggle, copy
control, and a batch checkbox.

`Prompt` (`src/features/prompts/types.ts:62-87`) supplies `isPinned`,
`title`, `description`, `tags`, `promptType`, `typeDefinitionId`, `usageCount`,
`updatedAt`, `currentVersion`, `isFavorite`, `isPrivate`, `isLocked`. Every field
the design shows is present except one.

**There is no draft state.** No `is_draft` column
(`src-tauri/src/storage/mod.rs:451-475`), no field on the model, and no service
that sets one. The design's 草稿 badge has no source.

Two further facts constrain the rendering:

1. **`currentVersion` starts at 0.** The column defaults to `0`
   (`storage/mod.rs:468`) and `version::append_snapshot` advances it. A prompt
   that has never been saved past creation shows `v0`.
2. **A locked prompt has a redacted body but intact metadata.**
   `present_prompt` (`services/prompt.rs:499-524`) redacts the body and leaves
   `title`, `tags`, `folder_id`, `is_favorite`, and `is_pinned` untouched. So a
   locked row can show title, tags, and counts truthfully, and must show nothing
   derived from the body.

## Decisions

### D1 — One projection, two renderers

PRD R1 requires that adding a field change one mapping, not two components.

```
promptStore.prompts: Prompt[]
        │
        ↓  toLibraryItem(prompt, promptTypeDefinitions, t)
LibraryItem { id, title, description, tags, overflowTagCount,
              typeLabel, typeIcon, usageCount, updatedLabel,
              versionLabel, isFavorite, isPinned, isPrivate, isLocked }
        │
        ├──→ PromptGrid  (cards)
        └──→ PromptList  (rows)
```

`toLibraryItem` is a pure function in a new
`src/features/prompts/libraryItem.ts`, unit-tested without React. It resolves the
custom type name from `promptTypeDefinitions` — the lookup `PromptList.tsx:84-88`
does inline today — and it decides the redaction, so neither renderer can leak a
body-derived string by omission.

Rejected: a shared row component parameterized by mode. The grid card and the
table row have different element structure (a card is a region with a footer, a
row is a set of fixed columns); one component with a mode branch would carry both
structures and be harder to read than two.

### D2 — The 草稿 badge is dropped

No backing field (above). The options:

- **Drop it.** Chosen.
- **Derive it from `currentVersion === 0`.** Wrong meaning. Version 0 means
  "never snapshotted", which is not what a reader understands by 草稿. A prompt
  created and used for a year without an explicit version is not a draft.
- **Add an `is_draft` column.** A backend contract change; the parent scope
  allows one, and it is spent on prompt-to-prompt references.

The 置顶 badge stays. It has a real field, and `PromptsView.tsx:498-528` already
exposes a pin toggle. Recorded from `08-24-shell-sidebar` design D3: `is_pinned`
still does not affect ordering or filtering, so a pinned prompt does not float to
the top of the grid. The badge reports state; it does not promise ordering.

### D3 — Both modes show the same fields; the grid adds a description

| Field            | Grid card           | List row            | Note                                       |
| ---------------- | ------------------- | ------------------- | ------------------------------------------ |
| batch checkbox   | yes                 | yes                 | only while `batchMode` (toolbar design D4) |
| 置顶 badge       | yes                 | yes                 |                                            |
| title            | yes                 | yes                 |                                            |
| description      | 2-line clamp        | 1-line truncate     | column in list                             |
| tags             | up to 3 + remainder | up to 3 + remainder |                                            |
| type             | footer              | column              | icon plus custom-type name                 |
| usage count      | footer              | column              |                                            |
| last update      | footer              | column              |                                            |
| version          | footer              | column              | `v{currentVersion}`                        |
| favorite         | top right toggle    | column toggle       |                                            |
| copy             | footer              | row end             |                                            |
| private / locked | icon                | icon                |                                            |

Nothing is in one mode only, which satisfies PRD AC2 without an omission list.
`PromptList` today shows up to four tags (`PromptList.tsx:147`); the design says
three. Three in both, with the remainder indicated, so the two modes agree.

### D4 — A locked prompt shows metadata and no body-derived text

`toLibraryItem` sets `description` to the locked notice when `isLocked` is true,
matching what `PromptList.tsx:140-144` does today, and keeps title, tags, type,
counts, and version, which are not encrypted (finding 2).

The copy control stays visible and disabled with a message stating that the
library must be unlocked. That is `CopyPromptButton`'s existing `locked` prop
(`CopyPromptButton.tsx:33,55,86`) and the archived `08-24-prompt-list-copy` R5
contract. No change.

### D5 — The copy control is migrated by `08-24-prompt-references`, not here

`CopyPromptButton` calls `buildPromptCopyText(source)`
(`CopyPromptButton.tsx:70`), a pure frontend function
(`promptText.ts:138-157`) that substitutes declared `defaultValue`s and leaves
unmatched placeholders in place. It does not call `prompt.copy`.

Earlier planning said this task keeps that arrangement, and
`08-24-prompt-references` said it does not touch copy controls. Both together
left parent R12 with no owner: reference expansion would ship as backend code
that no copy button reaches.

Resolved in the parent as assumption A11. `08-24-prompt-references` owns the
migration of `CopyPromptButton` to `prompt.copy`, in the same diff that corrects
the `api.ts` mirror type and splits `buildPromptCopyText` into a formatter. That
child lands before this one.

This task therefore mounts the already-migrated control and changes nothing
about how it obtains text. It verifies one thing: that a prompt whose body holds
`@@Title` copies from a library item with the target's body inlined (parent CC8).
The locked-prompt contract is unchanged — the control stays visible and disabled
with its message.

### D6 — Interactive children are buttons inside the item, not nested in its activator

PRD R6 requires that the checkbox, favorite toggle, and copy control not open the
prompt. `PromptList` solves this today by making the activator a sibling `div
role="button"` rather than wrapping the row
(`PromptList.tsx:98-115`), and `CopyPromptButton` also calls `stopPropagation`
(`CopyPromptButton.tsx:87-91`).

Both modes keep that shape: the item is a container, the title area is the
activator, and each control is a sibling button. A card is not one big `<button>`
with buttons inside it, which is invalid and would make the inner controls
unreachable by keyboard in some browsers.

### D7 — The grid reflows; the list truncates

Grid: `repeat(auto-fill, minmax(272px, 1fr))` with a 16px gap. The design
specifies 372px, which at the app's minimum usable width leaves one column with
horizontal overflow in the shell after the 264px sidebar. 272px keeps two columns
at typical widths and one column without a horizontal scrollbar at the minimum
(PRD R9). Verify the number against the real minimum in implement step 7 rather
than trusting it here.

List: a CSS grid with fixed column tracks and `min-width: 0` plus `truncate` on
every cell, so a long title shortens instead of pushing the row wide. The
container scrolls vertically only.

Neither mode virtualizes. Paging stays the mechanism for large libraries (PRD
out of scope), and the page is 50 items (`promptStore.ts:58`).

### D8 — Switching modes preserves selection and filters by construction

Selection lives in `selectedPromptIds` and filters in `filters`, both on the
store (`promptStore.ts:88,93`). `viewMode` is a third store field owned by
`08-24-library-toolbar` (that task's design D8). Switching it re-renders the
items from the same state, so PRD AC1's selection and filter preservation needs
no extra work — it needs a test that proves it.

Scroll position is not preserved across a mode switch. The two modes have
different item heights, so a preserved pixel offset would land on a different
item. The switch scrolls to the top instead, which is predictable.

## Interaction with `08-24-library-toolbar`

That task moves the loading and empty branches out of `PromptList` into the
library container (its design D7). This task therefore receives a `PromptList`
that renders items only, and adds a `PromptGrid` with the same contract.

`08-24-library-toolbar` owns the split unconditionally, per the parent's
`research/shared-ownership.md`. This task waits for it and does not perform the
split even if it starts first. Doing it in both produces a conflict in
`PromptsView.tsx` and `PromptList.test.tsx`, and "whichever starts first" is not
an owner — it is a race.

The same applies to `viewMode`: `08-24-library-toolbar` owns that store field
(its design D8). This task reads it.

## Accessibility

- Each item's activator carries the title as its accessible name and
  `aria-current` when it is the open prompt, matching `PromptList.tsx:107-109`.
- The checkbox carries its own label naming the prompt
  (`PromptList.tsx:95`), so selection is conveyed by the control's checked state,
  not by the card's border (PRD R5).
- The 置顶 badge has a text label, not a bare icon.
- The list header row is a real header: the grid container uses `role="table"`
  semantics or a `<table>`. Decide in implement step 4 and keep column headers
  associated with cells either way.
- Enter opens the focused item in both modes; the existing
  `activateSelect` helper (`PromptList.tsx:29-38`) already handles Enter and
  Space.

## Test impact

- New: `src/features/prompts/libraryItem.test.ts` — the projection, including
  tag overflow, locked redaction, and custom type resolution.
- New: `src/features/prompts/components/PromptGrid.test.tsx`.
- `PromptList.test.tsx` — loses loading and empty cases to the container, gains
  the fixed-column assertions.
- `CopyPromptButton.test.tsx` — unchanged; the control is reused, not rewritten.
- `PromptsView.layout.test.tsx` — covers the mode switch preserving selection.

## Rollback

`PromptGrid` is additive. Reverting means forcing `viewMode` to `"list"` and
dropping the grid; `PromptList` keeps working. The projection module is shared,
so reverting the grid alone leaves `toLibraryItem` in place with one consumer,
which is harmless.
