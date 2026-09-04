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

  it("drops uppercase group titles and collapses the tag cloud", () => {
    usePromptStore.setState({
      tags: ["t1", "t2", "t3", "t4", "t5", "t6", "t7", "t8", "t9"],
      libraryCounts: {
        views: { all: 20, favorites: 4 },
        folders: { f1: 12 },
        tags: {},
      },
    });
    const { container } = render(<PromptLibraryNav />);
    const headings = [...container.querySelectorAll("h2")];
    expect(headings.length).toBeGreaterThan(0);
    for (const heading of headings) {
      expect(heading.className).not.toContain("uppercase");
    }
    expect(screen.getByRole("button", { name: /t1/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /^t9/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "1 more" }));
    expect(screen.getByRole("button", { name: /^t9/ })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Show fewer" }));
    expect(screen.queryByRole("button", { name: /^t9/ })).toBeNull();
  });
});
