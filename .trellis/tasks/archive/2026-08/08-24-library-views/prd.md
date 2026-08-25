# Grid and list library views

Child of `08-24-ui-refactor`. Owns parent requirement R6.

## Goal

Render the prompt library in two interchangeable modes — a card grid and a dense
row list — matching the design concept.

## Ordering

Lands after `08-24-design-tokens`. It shares the view-mode flag with
`08-24-library-toolbar`; that child owns the toggle control, this child owns the
rendering.

## Background

- `src/features/prompts/components/PromptList.tsx` (5.9K) renders one vertical
  list. There is no grid mode.
- The list row is a checkbox plus a full-card `<button>`; a copy control was
  added by the archived task `08-24-prompt-list-copy`
  (`src/features/prompts/components/CopyPromptButton.tsx`).
- A locked private prompt must not expose redacted preview text; its copy
  control stays visible and disabled (archived task
  `08-24-prompt-list-copy`, R5).
- The `Prompt` model supplies every field the design shows:
  `title`, `description`, `prompt_type`, `tags`, `is_favorite`, `is_pinned`,
  `usage_count`, `current_version`, `updated_at`
  (`src-tauri/src/models/prompt.rs:33-83`).

## Design target

Grid: `repeat(auto-fill, minmax(372px, 1fr))`, 16px gap. Each card holds an
optional batch checkbox, 置顶 and 草稿 badges, the title, a two-line clamped
description, up to three tag chips, and a footer line in mono type carrying
type, usage count, last update, version, and a copy control. A favorite toggle
sits at the top right. Selected cards get an accent border and ring.

List: a fixed-column grid with a header row — checkbox, 标题, 描述, 标签, 使用,
版本, 最近使用 — and one row per prompt. Selected rows get an accent fill.

## Requirements

- R1: Both modes render from the same item projection. Adding a field means
  changing one mapping, not two components.
- R2: The grid card shows title, description, tags, type, usage count, last
  update, and version. Description clamps to two lines. Overflowing tags are
  truncated to three with the remainder indicated.
- R3: The list row shows the same fields in fixed columns, each truncated rather
  than wrapped.
- R4 (amended by `design.md` D2): Both modes render the pinned state, the
  favorite toggle, and the batch checkbox. The 草稿 badge is dropped: there is no
  `is_draft` column (`src-tauri/src/storage/mod.rs:451-475`), no field on the
  model, and no service that sets one. `currentVersion === 0` means "never
  snapshotted", not "draft", so deriving the badge from it would report a
  different fact under the design's label.
- R4b: The batch checkbox renders only while batch mode is active
  (`08-24-library-toolbar` design D4).
- R4c: The 置顶 badge reports state only. `is_pinned` does not affect ordering
  (`08-24-shell-sidebar` design D3), so a pinned prompt does not appear first.
- R5: Selection state is conveyed by more than color: the checkbox itself
  carries the checked state and an accessible label.
- R6: Clicking a card or a row opens the prompt. Clicking the checkbox, the
  favorite toggle, or the copy control does not open it.
- R7: A locked private prompt shows no body-derived preview text. Its copy
  control stays visible, disabled, and states that the library must be
  unlocked.
- R8: The copy control keeps the archived one-click copy contract: no variable
  modal, `defaultValue` substitution, unmatched placeholders left in place, and
  in-control success and failure feedback.
- R9: The grid reflows down to the app's minimum usable width without a
  horizontal scrollbar. The list keeps its columns readable by truncating.
- R10: Loading and empty states do not render a copy control.
- R11: All labels come from i18n keys, present in all 7 bundles.

## Acceptance criteria

Each criterion names the requirement it closes.

- [ ] AC1 (R1, R5): Switching grid and list preserves the selection and the
      active filters.
- [ ] AC1b (R1, design D8): Switching modes scrolls the list container to the
      top. The scroll offset is not preserved. The two modes have different item
      heights, so a preserved pixel offset lands on a different item; scrolling
      to top is the deterministic behavior and the one the test asserts. AC1
      previously said "preserves the scroll position intent", which named no
      testable outcome and contradicted the design.
- [ ] AC2 (R2, R3): Every field visible in one mode is present in the other or is
      deliberately omitted with a reason recorded in `design.md` D3.
- [ ] AC3 (R6): Clicking the checkbox, favorite, or copy control never opens the
      prompt; a row-level test covers each.
- [ ] AC4 (R7): A locked private prompt renders no preview text in either mode.
- [ ] AC5 (R8, R9): Both modes are keyboard operable: each item is reachable,
      focus is visible, and Enter opens it.
- [ ] AC5b (R6, parent CC11): Opening an item while the detail overlay holds
      unsaved edits routes through `08-24-detail-modal`'s guarded navigation
      rather than calling `selectPrompt` directly.
- [ ] AC6 (R2, R8): `PromptList.test.tsx` and `CopyPromptButton.test.tsx` pass or
      are extended to cover the grid.
- [ ] AC7: `just build` and `just test` pass.

## Out of scope

- The toolbar and its view toggle (`08-24-library-toolbar`).
- Virtualized rendering. Paging stays the mechanism for large libraries.
- Drag-and-drop reordering between folders.
