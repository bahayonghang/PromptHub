import type { ReactNode } from "react";
import { cn } from "./cn";

/**
 * A single key cap (design plan §5). Used for shortcut hints in footers, the
 * command palette, and the status bar, where plain text such as "⌘S 保存" reads
 * like an unfinished placeholder.
 */
export function Kbd({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <kbd
      className={cn(
        "inline-flex h-[18px] min-w-[18px] items-center justify-center rounded-sm px-1",
        "border border-border bg-surface-inset font-mono text-micro",
        "text-muted-foreground-subtle",
        className,
      )}
    >
      {children}
    </kbd>
  );
}
