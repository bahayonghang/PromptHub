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
