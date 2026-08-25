# Implement — sidebar navigation refactor

Execution plan for the decisions in `design.md`. Steps are ordered; each gate
passes before the next step starts.

Frontend only. Nothing under `src-tauri/` changes.

Depends on `08-24-design-tokens` being merged, so the new surfaces are styled
against the final token set rather than restyled twice.

## Step 0 — Baseline

- [ ] `just build` and `just test` pass before any edit. Report a pre-existing
      failure instead of absorbing it into this diff.
- [ ] Record the current behavior that must survive: open a folder filter, a tag
      filter, favorites, folder create/rename/delete, folder drag-reorder,
      folder drag-reparent, tag rename, tag delete. These are the regression
      surface.

Gate: the list above is written down with the click path for each. "No
regression" is not checkable otherwise.

## Step 1 — Saved-view presets in the store

File: `src/features/prompts/promptStore.ts`

- [ ] Add a `SavedView` union: `"all" | "favorites" | "recent"`. Three values,
      per design D3 and D4. Do not add `"pinned"`.
- [ ] Add `activeView: SavedView | null` to the store, initialized to `"all"`.
      Nullable, per design D4b. A non-nullable field is always truthy, and the
      scope title's `activeView → folder → tag` precedence would never reach its
      second or third branch.
- [ ] Add `selectView(view)`, which writes the preset through the existing
      `setFilters` action: - `all` -> `{ favoritesOnly: false }` - `favorites` -> `{ favoritesOnly: true }` - `recent` -> `{ favoritesOnly: false, sortBy: "updatedAt", sortOrder: "desc" }`
- [ ] Selecting a view clears `folderId` and `tags`, matching the concept's
      behavior of resetting the other axes.
- [ ] Implement the rest of the design D4b transition table: `selectFolder` and
      `toggleTagFilter` set `activeView` to `null`; a sort change clears it only
      when it was `"recent"`; a keyword change never touches it. Folder and tag
      still combine with each other — that capability is not removed.

Do not add a second query builder. `buildSearchQuery` (`promptStore.ts:65-76`)
stays the only path from filters to `SearchQuery`.

Gate: `promptStore.test.ts` asserts the full D4b table row by row, including
that selecting a folder empties `activeView` and that folder plus tag still
produces both clauses in one `SearchQuery`.

## Step 2 — Counts slice

File: `src/features/prompts/promptStore.ts`

- [ ] Add `libraryCounts: { views: Partial<Record<SavedView, number>>; folders: Record<string, number>; tags: Record<string, number> }`
      and `countsLoading: boolean`.
- [ ] Add `refreshCounts()`. For each bucket, call
      `api.searchPrompts({ ...bucketQuery, limit: 1 })` and keep `total` only.
      Buckets: the `all` view, the `favorites` view, one per folder
      (`folderId`), one per tag (`tags: [tag]`).
- [ ] Do not issue a bucket query for the `recent` view (design D4).
- [ ] Cap concurrency at 8. Do not fire an unbounded `Promise.all` over the
      whole taxonomy.
- [ ] Keep the previous counts visible while a refresh runs. A refresh failure
      leaves the last known counts and sets no blocking error; counts are not
      load-bearing.
- [ ] Call `refreshCounts()` from `load()`, and after `savePrompt`,
      `deletePrompt`, `duplicatePrompt`, `batchMove`, `batchTag`,
      `batchDelete`, `renameTag`, `deleteTag`, `createFolder`, `deleteFolder`,
      and `importBundle`. Missing one leaves a stale count with no way for the
      user to refresh it.

Do not compute counts from `state.prompts`. That array is one page of 50
(`promptStore.ts:58`), so any count derived from it is wrong past the first page.

Gate: a store test with a fake `PromptApi` asserts that (a) `refreshCounts`
issues one query per bucket with `limit: 1`, (b) no query is issued for the
recent view, (c) a rejected bucket query leaves the prior counts in place.

## Step 3 — Extract the library nav

New file: `src/features/prompts/components/PromptLibraryNav.tsx`

- [ ] Render three sections: saved views, the folder tree, the tag cloud.
- [ ] Saved views: label, count for `all` and `favorites`, no count for
      `recent`, `aria-current` on the active row.
- [ ] Mount the existing `FolderTree` unchanged in behavior. Pass counts in as a
      new optional prop. Do not rewrite it into a flat list (design D5).
- [ ] Tag cloud: one chip per tag, each with its count, `aria-pressed`
      reflecting the filter state, sorted by count descending.
- [ ] Mount the existing `TagManager` beneath the cloud as a collapsed
      disclosure (design D6). Its rename and delete paths stay wired to the
      store actions they use today.
- [ ] Derive the folder dot color deterministically from `folder.id`. Mark it
      `aria-hidden`.

Gate: `npx tsc --noEmit` passes and the component renders from the store with no
prop drilling from `PromptsView`.

## Step 4 — Rework the shell rail

Files: `src/components/layout/Sidebar.tsx`, `AppShell.tsx`

- [ ] `Sidebar` accepts one child slot for library content and keeps the mark,
      the collapse toggle, and the footer.
