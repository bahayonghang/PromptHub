# Design — detail overlay and four tabs

This is the largest child in the tree. It moves a 1172-line component into a
modal, and it is the first place in the product that needs real modal behavior.

## What exists to build on

1. **One dialog exists; no modal behavior does.** `CloseDialog`
   (`src/features/system/components/CloseDialog.tsx`) sets `role="dialog"` and
   `aria-modal="true"` on a fixed overlay. It has no focus trap, no focus
   restore, no Escape handler, and it does not make the background inert. It is
   the markup precedent, not a reusable primitive.

2. **The editor has no dirty tracking.** `PromptEditor` holds `draft` in local
   state and resets it whenever the selected prompt changes
   (`PromptEditor.tsx:513-524`). Nothing compares the draft to the loaded prompt,
   so PRD R2's unsaved-changes behavior is new work, not a re-wiring.

3. **`.prompt-editor` is its own container.** `container-type: inline-size` is on
   the form (`globals.css:184-187`), so every `@container prompt-editor` rule
   (`:266-296`) keeps working wherever the form is mounted. The overlay does not
   break them.

4. **`.prompt-workspace__*` rules do not survive.** The queries at
   `globals.css:204-262` size `__folders`, `__discovery`, `__detail`,
   `__detail-header`, `__detail-title`, `__history`, and `__compact-control` —
   the three-pane layout this task removes. `08-24-shell-sidebar` already deletes
   `__folders`. The rest are orphaned here.

5. **The locked case is handled outside the editor.** `PromptsView.tsx:594-606`
   renders a locked placeholder instead of mounting `PromptEditor` at all. The
   overlay must keep that branch, or a locked prompt would mount an editor over
   a redacted body.

6. **The name "references" is already taken.** The editor's section keyed
   `promptsView.editor.sections.references` (`PromptEditor.tsx:1079-1087`) holds
   images, videos, source, and notes — media provenance. The design's 引用 tab is
   prompt-to-prompt references. Two concepts, one word, one feature.

## Decisions

### D1 — Build one modal primitive, here, and hand it to the palette

`08-24-command-palette` needs the same focus rules (its R5). Writing them twice
guarantees they drift.

New `src/components/ui/Modal.tsx`, app-wide because two features use it:

- Renders through `createPortal` to `document.body`. `SearchBar` already uses
  `createPortal` for its filter panel (`SearchBar.tsx:216`), so the pattern is
  established.
- `role="dialog"`, `aria-modal="true"`, labelled by its heading.
- Focus moves to the first focusable element on open, is trapped on Tab and
  Shift+Tab, and returns to the element that had it before open.
- `aria-hidden` (or `inert` where available) on the **app content region**, not
  on the shell root. See the next paragraph; this distinction is load-bearing.
- Escape and scrim click call `onClose`. It does not decide _whether_ closing is
  allowed; the caller does, which is what lets the unsaved-changes guard work.
- Registers itself on a small module-level stack so the topmost open modal is the
  one Escape closes. `08-24-command-palette` R7 depends on exactly this.

**What gets inerted, and why not the shell root.** `CloseDialog` is a direct
child of the AppShell root `div` (`AppShell.tsx:41`) and does not portal
(`CloseDialog.tsx:19-25`). Inerting the shell root would inert `CloseDialog` too.
That is not hypothetical: the native window close is available at any time, so a
user with the detail overlay open can press the window's close button, the
backend emits `window:close-requested`, `CloseDialog` renders — inside an inert
subtree, unfocusable and unclickable — and neither of its buttons works. The
window then neither closes nor confirms.

`AppShell` therefore grows one wrapper around the region that may be inerted:

```
<div class="shell-root">          ← never inert
  <TitleBar />
  <div id="app-content"> …        ← the Modal marks THIS inert
    Sidebar + Header + main
  </div>
  <CloseDialog />                 ← sibling of the inert region, stays live
  <ToastHost />                   ← same, added by 08-24-command-palette
</div>
```

`Modal` portals to `document.body`, so it is outside `#app-content` and is never
inerted by itself or by a modal beneath it.

