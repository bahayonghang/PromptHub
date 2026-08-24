# Implement — library toolbar and filter state

Execution plan for the decisions in `design.md`. Steps are ordered; each gate
passes before the next step starts.

Frontend only. Nothing under `src-tauri/` changes.

Depends on `08-24-design-tokens` (token set) and `08-24-shell-sidebar` (folder
and tag state already moved out of this view).

## Step 0 — Baseline

- [ ] `just build` and `just test` pass before any edit. Report a pre-existing
      failure instead of absorbing it into this diff.
- [ ] Record the click path for each behavior that must survive: keyword search,
      tag filter, favorites filter, sort field, sort direction, page forward and
      back, select prompts, batch move, batch tag, batch delete, bundle export,
      bundle preview, bundle import with each conflict policy, create prompt.

Gate: the list above exists with a click path per item. "No regression" is not
checkable otherwise.

## Step 1 — Store: sequence guard

File: `src/features/prompts/promptStore.ts`

- [ ] Add a module-level monotonic counter. `refreshPrompts` captures the next
      value before awaiting and writes `prompts` / `total` / `offset` only when
      its value is still the newest.
- [ ] Apply the same guard to the retry branch that re-queries the last page
      (`promptStore.ts:220-228`).
- [ ] Guard `load()` with the **same** counter (design D1). It awaits four
      commands (`:188-197`) and then writes the page unconditionally (`:198-206`)
      while the search field is already focusable, so a search issued during
      startup can be overwritten by the initial unfiltered page.
- [ ] In `load()`, guard only `prompts` / `total` / `offset`. Keep the
      `folders` / `tags` / `promptTypeDefinitions` writes unconditional; they have
      no competing writer and the taxonomy must populate even when the page is
      discarded.

Gate: `promptStore.test.ts` covers two cases — two overlapping `refreshPrompts`
calls resolving in reverse order leave the newer result, and a `refreshPrompts`
issued while `load` is still pending survives `load` resolving afterwards, with
`folders` still populated (PRD AC6c, AC6e).

Land this before Step 2. The debounce reduces how often overlap happens; it does
not remove it.

## Step 2 — Store: keyword debounce

File: `src/features/prompts/promptStore.ts`

- [ ] Add `setKeyword(value)`, which updates `filters.keyword` immediately so the
      input stays controlled, then schedules `refreshPrompts` 200 ms later,
      cancelling any pending schedule.
- [ ] Leave `setFilters` immediate. Folder, tag, favorite, sort, and view changes
      keep going through it unchanged.
- [ ] A pending keyword refresh is cancelled when any immediate `setFilters`
      runs, so the two paths cannot both fire for one user action.

Gate: a store test with fake timers asserts that four keystrokes produce one
`searchPrompts` call, and that the query carries the final keyword.

## Step 3 — Store: `batchMode` and `viewMode`

File: `src/features/prompts/promptStore.ts`

- [ ] Add `batchMode: boolean` (default `false`) and `setBatchMode(next)`.
      Leaving batch mode calls the existing `clearPromptSelection`
      (`promptStore.ts:358`).
- [ ] Add `viewMode: "grid" | "list"` (default per design; pick one and state it
      here) and `setViewMode(next)`. Session-only — do not write it through
      `settings.update`.

Gate: `npx tsc --noEmit` passes and `promptStore.test.ts` covers that leaving
batch mode empties `selectedPromptIds`.

## Step 4 — Header

New file: `src/features/prompts/components/LibraryHeader.tsx`

- [ ] Scope title per the table in design D2b, reading the nullable `activeView`
      from `08-24-shell-sidebar` design D4b. Verify `activeView` is
      `SavedView | null` before writing the precedence; against a non-nullable
      field the folder and tag branches are unreachable.
- [ ] Subtitle: result count from `state.total`, plus the library location from
      `useSystemStore.paths`. Render the count alone while `paths` is `null`
      (design D2). Do not render a placeholder path.
- [ ] Move the export, import, and create controls out of `PromptsView.tsx:203-241`
      unchanged in behavior. The import panel keeps its typed path input, its
      conflict-policy select, its preview summary, and its type-definition
      conflict guard.

Do not add an aggregate usage count. There is no source for it (design D2).

Gate: export writes a bundle and reports its path; preview then import completes
under each of `skip`, `duplicate`, and `replace` (PRD AC3).

## Step 5 — Toolbar

New file: `src/features/prompts/components/LibraryToolbar.tsx`

- [ ] Keyword field bound to `setKeyword`, with an inline `shown / total` count
      where `shown` is `state.prompts.length` and `total` is `state.total`.
- [ ] Sort field select and sort direction select, both moved out of the
      `SearchBar` popover (design D5). Keep all four fields and both directions.
- [ ] Grid/list segmented toggle calling `setViewMode`, with `aria-pressed` on
      each segment.
- [ ] Batch toggle calling `setBatchMode`, `aria-pressed`, and a live-region
      announcement of the resulting state.
- [ ] Put the result count in a polite live region.

Gate: changing sort re-queries through the store. A test asserts a new
`searchPrompts` call rather than a client-side reorder of `state.prompts`
(PRD AC5).

