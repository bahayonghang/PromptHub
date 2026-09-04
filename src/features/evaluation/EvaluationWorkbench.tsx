import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CheckIcon,
  FlaskConicalIcon,
  HistoryIcon,
  PlayIcon,
  PlusIcon,
  RefreshCwIcon,
  SquareIcon,
  TagIcon,
  XIcon,
} from "lucide-react";
import type { Prompt, PromptVersion } from "../prompts/types";
import { useEvaluationStore } from "./evaluationStore";
import type {
  EvaluationCell,
  EvaluatorInput,
  ExecutionProfileInput,
  PromptLabel,
  TestCaseInput,
} from "./types";

import { Select } from "../../components/ui";
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

const inputClass =
  "w-full rounded-sm border border-input bg-background px-2 py-1.5 text-xs text-foreground outline-none focus:ring-1 focus:ring-ring";

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
  const runById = useMemo(
    () => new Map(runs.map((item) => [item.id, item])),
    [runs],
  );
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
      <div className="flex shrink-0 items-center justify-between border-b border-border px-4 py-2">
        <div className="flex items-center gap-1" role="tablist" aria-label={t("evaluation.title")}>
          {tabs.map(([value, Icon, label]) => (
            <button
              key={value}
              type="button"
              role="tab"
              aria-selected={tab === value}
              onClick={() => setTab(value)}
              className={`flex items-center gap-1.5 rounded-sm px-2.5 py-1.5 text-xs ${
                tab === value ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Icon className="h-3.5 w-3.5" aria-hidden="true" />
              {label}
            </button>
          ))}
        </div>
        <span className="min-w-0 truncate text-xs text-muted-foreground">{prompt.title}</span>
      </div>

      {error && (
        <div role="alert" className="border-b border-destructive/40 px-4 py-2 text-xs text-destructive">
          {error}
        </div>
      )}

      {tab === "playground" && (
        <div className="grid min-h-0 flex-1 grid-cols-1 overflow-y-auto lg:grid-cols-[minmax(18rem,0.9fr)_minmax(20rem,1.1fr)] lg:overflow-hidden">
          <section className="overflow-y-auto border-r border-border p-4">
            <div className="grid grid-cols-2 gap-3">
              <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                {t("evaluation.revision")}
                <Select
                  value={revisionId}
                  onChange={(event) => setRevisionId(event.target.value)}
                  wrapperClassName={inputClass}
                >
                  {versions.map((version) => (
                    <option key={version.id} value={version.id}>v{version.version}</option>
                  ))}
                </Select>
              </label>
              <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                {t("evaluation.profile")}
                <Select
                  value={profileId}
                  onChange={(event) => setProfileId(event.target.value)}
                  wrapperClassName={inputClass}
                >
                  <option value="">{t("evaluation.noProfile")}</option>
                  {profiles.map((profile) => (
                    <option key={profile.id} value={profile.id}>{profile.name} r{profile.revision}</option>
                  ))}
                </Select>
              </label>
            </div>
            <button
              type="button"
              onClick={() => setShowProfileForm((open) => !open)}
              className="mt-2 flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
            >
              <PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
              {t("evaluation.newProfile")}
            </button>
            {showProfileForm && (
              <div className="mt-3 grid gap-2 border-y border-border py-3">
                <input value={profileDraft.name} onChange={(event) => setProfileDraft({ ...profileDraft, name: event.target.value })} placeholder={t("evaluation.profileName")} aria-label={t("evaluation.profileName")} className={inputClass} />
                <div className="grid grid-cols-2 gap-2">
                  <Select
                    value={profileDraft.provider}
                    onChange={(event) => setProfileDraft({ ...profileDraft, provider: event.target.value as ExecutionProfileInput["provider"] })}
                    aria-label={t("evaluation.provider")}
                    wrapperClassName={inputClass}
                  >
                    <option value="mock">mock</option>
                    <option value="openai-compatible">openai-compatible</option>
                  </Select>
                  <input value={profileDraft.model} onChange={(event) => setProfileDraft({ ...profileDraft, model: event.target.value })} placeholder={t("evaluation.model")} aria-label={t("evaluation.model")} className={inputClass} />
                </div>
                {profileDraft.provider === "openai-compatible" && (
                  <>
                    <input value={profileDraft.endpoint ?? ""} onChange={(event) => setProfileDraft({ ...profileDraft, endpoint: event.target.value })} placeholder={t("evaluation.endpoint")} aria-label={t("evaluation.endpoint")} className={inputClass} />
                    <input type="password" value={profileDraft.credential ?? ""} onChange={(event) => setProfileDraft({ ...profileDraft, credential: event.target.value })} placeholder={t("evaluation.credential")} aria-label={t("evaluation.credential")} className={inputClass} />
                  </>
                )}
                <textarea value={parametersJson} onChange={(event) => setParametersJson(event.target.value)} aria-label={t("evaluation.parameters")} rows={3} className={`${inputClass} font-mono`} />
                <button type="button" disabled={!parseObject(parametersJson) || !profileDraft.name.trim()} onClick={() => { const parameters = parseObject(parametersJson); if (parameters) void saveProfile({ ...profileDraft, parameters }).then((saved) => { if (saved) { setProfileId(saved.id); setShowProfileForm(false); } }); }} className="w-fit rounded-sm bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-50">
                  {t("common.save")}
                </button>
              </div>
            )}

            <div className="mt-4 flex flex-col gap-2">
              <h3 className="text-xs font-semibold text-foreground">{t("evaluation.variables")}</h3>
              {activeVersion?.variables.map((variable) => (
                <label key={variable.name} className="flex flex-col gap-1 text-xs text-muted-foreground">
                  {variable.label || variable.name}{variable.required ? " *" : ""}
                  <input value={inputs[variable.name] ?? variable.defaultValue ?? ""} onChange={(event) => setInputs((current) => ({ ...current, [variable.name]: event.target.value }))} className={inputClass} />
                </label>
              ))}
              {activeVersion?.variables.length === 0 && <span className="text-xs text-muted-foreground">{t("evaluation.noVariables")}</span>}
            </div>
            <div className="mt-4 flex gap-2">
              <button type="button" disabled={!revisionId} onClick={() => void render(revisionId, inputs)} className="rounded-sm border border-input px-3 py-1.5 text-xs text-foreground hover:bg-accent disabled:opacity-50">
                {t("evaluation.preview")}
              </button>
              {playgroundRequestId ? (
                <button type="button" onClick={() => void cancel("playground")} className="flex items-center gap-1.5 rounded-sm border border-input px-3 py-1.5 text-xs text-foreground hover:bg-accent">
                  <SquareIcon className="h-3.5 w-3.5" aria-hidden="true" />{t("common.cancel")}
                </button>
              ) : (
                <button type="button" disabled={!revisionId || !profileId} onClick={() => void run({ promptRevisionId: revisionId, profileRevisionId: profileId, inputs })} className="flex items-center gap-1.5 rounded-sm bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-50">
                  <PlayIcon className="h-3.5 w-3.5" aria-hidden="true" />{t("evaluation.run")}
                </button>
              )}
            </div>
          </section>

          <section className="grid min-h-0 grid-rows-2">
            <div className="overflow-y-auto border-b border-border p-4">
              <h3 className="mb-2 text-xs font-semibold text-foreground">{t("evaluation.renderedInput")}</h3>
              {rendered?.messages.map((message, index) => (
                <div key={index} className="mb-2 grid grid-cols-[5rem_minmax(0,1fr)] gap-2 text-xs">
                  <span className="font-medium text-muted-foreground">{message.role}</span>
                  <pre className="whitespace-pre-wrap text-foreground">{message.content}</pre>
                </div>
              )) ?? <span className="text-xs text-muted-foreground">{t("evaluation.previewEmpty")}</span>}
            </div>
            <div className="overflow-y-auto p-4" aria-live="polite">
              <h3 className="mb-2 text-xs font-semibold text-foreground">{t("evaluation.output")}</h3>
              <pre className="whitespace-pre-wrap text-sm text-foreground">{streamedOutput || t("evaluation.outputEmpty")}</pre>
            </div>
          </section>
        </div>
      )}

      {tab === "matrix" && (
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <div className="grid max-h-80 shrink-0 grid-cols-1 overflow-y-auto border-b border-border lg:max-h-none lg:grid-cols-3 lg:overflow-visible">
            <section className="border-r border-border p-3">
              <div className="flex items-center justify-between">
                <h3 className="text-xs font-semibold">{t("evaluation.testSet")}</h3>
                <button type="button" onClick={addCase} aria-label={t("evaluation.addCase")} title={t("evaluation.addCase")} className="rounded-sm p-1 text-muted-foreground hover:bg-accent"><PlusIcon className="h-3.5 w-3.5" /></button>
              </div>
              <input value={testSetName} onChange={(event) => setTestSetName(event.target.value)} placeholder={t("evaluation.testSetName")} aria-label={t("evaluation.testSetName")} className={`${inputClass} mt-2`} />
              <div className="mt-2 max-h-32 space-y-2 overflow-y-auto">
                {testCases.map((testCase, index) => (
                  <div key={index} className="grid grid-cols-[1fr_1.4fr_auto] gap-1">
                    <input value={testCase.name} onChange={(event) => setTestCases((cases) => cases.map((item, itemIndex) => itemIndex === index ? { ...item, name: event.target.value } : item))} aria-label={t("evaluation.caseName")} className={inputClass} />
                    <input value={JSON.stringify(testCase.inputs)} onChange={(event) => { const value = parseObject(event.target.value); if (value) setTestCases((cases) => cases.map((item, itemIndex) => itemIndex === index ? { ...item, inputs: Object.fromEntries(Object.entries(value).map(([key, itemValue]) => [key, String(itemValue)])) } : item)); }} aria-label={t("evaluation.caseInputs")} className={`${inputClass} font-mono`} />
                    <button type="button" onClick={() => setTestCases((cases) => cases.filter((_, itemIndex) => itemIndex !== index))} aria-label={t("evaluation.removeCase")} title={t("evaluation.removeCase")} className="rounded-sm p-1 text-muted-foreground hover:bg-accent"><XIcon className="h-3.5 w-3.5" /></button>
                  </div>
                ))}
              </div>
              <div className="mt-2 flex gap-1">
                <button type="button" disabled={!testSetName.trim() || testCases.length === 0} onClick={() => void saveTestSet({ name: testSetName, cases: testCases }).then((saved) => saved && setTestSetId(saved.id))} className="rounded-sm bg-primary px-2 py-1 text-xs text-primary-foreground disabled:opacity-50">{t("common.save")}</button>
                <Select
                  value={testSetId}
                  onChange={(event) => setTestSetId(event.target.value)}
                  aria-label={t("evaluation.testSet")}
                  wrapperClassName={`${inputClass} min-w-0`}
                >
                  <option value="">{t("evaluation.selectTestSet")}</option>
                  {testSets.map((set) => <option key={set.id} value={set.id}>{set.name} ({set.cases.length})</option>)}
                </Select>
              </div>
              <textarea value={testSetJson} onChange={(event) => setTestSetJson(event.target.value)} placeholder={t("evaluation.testSetJson")} aria-label={t("evaluation.testSetJson")} rows={2} className={`${inputClass} mt-2 font-mono`} />
              <div className="mt-1 flex gap-1">
                <button type="button" disabled={!testSetJson.trim()} onClick={() => void importTestSet(testSetJson).then((saved) => saved && setTestSetId(saved.id))} className="text-xs text-muted-foreground hover:text-foreground disabled:opacity-50">{t("evaluation.import")}</button>
                <button type="button" disabled={!testSetId} onClick={() => void exportTestSet(testSetId).then((value) => value && setTestSetJson(value))} className="text-xs text-muted-foreground hover:text-foreground disabled:opacity-50">{t("evaluation.export")}</button>
              </div>
            </section>

            <section className="border-r border-border p-3">
              <h3 className="text-xs font-semibold">{t("evaluation.evaluators")}</h3>
              <div className="mt-2 flex max-h-20 flex-wrap gap-1 overflow-y-auto">
                {evaluators.map((evaluator) => (
                  <label key={evaluator.id} className="flex items-center gap-1 text-xs text-muted-foreground">
                    <input type="checkbox" checked={selectedEvaluators.includes(evaluator.id)} onChange={() => toggle(selectedEvaluators, evaluator.id, setSelectedEvaluators)} />{evaluator.name}
                  </label>
                ))}
              </div>
              <div className="mt-2 grid grid-cols-1 gap-1 xl:grid-cols-2">
                <input value={evaluatorDraft.name} onChange={(event) => setEvaluatorDraft({ ...evaluatorDraft, name: event.target.value })} placeholder={t("evaluation.evaluatorName")} aria-label={t("evaluation.evaluatorName")} className={inputClass} />
                <Select
                  value={evaluatorDraft.kind}
                  onChange={(event) => setEvaluatorDraft({ ...evaluatorDraft, kind: event.target.value as EvaluatorInput["kind"] })}
                  aria-label={t("evaluation.evaluatorKind")}
                  wrapperClassName={inputClass}
                >
                  {(["manual", "exact", "contains", "regex", "numeric"] as const).map((kind) => <option key={kind} value={kind}>{kind}</option>)}
                </Select>
              </div>
              <textarea value={evaluatorConfigJson} onChange={(event) => setEvaluatorConfigJson(event.target.value)} aria-label={t("evaluation.evaluatorConfig")} rows={2} className={`${inputClass} mt-1 font-mono`} />
              <button type="button" disabled={!evaluatorDraft.name.trim() || !parseObject(evaluatorConfigJson)} onClick={() => { const config = parseObject(evaluatorConfigJson); if (config) void createEvaluator({ ...evaluatorDraft, config }).then((saved) => saved && setSelectedEvaluators((ids) => [...ids, saved.id])); }} className="mt-1 rounded-sm border border-input px-2 py-1 text-xs text-foreground hover:bg-accent disabled:opacity-50">{t("evaluation.addEvaluator")}</button>
            </section>

            <section className="p-3">
              <h3 className="text-xs font-semibold">{t("evaluation.matrixScope")}</h3>
              <div className="mt-2 grid grid-cols-1 gap-2 xl:grid-cols-2">
                <div className="max-h-24 overflow-y-auto">
                  {versions.map((version) => <label key={version.id} className="flex items-center gap-1 text-xs text-muted-foreground"><input type="checkbox" checked={selectedRevisions.includes(version.id)} onChange={() => toggle(selectedRevisions, version.id, setSelectedRevisions)} />v{version.version}</label>)}
                </div>
                <div className="max-h-24 overflow-y-auto">
                  {profiles.map((profile) => <label key={profile.id} className="flex items-center gap-1 text-xs text-muted-foreground"><input type="checkbox" checked={selectedProfiles.includes(profile.id)} onChange={() => toggle(selectedProfiles, profile.id, setSelectedProfiles)} />{profile.name}</label>)}
                </div>
              </div>
              <div className="mt-3 flex items-center gap-2">
                {matrixRequestId ? <button type="button" onClick={() => void cancel("matrix")} className="flex items-center gap-1 rounded-sm border border-input px-2 py-1 text-xs"><SquareIcon className="h-3 w-3" />{t("common.cancel")}</button> : <button type="button" disabled={!testSetId || selectedRevisions.length === 0 || selectedProfiles.length === 0 || selectedEvaluators.length === 0} onClick={() => void runMatrix({ testSetId, promptRevisionIds: selectedRevisions, profileRevisionIds: selectedProfiles, evaluatorIds: selectedEvaluators })} className="flex items-center gap-1 rounded-sm bg-primary px-2 py-1 text-xs text-primary-foreground disabled:opacity-50"><PlayIcon className="h-3 w-3" />{t("evaluation.runMatrix")}</button>}
                {progress && <span aria-live="polite" className="text-xs tabular-nums text-muted-foreground">{progress.completed}/{progress.total}</span>}
              </div>
            </section>
          </div>

          <div className="grid min-h-0 flex-1 grid-cols-1 overflow-y-auto lg:grid-cols-[minmax(0,1fr)_22rem] lg:overflow-hidden">
            <div className="overflow-auto">
              <table className="w-full table-fixed border-collapse text-xs">
                <thead className="sticky top-0 bg-background"><tr><th className="w-24 border-b border-r border-border p-2 text-left">{t("evaluation.case")}</th><th className="border-b border-border p-2 text-left">{t("evaluation.cells")}</th></tr></thead>
                <tbody>
                  {(selectedMatrix?.cells ?? []).map((cell) => (
                    <tr key={cell.id}>
                      <td className="border-b border-r border-border p-2 text-muted-foreground">{testSets.flatMap((set) => set.cases).find((item) => item.id === cell.testCaseId)?.name ?? cell.testCaseId.slice(0, 8)}</td>
                      <td className="border-b border-border p-1">
                        <button type="button" aria-pressed={compareCells.includes(cell.id)} onClick={() => setCompareCells((items) => items.includes(cell.id) ? items.filter((id) => id !== cell.id) : [...items.slice(-1), cell.id])} className={`flex w-full items-center justify-between rounded-sm border px-2 py-1.5 text-left ${compareCells.includes(cell.id) ? "border-primary bg-primary/10" : "border-input hover:bg-accent"}`}>
                          <span className="truncate">{profiles.find((profile) => profile.id === cell.profileRevisionId)?.name ?? cell.profileRevisionId.slice(0, 8)} / v{versions.find((version) => version.id === cell.promptRevisionId)?.version ?? "?"}</span>
                          <span className="ml-2 shrink-0 tabular-nums">{cell.cacheHit ? `${t("evaluation.cached")} ` : ""}{cell.status}</span>
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {!selectedMatrix && <div className="p-6 text-center text-xs text-muted-foreground">{t("evaluation.matrixEmpty")}</div>}
            </div>
            <ComparePanel cells={(selectedMatrix?.cells ?? []).filter((cell) => compareCells.includes(cell.id))} runById={runById} prompt={prompt} labels={labels} labelHistory={labelHistory} onManual={(cellId, evaluatorId, passed) => void setManualResult(cellId, evaluatorId, passed, "Manual review")} onLabel={(label, revision, rollback) => void moveLabel(prompt.id, label, revision, rollback)} />
          </div>
        </div>
      )}

      {tab === "history" && (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="grid shrink-0 grid-cols-1 gap-2 border-b border-border p-3 sm:grid-cols-2 lg:grid-cols-4">
            <Select
              value={historyStatus}
              onChange={(event) => setHistoryStatus(event.target.value)}
              aria-label={t("evaluation.status")}
              wrapperClassName={inputClass}
            ><option value="">{t("evaluation.allStatuses")}</option>{["success", "error", "cancelled"].map((status) => <option key={status}>{status}</option>)}            </Select>
            <Select
              value={historyProfile}
              onChange={(event) => setHistoryProfile(event.target.value)}
              aria-label={t("evaluation.profile")}
              wrapperClassName={inputClass}
            ><option value="">{t("evaluation.allProfiles")}</option>{profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}            </Select>
            <Select
              value={historyRevision}
              onChange={(event) => setHistoryRevision(event.target.value)}
              aria-label={t("evaluation.revision")}
              wrapperClassName={inputClass}
            ><option value="">{t("evaluation.allRevisions")}</option>{versions.map((version) => <option key={version.id} value={version.id}>v{version.version}</option>)}            </Select>
            <input type="date" value={historyDate} onChange={(event) => setHistoryDate(event.target.value)} aria-label={t("evaluation.date")} className={inputClass} />
          </div>
          <div className="grid min-h-0 flex-1 grid-cols-1 overflow-y-auto lg:grid-cols-[minmax(0,1fr)_18rem] lg:overflow-hidden">
            <div className="overflow-auto">
              <table className="w-full text-xs"><thead><tr className="border-b border-border text-left text-muted-foreground"><th className="p-2">{t("evaluation.date")}</th><th className="p-2">{t("evaluation.revision")}</th><th className="p-2">{t("evaluation.profile")}</th><th className="p-2">{t("evaluation.status")}</th><th className="p-2">{t("evaluation.duration")}</th><th className="p-2">{t("evaluation.output")}</th></tr></thead><tbody>{filteredRuns.map((item) => <tr key={item.id} className="border-b border-border"><td className="p-2 tabular-nums">{new Date(item.startedAt).toLocaleString()}</td><td className="p-2">v{versions.find((version) => version.id === item.promptRevisionId)?.version ?? "?"}</td><td className="p-2">{profiles.find((profile) => profile.id === item.profileRevisionId)?.name ?? "-"}</td><td className="p-2">{item.status}</td><td className="p-2 tabular-nums">{item.durationMs ?? "-"} ms</td><td className="max-w-xs truncate p-2">{item.output ?? item.error ?? "-"}</td></tr>)}</tbody></table>
            </div>
            <aside className="overflow-y-auto border-t border-border p-3 lg:border-l lg:border-t-0">
              <div className="flex items-center justify-between"><h3 className="text-xs font-semibold">{t("evaluation.matrixRuns")}</h3>{selectedMatrix && <button type="button" onClick={() => void retryMatrix(selectedMatrix.run.id)} aria-label={t("evaluation.retry")} title={t("evaluation.retry")} className="rounded-sm p-1 text-muted-foreground hover:bg-accent"><RefreshCwIcon className="h-3.5 w-3.5" /></button>}</div>
              {matrices.map((matrix) => <button key={matrix.id} type="button" onClick={() => void selectMatrix(matrix.id)} className="mt-2 block w-full rounded-sm border border-input p-2 text-left text-xs hover:bg-accent"><span className="block text-foreground">{matrix.status} · {matrix.completedCells}/{matrix.totalCells}</span><span className="text-muted-foreground">{new Date(matrix.startedAt).toLocaleString()}</span></button>)}
            </aside>
          </div>
        </div>
      )}
    </div>
  );
}

interface ComparePanelProps {
  cells: EvaluationCell[];
  runById: Map<string, { output?: string | null }>;
  prompt: Prompt;
  labels: PromptLabel[];
  labelHistory: Array<{ id: string; label: PromptLabel["label"]; fromRevisionId?: string | null; toRevisionId: string; action: "move" | "rollback"; createdAt: string }>;
  onManual: (cellId: string, evaluatorId: string, passed: boolean) => void;
  onLabel: (label: PromptLabel["label"], revisionId: string, rollback: boolean) => void;
}

function ComparePanel({ cells, runById, prompt, labels, labelHistory, onManual, onLabel }: ComparePanelProps) {
  const { t } = useTranslation();
  return (
    <aside className="overflow-y-auto border-t border-border p-3 lg:border-l lg:border-t-0">
      <h3 className="text-xs font-semibold">{t("evaluation.compare")}</h3>
      {cells.length === 0 && <p className="mt-2 text-xs text-muted-foreground">{t("evaluation.compareEmpty")}</p>}
      <div className={`mt-2 grid gap-2 ${cells.length === 2 ? "grid-cols-2" : "grid-cols-1"}`}>
        {cells.map((cell) => (
          <section key={cell.id} className="min-w-0 border-t border-border pt-2">
            <div className="flex items-center justify-between text-xs"><span className="font-medium">{cell.status}</span>{cell.cacheHit && <span className="text-muted-foreground">{t("evaluation.cached")}</span>}</div>
            <pre className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap text-xs text-foreground">{cell.promptRunId ? runById.get(cell.promptRunId)?.output ?? cell.error ?? "-" : cell.error ?? "-"}</pre>
            <div className="mt-2 space-y-2">
              {cell.results.map((result) => (
                <div key={result.evaluatorId} className="border-t border-border pt-2 text-xs">
                  <div className="flex items-center gap-1">{result.passed === true ? <CheckIcon className="h-3.5 w-3.5" /> : result.passed === false ? <XIcon className="h-3.5 w-3.5" /> : null}<span>{result.kind}</span></div>
                  <p className="text-muted-foreground">{result.evidence}</p>
                  {result.kind === "manual" && result.skipped && <div className="mt-1 flex gap-1"><button type="button" onClick={() => onManual(cell.id, result.evaluatorId, true)} className="rounded-sm border border-input px-1.5 py-0.5">{t("evaluation.pass")}</button><button type="button" onClick={() => onManual(cell.id, result.evaluatorId, false)} className="rounded-sm border border-input px-1.5 py-0.5">{t("evaluation.fail")}</button></div>}
                </div>
              ))}
            </div>
            {cell.status === "success" && (
              <div className="mt-2 flex flex-wrap gap-1">
                {(["baseline", "candidate"] as const).map((label) => <button key={label} type="button" onClick={() => onLabel(label, cell.promptRevisionId, false)} className="flex items-center gap-1 rounded-sm border border-input px-1.5 py-0.5 text-xs"><TagIcon className="h-3 w-3" />{label}</button>)}
              </div>
            )}
          </section>
        ))}
      </div>
      <div className="mt-4 border-t border-border pt-3">
        <h4 className="text-xs font-semibold">{t("evaluation.labels")}</h4>
        {labels.map((label) => <div key={label.label} className="mt-1 flex items-center justify-between text-xs"><span>{label.label}: {label.promptRevisionId.slice(0, 8)}</span></div>)}
        {labelHistory.slice(0, 5).map((item) => <button key={item.id} type="button" disabled={!item.fromRevisionId} onClick={() => item.fromRevisionId && onLabel(item.label, item.fromRevisionId, true)} className="mt-1 block text-left text-xs text-muted-foreground hover:text-foreground disabled:opacity-50">{new Date(item.createdAt).toLocaleString()} · {item.action ?? "move"} · {item.label}</button>)}
        {labels.length === 0 && <span className="text-xs text-muted-foreground">{prompt.title}: {t("evaluation.noLabels")}</span>}
      </div>
    </aside>
  );
}
