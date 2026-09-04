import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { ImageIcon, LockIcon, PinIcon, StarIcon, VideoIcon } from "lucide-react";
import type { LibraryItem } from "../libraryItem";
import { CopyPromptButton } from "./CopyPromptButton";

interface PromptListProps {
  items: LibraryItem[];
  selectedPromptId: string | null;
  selectedPromptIds: string[];
  batchMode?: boolean;
  onSelect: (id: string) => void;
  onToggleSelection: (id: string) => void;
  onToggleFavorite: (id: string, next: boolean) => void;
  writeText?: (text: string) => Promise<void>;
  copyPrompt?: (id: string, values: Record<string, string>) => Promise<import("../types").PromptCopyResult>;
}

function TypeBadge({ kind }: { kind: LibraryItem["typeKind"] }) {
  if (kind === "image") return <ImageIcon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />;
  if (kind === "video") return <VideoIcon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />;
  return null;
}

function activateSelect(
  event: KeyboardEvent<HTMLDivElement>,
  id: string,
  onSelect: (id: string) => void,
) {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    onSelect(id);
  }
}

/**
 * Dense library table. Column labels live in a real thead so cells are
 * associated with headers. Interactive controls are siblings of the title
 * activator, not nested inside a row-level button.
 */
export function PromptList({
  items,
  selectedPromptId,
  selectedPromptIds,
  batchMode = false,
  onSelect,
  onToggleSelection,
  onToggleFavorite,
  writeText,
  copyPrompt,
}: PromptListProps) {
  const { t } = useTranslation();

  return (
    <div className="overflow-x-hidden p-2">
      <table className="w-full table-fixed border-separate border-spacing-y-1 text-left">
        <thead>
          <tr className="text-meta font-medium text-muted-foreground-subtle">
            {batchMode && <th className="w-8 px-1">{t("promptsView.items.columns.select")}</th>}
            <th className="w-[28%] min-w-0 px-1">{t("promptsView.items.columns.title")}</th>
            <th className="w-[24%] min-w-0 px-1">{t("promptsView.items.columns.description")}</th>
            <th className="w-[12%] min-w-0 px-1">{t("promptsView.items.columns.tags")}</th>
            <th className="w-[10%] min-w-0 px-1">{t("promptsView.items.columns.type")}</th>
            <th className="w-[8%] min-w-0 px-1">{t("promptsView.items.columns.usage")}</th>
            <th className="w-[7%] min-w-0 px-1">{t("promptsView.items.columns.version")}</th>
            <th className="w-[9%] min-w-0 px-1">{t("promptsView.items.columns.updated")}</th>
            <th className="w-10 px-1">
              <span className="sr-only">{t("promptsView.favorite")}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => {
            const selected = item.id === selectedPromptId;
            const checked = selectedPromptIds.includes(item.id);
            return (
              <tr
                key={item.id}
                className={`text-body ${selected ? "bg-state-selected" : "hover:bg-accent/60"}`}
              >
                {batchMode && (
                  <td className="min-w-0 px-1 align-middle">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={() => onToggleSelection(item.id)}
                      aria-label={t("promptsView.batch.selectPrompt", { title: item.title })}
                      className="h-4 w-4 rounded-sm border-input text-primary"
                    />
                  </td>
                )}
                <td className="min-w-0 px-1 align-middle">
                  <div className="flex items-center gap-2">
                    <CopyPromptButton
                      source={item.source}
                      promptId={item.id}
                      copyPrompt={copyPrompt}
                      name={item.title}
                      locked={item.isLocked}
                      writeText={writeText}
                    />
                    <div
                      role="button"
                      tabIndex={0}
                      aria-label={item.title}
                      aria-current={selected ? "true" : undefined}
                      onClick={() => onSelect(item.id)}
                      onKeyDown={(event) => activateSelect(event, item.id, onSelect)}
                      className="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5 truncate text-left font-medium text-foreground"
                    >
                      {item.isPinned && (
                        <span className="inline-flex items-center gap-0.5 rounded-sm bg-muted px-1 text-micro text-muted-foreground">
                          <PinIcon className="h-3 w-3" aria-hidden="true" />
                          {t("promptsView.items.pinned")}
                        </span>
                      )}
                      {item.isPrivate && (
                        <LockIcon className="h-3.5 w-3.5 shrink-0" aria-label={t("promptsView.privatePrompt")} />
                      )}
                      <span className="min-w-0 truncate">{item.title}</span>
                    </div>
                  </div>
                </td>
                <td className="min-w-0 truncate px-1 align-middle text-label text-muted-foreground">
                  {item.description}
                </td>
                <td className="min-w-0 truncate px-1 align-middle text-meta text-muted-foreground">
                  {item.tags.join(", ")}
                  {item.overflowTagCount > 0
                    ? ` ${t("promptsView.items.moreTags", { count: item.overflowTagCount })}`
                    : ""}
                </td>
                <td className="min-w-0 truncate px-1 align-middle text-label">
                  <span className="inline-flex items-center gap-1">
                    <TypeBadge kind={item.typeKind} />
                    <span className="truncate">{item.typeLabel}</span>
                  </span>
                </td>
                <td className="min-w-0 truncate px-1 align-middle font-mono text-label tabular-nums text-muted-foreground-subtle">
                  {item.usageCount}
                </td>
                <td className="min-w-0 truncate px-1 align-middle font-mono text-label tabular-nums text-muted-foreground-subtle">
                  {item.versionLabel}
                </td>
                <td className="min-w-0 truncate px-1 align-middle font-mono text-label text-muted-foreground-subtle">
                  {item.updatedLabel}
                </td>
                <td className="px-1 align-middle">
                  <button
                    type="button"
                    aria-pressed={item.isFavorite}
                    aria-label={item.isFavorite ? t("promptsView.unfavorite") : t("promptsView.favorite")}
                    onClick={() => onToggleFavorite(item.id, !item.isFavorite)}
                    className="rounded-sm p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                  >
                    <StarIcon className={`h-3.5 w-3.5 ${item.isFavorite ? "fill-current text-primary" : ""}`} aria-hidden="true" />
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
