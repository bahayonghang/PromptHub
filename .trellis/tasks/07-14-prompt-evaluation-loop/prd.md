# Build prompt evaluation loop

## Goal

Turn prompt editing from trial-and-error into a local, reproducible loop of run,
compare, evaluate, and promote a better revision.

## Preconditions

- `07-14-prompt-library-foundations` has delivered stable complete revision ids,
  migrations, private data handling, and portable backup behavior.
- Skill retirement is complete; this work does not reintroduce Skill/Agent/MCP
  management.

## Requirements

- R1: Support simple text and ordered chat-message prompt definitions with roles,
  few-shot assistant examples, variables, and optional message placeholders.
- R2: Store reusable execution profiles for provider/model/parameters while
  encrypting credentials and enforcing backend SSRF policy.
- R3: Provide a single-run playground with rendered-input preview, streaming,
  cancellation, structured error state, and explicit save behavior.
- R4: Persist immutable run records linked to exact prompt revision, execution
  profile revision, variable inputs, rendered messages, output, status, timing,
  and optional provider usage/cost metadata.
- R5: Let users create test sets containing variable inputs, optional expected
  output, and annotations, and import/export them in a documented format.
- R6: MVP evaluators include manual pass/fail, exact match, contains, regex, and
  numeric threshold. LLM-as-judge is deferred until deterministic evaluation is
  trustworthy.
- R7: Run a matrix across selected prompt revisions/profiles/test cases, cache
  identical completed cells, and show side-by-side outputs plus per-case and
  aggregate results.
- R8: Allow a successful evaluated revision to receive a movable local label such
  as `baseline` or `candidate`; moving a label does not mutate history.
- R9: Keep all runs local except explicit provider calls; never enable hosted
  telemetry or upload datasets by default.

## Acceptance Criteria

- [ ] AC1: Text and multi-message chat prompts render variables deterministically
  and revision round-trip without losing role/order/content.
- [ ] AC2: Provider secrets are never returned to the frontend, logged, exported
  by default, or stored in plaintext.
- [ ] AC3: Run records reproduce the exact revision/profile/input combination and
  retain terminal success/cancel/error status across restart.
- [ ] AC4: A test-set matrix can compare at least 2 revisions or models across 20
  cases with stable progress, cancellation, and retry of failed cells.
- [ ] AC5: Evaluator results retain individual evidence; aggregate scores never
  hide failed/skipped cases.
- [ ] AC6: Identical matrix reruns use cache only when revision, profile, inputs,
  evaluator config, and relevant runtime version match.
- [ ] AC7: Baseline/candidate label history and rollback are attributable.
- [ ] AC8: Frontend/Rust gates plus native provider-mocked and one opt-in live
  smoke test pass.

## Out of Scope

- Online production tracing, hosted observability, traffic A/B routing, CI/CD,
  team approvals, webhooks, or public sharing.
- Autonomous prompt optimization and LLM-as-judge in the MVP.
- Provider-specific tool execution or Agent runtime management.
