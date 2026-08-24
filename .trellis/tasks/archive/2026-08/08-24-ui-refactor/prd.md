# PromptHub UI and feature refactor

Parent task. It owns the source requirement set, the child task map, the
cross-child acceptance criteria, and the final integration review. It has no
direct implementation work of its own.

## Goal

Rebuild the PromptHub desktop interface to match the Claude Design concept
`PromptHub.dc.html`, and add the backend prompt-reference capability that the
concept's "引用" tab depends on.

## Source

- Design project: `PromptHub 界面重设计`, id `b31c96f8-b82e-4d92-9e47-9734e0d3e899`.
- Design file: `PromptHub.dc.html` (read 2026-08-24).
- Screens in the file: shell + library, detail overlay with four tabs, command
  palette, batch bar, toast.

The design file is a concept prototype with mock data. It defines layout,
information architecture, interaction, and tokens. It is not source code to
port. Data must come from the existing stores and the Runtime Bridge.

The share link requires a login, so a reviewer cannot open it. Every concept
value the children's designs cite is transcribed into
`research/design-concept-values.md`, together with the deviations and their
reasons. That file is the reviewable record of the concept. It is a
transcription, not an export: it can be checked against the plan, and it cannot
be checked against the original. Exporting the file or capturing the five
screens into `research/` would close that gap and is not done here.

## Background — current state

Shell:

- `src/store/appStore.ts:9-15` — `AppView` is `"prompts" | "settings"` only.
- `src/components/layout/navigation.ts:31-35` — one nav entry per view, guarded
  at runtime by `NAV_ENTRIES.length !== APP_VIEWS.length`.
- `src/components/layout/Sidebar.tsx` — the sidebar renders only those two nav
  buttons plus a collapse toggle (`w-60` / `w-16` rail). It holds no folders,
  no tags, no theme control.
- `src/components/layout/AppShell.tsx` — `TitleBar` + `Sidebar` + `Header` +
  active view; `VIEW_COMPONENTS` maps the two views.

Library and detail:

- `src/features/prompts/PromptsView.tsx` (686 lines) is a three-pane layout:
  `FolderTree` | `PromptList` | editor pane. Local state drives `creating`,
  `compactPane`, `showFolders`, `workspaceMode`, `showHistory`, and the bundle
  import panel (`PromptsView.tsx:91-101`).
- `workspaceMode === "evaluation"` swaps `PromptEditor` for
  `EvaluationWorkbench` in the same pane (`PromptsView.tsx:607-611`). The
  evaluation workbench is reachable only from that toggle.
- `PromptList.tsx` renders a single vertical list. There is no grid view.
- `PromptEditor.tsx` is 1172 lines and holds the whole detail surface inline.
- `VersionHistory.tsx` is shown through the `showHistory` flag, not as a tab.
- There is no command palette and no global shortcut layer.

Tokens:

- `src/styles/globals.css:18-73` (light) and `:85-126` (dark) declare
  channel-only HSL tokens such as `--background: 220 14% 96%`.
- `tailwind.config.js:12-53` maps utilities to `hsl(var(--token))`, so alpha
  variants (`bg-primary/20`) work.
- Theme switching is a `.dark` class on `document.documentElement`
  (`src/theme/index.ts`), not a `data-theme` attribute.
- `src/appearance/index.ts:420-429` overrides `--primary` and
  `--primary-foreground` at runtime; `:533-548` overrides `--font-display`,
  `--font-body`, `--font-scale`, `--density-padding`, and `--density-gap`.

Backend:

- `src-tauri/src/models/prompt.rs:33-83` — the `Prompt` model already carries
  `is_favorite` (`:62`), `is_pinned` (`:64`), `usage_count` (`:72`), tags,
  folder, variables, images/videos, source, notes, and `current_version`.
- `src-tauri/src/commands/prompt.rs:93` — `prompt.copy` already substitutes
  placeholders.
