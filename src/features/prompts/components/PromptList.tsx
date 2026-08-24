import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { ImageIcon, LockIcon, StarIcon, VideoIcon } from "lucide-react";
import type { Prompt, PromptTypeDefinition } from "../types";
import { CopyPromptButton } from "./CopyPromptButton";

interface PromptListProps {
  prompts: Prompt[];
  promptTypeDefinitions: PromptTypeDefinition[];
  selectedPromptId: string | null;
  selectedPromptIds: string[];
  batchMode?: boolean;
  onSelect: (id: string) => void;
  onToggleSelection: (id: string) => void;
  writeText?: (text: string) => Promise<void>;
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
 * The scrollable prompt list (Req 6.3). Each row shows the title, a two-line
 * description preview, tags, and favorite/type indicators, and selects the
 * prompt for editing on click. Renders an empty state when no prompt matches
 * the search.
 */
export function PromptList({
  prompts,
  promptTypeDefinitions,
  selectedPromptId,
  selectedPromptIds,
  batchMode = false,
  onSelect,
  onToggleSelection,
  writeText,
}: PromptListProps) {
  const { t } = useTranslation();

  return (
    <ul className="flex flex-col gap-1 p-2">
      {prompts.map((prompt) => {
        const selected = prompt.id === selectedPromptId;
        const title = prompt.title.trim() || t("promptsView.untitled");
        const customType = prompt.typeDefinitionId
          ? promptTypeDefinitions.find(
              (definition) => definition.id === prompt.typeDefinitionId,
            )
          : null;
        return (
          <li key={prompt.id} className="flex items-start gap-1">
            {batchMode && (
              <input
                type="checkbox"
                checked={selectedPromptIds.includes(prompt.id)}
                onChange={() => onToggleSelection(prompt.id)}
                aria-label={t("promptsView.batch.selectPrompt", { title })}
                className="mt-3 h-4 w-4 shrink-0 rounded border-input text-primary focus:ring-ring"
              />
            )}
            <div
              className={`flex min-w-0 flex-1 items-start gap-1 rounded-lg border px-3 py-2 transition-colors ${
                selected
                  ? "border-primary bg-primary/10"
                  : "border-transparent hover:bg-accent"
              }`}
            >
              <div
                role="button"
                tabIndex={0}
                aria-label={title}
                aria-current={selected ? "true" : undefined}
                onClick={() => onSelect(prompt.id)}
                onKeyDown={(event) =>
                  activateSelect(event, prompt.id, onSelect)
                }
                className="flex min-w-0 flex-1 cursor-pointer flex-col gap-1 text-left"
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
                  {customType && (
                    <span className="max-w-24 truncate rounded border border-border bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                      {customType.name}
                    </span>
                  )}
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
                    : prompt.description?.trim() ||
                      t("promptsView.noDescription")}
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
              </div>
              <CopyPromptButton
                source={prompt}
                name={title}
                locked={prompt.isLocked}
                compact
                writeText={writeText}
              />
            </div>
          </li>
        );
      })}
    </ul>
  );
}
