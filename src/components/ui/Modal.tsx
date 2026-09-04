import {
  useEffect,
  useId,
  useRef,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

export interface ModalProps {
  open: boolean;
  title: string;
  titleId?: string;
  onClose: () => void;
  children: ReactNode;
  className?: string;
  /** Extra class on the scrim. */
  scrimClassName?: string;
}

interface StackEntry {
  id: number;
}

const stack: StackEntry[] = [];
let nextStackId = 1;

function focusableElements(root: HTMLElement): HTMLElement[] {
  const nodes = root.querySelectorAll<HTMLElement>(
    'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
  );
  return [...nodes].filter(
    (node) => !node.hasAttribute("disabled") && node.tabIndex !== -1,
  );
}

function setAppContentInert(inert: boolean): void {
  const region = document.getElementById("app-content");
  if (!region) return;
  if (inert) {
    region.setAttribute("inert", "");
    region.setAttribute("aria-hidden", "true");
  } else {
    region.removeAttribute("inert");
    region.removeAttribute("aria-hidden");
  }
}

/**
 * App-wide dialog primitive. Portals to document.body, traps focus, restores
 * it on close, and marks #app-content inert so CloseDialog stays live.
 */
export function Modal({
  open,
  title,
  titleId,
  onClose,
  children,
  className = "",
  scrimClassName = "",
}: ModalProps) {
  const generatedId = useId();
  const headingId = titleId ?? generatedId;
  const dialogRef = useRef<HTMLDivElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  const stackId = useRef<number | null>(null);

  useEffect(() => {
    if (!open) return;
    previousFocus.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    const id = nextStackId;
    nextStackId += 1;
    stackId.current = id;
    stack.push({ id });
    setAppContentInert(true);
    const frame = window.requestAnimationFrame(() => {
      const root = dialogRef.current;
      if (!root) return;
      const first = focusableElements(root)[0];
      (first ?? root).focus();
    });
    return () => {
      window.cancelAnimationFrame(frame);
      const index = stack.findIndex((entry) => entry.id === id);
      if (index >= 0) stack.splice(index, 1);
      if (stack.length === 0) setAppContentInert(false);
      previousFocus.current?.focus();
    };
  }, [open]);

  if (!open || typeof document === "undefined") return null;

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      const top = stack[stack.length - 1];
      if (top && top.id === stackId.current) {
        event.stopPropagation();
        event.preventDefault();
        onClose();
      }
      return;
    }
    if (event.key !== "Tab") return;
    const root = dialogRef.current;
    if (!root) return;
    const items = focusableElements(root);
    if (items.length === 0) {
      event.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    if (event.shiftKey) {
      if (active === first || !root.contains(active)) {
        event.preventDefault();
        last.focus();
      }
    } else if (active === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return createPortal(
    <div
      className={`fixed inset-0 z-40 flex items-center justify-center bg-scrim/55 p-4 backdrop-blur-[2px] ${scrimClassName}`}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={headingId}
        tabIndex={-1}
        onKeyDown={onKeyDown}
        className={`prompt-detail-modal max-h-[min(90vh,56rem)] w-[min(1180px,100%)] overflow-hidden rounded-lg border border-border bg-card text-card-foreground shadow-lg outline-none ${className}`}
      >
        <h2 id={headingId} className="sr-only">
          {title}
        </h2>
        {children}
      </div>
    </div>,
    document.body,
  );
}
