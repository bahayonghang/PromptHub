# Implement — detail overlay and four tabs

Execution plan for the decisions in `design.md`. Steps are ordered; each gate
passes before the next step starts.

Frontend only. Nothing under `src-tauri/` changes.

Depends on `08-24-design-tokens`, `08-24-library-toolbar`, and
`08-24-library-views` (they decide how a prompt is opened), and on
`08-24-prompt-references` for the 引用 tab only.

This is the largest child. Steps 1 to 3 are a pure refactor with no behavior
change; they land and are verified before anything moves into an overlay.

## Step 0 — Baseline

- [ ] `just build` and `just test` pass before any edit. Report a pre-existing
      failure instead of absorbing it.
- [ ] Walk the field checklist in design D3 in the running app and confirm every
      row is editable today. A field that is already broken is not this task's
      to fix, but it must be recorded so it is not blamed on the move.
- [ ] Record the click path for: create a prompt, save an edit, switch text and
      chat mode, add and remove a message, add and remove a tag, create a folder
      from the picker, create a type from the picker, toggle private, open
      version history, create a version, roll back a version, open the
      evaluation workbench, duplicate, delete, pin, favorite.

Gate: the checklist and the click paths are written down.

## Step 1 — Extract the pickers

Files: `src/features/prompts/components/detail/FolderPicker.tsx`,
`PromptTypePicker.tsx`

- [ ] Move `FolderPicker` (`PromptEditor.tsx:114-295`) and `PromptTypePicker`
      (`:296-498`) into their own files, unchanged.
- [ ] Import them back into `PromptEditor`.

No behavior change. If a test needs editing in this step, the move was not clean.

Gate: `PromptEditor.test.tsx` passes unchanged.

## Step 2 — Extract the sections

Directory: `src/features/prompts/components/detail/sections/`

- [ ] `IdentitySection`, `DefinitionSection`, `OrganizationSection`,
      `MediaSection`, per the table in design D2, from the four `<section>`
      blocks at `PromptEditor.tsx:671`, `:743`, `:995`, `:1079`.
- [ ] The draft stays in `PromptEditor` **for this step only**. Steps 1 and 2 are
      a pure extraction whose proof is that `PromptEditor.test.tsx` passes
      unchanged; moving the draft in the same step would destroy that proof.
      `PromptDetailModal` is the final owner (design D2b) and step 4 hoists it.
- [ ] Do not split the draft across sections, now or after the hoist.
      `syncVariables` reads the system prompt, the user prompt, and every message
      together (`PromptEditor.tsx:530-548`); a split draft breaks it.

Gate: `PromptEditor.test.tsx` still passes unchanged, and `PromptEditor.tsx` is
under 300 lines.

## Step 3 — The modal primitive

New file: `src/components/ui/Modal.tsx`

- [ ] Portal to `document.body`, following `SearchBar.tsx:216`.
- [ ] `role="dialog"`, `aria-modal="true"`, labelled by its heading.
- [ ] Focus into the first focusable element on open; trap Tab and Shift+Tab;
      restore focus to the previously focused element on close.
- [ ] Add an `#app-content` wrapper in `AppShell.tsx` around `Sidebar` + `Header` + `main`, leaving `TitleBar` and `CloseDialog` as siblings outside it
      (design D1).
- [ ] `aria-hidden` or `inert` on `#app-content` while open — never on the shell
      root. `CloseDialog` is a direct child of the shell root (`AppShell.tsx:41`)
      and does not portal, so inerting the root makes the native close
      confirmation unfocusable and unclickable while the overlay is open.
- [ ] Escape and scrim click call `onClose`. The primitive does not decide
      whether closing is allowed.
- [ ] A module-level stack so the topmost modal is the one Escape closes.
      `08-24-command-palette` R7 depends on this.

Do not migrate `CloseDialog` onto it. It works, and changing it puts an
unrelated dialog in this diff (design D1).

Gate: `Modal.test.tsx` covers focus in, trap in both directions, restore,
Escape, and two stacked modals where Escape closes only the top one. Plus one
coexistence test: with the overlay open, `CloseDialog` renders, both its buttons
are focusable and clickable, and confirming still calls `confirmClose`
(PRD AC1b).

## Step 4 — The overlay shell

New file: `src/features/prompts/components/detail/PromptDetailModal.tsx`

- [ ] `min(1180px, 100%)`, centered on a blurred scrim, entry animation as a CSS
      animation — not JavaScript-driven — so the existing reduced-motion block
      (`globals.css:310-320`) covers it (PRD R10, AC8).
- [ ] Header: title, version chip, metadata line, 复制正文, edit/read toggle,
      favorite, pin, an overflow menu holding duplicate and delete with its
      confirmation, close (design D4).
