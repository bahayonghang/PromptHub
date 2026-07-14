# Implementation Plan: Prompt-Only Product Refocus

## Preconditions

- Product owner reviews `research.md`, `prd.md`, and `design.md`.
- Confirm the planning assumption that Rules remain and only Skill is retired.
- Treat the completed `07-14-settings-appearance-localization` commits as baseline
  and keep their scope out of this task's future commits.
- Start exactly one child task; never start the parent as an implementation task.

## Ordered Work

1. Execute `07-14-retire-skill-management`.
2. Run the child full gate and a native smoke test; confirm old Skill data is
   untouched and no Skill surface is reachable.
3. Execute `07-14-prompt-library-foundations` in its internal migration-first
   order.
4. Test upgrade/rollback/import/large-library/private-content behavior against
   disposable copies of representative databases.
5. Execute `07-14-prompt-evaluation-loop` only after stable revision identifiers
   and provider-secret storage exist.
6. Run the parent integration review and update product/code-map documentation.

## Parent Verification Gate

```powershell
just ci
```

Additional required evidence:

- `rg` inventory shows no active `skill.*` command, Skill navigation/view,
  capability gate, managed runtime path, or Skill export option.
- A seeded database with more than 100 prompts can reach every row through UI
  paging/search and reports the correct total.
- Upgrade from a pre-change database preserves prompts, settings, Rules, media,
  backups, and dormant legacy Skill data.
- Portable export/import round-trips all prompt revision fields and media refs.
- Locked private prompts do not expose plaintext through list/get/search/export.
- Evaluation matrix results remain linked to exact prompt revisions and profiles.
- Native Tauri smoke covers create/edit/version/diff/export/import/test/compare,
  restart, and backup restore.

## Rollback Points

- After Skill retirement and before schema migrations.
- After migration framework + pagination, before revision-format conversion.
- After foundations, before evaluation tables and UI.

## Review Gate

Keep all four tasks in `planning` until the user approves the final artifacts.
Task creation is not authorization to edit application code or run destructive
data migrations.
