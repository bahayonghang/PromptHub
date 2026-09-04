import { useId, type ReactNode } from "react";
import { cn } from "./cn";

export interface SettingRowProps {
  title: string;
  hint?: string;
  /** The control. Receives the generated ids so it can be labelled correctly. */
  children: (ids: { titleId: string; hintId: string }) => ReactNode;
  /**
   * `inline` puts the control on the right of the label (switches, compact
   * selects); `stacked` puts it underneath (wide inputs, radio grids).
   */
  layout?: "inline" | "stacked";
  /** Extra content rendered under the row, e.g. a save-status line. */
  footer?: ReactNode;
  className?: string;
}

/**
 * One labelled setting (design plan §6.7).
 *
 * Settings panels previously each re-declared `labelClass`/`hintClass` and
 * hand-built the label/hint/control arrangement, so spacing and typography
 * drifted between sections. Centralising it also makes the label-control
 * association consistent instead of per-call-site.
 */
export function SettingRow({
  title,
  hint,
  children,
  layout = "inline",
  footer,
  className = "",
}: SettingRowProps) {
  const base = useId();
  const titleId = `${base}-title`;
  const hintId = `${base}-hint`;

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div
        className={cn(
          layout === "inline"
            ? "flex items-center justify-between gap-4"
            : "flex flex-col gap-2",
        )}
      >
        <div className="flex min-w-0 flex-col gap-0.5">
          <span id={titleId} className="text-body font-medium text-foreground">
            {title}
          </span>
          {hint != null && (
            <span id={hintId} className="text-label text-muted-foreground">
              {hint}
            </span>
          )}
        </div>
        {children({ titleId, hintId })}
      </div>
      {footer}
    </div>
  );
}
