# Implement — close new Prompt overlay after create

## Checklist

1. Add `detailOpen: boolean` (default `false`) to `promptStore`. Wire:
   - `requestSelectPrompt(id)` → `detailOpen = id != null` after a successful guard.
   - `createPrompt` → after `selectPrompt(newId)`, set `detailOpen = false`.
   - Keep `selectPrompt` free of overlay-open side effects.
2. Point `PromptsView` overlay `open` at `creating || (detailOpen && selectedPrompt != null)`. On dismiss, set `creating=false`, `detailOpen=false`, and `selectPrompt(null)`.
3. In `PromptDetailModal`, after a successful create-mode `save()`, do not call `onClose`. Create-mode dirty **Save and close** must resolve the confirm dialog without `onClose`.
4. Tests:
   - Store: `requestSelectPrompt(id)` sets `detailOpen`; `requestSelectPrompt(null)` and `createPrompt` clear it; `selectPrompt` does not set it.
   - `PromptDetailModal`: valid **Create** calls `onCreate` and not `onClose`; invalid title and failed `onCreate` leave the dialog open; create-mode **Save and close** does not call `onClose`.
   - `PromptsView.layout.test.tsx`: successful create closes the dialog, keeps `selectedPromptId`, and a second activation of that row reopens the overlay. Existing library-select and dirty-create cases still pass.
5. Run `npx vitest run` on the touched files, then `just build` and `just test` from the repository root.

## Validation

```text
npx vitest run src/features/prompts/promptStore.test.ts src/features/prompts/components/detail/PromptDetailModal.test.tsx src/features/prompts/PromptsView.layout.test.tsx
just build
just test
```

## Risky files

- `src/features/prompts/promptStore.ts` — selection is shared; do not make `selectPrompt` open the overlay.
- `src/features/prompts/PromptsView.tsx` — overlay `open` derivation and `onClose` deselect.
- `src/features/prompts/components/detail/PromptDetailModal.tsx` — create-mode dirty save-and-close must not deselect.
- `src/features/prompts/PromptsView.layout.test.tsx` — library click currently opens the overlay only if `requestSelectPrompt` still sets `detailOpen`.

## Rollback

Revert the three source files and their colocated tests. No schema or i18n rollback.
