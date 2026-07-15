# Implementation Plan: Prompt Workspace Layout and Localization

## Ordered Checklist

1. Add failing locale-parity tests for every rendered Prompts key in all seven
   bundles, then complete missing translations.
2. Add focused layout-state helpers or classes for wide/default/constrained
   panes without changing store/domain contracts.
3. Refactor `PromptsView` into clear folder, discovery, detail, and secondary
   workspace regions; preserve current selection and mode actions.
4. Refactor `PromptEditor` into unframed semantic sections with container-aware
   field stacking and the existing stable footer.
5. Normalize focus, labels, icon-button dimensions, loading/error/empty/locked
   states, and reduced-motion behavior within the touched surface.
6. Add component tests for pane switching, draft preservation, locale rendering,
   and accessible region/action names.
7. Run static, unit, visual, text-scale, and native smoke checks.

## Verification

```powershell
npx vitest run src/features/prompts/i18nKeys.test.ts
npx vitest run src/features/prompts
just build
just test
```

Capture browser evidence at 800x600, 1200x800, and 1600x900 in light/dark, then
repeat 800x600 at 200% text scale for `en`, `zh`, `de`, and `ja`.

## Risk and Rollback

- Keep store/API files unchanged unless a test proves state must move to preserve
  a draft across structural visibility.
- Review after locale parity before layout changes so translation failures are
  not confused with geometry regressions.
- Review the workspace shell before changing editor internals.
- Revert this child independently if any existing prompt workflow becomes
  unreachable; no data migration or compatibility rollback is required.
