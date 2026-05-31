import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FilterIcon, SearchIcon, StarIcon, XIcon } from "lucide-react";
import type { PromptFilters } from "../promptStore";
import type { SortField, SortOrder } from "../types";

interface SearchBarProps {
  filters: PromptFilters;
  /** All known tags for the tag filter (Req 6.8). */
  tags: string[];
  onChange: (patch: Partial<PromptFilters>) => void;
  onToggleTag: (tag: string) => void;
  onClear: () => void;
}

const SORT_FIELDS: { value: SortField; labelKey: string }[] = [
  { value: "updatedAt", labelKey: "promptsView.sortUpdated" },
  { value: "createdAt", labelKey: "promptsView.sortCreated" },
  { value: "title", labelKey: "promptsView.sortTitle" },
  { value: "usageCount", labelKey: "promptsView.sortUsage" },
];

/**
 * The prompt search bar (Req 5.3–5.5): a keyword field plus a collapsible filter
 * panel with a favorites toggle, conjunctive tag filter (Req 5.4), and sort
 * controls (Req 5.5). State is lifted to the store via `onChange`/`onToggleTag`.
 */
export function SearchBar({
  filters,
  tags,
  onChange,
  onToggleTag,
  onClear,
}: SearchBarProps) {
  const { t } = useTranslation();
  const [showFilters, setShowFilters] = useState(false);

  const activeFilterCount =
    filters.tags.length + (filters.favoritesOnly ? 1 : 0);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <SearchIcon
            className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <input
            type="search"
            value={filters.keyword}
            placeholder={t("promptsView.searchPlaceholder")}
            aria-label={t("promptsView.searchPlaceholder")}
            onChange={(e) => onChange({ keyword: e.target.value })}
            className="w-full rounded-lg border border-input bg-background py-2 pl-9 pr-3 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
        <button
          type="button"
          onClick={() => setShowFilters((v) => !v)}
          aria-pressed={showFilters}
          title={t("promptsView.filters")}
          className={`relative flex h-9 w-9 items-center justify-center rounded-lg border border-input transition-colors ${
            showFilters || activeFilterCount > 0
              ? "bg-primary/15 text-foreground"
              : "text-muted-foreground hover:bg-accent hover:text-foreground"
          }`}
        >
          <FilterIcon className="h-4 w-4" aria-hidden="true" />
          {activeFilterCount > 0 && (
            <span className="absolute -right-1 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[10px] font-semibold text-primary-foreground">
              {activeFilterCount}
            </span>
          )}
        </button>
      </div>

      {showFilters && (
        <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-3">
          <div className="flex flex-wrap items-center gap-3">
            <label className="flex items-center gap-2 text-sm text-foreground">
              <input
                type="checkbox"
                checked={filters.favoritesOnly}
                onChange={(e) => onChange({ favoritesOnly: e.target.checked })}
                className="h-4 w-4 accent-[hsl(var(--primary))]"
              />
              <StarIcon className="h-4 w-4" aria-hidden="true" />
              {t("promptsView.favoritesOnly")}
            </label>

            <label className="ml-auto flex items-center gap-2 text-sm text-muted-foreground">
              {t("promptsView.sortBy")}
              <select
                value={filters.sortBy}
                onChange={(e) =>
                  onChange({ sortBy: e.target.value as SortField })
                }
                className="rounded-md border border-input bg-background px-2 py-1 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring"
              >
                {SORT_FIELDS.map((field) => (
                  <option key={field.value} value={field.value}>
                    {t(field.labelKey)}
                  </option>
                ))}
              </select>
              <select
                value={filters.sortOrder}
                onChange={(e) =>
                  onChange({ sortOrder: e.target.value as SortOrder })
                }
                className="rounded-md border border-input bg-background px-2 py-1 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring"
              >
                <option value="desc">{t("promptsView.sortDesc")}</option>
                <option value="asc">{t("promptsView.sortAsc")}</option>
              </select>
            </label>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="text-xs font-medium text-muted-foreground">
              {t("promptsView.filterByTags")}
            </span>
            {tags.length === 0 ? (
              <span className="text-xs text-muted-foreground">
                {t("promptsView.noTags")}
              </span>
            ) : (
              <div className="flex flex-wrap gap-1.5">
                {tags.map((tag) => {
                  const active = filters.tags.includes(tag);
                  return (
                    <button
                      key={tag}
                      type="button"
                      onClick={() => onToggleTag(tag)}
                      aria-pressed={active}
                      className={`rounded-full border px-2.5 py-0.5 text-xs transition-colors ${
                        active
                          ? "border-primary bg-primary/15 text-foreground"
                          : "border-border text-muted-foreground hover:bg-accent hover:text-foreground"
                      }`}
                    >
                      {tag}
                    </button>
                  );
                })}
              </div>
            )}
          </div>

          {activeFilterCount > 0 && (
            <button
              type="button"
              onClick={onClear}
              className="flex items-center gap-1 self-start text-xs text-muted-foreground hover:text-foreground"
            >
              <XIcon className="h-3.5 w-3.5" aria-hidden="true" />
              {t("promptsView.clearFilters")}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
