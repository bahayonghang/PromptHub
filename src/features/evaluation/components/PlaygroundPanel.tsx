import { useTranslation } from "react-i18next";
import { PlayIcon, PlusIcon, SquareIcon } from "lucide-react";
import { Button, EmptyHint, Input, Select, Textarea } from "../../../components/ui";
import { Field, PanelHeading } from "./Field";
import type { ExecutionProfileInput, ExecutionProfileRevision } from "../types";
import type { PromptVersion } from "../../prompts/types";

export interface PlaygroundPanelProps {
  versions: PromptVersion[];
  profiles: ExecutionProfileRevision[];
  revisionId: string;
  onRevisionChange: (id: string) => void;
  profileId: string;
  onProfileChange: (id: string) => void;
  activeVersion?: PromptVersion;
  inputs: Record<string, string>;
  onInputChange: (name: string, value: string) => void;
  showProfileForm: boolean;
  onToggleProfileForm: () => void;
  profileDraft: ExecutionProfileInput;
  onProfileDraftChange: (draft: ExecutionProfileInput) => void;
  parametersJson: string;
  onParametersJsonChange: (value: string) => void;
  parametersValid: boolean;
  onSaveProfile: () => void;
  onPreview: () => void;
  onRun: () => void;
  onCancel: () => void;
  running: boolean;
}

/**
 * Left column of the Playground tab: pick a revision and profile, fill the
 * prompt variables, then preview or run.
 */
export function PlaygroundPanel({
  versions,
  profiles,
  revisionId,
  onRevisionChange,
  profileId,
  onProfileChange,
  activeVersion,
  inputs,
  onInputChange,
  showProfileForm,
  onToggleProfileForm,
  profileDraft,
  onProfileDraftChange,
  parametersJson,
  onParametersJsonChange,
  parametersValid,
  onSaveProfile,
  onPreview,
  onRun,
  onCancel,
  running,
}: PlaygroundPanelProps) {
  const { t } = useTranslation();

  return (
    <section className="overflow-y-auto border-r border-border p-4">
      <div className="grid grid-cols-2 gap-3">
        <Field label={t("evaluation.revision")}>
          <Select
            value={revisionId}
            onChange={(event) => onRevisionChange(event.target.value)}
            block
          >
            {versions.map((version) => (
              <option key={version.id} value={version.id}>
                v{version.version}
              </option>
            ))}
          </Select>
        </Field>
        <Field label={t("evaluation.profile")}>
          <Select
            value={profileId}
            onChange={(event) => onProfileChange(event.target.value)}
            block
          >
            <option value="">{t("evaluation.noProfile")}</option>
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name} r{profile.revision}
              </option>
            ))}
          </Select>
        </Field>
      </div>

      <Button
        variant="ghost"
        size="sm"
        className="mt-2"
        onClick={onToggleProfileForm}
        aria-expanded={showProfileForm}
      >
        <PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
        {t("evaluation.newProfile")}
      </Button>

      {showProfileForm && (
        <div className="mt-3 grid gap-2 border-y border-border py-3">
          <Input
            value={profileDraft.name}
            onChange={(event) =>
              onProfileDraftChange({ ...profileDraft, name: event.target.value })
            }
            placeholder={t("evaluation.profileName")}
            aria-label={t("evaluation.profileName")}
          />
          <div className="grid grid-cols-2 gap-2">
            <Select
              value={profileDraft.provider}
              onChange={(event) =>
                onProfileDraftChange({
                  ...profileDraft,
                  provider: event.target.value as ExecutionProfileInput["provider"],
                })
              }
              aria-label={t("evaluation.provider")}
              block
            >
              <option value="mock">mock</option>
              <option value="openai-compatible">openai-compatible</option>
            </Select>
            <Input
              value={profileDraft.model}
              onChange={(event) =>
                onProfileDraftChange({ ...profileDraft, model: event.target.value })
              }
              placeholder={t("evaluation.model")}
              aria-label={t("evaluation.model")}
            />
          </div>
          {profileDraft.provider === "openai-compatible" && (
            <>
              <Input
                value={profileDraft.endpoint ?? ""}
                onChange={(event) =>
                  onProfileDraftChange({ ...profileDraft, endpoint: event.target.value })
                }
                placeholder={t("evaluation.endpoint")}
                aria-label={t("evaluation.endpoint")}
              />
              <Input
                type="password"
                value={profileDraft.credential ?? ""}
                onChange={(event) =>
                  onProfileDraftChange({ ...profileDraft, credential: event.target.value })
                }
                placeholder={t("evaluation.credential")}
                aria-label={t("evaluation.credential")}
              />
            </>
          )}
          <Textarea
            value={parametersJson}
            onChange={(event) => onParametersJsonChange(event.target.value)}
            aria-label={t("evaluation.parameters")}
            rows={3}
            mono
            invalid={!parametersValid}
          />
          <Button
            variant="primary"
            size="sm"
            className="w-fit"
            disabled={!parametersValid || !profileDraft.name.trim()}
            onClick={onSaveProfile}
          >
            {t("common.save")}
          </Button>
        </div>
      )}

      <div className="mt-4 flex flex-col gap-2">
        <PanelHeading>{t("evaluation.variables")}</PanelHeading>
        {activeVersion?.variables.map((variable) => (
          <Field
            key={variable.name}
            label={`${variable.label || variable.name}${variable.required ? " *" : ""}`}
          >
            <Input
              value={inputs[variable.name] ?? variable.defaultValue ?? ""}
              onChange={(event) => onInputChange(variable.name, event.target.value)}
            />
          </Field>
        ))}
        {activeVersion?.variables.length === 0 && (
          <EmptyHint>{t("evaluation.noVariables")}</EmptyHint>
        )}
      </div>

      <div className="mt-4 flex gap-2">
        <Button size="sm" disabled={!revisionId} onClick={onPreview}>
          {t("evaluation.preview")}
        </Button>
        {running ? (
          <Button size="sm" onClick={onCancel}>
            <SquareIcon className="h-3.5 w-3.5" aria-hidden="true" />
            {t("common.cancel")}
          </Button>
        ) : (
          <Button
            variant="primary"
            size="sm"
            disabled={!revisionId || !profileId}
            onClick={onRun}
          >
            <PlayIcon className="h-3.5 w-3.5" aria-hidden="true" />
            {t("evaluation.run")}
          </Button>
        )}
      </div>
    </section>
  );
}