- `src-tauri/src/storage/mod.rs:451` — the `prompts` table; `:55` — the
  `MIGRATIONS` list keyed on SQLite `user_version` (`:336`).
- No table, service, or command represents a reference between two prompts.

## Scope decisions (confirmed 2026-08-24)

1. Scope is the frontend plus one backend addition: prompt-to-prompt references.
   No other backend contract changes.
2. The design token set is adopted in full. The current palette is replaced in
   both themes.
3. The work is split into a parent task with seven child tasks.

## Assumptions

- A1: The design's "最近使用" sort and "最近使用" sidebar view have no backing
  field. `SearchQuery` sorts by `title | createdAt | updatedAt | usageCount`
  (`src/features/prompts/types.ts:183-194`) and the model has no `last_used_at`.
  Both surfaces map to `updatedAt`. Adding `last_used_at` is out of scope.
- A2: The design's "试跑对比" tab is backed by the existing
  `EvaluationWorkbench`, not by a new two-model runner.
- A3: The design's fixed `min-width:1280px` shell is a concept constraint. The
  refactor keeps the existing responsive and container-query behavior.
- A4: The design's mock 中文 strings are content, not copy decisions. Every
  visible string stays behind an i18n key across all 7 locale bundles.
- A5: The design's 置顶 saved view is dropped. `SearchQuery` has no `is_pinned`
  field and `search` has no pinned ordering term
  (`src-tauri/src/services/prompt.rs:934-981`), so `is_pinned` is a stored flag
  with no behavior anywhere in the product. Making the view real needs an
  `is_pinned` filter plus a pinned-first `ORDER BY` term — roughly ten lines in
  `services/prompt.rs`, but a second backend contract change beyond the one this
  scope allows. Pinned state stays a badge owned by `08-24-library-views`.
  Raising this as a scope amendment is open to the user.
- A6: The design's 草稿 badge is dropped. There is no `is_draft` column
  (`src-tauri/src/storage/mod.rs:451-475`), no field on the model, and no service
  that sets one. `currentVersion === 0` means "never snapshotted", not "draft".
  See `08-24-library-views` design D2.
- A7: Batch favorite is dropped from the batch bar. No `prompt.batchFavorite`
  command exists, and a frontend loop over `prompt.update` would be the only
  non-atomic action beside three that each run in one transaction
  (`services/prompt.rs:771-850`). The per-row toggle keeps the capability.
  Adding the command is roughly fifteen lines and a second backend contract
  change; raising it as a scope amendment is open to the user. See
  `08-24-library-toolbar` design D3.
- A8: `prompt.copy` has no caller today. Every copy runs through the frontend
  `buildPromptCopyText` (`promptText.ts:138-157`), the declared mirror type is
  wrong (`api.ts:38` says `Promise<string>`, the command returns `PromptCopy`),
  and `copy_secure` ignores `prompt.messages` (`services/prompt.rs:1108-1127`).
  R12 is therefore unobservable unless `prompt.copy` becomes the real copy path.
  `08-24-prompt-references` design D4 extends `PromptCopy` with `messages` and
  `unexpanded` and corrects the mirror type. This changes an existing DTO, which
  is safe because nothing reads it.
- A9: No reusable modal primitive exists. `CloseDialog` sets `role="dialog"` and
  `aria-modal` with no focus trap, no focus restore, no Escape handler, and no
  background inerting. `08-24-detail-modal` design D1 builds one with a modal
  stack, and `08-24-command-palette` reuses it. Whichever child lands first owns
  it; it is written once.
- A10: The design's aggregate usage count in the header subtitle is dropped.
  `PromptPage` carries no sum of `usage_count` (`types.ts:201-207`) and
  `state.prompts` is one page of 50. See `08-24-library-toolbar` design D2.
