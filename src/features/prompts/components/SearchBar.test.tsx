// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import { DEFAULT_FILTERS, usePromptStore } from "../promptStore";
import { FilterChips } from "./FilterChips";
import { LibraryToolbar } from "./LibraryToolbar";

const initial = usePromptStore.getState();

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
  usePromptStore.setState({
    filters: { ...DEFAULT_FILTERS },
    prompts: [{ id: "p1" } as never],
    total: 3,
    folders: [
      {
        id: "f1",
        name: "Shipping",
        parentId: null,
        sortOrder: 0,
        createdAt: "2024-01-01T00:00:00.000Z",
        updatedAt: null,
      },
    ],
    viewMode: "list",
    batchMode: false,
    setKeyword: vi.fn(),
    setFilters: vi.fn(async () => undefined),
    setViewMode: vi.fn(),
    setBatchMode: vi.fn(),
    toggleTagFilter: vi.fn(async () => undefined),
    resetLibraryFilters: vi.fn(async () => undefined),
  });
});

afterEach(() => {
  cleanup();
  usePromptStore.setState(initial, true);
});

describe("LibraryToolbar", () => {
  it("exposes sort controls and a result count on the keyword field", () => {
    render(<LibraryToolbar />);
    expect(screen.getByRole("searchbox", { name: "Search prompts..." })).toBeTruthy();
    expect(screen.getByText("1 / 3")).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Sort by" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Sort direction" })).toBeTruthy();
  });

  it("re-queries through setFilters when sort changes", () => {
    render(<LibraryToolbar />);
    fireEvent.change(screen.getByRole("combobox", { name: "Sort by" }), {
      target: { value: "title" },
    });
    expect(usePromptStore.getState().setFilters).toHaveBeenCalledWith({ sortBy: "title" });
  });
});

describe("FilterChips", () => {
  it("renders one chip per active axis and clear-all", () => {
    usePromptStore.setState({
      filters: {
        ...DEFAULT_FILTERS,
        keyword: "alpha",
        folderId: "f1",
        tags: ["writing"],
      },
    });
    render(<FilterChips />);
    expect(screen.getByRole("button", { name: /Remove Keyword alpha/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Remove Folder Shipping/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Remove Tag writing/ })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Clear all" }));
    expect(usePromptStore.getState().resetLibraryFilters).toHaveBeenCalled();
  });
});
