import { beforeEach, describe, expect, it, vi } from "vitest";
import { BridgeError } from "../../runtime";
import type { EvaluationApi } from "./api";
import { useEvaluationStore } from "./evaluationStore";
import type {
  ExecutionProfileRevision,
  MatrixProgressEvent,
  PromptLabel,
  PromptRun,
  RunChunkEvent,
  TestSet,
} from "./types";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function makeProfile(id: string): ExecutionProfileRevision {
  return {
    id,
    profileId: id,
    revision: 1,
    name: id,
    provider: "mock",
    model: "deterministic",
    parameters: {},
    hasCredential: false,
    createdAt: "2026-01-01T00:00:00.000Z",
  };
}

function makeRun(partial: Partial<PromptRun> = {}): PromptRun {
  return {
    id: "run-1",
    promptRevisionId: "revision-1",
    profileRevisionId: "profile-1",
    inputs: {},
    renderedMessages: [],
    status: "success",
    startedAt: "2026-01-01T00:00:00.000Z",
    ...partial,
  };
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
      playgroundRequestId: null,
      matrixRequestId: null,
      progress: null,
      labels: [],
      labelHistory: [],
      loading: false,
      error: null,
    });
  });

  it("does not let a slower load overwrite newer collections", async () => {
    const firstProfiles = deferred<ExecutionProfileRevision[]>();
    const secondProfiles = deferred<ExecutionProfileRevision[]>();
    let profileCalls = 0;
    const api = {
      listProfiles: vi.fn(() => {
        profileCalls += 1;
        return profileCalls === 1 ? firstProfiles.promise : secondProfiles.promise;
      }),
      listRuns: vi.fn(async () => []),
      listTestSets: vi.fn(async () => []),
      listEvaluators: vi.fn(async () => []),
      listMatrices: vi.fn(async () => []),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });

    const first = useEvaluationStore.getState().load();
    const second = useEvaluationStore.getState().load();
    secondProfiles.resolve([makeProfile("new")]);
    await second;
    firstProfiles.resolve([makeProfile("old")]);
    await first;

    expect(useEvaluationStore.getState().profiles.map((profile) => profile.id)).toEqual([
      "new",
    ]);
    expect(useEvaluationStore.getState().loading).toBe(false);
  });

  it("does not apply a slower load failure after a newer load succeeds", async () => {
    const firstProfiles = deferred<ExecutionProfileRevision[]>();
    const secondProfiles = deferred<ExecutionProfileRevision[]>();
    let profileCalls = 0;
    const api = {
      listProfiles: vi.fn(() => {
        profileCalls += 1;
        return profileCalls === 1 ? firstProfiles.promise : secondProfiles.promise;
      }),
      listRuns: vi.fn(async () => []),
      listTestSets: vi.fn(async () => []),
      listEvaluators: vi.fn(async () => []),
      listMatrices: vi.fn(async () => []),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });

    const first = useEvaluationStore.getState().load();
    const second = useEvaluationStore.getState().load();
    secondProfiles.resolve([makeProfile("new")]);
    await second;
    firstProfiles.reject(new Error("stale load"));
    await first;

    expect(useEvaluationStore.getState().profiles.map((profile) => profile.id)).toEqual([
      "new",
    ]);
    expect(useEvaluationStore.getState().error).toBeNull();
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
    let chunkHandler: ((event: RunChunkEvent) => void) | undefined;
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
    const requestId = useEvaluationStore.getState().playgroundRequestId;
    expect(requestId).toEqual(expect.any(String));
    chunkHandler?.({ requestId: requestId!, runId: "run-1", chunk: "partial" });
    await useEvaluationStore.getState().cancel();
    expect(cancel).toHaveBeenCalledWith(requestId);
    pending.resolve(
      makeRun({
        output: "complete",
        status: "success",
      }),
    );
    await running;
    expect(useEvaluationStore.getState().streamedOutput).toBe("complete");
  });

  it("ignores run chunks from request A while request B is active", async () => {
    const first = deferred<PromptRun>();
    const second = deferred<PromptRun>();
    let runCalls = 0;
    let chunkHandler: ((event: RunChunkEvent) => void) | undefined;
    const api = {
      run: vi.fn(() => {
        runCalls += 1;
        return runCalls === 1 ? first.promise : second.promise;
      }),
      listRuns: vi.fn(async () => []),
      onRunChunk: vi.fn((handler) => {
        chunkHandler = handler;
        return () => undefined;
      }),
      onMatrixProgress: vi.fn(() => () => undefined),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });
    useEvaluationStore.getState().subscribe();

    const runningA = useEvaluationStore.getState().run({
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
    });
    const idA = useEvaluationStore.getState().playgroundRequestId;
    const runningB = useEvaluationStore.getState().run({
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
    });
    const idB = useEvaluationStore.getState().playgroundRequestId;
    expect(idA).toEqual(expect.any(String));
    expect(idB).toEqual(expect.any(String));
    expect(idA).not.toBe(idB);

    chunkHandler?.({ requestId: idA!, runId: "run-a", chunk: "from-a" });
    expect(useEvaluationStore.getState().streamedOutput).toBe("");
    chunkHandler?.({ requestId: idB!, runId: "run-b", chunk: "from-b" });
    expect(useEvaluationStore.getState().streamedOutput).toBe("from-b");

    first.resolve(makeRun({ id: "run-a", output: "a-complete", status: "success" }));
    await runningA;
    expect(useEvaluationStore.getState().streamedOutput).toBe("from-b");
    expect(useEvaluationStore.getState().playgroundRequestId).toBe(idB);

    second.resolve(makeRun({ id: "run-b", output: "b-complete", status: "success" }));
    await runningB;
    expect(useEvaluationStore.getState().streamedOutput).toBe("b-complete");
    expect(useEvaluationStore.getState().playgroundRequestId).toBeNull();
  });

  it("does not apply playground chunks to an in-flight matrix request", async () => {
    const pendingRun = deferred<PromptRun>();
    const pendingMatrix = deferred<{ run: { id: string } }>();
    let chunkHandler: ((event: RunChunkEvent) => void) | undefined;
    let progressHandler: ((event: MatrixProgressEvent) => void) | undefined;
    const api = {
      run: vi.fn(() => pendingRun.promise),
      runMatrix: vi.fn(() => pendingMatrix.promise),
      listRuns: vi.fn(async () => []),
      listMatrices: vi.fn(async () => []),
      onRunChunk: vi.fn((handler) => {
        chunkHandler = handler;
        return () => undefined;
      }),
      onMatrixProgress: vi.fn((handler) => {
        progressHandler = handler;
        return () => undefined;
      }),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });
    useEvaluationStore.getState().subscribe();

    const running = useEvaluationStore.getState().run({
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
    });
    const playgroundId = useEvaluationStore.getState().playgroundRequestId;
    const matrixRunning = useEvaluationStore.getState().runMatrix({
      testSetId: "set-1",
      promptRevisionIds: ["revision-1"],
      profileRevisionIds: ["profile-1"],
      evaluatorIds: ["evaluator-1"],
    });
    const matrixId = useEvaluationStore.getState().matrixRequestId;
    expect(playgroundId).not.toBe(matrixId);

    chunkHandler?.({ requestId: matrixId!, runId: "cell-run", chunk: "matrix-chunk" });
    expect(useEvaluationStore.getState().streamedOutput).toBe("");
    progressHandler?.({
      requestId: playgroundId!,
      evaluationRunId: "eval-1",
      completed: 1,
      total: 4,
      cellId: "cell-1",
    });
    expect(useEvaluationStore.getState().progress).toEqual({ completed: 0, total: 0 });
    progressHandler?.({
      requestId: matrixId!,
      evaluationRunId: "eval-1",
      completed: 1,
      total: 4,
      cellId: "cell-1",
    });
    expect(useEvaluationStore.getState().progress).toEqual({ completed: 1, total: 4 });
    pendingRun.resolve(makeRun());
    await running;
    pendingMatrix.resolve({ run: { id: "eval-1" } });
    await matrixRunning;
  });

  it("clears playground request state when run returns cancelled", async () => {
    const api = {
      run: vi.fn(async () => makeRun({ status: "cancelled", output: "partial" })),
      listRuns: vi.fn(async () => []),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });
    const result = await useEvaluationStore.getState().run({
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
    });
    expect(result?.status).toBe("cancelled");
    expect(useEvaluationStore.getState().error).toBeNull();
    expect(useEvaluationStore.getState().playgroundRequestId).toBeNull();
  });

  it("clears error when loadLabels succeeds after a failure", async () => {
    const api = {
      listLabels: vi
        .fn()
        .mockRejectedValueOnce(new Error("labels failed"))
        .mockResolvedValueOnce([
          {
            promptId: "prompt-1",
            label: "candidate",
            promptRevisionId: "revision-1",
            updatedAt: "2026-01-01T00:00:00.000Z",
          },
        ]),
      labelHistory: vi.fn(async () => []),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });
    await useEvaluationStore.getState().loadLabels("prompt-1");
    expect(useEvaluationStore.getState().error).toBe("labels failed");
    await useEvaluationStore.getState().loadLabels("prompt-1");
    expect(useEvaluationStore.getState().error).toBeNull();
    expect(useEvaluationStore.getState().labels).toHaveLength(1);
  });

  it("does not apply a slower loadLabels failure after a newer success", async () => {
    const firstLabels = deferred<PromptLabel[]>();
    const secondLabels = deferred<PromptLabel[]>();
    let calls = 0;
    const api = {
      listLabels: vi.fn(() => {
        calls += 1;
        return calls === 1 ? firstLabels.promise : secondLabels.promise;
      }),
      labelHistory: vi.fn(async () => []),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });

    const first = useEvaluationStore.getState().loadLabels("prompt-a");
    const second = useEvaluationStore.getState().loadLabels("prompt-b");
    secondLabels.resolve([
      {
        promptId: "prompt-b",
        label: "baseline",
        promptRevisionId: "revision-2",
        updatedAt: "2026-01-01T00:00:00.000Z",
      },
    ]);
    await second;
    firstLabels.reject(new Error("stale labels"));
    await first;

    expect(useEvaluationStore.getState().labels.map((label) => label.promptId)).toEqual([
      "prompt-b",
    ]);
    expect(useEvaluationStore.getState().error).toBeNull();
  });

  it("sets error when run returns status error", async () => {
    const api = {
      run: vi.fn(async () =>
        makeRun({
          status: "error",
          error: "provider timeout",
          output: null,
        }),
      ),
      listRuns: vi.fn(async () => []),
    } as unknown as EvaluationApi;
    useEvaluationStore.setState({ api });
    const result = await useEvaluationStore.getState().run({
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: {},
    });
    expect(result?.status).toBe("error");
    expect(useEvaluationStore.getState().error).toBe("provider timeout");
    expect(useEvaluationStore.getState().playgroundRequestId).toBeNull();
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
      playgroundRequestId: null,
      matrixRequestId: null,
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
    expect(useEvaluationStore.getState().playgroundRequestId).toBeNull();
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