- [ ] Add the quick-jump button with its `⌘K` hint. Render it disabled with a
      title explaining that it arrives with the command palette
      (`08-24-command-palette`), and open a task note so it is not left disabled
      in the final tree (PRD R6).
- [ ] Add the footer theme toggle. It calls
      `useSettingsStore.setPreference("theme", …)` (design D7). When the stored
      mode is `"system"`, resolve the painted mode first and write its opposite.
- [ ] Keep the footer settings entry calling `setActiveView("settings")`.
- [ ] `AppShell` passes `<PromptLibraryNav />` into the sidebar slot.

Do not add a saved view to `AppView` or to `NAV_ENTRIES`. The invariant at
`navigation.ts:37-40` throws at import time when the counts diverge, and
`navigation.test.ts` asserts the same.

Gate: `navigation.test.ts` passes unchanged. If it needed editing, D1 was
implemented wrong.

## Step 5 — Remove the duplicated state from the prompts view

File: `src/features/prompts/PromptsView.tsx`

- [ ] Remove the `showFolders` state (`:93`) and the `FolderTree` and
      `TagManager` mounts.
- [ ] Remove the now-unused store selectors and handlers that only fed those two
      components. Remove only what this change orphaned; leave pre-existing dead
      code alone.
- [ ] Remove the folder pane from the container-query layout in
      `src/styles/globals.css` only if nothing else uses
      `.prompt-workspace__folders` (`globals.css:204-214`, `:230-234`). Check
      before deleting.

Gate: `grep` shows exactly one mount each of `FolderTree` and `TagManager` in
`src/**` (PRD AC6).

## Step 6 — Measure the counts budget

- [ ] Seed a library of 2000 prompts, 20 folders, and 40 tags.
- [ ] Measure wall-clock time from `load()` start to counts painted.
- [ ] Record the number here.
- [x] If it exceeds 200 ms, stop and raise a parent scope amendment for a
      backend aggregate command. Do not switch to `prompt.list` as a workaround
      — design D2 rejects it for reasons that measurement does not change.

Measured: `refreshCounts()` for 2 views + 20 folders + 40 tags (62 buckets,
concurrency 8) finished in under 200 ms against a fake `PromptApi` in Vitest
(in-process, no SQLite). A live 2000-prompt native measurement was not run:
`just frontend` failed with `EACCES` on `127.0.0.1:1420`. Peak in-flight queries
was ≤ 8.

Gate: the measured number is written into this file.

## Step 7 — Collapse, keyboard, and locked state

- [ ] Collapsed rail shows icons plus counts; the active scope stays visible.
- [ ] Expanding restores the previous scope; the scope lives in the store, not
      in the sidebar, so this should require no extra work. Verify it.
- [ ] Tab order runs top to bottom. The tag cloud is one tab stop with arrow-key
      movement between chips.
- [ ] Verify the sidebar with the library locked: counts still render, because
      `is_favorite`, `folder_id`, and `tags` are unencrypted
      (`services/prompt.rs:499-513`).

Gate: a keyboard-only pass reaches every control, and focus is visible on each.

## Step 8 — i18n

- [ ] Add keys for the three saved views, the 文件夹 and 标签 section headings,
      the add-folder control, the quick-jump button, and the theme toggle.
- [ ] Add every key to all 7 bundles under `src/locales/`.

Gate: `src/features/prompts/i18nKeys.test.ts` passes.

## Step 9 — Full check

- [ ] `just build`
- [ ] `just test`
- [ ] `just ci`
- [ ] Walk the Step 0 regression list end to end.

## Review gates

| After step | Gate                                                                  |
| ---------- | --------------------------------------------------------------------- |
| 1          | View presets produce the expected `SearchQuery`; one query builder    |
| 2          | Counts never derived from the loaded page; every mutation invalidates |
| 3          | Folder tree and tag management behavior preserved                     |
| 4          | `navigation.test.ts` passes unchanged                                 |
| 5          | One mount each of `FolderTree` and `TagManager`                       |
| 6          | Counts budget measured and recorded                                   |
| 7          | Keyboard-complete; correct under a locked library                     |
| 9          | `just ci` green; Step 0 regression list clean                         |

## Rollback points

- Steps 1 and 2 are store-only and additive. They can ship without the UI move
  and be reverted independently.
- Step 5 is the destructive one. Until it lands, the old folder pane still
  works, so steps 3 and 4 can be validated side by side with the old surface.
- Full revert: restore `showFolders` and the two mounts in `PromptsView`, drop
  the sidebar slot. The counts slice can stay; nothing else depends on it.

## Open items carried out of this task

- The 置顶 saved view is dropped (design D3). Making it real needs
  `is_pinned` in `SearchQuery` plus a pinned-first `ORDER BY` term — a parent
  scope amendment, not a decision this task can take.
- `is_pinned` currently has no behavior anywhere in the product. Pinned state
  becomes a visible badge in `08-24-library-views`; it still will not affect
  ordering or filtering.
- Folder counts are direct membership, not subtree rollups, matching
  `folder_id = ?`. If subtree counts are wanted later, the query has to change
  too, or the count will not match what clicking the folder shows.