`CloseDialog` is not migrated onto the primitive in this task. It works, it is
out of scope, and changing it would put an unrelated dialog in this diff. It is
also not on the modal stack, so Escape handling between the two is not ordered by
the stack — `CloseDialog` has no Escape handler at all today, so there is nothing
to order. If it later gains one, it must join the stack. Record both as
follow-ups.

### D2 — `PromptEditor` splits by section, and the tabs are separate modules

PRD R8 requires decomposition, and R3 requires that no capability be lost. The
safe order is: extract first with no behavior change, then move the extracted
parts into the overlay.

`PromptEditor.tsx` (1172 lines) already contains three components — `FolderPicker`
(`:114`), `PromptTypePicker` (`:296`), and the editor itself (`:499`) — and four
`<section>` blocks: identity (`:671`), definition (`:743`), organization
(`:995`), and media (`:1079`).

Target layout, all under `src/features/prompts/components/detail/`:

| Module                             | Holds                                                      |
| ---------------------------------- | ---------------------------------------------------------- |
| `PromptDetailModal.tsx`            | the `Modal`, the header, the tab strip, the footer         |
| `ContentTab.tsx`                   | body column plus metadata column, composing the sections   |
| `sections/IdentitySection.tsx`     | title, description, private toggle                         |
| `sections/DefinitionSection.tsx`   | text/chat switch, messages, system, user, variable preview |
| `sections/OrganizationSection.tsx` | folder picker, type picker, tags                           |
| `sections/MediaSection.tsx`        | images, videos, source, notes                              |
| `VersionTab.tsx`                   | wraps the existing `VersionHistory`                        |
| `RunTab.tsx`                       | wraps the existing `EvaluationWorkbench`                   |
| `ReferencesTab.tsx`                | new; consumes `reference.list`                             |

`FolderPicker` and `PromptTypePicker` move out as their own files unchanged.

The draft state stays in one place — `PromptDetailModal` — and sections receive
values and change handlers. Splitting the draft across sections would break
`syncVariables`, which reads the system prompt, the user prompt, and every
message together (`PromptEditor.tsx:530-548`).

### D2b — The draft owner is `PromptDetailModal`, and the move happens at step 4

An earlier draft of this plan said both things: `design.md` put the draft in
`PromptDetailModal` and `implement.md` step 2 said it stays in `PromptEditor`.
That is not a difference of wording — the dirty flag, the footer save, the
`⌘S` binding, and the references picker's insert-into-body all need to reach the
same draft, and they cannot if it is one level below the component that owns
them.

**The owner is `PromptDetailModal`.** `PromptEditor` does not survive as a draft
owner; the D2 module table already dissolves it into `ContentTab` plus sections.

The confusion came from ordering, so state the ordering: steps 1 and 2 are a pure
extraction that must not change behavior, and during them the draft is still in
`PromptEditor`. That is an intermediate state, not the design. Step 4 hoists it.
`PromptEditor.test.tsx` passing unchanged through step 2 is what proves the
extraction was pure; the draft move is a separate, later, tested change.

The interface after the move:

```ts
type ContentTabProps = {
  draft: PromptDraft; // owned above
  onChange: (patch: Partial<PromptDraft>) => void;
  errors: ValidationErrors; // from the existing rules at :580-585
  readOnly: boolean; // the header's edit/read toggle
};
```

`PromptDetailModal` holds `draft`, runs `syncVariables` on the whole draft
(`:530-548`), computes `dirty` (D5), builds the create and update payloads
(`:617-652`) unchanged, and calls the store. `ContentTab` and the sections are
controlled and hold no draft state of their own.

Save is async and can fail. `PromptDetailModal.save()` resolves to
`{ ok: true } | { ok: false; errors }`. A failed save does not close the overlay
and does not clear `dirty`; the dirty-guard's "save and close" (D5) closes only
on `ok: true`. Without a return value, save-and-close would close over a
validation error and discard the edit it promised to save.

### D3 — Field checklist, carried verbatim

