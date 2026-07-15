# Implementation Plan: Inline Folder Creation

## Ordered Checklist

1. Add PromptEditor/FolderPicker behavior tests for open, cancel, validation,
   success, failure, busy state, focus, and draft preservation.
2. Extend the editor callback contract and wire it to the existing
   `usePromptStore.createFolder` action in `PromptsView`.
3. Implement the select-plus-inline-input interaction with root-level creation,
   localized accessible labels, and stable geometry.
4. Select only the returned folder id after success; keep the name/selection on
   failure and prevent duplicate submission.
5. Update every locale bundle and prompt i18n coverage.
6. Run targeted store/component tests and full frontend gates.

## Verification

```powershell
npx vitest run src/features/prompts/promptStore.test.ts
npx vitest run src/features/prompts/components/PromptEditor.test.tsx
just build
just test
```

Manually verify create and edit drafts with no folders, many folders, duplicate
names, validation failure, simulated bridge failure, and a successful root folder.

## Risk and Rollback

- Do not duplicate folder loading or backend calls inside the editor.
- Do not clear/reset the full draft when the folder list prop changes.
- Revert the editor wiring and focused component/tests if draft preservation
  cannot be proven; no persisted-data rollback is required.
