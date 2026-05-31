/**
 * Pure helpers for prompt text: extracting `{{variable}}` placeholders and
 * substituting values for the editor preview (Req 6.7, 6.11). Kept React-free
 * so the parsing rules can be unit-tested directly.
 */
import type { Variable } from "./types";

/**
 * Matches a `{{name}}` or `{{name:example}}` placeholder. Names are letters,
 * digits, underscores, dots, and hyphens; surrounding spaces inside the braces
 * are tolerated. The example segment (after `:`) is ignored for the name.
 */
const PLACEHOLDER_RE = /\{\{\s*([\w.-]+)\s*(?::[^}]*)?\}\}/g;

/**
 * Returns the distinct variable names referenced in `text`, in first-seen order
 * (Req 6.7). Returns an empty array when the text declares no placeholders.
 */
export function extractVariableNames(text: string): string[] {
  const seen = new Set<string>();
  const names: string[] = [];
  for (const match of text.matchAll(PLACEHOLDER_RE)) {
    const name = match[1];
    if (!seen.has(name)) {
      seen.add(name);
      names.push(name);
    }
  }
  return names;
}

/**
 * Reconciles a prompt's declared `variables` with the placeholders found across
 * its system and user prompt text. Variables still referenced are kept (their
 * metadata preserved); newly referenced names are appended as required text
 * variables; names no longer referenced are dropped. Order follows the text.
 */
export function syncVariables(
  existing: readonly Variable[],
  ...texts: string[]
): Variable[] {
  const names = extractVariableNames(texts.join("\n"));
  const byName = new Map(existing.map((v) => [v.name, v]));
  return names.map(
    (name) =>
      byName.get(name) ?? { name, type: "text", required: true },
  );
}

/**
 * Substitutes `{{name}}` / `{{name:example}}` placeholders in `text` with the
 * supplied `values`, leaving any placeholder whose name has no value unchanged
 * (Req 6.11). Used for the editor's local preview; the backend's `prompt.copy`
 * is the authoritative implementation.
 */
export function substituteVariables(
  text: string,
  values: Readonly<Record<string, string>>,
): string {
  return text.replace(PLACEHOLDER_RE, (whole, name: string) =>
    Object.prototype.hasOwnProperty.call(values, name) ? values[name] : whole,
  );
}
