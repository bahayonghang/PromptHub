# Copy control placement, success toast, and usage count

## Goal

A user copies a Prompt from the library list, the library grid, or the detail overlay by activating a copy control that sits immediately before that Prompt's title (or, in the content tab, before the Prompt content heading). The control is large enough to hit. A successful copy shows a designed success toast. Each successful copy of a persisted Prompt increments `usageCount` by one.

## User value

Copy is the primary reuse action. Today the control sits at the far right of a dense table, in a card footer, or beside the text/chat toggle, at 14px. Usage stays at 0 because copy never records it. After this change, copy is next to the title the user is already reading, success is visible, and the 使用 column reports real copy activity.

## Background

Confirmed from the 2026-09-03 screenshots and the current tree.

- List: `PromptList.tsx` renders copy as the last `th`/`td` (`w-8`, `compact`). The title lives in its own column (`w-[22%]`). The copy icon is `h-3.5 w-3.5` inside a `h-7 w-7` button (`CopyPromptButton.tsx:107-115`).
- Grid: `PromptGrid.tsx:126-136` puts copy in the footer `ml-auto`, after type, usage, date, and version. The title row holds pin/lock, title, and favorite.
- Detail overlay header: `PromptDetailModal.tsx:371-397` already has a `CopyPromptButton` in the right-hand action cluster (copy, edit, favorite, pin, more, close). The title is a left `flex-1` block.
- Detail content tab: `DefinitionSection.tsx:70-123` places a second `CopyPromptButton` next to the text/chat toggle, to the right of the `提示词内容` heading. That is the control boxed in the third screenshot.
- Success feedback on `CopyPromptButton` is an in-control `CheckIcon` for 1500 ms plus `sr-only` live text (`CopyPromptButton.tsx:85-121`). Archived `08-24-prompt-list-copy` R6 required in-control confirmation and listed toast as out of scope.
- `ToastHost` already exists (`src/features/notifications/`). Save already pushes `tone: "success"` (`PromptDetailModal.tsx:256-259`). Success toasts use the same chrome as info toasts; only `danger` gets a distinct border (`ToastHost.tsx:20-22`).
- `prompt.copy` is read-only and "never mutates stored data" (`services/prompt.rs:1138-1140`). `usage_count` exists on the model and is sortable, but no copy path increments it. Frontend `UpdatePromptInput` has no `usageCount` field (`types.ts:162-180`).
- `prompt.update` that writes `usage_count` also writes `updated_at` (`services/prompt.rs:429-434`). Using update for copy would reshuffle "最近更新" order. The Electron reference increments with `UPDATE prompts SET usage_count = usage_count + 1 WHERE id = ?` and no `updated_at` (`ref/.../packages/db/src/prompt.ts:374-379`), then shows `toast.copied`.
- Locked private copy stays visible, disabled, and does not write the clipboard (`CopyPromptButton.tsx:70-71, 100`; archived list-copy R5).
- Create-draft copy has no `promptId` and uses `buildPromptCopyText` locally (`CopyPromptButton.tsx:82-84`). That path must not increment usage.
- `Ctrl+Enter` / `Cmd+Enter` and the variable **fill and copy** button share `copyFilled` (`PromptDetailModal.tsx:263-280, 313-316`). That is a copy of a persisted Prompt when `prompt` is set.

## Requirements

- R1. List: each row's copy control sits immediately before the title text in the title cell. The trailing copy column is removed. The copy control remains a sibling of the title activator, not nested inside it.
- R2. Grid: each card's copy control sits immediately before the title in the title row. The footer no longer contains a copy control. Favorite stays top-right. Footer still shows type, usage, date, and version.
- R3. Detail overlay header: the header copy control sits immediately before the Prompt title, not in the trailing action cluster. Edit, favorite, pin, more, and close stay in that cluster.
- R4. Detail content tab: the copy control beside the text/chat toggle moves to sit immediately before the Prompt content heading (`promptsView.editor.sections.definition`). The text/chat toggle stays on the right of that heading row.
- R5. Icon size: list, grid, overlay header, and content-heading copy controls use a 36px (`h-9 w-9`) hit target and a 20px (`h-5 w-5`) `ClipboardCopy` / `Check` icon. Compact 28px / 14px copy controls are removed from these three surfaces.
- R6. List layout: after dropping the copy column, rebalance `table-fixed` widths so the title column (now copy + title) is the widest text column, description keeps a readable share, the often-empty tags column shrinks, and type / usage / version / updated stay compact mono columns. Favorite remains the last column. Cells still truncate rather than wrap. Batch-mode checkbox column is unchanged.
- R7. A successful copy of any in-scope control shows both the existing in-control `Check` confirmation (about 1.5 s) and a designed success toast. The toast uses `ToastHost`, `tone: "success"`, a check icon, a success-token border, and the localized copied message. Rapid successive copies replace the previous copy toast instead of stacking.
- R8. A failed copy still announces on the control that was used and does not show a success toast. It may show a danger toast with the existing failed message. A locked control stays visible, disabled, and does not write the clipboard or increment usage.
- R9. Each successful clipboard write of a persisted Prompt (`promptId` present) increments that Prompt's `usage_count` by exactly one. The increment is a dedicated backend command, atomic, and does not change `updated_at`. The library usage figure and the selected Prompt's `usageCount` update without a full search reload.
- R10. Increment runs only after the clipboard write succeeds. Failed clipboard, locked copy, missing Prompt, and create-draft copy (no `promptId`) do not increment. `Ctrl+Enter` / `Cmd+Enter` and **fill and copy** on a persisted Prompt increment once, because they are the same copy action.
- R11. Copy payload, variable default substitution, `@@Title` expansion via `prompt.copy`, keyboard isolation from row select, and i18n key parity stay as they are after `08-24-prompt-list-copy` and `08-24-prompt-references`.
- R12. All new user-facing strings live in all seven locale bundles and remain in the prompts i18n key-parity gate.

