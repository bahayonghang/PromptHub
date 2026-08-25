# Implement — command palette, shortcuts, and toast

Execution plan for the decisions in `design.md`. Steps are ordered; each gate
passes before the next step starts.

Frontend only. Nothing under `src-tauri/` changes.

Lands last. It drives surfaces the other five children own, and it consumes the
`Modal` primitive from `08-24-detail-modal` design D1.

## Step 0 — Baseline and prerequisite

- [ ] `just build` and `just test` pass before any edit. Report a pre-existing
      failure instead of absorbing it.
- [ ] Confirm `src/components/ui/Modal.tsx` exists with its stack behavior. If
      `08-24-detail-modal` has not landed, build it here per that task's design
      D1 and record that this task now owns it.
- [ ] Confirm which of `⌘S` and `⌘Enter` `08-24-detail-modal` implemented
      locally. Step 2 moves them into the one binding table; two listeners for
      one key is the defect this task exists to prevent.
- [ ] Confirm `usePromptStore` carries `registerDetailActions` and
      `requestSelectPrompt` from `08-24-detail-modal` design D10. Steps 2 and 5
      both depend on them. If that task has not landed, stop — this task cannot
      bind save or copy, and cannot open a prompt safely, without them.

Gate: all three answers are written down.

## Step 1 — Toast

New files: `src/features/notifications/toastStore.ts`, `ToastHost.tsx`

- [ ] `push({ message, tone })`, auto-dismiss at roughly 4s, and a dismiss
      control. The design concept's 1.8s is too short to read an import summary
      or a file path, and a dismissible toast does not need to be brief.
- [ ] One `role="status"` `aria-live="polite"` region. It never takes focus.
- [ ] Enter and exit are CSS transitions, so the existing reduced-motion block
      (`globals.css:310-320`) covers them.
- [ ] Mount `ToastHost` once in `AppShell`.
- [ ] Replace `transferMessage` (`PromptsView.tsx:100`, `:346-350`) with toast
      calls for export, import, batch results, and save.

Do not route copy feedback through the toast. The archived
`08-24-prompt-list-copy` contract keeps success and failure on the control that
was used, and a grid of copy buttons reporting into one region would not say
which one fired (PRD R10).

Gate: a toast is announced by a screen reader, disappears without stealing
focus, and can be dismissed by keyboard (PRD AC7).

## Step 2 — The shortcut layer

New file: `src/shortcuts/useGlobalShortcuts.ts`

- [ ] One `keydown` listener on `document`, added on mount and removed on
      unmount. Mounted once, in `AppShell` (PRD R9).
- [ ] One binding table, per the table in design D2. The footer hints, the
      palette rows, and the handler all read it.
- [ ] Per-binding typing guard. A row without `allowWhileTyping` is ignored while
      focus is in an `input`, `textarea`, or `contenteditable`; the palette's own
      input is always exempt (PRD R1). `⌘S` and `⌘Enter` set `allowWhileTyping`
      and call `preventDefault`. A global guard would make both overlay footer
      hints print keys that never fire — the caret is in a textarea whenever the
      user is editing a body.
- [ ] `⌘S` and `⌘Enter` read `registerDetailActions` on `usePromptStore`
      (`08-24-detail-modal` design D10). Do not reach into the overlay component;
      a `document` listener cannot. With a null slot both bindings do nothing.
- [ ] One platform helper returning both the modifier symbol for hints and the
      modifier key for the check, so the printed symbol and the accepted key
      cannot diverge (PRD R12).
- [ ] Remove any local `⌘S` / `⌘Enter` handler `08-24-detail-modal` added, and
      point the table's entries at that overlay's save and copy actions.

Do not register through `shortcut.register`. A `Local` shortcut there is never
bound (`commands/window.rs:227-246`) and a `Global` one would grab the key while
the app is in the background (design D1).

Gate: `useGlobalShortcuts.test.ts` covers each binding, listener removal on
unmount, and both sides of the guard — `⌘K` ignored with the caret in a textarea,
`⌘S` fired from the same position (PRD AC1, AC1b). `grep` finds exactly one
`keydown` listener for these keys.

## Step 3 — Palette shell

New file: `src/features/prompts/components/CommandPalette.tsx`

- [ ] Render through `Modal`, so focus in, trap, restore, and Escape ordering
      come from the primitive (design D3). Write no Escape ordering logic here.
- [ ] A query input and two groups: PROMPT and 操作.
- [ ] Result list is a `listbox` with `aria-activedescendant` on the input, so
      the highlight is announced while focus stays in the field.
