import type { ReactNode } from "react";

/**
 * Label + control pair used throughout the workbench.
 *
 * Every form control here previously repeated
 * `flex flex-col gap-1 text-label text-muted-foreground` inline, and the label
 * text was a bare string sitting next to the control rather than a real
 * `<label>` wrapper in some places.
 */
export function Field({
  label,
  hint,
  children,
  className = "",
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={`flex min-w-0 flex-col gap-1 text-label text-muted-foreground ${className}`}>
      <span className="truncate">{label}</span>
      {children}
      {hint != null && <span className="text-meta text-muted-foreground-subtle">{hint}</span>}
    </label>
  );
}

/** Section heading inside a workbench panel. */
export function PanelHeading({
  children,
  action,
}: {
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <h3 className="text-label font-semibold text-foreground">{children}</h3>
      {action}
    </div>
  );
}
