# Design: Local Prompt Evaluation Loop

## Domain Model

- `PromptRevision`: immutable complete prompt definition from the foundation task.
- `ExecutionProfileRevision`: provider, model, request parameters, and a backend
  reference to encrypted credentials.
- `PromptRun`: revision/profile/input/output/status/timing/usage record.
- `TestSet` and `TestCase`: reusable inputs, expected output, annotations.
- `EvaluatorConfig` and `EvaluationResult`: deterministic/manual scoring contract.
- `EvaluationRun` and `EvaluationCell`: matrix definition, progress, cached output,
  score, and failure evidence.
- `PromptLabel`: movable `baseline`/`candidate` pointer with audit history.

## Execution Flow

```text
revision + profile + variables
        -> backend render/validate
        -> SSRF-safe provider adapter
        -> stream/cancel
        -> immutable run record
        -> evaluator(s)
        -> comparison matrix + optional label move
```

Rendering and provider request construction belong to Tauri services. The React
frontend sends typed ids/inputs and receives sanitized events/DTOs; it never
receives stored secrets or builds raw authenticated HTTP requests.

## Reproducibility

- Profile changes create profile revisions.
- Cache keys include prompt revision, profile revision, normalized inputs,
  evaluator config, and renderer/adapter version.
- Provider-reported token/cost fields are optional and provenance-tagged; missing
  values are displayed as unavailable, never estimated silently.
- Cancellation writes a terminal record and does not count as a failed score.

## UX Shape

- Editor supports text/chat mode with ordered role rows and variable validation.
- Playground is an unframed work surface beside the prompt editor, not a separate
  dashboard.
- Evaluation view uses a dense matrix/table with inspectable cells and a compare
  drawer for output/diff/score evidence.
- Run history filters by revision, profile, status, test set, and date.

## Security and Privacy

- All outbound endpoints pass existing SSRF checks including redirects.
- Headers/keys remain encrypted backend state and are redacted in errors.
- Local test cases and outputs follow private-prompt lock/export policy.
- Live-provider tests are opt-in and use user-configured credentials; normal CI
  uses deterministic mocked adapters.