PRD AC2 asks for a field-by-field checklist. Every field below is editable today
and must be editable after the move. Nothing is deliberately dropped.

| Field                                           | Today                             | After                     |
| ----------------------------------------------- | --------------------------------- | ------------------------- |
| title                                           | identity section                  | 内容 tab, metadata column |
| description                                     | identity section                  | 内容 tab, metadata column |
| isPrivate                                       | identity section                  | 内容 tab, 安全与同步      |
| promptType                                      | organization                      | 内容 tab, metadata column |
| typeDefinitionId + create                       | `PromptTypePicker`                | same, metadata column     |
| chat / text mode                                | definition, `messages.length > 0` | 内容 tab, body column     |
| messages (role + content, add, remove, reorder) | definition                        | 内容 tab, body column     |
| systemPrompt                                    | definition                        | 内容 tab, body column     |
| userPrompt                                      | definition                        | 内容 tab, body column     |
| variables (synced from placeholders)            | `syncVariables`                   | unchanged                 |
| variable preview values                         | definition                        | 内容 tab, variable form   |
| tags (add, remove)                              | organization                      | 内容 tab, 组织方式        |
| folderId + create                               | `FolderPicker`                    | 内容 tab, 组织方式        |
| images                                          | media section                     | 内容 tab, 补充信息        |
| videos                                          | media section                     | 内容 tab, 补充信息        |
| source                                          | media section                     | 内容 tab, 补充信息        |
| notes                                           | media section                     | 内容 tab, 补充信息        |

Validation is unchanged: title non-empty, and either a non-empty user prompt or
every message non-empty (`PromptEditor.tsx:580-585`). The create and update
payloads keep the exact field sets at `:617-652`, so PRD AC3 holds by
construction.

### D4 — Header actions the design drops are kept

The design's header carries copy, an edit/read toggle, favorite, and close. The
current header also carries pin, duplicate, delete, and the editor/evaluation
mode tabs (`PromptsView.tsx:457-588`).

The mode tabs are replaced by the tab strip — that is the point of the redesign.
The other three are capabilities, not chrome:

- **favorite** — header, as the design shows.
- **pin** — header, beside favorite. `is_pinned` has no ordering effect
  (`08-24-shell-sidebar` design D3), but the toggle exists today and the badge in
  `08-24-library-views` reads it.
- **duplicate** and **delete** — an overflow menu in the header. Delete keeps its
  confirmation (`PromptsView.tsx:122-126`).

Dropping them would remove the only path to duplicating or deleting an open
prompt.

### D5 — Closing with unsaved edits asks, and the answer is explicit

New in this task (finding 2). `PromptDetailModal` compares the draft against the
loaded prompt on the same field set the patch uses (D3) and holds a `dirty` flag.

- Not dirty: Escape, scrim, and 关闭 close immediately.
- Dirty: all three raise a confirmation naming the choice — save and close,
  discard and close, keep editing. Not a bare "are you sure".

The confirmation is a second `Modal` on the stack (D1), so Escape inside it
returns to the editor rather than closing everything.

Rejected: autosave on close. The product has an explicit save button and a
version history keyed to saves (`services/version::append_snapshot`); an autosave
would write a version the user did not ask for.

### D6 — The 试跑对比 tab hosts the existing workbench unchanged

`EvaluationWorkbench` takes `prompt` and `versions` (`PromptsView.tsx:608`) and
is 32.3K. The design's two-card layout is presentation over a run engine this
task does not touch (parent assumption A2).

`RunTab` passes the same two props. The tab's count in the strip comes from the
workbench's own run list; if that count is not available without reaching into
the evaluation store, the tab renders no count rather than a wrong one.

One behavior changes: the workbench is currently reachable only when a prompt is
selected and `workspaceMode === "evaluation"`, and selecting that mode hides the
version history (`PromptsView.tsx:481-484`). In the overlay both are tabs, so
they no longer exclude each other. That is an improvement, and it is recorded so
the mode-tab removal is not read as a loss.

### D7 — The 引用 tab, and the word collision

