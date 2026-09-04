# New Prompt title focus loss root cause

## Symptom

In create mode, entering the first title character retained the value but moved
focus from the title input to the top-right `Read only` pencil button. Further
typing therefore stopped updating the title.

## Red-capable loop

The isolated research test renders the real `PromptDetailModal` create path,
focuses `Title`, changes its value to `u`, waits for pending focus work, and
asserts that the title input remains `document.activeElement`.

```powershell
rtk npx.cmd vitest run --config .trellis/tasks/archive/2026-08/08-27-fix-new-prompt-title-focus/research/vitest.config.ts --root .
```

Before the fix, the assertion deterministically received the `Read only`
button instead of the title input. After the fix, the research test passes.

## Confirmed cause

`PromptDetailModal` creates `requestClose` during each render. The shared
`Modal` entry/exit focus effect depended on both `open` and `onClose`, so every
title state update changed the callback identity and restarted the entire focus
lifecycle. The restarted animation-frame callback focused the first enabled
control, which is the pencil button in create mode.

The title input remained the same DOM node, read-only state did not change, and
the isolated component did not mount global shortcuts. These probes ruled out
input remounting, read-only toggling, and shortcut interception.

## Resolution and evidence

The modal stack now stores only its stable entry id, and the focus lifecycle is
keyed only to `open`. Rendered Escape and scrim handlers continue to invoke the
latest `onClose` prop.

- Focused Modal and Prompt detail tests: 7 passed.
- Archived research reproduction: 1 passed.
- Full `just ci`: frontend build and 320 tests, Rust fmt, Clippy, 380 unit tests,
  property tests, and documentation tests passed.
- Native Windows WebView interaction remains `UNVERIFIED`.