## Acceptance criteria

Each criterion names the requirement it closes.

- [ ] AC1 (R1): In list mode, the copy control for a Prompt is in the title cell, immediately before the title text. There is no trailing copy column. Activating copy does not call `onSelect` or `onToggleSelection`. Activating the title still opens the Prompt.
- [ ] AC2 (R2): In grid mode, the copy control is in the title row immediately before the title. The footer has no copy control. Activating copy does not open the Prompt.
- [ ] AC3 (R3, R4): With the detail overlay open on a saved Prompt, the header copy control is immediately before the Prompt title, and the content-tab copy control is immediately before the Prompt content heading. The text/chat cluster has no copy control.
- [ ] AC4 (R5): Those copy controls render a `h-5 w-5` icon inside a `h-9 w-9` button in list, grid, overlay header, and content heading.
- [ ] AC5 (R6): List header and body columns match after the copy column is removed. Title is wider than tags. Long titles and descriptions truncate. No horizontal scrollbar at the app's minimum usable width.
- [ ] AC6 (R7): After a successful list, grid, header, or content-heading copy, `ToastHost` shows one success toast with the copied message, a check icon, and a success-token treatment. The same control shows `Check` for about 1.5 s. A second copy while the toast is visible replaces that toast; two copy-success toasts are not listed at once.
- [ ] AC7 (R8): A clipboard rejection leaves the control in the failed state, does not show a success toast, and does not increment usage. A locked row's copy control stays disabled and does not call the clipboard writer or increment.
- [ ] AC8 (R9, R10): Copying an unlocked persisted Prompt with `usageCount` 0 then 1 yields stored counts 1 then 2. `updated_at` is unchanged. The visible 使用 figure updates to 1 then 2 without requiring a manual refresh.
- [ ] AC9 (R9, R10): Create-draft copy (no `promptId`) writes the clipboard and does not call increment. A missing id returns `NOT_FOUND` and does not create a row.
- [ ] AC10 (R10): `copyFilled` (keyboard copy and **fill and copy**) on a persisted Prompt increments once per successful write.
- [ ] AC11 (R11): Existing `CopyPromptButton`, `PromptList`, and `PromptGrid` copy-payload / locked / non-select tests still pass, updated only for the new placement and size.
- [ ] AC12 (R12): Adding, removing, or renaming a locale key fails `src/features/prompts/i18nKeys.test.ts` until all seven bundles match.
- [ ] AC13: `just fmt-check`, `just clippy`, `just test-rust`, `just build`, and `just test` pass from the repository root.

## Key decisions

- D1. Copy leads the title on every surface the user named (list, grid, overlay header, content heading). Favorite stays where it is.
- D2. Overlay header copy and content-heading copy both remain. They copy the current draft. Header copy is the chrome action next to the Prompt title. Content-heading copy is the work-area action the third screenshot boxed. They share `CopyPromptButton`.
- D3. Success uses in-control `Check` plus a designed `ToastHost` success toast. In-control feedback stays for the control that was used; the toast is the visible confirmation the current 14px check does not provide.
- D4. Usage increment is a new `prompt.incrementUsage` command, not `prompt.update` and not a mutation inside read-only `prompt.copy`. Increment happens after a successful clipboard write.
- D5. Keyboard copy and **fill and copy** of a persisted Prompt count as one use each. Create-draft copy does not.

## Out of scope

- Variable-fill modal.
- A settings toggle equivalent to Electron `showCopyNotification`.
- Changing the overflow-menu duplicate action (`CopyIcon`).
- Per-message copy, batch copy, command-palette copy, or version-tab "copy this revision".
- Reordering the library when `sortBy` is `usageCount` immediately after increment.
- Incrementing usage from evaluation runs or AI test (Electron did; this task only counts copy).
- Schema change. `usage_count` already exists.
- New clipboard plugin.
- Moving the favorite star.
- Light/dark theme redesign beyond the success-toast treatment.

## Risks

- Nested buttons: the title activator is `role="button"`. Copy must stay a sibling in the title cell, as list-copy already required.
- `prompt.copy` then `writeText` then increment: a prior IPC can drop the webview user-activation token. Current `CopyPromptButton` already awaits `prompt.copy` before `writeText` and it works in the packaged app. Do not add another await before the clipboard write. Increment is after `writeText`.
- Increment failure after a successful copy must not flip the control to failed or retract the toast. Usage can stay stale until the next load.
- Two copy controls in the overlay both increment if the user activates both. That matches "one click, one count".
