# Design — sidebar navigation refactor

## What the backend can and cannot answer

The sidebar in the design concept is a filter surface with counts. Three of its
four elements need data the current contracts do not supply. This section
establishes what is true before any decision is made.

`SearchQuery` (`src/features/prompts/types.ts:189-198`, mirrored at
`src-tauri/src/models/prompt.rs:165-182`) accepts exactly:
`keyword`, `tags`, `folderId`, `isFavorite`, `sortBy`, `sortOrder`, `limit`,
`offset`.

`search` (`src-tauri/src/services/prompt.rs:919-1021`) builds its `WHERE` from
four clauses only — FTS match, tag membership via `json_each`, `folder_id = ?`,
and `is_favorite = ?` (`:934-952`). It then runs `SELECT COUNT(*)` over the same
`FROM` (`:958-971`) and returns it as `PromptPage.total`.

Consequences:

1. **There is no `is_pinned` filter.** The design's 置顶 saved view has no
   backing query.
2. **The result ordering has no pinned term.** `ORDER BY` is the sort column
   plus `prompts.id` (`:977-981`). Pinned prompts do not float to the top today;
   `is_pinned` is currently a stored flag with no behavior anywhere.
3. **There is no recency filter.** No date-range field exists, and the model has
   no `last_used_at` (parent assumption A1).
4. **`folder_id = ?` is an exact match, not a subtree rollup.** Folders nest
   (`Folder.parentId`, `src/features/prompts/types.ts:136`) and
   `src/features/prompts/folderTree.ts` builds the hierarchy, but a folder query
   returns only its direct children prompts.
5. **`limit` is clamped to `1..=100`** (`prompt.rs:923`). A count-only query
   still returns one row.
6. **`Folder` carries no prompt count** (`types.ts:132-140`) and `tag.list`
   returns bare strings (`api.ts:92`). No count arrives with either list.

Locked-state behavior is safe for counting. `unlocked_key`
(`src-tauri/src/services/security.rs:227-236`) returns `Ok(None)` when locked,
never an error, and `present_prompt` (`services/prompt.rs:499-524`) redacts the
body while leaving `title`, `tags`, `folder_id`, `is_favorite`, and `is_pinned`
untouched — those columns are never encrypted. Counts are exact in every state.

## Decisions

### D1 — The sidebar rail is app-wide; its library content is feature-owned

`.trellis/spec/frontend/directory-structure.md` states that shared layout
belongs outside features only when it is truly app-wide, and that a one-feature
concern must not get its own top-level home. Saved views, folders, and tags are
prompt-feature concerns.

Split accordingly:

- `src/components/layout/Sidebar.tsx` keeps the rail: product mark, the
  quick-jump button, the collapse toggle, and the footer (theme, settings). It
  renders one child slot.
- A new `src/features/prompts/components/PromptLibraryNav.tsx` owns the saved
  views, the folder tree, and the tag cloud. It reads `usePromptStore`.

The layout layer then imports one component, not the prompt store, the folder
helpers, and the tag list. `AppShell.tsx` composes the two.

Rejected: putting the library content directly in `Sidebar.tsx`. It would make
an app-wide layout file depend on prompt DTOs, folder tree helpers, and prompt
store actions, which is the shape the directory spec calls out as a mistake.

### D2 — Counts come from per-bucket counted searches, not from `prompt.list`

Each count is one `prompt.search` with `limit: 1`, reading `PromptPage.total`
and discarding `items`.

The alternative is `prompt.list` (`api.ts:71`, already wired but unused by the
store), which returns every prompt in one call and would let the frontend
compute every count locally.

`prompt.list` is rejected because its cost scales with the **library**, which is
unbounded and grows with import, while the per-bucket cost scales with the
**taxonomy** — folders plus tags — which is human-curated and realistically
10 to 50 buckets. `prompt.list` also runs `present_prompt` over every row
(`services/prompt.rs:670-679`), decrypting every private prompt, and the counts
must be recomputed after every mutation: save, delete, batch tag, batch move,
tag rename, tag delete, and import. A counted search with `limit: 1` decrypts
exactly one row.

Shape:

- A `libraryCounts` slice on `usePromptStore`: `{ views, folders, tags }`, plus
  a `countsLoading` flag.
- One `refreshCounts()` action, issuing the bucket queries with a concurrency
  cap of 8.
- Called from `load()` and after every mutation that can change membership.
- Counts render as stale-but-visible while refreshing. They never block the
  prompt list.

**What a bucket count counts.** Each bucket query applies that bucket's own
filter and nothing else. A folder count is `folder_id = ?` over the whole
library. It is not intersected with the active keyword, the active tags, or the
favorites flag.

So a folder showing 12 while the toolbar reads `3 / 3` is correct and expected:
the folder holds 12 prompts, and 3 of them match the current keyword. The two
numbers answer different questions. The alternative — recomputing every bucket
against the live filter set — turns every keystroke into 10 to 50 counted
searches and makes each row's number change meaning as the user types.

