# Design: Skill Domain Retirement

## Removal Order

1. Extract SSRF hostname/IP helpers from `skill_safety` to a neutral backend
   module and switch media download to it.
2. Remove frontend navigation/view/store/API and runtime capability contracts.
3. Remove settings/system path/export Skill fields from both sides of the bridge.
4. Remove Tauri command registrations and Skill command/service/model modules.
5. Remove fresh-schema Skill DDL, mappings, property generators, and indexes.
6. Remove locale/product copy and dependencies that are now demonstrably unused.

## Legacy Data Policy

No destructive SQL or filesystem operation is introduced. On upgrade:

- Existing `skills`/`skill_versions` tables remain in SQLite but have no compiled
  reader or writer.
- Existing `<app-data>/skill` contents remain untouched.
- Whole-data backup/restore may carry those opaque bytes because it snapshots the
  data directory; selective Skill export is removed.
- A fresh database no longer creates Skill tables, indexes, or directories.

This policy makes the removal reversible without claiming continued Skill
support. A future explicit data-cleanup task may drop legacy data only with user
consent and a migration/backup contract.

## Cross-Layer Contracts

- `AppView`, navigation entries, and shell registry must change atomically.
- Runtime capability DTO/defaults/gates and every test fixture must change
  atomically.
- `ExportScope`, runtime path reports, backend path ownership, settings/system UI,
  and all bridge tests must change together.
- SSRF policy behavior remains owned by a generic network-safety module and is
  used by every outbound fetch that needs public-address enforcement.

## Risk Controls

- Test startup against both a fresh database and a copied legacy database.
- Hash the legacy Skill tables/directories before and after the smoke scenario.
- Verify backups still restore prompts/settings/rules from an archive that also
  contains dormant Skill data.
- Search generated command registry and production bundle for Skill routes/copy.
