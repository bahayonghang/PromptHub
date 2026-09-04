# Implement — prompt detail content-tab workbench

## Checklist

1. Add workspace / pane / area rules in `src/styles/globals.css`.
2. Add area class names on Identity, Definition, Organization, Media section roots.
3. Replace the `max-w-[68ch] mx-auto` stack in `PromptDetailModal.tsx` with workspace + meta pane + body pane.
4. Add layout assertions to `PromptDetailModal.test.tsx` (no 68ch; definition in body pane; identity/org/attachments in meta pane; existing copy-button test still two controls).
5. Run `npx vitest run src/features/prompts/components/detail/PromptDetailModal.test.tsx src/features/prompts/components/PromptEditor.test.tsx src/theme/style-conventions.test.ts`.
6. Run `just build` and `just test`.

## Validation

- AC1: grep / test that `PromptDetailModal.tsx` has no `max-w-[68ch]`.
- AC2–AC4: markup + CSS grid areas; jsdom cannot evaluate container queries, so AC4 is the default (narrow) grid-template-areas in CSS plus classes in the DOM.
- AC5: existing modal tests.
- AC6: `just build`, `just test`.

## Risky files

- `src/styles/globals.css` — do not break `.prompt-editor__body` used by `PromptEditor.tsx`.
- `DefinitionSection.tsx` — class merge only; keep copy-button DOM for the existing test.

## Rollback

Git revert the files in the checklist. No schema or i18n migration.
