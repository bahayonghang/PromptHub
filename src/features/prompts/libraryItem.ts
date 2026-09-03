import type { PromptCopySource } from "./promptText";
import type { PromptListItem, PromptType, PromptTypeDefinition } from "./types";

export const LIBRARY_TAG_LIMIT = 3;

export interface LibraryItem {
  id: string;
  title: string;
  description: string;
  tags: string[];
  overflowTagCount: number;
  typeLabel: string;
  typeKind: PromptType;
  usageCount: number;
  updatedLabel: string;
  versionLabel: string;
  isFavorite: boolean;
  isPinned: boolean;
  isPrivate: boolean;
  isLocked: boolean;
  source: PromptCopySource;
}

export type LibraryTranslator = (key: string, options?: Record<string, unknown>) => string;

function typeLabel(
  prompt: PromptListItem,
  definitions: readonly PromptTypeDefinition[],
  t: LibraryTranslator,
): string {
  if (prompt.typeDefinitionId) {
    const custom = definitions.find((item) => item.id === prompt.typeDefinitionId);
    if (custom) return custom.name;
  }
  if (prompt.promptType === "image") return t("promptsView.editor.typeImage");
  if (prompt.promptType === "video") return t("promptsView.editor.typeVideo");
  return t("promptsView.editor.typeText");
}

function updatedLabel(value: string): string {
  const date = value.slice(0, 10);
  return date.length === 10 ? date : value;
}

const EMPTY_COPY_SOURCE: PromptCopySource = {
  userPrompt: "",
  messages: [],
  variables: [],
};

/** Maps a stored prompt onto the fields both library renderers consume. */
export function toLibraryItem(
  prompt: PromptListItem,
  definitions: readonly PromptTypeDefinition[],
  t: LibraryTranslator,
): LibraryItem {
  const title = prompt.title.trim() || t("promptsView.untitled");
  const description = prompt.isLocked
    ? t("promptsView.privateLockedPreview")
    : prompt.description?.trim() || t("promptsView.noDescription");
  const tags = prompt.tags.slice(0, LIBRARY_TAG_LIMIT);
  return {
    id: prompt.id,
    title,
    description,
    tags,
    overflowTagCount: Math.max(0, prompt.tags.length - LIBRARY_TAG_LIMIT),
    typeLabel: typeLabel(prompt, definitions, t),
    typeKind: prompt.promptType,
    usageCount: prompt.usageCount,
    updatedLabel: updatedLabel(prompt.updatedAt),
    versionLabel: `v${prompt.currentVersion}`,
    isFavorite: prompt.isFavorite,
    isPinned: prompt.isPinned,
    isPrivate: prompt.isPrivate,
    isLocked: prompt.isLocked,
    source: EMPTY_COPY_SOURCE,
  };
}
