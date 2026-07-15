import { useTranslation } from "react-i18next";
import { ImageIcon, LockIcon, StarIcon, VideoIcon } from "lucide-react";
import type { Prompt } from "../types";

interface PromptListProps {
  prompts: Prompt[];
  selectedPromptId: string | null;
  selectedPromptIds: string[];
  loading: boolean;
  onSelect: (id: string) => void;
  onToggleSelection: (id: string) => void;
}

/** A small type badge for image/video prompts (text prompts show none). */
function TypeBadge({ type }: { type: Prompt["promptType"] }) {
  if (type === "image") {
    return <ImageIcon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />;
  }
  if (type === "video") {
    return <VideoIcon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />;
  }
  return null;
}

/**
 * The scrollable prompt list (Req 6.3). Each row shows the title, a one-line
 * preview, tags, and favorite/type indicators, and selects the prompt for
 * editing on click. Renders an empty state when no prompt matches the search.
 */
export function PromptList({
  prompts,
  selectedPromptId,
  selectedPromptIds,
  loading,
  onSelect,
  onToggleSelection,
}: PromptListProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
        {t("promptsView.loading")}
      </div>
    );
  }

  if (prompts.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-1 p-6 text-center">
        <p className="text-sm font-medium text-foreground">
          {t("promptsView.noPrompts")}
        </p>
        <p className="max-w-xs text-xs text-muted-foreground">
          {t("promptsView.noPromptsHint")}
        </p>
      </div>
    );
  }

  return (
    <ul className="flex flex-col gap-1 p-2">
      {prompts.map((prompt) => {
        const selected = prompt.id === selectedPromptId;
        const title = prompt.title.trim() || t("promptsView.untitled");
        return (
          <li key={prompt.id} className="flex items-start gap-1">
            <input
              type="checkbox"
              checked={selectedPromptIds.includes(prompt.id)}
              onChange={() => onToggleSelection(prompt.id)}
              aria-label={t("promptsView.batch.selectPrompt", { title })}
              className="mt-3 h-4 w-4 shrink-0 rounded border-input text-primary focus:ring-ring"
            />
            <button
              type="button"
              onClick={() => onSelect(prompt.id)}
              aria-current={selected ? "true" : undefined}
              className={`flex min-w-0 flex-1 flex-col gap-1 rounded-lg border px-3 py-2 text-left transition-colors ${
                selected
                  ? "border-primary bg-primary/10"
                  : "border-transparent hover:bg-accent"
              }`}
            >
              <span className="flex items-center gap-1.5">
                <TypeBadge type={prompt.promptType} />
                {prompt.isPrivate && (
                  <LockIcon
                    className="h-3.5 w-3.5 shrink-0 text-muted-foreground"
                    aria-label={t("promptsView.privatePrompt")}
                  />
                )}
                <span className="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
                  {title}
                </span>
                {prompt.isFavorite && (
                  <StarIcon
                    className="h-3.5 w-3.5 shrink-0 fill-current text-primary"
                    aria-label={t("promptsView.favorite")}
                  />
                )}
              </span>
              <span className="line-clamp-2 text-xs text-muted-foreground">
                {prompt.isLocked
                  ? t("promptsView.privateLockedPreview")
                  : prompt.userPrompt}
              </span>
              {prompt.tags.length > 0 && (
                <span className="flex flex-wrap gap-1">
                  {prompt.tags.slice(0, 4).map((tag) => (
                    <span
                      key={tag}
                      className="rounded-full bg-muted px-2 py-0.5 text-[10px] text-muted-foreground"
                    >
                      {tag}
                    </span>
                  ))}
                </span>
              )}
            </button>
          </li>
        );
      })}
    </ul>
  );
}
