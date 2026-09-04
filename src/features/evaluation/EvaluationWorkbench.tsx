import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { FlaskConicalIcon, HistoryIcon, PlayIcon } from "lucide-react";
import type { Prompt, PromptVersion } from "../prompts/types";
import { useEvaluationStore } from "./evaluationStore";
import type { EvaluatorInput, ExecutionProfileInput, TestCaseInput } from "./types";
import { ComparePanel } from "./components/ComparePanel";
import { HistoryPanel } from "./components/HistoryPanel";
import { MatrixGrid } from "./components/MatrixGrid";
import { MatrixSetupPanel } from "./components/MatrixSetupPanel";
import { OutputPane } from "./components/OutputPane";
import { PlaygroundPanel } from "./components/PlaygroundPanel";

interface EvaluationWorkbenchProps {
  prompt: Prompt;
  versions: PromptVersion[];
}

type WorkbenchTab = "playground" | "matrix" | "history";

function parseObject(raw: string): Record<string, unknown> | null {
  try {
    const value: unknown = JSON.parse(raw);
    return value != null && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

/**
 * Container for the evaluation workbench.
 *
 * Owns the draft/filter state and the store wiring; every pixel of the three
 * tabs lives in `./components`. Keeping the split at the state boundary means
 * the presentational panels take plain props and stay readable, which the
 * previous single 500-line file did not.
 */
export function EvaluationWorkbench({ prompt, versions }: EvaluationWorkbenchProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<WorkbenchTab>("playground");
  const [revisionId, setRevisionId] = useState(versions[versions.length - 1]?.id ?? "");
  const [profileId, setProfileId] = useState("");
  const [inputs, setInputs] = useState<Record<string, string>>({});
  const [showProfileForm, setShowProfileForm] = useState(false);
  const [profileDraft, setProfileDraft] = useState<ExecutionProfileInput>({
    name: "",
    provider: "mock",
    model: "deterministic",
    parameters: { response: "" },
  });
  const [parametersJson, setParametersJson] = useState('{"response":""}');
  const [testSetName, setTestSetName] = useState("");
  const [testCases, setTestCases] = useState<TestCaseInput[]>([]);
  const [testSetId, setTestSetId] = useState("");
  const [testSetJson, setTestSetJson] = useState("");
  const [evaluatorDraft, setEvaluatorDraft] = useState<EvaluatorInput>({
    name: "",
    kind: "exact",
    config: {},
  });
  const [evaluatorConfigJson, setEvaluatorConfigJson] = useState("{}");
  const [selectedRevisions, setSelectedRevisions] = useState<string[]>([]);
  const [selectedProfiles, setSelectedProfiles] = useState<string[]>([]);
  const [selectedEvaluators, setSelectedEvaluators] = useState<string[]>([]);
  const [compareCells, setCompareCells] = useState<string[]>([]);
  const [historyStatus, setHistoryStatus] = useState("");
  const [historyProfile, setHistoryProfile] = useState("");
  const [historyRevision, setHistoryRevision] = useState("");
  const [historyDate, setHistoryDate] = useState("");

  const profiles = useEvaluationStore((state) => state.profiles);
  const runs = useEvaluationStore((state) => state.runs);
  const testSets = useEvaluationStore((state) => state.testSets);
  const evaluators = useEvaluationStore((state) => state.evaluators);
  const matrices = useEvaluationStore((state) => state.matrices);
  const selectedMatrix = useEvaluationStore((state) => state.selectedMatrix);
  const rendered = useEvaluationStore((state) => state.rendered);
  const streamedOutput = useEvaluationStore((state) => state.streamedOutput);
  const playgroundRequestId = useEvaluationStore((state) => state.playgroundRequestId);
  const matrixRequestId = useEvaluationStore((state) => state.matrixRequestId);
  const progress = useEvaluationStore((state) => state.progress);
  const labels = useEvaluationStore((state) => state.labels);
  const labelHistory = useEvaluationStore((state) => state.labelHistory);
  const error = useEvaluationStore((state) => state.error);
  const load = useEvaluationStore((state) => state.load);
  const subscribe = useEvaluationStore((state) => state.subscribe);
  const saveProfile = useEvaluationStore((state) => state.saveProfile);
  const render = useEvaluationStore((state) => state.render);
  const run = useEvaluationStore((state) => state.run);
  const cancel = useEvaluationStore((state) => state.cancel);
  const saveTestSet = useEvaluationStore((state) => state.saveTestSet);
  const importTestSet = useEvaluationStore((state) => state.importTestSet);
  const exportTestSet = useEvaluationStore((state) => state.exportTestSet);
  const createEvaluator = useEvaluationStore((state) => state.createEvaluator);
  const runMatrix = useEvaluationStore((state) => state.runMatrix);
  const retryMatrix = useEvaluationStore((state) => state.retryMatrix);
  const selectMatrix = useEvaluationStore((state) => state.selectMatrix);
  const setManualResult = useEvaluationStore((state) => state.setManualResult);
  const loadLabels = useEvaluationStore((state) => state.loadLabels);
  const moveLabel = useEvaluationStore((state) => state.moveLabel);

  useEffect(() => {
    void load();
    return subscribe();
  }, [load, subscribe]);

  useEffect(() => {
    void loadLabels(prompt.id);
  }, [loadLabels, prompt.id]);

  useEffect(() => {
    const latest = versions[versions.length - 1]?.id ?? "";
    setRevisionId(latest);
    setSelectedRevisions(latest ? [latest] : []);
  }, [prompt.id, versions]);

  useEffect(() => {
    if (!profileId && profiles[0]) setProfileId(profiles[0].id);
    if (selectedProfiles.length === 0 && profiles[0]) {
      setSelectedProfiles([profiles[0].id]);
    }
  }, [profileId, profiles, selectedProfiles.length]);

  const activeVersion = versions.find((version) => version.id === revisionId);
  const runById = useMemo(() => new Map(runs.map((item) => [item.id, item])), [runs]);
  const filteredRuns = useMemo(
    () =>
      runs.filter(
        (item) =>
          (!historyStatus || item.status === historyStatus) &&
          (!historyProfile || item.profileRevisionId === historyProfile) &&
          (!historyRevision || item.promptRevisionId === historyRevision) &&
          (!historyDate || item.startedAt.slice(0, 10) === historyDate),
      ),
    [historyDate, historyProfile, historyRevision, historyStatus, runs],
  );

  const toggle = (items: string[], value: string, update: (next: string[]) => void) => {
    update(items.includes(value) ? items.filter((item) => item !== value) : [...items, value]);
  };

  const addCase = () =>
    setTestCases((cases) => [
      ...cases,
      { name: `${t("evaluation.case")} ${cases.length + 1}`, inputs: {}, annotations: {} },
    ]);

  const tabs: Array<[WorkbenchTab, typeof PlayIcon, string]> = [
    ["playground", PlayIcon, t("evaluation.playground")],
    ["matrix", FlaskConicalIcon, t("evaluation.matrix")],
    ["history", HistoryIcon, t("evaluation.history")],
  ];

  return (
    <div className="flex h-full min-w-0 flex-col bg-background">
      <div className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-4">
        <div
          className="flex items-center gap-1"
          role="tablist"
          aria-label={t("evaluation.title")}
        >
          {tabs.map(([value, Icon, label]) => {
            const active = tab === value;
            return (
              <button
                key={value}
                type="button"
                role="tab"
                aria-selected={active}
                onClick={() => setTab(value)}
                className={`-mb-px flex items-center gap-1.5 border-b-2 px-2.5 py-2.5 text-label transition-colors duration-fast ease-out ${
                  active
                    ? "border-primary font-medium text-foreground"
                    : "border-transparent text-muted-foreground hover:text-foreground"
                }`}
              >
                <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                {label}
              </button>
            );
          })}
        </div>
        <span className="min-w-0 truncate text-label text-muted-foreground">{prompt.title}</span>
      </div>

      {error && (
        <div
          role="alert"
          className="shrink-0 border-b border-destructive/40 bg-destructive/10 px-4 py-2 text-label text-destructive"
        >
          {error}
        </div>
      )}

      {tab === "playground" && (
        <div className="grid min-h-0 flex-1 grid-cols-1 overflow-y-auto lg:grid-cols-[minmax(18rem,0.9fr)_minmax(20rem,1.1fr)] lg:overflow-hidden">
          <PlaygroundPanel
            versions={versions}
            profiles={profiles}
            revisionId={revisionId}
            onRevisionChange={setRevisionId}
            profileId={profileId}
            onProfileChange={setProfileId}
            activeVersion={activeVersion}
            inputs={inputs}
            onInputChange={(name, value) =>
              setInputs((current) => ({ ...current, [name]: value }))
            }
            showProfileForm={showProfileForm}
            onToggleProfileForm={() => setShowProfileForm((open) => !open)}
            profileDraft={profileDraft}
            onProfileDraftChange={setProfileDraft}
            parametersJson={parametersJson}
            onParametersJsonChange={setParametersJson}
            parametersValid={parseObject(parametersJson) != null}
            onSaveProfile={() => {
              const parameters = parseObject(parametersJson);
              if (!parameters) return;
              void saveProfile({ ...profileDraft, parameters }).then((saved) => {
                if (saved) {
                  setProfileId(saved.id);
                  setShowProfileForm(false);
                }
              });
            }}
            onPreview={() => void render(revisionId, inputs)}
            onRun={() =>
              void run({
                promptRevisionId: revisionId,
                profileRevisionId: profileId,
                inputs,
              })
            }
            onCancel={() => void cancel("playground")}
            running={playgroundRequestId != null}
          />
          <OutputPane
            rendered={rendered}
            streamedOutput={streamedOutput}
            streaming={playgroundRequestId != null}
          />
        </div>
      )}

      {tab === "matrix" && (
        <div className="flex min-h-0 flex-1 flex-col">
          <MatrixSetupPanel
            versions={versions}
            profiles={profiles}
            testSets={testSets}
            evaluators={evaluators}
            testSetName={testSetName}
            onTestSetNameChange={setTestSetName}
            testCases={testCases}
            onAddCase={addCase}
            onUpdateCase={(index, next) =>
              setTestCases((cases) =>
                cases.map((item, position) => (position === index ? next : item)),
              )
            }
            onRemoveCase={(index) =>
              setTestCases((cases) => cases.filter((_, position) => position !== index))
            }
            onSaveTestSet={() => {
              void saveTestSet({ name: testSetName, cases: testCases }).then((saved) => {
                if (saved) setTestSetId(saved.id);
              });
            }}
            testSetId={testSetId}
            onTestSetIdChange={setTestSetId}
            testSetJson={testSetJson}
            onTestSetJsonChange={setTestSetJson}
            onImportTestSet={() => {
              void importTestSet(testSetJson).then((saved) => {
                if (saved) {
                  setTestSetId(saved.id);
                  setTestSetJson("");
                }
              });
            }}
            onExportTestSet={() => {
              void exportTestSet(testSetId).then((json) => {
                if (json) setTestSetJson(json);
              });
            }}
            selectedEvaluators={selectedEvaluators}
            onToggleEvaluator={(id) => toggle(selectedEvaluators, id, setSelectedEvaluators)}
            evaluatorDraft={evaluatorDraft}
            onEvaluatorDraftChange={setEvaluatorDraft}
            evaluatorConfigJson={evaluatorConfigJson}
            onEvaluatorConfigJsonChange={setEvaluatorConfigJson}
            evaluatorConfigValid={parseObject(evaluatorConfigJson) != null}
            onCreateEvaluator={() => {
              const config = parseObject(evaluatorConfigJson);
              if (!config) return;
              void createEvaluator({ ...evaluatorDraft, config });
            }}
            selectedRevisions={selectedRevisions}
            onToggleRevision={(id) => toggle(selectedRevisions, id, setSelectedRevisions)}
            selectedProfiles={selectedProfiles}
            onToggleProfile={(id) => toggle(selectedProfiles, id, setSelectedProfiles)}
            onRunMatrix={() =>
              void runMatrix({
                testSetId,
                promptRevisionIds: selectedRevisions,
                profileRevisionIds: selectedProfiles,
                evaluatorIds: selectedEvaluators,
              })
            }
            onCancelMatrix={() => void cancel("matrix")}
            matrixRunning={matrixRequestId != null}
            progress={progress}
          />

          <div className="grid min-h-0 flex-1 grid-cols-1 overflow-y-auto lg:grid-cols-[minmax(0,1fr)_20rem] lg:overflow-hidden">
            <MatrixGrid
              matrix={selectedMatrix}
              testSet={testSets.find((set) => set.id === testSetId)}
              versions={versions}
              profiles={profiles}
              compareCells={compareCells}
              onToggleCompare={(cellId) =>
                setCompareCells((items) =>
                  items.includes(cellId)
                    ? items.filter((id) => id !== cellId)
                    : [...items.slice(-1), cellId],
                )
              }
            />
            <ComparePanel
              cells={(selectedMatrix?.cells ?? []).filter((cell) =>
                compareCells.includes(cell.id),
              )}
              runById={runById}
              prompt={prompt}
              labels={labels}
              labelHistory={labelHistory}
              onManual={(cellId, evaluatorId, passed) =>
                void setManualResult(cellId, evaluatorId, passed, "Manual review")
              }
              onLabel={(label, revision, rollback) =>
                void moveLabel(prompt.id, label, revision, rollback)
              }
            />
          </div>
        </div>
      )}

      {tab === "history" && (
        <HistoryPanel
          runs={filteredRuns}
          versions={versions}
          profiles={profiles}
          matrices={matrices}
          selectedMatrix={selectedMatrix}
          status={historyStatus}
          onStatusChange={setHistoryStatus}
          profileId={historyProfile}
          onProfileChange={setHistoryProfile}
          revisionId={historyRevision}
          onRevisionChange={setHistoryRevision}
          date={historyDate}
          onDateChange={setHistoryDate}
          onSelectMatrix={(id) => void selectMatrix(id)}
          onRetryMatrix={(id) => void retryMatrix(id)}
        />
      )}
    </div>
  );
}
