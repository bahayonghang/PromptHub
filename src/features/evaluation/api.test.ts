import { describe, expect, it, vi } from "vitest";
import type { RuntimeBridge } from "../../runtime";
import { createEvaluationApi } from "./api";

describe("evaluation api", () => {
  it("uses the stable evaluation command names and camelCase arguments", async () => {
    const invokeMock = vi.fn(async () => []);
    const invoke = invokeMock as unknown as RuntimeBridge["invoke"];
    const bridge: RuntimeBridge = {
      capabilities: () => ({
        appUpdate: true,
        dataRecovery: true,
        desktopWindowControls: true,
      }),
      invoke,
      on: vi.fn(() => () => undefined),
    };
    const api = createEvaluationApi(bridge);

    await api.render("revision-1", { name: "Ada" });
    await api.run("request-1", {
      promptRevisionId: "revision-1",
      profileRevisionId: "profile-1",
      inputs: { name: "Ada" },
    });
    await api.runMatrix("matrix-request", {
      testSetId: "set-1",
      promptRevisionIds: ["revision-1"],
      profileRevisionIds: ["profile-1"],
      evaluatorIds: ["evaluator-1"],
    });

    expect(invokeMock).toHaveBeenNthCalledWith(1, "evaluation.render", {
      promptRevisionId: "revision-1",
      inputs: { name: "Ada" },
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "evaluation.run", {
      requestId: "request-1",
      input: expect.objectContaining({ promptRevisionId: "revision-1" }),
    });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "evaluation.matrixRun", {
      requestId: "matrix-request",
      input: expect.objectContaining({ testSetId: "set-1" }),
    });
  });

  it("subscribes through the Runtime Bridge", () => {
    const on = vi.fn(() => () => undefined);
    const api = createEvaluationApi({
      capabilities: () => ({
        appUpdate: true,
        dataRecovery: true,
        desktopWindowControls: true,
      }),
      invoke: vi.fn(),
      on,
    });
    const handler = vi.fn();
    api.onRunChunk(handler);
    api.onMatrixProgress(handler);
    expect(on).toHaveBeenCalledWith("evaluation:run-chunk", handler);
    expect(on).toHaveBeenCalledWith("evaluation:matrix-progress", handler);
  });
});
