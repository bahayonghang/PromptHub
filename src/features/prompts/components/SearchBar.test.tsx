// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import { DEFAULT_FILTERS, type PromptFilters } from "../promptStore";
import { SearchBar } from "./SearchBar";

function renderSearchBar(
  filters: PromptFilters = { ...DEFAULT_FILTERS },
  tags = ["writing", "release-notes"],
) {
  const onChange = vi.fn();
  const onToggleTag = vi.fn();
  const onClear = vi.fn();

  render(
    <SearchBar
      filters={filters}
      tags={tags}
      onChange={onChange}
      onToggleTag={onToggleTag}
      onClear={onClear}
    />,
  );

  return { onChange, onToggleTag, onClear };
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

afterEach(cleanup);

describe("SearchBar", () => {
  it("exposes a named disclosure surface with stable field labels", () => {
    renderSearchBar();

    const trigger = screen.getByRole("button", { name: "Filters" });
    expect(trigger.getAttribute("aria-expanded")).toBe("false");

    fireEvent.click(trigger);

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    const surface = screen.getByRole("region", { name: "Filters" });
    expect(trigger.getAttribute("aria-controls")).toBe(surface.id);
    expect(document.activeElement).toBe(
      screen.getByRole("checkbox", { name: "Favorites only" }),
    );
    expect(
      screen.getByRole("combobox", { name: "Sort by" }),
    ).toBeTruthy();
    expect(
      screen.getByRole("combobox", { name: "Sort direction" }),
    ).toBeTruthy();
  });

  it("closes from Escape, outside pointer input, and a second trigger click", () => {
    renderSearchBar();

    const trigger = screen.getByRole("button", { name: "Filters" });
    trigger.focus();
    fireEvent.click(trigger);
    fireEvent.keyDown(document, { key: "Escape" });

    expect(screen.queryByRole("region", { name: "Filters" })).toBeNull();
    expect(document.activeElement).toBe(trigger);

    fireEvent.click(trigger);
    fireEvent.pointerDown(
      screen.getByRole("searchbox", { name: "Search prompts..." }),
    );
    expect(screen.queryByRole("region", { name: "Filters" })).toBeNull();

    fireEvent.click(trigger);
    fireEvent.click(trigger);
    expect(screen.queryByRole("region", { name: "Filters" })).toBeNull();
  });

  it("portals and flips the surface when there is more room above", () => {
    renderSearchBar();

    const trigger = screen.getByRole("button", { name: "Filters" });
    const root = trigger.parentElement?.parentElement as HTMLDivElement;
    vi.spyOn(root, "getBoundingClientRect").mockReturnValue({
      right: 1000,
      width: 280,
    } as DOMRect);
    vi.spyOn(trigger, "getBoundingClientRect").mockReturnValue({
      top: 680,
      bottom: 720,
    } as DOMRect);

    fireEvent.click(trigger);

    const surface = screen.getByRole("region", { name: "Filters" });
    expect(surface.parentElement).toBe(document.body);
    expect(surface.style.left).toBe("680px");
    expect(surface.style.bottom).toBe("96px");
    expect(surface.style.width).toBe("320px");
    expect(surface.style.maxHeight).toBe("656px");
  });

  it("preserves the existing filter, sort, tag, and clear callbacks", () => {
    const filters: PromptFilters = {
      ...DEFAULT_FILTERS,
      tags: ["writing"],
      favoritesOnly: true,
    };
    const { onChange, onToggleTag, onClear } = renderSearchBar(filters);

    fireEvent.click(screen.getByRole("button", { name: "Filters: 2" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Favorites only" }));
    fireEvent.change(screen.getByRole("combobox", { name: "Sort by" }), {
      target: { value: "title" },
    });
    fireEvent.change(
      screen.getByRole("combobox", { name: "Sort direction" }),
      { target: { value: "asc" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "release-notes" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));

    expect(onChange).toHaveBeenNthCalledWith(1, { favoritesOnly: false });
    expect(onChange).toHaveBeenNthCalledWith(2, { sortBy: "title" });
    expect(onChange).toHaveBeenNthCalledWith(3, { sortOrder: "asc" });
    expect(onToggleTag).toHaveBeenCalledWith("release-notes");
    expect(onClear).toHaveBeenCalledOnce();
  });

  it("renders long localized labels and tags without hiding controls", async () => {
    await ensureBundle("de");
    await i18n.changeLanguage("de");
    const longTag = "Produktionsfreigabe-mit-außergewöhnlich-langem-Namen";

    renderSearchBar(undefined, [longTag, ...Array.from({ length: 29 }, (_, i) => `Tag ${i + 1}`)]);
    fireEvent.click(screen.getByRole("button", { name: "Filter" }));

    expect(
      screen.getByRole("combobox", { name: "Sortieren nach" }),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: longTag })).toBeTruthy();
    expect(screen.getAllByRole("button")).toHaveLength(31);

    await ensureBundle("zh");
    await i18n.changeLanguage("zh");
    expect(screen.getByRole("region", { name: "筛选" })).toBeTruthy();
    expect(
      screen.getByRole("combobox", { name: "排序方向" }),
    ).toBeTruthy();
  });
});
