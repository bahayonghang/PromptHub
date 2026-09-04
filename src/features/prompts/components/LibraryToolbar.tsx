import { useTranslation } from "react-i18next";
import { LayoutGridIcon, LayoutListIcon, SearchIcon } from "lucide-react";
import { usePromptStore, type LibraryViewMode } from "../promptStore";
import type { SortField, SortOrder } from "../types";

import { IconButton, Select} from "../../../components/ui";

const SORT_FIELDS: { value: SortField; labelKey: string }[] = [
  { value: "updatedAt", labelKey: "promptsView.sortUpdated" },
  { value: "createdAt", labelKey: "promptsView.sortCreated" },
  { value: "title", labelKey: "promptsView.sortTitle" },
  { value: "usageCount", labelKey: "promptsView.sortUsage" },
];

export function LibraryToolbar() {
  const { t } = useTranslation();
  const filters = usePromptStore((state) => state.filters);
  const prompts = usePromptStore((state) => state.prompts);
  const total = usePromptStore((state) => state.total);
  const viewMode = usePromptStore((state) => state.viewMode);
  const batchMode = usePromptStore((state) => state.batchMode);
  const setKeyword = usePromptStore((state) => state.setKeyword);
  const setFilters = usePromptStore((state) => state.setFilters);
  const setViewMode = usePromptStore((state) => state.setViewMode);
  const setBatchMode = usePromptStore((state) => state.setBatchMode);

  const shown = prompts.length;
  const countLabel = t("promptsView.chrome.resultCount", { shown, total });

  const setMode = (next: LibraryViewMode) => setViewMode(next);

  return (
    <div className="flex h-10 shrink-0 items-center gap-2 border-b border-border px-3">
        <label className="relative min-w-0 flex-1">
          <SearchIcon
            className="pointer-events-none absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <input
            type="search"
            value={filters.keyword}
            onChange={(event) => setKeyword(event.target.value)}
            placeholder={t("promptsView.searchPlaceholder")}
            aria-label={t("promptsView.searchPlaceholder")}
            className="h-control-md w-full rounded-md border border-input bg-background py-1 pl-7 pr-16 text-body text-foreground outline-none"
          />
          <span
            className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 font-mono text-meta tabular-nums text-muted-foreground-subtle"
            aria-live="polite"
          >
            {countLabel}
          </span>
        </label>
        <Select
          value={filters.sortBy}
          aria-label={t("promptsView.sortBy")}
          onChange={(event) => void setFilters({ sortBy: event.target.value as SortField })}
        >
          {SORT_FIELDS.map((field) => (
            <option key={field.value} value={field.value}>
              {t(field.labelKey)}
            </option>
          ))}
        </Select>
        <Select
          value={filters.sortOrder}
          aria-label={t("promptsView.sortDirection")}
          onChange={(event) => void setFilters({ sortOrder: event.target.value as SortOrder })}
        >
          <option value="desc">{t("promptsView.sortDesc")}</option>
          <option value="asc">{t("promptsView.sortAsc")}</option>
        </Select>
        <div className="inline-flex rounded-md border border-input p-0.5" role="group" aria-label={t("promptsView.chrome.viewMode")}>
          <IconButton
            label={t("promptsView.chrome.viewList")}
            icon={<LayoutListIcon className="h-3.5 w-3.5" aria-hidden="true" />}
            onClick={() => setMode("list")}
            aria-pressed={viewMode === "list"}
            className={`flex h-control-sm w-control-sm items-center justify-center rounded-sm ${
              viewMode === "list" ? "bg-state-selected text-foreground" : "text-muted-foreground hover:bg-accent"
            }`}
          />
          <IconButton
            label={t("promptsView.chrome.viewGrid")}
            icon={<LayoutGridIcon className="h-3.5 w-3.5" aria-hidden="true" />}
            onClick={() => setMode("grid")}
            aria-pressed={viewMode === "grid"}
            className={`flex h-control-sm w-control-sm items-center justify-center rounded-sm ${
              viewMode === "grid" ? "bg-state-selected text-foreground" : "text-muted-foreground hover:bg-accent"
            }`}
          />
        </div>
        <button
          type="button"
          aria-pressed={batchMode}
          aria-label={t("promptsView.chrome.batchToggle")}
          onClick={() => setBatchMode(!batchMode)}
          className={`h-control-md rounded-md border px-2 text-label ${
            batchMode
              ? "border-primary bg-state-selected text-foreground"
              : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
          }`}
        >
          {t("promptsView.chrome.batchToggle")}
        </button>
        <span className="sr-only" aria-live="polite">
          {batchMode ? t("promptsView.chrome.batchOn") : t("promptsView.chrome.batchOff")}
        </span>
    </div>
  );
}
