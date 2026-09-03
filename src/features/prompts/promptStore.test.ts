import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BridgeError } from "../../runtime";
import {
  buildSearchQuery,
  COUNT_QUERY_CONCURRENCY,
  DEFAULT_FILTERS,
  PROMPT_PAGE_SIZE,
  resolveLibraryScope,
  selectSelectedPrompt,
  usePromptStore,
  type PromptFilters,
} from "./promptStore";
import type { PromptApi } from "./api";
import type {
  Folder,
  Prompt,
  PromptPage,
  PromptTypeDefinition,
  PromptVersion,
  SearchQuery,
} from "./types";

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
    messages: partial.messages ?? [],
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

function makePromptType(id = "type-1"): PromptTypeDefinition {
  return {
    id,
    name: "Storyboard",
    baseKind: "image",
    createdAt: "2024-01-01T00:00:00.000Z",
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
    messages: partial.messages ?? [],
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
    copyPrompt: vi.fn(async () => ({
      systemPrompt: null,
      userPrompt: "copied",
      messages: [],
      unexpanded: [],
    })),
    listReferences: vi.fn(async () => ({ outgoing: [], incoming: [] })),
    listPromptTypes: vi.fn(async () => []),
    createPromptType: vi.fn(async (input) => ({
      id: "type-1",
      name: input.name,
      baseKind: input.baseKind,
      createdAt: "2024-01-01T00:00:00.000Z",
    })),
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
      typeDefinitionAdditions: 0,
      typeDefinitionConflicts: 0,
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
    promptTypeDefinitions: [],
    filters: { ...DEFAULT_FILTERS },
    activeView: "all",
    libraryCounts: { views: {}, folders: {}, tags: {} },
    countsLoading: false,
    batchMode: false,
    viewMode: "list",
    selectedPromptId: null,
    selectedPrompt: null,
    versions: [],
    detailOpen: false,
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

  it("load() fetches folders, tags, prompt types, and the filtered prompt list", async () => {
    const api = makeApi({
      listFolders: vi.fn(async () => [makeFolder("f1")]),
      listTags: vi.fn(async () => ["t1"]),
      listPromptTypes: vi.fn(async () => [makePromptType()]),
      searchPrompts: vi.fn(async () => makePage([makePrompt({ id: "p1" })])),
    });
    resetStore(api);

    await usePromptStore.getState().load();

    const state = usePromptStore.getState();
    expect(state.folders).toHaveLength(1);
    expect(state.tags).toEqual(["t1"]);
    expect(state.promptTypeDefinitions).toEqual([makePromptType()]);
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

  it("applies the D4b library-scope transition table", async () => {
    const search = vi.fn(async () => makePage([]));
    resetStore(makeApi({ searchPrompts: search }));
    usePromptStore.setState({ folders: [makeFolder("f1")] });

    await usePromptStore.getState().selectView("favorites");
    expect(usePromptStore.getState().activeView).toBe("favorites");
    expect(usePromptStore.getState().filters).toMatchObject({
      folderId: null,
      tags: [],
      favoritesOnly: true,
    });

    await usePromptStore.getState().selectFolder("f1");
    expect(usePromptStore.getState().activeView).toBeNull();
    expect(usePromptStore.getState().filters.folderId).toBe("f1");
    expect(usePromptStore.getState().filters.favoritesOnly).toBe(true);

    await usePromptStore.getState().toggleTagFilter("writing");
    expect(usePromptStore.getState().activeView).toBeNull();
    expect(usePromptStore.getState().filters.folderId).toBe("f1");
    expect(usePromptStore.getState().filters.tags).toEqual(["writing"]);
    expect(buildSearchQuery(usePromptStore.getState().filters)).toMatchObject({
      folderId: "f1",
      tags: ["writing"],
      isFavorite: true,
    });

    await usePromptStore.getState().setFilters({ keyword: "alpha" });
    expect(usePromptStore.getState().activeView).toBeNull();
    expect(usePromptStore.getState().filters.folderId).toBe("f1");
    expect(usePromptStore.getState().filters.keyword).toBe("alpha");

    await usePromptStore.getState().selectView("recent");
    expect(usePromptStore.getState().activeView).toBe("recent");
    expect(usePromptStore.getState().filters.folderId).toBeNull();
    expect(usePromptStore.getState().filters.tags).toEqual([]);
    expect(usePromptStore.getState().filters.sortBy).toBe("updatedAt");
    expect(usePromptStore.getState().filters.sortOrder).toBe("desc");

    await usePromptStore.getState().setFilters({ sortBy: "title" });
    expect(usePromptStore.getState().activeView).toBeNull();

    await usePromptStore.getState().selectView("all");
    expect(usePromptStore.getState().activeView).toBe("all");
    expect(usePromptStore.getState().filters.favoritesOnly).toBe(false);
    expect(usePromptStore.getState().filters.folderId).toBeNull();
    expect(usePromptStore.getState().filters.tags).toEqual([]);

    await usePromptStore.getState().selectFolder("f1");
    await usePromptStore.getState().selectFolder("f1");
    expect(usePromptStore.getState().filters.folderId).toBeNull();
  });

  it("titles the library scope as view, then folder, then a single tag", () => {
    const folder = makeFolder("f1");
    folder.name = "Shipping";
    expect(
      resolveLibraryScope({
        activeView: "favorites",
        filters: { ...DEFAULT_FILTERS, folderId: "f1" },
        folders: [folder],
      }),
    ).toEqual({ kind: "view", view: "favorites" });
    expect(
      resolveLibraryScope({
        activeView: null,
        filters: { ...DEFAULT_FILTERS, folderId: "f1" },
        folders: [folder],
      }),
    ).toEqual({ kind: "folder", folder });
    expect(
      resolveLibraryScope({
        activeView: null,
        filters: { ...DEFAULT_FILTERS, tags: ["alpha"] },
        folders: [],
      }),
    ).toEqual({ kind: "tag", tag: "alpha" });
    expect(
      resolveLibraryScope({
        activeView: null,
        filters: { ...DEFAULT_FILTERS, tags: ["alpha", "beta"] },
        folders: [],
      }),
    ).toEqual({ kind: "all" });
  });

  it("refreshCounts issues one limit-1 query per bucket and skips recent", async () => {
    const queries: SearchQuery[] = [];
    const searchPrompts = vi.fn(async (query: SearchQuery) => {
      queries.push(query);
      if (query.isFavorite) return makePage([], 4);
      if (query.folderId === "f1") return makePage([], 12);
      if (query.tags?.[0] === "a") return makePage([], 3);
      return makePage([], 20);
    });
    resetStore(makeApi({ searchPrompts }));
    usePromptStore.setState({
      folders: [makeFolder("f1")],
      tags: ["a"],
    });

    await usePromptStore.getState().refreshCounts();

    expect(queries.every((query) => query.limit === 1)).toBe(true);
    expect(queries.some((query) => query.sortBy === "updatedAt" && !query.folderId && !query.tags && !query.isFavorite && !query.keyword)).toBe(false);
    expect(queries).toEqual(
      expect.arrayContaining([
        { limit: 1 },
        { isFavorite: true, limit: 1 },
        { folderId: "f1", limit: 1 },
        { tags: ["a"], limit: 1 },
      ]),
    );
    expect(usePromptStore.getState().libraryCounts).toEqual({
      views: { all: 20, favorites: 4 },
      folders: { f1: 12 },
      tags: { a: 3 },
    });
  });

  it("keeps the prior count when a bucket query rejects", async () => {
    const searchPrompts = vi.fn(async (query: SearchQuery) => {
      if (query.isFavorite) throw new Error("unavailable");
      return makePage([], 5);
    });
    resetStore(makeApi({ searchPrompts }));
    usePromptStore.setState({
      libraryCounts: { views: { all: 1, favorites: 9 }, folders: {}, tags: {} },
    });

    await usePromptStore.getState().refreshCounts();

    expect(usePromptStore.getState().libraryCounts.views.favorites).toBe(9);
    expect(usePromptStore.getState().libraryCounts.views.all).toBe(5);
  });

  it("discards an older refreshPrompts result that arrives last", async () => {
    const pages = [
      makePage([makePrompt({ id: "old" })], 1),
      makePage([makePrompt({ id: "new" })], 1),
    ];
    const deferred: Array<{ resolve: (page: PromptPage) => void }> = [];
    const searchPrompts = vi.fn(
      () =>
        new Promise<PromptPage>((resolve) => {
          deferred.push({ resolve });
        }),
    );
    resetStore(makeApi({ searchPrompts }));
    const first = usePromptStore.getState().refreshPrompts();
    const second = usePromptStore.getState().refreshPrompts();
    deferred[1].resolve(pages[1]);
    await second;
    deferred[0].resolve(pages[0]);
    await first;
    expect(usePromptStore.getState().prompts.map((item) => item.id)).toEqual(["new"]);
  });

  it("keeps a search issued during load after load resolves", async () => {
    let resolveLoadPage!: (page: PromptPage) => void;
    let n = 0;
    const searchPrompts = vi.fn(async () => {
      n += 1;
      if (n === 1) {
        return new Promise<PromptPage>((resolve) => {
          resolveLoadPage = resolve;
        });
      }
      return makePage([makePrompt({ id: "typed" })], 1);
    });
    resetStore(
      makeApi({
        searchPrompts,
        listFolders: vi.fn(async () => [makeFolder("f1")]),
        listTags: vi.fn(async () => ["t1"]),
      }),
    );
    const loading = usePromptStore.getState().load();
    await usePromptStore.getState().refreshPrompts();
    resolveLoadPage(makePage([makePrompt({ id: "initial" })], 1));
    await loading;
    expect(usePromptStore.getState().prompts.map((item) => item.id)).toEqual(["typed"]);
    expect(usePromptStore.getState().folders).toHaveLength(1);
    expect(usePromptStore.getState().tags).toEqual(["t1"]);
  });

  it("debounces keyword keystrokes into one search", async () => {
    vi.useFakeTimers();
    const searchPrompts = vi.fn(async () => makePage([]));
    resetStore(makeApi({ searchPrompts }));
    searchPrompts.mockClear();
    usePromptStore.getState().setKeyword("a");
    usePromptStore.getState().setKeyword("ab");
    usePromptStore.getState().setKeyword("abc");
    usePromptStore.getState().setKeyword("abcd");
    expect(searchPrompts).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(200);
    expect(searchPrompts).toHaveBeenCalledTimes(1);
    expect(searchPrompts).toHaveBeenCalledWith(expect.objectContaining({ keyword: "abcd" }));
    vi.useRealTimers();
  });

  it("clears the selection when leaving batch mode", () => {
    resetStore(makeApi());
    usePromptStore.setState({ selectedPromptIds: ["p1", "p2"], batchMode: true });
    usePromptStore.getState().setBatchMode(false);
    expect(usePromptStore.getState().batchMode).toBe(false);
    expect(usePromptStore.getState().selectedPromptIds).toEqual([]);
  });

  it("resetLibraryFilters restores the all view and default filters", async () => {
    resetStore(makeApi());
    usePromptStore.setState({
      activeView: null,
      filters: { ...DEFAULT_FILTERS, keyword: "x", folderId: "f1", tags: ["a"], favoritesOnly: true },
    });
    await usePromptStore.getState().resetLibraryFilters();
    expect(usePromptStore.getState().activeView).toBe("all");
    expect(usePromptStore.getState().filters).toEqual(DEFAULT_FILTERS);
  });

  it("caps count refresh concurrency at 8", async () => {
    const folders = Array.from({ length: 20 }, (_, index) => makeFolder(`f${index}`));
    const tags = Array.from({ length: 40 }, (_, index) => `t${index}`);
    let inflight = 0;
    let peak = 0;
    const searchPrompts = vi.fn(async () => {
      inflight += 1;
      peak = Math.max(peak, inflight);
      await Promise.resolve();
      inflight -= 1;
      return makePage([], 1);
    });
    resetStore(makeApi({ searchPrompts }));
    usePromptStore.setState({ folders, tags });

    const started = Date.now();
    await usePromptStore.getState().refreshCounts();
    const elapsedMs = Date.now() - started;

    expect(peak).toBeLessThanOrEqual(COUNT_QUERY_CONCURRENCY);
    expect(elapsedMs).toBeLessThan(200);
  });

  it("selectPrompt() loads the prompt's version history (Req 7.1)", async () => {
    const versions: PromptVersion[] = [makeVersion()];
    resetStore(makeApi({ listVersions: vi.fn(async () => versions) }));

    await usePromptStore.getState().selectPrompt("p1");

    expect(usePromptStore.getState().selectedPromptId).toBe("p1");
    expect(usePromptStore.getState().versions).toEqual(versions);
  });

  it("requestSelectPrompt() waits for a registered navigation guard", async () => {
    resetStore(makeApi());
    const guard = vi.fn(async () => "cancel" as const);
    usePromptStore.getState().registerNavigationGuard(guard);
    const ok = await usePromptStore.getState().requestSelectPrompt("p1");
    expect(ok).toBe(false);
    expect(usePromptStore.getState().selectedPromptId).toBeNull();
    expect(usePromptStore.getState().detailOpen).toBe(false);
    usePromptStore.getState().registerNavigationGuard(async () => "proceed");
    const next = await usePromptStore.getState().requestSelectPrompt("p1");
    expect(next).toBe(true);
    expect(usePromptStore.getState().selectedPromptId).toBe("p1");
    expect(usePromptStore.getState().detailOpen).toBe(true);
  });

  it("requestSelectPrompt() sets detailOpen and selectPrompt() does not", async () => {
    resetStore(makeApi());
    await usePromptStore.getState().selectPrompt("p1");
    expect(usePromptStore.getState().detailOpen).toBe(false);

    usePromptStore.setState({ detailOpen: true });
    await usePromptStore.getState().selectPrompt("p1");
    expect(usePromptStore.getState().detailOpen).toBe(true);

    const opened = await usePromptStore.getState().requestSelectPrompt("p1");
    expect(opened).toBe(true);
    expect(usePromptStore.getState().detailOpen).toBe(true);

    const closed = await usePromptStore.getState().requestSelectPrompt(null);
    expect(closed).toBe(true);
    expect(usePromptStore.getState().selectedPromptId).toBeNull();
    expect(usePromptStore.getState().detailOpen).toBe(false);
  });

  it("closeDetail() hides the overlay without changing selection", async () => {
    resetStore(makeApi());
    await usePromptStore.getState().selectPrompt("p1");
    usePromptStore.setState({ detailOpen: true });

    usePromptStore.getState().closeDetail();

    expect(usePromptStore.getState().detailOpen).toBe(false);
    expect(usePromptStore.getState().selectedPromptId).toBe("p1");
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
    usePromptStore.setState({ detailOpen: true });

    const result = await usePromptStore
      .getState()
      .createPrompt({ title: "Fresh", userPrompt: "body" });

    expect(result).toEqual(created);
    expect(usePromptStore.getState().selectedPromptId).toBe("new");
    expect(usePromptStore.getState().detailOpen).toBe(false);
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

  it("surfaces UNAUTHORIZED from create, update, and copy of locked private content", async () => {
    const locked = new BridgeError(
      "UNAUTHORIZED",
      "unlock the prompt library to access private content",
    );
    const existing = [makePrompt({ id: "p1", title: "Public" })];
    const searchPrompts = vi.fn(async () => makePage(existing));
    resetStore(
      makeApi({
        searchPrompts,
        createPrompt: vi.fn(async () => {
          throw locked;
        }),
        updatePrompt: vi.fn(async () => {
          throw locked;
        }),
        duplicatePrompt: vi.fn(async () => {
          throw locked;
        }),
      }),
    );
    usePromptStore.setState({ prompts: existing, total: 1 });

    const created = await usePromptStore.getState().createPrompt({
      title: "Private",
      userPrompt: "secret",
      isPrivate: true,
    });
    expect(created).toBeNull();
    expect(usePromptStore.getState().error).toBe(locked.message);
    expect(usePromptStore.getState().prompts).toEqual(existing);
    expect(searchPrompts).not.toHaveBeenCalled();

    const updated = await usePromptStore
      .getState()
      .savePrompt("p1", { isPrivate: true });
    expect(updated).toBeNull();
    expect(usePromptStore.getState().error).toBe(locked.message);
    expect(usePromptStore.getState().prompts).toEqual(existing);
    expect(searchPrompts).not.toHaveBeenCalled();

    const copied = await usePromptStore.getState().duplicatePrompt("p1");
    expect(copied).toBeNull();
    expect(usePromptStore.getState().error).toBe(locked.message);
    expect(usePromptStore.getState().prompts).toEqual(existing);
    expect(usePromptStore.getState().selectedPromptId).toBeNull();
    expect(searchPrompts).not.toHaveBeenCalled();
  });

  it("deletePrompt() clears the selection when the deleted prompt was selected (Req 6.5)", async () => {
    resetStore(makeApi());
    usePromptStore.setState({ selectedPromptId: "p1", detailOpen: true });

    await usePromptStore.getState().deletePrompt("p1");

    expect(usePromptStore.getState().selectedPromptId).toBeNull();
    expect(usePromptStore.getState().versions).toEqual([]);
    expect(usePromptStore.getState().detailOpen).toBe(false);
  });

  it("createPromptType() refreshes definitions and returns the authoritative row", async () => {
    const created = makePromptType();
    const createPromptType = vi.fn(async () => created);
    const listPromptTypes = vi.fn(async () => [created]);
    resetStore(makeApi({ createPromptType, listPromptTypes }));

    const result = await usePromptStore
      .getState()
      .createPromptType({ name: "Storyboard", baseKind: "image" });

    expect(result).toEqual(created);
    expect(createPromptType).toHaveBeenCalledWith({
      name: "Storyboard",
      baseKind: "image",
    });
    expect(usePromptStore.getState().promptTypeDefinitions).toEqual([created]);
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
