import { cn } from "./cn";

export interface UsageBarProps {
  value: number;
  /** Upper bound for the fill; typically the max usage across the page. */
  max: number;
  /** Accessible description, e.g. "使用次数：12". */
  label: string;
  className?: string;
}

/**
 * Compact magnitude indicator for list and card rows (design plan §10.1).
 *
 * A bare number tells you the value but not the scale; the bar makes relative
 * magnitude scannable while the figure stays visible for the exact count. The
 * number uses tabular figures so the column does not jitter while scrolling.
 */
export function UsageBar({ value, max, label, className = "" }: UsageBarProps) {
  const safeMax = max > 0 ? max : 1;
  const pct = Math.max(0, Math.min(100, Math.round((value / safeMax) * 100)));

  return (
    <div className={cn("flex items-center gap-2", className)}>
      <div
        role="img"
        aria-label={label}
        className="h-1 min-w-[1.75rem] flex-1 overflow-hidden rounded-full bg-surface-inset"
      >
        <div
          className="h-full rounded-full bg-primary/70"
          style={{ width: `${pct}%` }}
        />
      </div>
      <span
        aria-hidden="true"
        className="w-6 shrink-0 text-right font-mono text-meta tabular-nums text-muted-foreground"
      >
        {value}
      </span>
    </div>
  );
}
