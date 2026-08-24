# Library toolbar and filter state

Child of `08-24-ui-refactor`. Owns parent requirements R2, R3, R4, R5.

## Goal

Replace the prompts view's current pane chrome with the design concept's main
header, toolbar, filter-chip row, batch bar, and empty state.

## Ordering

Lands after `08-24-shell-sidebar`, which moves folder and tag state out of this
view, and before `08-24-library-views`. This task owns two things that child
consumes: the `viewMode` store field (design D8) and the `PromptList` loading /
empty split (design D7). The parent's `research/shared-ownership.md` records
both. The two children are not parallel on those files.

## Background

- `src/features/prompts/PromptsView.tsx` (686 lines) holds the pane chrome
  inline, together with `creating`, `compactPane`, `showFolders`,
  `workspaceMode`, `showHistory`, and the bundle import panel state
  (`PromptsView.tsx:91-101`).
- Search lives in `src/features/prompts/components/SearchBar.tsx` (12.4K).
- Batch actions live in `src/features/prompts/components/BatchToolbar.tsx`, fed
  by the store's `selectedPromptIds`, `batchMove`, `batchTag`, `batchDelete`
  (`PromptsView.tsx:57,74-77`).
- Sorting is `SearchQuery.sortBy` over `title | createdAt | updatedAt |
  usageCount` with `sortOrder` (`src/features/prompts/types.ts:183-194`).
- Paging is `PROMPT_PAGE_SIZE` with `loadPreviousPage` / `loadNextPage`
  (`PromptsView.tsx:64-65`).
- Import and export run through `exportBundle` / `previewBundle` /
  `importBundle` (`PromptsView.tsx:79-81`), backed by
  `prompt.bundleExport` / `prompt.bundlePreview` / `prompt.bundleImport`
  (`src-tauri/src/commands/portable.rs:11,30,41`).

## Design target

Header row: scope title, a one-line statistic subtitle, then 导入, 导出, and a
filled 新建 Prompt button.

Toolbar row: a search field with an inline result count (`shown / total`), a
sort select, a grid/list segmented toggle, and a 批量整理 toggle.

Filter row (only when a filter is active): one removable chip per active filter
plus 全部清除.

Batch bar (only in batch mode): selected count, 选中当前全部, move-to-folder
select, add-tag select, favorite, delete, and 退出批量.

Empty state: a headline, a hint line, and a clear-filters button.

## Requirements

- R1 (amended by `design.md` D2): The header shows the active scope title,
  derived from the sidebar scope (view, folder, or tag), and a subtitle carrying
  the result count and the local library location, since local-first must be
  tangible (PRODUCT.md). The aggregate usage count is dropped: `PromptPage`
  carries no sum of `usage_count` (`types.ts:201-207`) and `state.prompts` is one
  page of 50, so any sum shown beside an exact total would have a different
  scope from it.
- R2: 导入 and 导出 keep their current bundle behavior, including the preview
  and conflict-policy step, and the typed import path. There is no file dialog
  today (`src-tauri/src/commands/portable.rs:17-27`). Only placement and styling
  change.
- R3: 新建 Prompt opens the detail overlay on a new draft. Until
  `08-24-detail-modal` lands, it keeps the current create path.
- R4 (amended by `design.md` D1): The search field shows `shown / total`. The
  keyword path gains a 200 ms debounce and the search gains a request-sequence
  guard. Neither exists today: `SearchBar.tsx:184` issues one `prompt.search` per
  keystroke and `refreshPrompts` (`promptStore.ts:212-233`) writes whichever
  response arrives last. Keyword semantics are unchanged.
- R5: The sort control exposes the four backed sort fields. The design's
  "最近使用" option maps to `updatedAt` (parent assumption A1). The visible
  label must not claim last-used ordering that the data cannot support.
- R6: The view toggle switches grid and list. The flag persists for the session
  and is read by `08-24-library-views`.
