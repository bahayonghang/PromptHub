import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { ImageIcon, LockIcon, PinIcon, StarIcon, VideoIcon } from "lucide-react";
import type { LibraryItem } from "../libraryItem";
import { CopyPromptButton } from "./CopyPromptButton";

import { IconButton, Skeleton, Tag } from "../../../components/ui";
import { cn } from "../../../components/ui/cn";

interface PromptGridProps {
  items: LibraryItem[];
  selectedPromptId: string | null;
  selectedPromptIds: string[];
  batchMode?: boolean;
  /** When true, render skeleton cards that match the grid geometry. */
  loading?: boolean;
  onSelect: (id: string) => void;
  onToggleSelection: (id: string) => void;
  onToggleFavorite: (id: string, next: boolean) => void;
  writeText?: (text: string) => Promise<void>;
  copyPrompt?: (id: string, values: Record<string, string>) => Promise<import("../types").PromptCopyResult>;
}

const SKELETON_CARDS = 6;

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
 * Card grid of library items. Cards share Panel's surface (card fill + hairline)
 * and select with a ring so a 1px border change never shifts neighbours.
 */
export function PromptGrid({
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
}: PromptGridProps) {
  const { t } = useTranslation();

  return (
    <ul
      className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3 p-3"
      role={loading ? "status" : undefined}
      aria-label={loading ? t("promptsView.loading") : undefined}
      aria-busy={loading || undefined}
    >
      {loading
        ? Array.from({ length: SKELETON_CARDS }, (_, index) => (
            <li key={index}>
              <article className="flex h-full flex-col gap-2 rounded-lg border border-border bg-card p-3 shadow-hairline">
                <Skeleton className="h-4 w-2/3" />
                <Skeleton className="h-3 w-full" />
                <Skeleton className="h-3 w-4/5" />
                <div className="mt-auto flex items-center gap-2">
                  <Skeleton className="h-3 w-16" />
                  <Skeleton className="h-3 w-10" />
                  <Skeleton className="h-3 w-14" />
                </div>
              </article>
            </li>
          ))
        : items.map((item) => {
        const selected = item.id === selectedPromptId;
        const checked = selectedPromptIds.includes(item.id);
        return (
          <li key={item.id}>
            <article
              className={cn(
                "flex h-full flex-col gap-2 rounded-lg border bg-card p-3 shadow-hairline",
                "transition-colors duration-fast ease-out",
                selected
                  ? "border-transparent ring-2 ring-primary/60 ring-offset-2 ring-offset-background"
                  : "border-border hover:border-border-strong hover:bg-state-hover",
              )}
            >
              <div className="flex items-start gap-2">
                {batchMode && (
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={() => onToggleSelection(item.id)}
                    aria-label={t("promptsView.batch.selectPrompt", { title: item.title })}
                    className="mt-1 h-4 w-4 rounded-sm border-input text-primary"
                  />
                )}
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
                  className="min-w-0 flex-1 cursor-pointer text-left"
                >
                  <div className="flex items-center gap-1.5">
                    {item.isPinned && (
                      <span className="inline-flex items-center gap-0.5 rounded-sm bg-muted px-1 text-micro text-muted-foreground">
                        <PinIcon className="h-3 w-3" aria-hidden="true" />
                        {t("promptsView.items.pinned")}
                      </span>
                    )}
                    {item.isPrivate && (
                      <LockIcon className="h-3.5 w-3.5 shrink-0" aria-label={t("promptsView.privatePrompt")} />
                    )}
                    <h3 className="min-w-0 truncate text-body font-medium text-foreground">{item.title}</h3>
                  </div>
                </div>
                <IconButton
                  label={item.isFavorite ? t("promptsView.unfavorite") : t("promptsView.favorite")}
                  icon={
                    <StarIcon
                      className={cn("h-4 w-4", item.isFavorite && "fill-current text-favorite")}
                      aria-hidden="true"
                    />
                  }
                  onClick={() => onToggleFavorite(item.id, !item.isFavorite)}
                  aria-pressed={item.isFavorite}
                />
              </div>
              <p className="min-h-10 line-clamp-2 text-label text-muted-foreground">{item.description}</p>
              {(item.tags.length > 0 || item.overflowTagCount > 0) && (
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
              )}
              <div className="mt-auto flex items-center gap-1.5 text-meta text-muted-foreground-subtle">
                <span className="inline-flex min-w-0 items-center gap-1 truncate">
                  <TypeBadge kind={item.typeKind} />
                  {item.typeLabel}
                </span>
                <span aria-hidden="true">·</span>
                <span className="tabular-nums">{item.usageCount}</span>
                <span aria-hidden="true">·</span>
                <span>{item.updatedLabel}</span>
                <span className="ml-auto rounded-sm bg-muted px-1.5 py-0.5 font-mono tabular-nums text-foreground">
                  {item.versionLabel}
                </span>
              </div>
            </article>
          </li>
        );
      })}
    </ul>
  );
}
