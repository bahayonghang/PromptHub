# Design: Prompt copy, default chat mode, and definition width

## Boundaries

Frontend-only. No new command, store action, toast system, or clipboard plugin.

| Area | Files |
| --- | --- |
| Copy text | `src/features/prompts/promptText.ts` |
| List | `src/features/prompts/components/PromptList.tsx` |
| Editor | `src/features/prompts/components/PromptEditor.tsx` |
| Width | `src/styles/globals.css` (`.prompt-editor__message`) |
| i18n | `promptsView.copy*` in all seven `src/locales/*.json` |
| Tests | `promptText.test.ts`, new `PromptList.test.tsx`, `PromptEditor.test.tsx` |

`prompt.copy` stays unused. List rows and the editor draft already hold the
fields needed to build the payload. A prior IPC `await` can drop the webview
user-activation token before `navigator.clipboard.writeText`.

## Copy payload

Pure helper, React-free:

```ts
export interface PromptCopySource {
  systemPrompt?: string | null;
  userPrompt: string;
  messages: PromptMessage[];
  variables: Variable[];
}

buildPromptCopyText(source: PromptCopySource): string
```

`Prompt` satisfies `PromptCopySource`. The editor maps the draft into the same
shape.

Substitution values are each variable's `defaultValue` when that string is
non-empty. Names without a default are omitted so `substituteVariables` leaves
those placeholders intact.

Chat mode (`messages.length > 0`):

- each message becomes `[System]` / `[User]` / `[Assistant]` plus substituted
  content
- blocks join with a blank line

Text mode (`messages.length === 0`):

- trimmed system prompt empty → substituted user prompt only
- otherwise → `[System]\n${system}\n\n[User]\n${user}`

Role labels are clipboard contract text in English, matching the Electron helper
(`ref/.../prompt-copy-utils.ts:42-51`).

Locked list rows never enter this helper. The control is disabled first.

## Shared copy control

List and editor share the same interaction: icon-only `ClipboardCopy`, 1500 ms
`Check` success, row/header-local error, injectable `writeText`.

A small feature-local `CopyPromptButton` is justified because the two call sites
need the same busy, success, failure, and locked states. It stays under
`src/features/prompts/components/`. It does not become a generic app button.

Props:

```ts
interface CopyPromptButtonProps {
  source: PromptCopySource;
  label: string;
  locked?: boolean;
  lockedLabel?: string;
  writeText?: (text: string) => Promise<void>;
}
```

Default `writeText` is `navigator.clipboard.writeText`. Tests inject a fake.

## List row structure

The current row cannot host a copy button:

```
li > input[checkbox] + button.select-card
```

Replace with:

```
li.flex.items-start.gap-1
  input[checkbox]
  div.card.flex.min-w-0.flex-1     // selected border + hover fill
    div.select-target.flex-1.min-w-0  // role="button" tabIndex=0
      title / preview / tags
    CopyPromptButton               // sibling, not nested
```

Card chrome stays `rounded-lg border px-3 py-2`, selected
`border-primary bg-primary/10`, idle `border-transparent hover:bg-accent`.

Select target keeps `aria-current="true"` when selected and handles click,
Enter, and Space. Copy calls `stopPropagation` so the row is not selected.

This matches FolderTree: non-button chrome with inner icon buttons
(`FolderTree.tsx:119-154`).

Title row:

```
[type] [lock] [title truncate] [type chip] [star] [copy]
preview
tags
```

Copy is `shrink-0` on the title row. Title keeps `min-w-0 flex-1 truncate`.
The selected border includes the copy control. Do not `absolute`-position it.

## Editor definition header

```
Prompt 定义                    [copy] [文本 | 对话]
```

Copy sits immediately left of the existing mode `role="group"`. Same
`CopyPromptButton` as the list. Source is the live draft, so unsaved edits copy.

The locked editor placeholder (library locked) does not render this header.

## Copy button style

Reuse compact icon-button language from `PromptsView` (`iconButtonClass`),
scaled to the host density:

