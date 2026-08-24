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
  BundlePreview,
  Folder,
  ImportConflictPolicy,
  PortableExportResult,
  PortableImportResult,
  Prompt,
  PromptTypeDefinition,
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

/** Named library scope presets rendered in the sidebar (not AppView). */
export type SavedView = "all" | "favorites" | "recent";

export interface LibraryCounts {
  views: Partial<Record<SavedView, number>>;
  folders: Record<string, number>;
  tags: Record<string, number>;
}

export const DEFAULT_LIBRARY_COUNTS: LibraryCounts = {
  views: {},
  folders: {},
  tags: {},
};

export const COUNT_QUERY_CONCURRENCY = 8;

export const PROMPT_PAGE_SIZE = 50;

export function resolveLibraryScope(state: {
  activeView: SavedView | null;
  filters: PromptFilters;
  folders: Folder[];
}):
  | { kind: "view"; view: SavedView }
  | { kind: "folder"; folder: Folder }
  | { kind: "tag"; tag: string }
  | { kind: "all" } {
  if (state.activeView != null) return { kind: "view", view: state.activeView };
  if (state.filters.folderId != null) {
    const folder = state.folders.find((item) => item.id === state.filters.folderId);
    if (folder) return { kind: "folder", folder };
  }
  if (state.filters.tags.length === 1) {
    return { kind: "tag", tag: state.filters.tags[0] };
  }
  return { kind: "all" };
}

async function runPool<T>(
  items: readonly T[],
  limit: number,
  worker: (item: T) => Promise<void>,
): Promise<void> {
  if (items.length === 0) return;
  let index = 0;
  const runners = Array.from(
    { length: Math.min(limit, items.length) },
    async () => {
      while (index < items.length) {
        const current = items[index];
        index += 1;
        await worker(current);
      }
    },
  );
  await Promise.all(runners);
}

let countsGeneration = 0;

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
  total: number;
  offset: number;
  tags: string[];
  promptTypeDefinitions: PromptTypeDefinition[];
  filters: PromptFilters;
  /** Presentation preset over `filters`. Null when a folder or tag is the scope. */
  activeView: SavedView | null;
  libraryCounts: LibraryCounts;
  countsLoading: boolean;

  /** The id of the prompt open in the editor, or `null` when none is selected. */
  selectedPromptId: string | null;
  selectedPrompt: Prompt | null;
  selectedPromptIds: string[];
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
  /** Applies a saved-view preset and clears folder/tag axes. */
  selectView: (view: SavedView) => Promise<void>;
  /** Sets or toggles the folder filter and clears the saved-view row. */
  selectFolder: (folderId: string | null) => Promise<void>;
  /** Toggles a tag in the conjunctive tag filter (Req 5.4). */
  toggleTagFilter: (tag: string) => Promise<void>;
  /** Refreshes per-bucket totals; never derived from the loaded page. */
  refreshCounts: () => Promise<void>;
  loadPreviousPage: () => Promise<void>;
  loadNextPage: () => Promise<void>;

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
  duplicatePrompt: (id: string) => Promise<Prompt | null>;
  togglePromptSelection: (id: string) => void;
  selectPage: () => void;
  clearPromptSelection: () => void;
  batchMove: (folderId: string | null) => Promise<void>;
  batchTag: (tags: string[]) => Promise<void>;
  batchDelete: () => Promise<void>;
  renameTag: (old: string, next: string) => Promise<void>;
  deleteTag: (tag: string) => Promise<void>;
  exportBundle: () => Promise<PortableExportResult | null>;
  previewBundle: (filePath: string) => Promise<BundlePreview | null>;
  importBundle: (
    filePath: string,
    policy: ImportConflictPolicy,
  ) => Promise<PortableImportResult | null>;

  createPromptType: (
    input: Parameters<PromptApi["createPromptType"]>[0],
  ) => Promise<PromptTypeDefinition | null>;

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
}

