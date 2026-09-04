import { useTranslation } from "react-i18next";
import { MediaRefList } from "../../MediaRefList";
import type { PromptDraft } from "../promptDraft";
import { Input, Textarea } from "../../../../../components/ui";

export interface MediaSectionProps {
  draft: PromptDraft;
  readOnly: boolean;
  onChange: (patch: Partial<PromptDraft>) => void;
}

const labelClass = "text-label font-medium text-muted-foreground";
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
        className="text-body font-semibold text-foreground"
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
          <Input
            id="prompt-source"
            value={draft.source}
            placeholder={t("promptsView.editor.sourcePlaceholder")}
            disabled={readOnly}
            onChange={(e) => onChange({ source: e.target.value })}
            size="lg"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-notes">
            {t("promptsView.editor.notes")}
          </label>
          <Textarea
            id="prompt-notes"
            value={draft.notes}
            placeholder={t("promptsView.editor.notesPlaceholder")}
            disabled={readOnly}
            onChange={(e) => onChange({ notes: e.target.value })}
            rows={3}
            size="lg"
            resizable
          />
        </div>
      </div>
    </section>
  );
}
