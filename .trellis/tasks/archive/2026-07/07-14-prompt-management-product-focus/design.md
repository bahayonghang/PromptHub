# Design: Prompt-Only Product Refocus

## Product Boundary

PromptHub remains a local-first desktop prompt workbench. Prompt records,
prompt-like Rules, folders, versions, testing, evaluation, and recovery are in
scope. Skill authoring, import, local-repository editing, platform distribution,
safety scanning, and remote Skill discovery are removed.

## Task Architecture

```text
retire-skill-management
        |
        v
prompt-library-foundations
        |
        v
prompt-evaluation-loop
        |
        v
parent integration review
```

The first child simplifies the product boundary. The second creates stable,
complete prompt revisions and safe data evolution. The third may then reference
those revisions from run and evaluation records.

## Skill Retirement Boundary

- Remove every reachable Skill UI, bridge capability, backend command/service,
  managed runtime path, selective export option, and product claim.
- Move the public-network/hostname helpers used by media download from
  `skill_safety` to a neutral SSRF module before deleting Skill services.
- Stop creating Skill tables on fresh databases, but do not issue a destructive
  migration against existing databases.
- Stop creating/managing the Skill directory. Existing directories are left on
  disk untouched and are not reported as active runtime paths.
- Old full backups may still contain dormant Skill tables/files. Restore treats
  them as opaque legacy data; the current application never exposes them.

## Prompt Foundation Boundary

- Introduce ordered schema migrations before any prompt schema evolution.
- Replace uncounted one-page search with `{ items, total, limit, offset }` or a
  cursor contract and expose deterministic paging/virtual loading in the UI.
- Define a complete immutable `PromptRevision` snapshot. It includes prompt
  type, ordered messages/content, variables, tags, folder/provenance metadata,
  media references, notes, and the reproducibility metadata selected by design.
- Saving content creates a revision atomically; rollback creates a new revision
  from the selected snapshot rather than mutating history.
- Add side-by-side/unified diff for every versioned field.
- Define a versioned portable manifest plus referenced media files. Import uses
  preview, validation, conflict policy, and automatic safety backup.
- Wire private prompt fields to encryption and exclude protected plaintext from
  FTS while locked. Provider secrets use the same protected storage boundary.

## Evaluation Boundary

- `PromptDefinition`: text or ordered chat messages with declared variables.
- `ExecutionProfile`: provider/model/parameters and encrypted credentials,
  separate from prompt content.
- `PromptRun`: immutable link to revision + profile + rendered inputs + output +
  status + timing + optional usage/cost metadata.
- `TestCase`: variable inputs, optional expected output, annotations.
- `Evaluator`: manual pass/fail, exact/contains/regex, numeric threshold, or
  explicitly configured LLM judge in later scope.
- `EvaluationRun`: matrix of revisions/profiles/test cases with cached results
  and a summary that never hides individual failures.

All outbound requests continue through the backend SSRF policy. The frontend
does not assemble arbitrary credential-bearing HTTP requests.

## Compatibility and Rollback

- Before the first schema migration or import, create a verified local backup.
- Each child is independently revertible. Skill retirement never deletes user
  data, so reverting restores code access without a data restore.
- Portable format versions are explicit; unsupported future versions fail with
  a structured error before writes.
- Evaluation migrations are additive and do not change existing prompt content.

## Deferred Scope

- Team roles/approvals, webhooks, hosted deployment labels, public marketplace,
  online production traces, traffic A/B routing, and CI/CD.
- Prompt composition/references and experiment branches until the linear
  revision/evaluation loop is proven usable.
