# Type Safety

> Type safety patterns in this project.

---

## Overview

The frontend is TypeScript in `strict` mode with `noUnusedLocals`,
`noUnusedParameters`, and `noFallthroughCasesInSwitch`. The dominant pattern is
compile-time DTO mirroring plus small runtime guards/normalizers where user input
or persisted settings can be invalid.

There is no runtime schema library such as Zod. Do not add one for a small local
change unless the task explicitly calls for a broader validation layer.

---

## Type Organization

Feature DTOs live in `src/features/<feature>/types.ts` and mirror backend command
payloads. The backend serializes Rust structs as camelCase, so frontend fields
are camelCase too.

Example from `src/features/settings/types.ts`:

```ts
export interface DataPathStatus {
  activePath: string;
  configuredPath?: string | null;
  restartRequired: boolean;
}

export type DataPathAction = "migrate" | "switch" | "overwrite";
```

Use local interfaces for component props and store state. Export only what other
modules or tests need.

Use literal unions and readonly catalogs for bounded settings and option lists.
`src/appearance/index.ts` is the main example:

```ts
export type FontScale = "Small" | "Default" | "Large" | "Extra Large";
export const FONT_SCALES: readonly FontScale[] = [
  "Small",
  "Default",
  "Large",
  "Extra Large",
];
```

---

## Bridge and API Types

Every bridge wrapper supplies the expected result type to `bridge.invoke<T>()`:

```ts
getSettings: () => bridge.invoke<Settings>("settings.get"),
updateSettings: (patch) =>
  bridge.invoke<Settings>("settings.update", { patch }),
```

The Runtime Bridge itself mirrors the backend result envelope:

```ts
type CommandResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: { code: string; message: string; details?: unknown } };
```

When store action parameters should track an API method, derive them with
`Parameters<...>` instead of duplicating the shape:

```ts
createPrompt: (
  input: Parameters<PromptApi["createPrompt"]>[0],
) => Promise<Prompt | null>;
```

---

## Runtime Validation and Normalization

Use small pure helpers for frontend validation when immediate UI feedback is
needed. The backend remains the source of truth.

Example from `src/features/settings/validation.ts`:

```ts
export function validateNewPassword(
  password: string,
  confirm: string,
): string | null {
  const lengthError = validatePasswordLength(password);
  if (lengthError) return lengthError;
  if (password !== confirm) return "settingsView.security.passwordMismatch";
  return null;
}
```

Use total normalizers for persisted settings or unknown input that must never
crash the UI. `src/appearance/index.ts` normalizes each appearance field from
`unknown` and falls back to defaults.

Use type guards for bounded external/persisted values. `src/runtime/i18n.ts`
uses locale support checks before accepting an active language value.

---

## Unknown, Assertions, and Narrowing

Prefer `unknown` plus narrowing for caught errors and untrusted values:

```ts
function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}
```

Assertions are acceptable when they are bounded and immediately guarded by a
runtime check, such as checking a catalog before casting a string to a literal
union. Avoid assertions that simply silence TypeScript.

For nullable state, model nullability explicitly:
- `selectedPromptId: string | null`
- `settings: Settings | null`
- `configuredPath?: string | null`
- `downloaded: number | null`

---

## Forbidden Patterns

- Do not use `any` for command payloads, store state, or component props.
- Do not cast raw bridge results in components. Add a typed API wrapper.
- Do not invent frontend field names that differ from backend camelCase DTOs.
- Do not ignore `null`/`undefined` cases by assertion.
- Do not add unused imports, locals, or parameters; the TypeScript build rejects
  them.
- Do not parse user-facing data with ad hoc string casts when an existing helper
  or type guard covers the case.

---

## Common Mistakes

- Adding a new settings field to `Settings` but not to `SettingsPatch` usage,
  normalization, store merge logic, and tests.
- Adding a new command without typing the `bridge.invoke<T>()` result.
- Returning raw `Error` objects to store state instead of stable display strings.
- Forgetting that optional backend fields commonly use `?: T | null`, not only
  `?: T`.