State it in the UI rather than relying on the reader to infer it: the bucket
counts carry an accessible description saying the count is the bucket total.
This is parent assumption A14, and `08-24-library-toolbar` AC2 is scoped to the
toolbar's own count so the two are not asserted equal.

Escape hatch, not taken now: if measurement fails the budget in
`implement.md` step 6, the fix is a backend aggregate command, which is a parent
scope amendment — not a fallback to `prompt.list`.

### D3 — The 置顶 saved view is dropped

There is no `is_pinned` filter (finding 1) and no pinned-first ordering
(finding 2). The three ways to ship it:

- **Drop the view.** Chosen.
- **Filter the loaded page client-side.** Wrong. The page is 50 of `total`
  (`promptStore.ts:58`), so the view would silently show pins from page one only.
- **Add `isPinned` to `SearchQuery`.** Correct, but a backend contract change.
  The parent PRD's confirmed scope is "the frontend plus one backend addition:
  prompt-to-prompt references. No other backend contract changes."

Supporting observation: the design concept does not implement this view either.
Its own handler rewrites the pin view to the all view —
`nav: v.key === "pin" ? "all" : v.key` — and only changes the sort. The concept
shows a control that its own prototype does not honor.

Recommended amendment for the user, not applied here: adding
`is_pinned: Option<bool>` to `SearchQuery`, one `WHERE` clause, and a
`prompts.is_pinned DESC` leading term in `ORDER BY` is roughly ten lines in
`services/prompt.rs`. It would make both this saved view and the design's
pins-first grid real. Until that is approved, pinned state stays a badge owned
by `08-24-library-views`, and `is_pinned` keeps having no behavior.

### D4 — Saved views are a mix of filters and sort presets, and only filters get counts

| View        | Mechanism                                  | Count |
| ----------- | ------------------------------------------ | ----- |
| 全部 Prompt | no constraint                              | yes   |
| 收藏        | `isFavorite: true` (`prompt.rs:950-953`)   | yes   |
| 最近        | `sortBy: "updatedAt"`, `sortOrder: "desc"` | no    |

The recent view narrows nothing, so its count would equal the all count. Showing
the same number twice invites the reader to believe it is a filter. It shows no
count instead.

`PromptFilters` (`promptStore.ts:33-46`) already carries `folderId`, `tags`, and
`favoritesOnly`. A saved view is therefore not new state; it is a named preset
over existing fields. The store gains `activeView` only to render the selected
row, and selecting a view writes the preset through the existing
`setFilters` action so the query path stays single.

### D4b — `activeView` is nullable, and one transition table defines the scope

The problem this closes: with `activeView` defaulting to `"all"` and never
emptying, it is always truthy. A scope title that reads
`activeView → folderId → tag` in that precedence order can never reach its
second or third branch, so a folder or tag scope would still title itself
全部 Prompt. The sidebar row, the header title, the chips, and the counts would
each describe a different scope at the same time.

`filters` (`promptStore.ts:33-46`) stays the single source of truth for what is
queried. `activeView` is presentation state over it, and it is
`SavedView | null`.

| Transition                | `activeView`                                | `folderId` | `tags`    | `favoritesOnly` | `sortBy` / `sortOrder` |
| ------------------------- | ------------------------------------------- | ---------- | --------- | --------------- | ---------------------- |
| initial                   | `"all"`                                     | `null`     | `[]`      | `false`         | `updatedAt` / `desc`   |
| `selectView("all")`       | `"all"`                                     | cleared    | cleared   | `false`         | untouched              |
| `selectView("favorites")` | `"favorites"`                               | cleared    | cleared   | `true`          | untouched              |
| `selectView("recent")`    | `"recent"`                                  | cleared    | cleared   | `false`         | `updatedAt` / `desc`   |
| `selectFolder(id)`        | `null`                                      | `id`       | untouched | untouched       | untouched              |
| `selectFolder(null)`      | `null`                                      | `null`     | untouched | untouched       | untouched              |
| `toggleTagFilter(t)`      | `null`                                      | untouched  | toggled   | untouched       | untouched              |
| `setKeyword(k)`           | untouched                                   | untouched  | untouched | untouched       | untouched              |
| sort control              | `null` if it was `"recent"`, else untouched | untouched  | untouched | untouched       | written                |
| 全部清除                  | `"all"`                                     | `null`     | `[]`      | `false`         | `DEFAULT_FILTERS`      |

Three rules produce that table:

1. Choosing a view is exclusive over the taxonomy axes. It clears folder and
   tags, which is the concept's behavior of resetting the other axes.
2. Choosing a folder or a tag leaves the view rows unselected. It does **not**
   clear the other taxonomy axis: folder plus tag combines today
   (`prompt.rs:934-952` ANDs the clauses) and that capability stays.
3. Keyword is orthogonal to all of it. Searching inside a folder is normal and
   does not deselect the folder.

The sort control clearing `"recent"` follows from rule 1's converse: 最近 _is_ a
sort preset (D4), so sorting by something else has left that view.

**What the header titles.** First non-empty of: `activeView` label → folder name
→ the single active tag → 全部 Prompt when several tags are active. The
precedence is now reachable because `activeView` empties.

