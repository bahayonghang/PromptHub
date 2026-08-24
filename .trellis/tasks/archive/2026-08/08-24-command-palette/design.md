# Design — command palette, shortcuts, and toast

## What the existing shortcut path is, and is not

The PRD asks for the relationship between `shortcut:triggered` and a frontend
`Cmd/Ctrl+K` to be settled before a second path is written. It is settled by
three facts.

1. **Only global shortcuts are registered.** `shortcut.register`
   (`src-tauri/src/commands/window.rs:210-246`) walks the accepted registry and
   calls `on_shortcut` only when `shortcut.mode == ShortcutMode::Global`. A
   shortcut registered as `Local` is validated, stored, and never bound to
   anything.

2. **Nothing dispatches the action.** The frontend subscribes to
   `shortcut:triggered` (`src/features/system/systemStore.ts:35`) and stores the
   action id as `lastTriggeredAction`, which `ShortcutsPanel` displays as
   confirmation that a binding fired. No action string maps to a behavior
   anywhere in `src/`.

3. **There is no frontend shortcut layer at all.** The only `keydown` listener in
   `src/` is `SearchBar`'s Escape handler for its filter popover
   (`SearchBar.tsx:87`).

So `shortcut:triggered` is a user-configured OS-level binding whose payload is
currently only displayed. It is not a dispatcher this task can extend.

Two more facts shape the palette itself:

4. **The store's prompt list is the library's, not a search scratchpad.**
   `state.prompts` is one page of 50 (`promptStore.ts:58`) and `filters` drives
   what the library shows. Running the palette's query through `setFilters`
   would change the library behind the palette.

5. **There is no toast.** `PromptsView` reports transfer results through a local
   `transferMessage` string rendered inline (`PromptsView.tsx:100`, `:346-350`),
   and copy reports inside its own control (`CopyPromptButton.tsx:103-108`).

## Decisions

### D1 — `Cmd/Ctrl+K` is a frontend key handler; the OS registry is left alone

The palette binds through one `keydown` listener on `document`, registered once.
It does not go through `shortcut.register`, because a `Local` shortcut there is
inert (finding 1) and a `Global` one would grab the key while the app is in the
background, which is wrong for a palette.

The collision that does exist: a user may register `Ctrl+K` as a global shortcut
in settings. The OS then consumes the key before the web view sees it, and the
palette will not open. This is the user's own configuration, it is visible in
the shortcuts panel, and the correct response is to leave it alone rather than
fight for the key. Record it in the task notes; do not add a warning to the
settings panel, which is another feature's surface.

### D2 — One shortcut layer, one module, one registration

New `src/shortcuts/useGlobalShortcuts.ts`, mounted once in `AppShell`.

- One `keydown` listener on `document`, added on mount and removed on unmount
  (PRD R9).
- A per-binding typing guard, not a global one. A binding without
  `allowWhileTyping` is ignored while focus is in an `input`, `textarea`, or
  `contenteditable`; the palette's own input is always exempt (PRD R1). See the
  table's last column for why this is per binding.
- Bindings live in one table so the footer hints, the palette rows, and the
  handler read the same source. A hint that has no row in the table cannot be
  printed (PRD R8).

| Binding            | Action           | Owner of the action        | While typing |
| ------------------ | ---------------- | -------------------------- | ------------ |
| `Cmd/Ctrl+K`       | toggle palette   | this task                  | ignored      |
| `Cmd/Ctrl+N`       | new prompt       | `08-24-library-toolbar`    | ignored      |
| `Cmd/Ctrl+Shift+L` | toggle theme     | `08-24-shell-sidebar` (D7) | ignored      |
| `Cmd/Ctrl+S`       | save open prompt | `08-24-detail-modal` (D10) | **allowed**  |
| `Cmd/Ctrl+Enter`   | copy open prompt | `08-24-detail-modal` (D10) | **allowed**  |

**Why the last column exists.** A blanket "ignore every binding while focus is in
an `input`, `textarea`, or `contenteditable`" and a `⌘S` that saves the open
prompt cannot both hold. Editing a prompt body means the caret _is_ in a
textarea, which is exactly when a user reaches for save — so a blanket guard
makes the two hints in the overlay footer print keys that never fire.

So the guard is per binding, not global. `allowWhileTyping` is a field on the
binding row:

- `⌘K`, `⌘N`, `⌘⇧L` are ignored while typing. They are navigation and would
  otherwise fire mid-sentence.
