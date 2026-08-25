# Design — library toolbar and filter state

## What the current code actually does

The PRD's background was written from the design concept. Four of its statements
do not match the code. This section establishes the facts before any decision.

1. **There is no keyword debounce.** `SearchBar` calls
   `onChange({ keyword })` on every keystroke (`SearchBar.tsx:184`), which reaches
   `setFilters` and then `refreshPrompts` (`promptStore.ts:235-238`). One
   `prompt.search` per keystroke. PRD R4's "keeps the existing debounce" describes
   a mechanism that does not exist.

2. **`refreshPrompts` has no request-ordering guard.** Two searches in flight can
   resolve out of order and the later `set({ prompts, total, offset })`
   (`promptStore.ts:229`) wins by arrival, not by recency. `selectPrompt` guards
   this case (`promptStore.ts:272`); `refreshPrompts` does not.

3. **There is no batch mode.** Row checkboxes render unconditionally
   (`PromptList.tsx:91-97`) and `BatchToolbar` appears whenever the selection is
   non-empty (`PromptsView.tsx:360`). Entering and leaving a mode is new
   behavior, not a re-styling.

4. **There is no batch favorite command.** The backend exposes `prompt.batchMove`,
   `prompt.batchTag`, and `prompt.batchDelete`
   (`src-tauri/src/commands/prompt.rs:65,76,85`) and nothing else. PRD R8 lists
   favorite as a batch action.

Two further facts constrain the header:

5. **No aggregate usage count exists.** `PromptPage` carries `items`, `total`,
   `limit`, `offset`, `hasMore` (`types.ts:201-207`). There is no sum of
   `usage_count` over the result set, and `state.prompts` is one page of 50
   (`promptStore.ts:58`).

6. **There is no file dialog.** `prompt.bundleExport` takes an optional
   `destination` and defaults to the backup directory
   (`src-tauri/src/commands/portable.rs:17-27`); the import path is typed into a
   text input (`PromptsView.tsx:255-264`). The design's 导入 / 导出 buttons keep
   this behavior.

## Decisions

### D1 — The keyword input debounces, and the search is sequence-guarded

The design puts a live `shown / total` count next to the keyword field. That
count makes both defects above visible: typing eight characters issues eight
searches, and the count can settle on the result of an earlier keystroke.

Two changes, both in `promptStore.ts`:

- A 200 ms debounce on the keyword path only. Folder, tag, favorite, sort, and
  view changes stay immediate — they are single discrete actions, not typing.
- A monotonic request counter shared by `refreshPrompts` **and** `load`. A
  response whose sequence is not the newest is discarded rather than written to
  the store.

The sequence guard is required whether or not the debounce lands, because paging
and filter changes can also overlap. The debounce reduces how often it matters.

`load` is inside the guard, not outside it. Earlier planning excluded it on the
grounds that it runs once on mount. "Runs once" is not "cannot overlap": `load`
awaits four commands in one `Promise.all` (`promptStore.ts:188-197`), the search
bar is rendered and focusable throughout, and its completion handler writes
`prompts` / `total` / `offset` unconditionally (`:198-206`). A user who types
before that resolves gets the initial unfiltered page written over their search
result. One counter covering both writers closes it; two counters, or a guard on
only one writer, does not.

The counter is module-level and monotonic, and each writer captures its value
before awaiting and compares before `set`. `load`'s `folders` / `tags` /
`promptTypeDefinitions` writes are not part of the guarded slice — they have no
competing writer — so a superseded `load` still populates the taxonomy and
discards only the page.

Rejected: debouncing inside `SearchBar`. The store is what issues the query, and
a component-level timer would leave `setFilters` callers outside the search bar
unguarded.

### D2 — The header subtitle states the result count and the library location, not an aggregate usage count

PRD R1 asks for "the result count and the aggregate usage count". The result
count is `state.total`, which is exact. The aggregate usage count has no source
(finding 5). Summing `state.prompts` would produce the sum over the loaded 50,
displayed beside an exact total — two numbers of different scope on one line.

