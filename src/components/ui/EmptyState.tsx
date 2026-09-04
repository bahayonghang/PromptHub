import type { ReactNode } from "react";
import { cn } from "./cn";

export interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  /** Primary recovery action, e.g. "clear filters" or "create the first item". */
  action?: ReactNode;
  className?: string;
}

/**
 * Placeholder for an empty list or result set (design plan §5).
 *
 * Gives the user a reason and a way out instead of a bare line of grey text.
 * Copy is constrained to a readable measure rather than stretching across the
 * full workspace width.
 *
 * Use this only when the empty region owns the whole pane. Inline "nothing
 * here" copy under an existing heading belongs in {@link EmptyHint} — wrapping
 * those sites in a centered full-height block would dominate the surrounding
 * chrome.
 */
export function EmptyState({ icon, title, description, action, className = "" }: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex h-full flex-col items-center justify-center gap-2 p-6 text-center",
        className,
      )}
    >
      {icon != null && (
        <div className="mb-1 flex h-10 w-10 items-center justify-center rounded-lg border border-border bg-surface-inset text-muted-foreground-subtle">
          {icon}
        </div>
      )}
      <p className="text-body font-medium text-foreground">{title}</p>
      {description != null && (
        <p className="max-w-[38ch] text-label text-muted-foreground">{description}</p>
      )}
      {action != null && <div className="mt-2">{action}</div>}
    </div>
  );
}

export interface EmptyHintProps {
  children: ReactNode;
  className?: string;
}

/**
 * One-line empty copy that sits under an existing heading or inside a list.
 *
 * Deliberately not EmptyState. The sites that use this already have a title,
 * a toolbar, or a list row of their own; a centered icon-and-action block
 * would shout over that context. Keep the two primitives distinct so a later
 * "just unify them" pass cannot quietly inflate every inline hint.
 */
export function EmptyHint({ children, className = "" }: EmptyHintProps) {
  return <p className={cn("text-label text-muted-foreground", className)}>{children}</p>;
}
