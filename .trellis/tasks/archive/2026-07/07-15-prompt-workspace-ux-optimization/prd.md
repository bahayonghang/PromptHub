# Optimize prompt workspace UX and organization

## Goal

Make the Prompts management and editing workspace compact, legible, localized,
and efficient across the supported desktop window range while allowing users to
create the organizational values they need without leaving the editor.

## Background

- The supplied screenshots show the Prompts workspace, not the global Settings
  route. This task treats "the whole settings page" as the prompt configuration
  and editing surface shown in those screenshots.
- The search column is fixed at `w-80`, while the expanded filter row uses
  `ml-auto` plus two selects. The controls exceed the available inline space and
  wrap the label at `src/features/prompts/components/SearchBar.tsx:92`.
- The shell always reserves `w-56` and `w-80` panes even though the Tauri window
  is resizable down to 800 px (`src/features/prompts/PromptsView.tsx:123`,
  `src/features/prompts/PromptsView.tsx:139`, `src-tauri/tauri.conf.json:18`).
- Folder creation already exists through the tree, store, Runtime Bridge, and
  backend service. The editor exposes only a select, so creating a folder causes
  a context switch (`PromptEditor.tsx:300`, `promptStore.ts:451`).
- `PromptType` is a structural enum and database constraint, not a free-form
  label (`types.ts:10`, `models/enums.rs:13`, `storage/mod.rs:427`). Arbitrary
  values cannot be accepted without a separate compatibility-safe contract.
- The English Prompt bundle is complete, but non-English bundles cover only a
  subset of rendered keys. English fallback therefore produces mixed-language
  pages (`i18nKeys.test.ts:156`, `runtime/i18n.ts:107`).

## Requirements

- R1: Repair the filter interaction so all controls remain readable and usable
  in the prompt-list pane for every supported locale and font scale.
- R2: Refactor the full Prompts workspace hierarchy and responsive behavior
  without removing current prompt, folder, version, batch, transfer, private,
  or evaluation workflows.
- R3: Let a user create a folder from the prompt editor and select it immediately
  after the backend confirms creation.
- R4: Let a user create a named prompt type without weakening the stable
  `text`/`image`/`video` base-format invariant used by execution and storage.
- R5: Complete every rendered Prompts key in all seven locale bundles and make
  locale completeness a regression gate instead of relying on English fallback.
- R6: Meet WCAG 2.2 AA keyboard, focus, semantics, text-scaling, and status
  requirements. No workflow may rely on hover or color alone.
- R7: Preserve the repository design system: semantic tokens, system interface
  type, compact desktop density, Lucide icons, flat tonal surfaces, and no
  nested-card or dashboard-style redesign.
- R8: Keep frontend/backend access through the existing feature API, Zustand
  store, Runtime Bridge, `CommandResult<T>`, migration, revision, portable
  bundle, and evaluation boundaries.

## Acceptance Criteria

- [ ] AC1: At 800x600, 1200x800, and a wide desktop viewport, the folder, list,
  filter, editor, history, and evaluation states have no overlap, clipping, or
  incoherent horizontal scrolling.
- [ ] AC2: The Prompts workspace remains usable at 200% text scaling and with
  long German, French, Chinese, and Japanese labels.
- [ ] AC3: Opening, changing, clearing, and closing filters is keyboard operable,
  announces its disclosure state, and does not cause the sort controls to wrap
  or escape their surface.
- [ ] AC4: Folder creation from the editor handles submit, cancel, validation,
  loading, success, and failure; success selects the returned folder without
  discarding the prompt draft.
- [ ] AC5: Custom type creation records a user-facing name plus a stable base
  format, and round-trips through prompts, revisions, diffs, portable bundles,
  import conflicts, and evaluation without changing legacy prompt behavior.
- [ ] AC6: Every rendered Prompts key is non-empty in every supported locale;
  screenshots in Simplified Chinese contain no accidental English fallback.
- [ ] AC7: Focus order, labels, pressed/expanded states, error messages, and
  compact control target sizes pass component and browser accessibility checks.
- [ ] AC8: Each child passes its targeted checks; the final integration review
  passes `just ci` plus native smoke coverage of create, edit, filter, version,
  import/export, private lock, and evaluation flows.

## Task Map

1. `07-15-prompt-workspace-layout-localization`: restructure the workbench and
   complete locale/accessibility coverage without changing domain behavior.
2. `07-15-prompt-filter-panel`: replace the cramped disclosure with a stable,
   tested filter interaction.
3. `07-15-prompt-inline-folder-create`: reuse the existing folder domain from
   inside the editor and select successful creations.
4. `07-15-prompt-type-definitions`: add persisted custom type definitions while
   keeping the system base format bounded.

## Out of Scope

- Redesigning the global Settings route or unrelated application views.
- Editing `ref/PromptHub/**`, adding CLI/Web variants, or changing product scope.
- Adding type-specific plugins, arbitrary execution handlers, icons, colors,
  type deletion/renaming, or a general taxonomy administration screen.
- Replacing Tailwind, Zustand, i18next, or the Runtime Bridge.
- Implementing any child before the user reviews and starts that child.

## Planning Decisions

- D1: Confirmed by the product owner on 2026-07-15: user-created types are
  organizational definitions with a required `text`, `image`, or `video` base
  format; they are not new executable prompt modalities.
- D2: Inline folder creation creates a root folder. Nesting, reordering, rename,
  and delete remain in the folder tree.
- D3: The redesign is structural and restrained. It may split oversized
  components when ownership becomes clearer, but it does not create a new
  generic component framework.
- D4: Start and archive children independently; keep the parent in `planning`
  until the final cross-child review is complete.