The subtitle therefore carries the result count and the library location, read
from `useSystemStore.paths` (`system/api.ts:48`, `app.getRuntimePaths`). The
location satisfies the PRODUCT.md requirement that local-first be tangible. The
paths report loads best-effort and is window-gated
(`systemStore.ts:397`), so the subtitle renders the count alone until it
arrives, and never shows a placeholder path.

Adding a summed usage count would need a backend aggregate. That is a scope
amendment, not taken here.

### D2b — The scope title reads the one scope model, and it is reachable

The header title, the sidebar's selected row, the filter chips, and the sidebar
counts must not describe four different scopes at once. They all read the state
model in `08-24-shell-sidebar` design D4b, in which `activeView` is
`SavedView | null` and empties whenever a folder or a tag is chosen.

Title, first non-empty:

| Condition                        | Title              |
| -------------------------------- | ------------------ |
| `activeView === "all"`           | 全部 Prompt        |
| `activeView === "favorites"`     | 收藏               |
| `activeView === "recent"`        | 最近               |
| `folderId` set                   | that folder's name |
| exactly one tag                  | that tag           |
| several tags, no folder, no view | 全部 Prompt        |

The precedence only works because `activeView` is nullable. Against a
non-nullable `activeView` defaulting to `"all"`, the first row always matches and
the folder and tag rows are dead code.

This task does not own that state; it consumes it. If `08-24-shell-sidebar` has
not landed, this task adds `activeView` per D4b and records the transfer, rather
than reading `filters` directly and inventing a second precedence.

### D3 — Batch favorite is dropped from the batch bar

`prompt.batchMove`, `batchTag`, and `batchDelete` each open one transaction,
apply every id, append one version snapshot per changed prompt, and commit
(`services/prompt.rs:771-850`). They are atomic.

The alternatives for favorite:

- **Drop it.** Chosen. The per-row favorite toggle stays, so the capability is
  not lost, only the bulk form the design draws.
- **Loop `prompt.update` on the frontend.** N round trips, N version snapshots
  (`services/prompt.rs:442`), and no transaction — a failure at item 30 of 50
  leaves half the selection changed with no report of where it stopped. It would
  also be the only batch action in the bar that is not atomic.
- **Add `prompt.batchFavorite`.** Correct. Roughly fifteen lines mirroring
  `batch_tag`, plus a command and an `api.ts` entry. It is a second backend
  contract change, and the parent PRD's confirmed scope allows exactly one
  (prompt-to-prompt references).

Recommended amendment for the user, not applied here: add `prompt.batchFavorite`
alongside the three existing batch commands. Until then the batch bar exposes
count, select-all-on-page, move, tag, and delete.

### D4 — Batch mode is a store flag, and leaving it clears the selection

`batchMode: boolean` joins `usePromptStore`. Row checkboxes render only while it
is true; `PromptList` and the grid read it rather than always rendering the
checkbox column.

Consequence to record: today a user can select prompts at any time, because the
checkboxes are always present. After this change selection requires entering
batch mode first. That is the design's model and it is a deliberate change, not
a regression to fix later.

Leaving batch mode calls the existing `clearPromptSelection`
(`promptStore.ts:358`). Entering it does not preselect anything.

### D5 — The sort control keeps both axes

The design shows one sort select with four options: 最近更新, 最近使用, 使用最多,
名称. Two of those collapse: 最近使用 has no backing field and maps to
`updatedAt` (parent assumption A1), which is what 最近更新 already selects. The
design's list would show the same ordering under two labels.

The toolbar therefore keeps the two controls the product already has — a field
select over `updatedAt | createdAt | title | usageCount`
(`SearchBar.tsx:23-28`) and a direction select — moved out of the filter popover
and styled into the toolbar row. Every combination stays reachable, and no label
claims an ordering the data cannot produce (PRD R5).

### D6 — Chips render the four real filter axes; there is no separate view chip

`PromptFilters` holds `keyword`, `folderId`, `tags[]`, `favoritesOnly`
(`promptStore.ts:33-46`). The chip row renders:

| Chip          | Source                           | Remove writes              |
| ------------- | -------------------------------- | -------------------------- |
| keyword       | `filters.keyword`                | `{ keyword: "" }`          |
| folder        | `filters.folderId` → folder name | `{ folderId: null }`       |
| tag (one per) | `filters.tags`                   | that tag removed           |
| favorites     | `filters.favoritesOnly`          | `{ favoritesOnly: false }` |

