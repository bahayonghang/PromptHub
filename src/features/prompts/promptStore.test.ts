import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  buildSearchQuery,
  DEFAULT_FILTERS,
  PROMPT_PAGE_SIZE,
  selectSelectedPrompt,
  usePromptStore,
  type PromptFilters,
} from "./promptStore";
import type { PromptApi } from "./api";
import type { Folder, Prompt, PromptPage, PromptVersion } from "./types";

function makePrompt(partial: Partial<Prompt> & { id: string }): Prompt {
  return {
    title: partial.id,
    promptType: "text",
    userPrompt: "body",
    variables: [],
    tags: [],
    images: [],
    videos: [],
    isFavorite: false,
    isPinned: false,
    isPrivate: false,
    isLocked: false,
    currentVersion: 0,
    usageCount: 0,
    createdAt: "2024-01-01T00:00:00.000Z",
    updatedAt: "2024-01-01T00:00:00.000Z",
    ...partial,
  };
}

function makeFolder(id: string): Folder {
  return {
    id,
    name: id,
    parentId: null,
    sortOrder: 0,
    createdAt: "2024-01-01T00:00:00.000Z",
    updatedAt: null,
  };
}

function makePage(items: Prompt[], total = items.length, offset = 0): PromptPage {
  return {
    items,
    total,
    limit: PROMPT_PAGE_SIZE,
    offset,
    hasMore: offset + items.length < total,
  };
}

function makeVersion(partial: Partial<PromptVersion> = {}): PromptVersion {
  return {
    id: "v1",
    promptId: "p1",
    version: 1,
    title: "p1",
    promptType: "text",
    userPrompt: "body",
    variables: [],
    tags: [],
    images: [],
    videos: [],
    isFavorite: false,
    isPinned: false,
    isPrivate: false,
    sourceAction: "create",
    createdAt: "2024-01-01T00:00:00.000Z",
    ...partial,
  };
}

/** A controllable fake PromptApi. Each method is a vi mock with a default. */
function makeApi(overrides: Partial<PromptApi> = {}): PromptApi {
  return {
    listPrompts: vi.fn(async () => []),
    getPrompt: vi.fn(async () => makePrompt({ id: "p1" })),
    searchPrompts: vi.fn(async () => makePage([])),
    createPrompt: vi.fn(async () => makePrompt({ id: "new" })),
    updatePrompt: vi.fn(async () => makePrompt({ id: "p1" })),
    deletePrompt: vi.fn(async () => undefined),
    duplicatePrompt: vi.fn(async () => makePrompt({ id: "copy" })),
    batchMove: vi.fn(async () => undefined),
    batchTag: vi.fn(async () => undefined),
    batchDelete: vi.fn(async () => undefined),
    copyPrompt: vi.fn(async () => "copied"),
    listTags: vi.fn(async () => []),
    renameTag: vi.fn(async () => undefined),
    deleteTag: vi.fn(async () => undefined),
    exportBundle: vi.fn(async () => ({
      filePath: "bundle.prompthub",
      prompts: 0,
      revisions: 0,
      mediaFiles: 0,
    })),
    previewBundle: vi.fn(async () => ({
      formatVersion: 1,
      prompts: 0,
      revisions: 0,
      folders: 0,
      mediaFiles: 0,
      additions: 0,
      conflicts: 0,
      privatePrompts: 0,
    })),
    importBundle: vi.fn(async () => ({
      added: 0,
      skipped: 0,
      replaced: 0,
      backupId: "backup-1",
    })),
    listFolders: vi.fn(async () => []),
    createFolder: vi.fn(async () => makeFolder("f1")),
    updateFolder: vi.fn(async () => makeFolder("f1")),
    deleteFolder: vi.fn(async () => undefined),
    reorderFolders: vi.fn(async () => undefined),
    listVersions: vi.fn(async () => [] as PromptVersion[]),
    createVersion: vi.fn(async () => makeVersion()),
    rollbackVersion: vi.fn(async () => makePrompt({ id: "p1" })),
    ...overrides,
  };
}

function resetStore(api: PromptApi) {
  usePromptStore.setState({
    api,
    folders: [],
    prompts: [],
    total: 0,
    offset: 0,
    tags: [],
    filters: { ...DEFAULT_FILTERS },
    selectedPromptId: null,
    selectedPrompt: null,
    versions: [],
    loading: false,
    error: null,
  });
}

afterEach(() => vi.restoreAllMocks());

describe("buildSearchQuery (Req 5.3-5.5)", () => {
  it("always carries the sort field and order", () => {
    const query = buildSearchQuery(DEFAULT_FILTERS);
    expect(query.sortBy).toBe("updatedAt");
    expect(query.sortOrder).toBe("desc");
  });

  it("omits empty keyword/tags/folder/favorite filters", () => {
    const query = buildSearchQuery(DEFAULT_FILTERS);
    expect(query.keyword).toBeUndefined();
    expect(query.tags).toBeUndefined();
    expect(query.folderId).toBeUndefined();
    expect(query.isFavorite).toBeUndefined();
  });

  it("includes active filters with conjunctive intent (Req 5.4)", () => {
    const filters: PromptFilters = {
      keyword: "  hello  ",
      folderId: "f1",
      tags: ["x", "y"],
      favoritesOnly: true,
      sortBy: "title",
      sortOrder: "asc",
    };
    const query = buildSearchQuery(filters);
    expect(query.keyword).toBe("hello");
    expect(query.folderId).toBe("f1");
    expect(query.tags).toEqual(["x", "y"]);
    expect(query.isFavorite).toBe(true);
    expect(query.sortBy).toBe("title");
    expect(query.sortOrder).toBe("asc");
  });
});

