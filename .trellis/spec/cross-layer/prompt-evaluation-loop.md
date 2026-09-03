# Prompt Evaluation Loop

## 1. Scope / Trigger

Use this contract when changing chat prompt definitions, execution profiles,
provider requests, prompt runs, test sets, evaluators, evaluation matrices,
cache behavior, or baseline/candidate labels. The flow crosses SQLite, Rust
services, Tauri commands/events, the Runtime Bridge, Zustand, and React.

## 2. Signatures

Core wire commands:

```text
evaluation.profileList() -> ExecutionProfileRevision[]
evaluation.profileSave({ input }) -> ExecutionProfileRevision
evaluation.render({ promptRevisionId, inputs }) -> RenderedPrompt
evaluation.run({ requestId, input }) -> PromptRun
evaluation.cancel({ requestId }) -> void
evaluation.runList() -> PromptRun[]
evaluation.testSetList/Save/Import/Export(...) -> TestSet | TestSet[] | string
evaluation.evaluatorList/Create(...) -> EvaluatorConfig | EvaluatorConfig[]
evaluation.matrixRun({ requestId, input }) -> EvaluationRunDetail
evaluation.matrixRetry({ requestId, id }) -> EvaluationRunDetail
evaluation.matrixList/Get(...) -> EvaluationRun[] | EvaluationRunDetail
evaluation.manualResult(...) -> EvaluationCell
evaluation.labelList/Move/Rollback/History(...) -> label DTOs
```

Events:

```text
evaluation:run-chunk       { runId, chunk }
evaluation:run-terminal    { runId, status }
evaluation:matrix-progress { evaluationRunId, completed, total, cellId }
```

Schema version 4 adds `prompts.messages`, `prompt_versions.messages`, and the
evaluation tables `execution_profile_revisions`, `prompt_runs`, `test_sets`,
`test_cases`, `evaluator_configs`, `evaluation_runs`, `evaluation_cells`,
`prompt_labels`, and `prompt_label_history`.

## 3. Contracts

- `messages` is an ordered JSON array of `{ role, content }`; role is `system`,
  `user`, or `assistant`. An empty array means the legacy text definition uses
  `systemPrompt` plus `userPrompt`.
- Rendering happens in Rust against an exact immutable prompt revision. Required
  variables must resolve from inputs or defaults before any provider call.
- Profile changes append `ExecutionProfileRevision`; they never mutate an older
  revision. DTOs expose only `hasCredential`, never the credential value.
- Credentials are AES-GCM `ENC::` values. Creating or using a credential
  requires the library to be unlocked, and master-password changes re-key every
  profile revision atomically with private Prompt content.
- Native provider traffic is constructed in Rust. `openai-compatible` endpoints
  pass shared DNS-pinned SSRF validation on the initial URL and every redirect.
  Frontend code never sends arbitrary credential-bearing headers or bodies.
- OpenAI-compatible SSE is decoded one complete line at a time while bytes
  arrive. Each content delta emits `evaluation:run-chunk` immediately; buffering
  the complete response before emitting chunks does not satisfy streaming.
- Every started request writes a `prompt_runs` row and ends as `success`,
  `error`, or `cancelled`, including duration and optional provider usage.
- Matrix ordering is revision, then profile, then test-case order. Cancellation
  persists remaining cells as cancelled. Retry creates a new attributable run.
- A cell-level preflight failure, such as a missing render variable, is persisted
  as terminal `error` evidence without a `promptRunId`; it must not leave the
  cell or parent evaluation run in `running` or abort later cells.
- Cache keys include exact prompt revision, profile revision, normalized inputs,
  expected output, evaluator configs, and `RUNTIME_VERSION`. Only successful
  cells are reusable; cache hits remain visible on the new cell.
- Manual results stay skipped until explicitly reviewed. Deterministic evaluator
  evidence is retained per cell; failed and skipped cases remain inspectable.
- `baseline` and `candidate` are movable pointers. A move or rollback appends
  `prompt_label_history` and never mutates prompt/evaluation history.

## 4. Validation & Error Matrix

| Condition | Error / behavior |
|-----------|------------------|
| Text body and messages are both empty | `VALIDATION`; no Prompt write |
| Message role is outside the three-role catalog | `VALIDATION`; no revision |
| Required render variable is missing | `VALIDATION`; no provider call/run; a matrix records a terminal error cell and continues |
| Credential is saved or used while locked | `UNAUTHORIZED`; no plaintext; no `prompt_runs` row |
| Provider endpoint is local/private or resolves privately | `SSRF_BLOCKED` before the initial `prompt_runs` insert and again on every redirect hop |
| Provider redirects to a blocked host | `SSRF_BLOCKED` before that hop |
| Provider timeout/network/malformed response | terminal `error` run record |
| Request is cancelled | terminal `cancelled` run/cells; no failed score |
| Regex config is invalid | `VALIDATION`; evaluator is not created |
| Numeric output/config is invalid | failed result with retained evidence |
| Matrix input omits revisions/profiles/evaluators/cases | `VALIDATION` |
| Label target was not successfully evaluated | `VALIDATION`; label unchanged |

## 5. Good / Base / Bad Cases

- Good: render a three-message few-shot revision, run two revisions across two
  profiles and 20 cases, inspect every result, rerun with visible cache hits,
  then move `candidate` with an audit entry.
- Base: run a public text revision with a credential-free deterministic mock;
  import/export an empty or simple test set; manual results remain skipped.
- Bad: save a plaintext key, call localhost, buffer an SSE response before
  emitting chunks, abort a matrix on one preflight error, reuse cache after an
  evaluator change, lose message order, treat cancellation as failure, or show
  an aggregate without failed/skipped cell evidence.

## 6. Tests Required

- Prompt/revision: role order, few-shot assistant messages, Unicode variables,
  private at-rest ciphertext, locked redaction, re-key, rollback, and portable
  round-trip.
- Profile/provider: DTO redaction, encrypted storage, re-key, SSRF local/private
  rejection with no `prompt_runs` row, redirect re-check, timeout/cancel,
  malformed response, and optional usage provenance. Live-provider smoke is
  opt-in only.
- Run/matrix: success/error/cancel persistence, restart reads, deterministic
  ordering, 20-case progress, preflight-error terminalization, partial failure,
  retry, cache hit and invalidation. A cancelled matrix must not persist
  remaining cells as `error` or `running`.
- Provider streaming: split SSE input across transport chunks and assert each
  completed content line emits immediately while usage metadata is retained.
- Evaluators/labels: exact/contains/regex/numeric/manual evidence, manual update,
  aggregate visibility, evaluated-target enforcement, history and rollback.
- Frontend: bridge command arguments/events, injected store state, i18n namespace,
  keyboard-accessible controls, non-color status, narrow/default desktop layouts.
- Finish with `just ci` and an isolated native Tauri deterministic-mock smoke.

## 7. Wrong vs Correct

### Wrong

```ts
runtime.invoke("ai.request", {
  request: { url: profile.endpoint, headers: { Authorization: secret } },
});
```

This exposes a secret to the frontend and lets UI code bypass profile revision,
SSRF, rendering, run persistence, and cache contracts.

### Correct

```ts
evaluationApi.run(requestId, {
  promptRevisionId,
  profileRevisionId,
  inputs,
});
```

The backend resolves the encrypted profile, renders the immutable revision,
checks every outbound hop, persists the terminal run, and emits typed progress.
