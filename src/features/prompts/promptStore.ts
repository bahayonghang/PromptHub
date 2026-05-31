/**
 * View-state store for the prompt-editing view (Req 6, 7, 8). Holds the loaded
 * folders/prompts/tags, the active search and filter state, the current
 * selection, and the version history of the selected prompt. All backend access
 * goes through an injectable {@link PromptApi} (default: the live bridge-bound
 * API) so the store can be driven in tests without a backend (Req 3.1).
 */
import { create } from "zustand";
import { promptApi, type PromptApi } from "./api";
import type {
  Folder,
  Prompt,
  PromptVersion,
  SearchQuery,
  SortField,
  SortOrder,
} from "./types";

/** A `BridgeError`-shaped failure surfaced to the view (Req 3.5). */
function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

/** The mutable filter/sort state the user controls from the search bar. */
export interface PromptFilters {
  /** Free-text keyword (Req 5.3). Empty string means "no keyword". */
  keyword: string;
  /** Selected folder id, or `null` for "all folders" (Req 5.4). */
  folderId: string | null;
  /** Tags that must all be present (conjunctive, Req 5.4). */
  tags: string[];
  /** When true, restrict to favorites (Req 5.4). */
  favoritesOnly: boolean;
  /** Sort field (Req 5.5); defaults to `updatedAt`. */
  sortBy: SortField;
  /** Sort direction (Req 5.5); defaults to `desc`. */
  sortOrder: SortOrder;
}

/** The default filter state: everything, newest-updated first (Req 5.8). */
export const DEFAULT_FILTERS: PromptFilters = {
  keyword: "",
  folderId: null,
  tags: [],
  favoritesOnly: false,
  sortBy: "updatedAt",
  sortOrder: "desc",
};

/**
 * Builds the {@link SearchQuery} sent to `prompt.search` from the view filters
 * (Req 5.3–5.5). Omits empty keyword/tag/folder/favorite fields so the backend
 * applies only the active constraints with conjunctive logic.
 */
export function buildSearchQuery(filters: PromptFilters): SearchQuery {
  const query: SearchQuery = {
    sortBy: filters.sortBy,
    sortOrder: filters.sortOrder,
  };
  const keyword = filters.keyword.trim();
  if (keyword !== "") query.keyword = keyword;
  if (filters.folderId != null) query.folderId = filters.folderId;
  if (filters.tags.length > 0) query.tags = filters.tags;
  if (filters.favoritesOnly) query.isFavorite = true;
  return query;
}

interface PromptStoreState {
  /** Backend command surface; injectable so tests can supply a fake. */
  api: PromptApi;

  folders: Folder[];
  prompts: Prompt[];
  tags: string[];
  filters: PromptFilters;

  /** The id of the prompt open in the editor, or `null` when none is selected. */
  selectedPromptId: string | null;
  /** Version history of the selected prompt (Req 7.1), ascending by version. */
  versions: PromptVersion[];

  loading: boolean;
  error: string | null;

  /** Loads folders, tags, and the filtered prompt list (Req 6.3, 6.8, 8.2). */
  load: () => Promise<void>;
  /** Re-runs the search with the current filters (Req 5.3). */
  refreshPrompts: () => Promise<void>;
  /** Replaces the active filters and re-runs the search. */
  setFilters: (patch: Partial<PromptFilters>) => Promise<void>;
  /** Toggles a tag in the conjunctive tag filter (Req 5.4). */
  toggleTagFilter: (tag: string) => Promise<void>;

  /** Selects a prompt and loads its version history (Req 7.1). */
  selectPrompt: (id: string | null) => Promise<void>;

  /** Creates a prompt, refreshes the list, and selects it (Req 6.1). */
  createPrompt: (
    input: Parameters<PromptApi["createPrompt"]>[0],
  ) => Promise<Prompt | null>;
  /** Applies a partial update and refreshes the list (Req 6.4). */
  savePrompt: (
    id: string,
    patch: Parameters<PromptApi["updatePrompt"]>[1],
  ) => Promise<Prompt | null>;
  /** Deletes a prompt and clears the selection if it was selected (Req 6.5). */
  deletePrompt: (id: string) => Promise<void>;

  /** Creates a folder and refreshes the folder list (Req 8.1). */
  createFolder: (
    input: Parameters<PromptApi["createFolder"]>[0],
  ) => Promise<Folder | null>;
  /** Updates a folder and refreshes the folder list (Req 8.3). */
  updateFolder: (
    id: string,
    patch: Parameters<PromptApi["updateFolder"]>[1],
  ) => Promise<void>;
  /** Deletes a folder (and its subtree) and refreshes (Req 8.4). */
  deleteFolder: (id: string) => Promise<void>;
  /** Persists a new folder order (Req 8.5). */
  reorderFolders: (orderedIds: string[]) => Promise<void>;

