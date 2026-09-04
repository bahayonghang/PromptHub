import { useTranslation } from "react-i18next";
import { PlayIcon, PlusIcon, SquareIcon, XIcon } from "lucide-react";
import { Button, IconButton, Input, Select, Textarea } from "../../../components/ui";
import { PanelHeading } from "./Field";
import type {
  EvaluatorConfig,
  EvaluatorInput,
  ExecutionProfileRevision,
  TestCaseInput,
  TestSet,
} from "../types";
import type { PromptVersion } from "../../prompts/types";

/** Checkbox row shared by the evaluator and scope pickers. */
function CheckRow({
  checked,
  onChange,
  children,
}: {
  checked: boolean;
  onChange: () => void;
  children: React.ReactNode;
}) {
  return (
    <label className="flex items-center gap-1.5 text-label text-muted-foreground">
      <input
        type="checkbox"
        checked={checked}
        onChange={onChange}
        className="h-3.5 w-3.5 shrink-0 accent-primary"
      />
      <span className="min-w-0 truncate">{children}</span>
    </label>
  );
}

export interface MatrixSetupPanelProps {
  versions: PromptVersion[];
  profiles: ExecutionProfileRevision[];
  testSets: TestSet[];
  evaluators: EvaluatorConfig[];

  testSetName: string;
  onTestSetNameChange: (value: string) => void;
  testCases: TestCaseInput[];
  onAddCase: () => void;
  onUpdateCase: (index: number, next: TestCaseInput) => void;
  onRemoveCase: (index: number) => void;
  onSaveTestSet: () => void;
  testSetId: string;
  onTestSetIdChange: (id: string) => void;
  testSetJson: string;
  onTestSetJsonChange: (value: string) => void;
  onImportTestSet: () => void;
  onExportTestSet: () => void;

  selectedEvaluators: string[];
  onToggleEvaluator: (id: string) => void;
  evaluatorDraft: EvaluatorInput;
  onEvaluatorDraftChange: (draft: EvaluatorInput) => void;
  evaluatorConfigJson: string;
  onEvaluatorConfigJsonChange: (value: string) => void;
  evaluatorConfigValid: boolean;
  onCreateEvaluator: () => void;

  selectedRevisions: string[];
  onToggleRevision: (id: string) => void;
  selectedProfiles: string[];
  onToggleProfile: (id: string) => void;
  onRunMatrix: () => void;
  onCancelMatrix: () => void;
  matrixRunning: boolean;
  progress: { completed: number; total: number } | null;
}

/**
 * Top three columns of the Matrix tab: the test set, the evaluators, and the
 * revision/profile scope that together define one matrix run.
 */
