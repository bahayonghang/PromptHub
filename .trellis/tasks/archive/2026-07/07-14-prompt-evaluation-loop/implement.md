# Implementation Plan: Local Prompt Evaluation Loop

## Ordered Checklist

1. Extend prompt definition/revision contracts for ordered chat messages and
   renderer tests.
2. Add encrypted execution-profile revisions and provider adapter abstraction.
3. Implement single-run playground, streaming/cancel event contracts, and run
   persistence.
4. Add run-history query/filter/detail UI and retention controls.
5. Add test sets/cases plus portable import/export.
6. Add deterministic/manual evaluator contracts and per-run results.
7. Add cancellable/retryable evaluation matrix, conservative cache, progress
   events, and side-by-side inspection.
8. Add baseline/candidate labels and label-history UI.
9. Update product docs/code maps and run full/native verification.

## Targeted Verification

- Renderer property tests for role order, Unicode, variables, missing required
  values, and message placeholders.
- Provider adapter tests for redaction, timeout, cancel, redirects, SSRF blocks,
  malformed responses, and usage metadata.
- Persistence tests for success/error/cancel and app restart.
- Matrix tests for deterministic ordering, concurrency limit, retry, partial
  failure, cancellation, and cache invalidation.
- Accessibility tests for keyboard matrix navigation, progress announcements,
  focus restoration, and non-color score/status indicators.

## Full Gate

```powershell
just ci
```

Native smoke uses a local deterministic mock provider by default. A real-provider
smoke is optional, explicit, and never part of unattended CI.

Run the opt-in live smoke only with a user-configured public endpoint:

```powershell
$env:PROMPTHUB_LIVE_OPENAI_ENDPOINT = "https://provider.example/v1/chat/completions"
$env:PROMPTHUB_LIVE_OPENAI_MODEL = "model-id"
$env:PROMPTHUB_LIVE_OPENAI_API_KEY = "..." # optional for credential-free endpoints
cargo test --manifest-path src-tauri/Cargo.toml live_openai_compatible_smoke -- --ignored --nocapture
```

## Rollback

Evaluation storage is additive. Disable/remove its navigation and command
registration, then roll back additive tables through the migration policy only
after preserving run/test data in a portable export.