- [ ] Arrow keys move the highlight, Enter activates, Escape closes (PRD R6).

Gate: with both the palette and the detail overlay open, one Escape closes only
the palette (PRD AC5).

## Step 4 — Palette query

- [ ] A palette-local slice calling `api.searchPrompts({ keyword, limit: 5 })`
      through the existing `PromptApi`.
- [ ] Debounce 150 ms and sequence-guard, reusing the pattern from
      `08-24-library-toolbar` design D1.
- [ ] Do not write into `state.prompts` and do not call `setFilters`. Both would
      change the library behind the open palette (design D4).
- [ ] Do not filter `state.prompts` locally. It is one page of 50
      (`promptStore.ts:58`), so a prompt on page two would be unfindable.

Gate: a prompt that is not on the library's current page is findable from the
palette, and closing the palette leaves the library's filters untouched.

## Step 5 — Palette actions

- [ ] Wire each row to the owning child's existing action, per the table in
      design D5. The palette holds no business logic.
- [ ] Selecting a prompt result calls `requestSelectPrompt(id)`, the guarded
      entry point from `08-24-detail-modal` design D10 (PRD R3). Do not call
      `selectPrompt` — `PromptEditor` resets its draft on a prompt change
      (`PromptEditor.tsx:513-524`), so an unguarded call discards unsaved edits
      without asking.
- [ ] On a `cancel` result, keep the palette open with the row highlighted and
      the query intact.
- [ ] If a row needs a behavior that does not exist as an action, add the action
      to the owning store, not to the palette.

Gate: each listed action performs the same effect as its own control in the
sidebar or toolbar, asserted side by side (PRD AC3).

## Step 6 — Enable the sidebar quick-jump

File: `src/components/layout/Sidebar.tsx`

- [ ] Enable the `⌘K` button `08-24-shell-sidebar` left disabled and point it at
      the palette's open action.
- [ ] Confirm it is no longer disabled anywhere in the tree. That is the
      condition `08-24-shell-sidebar` R6 states.

Gate: `grep` finds no `disabled` on the quick-jump button.

## Step 7 — Audit every advertised shortcut

- [ ] List every shortcut hint printed anywhere: the palette rows, the detail
      overlay footer, the sidebar button.
- [ ] Confirm each has a row in the Step 2 binding table and a test.
- [ ] Remove any hint that does not. A printed hint that does nothing is a defect
      (PRD R8, AC4).

Gate: the audit list is written into this file, with one test named per hint.

## Step 8 — i18n

- [ ] Add keys for the palette placeholder, both group headings, each action
      label, the empty-result message, and every toast message replacing
      `transferMessage`.
- [ ] Add every key to all 7 bundles under `src/locales/`.
- [ ] Shortcut hints render the platform symbol from the Step 2 helper, not from
      a translated string.

Gate: `src/features/prompts/i18nKeys.test.ts` passes.

## Step 9 — Full check

- [ ] `just build`
- [ ] `just test`
- [ ] `just ci`
- [ ] `Cmd/Ctrl+K` opens and closes the palette from both views, and does not
      fire while typing in the library search field or a prompt body
      (PRD AC1).
- [ ] Enable reduced motion and confirm neither the palette nor the toast
      animates (PRD AC8).

## Review gates

| After step | Gate                                                            |
| ---------- | --------------------------------------------------------------- |
| 1          | Toast announced, dismissible, and copy feedback left in-control |
| 2          | One listener; typing guard holds; no second path for one key    |
| 3          | Escape closes exactly one layer                                 |
| 4          | Off-page prompts findable; library filters untouched            |
| 5          | Every action delegates to the owning store                      |
| 6          | Quick-jump button no longer disabled                            |
| 7          | Every printed hint has a binding and a test                     |
| 9          | `just ci` green; reduced motion respected                       |

## Rollback points

- Step 1 is independent. The toast works with no palette.
- Step 2 is independent. The bindings work with no palette, except the toggle
  row, which needs Step 3.
- Steps 3 to 5 are additive. Nothing else depends on the palette.
- Step 6 couples the sidebar to the palette. Revert it together with Step 3, or
  the quick-jump button loses its target.

## Open items carried out of this task

- A user who registers `Ctrl+K` as an OS global shortcut in settings will find
  the palette does not open: the OS consumes the key before the web view sees it
  (design D1). This is their own visible configuration. No warning is added to
  the settings panel, which is another feature's surface.
- `shortcut:triggered` still only records `lastTriggeredAction` for display
  (`systemStore.ts:35`). No action string maps to a behavior. Making user-defined
  global shortcuts actually invoke app actions is a separate capability, not this
  task.
