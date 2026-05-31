import { describe, expect, it, vi } from "vitest";
import fc from "fast-check";
import {
  BridgeError,
  createRuntimeBridge,
  type RuntimeBridgeDeps,
  type RuntimeCapabilities,
} from "./index";

const ALL_TRUE: RuntimeCapabilities = {
  appUpdate: true,
  dataRecovery: true,
  desktopWindowControls: true,
  skillDistribution: true,
  skillFileEditing: true,
  skillLocalScan: true,
  skillPlatformIntegration: true,
  skillStore: true,
};

/** Builds a bridge whose injected primitives are vi mocks, with sensible defaults. */
function makeBridge(overrides: Partial<RuntimeBridgeDeps> = {}) {
  const invoke = vi.fn(async () => ({ ok: true, data: null }) as unknown);
  const listen = vi.fn(async () => () => {});
  const deps: RuntimeBridgeDeps = {
    capabilities: ALL_TRUE,
    invoke: invoke as RuntimeBridgeDeps["invoke"],
    listen: listen as RuntimeBridgeDeps["listen"],
    ...overrides,
  };
  return { bridge: createRuntimeBridge(deps), invoke: deps.invoke, listen: deps.listen };
}

describe("capabilities (Req 3.2)", () => {
  it("reports all capabilities as available on the desktop runtime", () => {
    const { bridge } = makeBridge();
    expect(bridge.capabilities()).toEqual(ALL_TRUE);
  });

  it("returns a defensive copy that cannot mutate the descriptor", () => {
    const { bridge } = makeBridge();
    const caps = bridge.capabilities();
    caps.appUpdate = false;
    expect(bridge.capabilities().appUpdate).toBe(true);
  });
});

describe("invoke success path (Req 3.3)", () => {
  it("returns the data payload from an ok result", async () => {
    const invoke = vi.fn(async () => ({ ok: true, data: { id: "p1" } }));
    const { bridge } = makeBridge({ invoke: invoke as RuntimeBridgeDeps["invoke"] });
    await expect(bridge.invoke("prompt.get", { id: "p1" })).resolves.toEqual({ id: "p1" });
    expect(invoke).toHaveBeenCalledWith("prompt.get", { id: "p1" });
  });

  it("returns arbitrary data unchanged for ok results (property)", async () => {
    await fc.assert(
      fc.asyncProperty(fc.jsonValue(), async (data) => {
        const invoke = vi.fn(async () => ({ ok: true, data }));
        const { bridge } = makeBridge({ invoke: invoke as RuntimeBridgeDeps["invoke"] });
        await expect(bridge.invoke("prompt.list")).resolves.toStrictEqual(data);
      }),
      { numRuns: 100 },
    );
  });
});

describe("invoke failure path (Req 3.5)", () => {
  it("throws a BridgeError carrying the structured code and message", async () => {
    const invoke = vi.fn(async () => ({
      ok: false,
      error: { code: "NOT_FOUND", message: "Prompt p9 not found" },
    }));
    const { bridge } = makeBridge({ invoke: invoke as RuntimeBridgeDeps["invoke"] });
    await expect(bridge.invoke("prompt.get", { id: "p9" })).rejects.toMatchObject({
      name: "BridgeError",
      code: "NOT_FOUND",
      message: "Prompt p9 not found",
    });
  });

  it("surfaces a transport rejection as an INTERNAL BridgeError", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("command not found");
    });
    const { bridge } = makeBridge({ invoke: invoke as RuntimeBridgeDeps["invoke"] });
    const err = await bridge.invoke("prompt.get").catch((e: unknown) => e);
    expect(err).toBeInstanceOf(BridgeError);
    expect((err as BridgeError).code).toBe("INTERNAL");
    expect((err as BridgeError).message).toBe("command not found");
  });
});

describe("capability gate (Req 3.7)", () => {
  it("short-circuits a gated command with CAPABILITY_UNAVAILABLE without calling the backend", async () => {
    const invoke = vi.fn(async () => ({ ok: true, data: null }));
    const { bridge } = makeBridge({
      capabilities: { ...ALL_TRUE, appUpdate: false },
      invoke: invoke as RuntimeBridgeDeps["invoke"],
    });
    const err = await bridge.invoke("updater.check").catch((e: unknown) => e);
    expect(err).toBeInstanceOf(BridgeError);
    expect((err as BridgeError).code).toBe("CAPABILITY_UNAVAILABLE");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("permits a gated command when its capability is available", async () => {
    const invoke = vi.fn(async () => ({ ok: true, data: { available: false } }));
    const { bridge } = makeBridge({ invoke: invoke as RuntimeBridgeDeps["invoke"] });
    await expect(bridge.invoke("updater.check")).resolves.toEqual({ available: false });
    expect(invoke).toHaveBeenCalledOnce();
  });

  it("never gates an ungated domain command", async () => {
    const invoke = vi.fn(async () => ({ ok: true, data: [] }));
    const { bridge } = makeBridge({
      // Even with every capability off, plain prompt/folder commands are ungated.
      capabilities: {
        appUpdate: false,
        dataRecovery: false,
        desktopWindowControls: false,
        skillDistribution: false,
        skillFileEditing: false,
        skillLocalScan: false,
        skillPlatformIntegration: false,
        skillStore: false,
      },
      invoke: invoke as RuntimeBridgeDeps["invoke"],
    });
    await expect(bridge.invoke("prompt.list")).resolves.toEqual([]);
    expect(invoke).toHaveBeenCalledOnce();
  });
});

describe("event subscription (Req 3.4, 3.6)", () => {
  it("delivers event payloads to the handler", async () => {
    let emit: ((payload: unknown) => void) | undefined;
    const listen = vi.fn(
      async (_event: string, cb: (e: { payload: unknown }) => void) => {
        emit = (payload) => cb({ payload });
        return () => {};
      },
    );
    const handler = vi.fn();
    const { bridge } = makeBridge({ listen: listen as RuntimeBridgeDeps["listen"] });

    bridge.on("updater:status", handler);
    await vi.waitFor(() => expect(emit).toBeDefined());
    emit!({ phase: "downloading" });

    expect(handler).toHaveBeenCalledWith({ phase: "downloading" });
  });

  it("stops delivering events after unsubscribe", async () => {
    const detach = vi.fn();
    const listen = vi.fn(async () => detach);
    const { bridge } = makeBridge({ listen: listen as RuntimeBridgeDeps["listen"] });

    const unsubscribe = bridge.on("shortcut:triggered", vi.fn());
    // Allow the listen promise to resolve before unsubscribing.
    await vi.waitFor(() => expect(listen).toHaveBeenCalled());
    unsubscribe();

    await vi.waitFor(() => expect(detach).toHaveBeenCalledOnce());
  });

  it("detaches immediately when unsubscribed before the listener attaches", async () => {
    const detach = vi.fn();
    let resolveListen: ((fn: () => void) => void) | undefined;
    const listen = vi.fn(
      () =>
        new Promise<() => void>((resolve) => {
          resolveListen = resolve;
        }),
    );
    const { bridge } = makeBridge({ listen: listen as RuntimeBridgeDeps["listen"] });

    const unsubscribe = bridge.on("ai:stream-chunk", vi.fn());
    unsubscribe(); // unsubscribe wins the race
    resolveListen!(detach); // listener resolves afterwards
    await vi.waitFor(() => expect(detach).toHaveBeenCalledOnce());
  });
});