export function MatrixSetupPanel(props: MatrixSetupPanelProps) {
  const { t } = useTranslation();
  const {
    versions,
    profiles,
    testSets,
    evaluators,
    testSetName,
    onTestSetNameChange,
    testCases,
    onAddCase,
    onUpdateCase,
    onRemoveCase,
    onSaveTestSet,
    testSetId,
    onTestSetIdChange,
    testSetJson,
    onTestSetJsonChange,
    onImportTestSet,
    onExportTestSet,
    selectedEvaluators,
    onToggleEvaluator,
    evaluatorDraft,
    onEvaluatorDraftChange,
    evaluatorConfigJson,
    onEvaluatorConfigJsonChange,
    evaluatorConfigValid,
    onCreateEvaluator,
    selectedRevisions,
    onToggleRevision,
    selectedProfiles,
    onToggleProfile,
    onRunMatrix,
    onCancelMatrix,
    matrixRunning,
    progress,
  } = props;

  const scopeIncomplete =
    !testSetId ||
    selectedRevisions.length === 0 ||
    selectedProfiles.length === 0 ||
    selectedEvaluators.length === 0;

  return (
    <div className="grid max-h-80 shrink-0 grid-cols-1 overflow-y-auto border-b border-border lg:max-h-none lg:grid-cols-3 lg:overflow-visible">
      {/* Test set */}
      <section className="flex flex-col gap-2 border-r border-border p-3">
        <PanelHeading
          action={
            <IconButton
              label={t("evaluation.addCase")}
              icon={<PlusIcon className="h-3.5 w-3.5" />}
              size="sm"
              onClick={onAddCase}
            />
          }
        >
          {t("evaluation.testSet")}
        </PanelHeading>
        <Input
          value={testSetName}
          onChange={(event) => onTestSetNameChange(event.target.value)}
          placeholder={t("evaluation.testSetName")}
          aria-label={t("evaluation.testSetName")}
        />
        <div className="max-h-32 space-y-1.5 overflow-y-auto">
          {testCases.map((testCase, index) => (
            <div key={index} className="grid grid-cols-[1fr_1.4fr_auto] gap-1">
              <Input
                value={testCase.name}
                onChange={(event) =>
                  onUpdateCase(index, { ...testCase, name: event.target.value })
                }
                aria-label={t("evaluation.caseName")}
              />
              <Input
                value={JSON.stringify(testCase.inputs)}
                onChange={(event) => {
                  try {
                    const parsed: unknown = JSON.parse(event.target.value);
                    if (parsed == null || typeof parsed !== "object" || Array.isArray(parsed)) {
                      return;
                    }
                    onUpdateCase(index, {
                      ...testCase,
                      inputs: Object.fromEntries(
                        Object.entries(parsed as Record<string, unknown>).map(
                          ([key, value]) => [key, String(value)],
                        ),
                      ),
                    });
                  } catch {
                    // Keep the previous value while the JSON is mid-edit.
                  }
                }}
                aria-label={t("evaluation.caseInputs")}
                className="font-mono"
              />
              <IconButton
                label={t("evaluation.removeCase")}
                icon={<XIcon className="h-3.5 w-3.5" />}
                size="sm"
                onClick={() => onRemoveCase(index)}
              />
            </div>
          ))}
        </div>
        <div className="flex gap-1">
          <Button
            variant="primary"
            size="sm"
            disabled={!testSetName.trim() || testCases.length === 0}
            onClick={onSaveTestSet}
          >
            {t("common.save")}
          </Button>
          <Select
            value={testSetId}
            onChange={(event) => onTestSetIdChange(event.target.value)}
            aria-label={t("evaluation.testSet")}
            size="sm"
            wrapperClassName="min-w-0 flex-1"
          >
            <option value="">{t("evaluation.selectTestSet")}</option>
            {testSets.map((set) => (
              <option key={set.id} value={set.id}>
                {set.name} ({set.cases.length})
              </option>
            ))}
          </Select>
        </div>
        <Textarea
          value={testSetJson}
          onChange={(event) => onTestSetJsonChange(event.target.value)}
          placeholder={t("evaluation.testSetJson")}
          aria-label={t("evaluation.testSetJson")}
          rows={2}
          mono
        />
        <div className="flex gap-1">
          <Button
            variant="ghost"
            size="sm"
            disabled={!testSetJson.trim()}
            onClick={onImportTestSet}
          >
            {t("evaluation.import")}
          </Button>
          <Button variant="ghost" size="sm" disabled={!testSetId} onClick={onExportTestSet}>
            {t("evaluation.export")}
          </Button>
        </div>
      </section>

      {/* Evaluators */}
      <section className="flex flex-col gap-2 border-r border-border p-3">
        <PanelHeading>{t("evaluation.evaluators")}</PanelHeading>
        <div className="flex max-h-20 flex-wrap gap-x-3 gap-y-1 overflow-y-auto">
          {evaluators.map((evaluator) => (
            <CheckRow
              key={evaluator.id}
              checked={selectedEvaluators.includes(evaluator.id)}
              onChange={() => onToggleEvaluator(evaluator.id)}
            >
              {evaluator.name}
            </CheckRow>
          ))}
        </div>
        <div className="grid grid-cols-1 gap-1 xl:grid-cols-2">
          <Input
            value={evaluatorDraft.name}
            onChange={(event) =>
              onEvaluatorDraftChange({ ...evaluatorDraft, name: event.target.value })
            }
            placeholder={t("evaluation.evaluatorName")}
            aria-label={t("evaluation.evaluatorName")}
          />
          <Select
            value={evaluatorDraft.kind}
            onChange={(event) =>
              onEvaluatorDraftChange({
                ...evaluatorDraft,
                kind: event.target.value as EvaluatorInput["kind"],
              })
            }
            aria-label={t("evaluation.evaluatorKind")}
            block
          >
            {(["manual", "exact", "contains", "regex", "numeric"] as const).map((kind) => (
              <option key={kind} value={kind}>
                {kind}
              </option>
            ))}
          </Select>
        </div>
        <Textarea
          value={evaluatorConfigJson}
          onChange={(event) => onEvaluatorConfigJsonChange(event.target.value)}
          aria-label={t("evaluation.evaluatorConfig")}
          rows={2}
          mono
          invalid={!evaluatorConfigValid}
        />
        <Button
          size="sm"
          className="w-fit"
          disabled={!evaluatorDraft.name.trim() || !evaluatorConfigValid}
          onClick={onCreateEvaluator}
        >
          {t("evaluation.addEvaluator")}
        </Button>
      </section>

      {/* Scope */}
      <section className="flex flex-col gap-2 p-3">
        <PanelHeading>{t("evaluation.matrixScope")}</PanelHeading>
        <div className="grid grid-cols-1 gap-2 xl:grid-cols-2">
          <div className="max-h-24 space-y-1 overflow-y-auto">
            {versions.map((version) => (
              <CheckRow
                key={version.id}
                checked={selectedRevisions.includes(version.id)}
                onChange={() => onToggleRevision(version.id)}
              >
                <span className="font-mono tabular-nums">v{version.version}</span>
              </CheckRow>
            ))}
          </div>
          <div className="max-h-24 space-y-1 overflow-y-auto">
            {profiles.map((profile) => (
              <CheckRow
                key={profile.id}
                checked={selectedProfiles.includes(profile.id)}
                onChange={() => onToggleProfile(profile.id)}
              >
                {profile.name}
              </CheckRow>
            ))}
          </div>
        </div>
        <div className="mt-1 flex items-center gap-2">
          {matrixRunning ? (
            <Button size="sm" onClick={onCancelMatrix}>
              <SquareIcon className="h-3 w-3" aria-hidden="true" />
              {t("common.cancel")}
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              disabled={scopeIncomplete}
              onClick={onRunMatrix}
            >
              <PlayIcon className="h-3 w-3" aria-hidden="true" />
              {t("evaluation.runMatrix")}
            </Button>
          )}
          {progress && (
            <span aria-live="polite" className="text-label tabular-nums text-muted-foreground">
              {progress.completed}/{progress.total}
            </span>
          )}
        </div>
      </section>
    </div>
  );
}