- A11: `08-24-prompt-references` owns migrating `CopyPromptButton` from the
  frontend `buildPromptCopyText` to the backend `prompt.copy`. Earlier planning
  left this to "whichever library child lands later", which named no owner, and
  `08-24-library-views` design D5 stated the opposite — that the control keeps
  building text locally. With no owner, R12 would ship as backend code no copy
  button reaches. The references child already edits `api.ts` and
  `promptText.ts`, so the switch is contained in one diff and R12 becomes
  observable at that child's completion.
- A12: The library's scope is one state model, not three independent axes.
  `activeView` is nullable and is cleared whenever a folder or tag filter is set
  by any path other than `selectView`. Without that, `activeView` defaults to
  `"all"` and never empties, so a scope title with precedence
  `activeView → folder → tag` can never reach its second or third branch. See
  `08-24-shell-sidebar` design D3b and `08-24-library-toolbar` design D2b.
- A13: The saved view 最近 has no filter chip. It sets `sortBy: "updatedAt"` and
  `sortOrder: "desc"`, and the sort control is its visible representation. 收藏
  is represented by the favorites chip and 全部 by the absence of chips. No view
  chip is rendered for any of the three; R4 is amended accordingly.
- A14: Sidebar bucket counts are per-bucket totals, each computed with only that
  bucket's own filter applied. They are not intersected with the active keyword,
  folder, or tags. A folder count answers "how many prompts are in this folder",
  not "how many would remain if I also clicked it". The result count in the
  toolbar is the intersected number; the two are different questions and are
  allowed to disagree. See `08-24-shell-sidebar` design D5.

## Requirements

Numbered from the design file. Each requirement is owned by exactly one child
task; see the task map.

- R1: The sidebar holds the library's organizational state: saved views with
  counts, the folder list with counts, a tag cloud with counts, a theme toggle,
  a settings entry, and a command-palette entry. The saved views are all,
  favorites, and recent; see assumption A5 for the dropped pinned view.
- R2: The main area header shows the active scope title, a one-line statistic
  subtitle, import/export controls, and a primary "new prompt" action.
- R3: The toolbar holds keyword search with a result count, a sort control, a
  grid/list view toggle, and a batch-mode toggle.
- R4: Active filters render as removable chips with a "clear all" action. The
  chip axes are keyword, folder, each tag, and favorites. A saved view gets no
  chip of its own; see assumption A13.
- R5: Batch mode shows a selection bar with count, select-all, move to folder,
  add tag, delete, and exit; see assumption A7 for the dropped favorite action.
- R6: The library renders in two interchangeable modes: a card grid and a dense
  table-like list. Both show title, description, tags, type, usage count, last
  update, and version. Both support the batch checkbox state.
- R7: Opening a prompt raises a centered overlay, not an inline pane. The
  overlay has a header (title, version, metadata, copy, edit toggle, favorite,
  close), a tab strip, a scrolling body, and a footer with a shortcut hint and
  save.
- R8: The overlay's tabs are content, version history, run comparison, and
  references.
- R9: The content tab shows the prompt body beside a metadata column (title,
  description, type, folder, tags) plus collapsible sections for organization,
  extra fields, and security. It also shows a variable form with a
  "fill variables and copy" action.
- R10: The version-history tab shows a version timeline beside a line-level
  diff, with copy-this-version and roll-back actions.
- R11: The references tab shows what this prompt references, what references it,
  and a picker that inserts a reference.
- R12: A prompt body may reference another prompt by `@@<title>`. Copy expands
  every reference to the referenced prompt's body. Making this observable
  requires `prompt.copy` to become the real copy path; see assumption A8.
  `08-24-prompt-references` owns that switch end to end, including migrating
  `CopyPromptButton` (assumption A11). R12 is verified inside that child, not
  deferred to a later one.
- R13: `Cmd/Ctrl+K` opens a command palette that searches prompts and offers
  actions. `Escape` closes the palette and the overlay.