- [ ] Tab strip as a real tablist with arrow-key movement between tabs.
- [ ] Footer: shortcut hints and save. Print a hint only for a shortcut that is
      implemented. `⌘S` and `⌘Enter` come from `08-24-command-palette`; if that
      has not landed, implement them here or omit the hints.
- [ ] Hoist the draft out of `PromptEditor` into `PromptDetailModal`
      (design D2b). `ContentTab` and the sections become controlled: `draft`,
      `onChange`, `errors`, `readOnly`. `PromptEditor` does not survive as a
      draft owner.
- [ ] `syncVariables` (`:530-548`) runs in `PromptDetailModal` over the whole
      draft. Do not run it per section.
- [ ] Keep the validation rules (`:580-585`) and the create and update payloads
      (`:617-652`) exactly.
- [ ] `save()` returns `{ ok: true } | { ok: false; errors }`. A failed save
      leaves the overlay open and `dirty` set. Step 5's save-and-close depends on
      this return value; without it, save-and-close discards an edit that failed
      validation.

Gate: saving from the overlay produces the same store calls with the same
arguments as the inline editor (PRD AC3), asserted against a fake api, and a save
that fails validation leaves the overlay open with the edit intact (PRD AC3b).

## Step 5 — The dirty guard

File: `PromptDetailModal.tsx`

- [ ] Compare the draft against the loaded prompt over the same field set the
      update payload uses (design D3), producing a `dirty` flag.
- [ ] Not dirty: Escape, scrim, and 关闭 close immediately.
- [ ] Dirty: all three raise a confirmation offering save-and-close,
      discard-and-close, and keep-editing. Not a bare "are you sure".
- [ ] The confirmation is a second `Modal` on the stack, so Escape inside it
      returns to the editor.
- [ ] Save-and-close closes only when `save()` resolves `{ ok: true }`
      (design D2b). A failed save keeps the overlay open with the edit intact.

Do not autosave on close. Saves write a version snapshot
(`services/version::append_snapshot`), and an autosave would create a version the
user did not ask for (design D5).

### Step 5b — Guarded navigation (design D10)

Closing is not the only way to lose the draft. `PromptEditor` resets it whenever
the selected prompt changes (`PromptEditor.tsx:513-524`), so every caller of
`selectPrompt` is a door out of a dirty editor.

- [ ] Add `requestSelectPrompt(id)` and `registerNavigationGuard(guard)` to
      `usePromptStore`. `selectPrompt` stays as the unguarded primitive the guard
      calls.
- [ ] `PromptDetailModal` registers the guard on mount, clears it on unmount, and
      reuses the D5 confirmation. Keep-editing and a failed save both resolve
      `cancel`.
- [ ] Migrate every caller: the list and grid items, paging, and any view or
      folder change that can drop the open prompt. `08-24-command-palette` step 5
      migrates its own call.
- [ ] `grep` for `selectPrompt(` and confirm the only remaining call sites are
      the guard itself and the store's own definition.

### Step 5c — The detail action registry (design D10)

- [ ] Add `registerDetailActions({ save, copy })` to `usePromptStore`, registered
      by `PromptDetailModal` on mount and cleared on unmount.
- [ ] `08-24-command-palette`'s `⌘S` and `⌘Enter` read this slot. A
      `document`-level listener cannot reach a component callback, and no action
      string maps to a behavior in `src/` today.
- [ ] With no overlay open the slot is null and both bindings are inert.
- [ ] Print the `⌘S` / `⌘Enter` footer hints only if a binding exists — either
      the palette's, or a local listener this task adds against the same registry
      that the palette later deletes. Never two listeners for one key, and never
      a hint with no handler (parent CC9).

Gate: editing a field then pressing Escape raises the confirmation; each of the
three answers behaves as labelled. Opening a different prompt from the library and
from the palette both raise it too (PRD AC2b, parent CC11). `⌘S` saves while the
caret is in the prompt body (PRD AC2c).

## Step 6 — Version and run tabs

- [ ] `VersionTab` wraps the existing `VersionHistory` with the same five props
      it takes today (`PromptsView.tsx:633-647`), including the rollback
      confirmation.
- [ ] `RunTab` wraps `EvaluationWorkbench` with `prompt` and `versions`
      (`PromptsView.tsx:608`). No change to the engine, its providers, or its
      persistence.
- [ ] The tab counts come from the data already loaded. If a count is not
      available without reaching into another feature's store, render no count
      rather than a wrong one.

Gate: a rollback from the version tab changes the body and the header version
chip (PRD AC4); a run from the run tab matches the current workspace toggle
(PRD AC5).

## Step 7 — References tab

New file: `.../detail/ReferencesTab.tsx`

- [ ] Consume `reference.list` through `src/features/prompts/api.ts`. No
      `invoke` outside the feature api (parent CC4).
