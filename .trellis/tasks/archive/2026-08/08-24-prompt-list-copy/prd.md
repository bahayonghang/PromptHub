# Prompt one-click copy, default chat mode, and wider definition fields

## Goal

A user can copy paste-ready prompt text from the Prompts list or the editor
definition header in one click, without filling a variable modal. New prompts
open in chat mode. Definition message fields use the full editor pane width.

## Background

The Prompts list (`src/features/prompts/components/PromptList.tsx:74-132`) is a
checkbox plus a full-card `<button>` with no copy control. The editor toolbar
`CopyIcon` duplicates the selected prompt (`PromptsView.tsx:529-537`).

The Electron reference copies from each list row
(`ref/PromptHub/apps/desktop/src/renderer/components/prompt/PromptListView.tsx:107-116`).
This rewrite's `prompt.copy` command substitutes placeholders
(`src-tauri/src/services/prompt.rs:1026-1126`) but the frontend never calls it
and types the result as `string` (`api.ts:38-39`). Locked private prompts must
not copy redacted preview text (`PromptList.tsx:116-118`).

Chat mode is `draft.messages.length > 0` (`PromptEditor.tsx:533`). Create drafts
start with `messages: []` (`PromptEditor.tsx:63`), so new prompts open in text
mode. Submit writes `draft.userPrompt` even in chat mode (`PromptEditor.tsx:595-600`);
the list preview reads `userPrompt`, so a chat-only save can leave the list
preview empty.

The definition body is capped at `max-w-5xl` (`PromptEditor.tsx:646`). At
`prompt-editor` ≥ 40rem, each chat message is
`8rem | minmax(0,1fr) | auto` (`globals.css:272-287`). The 8rem role column
plus the max-width cap leave unused pane width beside the message textarea.

There is no toast system and no clipboard plugin. Clipboard writes use
`navigator.clipboard.writeText` from a user click.

Confirmed 2026-08-24: copy immediately with no variable modal; substitute
declared `defaultValue`s; leave unmatched placeholders; format text mode as
`[System]` / `[User]` when a system prompt exists, otherwise user prompt only;
format chat mode as `[System]` / `[User]` / `[Assistant]` blocks.

## Requirements

- R1: Every rendered prompt list item exposes a dedicated copy control. Empty
  and loading states do not show a copy control.
- R2: The editor definition header exposes one copy control beside the text/chat
  mode toggle. It copies the current draft, including unsaved edits.
- R3: Both copy controls write the confirmed payload in one click, without
  opening the editor from the list, without a variable modal, and without
  selecting or deselecting the list row. Substitution uses each variable's
  `defaultValue` when non-empty; unmatched `{{placeholders}}` stay in the text.
- R4: Text-mode payload is `[System]\n${system}\n\n[User]\n${user}` when the
  system prompt is non-empty after trim; otherwise the user prompt only.
  Chat-mode payload (`messages.length > 0`) joins messages as `[System]`,
  `[User]`, or `[Assistant]` blocks separated by a blank line. Role labels are
  clipboard contract text in English.
- R5: A locked private prompt's list copy control stays visible, is disabled,
  and states that the library must be unlocked. It does not write the clipboard.
  The editor copy control is absent on the locked placeholder surface.
- R6: Success and failure are observable on the control that was used, without
  hover and without color alone. Success is a short in-control confirmation.
  Failure is announced on that control and does not replace the Prompts view
  with a global error.
- R7: Copy controls are keyboard-operable, have localized accessible names, and
  do not nest a button inside the list row's select target.
- R8: Chat is the remembered editor mode. A create draft opens in chat. After
  the user selects Chat, later prompts and create drafts stay in chat and do
  not snap back to text. A stored chat prompt (`messages.length > 0`) always
  opens in chat. A stored text prompt is presented as chat while chat is
  preferred. Text mode returns only after an explicit Text toggle, which is
  then remembered.
- R9: Create and save in chat mode persist derived `systemPrompt` / `userPrompt`
  using the same extraction as leaving chat mode, so search still has text
  fields.
