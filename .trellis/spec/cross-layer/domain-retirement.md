# Domain Retirement

## 1. Scope / Trigger

Use this contract when removing a first-class managed domain across the React
shell, Runtime Bridge, Tauri backend, filesystem paths, and SQLite schema. The
current reference case is the retired Skill domain.

## 2. Signatures

The active wire and storage shapes after Skill retirement are:

```text
AppView = "prompts" | "settings"
ExportScope = { data: bool, media: bool, rule: bool }
RuntimePaths = { data, media, rule, backup, log }
RuntimePathsReport = { data, media, rule, backup, log, database }
```

`RuntimeCapabilities`, the Tauri invoke registry, and `SCHEMA_SQL` must not
contain a Skill field, command, table, or index. Existing databases may still
contain `skills` and `skill_versions`; startup must not drop or rewrite them.

## 3. Contracts

- Remove the domain atomically from navigation, app state, bridge capabilities,
  frontend APIs/stores/types, command registration, services, and models.
- Stop creating or reporting its runtime directory and remove it from selective
  export. Never delete an existing retired-domain directory implicitly.
- Fresh databases omit retired-domain DDL. Existing tables and rows remain
  dormant and byte-preserved so rollback to an older binary remains possible.
- Move shared behavior to a neutral owner before deleting a domain module. For
  Skill retirement, media SSRF classification moved to `network_safety` first.
- Remove direct dependencies and product/localization claims that only served
  the retired domain.

## 4. Validation & Error Matrix

| Condition | Required behavior |
|-----------|-------------------|
| Fresh database startup | Retired tables and indexes are absent |
| Legacy database startup | Existing retired rows are unchanged; no error |
| Normal Prompt operation on a legacy database | Prompt succeeds; retired rows remain unchanged |
| Existing retired runtime directory | Directory is neither scanned nor changed nor deleted |
| Selective export | No retired-domain scope is accepted or emitted |
| Untrusted media URL resolves locally | Return `SSRF_BLOCKED` through the neutral network policy |

## 5. Good / Base / Bad Cases

- Good: initialize a database containing legacy rows, create a Prompt through
  the real service, and compare the legacy rows before and after.
- Base: initialize a fresh database and assert only active-domain tables and
  indexes exist.
- Bad: add `DROP TABLE`, recursively delete the old directory, leave a hidden
  bridge capability, or keep a selective export flag for an unsupported domain.

## 6. Tests Required

- Navigation/app-store tests assert the exact active view set.
- Runtime Bridge tests assert the exact capability fields and gates.
- Settings/system tests assert the exact export and runtime-path DTOs.
- Storage tests assert fresh-schema absence and legacy-row preservation after a
  real active-domain write.
- Network-safety unit tests cover public and blocked IPv4/IPv6/hostname cases;
  media tests assert blocked URLs still return `SSRF_BLOCKED`.
- Final inventory searches must find no active UI, command, capability, module,
  path, export field, or fresh-schema DDL for the retired domain.

## 7. Wrong vs Correct

Wrong: delete the old tables or directory during startup because the feature is
no longer reachable.

Correct: remove every active reader/writer and creation path while leaving old
bytes untouched; prove both fresh-install absence and upgrade preservation.