## Step 6 — Filter chips

New file: `src/features/prompts/components/FilterChips.tsx`

- [ ] One chip per active axis, per the table in design D6: keyword, folder,
      one per tag, favorites.
- [ ] Each chip's accessible name states the axis and the value.
- [ ] 全部清除 writes `DEFAULT_FILTERS` (`promptStore.ts:49-56`) and resets
      `activeView` to `all`.
- [ ] The row renders nothing when no filter is active.

Do not add a saved-view chip. The view is a preset over these same fields, so a
view chip would duplicate the favorites chip (design D6).

Gate: with a folder, a tag, and a keyword active, three chips render; removing
each clears exactly that filter (PRD AC1).

## Step 7 — Batch bar

File: `src/features/prompts/components/BatchToolbar.tsx`

- [ ] Render it while `batchMode` is true, not while the selection is non-empty
      (`PromptsView.tsx:360`).
- [ ] Keep count, select-all-on-page, move-to-folder, add-tag, and delete with
      its confirmation, all wired to the store actions they use today.
- [ ] Keep the add-tag control a free-text input. The design draws a select over
      existing tags; a select cannot create a tag, and `prompt.batchTag` accepts
      new tags today (`services/prompt.rs:802-837`).
- [ ] Add an exit control calling `setBatchMode(false)`.

Do not add a batch favorite control. There is no `prompt.batchFavorite` command
and a frontend loop would be the only non-atomic action in the bar (design D3).

Gate: entering batch mode, selecting all on the page, applying a tag, and
exiting leaves `selectedPromptIds` empty (PRD AC4).

## Step 8 — Move the loading and empty branches

Files: `src/features/prompts/PromptsView.tsx`,
`src/features/prompts/components/PromptList.tsx`

- [ ] The library container owns the three-way branch from design D7. The empty
      branch is guarded by `!loading`, so it cannot render during a pending load
      (PRD AC6).
- [ ] The empty state offers a clear-filters action reusing the same handler as
      全部清除.
- [ ] Remove the loading and empty branches from `PromptList`
      (`PromptList.tsx:58-77`). It renders items only.
- [ ] Move the corresponding cases from `PromptList.test.tsx` to the container's
      test rather than deleting them.

This task owns the split. `research/shared-ownership.md` in the parent names it,
and `08-24-library-views` consumes the result. That task does not perform the
split even if it starts first; it waits, because doing it in both produces a
conflict in `PromptsView.tsx` and `PromptList.test.tsx`.

Gate: `PromptList` has no `loading` prop path left, and the container test
covers loading, empty, and populated.

## Step 9 — Remove the superseded chrome

File: `src/features/prompts/PromptsView.tsx`

- [ ] Remove the inline pane chrome the new components replaced.
- [ ] Remove the local state that only fed it. Remove only what this change
      orphaned; leave pre-existing dead code alone.
- [ ] `SearchBar`'s popover: remove the sort and favorites controls now owned by
      the toolbar. Remove its tag list only if `08-24-shell-sidebar` has landed
      and the sidebar tag cloud is the live filter surface; otherwise leave it
      and record that here.

Gate: `grep` finds no duplicate sort control and no second favorites toggle.

## Step 10 — i18n

- [ ] Add keys for the scope titles, the subtitle, the result count, the view
      toggle, the batch toggle and its announcement, every chip's accessible
      name, 全部清除, and the empty state.
- [ ] Add every key to all 7 bundles under `src/locales/`.

Gate: `src/features/prompts/i18nKeys.test.ts` passes.

## Step 11 — Full check

- [ ] `just build`
- [ ] `just test`
- [ ] `just ci`
- [ ] Walk the Step 0 regression list end to end.

## Review gates

| After step | Gate                                                           |
| ---------- | -------------------------------------------------------------- |
| 1          | Out-of-order search responses are discarded                    |
| 2          | Four keystrokes produce one query                              |
| 3          | Leaving batch mode clears the selection                        |
| 4          | Import and export unchanged across all three conflict policies |
| 5          | Sort re-queries; it does not reorder the loaded page           |
| 6          | Each chip clears exactly its own axis                          |
| 7          | No non-atomic batch action was added                           |
| 8          | The empty state cannot render while loading                    |
| 11         | `just ci` green; Step 0 regression list clean                  |

## Rollback points

- Steps 1 to 3 are store-only and additive. They can ship alone and be reverted
  independently of the UI.
- Steps 4 to 7 add new components without deleting the old chrome. Until Step 9
  the previous surface still works, so the new one can be validated beside it.
- Step 8 changes a component `08-24-library-views` also touches. Revert it
  together with that task's changes, not alone.

## Open items carried out of this task

- Batch favorite is dropped (design D3). Making it real needs a
  `prompt.batchFavorite` command mirroring `batch_tag` — roughly fifteen lines
  plus a command and an `api.ts` entry, and a second backend contract change
  beyond the one the parent scope allows.
- The header subtitle carries no aggregate usage count (design D2). A summed
  count needs a backend aggregate.
- Selection now requires entering batch mode. Today checkboxes are always
  visible. This is a deliberate behavior change, recorded here so it is not
  filed as a regression.