- R7: Batch mode is a toggle. Entering it reveals per-row checkboxes; leaving it
  clears the selection. This is a behavior change: today the checkboxes render
  unconditionally (`PromptList.tsx:91-97`) and the batch bar appears whenever the
  selection is non-empty (`PromptsView.tsx:360`).
- R8 (amended by `design.md` D3): The batch bar exposes count,
  select-all-on-page, move to folder, add tag, and delete. Delete keeps a
  confirmation. Batch favorite is dropped: no `prompt.batchFavorite` command
  exists, and a frontend loop over `prompt.update` would be the only non-atomic
  action in a bar whose other three actions each run in one transaction
  (`services/prompt.rs:771-850`). The per-row favorite toggle keeps the
  capability.
- R8b: The add-tag control stays a free-text input. The design concept draws a
  select over existing tags, which cannot create one; `prompt.batchTag` accepts
  new tags today (`services/prompt.rs:802-837`).
- R9 (amended by `design.md` D6): Every active filter renders as a removable
  chip on four axes: keyword, folder, each tag, and favorites. 全部清除 resets
  all of them and returns `activeView` to `all`. No chip is rendered for a saved
  view. 收藏 is already the favorites chip, 最近 sets sort only and is shown by
  the sort control, and 全部 is the absence of chips. R9 previously listed a
  saved-view chip that `design.md` D6 forbids; the design is correct and this
  requirement follows it (parent A13).
- R10: The empty state appears only when a load finished with zero results, not
  while loading. It offers a clear-filters action.
- R11: Paging controls remain reachable and keep the counted-page contract.
- R12: All labels come from i18n keys, present in all 7 bundles.

## Acceptance criteria

Each criterion names the requirement it closes.

- [ ] AC1 (R9): With a folder, a tag, and a keyword active, three chips render;
      removing each clears exactly that filter; 全部清除 clears all three and
      returns `activeView` to `all`.
- [ ] AC1b (R9): No chip renders for a saved view in any of the three view
      states. Selecting 最近 renders no chip and moves the sort control to
      最近更新 / 降序.
- [ ] AC2 (R3): The `shown` count in the toolbar equals the number of rendered
      items, and `total` equals the store's `total`. This is the toolbar's own
      intersected count; it is not asserted equal to a sidebar bucket count,
      which is a bucket total (parent A14).
- [ ] AC2b (R2, design D2b): The header title reads the folder name under a
      folder scope and the tag under a single-tag scope, and 全部 Prompt only
      when no folder, no tag, and `activeView === "all"`.
- [ ] AC3 (R2): Import and export complete with no behavior change against the
      pre-refactor flow, including the conflict-policy choice.
- [ ] AC4 (R5, R6): Entering batch mode, selecting all on the page, applying a
      tag, and exiting leaves no stale selection.
- [ ] AC5 (R3): Changing sort re-queries through the store, not by sorting the
      already-loaded page in the component.
- [ ] AC6 (R10): The empty state never renders during a pending load.
- [ ] AC6b (R4): Typing four characters into the keyword field issues one
      `prompt.search`, and the result set matches the final keyword.
- [ ] AC6c (R4): Two overlapping searches resolving out of order leave the store
      holding the newer result.
- [ ] AC6d (R2): The header subtitle renders the result count alone while the
      runtime paths report is unavailable, and never a placeholder path.
- [ ] AC6e (R4): A search issued while the initial `load()` is still pending
      survives that load resolving afterwards. `folders`, `tags`, and
      `promptTypeDefinitions` are still populated after the discarded page.
- [ ] AC7 (R3, R5, R12): The whole toolbar is keyboard operable and every control
      has an accessible name. Batch mode state is announced, not shown by color
      alone.
- [ ] AC8: `just build` and `just test` pass, including
      `PromptsView.layout.test.tsx` and `SearchBar.test.tsx` or their updates.

## Out of scope

- Rendering the items themselves (`08-24-library-views`).
- The detail overlay (`08-24-detail-modal`).
- Adding a `last_used_at` field (parent A1).
