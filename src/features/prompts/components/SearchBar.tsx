import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import {
  CheckIcon,
  FilterIcon,
  SearchIcon,
  StarIcon,
  XIcon,
} from "lucide-react";
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

interface FilterPanelPlacement {
  left: number;
  width: number;
  maxHeight: number;
  top?: number;
  bottom?: number;
}

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
  const [panelPlacement, setPanelPlacement] =
    useState<FilterPanelPlacement | null>(null);
  const filterPanelId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const panelFocusedRef = useRef(false);

  const activeFilterCount =
    filters.tags.length + (filters.favoritesOnly ? 1 : 0);
  const filterLabel =
    activeFilterCount > 0
      ? `${t("promptsView.filters")}: ${activeFilterCount}`
      : t("promptsView.filters");

  useEffect(() => {
    if (!showFilters) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (
        event.target instanceof Node &&
        !triggerRef.current?.contains(event.target) &&
        !panelRef.current?.contains(event.target)
      ) {
        setShowFilters(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      setShowFilters(false);
      triggerRef.current?.focus();
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [showFilters]);

  useLayoutEffect(() => {
    if (!showFilters) return;

    const updatePlacement = () => {
      const root = rootRef.current;
      const trigger = triggerRef.current;
      if (!root || !trigger) return;

      const rootRect = root.getBoundingClientRect();
      const triggerRect = trigger.getBoundingClientRect();
      const viewportPadding = 16;
      const panelGap = 8;
      const fontScale =
        Number.parseFloat(
          getComputedStyle(document.documentElement).getPropertyValue(
            "--font-scale",
          ),
        ) || 1;
      const scaledWidth = 320 + Math.min(Math.max(fontScale - 1, 0), 1) * 64;
      const width = Math.min(
        scaledWidth,
        window.innerWidth - viewportPadding * 2,
      );
      const left = Math.min(
        Math.max(rootRect.right - width, viewportPadding),
        window.innerWidth - width - viewportPadding,
      );
      const availableAbove = triggerRect.top - viewportPadding - panelGap;
      const availableBelow =
        window.innerHeight - triggerRect.bottom - viewportPadding - panelGap;

      if (availableBelow >= availableAbove) {
        setPanelPlacement({
          left,
          width,
          top: triggerRect.bottom + panelGap,
          maxHeight: Math.max(0, availableBelow),
        });
      } else {
        setPanelPlacement({
          left,
          width,
          bottom: window.innerHeight - triggerRect.top + panelGap,
          maxHeight: Math.max(0, availableAbove),
        });
      }
    };

    updatePlacement();
    window.addEventListener("resize", updatePlacement);
    window.addEventListener("scroll", updatePlacement, true);
    const resizeObserver =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(updatePlacement);
    if (rootRef.current) resizeObserver?.observe(rootRef.current);

    return () => {
      window.removeEventListener("resize", updatePlacement);
      window.removeEventListener("scroll", updatePlacement, true);
      resizeObserver?.disconnect();
    };
  }, [showFilters]);

  useEffect(() => {
    if (!showFilters) {
      panelFocusedRef.current = false;
      return;
    }
    if (!panelPlacement || panelFocusedRef.current) return;

    panelRef.current
      ?.querySelector<HTMLElement>("input, select, button")
      ?.focus();
    panelFocusedRef.current = true;
  }, [panelPlacement, showFilters]);

  return (
    <div ref={rootRef} className="relative flex flex-col gap-2">
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
          ref={triggerRef}
          type="button"
          onClick={() => setShowFilters((v) => !v)}
          aria-label={filterLabel}
          aria-expanded={showFilters}
          aria-controls={filterPanelId}
          title={t("promptsView.filters")}
          className={`relative flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-input transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
            showFilters || activeFilterCount > 0
              ? "bg-primary/15 text-foreground"
              : "text-muted-foreground hover:bg-accent hover:text-foreground"
          }`}
        >
          <FilterIcon className="h-4 w-4" aria-hidden="true" />
          {activeFilterCount > 0 && (
            <span
              aria-hidden="true"
              className="absolute -right-1 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[10px] font-semibold text-primary-foreground"
            >
              {activeFilterCount}
            </span>
          )}
        </button>
      </div>

      {showFilters &&
        panelPlacement &&
        createPortal(
          <div
            ref={panelRef}
            id={filterPanelId}
            role="region"
            aria-label={t("promptsView.filters")}
            style={panelPlacement}
            className="fixed z-40 flex flex-col gap-3 overflow-y-auto rounded-lg border border-border bg-popover p-3 text-popover-foreground shadow-lg"
          >
            <div className="text-sm font-semibold text-foreground">
              {t("promptsView.filters")}
            </div>

            <label className="flex min-h-10 items-center gap-2 rounded-md text-sm text-foreground focus-within:ring-2 focus-within:ring-ring">
              <input
                type="checkbox"
                checked={filters.favoritesOnly}
                onChange={(e) => onChange({ favoritesOnly: e.target.checked })}
                className="h-4 w-4 shrink-0 accent-[hsl(var(--primary))]"
              />
              <StarIcon className="h-4 w-4 shrink-0" aria-hidden="true" />
              <span>{t("promptsView.favoritesOnly")}</span>
            </label>

            <div className="grid grid-cols-1 gap-3">
              <label className="flex min-w-0 flex-col gap-1.5 text-xs font-medium text-muted-foreground">
                <span>{t("promptsView.sortBy")}</span>
                <select
                  value={filters.sortBy}
                  onChange={(e) =>
                    onChange({ sortBy: e.target.value as SortField })
                  }
                  className="min-h-10 w-full min-w-0 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-2 focus:ring-ring"
                >
                  {SORT_FIELDS.map((field) => (
                    <option key={field.value} value={field.value}>
                      {t(field.labelKey)}
                    </option>
                  ))}
                </select>
              </label>

              <label className="flex min-w-0 flex-col gap-1.5 text-xs font-medium text-muted-foreground">
                <span>{t("promptsView.sortDirection")}</span>
                <select
                  value={filters.sortOrder}
                  onChange={(e) =>
                    onChange({ sortOrder: e.target.value as SortOrder })
                  }
                  className="min-h-10 w-full min-w-0 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-2 focus:ring-ring"
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
                        className={`inline-flex min-h-8 max-w-full items-center gap-1 rounded-full border px-2.5 py-1 text-left text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                          active
                            ? "border-primary bg-primary/15 text-foreground"
                            : "border-border text-muted-foreground hover:bg-accent hover:text-foreground"
                        }`}
                      >
                        {active && (
                          <CheckIcon
                            className="h-3.5 w-3.5 shrink-0"
                            aria-hidden="true"
                          />
                        )}
                        <span className="min-w-0 whitespace-normal break-words">
                          {tag}
                        </span>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>

            <div className="flex min-h-8 items-center justify-between gap-2 border-t border-border pt-2">
              <span className="text-xs text-muted-foreground">
                {t("promptsView.filters")}: {activeFilterCount}
              </span>
              {activeFilterCount > 0 && (
                <button
                  type="button"
                  onClick={onClear}
                  className="flex min-h-8 items-center gap-1 rounded-md px-2 text-xs text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <XIcon className="h-3.5 w-3.5" aria-hidden="true" />
                  {t("promptsView.clearFilters")}
                </button>
              )}
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
}
