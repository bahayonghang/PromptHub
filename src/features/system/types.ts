/**
 * Frontend domain types for the system view (Requirements 20, 24): window
 * controls, close-action, auto-launch, keyboard shortcuts, notifications, and
 * the in-app updater.
 *
 * These mirror the Command_Layer DTOs the Tauri_Backend returns. The Rust
 * structs in `src-tauri/src/services/window.rs` and
 * `src-tauri/src/services/updater.rs` derive `#[serde(rename_all = "camelCase")]`,
 * so every field below is the camelCase form of its `snake_case` counterpart.
 */

// ===========================================================================
// Window controls (Req 20.1-20.4) — mirrors `services::window`
// ===========================================================================

/** What happens when a window-close is attempted (Req 20.4). */
export type CloseAction = "ask" | "minimize" | "exit";

/** All close actions in display order. */
export const CLOSE_ACTIONS: readonly CloseAction[] = ["ask", "minimize", "exit"];

/** Payload of the `window:fullscreen-changed` event (Req 20.2). */
export interface FullscreenChanged {
  fullscreen: boolean;
}

/** Payload of the `window:visibility-changed` event (Req 20.3). */
export interface VisibilityChanged {
  visible: boolean;
}

// ===========================================================================
// Keyboard shortcuts (Req 20.6, 20.11) — mirrors `services::window::Shortcut`
// ===========================================================================

/** Whether a shortcut fires globally or only when the window is focused (Req 20.6). */
export type ShortcutMode = "global" | "local";

/** A single registered keyboard shortcut (Req 20.6). */
export interface Shortcut {
  /** The action identifier emitted by `shortcut:triggered` when this fires. */
  action: string;
  /** The key combination, e.g. `CmdOrCtrl+Shift+K`. */
  accelerator: string;
  /** Whether this is a global or local shortcut. */
  mode: ShortcutMode;
}

/** Payload of the `shortcut:triggered` event (Req 20.6). */
export interface ShortcutTriggered {
  action: string;
}

/** The maximum number of shortcuts that may be registered (Req 20.6). */
export const MAX_SHORTCUTS = 50;

// ===========================================================================
// Runtime paths (Req 20.9) — mirrors `services::window::RuntimePathsReport`
// ===========================================================================

/** The resolved absolute filesystem paths reported to the Frontend (Req 20.9). */
export interface RuntimePathsReport {
  data: string;
  database: string;
  media: string;
  skill: string;
  rule: string;
  backup: string;
  log: string;
}

// ===========================================================================
// Updater (Req 24.2-24.7) — mirrors `services::updater`
// ===========================================================================

/** Outcome of an update check (Req 24.2). */
export interface UpdateCheckResult {
  /** Whether an update is available. */
  available: boolean;
  /** The available version, present only when `available` is true. */
  version?: string | null;
  /** The currently running application version. */
  currentVersion: string;
  /** Release notes for the available update, when the server provided them. */
  notes?: string | null;
}

/** A single `updater:status` progress event (Req 24.3). */
export interface UpdaterStatus {
  /** Lifecycle phase: `downloading` for progress, `done` for completion. */
  phase: string;
  /** Bytes downloaded so far (cumulative). */
  downloaded?: number | null;
  /** Total bytes to download, when the server reported a content length. */
  total?: number | null;
}

/**
 * The updater's UI lifecycle, tracked by the store (Req 24.2-24.7). Distinct
 * from {@link UpdaterStatus.phase}, which is the backend's per-event phase.
 */
export type UpdaterPhase =
  | "idle"
  | "checking"
  | "available"
  | "upToDate"
  | "downloading"
  | "downloaded"
  | "installing"
  | "error";
