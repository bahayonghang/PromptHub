# Implement — copy placement, success toast, usage increment

## Order

1. Backend `prompt.incrementUsage`
   - Service + unit tests: increment, missing id → `NOT_FOUND`, `updated_at` unchanged, two calls → +2.
   - Command adapter `rename = "prompt.incrementUsage"`.
   - Register in `lib.rs` `invoke_handler!`.
2. Frontend API/store
   - `PromptApi.incrementUsage`, DTO `{ id, usageCount }`.
   - `promptStore.incrementUsage` patches `prompts` and `selectedPrompt`.
   - `api.test.ts` asserts the wire name.
3. Toast
   - `replaceGroup` on `push`.
   - Success (and danger) visual treatment in `ToastHost`.
   - Tests for replace and success chrome.
4. Shared copy helper / `CopyPromptButton`
   - Drop `compact`. Fixed `h-9 w-9` / `h-5 w-5`.
   - After successful write: toast + optional `onCopied` / store increment.
   - Inject `incrementUsage` and toast in tests.
5. List placement + column widths (`PromptList.tsx` + tests).
6. Grid placement (`PromptGrid.tsx` + tests).
7. Overlay header: move copy before the title (`PromptDetailModal.tsx`).
8. Content heading: move copy before the definition heading; remove it from the text/chat cluster (`DefinitionSection.tsx`).
9. Route `copyFilled` through the same toast + increment path.
10. i18n keys in all seven bundles; `i18nKeys.test.ts` stays green.
11. Gates.

## Validation

From the repository root:

```bash
cargo test increment_usage --manifest-path src-tauri/Cargo.toml
npx vitest run src/features/prompts/components/CopyPromptButton.test.tsx src/features/prompts/components/PromptList.test.tsx src/features/prompts/components/PromptGrid.test.tsx src/features/prompts/components/detail/PromptDetailModal.test.tsx src/features/prompts/api.test.ts src/features/notifications/ToastHost.test.tsx src/features/prompts/i18nKeys.test.ts
just fmt-check
just clippy
just test-rust
just build
just test
```

Broad or cross-boundary change: `just ci` before reporting done.

## Risky files

- `CopyPromptButton.tsx` — every library and detail copy path.
- `PromptList.tsx` — nested-button rule, column math.
- `prompt.rs` — do not fold increment into `copy_secure`.
- `toastStore.ts` — `replaceGroup` must not drop unrelated toasts (save, export).

## Follow-up before `task.py start`

- Planning summary approved in a later user message.
- `implement.jsonl` and `check.jsonl` have real spec/research rows (seed `_example` removed).
- No product-code edits in the planning turn.

## Rollback points

- After step 1: command unused, safe.
- After step 4: list/grid still old placement but new size/toast if wired; keep steps 4–8 in one frontend pass if the button API changes.
- Prefer one frontend commit-sized pass for placement + toast + increment wiring once the backend command exists.
