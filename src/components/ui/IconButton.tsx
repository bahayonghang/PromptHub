import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { cn } from "./cn";

export type IconButtonVariant = "ghost" | "bordered" | "danger";
export type IconButtonSize = "xs" | "sm" | "md" | "lg";

export interface IconButtonProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  /**
   * Accessible name. Required: an icon-only control has no text content, so
   * omitting this would ship an unlabelled button to assistive technology.
   */
  label: string;
  icon: ReactNode;
  variant?: IconButtonVariant;
  size?: IconButtonSize;
  /** Renders the icon in the accent colour (e.g. an active favourite star). */
  active?: boolean;
}

/**
 * Square icon-only control (design plan §5).
 *
 * This replaces the `iconButtonClass` constant that had been copy-pasted into
 * six separate files with slightly different strings. `label` drives both
 * `aria-label` and `title`, so the tooltip and the accessible name can never
 * drift apart.
 */
const BASE =
  "inline-flex shrink-0 items-center justify-center rounded-sm " +
  "transition-colors duration-fast ease-out " +
  "disabled:pointer-events-none disabled:opacity-50";

const VARIANTS: Record<IconButtonVariant, string> = {
  ghost: "text-muted-foreground hover:bg-state-hover hover:text-foreground",
  bordered:
    "border border-border bg-card text-muted-foreground shadow-hairline " +
    "hover:bg-state-hover hover:text-foreground hover:border-border-strong",
  danger: "text-muted-foreground hover:bg-destructive/15 hover:text-destructive",
};

const SIZES: Record<IconButtonSize, string> = {
  xs: "h-control-xs w-control-xs",
  sm: "h-control-sm w-control-sm",
  md: "h-control-md w-control-md",
  lg: "h-control-lg w-control-lg",
};

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  function IconButton(
    {
      label,
      icon,
      variant = "ghost",
      size = "md",
      active = false,
      className = "",
      type = "button",
      ...rest
    },
    ref,
  ) {
    return (
      <button
        ref={ref}
        type={type}
        aria-label={label}
        title={label}
        className={cn(
          BASE,
          VARIANTS[variant],
          SIZES[size],
          active && "text-primary",
          className,
        )}
        {...rest}
      >
        {icon}
      </button>
    );
  },
);
