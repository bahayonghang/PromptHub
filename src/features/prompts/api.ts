/**
 * Thin command wrappers for the prompt-editing view (Req 6, 7, 8). Every call is
 * routed through the Runtime_Bridge (Req 3.1); none touches `@tauri-apps/api`
 * directly. Command names follow the design's `domain.action` convention and
 * argument/field names use the camelCase DTO shapes the backend returns.
 */
import { runtime, type RuntimeBridge } from "../../runtime";
import type {
  CreateFolderInput,
  CreatePromptInput,
  Folder,
  Prompt,
  PromptVersion,
  SearchQuery,
  UpdateFolderInput,
  UpdatePromptInput,
} from "./types";

/** The backend command surface this view depends on, grouped for injection. */
export interface PromptApi {
  listPrompts(): Promise<Prompt[]>;
  getPrompt(id: string): Promise<Prompt>;
  searchPrompts(query: SearchQuery): Promise<Prompt[]>;
  createPrompt(input: CreatePromptInput): Promise<Prompt>;
  updatePrompt(id: string, patch: UpdatePromptInput): Promise<Prompt>;
  deletePrompt(id: string): Promise<void>;
  copyPrompt(id: string, values: Record<string, string>): Promise<string>;

  listTags(): Promise<string[]>;

  listFolders(): Promise<Folder[]>;
  createFolder(input: CreateFolderInput): Promise<Folder>;
  updateFolder(id: string, patch: UpdateFolderInput): Promise<Folder>;
  deleteFolder(id: string): Promise<void>;
  reorderFolders(orderedIds: string[]): Promise<void>;

  listVersions(promptId: string): Promise<PromptVersion[]>;
  createVersion(promptId: string, note?: string): Promise<PromptVersion>;
  rollbackVersion(promptId: string, version: number): Promise<Prompt>;
  deleteVersion(versionId: string): Promise<void>;
}

/**
 * Builds the {@link PromptApi} bound to a Runtime_Bridge (the live `runtime` by
 * default). Tests inject a fake bridge to drive the view without a backend.
 */
export function createPromptApi(bridge: RuntimeBridge = runtime): PromptApi {
  return {
    listPrompts: () => bridge.invoke<Prompt[]>("prompt.list"),
    getPrompt: (id) => bridge.invoke<Prompt>("prompt.get", { id }),
    searchPrompts: (query) => bridge.invoke<Prompt[]>("prompt.search", { query }),
    createPrompt: (input) => bridge.invoke<Prompt>("prompt.create", { input }),
    updatePrompt: (id, patch) =>
      bridge.invoke<Prompt>("prompt.update", { id, patch }),
    deletePrompt: (id) => bridge.invoke<void>("prompt.delete", { id }),
    copyPrompt: (id, values) =>
      bridge.invoke<string>("prompt.copy", { id, values }),

    listTags: () => bridge.invoke<string[]>("tag.list"),

    listFolders: () => bridge.invoke<Folder[]>("folder.list"),
    createFolder: (input) => bridge.invoke<Folder>("folder.create", { input }),
    updateFolder: (id, patch) =>
      bridge.invoke<Folder>("folder.update", { id, patch }),
    deleteFolder: (id) => bridge.invoke<void>("folder.delete", { id }),
    reorderFolders: (orderedIds) =>
      bridge.invoke<void>("folder.reorder", { orderedIds }),

    listVersions: (promptId) =>
      bridge.invoke<PromptVersion[]>("version.list", { promptId }),
    createVersion: (promptId, note) =>
      bridge.invoke<PromptVersion>("version.create", { promptId, note }),
    rollbackVersion: (promptId, version) =>
      bridge.invoke<Prompt>("version.rollback", { promptId, version }),
    deleteVersion: (versionId) =>
      bridge.invoke<void>("version.delete", { id: versionId }),
  };
}

/** The production prompt API bound to the live Runtime_Bridge. */
export const promptApi: PromptApi = createPromptApi();