- [ ] Three groups: outgoing, incoming, and a picker.
- [ ] An unresolved reference carries an icon and a text reason — missing,
      ambiguous, locked. Never color alone (PRD AC6).
- [ ] The picker inserts the explicit `@@Title@@` form from
      `08-24-prompt-references` design D7, and marks the draft dirty.
- [ ] If that task has not landed, render the tab disabled with a stated reason.
      Do not omit it.

Gate: the tab shows a resolved reference, an unresolved one, and an incoming
one, each distinguishable in a grayscale screenshot.

## Step 8 — The "references" rename

- [ ] Rename `promptsView.editor.sections.references`
      (`PromptEditor.tsx:1087`) to `...sections.attachments` in all 7 bundles
      under `src/locales/`, and update the one call site.
- [ ] Give `references` to the new tab.

Both names cannot mean two things in one feature (design D7). If this rename is
refused, raise it rather than naming the new tab something the UI does not say.

Gate: `grep` finds exactly one meaning per key, and
`src/features/prompts/i18nKeys.test.ts` passes.

## Step 9 — Remove the inline pane

File: `src/features/prompts/PromptsView.tsx`

- [ ] Remove the detail pane, the `compactPane` state, the `workspaceMode`
      state, the `showHistory` state, and the mode tab strip
      (`PromptsView.tsx:91-94`, `:426-683`).
- [ ] Keep the locked branch (`:594-606`), moved into the overlay per design D9:
      header and tab strip render, the content tab shows the locked notice, the
      other three tabs are disabled with a stated reason, and 复制正文 is
      disabled.
- [ ] Delete the orphaned container-query rules for `__discovery`, `__detail`,
      `__detail-header`, `__detail-title`, `__history`, and
      `__compact-control` (`globals.css:204-262`). Check that
      `08-24-shell-sidebar` already removed `__folders` rather than assuming it.
- [ ] Keep `.prompt-workspace` itself. The library grid and list size against it.

Gate: `grep` finds no remaining reference to the deleted class names, and
`just build` passes with no unused-CSS warning left behind.

## Step 10 — Responsive check

- [ ] At the app's minimum usable width, the overlay fits with no horizontal
      scrollbar and the 内容 tab's two columns collapse to one.
- [ ] The `prompt-editor` container queries (`globals.css:266-296`) still fire
      inside the overlay. They are keyed to the form's own container
      (design finding 3), so this is a verification, not a rewrite.
- [ ] Record the measured minimum width here.

Gate: the measured width is in this file.

## Step 11 — Full check

- [ ] `just build`
- [ ] `just test`
- [ ] `just ci`
- [ ] Walk the Step 0 field checklist in the overlay. Every row editable
      (PRD AC2).
- [ ] Walk the Step 0 click paths.
- [ ] Open a locked private prompt and confirm no content is exposed (PRD AC7).
- [ ] Enable reduced motion and confirm the overlay does not animate (PRD AC8).

## Review gates

| After step | Gate                                                          |
| ---------- | ------------------------------------------------------------- |
| 1          | Tests pass unchanged; the move was clean                      |
| 2          | Tests pass unchanged; `PromptEditor.tsx` under 300 lines      |
| 3          | Focus trap, restore, and stack ordering all covered           |
| 4          | Save produces identical store calls                           |
| 5          | Close-with-edits offers three explicit answers                |
| 6          | Version and run behavior unchanged                            |
| 7          | Unresolved state readable in grayscale                        |
| 8          | One meaning per i18n key                                      |
| 9          | No orphaned container-query rules remain                      |
| 11         | `just ci` green; field checklist complete; locked prompt safe |

## Rollback points

- Steps 1 and 2 are a pure refactor. They can ship alone and are worth keeping
  even if the overlay is abandoned.
- Step 3 is a standalone primitive with no caller until Step 4.
- Steps 4 to 8 add the overlay beside the existing pane. Until Step 9 both
  surfaces exist, so the overlay can be validated against the pane directly.
- Step 9 is the destructive one. Keep it in its own commit, separate from the
  extraction, so a revert does not undo the decomposition.

## Open items carried out of this task

- `CloseDialog` still implements dialog markup without focus trapping or focus
  restore (design D1). Migrating it onto `Modal` is a follow-up, not part of this
  diff.
- The version and evaluation surfaces stop excluding each other. Today selecting
  the evaluation mode hides the history (`PromptsView.tsx:481-484`); as tabs they
  coexist. Recorded so the mode-tab removal is not read as a loss.
- `⌘S` and `⌘Enter` are advertised by the design footer and owned by
  `08-24-command-palette`. Whichever lands second verifies that every printed
  hint corresponds to a working shortcut (that task's R8).
