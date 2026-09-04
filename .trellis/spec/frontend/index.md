# Frontend Development Guidelines

> Project-specific conventions for the React/TypeScript frontend in `src/**`.

---

## Overview

PromptHub Desktop uses React 18, TypeScript 5, Vite, Zustand, TailwindCSS
design tokens, `react-i18next`, and `lucide-react`. The frontend reaches the
Tauri backend only through the Runtime Bridge in `src/runtime`.

These guidelines document the codebase as it exists today. They are meant for
AI agents and developers who need to write code that matches local patterns,
not a wishlist for a future rewrite.

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | Module organization and file layout | Filled |
| [Component Guidelines](./component-guidelines.md) | Component patterns, props, composition, styling, accessibility | Filled |
| [Hook Guidelines](./hook-guidelines.md) | Built-in hooks, Zustand hook usage, future custom-hook rules | Filled |
| [State Management](./state-management.md) | Local state, feature stores, server-backed state, selectors | Filled |
| [Quality Guidelines](./quality-guidelines.md) | Forbidden patterns, tests, review checks, verification commands | Filled |
| [Type Safety](./type-safety.md) | TypeScript strictness, DTO mirrors, validation, narrowing | Filled |
| [Design Tokens](./design-tokens.md) | Channel-only HSL tokens, appearance override split, mono stack | Filled |
| [Prompt References](./prompt-references.md) | `@@Title` token syntax and copy expansion | Filled |

---

## Pre-Development Checklist

Always start with `src/code_map.md` before broad searches in `src/**`.

For any frontend change:
- Read [Directory Structure](./directory-structure.md).
- Read [Quality Guidelines](./quality-guidelines.md).
- Read [Type Safety](./type-safety.md).

For component or view work:
- Read [Component Guidelines](./component-guidelines.md).
- Read [Hook Guidelines](./hook-guidelines.md).
- Read [State Management](./state-management.md) if the component reads or
  mutates store state.

For bridge/API/store work:
- Read [State Management](./state-management.md).
- Read [Type Safety](./type-safety.md).
- Check `src/runtime/index.ts` and the feature's `api.ts` before touching
  backend calls.

For i18n, theme, appearance, or UI quality work:
- Read [Component Guidelines](./component-guidelines.md).
- Read [Quality Guidelines](./quality-guidelines.md).
- Read [Design Tokens](./design-tokens.md).
- Check `src/locales/*.json`, `src/styles/globals.css`, and Tailwind token usage.

---

## Shared Rules

- The cross-boundary rules (Runtime Bridge, wire names, `CommandResult`) live
  in the root `AGENTS.md`; [Quality Guidelines](./quality-guidelines.md) holds
  the checklist form.
- User-facing text goes through `useTranslation()` and dot-notation i18n keys.
- Styling uses Tailwind token classes such as `bg-card`, `text-muted-foreground`,
  `border-border`, `bg-primary/15`, and `focus:ring-ring`.
- Icons come from `lucide-react`.
- Feature modules keep backend access in `api.ts`, state in `<feature>Store.ts`,
  DTOs in `types.ts`, and UI in `components/`.
- Tests are colocated as `*.test.ts` / `*.test.tsx`; i18n coverage lives in
  per-feature `i18nKeys.test.ts` files.

---

**Language**: All documentation and comments in this repository are written in
English.
