# Current copy surfaces (2026-09-03)

## Screenshots

1. Library list: copy is the last icon on each row, 14px, after favorite. Tags column is often empty. Title, description, type, usage, version, updated sit to the left.
2. Library grid: copy is bottom-right of each card footer. Title is top-left; favorite is top-right.
3. Detail overlay content tab: copy is grouped with the text/chat toggle, to the right of `提示词内容`. Overlay header already has a smaller copy in the right-hand action cluster.

## Code

- List copy column: `src/features/prompts/components/PromptList.tsx:70-72, 152-161`
- Grid footer copy: `src/features/prompts/components/PromptGrid.tsx:118-136`
- Overlay header copy: `src/features/prompts/components/detail/PromptDetailModal.tsx:385-397`
- Content-tab copy: `src/features/prompts/components/detail/sections/DefinitionSection.tsx:77-88`
- Control size: `CopyPromptButton.tsx:20-21, 107-115` (`compact` 28px button, icon always 14px)
- In-control success: `CopyPromptButton.tsx:27, 85-89, 112-118`
- Toast host: `src/features/notifications/ToastHost.tsx` (success tone unused visually)
- `prompt.copy` read-only: `src-tauri/src/services/prompt.rs:1138-1140`
- `usage_count` update stamps `updated_at`: `src-tauri/src/services/prompt.rs:429-434`
- Electron increment: `ref/PromptHub/packages/db/src/prompt.ts:374-379`
- Archived list-copy out of scope: toast and `usageCount` (`08-24-prompt-list-copy/prd.md` Out of Scope)

## Why increment cannot use `prompt.update`

`prompt.update` writes `updated_at` on any non-noop patch, including `usage_count`. A copy would then jump the row under 最近更新. Electron avoided that with a dedicated `usage_count = usage_count + 1` statement.