- `⌘S` and `⌘Enter` are allowed while typing. They are document-scoped actions
  on the thing being typed into, they carry a modifier that no text input
  consumes, and both call `preventDefault` so the browser's own Save dialog does
  not open.

**How a `document` listener reaches the overlay's save.** It does not reach into
the component. `08-24-detail-modal` design D10 adds
`registerDetailActions({ save, copy })` to `usePromptStore`; `PromptDetailModal`
registers on mount and clears on unmount. The shortcut layer reads that slot.
With no overlay open the slot is null and both bindings do nothing, which is the
correct behavior for "save the open prompt" when none is open.

That registry is the mechanism, and it replaces the earlier assumption that the
global layer could call the overlay directly. Nothing in `src/` maps an action
string to a behavior today (finding 2), so the mapping had to be built somewhere;
it is built once, in the store, by the task that owns the overlay.

If `08-24-detail-modal` landed a local listener for these two keys against that
same registry, this task deletes it and adds the two rows here. One key, one
listener.

Platform detection for the modifier symbol is one helper. `⌘` on macOS, `Ctrl`
elsewhere, used for both the hint text and the binding check.

### D3 — Escape order comes from the modal stack, not from a second rule

PRD R7 requires that one Escape close exactly one layer. `08-24-detail-modal`
design D1 builds `Modal` with a module-level stack, and Escape reaches only the
topmost entry.

The palette is a `Modal`. Opening it over the detail overlay pushes it onto the
stack, so Escape closes the palette and the overlay stays. No ordering logic is
written in this task; the correct behavior follows from using the primitive.

If `08-24-detail-modal` has not landed, this task builds `Modal` instead and that
task consumes it. Whichever comes first owns it — but it is written once.

### D4 — The palette queries the backend on its own slice

The palette calls `api.searchPrompts({ keyword, limit: 5 })` through the existing
`PromptApi`, into a palette-local slice — not into `state.prompts` and not
through `setFilters` (finding 4).

Reasons, in order:

- Filtering `state.prompts` locally would search the loaded 50 of an unbounded
  library, so a prompt on page two would be unfindable from the palette. PRD R2
  asks for exactly this to be decided.
- Writing through `setFilters` would change the library's visible result set
  while the palette is open, and closing the palette would leave the library
  filtered by whatever was typed.

The query is debounced 150 ms and sequence-guarded, reusing the pattern
`08-24-library-toolbar` design D1 introduces. `limit: 5` matches the design's
group size and stays inside the backend's `1..=100` clamp
(`services/prompt.rs:923`).

**Match semantics are the library's, and they are not substring.**
`build_fts_match` (`services/prompt.rs:868-878`) splits the keyword on
whitespace, wraps each token in double quotes as a literal FTS5 phrase, and joins
them with implicit AND. So a query matches on whole indexed terms, every term
must be present, and a fragment from the middle of a word does not match. Typing
`prom` does not find a prompt titled `prompt`.

Calling this "substring" would set a wrong expectation in the tests and in the
empty-state copy. The palette uses the same command the library uses, so its
results are consistent with what the search field returns — that consistency is
the property worth having, and it is what the tests assert. No fuzzy scoring and
no second matcher (PRD out of scope); PRD R2 and the out-of-scope note are
amended to say "the library's match semantics" rather than "substring".

### D5 — Every action calls the owning child's action; the palette adds none

| Palette row     | Calls                                                                   |
| --------------- | ----------------------------------------------------------------------- |
| 新建 Prompt     | the create path the toolbar's 新建 button uses                          |
| 只看收藏        | `usePromptStore.selectView("favorites")` (`08-24-shell-sidebar` D4)     |
| 切换列表视图    | `usePromptStore.setViewMode` (`08-24-library-toolbar` D8)               |
| 切换深浅主题    | `useSettingsStore.setPreference("theme", …)` (`08-24-shell-sidebar` D7) |
| a prompt result | `usePromptStore.requestSelectPrompt(id)`, opening the overlay           |

The palette holds no business logic. If a row needs a behavior that does not
exist as an action, the action is added to the owning store, not to the palette.

**The prompt row calls `requestSelectPrompt`, not `selectPrompt`.** An earlier
draft called `selectPrompt` directly, which would discard unsaved edits without
asking: `PromptEditor` resets its draft whenever the selected prompt changes
(`PromptEditor.tsx:513-524`), and `08-24-detail-modal`'s dirty guard covers only
Escape, the scrim, and 关闭. Opening a second prompt from the palette while the
first has unsaved edits would silently lose them — a path the guard was written
to close.

