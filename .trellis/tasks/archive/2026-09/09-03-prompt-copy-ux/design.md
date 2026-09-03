# Design — copy placement, success toast, usage increment

## Boundaries

| Area | Files |
| --- | --- |
| Shared copy control | `src/features/prompts/components/CopyPromptButton.tsx` |
| List | `src/features/prompts/components/PromptList.tsx` |
| Grid | `src/features/prompts/components/PromptGrid.tsx` |
| Overlay header | `src/features/prompts/components/detail/PromptDetailModal.tsx` |
| Content heading | `src/features/prompts/components/detail/sections/DefinitionSection.tsx` |
| Toast chrome | `src/features/notifications/toastStore.ts`, `ToastHost.tsx` |
| Store patch | `src/features/prompts/promptStore.ts`, `api.ts`, `types.ts` |
| Backend | `src-tauri/src/services/prompt.rs`, `commands/prompt.rs`, `lib.rs` |
| i18n | `promptsView.copy*` and toast strings in all seven `src/locales/*.json` |

`prompt.copy` stays a read-only expansion/substitution command. Clipboard writes stay in the frontend. Increment is a separate command.

## Placement

Copy is a sibling of the title activator, never a child of it.

```
List title cell
  [CopyPromptButton] [title activator: pin? lock? title]

Grid title row
  [CopyPromptButton] [title activator] [favorite]

Overlay header
  [CopyPromptButton] [title + version + description] [edit favorite pin more close]

Content heading row
  [CopyPromptButton] [h3 Prompt content] ........ [text | chat]
```

`CopyPromptButton` drops `compact`. These four call sites use the same 36×36 control and 20×20 icon. The in-control `CheckIcon` on success stays.

## List columns

Current `table-fixed` tracks (`PromptList.tsx:59-72`):

| Column | Now | After |
| --- | --- | --- |
| batch checkbox | `w-8` | unchanged |
| title | `w-[22%]` | `w-[28%]`, contains copy + title |
| description | `w-[22%]` | `w-[24%]` |
| tags | `w-[16%]` | `w-[12%]` |
| type | `w-[10%]` | `w-[10%]` |
| usage | `w-[8%]` | `w-[8%]`, `tabular-nums` |
| version | `w-[8%]` | `w-[7%]` |
| updated | `w-[10%]` | `w-[9%]` |
| favorite | `w-8` | `w-10` |
| copy | `w-8` | removed |

Title cell: `flex items-center gap-2`, copy `shrink-0`, activator `min-w-0 flex-1 truncate`. Header `sr-only` for copy moves onto the copy control itself (already has an accessible name). Favorite column header stays `sr-only`.

## Success toast

Reuse `ToastHost`. Do not add a second notification system.

1. `toastStore.push` gains an optional `replaceGroup?: string`. When set, any existing toast with that group is dismissed before the new one is appended, and the timer resets. Copy uses `replaceGroup: "prompt-copy"`.
2. `ToastHost` paints `tone === "success"` with `border-success/40`, a leading `CheckIcon` (`h-4 w-4 text-success`), and the existing card surface. `danger` keeps `border-destructive/40` and gains a leading `AlertCircleIcon` so success and failure are not color-only. `info` stays icon-less.
3. Copy success pushes `{ message: t("promptsView.copyPromptCopied"), tone: "success", replaceGroup: "prompt-copy" }`. Named copies may use `copyPromptCopiedNamed` with `title` when `name` is present; add the key to all seven bundles.
4. Copy failure may push `{ message: t("promptsView.copyPromptFailed"), tone: "danger", replaceGroup: "prompt-copy" }` in addition to the in-control failed state.

`CopyPromptButton` is the single place that pushes copy toasts for list, grid, header, and content heading. `copyFilled` in `PromptDetailModal` must call the same helper or the same button path so keyboard copy and **fill and copy** get the same toast and increment. Extract a small `copyPromptToClipboard` helper if `copyFilled` cannot reuse the button.

## Usage increment

### Wire

```
prompt.incrementUsage({ id: string }) -> { id: string, usageCount: number }
```

Errors: missing row → `NOT_FOUND`. No schema migration.

### Service

```sql
UPDATE prompts SET usage_count = usage_count + 1 WHERE id = ?1
RETURNING usage_count
```

Do not write `updated_at`. Do not go through `prompt.update`. Do not mutate inside `prompt.copy`.

Electron's `incrementUsage` ignored a missing id. This rewrite returns `NOT_FOUND` so the frontend can ignore it without guessing.

### Frontend sequence in the copy helper

1. If locked, return.
2. `prompt.copy` (when `promptId` is set) or `buildPromptCopyText` (create draft).
3. `writeText`.
4. On write success: in-control `copied`, success toast.
5. If `promptId` is set: `prompt.incrementUsage`, then `promptStore` patches `prompts` and `selectedPrompt` for that id. Increment failure is ignored for UX (copy already succeeded).
6. On write failure: in-control `failed`, danger toast, skip increment.

Do not call increment before `writeText`.

### Store

```ts
incrementUsage(id: string): Promise<number | null>
```

Injectable through `PromptApi`. Patches matching items in `prompts` and `selectedPrompt`. Does not call `searchPrompts`. List order under `sortBy: "usageCount"` waits until the next load.

## Rejected options

- Mutating `usage_count` inside `prompt.copy`. That command is documented read-only. Clipboard can still fail after copy returns, which would count a use that never reached the clipboard.
- `prompt.update({ usageCount: n+1 })`. Racy, and it stamps `updated_at`.
- Toast only, drop in-control `Check`. The archived list-copy contract required in-control confirmation without hover and without color alone. Keep it.
- Electron `showCopyNotification` setting. No such setting exists in this rewrite. Do not add one.
- One overlay copy only. The user boxed the content-heading control and asked to move it onto that heading. Header copy also moves onto the Prompt title so list/grid/header share one pattern.

## Compatibility

- No Electron data migration.
- Portable bundle already serializes `usage_count`; increment only changes the live row.
- Tauri `capabilities/default.json` does not list per-command ACL. Register the adapter in `invoke_handler!` in `lib.rs`.
- `prompt.copy` comment and tests that assert read-only behavior stay true.

## Rollback

Revert the frontend placement and toast changes independently of the backend command. A leftover `prompt.incrementUsage` with no caller is harmless. Do not leave a UI that calls a command that is not registered.
