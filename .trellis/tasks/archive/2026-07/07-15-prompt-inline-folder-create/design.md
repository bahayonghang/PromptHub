# Design: Inline Folder Creation

## Component Contract

Extend `PromptEditor` with a focused async callback that returns the authoritative
created folder or `null`, mirroring the existing store action. `PromptsView`
adapts `usePromptStore.createFolder` and passes it down. Backend access remains
outside the component.

A small `FolderPicker` child is justified because the select, create disclosure,
temporary name, busy state, validation, focus, and callbacks form one coherent
interaction. It remains feature-local and is not generalized into a universal
combobox.

## Interaction

Render the current folder select and an icon-only plus button with localized
label/tooltip. Activating plus reveals an inline text field with create and cancel
icons. Enter creates a root folder; Escape cancels. The input is focused on open.

On success:

1. the store reloads folders and returns the created folder;
2. the editor sets `draft.folderId` to the returned id;
3. the inline input closes and focus moves to the folder picker.

On failure, keep the input open and unchanged. Do not optimistically add an id
or infer success from a reloaded list.

## Validation and Error State

Mirror the backend's trimmed 1-255 character rule for immediate feedback while
the backend remains authoritative. Error text sits adjacent to the input and is
associated with `aria-describedby`; busy/disabled state remains readable.

## Data and Compatibility

No schema, command, DTO, or folder-policy changes are required. The existing
prompt draft keeps an ordinary nullable folder id, so versioning, portable
bundles, filters, and batch moves remain unchanged.