describe("prompt store (Req 3.1, 5, 6, 7, 8)", () => {
  beforeEach(() => resetStore(makeApi()));

  it("load() fetches folders, tags, and the filtered prompt list", async () => {
    const api = makeApi({
      listFolders: vi.fn(async () => [makeFolder("f1")]),
      listTags: vi.fn(async () => ["t1"]),
      searchPrompts: vi.fn(async () => makePage([makePrompt({ id: "p1" })])),
    });
    resetStore(api);

    await usePromptStore.getState().load();

    const state = usePromptStore.getState();
    expect(state.folders).toHaveLength(1);
    expect(state.tags).toEqual(["t1"]);
    expect(state.prompts.map((p) => p.id)).toEqual(["p1"]);
    expect(api.searchPrompts).toHaveBeenCalledWith(
      {
        ...buildSearchQuery(DEFAULT_FILTERS),
        limit: PROMPT_PAGE_SIZE,
        offset: 0,
      },
    );
  });

  it("setFilters() updates filters and re-runs the search (Req 5.4)", async () => {
    const search = vi.fn(async () => makePage([]));
    resetStore(makeApi({ searchPrompts: search }));

    await usePromptStore.getState().setFilters({ keyword: "hi" });

    expect(usePromptStore.getState().filters.keyword).toBe("hi");
    expect(search).toHaveBeenLastCalledWith(
      expect.objectContaining({ keyword: "hi" }),
    );
  });

  it("toggleTagFilter() adds then removes a tag (Req 5.4)", async () => {
    resetStore(makeApi());
    await usePromptStore.getState().toggleTagFilter("a");
    expect(usePromptStore.getState().filters.tags).toEqual(["a"]);
    await usePromptStore.getState().toggleTagFilter("a");
    expect(usePromptStore.getState().filters.tags).toEqual([]);
  });

  it("selectPrompt() loads the prompt's version history (Req 7.1)", async () => {
    const versions: PromptVersion[] = [makeVersion()];
    resetStore(makeApi({ listVersions: vi.fn(async () => versions) }));

    await usePromptStore.getState().selectPrompt("p1");

    expect(usePromptStore.getState().selectedPromptId).toBe("p1");
    expect(usePromptStore.getState().versions).toEqual(versions);
  });

  it("createPrompt() refreshes the list and selects the new prompt (Req 6.1)", async () => {
    const created = makePrompt({ id: "new", title: "Fresh" });
    resetStore(
      makeApi({
        createPrompt: vi.fn(async () => created),
        searchPrompts: vi.fn(async () => makePage([created])),
        getPrompt: vi.fn(async () => created),
      }),
    );

    const result = await usePromptStore
      .getState()
      .createPrompt({ title: "Fresh", userPrompt: "body" });

    expect(result).toEqual(created);
    expect(usePromptStore.getState().selectedPromptId).toBe("new");
  });

  it("savePrompt() surfaces a BridgeError message on failure (Req 3.5)", async () => {
    resetStore(
      makeApi({
        updatePrompt: vi.fn(async () => {
          throw { code: "NOT_FOUND", message: "Prompt p9 not found" };
        }),
      }),
    );

    const result = await usePromptStore
      .getState()
      .savePrompt("p9", { title: "x" });

    expect(result).toBeNull();
    expect(usePromptStore.getState().error).toBe("Prompt p9 not found");
  });

  it("deletePrompt() clears the selection when the deleted prompt was selected (Req 6.5)", async () => {
    resetStore(makeApi());
    usePromptStore.setState({ selectedPromptId: "p1" });

    await usePromptStore.getState().deletePrompt("p1");

    expect(usePromptStore.getState().selectedPromptId).toBeNull();
    expect(usePromptStore.getState().versions).toEqual([]);
  });

  it("deleteFolder() resets the folder filter when it pointed at the deleted folder (Req 8.4)", async () => {
    const search = vi.fn(async () => makePage([]));
    resetStore(makeApi({ searchPrompts: search }));
    usePromptStore.setState({ filters: { ...DEFAULT_FILTERS, folderId: "f1" } });

    await usePromptStore.getState().deleteFolder("f1");

    expect(usePromptStore.getState().filters.folderId).toBeNull();
  });

  it("reorderFolders() persists the order then reloads folders (Req 8.5)", async () => {
    const reorder = vi.fn(async () => undefined);
    const listFolders = vi.fn(async () => [makeFolder("a"), makeFolder("b")]);
    resetStore(makeApi({ reorderFolders: reorder, listFolders }));

    await usePromptStore.getState().reorderFolders(["b", "a"]);

    expect(reorder).toHaveBeenCalledWith(["b", "a"]);
    expect(listFolders).toHaveBeenCalled();
  });

  it("rollbackVersion() reloads prompts and history (Req 7.3)", async () => {
    const rollback = vi.fn(async () => makePrompt({ id: "p1" }));
    const listVersions = vi.fn(async () => [] as PromptVersion[]);
    resetStore(makeApi({ rollbackVersion: rollback, listVersions }));
    usePromptStore.setState({ selectedPromptId: "p1" });

    await usePromptStore.getState().rollbackVersion(2);

    expect(rollback).toHaveBeenCalledWith("p1", 2);
  });

  it("selectSelectedPrompt() resolves the open prompt from the list", () => {
    resetStore(makeApi());
    const p1 = makePrompt({ id: "p1" });
    usePromptStore.setState({
      prompts: [p1],
      selectedPromptId: "p1",
      selectedPrompt: p1,
    });
    expect(selectSelectedPrompt(usePromptStore.getState())).toEqual(p1);
    usePromptStore.setState({ selectedPromptId: "missing" });
    expect(selectSelectedPrompt(usePromptStore.getState())).toBeNull();
  });
});
