# Detail overlay and four tabs

Child of `08-24-ui-refactor`. Owns parent requirements R7, R8, R9, R10, R11.
This is the largest child in the tree.

## Goal

Replace the inline editor pane with a centered overlay carrying four tabs:
content, version history, run comparison, and references.

## Ordering

Lands after `08-24-library-toolbar` and `08-24-library-views`, which decide how
a prompt is opened. Its references tab needs `08-24-prompt-references`; the
other three tabs do not. If references land late, ship the tab disabled with a
stated reason rather than absent.

## Background

- `src/features/prompts/components/PromptEditor.tsx` is 1172 lines and holds the
  entire detail surface inline. It is the single largest file in `src/`.
- The editor pane is one of two workspace modes;
  `workspaceMode === "evaluation"` swaps in `EvaluationWorkbench`
  (`PromptsView.tsx:607-611`). The workbench is 32.3K and is reachable only
  through that toggle.
- Version history is a separate component behind the `showHistory` flag
  (`PromptsView.tsx:94`), backed by `versionDiff.ts` structured diffing and the
  store's `createVersion` / `rollbackVersion` (`PromptsView.tsx:87-88`).
- Text mode versus chat mode is `draft.messages.length > 0`
  (`PromptEditor.tsx:533`). New drafts start in chat mode after the archived
  task `08-24-prompt-list-copy`.
- Container queries `prompt-editor` and `prompt-workspace`
  (`globals.css:185-290`) drive the editor's responsive layout. Moving the
  editor into an overlay changes its container, so these queries need review.
- Variables are declared on the model (`src-tauri/src/models/prompt.rs:86-93`)
  and edited in `VariableEditor.tsx`.

## Design target

Overlay: `min(1180px, 100%)`, centered on a blurred scrim, with an entry
animation. Escape and a scrim click close it.

Header: title, version chip, a metadata line, 复制正文, an edit/read toggle,
favorite, close.

Tab strip: 内容, 版本历史 (count), 试跑对比 (count), 引用 (count).

Content tab: a two-column body — the prompt body on the left with a character
count and a read/edit switch, a variable form beneath it with a
"fill variables and copy" action; a metadata column on the right with title,
description, type, folder, tags, and three collapsible sections (组织方式,
补充信息, 安全与同步).

Version tab: a version timeline on the left, each entry showing tag, date, note,
and added/removed counts; a line-level diff on the right with 复制此版本 and
回滚到此版本.

Run tab: a run bar plus side-by-side model output cards with per-card actions.

References tab: outgoing references, incoming references, and a picker that
inserts a reference token.

Footer: a shortcut hint line, 关闭, 保存.

## Requirements

- R1: The overlay is a modal dialog. Focus moves into it on open, is trapped
  while open, and returns to the trigger on close. Escape and scrim click close
  it. Background content is inert to assistive technology.
- R2: Closing with unsaved edits does not silently discard them. The behavior is
  explicit and stated in the UI.
- R3: The content tab preserves every capability the current editor has,
  including text and chat mode, system prompt, messages, variables, media
  references, type definitions, private/locked handling, and the existing save
  and validation rules. Any capability deliberately dropped is listed in
  `design.md` with a reason.
- R4: The variable form fills declared variables and copies the filled body in
  one action, reusing the existing copy contract rather than a second
  implementation.
- R5: The version tab reuses `versionDiff.ts` and the existing create/rollback
  store actions. Rollback keeps its confirmation.
- R6: The run tab hosts the existing `EvaluationWorkbench` for the open prompt.
  The design's two-card layout is presentation; the underlying engine is
  unchanged (parent assumption A2).
- R7: The references tab lists outgoing and incoming references from
  `08-24-prompt-references`, marks unresolved references, and inserts a
  reference token into the body from the picker.
- R7b (added by `design.md` D7): The 引用 tab takes the `references` i18n name.
  The editor's existing `promptsView.editor.sections.references` section
  (`PromptEditor.tsx:1079-1087`) holds images, videos, source, and notes — media
  provenance, not prompt-to-prompt references — and is renamed to
  `...sections.attachments`. One feature cannot have two meanings for the word.
- R7c (added by `design.md` D4): The header keeps pin, duplicate, and delete,
  which the design concept's header drops. They exist today
  (`PromptsView.tsx:498-588`) and are the only path to those actions on an open
  prompt. Favorite and copy sit in the header as the design shows; duplicate and
  delete move to an overflow menu.
- R8: `PromptEditor.tsx` is decomposed. No single new file carries the whole
  overlay. Each tab is its own module.
- R8b (added by `design.md` D1): The modal focus behavior is a shared primitive,
  built here and reused by `08-24-command-palette`. No such primitive exists
  today: `CloseDialog` sets `role="dialog"` and `aria-modal` but has no focus
  trap, no focus restore, no Escape handler, and does not make the background
  inert. The primitive keeps a stack so the topmost modal is the one Escape
  closes.
- R9: The container-query layout still works inside the overlay, or is replaced
  by an equivalent that is verified at the app's minimum usable width.
- R10: The overlay respects reduced-motion preferences (PRODUCT.md).
- R11: All labels come from i18n keys, present in all 7 bundles.

## Acceptance criteria

Each criterion names the requirement it closes.

- [ ] AC1 (R1): Opening and closing the overlay moves focus correctly in both
      directions, and Escape closes it from any tab.
- [ ] AC1b (R1, R8b, design D1): With the overlay open, the native close
      confirmation still renders, focuses, and operates. Only `#app-content` is
      inerted; `CloseDialog` is a sibling of it and stays live.
- [ ] AC2 (R3): Every field editable before this change is editable after it.
      The `design.md` D3 checklist is verified row by row.
- [ ] AC2b (R2, design D10, parent CC11): Opening a different prompt while the
      draft is dirty raises the same confirmation as closing — from a library
      item, from the palette, and from a paging control. No path calls
      `selectPrompt` unguarded.
- [ ] AC2c (R2, design D10): `⌘S` saves while the caret is in the prompt body,
      and `⌘Enter` copies from the same position. A hint is printed only for a
      binding that exists (parent CC9).
- [ ] AC3 (R3): Saving from the overlay produces the same backend calls as the
      current editor, verified against the store actions.
- [ ] AC3b (R2, design D2b): A save that fails validation leaves the overlay
      open with the edit intact, and save-and-close does not close.
- [ ] AC4 (R5): A version rollback from the version tab changes the prompt body
      and the version chip in the header.
- [ ] AC5 (R6): Running a comparison from the run tab produces the same result as
      the current workspace toggle.
- [ ] AC6 (R7): The references tab shows a resolved reference, an unresolved one,
      and an incoming one, each distinguishable without color alone. An incoming
      entry names and navigates to the prompt that references this one
      (`08-24-prompt-references` AC3c).
- [ ] AC7 (R3): A locked private prompt opens the overlay without exposing
      content.
- [ ] AC8 (R10): With reduced motion enabled, the overlay does not animate.
- [ ] AC9 (R8): `PromptEditor.test.tsx` passes unchanged through the extraction
      steps, then is split alongside the component, with no loss of covered
      behavior.
- [ ] AC10: `just build` and `just test` pass.

## Out of scope

- Changing the evaluation engine, its providers, or its persistence.
- Changing version storage or the diff algorithm.
- `@@` autocomplete while typing in the body.
