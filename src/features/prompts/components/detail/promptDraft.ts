import type {
  CreatePromptInput,
  Prompt,
  PromptMessage,
  PromptType,
  UpdatePromptInput,
  Variable,
} from "../../types";
import { preferredChatMode } from "../../definitionMode";
import {
  deriveTextFieldsFromMessages,
  seedChatMessages,
} from "../../promptText";

/** The editable draft backing the form, independent of the saved prompt. */
export interface PromptDraft {
  title: string;
  description: string;
  promptType: PromptType;
  typeDefinitionId: string | null;
  folderId: string | null;
  systemPrompt: string;
  userPrompt: string;
  messages: PromptMessage[];
  variables: Variable[];
  tags: string[];
  images: string[];
  videos: string[];
  source: string;
  notes: string;
  isPrivate: boolean;
}

/** Builds a fresh draft from a saved prompt, or a blank draft for creation. */
export function toDraft(prompt: Prompt | null): PromptDraft {
  return {
    title: prompt?.title ?? "",
    description: prompt?.description ?? "",
    promptType: prompt?.promptType ?? "text",
    typeDefinitionId: prompt?.typeDefinitionId ?? null,
    folderId: prompt?.folderId ?? null,
    systemPrompt: prompt?.systemPrompt ?? "",
    userPrompt: prompt?.userPrompt ?? "",
    messages:
      prompt != null && prompt.messages.length > 0
        ? prompt.messages
        : preferredChatMode()
          ? seedChatMessages(prompt?.systemPrompt, prompt?.userPrompt ?? "")
          : [],
    variables: prompt?.variables ?? [],
    tags: prompt?.tags ?? [],
    images: prompt?.images ?? [],
    videos: prompt?.videos ?? [],
    source: prompt?.source ?? "",
    notes: prompt?.notes ?? "",
    isPrivate: prompt?.isPrivate ?? false,
  };
}

export function isChatMode(draft: PromptDraft): boolean {
  return draft.messages.length > 0;
}

export function titleIsValid(draft: PromptDraft): boolean {
  return draft.title.trim() !== "";
}

export function userPromptIsValid(draft: PromptDraft): boolean {
  if (isChatMode(draft)) {
    return (
      draft.messages.length > 0 &&
      draft.messages.every((message) => message.content.trim() !== "")
    );
  }
  return draft.userPrompt.trim() !== "";
}

export function canSubmitDraft(draft: PromptDraft): boolean {
  return titleIsValid(draft) && userPromptIsValid(draft);
}

export function textFieldsFromDraft(draft: PromptDraft): {
  systemPrompt: string;
  userPrompt: string;
} {
  if (isChatMode(draft)) {
    return deriveTextFieldsFromMessages(draft.messages);
  }
  return {
    systemPrompt: draft.systemPrompt,
    userPrompt: draft.userPrompt,
  };
}

export function toCreateInput(draft: PromptDraft): CreatePromptInput {
  const textFields = textFieldsFromDraft(draft);
  return {
    title: draft.title.trim(),
    userPrompt: textFields.userPrompt,
    promptType: draft.promptType,
    typeDefinitionId: draft.typeDefinitionId,
    description: draft.description || undefined,
    systemPrompt: textFields.systemPrompt || undefined,
    messages: draft.messages,
    variables: draft.variables,
    tags: draft.tags,
    folderId: draft.folderId,
    images: draft.images,
    videos: draft.videos,
    source: draft.source || undefined,
    notes: draft.notes || undefined,
    isPrivate: draft.isPrivate,
  };
}

export function toUpdatePatch(draft: PromptDraft): UpdatePromptInput {
  const textFields = textFieldsFromDraft(draft);
  return {
    title: draft.title.trim(),
    description: draft.description,
    promptType: draft.promptType,
    typeDefinitionId: draft.typeDefinitionId,
    systemPrompt: textFields.systemPrompt,
    userPrompt: textFields.userPrompt,
    messages: draft.messages,
    variables: draft.variables,
    tags: draft.tags,
    folderId: draft.folderId,
    images: draft.images,
    videos: draft.videos,
    source: draft.source,
    notes: draft.notes,
    isPrivate: draft.isPrivate,
  };
}

export function draftSnapshot(draft: PromptDraft): string {
  return JSON.stringify(toUpdatePatch(draft));
}
