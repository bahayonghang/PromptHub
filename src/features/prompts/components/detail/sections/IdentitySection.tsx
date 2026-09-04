import { useTranslation } from "react-i18next";
import type {
  CreatePromptTypeInput,
  PromptType,
  PromptTypeDefinition,
} from "../../../types";
import type { PromptDraft } from "../promptDraft";
import { PromptTypePicker } from "../PromptTypePicker";
import { Input } from "../../../../../components/ui";

export interface IdentitySectionProps {
  draft: PromptDraft;
  titleValid: boolean;
  readOnly: boolean;
  promptTypeDefinitions: PromptTypeDefinition[];
  onChange: (patch: Partial<PromptDraft>) => void;
  onCreatePromptType: (
    input: CreatePromptTypeInput,
  ) => Promise<PromptTypeDefinition | null>;
}

const labelClass = "text-label font-medium text-muted-foreground";
export function IdentitySection({
  draft,
  titleValid,
  readOnly,
  promptTypeDefinitions,
  onChange,
  onCreatePromptType,
}: IdentitySectionProps) {
  const { t } = useTranslation();

  return (
    <section aria-labelledby="prompt-editor-basics">
      <h3
        id="prompt-editor-basics"
        className="text-body font-semibold text-foreground"
      >
        {t("promptsView.editor.sections.identity")}
      </h3>
      <div className="prompt-editor__two-column mt-3 grid grid-cols-1 gap-4">
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-title">
            {t("promptsView.editor.title")}
          </label>
          <Input
            id="prompt-title"
            value={draft.title}
            placeholder={t("promptsView.editor.titlePlaceholder")}
            disabled={readOnly}
            onChange={(e) => onChange({ title: e.target.value })}
            size="lg"
          />
          {!titleValid && (
            <span className="text-label text-destructive">
              {t("promptsView.editor.titleRequired")}
            </span>
          )}
        </div>

        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-description">
            {t("promptsView.editor.description")}
          </label>
          <Input
            id="prompt-description"
            value={draft.description}
            placeholder={t("promptsView.editor.descriptionPlaceholder")}
            disabled={readOnly}
            onChange={(e) => onChange({ description: e.target.value })}
            size="lg"
          />
        </div>

        <PromptTypePicker
          definitions={promptTypeDefinitions}
          baseKind={draft.promptType}
          definitionId={draft.typeDefinitionId}
          disabled={readOnly}
          onChange={(promptType: PromptType, typeDefinitionId) =>
            onChange({ promptType, typeDefinitionId })
          }
          onCreate={onCreatePromptType}
        />

        <label className="flex items-center gap-2 self-end py-2 text-body text-foreground">
          <Input
            type="checkbox"
            checked={draft.isPrivate}
            disabled={readOnly}
            onChange={(event) => onChange({ isPrivate: event.target.checked })}
            className="h-4 w-4 shrink-0 rounded-sm border-input text-primary"
          />
          <span>
            {t("promptsView.editor.privatePrompt")}
            <span className="block text-label text-muted-foreground">
              {t("promptsView.editor.privatePromptHint")}
            </span>
          </span>
        </label>
      </div>
    </section>
  );
}
