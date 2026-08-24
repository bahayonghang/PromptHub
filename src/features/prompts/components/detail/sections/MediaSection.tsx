import { useTranslation } from "react-i18next";
import { MediaRefList } from "../../MediaRefList";
import type { PromptDraft } from "../promptDraft";

export interface MediaSectionProps {
  draft: PromptDraft;
  readOnly: boolean;
  onChange: (patch: Partial<PromptDraft>) => void;
}

const labelClass = "text-xs font-medium text-muted-foreground";
const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60";

export function MediaSection({
  draft,
  readOnly,
  onChange,
}: MediaSectionProps) {
  const { t } = useTranslation();

  return (
    <section
      aria-labelledby="prompt-editor-references"
      className="border-t border-border pt-5"
    >
      <h3
        id="prompt-editor-references"
        className="text-sm font-semibold text-foreground"
      >
        {t("promptsView.editor.sections.attachments")}
      </h3>
      <div className="prompt-editor__two-column mt-3 grid grid-cols-1 gap-4">
        <MediaRefList
          label={t("promptsView.editor.images")}
          kind="image"
          refs={draft.images}
          onChange={(images) => onChange({ images })}
        />
        <MediaRefList
          label={t("promptsView.editor.videos")}
          kind="video"
          refs={draft.videos}
          onChange={(videos) => onChange({ videos })}
        />
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-source">
            {t("promptsView.editor.source")}
          </label>
          <input
            id="prompt-source"
            value={draft.source}
            placeholder={t("promptsView.editor.sourcePlaceholder")}
            disabled={readOnly}
            onChange={(e) => onChange({ source: e.target.value })}
            className={inputClass}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-notes">
            {t("promptsView.editor.notes")}
          </label>
          <textarea
            id="prompt-notes"
            value={draft.notes}
            placeholder={t("promptsView.editor.notesPlaceholder")}
            disabled={readOnly}
            onChange={(e) => onChange({ notes: e.target.value })}
            rows={3}
            className={`${inputClass} resize-y`}
          />
        </div>
      </div>
    </section>
  );
}
