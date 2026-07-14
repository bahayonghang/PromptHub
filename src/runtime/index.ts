/**
 * Runtime_Bridge: the single abstraction layer between the React Frontend and
 * the Tauri Command_Layer (Requirement 3).
 *
 * UI components MUST route every backend call and event subscription through
 * this module rather than importing `@tauri-apps/api` directly (Req 3.1).
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

/**
 * Capability descriptor: reports, for each named desktop capability, whether it
 * is available in the current runtime (Req 3.2). On the Tauri desktop runtime
 * every capability is available.
 */
export interface RuntimeCapabilities {
  appUpdate: boolean;
  dataRecovery: boolean;
  desktopWindowControls: boolean;
}

/** Detaches an event subscription created with {@link RuntimeBridge.on} (Req 3.6). */
export type UnsubscribeFn = () => void;

export interface RuntimeBridge {
  /** Returns the capability descriptor for the current runtime (Req 3.2). */
  capabilities(): RuntimeCapabilities;
  /**
   * Invokes a Command_Layer command and resolves with its data on success, or
   * rejects with a {@link BridgeError} on failure (Req 3.3, 3.5). Capability-gated
   * commands short-circuit with `CAPABILITY_UNAVAILABLE` when their capability is
   * unavailable, without calling the backend (Req 3.7).
   */
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  /**
   * Subscribes to a Command_Layer event, delivering each payload to `handler`
   * (Req 3.4). The returned function detaches the subscription (Req 3.6).
   */
  on<E>(event: string, handler: (payload: E) => void): UnsubscribeFn;
}

/**
 * Typed error surfaced to callers when a command fails. Carries the stable
 * `code` and human-readable `message` from the backend `AppError` (Req 3.5).
 */
export class BridgeError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "BridgeError";
    this.code = code;
  }
}

/** Frontend mirror of the backend `CommandResult<T>` envelope (design Error Model). */
type CommandResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: { code: string; message: string; details?: unknown } };

/** All capabilities are available on the Tauri desktop runtime. */
const DESKTOP_CAPABILITIES: RuntimeCapabilities = {
  appUpdate: true,
  dataRecovery: true,
  desktopWindowControls: true,
};

/**
 * Maps capability-gated command name prefixes to the capability that guards
 * them. Commands not listed here are always permitted. Prefixes follow the
 * `domain.action` command names defined in the design Command_Layer section.
 */
const CAPABILITY_GATES: ReadonlyArray<{
  capability: keyof RuntimeCapabilities;
  prefixes: readonly string[];
}> = [
  { capability: "appUpdate", prefixes: ["updater."] },
  { capability: "dataRecovery", prefixes: ["data.recovery"] },
  { capability: "desktopWindowControls", prefixes: ["window."] },
];

/** Returns the capability guarding `command`, or `undefined` if it is ungated. */
function requiredCapability(command: string): keyof RuntimeCapabilities | undefined {
  for (const gate of CAPABILITY_GATES) {
    if (gate.prefixes.some((prefix) => command === prefix || command.startsWith(prefix))) {
      return gate.capability;
    }
  }
  return undefined;
}

/** Injectable Tauri primitives so the bridge can be unit-tested without a webview. */
export interface RuntimeBridgeDeps {
  capabilities: RuntimeCapabilities;
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  listen: <E>(
    event: string,
    handler: (event: { payload: E }) => void,
  ) => Promise<() => void>;
}

const defaultDeps: RuntimeBridgeDeps = {
  capabilities: DESKTOP_CAPABILITIES,
  invoke: tauriInvoke,
  listen: tauriListen,
};

/**
 * Builds a {@link RuntimeBridge}. Dependencies default to the live Tauri APIs and
 * the all-true desktop capability descriptor; tests may override them.
 */
export function createRuntimeBridge(deps: Partial<RuntimeBridgeDeps> = {}): RuntimeBridge {
  const { capabilities, invoke, listen } = { ...defaultDeps, ...deps };

  return {
    capabilities() {
      return { ...capabilities };
    },

    async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      const gated = requiredCapability(command);
      if (gated && !capabilities[gated]) {
        // Capability unavailable: fail without touching the backend (Req 3.7).
        throw new BridgeError(
          "CAPABILITY_UNAVAILABLE",
          `Capability "${gated}" is not available in the current runtime.`,
        );
      }

      let result: CommandResult<T>;
      try {
        result = await invoke<CommandResult<T>>(command, args);
      } catch (err) {
        if (err instanceof BridgeError) {
          throw err;
        }
        // The IPC transport itself failed (command missing, backend panic, etc.).
        throw new BridgeError("INTERNAL", err instanceof Error ? err.message : String(err));
      }

      if (result.ok) {
        return result.data;
      }
      // Structured failure: surface code + message, never a success (Req 3.5).
      throw new BridgeError(result.error.code, result.error.message);
    },

    on<E>(event: string, handler: (payload: E) => void): UnsubscribeFn {
      let unlisten: (() => void) | undefined;
      let cancelled = false;

      void listen<E>(event, (e) => handler(e.payload)).then((fn) => {
        // If unsubscribe ran before the listener attached, detach immediately.
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      });

      return () => {
        cancelled = true;
        unlisten?.();
      };
    },
  };
}

/** The production Runtime_Bridge bound to the live Tauri desktop runtime. */
export const runtime: RuntimeBridge = createRuntimeBridge();