**What 最近 shows as a chip.** Nothing. It sets sort only, and the sort control
already displays 最近更新 / 降序. A chip would restate a visible control. 收藏
is covered by the favorites chip and 全部 by the absence of chips, so no saved
view gets a chip of its own (parent A13). `08-24-library-toolbar` R9 is amended
to match; it previously listed a saved-view chip that its own design forbids.

### D5 — The folder tree keeps its hierarchy and its management actions

The design draws a flat list with a color dot. `FolderTree.tsx` is a real tree:
expand/collapse, inline create/rename/delete, drag-to-reorder root siblings, and
drag-to-reparent with a cycle guard (`FolderTree.tsx:21-33`, `folderTree.ts`).

Flattening it would delete shipped behavior that PRD R8 requires to stay
reachable. The tree stays. The design's contribution is the row styling, the
count, and the add control, applied to the existing component.

The color dot has no backing field. `Folder` has `icon?: string | null`
(`types.ts:135`) and no color. The dot derives deterministically from the folder
id, so the same folder keeps the same color across sessions without a schema
change. It is decorative only: it never carries state, which keeps it clear of
the color-alone rule.

Counts are direct membership, matching `folder_id = ?` (finding 4). A parent
folder does not show the sum of its subtree. Rolling up would report a number
that clicking the folder cannot reproduce, which is worse than a small number.

### D6 — Tag management stays reachable

`TagManager.tsx` is a `<details>` block with per-tag rename and delete, wired to
`renameTag` / `deleteTag` (`promptStore.ts:403-425`). The design's tag cloud has
no management affordance.

PRD R8 names folder management only. Tag rename and delete are equally shipped
behavior, so this design keeps them: the cloud is the filter surface, and the
existing `TagManager` moves with it as a collapsed "manage tags" disclosure
beneath the cloud. Dropping it would remove the only path to renaming or
deleting a tag.

### D7 — The theme toggle reuses the preference write path

The footer toggle switches `theme` between `"light"` and `"dark"`. It calls
`useSettingsStore.setPreference("theme", …)` (`settingsStore.ts:208-210`,
implementation `:350-401`), which applies the appearance optimistically, writes
through `settings.update`, reconciles the canonical result, and tracks a per-key
save status.

It does not call the appearance controller directly and does not introduce a
second persistence path. When the stored mode is `"system"`, the toggle resolves
the currently painted mode first, then writes its opposite, so one press always
produces a visible change.

## Data flow

```
Sidebar (layout)                PromptLibraryNav (feature)
  mark / quick-jump               savedViews ──┐
  collapse toggle                 folderTree ──┼─→ usePromptStore.setFilters()
  footer: theme, settings         tagCloud   ──┘        │
        │                                              ↓
        └→ useSettingsStore.setPreference       refreshPrompts() → prompt.search
                                                       │
                                            refreshCounts() → prompt.search × N
                                                               (limit 1, total only)
```

`PromptsView` stops owning `showFolders` (`PromptsView.tsx:93`) and stops
rendering `FolderTree` and `TagManager`. It reads the same filters it reads
today, so its query path is unchanged.

## Compatibility

- No backend change. No new command, no contract change.
- No change to `prompt.search` call shape beyond `limit: 1` count queries, which
  the existing clamp already accepts.
- `NAV_ENTRIES` keeps its one-entry-per-`AppView` invariant
  (`navigation.ts:37-40`) and `navigation.test.ts` keeps passing, because saved
  views are not `AppView`s. Settings stays a real view; the footer entry keeps
  calling `setActiveView("settings")`.
- The collapse rail (`Sidebar.tsx:56-60`, `w-16` / `w-60`) stays. Collapsed, the
  library nav renders icons and counts; folders and tags collapse to a count
  summary rather than disappearing, so the active scope stays visible.

## Accessibility

- Saved views are a single-select group: `aria-current="true"` on the active
  row, matching the existing `NavButton` pattern (`Sidebar.tsx:26`).
- Folder and tag rows are toggles: `aria-pressed` reflects the filter state.
  Both carry a text count, so selection never depends on the accent fill alone.
- The folder color dot is `aria-hidden`.
- Tab order is rail top to bottom. The tag cloud is one tab stop with arrow-key
  movement between chips, so a large cloud does not trap keyboard users in
  dozens of stops.

## Test impact

- `src/components/layout/navigation.test.ts` — must still pass unchanged. If it
  fails, the split in D1 was done wrong.
- `src/features/prompts/PromptsView.layout.test.tsx` — asserts the current
  three-pane layout; updated to reflect the removed folder pane.
- `src/features/prompts/promptStore.test.ts` — extended for the counts slice and
  the saved-view presets.
- `src/features/prompts/i18nKeys.test.ts` — new keys for the saved views, the
  section headings, and the theme toggle.
- New: `PromptLibraryNav.test.tsx`.

## Rollback

The split is additive at the shell level: `AppShell` renders the library nav in
the sidebar slot. Reverting means restoring `showFolders` and the two mounts in
`PromptsView` and dropping the slot. The counts slice is independent and can be
reverted on its own, leaving the nav rendering without counts.