`ReferencesTab` consumes `reference.list` from `08-24-prompt-references` and
renders three groups: outgoing, incoming, and a picker that inserts a token into
the body.

- An unresolved reference is marked by an icon **and** a text reason — missing,
  ambiguous, locked — never by color alone (PRD AC6, parent CC3).
- The picker inserts the `@@Title@@` explicit form from that task's design D7, so
  a title containing spaces resolves.
- Inserting marks the draft dirty. The edge is not written until save, because
  resolution runs inside `prompt.create` / `prompt.update`.

**The collision (finding 6).** The existing i18n key
`promptsView.editor.sections.references` names media provenance. This design
renames that section to `promptsView.editor.sections.attachments` and gives 引用
the `references` name, because the design concept's tab is what a user will call
"引用" from now on. Renaming a key touches all 7 locale bundles; it is mechanical
and it is better than shipping two "references" in one feature.

If the rename is refused, the alternative is naming the new tab `promptLinks`,
which is worse: the UI would say 引用 while the code says something else.

### D8 — The overlay is 1180px wide and the workspace container queries go

`min(1180px, 100%)` on a blurred scrim, per the design. The `prompt-editor`
container queries survive untouched (finding 3).

The `.prompt-workspace__*` rules for `__discovery`, `__detail`,
`__detail-header`, `__detail-title`, `__history`, and `__compact-control` are
orphaned (finding 4). Delete them together with the panes they size, and check
that `08-24-shell-sidebar` already removed `__folders` rather than assuming it.

The `.prompt-workspace` container itself stays: the library grid and list still
size against it.

Reduced motion is already handled globally — `globals.css:310-320` reduces every
animation and transition to 0.01ms under `prefers-reduced-motion: reduce`. The
overlay's entry animation must be a CSS animation or transition, not a
JavaScript-driven one, so that block covers it (PRD R10, AC8).

### D9 — A locked prompt opens the overlay and shows no content

Keep the branch at `PromptsView.tsx:594-606`, moved into the overlay: header,
tab strip, and a locked notice in place of the content tab. The version, run, and
references tabs are disabled with a stated reason, because each would surface
body-derived text.

`copy_secure` refuses a locked source (`services/prompt.rs:1115-1119`), so the
header's 复制正文 is disabled too, consistent with `CopyPromptButton`'s existing
`locked` handling.

### D10 — The dirty guard covers navigation, not only closing, and it is reachable from outside

D5 guards Escape, the scrim, and 关闭. Those are not the only ways an edit is
lost. `PromptEditor` resets its draft whenever the selected prompt changes
(`PromptEditor.tsx:513-524`), so **anything** that calls `selectPrompt(id)` while
the draft is dirty discards it silently:

- clicking another item in the grid or list,
- picking a prompt in the command palette (that task's design D5 calls
  `selectPrompt` directly),
- paging, which changes the visible set,
- a saved-view or folder change that drops the open prompt from the results.

A guard that only covers the close button is not a guard; it is a guard on one of
five doors.

**One guarded entry point.** `usePromptStore` gains
`requestSelectPrompt(id): Promise<boolean>` and a registered guard slot:

```ts
type NavigationGuard = () => Promise<"proceed" | "cancel">;
registerNavigationGuard(guard: NavigationGuard | null): void;
```

- `PromptDetailModal` registers a guard on mount and clears it on unmount. The
  guard resolves `proceed` immediately when not dirty, and otherwise raises the
  same three-answer confirmation D5 uses — save and close, discard, keep editing
  — resolving `cancel` on keep-editing and on a failed save (D2b).
- Every caller listed above calls `requestSelectPrompt`, never `selectPrompt`.
  `selectPrompt` stays as the unguarded primitive the guard itself calls.
- With no overlay mounted there is no guard, so `requestSelectPrompt` is
  `selectPrompt` plus one resolved promise. Nothing regresses when the overlay is
  closed.

This lives in the store rather than in the modal because the callers are in three
different features and must not import the modal to navigate.

