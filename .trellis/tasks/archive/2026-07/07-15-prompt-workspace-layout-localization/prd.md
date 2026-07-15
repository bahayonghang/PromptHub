# Refactor prompt workspace layout and localization

## Goal

Give the Prompts workbench a stable information hierarchy and responsive desktop
layout while eliminating mixed-language rendering and preserving every existing
workflow.

## Confirmed Defects

- `PromptsView` always reserves 224 px and 320 px navigation panes even at the
  supported 800 px minimum (`PromptsView.tsx:123`, `PromptsView.tsx:139`).
- `PromptEditor` is one long flat field sequence inside one scroll region;
  organization and preview grids are fixed at two columns
  (`PromptEditor.tsx:242`, `PromptEditor.tsx:278`, `PromptEditor.tsx:495`).
- Simplified Chinese supplies only a subset of the English `promptsView` keys.
  The current test checks every key only in English and checks a short operation
  subset in other locales (`i18nKeys.test.ts:179-202`).
- Header actions, history, and evaluation mode can compete for the remaining
  detail width; no breakpoint or container rule defines which pane yields first.

## Requirements

- R1: Define wide, default, constrained, and 800 px minimum workspace states
  that preserve folder/list/editor selection and draft state.
- R2: Keep the active task primary: editor/evaluation content receives remaining
  width through `minmax(0, 1fr)` and secondary panes collapse rather than squeeze.
- R3: Reorganize the editor into clear unframed sections with responsive field
  groups and one predictable scroll owner/action footer.
- R4: Standardize toolbar and form state styling using existing semantic tokens,
  typography, radius, spacing, and Lucide conventions from `DESIGN.md`.
- R5: Fill every rendered Prompts key in `en`, `zh`, `zh-TW`, `ja`, `fr`, `de`,
  and `es`; update tests to reject missing/empty per-locale keys.
- R6: Preserve all existing prompt, folder, tag, batch, private, revision,
  portable, and evaluation behavior. This child must not change persistence or
  command contracts.
- R7: Make all states keyboard reachable and retain labels, focus visibility,
  non-color selection cues, disabled/loading/error behavior, and reduced-motion
  compatibility.

## Acceptance Criteria

- [x] AC1: Browser screenshots at 800x600, 1200x800, and 1600x900 show coherent
  pane states with no overlap, clipping, or unusable editor width.
- [x] AC2: 200% text scaling works in English, Simplified Chinese, German, and
  Japanese without horizontal page scrolling or covered actions.
- [x] AC3: Switching folder/list/detail/history/evaluation states preserves the
  selected prompt, filters, and unsaved draft.
- [x] AC4: Editor sections are scanable, use no nested cards, and stack fields
  when the content region cannot support two columns.
- [x] AC5: Every key rendered by the Prompts feature is present and non-empty in
  all seven bundles; Chinese visual smoke contains no English fallback.
- [x] AC6: Existing feature/store tests remain green and new layout/locale tests
  cover the structural states and complete bundle parity.
- [x] AC7: `just build` and `just test` pass.

## Out of Scope

- Reworking filter internals, inline folder creation, or custom type persistence;
  those belong to sibling tasks.
- Resizable splitters, saved pane sizes, a mobile app layout, or a new design
  system/component framework.
- Domain, database, Runtime Bridge, or backend changes.
