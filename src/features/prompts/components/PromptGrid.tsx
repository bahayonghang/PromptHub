import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { ImageIcon, LockIcon, PinIcon, StarIcon, VideoIcon } from "lucide-react";
import type { LibraryItem } from "../libraryItem";
import { CopyPromptButton } from "./CopyPromptButton";

interface PromptGridProps {
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

export function PromptGrid({
  items,
  selectedPromptId,
  selectedPromptIds,
  batchMode = false,
  onSelect,
  onToggleSelection,
  onToggleFavorite,
  writeText,
  copyPrompt,
}: PromptGridProps) {
  const { t } = useTranslation();

  return (
    <ul className="grid grid-cols-[repeat(auto-fill,minmax(272px,1fr))] gap-4 p-3">
      {items.map((item) => {
        const selected = item.id === selectedPromptId;
        const checked = selectedPromptIds.includes(item.id);
        return (
          <li key={item.id}>
            <article
              className={`flex h-full flex-col gap-2 rounded-lg border p-3 ${
                selected ? "border-primary ring-1 ring-primary" : "border-border"
              }`}
            >
              <div className="flex items-start gap-2">
                {batchMode && (
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={() => onToggleSelection(item.id)}
                    aria-label={t("promptsView.batch.selectPrompt", { title: item.title })}
                    className="mt-1 h-4 w-4 rounded-sm border-input text-primary focus:ring-ring"
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
                    <h3 className="min-w-0 truncate text-sm font-medium text-foreground">{item.title}</h3>
                  </div>
                  <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{item.description}</p>
                </div>
                <button
                  type="button"
                  aria-pressed={item.isFavorite}
                  aria-label={item.isFavorite ? t("promptsView.unfavorite") : t("promptsView.favorite")}
                  onClick={() => onToggleFavorite(item.id, !item.isFavorite)}
                  className="rounded-sm p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                >
                  <StarIcon className={`h-4 w-4 ${item.isFavorite ? "fill-current text-primary" : ""}`} aria-hidden="true" />
                </button>
              </div>
              {(item.tags.length > 0 || item.overflowTagCount > 0) && (
                <div className="flex flex-wrap gap-1">
                  {item.tags.map((tag) => (
                    <span key={tag} className="rounded-full bg-muted px-2 py-0.5 text-micro text-muted-foreground">
                      {tag}
                    </span>
                  ))}
                  {item.overflowTagCount > 0 && (
                    <span className="text-micro text-muted-foreground-subtle">
                      {t("promptsView.items.moreTags", { count: item.overflowTagCount })}
                    </span>
                  )}
                </div>
              )}
              <div className="mt-auto flex items-center gap-2 font-mono text-meta text-muted-foreground-subtle">
                <span className="inline-flex min-w-0 items-center gap-1 truncate">
                  <TypeBadge kind={item.typeKind} />
                  {item.typeLabel}
                </span>
                <span className="tabular-nums">{item.usageCount}</span>
                <span>{item.updatedLabel}</span>
                <span>{item.versionLabel}</span>
              </div>
            </article>
          </li>
        );
      })}
    </ul>
  );
}
