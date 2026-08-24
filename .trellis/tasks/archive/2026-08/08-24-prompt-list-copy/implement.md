# Implementation Plan: Prompt copy, default chat mode, and definition width

## Ordered Checklist

1. Add `PromptCopySource` and `buildPromptCopyText` to
   `src/features/prompts/promptText.ts`. Cover text-only, system+user, chat
   messages, default substitution, unmatched placeholders, empty system, and
   empty/whitespace system treated as absent.
2. Add `CopyPromptButton` with injected `writeText`, locked/disabled, busy,
   success (1.5s Check), and failure announcement. Colocate
   `CopyPromptButton.test.tsx`.
3. Add the five `promptsView.copy*` keys to all seven locale bundles. Confirm
   `i18nKeys.test.ts` still gates parity.
4. Add `PromptList.test.tsx` covering one control per row, copy without select,
   locked skip, success label, and distinct keyboard activation.
5. Restructure `PromptList` so the visual card is a `div`, the select target is
   a sibling of `CopyPromptButton`, and the title row reserves space for copy.
6. Seed create drafts with `[{ role: "user", content: "" }]`. On chat submit,
   derive `systemPrompt` / `userPrompt` with the leaving-chat-mode extraction.
   Put `CopyPromptButton` in the definition header left of the mode toggle.
   Update `PromptEditor.test.tsx` create paths that currently type into
   "User Prompt" so they use the chat message field, and add tests for default
   chat, derived save fields, and definition copy.
7. Remove `max-w-5xl` from the editor inner wrapper. Change
   `.prompt-editor__message` at `min-width: 40rem` to
   `max-content minmax(0, 1fr) auto`. Make the role select `w-auto` in that
   layout.
8. Run targeted tests, then `just build` and `just test`. Do not run Rust
   fmt/clippy unless a Rust file is touched (none should be).

## Verification

```powershell
npx vitest run src/features/prompts/promptText.test.ts
npx vitest run src/features/prompts/components/CopyPromptButton.test.tsx
npx vitest run src/features/prompts/components/PromptList.test.tsx
npx vitest run src/features/prompts/components/PromptEditor.test.tsx
npx vitest run src/features/prompts/i18nKeys.test.ts
npx vitest run src/features/prompts/PromptsView.layout.test.tsx
just build
just test
```

Manual check in the desktop window or `just frontend`:

- New Prompt: chat toggle pressed; one empty user message; Save disabled until
  that message and the title are filled.
- Save a chat prompt; list preview shows the last user message.
- Open a saved text-mode prompt: text toggle remains pressed.
- List: copy one row; paste; selection unchanged. Locked row copy disabled.
- Editor: edit a message, copy from the definition header, paste includes the
  unsaved edit.
- Wide editor pane: message textarea is the flexible column; definition fields
  reach the pane edges; no `max-w-5xl` gutter.

## Risk and Rollback

- Do not nest a copy `<button>` inside the list card `<button>`.
- Do not call `prompt.copy` before `writeText`.
- Do not copy `privateLockedPreview` or any redacted body.
- Do not reuse `CopyIcon` (duplicate) for clipboard copy.
- Do not open existing text-mode prompts as chat.
- Do not save chat drafts with empty `userPrompt` when a user message exists.
- Do not add locale keys to only `zh.json` / `en.json`.
- Revert the frontend files in the checklist if clipboard, default-mode, or
  width checks fail. No persisted-data rollback.

## Context to curate before `task.py start`

Grok dispatches Trellis sub-agents. Add real `implement.jsonl` / `check.jsonl`
entries (not the seed `_example` row) for:

- `.trellis/spec/frontend/component-guidelines.md`
- `.trellis/spec/frontend/quality-guidelines.md`
- `.trellis/tasks/08-24-prompt-list-copy/prd.md`
- `.trellis/tasks/08-24-prompt-list-copy/design.md`
