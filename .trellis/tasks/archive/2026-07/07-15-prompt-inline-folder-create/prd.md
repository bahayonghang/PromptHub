# Add inline folder creation to prompt editor

## Goal

Let users create and immediately select a root folder without leaving the prompt
editor or losing the current prompt draft, reusing the existing folder domain.

## Confirmed Facts

- Folder creation already exists at every backend boundary:
  `FolderTree.tsx:26-74` -> `promptStore.ts:451-461` -> `api.ts:98` ->
  `folder.create` -> `services/folder.rs:87-105`.
- The store returns the created `Folder | null` and reloads the folder list, so
  the editor can select the authoritative returned id without a new command.
- The editor currently receives `folders` but no creation callback and renders a
  selection-only native control (`PromptEditor.tsx:71-74`,
  `PromptEditor.tsx:296-312`).
- Backend names are trimmed and validated to 1-255 characters. Duplicate names
  are currently allowed and this task does not change that domain policy.

## Requirements

- R1: Place a familiar create-folder affordance adjacent to the editor's folder
  picker; it must not require leaving the prompt draft.
- R2: Reveal a small inline name input rather than a modal. Enter submits, Escape
  cancels, and an explicit icon action is available for pointer users.
- R3: Call the existing store/API creation path. Only a successful returned
  folder id updates the draft selection.
- R4: Preserve title, prompt body/messages, variables, tags, media, privacy,
  notes, and all other unsaved draft fields while creating/canceling/failing.
- R5: Surface validation/backend errors through the existing error channel and
  keep the typed name available after failure for correction.
- R6: Prevent duplicate submissions while busy and restore a predictable focus
  target after success or cancel.
- R7: Create at the root level (`parentId: null`). Nested creation and folder
  management remain in the tree.
- R8: Localize visible text and accessible names in all seven locale bundles.

## Acceptance Criteria

- [ ] AC1: From create and edit modes, the user can open the inline input, type a
  valid name, submit, and see the returned folder selected in the draft.
- [ ] AC2: No prompt create/update occurs until the user submits the prompt form;
  folder creation alone never discards or saves the prompt draft.
- [ ] AC3: Empty/whitespace and >255-character input is rejected accessibly;
  backend failure retains the input and does not change folder selection.
- [ ] AC4: Escape/cancel returns focus to the create affordance and leaves the
  existing selection/draft untouched.
- [ ] AC5: Busy state prevents duplicate creation and communicates progress.
- [ ] AC6: Store/API contracts remain unchanged and tests prove success, cancel,
  validation, failure, focus, and draft preservation.
- [ ] AC7: `just build` and `just test` pass.

## Out of Scope

- Nested folder creation, rename/delete/reorder, duplicate-name policy changes,
  icons, colors, bulk folder creation, or a new backend command.
- Custom prompt types or filter changes.
