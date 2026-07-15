# Remove skill management and optimize prompt workflows

## Goal

Refocus PromptHub as a dedicated local-first prompt-management application by
removing app-level Skill management and improving the prompt lifecycle according
to evidence from the current implementation, comparable products, and published
prompt-management practices.

## Background

- The product owner has decided that Skills are outside the application's scope.
- PromptHub currently presents Prompt and Skill as first-class managed objects.
- The requested outcome is a reviewed optimization plan, not implementation in
  this session.
- The separate `settings-appearance-localization` task completed and was archived
  concurrently during this planning session; its committed result is the baseline
  and must not be folded into this task's future change set.
- The live Prompt module currently implements CRUD, folders, FTS search, tags,
  favorites, variables, media references, copy-time substitution, and manual
  version snapshots. It does not implement the advanced prompt workflows still
  described by many legacy locale keys.
- The first Skill-retirement release must be non-destructive: existing Skill
  tables and directories may remain as dormant user data, but no Skill UI,
  command, service, runtime capability, managed path, or selective export option
  remains active.

## Requirements

- R1: Remove user-facing Skill management and the supporting app capabilities
  that exist only for Skill storage, import, editing, deployment, or sync.
- R2: Preserve non-Skill prompt workflows and user data while removing the Skill
  domain; any database or backup compatibility decision must be explicit before
  destructive migration work begins.
- R3: Audit the current prompt-management workflow across discovery, authoring,
  organization, versioning, testing/evaluation, reuse, import/export, and
  recovery.
- R4: Compare the current product against relevant prompt-management projects
  and authoritative best-practice sources, with source links and retrieval dates.
- R5: Convert verified gaps into a prioritized implementation scope; do not add
  features merely because a competitor exposes them.
- R6: Keep CLI and Web variants out of scope, consistent with repository policy.
- R7: Treat Rules as system/project prompt assets and leave them unchanged in
  this task. This is an explicit planning assumption because the request named
  Skill removal specifically.
- R8: Split implementation into independently verifiable children and execute
  them in dependency order.

## Acceptance Criteria

- [ ] AC1: The plan inventories every frontend route/component/store/API, Runtime
  Bridge command, Rust command/service/model, persistence table, backup/sync
  payload, localization key group, and test owned by the Skill domain.
- [ ] AC2: The plan defines Skill-removal behavior for navigation, persisted Skill
  records, backup/import archives, remote sync compatibility, and stale settings.
- [ ] AC3: A current-state prompt feature matrix is backed by code and test anchors.
- [ ] AC4: The research matrix cites relevant projects and primary documentation,
  distinguishes observed features from recommendations, and records access dates.
- [ ] AC5: Proposed prompt improvements are prioritized by user value, evidence,
  dependency, risk, and implementation size, with explicit non-goals.
- [ ] AC6: `design.md` defines cross-layer removal and compatibility boundaries;
  `implement.md` provides ordered work packages and concrete validation gates.
- [ ] AC7: No application implementation begins until the planning artifacts are
  reviewed and the intended implementation task is explicitly started.
- [ ] AC8: The parent integration review proves there are no active Skill
  surfaces and that prompt library, backup/recovery, media download, settings,
  and Rules behavior still pass their full verification gates.

## Task Map

1. `07-14-retire-skill-management` (P0): remove the active Skill domain without
   deleting existing user data.
2. `07-14-prompt-library-foundations` (P0): repair large-library reachability,
   version integrity, portability, privacy, and stale contracts.
3. `07-14-prompt-evaluation-loop` (P1): add structured testing, reproducible run
   history, test cases, evaluators, and comparison.

## Out of Scope

- Implementing the plan during this planning session.
- Editing `ref/PromptHub/**`.
- Adding CLI or Web product variants.
- Expanding PromptHub into a general Agent, MCP, or Skill operations platform.
- Team RBAC, webhooks, public prompt marketplaces, production traffic A/B
  routing, hosted observability, or deployment environments.

## Planning Decisions

- D1: Existing Skill data is preserved but becomes dormant. The first release
  does not drop legacy tables or recursively delete Skill directories.
- D2: Prompt foundations land before the evaluation workbench so run records can
  reference stable, complete prompt revisions.
- D3: Local-first and single-user value outrank enterprise collaboration features.
- D4: Locale strings are not evidence of implemented behavior; only reachable
  code and tests establish the current feature baseline.
- D5: Rules remain in scope as prompt-like project/system instructions unless the
  product owner explicitly broadens the removal request.
