export type RunStatus =
  | "pending"
  | "running"
  | "success"
  | "error"
  | "cancelled"
  | "skipped";

const TONE: Record<RunStatus, string> = {
  pending: "bg-muted-foreground-subtle",
  running: "bg-primary animate-pulse",
  success: "bg-success",
  error: "bg-destructive",
  cancelled: "bg-muted-foreground",
  skipped: "bg-muted-foreground-subtle",
};

/**
 * A colour-only status marker.
 *
 * Deliberately `aria-hidden`: every call site already renders the status word
 * as visible text next to the dot, so exposing it again would make screen
 * readers announce the status twice. The dot is redundant encoding for sighted
 * users, not the primary signal — which also keeps it safe under greyscale.
 */
export function StatusDot({ status }: { status: string }) {
  const tone = TONE[status as RunStatus] ?? "bg-muted-foreground-subtle";
  return (
    <span
      aria-hidden="true"
      className={`inline-block h-1.5 w-1.5 shrink-0 rounded-full ${tone}`}
    />
  );
}
