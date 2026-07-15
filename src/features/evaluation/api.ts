import { runtime, type RuntimeBridge, type UnsubscribeFn } from "../../runtime";
import type {
  EvaluationCell,
  EvaluationMatrixInput,
  EvaluationRun,
  EvaluationRunDetail,
  EvaluatorConfig,
  EvaluatorInput,
  ExecutionProfileInput,
  ExecutionProfileRevision,
  MatrixProgressEvent,
  PromptLabel,
  PromptLabelHistory,
  PromptRun,
  PromptRunInput,
  RenderedPrompt,
  RunChunkEvent,
  TestSet,
  TestSetInput,
} from "./types";

export interface EvaluationApi {
  listProfiles(): Promise<ExecutionProfileRevision[]>;
  saveProfile(input: ExecutionProfileInput): Promise<ExecutionProfileRevision>;
  render(promptRevisionId: string, inputs: Record<string, string>): Promise<RenderedPrompt>;
  run(requestId: string, input: PromptRunInput): Promise<PromptRun>;
  cancel(requestId: string): Promise<void>;
  listRuns(): Promise<PromptRun[]>;
  getRun(id: string): Promise<PromptRun>;
  listTestSets(): Promise<TestSet[]>;
  saveTestSet(input: TestSetInput): Promise<TestSet>;
  exportTestSet(id: string): Promise<string>;
  importTestSet(json: string): Promise<TestSet>;
  listEvaluators(): Promise<EvaluatorConfig[]>;
  createEvaluator(input: EvaluatorInput): Promise<EvaluatorConfig>;
  runMatrix(
    requestId: string,
    input: EvaluationMatrixInput,
  ): Promise<EvaluationRunDetail>;
  retryMatrix(requestId: string, id: string): Promise<EvaluationRunDetail>;
  listMatrices(): Promise<EvaluationRun[]>;
  getMatrix(id: string): Promise<EvaluationRunDetail>;
  setManualResult(
    cellId: string,
    evaluatorId: string,
    passed: boolean,
    evidence: string,
  ): Promise<EvaluationCell>;
  listLabels(promptId: string): Promise<PromptLabel[]>;
  moveLabel(
    promptId: string,
    label: PromptLabel["label"],
    promptRevisionId: string,
  ): Promise<PromptLabel>;
  rollbackLabel(
    promptId: string,
    label: PromptLabel["label"],
    promptRevisionId: string,
  ): Promise<PromptLabel>;
  labelHistory(promptId: string): Promise<PromptLabelHistory[]>;
  onRunChunk(handler: (event: RunChunkEvent) => void): UnsubscribeFn;
  onMatrixProgress(handler: (event: MatrixProgressEvent) => void): UnsubscribeFn;
}

export function createEvaluationApi(bridge: RuntimeBridge = runtime): EvaluationApi {
  return {
    listProfiles: () =>
      bridge.invoke<ExecutionProfileRevision[]>("evaluation.profileList"),
    saveProfile: (input) =>
      bridge.invoke<ExecutionProfileRevision>("evaluation.profileSave", { input }),
    render: (promptRevisionId, inputs) =>
      bridge.invoke<RenderedPrompt>("evaluation.render", { promptRevisionId, inputs }),
    run: (requestId, input) =>
      bridge.invoke<PromptRun>("evaluation.run", { requestId, input }),
    cancel: (requestId) =>
      bridge.invoke<void>("evaluation.cancel", { requestId }),
    listRuns: () => bridge.invoke<PromptRun[]>("evaluation.runList"),
    getRun: (id) => bridge.invoke<PromptRun>("evaluation.runGet", { id }),
    listTestSets: () => bridge.invoke<TestSet[]>("evaluation.testSetList"),
    saveTestSet: (input) =>
      bridge.invoke<TestSet>("evaluation.testSetSave", { input }),
    exportTestSet: (id) =>
      bridge.invoke<string>("evaluation.testSetExport", { id }),
    importTestSet: (json) =>
      bridge.invoke<TestSet>("evaluation.testSetImport", { json }),
    listEvaluators: () =>
      bridge.invoke<EvaluatorConfig[]>("evaluation.evaluatorList"),
    createEvaluator: (input) =>
      bridge.invoke<EvaluatorConfig>("evaluation.evaluatorCreate", { input }),
    runMatrix: (requestId, input) =>
      bridge.invoke<EvaluationRunDetail>("evaluation.matrixRun", { requestId, input }),
    retryMatrix: (requestId, id) =>
      bridge.invoke<EvaluationRunDetail>("evaluation.matrixRetry", { requestId, id }),
    listMatrices: () =>
      bridge.invoke<EvaluationRun[]>("evaluation.matrixList"),
    getMatrix: (id) =>
      bridge.invoke<EvaluationRunDetail>("evaluation.matrixGet", { id }),
    setManualResult: (cellId, evaluatorId, passed, evidence) =>
      bridge.invoke<EvaluationCell>("evaluation.manualResult", {
        cellId,
        evaluatorId,
        passed,
        evidence,
      }),
    listLabels: (promptId) =>
      bridge.invoke<PromptLabel[]>("evaluation.labelList", { promptId }),
    moveLabel: (promptId, label, promptRevisionId) =>
      bridge.invoke<PromptLabel>("evaluation.labelMove", {
        promptId,
        label,
        promptRevisionId,
      }),
    rollbackLabel: (promptId, label, promptRevisionId) =>
      bridge.invoke<PromptLabel>("evaluation.labelRollback", {
        promptId,
        label,
        promptRevisionId,
      }),
    labelHistory: (promptId) =>
      bridge.invoke<PromptLabelHistory[]>("evaluation.labelHistory", { promptId }),
    onRunChunk: (handler) => bridge.on<RunChunkEvent>("evaluation:run-chunk", handler),
    onMatrixProgress: (handler) =>
      bridge.on<MatrixProgressEvent>("evaluation:matrix-progress", handler),
  };
}

export const evaluationApi = createEvaluationApi();
