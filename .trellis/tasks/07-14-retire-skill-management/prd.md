# Retire app skill management safely

## Goal

Remove Skill as an application-managed object while preserving existing user
data and all non-Skill PromptHub behavior.

## Confirmed Facts

- The frontend has a complete `src/features/skills/**` domain and first-class
  navigation entry.
- The backend registers 28 `skill.*` commands and owns CRUD/version, Markdown,
  local repository, platform integration, safety, and remote fetch services.
- Fresh databases create `skills` and `skill_versions`; runtime paths and ZIP
  export also expose a Skill category.
- Media download imports shared SSRF helpers from `skill_safety`, so that module
  cannot be deleted before the helpers move to a neutral owner.

## Requirements

- R1: Remove Skill navigation, view/store/API/components/types, app-view state,
  Runtime Bridge capabilities, and associated tests.
- R2: Remove all `skill.*` command registration and Skill-only command, service,
  model, mapping, and test code.
- R3: Move public-network SSRF helpers still needed by media download into a
  neutral service with equivalent tests before removing `skill_safety`.
- R4: Stop creating/managing/reporting the Skill runtime directory and remove
  Skill from selective export, runtime information, and settings contracts.
- R5: Stop creating Skill tables in new databases, but do not drop existing
  tables or delete existing Skill directories in this release.
- R6: Remove Skill-specific product claims and locale namespaces across all
  seven locales; preserve unrelated prompt/rule translations.
- R7: Remove dependencies made unused by this change, but do not refactor
  unrelated Prompt, Rule, media, sync, or settings code.
- R8: Old databases and backups containing Skill data remain readable for all
  non-Skill domains; restored legacy Skill data remains dormant.

## Acceptance Criteria

- [x] AC1: Navigation and app state contain only Prompts and Settings.
- [x] AC2: No frontend production code invokes a `skill.*` command or references
  a Skill capability.
- [x] AC3: Tauri registers no `skill.*` command and no Skill command/service/model
  module is compiled.
- [x] AC4: Fresh schema contains no Skill table/index; an old database retains
  its Skill rows byte-for-byte after startup and normal prompt operations.
- [x] AC5: Existing Skill directories are not scanned, changed, exported as a
  category, displayed, or deleted.
- [x] AC6: Media URL SSRF enforcement and tests remain equivalent after helper
  extraction.
- [x] AC7: Seven-locale key tests, frontend build/tests, Rust fmt/clippy/tests,
  and native Prompt/Settings/backup smoke tests pass.
- [x] AC8: A final repository search finds no active Skill UI/command/capability,
  allowing only explicit legacy-data compatibility comments/tests.

## Out of Scope

- Deleting legacy Skill data or offering a new Skill export tool.
- Removing Rules, Prompt features, media download, backups, or remote sync.
- Migrating Skills into Prompts.