- R14: Transient results — batch action, save, import, export — report through a
  toast. Copy is excluded: the archived `08-24-prompt-list-copy` contract keeps
  copy success and failure on the control that was used, and a grid of copy
  buttons reporting into one shared region would not say which one fired. See
  `08-24-command-palette` design D6.
- R15: Both themes use the design token set. Dark is the design's primary
  presentation.

## Task map

| Order | Child task | Deliverable | Owns |
|-------|-----------|-------------|------|
| 1 | `08-24-design-tokens` | Token layer and typography baseline | R15 |
| 2 | `08-24-shell-sidebar` | Sidebar navigation and library scope state | R1 |
| 3 | `08-24-library-toolbar` | Header, toolbar, filter chips, batch bar | R2, R3, R4, R5 |
| 4 | `08-24-library-views` | Grid and list views | R6 |
| 5 | `08-24-prompt-references` | Backend reference model and copy expansion | R12 |
| 6 | `08-24-detail-modal` | Detail overlay and four tabs | R7, R8, R9, R10, R11 |
| 7 | `08-24-command-palette` | Command palette, shortcuts, toast | R13, R14 |

Ordering constraints:

- `design-tokens` lands first. Every other child styles against the new tokens.
- `shell-sidebar` moves folder and tag state out of `PromptsView`;
  `library-toolbar` and `library-views` both depend on that move.
- `detail-modal` needs `prompt-references` for its references tab. The other
  three tabs do not depend on it.
- `command-palette` lands last; it drives the surfaces the earlier children own.

Parent/child links are not a dependency system. Each child repeats its own
ordering constraint in its `prd.md`.

Shared files, shared store fields, and shared primitives have exactly one named
owner in `research/shared-ownership.md`. A child does not renegotiate ownership
at implementation time; that is how the same split gets performed twice.

## Cross-child acceptance criteria

- [ ] CC1: `just ci` passes on the integration branch after every child merges.
- [ ] CC2: No visible string is hard-coded. Every new key exists in all 7
      bundles under `src/locales/`, and each feature's `i18nKeys.test.ts` passes.
- [ ] CC3: Every workflow in the design is reachable by keyboard alone, focus
      stays visible, and no state is signalled by color alone (PRODUCT.md,
      WCAG 2.2 AA).
- [ ] CC4: Every backend call still goes through the Runtime Bridge. No new
      `invoke` call site outside a feature `api.ts`.
- [ ] CC5: `src/code_map.md` and the `.trellis/spec/frontend/` guides describe
      the post-refactor structure.
- [ ] CC6: The prompt library opens, filters, edits, versions, evaluates, and
      copies with no regression against the pre-refactor behavior.
- [ ] CC7: Both themes render every new surface with tokens only; no literal
      hex or rgba color remains in component files.
- [ ] CC8: Copying a prompt whose body holds `@@Title` inlines the target's body,
      from the library item and from the detail overlay, in text mode and in chat
      mode (R12, assumption A11).
- [ ] CC9: Every shortcut hint printed anywhere in the product has a row in the
      one binding table and a test. A printed hint that does nothing is a defect
      (`08-24-command-palette` R8).
- [ ] CC10: One `Escape` closes exactly one layer with the detail overlay, the
      dirty-guard confirmation, the palette, and the native close dialog in every
      combination that can occur.
- [ ] CC11: Opening a different prompt while the detail overlay holds unsaved
      edits routes through the dirty guard, from the library, from the palette,
      and from a paging control (assumption A12, `08-24-detail-modal` D10).
- [ ] CC12: No shared file in `research/shared-ownership.md` was split by two
      children, and no primitive in it was written twice.

## Out of scope

- A `last_used_at` field and true last-used sorting (see A1).
- A new two-model run engine (see A2).
- Changes to the evaluation, skills, settings, or updater backends.
- The design file's `support.js` runtime, its mock data, and its
  `sc-for` / `sc-if` template elements.
