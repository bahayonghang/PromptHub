import type { Prompt, PromptVersion } from "./types";

export type PromptRevisionField =
  | "title"
  | "description"
  | "promptType"
  | "systemPrompt"
  | "userPrompt"
  | "messages"
  | "variables"
  | "tags"
  | "folder"
  | "images"
  | "videos"
  | "favorite"
  | "pinned"
  | "private"
  | "source"
  | "notes"
  | "aiResponse";

export interface PromptRevisionDiff {
  field: PromptRevisionField;
  revisionValue: string;
  currentValue: string;
}

function sameValue(left: unknown, right: unknown): boolean {
  return JSON.stringify(left ?? null) === JSON.stringify(right ?? null);
}

function displayValue(value: unknown): string {
  if (value == null || value === "") return "-";
  if (Array.isArray(value)) {
    if (value.length === 0) return "-";
    if (value.every((item) => typeof item === "string")) return value.join(", ");
    return JSON.stringify(value);
  }
  return String(value);
}

export function diffPromptRevision(
  prompt: Prompt,
  revision: PromptVersion,
): PromptRevisionDiff[] {
  const values: Array<[PromptRevisionField, unknown, unknown]> = [
    ["title", revision.title, prompt.title],
    ["description", revision.description, prompt.description],
    ["promptType", revision.promptType, prompt.promptType],
    ["systemPrompt", revision.systemPrompt, prompt.systemPrompt],
    ["userPrompt", revision.userPrompt, prompt.userPrompt],
    ["messages", revision.messages, prompt.messages],
    ["variables", revision.variables, prompt.variables],
    ["tags", revision.tags, prompt.tags],
    ["folder", revision.folderId, prompt.folderId],
    ["images", revision.images, prompt.images],
    ["videos", revision.videos, prompt.videos],
    ["favorite", revision.isFavorite, prompt.isFavorite],
    ["pinned", revision.isPinned, prompt.isPinned],
    ["private", revision.isPrivate, prompt.isPrivate],
    ["source", revision.source, prompt.source],
    ["notes", revision.notes, prompt.notes],
    ["aiResponse", revision.aiResponse, prompt.lastAiResponse],
  ];

  return values
    .filter(([, revisionValue, currentValue]) =>
      !sameValue(revisionValue, currentValue),
    )
    .map(([field, revisionValue, currentValue]) => ({
      field,
      revisionValue: displayValue(revisionValue),
      currentValue: displayValue(currentValue),
    }));
}
