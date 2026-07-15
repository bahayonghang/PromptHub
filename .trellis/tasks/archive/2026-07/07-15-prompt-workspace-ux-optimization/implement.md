# Implementation Plan: Prompt Workspace UX and Organization

## Preconditions

- Review `research.md`, `prd.md`, and `design.md`.
- Keep this parent in `planning`; start exactly one child task at a time.
- Treat the completed prompt foundations/evaluation work and current `main` as
  baseline. Do not reopen their scope.

## Ordered Work

1. Execute `07-15-prompt-workspace-layout-localization` to establish responsive
   pane behavior, editor hierarchy, locale completeness, and shared UI states.
2. Execute `07-15-prompt-filter-panel` against the new discovery-pane geometry.
3. Execute `07-15-prompt-inline-folder-create` using the existing folder action.
4. Execute `07-15-prompt-type-definitions` last because it owns schema, revision,
   portable bundle, and evaluation compatibility.
5. Run the parent integration review across all themes, locales, widths, text
   scales, locked/private states, empty/error/loading states, history, and
   evaluation mode.

## Parent Verification Gate

```powershell
just build
just test
just fmt-check
just clippy
just test-rust
just ci
```

Required visual/interaction evidence:

- browser screenshots at 800x600, 1200x800, and 1600x900 in light and dark;
- a 200% font-scale pass in English, Simplified Chinese, German, and Japanese;
- keyboard-only create/select/filter/save/cancel/history/evaluation paths;
- no overlap, clipping, blank panes, inaccessible popovers, or accidental
  English fallback;
- native Tauri smoke after the browser/static gate passes.

## Review and Rollback Points

- Review after the layout/localization child before changing filter geometry.
- Review after the two frontend-only interaction children before schema work.
- Back up and rehearse restoration before the type-definition migration.
- If the type migration or bundle compatibility fails, revert that child without
  rolling back the completed frontend-only children.

No application implementation begins until the selected child artifacts are
reviewed and `task.py start <child>` is explicitly run.
