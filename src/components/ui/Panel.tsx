import type { ReactNode } from "react";
import { cn } from "./cn";

export interface PanelProps {
  children: ReactNode;
  /** Optional heading rendered above the body with a hairline divider. */
  title?: string;
  /** Rendered on the trailing edge of the title row. */
  actions?: ReactNode;
  /** Removes the default body padding for flush content such as a table. */
  flush?: boolean;
  className?: string;
}

/**
 * Framed surface (design plan §5).
 *
 * Resting panels are flat by design: a 12px radius, a 1px semantic border, and
 * the `--hairline` edge highlight. No drop shadow — that is reserved for
 * overlays, per the tonal-first rule in DESIGN.md.
 */
export function Panel({ children, title, actions, flush = false, className = "" }: PanelProps) {
  return (
    <section
      className={cn(
        "rounded-lg border border-border bg-card text-card-foreground shadow-hairline",
        className,
      )}
    >
      {(title != null || actions != null) && (
        <header className="flex h-control-lg items-center gap-2 border-b border-border px-3">
          {title != null && <h3 className="min-w-0 truncate text-title">{title}</h3>}
          {actions != null && <div className="ml-auto flex items-center gap-1">{actions}</div>}
        </header>
      )}
      <div className={flush ? "" : "p-3"}>{children}</div>
    </section>
  );
}