  /** Snapshots the selected prompt as a new version (Req 7.2). */
  createVersion: (note?: string) => Promise<void>;
  /** Rolls the selected prompt back to a version (Req 7.3). */
  rollbackVersion: (version: number) => Promise<void>;
  /** Deletes a single version from history (Req 7.4). */
  deleteVersion: (versionId: string) => Promise<void>;
}

export const usePromptStore = create<PromptStoreState>((set, get) => ({
  api: promptApi,

  folders: [],
  prompts: [],
  tags: [],
  filters: { ...DEFAULT_FILTERS },

  selectedPromptId: null,
  versions: [],

  loading: false,
  error: null,

  load: async () => {
    const { api, filters } = get();
    set({ loading: true, error: null });
    try {
      const [folders, tags, prompts] = await Promise.all([
        api.listFolders(),
        api.listTags(),
        api.searchPrompts(buildSearchQuery(filters)),
      ]);
      set({ folders, tags, prompts, loading: false });
    } catch (err) {
      set({ error: errorMessage(err), loading: false });
    }
  },

  refreshPrompts: async () => {
    const { api, filters } = get();
    try {
      const prompts = await api.searchPrompts(buildSearchQuery(filters));
      set({ prompts });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  setFilters: async (patch) => {
    set({ filters: { ...get().filters, ...patch } });
    await get().refreshPrompts();
  },

  toggleTagFilter: async (tag) => {
    const current = get().filters.tags;
    const tags = current.includes(tag)
      ? current.filter((t) => t !== tag)
      : [...current, tag];
    await get().setFilters({ tags });
  },

  selectPrompt: async (id) => {
    set({ selectedPromptId: id, versions: [] });
    if (id == null) return;
    try {
      const versions = await get().api.listVersions(id);
      // Ignore the result if the selection changed while loading.
      if (get().selectedPromptId === id) set({ versions });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  createPrompt: async (input) => {
    const { api } = get();
    set({ error: null });
    try {
      const prompt = await api.createPrompt(input);
      await get().refreshPrompts();
      await get().selectPrompt(prompt.id);
      return prompt;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  savePrompt: async (id, patch) => {
    const { api } = get();
    set({ error: null });
    try {
      const prompt = await api.updatePrompt(id, patch);
      await get().refreshPrompts();
      // Refresh tags too: an edit may have introduced or removed a tag.
      try {
        set({ tags: await api.listTags() });
      } catch {
        // Tag refresh is best-effort; the save itself already succeeded.
      }
      return prompt;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  deletePrompt: async (id) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.deletePrompt(id);
      if (get().selectedPromptId === id) {
        set({ selectedPromptId: null, versions: [] });
      }
      await get().refreshPrompts();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  createFolder: async (input) => {
    const { api } = get();
    set({ error: null });
    try {
      const folder = await api.createFolder(input);
      set({ folders: await api.listFolders() });
      return folder;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  updateFolder: async (id, patch) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.updateFolder(id, patch);
      set({ folders: await api.listFolders() });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  deleteFolder: async (id) => {
    const { api, filters } = get();
    set({ error: null });
    try {
      await api.deleteFolder(id);
      set({ folders: await api.listFolders() });
      // A deleted folder cannot remain the active filter.
      if (filters.folderId === id) {
        await get().setFilters({ folderId: null });
      } else {
        await get().refreshPrompts();
      }
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  reorderFolders: async (orderedIds) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.reorderFolders(orderedIds);
      set({ folders: await api.listFolders() });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  createVersion: async (note) => {
    const { api, selectedPromptId } = get();
    if (selectedPromptId == null) return;
    set({ error: null });
    try {
      await api.createVersion(selectedPromptId, note);
      set({ versions: await api.listVersions(selectedPromptId) });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  rollbackVersion: async (version) => {
    const { api, selectedPromptId } = get();
    if (selectedPromptId == null) return;
    set({ error: null });
    try {
      await api.rollbackVersion(selectedPromptId, version);
      await get().refreshPrompts();
      set({ versions: await api.listVersions(selectedPromptId) });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  deleteVersion: async (versionId) => {
    const { api, selectedPromptId } = get();
    set({ error: null });
    try {
      await api.deleteVersion(versionId);
      if (selectedPromptId != null) {
        set({ versions: await api.listVersions(selectedPromptId) });
      }
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },
}));

/** Selects the currently open prompt from the loaded list, or `null`. */
export function selectSelectedPrompt(state: PromptStoreState): Prompt | null {
  if (state.selectedPromptId == null) return null;
  return state.prompts.find((p) => p.id === state.selectedPromptId) ?? null;
}
