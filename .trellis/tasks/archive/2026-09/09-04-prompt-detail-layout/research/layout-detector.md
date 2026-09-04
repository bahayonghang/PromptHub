# Assessment B — mechanical layout scan

Browser visualization skipped: Tauri overlay, no localhost surface for injection.

## Command

```
node "C:\Users\lyh\.claude\skills\impeccable\scripts\detect.mjs" --json --scope layout src/features/prompts/components/detail/PromptDetailModal.tsx src/features/prompts/components/detail/sections src/components/ui/Modal.tsx src/styles/globals.css
```

- cwd: repo root
- exit code: 0
- engine: regex `detectText` on `.tsx` / `.css`
- findings JSON: `[]` (0)

Layout-scoped rules in the filter (none fired): `nested-cards`, `monotonous-spacing`, `icon-tile-stack`, `content-hidden-at-rest`, `edge-flush-cards`, `text-occlusion`, `first-viewport-column-overflow`, `line-length`, `cramped-padding`, `body-text-viewport-edge`, `heading-rhythm`, `text-overflow`, `clipped-overflow-container`.

Several of those need live layout. This pass could not evaluate them.

## Manual overflow / spacing (path:line)

- `Modal.tsx:142` — dialog `max-h-[min(90vh,56rem)] w-[min(1180px,100%)] overflow-hidden`.
- `PromptDetailModal.tsx:385` — inner column same height as dialog `max-h`; dialog also has `border`. Inner height can exceed the content box by the border thickness and clip.
- Confirm dialog classes concatenate after the 1180px / 90vh utilities with no `twMerge`.
- `PromptDetailModal.tsx:563` — `mx-auto flex w-full max-w-[68ch] flex-col gap-7` is the reading-column clamp.
- `.prompt-editor` container queries measure the **form**, not the 68ch inner column, so 40rem field-level two-column rules fire even while the inner column is 68ch.
- `.prompt-editor__footer` compact rules never apply: the live footer is outside the form (`PromptDetailModal.tsx:633`).
- `.prompt-editor__message-role` is unused; the Select has no that class.
- Adjacent sections stack `gap-7` plus `pt-5` + `border-t`.
- More-actions menu is `absolute` inside `overflow-hidden` dialog (`PromptDetailModal.tsx:446-456`, `Modal.tsx:142`).
