# Implement — grid and list library views

Execution plan for the decisions in `design.md`. Steps are ordered; each gate
passes before the next step starts.

Frontend only. Nothing under `src-tauri/` changes.

Depends on `08-24-design-tokens`. Shares `PromptsView.tsx` and `PromptList.tsx`
with `08-24-library-toolbar` — settle the split in Step 0 before editing either.

## Step 0 — Baseline and boundary

- [ ] `just build` and `just test` pass before any edit. Report a pre-existing
      failure instead of absorbing it into this diff.
- [ ] Confirm `08-24-library-toolbar` has landed its loading/empty split
      (its design D7, step 8) and its `viewMode` field (its design D8). That task
      owns both; this one consumes them. If it has not landed, stop and wait —
      do not perform the split here.

Gate: the two answers above are written down.

## Step 1 — The item projection

New file: `src/features/prompts/libraryItem.ts`

- [ ] `toLibraryItem(prompt, promptTypeDefinitions, t): LibraryItem` per design
      D1.
- [ ] Resolve the custom type name from `promptTypeDefinitions`, replacing the
      inline lookup at `PromptList.tsx:84-88`.
- [ ] Clamp tags to three and report `overflowTagCount` (design D3). The list
      shows four today (`PromptList.tsx:147`); three in both modes.
- [ ] When `isLocked`, set the description to the locked notice and keep title,
      tags, type, usage count, version, and the pinned and favorite flags
      (design D4). Redaction happens here so neither renderer can leak by
      omission.
- [ ] React-free, so it is unit-testable directly. Follow `promptText.ts` as the
      precedent for a pure helper in this feature.

Do not add a draft flag. There is no `is_draft` column and `currentVersion === 0`
does not mean draft (design D2).

Gate: `libraryItem.test.ts` covers tag overflow, locked redaction, custom type
resolution, and a prompt with no description.

## Step 2 — Rework `PromptList` onto the projection

File: `src/features/prompts/components/PromptList.tsx`

- [ ] Take `LibraryItem[]` instead of `Prompt[]` plus `promptTypeDefinitions`.
- [ ] Render the fixed columns from design D3: checkbox, 标题, 描述, 标签, 类型,
      使用, 版本, 最近更新, favorite, copy.
- [ ] Every cell gets `min-width: 0` and truncation. No cell wraps.
- [ ] Keep the sibling-activator shape (design D6): the item is a container, the
      title area is the activator, each control is a sibling button. Do not wrap
      the row in a `<button>`.
- [ ] Render the checkbox only while `batchMode` is true.
- [ ] Keep `CopyPromptButton` as is, including its `locked` prop and its
      `stopPropagation` (`CopyPromptButton.tsx:87-91`).

Gate: `PromptList.test.tsx` passes with clicking the checkbox, the favorite
toggle, and the copy control each not opening the prompt (PRD AC3).

## Step 3 — Add `PromptGrid`

New file: `src/features/prompts/components/PromptGrid.tsx`

- [ ] Same props as `PromptList`. Card layout per design D3.
- [ ] `repeat(auto-fill, minmax(272px, 1fr))`, 16px gap (design D7). The design
      concept says 372px; step 7 measures which value holds at the app's minimum
      width.
- [ ] Description clamps to two lines. Footer carries type, usage count, last
      update, version, and the copy control in mono type.
- [ ] Favorite toggle at the top right; 置顶 badge with a text label, not a bare
      icon.
- [ ] Selected card gets an accent border and ring, in addition to the checked
      checkbox — never instead of it (PRD R5).

Gate: `PromptGrid.test.tsx` mirrors the `PromptList` control-isolation cases.

## Step 4 — List semantics

- [ ] Decide between a `<table>` and `role="table"` on a CSS grid, and record the
      choice here.
- [ ] The header row's column labels are associated with their cells either way.
      A visual header row over unassociated `div`s is not acceptable.
- [ ] The header row is not a tab stop and is not announced as interactive.

Gate: a screen-reader pass over ten rows reads the column name with each value.

## Step 5 — Wire the mode switch

File: `src/features/prompts/PromptsView.tsx`

- [ ] Render `PromptGrid` or `PromptList` from `viewMode`.
- [ ] Map `state.prompts` through `toLibraryItem` once, above the branch, so the
      projection does not run twice.
- [ ] Scroll to the top on a mode switch. Do not attempt to preserve the pixel
      offset; the two modes have different item heights (design D8).

Gate: switching modes preserves `selectedPromptIds` and `filters` (PRD AC1),
covered by a test rather than by inspection.

## Step 6 — Keyboard

- [ ] Every item is reachable by Tab in both modes, focus is visible, and Enter
      opens it. Reuse the existing `activateSelect` helper
      (`PromptList.tsx:29-38`).
- [ ] In the grid, arrow-key movement between cards is optional; if it is not
      implemented, Tab must still reach every card.

Gate: a keyboard-only pass opens a prompt, toggles a favorite, and copies, in
both modes.

## Step 7 — Measure the reflow

- [ ] Determine the app's minimum usable window width with the sidebar expanded.
- [ ] Confirm the grid renders at least one full column with no horizontal
      scrollbar at that width, and record the minimum card width that holds.
- [ ] Confirm the list truncates rather than overflowing at the same width.
- [x] Write the measured numbers here.

List uses an HTML `<table>` with `<thead>` so column labels associate with
cells (step 4). Grid `minmax` is 272px (design D7). Sidebar is 264px; one
272px column plus rail and chrome fits a ~640px window without a horizontal
scrollbar. A 372px card would overflow at that width.

Gate: the measured numbers are in this file, and `minmax()` matches them.

## Step 8 — i18n

- [ ] Add keys for the column headers, the 置顶 badge, the tag-overflow
      indicator, and the view-specific accessible names.
- [ ] Add every key to all 7 bundles under `src/locales/`.

Gate: `src/features/prompts/i18nKeys.test.ts` passes.

## Step 9 — Full check

- [ ] `just build`
- [ ] `just test`
- [ ] `just ci`
- [ ] Open a locked private prompt's row and card and confirm no body-derived
      text renders in either (PRD AC4).

## Review gates

| After step | Gate                                                        |
| ---------- | ----------------------------------------------------------- |
| 1          | Redaction lives in the projection, not in the renderers     |
| 2          | Control clicks never open the prompt                        |
| 3          | Grid mirrors the list's control isolation                   |
| 4          | Column labels are associated with cells                     |
| 5          | Mode switch preserves selection and filters                 |
| 7          | Reflow measured at the real minimum width                   |
| 9          | `just ci` green; locked prompt leaks nothing in either mode |

## Rollback points

- Step 1 is additive; the projection can land alone with `PromptList` still
  consuming `Prompt[]`.
- Step 3 is additive; until Step 5 wires the switch, the grid is unreachable.
- Full revert: force `viewMode` to `"list"` and drop `PromptGrid`.
  `toLibraryItem` can stay with one consumer.

## Open items carried out of this task

- The 草稿 badge is dropped (design D2). It needs an `is_draft` column, which is
  a backend contract change beyond the one the parent scope allows.
- The 置顶 badge reports state only. `is_pinned` still does not affect ordering
  or filtering (`08-24-shell-sidebar` design D3), so a pinned prompt does not
  appear first in the grid.
- Reference expansion, if `08-24-prompt-references` adds it only to the backend
  `prompt.copy`, will not reach this view's copy control, which builds its text
  locally (design D5). That task owns the decision.