| State | Treatment |
| --- | --- |
| Rest | List `h-7 w-7`; editor header `h-8 w-8` to match the mode toggle `min-h-8`. `ClipboardCopy` `h-3.5 w-3.5`. `text-muted-foreground` |
| Hover | `hover:bg-accent hover:text-foreground` |
| Focus | `focus-visible:ring-2 focus-visible:ring-ring` |
| Press | `active:scale-[0.96]`; transition `transform`, `color`, `background-color` only |
| Success | `Check` + `text-primary` for 1500 ms; accessible name becomes copied |
| Disabled | `disabled` + `opacity-40`; no clipboard call |
| Busy | `disabled` while the clipboard promise is in flight |

Always visible. Do not hide copy behind hover. Do not use `Copy` (toolbar
duplicate). One SVG, `currentColor`, outline at rest.

## Default chat mode

`toDraft(null)` (create) seeds `messages: [{ role: "user", content: "" }]`.
`chatMode` is then true. Save stays disabled until that message is non-empty,
same as an empty user prompt in text mode.

`toDraft(savedPrompt)` is unchanged: empty `messages` stays text; non-empty
stays chat. Opening an existing text prompt does not convert it.

`setChatMode` conversion is unchanged.

On create/save while `chatMode` is true, derive text fields with the same rules
as leaving chat mode (`PromptEditor.tsx:545-552`):

- `systemPrompt` ← first message with role `system`, or `""`
- `userPrompt` ← last message with role `user`, or `""`

Submit those derived strings plus `messages`. List preview and search keep a
user-prompt string.

## Definition width

1. Remove `max-w-5xl` from the editor inner wrapper
   (`PromptEditor.tsx:646`). Keep `w-full`. Identity two-column, organization,
   and definition fields then use the pane width.
2. Wide chat message track (`globals.css` `@container prompt-editor (min-width: 40rem)`):

```css
.prompt-editor__message {
  grid-template-columns: max-content minmax(0, 1fr) auto;
}
```

Role `<select>`: drop `w-full` at that breakpoint (the 1-col stacked layout
still uses `w-full`). Use `min-w-[6.5rem] w-auto` so short labels such as 用户
do not steal 8rem from the textarea.

3. Text-mode system/user textareas stay `${inputClass} w-full`.

Do not change `prompt-editor__two-column` or footer container queries.

## i18n

Add under `promptsView` in all seven bundles:

| Key | English |
| --- | --- |
| `copyPrompt` | `Copy prompt` |
| `copyPromptNamed` | `Copy {{title}}` |
| `copyPromptCopied` | `Copied` |
| `copyPromptFailed` | `Could not copy prompt` |
| `copyPromptLocked` | `Unlock the prompt library to copy private content` |

List uses `copyPromptNamed`. Editor header uses `copyPrompt` (the title field
is already on screen). Locked list rows use `copyPromptLocked` for `title` and
`aria-label`.

## Compatibility

No schema, command, DTO, or settings change. Existing text-mode rows keep
text mode until the user switches. Portable bundles, search, batch selection,
private encryption, and evaluation stay unchanged.

If `navigator.clipboard.writeText` is missing or rejects, the control shows
`copyPromptFailed`. A later task can add a Runtime Bridge clipboard path.

## Trade-offs

| Choice | Why |
| --- | --- |
| Client DTO / draft instead of `prompt.copy` | Keeps the user-gesture token |
| No variable modal | Confirmed one-click policy |
| Shared `CopyPromptButton` | Two identical success/error/busy UIs |
| Derive `userPrompt` on chat save | List preview and search read that field |
| Seed chat only on create | Opening a saved text prompt must not switch mode |
| Content-sized role column | The 8rem track plus `max-w-5xl` wastes textarea width |

## Rollback

Revert `PromptList.tsx`, `PromptEditor.tsx`, `promptText.ts`, `globals.css`,
locale keys, and tests. No data migration. Prompts saved under the new default
remain valid chat-mode records.
