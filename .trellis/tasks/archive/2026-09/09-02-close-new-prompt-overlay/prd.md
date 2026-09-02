# Close new Prompt overlay after create

## Goal

When a user creates a Prompt from the detail overlay, the primary **Create** action persists the Prompt, dismisses the overlay, and leaves the new row selected in the library. The overlay must not stay open as an editor for the new Prompt.

## User value

Create is a completing action. After **Create**, the user returns to the library, sees the new Prompt highlighted, and can reopen it with another click.

## Background

- The reported desktop path is: **New Prompt** → fill title and body → click **Create** (Chinese label `创建`). The Prompt is saved, but the overlay stays open.
- Overlay visibility in `src/features/prompts/PromptsView.tsx:97` is `creating || selectedPrompt != null`. Any selected Prompt opens the overlay.
- Footer **Create** calls `save()` only (`PromptDetailModal.tsx:658-667`). `save()` in create mode calls `onCreate` and does not call `onClose` (`PromptDetailModal.tsx:238-250`).
- `onCreate` in `PromptsView.tsx:238-241` runs `createPrompt(input)` and then `setCreating(false)`.
- `createPrompt` persists, refreshes the list, and selects the new Prompt (`promptStore.ts:517-525`, documented as Req 6.1). `PromptsView.tsx:107-109` then clears `creating` because `selectedPromptId` is set.
- After that sequence, `creating` is false and `selectedPrompt` is the new row, so `overlayOpen` stays true. The overlay title switches from **New Prompt** to the saved Prompt.
- `CommandPalette` is a sibling of `PromptsView` in `AppShell.tsx:52` and opens a Prompt with `requestSelectPrompt` (`CommandPalette.tsx:95-96`). Palette cannot see PromptsView local state.
- List and grid `onSelect` still fire for the already-selected row (`PromptList.tsx:101`, `PromptGrid.tsx:76`).
- Edit-mode **Save** persists and keeps the overlay open. Validation failure already leaves the overlay open (`PromptDetailModal.test.tsx`).
- Dirty close already has **Save and close**, which calls `save()` then `finishProceed()` → `onClose()` (`PromptDetailModal.tsx:698-708`). `onClose` in `PromptsView.tsx:234-237` clears `creating` and calls `selectPrompt(null)`.
- `Ctrl+S` / `Cmd+S` calls the same registered `detailActions.save()` (`useGlobalShortcuts.ts:50-52`). The content-tab form `onSubmit` also calls `save()` (`PromptDetailModal.tsx:563-566`).

## Requirements

- R1. In create mode, a successful **Create** click persists the Prompt and closes the overlay.
- R2. In create mode, a successful save from `Ctrl+S` / `Cmd+S` and from the content-tab form submit uses the same persist-and-close outcome as **Create**.
- R3. If create validation fails or `onCreate` returns a failure, the overlay stays open with the draft intact. No close, no discard.
- R4. Edit-mode **Save** still persists and keeps the overlay open. Dirty-close **Keep editing** and **Discard and close** stay unchanged.
- R5. Overlay close after a successful create must not flash the saved Prompt as an edit overlay (no create-title → saved-title transition before dismiss).
- R6. After a successful create, the library contains the new Prompt, `selectedPromptId` is that Prompt, and the overlay is closed. Clicking the selected list row or grid card opens the overlay on that Prompt.
- R7. A regression test must drive the real create overlay, submit a valid draft, and observe persist, close, and retained library selection. Failure paths from R3 stay covered.
- R8. Selecting a Prompt from the command palette still opens the overlay.
- R9. Edit-mode **Close** (clean close, discard-and-close, and save-and-close) still dismisses the overlay and clears library selection, matching current close behavior.
- R10. Create-mode dirty **Save and close** uses the same persist-and-close-with-selection outcome as **Create**. It must not call the deselecting close path.

## Acceptance criteria

- [ ] AC1 (R1, R7): `PromptDetailModal` with `creating=true` and a valid draft; click **Create**; `onCreate` is called once; `onClose` is not called (so the parent does not deselect).
- [ ] AC2 (R1, R5, R6, R7): From `PromptsView`, open **New Prompt**, fill a valid title and body, click **Create**; the dialog is gone; `selectedPromptId` is the created id; the overlay does not show the saved Prompt title.
- [ ] AC3 (R2): The same persist-and-close-with-selection outcome occurs when create-mode `save()` succeeds through the registered detail save action (the `Ctrl+S` path).
- [ ] AC4 (R3): Empty/invalid title in create mode; **Create** does not call `onCreate` or `onClose`; the dialog remains.
- [ ] AC5 (R3): `onCreate` resolves to a falsy value; the overlay stays open.
- [ ] AC6 (R4, R9): Existing edit-mode save, validation-failure, and dirty-close tests still pass. Closing an existing Prompt overlay still clears `selectedPromptId`.
- [ ] AC7 (R6): After a successful create with the overlay closed and the new row selected, activating that row opens the overlay on the created Prompt.
- [ ] AC8 (R8): `requestSelectPrompt(id)` for a non-null id leaves the detail overlay open for that Prompt (palette and library share this path).
- [ ] AC9 (R10): Create-mode dirty **Save and close** persists, closes the overlay, and keeps `selectedPromptId` on the created Prompt.
- [ ] AC10: `just build` and `just test` pass from the repository root.

## Key decisions

- D1 (Q1 = A): After successful create, keep the new Prompt selected in the library and close the overlay. Clicking the highlighted row reopens the overlay.
- D2: Overlay visibility is not derived from `selectedPrompt != null`. Selection and overlay-open are independent states. `createPrompt` continues to select (Req 6.1) and does not open the overlay.
- D3: Create-mode completing saves must not go through `PromptsView` `onClose`, because that callback calls `selectPrompt(null)`.
- D4: Edit-mode close still deselects. That behavior is unchanged.

## Out of scope

- Changing edit-mode **Save** into save-and-close.
- Making edit-mode **Close** keep library selection.
- Redesigning overlay tabs, focus, dirty confirmation copy, or create-form fields.
- Backend, persistence, Runtime Bridge, localization, or visual-style changes.
- `PromptEditor.tsx` behavior, except if a shared helper is reused without changing the live overlay contract.
- `duplicatePrompt` overlay behavior.

## Risks

- `CommandPalette` opens Prompts through `requestSelectPrompt`. A PromptsView-only `detailOpen` flag would drop palette-open. Overlay-open belongs in `promptStore`.
- Create-mode dirty **Save and close** currently calls `onClose` after `save()` and would clear the new selection unless that path is split from dismiss-and-deselect.
- Existing `PromptsView.layout.test.tsx` cases open the overlay by selecting a library row. Those cases depend on `requestSelectPrompt` still opening the overlay.

## Notes

- Option A forces an overlay/selection split, so this task includes `design.md` and `implement.md`.
