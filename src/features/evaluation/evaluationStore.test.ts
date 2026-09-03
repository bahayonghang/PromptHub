import { beforeEach, describe, expect, it, vi } from "vitest";
import { BridgeError } from "../../runtime";
import type { EvaluationApi } from "./api";
import { useEvaluationStore } from "./evaluationStore";
import type {
  ExecutionProfileRevision,
  PromptLabel,
  PromptRun,
  TestSet,
} from "./types";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

describe("evaluation store", () => {
  beforeEach(() => {
    useEvaluationStore.setState({
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
    });
  });

  it("loads all evaluation collections together", async () => {
    const api = {
      listProfiles: vi.fn(async () => []),
      listRuns: vi.fn(async () => []),
      listTestSets: vi.fn(async () => []),
      listEvaluators: vi.fn(async () => []),
      listMatrices: vi.fn(async () => []),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });
    await useEvaluationStore.getState().load();
    expect(api.listProfiles).toHaveBeenCalledOnce();
    expect(api.listMatrices).toHaveBeenCalledOnce();
    expect(useEvaluationStore.getState().loading).toBe(false);
  });

  it("keeps streamed output and cancels by the active request id", async () => {
    const pending = deferred<PromptRun>();
    let chunkHandler: ((event: { runId: string; chunk: string }) => void) | undefined;
    const cancel = vi.fn(async () => undefined);
    const api = {
      run: vi.fn(() => pending.promise),
      cancel,
      listRuns: vi.fn(async () => []),
      onRunChunk: vi.fn((handler) => {
        chunkHandler = handler;
        return () => undefined;
      }),
      onMatrixProgress: vi.fn(() => () => undefined),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });
    useEvaluationStore.getState().subscribe();
    const running = useEvaluationStore.getState().run({
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
    });
    chunkHandler?.({ runId: "run-1", chunk: "partial" });
    await useEvaluationStore.getState().cancel();
    expect(cancel).toHaveBeenCalledWith(expect.stringMatching(/^evaluation-|[0-9a-f-]{36}$/));
    pending.resolve({
      id: "run-1",
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
      renderedMessages: [],
      output: "complete",
      status: "success",
      startedAt: "2026-01-01T00:00:00.000Z",
    });
    await running;
    expect(useEvaluationStore.getState().streamedOutput).toBe("complete");
  });

  it("surfaces BridgeError from render, run, saveProfile, and moveLabel without mutating collections", async () => {
    const profiles: ExecutionProfileRevision[] = [
      {
        id: "profile-1",
        profileId: "profile-1",
        revision: 1,
        name: "Mock",
        provider: "mock",
        model: "deterministic",
        parameters: {},
        hasCredential: false,
        createdAt: "2026-01-01T00:00:00.000Z",
      },
    ];
    const runs: PromptRun[] = [
      {
        id: "run-1",
        promptRevisionId: "revision-1",
        profileRevisionId: "profile-1",
        inputs: {},
        renderedMessages: [],
        status: "success",
        startedAt: "2026-01-01T00:00:00.000Z",
      },
    ];
    const testSets: TestSet[] = [
      {
        id: "set-1",
        name: "Cases",
        cases: [],
        createdAt: "2026-01-01T00:00:00.000Z",
        updatedAt: "2026-01-01T00:00:00.000Z",
      },
    ];
    const labels: PromptLabel[] = [
      {
        promptId: "prompt-1",
        label: "candidate",
        promptRevisionId: "revision-1",
        updatedAt: "2026-01-01T00:00:00.000Z",
      },
    ];
    const snapshot = {
      profiles,
      runs,
      testSets,
      evaluators: [],
      matrices: [],
      labels,
      labelHistory: [],
    };

    const api = {
      render: vi.fn(async () => {
        throw new BridgeError("VALIDATION", "required variable `name` is missing");
      }),
      run: vi.fn(async () => {
        throw new BridgeError("SSRF_BLOCKED", "host `127.0.0.1` names the local machine");
      }),
      saveProfile: vi.fn(async () => {
        throw new BridgeError(
          "UNAUTHORIZED",
          "unlock the library before storing or using provider credentials",
        );
      }),
      listProfiles: vi.fn(async () => profiles),
      listRuns: vi.fn(async () => runs),
      moveLabel: vi.fn(async () => {
        throw new BridgeError(
          "VALIDATION",
          "only a successfully evaluated revision can receive a label",
        );
      }),
      listLabels: vi.fn(async () => labels),
      labelHistory: vi.fn(async () => []),
    } as unknown as EvaluationApi;

    useEvaluationStore.setState({
      api,
      ...snapshot,
      selectedMatrix: null,
      rendered: {
        promptRevisionId: "revision-1",
        messages: [{ role: "user", content: "Hello" }],
      },
      streamedOutput: "kept",
      activeRequestId: null,
      progress: null,
      loading: false,
      error: null,
    });

    const expectCollectionsUnchanged = () => {
      const state = useEvaluationStore.getState();
      expect(state.profiles).toEqual(snapshot.profiles);
      expect(state.runs).toEqual(snapshot.runs);
      expect(state.testSets).toEqual(snapshot.testSets);
      expect(state.evaluators).toEqual(snapshot.evaluators);
      expect(state.matrices).toEqual(snapshot.matrices);
      expect(state.labels).toEqual(snapshot.labels);
      expect(state.labelHistory).toEqual(snapshot.labelHistory);
    };

    await useEvaluationStore.getState().render("revision-1", {});
    expect(useEvaluationStore.getState().error).toBe(
      "required variable `name` is missing",
    );
    expect(useEvaluationStore.getState().rendered).toBeNull();
    expectCollectionsUnchanged();
    expect(api.listProfiles).not.toHaveBeenCalled();

    const runResult = await useEvaluationStore.getState().run({
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
    });
    expect(runResult).toBeNull();
    expect(useEvaluationStore.getState().error).toBe(
      "host `127.0.0.1` names the local machine",
    );
    expect(useEvaluationStore.getState().activeRequestId).toBeNull();
    expect(useEvaluationStore.getState().streamedOutput).toBe("");
    expectCollectionsUnchanged();
    expect(api.listRuns).not.toHaveBeenCalled();

    const saved = await useEvaluationStore.getState().saveProfile({
      name: "Remote",
      provider: "openai-compatible",
      endpoint: "https://example.com/v1/chat/completions",
      model: "model",
      parameters: {},
      credential: "secret",
    });
    expect(saved).toBeNull();
    expect(useEvaluationStore.getState().error).toBe(
      "unlock the library before storing or using provider credentials",
    );
    expectCollectionsUnchanged();
    expect(api.listProfiles).not.toHaveBeenCalled();

    await useEvaluationStore
      .getState()
      .moveLabel("prompt-1", "candidate", "revision-2");
    expect(useEvaluationStore.getState().error).toBe(
      "only a successfully evaluated revision can receive a label",
    );
    expectCollectionsUnchanged();
    expect(api.listLabels).not.toHaveBeenCalled();
    expect(api.labelHistory).not.toHaveBeenCalled();
  });
});