`08-24-detail-modal` design D10 makes `requestSelectPrompt` the guarded entry
point for every caller. The palette is one of them (parent CC11). With no overlay
open the guard slot is null and the call is `selectPrompt` plus a resolved
promise, so nothing about the palette slows down.

If the guard resolves `cancel`, the palette stays open with the row still
highlighted, so the user can decide again rather than losing their query.

### D6 — The toast is one store, one live region, and does not displace in-control feedback

New `src/features/notifications/toastStore.ts` and a `ToastHost` mounted once in
`AppShell`.

- `push({ message, tone })`, auto-dismiss after roughly 4s, and a dismiss
  control. The design concept uses 1.8s; that is too short to read a path or an
  import summary, and a dismissible toast does not need to be brief.
- One `aria-live="polite"` region, `role="status"`. It never takes focus
  (PRD AC7).
- Enter and exit are CSS transitions, so the existing reduced-motion block
  (`globals.css:310-320`) covers them (PRD AC8).

What moves to the toast: bundle export result, bundle import summary, batch
action results, save results. That replaces `transferMessage`
(`PromptsView.tsx:100`, `:346-350`), which is an inline string with no dismissal
and no announcement.

What does **not** move: copy feedback. The archived `08-24-prompt-list-copy`
contract requires success and failure to stay on the control that was used, and a
grid of copy buttons reporting into one shared toast would not say which one
fired (PRD R10).

Placement note: this is a new top-level feature directory for a surface two
features write to. `.trellis/spec/frontend/directory-structure.md` allows shared
placement when a concern is genuinely app-wide; the toast is, because prompts,
settings, and system all report transient results. If it is judged not app-wide
at review, the alternative is `src/components/ui/Toast.tsx` with the store beside
it.

### D7 — The sidebar's quick-jump button is wired here

`08-24-shell-sidebar` R6 leaves the `⌘K` button rendered disabled with a stated
reason. This task enables it and points it at the palette's open action. Verify
it is not left disabled — that is the condition R6 states.

## Data flow

```
document keydown ─→ useGlobalShortcuts (one listener, one binding table)
                          │
        ┌─────────────────┼──────────────────┬───────────────┐
        ↓                 ↓                  ↓               ↓
   toggle palette    new prompt        toggle theme     save / copy
        │                                                (detail overlay)
        ↓
  CommandPalette (Modal, on the stack)
        │
        ├─ query → api.searchPrompts({ keyword, limit: 5 })   [palette slice]
        │            debounced 150ms, sequence-guarded
        │
        └─ actions → the owning store's existing action (D5)

any feature ─→ toastStore.push() ─→ ToastHost (one polite live region)
```

## Compatibility

- No backend change. `shortcut.register` and `shortcut:triggered` are untouched;
  the user's configured shortcuts keep working exactly as they do now.
- No change to `prompt.search`; the palette uses the existing command with a
  smaller limit.
- `PromptsView`'s `transferMessage` is replaced by toast calls. That is a
  deletion inside a file `08-24-library-toolbar` has already reworked — land
  after it.

## Accessibility

- The palette is a `Modal`: focus in on open, trapped, restored on close
  (PRD R5), inherited from the primitive.
- The result list is a `listbox` with `aria-activedescendant` on the input, so
  the highlighted row is announced while focus stays in the query field
  (PRD R6). Arrow keys move the highlight, Enter activates, Escape closes.
- Shortcut hints render the platform's own modifier symbols (PRD R12), from the
  same helper the binding check uses, so the printed symbol and the accepted key
  cannot diverge.
- The toast is `role="status"` with `aria-live="polite"` and never steals focus.

## Test impact

- New: `src/shortcuts/useGlobalShortcuts.test.ts` — the binding table, the
  typing guard, and listener removal on unmount.
- New: `CommandPalette.test.tsx` — query debounce, group rendering, arrow and
  Enter, Escape with the overlay open closing only the palette.
- New: `toastStore.test.ts` and `ToastHost.test.tsx`.
- `PromptsView.layout.test.tsx` — `transferMessage` assertions move to the toast.
- `i18nKeys.test.ts` — palette, action, and toast keys.

## Rollback

Every piece is additive and independently revertible: the shortcut layer without
the palette leaves working bindings; the palette without the toast works; the
toast without the palette works. The one coupling is that removing the palette
leaves the sidebar's quick-jump button without a target, so revert D7 with it.
