# Command palette, shortcuts, and toast

Child of `08-24-ui-refactor`. Owns parent requirements R13 and R14.

## Goal

Add the design concept's `⌘K` command palette, the global shortcut layer it
implies, and the toast surface that reports transient results.

## Ordering

Lands last. The palette drives surfaces the earlier children own: the sidebar
scope, the library view mode, the theme, the create action, and the detail
overlay.

## Background

- There is no command palette and no global shortcut layer in `src/`.
- There is no toast system. The archived task `08-24-prompt-list-copy` reports
  copy results inside the control that was used, and `PromptsView` reports
  transfer results through a local `transferMessage` string
  (`PromptsView.tsx:101`).
- A backend `shortcut:triggered` event already exists
  (`src-tauri/src/commands/events.rs`, listed in `src/code_map.md:39-40`). Its
  relationship to a frontend-only `Cmd/Ctrl+K` must be settled before writing
  a second, conflicting shortcut path.
- The sidebar's quick-jump button is created by `08-24-shell-sidebar` and is
  wired here.

## Design target

The palette is a centered overlay with a query input and grouped results. The
design shows two groups: PROMPT, listing up to five matching prompts with their
usage count, and 操作, listing 新建 Prompt (`⌘N`), 只看收藏, 切换列表视图, and
切换深浅主题 (`⌘⇧L`).

`Escape` closes the palette and the detail overlay. The detail overlay's footer
advertises `⌘S` save and `⌘Enter` copy.

The toast is a transient message, roughly 1.8s in the concept.

## Requirements

- R1 (amended by `design.md` D2): `Cmd+K` on macOS and `Ctrl+K` elsewhere toggle
  the palette. The typing guard is per binding, not global: `⌘K`, `⌘N`, and
  `⌘⇧L` do not fire while the user types in an input, textarea, or
  contenteditable, and the palette's own input is always exempt. `⌘S` and
  `⌘Enter` do fire while typing, because editing a prompt body puts the caret in
  a textarea and that is exactly when save and copy are wanted. A global guard
  would make both footer hints print keys that never fire.
- R1b (added by `design.md` D2): The overlay's save and copy reach the
  `document`-level listener through the `registerDetailActions` slot that
  `08-24-detail-modal` design D10 adds to `usePromptStore`. With no overlay open
  the slot is null and both bindings are inert.
- R2 (settled by `design.md` D4): The palette calls
  `api.searchPrompts({ keyword, limit: 5 })` into a palette-local slice. It does
  not filter `state.prompts`, which is one page of 50 (`promptStore.ts:58`), and
  it does not write through `setFilters`, which would change the library's result
  set behind the open palette.
- R3: Selecting a prompt result opens it in the detail overlay through
  `requestSelectPrompt`, the guarded entry point from `08-24-detail-modal`
  design D10. It never calls `selectPrompt` directly, which would discard an
  unsaved draft without asking (parent CC11).
- R4: The action group offers, at minimum: new prompt, show favorites only,
  switch view mode, and toggle theme. Each action reuses the owning child's
  existing action, not a second implementation.
- R5: The palette is a modal dialog with the same focus rules as the detail
  overlay: focus in on open, trapped while open, restored on close.
- R6: Arrow keys move the highlighted row, Enter activates it, Escape closes.
  The highlighted row is announced to assistive technology.
- R7: `Escape` closes the palette when the palette is open, otherwise the detail
  overlay. Only one layer closes per press.
- R8: Advertised shortcuts work. A shortcut shown in the footer or in the
  palette and not implemented is a defect.
- R9 (settled by `design.md` D1, D2): The shortcut layer is one `keydown`
  listener on `document`, registered once in `AppShell` and removed on unmount,
  reading one binding table. It does not go through `shortcut.register`: a
  `Local` shortcut there is validated and never bound
  (`src-tauri/src/commands/window.rs:227-246`), and a `Global` one would grab the
  key while the app is in the background. The existing `shortcut:triggered` path
  is untouched — the frontend only records `lastTriggeredAction` for display
  (`systemStore.ts:35`) and maps no action string to a behavior.
- R9b: A user who registers `Ctrl+K` as an OS global shortcut will find the
  palette does not open, because the OS consumes the key first. This is their own
  visible configuration; no warning is added to the settings panel.
- R10 (amended by `design.md` D6): The toast reports transient results — batch
  action, save, import, export. Copy is **not** among them. The archived
  `08-24-prompt-list-copy` contract keeps copy success and failure on the control
  that was used, and a grid of copy buttons reporting into one shared region
  would not say which one fired. R10 previously listed copy first and then
  excluded it in the same sentence; parent R14 is amended to match.
- R11: The toast is announced through a live region, is dismissible, and
  respects reduced motion.
- R12: All labels come from i18n keys, present in all 7 bundles. Shortcut hints
  render the platform's own modifier symbols.

## Acceptance criteria

Each criterion names the requirement it closes.

- [ ] AC1 (R1): `Cmd/Ctrl+K` opens and closes the palette from any view, and does
      not fire while typing in the search field or the prompt body.
- [ ] AC1b (R1, R1b): `⌘S` and `⌘Enter` **do** fire with the caret in the prompt
      body, and both call `preventDefault`. With no overlay open they do nothing.
- [ ] AC2 (R2): Typing filters both groups; selecting a prompt opens its overlay.
      A prompt that is not on the library's current page is findable.
- [ ] AC2b (R2): Closing the palette leaves the library's filters and result set
      exactly as they were before it opened.
- [ ] AC2c (R3, parent CC11): Selecting a prompt while the overlay holds unsaved
      edits raises the dirty confirmation. On keep-editing the palette stays open
      with the query intact.
- [ ] AC3 (R4): Each listed action performs the same effect as its own control in
      the sidebar or toolbar, asserted side by side.
- [ ] AC4 (R8, parent CC9): Every shortcut advertised in the palette and in the
      overlay footer has a row in the one binding table and a test. The audit list
      is written into `implement.md` step 7.
- [ ] AC5 (R7): With both the palette and the overlay open, one `Escape` closes
      only the palette.
- [ ] AC5b (R7, parent CC10): With the palette, the overlay, and the dirty
      confirmation open, each `Escape` closes exactly one layer, topmost first.
      The native close dialog stays operable throughout
      (`08-24-detail-modal` AC1b).
- [ ] AC6 (R5, R6): The palette is fully keyboard operable, and focus returns to
      the trigger on close.
- [ ] AC7 (R10, R11): A toast is announced by a screen reader and disappears
      without stealing focus.
- [ ] AC7b (R10): No copy control reports through the toast. Copy feedback stays
      on the control that was used.
- [ ] AC8 (R11): With reduced motion enabled, neither the palette nor the toast
      animates.
- [ ] AC9: `just build` and `just test` pass.

## Out of scope

- A user-editable keybinding map.
- Registering new OS-level global shortcuts in the backend.
- Fuzzy scoring, and any matcher other than the one `prompt.search` already uses.
  That matcher is whole-term FTS5 phrase matching with implicit AND
  (`services/prompt.rs:868-878`), not substring matching. The palette returns the
  same results the library's search field would; see `design.md` D4.
