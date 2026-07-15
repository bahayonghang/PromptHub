# Implementation Plan: Prompt Filter Panel

## Ordered Checklist

1. Add SearchBar component tests that reproduce the current disclosure,
   semantics, long-label, clear, and keyboard failures.
2. Introduce controlled trigger/surface ids, refs, open/close handlers, Escape,
   outside-click handling, and focus restoration.
3. Replace the inline horizontal sort row with vertical labeled field groups and
   preserve current filter callbacks/defaults.
4. Normalize compact trigger size, focus ring, count/status cue, empty tags, and
   long-tag wrapping against `DESIGN.md`.
5. Test English, Chinese, and German rendering plus 100%/200% text scale in the
   final discovery-pane widths.
6. Run targeted and full frontend gates.

## Verification

```powershell
npx vitest run src/features/prompts/components/SearchBar.test.tsx
just build
just test
```

Visual scenarios: closed/default, open/default, favorites applied, sort changed,
zero tags, many/long tags, 800x600, 1200x800, and 200% text scale.

## Risk and Rollback

- Do not change `PromptFilters`, store query conversion, or backend search.
- Verify stacking/overflow in the final parent layout before choosing inline
  positioning versus a portal.
- Revert the component/test slice if focus restoration or clipping cannot be
  proven; no persisted state rollback is required.
