# Design — close new Prompt overlay after create

## Problem

The overlay is open whenever `creating || selectedPrompt != null`. `createPrompt` selects the new Prompt (Req 6.1), so a successful **Create** leaves the overlay open on the saved row. The user wants persist + dismiss + keep the new row highlighted.

## Boundaries

- Frontend only: `promptStore`, `PromptsView`, `PromptDetailModal`.
- `CommandPalette` stays a `requestSelectPrompt` caller. It must not grow a second open API.
- No Runtime Bridge, backend, i18n, or `PromptEditor.tsx` contract change.

## State split

Add `detailOpen: boolean` to `promptStore`, default `false`.

| Action | Selection | `detailOpen` |
|---|---|---|
| `selectPrompt(id)` | sets id, loads row | unchanged |
| `requestSelectPrompt(id)` when `id != null` | `selectPrompt(id)` after the nav guard | `true` |
| `requestSelectPrompt(null)` | `selectPrompt(null)` after the nav guard | `false` |
| `createPrompt` | persist, refresh, `selectPrompt(newId)` | `false` |
| PromptsView dismiss (`onClose`) | `selectPrompt(null)` | `false` plus local `creating=false` |

`creating` stays local in `PromptsView`. A create draft is not a selected Prompt.

Overlay visibility:

```text
open = creating || (detailOpen && selectedPrompt != null)
prompt = creating ? null : selectedPrompt
```

That yields a valid post-create state: `creating=false`, `detailOpen=false`, `selectedPromptId=newId`, overlay closed, row highlighted.

## Why the flag lives in the store

`CommandPalette` is a sibling of `PromptsView` (`AppShell.tsx`). It already opens a Prompt with `requestSelectPrompt`. A view-local flag cannot receive that signal. Store state is the existing pattern for selection shared by library, palette, and shortcuts.

`selectPrompt` must not open the overlay. `createPrompt` and `duplicatePrompt` both call `selectPrompt`. Opening on `selectPrompt` would recreate the create-stays-open bug. `duplicatePrompt` is out of scope: if the overlay is already open, `detailOpen` stays true and the selected row swaps.

## Create-success close

`PromptDetailModal.save()` in create mode already calls `onCreate` and does not call `onClose`. Keep that. The parent closes because `createPrompt` sets `detailOpen=false` and `PromptsView` sets `creating=false` (existing `selectedPromptId` effect plus `onCreate`).

Do not route create-success through `onClose`. That callback deselects.

Create-mode dirty **Save and close** currently does `save()` then `finishProceed()` → `onClose()`. After a successful create-mode save, skip `onClose`: resolve the dirty dialog and let the parent close without clearing selection.

Edit-mode dirty **Save and close** still calls `onClose` (R9).

## Flash (R5)

While `creating` is true, `prompt` is forced to `null`, so the overlay stays on the New Prompt chrome even after `createPrompt` writes `selectedPrompt`. The next state is `creating=false` and `detailOpen=false`, which unmounts the overlay. The saved title never becomes the overlay heading.

## Reopen

List and grid already call `onSelect` for the selected row. That path is `requestSelectPrompt(id)`, which sets `detailOpen=true` and reopens the overlay.

## Compatibility

- Library click, palette prompt row, and dirty-nav proceed onto another Prompt all go through `requestSelectPrompt(id)` and still open the overlay.
- **New Prompt** still does `requestSelectPrompt(null)` then `setCreating(true)`.
- Edit **Close** still deselects.
- `createPrompt` still selects (Req 6.1 store test unchanged except it also asserts `detailOpen === false`).

## Rollback

Revert the store flag and the two UI call sites. Overlay visibility returns to `creating || selectedPrompt != null`.
