# Implementation Plan: Skill Domain Retirement

## Checklist

1. Add neutral SSRF helper module; move tests; update media imports.
2. Delete Skill feature module and wrapper; narrow `AppView`, navigation, shell,
   stores, capability types/defaults/gates, and fixtures.
3. Remove Skill export scope and runtime path from settings/system/backend DTOs,
   directory creation, sync selection, and tests.
4. Remove `commands::skill` registrations and command module.
5. Remove Skill services/models/mappings/property tests and fresh-schema DDL.
6. Remove Skill locale namespaces and scattered product claims in all locales;
   update i18n key tests.
7. Remove only dependencies made unused by steps 1-6 and refresh code maps/docs.
8. Run targeted tests, full `just ci`, and native legacy-data smoke.

## Targeted Verification

```powershell
npx vitest run src/components/layout/navigation.test.ts src/runtime/index.test.ts src/features/settings/api.test.ts src/features/system/api.test.ts
cargo test skill_retirement --manifest-path src-tauri/Cargo.toml
just ci
```

Add purpose-built Rust tests for fresh-schema absence and legacy-table
preservation; do not rely on a test name that does not exist yet.

## Final Inventory Checks

```powershell
rg -n "skill\.|SkillsView|skillDistribution|skillFileEditing|skillLocalScan|skillPlatformIntegration|skillStore" src src-tauri/src
rg -n "CREATE TABLE IF NOT EXISTS skills|idx_skills|idx_skill_versions" src-tauri/src
```

Remaining matches must be reviewed individually and limited to legacy-data
compatibility notes/tests or unrelated natural-language uses.

## Rollback

Revert code changes. No user data migration or deletion is performed, so the old
binary can regain access to the dormant tables/directories.
