import { useTranslation } from "react-i18next";
import { XIcon } from "lucide-react";
import { usePromptStore } from "../promptStore";

export function FilterChips() {
  const { t } = useTranslation();
  const filters = usePromptStore((state) => state.filters);
  const folders = usePromptStore((state) => state.folders);
  const setFilters = usePromptStore((state) => state.setFilters);
  const toggleTagFilter = usePromptStore((state) => state.toggleTagFilter);
  const resetLibraryFilters = usePromptStore((state) => state.resetLibraryFilters);

  const folder = folders.find((item) => item.id === filters.folderId) ?? null;
  const keyword = filters.keyword.trim();
  const chips: { key: string; label: string; onRemove: () => void }[] = [];

  if (keyword !== "") {
    chips.push({
      key: "keyword",
      label: t("promptsView.chrome.chipKeyword", { value: keyword }),
      onRemove: () => void setFilters({ keyword: "" }),
    });
  }
  if (folder) {
    chips.push({
      key: "folder",
      label: t("promptsView.chrome.chipFolder", { value: folder.name }),
      onRemove: () => void setFilters({ folderId: null }),
    });
  }
  for (const tag of filters.tags) {
    chips.push({
      key: `tag:${tag}`,
      label: t("promptsView.chrome.chipTag", { value: tag }),
      onRemove: () => void toggleTagFilter(tag),
    });
  }
  if (filters.favoritesOnly) {
    chips.push({
      key: "favorites",
      label: t("promptsView.chrome.chipFavorites"),
      onRemove: () => void setFilters({ favoritesOnly: false }),
    });
  }

  if (chips.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center gap-1 border-b border-border px-3 py-2">
      {chips.map((chip) => (
        <button
          key={chip.key}
          type="button"
          onClick={chip.onRemove}
          aria-label={t("promptsView.chrome.removeChip", { label: chip.label })}
          className="inline-flex items-center gap-1 rounded-full border border-border bg-muted px-2 py-0.5 text-label text-foreground hover:bg-accent"
        >
          <span>{chip.label}</span>
          <XIcon className="h-3 w-3" aria-hidden="true" />
        </button>
      ))}
      <button
        type="button"
        onClick={() => void resetLibraryFilters()}
        className="text-label text-muted-foreground hover:text-foreground"
      >
        {t("promptsView.chrome.clearAll")}
      </button>
    </div>
  );
}