export const usePromptStore = create<PromptStoreState>((set, get) => ({
  api: promptApi,

  folders: [],
  prompts: [],
  total: 0,
  offset: 0,
  tags: [],
  promptTypeDefinitions: [],
  filters: { ...DEFAULT_FILTERS },
  activeView: "all",
  libraryCounts: { views: {}, folders: {}, tags: {} },
  countsLoading: false,

  selectedPromptId: null,
  selectedPrompt: null,
  selectedPromptIds: [],
  versions: [],

  loading: false,
  error: null,

  load: async () => {
    const { api, filters, offset } = get();
    set({ loading: true, error: null });
    try {
      const [folders, tags, promptTypeDefinitions, page] = await Promise.all([
        api.listFolders(),
        api.listTags(),
        api.listPromptTypes(),
        api.searchPrompts({
          ...buildSearchQuery(filters),
          limit: PROMPT_PAGE_SIZE,
          offset,
        }),
      ]);
      set({
        folders,
        tags,
        promptTypeDefinitions,
        prompts: page.items,
        total: page.total,
        offset: page.offset,
        loading: false,
      });
      await get().refreshCounts();
    } catch (err) {
      set({ error: errorMessage(err), loading: false });
    }
  },

  refreshPrompts: async () => {
    const { api, filters, offset } = get();
    try {
      let page = await api.searchPrompts({
        ...buildSearchQuery(filters),
        limit: PROMPT_PAGE_SIZE,
        offset,
      });
      if (page.items.length === 0 && page.total > 0 && page.offset > 0) {
        const lastOffset =
          Math.floor((page.total - 1) / PROMPT_PAGE_SIZE) * PROMPT_PAGE_SIZE;
        page = await api.searchPrompts({
          ...buildSearchQuery(filters),
          limit: PROMPT_PAGE_SIZE,
          offset: lastOffset,
        });
      }
      set({ prompts: page.items, total: page.total, offset: page.offset });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  setFilters: async (patch) => {
    const current = get();
    let activeView = current.activeView;
    if (
      activeView === "recent" &&
      (patch.sortBy !== undefined || patch.sortOrder !== undefined)
    ) {
      const sortBy = patch.sortBy ?? current.filters.sortBy;
      const sortOrder = patch.sortOrder ?? current.filters.sortOrder;
      if (sortBy !== "updatedAt" || sortOrder !== "desc") {
        activeView = null;
      }
    }
    set({
      filters: { ...current.filters, ...patch },
      offset: 0,
      activeView,
    });
    await get().refreshPrompts();
  },

  selectView: async (view) => {
    const preset: Partial<PromptFilters> = {
      folderId: null,
      tags: [],
      favoritesOnly: view === "favorites",
    };
    if (view === "recent") {
      preset.sortBy = "updatedAt";
      preset.sortOrder = "desc";
    }
    set({ activeView: view });
    await get().setFilters(preset);
  },

  selectFolder: async (folderId) => {
    const current = get().filters.folderId;
    const next = folderId != null && folderId === current ? null : folderId;
    set({ activeView: null });
    await get().setFilters({ folderId: next });
  },

  toggleTagFilter: async (tag) => {
    const current = get().filters.tags;
    const tags = current.includes(tag)
      ? current.filter((t) => t !== tag)
      : [...current, tag];
    set({ activeView: null });
    await get().setFilters({ tags });
  },

  refreshCounts: async () => {
    const generation = ++countsGeneration;
    const { api, folders, tags, libraryCounts } = get();
    set({ countsLoading: true });
    const next: LibraryCounts = {
      views: { ...libraryCounts.views },
      folders: { ...libraryCounts.folders },
      tags: { ...libraryCounts.tags },
    };
    type Bucket =
      | { kind: "view"; view: Exclude<SavedView, "recent">; query: SearchQuery }
      | { kind: "folder"; id: string; query: SearchQuery }
      | { kind: "tag"; tag: string; query: SearchQuery };
    const buckets: Bucket[] = [
      { kind: "view", view: "all", query: {} },
      { kind: "view", view: "favorites", query: { isFavorite: true } },
      ...folders.map((folder) => ({
        kind: "folder" as const,
        id: folder.id,
        query: { folderId: folder.id },
      })),
      ...tags.map((tag) => ({
        kind: "tag" as const,
        tag,
        query: { tags: [tag] },
      })),
    ];
    await runPool(buckets, COUNT_QUERY_CONCURRENCY, async (bucket) => {
      try {
        const page = await api.searchPrompts({ ...bucket.query, limit: 1 });
        if (bucket.kind === "view") next.views[bucket.view] = page.total;
        else if (bucket.kind === "folder") next.folders[bucket.id] = page.total;
        else next.tags[bucket.tag] = page.total;
      } catch {
        // Keep the previous count for this bucket.
      }
    });
    if (generation !== countsGeneration) return;
    set({ libraryCounts: next, countsLoading: false });
  },

  loadPreviousPage: async () => {
    const offset = Math.max(0, get().offset - PROMPT_PAGE_SIZE);
    if (offset === get().offset) return;
    set({ offset });
    await get().refreshPrompts();
  },

  loadNextPage: async () => {
    const { offset, total } = get();
    const nextOffset = offset + PROMPT_PAGE_SIZE;
    if (nextOffset >= total) return;
    set({ offset: nextOffset });
    await get().refreshPrompts();
  },

  selectPrompt: async (id) => {
    set({ selectedPromptId: id, selectedPrompt: null, versions: [] });
    if (id == null) return;
    try {
      const [selectedPrompt, versions] = await Promise.all([
        get().api.getPrompt(id),
        get().api.listVersions(id),
      ]);
      // Ignore the result if the selection changed while loading.
      if (get().selectedPromptId === id) set({ selectedPrompt, versions });
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
      await get().refreshCounts();
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
      if (get().selectedPromptId === id) set({ selectedPrompt: prompt });
      await get().refreshPrompts();
      // Refresh tags too: an edit may have introduced or removed a tag.
      try {
        set({ tags: await api.listTags() });
      } catch {
        // Tag refresh is best-effort; the save itself already succeeded.
      }
      await get().refreshCounts();
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
        set({ selectedPromptId: null, selectedPrompt: null, versions: [] });
      }
      set({
        selectedPromptIds: get().selectedPromptIds.filter(
          (selectedId) => selectedId !== id,
        ),
      });
      await get().refreshPrompts();
      await get().refreshCounts();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  duplicatePrompt: async (id) => {
    set({ error: null });
    try {
      const prompt = await get().api.duplicatePrompt(id);
      set({ offset: 0 });
      await get().refreshPrompts();
      await get().refreshCounts();
      await get().selectPrompt(prompt.id);
      return prompt;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  togglePromptSelection: (id) => {
    const selected = get().selectedPromptIds;
    set({
      selectedPromptIds: selected.includes(id)
        ? selected.filter((selectedId) => selectedId !== id)
        : [...selected, id],
    });
  },

  selectPage: () => {
    set({ selectedPromptIds: get().prompts.map((prompt) => prompt.id) });
  },

  clearPromptSelection: () => set({ selectedPromptIds: [] }),

  batchMove: async (folderId) => {
    const ids = get().selectedPromptIds;
    if (ids.length === 0) return;
    set({ error: null });
    try {
      await get().api.batchMove(ids, folderId);
      await get().refreshPrompts();
      await get().refreshCounts();
      set({ selectedPromptIds: [] });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  batchTag: async (tags) => {
    const ids = get().selectedPromptIds;
    if (ids.length === 0) return;
    set({ error: null });
    try {
      await get().api.batchTag(ids, tags);
      await get().refreshPrompts();
      set({ tags: await get().api.listTags(), selectedPromptIds: [] });
      await get().refreshCounts();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  batchDelete: async () => {
    const ids = get().selectedPromptIds;
    if (ids.length === 0) return;
    set({ error: null });
    try {
      await get().api.batchDelete(ids);
      const selectedPromptId = get().selectedPromptId;
      if (selectedPromptId != null && ids.includes(selectedPromptId)) {
        set({ selectedPromptId: null, selectedPrompt: null, versions: [] });
      }
      set({ selectedPromptIds: [] });
      await get().refreshPrompts();
      await get().refreshCounts();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  renameTag: async (old, next) => {
    set({ error: null });
    try {
      await get().api.renameTag(old, next);
      set({ tags: await get().api.listTags() });
      await get().refreshPrompts();
      await get().refreshCounts();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  deleteTag: async (tag) => {
    set({ error: null });
    try {
      await get().api.deleteTag(tag);
      set({ tags: await get().api.listTags() });
      await get().setFilters({
        tags: get().filters.tags.filter((selected) => selected !== tag),
      });
      await get().refreshCounts();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  exportBundle: async () => {
    set({ error: null });
    try {
      return await get().api.exportBundle();
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  previewBundle: async (filePath) => {
    set({ error: null });
    try {
      return await get().api.previewBundle(filePath);
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  importBundle: async (filePath, policy) => {
    set({ error: null });
    try {
      const result = await get().api.importBundle(filePath, policy);
      set({ offset: 0, selectedPromptIds: [] });
      await get().load();
      return result;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  createPromptType: async (input) => {
    const { api } = get();
    set({ error: null });
    try {
      const definition = await api.createPromptType(input);
      set({ promptTypeDefinitions: await api.listPromptTypes() });
      return definition;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  createFolder: async (input) => {
    const { api } = get();
    set({ error: null });
    try {
      const folder = await api.createFolder(input);
      set({ folders: await api.listFolders() });
      await get().refreshCounts();
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
      await get().refreshCounts();
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
      const selectedPrompt = await api.rollbackVersion(selectedPromptId, version);
      set({ selectedPrompt });
      await get().refreshPrompts();
      set({ versions: await api.listVersions(selectedPromptId) });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

}));

/** Selects the currently open prompt from the loaded list, or `null`. */
export function selectSelectedPrompt(state: PromptStoreState): Prompt | null {
  if (state.selectedPromptId == null) return null;
  return state.selectedPrompt?.id === state.selectedPromptId
    ? state.selectedPrompt
    : null;
}
