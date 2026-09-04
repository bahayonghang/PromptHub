import type { ReactNode } from "react";
import { cn } from "./cn";
import { tagClasses } from "./tagColor";

export interface TagProps {
  /** The tag name; also selects the palette slot. */
  name: string;
  /** Optional trailing count, rendered in tabular figures. */
  count?: number;
  /**
   * Renders an interactive tag. When set, the tag becomes a `<button>` with
   * `aria-pressed` so the selected state is exposed non-visually.
   */
  onToggle?: () => void;
  pressed?: boolean;
  /** Neutral rendering: no hue, for a folder or a type label. */
  plain?: boolean;
  icon?: ReactNode;
  className?: string;
}

const BASE =
  "inline-flex max-w-full shrink-0 items-center gap-1 rounded-sm border px-1.5 " +
  "text-meta leading-none h-5";

/**
 * Colour-coded tag (design plan §10.1).
 *
 * The hue is derived from the tag name, so the same tag reads identically in
 * the sidebar cloud, the filter bar, list rows, cards, and the detail panel.
 * Colour is never the only signal: a pressed tag also sets `aria-pressed` and
 * carries a stronger border.
 */
export function Tag({
  name,
  count,
  onToggle,
  pressed = false,
  plain = false,
  icon,
  className = "",
}: TagProps) {
  const tone = plain
    ? "border-transparent bg-surface-inset text-muted-foreground"
    : tagClasses(name, pressed);

  const content = (
    <>
      {icon ?? (!plain && <i aria-hidden="true" className="h-1 w-1 shrink-0 rounded-full bg-current" />)}
      <span className="min-w-0 truncate">{name}</span>
      {count != null && (
        <span className="font-mono text-micro opacity-70 tabular-nums">{count}</span>
      )}
    </>
  );

  if (!onToggle) {
    return <span className={cn(BASE, tone, className)}>{content}</span>;
  }

  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onToggle}
      className={cn(
        BASE,
        tone,
        "transition-colors duration-fast ease-out",
        "",
        className,
      )}
    >
      {content}
    </button>
  );
}
