import { useTranslation } from "react-i18next";
import { PlusIcon, XIcon } from "lucide-react";
import type { CreateFolderInput, Folder } from "../../../types";
import type { PromptDraft } from "../promptDraft";
import { FolderPicker } from "../FolderPicker";

export interface OrganizationSectionProps {
  draft: PromptDraft;
  folders: Folder[];
  knownTags: string[];
  tagInput: string;
  resetKey: string;
  readOnly: boolean;
  onChange: (patch: Partial<PromptDraft>) => void;
  onTagInputChange: (value: string) => void;
  onAddTag: (raw: string) => void;
  onCreateFolder: (input: CreateFolderInput) => Promise<Folder | null>;
}

const labelClass = "text-xs font-medium text-muted-foreground";
const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60";

export function OrganizationSection({
  draft,
  folders,
  knownTags,
  tagInput,
  resetKey,
  readOnly,
  onChange,
  onTagInputChange,
  onAddTag,
  onCreateFolder,
}: OrganizationSectionProps) {
  const { t } = useTranslation();

  return (
    <section
      aria-labelledby="prompt-editor-organization"
      className="border-t border-border pt-5"
    >
      <h3
        id="prompt-editor-organization"
        className="text-sm font-semibold text-foreground"
      >
        {t("promptsView.editor.sections.organization")}
      </h3>
      <div className="prompt-editor__organization mt-3 grid grid-cols-1 gap-4">
        <FolderPicker
          key={resetKey}
          folders={folders}
          value={draft.folderId}
          disabled={readOnly}
          onChange={(folderId) => onChange({ folderId })}
          onCreateFolder={onCreateFolder}
        />

        <div className="flex min-w-0 flex-col gap-1.5">
          <label className={labelClass} htmlFor="prompt-tag">
            {t("promptsView.editor.tags")}
          </label>
          {draft.tags.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {draft.tags.map((tag) => (
                <span
                  key={tag}
                  className="flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs text-foreground"
                >
                  {tag}
                  <button
                    type="button"
                    aria-label={t("common.cancel")}
                    disabled={readOnly}
                    onClick={() =>
                      onChange({
                        tags: draft.tags.filter((item) => item !== tag),
                      })
                    }
                    className="rounded text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40"
                  >
                    <XIcon className="h-3 w-3" aria-hidden="true" />
                  </button>
                </span>
              ))}
            </div>
          )}
          <div className="prompt-editor__tag-row flex min-w-0 flex-col gap-2">
            <input
              id="prompt-tag"
              value={tagInput}
              list="known-tags"
              placeholder={t("promptsView.editor.addTagPlaceholder")}
              disabled={readOnly}
              onChange={(e) => onTagInputChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  onAddTag(tagInput);
                }
              }}
              className={inputClass}
            />
            <datalist id="known-tags">
              {knownTags.map((tag) => (
                <option key={tag} value={tag} />
              ))}
            </datalist>
            <button
              type="button"
              disabled={readOnly}
              onClick={() => onAddTag(tagInput)}
              className="flex min-h-9 shrink-0 items-center justify-center gap-1 rounded-md border border-input px-3 text-sm text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40"
            >
              <PlusIcon className="h-4 w-4" aria-hidden="true" />
              {t("promptsView.editor.addTag")}
            </button>
          </div>
        </div>
      </div>
    </section>
  );
}
