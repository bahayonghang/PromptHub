// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import { DEFAULT_FILTERS, usePromptStore } from "../promptStore";
import type { Folder } from "../types";
import { PromptLibraryNav } from "./PromptLibraryNav";

const initialStore = usePromptStore.getState();

function makeFolder(id: string, name = id): Folder {
  return {
    id,
    name,
    parentId: null,
    sortOrder: 0,
    createdAt: "2024-01-01T00:00:00.000Z",
    updatedAt: null,
  };
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
  usePromptStore.setState({
    folders: [makeFolder("f1", "Shipping")],
    tags: ["writing", "ops"],
    filters: { ...DEFAULT_FILTERS },
    activeView: "all",
    libraryCounts: {
      views: { all: 20, favorites: 4 },
      folders: { f1: 12 },
      tags: { writing: 8, ops: 2 },
    },
    countsLoading: false,
    selectView: vi.fn(async (view) => {
      usePromptStore.setState({ activeView: view });
    }),
    selectFolder: vi.fn(async (folderId) => {
      usePromptStore.setState({
        activeView: null,
        filters: { ...usePromptStore.getState().filters, folderId },
      });
    }),
    toggleTagFilter: vi.fn(async (tag) => {
      usePromptStore.setState({
        activeView: null,
        filters: {
          ...usePromptStore.getState().filters,
          tags: [tag],
        },
      });
    }),
    createFolder: vi.fn(async () => makeFolder("f2")),
    updateFolder: vi.fn(async () => undefined),
    deleteFolder: vi.fn(async () => undefined),
    reorderFolders: vi.fn(async () => undefined),
    renameTag: vi.fn(async () => undefined),
    deleteTag: vi.fn(async () => undefined),
  });
});

afterEach(() => {
  cleanup();
  usePromptStore.setState(initialStore, true);
});

describe("PromptLibraryNav", () => {
  it("marks the active saved view and omits a count on recent", () => {
    render(<PromptLibraryNav />);

    const all = screen.getByText("All prompts").closest("button");
    const favorites = screen.getByText("Favorites").closest("button");
    const recent = screen.getByText("Recent").closest("button");
    expect(all?.getAttribute("aria-current")).toBe("true");
    expect(all?.textContent).toContain("20");
    expect(favorites?.textContent).toContain("4");
    expect(recent?.textContent).toBe("Recent");
  });

  it("selects a folder and a tag through the store actions", () => {
    render(<PromptLibraryNav />);

    fireEvent.click(screen.getByRole("button", { name: /Shipping/ }));
    expect(usePromptStore.getState().selectFolder).toHaveBeenCalledWith("f1");

    fireEvent.click(screen.getByRole("button", { name: /writing/ }));
    expect(usePromptStore.getState().toggleTagFilter).toHaveBeenCalledWith("writing");
  });

  it("keeps tag management reachable under the cloud", () => {
    render(<PromptLibraryNav />);
    expect(screen.getByText("Manage tags")).toBeTruthy();
  });
});
