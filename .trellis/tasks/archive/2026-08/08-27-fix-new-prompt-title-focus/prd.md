# Fix new Prompt title focus loss

## Goal

Allow users to type a new Prompt title without focus leaving the title input after the first character, while preserving the shared modal's focus trap, close handling, and focus restoration behavior.

## Background

- The reported desktop behavior is that typing one letter into the new Prompt title moves focus to the top-right pencil button, preventing uninterrupted input.
- The isolated reproduction in `research/title-focus-repro.test.tsx` drives the real `PromptDetailModal` creation path. It deterministically fails because the title keeps the entered value `u`, but `document.activeElement` becomes the `Read only` pencil button.
- `src/components/ui/Modal.tsx:71-93` captures and restores focus, schedules focus on the first enabled control, and keys that lifecycle effect on both `open` and `onClose`.
- `src/features/prompts/components/detail/PromptDetailModal.tsx:295` creates `requestClose` during every render and passes it to `Modal` at line 369. The title change at `src/features/prompts/components/detail/sections/IdentitySection.tsx:49-53` updates the draft and re-renders the detail modal, changing the callback identity.
- Probes confirm that the title input DOM node remains mounted, the editor remains editable, and an extra animation-frame focus request is scheduled after the title change. The isolated test does not mount global shortcuts.

## Requirements

- R1. After the user focuses the new Prompt title input, each title edit must preserve focus on that input so typing can continue normally.
- R2. Modal entry focus, Tab trapping, Escape handling, stacked-modal behavior, and restoration of the previously focused element on close must remain intact.
- R3. Existing Prompt detail behavior, including dirty-draft close confirmation and read-only mode, must remain unchanged.
- R4. The fix must live at the smallest correct ownership boundary and must not add a dependency or change backend, persistence, localization, or visual styling contracts.
- R5. A regression test must exercise the real new-Prompt detail call path and assert both the entered value and retained focus after React effects settle.

## Acceptance Criteria

- [x] AC1 (R1, R5): A focused component test renders `PromptDetailModal` with `creating=true`, focuses `Title`, enters at least the first character, waits for pending focus effects, and observes that the value is retained and `document.activeElement` is still the title input.
- [x] AC2 (R2): Existing shared `Modal` tests for entry focus, Tab trapping, stacked Escape handling, inert app content, and close-time focus restoration pass.
- [x] AC3 (R3): Existing `PromptDetailModal` tests for save payloads, validation failure, and dirty-draft close choices pass.
- [x] AC4 (R4): No changes are made to `src-tauri/**`, `src/locales/**`, generated output, or the read-only `ref/PromptHub/**` tree, and no dependency is added.
- [x] AC5: The required frontend gates `just build` and `just test` pass from the repository root.

## Technical Notes

- The confirmed causal chain is: title `onChange` -> draft state update -> `PromptDetailModal` re-render -> new `requestClose` identity -> `Modal` focus effect cleanup/re-entry -> first focusable control receives focus. In create mode that control is the pencil button.
- The shared `Modal` lifecycle should be keyed to whether the modal is open, not to the identity of its latest close callback. The close handler used by rendered event handlers must still be current.
- `StackEntry.onClose` is stored but not read by the current stack logic; the stack uses only the entry id to decide which modal may handle Escape.
- This is a lightweight frontend bug fix, so PRD-only planning is sufficient.

## Out of Scope

- Redesigning the Prompt detail modal or changing its initial focus target.
- Changing pencil/read-only semantics, title validation, save behavior, shortcuts, or dirty-draft confirmation UX.
- Backend, database, Runtime Bridge, localization, or visual-style changes.

## Risks and Deferred Evidence

- Because `Modal` is shared, an incorrect dependency change could regress nested modal ordering or focus restoration. Existing shared-modal tests plus the full frontend gates are required.
- The automated jsdom reproduction establishes the React focus failure and the regression contract. Final native Windows WebView interaction remains `UNVERIFIED` unless the desktop app is launched and exercised after implementation.