- R10: Definition fields use the editor pane width. Remove the `max-w-5xl` cap.
  On the wide chat message row, the role select is content-sized; the textarea
  takes the remaining column. Text-mode system/user textareas stay full width
  of the definition section.
- R11: Visual language stays in the current Prompts workspace: Lucide icons,
  Tailwind tokens, compact desktop density. Copy uses `ClipboardCopy`, not the
  toolbar `Copy` icon used for duplicate.
- R12: User-facing copy strings live under `promptsView.*` in all seven locale
  bundles and remain in the i18n key-parity gate.
- R13: A single chat message textarea is about 24rem tall, matching the
  prompt-content work area. Extra messages use a shorter 12rem minimum.
- R14: List rows preview `description`, not the prompt body. Locked rows still
  show the locked placeholder. Empty descriptions show a localized empty hint.
- R15: The editor section heading is the localized "Prompt content" term
  (`提示词内容` in zh), not "Prompt definition".

## Acceptance Criteria

- [ ] AC1: With two or more prompts in the list, each row has exactly one copy
      control whose accessible name identifies copy for that prompt's title.
- [ ] AC2: Activating a list copy control on an unlocked prompt writes the R4
      payload and does not change `selectedPromptId` or `selectedPromptIds`.
- [ ] AC3: New Prompt opens with chat mode pressed and one empty user message
      field. After Chat is selected, a later text-stored prompt still opens in
      chat. Text mode appears only after an explicit Text toggle.
- [ ] AC4: The definition header copy control writes the R4 payload built from
      the current draft. Editing a message then copying includes the unsaved
      text.
- [ ] AC5: Creating or saving a chat draft stores non-empty derived
      `userPrompt` (last user message) and `systemPrompt` (first system
      message) alongside `messages`.
- [ ] AC6: At an editor pane wider than 40rem, the chat message textarea is
      the flexible column; the role select does not use a fixed 8rem track; the
      definition section is not capped at `max-w-5xl`.
- [ ] AC7: A locked private prompt's list copy control is disabled, exposes a
      localized locked reason, and does not call the clipboard writer.
- [ ] AC8: After a successful copy, the same control shows a localized "copied"
      confirmation for about 1.5 seconds and then returns to the copy
      affordance. A clipboard failure shows a localized error on that control.
- [ ] AC9: Tab, Enter/Space on a list copy control, and Enter/Space on the
      select target remain distinct. Copy controls have a visible focus ring.
- [ ] AC10: Adding, removing, or renaming a locale key fails
      `src/features/prompts/i18nKeys.test.ts` until all seven bundles match.
- [ ] AC11: Targeted tests for copy text, PromptList, and PromptEditor (create
      default, save derivation, copy control, remembered chat mode) pass.
      `just build` and `just test` pass.
- [ ] AC12: The Prompts list preview is the description (or the empty-description
      hint), not `userPrompt`.
- [ ] AC13: The editor section heading reads the Prompt content label in the
      active locale.
- [ ] AC14: A single chat message field has a 24rem minimum height.

## Out of Scope

- Variable-fill modal (Electron `VariableInputModal` `mode="copy"`).
- Incrementing `usageCount` on copy.
- Toast / `copyNotification` settings.
- Changing the editor-toolbar `CopyIcon` that duplicates a prompt.
- Per-message copy buttons, batch copy, or a global copy shortcut.
- New Tauri clipboard plugin, new backend command, or schema change.
- Converting existing text-mode prompts to chat on open.
- Fixing the unused `copyPrompt(): Promise<string>` API type mismatch, except
  as a later follow-up.

## Risks

- Nested buttons in the list card are invalid HTML. The row structure must
  change so the copy control is a sibling of the select target.
- `navigator.clipboard.writeText` after an `await` of `prompt.copy` can lose
  the webview user-activation token. Copy builds text from the list DTO or
  editor draft and writes the clipboard without a prior IPC call.
- Default chat on create without R9 leaves list previews blank.
- Existing PromptEditor tests type into "User Prompt" during create; those
  tests must use the chat message field after the default changes.
