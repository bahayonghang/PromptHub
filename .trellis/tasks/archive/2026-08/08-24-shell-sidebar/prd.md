# Sidebar navigation refactor

Child of `08-24-ui-refactor`. Owns parent requirement R1.

## Goal

Move the library's organizational state — saved views, folders, tags, theme —
out of the prompts view and into the application sidebar, matching the design
concept's left rail.

## Ordering

Lands after `08-24-design-tokens`. `08-24-library-toolbar` and
`08-24-library-views` both depend on this move, because after it the main area
no longer owns folder or tag state.

## Background

- `src/components/layout/Sidebar.tsx` renders `PRIMARY_NAV` and `FOOTER_NAV`
  only: two buttons and a collapse toggle. It has no library awareness.
- `src/components/layout/navigation.ts:31-35` holds two `NAV_ENTRIES` and a
  runtime guard that throws when the entry count differs from `APP_VIEWS`
  (`src/store/appStore.ts:12`).
- The folder tree lives inside the prompts view
  (`src/features/prompts/components/FolderTree.tsx`, 11.6K) behind a
  `showFolders` flag (`PromptsView.tsx:93`).
- Tags live in `src/features/prompts/components/TagManager.tsx` and the store's
  `tags` / `toggleTagFilter` (`PromptsView.tsx:50,63`).
- Theme control lives only in settings. The design puts a toggle in the sidebar
  footer.

## Design target

From `PromptHub.dc.html`, the rail is 264px and contains, top to bottom:

1. Product mark, name, version.
2. A quick-jump button labelled `⌘K` that opens the command palette.
3. Saved views with counts: 全部 Prompt, 收藏, 最近使用, 置顶.
4. A `文件夹` group with an add control; each row shows a color dot, the name,
   and a count.
5. A `标签` group rendered as a wrapping chip cloud; each chip shows name and
   count.
6. A footer row: theme toggle and a settings entry.

Selecting a view, a folder, or a tag is a filter action. The design clears the
other axes when a view is picked and toggles folder and tag selection off when
re-clicked.

## Requirements

- R1 (amended by `design.md` D3, D4): The sidebar renders three saved views —
  all, favorites, recent. The 置顶 view in the design concept is dropped:
  `SearchQuery` has no `is_pinned` filter and `search` has no pinned ordering
  term (`src-tauri/src/services/prompt.rs:934-981`), so it has no backing query,
  and the design concept's own prototype rewrites that view to the all view.
  Counts are shown for the two views that are real filters. The recent view is a
  sort preset and shows no count, because its count would equal the all count.
  Selecting a view sets the library scope.
- R1b: Counts are never derived from the loaded page. `state.prompts` holds one
  page of 50 (`promptStore.ts:58`); every count comes from a counted query.
- R2: The sidebar renders the folder list with per-folder counts and an add
  control. Selecting a folder filters the library; selecting it again clears
  the folder filter.
- R3: The sidebar renders the tag cloud with per-tag counts, sorted by count
  descending. Selecting a tag toggles that tag filter.
- R4: Selection state is visible without relying on color alone. The active row
  carries an accessible current/pressed state.
- R5: The sidebar footer holds a theme toggle and a settings entry. The theme
  toggle switches light and dark and persists through the existing theme
  module.
- R6: The quick-jump button is present and opens the command palette. Until
  `08-24-command-palette` lands, it may be rendered disabled with its shortcut
  hint; it must not be a dead control in the final tree.
- R7: The collapse rail keeps working. Collapsed, the sidebar shows icons and
  counts only, and every control keeps an accessible name.
- R8: Folder management actions that exist today (create, rename, delete,
  reorder, drag-reparent with the cycle guard) remain reachable, and the folder
  hierarchy is preserved. The design concept draws a flat list; flattening
  `FolderTree.tsx` would delete shipped behavior.
- R8b: Tag management that exists today (rename, delete, via `TagManager.tsx`)
  remains reachable. The design concept's tag cloud has no management
  affordance; the cloud is the filter surface and the existing manager moves
  with it. Without this the product loses its only path to renaming a tag.
- R8c: Folder counts are direct membership, matching `folder_id = ?`. A parent
  folder does not show a subtree rollup, because clicking it could not reproduce
  that number.
- R9: Scope state is owned by one store, not duplicated between the sidebar and
  the prompts view. The prompts view reads the same state.
- R10: All labels come from i18n keys, present in all 7 bundles.

## Acceptance criteria

- [ ] AC1 (R2, R3, R4): Picking each saved view, each folder, and each tag
      changes the library result set. With no other filter active, the bucket
      count equals the result count.
- [ ] AC1b (R2, design D2): With a keyword also active, the bucket count keeps
      its bucket total and does not fall to the intersected number. The count
      carries an accessible description saying it is the bucket total
      (parent A14).
- [ ] AC1c (R2, design D4b): The scope transition table holds row for row.
      Selecting a folder empties `activeView`, selecting a view clears the folder
      and the tags, folder and tag still combine, and a keyword change alters
      neither. Asserted in `promptStore.test.ts`.
- [ ] AC1d (R2, design D4b): The header scope title reads the folder name when a
      folder is the scope, and the tag name when exactly one tag is. Neither
      titles itself 全部 Prompt.
- [ ] AC2 (R4): Re-selecting an active folder or tag clears that filter.
- [ ] AC3: The whole sidebar is operable by keyboard: tab order is top to
      bottom, focus is visible on every control, and Enter/Space activate.
- [ ] AC4: Collapsing and expanding the sidebar preserves the active scope.
- [ ] AC5: The theme toggle switches themes and survives a restart.
- [ ] AC6: No folder or tag state remains duplicated in `PromptsView`.
      `FolderTree` and `TagManager` have exactly one mount point.
- [ ] AC6b: A count stays correct past the first page. With a folder holding
      more than 50 prompts, the sidebar count equals the folder query's `total`.
- [ ] AC6c: Every mutation that can change membership refreshes the counts:
      save, delete, duplicate, batch move, batch tag, batch delete, tag rename,
      tag delete, folder create, folder delete, bundle import.
- [ ] AC6d: The sidebar renders correct counts with the library locked.
- [ ] AC6e: Folder rename, delete, reorder, and reparent still work from the
      sidebar, and tag rename and delete are still reachable.
- [ ] AC7: `src/components/layout/navigation.test.ts` still passes, or is
      updated together with the `NAV_ENTRIES` invariant it guards.
- [ ] AC8: `just build` and `just test` pass, including
      `src/features/prompts/i18nKeys.test.ts`.

## Out of scope

- The command palette itself (`08-24-command-palette`).
- Main-area header, toolbar, and filter chips (`08-24-library-toolbar`).
- Nested folder hierarchy changes. The existing `folderTree.ts` behavior is
  reused as is.
