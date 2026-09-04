import { cn } from "./cn";

export interface SkeletonProps {
  className?: string;
}

/**
 * Pulse placeholder for content that has not arrived yet (design plan §6).
 *
 * Feature code should compose this rather than spelling `animate-pulse bg-muted`
 * at each call site. The one existing skeleton (AppearancePanel's font-list
 * pulse) already drifted from any shared recipe, which is how EmptyState sat
 * unused: nothing stopped the next loading view from inventing its own bar.
 *
 * The pulse is a CSS animation; `prefers-reduced-motion` already collapses
 * animation-duration to 0.01ms in `globals.css`, so this becomes a static
 * muted bar for those users without a second code path.
 */
export function Skeleton({ className = "" }: SkeletonProps) {
  return <div className={cn("animate-pulse rounded-sm bg-muted", className)} aria-hidden="true" />;
}
