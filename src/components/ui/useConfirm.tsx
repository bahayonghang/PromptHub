import { useCallback, useRef, useState } from "react";
import { ConfirmDialog } from "./ConfirmDialog";

export interface ConfirmRequest {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}

interface PendingState extends ConfirmRequest {
  open: boolean;
}

const CLOSED: PendingState = { open: false, title: "", message: "" };

/**
 * Promise-based replacement for `window.confirm` (design plan §5).
 *
 * `window.confirm` blocks the main thread and is drawn by the OS, so it ignores
 * the app theme entirely — inside a Tauri window it reads as a bug. Returning a
 * promise keeps the call sites shaped almost exactly like the originals:
 *
 * ```tsx
 * const { confirm, confirmDialog } = useConfirm();
 * // if (window.confirm(msg)) { act(); }
 * if (await confirm({ title, message: msg })) { act(); }
 * // ...and render {confirmDialog} once, anywhere in the subtree.
 * ```
 *
 * The returned element is a single dialog instance reused by every request, so
 * a component with five confirmable actions still mounts one `Modal`.
 */
export function useConfirm() {
  const [state, setState] = useState<PendingState>(CLOSED);
  const resolverRef = useRef<((value: boolean) => void) | null>(null);

  const settle = useCallback((value: boolean) => {
    // Resolve before closing so a caller awaiting the promise observes the
    // decision even if this component unmounts during the state update.
    resolverRef.current?.(value);
    resolverRef.current = null;
    setState(CLOSED);
  }, []);

  const confirm = useCallback((request: ConfirmRequest) => {
    // A second request while one is pending would strand the first promise
    // forever, so decline the outstanding one first.
    resolverRef.current?.(false);
    setState({ ...request, open: true });
    return new Promise<boolean>((resolve) => {
      resolverRef.current = resolve;
    });
  }, []);

  const confirmDialog = (
    <ConfirmDialog
      open={state.open}
      title={state.title}
      message={state.message}
      confirmLabel={state.confirmLabel}
      cancelLabel={state.cancelLabel}
      destructive={state.destructive}
      onConfirm={() => settle(true)}
      onCancel={() => settle(false)}
    />
  );

  return { confirm, confirmDialog };
}
