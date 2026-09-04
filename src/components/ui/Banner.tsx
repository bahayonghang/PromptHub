import type { ReactNode } from "react";
import {
  AlertTriangleIcon,
  CheckCircle2Icon,
  InfoIcon,
  XCircleIcon,
} from "lucide-react";
import { cn } from "./cn";

export type BannerTone = "info" | "success" | "warning" | "danger";

const TONES: Record<
  BannerTone,
  { wrap: string; icon: string; Icon: typeof InfoIcon; role: "status" | "alert" }
> = {
  info: {
    wrap: "border-primary/40 bg-state-selected text-foreground",
    icon: "text-primary",
    Icon: InfoIcon,
    role: "status",
  },
  success: {
    wrap: "border-success/40 bg-success/10 text-foreground",
    icon: "text-success",
    Icon: CheckCircle2Icon,
    role: "status",
  },
  warning: {
    wrap: "border-warning/40 bg-warning/10 text-foreground",
    icon: "text-warning",
    Icon: AlertTriangleIcon,
    role: "status",
  },
  danger: {
    wrap: "border-destructive/40 bg-destructive/10 text-destructive",
    icon: "text-destructive",
    Icon: XCircleIcon,
    role: "alert",
  },
};

export interface BannerProps {
  tone?: BannerTone;
  children: ReactNode;
  /** Trailing content, typically an action button. */
  action?: ReactNode;
  /** Overrides the tone's default icon. Pass `null` to omit it. */
  icon?: ReactNode | null;
  className?: string;
}

/**
 * Full-width inline notice (design plan §6.7).
 *
 * Tone drives colour *and* icon *and* the ARIA live semantics together, so a
 * danger banner cannot accidentally be announced as a passive status the way
 * the hand-rolled banners could.
 */
export function Banner({
  tone = "info",
  children,
  action,
  icon,
  className = "",
}: BannerProps) {
  const spec = TONES[tone];
  const Icon = spec.Icon;

  return (
    <div
      role={spec.role}
      className={cn(
        "flex items-center gap-2 border-b px-4 py-2 text-body",
        spec.wrap,
        className,
      )}
    >
      {icon === null ? null : icon != null ? (
        <span className={cn("flex shrink-0 items-center", spec.icon)}>{icon}</span>
      ) : (
        <Icon className={cn("h-4 w-4 shrink-0", spec.icon)} aria-hidden="true" />
      )}
      <span className="min-w-0 flex-1">{children}</span>
      {action != null && <span className="shrink-0">{action}</span>}
    </div>
  );
}
