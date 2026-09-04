import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { ImageIcon, LockIcon, PinIcon, StarIcon, VideoIcon } from "lucide-react";
import type { LibraryItem } from "../libraryItem";
import { CopyPromptButton } from "./CopyPromptButton";

import { IconButton, Skeleton, Tag, UsageBar } from "../../../components/ui";
import { cn } from "../../../components/ui/cn";

interface PromptListProps {
  items: LibraryItem[];
  selectedPromptId: string | null;
  selectedPromptIds: string[];
  batchMode?: boolean;
  /** When true, render skeleton rows that match the table geometry. */
  loading?: boolean;
  onSelect: (id: string) => void;
  onToggleSelection: (id: string) => void;
  onToggleFavorite: (id: string, next: boolean) => void;
  writeText?: (text: string) => Promise<void>;
  copyPrompt?: (id: string, values: Record<string, string>) => Promise<import("../types").PromptCopyResult>;
}

const SKELETON_ROWS = 8;

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
 * Dense library table. The title cell is two lines (name + description) so the
 * description column can drop; usage and version sit under the title. Copy is
 * the previous sibling of the title activator so tests can walk
 * `nextElementSibling`.
 */
export function PromptList({
  items,
  selectedPromptId,
  selectedPromptIds,
  batchMode = false,
  loading = false,
  onSelect,
  onToggleSelection,
  onToggleFavorite,
  writeText,
  copyPrompt,
}: PromptListProps) {
  const { t } = useTranslation();
  const usageMax = Math.max(1, ...items.map((item) => item.usageCount));

  return (
    <div
      className="p-2"
      role={loading ? "status" : undefined}
      aria-label={loading ? t("promptsView.loading") : undefined}
      aria-busy={loading || undefined}
    >
      <table className="w-full table-fixed border-collapse text-left">
        <thead className="sticky top-0 z-10 bg-background">
          <tr className="border-b border-border text-meta font-medium text-muted-foreground-subtle">
            {batchMode && <th className="w-10 px-3 py-2">{t("promptsView.items.columns.select")}</th>}
            <th className="min-w-0 px-3 py-2">{t("promptsView.items.columns.title")}</th>
            <th className="w-[22%] min-w-0 px-3 py-2">{t("promptsView.items.columns.tags")}</th>
            <th className="w-[12%] min-w-0 px-3 py-2">{t("promptsView.items.columns.type")}</th>
            <th className="w-[12%] min-w-0 px-3 py-2">{t("promptsView.items.columns.updated")}</th>
            <th className="w-12 px-3 py-2">
              <span className="sr-only">{t("promptsView.favorite")}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {loading
            ? Array.from({ length: SKELETON_ROWS }, (_, index) => (
                <tr key={index} className="border-b border-border text-body">
                  {batchMode && (
                    <td className="px-3 py-2 align-middle">
                      <Skeleton className="h-4 w-4" />
                    </td>
                  )}
                  <td className="px-3 py-2 align-middle">
                    <Skeleton className="h-3.5 w-4/5" />
                    <Skeleton className="mt-1 h-3 w-3/4" />
                  </td>
                  <td className="px-3 py-2 align-middle">
                    <Skeleton className="h-3 w-2/3" />
                  </td>
                  <td className="px-3 py-2 align-middle">
                    <Skeleton className="h-3 w-16" />
                  </td>
                  <td className="px-3 py-2 align-middle">
                    <Skeleton className="h-3 w-12" />
                  </td>
                  <td className="px-3 py-2 align-middle">
                    <Skeleton className="h-4 w-4" />
                  </td>
                </tr>
              ))
            : items.map((item) => {
            const selected = item.id === selectedPromptId;
            const checked = selectedPromptIds.includes(item.id);
            return (
              <tr
                key={item.id}
                className={cn(
                  "group border-b border-border text-body transition-colors duration-fast ease-out",
                  selected ? "bg-state-selected" : "hover:bg-state-hover",
                )}
              >
                {batchMode && (
                  <td className="min-w-0 px-3 py-2 align-middle">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={() => onToggleSelection(item.id)}
                      aria-label={t("promptsView.batch.selectPrompt", { title: item.title })}
                      className="h-4 w-4 rounded-sm border-input text-primary"
                    />
                  </td>
                )}
                <td className="relative min-w-0 px-3 py-2 align-middle">
                  {selected && (
                    <span
                      aria-hidden="true"
                      className="absolute inset-y-0 left-0 w-0.5 bg-primary"
                    />
                  )}
                  <div className="flex items-start gap-2">
                    <CopyPromptButton
                      source={item.source}
                      promptId={item.id}
                      copyPrompt={copyPrompt}
                      name={item.title}
                      locked={item.isLocked}
                      writeText={writeText}
                      className="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
                    />
                    <div
                      role="button"
                      tabIndex={0}
                      aria-label={item.title}
                      aria-current={selected ? "true" : undefined}
                      onClick={() => onSelect(item.id)}
                      onKeyDown={(event) => activateSelect(event, item.id, onSelect)}
                      className="min-w-0 flex-1 cursor-pointer text-left"
                    >
                      <div className="flex min-w-0 items-center gap-1.5 font-medium text-foreground">
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
                      <p className="mt-0.5 truncate text-label text-muted-foreground">
                        {item.description}
                      </p>
                      <div className="mt-0.5 flex items-center gap-2">
                        <UsageBar
                          value={item.usageCount}
                          max={usageMax}
                          label={t("promptsView.items.usageCount", { count: item.usageCount })}
                          className="w-16"
                        />
                        <span className="rounded-sm bg-muted px-1.5 py-0.5 font-mono text-meta tabular-nums text-foreground">
                          {item.versionLabel}
                        </span>
                      </div>
                    </div>
                  </div>
                </td>
                <td className="min-w-0 px-3 py-2 align-middle">
                  <div className="flex flex-wrap gap-1">
                    {item.tags.map((tag) => (
                      <Tag key={tag} name={tag} />
                    ))}
                    {item.overflowTagCount > 0 && (
                      <span className="text-micro text-muted-foreground-subtle">
                        {t("promptsView.items.moreTags", { count: item.overflowTagCount })}
                      </span>
                    )}
                  </div>
                </td>
                <td className="min-w-0 truncate px-3 py-2 align-middle text-label">
                  <span className="inline-flex items-center gap-1">
                    <TypeBadge kind={item.typeKind} />
                    <span className="truncate">{item.typeLabel}</span>
                  </span>
                </td>
                <td className="min-w-0 truncate px-3 py-2 align-middle font-mono text-label tabular-nums text-muted-foreground-subtle">
                  {item.updatedLabel}
                </td>
                <td className="px-3 py-2 align-middle">
                  <div
                    className={cn(
                      "flex justify-end transition-opacity duration-fast ease-out",
                      item.isFavorite
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100",
                    )}
                  >
                    <IconButton
                      label={item.isFavorite ? t("promptsView.unfavorite") : t("promptsView.favorite")}
                      icon={
                        <StarIcon
                          className={cn("h-3.5 w-3.5", item.isFavorite && "fill-current text-favorite")}
                          aria-hidden="true"
                        />
                      }
                      onClick={() => onToggleFavorite(item.id, !item.isFavorite)}
                      aria-pressed={item.isFavorite}
                    />
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
