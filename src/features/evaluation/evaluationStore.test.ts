import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EvaluationApi } from "./api";
import { useEvaluationStore } from "./evaluationStore";
import type { PromptRun } from "./types";

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
});
