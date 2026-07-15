import { create } from "zustand";
import { evaluationApi, type EvaluationApi } from "./api";
import type {
  EvaluationMatrixInput,
  EvaluationRun,
  EvaluationRunDetail,
  EvaluatorConfig,
  EvaluatorInput,
  ExecutionProfileInput,
  ExecutionProfileRevision,
  PromptLabel,
  PromptLabelHistory,
  PromptRun,
  PromptRunInput,
  RenderedPrompt,
  TestSet,
  TestSetInput,
} from "./types";

function errorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

function requestId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `evaluation-${Date.now()}`;
}

interface EvaluationStoreState {
  api: EvaluationApi;
  profiles: ExecutionProfileRevision[];
  runs: PromptRun[];
  testSets: TestSet[];
  evaluators: EvaluatorConfig[];
  matrices: EvaluationRun[];
  selectedMatrix: EvaluationRunDetail | null;
  rendered: RenderedPrompt | null;
  streamedOutput: string;
  activeRequestId: string | null;
  progress: { completed: number; total: number } | null;
  labels: PromptLabel[];
  labelHistory: PromptLabelHistory[];
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  subscribe: () => () => void;
  saveProfile: (input: ExecutionProfileInput) => Promise<ExecutionProfileRevision | null>;
  render: (revisionId: string, inputs: Record<string, string>) => Promise<void>;
  run: (input: PromptRunInput) => Promise<PromptRun | null>;
  cancel: () => Promise<void>;
  saveTestSet: (input: TestSetInput) => Promise<TestSet | null>;
  importTestSet: (json: string) => Promise<TestSet | null>;
  exportTestSet: (id: string) => Promise<string | null>;
  createEvaluator: (input: EvaluatorInput) => Promise<EvaluatorConfig | null>;
  runMatrix: (input: EvaluationMatrixInput) => Promise<EvaluationRunDetail | null>;
  retryMatrix: (id: string) => Promise<EvaluationRunDetail | null>;
  selectMatrix: (id: string) => Promise<void>;
  setManualResult: (
    cellId: string,
    evaluatorId: string,
    passed: boolean,
    evidence: string,
  ) => Promise<void>;
  loadLabels: (promptId: string) => Promise<void>;
  moveLabel: (
    promptId: string,
    label: PromptLabel["label"],
    revisionId: string,
    rollback?: boolean,
  ) => Promise<void>;
}

export const useEvaluationStore = create<EvaluationStoreState>((set, get) => ({
  api: evaluationApi,
  profiles: [],
  runs: [],
  testSets: [],
  evaluators: [],
  matrices: [],
  selectedMatrix: null,
  rendered: null,
  streamedOutput: "",
  activeRequestId: null,
  progress: null,
  labels: [],
  labelHistory: [],
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const [profiles, runs, testSets, evaluators, matrices] = await Promise.all([
        get().api.listProfiles(),
        get().api.listRuns(),
        get().api.listTestSets(),
        get().api.listEvaluators(),
        get().api.listMatrices(),
      ]);
      set({ profiles, runs, testSets, evaluators, matrices, loading: false });
    } catch (error) {
      set({ error: errorMessage(error), loading: false });
    }
  },

  subscribe: () => {
    const unsubscribeChunk = get().api.onRunChunk((event) => {
      if (get().activeRequestId != null) {
        set({ streamedOutput: get().streamedOutput + event.chunk });
      }
    });
    const unsubscribeProgress = get().api.onMatrixProgress((event) => {
      if (get().activeRequestId != null) {
        set({ progress: { completed: event.completed, total: event.total } });
      }
    });
    return () => {
      unsubscribeChunk();
      unsubscribeProgress();
    };
  },

  saveProfile: async (input) => {
    try {
      const profile = await get().api.saveProfile(input);
      set({ profiles: await get().api.listProfiles(), error: null });
      return profile;
    } catch (error) {
      set({ error: errorMessage(error) });
      return null;
    }
  },

  render: async (revisionId, inputs) => {
    try {
      set({ rendered: await get().api.render(revisionId, inputs), error: null });
    } catch (error) {
      set({ rendered: null, error: errorMessage(error) });
    }
  },

  run: async (input) => {
    const id = requestId();
    set({ activeRequestId: id, streamedOutput: "", error: null });
    try {
      const run = await get().api.run(id, input);
      set({
        activeRequestId: null,
        runs: await get().api.listRuns(),
        streamedOutput: run.output ?? get().streamedOutput,
      });
      return run;
    } catch (error) {
      set({ activeRequestId: null, error: errorMessage(error) });
      return null;
    }
  },

  cancel: async () => {
    const id = get().activeRequestId;
    if (id == null) return;
    try {
      await get().api.cancel(id);
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  saveTestSet: async (input) => {
    try {
      const testSet = await get().api.saveTestSet(input);
      set({ testSets: await get().api.listTestSets(), error: null });
      return testSet;
    } catch (error) {
      set({ error: errorMessage(error) });
      return null;
    }
  },

  importTestSet: async (json) => {
    try {
      const testSet = await get().api.importTestSet(json);
      set({ testSets: await get().api.listTestSets(), error: null });
      return testSet;
    } catch (error) {
      set({ error: errorMessage(error) });
      return null;
    }
  },

  exportTestSet: async (id) => {
    try {
      return await get().api.exportTestSet(id);
    } catch (error) {
      set({ error: errorMessage(error) });
      return null;
    }
  },

  createEvaluator: async (input) => {
    try {
      const evaluator = await get().api.createEvaluator(input);
      set({ evaluators: await get().api.listEvaluators(), error: null });
      return evaluator;
    } catch (error) {
      set({ error: errorMessage(error) });
      return null;
    }
  },

  runMatrix: async (input) => {
    const id = requestId();
    set({ activeRequestId: id, progress: { completed: 0, total: 0 }, error: null });
    try {
      const detail = await get().api.runMatrix(id, input);
      set({
        activeRequestId: null,
        progress: null,
        selectedMatrix: detail,
        matrices: await get().api.listMatrices(),
        runs: await get().api.listRuns(),
      });
      return detail;
    } catch (error) {
      set({ activeRequestId: null, progress: null, error: errorMessage(error) });
      return null;
    }
  },

  retryMatrix: async (matrixId) => {
    const id = requestId();
    set({ activeRequestId: id, progress: { completed: 0, total: 0 }, error: null });
    try {
      const detail = await get().api.retryMatrix(id, matrixId);
      set({
        activeRequestId: null,
        progress: null,
        selectedMatrix: detail,
        matrices: await get().api.listMatrices(),
      });
      return detail;
    } catch (error) {
      set({ activeRequestId: null, progress: null, error: errorMessage(error) });
      return null;
    }
  },

  selectMatrix: async (id) => {
    try {
      set({ selectedMatrix: await get().api.getMatrix(id), error: null });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  setManualResult: async (cellId, evaluatorId, passed, evidence) => {
    try {
      await get().api.setManualResult(cellId, evaluatorId, passed, evidence);
      const matrixId = get().selectedMatrix?.run.id;
      if (matrixId) set({ selectedMatrix: await get().api.getMatrix(matrixId) });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  loadLabels: async (promptId) => {
    try {
      const [labels, labelHistory] = await Promise.all([
        get().api.listLabels(promptId),
        get().api.labelHistory(promptId),
      ]);
      set({ labels, labelHistory });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  moveLabel: async (promptId, label, revisionId, rollback = false) => {
    try {
      if (rollback) await get().api.rollbackLabel(promptId, label, revisionId);
      else await get().api.moveLabel(promptId, label, revisionId);
      await get().loadLabels(promptId);
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },
}));
