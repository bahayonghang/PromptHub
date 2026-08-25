/**
 * Pure helpers for prompt text: extracting `{{variable}}` placeholders,
 * substituting values for preview and clipboard copy, and deriving text fields
 * from chat messages (Req 6.7, 6.11). Kept React-free so the parsing rules can
 * be unit-tested directly.
 */
import type { PromptMessage, PromptMessageRole, Variable } from "./types";

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

/** Clipboard source shared by list rows and the editor draft. */
export interface PromptCopySource {
  systemPrompt?: string | null;
  userPrompt: string;
  messages: PromptMessage[];
  variables: Variable[];
}

const COPY_ROLE_LABELS: Record<PromptMessageRole, string> = {
  system: "System",
  user: "User",
  assistant: "Assistant",
};

/**
 * Builds a substitution map from declared non-empty `defaultValue`s. Names
 * without a default are omitted so unmatched placeholders stay intact.
 */
export function defaultVariableValues(
  variables: readonly Variable[],
): Record<string, string> {
  const values: Record<string, string> = {};
  for (const variable of variables) {
    if (variable.defaultValue != null && variable.defaultValue !== "") {
      values[variable.name] = variable.defaultValue;
    }
  }
  return values;
}

/**
 * Derives the stored system/user text fields from chat messages, matching the
 * editor's leave-chat conversion: first system message and last user message.
 */
/**
 * Builds a chat-mode message list from the stored system/user text fields.
 * Used when the editor prefers chat but a prompt still has empty `messages`.
 */
export function seedChatMessages(
  systemPrompt?: string | null,
  userPrompt = "",
): PromptMessage[] {
  const messages: PromptMessage[] = [];
  if ((systemPrompt ?? "").trim() !== "") {
    messages.push({ role: "system", content: systemPrompt ?? "" });
  }
  messages.push({ role: "user", content: userPrompt });
  return messages;
}

export function deriveTextFieldsFromMessages(
  messages: readonly PromptMessage[],
): { systemPrompt: string; userPrompt: string } {
  const system = messages.find((message) => message.role === "system");
  const user = [...messages]
    .reverse()
    .find((message) => message.role === "user");
  return {
    systemPrompt: system?.content ?? "",
    userPrompt: user?.content ?? "",
  };
}

function formatLabeledBlock(label: string, content: string): string {
  return `[${label}]\n${content}`;
}

/**
 * Builds paste-ready clipboard text. Chat mode (`messages.length > 0`) joins
 * labeled message blocks. Text mode emits `[System]` / `[User]` when a
 * non-whitespace system prompt exists, otherwise the user prompt only.
 */
export function formatCopiedPrompt(parts: {
  systemPrompt?: string | null;
  userPrompt: string;
  messages: readonly { role: PromptMessageRole; content: string }[];
}): string {
  if (parts.messages.length > 0) {
    return parts.messages
      .map((message) =>
        formatLabeledBlock(COPY_ROLE_LABELS[message.role], message.content),
      )
      .join("\n\n");
  }
  const system = parts.systemPrompt ?? "";
  if (system.trim() === "") {
    return parts.userPrompt;
  }
  return `${formatLabeledBlock("System", system)}\n\n${formatLabeledBlock("User", parts.userPrompt)}`;
}

export function buildPromptCopyText(source: PromptCopySource): string {
  const values = defaultVariableValues(source.variables);
  if (source.messages.length > 0) {
    return formatCopiedPrompt({
      userPrompt: "",
      messages: source.messages.map((message) => ({
        role: message.role,
        content: substituteVariables(message.content, values),
      })),
    });
  }
  return formatCopiedPrompt({
    systemPrompt: substituteVariables(source.systemPrompt ?? "", values),
    userPrompt: substituteVariables(source.userPrompt, values),
    messages: [],
  });
}