The saved view from `08-24-shell-sidebar` is a preset over these same fields
(that task's design D4), so a view chip would duplicate the favorites chip and
give the user two controls that clear the same state. 全部清除 writes
`DEFAULT_FILTERS` and resets `activeView` to `all`.

### D7 — Loading and empty states move to the view container

`PromptList` currently owns three branches: loading, empty, and items
(`PromptList.tsx:58-77`). PRD R10 assigns the empty state to this task, while
the item rendering belongs to `08-24-library-views`. Splitting the same
component between two children invites both to edit it.

Instead the library container in `PromptsView` owns the branch:

```
loading            → skeleton or loading row
!loading && 0 items → empty state with a clear-filters action
otherwise           → <PromptGrid /> or <PromptList />
```

`PromptList` and the new grid render items only. This satisfies R10 by
construction — the empty branch is unreachable while `loading` is true — and it
gives `08-24-library-views` two components with one job each.

### D8 — The view-mode flag lives in `usePromptStore`

This task owns the flag; the parent's `research/shared-ownership.md` records it.
`viewMode: "grid" | "list"` joins `usePromptStore`, next to `filters` and
`batchMode`, with a `setViewMode` action. This task owns the toggle control;
`08-24-library-views` and `08-24-command-palette` read the flag and call the
action.

Session-only, matching PRD R6. It is not written through `settings.update`,
because that path is for user preferences that survive a restart
(`settingsStore.ts:350-401`) and R6 asks for the session only.

## Data flow

```
Header      scope title ← activeView / folder / tag
            subtitle    ← state.total + useSystemStore.paths
            import/export → existing exportBundle / previewBundle / importBundle
            new prompt  → existing create path (until 08-24-detail-modal)

Toolbar     keyword  ─ debounce 200ms ─┐
            sort field / direction ────┤
            view toggle → setViewMode  ├→ usePromptStore.setFilters()
            batch toggle → setBatchMode│         │
                                       │         ↓
Chips       one per active axis ───────┘   refreshPrompts()  [sequence-guarded]
                                                   │
Batch bar   visible while batchMode ────────→ batchMove / batchTag / batchDelete
```

## Compatibility

- No backend change. No new command.
- `prompt.search` call shape is unchanged; only how often it is called changes.
- Import and export keep their commands, their conflict-policy step, their
  preview summary, and their typed-path input (finding 6).
- `SearchBar`'s filter popover loses the sort and favorites controls to the
  toolbar. The tag list inside it is superseded by the sidebar's tag cloud from
  `08-24-shell-sidebar`; verify that task landed before removing it here.

## Accessibility

- The batch toggle is `aria-pressed`, and entering batch mode announces the
  state through a live region. The mode is not signalled by the toolbar's fill
  alone (PRD AC7).
- Each chip is a button whose accessible name states both the axis and the
  value, so "移除标签 rust" is distinguishable from "移除文件夹 rust".
- The result count is in a polite live region so a keyboard user hears the
  count settle after typing.
- The empty state's clear-filters button is reachable in the normal tab order,
  not only by pointer.

## Test impact

- `src/features/prompts/promptStore.test.ts` — extended for the debounce, the
  sequence guard, `batchMode`, and `viewMode`.
- `src/features/prompts/components/SearchBar.test.tsx` — updated for the moved
  controls, or split if the popover shrinks to the tag list only.
- `src/features/prompts/PromptsView.layout.test.tsx` — updated for the header,
  toolbar, chip row, and the moved loading/empty branch.
- `src/features/prompts/components/PromptList.test.tsx` — loses its loading and
  empty cases to the container (D7). Those cases move rather than disappear.
- `src/features/prompts/i18nKeys.test.ts` — new keys.

## Rollback

The header, toolbar, chip row, and batch bar are new components composed into
`PromptsView`. Reverting means restoring the previous pane chrome and dropping
them. The store additions — debounce, sequence guard, `batchMode`, `viewMode` —
are independent and can stay; nothing else breaks if they do.