**The detail action registry.** `08-24-command-palette` binds `⌘S` and `⌘Enter`
to "save the open prompt" and "copy the open prompt". A `document`-level listener
cannot reach a component's internal callbacks, and there is no action string
mapped to a behavior anywhere in `src/` today. So the same registration shape
carries the two actions:

```ts
type DetailActions = { save: () => Promise<SaveResult>; copy: () => Promise<void> };
registerDetailActions(actions: DetailActions | null): void;
```

`PromptDetailModal` registers on mount and clears on unmount. The shortcut layer
reads the slot; with no overlay open the slot is null and the bindings are inert,
which is the correct behavior for "save the open prompt" when none is open. This
task owns the registry; the palette consumes it (parent
`research/shared-ownership.md`).

**Footer hints.** The footer prints `⌘S` and `⌘Enter` only when a binding exists
for them. If `08-24-command-palette` has not landed, this task either implements
the two bindings locally against the same registry — and the palette then deletes
the local listener rather than adding a second one — or prints no hint. It does
not print a hint for a key nothing handles (that task's R8, parent CC9).

## Data flow

```
Library item / palette / paging
        │
        ↓
requestSelectPrompt(id)  ──→ registered guard?  ──dirty──→ D5 confirmation
        │                         │                              │
        │                      no guard / proceed          cancel │
        ↓                         ↓                              ↓
   selectPrompt(id) → store loads prompt + versions          no change
                                              │
                                              ↓
                                    PromptDetailModal (draft, dirty, activeTab)
                                    registers: navigation guard, detail actions
                                              │
   ┌──────────────┬───────────────┬───────────┴────┬──────────────────┐
   ↓              ↓               ↓                ↓                  ↓
ContentTab    VersionTab       RunTab        ReferencesTab        header/footer
sections      VersionHistory   Evaluation    reference.list       favorite, pin,
              versionDiff.ts   Workbench                          copy, overflow,
   │              │                                               save, close
   └──────────────┴───────────────────────────────────────────────────┘
                          store actions, unchanged:
        savePrompt · createPrompt · createVersion · rollbackVersion
```

## Compatibility

- No backend change in this task.
- Store actions are called with the same arguments as today (D3), so the backend
  call sequence for a save is identical (PRD AC3).
- `EvaluationWorkbench`, `VersionHistory`, `versionDiff.ts`, `VariableEditor`,
  and `MediaRefList` are reused as they are.
- The references tab is disabled with a stated reason if
  `08-24-prompt-references` has not landed (PRD ordering).

## Accessibility

- The tab strip is a real tablist: `role="tablist"` / `role="tab"` /
  `role="tabpanel"`, arrow keys move between tabs, and the panel is labelled by
  its tab. The existing mode tabs already use `role="tab"`
  (`PromptsView.tsx:460-497`) but without arrow-key movement; the strip fixes
  that.
- Escape closes from any tab (PRD AC1), through the modal stack in D1.
- Unresolved references carry a text reason, not only an icon color (D7).
- The footer's shortcut hints render the platform's modifier symbols and must
  correspond to shortcuts that exist. `⌘S` and `⌘Enter` are advertised by the
  design; they are implemented by `08-24-command-palette` R8. Until then, either
  implement them here or do not print them. Do not print an unimplemented hint.

## Test impact

- `PromptEditor.test.tsx` (16.8K) splits alongside the component. Every case
  moves to the module that now owns the behavior; none is deleted (PRD AC9).
- New: `Modal.test.tsx` — focus in, trap, restore, Escape, stack ordering.
- New: `PromptDetailModal.test.tsx` — dirty guard, tab switching, locked prompt.
- New: `ReferencesTab.test.tsx`.
- `PromptsView.layout.test.tsx` — the three-pane assertions are replaced.
- `i18nKeys.test.ts` — new tab and overlay keys, plus the section rename in D7.

## Rollback

The overlay is additive until the inline pane is removed. The safe sequence is:
extract the editor into modules with no behavior change and confirm the tests
pass; mount the overlay beside the existing pane behind a flag; remove the pane
last. Reverting after the last step means restoring the pane's markup, which is
why the extraction commit and the removal commit stay separate.
